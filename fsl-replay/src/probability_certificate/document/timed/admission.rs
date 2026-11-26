// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921-AC-5, ADR-038 RL-7: the route a timed certificate may claim.
//!
//! The checker establishes every condition of the route admission table with
//! its own code, and its own time-lock search over the region graph of the
//! model, then looks the row up in the one declared table
//! ([`fsl_eval::simulation::model::en5_admission`]). The producer's reading
//! and its zone search are not read, and the document's route member is
//! never authoritative.

use std::collections::{BTreeMap, VecDeque};

use fsl_eval::simulation::model::{
    en5_admission, Admission, ClaimForm, DelayClass, PredicateReading, ReadFrom, RegionCause,
    RouteConditions,
};
use fsl_semantics::check::{
    BinaryTemporal, BoundedMeasure, CheckedInterval, CheckedTemporalFormula, ClaimActivation,
    DelayDistribution, Event, FormulaNode, FractionMeasure, ProbForm, ReachTarget, UnaryTemporal,
    Weighting,
};

use super::*;

/// The route a certificate for `claim` must name: `region` for a strict
/// constraint, interval or `M < c`, and `digital` where the table admits the
/// claim.
///
/// # Errors
///
/// `ObjectiveMismatch` when the table admits no certificate for the claim,
/// `Unsupported` for a continuous delay, and the time-lock search's limit,
/// cancellation or reading failure.
#[implements("FR-921-AC-5", "FSpec-FR-421-AC-8")]
pub(super) fn certificate_route(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<TimedProbabilityRoute, ProbabilityDocumentError> {
    let (form, takes_event_maximum) = claimed_form(claim);
    let conditions = RouteConditions {
        closed: !super::region_route(timed, claim),
        delays: delays_of(system, claim)?,
        fairness: !claim.fairness.is_empty(),
        form,
        read_from: position_of(system, timed, claim, cancel)?,
        takes_event_maximum,
    };
    match en5_admission(conditions) {
        Admission::Digital => Ok(TimedProbabilityRoute::Digital),
        Admission::Region(RegionCause::Strict) => Ok(TimedProbabilityRoute::Region),
        Admission::DigitalIfNoTimeLock if !time_lock_reachable(system, timed, limits, cancel)? => {
            Ok(TimedProbabilityRoute::Digital)
        }
        Admission::DigitalIfNoTimeLock | Admission::Region(_) => {
            Err(ProbabilityDocumentError::ObjectiveMismatch)
        }
    }
}

fn delays_of(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
) -> Result<DelayClass, ProbabilityDocumentError> {
    let declared: Vec<_> = system
        .input()
        .probabilistic
        .delays()
        .iter()
        .filter(|delay| delay.operation().model() == claim.subject.alias.as_str())
        .collect();
    if declared
        .iter()
        .any(|delay| !matches!(delay.distribution(), DelayDistribution::Discrete(_)))
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let (mut given, mut free) = (false, false);
    for (declaring, name, _) in system.operations() {
        let has_delay = declared.iter().any(|delay| {
            delay.operation().declaring() == declaring && delay.operation().name() == name
        });
        given |= has_delay;
        free |= !has_delay;
    }
    Ok(match (given, free) {
        (true, true) => DelayClass::Mixed,
        (true, false) => DelayClass::Given,
        (false, _) => DelayClass::Free,
    })
}

/// The position the claim is read from: a later one for `on each`, and for an
/// elapsed measure whose `from` is false at some initial state.
fn position_of(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    cancel: &Cancel,
) -> Result<ReadFrom, ProbabilityDocumentError> {
    if !matches!(claim.activation, ClaimActivation::Origin) {
        return Ok(ReadFrom::Later);
    }
    let measure = match &claim.form {
        ProbForm::Quantile { measure, .. }
        | ProbForm::Probability {
            event: Event::Compare { measure, .. },
            ..
        } => measure,
        _ => return Ok(ReadFrom::Origin),
    };
    let BoundedMeasure::Elapsed { from, .. } = measure else {
        return Ok(ReadFrom::Origin);
    };
    let binding = claim_binding(system, claim)?.map(|binding| binding.reference);
    for initial in timed.initial() {
        poll(cancel)?;
        let readings = system
            .state_predicates_with_cancel(
                initial.discrete(),
                &[(&**from, binding.as_ref())],
                cancel,
            )
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        if readings.contains(&PredicateReading::False) {
            return Ok(ReadFrom::Later);
        }
    }
    Ok(ReadFrom::Origin)
}

/// The claim's form, and whether its bound reads the maximum probability of a
/// reach, until or deadline event: `<=` on `eventually` or `until`, `>=` on
/// `always`.
fn claimed_form(claim: &CheckedProbabilisticClaim) -> (ClaimForm, bool) {
    let upper = |bound: &Bound| matches!(bound, Bound::AtMost(_));
    let form = match &claim.form {
        ProbForm::Reach { bound, target } => {
            let reads_maximum = match target {
                ReachTarget::Always(_) => !upper(bound),
                ReachTarget::Eventually(_) | ReachTarget::Until(..) => upper(bound),
            };
            return (ClaimForm::Reachability, reads_maximum);
        }
        ProbForm::Probability {
            bound,
            event: Event::Formula { formula, .. },
        } => {
            return match single_event(formula) {
                Some((form, SingleEvent::Always)) => (form, !upper(bound)),
                Some((form, SingleEvent::Eventual)) => (form, upper(bound)),
                None if steps_only(formula) => (ClaimForm::DiscreteStepMeasure, false),
                None => (ClaimForm::OtherFormula, false),
            };
        }
        ProbForm::ExpectedReward { .. } => ClaimForm::ExpectedAccumulate,
        ProbForm::MeanTime { .. } => ClaimForm::ExpectedElapsed,
        ProbForm::LongRunFraction {
            weight: Some(Weighting::Delay),
            ..
        } => ClaimForm::DelayWeightedFraction,
        ProbForm::LongRunFraction { .. } => ClaimForm::DiscreteStepMeasure,
        ProbForm::MeanOfFraction { fraction, .. } => windowed_fraction(fraction),
        ProbForm::Quantile {
            measure, threshold, ..
        } => compared_measure(measure, threshold),
        ProbForm::Probability {
            event: Event::Compare { measure, bound },
            ..
        } => compared_measure(measure, bound),
    };
    (form, false)
}

fn compared_measure(measure: &BoundedMeasure, compared: &Bound) -> ClaimForm {
    match measure {
        BoundedMeasure::Elapsed { .. } if matches!(compared, Bound::AtMost(_)) => {
            ClaimForm::ElapsedAtMost
        }
        BoundedMeasure::Elapsed { .. } => ClaimForm::OtherElapsed,
        BoundedMeasure::Fraction(fraction) => windowed_fraction(fraction),
        BoundedMeasure::Accumulate {
            timed_within: Some(_),
            ..
        }
        | BoundedMeasure::Steps {
            timed_within: Some(_),
            ..
        } => ClaimForm::TimeHorizonMeasure,
        BoundedMeasure::Accumulate { .. } | BoundedMeasure::Steps { .. } => {
            ClaimForm::DiscreteStepMeasure
        }
    }
}

fn windowed_fraction(fraction: &FractionMeasure) -> ClaimForm {
    if fraction.weight == Some(Weighting::Delay) {
        ClaimForm::DelayWeightedFraction
    } else if fraction.timed_window.is_none() {
        ClaimForm::DiscreteStepMeasure
    } else if fraction.weight.is_some() {
        ClaimForm::TimeHorizonMeasure
    } else {
        ClaimForm::UnweightedTimeFraction
    }
}

enum SingleEvent {
    Eventual,
    Always,
}

/// One `eventually`, `until` or `always` over state predicates, unbounded
/// (reachability) or over `[0, D]` (a deadline event).
fn single_event(formula: &CheckedTemporalFormula) -> Option<(ClaimForm, SingleEvent)> {
    let formula = formula.formula();
    let nodes = formula.nodes();
    let (event, interval, operands) = match nodes.get(formula.root().index())? {
        FormulaNode::Unary {
            operator: UnaryTemporal::Eventually,
            interval,
            operand,
        } => (SingleEvent::Eventual, interval, vec![*operand]),
        FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval,
            operand,
        } => (SingleEvent::Always, interval, vec![*operand]),
        FormulaNode::Binary {
            operator: BinaryTemporal::Until,
            interval,
            left,
            right,
        } => (SingleEvent::Eventual, interval, vec![*left, *right]),
        _ => return None,
    };
    if !operands
        .iter()
        .all(|id| matches!(nodes.get(id.index()), Some(FormulaNode::Holds { .. })))
    {
        return None;
    }
    let form = match interval {
        None => ClaimForm::Reachability,
        Some(CheckedInterval::Timed(window)) => {
            let from_zero = window.lower() == &Rational::from_integer(Integer::from(0_i64));
            if !from_zero || window.upper().is_none() {
                return None;
            }
            ClaimForm::DeadlineEvent
        }
        Some(CheckedInterval::Steps(_)) => return None,
    };
    Some((form, event))
}

