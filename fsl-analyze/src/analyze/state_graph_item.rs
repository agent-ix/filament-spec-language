// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over a state-graph claim (`possible`, `always possible`,
//! `unique path`; FR-628, FSpec FR-390): built from bytes as
//! [`super::ClauseSubject`] builds an authored clause's, so the checked
//! package, the request's subject binding and the item's budgets are
//! everything it needs.
//!
//! The run decides every instance of the claim (FR-125 Instancing) and settles
//! each one (FR-169), replaying its evidence through `replay_model_graph`
//! (FR-170). A request item carries one terminal record, so the instance that
//! decides the item is the first refuted one, else the first that is not
//! proved, else the first.

use filament_exact::{Cancel, ObjectReference, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::model_subject::{ModelInstance, ModelSubject};
use fsl_eval::simulation::model::PackageModelInput;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PendingClauseForm,
    TimedState, TimedSystem,
};
use fsl_foundation::RequestIndex;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::{CheckedPackage, ObligationIdentity};
use fsl_replay::{
    CertificateChecker, CertificateRule, CertifiedItem, GraphEvidence, GraphEvidenceEnvelope,
    GraphReplayResult, GraphReplaySettlement, ModelTraceSubject, ProfileSelection, ReplayRefusal,
    ReplayRequestWire, ReplayResult, ReplaySource, RequestTargetWire, StateGraphCertificateForm,
    StateGraphInstanceOutcome, StateGraphOutcome, StateGraphPropertyForm, StateGraphRecord,
    TerminalValue, TrapClosure, WitnessPath, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use fsl_route::StateGraphFormKind;
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_route::reductions::ObligationReductions;
use fsl_semantics::check::{Capability, CheckedStateGraphClaim, DeclarationFamily, StateGraphForm};

use super::items::{INVALID_VALUE, model_refusal};
use super::timed_item::subject_refusal;
use super::{
    AnalyzeItem, AnalyzeLimits, EngineReport, GraphReport, Refusal, SourceProvision,
    SubjectBinding, TemporalItem,
};
use crate::model_check::manifest as en1;
use crate::model_check::{
    StateGraphItem, StateGraphPipelineRefusal, StateGraphRequest, check_and_settle_state_graph,
};
use crate::zone_check::{
    TimedStateGraphDecision, ZoneCheckLimit, ZoneCheckRefusal, ZoneStop, certificate_item,
    decide_timed_state_graph, explore_timed_state_graph,
};

/// The property form of a checked claim.
fn property_form(form: &StateGraphForm) -> StateGraphPropertyForm {
    match StateGraphFormKind::of(form) {
        StateGraphFormKind::Possible => StateGraphPropertyForm::Possible,
        StateGraphFormKind::AlwaysPossible => StateGraphPropertyForm::AlwaysPossible,
        StateGraphFormKind::UniquePath => StateGraphPropertyForm::UniquePath,
    }
}

/// The EN-1 item that decides the state-graph claim at `key` over the
/// binding, with the item's budgets, or `None` when the package holds no
/// such claim.
#[allow(
    clippy::too_many_arguments,
    reason = "the item binds source provision, subject, request identity, method, and independent budgets"
)]
pub(super) fn state_graph_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
    method: Option<&str>,
) -> Option<AnalyzeItem<'a>> {
    let (package, input) = (analysis.package, &analysis.model);
    let claim = package
        .graph()
        .state_graph_claims()
        .iter()
        .find(|claim| claim.occurrence() == Some(key))?;
    let (binding, form, input) = (binding.clone(), form.clone(), input.clone());
    let family =
        crate::claim_family::claim_family(package, fsl_route::request::ItemKind::Formula, key);
    let backend = if super::document::timed(package)
        && method != Some(crate::model_check::manifest::METHOD_DIGITAL_CLOCKS)
    {
        crate::zone_check::manifest::BACKEND_ID
    } else {
        en1::BACKEND_ID
    };
    let digital = super::document::timed(package)
        && method == Some(crate::model_check::manifest::METHOD_DIGITAL_CLOCKS);
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            fsl_route::Mode::Bounded,
            backend,
            move |cancel| {
                run(
                    &Run {
                        source: Source::Checked { package, provision },
                        input: &input,
                        binding: &binding,
                        limits,
                        claim,
                        form: &form,
                        timed: backend == crate::zone_check::manifest::BACKEND_ID,
                        digital,
                    },
                    cancel,
                )
            },
        )
        .claimed(key.clone()),
    )
}

/// What one state-graph run reads.
#[derive(Clone, Copy)]
struct Run<'r, 'a> {
    source: Source<'a>,
    input: &'r ModelCheckInput<'a>,
    binding: &'r SubjectBinding<'a>,
    limits: AnalyzeLimits,
    claim: &'r CheckedStateGraphClaim,
    form: &'r TemporalItem,
    timed: bool,
    digital: bool,
}

impl<'a> Run<'_, 'a> {
    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            input: self.input.clone(),
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: self.form.max_candidates,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }
}

