// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 "Dispatch": a hybrid item whose candidate is a process provider.
//! FSL writes the FSpec FR-331 request stage (the emitted package's v2 bytes
//! and the item, with the registry snapshot its candidates are checked
//! against), hands its bytes to the engine's exchange, reads the FR-331
//! outcome stage the exchange returns and turns the item's one result entry
//! into a [`HybridAnswer`], which the in-process path then settles by the
//! same rules. The exchange carries bytes only: starting the process, FR-462
//! framing, limits, the deadline and cancellation are the driver's plugin
//! host (FR-635).

use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::fairness::FairnessKind;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_replay::planner::{read_result_entry, EntryOrigin};
use fsl_replay::{
    read_plugin_certification, HybridAnswer, HybridEntryRefusal, HybridProcessDependency,
    HybridProcessItem, HybridProcessPackageContext, HybridProcessRequest, HybridProcessStage,
    SolverBudget, TerminalValue, ToolFailure,
};
use fsl_route::BackendDescriptor;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{Capability, CheckedGraph, CheckedHybridClaim};
use serde_json::{json, Map, Value};

use super::hybrid_item::HybridReport;
use super::{BackendTool, EngineReport, Refusal};

/// The checked package feature FSL's reader supports for the request's
/// package (FSpec FR-322).
const SUPPORTED_FEATURE: &str = "filament.value.complete/v1";

const CONTRACT_VERSION: &str = "filament.backend-provider/v1";
const CAPABILITY_VOCABULARY: &str = "filament.capability-kind/v1";

/// The members of an FSpec FR-331 outcome stage, all required.
const OUTCOME_MEMBERS: [&str; 7] = [
    "contract_version",
    "capability_vocabulary",
    "dispositions",
    "artifacts",
    "results",
    "counterexamples",
    "accounting",
];

/// Everything one process hybrid item is asked and read against.
pub(super) struct ProcessCall<'a> {
    /// The engine's exchange.
    pub(super) exchange: &'a dyn super::HybridProcessExchange,
    /// The engine's backend identity.
    pub(super) backend: &'a BackendId,
    /// The item's candidate set, as negotiation computed it.
    pub(super) candidates: &'a [BackendId],
    /// The registry snapshot the candidates are checked against: one FSpec
    /// FR-331 `BackendDescriptor` per registered backend.
    pub(super) manifest: &'a [Value],
    /// The emitted package's v2 bytes.
    pub(super) package: &'a [u8],
    /// The package's dependency closure, each package by identity.
    pub(super) dependencies: &'a [HybridProcessDependency],
    /// The item's claim occurrence in the checked package.
    pub(super) claim: &'a OccurrenceKey,
    /// The checked graph and the claim read from it.
    pub(super) graph: &'a CheckedGraph,
    pub(super) checked: &'a CheckedHybridClaim,
    /// The emitted package's node ids.
    pub(super) wire: &'a WireGraph,
    /// The item's own `hybrid.limits`, each left unset when the request set
    /// none: the provider's published bounds supply the default.
    pub(super) budget: SolverBudget,
    /// The item's FSpec FR-331 `options`.
    pub(super) options: &'a std::collections::BTreeMap<String, String>,
    /// The run's provider deadline (FSpec FR-462).
    pub(super) deadline_ms: u64,
}

