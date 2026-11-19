// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the item of a checked hybrid claim, run on the in-process hybrid
//! provider its negotiated candidate names and settled by the arm of the
//! provider's verdict: a solver result by FR-250's map, a claimed refutation
//! by FR-450's exact replay, and an unsupported construct as a located
//! `unsupported` record.

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::{CheckedPackage, EmitLimits};
use fsl_replay::{
    hybrid_locus_wire, HybridCounterexample, HybridCounterexampleKind, HybridReplayMeter,
    HybridReplayRefusal, HybridSettlement, HybridSolveRequest, HybridVerdict, TerminalValue,
    UnsupportedConstruct,
};
use fsl_route::Mode;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{Capability, CheckedGraph, CheckedHybridModel, DeclarationFamily};
use fsl_semantics::model::intake::PackageInput;
use serde_json::{Map, Value};

use super::{AnalyzeItem, DeadlineSupply, EngineReport, HybridItem, Refusal};

/// What a hybrid item settled: its value and, for `refuted`, the replayed
/// counterexample with its FR-286 spelling.
#[derive(Debug)]
pub(crate) struct HybridReport {
    pub(crate) value: TerminalValue,
    pub(crate) counterexample: Option<Box<HybridCounterexample>>,
    pub(crate) wire: Result<Value, fsl_replay::CounterexampleWireError>,
    /// The certificate the provider returned, as its entry wrote it. FR-751's
    /// checker is a later slice: nothing here checks it, so the record
    /// settles `uncertified` and keeps the certificate unchecked beside it.
    pub(crate) certificate: Option<Value>,
}

/// The package and request context a hybrid item reads to spell a locus.
#[derive(Clone, Copy)]
pub(super) struct Spelling<'a> {
    pub(super) packages: &'a PackageInput,
    pub(super) emit: EmitLimits,
}

/// The item of the hybrid claim at `claim`, or `None` when the package holds
/// no checked hybrid claim there.
pub(super) fn hybrid_item<'a>(
    package: &'a CheckedPackage,
    request_index: RequestIndex,
    claim: &OccurrenceKey,
    form: &'a HybridItem,
    work_units: u64,
    deadline: DeadlineSupply,
    spelling: Spelling<'a>,
) -> Option<AnalyzeItem<'a>> {
    let graph = package.graph();
    let position = graph
        .hybrid_claim_sites()
        .iter()
        .position(|site| &site.identity == claim.node() && &site.claim == claim.origin())?;
    let checked = graph.hybrid_claims().get(position)?;
    let model = graph.hybrid_models().get(checked.model.get())?;
    Some(
        AnalyzeItem::new_with_engine(
            request_index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            Mode::Bounded,
            fsl_hybrid_reference::BACKEND,
            move |cancel, engine| {
                let Some(provider) = engine.hybrid_provider() else {
                    return EngineReport::Refused(Refusal::Unsupported(CatalogCode::new(
                        "unsupported_projection",
                        "tool-unavailable",
                    )));
                };
                let bounds = engine.descriptor().solver_bounds();
                let budget = form.limits.resolved_budget(
                    bounds
                        .and_then(|bounds| bounds.max_solver_time)
                        .unwrap_or(u64::MAX),
                    bounds
                        .and_then(|bounds| bounds.max_solver_steps)
                        .unwrap_or(u64::MAX),
                );
                let request = HybridSolveRequest {
                    claim: checked,
                    model,
                    budget,
                    options: form.options.clone(),
                    deadline_ms: match deadline {
                        DeadlineSupply::Unbounded => None,
                        DeadlineSupply::Milliseconds(milliseconds) => Some(milliseconds),
                    },
                };
                let answer = provider.solve(&request, cancel);
                let certificate = answer.certificate;
                let mut meter = HybridReplayMeter::new(work_units);
                let settled = match answer.verdict {
                    HybridVerdict::Solved(result) => result.settle(graph, checked, &mut meter),
                    HybridVerdict::Refuted(claimed) => HybridSettlement::from_provider_counterexample(
                        graph, checked, *claimed, &mut meter,
                    ),
                    HybridVerdict::Unsupported(construct) => {
                        return unsupported_construct(
                            package, graph, spelling, position, model, &construct,
                        );
                    }
                };
                match settled {
                    Ok(settled) => {
                        let wire = match &settled.counterexample {
                            Some(found) => counterexample_wire(
                                package, graph, spelling, position, model, found,
                            ),
                            None => Ok(Value::Null),
                        };
                        EngineReport::Hybrid(Box::new(HybridReport {
                            value: settled.value,
                            counterexample: settled.counterexample.map(Box::new),
                            wire,
                            certificate,
                        }))
                    }
                    Err(refusal) => EngineReport::Refused(Refusal::Failed(fault_of(&refusal))),
                }
            },
        )
        .hybrid(),
    )
}

