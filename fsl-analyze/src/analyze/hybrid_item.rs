// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the item of a checked hybrid claim, run on the in-process hybrid
//! provider its negotiated candidate names and settled by the arm of the
//! provider's verdict: a solver result by FR-250's map, a claimed refutation
//! by FR-450's exact replay, and an unsupported construct as a located
//! `unsupported` record. A provider's certificate is read against the entry's
//! form (a refusal is `invalid_result`), checked by FR-751 only where the
//! settlement rests on it, and written to the outcome with its check's result.

use filament_exact::LimitKind;
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::{CheckedPackage, EmitLimits};
use fsl_replay::{
    hybrid_locus_wire, Certification, ExactGap, HybridCounterexample, HybridCounterexampleKind, HybridReplayMeter,
    HybridReplayRefusal, HybridSettlement, HybridSolveRequest, HybridVerdict, IncompleteCause,
    ModelCheckLimit, TerminalValue, UnsupportedConstruct,
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
    pub(crate) certificate: Option<fsl_replay::CertificateRecord>,
    /// The FR-286 spelling of the provider's complete counterexample when
    /// replay disagreed with it (FR-450 "Settlement").
    pub(crate) disagreement: Option<Value>,
}

/// The package and request context a hybrid item reads to spell a locus.
#[derive(Clone, Copy)]
pub(super) struct Spelling<'a> {
    pub(super) packages: &'a PackageInput,
    pub(super) emit: EmitLimits,
    /// The run's one strict read of its package, when it holds it.
    pub(super) shared: Option<crate::provider::Shared<'a>>,
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
    provider_limits: crate::provider::ProviderLimits,
    manifest: Vec<Value>,
    candidates: Vec<fsl_foundation::BackendId>,
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
                // The package is emitted once for the item. When FSL cannot
                // emit it, no locus can be spelled or read: an FSL fault, not
                // the provider's.
                let Some(emitted) = emitted_bytes(package, spelling) else {
                    return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    )));
                };
                let Ok(wire) = WireGraph::of_package_bytes(&emitted) else {
                    return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    )));
                };
                let answer = if let Some(provider) = engine.hybrid_provider() {
                    // FSpec FR-779: an option value outside its domain
                    // settles before any solver step.
                    if let Err(refusal) = provider.check_options(&form.options) {
                        return EngineReport::Refused(Refusal::InvalidOptionValue {
                            option: refusal.option,
                            value: refusal.value,
                        });
                    }
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
                    provider.solve(&request, cancel)
                } else if let Some(exchange) = engine.hybrid_exchange() {
                    let Some(dependencies) =
                        super::request::dependency_packages(package, spelling.emit)
                    else {
                        return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        )));
                    };
                    let call = super::hybrid_process::ProcessCall {
                        exchange,
                        backend: engine.descriptor().id(),
                        candidates: &candidates,
                        manifest: &manifest,
                        dependencies: &dependencies,
                        package: &emitted,
                        claim,
                        graph,
                        checked,
                        wire: &wire,
                        budget: form.limits,
                        options: &form.options,
                        deadline_ms: provider_limits.deadline_ms,
                    };
                    match super::hybrid_process::answer(&call, cancel) {
                        Ok(answer) => answer,
                        Err(report) => return report,
                    }
                } else {
                    return EngineReport::Refused(Refusal::Unsupported(CatalogCode::new(
                        "unsupported_projection",
                        "tool-unavailable",
                    )));
                };
                let trusted = engine.hybrid_exchange().is_some();
                let certificate = match answer.read_certificate(graph, checked) {
                    Ok(certificate) => certificate,
                    Err(refusal) => {
                        let mut detail = Map::new();
                        detail.insert("pointer".to_owned(), refusal.pointer.into());
                        return EngineReport::Refused(Refusal::FailedWithDetail(
                            CatalogCode::new("invalid_result", refusal.code.as_str()),
                            detail,
                        ));
                    }
                };
                let mut meter = HybridReplayMeter::new(work_units);
                let settled = match answer.verdict {
                    HybridVerdict::Solved(result) => {
                        result.settle_certified(graph, checked, certificate.as_ref(), &mut meter)
                    }
                    HybridVerdict::Refuted(claimed) => HybridSettlement::from_provider_certified(
                        graph,
                        checked,
                        *claimed,
                        certificate.as_ref(),
                        &mut meter,
                    ),
                    HybridVerdict::Unsupported(construct) => {
                        return unsupported_construct(graph, &wire, position, model, &construct);
                    }
                };
                match settled {
                    Ok(settled) => {
                        let disagreement = match &settled.value {
                            TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::ReplayParity(
                                fsl_replay::DisagreementCause::HybridTrace(found),
                            )) => counterexample_wire(graph, &wire, position, model, found.counterexample())
                                .ok(),
                            _ => None,
                        };
                        let wire = match &settled.counterexample {
                            Some(found) => {
                                counterexample_wire(graph, &wire, position, model, found)
                            }
                            None => Ok(Value::Null),
                        };
                        EngineReport::Hybrid(Box::new(HybridReport {
                            disagreement,
                            value: if trusted { plugin_trust(settled.value) } else { settled.value },
                            counterexample: settled.counterexample.map(Box::new),
                            certificate: settled.certificate,
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

/// The v2 bytes of the package emitted for this request, whose node ids spell
/// a `Locus`.
fn emitted_bytes(package: &CheckedPackage, spelling: Spelling<'_>) -> Option<Vec<u8>> {
    match spelling.shared {
        Some(shared) => Some(shared.emitted.bytes().to_vec()),
        None => {
            let bytes =
                crate::provider::admitted_bytes(package, spelling.packages, spelling.emit).ok()?;
            Some(bytes.bytes().to_vec())
        }
    }
}

/// A process provider's `proved` word without an accepted certificate is
/// `trusted`, not the first-party `uncertified` (FR-290, FSpec FR-331
/// "Certification"); a `certified` proof stays so.
fn plugin_trust(value: TerminalValue) -> TerminalValue {
    match value {
        TerminalValue::Proved {
            basis,
            certification: Certification::Uncertified,
        } => TerminalValue::Proved {
            basis,
            certification: Certification::Trusted,
        },
        other => other,
    }
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
