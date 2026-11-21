// SPDX-License-Identifier: AGPL-3.0-or-later
//! The axes along which an edge of a timed-product graph makes no progress
//! (FR-924-AC-6, after Alur and Dill).
//!
//! A set of states closed under the actions it keeps is a time-convergent end
//! component when some clock is never reset inside it and never reaches its
//! cap: that clock bounds the elapsed time, so no run stays in the set
//! forever and still lets time diverge. A zero-time component is the special
//! case in which no clock changes at all. The axes are read from the
//! product-state keys alone, so a checker finds the same components as the
//! engine without trusting it.

use serde_json::Value;

use super::ProbabilityProductKey;

/// One way an edge can make no progress.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TimedAxis {
    /// No clock and no deadline changes: a zero-time edge.
    Still,
    /// The deadline clock keeps its whole part.
    Deadline,
    /// The clock at this position of the key keeps its whole part, is not
    /// reset, and is below its cap in both states.
    Clock(usize),
}

struct ClockView {
    whole: u64,
    cap: u64,
    rank: u64,
}

struct KeyView {
    clocks: Vec<ClockView>,
    deadline: Option<(u64, u64)>,
}

fn number(value: Option<&Value>) -> Option<u64> {
    value?.as_str()?.parse().ok()
}

fn view(key: &ProbabilityProductKey) -> Option<KeyView> {
    let root: Value = serde_json::from_slice(key.as_bytes()).ok()?;
    let model = root.get(4)?.get(1)?;
    let clocks = model
        .get(4)?
        .as_array()?
        .iter()
        .map(|clock| {
            Some(ClockView {
                whole: number(clock.get(1))?,
                cap: number(clock.get(2))?,
                rank: number(clock.get(3)).unwrap_or(0),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let deadline = model
        .get(5)
        .and_then(Value::as_array)
        .filter(|deadline| deadline.first().and_then(Value::as_str) == Some("deadline"))
        .map(|deadline| {
            Some((
                number(deadline.get(2))?,
                number(deadline.get(3)).unwrap_or(0),
            ))
        })
        .map(|deadline| deadline.unwrap_or((u64::MAX, u64::MAX)));
    Some(KeyView { clocks, deadline })
}

/// The number of clocks in a timed key, or `None` for any other key.
pub(crate) fn timed_clock_count(key: &ProbabilityProductKey) -> Option<usize> {
    view(key).map(|view| view.clocks.len())
}

/// The axes along which the edge `from` to `to` is nonprogressive. A key that
/// is not a timed key has no clocks to advance, so its edges are `Still`.
pub fn timed_edge_axes(from: &ProbabilityProductKey, to: &ProbabilityProductKey) -> Vec<TimedAxis> {
    let (Some(from), Some(to)) = (view(from), view(to)) else {
        return vec![TimedAxis::Still];
    };
    if from.clocks.len() != to.clocks.len() || from.deadline.is_some() != to.deadline.is_some() {
        return Vec::new();
    }
    let mut axes = Vec::new();
    let clocks_equal = from
        .clocks
        .iter()
        .zip(&to.clocks)
        .all(|(a, b)| a.whole == b.whole && a.rank == b.rank);
    if clocks_equal && from.deadline == to.deadline {
        axes.push(TimedAxis::Still);
    }
    if let (Some(before), Some(after)) = (from.deadline, to.deadline) {
        if before.0 == after.0 {
            axes.push(TimedAxis::Deadline);
        }
    }
    for (index, (before, after)) in from.clocks.iter().zip(&to.clocks).enumerate() {
        let reset = before.rank > 0 && after.rank == 0;
        if before.whole == after.whole
            && before.whole < before.cap
            && after.whole < after.cap
            && !reset
        {
            axes.push(TimedAxis::Clock(index));
        }
    }
    axes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        digital_probability_product_key, DigitalClockKey, DigitalDeadlineKey,
        DigitalProductKeyInput, DigitalTail,
    };
    use serde_json::json;

    fn key(clocks: &[(u64, u64)], elapsed: Option<u64>) -> ProbabilityProductKey {
        digital_probability_product_key(DigitalProductKeyInput {
            binding: None,
            discrete: json!([]),
            unit: json!("second"),
            scale: 1,
            clocks: clocks
                .iter()
                .enumerate()
                .map(|(index, (value, cap))| DigitalClockKey {
                    identity: json!(["Type", format!("clock{index}"), ["u", "object"]]),
                    value: *value,
                    cap: *cap,
                })
                .collect(),
            deadline: elapsed.map(|elapsed| DigitalDeadlineKey { limit: 4, elapsed }),
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .expect("digital timed keys are valid")
    }

    /// FR-924-AC-6: a clock that is neither reset nor saturated freezes its
    /// axis, while a delay of the other clock does not make the edge `Still`.
    #[test]
    fn an_unreset_unsaturated_clock_freezes_its_axis() {
        let from = key(&[(1, 3), (0, 3)], None);
        let to = key(&[(1, 3), (1, 3)], None);
        let axes = timed_edge_axes(&from, &to);
        assert!(axes.contains(&TimedAxis::Clock(0)));
        assert!(!axes.contains(&TimedAxis::Clock(1)));
        assert!(!axes.contains(&TimedAxis::Still));
    }

    /// A saturated clock bounds nothing, so it freezes no axis; an edge that
    /// changes nothing is `Still`.
    #[test]
    fn a_saturated_clock_freezes_no_axis_and_an_unchanged_key_is_still() {
        let saturated = key(&[(3, 3)], None);
        assert_eq!(
            timed_edge_axes(&saturated, &saturated),
            vec![TimedAxis::Still]
        );
        let moving = key(&[(1, 3)], Some(1));
        let later = key(&[(2, 3)], Some(1));
        let axes = timed_edge_axes(&moving, &later);
        assert!(axes.contains(&TimedAxis::Deadline));
        assert!(!axes.contains(&TimedAxis::Clock(0)));
    }
}