/// Whether the formula's intervals, at least one, all count discrete steps.
fn steps_only(formula: &CheckedTemporalFormula) -> bool {
    let mut counted = false;
    for node in formula.formula().nodes() {
        if let FormulaNode::Unary {
            interval: Some(interval),
            ..
        }
        | FormulaNode::Binary {
            interval: Some(interval),
            ..
        } = node
        {
            if matches!(interval, CheckedInterval::Timed(_)) {
                return false;
            }
            counted = true;
        }
    }
    counted
}

/// A node of the region graph: the model state and one representative of the
/// clock region, the last clock the one that measures divergence.
type RegionNode = (Vec<u8>, Vec<(BigInt, BigInt)>);

/// The region partition of one model: constants are multiples of
/// `1 / scale`, and no clock is compared above `largest` such units.
struct Regions {
    scale: u64,
    largest: u64,
}

impl Regions {
    fn of(timed: &TimedSystem<'_, '_>) -> Result<Self, ProbabilityDocumentError> {
        let mut constants: Vec<Rational> = super::clock_constraints(timed)
            .into_iter()
            .map(|constraint| constraint.bound().clone())
            .collect();
        for resets in timed.model().resets() {
            constants.extend(resets.resets().iter().map(|reset| reset.value().clone()));
        }
        let scale = constants
            .iter()
            .fold(BigInt::one(), |scale, value| {
                scale.lcm(value.denominator().as_big())
            })
            .to_u64()
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let mut largest = 0;
        for value in &constants {
            let units =
                value.numerator().as_big() * BigInt::from(scale) / value.denominator().as_big();
            largest = largest.max(
                units
                    .to_u64()
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            );
        }
        Ok(Self { scale, largest })
    }

