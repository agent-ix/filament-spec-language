// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty command items over the request's admitted model subject.

use filament_exact::{Cancel, ValueType};
use fsl_eval::hyper::{CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    CheckedHyperLetters, ExpansionLimits, ModelSimulationRequest, ModelSystem,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{replay_hyper_trace_tuple, settle_hyper_check, HyperReplay};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::items::{admission, model_refusal, BROKEN};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::model_check::{check_hyper, manifest, HyperCheckRequest};

/// Build an item only when the occurrence identifies a checked hyper clause.
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
                let system = match ModelSystem::new(simulation.clone()) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let systems = vec![&system; clause.quantifiers().len()];
                let letters = match CheckedHyperLetters::new(package, clause, systems.clone()) {
                    Ok(letters) => letters,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                };
                let matcher = CheckedStepMatch::new(package, clause);
                if matches!(
                    clause.form(),
                    Some(fsl_semantics::check::HyperForm::SingleExistential)
                ) {
                    return single(package, clause, simulation, &letters, &form, limits, cancel);
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
                let subjects = clause
                    .aliases()
                    .iter()
                    .map(|_| ModelSubject {
                        simulation: simulation.clone(),
                        proof_bounds: form.proof_bounds.clone(),
                    })
                    .collect::<Vec<_>>();
                let certificate = fsl_replay::ProductClosureRequest {
                    package,
                    clause,
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
                            let admitted = system
                                .instances(&ValueType::Reference(parameter.population.object()))
                                .map_err(|_| {
                                    fsl_replay::ReplayRefusal::Fault(
                                        filament_semantic_value::catalog::InternalFault::new(
                                            "hyper-item",
                                            "parameter-universe",
                                        ),
                                    )
                                })?;
                            let binding = admitted
                                .into_iter()
                                .find(|instance| Some(&instance.key) == key)
                                .ok_or_else(|| {
                                    fsl_replay::ReplayRefusal::Fault(
                                        filament_semantic_value::catalog::InternalFault::new(
                                            "hyper-item",
                                            "counterexample-instance",
                                        ),
                                    )
                                })?;
                            instance.push(ParameterBinding {
                                parameter: parameter.name.clone(),
                                instance: binding,
                            });
                        }
                        replay_hyper_trace_tuple(
                            &HyperReplay {
                                clause,
                                systems,
                                instance: &instance,
                                letters: &letters,
                                matcher: &matcher,
                                meter: limits.accounting,
                                max_witness_set: form.limits.max_witness_set,
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

fn single(
    package: &fsl_package::CheckedPackage,
    clause: &fsl_semantics::check::CheckedHyperClause,
    simulation: ModelSimulationRequest<'_>,
    letters: &CheckedHyperLetters<'_, '_>,
    form: &TemporalItem,
    limits: AnalyzeLimits,
    cancel: &Cancel,
) -> EngineReport {
    use fsl_replay::{GraphEvidence, StateGraphMethod, StateGraphPropertyForm, TerminalValue};
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
    let system = match ModelSystem::new(subjects[0].simulation.clone()) {
        Ok(system) => system,
        Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
    };
    let request = fsl_replay::ProductClosureRequest {
        package,
        clause,
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
