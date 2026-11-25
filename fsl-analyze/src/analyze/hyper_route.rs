// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty command items over the request's admitted model subject
//! (FR-631, FR-640-AC-12): one EN-1 run of the clause, settled through the
//! replay gate.

use filament_exact::{Cancel, ValueType};
use fsl_eval::hyper::{CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    CheckedHyperLetters, ExpansionLimits, ModelSimulationRequest, ModelSystem,
};
use fsl_foundation::RequestIndex;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    BoundCovers, HyperOutcome, HyperReplay, IncompleteCause, RunStats, UndecidedCause,
    replay_hyper_trace_tuple, settle_hyper_check,
};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::items::{BROKEN, admission, model_refusal};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::model_check::{
    HyperCheckRequest, ModelRelationOutcome, ModelRelationRefusal, ModelRelationStats,
    alias_provisions, alias_system, check_hyper, manifest,
};

/// The item that runs the checked hyper clause `key` names, or none when the
/// occurrence names no hyper clause. Every alias ranges over the one admitted
/// request subject: its populations carry each model's objects, and each
/// object parameter names its own model's population.
pub(super) fn hyper_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let clause = analysis
        .package
        .graph()
        .hyper_clauses()
        .iter()
        .find(|clause| clause.assembled_occurrence() == Some(key))?;
    let package = analysis.package;
    let input = analysis.model.clone();
    let binding = binding.clone();
    let form = form.clone();
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            Mode::Bounded,
            manifest::BACKEND_ID,
            move |cancel: &Cancel| {
                let simulation = ModelSimulationRequest {
                    input: input.clone(),
                    packages: binding.packages,
                    normalization: limits.normalization,
                    snapshots: binding.snapshots,
                    initial: binding.initial.clone(),
                    universes: binding.universes.clone(),
                    observation: limits.observation,
                    expansion: ExpansionLimits {
                        max_candidates: form.max_candidates,
                    },
                    meter: limits.accounting,
                    emit: limits.emit,
                };
                // One system per alias, over its own model (FR-173 "Event-position
                // form": a trace is read over its own model), and the system of each
                // trace variable out of them, in quantifier order.
                let provisions = alias_provisions(&simulation, clause);
                let mut owned = Vec::with_capacity(clause.aliases().len());
                for (index, alias) in clause.aliases().iter().enumerate() {
                    let provision = provisions.get(index).and_then(Option::as_ref);
                    match alias_system(&simulation, provision, alias.as_str()) {
                        Ok(system) => owned.push(system),
                        Err(failure) => return EngineReport::Refused(admission(&failure)),
                    }
                }
                let slot_of = |name: &str| {
                    clause
                        .aliases()
                        .iter()
                        .position(|alias| alias.as_str() == name)
                };
                let mut systems = Vec::with_capacity(clause.quantifiers().len());
                for quantifier in clause.quantifiers() {
                    match slot_of(quantifier.alias.as_str()).and_then(|slot| owned.get(slot)) {
                        Some(system) => systems.push(system),
                        None => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                    }
                }
                let letters = match CheckedHyperLetters::new(package, clause, systems.clone()) {
                    Ok(letters) => letters,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                };
                let matcher = CheckedStepMatch::new(package, clause);
                // An HP-5 clause is decided by its own engine: lasso-and-product
                // closure, or closed finite executions under the event-position
                // profile (FR-173).
                if matches!(
                    clause.form(),
                    Some(fsl_semantics::check::HyperForm::SingleExistential)
                ) {
                    let Some(system) = owned.first() else {
                        return EngineReport::Refused(Refusal::Failed(BROKEN));
                    };
                    let mut alone = simulation.clone();
                    if let Some(provision) = provisions.first().and_then(Option::as_ref) {
                        alone.snapshots = provision;
                    }
                    return single(
                        package,
                        clause,
                        alone,
                        system,
                        &letters,
                        SingleControl {
                            form: &form,
                            limits,
                            cancel,
                        },
                    );
                }
                let outcome = match check_hyper(
                    HyperCheckRequest {
                        clause,
                        subjects: clause
                            .aliases()
                            .iter()
                            .map(|_| ModelSubject {
                                simulation: simulation.clone(),
                                proof_bounds: form.proof_bounds.clone(),
                            })
                            .collect(),
                        letters: &letters,
                        matcher: &matcher,
                        limits: form.limits,
                        max_depth: form.max_depth,
                        meter: limits.accounting,
                        reductions: form.reductions.clone(),
                    },
                    cancel,
                ) {
                    Ok(outcome) => outcome,
                    Err(refusal) => return EngineReport::Refused(model_refusal(&refusal)),
                };
                // The certificate is checked over each alias's own model, as the
                // engine built it.
                let subjects = (0..clause.aliases().len())
                    .map(|slot| {
                        let mut simulation = simulation.clone();
                        if let Some(provision) = provisions.get(slot).and_then(Option::as_ref) {
                            simulation.snapshots = provision;
                        }
                        ModelSubject {
                            simulation,
                            proof_bounds: form.proof_bounds.clone(),
                        }
                    })
                    .collect::<Vec<_>>();
                let certificate = fsl_replay::ProductClosureRequest {
                    package,
                    clause: fsl_replay::ProductClosureClause::Hyper(clause),
                    subjects: &subjects,
                    meter: limits.accounting,
                };
                let settled = settle_hyper_check(
                    &outcome,
                    &form.limits,
                    &form.requested_limits,
                    Some(&certificate),
                    cancel,
                    |payload| {
                        let mut instance = Vec::with_capacity(clause.parameters().len());
                        for parameter in clause.parameters() {
                            let key = payload
                                .instance
                                .iter()
                                .find(|(name, _)| name == &parameter.name)
                                .map(|(_, key)| key);
                            let fault = |cause| {
                                fsl_replay::ReplayRefusal::Fault(
                                    filament_semantic_value::catalog::InternalFault::new(
                                        "hyper-item",
                                        cause,
                                    ),
                                )
                            };
                            let system = slot_of(parameter.alias.as_str())
                                .and_then(|slot| owned.get(slot))
                                .ok_or_else(|| fault("parameter-alias"))?;
                            let admitted = system
                                .hyper_instances(&ValueType::Reference(
                                    parameter.population.object(),
                                ))
                                .map_err(|_| fault("parameter-universe"))?;
                            let bound = admitted
                                .into_iter()
                                .find(|instance| Some(&instance.key) == key)
                                .ok_or_else(|| fault("counterexample-instance"))?;
                            instance.push(ParameterBinding {
                                parameter: parameter.name.clone(),
                                instance: bound,
                            });
                        }
                        replay_hyper_trace_tuple(
                            &HyperReplay {
                                clause,
                                systems: systems.clone(),
                                instance: &instance,
                                letters: &letters,
                                matcher: &matcher,
                                meter: limits.accounting,
                                max_witness_set: form.limits.max_witness_set,
                                max_states: u64::try_from(form.limits.limits.max_states)
                                    .unwrap_or(u64::MAX),
                                cancel,
                            },
                            payload,
                        )
                    },
                );
                match settled {
                    Ok(record) => EngineReport::Hyper(Box::new(record)),
                    Err(_) => EngineReport::Refused(Refusal::Failed(BROKEN)),
                }
            },
        )
        .claimed(key.clone()),
    )
}

