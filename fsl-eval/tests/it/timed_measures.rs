// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-407 "Bounded measures" and "Time windows" on one timed trace:
//! the reference readings of `elapsed`, `steps`, `accumulate` and a
//! `fraction`. Every expected value is the one the criterion states.

use ix_trace_rs::trace;

use filament_exact::{Integer, Meter, Rational, ScalarLimits};
use fsl_eval::temporal::{
    timed_accumulate, timed_delay_fraction, timed_elapsed, timed_fraction, timed_steps,
    timed_weighted_fraction, MeasureReading, TemporalPosition, TimedTrace,
};

use MeasureReading::{Censored, NoWeight, NotActivated, Pending, Value};

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

#[trace("FSpec-FR-407-AC-23")]
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

#[trace("FSpec-FR-407-AC-23")]
#[test]
fn accumulate_sums_the_rewards_of_the_steps_between_the_two_positions() {
    let trace = complete(&[0, 2, 2, 7], 9);
    // The rewards of the steps into positions 1, 2 and 3 are 1, 3 and 2.
    let rewards = stamps(&[0, 1, 3, 2]);
    let accumulate = |from: &[bool], until: &[bool]| {
        timed_accumulate(
            &trace,
            TemporalPosition::new(0),
            from,
            until,
            &rewards,
            &number(5),
            &mut meter(),
        )
        .expect("a represented position")
    };
    assert_eq!(accumulate(&only(4, &[0]), &only(4, &[2])), Value(number(4)));
    assert_eq!(accumulate(&only(4, &[0]), &only(4, &[3])), Censored);
    assert_eq!(accumulate(&only(4, &[3]), &only(4, &[3])), NotActivated);
}

#[trace("FSpec-FR-407-AC-20", "FSpec-FR-407-AC-22")]
#[test]
fn a_weighted_fraction_weighs_the_step_or_the_delay_that_leaves_a_position() {
    let value = |n: u64, d: u64| Value(number(n).div(&number(d)).expect("a non-zero divisor"));
    let weighted = |trace: &TimedTrace, holds: &[bool], weights: &[u64]| {
        timed_weighted_fraction(
            trace,
            TemporalPosition::new(0),
            holds,
            &stamps(weights),
            &number(5),
            &mut meter(),
        )
        .expect("a represented position")
    };
    // The step leaving position 1 lands at 7, past the window, and weighs it.
    let trace = complete(&[0, 2], 3);
    assert_eq!(weighted(&trace, &only(2, &[1]), &[1, 3]), value(3, 4));
    assert_eq!(weighted(&trace, &only(2, &[1]), &[1, 0]), value(0, 1));
    assert_eq!(weighted(&trace, &only(2, &[1]), &[0, 0]), NoWeight);
    // Stamps 0 and 1, then idle: weights 1 and 4 - 1 = 3 within [0, 4].
    let trace = complete(&[0, 1], 3);
    assert_eq!(
        timed_delay_fraction(
            &trace,
            TemporalPosition::new(0),
            &only(2, &[0]),
            &number(4),
            &mut meter(),
        )
        .expect("a represented position"),
        value(1, 4)
    );
}