/// The `failed` diagnostic of a replay refusal. A trace that breaks FSpec
/// FR-193 "Hybrid trace" is the provider's fault, `invalid_result`/
/// `invalid-value`, as the reader refuses it. FR-750 gives no settlement for
/// the other refusals (a claim form exact replay does not decide, a
/// counterexample kind the replay does not admit, an unreadable model or
/// function), so they keep the conservative `runtime_invariant`.
fn fault_of(refusal: &HybridReplayRefusal) -> CatalogCode {
    match refusal {
        HybridReplayRefusal::Trace(_) => CatalogCode::new("invalid_result", "invalid-value"),
        HybridReplayRefusal::UnknownModel(_)
        | HybridReplayRefusal::Expansion(_)
        | HybridReplayRefusal::Exact(_)
        | HybridReplayRefusal::UnsupportedCounterexampleKind
        | HybridReplayRefusal::ResourceExhausted { .. } => {
            CatalogCode::new("runtime_invariant", "established-invariant-broken")
        }
    }
}

/// The record of a provider's `Unsupported` verdict: the FSpec FR-272
/// `expression-form` detail with the construct's `locus`, or an engine fault
/// when the model has no such mode, variable, expression or term.
fn unsupported_construct(
    package: &CheckedPackage,
    graph: &CheckedGraph,
    spelling: Spelling<'_>,
    claim: usize,
    model: &CheckedHybridModel,
    construct: &UnsupportedConstruct,
) -> EngineReport {
    let fault = || {
        EngineReport::Refused(Refusal::Failed(CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken",
        )))
    };
    let Some(mode) = model.modes.get(construct.mode.get()) else {
        return fault();
    };
    let variable = match construct.variable {
        Some(index) => match model.variables.get(index.get()) {
            Some(variable) => Some(variable),
            None => return fault(),
        },
        None => None,
    };
    let located = wire_graph(package, spelling).and_then(|wire| {
        hybrid_locus_wire(
            graph,
            &wire,
            claim,
            &fsl_replay::ExpressionLocus {
                site: construct.site,
                term: construct.term.clone(),
            },
        )
    });
    let Some(locus) = located else {
        return fault();
    };
    let mut detail = Map::new();
    detail.insert("form".to_owned(), construct.form.clone().into());
    detail.insert("declaration".to_owned(), model.name.as_str().into());
    detail.insert("mode".to_owned(), mode.name.as_str().into());
    if let Some(variable) = variable {
        detail.insert("variable".to_owned(), variable.name.as_str().into());
    }
    detail.insert("locus".to_owned(), locus);
    EngineReport::Refused(Refusal::UnsupportedConstruct(detail))
}

/// The graph of the package emitted for this request, whose node ids spell
/// a `Locus`.
fn wire_graph(package: &CheckedPackage, spelling: Spelling<'_>) -> Option<WireGraph> {
    let bytes = crate::provider::admitted_bytes(package, spelling.packages, spelling.emit)?;
    WireGraph::of_package_bytes(bytes.bytes()).ok()
}

/// The reproduced counterexample in FR-286's spelling. A counterexample whose
/// locus the package cannot spell stays the item's counterexample with its
/// spelling refused: the item never settles `failed` for it (FR-750).
fn counterexample_wire(
    package: &CheckedPackage,
    graph: &CheckedGraph,
    spelling: Spelling<'_>,
    claim: usize,
    model: &CheckedHybridModel,
    found: &HybridCounterexample,
) -> Result<Value, fsl_replay::CounterexampleWireError> {
    let unspelled = || fsl_replay::CounterexampleWireError::Unspelled("hybrid locus");
    match &found.kind {
        HybridCounterexampleKind::Formula => Ok(found.to_wire(model, &|_| Value::Null)),
        HybridCounterexampleKind::UndefinedEvaluation { at, .. } => {
            let wire = wire_graph(package, spelling).ok_or_else(unspelled)?;
            let locus = hybrid_locus_wire(graph, &wire, claim, &at.locus).ok_or_else(unspelled)?;
            Ok(found.to_wire(model, &|_| locus.clone()))
        }
    }
}
