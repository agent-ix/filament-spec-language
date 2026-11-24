// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent replay of a timed path ending at its first undefined read.

use super::*;
use fsl_eval::simulation::model::TimedState;
use fsl_foundation::bound::TimedInterval;
use fsl_semantics::check::Event;

pub(super) fn is_timed_key(key: &ProbabilityProductKey) -> bool {
    serde_json::from_slice::<serde_json::Value>(key.as_bytes())
        .ok()
        .and_then(|root| root.get(4)?.get(1)?.get(0)?.as_str().map(str::to_owned))
        .is_some_and(|tag| matches!(tag.as_str(), "digital" | "region" | "boundary-region"))
}

/// Replays a digital timed undefined path without consulting its probability
/// threshold. The path itself must have positive probability and end at the
/// exact pre-read state and clock valuation it reports.
pub(super) fn replay(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let evidence = &witness.path;
    let [path] = evidence.paths.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if limits.max_witness_paths == 0 {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
        });
    }
    let timed = TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ExactProbabilityGraphError::UnsupportedClaimForm)?;
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut storage = 0;
    let workload = claim_workload(system, claim)?;
    let binding = claim_binding(system, claim)?;
    if binding.as_ref().map(|binding| &binding.reference) != evidence.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let deterministic = deterministic_scheduler_rows(&evidence.scheduler)?;
    let scheduler = step::scheduler(&deterministic, limits, &mut meter, &mut storage, cancel)?;
    let initial = timed.initial();
    let [current] = initial.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let model_initials = system.initial();
    let [model_initial] = model_initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let monitor = initial_monitor(claim, witness)?;
    let elapsed_quantile = elapsed_quantile_predicates(claim);
    let mut elapsed_phase = elapsed_quantile.map(|_| String::from("inactive"));
    let mut measure_started_at = None;
    if let Some((from, until, _)) = elapsed_quantile {
        if let Some((from, until)) = elapsed_truths(
            system,
            current.discrete(),
            from,
            until,
            binding.as_ref().map(|binding| &binding.reference),
            cancel,
        )? {
            update_elapsed_phase(
                elapsed_phase.as_mut().ok_or(ProbabilityWitnessError::Refused)?,
                &mut measure_started_at,
                &from,
                &until,
                &rational(0),
            );
        }
    } else {
        measure_started_at = measure_phase(&evidence.initial_key)
            .filter(|phase| *phase != "inactive")
            .map(|_| rational(0));
    }
    if !same_model_state(system, model_initial, &evidence.initial)?
        || !same_model_state(system, current.discrete(), &evidence.initial)?
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    if claim.bounded_measure() && elapsed_quantile.is_none() {
        match replay_measure_control(system, claim, witness, limits, cancel)? {
            ProbabilityWitnessReplay::Reproduced(_) => {}
            replay => return Ok(replay),
        }
    }
    if !key_matches_timed_state(
        system,
        claim,
        &evidence.initial_key,
        current,
        elapsed_measure_monitor(elapsed_phase.as_deref()).or_else(|| monitor.clone()),
        rational(0),
        measure_started_at.as_ref(),
    )? {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let mut current = current.clone();
    let monitor_phase = String::from("pending");
    let mut mass = rational(1);
    let mut edges = 0;
    let mut elapsed_total = rational(0);
    let mut seen_product = BTreeSet::new();
    let mut used = BTreeSet::new();
    bounded::admit_state(
        &evidence.initial_key,
        &mut seen_product,
        &mut storage,
        limits,
    )?;
    for (position, supplied) in path.steps.iter().enumerate() {
        bounded::visit(&mut meter, cancel)?;
        let source_key = if position == 0 {
            &evidence.initial_key
        } else {
            &path.steps[position - 1].post_key
        };
        bounded::admit_state(source_key, &mut seen_product, &mut storage, limits)?;
        if workload.is_none() && scheduler.get(source_key).copied() != Some(&supplied.choice) {
            return Err(ProbabilityWitnessError::Refused);
        }
        if matches!(claim.scheduler, ProbScheduler::Every) && !used.insert(source_key.clone()) {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if !source_claim_undecided(
            system,
            claim,
            current.discrete(),
            witness,
            binding.as_ref().map(|binding| &binding.reference),
            &elapsed_total,
            cancel,
        )? {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        let delayed = timed
            .delay(&current, &supplied.elapsed)
            .map_err(|_| ProbabilityWitnessError::Refused)?
            .ok_or(ProbabilityWitnessError::Refused)?;
        elapsed_total = elapsed_total.add(&supplied.elapsed);
        if let (Some(phase), Some((_, _, window))) =
            (elapsed_phase.as_mut(), elapsed_quantile)
        {
            if phase == "active"
                && measure_started_at
                    .as_ref()
                    .is_some_and(|started| elapsed_total.sub(started) > *window)
            {
                *phase = String::from("censored");
            }
        }
        let mut untimed_step = supplied.clone();
        untimed_step.elapsed = rational(0);
        let (probability, post, ()) = step::replay(
            system,
            delayed.discrete(),
            workload,
            &untimed_step,
            limits,
            &mut meter,
            &mut storage,
            &mut edges,
            cancel,
            step::ActionRefusal::UnsupportedDraw,
            |_, _, _, _| Ok(Some(())),
        )?;
        if probability <= rational(0) || !same_model_state(system, &post, &supplied.post)? {
            return Err(ProbabilityWitnessError::Refused);
        }
        let transition = supplied
            .transition
            .as_ref()
            .ok_or(ProbabilityWitnessError::Refused)?;
        let candidates = timed
            .steps(&delayed)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        let mut reached = None;
        for (guarded, next) in candidates {
            if guarded.transition() == transition
                && same_model_state(system, guarded.post(), &post)?
            {
                if reached.is_some() {
                    return Err(ProbabilityWitnessError::Refused);
                }
                reached = Some(next);
            }
        }
        current = reached.ok_or(ProbabilityWitnessError::Refused)?;
        bounded::admit_state(&supplied.post_key, &mut seen_product, &mut storage, limits)?;
        if matches!(claim.scheduler, ProbScheduler::Every) && used.contains(&supplied.post_key) {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if let Some((from, until, _)) = elapsed_quantile {
            if let Some((from, until)) = elapsed_truths(
                system,
                current.discrete(),
                from,
                until,
                binding.as_ref().map(|binding| &binding.reference),
                cancel,
            )? {
                update_elapsed_phase(
                    elapsed_phase.as_mut().ok_or(ProbabilityWitnessError::Refused)?,
                    &mut measure_started_at,
                    &from,
                    &until,
                    &elapsed_total,
                );
            }
        }
        if !key_matches_timed_state(
            system,
            claim,
            &supplied.post_key,
            &current,
            elapsed_measure_monitor(elapsed_phase.as_deref()).or_else(|| monitor.clone()).map(|mut value| {
                if let Some(phase) = value.as_array_mut().and_then(|parts| parts.get_mut(2)) {
                    *phase = serde_json::Value::String(monitor_phase.clone());
                }
                value
            }),
            elapsed_total.clone(),
            measure_started_at.as_ref(),
        )? {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        mass = bounded::arithmetic(
            filament_exact::RationalArithmetic::Multiply(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
    }
    let expected_position = u64::try_from(path.steps.len()).unwrap_or(u64::MAX);
    let (raw, state_key) = system
        .state_key(current.discrete())
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let live_clocks = timed
        .places()
        .iter()
        .zip(current.clocks().values())
        .filter(|(place, _)| timed.live(place, current.discrete()))
        .map(|(_, clock)| clock.clone())
        .collect::<Vec<_>>();
    if expected_position != witness.evaluation.site.position
        || state_key != witness.evaluation.site.state_key
        || &witness.product_state
            != path
                .steps
                .last()
                .map(|step| &step.post_key)
                .unwrap_or(&evidence.initial_key)
        || !key_matches_timed_state(
            system,
            claim,
            &witness.product_state,
            &current,
            elapsed_measure_monitor(elapsed_phase.as_deref()).or_else(|| monitor.clone()).map(|mut value| {
                if let Some(phase) = value.as_array_mut().and_then(|parts| parts.get_mut(2)) {
                    *phase = serde_json::Value::String(monitor_phase.clone());
                }
                value
            }),
            elapsed_total.clone(),
            measure_started_at.as_ref(),
        )?
        || !undefined_read_matches(system, claim, current.discrete(), witness, cancel)?
        || !final_control_undecided(claim, witness, &elapsed_total, elapsed_phase.as_deref())?
        || witness
            .evaluation
            .site
            .time
            .as_ref()
            .is_none_or(|point| point.stamp != elapsed_total || point.clocks != live_clocks)
    {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    if scheduler.keys().any(|key| !used.contains(*key)) {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    bounded::retain(raw.len(), &mut storage, limits)?;
    check_bits(&mass, limits.max_rational_bits)?;
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence: evidence.clone(),
            mass,
            undefined: Some(Box::new(witness.clone())),
        },
    )))
}

fn final_control_undecided(
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
    elapsed: &Rational,
    measure_phase: Option<&str>,
) -> Result<bool, ProbabilityWitnessError> {
    if let Some((_, _, _, interval)) = timed_formula(claim, witness)? {
        return Ok(elapsed < interval.lower()
            || (elapsed == interval.lower() && !interval.lower_closed())
            || current_time(elapsed, interval));
    }
    if elapsed_quantile_predicates(claim).is_some() {
        return Ok(matches!(measure_phase, Some("inactive" | "active")));
    }
    Ok(true)
}

/// Recompute the monitor and accumulator with the checked measure replay,
/// replacing only the timed model envelope with the same discrete state key.
/// The timed replay below independently checks clocks, deadlines and elapsed
/// values at every supplied position.
fn replay_measure_control(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let mut untimed = witness.clone();
    untimed.path.initial_key =
        untimed_key(system, &witness.path.initial_key, &witness.path.initial)?;
    for (target, source) in untimed
        .path
        .paths
        .iter_mut()
        .flat_map(|path| path.steps.iter_mut())
        .zip(witness.path.paths.iter().flat_map(|path| path.steps.iter()))
    {
        target.elapsed = rational(0);
        target.post_key = untimed_key(system, &source.post_key, &source.post)?;
    }
    let final_state = witness
        .path
        .paths
        .first()
        .and_then(|path| path.steps.last())
        .map(|step| &step.post)
        .unwrap_or(&witness.path.initial);
    untimed.product_state = untimed_key(system, &witness.product_state, final_state)?;
    untimed.evaluation.site.time = None;
    super::measure::replay_undefined(system, claim, &untimed, limits, cancel)
}

fn untimed_key(
    system: &ModelSystem<'_>,
    timed_key: &ProbabilityProductKey,
    state: &ModelSimState,
) -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
    let mut root: serde_json::Value = serde_json::from_slice(timed_key.as_bytes())
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let (raw, _) = system
        .state_key(state)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    root[4] = serde_json::from_slice(&raw).map_err(|_| ProbabilityWitnessError::Refused)?;
    Ok(ProbabilityProductKey::from_canonical_bytes(
        serde_json::to_vec(&root).map_err(|_| ProbabilityWitnessError::Refused)?,
    ))
}

fn source_claim_undecided(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    state: &ModelSimState,
    witness: &ProbabilityUndefinedWitness,
    binding: Option<&filament_exact::ObjectReference>,
    elapsed: &Rational,
    cancel: &Cancel,
) -> Result<bool, ProbabilityWitnessError> {
    if claim.bounded_measure() && elapsed_quantile_predicates(claim).is_none() {
        // `measure::replay_undefined` independently replays the exact control
        // and accumulator for this pre-read state.
        return Ok(true);
    }
    if let Some((first, second, operator, interval)) = timed_formula(claim, witness)? {
        if elapsed < interval.lower() || (elapsed == interval.lower() && !interval.lower_closed()) {
            return Ok(true);
        }
        if !current_time(elapsed, interval) {
            return Ok(false);
        }
        return match operator {
            TimedOperator::Unary(operator) => {
                let holds = read_bool(
                    system,
                    state,
                    first.ok_or(ProbabilityWitnessError::Refused)?,
                    binding,
                    cancel,
                )?;
                Ok(match operator {
                    fsl_semantics::check::UnaryTemporal::Eventually => !holds,
                    fsl_semantics::check::UnaryTemporal::Always => holds,
                    _ => false,
                })
            }
            TimedOperator::Until => {
                let from = read_bool(
                    system,
                    state,
                    first.ok_or(ProbabilityWitnessError::Refused)?,
                    binding,
                    cancel,
                )?;
                let until = read_bool(
                    system,
                    state,
                    second.ok_or(ProbabilityWitnessError::Refused)?,
                    binding,
                    cancel,
                )?;
                Ok(from && !until)
            }
        };
    }
    Ok(match &claim.form {
        ProbForm::Reach {
            target: ReachTarget::Eventually(predicate),
            ..
        } => !read_bool(system, state, predicate, binding, cancel)?,
        ProbForm::Reach {
            target: ReachTarget::Always(predicate),
            ..
        } => read_bool(system, state, predicate, binding, cancel)?,
        ProbForm::Reach {
            target: ReachTarget::Until(from, until),
            ..
        } => {
            let holds_from = read_bool(system, state, from, binding, cancel)?;
            let holds_until = read_bool(system, state, until, binding, cancel)?;
            holds_from && !holds_until
        }
        ProbForm::ExpectedReward { until, .. } | ProbForm::MeanTime { until, .. } => {
            !read_bool(system, state, until, binding, cancel)?
        }
        ProbForm::LongRunFraction { predicate, .. } => {
            let _ = read_bool(system, state, predicate, binding, cancel)?;
            true
        }
        _ => false,
    })
}

fn read_bool(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    cancel: &Cancel,
) -> Result<bool, ProbabilityWitnessError> {
    let readings = system
        .state_predicates_with_cancel(state, &[(predicate, binding)], cancel)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    match readings.as_slice() {
        [PredicateReading::True] => Ok(true),
        [PredicateReading::False] => Ok(false),
        [PredicateReading::Undefined { .. }] => Err(ProbabilityWitnessError::Refused),
        _ => Err(ProbabilityWitnessError::Refused),
    }
}

fn initial_monitor(
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
) -> Result<Option<serde_json::Value>, ProbabilityWitnessError> {
    Ok(timed_formula(claim, witness)?.map(|(_, _, operator, _)| {
        serde_json::json!([
            "deadline",
            match operator {
                TimedOperator::Unary(fsl_semantics::check::UnaryTemporal::Eventually) =>
                    "eventually",
                TimedOperator::Unary(fsl_semantics::check::UnaryTemporal::Always) => "always",
                TimedOperator::Until => "until",
                _ => "unsupported",
            },
            "pending"
        ])
    }))
}

#[derive(Clone, Copy)]
enum TimedOperator {
    Unary(fsl_semantics::check::UnaryTemporal),
    Until,
}

fn timed_formula<'a>(
    claim: &'a CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
) -> Result<
    Option<(
        Option<&'a fsl_semantics::check::CheckedPredicate>,
        Option<&'a fsl_semantics::check::CheckedPredicate>,
        TimedOperator,
        &'a TimedInterval,
    )>,
    ProbabilityWitnessError,
> {
    let ProbForm::Probability {
        event:
            Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
        ..
    } = &claim.form
    else {
        return Ok(None);
    };
    let root = formula.formula().root();
    let Some(crate::UndefinedLocus::Atom(atom)) = Some(&witness.evaluation.site.locus) else {
        return Err(ProbabilityWitnessError::Refused);
    };
    match formula.formula().nodes().get(root.index()) {
        Some(fsl_semantics::check::FormulaNode::Unary {
            operator:
                operator @ (fsl_semantics::check::UnaryTemporal::Eventually
                | fsl_semantics::check::UnaryTemporal::Always),
            interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
            operand,
        }) if atom == operand
            && matches!(
                formula.formula().nodes().get(operand.index()),
                Some(fsl_semantics::check::FormulaNode::Holds { .. })
            ) =>
        {
            let predicate = formula
                .condition(operand.index())
                .ok_or(ProbabilityWitnessError::Refused)?;
            Ok(Some((
                Some(predicate),
                None,
                TimedOperator::Unary(*operator),
                interval,
            )))
        }
        Some(fsl_semantics::check::FormulaNode::Binary {
            operator: fsl_semantics::check::BinaryTemporal::Until,
            interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
            left,
            right,
        }) if (atom == left || atom == right)
            && matches!(
                formula.formula().nodes().get(left.index()),
                Some(fsl_semantics::check::FormulaNode::Holds { .. })
            )
            && matches!(
                formula.formula().nodes().get(right.index()),
                Some(fsl_semantics::check::FormulaNode::Holds { .. })
            ) =>
        {
            let from = formula
                .condition(left.index())
                .ok_or(ProbabilityWitnessError::Refused)?;
            let until = formula
                .condition(right.index())
                .ok_or(ProbabilityWitnessError::Refused)?;
            Ok(Some((
                Some(from),
                Some(until),
                TimedOperator::Until,
                interval,
            )))
        }
        _ => Err(ProbabilityWitnessError::Refused),
    }
}

fn current_time(elapsed: &Rational, interval: &TimedInterval) -> bool {
    elapsed >= interval.lower()
        && (interval.lower_closed() || elapsed > interval.lower())
        && interval
            .upper()
            .is_none_or(|upper| elapsed < upper || (interval.upper_closed() && elapsed == upper))
}

fn formula_timed_interval(claim: &CheckedProbabilisticClaim) -> Option<&TimedInterval> {
    let ProbForm::Probability {
        event:
            Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
        ..
    } = &claim.form
    else {
        return None;
    };
    let root = formula.formula().root();
    match formula.formula().nodes().get(root.index())? {
        fsl_semantics::check::FormulaNode::Unary {
            interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
            ..
        }
        | fsl_semantics::check::FormulaNode::Binary {
            interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
            ..
        } => Some(interval),
        _ => None,
    }
}

fn measure_phase(key: &ProbabilityProductKey) -> Option<String> {
    let root: serde_json::Value = serde_json::from_slice(key.as_bytes()).ok()?;
    let monitor = root.get(5)?.as_array()?;
    (monitor.first()?.as_str()? == "measure")
        .then(|| monitor.get(2)?.as_str().map(str::to_owned))
        .flatten()
}

fn elapsed_quantile_predicates(
    claim: &CheckedProbabilisticClaim,
) -> Option<(
    &fsl_semantics::check::CheckedPredicate,
    &fsl_semantics::check::CheckedPredicate,
    &Rational,
)> {
    match &claim.form {
        ProbForm::Quantile {
            measure:
                fsl_semantics::check::BoundedMeasure::Elapsed {
                    from, until, within, ..
                },
            ..
        } => Some((from.as_ref(), until.as_ref(), within)),
        _ => None,
    }
}

fn elapsed_measure_monitor(phase: Option<&str>) -> Option<serde_json::Value> {
    phase.map(|phase| serde_json::json!(["measure", null, phase]))
}

fn elapsed_truths(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    from: &fsl_semantics::check::CheckedPredicate,
    until: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    cancel: &Cancel,
) -> Result<Option<(bool, bool)>, ProbabilityWitnessError> {
    let readings = system
        .state_predicates_with_cancel(state, &[(from, binding), (until, binding)], cancel)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    match readings.as_slice() {
        [PredicateReading::True, PredicateReading::True] => Ok(Some((true, true))),
        [PredicateReading::True, PredicateReading::False] => Ok(Some((true, false))),
        [PredicateReading::False, PredicateReading::True] => Ok(Some((false, true))),
        [PredicateReading::False, PredicateReading::False] => Ok(Some((false, false))),
        [PredicateReading::Undefined { .. }, _] | [_, PredicateReading::Undefined { .. }] => {
            Ok(None)
        }
        [PredicateReading::Refused(_), _] | [_, PredicateReading::Refused(_)] => {
            Err(ProbabilityWitnessError::Refused)
        }
        _ => Err(ProbabilityWitnessError::Refused),
    }
}

fn update_elapsed_phase(
    phase: &mut String,
    started_at: &mut Option<Rational>,
    from: &bool,
    until: &bool,
    elapsed: &Rational,
) {
    match phase.as_str() {
        "inactive" if *from => {
            *started_at = Some(elapsed.clone());
            *phase = if *until { "complete" } else { "active" }.to_owned();
        }
        "active" if *until => *phase = String::from("complete"),
        _ => {}
    }
}

fn measure_window(claim: &CheckedProbabilisticClaim) -> Option<&Rational> {
    match &claim.form {
        ProbForm::Quantile {
            measure: fsl_semantics::check::BoundedMeasure::Elapsed { within, .. },
            ..
        } => Some(within),
        ProbForm::MeanOfFraction { fraction, .. } => fraction.timed_window.as_ref(),
        _ => None,
    }
}

fn scaled_integer(value: &Rational, scale: u64) -> Option<String> {
    let numerator = value.numerator().as_big() * scale;
    let denominator = value.denominator().as_big();
    if &numerator % denominator != num_bigint::BigInt::from(0) {
        return None;
    }
    Some((numerator / denominator).to_string())
}

fn same_model_state(
    system: &ModelSystem<'_>,
    left: &ModelSimState,
    right: &ModelSimState,
) -> Result<bool, ProbabilityWitnessError> {
    Ok(system
        .state_key(left)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
        .0
        == system
            .state_key(right)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
            .0)
}

fn key_matches_timed_state(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    key: &ProbabilityProductKey,
    state: &TimedState,
    monitor: Option<serde_json::Value>,
    elapsed: Rational,
    measure_started_at: Option<&Rational>,
) -> Result<bool, ProbabilityWitnessError> {
    let root: serde_json::Value =
        serde_json::from_slice(key.as_bytes()).map_err(|_| ProbabilityWitnessError::Refused)?;
    let (raw, _) = system
        .state_key(state.discrete())
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let discrete: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| ProbabilityWitnessError::Refused)?;
    let timed = TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let unit = serde_json::json!([
        "declared",
        fsl_foundation::node_wire::node_key_to_json(&timed.source().unit_node)
            .map_err(|_| ProbabilityWitnessError::Refused)?
    ]);
    let binding = claim_binding(system, claim)?
        .map(|binding| {
            let text = fsl_eval::simulation::model::reference_canonical_text(&binding.reference)
                .map_err(|_| ProbabilityWitnessError::Refused)?;
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|_| ProbabilityWitnessError::Refused)
        })
        .transpose()?
        .unwrap_or(serde_json::Value::Null);
    let elapsed_measure = elapsed_quantile_predicates(claim).is_some();
    if root.get(0).and_then(serde_json::Value::as_str)
        != Some("filament.probability-product-key/v1")
        || root.get(1) != Some(&binding)
        || root.get(2).is_none_or(|value| !value.is_null())
        || root.get(3).is_none_or(|value| !value.is_null())
        || root
            .get(4)
            .and_then(|model| model.get(1))
            .and_then(|timed| timed.get(1))
            != Some(&discrete)
        || root
            .get(4)
            .and_then(|model| model.get(1))
            .and_then(|timed| timed.get(2))
            != Some(&unit)
        || ((!claim.bounded_measure() || elapsed_measure)
            && root.get(5) != Some(&monitor.unwrap_or(serde_json::Value::Null)))
        || ((!claim.bounded_measure() || elapsed_measure)
            && (root.get(6).is_none_or(|value| !value.is_null())
                || root.get(7).is_none_or(|value| !value.is_null())))
        || root
            .get(4)
            .and_then(|model| model.get(1))
            .and_then(|timed| timed.get(6))
            .and_then(serde_json::Value::as_str)
            != Some("running")
    {
        return Ok(false);
    }
    if let Some(clocks) = crate::digital_key_clocks(system, claim, key) {
        if clocks.values() != state.clocks().values() {
            return Ok(false);
        }
    } else if !region_clocks_match(system, claim, state, &root)? {
        return Ok(false);
    }
    let model = root.get(4).and_then(|model| model.get(1));
    let deadline = model.and_then(|timed| timed.get(5));
    let Some(deadline) = deadline else {
        return Ok(false);
    };
    if deadline.is_null() {
        return Ok(measure_window(claim).is_none());
    }
    let scale = model
        .and_then(|timed| timed.get(3))
        .and_then(serde_json::Value::as_str)
        .and_then(|scale| scale.parse::<u64>().ok())
        .ok_or(ProbabilityWitnessError::Refused)?;
    if let Some(interval) = formula_timed_interval(claim) {
        let upper = interval.upper().ok_or(ProbabilityWitnessError::Refused)?;
        let Some(expected_limit) = scaled_integer(upper, scale) else {
            return Ok(false);
        };
        if deadline.get(1).and_then(serde_json::Value::as_str) != Some(expected_limit.as_str()) {
            return Ok(false);
        }
    }
    if deadline.get(0).and_then(serde_json::Value::as_str) == Some("elapsed") {
        let Some(window) = measure_window(claim) else {
            return Ok(false);
        };
        let Some(expected_window) = scaled_integer(window, scale) else {
            return Ok(false);
        };
        if deadline.get(1).and_then(serde_json::Value::as_str) != Some(expected_window.as_str()) {
            return Ok(false);
        }
        let phase = measure_phase(key).ok_or(ProbabilityWitnessError::Refused)?;
        let encoded = deadline.get(2).ok_or(ProbabilityWitnessError::Refused)?;
        if phase == "inactive" {
            return Ok(encoded.is_null());
        }
        let Some(started) = measure_started_at else {
            return Ok(false);
        };
        let Some(ticks) = scaled_integer(&elapsed.sub(started), scale) else {
            return Ok(false);
        };
        let cap = expected_window
            .parse::<u64>()
            .ok()
            .and_then(|window| window.checked_add(1))
            .ok_or(ProbabilityWitnessError::Refused)?;
        let expected = ticks.parse::<u64>().ok().map(|ticks| ticks.min(cap));
        return Ok(encoded.as_str().and_then(|value| value.parse::<u64>().ok()) == expected);
    }
    if model
        .and_then(|timed| timed.get(0))
        .and_then(serde_json::Value::as_str)
        == Some("region")
    {
        let (whole, fraction) = scaled_parts(&elapsed, scale)?;
        return Ok(fraction.is_zero()
            && deadline.get(0).and_then(serde_json::Value::as_str) == Some("deadline")
            && deadline.get(2).and_then(serde_json::Value::as_str)
                == Some(whole.to_string().as_str())
            && deadline.get(3).and_then(serde_json::Value::as_str) == Some("0"));
    }
    let scaled_numerator = elapsed.numerator().as_big() * scale;
    let denominator = elapsed.denominator().as_big();
    if &scaled_numerator % denominator != num_bigint::BigInt::from(0) {
        return Ok(false);
    }
    let ticks = (scaled_numerator / denominator).to_string();
    Ok(
        deadline.get(0).and_then(serde_json::Value::as_str) == Some("deadline")
            && deadline.get(2).and_then(serde_json::Value::as_str) == Some(ticks.as_str()),
    )
}

