// SPDX-License-Identifier: AGPL-3.0-or-later
//! Materialize the builder's undefined path for independent checked replay.

use super::*;
use crate::exact_probabilistic::{PredicateRole, UndefinedPath, UndefinedRead};
use fsl_eval::automaton::{Monitor, PropertyAutomaton, QualifiedBoundedMonitor};
use fsl_eval::simulation::model::{
    ModelSimState, ModelSimulationRequest, PredicateReading, TimedSystem,
};
use fsl_replay::{ProbabilityUndefinedWitness, replay_probabilistic_undefined};

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    simulation: ModelSimulationRequest<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    path: UndefinedPath,
    monitor: Option<Monitor<PropertyAutomaton>>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    // The checked-subject undefined entry is independent of the numerical
    // threshold and covers each supported product-predicate read.
    if !matches!(
        claim.form,
        ProbForm::Reach { .. }
            | ProbForm::Probability { .. }
            | ProbForm::MeanOfFraction { .. }
            | ProbForm::Quantile { .. }
            | ProbForm::LongRunFraction { .. }
            | ProbForm::ExpectedReward { .. }
            | ProbForm::MeanTime { .. }
    ) {
        return not_yet();
    }
    let system = match ModelSystem::new_with_cancel(simulation, cancel) {
        Ok(system) => system,
        Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
    };
    let built = materialize(&system, claim, &path, monitor, limits, cancel);
    let (mut witness, scheduler) = match built {
        Ok(built) => built,
        Err(ProducerFailure::Invariant) => return broken(),
        Err(ProducerFailure::Stopped(error)) => return failure(error, subject, limits),
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness.path);
    }
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
            let quantile_bound;
            let Some(claim_type) = super::probabilistic_bound_type(&claim.form) else {
                return not_yet();
            };
            let bound = match &claim.form {
                ProbForm::Reach { bound, .. }
                | ProbForm::Probability { bound, .. }
                | ProbForm::LongRunFraction { bound, .. }
                | ProbForm::ExpectedReward { bound, .. }
                | ProbForm::MeanOfFraction { bound, .. }
                | ProbForm::MeanTime { bound, .. } => bound,
                ProbForm::Quantile { q, threshold, .. } => {
                    quantile_bound = match threshold {
                        Bound::AtMost(_) => Bound::AtLeast(q.clone()),
                        Bound::AtLeast(_) => Bound::AtMost(q.clone()),
                    };
                    &quantile_bound
                }
            };
            let scope = package.graph().scope();
            let Some(occurrence) = claim.occurrence() else {
                return broken();
            };
            let wire = fsl_replay::probabilistic_undefined_wire(
                &witness,
                occurrence,
                bound,
                claim_type,
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

pub(super) fn failure(
    error: ProbabilityWitnessError,
    subject: SubjectLimits<'_>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
) -> EngineReport {
    match error {
        ProbabilityWitnessError::Build(issue) => settled(&issue, subject),
        ProbabilityWitnessError::Limit { limit, value } => {
            settled(&ExactProbBuildIssue::Limit { limit, value }, subject)
        }
        ProbabilityWitnessError::CertificateRejected(rejection) => checked_report(
            Ok(ExactProbabilityCertificateCheck::Rejected(rejection)),
            ExactProbabilityCertificate { values: Vec::new() },
            None,
            None,
            subject,
        ),
        ProbabilityWitnessError::Refused => {
            EngineReport::Exact(Box::new(ExactReport::ReplayRefused))
        }
        ProbabilityWitnessError::Graph(error) => graph_failure(error, limits, subject),
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
) -> Result<(ProbabilityUndefinedWitness, SpelledScheduler), ProducerFailure> {
    materialize_inner(system, claim, path, monitor, limits, cancel).map_err(ProducerFailure::from)
}

fn materialize_inner(
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
    let timed = path.key.certificate_state.is_some();
    let measured = claim.bounded_measure() && !timed;
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let mut measure_meter =
        filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut measure_storage = 0;
    let reward = if measured {
        let kind = match claim.form {
            ProbForm::Quantile { .. } => fsl_replay::MeasurePayoffKind::Satisfied,
            ProbForm::MeanOfFraction { .. } => fsl_replay::MeasurePayoffKind::Fraction,
            _ => fsl_replay::MeasurePayoffKind::Event,
        };
        fsl_replay::measure_probability_objective(
            system,
            claim,
            kind,
            document_limits,
            cancel,
            &mut measure_meter,
            &mut measure_storage,
        )
        .map_err(ProbabilityWitnessError::from)?
        .1
    } else {
        None
    };
    let measure_scope = std::cell::RefCell::new((measure_meter, measure_storage));
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
                QualifiedBoundedMonitor::from_observation_monitor(
                    formula.formula(),
                    &context,
                    monitor,
                    limits.model.max_automaton_states,
                )
                .map_err(ProbabilityWitnessError::from)?,
            )
        }
        (
            ProbForm::Probability {
                event:
                    Event::Formula {
                        timed_horizon: Some(_),
                        ..
                    },
                ..
            },
            None,
        ) => None,
        (ProbForm::Reach { .. }, None) => None,
        (ProbForm::MeanTime { .. }, None) => None,
        (ProbForm::LongRunFraction { .. } | ProbForm::ExpectedReward { .. }, None) => None,
        (_, None) if measured || timed => None,
        _ => return Err(ProbabilityWitnessError::Refused),
    };
    let key = |state: &crate::exact_probabilistic::ProbKey| -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
        match monitor.as_ref() {
            Some(monitor) => bounded_product_key(state, reference, monitor, limits.document, cancel).map_err(ProbabilityWitnessError::from),
            None if state.certificate_state.is_some() => Ok(state
                .certificate_state
                .clone()
                .ok_or(ProbabilityWitnessError::Refused)?),
            None if measured => {
                let mut scope=measure_scope.borrow_mut();
                let (meter,storage)=&mut *scope;
                fsl_replay::measure_probability_product_key(reference,&state.model,state.monitor.ok_or(ProbabilityWitnessError::Refused)?,state.accumulator.as_ref().ok_or(ProbabilityWitnessError::Refused)?,None,reward.as_ref(),document_limits,cancel,meter,storage).map_err(ProbabilityWitnessError::from)
            },
            None => ProbabilityProductKey::for_model_state_with_limits(reference, &state.model, limits.document, cancel).map_err(ProbabilityWitnessError::from),
        }
    };
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    let mut measure_source = None;
    let initial_key = if let Some(initial_key) = path.initial_key.certificate_state.clone() {
        initial_key
    } else if path.steps.is_empty() {
        match monitor.as_ref() {
            Some(monitor) => {
                let control = monitor
                    .unread_key()
                    .map_err(ProbabilityWitnessError::from)?;
                ProbabilityProductKey::for_formula_state(
                    reference,
                    &path.key.model,
                    &control,
                    limits.document,
                    cancel,
                )
                .map_err(ProbabilityWitnessError::from)?
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
        let mut initial_monitor = QualifiedBoundedMonitor::for_observations(
            formula.formula(),
            &context,
            limits.model.max_automaton_states,
        )
        .map_err(ProbabilityWitnessError::from)?;
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
            [PredicateReading::Refused(outcome)] => {
                return Err(ProbabilityWitnessError::Build(Box::new(
                    ExactProbBuildIssue::Predicate(outcome.clone()),
                )));
            }
            _ => return Err(ProbabilityWitnessError::Refused),
        };
        let control = initial_monitor
            .start(truth)
            .map_err(ProbabilityWitnessError::from)?;
        let control = initial_monitor
            .key(control)
            .map_err(ProbabilityWitnessError::from)?;
        let (raw, _) = system
            .state_key(initial)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        ProbabilityProductKey::for_formula_state(reference, &raw, &control, limits.document, cancel)
            .map_err(ProbabilityWitnessError::from)?
    } else if measured {
        let mut scope = measure_scope.borrow_mut();
        let (meter, storage) = &mut *scope;
        let kind = match claim.form {
            ProbForm::Quantile { .. } => fsl_replay::MeasurePayoffKind::Satisfied,
            ProbForm::MeanOfFraction { .. } => fsl_replay::MeasurePayoffKind::Fraction,
            _ => fsl_replay::MeasurePayoffKind::Event,
        };
        let (control, sums) = kind
            .initial_control(
                system,
                claim,
                false,
                document_limits,
                cancel,
                meter,
                storage,
            )
            .map_err(ProbabilityWitnessError::from)?;
        measure_source = Some((
            control.ok_or(ProbabilityWitnessError::Refused)?,
            sums.clone().ok_or(ProbabilityWitnessError::Refused)?,
        ));
        let mut admission = fsl_replay::ProbabilityKeyAdmission::new(
            meter,
            storage,
            document_limits.storage_bytes,
            cancel,
        );
        let raw = system
            .state_key_bytes_with_budget(
                initial,
                fsl_foundation::IdentityLimits::default(),
                &mut admission,
                cancel,
            )
            .map_err(|error| ProbabilityWitnessError::from(admission.refusal(error)))?;
        fsl_replay::measure_probability_product_key(
            reference,
            &raw,
            control.ok_or(ProbabilityWitnessError::Refused)?,
            sums.as_ref().ok_or(ProbabilityWitnessError::Refused)?,
            None,
            reward.as_ref(),
            document_limits,
            cancel,
            meter,
            storage,
        )
        .map_err(ProbabilityWitnessError::from)?
    } else {
        let (raw, _) = system
            .state_key(initial)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        ProbabilityProductKey::for_model_state_with_limits(reference, &raw, limits.document, cancel)
            .map_err(ProbabilityWitnessError::from)?
    };
    let mut state = initial.clone();
    let timed_system = if timed {
        Some(
            TimedSystem::new(system, &claim.subject.alias)
                .map_err(|_| ProbabilityWitnessError::Refused)?,
        )
    } else {
        None
    };
    let mut timed_state = timed_system.as_ref().and_then(|timed| {
        let initial_key = system.state_key(initial).ok()?.0;
        timed.initial().into_iter().find(|candidate| {
            system
                .state_key(candidate.discrete())
                .ok()
                .is_some_and(|(candidate, _)| candidate == initial_key)
        })
    });
    let timed_time = path
        .key
        .certificate_state
        .as_ref()
        .and_then(|_| TimedSystem::new(system, &claim.subject.alias).ok())
        .map(|_| {
            path.steps.iter().fold(
                Rational::from_integer(filament_exact::Integer::zero()),
                |sum, step| sum.add(&step.elapsed),
            )
        });
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
        let Some(action) = step.action.as_ref() else {
            return Err(ProbabilityWitnessError::Refused);
        };
        let choice = fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(system, action)
                .map_err(|_| ProbabilityWitnessError::Refused)?,
        );
        if matches!(claim.scheduler, ProbScheduler::Every) {
            scheduler.insert(source_key.clone(), choice.clone());
            spelled.push((
                if measured || timed_system.is_some() {
                    fsl_eval::simulation::StateKey::from_canonical_bytes(
                        source_key.as_bytes().to_vec(),
                    )
                } else {
                    system
                        .state_key(&state)
                        .map_err(|_| ProbabilityWitnessError::Refused)?
                        .1
                },
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
                if step.draw.as_ref().is_some_and(|draw| {
                    !fsl_eval::simulation::model::same_values(&candidate_draw.random, draw)
                }) {
                    continue;
                }
                for (transition, post) in candidate_draw.successors {
                    if target_matches_model(system, &step.target, &post)? {
                        if matched.is_some() {
                            if step.draw.is_some() {
                                return Err(ProbabilityWitnessError::Refused);
                            }
                            continue;
                        }
                        matched = Some((transition, post, candidate_draw.random.clone()));
                    }
                }
            }
        }
        let Some((transition, post, draw)) = matched else {
            return Err(ProbabilityWitnessError::Refused);
        };
        if let (Some(timed), Some(current)) = (timed_system.as_ref(), timed_state.as_ref()) {
            let delayed = timed
                .delay(current, &step.elapsed)
                .map_err(|_| ProbabilityWitnessError::Refused)?
                .ok_or(ProbabilityWitnessError::Refused)?;
            let post_key = system
                .state_key(&post)
                .map_err(|_| ProbabilityWitnessError::Refused)?
                .0;
            let mut reached = None;
            for (guarded, next) in timed
                .steps(&delayed)
                .map_err(|_| ProbabilityWitnessError::Refused)?
            {
                if guarded.transition() == &transition
                    && system
                        .state_key(guarded.post())
                        .map_err(|_| ProbabilityWitnessError::Refused)?
                        .0
                        == post_key
                {
                    if reached.is_some() {
                        return Err(ProbabilityWitnessError::Refused);
                    }
                    reached = Some(next);
                }
            }
            timed_state = Some(reached.ok_or(ProbabilityWitnessError::Refused)?);
        }
        let post_key = key(&step.target)?;
        if measured
            && timed_system.is_none()
            && step.resolved.is_some()
            && matches!(claim.scheduler, ProbScheduler::Every)
        {
            let (control, sums) = measure_source
                .as_ref()
                .ok_or(ProbabilityWitnessError::Refused)?;
            let mut scope = measure_scope.borrow_mut();
            let (meter, storage) = &mut *scope;
            let mut admission = fsl_replay::ProbabilityKeyAdmission::new(
                meter,
                storage,
                document_limits.storage_bytes,
                cancel,
            );
            let raw = system
                .state_key_bytes_with_budget(
                    &state,
                    fsl_foundation::IdentityLimits::default(),
                    &mut admission,
                    cancel,
                )
                .map_err(|error| ProbabilityWitnessError::from(admission.refusal(error)))?;
            let pending = system
                .pending_key_bytes_with_budget(
                    action,
                    &draw,
                    fsl_foundation::IdentityLimits::default(),
                    &mut admission,
                    cancel,
                )
                .map_err(|error| ProbabilityWitnessError::from(admission.refusal(error)))?;
            drop(admission);
            let pending = fsl_replay::measure_probability_product_key(
                reference,
                &raw,
                *control,
                sums,
                Some(&pending),
                reward.as_ref(),
                document_limits,
                cancel,
                meter,
                storage,
            )
            .map_err(ProbabilityWitnessError::from)?;
            scheduler.insert(
                pending,
                fsl_replay::ProductAction::PostState(post_key.clone()),
            );
        }
        if measured && timed_system.is_none() {
            measure_source = Some((
                step.target
                    .monitor
                    .ok_or(ProbabilityWitnessError::Refused)?,
                step.target
                    .accumulator
                    .clone()
                    .ok_or(ProbabilityWitnessError::Refused)?,
            ));
        }
        steps.push(ProbabilityWitnessStep {
            choice,
            action: Some(action.clone()),
            transition: Some(transition),
            draw: Some(draw),
            post: post.clone(),
            post_key: post_key.clone(),
            elapsed: step.elapsed.clone(),
        });
        source_key = post_key;
        state = post;
    }
    let locus = match path.read {
        UndefinedRead::FractionValue => fsl_replay::UndefinedLocus::FractionValue,
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
                instance: None,
                initial: initial.clone(),
                initial_key,
                scheduler: scheduler
                    .into_iter()
                    .map(|(key, action)| {
                        (
                            key,
                            fsl_replay::ProbabilisticSchedulerChoice::Deterministic(action),
                        )
                    })
                    .collect(),
                paths: vec![ProbabilityWitnessPath { steps }],
                subsystem: None,
            },
            evaluation: fsl_replay::UndefinedEvaluation {
                site: fsl_replay::UndefinedSite {
                    position: path.position,
                    state_key: path.state_key.clone(),
                    locus,
                    transition: None,
                    time: timed_time.clone().and_then(|stamp| {
                        let timed = timed_system.as_ref()?;
                        let state = timed_state.as_ref()?;
                        let clocks = timed
                            .places()
                            .iter()
                            .zip(state.clocks().values())
                            .filter(|(place, _)| timed.live(place, state.discrete()))
                            .map(|(_, clock)| clock.clone())
                            .collect();
                        Some(Box::new(fsl_replay::TimedPoint { stamp, clocks }))
                    }),
                },
                cause: path.cause.clone(),
            },
            product_state,
        },
        spelled,
    ))
}