/// The item's answer, or the report that settles it without one.
#[allow(clippy::result_large_err, reason = "failure directly settles the item with its existing EngineReport API rather than an intermediate error")]
pub(super) fn answer(
    call: &ProcessCall<'_>,
    cancel: &filament_exact::Cancel,
) -> Result<HybridAnswer, EngineReport> {
    let request = compose(call)?;
    let body = call
        .exchange
        .exchange(request.bytes(), cancel)
        .map_err(tool_failed)?;
    let outcome: Value =
        serde_json::from_slice(&body).map_err(|_| tool_failed(ToolFailure::MalformedFrame))?;
    let (entry, counterexamples) = item_entry(&outcome, call.backend)?;
    if entry.get("result").and_then(Value::as_str) == Some("proved") {
        read_plugin_certification(entry.get("certification")).map_err(|refusal| {
            let (code, cause, pointer) = refusal.catalog();
            invalid(code, cause, pointer)
        })?;
    }
    match HybridAnswer::read_entry(entry, &counterexamples, call.graph, call.checked, call.wire) {
        Ok(answer) => Ok(answer),
        Err(HybridEntryRefusal::Entry(refusal)) => {
            let (code, cause, pointer) = refusal.catalog();
            Err(invalid(code, cause, pointer))
        }
        Err(HybridEntryRefusal::UnknownModel(model)) => {
            let mut detail = Map::new();
            detail.insert("invariant".to_owned(), "hybrid-claim-model".into());
            detail.insert("model".to_owned(), model.get().into());
            Err(EngineReport::Refused(Refusal::FailedWithDetail(
                CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                detail,
            )))
        }
        Err(HybridEntryRefusal::NotRead(_)) => Err(unread_entry(entry)),
    }
}

/// The FSpec FR-331 request stage for the item.
#[allow(clippy::result_large_err, reason = "failure directly settles the item with its existing EngineReport API rather than an intermediate error")]
fn compose(call: &ProcessCall<'_>) -> Result<HybridProcessRequest, EngineReport> {
    let node = call
        .wire
        .wire_value(call.claim.node())
        .map_err(|_| tool_failed(ToolFailure::MalformedBody))?;
    let occurrence = call.claim.origin();
    let mut backends: Vec<&str> = call.candidates.iter().map(BackendId::as_str).collect();
    backends.sort_unstable();
    let item = HybridProcessItem::new(
        RequestIndex::new(0),
        json!({
            "node": node,
            "occurrence": {"role": occurrence.role().as_str(), "ordinal": occurrence.ordinal()},
        }),
        json!({"classification": "bounded", "bounds": []}),
        json!({"type": "set", "backends": backends}),
        call.budget,
    )
    .with_options(call.options.clone());
    let mut context = HybridProcessPackageContext::new();
    context.support_feature(SUPPORTED_FEATURE);
    for dependency in call.dependencies {
        let package = context
            .read_dependency_package(&dependency.package)
            .map_err(|_| tool_failed(ToolFailure::MalformedBody))?;
        context
            .insert_dependency_package(dependency.identity.as_str(), package)
            .map_err(|_| tool_failed(ToolFailure::MalformedBody))?;
    }
    let stage = HybridProcessStage {
        manifest: call.manifest.to_vec(),
        dependencies: call.dependencies.to_vec(),
    };
    HybridProcessRequest::compose(call.package, &context, call.deadline_ms, &[item], &stage)
        .map_err(|_| tool_failed(ToolFailure::MalformedBody))
}

