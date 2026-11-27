// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked-subject replay of a first undefined probability observation.

use super::*;
use fsl_eval::automaton::{BoundedState, QualifiedBoundedMonitor, StateId};
use fsl_semantics::check::Event;
use ix_trace_rs::implements;

/// Untrusted path and the exact pre-read location it claims is undefined.
#[derive(Clone, Debug)]
pub struct ProbabilityUndefinedWitness {
    /// Exactly one positive-probability path, with its scheduler and binding.
    pub path: ProbabilisticCounterexample,
    /// First undefined evaluation, including predicate locus and position.
    pub evaluation: crate::UndefinedEvaluation,
    /// Complete product key before the failing observation advances control.
    pub product_state: ProbabilityProductKey,
}

/// Re-execute an undefined path against its checked package and admitted subject.
/// The threshold has no role: only the first undefined observation of an
/// undecided claim can issue a reproduced-witness token. This entry admits
/// untimed product forms whose checked predicates are replayed here.
pub fn replay_probabilistic_undefined(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    replay_probabilistic_undefined_with_work_scope(
        package, system, item, witness, limits, cancel, None,
    )
}

/// Replay an undefined path under the item's shared work allowance.
#[implements("FR-203-AC-33")]
#[allow(clippy::too_many_arguments)]
pub fn replay_probabilistic_undefined_with_work_scope(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    work: Option<&filament_exact::WorkScope>,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    let evidence = &witness.path;
    if evidence.subsystem.is_some() {
        return Err(ProbabilityWitnessError::Refused);
    }
    if !system
        .source_package()
        .is_some_and(|own| std::ptr::eq(package, own))
        || item != &evidence.item
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let claim = bind_claim(package.graph().probabilistic_model().claims(), item, cancel)?;
    if claim.bounded_measure() {
        return super::measure::replay_undefined(system, claim, witness, limits, cancel, work);
    }
    if !matches!(
        TimedSystem::new(system, &claim.subject.alias),
        Err(TimedSubjectRefusal::NoTimedModel { .. })
    ) {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != evidence.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let workload = claim_workload(system, claim)?;
    if workload.is_some() && !evidence.scheduler.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let [path] = evidence.paths.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if limits.max_witness_paths == 0 {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
        });
    }
    let mut monitor = match &claim.form {
        ProbForm::Probability {
            event:
                Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
            ..
        } => {
            let context = probabilistic_formula_monitor_context(claim, reference)?;
            Some(
                QualifiedBoundedMonitor::for_observations(
                    formula.formula(),
                    &context,
                    limits.model.max_automaton_states,
                )
                .map_err(bounded::monitor_error)?,
            )
        }
        ProbForm::Reach { .. }
            if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                == Some(fsl_semantics::check::TemporalProfile::InfiniteTrace) =>
        {
            None
        }
        ProbForm::LongRunFraction { .. } | ProbForm::ExpectedReward { .. } => None,
        _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
    };
    let mut control: Option<StateId> = None;
    let mut storage = 0;
    let mut meter = work
        .map_or_else(
            || filament_exact::Meter::new(system.meter_limits()),
            |scope| scope.meter(),
        )
        .with_cancel(cancel.clone());
    let deterministic = super::deterministic_scheduler_rows(&evidence.scheduler)?;
    let scheduler = step::scheduler(&deterministic, limits, &mut meter, &mut storage, cancel)?;
    let mut transition_count = 0;
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    if key_of(system, reference, initial, limits, cancel)?
        != key_of(system, reference, &evidence.initial, limits, cancel)?
        || initial.anchor() != evidence.initial.anchor()
        || initial.reached_by() != evidence.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let mut state = initial.clone();
    let mut mass = rational(1);
    let mut seen = BTreeSet::new();
    let mut used = BTreeSet::new();
    // Past a state of decided control the path only walks model steps toward
    // a contract conjunction with no value (ADR-018 UE-6): no predicate is
    // read there and the decided control is carried unchanged.
    let mut swept = false;
    for position in 0..=path.steps.len() {
        bounded::visit(&mut meter, cancel)?;
        if let (Some(monitor), Some(control), false) = (monitor.as_ref(), control, swept) {
            if !matches!(monitor.state(control).map_err(bounded::monitor_error)?, BoundedState::Open { remaining } if remaining > 0)
            {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        let (raw, state_key) = system
            .state_key(&state, fsl_foundation::IdentityLimits::default())
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        let pre_key = match monitor.as_ref() {
            Some(monitor) => {
                let key = match control {
                    Some(control) => monitor.key(control),
                    None => monitor.unread_key(),
                }
                .map_err(bounded::monitor_error)?;
                ProbabilityProductKey::for_formula_state(
                    reference,
                    &raw,
                    &key,
                    limits.document,
                    cancel,
                )
                .map_err(bounded::document_error)?
            }
            None => key_of(system, reference, &state, limits, cancel)?,
        };
        let mut truths = Vec::new();
        if !swept {
            let predicates: Vec<(
                &fsl_semantics::check::CheckedPredicate,
                crate::UndefinedLocus,
            )> = match &claim.form {
                ProbForm::Probability {
                    event: Event::Formula { formula, .. },
                    ..
                } => {
                    let atom = monitor
                        .as_ref()
                        .ok_or(ProbabilityWitnessError::Refused)?
                        .atom();
                    vec![(
                        formula
                            .condition(atom.index())
                            .ok_or(ProbabilityWitnessError::Refused)?,
                        crate::UndefinedLocus::Atom(atom),
                    )]
                }
                ProbForm::Reach {
                    target: ReachTarget::Eventually(p) | ReachTarget::Always(p),
                    ..
                } => vec![(
                    p,
                    crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Target),
                )],
                ProbForm::Reach {
                    target: ReachTarget::Until(a, b),
                    ..
                } => vec![
                    (
                        a,
                        crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::From),
                    ),
                    (
                        b,
                        crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
                    ),
                ],
                ProbForm::LongRunFraction { predicate, .. } => vec![(
                    predicate,
                    crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Target),
                )],
                ProbForm::ExpectedReward { until, .. } => vec![(
                    until,
                    crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
                )],
                _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
            };
            let inputs = predicates
                .iter()
                .map(|(p, _)| (*p, reference))
                .collect::<Vec<_>>();
            let readings = system
                .state_predicates_with_cancel(&state, &inputs, cancel)
                .map_err(|stop| cancelled_expansion(stop, cancel))?;
            poll_cancel(cancel)?;
            for ((_, locus), reading) in predicates.iter().zip(readings) {
                match reading {
                    PredicateReading::True => truths.push(true),
                    PredicateReading::False => truths.push(false),
                    PredicateReading::Refused(outcome) => {
                        return Err(ProbabilityWitnessError::Build(Box::new(
                            crate::ExactProbBuildIssue::Predicate(outcome),
                        )));
                    }
                    PredicateReading::Undefined { cause, .. } => {
                        if !seen.contains(&pre_key) && seen.len() >= limits.model.limits.max_states {
                            return Err(ExactProbabilityGraphError::LimitReached.into());
                        }
                        let evaluation = crate::UndefinedEvaluation {
                            site: crate::UndefinedSite {
                                position: u64::try_from(position).unwrap_or(u64::MAX),
                                state_key,
                                locus: locus.clone(),
                                transition: None,
                                time: None,
                            },
                            cause,
                        };
                        let offered_key = if position == 0 {
                            &evidence.initial_key
                        } else {
                            &path.steps[position - 1].post_key
                        };
                        if position != path.steps.len()
                            || evaluation != witness.evaluation
                            || pre_key != witness.product_state
                            || offered_key != &pre_key
                            || scheduler.keys().any(|key| !used.contains(*key))
                        {
                            return Ok(ProbabilityWitnessReplay::Parity);
                        }
                        bounded::retain(pre_key.as_bytes().len(), &mut storage, limits)?;
                        check_bits(&mass, limits.max_rational_bits)?;
                        let token = ReproducedProbabilityWitness {
                            evidence: evidence.clone(),
                            mass,
                            undefined: Some(Box::new(witness.clone())),
                        };
                        poll_cancel(cancel)?;
                        return Ok(ProbabilityWitnessReplay::Reproduced(Box::new(token)));
                    }
                }
            }
        }
        let (key, decided) = match monitor.as_mut() {
            Some(monitor) if swept => {
                let control = control.ok_or(ProbabilityWitnessError::Refused)?;
                let accepted = match monitor.state(control).map_err(bounded::monitor_error)? {
                    BoundedState::Accept => true,
                    BoundedState::Reject => false,
                    _ => return Ok(ProbabilityWitnessReplay::Parity),
                };
                let decision = monitor
                    .decision_key(accepted)
                    .map_err(bounded::monitor_error)?;
                (
                    ProbabilityProductKey::for_formula_state(
                        reference,
                        &raw,
                        &decision,
                        limits.document,
                        cancel,
                    )
                    .map_err(bounded::document_error)?,
                    true,
                )
            }
            None if swept => (pre_key.clone(), true),
            Some(monitor) => {
                let next = match control {
                    Some(control) => monitor.step(control, truths[0]),
                    None => monitor.start(truths[0]),
                }
                .map_err(bounded::monitor_error)?;
                control = Some(next);
                let key = monitor.key(next).map_err(bounded::monitor_error)?;
                (
                    ProbabilityProductKey::for_formula_state(
                        reference,
                        &raw,
                        &key,
                        limits.document,
                        cancel,
                    )
                    .map_err(bounded::document_error)?,
                    !matches!(
                        monitor.state(next).map_err(bounded::monitor_error)?,
                        BoundedState::Open { .. }
                    ),
                )
            }
            None => {
                let decided = match &claim.form {
                    ProbForm::Reach {
                        target: ReachTarget::Eventually(_),
                        ..
                    } => truths[0],
                    ProbForm::Reach {
                        target: ReachTarget::Always(_),
                        ..
                    } => !truths[0],
                    ProbForm::Reach {
                        target: ReachTarget::Until(..),
                        ..
                    } => truths[1] || !truths[0],
                    ProbForm::LongRunFraction { .. } => false,
                    ProbForm::ExpectedReward { .. } => truths[0],
                    _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
                };
                (pre_key, decided)
            }
        };
        let offered_key = if position == 0 {
            &evidence.initial_key
        } else {
            &path.steps[position - 1].post_key
        };
        // A contract conjunction with no value at the state is read from the
        // state's expansion, decided control or not (ADR-018 UE-6): the
        // claim need not read it.
        if position == path.steps.len() && &key == offered_key {
            if let crate::UndefinedLocus::Contract(clause) = &witness.evaluation.site.locus {
                return contract_witness(
                    system,
                    &state,
                    clause,
                    position,
                    state_key,
                    &key,
                    ContractWitness {
                        witness,
                        evidence,
                        mass,
                        scheduler_unused: scheduler.keys().any(|key| !used.contains(*key)),
                    },
                    cancel,
                    (&mut storage, limits),
                );
            }
        }
        if &key != offered_key {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if decided {
            // Only a contract witness walks on from decided control.
            if !matches!(
                &witness.evaluation.site.locus,
                crate::UndefinedLocus::Contract(_)
            ) {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            swept = true;
        }
        if !seen.insert(key.clone()) && workload.is_none() {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if seen.len() > limits.model.limits.max_states {
            return Err(ExactProbabilityGraphError::LimitReached.into());
        }
        bounded::retain(key.as_bytes().len(), &mut storage, limits)?;
        let Some(step) = path.steps.get(position) else {
            return Ok(ProbabilityWitnessReplay::Parity);
        };
        if position >= limits.model.limits.max_transitions {
            return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
        }
        if workload.is_none() && scheduler.get(&key).copied() != Some(&step.choice) {
            return Err(ProbabilityWitnessError::Refused);
        }
        used.insert(key);
        let expected = key_of(system, reference, &step.post, limits, cancel)?;
        let (probability, post, ()) = step::replay(
            system,
            &state,
            workload,
            step,
            limits,
            &mut meter,
            &mut storage,
            &mut transition_count,
            cancel,
            step::ActionRefusal::UnsupportedDraw,
            |post, _, _, _| {
                Ok((key_of(system, reference, post, limits, cancel)? == expected).then_some(()))
            },
        )?;
        mass = bounded::arithmetic(
            filament_exact::RationalArithmetic::Multiply(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
        state = post;
    }
    Ok(ProbabilityWitnessReplay::Parity)
}

/// What the replay of the path to a contract-locus witness carries.
struct ContractWitness<'w> {
    witness: &'w ProbabilityUndefinedWitness,
    evidence: &'w ProbabilisticCounterexample,
    mass: Rational,
    /// Whether the scheduler names a state the path did not use.
    scheduler_unused: bool,
}

/// Re-expand `state`, the path's last state, and reproduce the witness when
/// the named clause has no value there with the offered cause and guarded
/// transition (ADR-018 UE-6).
#[implements("FR-203-AC-33")]
#[allow(clippy::too_many_arguments)]
fn contract_witness(
    system: &ModelSystem<'_>,
    state: &fsl_eval::simulation::model::ModelSimState,
    clause: &str,
    position: usize,
    state_key: fsl_eval::simulation::StateKey,
    product: &ProbabilityProductKey,
    carried: ContractWitness<'_>,
    cancel: &Cancel,
    (storage, limits): (&mut u64, &ExactProbLimits),
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let expansion = system
        .actions_with_cancel(state, cancel)
        .map_err(|stop| cancelled_expansion(stop, cancel))?;
    poll_cancel(cancel)?;
    let Err(undecided) = expansion else {
        return Ok(ProbabilityWitnessReplay::Parity);
    };
    let Some(contract) = undecided.undefined else {
        return Ok(ProbabilityWitnessReplay::Parity);
    };
    let evaluation = crate::UndefinedEvaluation {
        site: crate::UndefinedSite {
            position: u64::try_from(position).unwrap_or(u64::MAX),
            state_key,
            locus: crate::UndefinedLocus::Contract(contract.clause.clone()),
            transition: contract.guard_of,
            time: None,
        },
        cause: contract.cause,
    };
    if contract.clause != clause
        || evaluation != carried.witness.evaluation
        || product != &carried.witness.product_state
        || carried.scheduler_unused
    {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    bounded::retain(product.as_bytes().len(), storage, limits)?;
    check_bits(&carried.mass, limits.max_rational_bits)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence: carried.evidence.clone(),
            mass: carried.mass,
            undefined: Some(Box::new(carried.witness.clone())),
        },
    )))
}