struct SingleControl<'a> {
    form: &'a TemporalItem,
    limits: AnalyzeLimits,
    cancel: &'a Cancel,
}

fn single(
    package: &fsl_package::CheckedPackage,
    clause: &fsl_semantics::check::CheckedHyperClause,
    simulation: ModelSimulationRequest<'_>,
    system: &ModelSystem<'_>,
    letters: &CheckedHyperLetters<'_, '_>,
    control: SingleControl<'_>,
) -> EngineReport {
    use fsl_replay::{GraphEvidence, StateGraphMethod, StateGraphPropertyForm, TerminalValue};
    let SingleControl {
        form,
        limits,
        cancel,
    } = control;
    let mut budgets = form.limits;
    if let Some(samples) = form.witness_samples {
        budgets.witness_samples = samples;
    }
    let subject = ModelSubject {
        simulation: simulation.clone(),
        proof_bounds: form.proof_bounds.clone(),
    };
    let run = match crate::model_check::check_single_existential(
        crate::model_check::SingleExistentialRequest {
            clause,
            subject,
            letters,
            limits: budgets,
            max_depth: form.max_depth,
            seed: form.seed,
            meter: limits.accounting,
        },
        cancel,
    ) {
        Ok(run) => run,
        Err(refusal) => return EngineReport::Refused(model_refusal(&refusal)),
    };
    let subjects = [ModelSubject {
        simulation,
        proof_bounds: form.proof_bounds.clone(),
    }];
    let request = fsl_replay::ProductClosureRequest {
        package,
        clause: fsl_replay::ProductClosureClause::Hyper(clause),
        subjects: &subjects,
        meter: limits.accounting,
    };
    let max_depth = match form.max_depth.map(u64::try_from).transpose() {
        Ok(depth) => depth,
        Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
    };
    let method = StateGraphMethod {
        limits: budgets,
        max_depth,
        seed: run.seed,
    };
    let mut chosen = None;
    for instance in &run.instances {
        let settled = match fsl_replay::settle_existential(&request, instance, &method, cancel) {
            Ok(settled) => settled,
            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        let mut wire = None;
        if let Some(evidence) = settled.evidence.as_ref() {
            let over = match evidence {
                GraphEvidence::Witness { over, .. }
                | GraphEvidence::Trap { over, .. }
                | GraphEvidence::PathPair { over, .. }
                | GraphEvidence::Undefined { over, .. } => over.as_ref(),
            };
            let reference = match (clause.parameters().first(), over) {
                (Some(parameter), Some(instance)) => {
                    match system.bound_reference(parameter.population.object(), instance) {
                        Ok(reference) => Some(reference),
                        Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                    }
                }
                (None, None) => None,
                _ => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
            wire = match fsl_replay::graph_evidence_wire(
                evidence,
                StateGraphPropertyForm::Possible,
                clause
                    .parameters()
                    .first()
                    .map(|parameter| parameter.name.as_str()),
                reference.as_ref(),
                &[],
                clause.assembled_occurrence(),
            ) {
                Ok(wire) => Some(wire),
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
        }
        let refuted = matches!(settled.record.value, TerminalValue::Refuted);
        let proved = matches!(settled.record.value, TerminalValue::Proved { .. });
        let report = super::GraphReport {
            record: settled.record,
            exploration: instance.stats,
            counterexample: if refuted { wire.clone() } else { None },
            evidence: if !refuted { wire } else { None },
            certificate_document: None,
            checker: proved.then_some(fsl_replay::CertificateChecker::ProductClosure),
        };
        if !proved {
            return EngineReport::StateGraph(Box::new(report));
        }
        chosen = Some(report);
    }
    match chosen {
        Some(report) => EngineReport::StateGraph(Box::new(report)),
        None => EngineReport::Refused(Refusal::Failed(BROKEN)),
    }
}

/// The item that runs the checked step relation `key` names (FR-179): the
/// reachable transition tuples of its executions enumerated explicitly and
/// settled through the replay gate, or none when the occurrence names no step
/// relation.
pub(super) fn relation_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
    shared: Option<crate::provider::Shared<'_>>,
) -> Option<AnalyzeItem<'a>> {
    let checked = analysis
        .package
        .graph()
        .step_relations()
        .iter()
        .find(|relation| relation.occurrence() == Some(key))?;
    let relation = match shared {
        Some(shared) => (|| {
            use fsl_package::AnalysisView;
            let view = fsl_package::BarePackage::new(shared.admitted);
            let row = view.claim_names().iter().find(|row| {
                view.node_key(&row.declaration).as_ref() == Some(key.node())
                    && row.occurrence.ordinal == key.origin().ordinal()
                    && row.occurrence.role == fsl_package::CheckedOccurrenceRole::Claim
            })?;
            let selected = fsl_package::ClaimOccurrence {
                declaration: row.declaration.clone(),
                occurrence: row.occurrence.clone(),
            };
            let package_form = view.model_relation_form(&selected).ok()?;
            fsl_eval::simulation::model::ModelRelationDescriptor::of_source_form(
                analysis.package,
                checked,
                &package_form,
            )
            .ok()
        })(),
        None => fsl_eval::simulation::model::ModelRelationDescriptor::of_source(
            analysis.package,
            checked,
        )
        .ok(),
    };
    let Some(relation) = relation else {
        return Some(
            AnalyzeItem::new(
                index,
                Capability::TemporalSatisfaction,
                Some(DeclarationFamily::RelationClaim),
                Mode::Bounded,
                manifest::BACKEND_ID,
                |_| EngineReport::Refused(Refusal::Failed(BROKEN)),
            )
            .claimed(key.clone()),
        );
    };
    let input = analysis.model.clone();
    let binding = binding.clone();
    let form = form.clone();
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::RelationClaim),
            Mode::Bounded,
            manifest::BACKEND_ID,
            move |cancel| run_relation(&input, &binding, limits, &relation, &form, cancel),
        )
        .claimed(key.clone()),
    )
}

