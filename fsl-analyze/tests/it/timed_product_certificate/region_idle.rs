// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3443: the region product has the idle tail of a quiescent state, so a
//! stutter position can fall in any later region (FR-921, FR-922).
//!
//! Every value here is read off the dense pointwise semantics by hand. The
//! positions are the start, the post-state of `step`, and the stutters of an
//! idle tail. A quiescent state may idle and put a stutter at any later
//! instant, or never put one.

use fsl_analyze::analyze::AnalyzeRecord;
use fsl_replay::{ProofBasis, TerminalValue};
use ix_trace_rs::trace;

use super::{settle, PROFILES};
use crate::timed_world::{self, World};

fn outcome(record: &AnalyzeRecord) -> String {
    match record.terminal().map(|terminal| terminal.value()) {
        Some(TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        }) => format!(
            "proved {}/{}",
            value.value().numerator().as_big(),
            value.value().denominator().as_big()
        ),
        Some(TerminalValue::Refuted) => "refuted".to_owned(),
        Some(value) => format!("{value:?}"),
        None => format!("{record:?}"),
    }
}

/// `step` sets `late` and leaves the subject quiescent. It must run by 1 ms.
const FORCED: (&str, &str) = (
    "self.x > 0 ms",
    "time invariant Wait when { not self.late } { self.x <= 1 ms }\n",
);
/// `step` may run strictly between 0 and 1 ms, or never.
const UNFORCED: (&str, &str) = ("self.x > 0 ms and self.x < 1 ms", "");

/// The subject whose `step` has the guard and invariant `subject`, claiming
/// `probability <bound> [ <formula> ]` for each of `claims`.
fn one_step(subject: (&str, &str), claims: &[(String, String, &str)]) -> World {
    let (guard, invariant) = subject;
    let claims = claims
        .iter()
        .map(|(name, bound, formula)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
                 on origin {{ probability {bound} [ {formula} ] }}\n"
            )
        })
        .collect::<String>();
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ {guard} and not self.late }}\n\
         post Stepped using v on M::Call::step {{ self.late = true }}\n\
         {invariant}\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(&["x"], &[("step", &["late", "x"])]),
        PROFILES,
        &body,
    )
}

/// One formula over a subject, with its dense extrema.
struct Dense {
    formula: &'static str,
    minimum: u8,
    maximum: u8,
}

const fn dense(formula: &'static str, minimum: u8, maximum: u8) -> Dense {
    Dense {
        formula,
        minimum,
        maximum,
    }
}

/// `Low` claims `probability >= 0.001` and `High` `probability <= 0.999`. No
/// draw is made, so each extremum is 0 or 1: a minimum of 0 refutes `Low` and
/// a minimum of 1 proves it with that value; a maximum of 1 refutes `High` and
/// a maximum of 0 proves it with that value.
fn assert_extrema(subject: (&str, &str), rows: &[Dense]) {
    let claims = rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| {
            [
                (format!("Low{index}"), ">= 0.001".to_owned(), row.formula),
                (format!("High{index}"), "<= 0.999".to_owned(), row.formula),
            ]
        })
        .collect::<Vec<_>>();
    let world = one_step(subject, &claims);
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        observed.push((
            row.formula,
            outcome(&settle(&world, &format!("Low{index}"))),
            outcome(&settle(&world, &format!("High{index}"))),
        ));
        expected.push((
            row.formula,
            if row.minimum == 1 {
                "proved 1/1"
            } else {
                "refuted"
            }
            .to_owned(),
            if row.maximum == 0 {
                "proved 0/1"
            } else {
                "refuted"
            }
            .to_owned(),
        ));
    }
    assert_eq!(observed, expected);
}

/// Nothing sets `retried`, so every behaviour, before and after `step`, leaves
/// the claim that waits to see it in [1, 2] ms at 0.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn a_quiescent_undecided_region_state_is_certified() {
    assert_extrema(
        ("self.x > 1 ms", ""),
        &[dense("eventually[1 ms, 2 ms] holds(c.retried)", 0, 0)],
    );
}

/// `step` before 1 ms leaves the subject quiescent with `late` true, and an
/// idle position in [2, 3] ms then reads it: the minimum is 0. Never taking
/// `step`, or observing nothing, gives the maximum 1.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn an_idle_position_in_a_later_region_decides_the_window() {
    assert_extrema(
        ("self.x < 1 ms", ""),
        &[dense("always[2 ms, 3 ms] holds(not c.late)", 0, 1)],
    );
}