fn region_clocks_match(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    state: &TimedState,
    root: &serde_json::Value,
) -> Result<bool, ProbabilityWitnessError> {
    let Some(timed_key) = root.get(4).and_then(|model| model.get(1)) else {
        return Ok(false);
    };
    let boundary = match timed_key.get(0).and_then(serde_json::Value::as_str) {
        Some("region") => false,
        Some("boundary-region") => true,
        _ => return Ok(false),
    };
    let scale = timed_key
        .get(3)
        .and_then(serde_json::Value::as_str)
        .and_then(|scale| scale.parse::<u64>().ok())
        .ok_or(ProbabilityWitnessError::Refused)?;
    let entries = timed_key
        .get(4)
        .and_then(serde_json::Value::as_array)
        .ok_or(ProbabilityWitnessError::Refused)?;
    let timed = TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    if timed.places().len() != entries.len()
        || timed.places().len() != state.clocks().values().len()
    {
        return Ok(false);
    }
    let mut fractions = Vec::new();
    let corner = if boundary {
        Some(
            timed_key
                .get(7)
                .and_then(serde_json::Value::as_array)
                .ok_or(ProbabilityWitnessError::Refused)?,
        )
    } else {
        None
    };
    if corner.is_some_and(|corner| corner.len() != entries.len()) {
        return Ok(false);
    }
    let mut subdivisions = None;
    for (place, value) in timed.places().iter().zip(state.clocks().values()) {
        let field = timed
            .model()
            .clock(place.clock())
            .ok_or(ProbabilityWitnessError::Refused)?;
        let owner_ref = field.owner();
        let owner = owner_ref.original();
        let declaration = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .map_err(|_| ProbabilityWitnessError::Refused)?;
        let declaration = fsl_foundation::node_wire::node_key_to_json(&declaration)
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        let receiver = fsl_eval::simulation::model::reference_canonical_text(place.object())
            .map_err(|_| ProbabilityWitnessError::Refused)?;
        let receiver: serde_json::Value =
            serde_json::from_str(&receiver).map_err(|_| ProbabilityWitnessError::Refused)?;
        let identity = serde_json::json!([declaration, field.name(), receiver]);
        let Some(entry) = entries.iter().find(|entry| entry.get(0) == Some(&identity)) else {
            return Ok(false);
        };
        let value = scaled_parts(value, scale)?;
        let whole = entry
            .get(1)
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(ProbabilityWitnessError::Refused)?;
        let cap = entry
            .get(2)
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(ProbabilityWitnessError::Refused)?;
        let rank = entry
            .get(3)
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(ProbabilityWitnessError::Refused)?;
        if (value.0 != whole && !(value.0 >= cap && whole == cap))
            || (value.1.is_zero() != (rank == 0))
        {
            return Ok(false);
        }
        if let Some(corner) = corner {
            let coordinate = entries
                .iter()
                .position(|candidate| candidate.get(0) == Some(&identity))
                .and_then(|position| corner.get(position))
                .and_then(serde_json::Value::as_str)
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or(ProbabilityWitnessError::Refused)?;
            if value.1.is_zero() {
                let Some(scale_factor) = (whole != 0).then(|| coordinate / whole) else {
                    if coordinate != 0 {
                        return Ok(false);
                    }
                    continue;
                };
                if coordinate % whole != 0
                    || subdivisions.is_some_and(|existing| existing != scale_factor)
                {
                    return Ok(false);
                }
                subdivisions = Some(scale_factor);
            } else {
                let scaled = value.1.add(&Rational::from_integer(whole.into()));
                let numerator =
                    num_bigint::BigInt::from(coordinate) * scaled.denominator().as_big();
                let denominator = scaled.numerator().as_big();
                if denominator == &num_bigint::BigInt::from(0)
                    || &numerator % denominator != num_bigint::BigInt::from(0)
                {
                    return Ok(false);
                }
                let factor = (numerator / denominator)
                    .to_string()
                    .parse::<u64>()
                    .map_err(|_| ProbabilityWitnessError::Refused)?;
                if subdivisions.is_some_and(|existing| existing != factor) {
                    return Ok(false);
                }
                subdivisions = Some(factor);
            }
        }
        if rank > 0 {
            fractions.push((value.1, rank));
        }
    }
    for (left, left_rank) in &fractions {
        for (right, right_rank) in &fractions {
            if (left < right && left_rank >= right_rank)
                || (left == right && left_rank != right_rank)
                || (left > right && left_rank <= right_rank)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn scaled_parts(value: &Rational, scale: u64) -> Result<(u64, Rational), ProbabilityWitnessError> {
    let numerator = value.numerator().as_big() * scale;
    let denominator = value.denominator().as_big();
    let whole = (&numerator / denominator)
        .to_string()
        .parse::<u64>()
        .map_err(|_| ProbabilityWitnessError::Refused)?;
    let remainder = &numerator % denominator;
    let fraction = Rational::new(
        filament_exact::Integer::from_big(remainder),
        filament_exact::Integer::from_big(denominator.clone()),
    )
    .map_err(|_| ProbabilityWitnessError::Refused)?;
    Ok((whole, fraction))
}

fn undefined_read_matches(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    state: &ModelSimState,
    witness: &ProbabilityUndefinedWitness,
    cancel: &Cancel,
) -> Result<bool, ProbabilityWitnessError> {
    if claim.bounded_measure() && elapsed_quantile_predicates(claim).is_none() {
        // The measure replay above verifies the exact undefined locus, cause,
        // and pre-read control without reading outside-window predicates.
        return Ok(true);
    }
    let predicate = match (&claim.form, &witness.evaluation.site.locus) {
        (
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            },
            crate::UndefinedLocus::Atom(atom),
        ) => formula.condition(atom.index()),
        (
            ProbForm::Reach {
                target: ReachTarget::Eventually(predicate) | ReachTarget::Always(predicate),
                ..
            },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Target),
        ) => Some(predicate),
        (
            ProbForm::Reach {
                target: ReachTarget::Until(from, _),
                ..
            },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::From),
        ) => Some(from),
        (
            ProbForm::Reach {
                target: ReachTarget::Until(_, until),
                ..
            },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
        ) => Some(until),
        (
            ProbForm::LongRunFraction { predicate, .. },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Target),
        ) => Some(predicate),
        (
            ProbForm::ExpectedReward { until, .. } | ProbForm::MeanTime { until, .. },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
        ) => Some(until),
        (
            ProbForm::Quantile {
                measure: fsl_semantics::check::BoundedMeasure::Elapsed { from, .. },
                ..
            },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::From),
        ) => Some(from.as_ref()),
        (
            ProbForm::Quantile {
                measure: fsl_semantics::check::BoundedMeasure::Elapsed { until, .. },
                ..
            },
            crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
        ) => Some(until.as_ref()),
        _ => None,
    }
    .ok_or(ProbabilityWitnessError::Refused)?;
    let readings = system
        .state_predicates_with_cancel(
            state,
            &[(
                predicate,
                claim_binding(system, claim)?
                    .as_ref()
                    .map(|binding| &binding.reference),
            )],
            cancel,
        )
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    Ok(
        matches!(readings.as_slice(), [PredicateReading::Undefined { cause, .. }] if cause == &witness.evaluation.cause),
    )
}
