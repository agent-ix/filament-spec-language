// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the item of a checked hybrid claim, run on the in-process hybrid
//! provider its negotiated candidate names and settled by the arm of the
//! provider's verdict: a solver result by FR-250's map, a claimed refutation
//! by FR-450's exact replay, and an unsupported construct as a located
//! `unsupported` record. A provider's certificate is not read here: FR-751's
//! checker is a later slice (QSL-2531), so the item settles `uncertified` and
//! the answer's certificate is neither checked nor written to the outcome.

use filament_exact::LimitKind;
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::{CheckedPackage, EmitLimits};
use fsl_replay::{
    hybrid_locus_wire, ExactGap, HybridAnswer, HybridCounterexample, HybridCounterexampleKind,
    HybridReplayMeter, HybridReplayRefusal, HybridSettlement, HybridSolveRequest, HybridVerdict,
    IncompleteCause, ModelCheckLimit, TerminalValue, UnsupportedConstruct,
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
    claim: &'a OccurrenceKey,
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
                if let Some(exchange) = engine.hybrid_exchange() {
                    return process_provider(
                        exchange,
                        engine,
                        package,
                        &claim,
                        graph,
                        checked,
                        model,
                        position,
                        work_units,
                        budget_for(&form, engine),
                        deadline,
                        spelling,
                        cancel,
                    );
                }
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
                // The package is emitted once for the item. When FSL cannot
                // emit it, no locus can be spelled or read: an FSL fault, not
                // the provider's.
                let Some(wire) = wire_graph(package, spelling) else {
                    return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    )));
                };
                let request = HybridSolveRequest {
                    graph,
                    wire: &wire,
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
                let mut meter = HybridReplayMeter::new(work_units);
                let settled = match answer.verdict {
                    HybridVerdict::Solved(result) => result.settle(graph, checked, &mut meter),
                    HybridVerdict::Refuted(claimed) => {
                        HybridSettlement::from_provider_counterexample(
                            graph, checked, *claimed, &mut meter,
                        )
                    }
                    HybridVerdict::Unsupported(construct) => {
                        return unsupported_construct(graph, &wire, position, model, &construct);
                    }
                };
                match settled {
                    Ok(settled) => {
                        let wire = match &settled.counterexample {
                            Some(found) => {
                                counterexample_wire(graph, &wire, position, model, found)
                            }
                            None => Ok(Value::Null),
                        };
                        EngineReport::Hybrid(Box::new(HybridReport {
                            value: settled.value,
                            counterexample: settled.counterexample.map(Box::new),
                            wire,
                        }))
                    }
                    Err(refusal) => fault_of(&refusal, work_units),
                }
            },
        )
        .hybrid(),
    )
}

