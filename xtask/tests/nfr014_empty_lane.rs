// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-014-M-1: a local lane cannot accept a suite run that violates the
//! zero-passing suite policy.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use xtask::test_suites::{evaluate, Counts, Finding, Lane, SuiteId, SuiteKind, SuiteOutcome};

#[test]
#[trace("NFR-014-M-1")]
fn a_local_lane_rejects_a_suite_that_passed_no_tests() {
    let lane = Lane {
        name: "fixture-lane",
        empty: &[],
        extra: &[],
    };
    let suite = SuiteId {
        source: "tests/it/main.rs".to_owned(),
        kind: SuiteKind::Tests,
    };
    let outcome = SuiteOutcome {
        id: suite.clone(),
        running: 0,
        counts: Counts {
            passed: 0,
            failed: 0,
            ignored: 0,
            measured: 0,
            filtered: 0,
        },
    };

    assert_eq!(
        evaluate(
            &lane,
            &BTreeSet::from([suite.clone()]),
            &[outcome],
            Vec::new()
        ),
        [Finding::Undeclared { suite }]
    );
}
