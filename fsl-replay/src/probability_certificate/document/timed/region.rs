// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker's symbolic clock region and its time successor (FR-922
//! "Behavior", FR-921).
//!
//! Written independently of the producer's region in `fsl-analyze`: here a
//! region is one `(integer part, fraction rank, saturated)` cell per clock,
//! with rank 0 for a zero fraction and ranks compacted to `1..=K`. The only
//! thing shared with the producer is the wire type [`ClockPlace`].

#![cfg_attr(
    not(test),
    allow(dead_code, reason = "the region route is wired in QSL-3495 step 6.3")
)]

use crate::ClockPlace;

/// One clock's cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cell {
    /// The integer part; the cap when saturated.
    whole: u64,
    /// 0 for a zero fraction (or a saturated clock), else the fraction rank.
    rank: u32,
    saturated: bool,
}

/// A clock region of the checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CheckerRegion {
    /// Each clock's cap: its largest compared constant plus one.
    caps: Vec<u64>,
    cells: Vec<Cell>,
}

impl CheckerRegion {
    /// The region of the given places, or `None` when a place is outside its
    /// cap or the ranks are not `1..=K` without a gap.
    pub(super) fn from_places(caps: &[u64], places: &[ClockPlace]) -> Option<Self> {
        if caps.len() != places.len() {
            return None;
        }
        let mut cells = Vec::with_capacity(caps.len());
        for (place, cap) in places.iter().zip(caps) {
            cells.push(match *place {
                ClockPlace::Integer(whole) if whole < *cap => Cell {
                    whole,
                    rank: 0,
                    saturated: false,
                },
                ClockPlace::Open { integer, class } if class >= 1 && integer + 2 <= *cap => Cell {
                    whole: integer,
                    rank: class,
                    saturated: false,
                },
                ClockPlace::Saturated => Cell {
                    whole: *cap,
                    rank: 0,
                    saturated: true,
                },
                _ => return None,
            });
        }
        let region = Self {
            caps: caps.to_vec(),
            cells,
        };
        let mut compact = region.clone();
        compact.renumber();
        (compact == region).then_some(region)
    }

    /// The places of the clocks, in clock order.
    pub(super) fn places(&self) -> Vec<ClockPlace> {
        self.cells
            .iter()
            .map(|cell| {
                if cell.saturated {
                    ClockPlace::Saturated
                } else if cell.rank == 0 {
                    ClockPlace::Integer(cell.whole)
                } else {
                    ClockPlace::Open {
                        integer: cell.whole,
                        class: cell.rank,
                    }
                }
            })
            .collect()
    }

    /// Whether every clock is saturated: time passes and the region stays.
    pub(super) fn is_unbounded(&self) -> bool {
        self.cells.iter().all(|cell| cell.saturated)
    }

    fn live(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter().filter(|cell| !cell.saturated)
    }

    /// The region first reached by any positive delay.
    pub(super) fn successor(&self) -> Self {
        let mut next = self.clone();
        let at_integer = self.live().any(|cell| cell.rank == 0);
        let top = self.live().map(|cell| cell.rank).max().unwrap_or(0);
        for (index, cell) in next.cells.iter_mut().enumerate() {
            if cell.saturated {
                continue;
            }
            if at_integer {
                if cell.rank == 0 {
                    if cell.whole + 1 == self.caps[index] {
                        *cell = Cell {
                            whole: self.caps[index],
                            rank: 0,
                            saturated: true,
                        };
                    } else {
                        cell.rank = 1;
                    }
                } else {
                    cell.rank += 1;
                }
            } else if top > 0 && cell.rank == top {
                cell.whole += 1;
                cell.rank = 0;
            }
        }
        next.renumber();
        next
    }

    /// The region after the clock is set to the integer `value`.
    pub(super) fn reset(&self, clock: usize, value: u64) -> Self {
        let mut next = self.clone();
        next.cells[clock] = if value >= self.caps[clock] {
            Cell {
                whole: self.caps[clock],
                rank: 0,
                saturated: true,
            }
        } else {
            Cell {
                whole: value,
                rank: 0,
                saturated: false,
            }
        };
        next.renumber();
        next
    }

    /// Close the gaps left in the rank order.
    fn renumber(&mut self) {
        let mut ranks: Vec<u32> = self
            .cells
            .iter()
            .filter(|cell| !cell.saturated && cell.rank > 0)
            .map(|cell| cell.rank)
            .collect();
        ranks.sort_unstable();
        ranks.dedup();
        for cell in &mut self.cells {
            if cell.rank > 0 {
                let position = ranks.binary_search(&cell.rank).unwrap_or(0);
                cell.rank = u32::try_from(position + 1).unwrap_or(u32::MAX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::Value;

    fn parse(text: &str) -> ClockPlace {
        if text == "s" {
            return ClockPlace::Saturated;
        }
        if let Some(rest) = text.strip_prefix('i') {
            return ClockPlace::Integer(rest.parse().expect("an integer"));
        }
        let rest = text.strip_prefix('o').expect("a place");
        let (integer, class) = rest.split_once('.').expect("o<integer>.<class>");
        ClockPlace::Open {
            integer: integer.parse().expect("an integer"),
            class: class.parse().expect("a class"),
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
    fn run_fixture(
        successor: impl Fn(&CheckerRegion) -> CheckerRegion,
    ) -> Option<Result<(), String>> {
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
            let mut region: Option<CheckerRegion> = None;
            for step in case["steps"].as_array().expect("steps") {
                let next = match step["op"].as_str().expect("an op") {
                    "start" => match CheckerRegion::from_places(&caps, &places(&step["places"])) {
                        Some(start) => start,
                        None => return Some(Err(format!("{name}: the start places are invalid"))),
                    },
                    "succ" => successor(region.as_ref().expect("a start")),
                    "reset" => region.as_ref().expect("a start").reset(
                        usize::try_from(step["clock"].as_u64().expect("a clock")).expect("fits"),
                        step["value"].as_u64().expect("a value"),
                    ),
                    other => return Some(Err(format!("{name}: unknown op {other}"))),
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
    fn the_checker_successor_follows_the_hand_paths() {
        if let Some(outcome) = run_fixture(CheckerRegion::successor) {
            outcome.expect("every hand path is followed");
        }
    }

    /// A successor that jumps every clock to its next integer skips the
    /// regions between two integer crossings: the fixture catches it.
    #[trace("FR-922-AC-1")]
    #[test]
    fn a_successor_that_skips_the_region_between_two_integer_crossings_is_caught() {
        let skipping = |region: &CheckerRegion| region.successor().successor();
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
        let region =
            CheckerRegion::from_places(&[2, 3], &[ClockPlace::Saturated, ClockPlace::Saturated])
                .expect("valid places");
        assert!(region.is_unbounded());
        assert_eq!(region.successor(), region);
    }

    #[trace("FR-920-AC-1")]
    #[test]
    fn places_with_a_rank_gap_or_above_the_cap_are_refused() {
        let open = |integer, class| ClockPlace::Open { integer, class };
        assert!(CheckerRegion::from_places(&[3], &[open(0, 2)]).is_none());
        assert!(CheckerRegion::from_places(&[3], &[open(2, 1)]).is_none());
        assert!(CheckerRegion::from_places(&[3], &[ClockPlace::Integer(3)]).is_none());
    }
}