/// Run one hybrid item through the process exchange and settle its entry
/// through the same reader and replay path as an in-process provider.
#[allow(clippy::too_many_arguments)]
fn process_provider(
    exchange: &dyn super::HybridProcessExchange,
    engine: &super::Engine,
    package: &CheckedPackage,
    claim: &OccurrenceKey,
    graph: &CheckedGraph,
    checked: &fsl_semantics::check::CheckedHybridClaim,
    model: &CheckedHybridModel,
    position: usize,
    work_units: u64,
    budget: fsl_replay::SolverBudget,
    deadline: DeadlineSupply,
    spelling: Spelling<'_>,
    cancel: &filament_exact::Cancel,
) -> EngineReport {
    let failed = |failure| {
        EngineReport::Hybrid(Box::new(HybridReport {
            value: TerminalValue::ToolFailed(failure),
            counterexample: None,
            wire: Ok(Value::Null),
        }))
    };
    let Some(emitted) = crate::provider::admitted_bytes(package, spelling.packages, spelling.emit)
    else {
        return EngineReport::Refused(Refusal::Unsupported(CatalogCode::new(
            "unsupported_projection",
            "tool-unavailable",
        )));
    };
    let Ok(wire) = WireGraph::of_package_bytes(emitted.bytes()) else {
        return failed(fsl_replay::ToolFailure::MalformedBody);
    };
    let Ok(node) = wire.wire_value(claim.node()) else {
        return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken",
        )));
    };
    let occurrence = claim.origin();
    let item = fsl_replay::HybridProcessItem::new(
        fsl_foundation::RequestIndex::new(0),
        serde_json::json!({
            "node": node,
            "occurrence": {"role": occurrence.role().as_str(), "ordinal": occurrence.ordinal()},
        }),
        serde_json::json!({"classification": "bounded", "bounds": []}),
        serde_json::json!({"type": "set", "backends": [engine.descriptor().id().as_str()]}),
        budget,
    );
    let mut context = fsl_replay::HybridProcessPackageContext::new();
    context.support_feature("filament.value.complete/v1");
    let deadline_ms = match deadline {
        DeadlineSupply::Unbounded => 600_000,
        DeadlineSupply::Milliseconds(value) => value,
    };
    let request = match fsl_replay::HybridProcessRequest::compose(
        emitted.bytes(),
        &context,
        deadline_ms,
        &[item],
    ) {
        Ok(request) => request,
        Err(_) => return failed(fsl_replay::ToolFailure::MalformedBody),
    };
    let response = match exchange.exchange(request.bytes(), cancel) {
        Ok(response) => response,
        Err(failure) => return failed(failure),
    };
    let entry: Value = match serde_json::from_slice(&response) {
        Ok(entry) => entry,
        Err(_) => return failed(fsl_replay::ToolFailure::MalformedFrame),
    };
    let counterexamples = entry["counterexamples"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let answer = match HybridAnswer::read_entry(&entry, counterexamples, graph, checked, &wire) {
        Ok(answer) => answer,
        Err(fsl_replay::HybridEntryRefusal::NotRead(_)) => {
            if let Some(failure) = fsl_replay::ToolFailure::read_cause(&entry["cause"]) {
                return failed(failure);
            }
            return EngineReport::Refused(Refusal::Unsupported(CatalogCode::new(
                "unsupported_projection",
                "tool-unavailable",
            )));
        }
        Err(_) => {
            return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                "invalid_result",
                "invalid-value",
            )))
        }
    };
    let mut meter = HybridReplayMeter::new(work_units);
    let settled = match answer.verdict {
        HybridVerdict::Solved(result) => result.settle(graph, checked, &mut meter),
        HybridVerdict::Refuted(claimed) => {
            HybridSettlement::from_provider_counterexample(graph, checked, *claimed, &mut meter)
        }
        HybridVerdict::Unsupported(construct) => {
            return unsupported_construct(graph, &wire, position, model, &construct);
        }
    };
    match settled {
        Ok(settled) => {
            let wire = match &settled.counterexample {
                Some(found) => counterexample_wire(graph, &wire, position, model, found),
                None => Ok(Value::Null),
            };
            EngineReport::Hybrid(Box::new(HybridReport {
                value: settled.value,
                counterexample: settled.counterexample.map(Box::new),
                wire,
            }))
        }
        Err(refusal) => fault_of(&refusal, work_units),
    }
}

fn budget_for(form: &super::HybridItem, engine: &super::Engine) -> fsl_replay::SolverBudget {
    let bounds = engine.descriptor().solver_bounds();
    form.limits.resolved_budget(
        bounds
            .and_then(|bounds| bounds.max_solver_time)
            .unwrap_or(u64::MAX),
        bounds
            .and_then(|bounds| bounds.max_solver_steps)
            .unwrap_or(u64::MAX),
    )
}

/// The settlement of a replay refusal, by FR-750 "Replay refusals" (analyze
/// path). A trace that breaks FSpec FR-193 "Hybrid trace" is the provider's
/// fault, `invalid_result`/`invalid-value`. A claim whose model is missing, a
/// call whose function cannot be expanded and an ill-kinded term are checked
/// package faults, `failed`, `runtime_invariant`/`established-invariant-broken`
/// naming the invariant. An exhausted `work_units` meter is `incomplete`.
fn fault_of(refusal: &HybridReplayRefusal, work_units: u64) -> EngineReport {
    let invariant = |name: &str, extra: Vec<(&str, Value)>| {
        let mut detail = Map::new();
        detail.insert("invariant".to_owned(), name.into());
        for (member, value) in extra {
            detail.insert(member.to_owned(), value);
        }
        EngineReport::Refused(Refusal::FailedWithDetail(
            CatalogCode::new("runtime_invariant", "established-invariant-broken"),
            detail,
        ))
    };
    match refusal {
        HybridReplayRefusal::Trace(trace) => {
            let mut detail = Map::new();
            detail.insert("pointer".to_owned(), trace.pointer.to_string().into());
            EngineReport::Refused(Refusal::FailedWithDetail(
                CatalogCode::new("invalid_result", "invalid-value"),
                detail,
            ))
        }
        HybridReplayRefusal::UnknownModel(model) => {
            invariant("hybrid-claim-model", vec![("model", model.get().into())])
        }
        HybridReplayRefusal::Expansion(gap) => invariant(
            "hybrid-call-expansion",
            vec![("gap", format!("{gap}").into())],
        ),
        HybridReplayRefusal::Exact(gap) => match gap {
            ExactGap::IllKindedTerm => invariant("hybrid-term-kind", Vec::new()),
            // `exact_replay` settles the other gaps `NotExact`, so none
            // reaches here.
            other => invariant(
                "hybrid-exact-replay",
                vec![("gap", format!("{other}").into())],
            ),
        },
        HybridReplayRefusal::ResourceExhausted { .. } => {
            EngineReport::Refused(Refusal::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: LimitKind::WorkUnits,
                },
                value: work_units,
            }))
        }
    }
}

