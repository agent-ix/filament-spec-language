// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-407 "Bounded measures" and "Time windows" on one timed trace:
//! the reference readings of `elapsed`, `steps` and an unweighted
//! `fraction`. Every expected value is the one the criterion states.

use ix_trace_rs::trace;

use filament_exact::{Integer, Meter, Rational, ScalarLimits};
use fsl_eval::temporal::{
    timed_elapsed, timed_fraction, timed_steps, MeasureReading, TemporalPosition, TimedTrace,
};

use MeasureReading::{Censored, NotActivated, Pending, Value};

fn number(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn stamps(values: &[u64]) -> Vec<Rational> {
    values.iter().copied().map(number).collect()
}

/// The trace complete through its last stamp plus `after`, so every window
/// the vectors read is closed.
fn complete(values: &[u64], after: u64) -> TimedTrace {
    let last = values.last().copied().expect("a non-empty trace");
    TimedTrace::through(stamps(values), number(last + after)).expect("a timed trace")
}

/// The truth slice of `length` positions holding exactly at `at`.
fn only(length: usize, at: &[usize]) -> Vec<bool> {
    (0..length).map(|position| at.contains(&position)).collect()
}

fn meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: 256,
        decimal_digits: 256,
        scale_expansion: 256,
        text_input_bytes: 4096,
        text_scalars: 4096,
        normalized_scalars: 4096,
        unit_edges: 256,
        value_occurrences: 4096,
        work_units: 4096,
        result_units: 4096,
    })
}

fn fraction(trace: &TimedTrace, from: u64, holds: &[bool], horizon: u64) -> MeasureReading {
    timed_fraction(
        trace,
        TemporalPosition::new(from),
        holds,
        &number(horizon),
        &mut meter(),
    )
    .expect("a represented position")
}

fn steps(trace: &TimedTrace, from: &[bool], until: &[bool], horizon: u64) -> MeasureReading {
    timed_steps(
        trace,
        TemporalPosition::new(0),
        from,
        until,
        &number(horizon),
        &mut meter(),
    )
    .expect("a represented position")
}

fn elapsed(trace: &TimedTrace, from: &[bool], until: &[bool], horizon: u64) -> MeasureReading {
    timed_elapsed(
        trace,
        TemporalPosition::new(0),
        from,
        until,
        &number(horizon),
        &mut meter(),
    )
    .expect("a represented position")
}

#[trace("FSpec-FR-407-AC-21")]
#[test]
fn a_time_window_fraction_divides_by_the_positions_of_the_window() {
    let value = |n: u64, d: u64| Value(number(n).div(&number(d)).expect("a non-zero divisor"));
    // Stamps 0, 1, 5, 6 with P at 1 and 5, over [0, 5]: 2 of 3, not 2 of 6.
    let trace = complete(&[0, 1, 5, 6], 9);
    assert_eq!(fraction(&trace, 0, &only(4, &[1, 2]), 5), value(2, 3));
    // P at the first position stamped 5 only: 1 of 3.
    let trace = complete(&[0, 5, 5, 6], 9);
    assert_eq!(fraction(&trace, 0, &only(4, &[1]), 5), value(1, 3));
    // From the activating position stamped 2, the window is the positions
    // stamped 2 and 7.
    let trace = complete(&[0, 2, 7, 8], 9);
    assert_eq!(fraction(&trace, 1, &only(4, &[1, 2]), 5), value(1, 1));
    // Two positions at stamp 0 are both in [0, 0].
    let trace = complete(&[0, 0, 1], 9);
    assert_eq!(fraction(&trace, 0, &only(3, &[0]), 0), value(1, 2));
}

#[test]
fn steps_within_a_time_horizon_are_censored_or_not_activated() {
    let trace = complete(&[0, 2, 2, 7], 9);
    assert_eq!(
        steps(&trace, &only(4, &[0]), &only(4, &[2]), 5),
        Value(number(2))
    );
    assert_eq!(steps(&trace, &only(4, &[0]), &only(4, &[3]), 5), Censored);
    assert_eq!(
        steps(&trace, &only(4, &[3]), &only(4, &[3]), 5),
        NotActivated
    );
}

#[trace("FSpec-FR-407-AC-26")]
#[test]
fn elapsed_starts_where_the_first_predicate_holds_however_late() {
    let (a, b) = (only(3, &[1]), only(3, &[2]));
    let trace = complete(&[0, 3, 6], 9);
    assert_eq!(elapsed(&trace, &a, &b, 4), Value(number(3)));
    assert_eq!(steps(&trace, &a, &b, 4), Censored);
    assert_eq!(
        elapsed(&complete(&[0, 3, 7], 9), &a, &b, 4),
        Value(number(4))
    );
    assert_eq!(elapsed(&complete(&[0, 3, 8], 9), &a, &b, 4), Censored);
    // A past the horizon of the activating position.
    let late = complete(&[0, 9, 12], 9);
    assert_eq!(elapsed(&late, &a, &b, 4), Value(number(3)));
    assert_eq!(steps(&late, &a, &b, 4), NotActivated);
    // No A at any position of a behaviour that loops on its last one.
    let looping = TimedTrace::lasso(stamps(&[0, 9, 12]), 2, number(1)).expect("a lasso");
    assert_eq!(elapsed(&looping, &only(3, &[]), &b, 4), NotActivated);
}

/// A lasso repeats its last position's letters at later stamps, and a prefix
/// of unknown continuation leaves an open window undecided.
#[trace("FSpec-FR-407-AC-21", "FSpec-FR-407-AC-26")]
#[test]
fn a_lasso_extends_a_window_and_an_open_prefix_leaves_it_pending() {
    // Stamps 0, 2 and then 3, 4, 5 repeating the second position: over
    // [0, 5] five positions, the four repeating the second holding.
    let looping = TimedTrace::lasso(stamps(&[0, 2]), 1, number(1)).expect("a lasso");
    assert_eq!(
        fraction(&looping, 0, &only(2, &[1]), 5),
        Value(number(4).div(&number(5)).expect("a non-zero divisor"))
    );
    let open = TimedTrace::new(stamps(&[0, 2])).expect("a timed trace");
    assert_eq!(fraction(&open, 0, &only(2, &[1]), 5), Pending);
    assert_eq!(steps(&open, &only(2, &[0]), &only(2, &[]), 5), Pending);
    assert_eq!(elapsed(&open, &only(2, &[0]), &only(2, &[]), 5), Pending);
}