/// The one result entry of the outcome stage that answers item 0, and the
/// stage's counterexample objects by position (FSpec FR-331: a `refuted`
/// entry's `counterexample` is a position in `counterexamples`).
///
/// A body that is not an outcome stage is the FR-462 `malformed-body`
/// failure; an entry or a counterexample of another item or backend is not
/// the provider's answer to this request, `invalid_result`.
#[allow(clippy::result_large_err, reason = "failure directly settles the item with its existing EngineReport API rather than an intermediate error")]
fn item_entry<'a>(
    outcome: &'a Value,
    backend: &BackendId,
) -> Result<(&'a Value, Vec<Value>), EngineReport> {
    let body = ToolFailure::MalformedBody;
    let members = outcome.as_object().ok_or_else(|| tool_failed(body))?;
    let stage = members.len() == OUTCOME_MEMBERS.len()
        && OUTCOME_MEMBERS.iter().all(|name| members.contains_key(*name))
        && members["contract_version"] == CONTRACT_VERSION
        && members["capability_vocabulary"] == CAPABILITY_VOCABULARY
        && ["dispositions", "artifacts", "results", "counterexamples", "accounting"]
            .iter()
            .all(|name| members[*name].is_array());
    if !stage {
        return Err(tool_failed(body));
    }
    let mut answering = None;
    for entry in members["results"].as_array().into_iter().flatten() {
        match entry.get("request_index").and_then(Value::as_u64) {
            Some(0) => {
                if answering.replace(entry).is_some() {
                    return Err(tool_failed(body));
                }
            }
            Some(_) => return Err(tool_failed(ToolFailure::UnroutedItem)),
            None => return Err(tool_failed(body)),
        }
    }
    let entry = answering.ok_or_else(|| tool_failed(ToolFailure::MissingTerminalRecord))?;
    match entry.get("kind").and_then(Value::as_str) {
        Some("temporal-satisfaction") => {}
        Some(_) => return Err(tool_failed(ToolFailure::UnadvertisedKind)),
        None => return Err(invalid("invalid_result", "missing-member", Some("/kind"))),
    }
    if entry.get("backend").and_then(Value::as_str) != Some(backend.as_str()) {
        return Err(invalid("invalid_result", "invalid-value", Some("/backend")));
    }
    let mut counterexamples = Vec::new();
    for (position, found) in members["counterexamples"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let ours = found.get("request_index").and_then(Value::as_u64) == Some(0)
            && found.get("backend").and_then(Value::as_str) == Some(backend.as_str());
        if !ours {
            let pointer = format!("/counterexamples/{position}");
            return Err(invalid_at(
                "invalid_result",
                "invalid-value",
                &pointer,
            ));
        }
        counterexamples.push(found.get("counterexample").cloned().unwrap_or(Value::Null));
    }
    Ok((entry, counterexamples))
}

/// FR-290's settlement of an admitted entry that is not a solver answer:
/// `failed`, `unsupported`, and the `inconclusive` and `incomplete` causes a
/// solver result cannot hold. No other result is a hybrid provider's answer:
/// a `proved`, `tested`, `measured`, `declined` or `refuted` entry that
/// `read_entry` did not read is refused, never settled as the entry says.
fn unread_entry(entry: &Value) -> EngineReport {
    match entry.get("result").and_then(Value::as_str) {
        Some("failed") => match entry.get("cause").and_then(failure_of) {
            Some(failure) => tool_failed(failure),
            None => report(TerminalValue::Failed),
        },
        Some("unsupported" | "inconclusive" | "incomplete") => {
            match read_result_entry(entry, 0, EntryOrigin::Plugin) {
                Ok(read)
                    if matches!(
                        read.record.value(),
                        TerminalValue::Unsupported(_)
                            | TerminalValue::Inconclusive(_)
                            | TerminalValue::Incomplete(_)
                    ) =>
                {
                    report(read.record.value().clone())
                }
                Ok(_) => invalid("invalid_result", "invalid-value", Some("/result")),
                Err(refusal) => invalid(
                    refusal.cause.code(),
                    refusal.cause.as_str(),
                    Some(&refusal.pointer),
                ),
            }
        }
        _ => invalid("invalid_result", "invalid-value", Some("/result")),
    }
}

/// A `tool-failure` cause as the wire spells it (`type`), read through the
/// outcome document's reader (`kind`).
fn failure_of(cause: &Value) -> Option<ToolFailure> {
    let mut members = cause.as_object()?.clone();
    let spelled = members.remove("type")?;
    members.insert("kind".to_owned(), spelled);
    ToolFailure::read_cause(&Value::Object(members))
}

fn report(value: TerminalValue) -> EngineReport {
    EngineReport::Hybrid(Box::new(HybridReport {
        value,
        counterexample: None,
        wire: Ok(Value::Null),
        certificate: None,
        disagreement: None,
    }))
}

fn tool_failed(failure: ToolFailure) -> EngineReport {
    report(TerminalValue::ToolFailed(failure))
}