/// The record of a provider's `Unsupported` verdict: the FSpec FR-272
/// `expression-form` detail with the construct's `locus`, or an engine fault
/// when the model has no such mode, variable, expression or term.
fn unsupported_construct(
    graph: &CheckedGraph,
    wire: &WireGraph,
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
    let located = hybrid_locus_wire(
        graph,
        wire,
        claim,
        &fsl_replay::ExpressionLocus {
            site: construct.site,
            term: construct.term.clone(),
        },
    );
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
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    model: &CheckedHybridModel,
    found: &HybridCounterexample,
) -> Result<Value, fsl_replay::CounterexampleWireError> {
    let unspelled = || fsl_replay::CounterexampleWireError::Unspelled("hybrid locus");
    match &found.kind {
        HybridCounterexampleKind::Formula => Ok(found.to_wire(model, &|_| Value::Null)),
        HybridCounterexampleKind::UndefinedEvaluation { at, .. } => {
            let locus = hybrid_locus_wire(graph, wire, claim, &at.locus).ok_or_else(unspelled)?;
            Ok(found.to_wire(model, &|_| locus.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use fsl_replay::{ExpansionGap, TraceFault, TraceRefusal};
    use fsl_semantics::check::{CheckedFunctionRef, HybridModelIndex};
    use ix_trace_rs::trace;

    use super::*;

    fn failed(report: EngineReport) -> (CatalogCode, Map<String, Value>) {
        match report {
            EngineReport::Refused(Refusal::FailedWithDetail(code, detail)) => (code, detail),
            _ => panic!("expected a failed refusal with detail"),
        }
    }

    /// FR-750-AC-12 (analyze path): a missing claim model, an unexpandable
    /// call and an ill-kinded term settle `failed`, `runtime_invariant`/
    /// `established-invariant-broken` naming `hybrid-claim-model`,
    /// `hybrid-call-expansion` and `hybrid-term-kind`; a trace that breaks
    /// FSpec FR-193 is `invalid_result`/`invalid-value` at its pointer; an
    /// exhausted meter is `incomplete`, `LimitReached` on `work_units`.
    #[trace("TC-3037", "FR-750-AC-12")]
    #[test]
    fn each_replay_refusal_settles_by_its_row() {
        let broken = CatalogCode::new("runtime_invariant", "established-invariant-broken");
        for (refusal, invariant) in [
            (
                HybridReplayRefusal::UnknownModel(HybridModelIndex::new(3)),
                "hybrid-claim-model",
            ),
            (
                HybridReplayRefusal::Expansion(ExpansionGap::UnreachableFunction(
                    CheckedFunctionRef::new(7),
                )),
                "hybrid-call-expansion",
            ),
            (
                HybridReplayRefusal::Exact(ExactGap::IllKindedTerm),
                "hybrid-term-kind",
            ),
        ] {
            let (code, detail) = failed(fault_of(&refusal, 10));
            assert_eq!(code, broken, "{refusal:?}");
            assert_eq!(detail["invariant"], invariant, "{refusal:?}");
        }
        let (code, detail) = failed(fault_of(
            &HybridReplayRefusal::Trace(TraceRefusal {
                pointer: "/loop".parse().expect("a pointer"),
                fault: TraceFault::LoopNotBeforeLast,
            }),
            10,
        ));
        assert_eq!(code, CatalogCode::new("invalid_result", "invalid-value"));
        assert_eq!(detail["pointer"], "/loop");
        let exhausted = fault_of(
            &HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits,
            },
            10,
        );
        assert!(
            matches!(
                exhausted,
                EngineReport::Refused(Refusal::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: LimitKind::WorkUnits
                    },
                    value: 10,
                }))
            ),
            "work_units is the limit"
        );
    }
}