fn run(read: &Run<'_, '_>, cancel: &Cancel) -> EngineReport {
    if read.digital {
        return run_digital(read, cancel);
    }
    if read.timed {
        return run_timed(read, cancel);
    }
    let Run { claim, form, .. } = *read;
    let Ok(system) = ModelSystem::new(read.simulation()) else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let mut model_limits = form.limits;
    if let Some(samples) = form.witness_samples {
        model_limits.witness_samples = samples;
    }
    let request = StateGraphRequest {
        subject: ModelSubject {
            simulation: read.simulation(),
            proof_bounds: form.proof_bounds.clone(),
        },
        temporal_clock_selections: Vec::new(),
        items: vec![StateGraphItem::new(claim)],
        reductions: form.reductions.clone(),
        deadlock_freedom: false,
        max_depth: form.max_depth,
        limits: model_limits,
        seed: form.seed,
    };
    let replay = |instance: &StateGraphInstanceOutcome| {
        let evidence = evidence_of(&system, claim, instance)
            .ok_or_else(|| ReplayRefusal::Fault(InternalFault::new("analyze", "graph-evidence")))?;
        replay_instance(read, evidence, None, false).map_err(|refusal| *refusal)
    };
    let settled = match check_and_settle_state_graph(request, cancel, replay) {
        Ok(settled) => settled,
        Err(StateGraphPipelineRefusal::Engine(refusal)) => {
            return EngineReport::Refused(model_refusal(&refusal));
        }
        Err(StateGraphPipelineRefusal::HorizonOutOfRange) => {
            return EngineReport::Refused(Refusal::Declined(
                fsl_replay::ProofRefusalCause::InvalidInput,
                fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                INVALID_VALUE,
            ));
        }
    };
    let (Some(records), Some(instances)) = (settled.records.first(), settled.run.items.first())
    else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let chosen = decisive(records);
    let (Some(record), Some(instance)) = (records.get(chosen), instances.get(chosen)) else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let property = property_form(claim.form());
    let parameter = claim.over().map(|over| over.name.as_str());
    let spelled = evidence_of(&system, claim, instance).map(|evidence| {
        fsl_replay::graph_evidence_wire(
            &evidence,
            property,
            parameter,
            instance.over.as_ref(),
            &instance.sampled,
            claim.occurrence(),
        )
    });
    let refuted = matches!(record.value, TerminalValue::Refuted);
    // A refuted record and a proof by witnesses (FR-628 Outputs) each carry
    // their evidence; one that cannot be spelled is a fault of the engine, never
    // a record without it.
    let witnessed = matches!(record.value, TerminalValue::Proved { .. })
        && matches!(
            instance.outcome,
            StateGraphOutcome::Witnessed { .. }
                | StateGraphOutcome::HoldsReduced {
                    witnesses: Some(_),
                    ..
                }
        );
    let wire = spelled.and_then(Result::ok);
    if (refuted || witnessed) && wire.is_none() {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    }
    let (counterexample, evidence) = if refuted {
        (wire, None)
    } else {
        (None, wire.filter(|_| witnessed))
    };
    let graph = &settled.run.graph;
    let exploration = fsl_replay::RunStats {
        product_states: graph.stats.nodes,
        product_edges: graph.stats.edges,
        depth: graph.stats.depth,
        ..fsl_replay::RunStats::default()
    };
    EngineReport::StateGraph(Box::new(GraphReport {
        record: record.clone(),
        exploration,
        counterexample,
        evidence,
        certificate_document: None,
        checker: None,
    }))
}

