// SPDX-License-Identifier: AGPL-3.0-or-later
//! The producer's realiser of a region delay edge (QSL-3495 design 6a).
//!
//! A region delay edge `R -> succ^n(R)` has no fixed delay. The realiser walks
//! the policy's chosen path with one concrete valuation, and gives each region
//! delay edge an exact rational delay that puts the valuation in the edge's
//! target region. Replay never reads it as authority: the checker recomputes
//! the run from the stated delays (FR-921, ruling Q20).

use filament_exact::{Integer, Rational};
use crate::exact_probabilistic::ProductAction;
use fsl_eval::simulation::model::{ClockValuation, ModelSystem, TimedSystem};
use fsl_replay::ClockPlace;
use fsl_semantics::check::CheckedProbabilisticClaim;
use num_bigint::BigInt;
use num_integer::Integer as _;
use serde_json::{json, Value as JsonValue};

/// The places a region key names: the model clocks in
/// [`TimedSystem::places`] order, then the deadline clock when the key has
/// one; `caps` gives each clock's cap in the same order.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct KeyPlaces {
    pub(super) places: Vec<ClockPlace>,
    pub(super) caps: Vec<u64>,
}

fn number(value: &JsonValue) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn place(integer: u64, cap: u64, class: u64) -> Option<ClockPlace> {
    if integer >= cap {
        Some(ClockPlace::Saturated)
    } else if class == 0 {
        Some(ClockPlace::Integer(integer))
    } else {
        Some(ClockPlace::Open {
            integer,
            class: u32::try_from(class).ok()?,
        })
    }
}

/// Whether `key` is a region key.
pub(super) fn is_region_key(key: &fsl_replay::ProbabilityProductKey) -> bool {
    serde_json::from_slice::<JsonValue>(key.as_bytes())
        .ok()
        .is_some_and(|root| {
            root.get(4)
                .and_then(|state| state.get(1))
                .and_then(|model| model.get(0))
                .and_then(JsonValue::as_str)
                == Some("region")
        })
}

/// The places of a region key, each clock found by its canonical identity so
/// the key's identity-sorted order never decides which clock a place is.
pub(super) fn region_key_places(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    key: &fsl_replay::ProbabilityProductKey,
) -> Option<KeyPlaces> {
    let root: JsonValue = serde_json::from_slice(key.as_bytes()).ok()?;
    let keyed = root.get(4)?.get(1)?;
    if keyed.get(0)?.as_str()? != "region" {
        return None;
    }
    let listed = keyed.get(4)?.as_array()?;
    let timed = TimedSystem::new(system, &claim.subject.alias).ok()?;
    if listed.len() != timed.places().len() {
        return None;
    }
    let mut places = Vec::with_capacity(listed.len() + 1);
    let mut caps = Vec::with_capacity(listed.len() + 1);
    for clock_place in timed.places() {
        let field = timed.model().clock(clock_place.clock())?;
        let owner = field.owner();
        let owner = owner.original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .ok()?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring).ok()?;
        let receiver =
            fsl_eval::simulation::model::reference_canonical_text(clock_place.object()).ok()?;
        let receiver: JsonValue = serde_json::from_str(&receiver).ok()?;
        let identity = json!([declaring, field.name(), receiver]);
        let mut matching = listed
            .iter()
            .filter(|clock| clock.get(0) == Some(&identity));
        let clock = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let cap = number(clock.get(2)?)?;
        places.push(place(number(clock.get(1)?)?, cap, number(clock.get(3)?)?)?);
        caps.push(cap);
    }
    if let Some(deadline) = keyed.get(5).filter(|value| !value.is_null()) {
        let limit = number(deadline.get(1)?)?;
        let cap = limit.checked_add(1)?;
        places.push(place(
            number(deadline.get(2)?)?,
            cap,
            number(deadline.get(3)?)?,
        )?);
        caps.push(cap);
    }
    Some(KeyPlaces { places, caps })
}

/// The concrete valuation threaded through a region path (D-001): the model
/// clocks and the never-reset deadline clock, recomputed from the delays the
/// path states, never read from a key.
pub(super) struct Walk {
    pub(super) clocks: ClockValuation,
    z: Rational,
}

/// The walk at the subject's one initial state.
pub(super) fn initial_walk(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
) -> Option<Walk> {
    let timed = TimedSystem::new(system, &claim.subject.alias).ok()?;
    let mut initial = timed.initial().into_iter();
    let first = initial.next()?;
    if initial.next().is_some() {
        return None;
    }
    Some(Walk {
        clocks: first.clocks().clone(),
        z: whole(0),
    })
}

impl Walk {
    /// The exact elapsed time of a product step: a region delay edge takes
    /// the realised delay into the target region, entering the idle tail takes
    /// none, any other step keeps the delay the product states.
    pub(super) fn elapsed_of(
        &self,
        system: &ModelSystem<'_>,
        claim: &CheckedProbabilisticClaim,
        action: &ProductAction,
        post: &fsl_replay::ProbabilityProductKey,
        stated: &Rational,
    ) -> Option<Rational> {
        match action {
            ProductAction::EnterIdleTail => Some(whole(0)),
            ProductAction::UnitDelay => {
                let target = region_key_places(system, claim, post)?;
                let mut values = self.clocks.values().to_vec();
                if target.places.len() == values.len() + 1 {
                    values.push(self.z.clone());
                }
                if target.places.len() != values.len() {
                    return None;
                }
                realise_delay(&values, &target.caps, &target.places)
            }
            _ => Some(stated.clone()),
        }
    }