fn run_relation(
    input: &fsl_eval::simulation::model::ModelCheckInput<'_>,
    binding: &SubjectBinding<'_>,
    limits: AnalyzeLimits,
    relation: &fsl_eval::simulation::model::ModelRelationDescriptor,
    form: &TemporalItem,
    cancel: &Cancel,
) -> EngineReport {
    let simulation = ModelSimulationRequest {
        input: input.clone(),
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits {
            max_candidates: form.max_candidates,
        },
        meter: limits.accounting,
        emit: limits.emit,
    };
    let subjects: Vec<_> = relation
        .aliases()
        .iter()
        .map(|_| ModelSubject {
            simulation: simulation.clone(),
            proof_bounds: form.proof_bounds.clone(),
        })
        .collect();
    let outcome = match crate::model_check::check_model_relation_input(
        crate::model_check::ModelRelationInput {
            relation,
            evaluator: input.evaluator,
            subjects: subjects
                .iter()
                .map(|subject| ModelSubject {
                    simulation: subject.simulation.clone(),
                    proof_bounds: subject.proof_bounds.clone(),
                })
                .collect(),
            limits: form.limits,
            max_depth: form.max_depth,
            meter: limits.accounting,
        },
        cancel,
    ) {
        Ok(outcome) => outcome,
        Err(ModelRelationRefusal::Model(refusal)) => {
            return EngineReport::Refused(model_refusal(&refusal));
        }
        Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
    };
    let counts = |stats: ModelRelationStats| RunStats {
        product_states: stats.states,
        product_edges: stats.transitions,
        ..RunStats::default()
    };
    let outcome = match outcome {
        ModelRelationOutcome::Holds { certificate, stats } => HyperOutcome::Holds {
            certificates: vec![certificate],
            stats: counts(stats),
        },
        ModelRelationOutcome::Refuted {
            counterexample,
            stats,
        } => HyperOutcome::Violated {
            counterexample,
            stats: counts(stats),
        },
        ModelRelationOutcome::BoundReached { depth, stats } => HyperOutcome::BoundReached {
            depth,
            covers: BoundCovers::JointSteps,
            stats: counts(stats),
        },
        ModelRelationOutcome::Stopped {
            limit,
            count,
            stats,
        } => HyperOutcome::Stopped {
            cause: match cancel.cause() {
                Some(source) if limit.is_none() => IncompleteCause::Cancelled { source },
                _ => IncompleteCause::ResourceExhausted,
            },
            limit,
            count,
            stats: counts(stats),
        },
        ModelRelationOutcome::Undecided { stats } => HyperOutcome::Undecided {
            cause: UndecidedCause::UndecidedSuccessor,
            stats: counts(stats),
        },
    };
    let mut systems = Vec::with_capacity(relation.executions().len());
    for alias in relation.execution_aliases() {
        let Some(identity) = relation.model_package_identity(alias) else {
            return EngineReport::Refused(Refusal::Failed(BROKEN));
        };
        match ModelSystem::for_model_identity(simulation.clone(), identity) {
            Ok(system) => systems.push(system),
            Err(failure) => return EngineReport::Refused(admission(&failure)),
        }
    }
    let certificate = fsl_replay::ModelRelationClosureRequest {
        evaluator: input.evaluator,
        relation,
        subjects: &subjects,
        meter: limits.accounting,
    };
    let settled = fsl_replay::settle_model_relation_check(
        &outcome,
        &form.limits,
        &form.requested_limits,
        &certificate,
        cancel,
        |payload| {
            fsl_replay::replay_model_relation_input(
                &fsl_replay::ModelRelationReplayInput {
                    relation,
                    evaluator: input.evaluator,
                    systems: systems.iter().collect(),
                    meter: limits.accounting,
                    cancel,
                },
                payload,
            )
        },
    );
    match settled {
        Ok(record) => EngineReport::Hyper(Box::new(record)),
        Err(_) => EngineReport::Refused(Refusal::Failed(BROKEN)),
    }
}

/// The package route's HP-1 subject, containing no checked source structures.
pub(super) struct PackageRelationSubject<'a> {
    input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    relation: fsl_eval::simulation::model::ModelRelationDescriptor,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
}

impl<'a> PackageRelationSubject<'a> {
    pub(super) fn new(
        input: &'a fsl_eval::simulation::model::PackageModelInput<'a>,
        relation: fsl_eval::simulation::model::ModelRelationDescriptor,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
    ) -> Self {
        Self {
            input: input.input(),
            relation,
            binding,
            limits,
            form: TemporalItem::default(),
        }
    }

    pub(super) fn configured(mut self, form: TemporalItem) -> Self {
        self.form = form;
        self
    }

    pub(super) fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let key = self.relation.occurrence().clone();
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::RelationClaim),
            Mode::Bounded,
            manifest::BACKEND_ID,
            move |cancel| {
                run_relation(
                    &self.input,
                    &self.binding,
                    self.limits,
                    &self.relation,
                    &self.form,
                    cancel,
                )
            },
        )
        .claimed(key)
    }
}