/// `failed` with the FR-272 `code`/`cause` of a refused entry at `pointer`.
fn invalid(code: &'static str, cause: &'static str, pointer: Option<&str>) -> EngineReport {
    let mut detail = Map::new();
    if let Some(pointer) = pointer {
        detail.insert("pointer".to_owned(), pointer.into());
    }
    EngineReport::Refused(Refusal::FailedWithDetail(
        CatalogCode::new(code, cause),
        detail,
    ))
}

fn invalid_at(code: &'static str, cause: &'static str, pointer: &str) -> EngineReport {
    invalid(code, cause, Some(pointer))
}

/// The FSpec FR-331 `BackendDescriptor` of a registered backend: the
/// snapshot member FSL writes for it in a process request.
pub(super) fn descriptor_wire(descriptor: &BackendDescriptor, tool: &BackendTool) -> Value {
    let mut advertises: Vec<(&str, &str)> = descriptor
        .advertises()
        .map(|(kind, mode)| (kind.to_wire(), mode.to_wire()))
        .collect();
    let advertised = |wanted: Capability| descriptor.advertises().any(|(kind, _)| kind == wanted);
    advertises.sort_unstable();
    let mut members = Map::new();
    members.insert("backend".to_owned(), descriptor.id().as_str().into());
    members.insert(
        "tool".to_owned(),
        json!({"name": tool.name, "version": tool.version}),
    );
    members.insert(
        "advertises".to_owned(),
        advertises
            .iter()
            .map(|(kind, mode)| json!({"kind": kind, "mode": mode}))
            .collect(),
    );
    let labels = |values: Vec<&str>| Value::from(values);
    if let Some(domains) = descriptor.domains().filter(|domains| !domains.is_empty()) {
        members.insert(
            "domains".to_owned(),
            labels(domains.iter().map(|kind| kind.to_wire()).collect()),
        );
    }
    if !descriptor.option_defaults().is_empty() {
        members.insert("options".to_owned(), json!(descriptor.option_defaults()));
    }
    let mut bounds = Map::new();
    if let Some(published) = descriptor.solver_bounds() {
        if let Some(steps) = published.max_solver_steps {
            bounds.insert("max_solver_steps".to_owned(), steps.into());
        }
        if let Some(time) = published.max_solver_time {
            bounds.insert("max_solver_time".to_owned(), time.into());
        }
    }
    if let Some(cases) = descriptor.max_cases() {
        bounds.insert("max_cases".to_owned(), cases.get().into());
    }
    if !bounds.is_empty() {
        members.insert("bounds".to_owned(), Value::Object(bounds));
    }
    if !descriptor.reductions().is_empty() {
        members.insert(
            "reductions".to_owned(),
            labels(descriptor.reductions().iter().map(|kind| kind.as_str()).collect()),
        );
    }
    if advertised(Capability::ProbabilisticSatisfaction) {
        members.insert(
            "evidence_kinds".to_owned(),
            labels(descriptor.evidence_kinds().iter().map(|kind| kind.to_wire()).collect()),
        );
    }
    if advertised(Capability::TemporalSatisfaction) {
        members.insert(
            "fairness_kinds".to_owned(),
            labels(
                descriptor
                    .fairness_kinds()
                    .iter()
                    .map(|kind| match kind {
                        FairnessKind::Weak => "weak",
                        FairnessKind::Strong => "strong",
                    })
                    .collect(),
            ),
        );
        if !descriptor.memory_models().is_empty() {
            members.insert(
                "memory_models".to_owned(),
                labels(descriptor.memory_models().iter().map(|model| model.label()).collect()),
            );
        }
        if !descriptor.state_graph_forms().is_empty() {
            members.insert(
                "state_graph_forms".to_owned(),
                labels(descriptor.state_graph_forms().iter().map(|form| form.as_str()).collect()),
            );
        }
        if let Some(dynamics) = descriptor.dynamics() {
            members.insert(
                "dynamics".to_owned(),
                labels(dynamics.iter().map(|kind| kind.to_wire()).collect()),
            );
        }
    }
    Value::Object(members)
}
