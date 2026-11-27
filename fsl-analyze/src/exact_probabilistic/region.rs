// SPDX-License-Identifier: AGPL-3.0-or-later
//! The producer's symbolic clock region and its time successor (FR-922
//! "Behavior", ADR-038 RL-3).
//!
//! A region fixes, for every clock, its integer part up to the clock's cap, whether
//! its fractional part is zero, and the order of the nonzero fractional parts
//! (ties kept). A clock above its largest constant is saturated and has no
//! fraction. The region is an ordered partition of the clocks: those at an
//! integer, the open fraction classes in ascending order, and the saturated.
//!
//! The checker writes its own region and successor with a different
//! representation (`fsl-replay`, `document/timed/region.rs`); the two share
//! nothing but the wire type [`fsl_replay::ClockPlace`] (FR-921).

#![cfg_attr(
    not(test),
    allow(dead_code, reason = "the region route is wired in QSL-3495 step 6.3")
)]

use std::collections::{BTreeMap, BTreeSet};

use fsl_replay::ClockPlace;

/// One clock region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Region {
    /// Each clock's cap: its largest compared constant plus one.
    caps: Vec<u64>,
    /// Each clock's integer part; the cap for a saturated clock.
    integer: Vec<u64>,
    /// The unsaturated clocks whose fractional part is zero.
    zero: BTreeSet<usize>,
    /// The nonempty fraction classes, ascending.
    open: Vec<BTreeSet<usize>>,
    /// The clocks above their largest constant.
    saturated: BTreeSet<usize>,
}

impl Region {
    /// The region of the given places, or `None` when a place is outside its
    /// cap or the classes are not `1..=K` without a gap.
    pub(super) fn of(caps: &[u64], places: &[ClockPlace]) -> Option<Self> {
        if caps.len() != places.len() {
            return None;
        }
        let mut region = Self {
            caps: caps.to_vec(),
            integer: vec![0; caps.len()],
            zero: BTreeSet::new(),
            open: Vec::new(),
            saturated: BTreeSet::new(),
        };
        let mut classes: BTreeMap<u32, BTreeSet<usize>> = BTreeMap::new();
        for (clock, (place, cap)) in places.iter().zip(caps).enumerate() {
            match *place {
                ClockPlace::Integer(integer) if integer < *cap => {
                    region.integer[clock] = integer;
                    region.zero.insert(clock);
                }
                ClockPlace::Open { integer, class }
                    if class >= 1 && integer.checked_add(2).is_some_and(|end| end <= *cap) =>
                {
                    region.integer[clock] = integer;
                    classes.entry(class).or_default().insert(clock);
                }
                ClockPlace::Saturated => {
                    region.integer[clock] = *cap;
                    region.saturated.insert(clock);
                }
                _ => return None,
            }
        }
        let dense = classes
            .keys()
            .zip(1_u32..)
            .all(|(class, want)| *class == want);
        if !dense {
            return None;
        }
        region.open = classes.into_values().collect();
        Some(region)
    }

    /// The places of the clocks, in clock order.
    pub(super) fn places(&self) -> Vec<ClockPlace> {
        let mut places = vec![ClockPlace::Saturated; self.caps.len()];
        for clock in &self.zero {
            places[*clock] = ClockPlace::Integer(self.integer[*clock]);
        }
        for (position, class) in self.open.iter().enumerate() {
            for clock in class {
                places[*clock] = ClockPlace::Open {
                    integer: self.integer[*clock],
                    class: u32::try_from(position + 1).unwrap_or(u32::MAX),
                };
            }
        }
        places
    }

    /// Whether every clock is saturated: time passes and the region stays.
    pub(super) fn is_unbounded(&self) -> bool {
        self.zero.is_empty() && self.open.is_empty()
    }

    /// The region first reached by any positive delay.
    pub(super) fn successor(&self) -> Self {
        let mut next = self.clone();
        if !self.zero.is_empty() {
            // Every clock at an integer leaves it into a new lowest class;
            // one at its constant leaves it for good.
            let mut lowest = BTreeSet::new();
            for clock in &self.zero {
                if self.integer[*clock] + 1 == self.caps[*clock] {
                    next.saturated.insert(*clock);
                    next.integer[*clock] = self.caps[*clock];
                } else {
                    lowest.insert(*clock);
                }
            }
            next.zero.clear();
            next.open.insert(0, lowest);
        } else if let Some(top) = self.open.last() {
            // The clocks of the top class reach the next integer together.
            for clock in top {
                next.integer[*clock] += 1;
                next.zero.insert(*clock);
            }
            next.open.pop();
        }
        next.normalized()
    }

    /// The region after the clock is set to the integer `value`.
    pub(super) fn reset(&self, clock: usize, value: u64) -> Self {
        let mut next = self.clone();
        next.zero.remove(&clock);
        next.saturated.remove(&clock);
        for class in &mut next.open {
            class.remove(&clock);
        }
        if value >= self.caps[clock] {
            next.integer[clock] = self.caps[clock];
            next.saturated.insert(clock);
        } else {
            next.integer[clock] = value;
            next.zero.insert(clock);
        }
        next.normalized()
    }