/// `late` is true from `step`, by 1 ms, so every position in a window from
/// 2 ms reads it true. A scheduler chooses whether there is such a position:
/// `eventually` and `until` need one, and `always` holds without one.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn the_forced_subject_reads_idle_positions_in_its_window() {
    assert_extrema(
        FORCED,
        &[
            dense("always[2 ms, 3 ms] holds(not c.late)", 0, 1),
            dense("always[2 ms, 3 ms] holds(false)", 0, 1),
            dense("eventually[2 ms, 3 ms] holds(c.late)", 0, 1),
            dense("eventually[2 ms, 3 ms] holds(true)", 0, 1),
            dense("holds(true) until[2 ms, 3 ms] holds(c.late)", 0, 1),
            dense("always[2 ms, 3 ms] holds(c.late)", 1, 1),
            dense("eventually[2 ms, 3 ms] holds(not c.late)", 0, 0),
        ],
    );
}

/// With `step` optional, `late` at a position from 2 ms is the scheduler's
/// choice, and so is having a position there.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn the_unforced_subject_reads_idle_positions_in_its_window() {
    assert_extrema(
        UNFORCED,
        &[
            dense("always[2 ms, 3 ms] holds(c.late)", 0, 1),
            dense("always[2 ms, 3 ms] holds(false)", 0, 1),
            dense("eventually[2 ms, 3 ms] holds(c.late)", 0, 1),
            dense("eventually[2 ms, 3 ms] holds(not c.late)", 0, 1),
            dense("eventually[2 ms, 3 ms] holds(true)", 0, 1),
            dense("holds(true) until[2 ms, 3 ms] holds(c.late)", 0, 1),
        ],
    );
}

/// Windows that end at, start at or are the region point 3 ms (the exact
/// route refuses a half-open window, `TimedFormShape`, so the open end is not
/// pinned here). `late` is true from `step`, by 1 ms. One stutter at the point
/// is a position of a closed window ending there and of the punctual window,
/// and a scheduler may put none there: `eventually` needs one and `always`
/// holds without one.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn a_window_ending_at_a_region_point_is_decided() {
    assert_extrema(
        FORCED,
        &[
            dense("always[2 ms, 3 ms] holds(not c.late)", 0, 1),
            dense("eventually[3 ms, 3 ms] holds(c.late)", 0, 1),
            dense("always[3 ms, 3 ms] holds(c.late)", 1, 1),
            dense("eventually[3 ms, 4 ms] holds(c.late)", 0, 1),
            dense("eventually[3 ms, 3 ms] holds(not c.late)", 0, 0),
        ],
    );
}

/// The verdicts of the ticket's own claims, as it states them: with a minimum
/// of 0 and a maximum of 1, each threshold of one half is refuted.
#[test]
#[trace("TC-3443", "FR-922-AC-4")]
fn the_tickets_claims_are_refuted() {
    let claim = |name: &str, bound: &str, formula: &'static str| {
        (name.to_owned(), bound.to_owned(), formula)
    };
    let forced = [
        claim("A", ">= 0.5", "always[2 ms, 3 ms] holds(not c.late)"),
        claim("B", ">= 0.5", "always[2 ms, 3 ms] holds(false)"),
        claim("C", "<= 0.5", "eventually[2 ms, 3 ms] holds(c.late)"),
        claim("D", "<= 0.5", "eventually[2 ms, 3 ms] holds(true)"),
        claim("E", "<= 0.5", "holds(true) until[2 ms, 3 ms] holds(c.late)"),
    ];
    let unforced = [
        claim("A", ">= 0.5", "always[2 ms, 3 ms] holds(c.late)"),
        claim("B", ">= 0.5", "always[2 ms, 3 ms] holds(false)"),
        claim("C", "<= 0.5", "eventually[2 ms, 3 ms] holds(c.late)"),
        claim("D", "<= 0.5", "eventually[2 ms, 3 ms] holds(not c.late)"),
        claim("E", "<= 0.5", "eventually[2 ms, 3 ms] holds(true)"),
        claim("F", "<= 0.5", "holds(true) until[2 ms, 3 ms] holds(c.late)"),
    ];
    for (subject, claims) in [(FORCED, &forced[..]), (UNFORCED, &unforced[..])] {
        let world = one_step(subject, claims);
        let observed = claims
            .iter()
            .map(|(name, _, formula)| (*formula, outcome(&settle(&world, name))))
            .collect::<Vec<_>>();
        let expected = claims
            .iter()
            .map(|(_, _, formula)| (*formula, "refuted".to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(observed, expected);
    }
}