    /// The clock `index` of `count` cap in units: one unit for the divergence
    /// clock, which comes last.
    fn cap(&self, index: usize, count: usize) -> u64 {
        if index + 1 == count {
            1
        } else {
            self.largest
        }
    }

    /// The representative of the region of `values`: a clock above its cap
    /// reads one unit above it, and the other fractional parts keep only
    /// their order and which are zero.
    fn canonical(&self, values: &[Rational]) -> Result<Vec<Rational>, ProbabilityDocumentError> {
        let split = |value: &Rational| {
            let units = value.numerator().as_big() * BigInt::from(self.scale);
            units.div_rem(value.denominator().as_big())
        };
        let mut fractions = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let (whole, rest) = split(value);
            let cap = BigInt::from(self.cap(index, values.len()));
            if (whole < cap) && rest != BigInt::from(0) {
                fractions.push(rational(rest, value.denominator().as_big().clone())?);
            }
        }
        fractions.sort();
        fractions.dedup();
        let parts = BigInt::from(fractions.len() + 1);
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let (whole, rest) = split(value);
                let cap = BigInt::from(self.cap(index, values.len()));
                let scale = BigInt::from(self.scale);
                if whole > cap || (whole == cap && rest != BigInt::from(0)) {
                    return rational(cap + 1, scale);
                }
                if rest == BigInt::from(0) {
                    return rational(whole, scale);
                }
                let fraction = rational(rest, value.denominator().as_big().clone())?;
                let rank = fractions
                    .iter()
                    .position(|held| held == &fraction)
                    .ok_or(ProbabilityDocumentError::Unsupported)?;
                rational(whole * &parts + BigInt::from(rank + 1), scale * &parts)
            })
            .collect()
    }

    /// The delay from the representative `values` into the next region, or
    /// `None` when every clock is above its cap.
    fn successor_delay(
        &self,
        values: &[Rational],
    ) -> Result<Option<Rational>, ProbabilityDocumentError> {
        let mut greatest = None::<Rational>;
        let mut on_boundary = false;
        for (index, value) in values.iter().enumerate() {
            let units = value.numerator().as_big() * BigInt::from(self.scale);
            let (whole, rest) = units.div_rem(value.denominator().as_big());
            if whole > BigInt::from(self.cap(index, values.len())) {
                continue;
            }
            on_boundary |= rest == BigInt::from(0);
            let fraction = rational(rest, value.denominator().as_big().clone())?;
            if greatest.as_ref().is_none_or(|held| &fraction > held) {
                greatest = Some(fraction);
            }
        }
        let Some(greatest) = greatest else {
            return Ok(None);
        };
        let (numerator, denominator) = (
            greatest.numerator().as_big().clone(),
            greatest.denominator().as_big().clone(),
        );
        let rest = &denominator - &numerator;
        let halves = if on_boundary { 2 } else { 1 };
        rational(rest, denominator * halves * BigInt::from(self.scale)).map(Some)
    }
}

