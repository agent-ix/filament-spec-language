// SPDX-License-Identifier: AGPL-3.0-or-later
//! Materialize the builder's undefined path for independent checked replay.

use super::*;
use crate::exact_probabilistic::{PredicateRole, UndefinedPath, UndefinedRead};
use fsl_eval::automaton::{Monitor, PropertyAutomaton, QualifiedBoundedMonitor};
use fsl_eval::simulation::model::{ModelSimulationRequest, PredicateReading};
use fsl_replay::{replay_probabilistic_undefined, ProbabilityUndefinedWitness};

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    simulation: ModelSimulationRequest<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    path: UndefinedPath,
    monitor: Option<Monitor<PropertyAutomaton>>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    let system = match ModelSystem::new_with_cancel(simulation, cancel) {
        Ok(system) => system,
        Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
    };
    let built = materialize(&system, claim, &path, monitor, limits, cancel);
    let (witness, scheduler) = match built {
        Ok(built) => built,
        Err(error) => return failure(error, subject, limits),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    match replay_probabilistic_undefined(
        package,
        &system,
        &witness.path.item,
        &witness,
        limits,
        cancel,
    ) {
        Ok(ProbabilityWitnessReplay::Reproduced(token)) => {
            let bound = match &claim.form {
                ProbForm::Reach { bound, .. } | ProbForm::Probability { bound, .. } => bound,
                _ => return not_yet(),
            };
            let scope = package.graph().scope();
            let Some(occurrence) = claim.occurrence() else {
                return broken();
            };
            let wire = fsl_replay::probabilistic_undefined_wire(
                &witness,
                occurrence,
                bound,
                scope.types(),
                scope.variant_types(),
                &|state| system.state_key(state).ok().map(|(_, key)| key),
                &scheduler,
            );
            EngineReport::Exact(Box::new(ExactReport::Refuted(token, Box::new(wire))))
        }
        Ok(ProbabilityWitnessReplay::Parity) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayParity))
        }
        Err(error) => failure(error, subject, limits),
    }
}

fn failure(
    error: ProbabilityWitnessError,
    subject: SubjectLimits<'_>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
) -> EngineReport {
    match error {
        ProbabilityWitnessError::Build(issue) => settled(&issue, subject),
        ProbabilityWitnessError::Limit { limit, value } => {
            settled(&ExactProbBuildIssue::Limit { limit, value }, subject)
        }
        ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(cause)) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        ProbabilityWitnessError::Graph(ExactProbabilityGraphError::LimitReached) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: u64::try_from(limits.model.limits.max_states).unwrap_or(u64::MAX),
            },
            subject,
        ),
        ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::EdgeLimitReached | ExactProbabilityGraphError::TooManyEdges,
        ) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxTransitions,
                value: u64::try_from(limits.model.limits.max_transitions).unwrap_or(u64::MAX),
            },
            subject,
        ),
        ProbabilityWitnessError::Refused => {
            EngineReport::Exact(Box::new(ExactReport::ReplayRefused))
        }
        ProbabilityWitnessError::Graph(_) => not_yet(),
    }
}

type SpelledScheduler = Vec<(
    fsl_eval::simulation::StateKey,
    fsl_eval::simulation::model::ScheduledIdentity,
)>;