fn run_digital(read: &Run<'_, '_>, cancel: &Cancel) -> EngineReport {
    let Some(model) = read
        .input
        .time_sources
        .first()
        .map(|source| source.model.clone())
    else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let system = match ModelSystem::new(read.simulation()) {
        Ok(system) => system,
        Err(failure) => return EngineReport::Refused(super::items::admission(&failure)),
    };
    let max_clocks = read
        .form
        .max_clocks
        .unwrap_or(fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS);
    let timed = match TimedSystem::with_max_clocks(&system, &model, max_clocks) {
        Ok(timed) => timed,
        Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
    };
    if let Some(refusal) = digital_closed_refusal(&timed) {
        return EngineReport::Refused(refusal);
    }
    let instances = match read.claim.over() {
        Some(parameter) => {
            match system.instances(&ValueType::Reference(parameter.object.clone())) {
                Ok(instances) => instances.into_iter().map(Some).collect::<Vec<_>>(),
                Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
            }
        }
        None => vec![None],
    };
    // FR-125 Instancing: no initial state settles before no instance.
    if timed.initial().is_empty() {
        return no_timed_graph_decision(read, fsl_replay::InconclusiveCause::NoInitialState);
    }
    if instances.is_empty() {
        return no_timed_graph_decision(read, fsl_replay::InconclusiveCause::NoInstance);
    }
    let graph = match fsl_replay::explore_digital_state_graph(
        &timed,
        u64::try_from(read.form.limits.limits.max_states).unwrap_or(u64::MAX),
        u64::try_from(read.form.limits.limits.max_transitions).unwrap_or(u64::MAX),
        cancel,
    ) {
        Ok(graph) => graph,
        Err(fsl_replay::DigitalGraphLimit::States(value)) => {
            return EngineReport::Refused(Refusal::Incomplete(
                fsl_replay::IncompleteCause::LimitReached {
                    limit: fsl_replay::ModelCheckLimit::MaxStates,
                    value,
                    required: None,
                },
            ));
        }
        Err(fsl_replay::DigitalGraphLimit::Transitions(value)) => {
            return EngineReport::Refused(Refusal::Incomplete(
                fsl_replay::IncompleteCause::LimitReached {
                    limit: fsl_replay::ModelCheckLimit::MaxTransitions,
                    value,
                    required: None,
                },
            ));
        }
        Err(fsl_replay::DigitalGraphLimit::Clocks(value)) => {
            return EngineReport::Refused(Refusal::Incomplete(
                fsl_replay::IncompleteCause::LimitReached {
                    limit: fsl_replay::ModelCheckLimit::MaxClocks,
                    value,
                    required: None,
                },
            ));
        }
        Err(fsl_replay::DigitalGraphLimit::Cancelled) => match cancel.cause() {
            Some(source) => {
                return EngineReport::Refused(Refusal::Incomplete(
                    fsl_replay::IncompleteCause::Cancelled { source },
                ));
            }
            None => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        },
    };
    let occurrence = match read.claim.occurrence() {
        Some(occurrence) => occurrence.clone(),
        None => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
    };
    let mut selected = None;
    for instance in instances {
        let over = match (read.claim.over(), instance.as_ref()) {
            (None, None) => None,
            (Some(parameter), Some(instance)) => {
                match system.written_reference(parameter.object.clone(), instance) {
                    Ok(reference) => Some(reference),
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                }
            }
            _ => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        let (source, target) = match read.claim.form() {
            StateGraphForm::Possible { target } => (None, target),
            StateGraphForm::AlwaysPossible { from, target } => (Some(from), target),
            StateGraphForm::UniquePath { from, to } => (Some(from), to),
        };
        let target_values =
            match digital_predicate_values(&system, &graph.nodes, target, over.as_ref(), cancel) {
                Ok(values) => values,
                Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
            };
        let source_values = match source {
            Some(source) => {
                match digital_predicate_values(&system, &graph.nodes, source, over.as_ref(), cancel)
                {
                    Ok(values) => Some(values),
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                }
            }
            None => None,
        };
        let decision = match read.claim.form() {
            StateGraphForm::Possible { .. } => digital_possible_paths(&graph, &target_values),
            StateGraphForm::AlwaysPossible { .. } => digital_always_possible_failure(
                &graph,
                source_values.as_deref().unwrap_or(&[]),
                &target_values,
            ),
            StateGraphForm::UniquePath { .. } => digital_unique_path_failure(
                &graph,
                source_values.as_deref().unwrap_or(&[]),
                &target_values,
            ),
        };
        let held = matches!(decision, DigitalDecision::Holds(_));
        let evidence = match decision {
            DigitalDecision::Holds(witnesses)
                if matches!(read.claim.form(), StateGraphForm::Possible { .. }) =>
            {
                let paths = witnesses
                    .into_iter()
                    .map(|(initial, edges)| {
                        digital_model_path(&timed, &graph, initial, &edges).map(|path| {
                            (WitnessPath::Path(path), fsl_replay::WitnessSource::Explored)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>();
                match paths {
                    Ok(paths) => Some(GraphEvidence::Witness {
                        paths,
                        over: instance.clone(),
                    }),
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                }
            }
            DigitalDecision::Holds(_) => None,
            DigitalDecision::Trap { initial, stem } => {
                let stem = match digital_model_path(&timed, &graph, initial, &stem) {
                    Ok(stem) => stem,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                };
                Some(GraphEvidence::Trap {
                    stem,
                    closure: TrapClosure::StateGraph,
                    over: instance.clone(),
                })
            }
            DigitalDecision::PathPair {
                initial,
                stem,
                first,
                second,
            } => {
                let stem = match digital_model_path(&timed, &graph, initial, &stem) {
                    Ok(stem) => stem,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                };
                let first = match digital_model_steps(&timed, &graph, &first) {
                    Ok(steps) => steps,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                };
                let second = match digital_model_steps(&timed, &graph, &second) {
                    Ok(steps) => steps,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                };
                Some(GraphEvidence::PathPair {
                    stem,
                    first,
                    second,
                    over: instance.clone(),
                })
            }
        };
        if let Some(evidence) = evidence.as_ref() {
            match replay_instance(read, evidence.clone(), None, true) {
                Ok(replay) if replay.settlement == GraphReplaySettlement::Reproduced => {}
                Err(refusal) => {
                    let allocation = super::document::effect_decision::ReplayAllocation::default();
                    let _: Result<(), _> = allocation.capture(Err(*refusal));
                    return allocation.report().unwrap_or_else(|| {
                        EngineReport::Refused(Refusal::Failed(super::items::BROKEN))
                    });
                }
                _ => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
            }
        }
        let wire = evidence
            .as_ref()
            .map(|evidence| {
                fsl_replay::graph_evidence_wire(
                    evidence,
                    property_form(read.claim.form()),
                    read.claim.over().map(|over| over.name.as_str()),
                    over.as_ref(),
                    &[],
                    Some(&occurrence),
                )
            })
            .transpose();
        let wire = match wire {
            Ok(wire) => wire,
            Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        let record = StateGraphRecord {
            value: if held {
                TerminalValue::Proved {
                    basis: fsl_replay::ProofBasis::Exhaustive,
                    certification: fsl_replay::Certification::Uncertified,
                }
            } else {
                TerminalValue::Refuted
            },
            label: if held {
                fsl_replay::ResultLabel::Proved
            } else {
                fsl_replay::ResultLabel::Refuted
            },
            basis: if held {
                fsl_replay::SettlementBasis::ClosedScope
            } else {
                fsl_replay::SettlementBasis::DecisiveCounterexample
            },
            execution: fsl_replay::Execution::Completed,
            truth: if held {
                fsl_replay::Truth::Satisfied
            } else {
                fsl_replay::Truth::Violated
            },
            method: fsl_replay::EXPLICIT_STATE,
            max_depth: read
                .form
                .max_depth
                .and_then(|depth| u64::try_from(depth).ok()),
            limits: read.form.limits.used(),
            limit_reached: None,
            seed: read.form.seed.unwrap_or_default(),
            witness_samples: read.form.witness_samples.unwrap_or(1),
            walks_stopped: 0,
            end: None,
            undefined: None,
        };
        let report = GraphReport {
            record,
            exploration: fsl_replay::RunStats {
                product_states: u64::try_from(graph.nodes.len()).unwrap_or(u64::MAX),
                product_edges: u64::try_from(graph.edges.len()).unwrap_or(u64::MAX),
                ..fsl_replay::RunStats::default()
            },
            counterexample: (!held).then(|| wire.clone()).flatten(),
            evidence: held.then(|| wire.clone()).flatten(),
            certificate_document: None,
            checker: None,
        };
        if selected.is_none() || !held {
            selected = Some(report);
        }
        if !held {
            break;
        }
    }
    selected.map_or_else(
        || EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        |report| EngineReport::StateGraph(Box::new(report)),
    )
}

/// EN-6's timed state-graph route. The zone certificate is checked by the
/// qualified core before this result is reported; a refutation is accepted
/// only when that checker re-derives the form's failed condition.
#[allow(dead_code)]
fn digital_predicate_values(
    system: &ModelSystem<'_>,
    nodes: &[TimedState],
    predicate: &fsl_semantics::check::CheckedStatePredicate,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
) -> Result<Vec<bool>, ()> {
    if predicate.is_constant_true() {
        return Ok(vec![true; nodes.len()]);
    }
    let body = predicate.predicate_body().ok_or(())?;
    nodes
        .iter()
        .map(|node| {
            let readings = system
                .state_predicates_with_cancel(node.discrete(), &[(body, over)], cancel)
                .map_err(|_| ())?;
            match readings.as_slice() {
                [fsl_eval::simulation::model::PredicateReading::True] => Ok(true),
                [fsl_eval::simulation::model::PredicateReading::False] => Ok(false),
                _ => Err(()),
            }
        })
        .collect()
}

fn digital_path_to(
    graph: &fsl_replay::DigitalStateGraph,
    initial: usize,
    target: &[bool],
) -> Option<Vec<usize>> {
    let mut parent = vec![None; graph.nodes.len()];
    let mut seen = vec![false; graph.nodes.len()];
    *seen.get_mut(initial)? = true;
    if target.get(initial).copied().unwrap_or(false) {
        return Some(Vec::new());
    }
    let mut queue = std::collections::VecDeque::from([initial]);
    while let Some(node) = queue.pop_front() {
        for (edge_index, edge) in graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node)
        {
            if seen.get(edge.to).copied().unwrap_or(true) {
                continue;
            }
            seen[edge.to] = true;
            parent[edge.to] = Some((node, edge_index));
            if target.get(edge.to).copied().unwrap_or(false) {
                let mut path = Vec::new();
                let mut at = edge.to;
                while at != initial {
                    let (previous, edge) = parent.get(at).copied().flatten()?;
                    path.push(edge);
                    at = previous;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(edge.to);
        }
    }
    None
}

enum DigitalDecision {
    Holds(Vec<(usize, Vec<usize>)>),
    Trap {
        initial: usize,
        stem: Vec<usize>,
    },
    PathPair {
        initial: usize,
        stem: Vec<usize>,
        first: Vec<usize>,
        second: Vec<usize>,
    },
}

fn digital_possible_paths(
    graph: &fsl_replay::DigitalStateGraph,
    target: &[bool],
) -> DigitalDecision {
    let mut paths = Vec::new();
    for (initial, &node) in graph.initials.iter().enumerate() {
        let Some(path) = digital_path_to(graph, node, target) else {
            return DigitalDecision::Trap {
                initial,
                stem: Vec::new(),
            };
        };
        paths.push((initial, path));
    }
    DigitalDecision::Holds(paths)
}

fn digital_always_possible_failure(
    graph: &fsl_replay::DigitalStateGraph,
    source: &[bool],
    target: &[bool],
) -> DigitalDecision {
    let reaches_target = digital_predecessors_of(graph, target);
    for (node, (&is_source, &can_reach)) in source.iter().zip(&reaches_target).enumerate() {
        if is_source && !can_reach {
            return digital_stem_to(graph, node)
                .map_or(DigitalDecision::Holds(Vec::new()), |(initial, stem)| {
                    DigitalDecision::Trap { initial, stem }
                });
        }
    }
    DigitalDecision::Holds(Vec::new())
}

fn digital_unique_path_failure(
    graph: &fsl_replay::DigitalStateGraph,
    source: &[bool],
    target: &[bool],
) -> DigitalDecision {
    for node in source
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(node, yes)| yes.then_some(node))
    {
        let dfa = digital_determinize(graph, node, target);
        let words = digital_first_accepted_words(&dfa);
        if words.len() != 1 {
            let Some((initial, stem)) = digital_stem_to(graph, node) else {
                return DigitalDecision::Holds(Vec::new());
            };
            return match words.as_slice() {
                [first, second, ..] => match (
                    digital_path_for_word(graph, node, first, target),
                    digital_path_for_word(graph, node, second, target),
                ) {
                    (Some(first), Some(second)) => DigitalDecision::PathPair {
                        initial,
                        stem,
                        first,
                        second,
                    },
                    _ => DigitalDecision::Holds(Vec::new()),
                },
                _ => DigitalDecision::Trap { initial, stem },
            };
        }
    }
    DigitalDecision::Holds(Vec::new())
}

struct DigitalDfa {
    transitions: Vec<
        Vec<(
            filament_semantic_value::model_transition::ModelTransition,
            usize,
        )>,
    >,
    accepting: Vec<bool>,
}

fn digital_determinize(
    graph: &fsl_replay::DigitalStateGraph,
    start: usize,
    target: &[bool],
) -> DigitalDfa {
    let mut subsets = vec![digital_epsilon_closure(graph, vec![start])];
    let mut transitions = vec![Vec::new()];
    let mut accepting = vec![false];
    let mut at = 0;
    while at < subsets.len() {
        let subset = subsets[at].clone();
        let mut symbols: Vec<(
            filament_semantic_value::model_transition::ModelTransition,
            Vec<usize>,
        )> = Vec::new();
        for node in subset {
            accepting[at] |= target.get(node).copied().unwrap_or(false);
            for edge in graph.edges.iter().filter(|edge| edge.from == node) {
                let Some(symbol) = &edge.transition else {
                    continue;
                };
                if let Some((_, destinations)) = symbols.iter_mut().find(|(old, _)| old == symbol) {
                    destinations.push(edge.to);
                } else {
                    symbols.push((symbol.clone(), vec![edge.to]));
                }
            }
        }
        for (symbol, destinations) in symbols {
            let next = digital_epsilon_closure(graph, destinations);
            let target_state = subsets
                .iter()
                .position(|old| *old == next)
                .unwrap_or_else(|| {
                    let index = subsets.len();
                    subsets.push(next);
                    transitions.push(Vec::new());
                    accepting.push(false);
                    index
                });
            transitions[at].push((symbol, target_state));
        }
        at += 1;
    }
    DigitalDfa {
        transitions,
        accepting,
    }
}

fn digital_epsilon_closure(
    graph: &fsl_replay::DigitalStateGraph,
    starts: Vec<usize>,
) -> Vec<usize> {
    let mut seen = vec![false; graph.nodes.len()];
    let mut queue = std::collections::VecDeque::new();
    for start in starts {
        if let Some(mark) = seen.get_mut(start) {
            if !*mark {
                *mark = true;
                queue.push_back(start);
            }
        }
    }
    while let Some(node) = queue.pop_front() {
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.from == node && edge.transition.is_none())
        {
            if let Some(mark) = seen.get_mut(edge.to) {
                if !*mark {
                    *mark = true;
                    queue.push_back(edge.to);
                }
            }
        }
    }
    seen.iter()
        .enumerate()
        .filter_map(|(node, yes)| yes.then_some(node))
        .collect()
}

fn digital_first_accepted_words(
    dfa: &DigitalDfa,
) -> Vec<Vec<filament_semantic_value::model_transition::ModelTransition>> {
    let mut can_accept = dfa.accepting.clone();
    loop {
        let mut changed = false;
        for (from, outgoing) in dfa.transitions.iter().enumerate() {
            if !can_accept[from] && outgoing.iter().any(|(_, to)| can_accept[*to]) {
                can_accept[from] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let max_len = dfa.transitions.len().saturating_mul(2).saturating_add(2);
    let mut queue = std::collections::VecDeque::from([(0_usize, Vec::new())]);
    let mut words = Vec::new();
    while let Some((state, word)) = queue.pop_front() {
        if dfa.accepting[state] {
            words.push(word.clone());
            if words.len() == 2 {
                break;
            }
        }
        if word.len() >= max_len {
            continue;
        }
        for (symbol, next_state) in &dfa.transitions[state] {
            if can_accept[*next_state] {
                let mut next = word.clone();
                next.push(symbol.clone());
                queue.push_back((*next_state, next));
            }
        }
    }
    words
}

fn digital_path_for_word(
    graph: &fsl_replay::DigitalStateGraph,
    start: usize,
    word: &[filament_semantic_value::model_transition::ModelTransition],
    target: &[bool],
) -> Option<Vec<usize>> {
    let mut seen = vec![vec![false; word.len() + 1]; graph.nodes.len()];
    seen.get_mut(start)?.get_mut(0).map(|value| *value = true)?;
    let mut paths = std::collections::VecDeque::from([(start, 0_usize, Vec::new())]);
    while let Some((node, at, path)) = paths.pop_front() {
        if at == word.len() && target.get(node).copied().unwrap_or(false) {
            return Some(path);
        }
        for (edge_index, edge) in graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node)
        {
            let next_at = match &edge.transition {
                None => at,
                Some(symbol) if word.get(at) == Some(symbol) => at + 1,
                Some(_) => continue,
            };
            if seen.get(edge.to)?.get(next_at).copied()? {
                continue;
            }
            seen[edge.to][next_at] = true;
            let mut next_path = path.clone();
            next_path.push(edge_index);
            paths.push_back((edge.to, next_at, next_path));
        }
    }
    None
}

fn digital_predecessors_of(graph: &fsl_replay::DigitalStateGraph, target: &[bool]) -> Vec<bool> {
    let mut reaches = target.to_vec();
    let mut queue: std::collections::VecDeque<usize> = reaches
        .iter()
        .enumerate()
        .filter_map(|(index, yes)| yes.then_some(index))
        .collect();
    while let Some(node) = queue.pop_front() {
        for edge in graph.edges.iter().filter(|edge| edge.to == node) {
            if let Some(mark) = reaches.get_mut(edge.from) {
                if !*mark {
                    *mark = true;
                    queue.push_back(edge.from);
                }
            }
        }
    }
    reaches
}

fn digital_stem_to(
    graph: &fsl_replay::DigitalStateGraph,
    target: usize,
) -> Option<(usize, Vec<usize>)> {
    for (initial, &start) in graph.initials.iter().enumerate() {
        let mut parent = vec![None; graph.nodes.len()];
        let mut seen = vec![false; graph.nodes.len()];
        *seen.get_mut(start)? = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(node) = queue.pop_front() {
            if node == target {
                let mut path = Vec::new();
                let mut at = target;
                while at != start {
                    let (previous, edge) = parent.get(at).copied().flatten()?;
                    path.push(edge);
                    at = previous;
                }
                path.reverse();
                return Some((initial, path));
            }
            for (edge_index, edge) in graph
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.from == node)
            {
                if !seen.get(edge.to).copied().unwrap_or(true) {
                    seen[edge.to] = true;
                    parent[edge.to] = Some((node, edge_index));
                    queue.push_back(edge.to);
                }
            }
        }
    }
    None
}

fn digital_model_path(
    timed: &TimedSystem<'_, '_>,
    graph: &fsl_replay::DigitalStateGraph,
    initial: usize,
    edges: &[usize],
) -> Result<fsl_replay::ModelPath, ()> {
    Ok(fsl_replay::ModelPath {
        initial: u32::try_from(initial).map_err(|_| ())?,
        steps: digital_model_steps(timed, graph, edges)?,
    })
}

fn digital_model_steps(
    timed: &TimedSystem<'_, '_>,
    graph: &fsl_replay::DigitalStateGraph,
    edges: &[usize],
) -> Result<Vec<fsl_replay::ModelStep>, ()> {
    let mut delay = filament_exact::Rational::from_integer(filament_exact::Integer::zero());
    let mut steps = Vec::new();
    for &index in edges {
        let edge = graph.edges.get(index).ok_or(())?;
        let Some(transition) = edge.transition.as_ref() else {
            delay = delay.add(&graph.unit);
            continue;
        };
        let post_state = timed
            .state_key(graph.nodes.get(edge.to).ok_or(())?)
            .map_err(|_| ())?;
        steps.push(fsl_replay::ModelStep {
            transition: transition.clone(),
            post_state,
            delay: Some(delay.clone()),
        });
        delay = filament_exact::Rational::from_integer(filament_exact::Integer::zero());
    }
    Ok(steps)
}

fn digital_closed_refusal(timed: &TimedSystem<'_, '_>) -> Option<Refusal> {
    let constraint = crate::exact_probabilistic::model_constraints(timed)
        .into_iter()
        .find(|constraint| {
            matches!(
                constraint.op(),
                fsl_semantics::check::ClockOp::Less | fsl_semantics::check::ClockOp::Greater
            )
        })?;
    let span = constraint.span();
    let mut detail = serde_json::Map::new();
    detail.insert(
        "strict_constraint".to_owned(),
        serde_json::json!({"start": span.start, "end": span.end}),
    );
    Some(Refusal::declined_with_detail(
        fsl_replay::ProofRefusalCause::InvalidInput,
        fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
        filament_semantic_value::catalog::CatalogCode::new(
            "invalid_runtime_input",
            "strict-clock-constraint",
        ),
        detail,
    ))
}

fn no_timed_graph_decision(
    read: &Run<'_, '_>,
    cause: fsl_replay::InconclusiveCause,
) -> EngineReport {
    let record = StateGraphRecord {
        value: TerminalValue::Inconclusive(cause),
        label: fsl_replay::ResultLabel::Inconclusive,
        basis: fsl_replay::SettlementBasis::Unsettled,
        execution: fsl_replay::Execution::Completed,
        truth: fsl_replay::Truth::Pending,
        method: fsl_replay::EXPLICIT_STATE,
        max_depth: read
            .form
            .max_depth
            .and_then(|depth| u64::try_from(depth).ok()),
        limits: read.form.limits.used(),
        limit_reached: None,
        seed: read.form.seed.unwrap_or_default(),
        witness_samples: read.form.witness_samples.unwrap_or(1),
        walks_stopped: 0,
        end: None,
        undefined: None,
    };
    EngineReport::StateGraph(Box::new(GraphReport {
        record,
        exploration: fsl_replay::RunStats::default(),
        counterexample: None,
        evidence: None,
        certificate_document: None,
        checker: None,
    }))
}

fn run_timed(read: &Run<'_, '_>, cancel: &Cancel) -> EngineReport {
    let Some(model) = read
        .input
        .time_sources
        .first()
        .map(|source| source.model.clone())
    else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let simulation = read.simulation();
    let system = match ModelSystem::new(simulation.clone()) {
        Ok(system) => system,
        Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
    };
    let max_clocks = read
        .form
        .max_clocks
        .unwrap_or(fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS);
    let timed = match TimedSystem::with_max_clocks(&system, &model, max_clocks) {
        Ok(timed) => timed,
        Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
    };
    let instances = match read.claim.over() {
        Some(parameter) => {
            match system.instances(&ValueType::Reference(parameter.object.clone())) {
                Ok(instances) => instances.into_iter().map(Some).collect::<Vec<_>>(),
                Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
            }
        }
        None => vec![None],
    };
    // FR-125 Instancing: no initial state settles before no instance.
    if timed.initial().is_empty() {
        return no_timed_graph_decision(read, fsl_replay::InconclusiveCause::NoInitialState);
    }
    if instances.is_empty() {
        return no_timed_graph_decision(read, fsl_replay::InconclusiveCause::NoInstance);
    }
    let bounds = super::timed_item::zone_bounds(read.form, read.limits.engines);
    let limits = super::timed_item::zone_limits(&bounds);
    let graph = match explore_timed_state_graph(&timed, limits, cancel) {
        Ok(graph) => graph,
        Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(limit))) => {
            let (limit, value) = match limit {
                // FR-852: the item's own `max_states` keeps its name.
                ZoneCheckLimit::MaxSymbolicStates(value) => {
                    let stated = read
                        .form
                        .requested_limits
                        .contains(&fsl_replay::ModelCheckLimit::MaxStates);
                    let limit = if stated {
                        fsl_replay::ModelCheckLimit::MaxStates
                    } else {
                        fsl_replay::ModelCheckLimit::ZoneMaxSymbolicStates
                    };
                    (limit, value)
                }
                ZoneCheckLimit::MaxSymbolicTransitions(value) => (
                    fsl_replay::ModelCheckLimit::ZoneMaxSymbolicTransitions,
                    value,
                ),
                ZoneCheckLimit::MaxAutomatonStates(value) => {
                    (fsl_replay::ModelCheckLimit::ZoneMaxAutomatonStates, value)
                }
                ZoneCheckLimit::MaxZoneBytes(value) => {
                    (fsl_replay::ModelCheckLimit::ZoneMaxZoneBytes, value)
                }
                ZoneCheckLimit::MaxClocks(value) => (fsl_replay::ModelCheckLimit::MaxClocks, value),
            };
            return EngineReport::Refused(Refusal::Incomplete(
                fsl_replay::IncompleteCause::LimitReached {
                    limit,
                    value,
                    required: None,
                },
            ));
        }
        Err(ZoneCheckRefusal::Stopped(ZoneStop::Cancelled)) => match cancel.cause() {
            Some(source) => {
                return EngineReport::Refused(Refusal::Incomplete(
                    fsl_replay::IncompleteCause::Cancelled { source },
                ));
            }
            None => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        },
        Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
    };
    let occurrence = match read.claim.occurrence() {
        Some(occurrence) => occurrence.clone(),
        None => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
    };
    let mut selected = None;
    for instance in instances {
        let over = match (read.claim.over(), instance.as_ref()) {
            (None, None) => None,
            (Some(parameter), Some(instance)) => {
                match system.written_reference(parameter.object.clone(), instance) {
                    Ok(reference) => Some(reference),
                    Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
                }
            }
            _ => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        let decision =
            match decide_timed_state_graph(&system, &graph, read.claim, over.as_ref(), cancel) {
                Ok(decision) => decision,
                Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
            };
        let held = matches!(decision, TimedStateGraphDecision::Holds { .. });
        let form = certificate_form(read.claim.form());
        let ef_nodes = match &decision {
            TimedStateGraphDecision::Holds { ef_nodes, .. }
            | TimedStateGraphDecision::Refuted { ef_nodes, .. } => ef_nodes,
        };
        let obligation = ItemObligation {
            subject: (),
            kind: ObligationKind::StateGraphAuthored(occurrence.clone()),
            reductions: ObligationReductions::default(),
        };
        let certificate = match crate::zone_check::state_graph_certificate(
            &timed,
            &graph,
            obligation.clone(),
            form,
            ef_nodes,
        ) {
            Ok(certificate) => certificate,
            Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        let check = fsl_replay::check_zone_certificate(
            ZoneCertificateRequest {
                simulation: simulation.clone(),
                model: &model,
                max_clocks,
                item: CertifiedItem::StateGraph {
                    claim: read.claim,
                    over: over.as_ref(),
                },
            },
            &obligation,
            certificate_item,
            &certificate,
            super::timed_item::certificate_limits(&bounds),
            cancel,
        );
        let accepted = matches!(check, Ok(ZoneCertificateVerdict::Accepted));
        let refuted = matches!(
            check,
            Ok(ZoneCertificateVerdict::Rejected(ref refusal))
                if matches!(
                    refusal.rule,
                    CertificateRule::InitialMissing
                        | CertificateRule::SuccessorMissing
                        | CertificateRule::CountBroken
                )
        );
        if (held && !accepted) || (!held && !refuted) {
            return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
        }
        let evidence = match timed_evidence(&timed, &graph, read.claim, &decision, instance.clone())
        {
            Ok(evidence) => evidence,
            Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        if let Some(evidence) = evidence.as_ref() {
            match replay_instance(read, evidence.clone(), Some(&certificate), false) {
                Ok(replay) if replay.settlement == GraphReplaySettlement::Reproduced => {}
                Err(refusal) => {
                    let allocation = super::document::effect_decision::ReplayAllocation::default();
                    let _: Result<(), _> = allocation.capture(Err(*refusal));
                    return allocation.report().unwrap_or_else(|| {
                        EngineReport::Refused(Refusal::Failed(super::items::BROKEN))
                    });
                }
                _ => {
                    return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
                }
            }
        }
        let wire = evidence
            .as_ref()
            .map(|evidence| {
                fsl_replay::graph_evidence_wire(
                    evidence,
                    property_form(read.claim.form()),
                    read.claim.over().map(|over| over.name.as_str()),
                    over.as_ref(),
                    &[],
                    Some(&occurrence),
                )
            })
            .transpose();
        let wire = match wire {
            Ok(wire) => wire,
            Err(_) => return EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        };
        let limit_values = read.form.limits.used();
        let record = StateGraphRecord {
            value: if held {
                TerminalValue::Proved {
                    basis: fsl_replay::ProofBasis::ZoneCertified,
                    certification: fsl_replay::Certification::Certified,
                }
            } else {
                TerminalValue::Refuted
            },
            label: if held {
                fsl_replay::ResultLabel::Proved
            } else {
                fsl_replay::ResultLabel::Refuted
            },
            basis: if held {
                fsl_replay::SettlementBasis::ClosedScope
            } else {
                fsl_replay::SettlementBasis::DecisiveCounterexample
            },
            execution: fsl_replay::Execution::Completed,
            truth: if held {
                fsl_replay::Truth::Satisfied
            } else {
                fsl_replay::Truth::Violated
            },
            method: fsl_replay::EXPLICIT_STATE,
            max_depth: read
                .form
                .max_depth
                .and_then(|depth| u64::try_from(depth).ok()),
            limits: limit_values,
            limit_reached: None,
            seed: read.form.seed.unwrap_or_default(),
            witness_samples: read.form.witness_samples.unwrap_or(1),
            walks_stopped: 0,
            end: None,
            undefined: None,
        };
        let certificate_document = (accepted || refuted).then(|| {
            serde_json::json!({
                "state_graph": {
                    "form": form.as_str(),
                    "ef_nodes": certificate.state_graph.as_ref().map_or_else(Vec::new, |part| part.ef_nodes.clone())
                }
            })
        });
        let report = GraphReport {
            record,
            exploration: fsl_replay::RunStats {
                product_states: graph.stats.symbolic_states,
                product_edges: graph.stats.retained_edges,
                ..fsl_replay::RunStats::default()
            },
            counterexample: (!held).then(|| wire.clone()).flatten(),
            evidence: held.then(|| wire.clone()).flatten(),
            certificate_document,
            checker: (accepted || refuted).then_some(CertificateChecker::Zone),
        };
        if selected.is_none() || !held {
            selected = Some(report);
        }
        if !held {
            break;
        }
    }
    selected.map_or_else(
        || EngineReport::Refused(Refusal::Failed(super::items::BROKEN)),
        |report| EngineReport::StateGraph(Box::new(report)),
    )
}

fn certificate_form(form: &StateGraphForm) -> StateGraphCertificateForm {
    match form {
        StateGraphForm::Possible { .. } => StateGraphCertificateForm::Possible,
        StateGraphForm::AlwaysPossible { .. } => StateGraphCertificateForm::AlwaysPossible,
        StateGraphForm::UniquePath { .. } => StateGraphCertificateForm::UniquePath,
    }
}

fn timed_evidence(
    timed: &TimedSystem<'_, '_>,
    graph: &crate::zone_check::TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    decision: &TimedStateGraphDecision,
    over: Option<ModelInstance>,
) -> Result<Option<GraphEvidence>, crate::zone_check::ZoneCheckRefusal> {
    match decision {
        TimedStateGraphDecision::Holds { witness_paths, .. }
            if matches!(claim.form(), StateGraphForm::Possible { .. }) =>
        {
            let paths = witness_paths
                .iter()
                .enumerate()
                .map(|(initial, edges)| {
                    let steps =
                        crate::zone_check::concretise_timed_state_graph_path(timed, graph, edges)?;
                    let path = fsl_replay::ModelPath {
                        initial: u32::try_from(initial).map_err(|_| {
                            crate::zone_check::ZoneCheckRefusal::Fault("state-graph-initial")
                        })?,
                        steps,
                    };
                    Ok((WitnessPath::Path(path), fsl_replay::WitnessSource::Explored))
                })
                .collect::<Result<Vec<_>, crate::zone_check::ZoneCheckRefusal>>()?;
            Ok(Some(GraphEvidence::Witness { paths, over }))
        }
        TimedStateGraphDecision::Holds { .. } => Ok(None),
        TimedStateGraphDecision::Refuted {
            stem, node, paths, ..
        } => {
            let stem = model_path(timed, graph, stem, *node)?;
            if let Some((first, second)) = paths {
                let first =
                    crate::zone_check::concretise_timed_state_graph_path(timed, graph, first)?;
                let second =
                    crate::zone_check::concretise_timed_state_graph_path(timed, graph, second)?;
                Ok(Some(GraphEvidence::PathPair {
                    stem,
                    first,
                    second,
                    over,
                }))
            } else {
                Ok(Some(GraphEvidence::Trap {
                    stem,
                    closure: TrapClosure::StateGraph,
                    over,
                }))
            }
        }
    }
}

fn model_path(
    timed: &TimedSystem<'_, '_>,
    graph: &crate::zone_check::TimedStateGraph,
    edges: &[usize],
    endpoint: usize,
) -> Result<fsl_replay::ModelPath, crate::zone_check::ZoneCheckRefusal> {
    let initial_node = edges
        .first()
        .and_then(|&edge| graph.edges.get(edge).map(|edge| edge.from))
        .unwrap_or(endpoint);
    let initial = graph
        .initials
        .iter()
        .position(|&node| node == initial_node)
        .ok_or(crate::zone_check::ZoneCheckRefusal::Fault(
            "state-graph-path-initial",
        ))?;
    Ok(fsl_replay::ModelPath {
        initial: u32::try_from(initial)
            .map_err(|_| crate::zone_check::ZoneCheckRefusal::Fault("state-graph-initial"))?,
        steps: crate::zone_check::concretise_timed_state_graph_path(timed, graph, edges)?,
    })
}

/// The index of the instance that decides the item.
fn decisive(records: &[StateGraphRecord]) -> usize {
    records
        .iter()
        .position(|record| matches!(record.value, TerminalValue::Refuted))
        .or_else(|| {
            records
                .iter()
                .position(|record| !matches!(record.value, TerminalValue::Proved { .. }))
        })
        .unwrap_or(0)
}

/// The FR-393 evidence an instance's outcome settles from, bound to the
/// instance it was found for.
fn evidence_of(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    instance: &StateGraphInstanceOutcome,
) -> Option<GraphEvidence> {
    let over = bound_instance(system, claim, instance.over.as_ref())?;
    let witnesses = |paths: &[fsl_replay::GraphWitness]| {
        paths
            .iter()
            .map(|found| (WitnessPath::Path(found.path.clone()), found.source.clone()))
            .collect()
    };
    match &instance.outcome {
        StateGraphOutcome::Witnessed { paths, .. }
        | StateGraphOutcome::WitnessedUnchecked { paths, .. }
        | StateGraphOutcome::HoldsReduced {
            witnesses: Some(paths),
            ..
        } => Some(GraphEvidence::Witness {
            paths: witnesses(paths),
            over: over.clone(),
        }),
        StateGraphOutcome::Trapped { stem } => Some(GraphEvidence::Trap {
            stem: stem.clone(),
            closure: TrapClosure::StateGraph,
            over: over.clone(),
        }),
        StateGraphOutcome::PathPair {
            stem,
            first,
            second,
        } => Some(GraphEvidence::PathPair {
            stem: stem.clone(),
            first: first.clone(),
            second: second.clone(),
            over: over.clone(),
        }),
        StateGraphOutcome::Undefined { stem, undefined } => Some(GraphEvidence::Undefined {
            stem: stem.clone(),
            undefined: undefined.clone(),
            over: over.clone(),
        }),
        _ => None,
    }
}

/// The instance `reference` is the binding of: `Some(None)` for a claim with
/// no `over` parameter, `None` when the reference names no instance.
fn bound_instance(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    reference: Option<&ObjectReference>,
) -> Option<Option<ModelInstance>> {
    let (Some(over), Some(reference)) = (claim.over(), reference) else {
        return Some(None);
    };
    system
        .instances(&ValueType::Reference(over.object.clone()))
        .ok()?
        .into_iter()
        .find(|instance| {
            system
                .written_reference(over.object.clone(), instance)
                .is_ok_and(|bound| &bound == reference)
        })
        .map(Some)
}

/// Where a run's evidence replays from.
#[derive(Clone, Copy)]
enum Source<'a> {
    /// The source route: the checked package, recompiled from its
    /// provision.
    Checked {
        package: &'a CheckedPackage,
        provision: &'a SourceProvision,
    },
    /// The package route (FR-940): the admitted package bytes and the
    /// handoff supply, never a source provision.
    Bytes {
        package_bytes: &'a [u8],
        supply: fsl_replay::HandoffSupply<'a>,
    },
}

/// FR-170's replay of an instance's evidence over the same source.
fn replay_instance(
    read: &Run<'_, '_>,
    evidence: GraphEvidence,
    certificate: Option<&fsl_replay::ZoneCertificate<ItemObligation<()>>>,
    digital: bool,
) -> Result<GraphReplayResult, Box<ReplayRefusal>> {
    let fault =
        |cause: &'static str| Box::new(ReplayRefusal::Fault(InternalFault::new("analyze", cause)));
    let occurrence = read
        .claim
        .occurrence()
        .cloned()
        .ok_or_else(|| fault("graph-claim-occurrence"))?;
    let node = occurrence.node().clone();
    let (bytes, dependencies, byte_provision) = match read.source {
        Source::Checked { package, provision } => {
            let emission =
                fsl_package::emit_checked_with_temporal_selections(package, &[], read.limits.emit)
                    .map_err(|_| fault("package-emission"))?;
            (
                emission.package().bytes().to_vec(),
                super::request::dependency_wires(package),
                provision.wire_entries(),
            )
        }
        Source::Bytes { package_bytes, .. } => (package_bytes.to_vec(), Vec::new(), Vec::new()),
    };
    let package = super::clause_item::read_package(&bytes, &read.limits).map_err(Box::new)?;
    let obligation = ObligationIdentity::new(node.clone(), None, 0);
    let mut envelope = GraphEvidenceEnvelope::new(
        obligation.clone(),
        occurrence,
        node,
        Vec::<ProfileSelection>::new(),
        read.limits.accounting,
        Vec::new(),
        fsl_foundation::BackendId::new(en1::BACKEND_ID),
        evidence,
    );
    if let Some(certificate) = certificate {
        envelope = envelope.with_zone_certificate(certificate.clone());
    }
    let wire = ReplayRequestWire {
        certificate: None,
        package,
        profile_selections: Vec::new(),
        dependencies,
        replay_target: RequestTargetWire::default(),
        source: ReplaySource::ModelGraph(Box::new(envelope)),
        obligation_identity: obligation,
        backend: en1::BACKEND_ID.to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: read.limits.accounting,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents: read.binding.replay_documents()?,
    };
    let timed = (digital || certificate.is_some())
        .then(|| {
            read.input
                .time_sources
                .first()
                .map(|source| source.model.clone())
        })
        .flatten()
        .map(|model| fsl_replay::TimedReplay {
            model,
            exploration: fsl_eval::simulation::Limits::default(),
            max_clocks: read
                .form
                .max_clocks
                .unwrap_or(fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS),
            digital,
        });
    let subject = ModelTraceSubject {
        timed,
        initial: read.binding.initial.clone(),
        universes: read.binding.universes.clone(),
        expansion: ExpansionLimits {
            max_candidates: read.form.max_candidates,
        },
        stutter_closure: false,
    };
    let replayed = match read.source {
        Source::Checked { .. } => {
            fsl_replay::replay_model_graph(wire, &subject, read.limits.replay)
        }
        Source::Bytes { supply, .. } => {
            fsl_replay::replay_model_graph_from_package(wire, supply, &subject, read.limits.replay)
        }
    };
    match replayed.map_err(Box::new)? {
        ReplayResult::ModelGraph(replayed) => Ok(replayed),
        _ => Err(fault("graph-replay-result")),
    }
}

/// FR-850, FR-940: the EN-1 item over a state-graph claim on the package
/// route. The claim is the package form of the occurrence its `claim_names`
/// row names ([`PackageModelInput::state_graph_claim`]), the model system is
/// built from the package route's model-check input, every predicate is read
/// through the evaluator port, and an instance's evidence replays from the
/// package bytes and the handoff supply, never from a source provision.
pub struct PackageStateGraphSubject<'a> {
    input: ModelCheckInput<'a>,
    claim: CheckedStateGraphClaim,
    occurrence: OccurrenceKey,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
}

impl<'a> PackageStateGraphSubject<'a> {
    /// The subject of the state-graph claim at `key`.
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] when the package does not select the claim or
    /// its facts do not rebuild it.
    pub fn new(
        package: &'a PackageModelInput<'a>,
        package_bytes: &'a [u8],
        supply: fsl_replay::HandoffSupply<'a>,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        key: &OccurrenceKey,
    ) -> Result<Self, PendingClauseForm> {
        let claim = package.state_graph_claim(
            key,
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )?;
        Ok(Self {
            input: package.input(),
            claim,
            occurrence: key.clone(),
            binding,
            limits,
            form: TemporalItem::default(),
            package_bytes,
            supply,
        })
    }

    /// The rebuilt claim.
    pub fn claim(&self) -> &CheckedStateGraphClaim {
        &self.claim
    }

    /// The same subject under the item's settings `form`.
    #[must_use]
    pub fn configured(mut self, form: TemporalItem) -> Self {
        self.form = form;
        self
    }

    /// The item that decides the claim as the source route's
    /// [`state_graph_item`] does.
    pub fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let key = self.occurrence.clone();
        let timed = !self.input.time_sources.is_empty();
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            fsl_route::Mode::Bounded,
            if timed {
                crate::zone_check::manifest::BACKEND_ID
            } else {
                en1::BACKEND_ID
            },
            move |cancel| {
                run(
                    &Run {
                        source: Source::Bytes {
                            package_bytes: self.package_bytes,
                            supply: self.supply,
                        },
                        input: &self.input,
                        binding: &self.binding,
                        limits: self.limits,
                        claim: &self.claim,
                        form: &self.form,
                        timed,
                        digital: false,
                    },
                    cancel,
                )
            },
        )
        .claimed(key)
    }
}
