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

/// One segment of a loop step's delay: what the alive candidates enable and
/// which candidates are alive. A candidate is alive at an instant while its
/// own time invariant holds there (FR-900 "Delay"); a dead one has no run
/// along the loop from then on.
#[derive(Clone, Debug)]
pub struct Segment {
    /// The transitions the alive candidates enable.
    pub enabled: SegmentEnabled,
    /// The state keys of the candidates whose invariant holds.
    pub alive: Vec<Vec<u8>>,
}

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

/// Each candidate's state key with its own time invariants.
fn candidate_invariants(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[ModelState],
) -> Result<Vec<(Vec<u8>, Vec<PlaceConstraint>)>, &'static str> {
    let mut held = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let key = abstract_system
            .state_key_of(candidate)
            .map_err(|_| "timed-abstract-state-key")?
            .0;
        let state = abstract_system.pre_state(candidate.clone(), abstract_system.declared_anchor());
        let own = timed
            .invariants(&state)
            .map_err(|_| "timed-abstract-invariant")?;
        held.push((key, own));
    }
    Ok(held)
}

/// Whether every invariant of `own` holds `instant` past `values`.
fn own_holds(own: &[PlaceConstraint], values: &[Value], instant: &Rational) -> bool {
    own.iter().all(|constraint| {
        values
            .get(constraint.place())
            .is_some_and(|value| holds(constraint.op(), value, instant, constraint.bound()))
    })
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

/// A loop step's clock operation: carries (destination, source), simultaneous
/// from the clocks before it, then resets.
#[derive(Clone, Debug, Default)]
struct ClockMove {
    carries: Vec<(usize, usize)>,
    resets: Vec<(usize, Rational)>,
}

impl ClockMove {
    fn of(step: Option<&GuardedStep>) -> Self {
        step.map_or_else(Self::default, |step| Self {
            carries: step.carries().to_vec(),
            resets: step.resets().to_vec(),
        })
    }
}

/// The abstract clocks after `delay` and then `clock_move`.
fn advance(values: &[Value], delay: Option<&Rational>, clock_move: &ClockMove) -> Vec<Value> {
    let mut next: Vec<Value> = values
        .iter()
        .map(|value| match (value, delay) {
            (Some(value), Some(delay)) => Some(value.add(delay)),
            (value, _) => value.clone(),
        })
        .collect();
    let before = next.clone();
    for (to, from) in &clock_move.carries {
        if let (Some(slot), Some(source)) = (next.get_mut(*to), before.get(*from)) {
            slot.clone_from(source);
        }
    }
    for (place, value) in &clock_move.resets {
        if let Some(slot) = next.get_mut(*place) {
            *slot = Some(value.clone());
        }
    }
    next
}

/// The clocks at each loop step's start once the loop's valuation repeats,
/// found in at most `passes` passes from every clock past every constant;
/// `None` when it does not repeat in them.
fn settled_starts(
    moves: &[ClockMove],
    delays: &[Option<Rational>],
    places: usize,
    passes: usize,
) -> Option<Vec<Vec<Value>>> {
    let mut start: Vec<Value> = vec![None; places];
    let mut starts: Vec<Vec<Value>> = Vec::new();
    for _ in 0..passes {
        let mut current = start.clone();
        starts.clear();
        for (clock_move, delay) in moves.iter().zip(delays) {
            starts.push(current.clone());
            current = advance(&current, delay.as_ref(), clock_move);
        }
        if current == start {
            return Some(starts);
        }
        start = current;
    }
    None
}

/// The instants of a delay of `span` (`None`: forever) at which an enabling
/// part of `parts` changes truth, with the points between them: the delay's
/// start, each guard constant crossed strictly inside it, its end, and the
/// midpoints of consecutive ones.
fn instants_of(
    parts: &[Vec<PlaceConstraint>],
    values: &[Value],
    span: Option<&Rational>,
) -> Result<Vec<Rational>, &'static str> {
    let two = Rational::from_integer(Integer::one().add(&Integer::one()));
    let mut crossings = BTreeSet::new();
    crossings.insert(zero());
    if let Some(span) = span {
        for constraint in parts.iter().flatten() {
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
    Ok(instants)
}

/// Whether some part of `parts` holds `instant` past `values`.
fn part_holds(parts: &[Vec<PlaceConstraint>], values: &[Value], instant: &Rational) -> bool {
    parts.iter().any(|part| {
        part.iter().all(|constraint| {
            values
                .get(constraint.place())
                .is_some_and(|value| holds(constraint.op(), value, instant, constraint.bound()))
        })
    })
}

/// For each loop step, the segments of its delay with the transitions each
/// enables. `candidates[j]` are the abstract candidates the step leaves,
/// `taken[j]` what it takes and `delays[j]` the delay before it (`None` for
/// an idle tail that waits forever). `None` when one abstract clock
/// valuation cannot follow the loop.
///
/// # Errors
///
/// A fault label when the abstract subject cannot be read or the inputs
/// disagree in length.
pub fn loop_segments(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[&[ModelState]],
    taken: &[Taken],
    delays: &[Option<Rational>],
) -> Result<Option<Vec<Vec<Segment>>>, &'static str> {
    if candidates.len() != taken.len() || taken.len() != delays.len() {
        return Err("timed-abstract-loop-shape");
    }
    let places = timed.places().len();
    let mut moves = Vec::with_capacity(taken.len());
    for (candidates, taken) in candidates.iter().zip(taken) {
        match clock_step(abstract_system, timed, candidates, taken)? {
            Ok(step) => moves.push(ClockMove::of(step.as_ref())),
            Err(()) => return Ok(None),
        }
    }
    let Some(starts) = settled_starts(&moves, delays, places, places + 2) else {
        return Ok(None);
    };
    let mut loop_segments = Vec::with_capacity(moves.len());
    for (index, candidates) in candidates.iter().enumerate() {
        let held = enablings(abstract_system, timed, candidates)?;
        let values = &starts[index];
        let invariants = candidate_invariants(abstract_system, timed, candidates)?;
        let mut all_parts: Vec<Vec<PlaceConstraint>> = held
            .iter()
            .flat_map(|entry| entry.parts.iter().cloned())
            .collect();
        all_parts.extend(invariants.iter().map(|(_, own)| own.clone()));
        // A terminal stutter waits past every constant.
        let ageing: Vec<Value> = if delays[index].is_none() {
            vec![None; places]
        } else {
            values.clone()
        };
        let instants = instants_of(&all_parts, values, delays[index].as_ref())?;
        let mut segments = Vec::with_capacity(instants.len());
        for instant in &instants {
            let alive: Vec<Vec<u8>> = invariants
                .iter()
                .filter(|(_, own)| own_holds(own, &ageing, instant))
                .map(|(key, _)| key.clone())
                .collect();
            let mut enabled: SegmentEnabled = Vec::new();
            for entry in held.iter().filter(|entry| alive.contains(&entry.key)) {
                let pair = (entry.key.clone(), entry.transition.clone());
                if part_holds(&entry.parts, values, instant) && !enabled.contains(&pair) {
                    enabled.push(pair);
                }
            }
            segments.push(Segment { enabled, alive });
        }
        loop_segments.push(segments);
    }
    Ok(Some(loop_segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        let numerator = if numerator >= 0 {
            (0..numerator).fold(Integer::zero(), |sum, _| sum.add(&Integer::one()))
        } else {
            (0..-numerator).fold(Integer::zero(), |sum, _| sum.sub(&Integer::one()))
        };
        let denominator = (0..denominator).fold(Integer::zero(), |sum, _| sum.add(&Integer::one()));
        Rational::new(numerator, denominator).expect("a nonzero denominator")
    }

    fn whole(value: i64) -> Rational {
        rational(value, 1)
    }

    fn part(place: usize, op: ClockOp, bound: i64) -> Vec<PlaceConstraint> {
        vec![PlaceConstraint::new(place, op, whole(bound))]
    }

    /// FR-899 "Timed subjects": a clock the loop never resets is past every
    /// constant, so a lower bound on it holds and an upper bound or an
    /// equality fails.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn an_unreset_clock_holds_each_lower_bound_and_fails_each_upper_bound() {
        let after = whole(5);
        let bound = whole(2);
        for (op, expected) in [
            (ClockOp::Greater, true),
            (ClockOp::GreaterOrEqual, true),
            (ClockOp::Less, false),
            (ClockOp::LessOrEqual, false),
            (ClockOp::Equal, false),
        ] {
            assert_eq!(holds(op, &None, &after, &bound), expected, "{op:?}");
        }
    }

    /// A finite clock compares by its value past the delay.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn a_finite_clock_compares_by_its_value_after_the_delay() {
        let value = Some(whole(1));
        assert!(holds(ClockOp::Equal, &value, &whole(1), &whole(2)));
        assert!(!holds(ClockOp::Equal, &value, &whole(2), &whole(2)));
        assert!(holds(ClockOp::LessOrEqual, &value, &whole(1), &whole(2)));
        assert!(!holds(ClockOp::Less, &value, &whole(1), &whole(2)));
        assert!(holds(ClockOp::GreaterOrEqual, &value, &whole(1), &whole(2)));
        assert!(!holds(ClockOp::Greater, &value, &whole(1), &whole(2)));
    }

    /// An equality guard is enabled only at the point where the clock
    /// crosses its constant, and that point is an instant of the delay.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn an_equality_guard_is_enabled_only_at_its_crossing_point() {
        let parts = vec![part(0, ClockOp::Equal, 1)];
        let values = vec![Some(whole(0))];
        let span = whole(2);
        let instants = instants_of(&parts, &values, Some(&span)).expect("instants");
        let enabled: Vec<&Rational> = instants
            .iter()
            .filter(|instant| part_holds(&parts, &values, instant))
            .collect();
        assert_eq!(enabled, [&whole(1)]);
        assert!(instants.len() > 3, "points and stretches: {instants:?}");
    }

    /// A crossing exactly at the end of the delay is the delay's end, not a
    /// crossing inside it, and the end is enabled.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn a_crossing_at_the_end_of_the_delay_is_the_end_instant() {
        let parts = vec![part(0, ClockOp::GreaterOrEqual, 2)];
        let values = vec![Some(whole(0))];
        let span = whole(2);
        let instants = instants_of(&parts, &values, Some(&span)).expect("instants");
        assert_eq!(instants.first(), Some(&whole(0)));
        assert_eq!(instants.last(), Some(&whole(2)));
        let enabled: Vec<&Rational> = instants
            .iter()
            .filter(|instant| part_holds(&parts, &values, instant))
            .collect();
        assert_eq!(enabled, [&whole(2)]);
    }

    /// A terminal stutter waits forever: one instant, read with every clock
    /// past every constant.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn a_terminal_stutter_reads_one_instant_past_every_constant() {
        let parts = vec![part(0, ClockOp::GreaterOrEqual, 7)];
        let values: Vec<Value> = vec![None];
        let instants = instants_of(&parts, &values, None).expect("instants");
        assert_eq!(instants, [whole(0)]);
        assert!(part_holds(&parts, &values, &instants[0]));
        let upper = vec![part(0, ClockOp::LessOrEqual, 7)];
        assert!(!part_holds(&upper, &values, &instants[0]));
    }

    /// A carry from an unreset clock leaves the destination past every
    /// constant; a reset makes a finite value that grows with the delay.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn a_carry_from_an_unreset_clock_stays_past_every_constant() {
        let moved = ClockMove {
            carries: vec![(1, 0)],
            resets: Vec::new(),
        };
        let after = advance(&[None, Some(whole(1))], Some(&whole(1)), &moved);
        assert_eq!(after, [None, None]);
        let reset = ClockMove {
            carries: Vec::new(),
            resets: vec![(0, whole(0))],
        };
        let after = advance(&[None, None], Some(&whole(3)), &reset);
        assert_eq!(after, [Some(whole(0)), None]);
        let after = advance(&after, Some(&whole(3)), &ClockMove::default());
        assert_eq!(after, [Some(whole(3)), None]);
    }

    /// A loop that resets its clock settles on the reset value; one that
    /// needs more passes than allowed settles on none.
    #[trace("TC-2536", "FR-899")]
    #[test]
    fn a_valuation_that_does_not_settle_in_the_passes_is_none() {
        let reset = ClockMove {
            carries: Vec::new(),
            resets: vec![(0, whole(0))],
        };
        let chain = ClockMove {
            carries: vec![(1, 0)],
            resets: vec![(0, whole(0))],
        };
        let delays = [Some(whole(1))];
        let starts = settled_starts(&[reset.clone()], &delays, 1, 3).expect("settles");
        assert_eq!(starts, [[Some(whole(0))]]);
        // The carry chain needs a pass to fill the second clock.
        assert!(settled_starts(&[chain.clone()], &delays, 2, 1).is_none());
        assert!(settled_starts(&[chain], &delays, 2, 4).is_some());
    }
}