fn rational(numerator: BigInt, denominator: BigInt) -> Result<Rational, ProbabilityDocumentError> {
    Rational::new(Integer::from_big(numerator), Integer::from_big(denominator))
        .map_err(|_| ProbabilityDocumentError::Unsupported)
}

/// One edge of the region graph, and whether it marks that a further unit of
/// time has passed.
struct RegionEdge {
    from: usize,
    to: usize,
    tick: bool,
}

/// Whether a state from which time cannot diverge is reachable over the
/// model's dense runs (FSpec FR-416 "Time-locks"), decided on the region
/// graph with one added clock that is reset each time it reaches a unit
/// (Tripakis). A state diverges when it has a path with ticks for ever:
/// the greatest set from whose states a tick back into the set is reachable.
#[implements("FR-921-AC-5")]
fn time_lock_reachable(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let regions = Regions::of(timed)?;
    let zero = Rational::from_integer(Integer::from(0_i64));
    let unit = rational(BigInt::one(), BigInt::from(regions.scale))?;
    let mut index = BTreeMap::<RegionNode, usize>::new();
    let mut states = Vec::<(TimedState, Vec<Rational>)>::new();
    let mut edges = Vec::<RegionEdge>::new();
    let mut visit = |discrete: &ModelSimState,
                     values: Vec<Rational>,
                     states: &mut Vec<(TimedState, Vec<Rational>)>|
     -> Result<usize, ProbabilityDocumentError> {
        let key = system
            .state_key(discrete, fsl_foundation::IdentityLimits::default())
            .map_err(ProbabilityDocumentError::KeyEncoding)?
            .0;
        let values = regions.canonical(&values)?;
        let exact = values.iter().map(|value| {
            (
                value.numerator().as_big().clone(),
                value.denominator().as_big().clone(),
            )
        });
        let node = (key.to_vec(), exact.collect());
        if let Some(found) = index.get(&node) {
            return Ok(*found);
        }
        if states.len() >= limits.states {
            return Err(exceeded(
                ProbabilityDocumentLimit::States,
                u64::try_from(limits.states).unwrap_or(u64::MAX),
                u64::try_from(states.len())
                    .unwrap_or(u64::MAX)
                    .saturating_add(1),
            ));
        }
        let clocks = values
            .get(..values.len().saturating_sub(1))
            .ok_or(ProbabilityDocumentError::Unsupported)?
            .to_vec();
        index.insert(node, states.len());
        states.push((
            TimedState::new(discrete.clone(), ClockValuation::new(clocks)),
            values,
        ));
        Ok(states.len() - 1)
    };
    for initial in timed.initial() {
        let mut values = initial.clocks().values().to_vec();
        values.push(zero.clone());
        visit(initial.discrete(), values, &mut states)?;
    }
    let stopped = |_| ProbabilityDocumentError::Unsupported;
    let mut next = 0;
    while let Some((state, values)) = states.get(next).cloned() {
        poll(cancel)?;
        let from = next;
        next += 1;
        let elapsed = values
            .last()
            .cloned()
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let mut push = |to: usize, tick: bool| {
            if edges.len() >= limits.edges {
                return Err(exceeded(
                    ProbabilityDocumentLimit::Edges,
                    u64::try_from(limits.edges).unwrap_or(u64::MAX),
                    u64::try_from(edges.len())
                        .unwrap_or(u64::MAX)
                        .saturating_add(1),
                ));
            }
            edges.push(RegionEdge { from, to, tick });
            Ok(())
        };
        if elapsed >= unit {
            let mut reset = values.clone();
            if let Some(last) = reset.last_mut() {
                *last = zero.clone();
            }
            push(visit(state.discrete(), reset, &mut states)?, true)?;
        }
        if let Some(delay) = regions.successor_delay(&values)? {
            if let Some(delayed) = timed.delay(&state, &delay).map_err(stopped)? {
                let mut after = delayed.clocks().values().to_vec();
                after.push(elapsed.add(&delay));
                push(visit(state.discrete(), after, &mut states)?, false)?;
            }
        }
        for (_, post) in timed.steps(&state).map_err(stopped)? {
            let mut after = post.clocks().values().to_vec();
            after.push(elapsed.clone());
            push(visit(post.discrete(), after, &mut states)?, false)?;
        }
    }
    let mut before = vec![Vec::new(); states.len()];
    for edge in &edges {
        if let Some(list) = before.get_mut(edge.to) {
            list.push(edge.from);
        }
    }
    let mut divergent = vec![true; states.len()];
    loop {
        poll(cancel)?;
        let mut kept = vec![false; states.len()];
        let mut queue = VecDeque::new();
        let ticking = edges
            .iter()
            .filter(|edge| edge.tick && divergent.get(edge.to) == Some(&true))
            .map(|edge| edge.from);
        for from in ticking {
            if kept
                .get_mut(from)
                .is_some_and(|slot| !std::mem::replace(slot, true))
            {
                queue.push_back(from);
            }
        }
        while let Some(state) = queue.pop_front() {
            for from in before.get(state).into_iter().flatten() {
                if kept
                    .get_mut(*from)
                    .is_some_and(|slot| !std::mem::replace(slot, true))
                {
                    queue.push_back(*from);
                }
            }
        }
        if kept == divergent {
            return Ok(divergent.contains(&false));
        }
        divergent = kept;
    }
}

/// The route a `timed-product/v1` certificate for `claim` must name, as the
/// checker establishes it from the checked model alone.
///
/// # Errors
///
/// `ObjectiveMismatch` when the table admits no certificate for the claim,
/// `Unsupported` for a subject with no timed reading or a continuous delay,
/// and the time-lock search's limit, cancellation or reading failure.
#[implements("FR-921-AC-5")]
pub fn timed_certificate_route(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<TimedProbabilityRoute, ProbabilityDocumentError> {
    let timed = TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    certificate_route(system, &timed, claim, limits, cancel)
}