fn materialize(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    path: &UndefinedPath,
    monitor: Option<Monitor<PropertyAutomaton>>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
) -> Result<(ProbabilityUndefinedWitness, SpelledScheduler), ProbabilityWitnessError> {
    let item = fsl_replay::probabilistic_claim_identity(claim)
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let monitor = match (&claim.form, monitor) {
        (
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            },
            Some(monitor),
        ) => {
            let context = fsl_replay::probabilistic_formula_monitor_context(claim, reference)?;
            Some(
                QualifiedBoundedMonitor::from_monitor(
                    formula.formula(),
                    &context,
                    monitor,
                    limits.model.max_automaton_states,
                )
                .map_err(|_| ProbabilityWitnessError::Refused)?,
            )
        }
        (_, None) => None,
        _ => return Err(ProbabilityWitnessError::Refused),
    };
    let key = |state: &crate::exact_probabilistic::ProbKey| -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
        match monitor.as_ref() {
            Some(monitor) => bounded_product_key(state, reference, monitor, limits.document, cancel).map_err(|_| ProbabilityWitnessError::Refused),
            None => ProbabilityProductKey::for_model_state_with_limits(reference, &state.model, limits.document, cancel).map_err(|_| ProbabilityWitnessError::Refused),
        }
    };
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    let initial_key = if path.steps.is_empty() {
        match monitor.as_ref() {
            Some(monitor) => {
                let control = monitor
                    .unread_key()
                    .map_err(|_| ProbabilityWitnessError::Refused)?;
                ProbabilityProductKey::for_formula_state(
                    reference,
                    &path.key.model,
                    &control,
                    limits.document,
                    cancel,
                )
                .map_err(|_| ProbabilityWitnessError::Refused)?
            }
            None => key(&path.key)?,
        }
    } else if monitor.is_some() {
        let ProbForm::Probability {
            event: Event::Formula { formula, .. },
            ..
        } = &claim.form
        else {
            return Err(ProbabilityWitnessError::Refused);
        };
        let context = fsl_replay::probabilistic_formula_monitor_context(claim, reference)?;
        let mut initial_monitor = QualifiedBoundedMonitor::new(
            formula.formula(),
            &context,
            limits.model.max_automaton_states,
        )
        .map_err(|_| ProbabilityWitnessError::Refused)?;
        let predicate = formula
            .condition(initial_monitor.atom().index())
            .ok_or(ProbabilityWitnessError::Refused)?;
        let readings = system
            .state_predicates_with_cancel(initial, &[(predicate, reference)], cancel)
            .map_err(|stop| {
                ProbabilityWitnessError::Build(Box::new(ExactProbBuildIssue::Expansion(stop)))
            })?;
        let truth = match readings.as_slice() {
            [PredicateReading::True] => true,
            [PredicateReading::False] => false,
            _ => return Err(ProbabilityWitnessError::Refused),
        };
        let control = initial_monitor
            .start(truth)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        let control = initial_monitor
            .key(control)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        let (raw, _) = system
            .state_key(initial)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        ProbabilityProductKey::for_formula_state(reference, &raw, &control, limits.document, cancel)
            .map_err(|_| ProbabilityWitnessError::Refused)?
    } else {
        let (raw, _) = system
            .state_key(initial)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        ProbabilityProductKey::for_model_state_with_limits(reference, &raw, limits.document, cancel)
            .map_err(|_| ProbabilityWitnessError::Refused)?
    };
    let mut state = initial.clone();
    let mut steps = Vec::new();
    let mut scheduler = std::collections::BTreeMap::new();
    let mut spelled = Vec::new();
    let mut source_key = initial_key.clone();
    for step in &path.steps {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityWitnessError::Graph(
                ExactProbabilityGraphError::Cancelled(cause),
            ));
        }
        let (Some(action), Some(draw)) = (&step.action, &step.draw) else {
            return Err(ProbabilityWitnessError::Refused);
        };
        let choice = fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(system, action)
                .map_err(|_| ProbabilityWitnessError::Refused)?,
        );
        if matches!(claim.scheduler, ProbScheduler::Every) {
            scheduler.insert(source_key.clone(), choice.clone());
            spelled.push((
                system
                    .state_key(&state)
                    .map_err(|_| ProbabilityWitnessError::Refused)?
                    .1,
                action.clone(),
            ));
        }
        let mut matched = None;
        for candidate in system
            .actions_with_cancel(&state, cancel)
            .map_err(|stop| {
                ProbabilityWitnessError::Build(Box::new(ExactProbBuildIssue::Expansion(stop)))
            })?
            .map_err(|_| ProbabilityWitnessError::Refused)?
        {
            if candidate.identity != *action {
                continue;
            }
            for candidate_draw in candidate.draws {
                if !fsl_eval::simulation::model::same_values(&candidate_draw.random, draw) {
                    continue;
                }
                for (transition, post) in candidate_draw.successors {
                    if system
                        .state_key(&post)
                        .map_err(|_| ProbabilityWitnessError::Refused)?
                        .0
                        == step.target.model
                    {
                        if matched.is_some() {
                            return Err(ProbabilityWitnessError::Refused);
                        }
                        matched = Some((transition, post));
                    }
                }
            }
        }
        let Some((transition, post)) = matched else {
            return Err(ProbabilityWitnessError::Refused);
        };
        let post_key = key(&step.target)?;
        steps.push(ProbabilityWitnessStep {
            choice,
            action: Some(action.clone()),
            transition: Some(transition),
            draw: Some(draw.clone()),
            post: post.clone(),
            post_key: post_key.clone(),
            elapsed: Rational::from_integer(0_i64.into()),
        });
        source_key = post_key;
        state = post;
    }
    let locus = match path.read {
        UndefinedRead::Atom(atom) => fsl_replay::UndefinedLocus::Atom(atom),
        UndefinedRead::Predicate(role) => fsl_replay::UndefinedLocus::ClaimPredicate(match role {
            PredicateRole::From => fsl_replay::ClaimPredicate::From,
            PredicateRole::Until => fsl_replay::ClaimPredicate::Until,
            PredicateRole::Target => fsl_replay::ClaimPredicate::Target,
        }),
    };
    let product_state = if path.steps.is_empty() {
        initial_key.clone()
    } else {
        key(&path.key)?
    };
    Ok((
        ProbabilityUndefinedWitness {
            path: ProbabilisticCounterexample {
                item,
                binding: reference.cloned(),
                initial: initial.clone(),
                initial_key,
                scheduler: scheduler.into_iter().collect(),
                paths: vec![ProbabilityWitnessPath { steps }],
            },
            evaluation: fsl_replay::UndefinedEvaluation {
                site: fsl_replay::UndefinedSite {
                    position: path.position,
                    state_key: path.state_key.clone(),
                    locus,
                    transition: None,
                    time: None,
                },
                cause: path.cause.clone(),
            },
            product_state,
        },
        spelled,
    ))
}
