// SPDX-License-Identifier: AGPL-3.0-or-later
//! Step-relation command items (FR-179) over the request's admitted model
//! subject.

use filament_exact::Cancel;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    replay_model_relation_tuple, settle_model_relation, IncompleteCause, ModelRelationReplay,
    ModelRelationRun, RunStats,
};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::items::{admission, model_refusal, BROKEN};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::model_check::{
    check_model_relation, manifest, ModelRelationCheckRequest, ModelRelationOutcome,
    ModelRelationRefusal, ModelRelationStats,
};

/// Build an item only when the occurrence identifies a checked step relation.
pub(super) fn relation_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let package = analysis.package;
    let relation = package
        .graph()
        .step_relations()
        .iter()
        .find(|relation| relation.occurrence() == Some(key))?;
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
                let mut aliases = Vec::new();
                for alias in relation.execution_aliases() {
                    if !aliases.contains(alias) {
                        aliases.push(alias);
                    }
                }
                let outcome = match check_model_relation(
                    ModelRelationCheckRequest {
                        relation,
                        package,
                        subjects: aliases
                            .iter()
                            .map(|_| ModelSubject {
                                simulation: simulation.clone(),
                                proof_bounds: form.proof_bounds.clone(),
                            })
                            .collect(),
                        limits: form.limits,
                        max_depth: form.max_depth,
                        meter: limits.accounting,
                    },
                    cancel,
                ) {
                    Ok(outcome) => outcome,
                    Err(refusal) => return EngineReport::Refused(relation_refusal(&refusal)),
                };
                let run = run_of(outcome, cancel);
                let systems = vec![&system; relation.executions().len()];
                let record = settle_model_relation(
                    &run,
                    &form.limits,
                    &form.requested_limits,
                    |counterexample| {
                        replay_model_relation_tuple(
                            &ModelRelationReplay {
                                relation,
                                package,
                                systems,
                                meter: limits.accounting,
                                cancel,
                            },
                            counterexample,
                        )
                    },
                );
                EngineReport::Hyper(Box::new(record))
            },
        )
        .claimed(key.clone()),
    )
}

/// The settlement input of an engine outcome.
fn run_of(outcome: ModelRelationOutcome, cancel: &Cancel) -> ModelRelationRun {
    match outcome {
        ModelRelationOutcome::Holds { stats } => ModelRelationRun::Holds {
            stats: run_stats(stats),
        },
        ModelRelationOutcome::Refuted {
            counterexample,
            stats,
        } => ModelRelationRun::Violated {
            counterexample,
            stats: run_stats(stats),
        },
        ModelRelationOutcome::BoundReached { depth, stats } => ModelRelationRun::BoundReached {
            depth,
            stats: run_stats(stats),
        },
        ModelRelationOutcome::Undecided { stats } => ModelRelationRun::Undecided {
            stats: run_stats(stats),
        },
        ModelRelationOutcome::Stopped {
            limit,
            count,
            stats,
        } => ModelRelationRun::Stopped {
            cause: match (limit, cancel.cause()) {
                (None, Some(source)) => IncompleteCause::Cancelled { source },
                _ => IncompleteCause::ResourceExhausted,
            },
            limit,
            count,
            stats: run_stats(stats),
        },
    }
}

fn run_stats(stats: ModelRelationStats) -> RunStats {
    RunStats {
        product_states: stats.states,
        product_edges: stats.transitions,
        ..RunStats::default()
    }
}

fn relation_refusal(refusal: &ModelRelationRefusal) -> Refusal {
    match refusal {
        ModelRelationRefusal::Model(refusal) => model_refusal(refusal),
        ModelRelationRefusal::SubjectCount { .. }
        | ModelRelationRefusal::SubjectAlias { .. }
        | ModelRelationRefusal::Fault(_) => Refusal::Failed(BROKEN),
    }
}
