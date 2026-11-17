// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-092-AC-9 (TC-1041): fsl-eval against an independent evaluator of
//! the standard's bounded past and future definitions, with false extension
//! after closure and the pre-origin extension before position 0, over every
//! small closed trace.
//!
//! The reference below is written from FSpec FR-091 and FR-092 alone. The
//! TL ecosystem's `tl-mltl` is held to the same definitions by FR-932's
//! root-crate suite (`tests/it/mltl_differential.rs`), which reaches it only
//! through `fsl-command`'s `tl_lowering`: fsl-eval names no TL crate, not even
//! as a dev-dependency (ADR-011).

use fsl_eval::temporal::{TemporalOutcome, TemporalTrace};
use ix_trace_rs::trace;

use crate::temporal_trace::{counters, fixture, outcome, violated, EVENT};

/// The atom `holds(c.level = k)` at position `j` under false extension on
/// both sides of a trace of `values`.
fn atom(values: &[i64], j: i64, k: i64) -> bool {
    usize::try_from(j)
        .ok()
        .and_then(|j| values.get(j))
        .is_some_and(|value| *value == k)
}

type Reference = fn(&[i64], i64) -> bool;

/// Each case: a formula over `p = holds(c.level = 0)` and
/// `q = holds(c.level = 1)`, and its reference truth at position `i`.
const CASES: &[(&str, Reference)] = &[
    ("once[0,2] holds(c.level = 1)", |v, i| {
        (0..=2).any(|d| atom(v, i - d, 1))
    }),
    ("historically[1,2] holds(c.level = 0)", |v, i| {
        (1..=2).all(|d| atom(v, i - d, 0))
    }),
    ("historically[0,1] not holds(c.level = 0)", |v, i| {
        (0..=1).all(|d| !atom(v, i - d, 0))
    }),
    (
        "holds(c.level = 0) since[0,2] holds(c.level = 1)",
        |v, i| (0..=2).any(|d| atom(v, i - d, 1) && (0..d).all(|e| atom(v, i - e, 0))),
    ),
    (
        "holds(c.level = 0) triggered[1,2] holds(c.level = 1)",
        |v, i| !(1..=2).any(|d| !atom(v, i - d, 1) && (1..d).all(|e| !atom(v, i - e, 0))),
    ),
    ("eventually[0,2] holds(c.level = 1)", |v, i| {
        (0..=2).any(|d| atom(v, i + d, 1))
    }),
    ("always[1,2] holds(c.level = 0)", |v, i| {
        (1..=2).all(|d| atom(v, i + d, 0))
    }),
    (
        "holds(c.level = 0) until[0,2] holds(c.level = 1)",
        |v, i| (0..=2).any(|d| atom(v, i + d, 1) && (0..d).all(|e| atom(v, i + e, 0))),
    ),
    (
        "holds(c.level = 0) release[0,1] holds(c.level = 1)",
        |v, i| !(0..=1).any(|d| !atom(v, i + d, 1) && (0..d).all(|e| !atom(v, i + e, 0))),
    ),
    (
        "eventually[1,1] once[1,1] not holds(c.level = 0)",
        |v, i| !atom(v, i, 0),
    ),
];

/// Every trace of length 1 to 4 over the values 0, 1 and 2.
fn traces() -> Vec<Vec<i64>> {
    let mut all = Vec::new();
    for len in 1..=4u32 {
        for code in 0..3i64.pow(len) {
            let mut code = code;
            let mut values = Vec::new();
            for _ in 0..len {
                values.push(code % 3);
                code /= 3;
            }
            all.push(values);
        }
    }
    all
}

/// FSpec FR-092-AC-9: under event-position false-extension, `on each`, every
/// case's fsl-eval outcome equals the outcome its reference gives: true at
/// every activation, or false at the first activation that fails.
#[trace("FSpec-TC-1041", "FSpec-FR-092-AC-9")]
#[test]
fn fsl_eval_agrees_with_the_reference_on_the_bounded_past_and_future_fragment() {
    for (body, truth) in CASES {
        let fx = fixture(EVENT, "c: Counter", "on each (q: Counter)", body);
        for values in traces() {
            let len = i64::try_from(values.len()).expect("a small trace");
            let expected = match (0..len).find(|i| !truth(&values, *i)) {
                None => TemporalOutcome::Holds,
                Some(i) => violated(u64::try_from(i).expect("a position")),
            };
            let settled = outcome(
                &fx,
                TemporalTrace::Finite {
                    len: u64::try_from(len).expect("a small trace"),
                },
                counters(&values),
            );
            assert_eq!(settled, expected, "{body} over {values:?}");
        }
    }
}

/// FR-932's pre-origin cases beyond [`CASES`].
const PRE_ORIGIN_CASES: &[(&str, Reference)] = &[
    ("once[1,1] holds(c.level = 0)", |v, i| atom(v, i - 1, 0)),
    ("once[1,1] not holds(c.level = 0)", |v, i| {
        !atom(v, i - 1, 0)
    }),
    ("historically[0,2] holds(c.level = 0)", |v, i| {
        (0..=2).all(|d| atom(v, i - d, 0))
    }),
    ("historically[0,2] true", |_, _| true),
    ("true since[1,1] not holds(c.level = 0)", |v, i| {
        !atom(v, i - 1, 0)
    }),
];

/// FR-932-AC-1: every corpus case, the pre-origin cases included, has the
/// reference outcome in fsl-eval over every small trace.
#[trace("TC-2604", "FR-932-AC-1")]
#[test]
fn fsl_eval_agrees_with_the_reference_on_the_whole_corpus() {
    for (body, truth) in CASES.iter().chain(PRE_ORIGIN_CASES) {
        let fx = fixture(EVENT, "c: Counter", "on each (q: Counter)", body);
        for values in traces() {
            let len = i64::try_from(values.len()).expect("a small trace");
            let expected = match (0..len).find(|i| !truth(&values, *i)) {
                None => TemporalOutcome::Holds,
                Some(i) => violated(u64::try_from(i).expect("a position")),
            };
            let settled = outcome(
                &fx,
                TemporalTrace::Finite {
                    len: u64::try_from(len).expect("a small trace"),
                },
                counters(&values),
            );
            assert_eq!(settled, expected, "{body} over {values:?}");
        }
    }
}
