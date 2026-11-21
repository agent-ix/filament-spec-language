// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Alur-Dill progress condition of FR-924 over timed-product keys.
//!
//! An end component of a timed product is progressive when, for every clock
//! entry of its states' keys (the deadline included), an action of the
//! component resets it, a delay of the component takes it across an integer
//! boundary, or it is at its cap at a state of the component. A component that
//! is not progressive lets a scheduler stay in it while the total delay
//! converges, so it is checked as one zero-time state with its exits.
//!
//! The checker reads the clock entries from the keys it enumerated itself; the
//! engine does not share this code.

use serde_json::Value;

use super::ProbabilityProductKey;

struct ClockView {
    whole: u64,
    cap: u64,
    rank: u64,
}

struct DeadlineView {
    limit: u64,
    whole: u64,
    rank: u64,
}

struct KeyView {
    clocks: Vec<ClockView>,
    deadline: Option<DeadlineView>,
}

/// What a product-state key says about time.
enum Shape {
    /// A key with no timed model component: no clock can advance.
    NotTimed,
    /// A timed key whose clock or deadline entries do not parse.
    Malformed,
    Timed(KeyView),
}

fn number(value: Option<&Value>) -> Option<u64> {
    value?.as_str()?.parse().ok()
}

fn shape(key: &ProbabilityProductKey) -> Shape {
    let Ok(root) = serde_json::from_slice::<Value>(key.as_bytes()) else {
        return Shape::NotTimed;
    };
    let Some(model) = root.get(4).and_then(|model| model.get(1)) else {
        return Shape::NotTimed;
    };
    if !matches!(
        model.get(0).and_then(Value::as_str),
        Some("digital" | "region")
    ) {
        return Shape::NotTimed;
    }
    let Some(entries) = model.get(4).and_then(Value::as_array) else {
        return Shape::Malformed;
    };
    let mut clocks = Vec::with_capacity(entries.len());
    for entry in entries {
        let (Some(whole), Some(cap)) = (number(entry.get(1)), number(entry.get(2))) else {
            return Shape::Malformed;
        };
        clocks.push(ClockView {
            whole,
            cap,
            rank: number(entry.get(3)).unwrap_or(0),
        });
    }
    let deadline = match model.get(5).and_then(Value::as_array) {
        Some(deadline) if deadline.first().and_then(Value::as_str) == Some("deadline") => {
            let (Some(limit), Some(whole)) = (number(deadline.get(1)), number(deadline.get(2)))
            else {
                return Shape::Malformed;
            };
            Some(DeadlineView {
                limit,
                whole,
                rank: number(deadline.get(3)).unwrap_or(0),
            })
        }
        _ => None,
    };
    Shape::Timed(KeyView { clocks, deadline })
}

/// Whether the edge `from` to `to` takes no time: no clock entry changes. A
/// key with no timed component takes none; a key that does not parse is not
/// known to take none.
pub(crate) fn edge_is_zero_time(from: &ProbabilityProductKey, to: &ProbabilityProductKey) -> bool {
    match (shape(from), shape(to)) {
        (Shape::NotTimed, Shape::NotTimed) => true,
        (Shape::Timed(from), Shape::Timed(to)) => {
            from.clocks.len() == to.clocks.len()
                && from
                    .clocks
                    .iter()
                    .zip(&to.clocks)
                    .all(|(a, b)| a.whole == b.whole && a.rank == b.rank)
                && match (&from.deadline, &to.deadline) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.whole == b.whole && a.rank == b.rank,
                    _ => false,
                }
        }
        _ => false,
    }
}

/// Whether the end component whose states have keys `members`, with the
/// internal edges `edges` (indices into `members`), is progressive.
///
/// A component that cannot be read is progressive: it is not collapsed, and
/// FR-201's plain conditions apply to it.
pub(crate) fn component_is_progressive(
    members: &[&ProbabilityProductKey],
    edges: &[(usize, usize)],
) -> bool {
    let mut views = Vec::with_capacity(members.len());
    for key in members {
        match shape(key) {
            Shape::Timed(view) => views.push(view),
            // With no clock entry there is nothing that bounds time.
            Shape::NotTimed | Shape::Malformed => return true,
        }
    }
    let Some(first) = views.first() else {
        return true;
    };
    let count = first.clocks.len();
    let has_deadline = first.deadline.is_some();
    if views
        .iter()
        .any(|view| view.clocks.len() != count || view.deadline.is_some() != has_deadline)
    {
        return true;
    }
    for clock in 0..count {
        let reset = edges.iter().any(|(from, to)| {
            let (before, after) = (&views[*from].clocks[clock], &views[*to].clocks[clock]);
            after.whole == 0 && after.rank == 0 && (before.whole > 0 || before.rank > 0)
        });
        let crosses = edges
            .iter()
            .any(|(from, to)| views[*to].clocks[clock].whole > views[*from].clocks[clock].whole);
        let capped = views
            .iter()
            .any(|view| view.clocks[clock].whole >= view.clocks[clock].cap);
        if !(reset || crosses || capped) {
            return false;
        }
    }
    if has_deadline {
        // The deadline clock is never reset; it is at its cap only past the
        // limit, where the state is decided and belongs to no component.
        let crosses = edges.iter().any(|(from, to)| {
            matches!(
                (&views[*from].deadline, &views[*to].deadline),
                (Some(before), Some(after)) if after.whole > before.whole
            )
        });
        let capped = views.iter().any(|view| {
            view.deadline
                .as_ref()
                .is_some_and(|deadline| deadline.whole > deadline.limit)
        });
        if !(crosses || capped) {
            return false;
        }
    }
    true
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
            elapsed: None,
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .expect("digital timed keys are valid")
    }

    /// FR-924-AC-6: a clock that no action of the component resets, no delay
    /// takes across a boundary and that is below its cap keeps the component
    /// from being progressive, whatever the other clock does.
    #[test]
    fn a_frozen_clock_makes_the_component_nonprogressive() {
        let from = key(&[(1, 3), (0, 3)], None);
        let to = key(&[(1, 3), (1, 3)], None);
        let members = [&from, &to];
        // The second clock is reset by the way back and crosses on the way
        // out; the first never changes.
        assert!(!component_is_progressive(&members, &[(0, 1), (1, 0)]));
    }

    /// Every clock reset, crossing or capped: the component is progressive.
    #[test]
    fn a_component_that_resets_or_crosses_every_clock_is_progressive() {
        let zero = key(&[(0, 3)], None);
        let one = key(&[(1, 3)], None);
        assert!(component_is_progressive(&[&zero, &one], &[(0, 1), (1, 0)]));
        let capped = key(&[(3, 3)], None);
        assert!(component_is_progressive(&[&capped], &[(0, 0)]));
    }

    /// A key that is not a timed key takes no time, and an edge between equal
    /// timed keys takes none either.
    #[test]
    fn unchanged_keys_take_no_time() {
        let saturated = key(&[(3, 3)], None);
        assert!(edge_is_zero_time(&saturated, &saturated));
        let moving = key(&[(1, 3)], Some(1));
        let later = key(&[(2, 3)], Some(1));
        assert!(!edge_is_zero_time(&moving, &later));
    }
}
