// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": which abstract transitions a timed abstract
//! subject enables at the instants of a concrete lasso's loop.
//!
//! The loop is read as repeated forever. A clock the loop resets takes the
//! value its last reset gives it, cyclically, plus the delays since; a clock
//! the loop never resets is past every constant of the model (FR-236 "Timed
//! lasso"), so a lower bound on it holds and an upper bound fails. The delay
//! before each loop step is split at the instants where a guard constant is
//! crossed into points and open stretches, and each segment names the steps
//! each hidden candidate enables there (FR-232: a convex part of the guard
//! holds and the target satisfies its time invariants). This is the
//! checker's own reading: the product in `fsl-analyze` splits its cells by
//! the same guards with its own code.

use std::collections::BTreeSet;

use filament_exact::{Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{GuardedStep, ModelSystem, PlaceConstraint, TimedSystem};
use fsl_semantics::check::ClockOp;
use fsl_semantics::model::state::ModelState;

use crate::refinement::clock_rows::{row_clock_step, RowClockStep};
use crate::refinement::failure::Taken;

/// The transitions enabled at one segment of a delay: each with the key of
/// the candidate state that enables it.
pub type SegmentEnabled = Vec<(Vec<u8>, ModelTransition)>;

/// A clock's value at the start of a loop step; `None` is past every
/// constant.
type Value = Option<Rational>;

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// Whether `place op bound` holds `after` time units past `value`.
fn holds(op: ClockOp, value: &Value, after: &Rational, bound: &Rational) -> bool {
    match value {
        None => matches!(op, ClockOp::Greater | ClockOp::GreaterOrEqual),
        Some(value) => {
            let now = value.add(after);
            match op {
                ClockOp::Less => now < *bound,
                ClockOp::LessOrEqual => now <= *bound,
                ClockOp::Equal => now == *bound,
                ClockOp::GreaterOrEqual => now >= *bound,
                ClockOp::Greater => now > *bound,
            }
        }
    }
}

/// Each convex part of `step`'s guard with the target's time invariants on
/// the clocks the step does not reset; none when a reset value breaks a
/// target bound.
fn enabling_parts(
    timed: &TimedSystem<'_, '_>,
    step: &GuardedStep,
) -> Result<Vec<Vec<PlaceConstraint>>, &'static str> {
    let target = timed
        .invariants(step.post())
        .map_err(|_| "timed-abstract-invariant")?;
    let mut pulled_back = Vec::with_capacity(target.len());
    for bound in target {
        if let Some((_, value)) = step
            .resets()
            .iter()
            .find(|(place, _)| *place == bound.place())
        {
            if !bound.op().holds(value, bound.bound()) {
                return Ok(Vec::new());
            }
            continue;
        }
        let place = step
            .carries()
            .iter()
            .find(|(target, _)| *target == bound.place())
            .map_or(bound.place(), |(_, source)| *source);
        pulled_back.push(PlaceConstraint::new(
            place,
            bound.op(),
            bound.bound().clone(),
        ));
    }
    Ok(step
        .parts()
        .iter()
        .map(|part| part.iter().chain(&pulled_back).cloned().collect())
        .collect())
}

/// One candidate step of a loop position with its enabling parts.
struct Enabling {
    key: Vec<u8>,
    transition: ModelTransition,
    parts: Vec<Vec<PlaceConstraint>>,
}

/// The enabling parts of every step of every candidate.
fn enablings(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[ModelState],
) -> Result<Vec<Enabling>, &'static str> {
    let mut held = Vec::new();
    for candidate in candidates {
        let key = abstract_system
            .state_key_of(candidate)
            .map_err(|_| "timed-abstract-state-key")?
            .0;
        let state = abstract_system.pre_state(candidate.clone(), abstract_system.declared_anchor());
        for step in timed
            .guarded_steps(&state)
            .map_err(|_| "timed-abstract-guard")?
        {
            held.push(Enabling {
                key: key.clone(),
                transition: step.transition().clone(),
                parts: enabling_parts(timed, &step)?,
            });
        }
    }
    Ok(held)
}

/// The clock step the loop step `taken` asks of the abstract clocks, from
/// the candidates it leaves: `None` for no change, `Some(Err(()))` when one
/// valuation cannot follow it.
fn clock_step(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[ModelState],
    taken: &Taken,
) -> Result<Result<Option<GuardedStep>, ()>, &'static str> {
    match taken {
        Taken::Stutter => Ok(Ok(None)),
        Taken::Abstract(transitions) => {
            for candidate in candidates {
                let state =
                    abstract_system.pre_state(candidate.clone(), abstract_system.declared_anchor());
                for step in timed
                    .guarded_steps(&state)
                    .map_err(|_| "timed-abstract-guard")?
                {
                    if transitions.contains(step.transition()) {
                        return Ok(Ok(Some(step)));
                    }
                }
            }
            Ok(Err(()))
        }
        Taken::CandidateSteps(rows) => {
            match row_clock_step(abstract_system, timed, candidates, rows)
                .map_err(|_| "timed-abstract-rows")?
            {
                RowClockStep::Stutter => Ok(Ok(None)),
                RowClockStep::Agreed(step) => Ok(Ok(Some(*step))),
                RowClockStep::Disagree => Ok(Err(())),
            }
        }
        Taken::Protocol(_) | Taken::ProtocolTraces(_) => Ok(Err(())),
    }
}

