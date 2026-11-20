// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 "Dispatch": a hybrid item whose candidate is a process provider.
//! FSL writes the FR-447 process request (the emitted package's v2 bytes and
//! the item), hands its bytes to the engine's exchange, reads the FSpec
//! FR-331 outcome stage the exchange returns and turns the item's one result
//! entry into a [`HybridAnswer`], which the in-process path then settles by
//! the same rules. The exchange carries bytes only: starting the process,
//! FR-462 framing, limits, the deadline and cancellation are the driver's
//! plugin host (FR-635).

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::planner::{read_result_entry, EntryOrigin};
use fsl_replay::{
    read_plugin_certification, HybridAnswer, HybridEntryRefusal, HybridProcessItem,
    HybridProcessPackageContext, HybridProcessRequest, SolverBudget, TerminalValue, ToolFailure,
};
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{CheckedGraph, CheckedHybridClaim};
use serde_json::{json, Map, Value};

use super::hybrid_item::HybridReport;
use super::{EngineReport, Refusal};

/// The checked package feature FSL's reader supports for the request's
/// package (FSpec FR-322).
const SUPPORTED_FEATURE: &str = "filament.value.complete/v1";

/// Everything one process hybrid item is asked and read against.
pub(super) struct ProcessCall<'a> {
    /// The engine's exchange.
    pub(super) exchange: &'a dyn super::HybridProcessExchange,
    /// The engine's backend identity, the item's one candidate.
    pub(super) backend: &'a str,
    /// The emitted package's v2 bytes.
    pub(super) package: &'a [u8],
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
    let (entry, counterexamples) = item_entry(&outcome).map_err(tool_failed)?;
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

/// The FR-447 process request for the item.
fn compose(call: &ProcessCall<'_>) -> Result<HybridProcessRequest, EngineReport> {
    let node = call
        .wire
        .wire_value(call.claim.node())
        .map_err(|_| tool_failed(ToolFailure::MalformedBody))?;
    let occurrence = call.claim.origin();
    let item = HybridProcessItem::new(
        RequestIndex::new(0),
        json!({
            "node": node,
            "occurrence": {"role": occurrence.role().as_str(), "ordinal": occurrence.ordinal()},
        }),
        json!({"classification": "bounded", "bounds": []}),
        json!({"type": "set", "backends": [call.backend]}),
        call.budget,
    )
    .with_options(call.options.clone());
    let mut context = HybridProcessPackageContext::new();
    context.support_feature(SUPPORTED_FEATURE);
    HybridProcessRequest::compose(call.package, &context, call.deadline_ms, &[item])
        .map_err(|_| tool_failed(ToolFailure::MalformedBody))
}

/// The one result entry of the outcome stage that answers item 0, and the
/// stage's counterexample objects by position (FSpec FR-331: a `refuted`
/// entry's `counterexample` is a position in `counterexamples`).
fn item_entry(outcome: &Value) -> Result<(&Value, Vec<Value>), ToolFailure> {
    let results = outcome
        .get("results")
        .and_then(Value::as_array)
        .ok_or(ToolFailure::MalformedBody)?;
    let mut answering = None;
    for entry in results {
        match entry.get("request_index").and_then(Value::as_u64) {
            Some(0) => {
                if answering.replace(entry).is_some() {
                    return Err(ToolFailure::MalformedBody);
                }
            }
            Some(_) => return Err(ToolFailure::UnroutedItem),
            None => return Err(ToolFailure::MalformedBody),
        }
    }
    let entry = answering.ok_or(ToolFailure::MissingTerminalRecord)?;
    if entry
        .get("kind")
        .is_some_and(|kind| kind != "temporal-satisfaction")
    {
        return Err(ToolFailure::UnadvertisedKind);
    }
    let counterexamples = match outcome.get("counterexamples") {
        None => Vec::new(),
        Some(Value::Array(found)) => found
            .iter()
            .map(|found| found.get("counterexample").cloned().unwrap_or(Value::Null))
            .collect(),
        Some(_) => return Err(ToolFailure::MalformedBody),
    };
    Ok((entry, counterexamples))
}

/// FR-290's settlement of an admitted entry that is not a solver answer.
fn unread_entry(entry: &Value) -> EngineReport {
    if entry.get("result").and_then(Value::as_str) == Some("failed") {
        return match entry.get("cause").and_then(failure_of) {
            Some(failure) => tool_failed(failure),
            None => report(TerminalValue::Failed),
        };
    }
    match read_result_entry(entry, 0, EntryOrigin::Plugin) {
        Ok(read) => report(read.record.value().clone()),
        Err(refusal) => invalid(
            refusal.cause.code(),
            refusal.cause.as_str(),
            Some(&refusal.pointer),
        ),
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