    /// The walk after a step of `elapsed`: the valuation the timed step
    /// produced, or the delayed one for a step that takes no model step.
    pub(super) fn advance(&mut self, elapsed: &Rational, stepped: Option<ClockValuation>) {
        self.z = self.z.add(elapsed);
        self.clocks = stepped.unwrap_or_else(|| self.clocks.delayed(elapsed));
    }
}

fn whole(value: u64) -> Rational {
    Rational::from_integer(Integer::from_big(BigInt::from(value)))
}

fn floor(value: &Rational) -> BigInt {
    value
        .numerator()
        .as_big()
        .div_floor(value.denominator().as_big())
}

/// The region of `values` under `caps` by the producer's own reading: a clock
/// above its largest constant `cap - 1` is saturated; the others sit at an
/// integer or in an open class ranked by fraction among the unsaturated
/// clocks' distinct nonzero fractions.
pub(super) fn places_of(values: &[Rational], caps: &[u64]) -> Option<Vec<ClockPlace>> {
    let mut fractions: Vec<Rational> = Vec::new();
    let mut parts: Vec<Option<(u64, Rational)>> = Vec::with_capacity(values.len());
    for (value, cap) in values.iter().zip(caps) {
        if *value > whole(cap.checked_sub(1)?) {
            parts.push(None);
            continue;
        }
        let integer = floor(value);
        let fraction = value.sub(&Rational::from_integer(Integer::from_big(integer.clone())));
        let integer = u64::try_from(integer).ok()?;
        if fraction != whole(0) && !fractions.contains(&fraction) {
            fractions.push(fraction.clone());
        }
        parts.push(Some((integer, fraction)));
    }
    fractions.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    parts
        .into_iter()
        .map(|part| match part {
            None => Some(ClockPlace::Saturated),
            Some((integer, fraction)) if fraction == whole(0) => {
                Some(ClockPlace::Integer(integer))
            }
            Some((integer, fraction)) => {
                let class = fractions.iter().position(|known| *known == fraction)? + 1;
                Some(ClockPlace::Open {
                    integer,
                    class: u32::try_from(class).ok()?,
                })
            }
        })
        .collect()
}

/// A delay `d > 0` after which `values + d` lies in `target`, chosen as the
/// design says: the midpoint of the interval of such delays, or its single
/// point when it is degenerate. The region of `values + d` changes only at
/// the delays that bring a clock to an integer, so the candidates are those
/// delays and the midpoints between them; an unbounded target takes one
/// base unit past the last.
pub(super) fn realise_delay(
    values: &[Rational],
    caps: &[u64],
    target: &[ClockPlace],
) -> Option<Rational> {
    let mut breaks: Vec<Rational> = Vec::new();
    for (value, cap) in values.iter().zip(caps) {
        let top = cap.checked_sub(1)?;
        let start = floor(value) + 1;
        let mut integer = u64::try_from(start).ok()?;
        while integer <= top {
            let delay = whole(integer).sub(value);
            if !breaks.contains(&delay) {
                breaks.push(delay);
            }
            integer += 1;
        }
    }
    breaks.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let two = whole(2);
    let mut candidates: Vec<Rational> = Vec::new();
    let mut before = whole(0);
    for point in &breaks {
        candidates.push(before.add(point).div(&two)?);
        candidates.push(point.clone());
        before = point.clone();
    }
    candidates.push(before.add(&whole(1)));
    candidates.into_iter().find(|delay| {
        let moved: Vec<Rational> = values.iter().map(|value| value.add(delay)).collect();
        places_of(&moved, caps).is_some_and(|places| places == target)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
    }

    fn open(integer: u64, class: u32) -> ClockPlace {
        ClockPlace::Open { integer, class }
    }

    /// x = 0.3 and y = 0.8 with constants 2: the delay that makes y reach 1
    /// is 1/5; the region after it is y at an integer and x open.
    #[trace("FR-922-AC-1")]
    #[test]
    fn a_delay_to_the_next_region_lands_in_it() {
        let values = [rational(3, 10), rational(8, 10)];
        let caps = [3, 3];
        assert_eq!(
            places_of(&values, &caps),
            Some(vec![open(0, 1), open(0, 2)])
        );
        let target = [open(0, 1), ClockPlace::Integer(1)];
        let delay = realise_delay(&values, &caps, &target).expect("a delay");
        assert_eq!(delay, rational(1, 5));
        let moved: Vec<Rational> = values.iter().map(|value| value.add(&delay)).collect();
        assert_eq!(places_of(&moved, &caps), Some(target.to_vec()));
    }

    /// The midpoint of the interval is taken, so a clock off the integer
    /// stays inside the open region (0.2 + 0.4 below 1).
    #[trace("FR-922-AC-1")]
    #[test]
    fn a_delay_into_an_open_region_takes_the_midpoint() {
        let values = [rational(0, 1), rational(1, 5)];
        let caps = [3, 3];
        let target = [open(0, 1), open(0, 2)];
        // Breakpoints are 4/5 (y reaches 1) and 1 (x reaches 1); the interval
        // before 4/5 has midpoint 2/5.
        let delay = realise_delay(&values, &caps, &target).expect("a delay");
        assert_eq!(delay, rational(2, 5));
    }

    /// An all-saturated target is its own successor: one base unit.
    #[trace("FR-922-AC-1")]
    #[test]
    fn an_unbounded_region_waits_one_base_unit() {
        let values = [rational(5, 1)];
        let caps = [3];
        let delay = realise_delay(&values, &caps, &[ClockPlace::Saturated]).expect("a delay");
        assert_eq!(delay, rational(1, 1));
    }
}