/// The abstract clocks after `delay` and then `step`.
fn advance(values: &[Value], delay: Option<&Rational>, step: Option<&GuardedStep>) -> Vec<Value> {
    let mut next: Vec<Value> = values
        .iter()
        .map(|value| match (value, delay) {
            (Some(value), Some(delay)) => Some(value.add(delay)),
            (value, _) => value.clone(),
        })
        .collect();
    if let Some(step) = step {
        let before = next.clone();
        for (to, from) in step.carries() {
            if let (Some(slot), Some(source)) = (next.get_mut(*to), before.get(*from)) {
                slot.clone_from(source);
            }
        }
        for (place, value) in step.resets() {
            if let Some(slot) = next.get_mut(*place) {
                *slot = Some(value.clone());
            }
        }
    }
    next
}

/// For each loop step, the segments of its delay with the transitions each
/// enables. `candidates[j]` are the abstract candidates the step leaves,
/// `taken[j]` what it takes and `delays[j]` the delay before it (`None` for
/// an idle tail that waits forever). `None` when one abstract clock
/// valuation cannot follow the loop.
///
/// # Errors
///
/// A fault label when the abstract subject cannot be read.
pub fn loop_segments(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[&[ModelState]],
    taken: &[Taken],
    delays: &[Option<Rational>],
) -> Result<Option<Vec<Vec<SegmentEnabled>>>, &'static str> {
    if candidates.len() != taken.len() || taken.len() != delays.len() {
        return Err("timed-abstract-loop-shape");
    }
    let places = timed.places().len();
    let mut steps = Vec::with_capacity(taken.len());
    for (candidates, taken) in candidates.iter().zip(taken) {
        match clock_step(abstract_system, timed, candidates, taken)? {
            Ok(step) => steps.push(step),
            Err(()) => return Ok(None),
        }
    }
    // The clocks at each step's start, once the loop's valuation repeats.
    let mut start: Vec<Value> = vec![None; places];
    let mut starts: Vec<Vec<Value>> = Vec::new();
    let mut settled = false;
    for _ in 0..=places + 1 {
        let mut current = start.clone();
        starts.clear();
        for (step, delay) in steps.iter().zip(delays) {
            starts.push(current.clone());
            current = advance(&current, delay.as_ref(), step.as_ref());
        }
        if current == start {
            settled = true;
            break;
        }
        start = current;
    }
    if !settled {
        return Ok(None);
    }
    let two = Rational::from_integer(Integer::one().add(&Integer::one()));
    let mut loop_segments = Vec::with_capacity(steps.len());
    for (index, candidates) in candidates.iter().enumerate() {
        let held = enablings(abstract_system, timed, candidates)?;
        let values = &starts[index];
        let span = delays[index].clone();
        // The instants of the delay where a guard constant is crossed.
        let mut crossings = BTreeSet::new();
        crossings.insert(zero());
        if let Some(span) = &span {
            for constraint in held.iter().flat_map(|e| e.parts.iter().flatten()) {
                if let Some(Some(value)) = values.get(constraint.place()) {
                    let at = constraint.bound().sub(value);
                    if at > zero() && at < *span {
                        crossings.insert(at);
                    }
                }
            }
            crossings.insert(span.clone());
        }
        let points: Vec<Rational> = crossings.into_iter().collect();
        let mut instants = Vec::new();
        for (position, point) in points.iter().enumerate() {
            instants.push(point.clone());
            if let Some(next) = points.get(position + 1) {
                instants.push(point.add(next).div(&two).ok_or("timed-abstract-midpoint")?);
            }
        }
        let mut segments = Vec::with_capacity(instants.len());
        for instant in &instants {
            let mut enabled: SegmentEnabled = Vec::new();
            for entry in &held {
                let on = entry.parts.iter().any(|part| {
                    part.iter().all(|constraint| {
                        values.get(constraint.place()).is_some_and(|value| {
                            holds(constraint.op(), value, instant, constraint.bound())
                        })
                    })
                });
                let pair = (entry.key.clone(), entry.transition.clone());
                if on && !enabled.contains(&pair) {
                    enabled.push(pair);
                }
            }
            segments.push(enabled);
        }
        loop_segments.push(segments);
    }
    Ok(Some(loop_segments))
}