    /// The representative valuation, one exact rational `(numerator,
    /// denominator)` per clock, in base units: the integer part plus
    /// `class / (K + 1)`; a saturated clock is at its cap.
    pub(super) fn representative(&self) -> Vec<(u64, u64)> {
        let classes = u64::try_from(self.open.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        self.places()
            .into_iter()
            .zip(&self.integer)
            .map(|(place, integer)| match place {
                ClockPlace::Open { class, .. } => (integer * classes + u64::from(class), classes),
                ClockPlace::Integer(_) | ClockPlace::Saturated => (*integer, 1),
            })
            .collect()
    }

    fn normalized(mut self) -> Self {
        self.open.retain(|class| !class.is_empty());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::Value;

    fn parse(text: &str) -> ClockPlace {
        match text.as_bytes()[0] {
            b's' => ClockPlace::Saturated,
            b'i' => ClockPlace::Integer(text[1..].parse().expect("an integer")),
            _ => {
                let (integer, class) = text[1..].split_once('.').expect("o<integer>.<class>");
                ClockPlace::Open {
                    integer: integer.parse().expect("an integer"),
                    class: class.parse().expect("a class"),
                }
            }
        }
    }

    fn places(value: &Value) -> Vec<ClockPlace> {
        value
            .as_array()
            .expect("places")
            .iter()
            .map(|place| parse(place.as_str().expect("a place")))
            .collect()
    }

    /// Run every fixture case against `successor`; the first mismatch is the
    /// error. `None` when the fixture is not on disk (a packaged crate).
    fn run_fixture(successor: impl Fn(&Region) -> Region) -> Option<Result<(), String>> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/timed/region_paths.json"
        );
        let text = std::fs::read_to_string(path).ok()?;
        let fixture: Value = serde_json::from_str(&text).expect("the fixture is JSON");
        for case in fixture["cases"].as_array().expect("cases") {
            let name = case["name"].as_str().expect("a name");
            let caps: Vec<u64> = case["caps"]
                .as_array()
                .expect("caps")
                .iter()
                .map(|cap| cap.as_u64().expect("a cap"))
                .collect();
            let mut region: Option<Region> = None;
            for step in case["steps"].as_array().expect("steps") {
                let next = match step["op"].as_str().expect("an op") {
                    "start" => Region::of(&caps, &places(&step["places"]))
                        .ok_or_else(|| format!("{name}: the start places are invalid")),
                    "succ" => Ok(successor(region.as_ref().expect("a start"))),
                    "reset" => Ok(region.as_ref().expect("a start").reset(
                        usize::try_from(step["clock"].as_u64().expect("a clock")).expect("fits"),
                        step["value"].as_u64().expect("a value"),
                    )),
                    other => Err(format!("{name}: unknown op {other}")),
                };
                let next = match next {
                    Ok(next) => next,
                    Err(failure) => return Some(Err(failure)),
                };
                if let Some(expect) = step.get("expect") {
                    if next.places() != places(expect) {
                        return Some(Err(format!(
                            "{name}: expected {expect}, got {:?}",
                            next.places()
                        )));
                    }
                }
                region = Some(next);
            }
        }
        Some(Ok(()))
    }

    #[trace("FR-922-AC-1")]
    #[test]
    fn the_producer_successor_follows_the_hand_paths() {
        if let Some(outcome) = run_fixture(Region::successor) {
            outcome.expect("every hand path is followed");
        }
    }

    /// A successor that jumps every clock to its next integer skips the
    /// regions between two integer crossings: the fixture catches it.
    #[trace("FR-922-AC-1")]
    #[test]
    fn a_successor_that_skips_the_region_between_two_integer_crossings_is_caught() {
        let skipping = |region: &Region| region.successor().successor();
        if let Some(outcome) = run_fixture(skipping) {
            assert!(
                outcome.is_err(),
                "the fixture must reject a skipping successor"
            );
        }
    }

    #[trace("FR-922-AC-1")]
    #[test]
    fn an_all_saturated_region_is_unbounded_and_its_own_successor() {
        let region = Region::of(&[2, 3], &[ClockPlace::Saturated, ClockPlace::Saturated])
            .expect("valid places");
        assert!(region.is_unbounded());
        assert_eq!(region.successor(), region);
    }

    #[trace("FR-922-AC-1")]
    #[test]
    fn the_representative_separates_the_classes_inside_one_integer_part() {
        let region = Region::of(
            &[3, 3],
            &[
                ClockPlace::Open {
                    integer: 0,
                    class: 1,
                },
                ClockPlace::Open {
                    integer: 0,
                    class: 2,
                },
            ],
        )
        .expect("valid places");
        // 1/3 and 2/3.
        assert_eq!(region.representative(), vec![(1, 3), (2, 3)]);
    }

    #[trace("FR-920-AC-1")]
    #[test]
    fn places_with_a_class_gap_or_above_the_cap_are_refused() {
        assert!(Region::of(
            &[3],
            &[ClockPlace::Open {
                integer: 0,
                class: 2
            }]
        )
        .is_none());
        assert!(Region::of(
            &[3],
            &[ClockPlace::Open {
                integer: 2,
                class: 1
            }]
        )
        .is_none());
        assert!(Region::of(&[3], &[ClockPlace::Integer(3)]).is_none());
    }
}