fn target_matches_model(
    system: &ModelSystem<'_>,
    target: &crate::exact_probabilistic::ProbKey,
    state: &ModelSimState,
) -> Result<bool, ProbabilityWitnessError> {
    let (raw, _) = system
        .state_key(state)
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let Some(certificate) = target.certificate_state.as_ref() else {
        return Ok(raw == target.model);
    };
    let value: serde_json::Value = serde_json::from_slice(certificate.as_bytes())
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let Some(discrete) = value
        .get(4)
        .and_then(|model| model.get(1))
        .and_then(|timed| timed.get(1))
    else {
        return Ok(false);
    };
    let expected = serde_json::to_vec(discrete).map_err(|_| ProbabilityWitnessError::Refused)?;
    Ok(raw == expected)
}

enum ProducerFailure {
    Invariant,
    Stopped(ProbabilityWitnessError),
}
impl From<ProbabilityWitnessError> for ProducerFailure {
    fn from(error: ProbabilityWitnessError) -> Self {
        match error {
            ProbabilityWitnessError::Refused
            | ProbabilityWitnessError::Graph(
                ExactProbabilityGraphError::ExpansionRefused
                | ExactProbabilityGraphError::MissingSuccessor
                | ExactProbabilityGraphError::UnsupportedInitial
                | ExactProbabilityGraphError::ClaimNotBound,
            ) => Self::Invariant,
            error => Self::Stopped(error),
        }
    }
}
