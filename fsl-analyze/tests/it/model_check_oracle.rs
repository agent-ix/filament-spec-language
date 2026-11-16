// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-625 (TC-1585): the explicit-state verdict and exploration counts that
//! `fsl analyze` reports for the Counter subject equal an independent
//! breadth-first oracle over the same transition rule.

use std::collections::{BTreeSet, VecDeque};

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze, bounded, counter_request, label, Workspace};

/// The Counter subject's reachable states and steps from `start`, with
/// `inc` enabled below 3 and adding 1, counted independently of EN-1.
fn oracle(start: i64) -> (usize, usize) {
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let mut steps = 0;
    while let Some(value) = queue.pop_front() {
        if value < 3 {
            steps += 1;
            if seen.insert(value + 1) {
                queue.push_back(value + 1);
            }
        }
    }
    (seen.len(), steps)
}

/// FR-625-AC-1: verdict and counts agree with the oracle.
#[trace("TC-1585", "FR-625-AC-1")]
#[test]
#[ignore = "QSL-2122: explicit-state items and their exploration counts are not reachable through fsl analyze yet"]
fn verdict_and_counts_match_the_breadth_first_oracle() {
    let (states, steps) = oracle(0);
    assert_eq!((states, steps), (4, 3));
    let ws = Workspace::new("oracle");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved");
    assert_eq!(item["record"]["certification"], "certified");
    assert_eq!(item["exploration"]["states"], states.to_string());
    assert_eq!(item["exploration"]["transitions"], steps.to_string());
}

/// FR-625-AC-3: a state budget and a horizon stop the search incomplete.
#[trace("TC-1585", "FR-625-AC-3")]
#[test]
#[ignore = "QSL-2122: explicit-state budgets are not reachable through fsl analyze yet"]
fn budgets_settle_incomplete_with_what_was_explored() {
    let ws = Workspace::new("oracle-budget");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}},
            {"claim": "Bounded", "temporal": {"max_depth": 1}}]),
    ));
    assert_eq!(label(&run.item(0)), "incomplete");
    assert_eq!(run.item(0)["exploration"]["states"], "2");
    // A horizon is not a resource limit: FR-127's V-5 row settles the cut
    // `inconclusive` `bound-reached` at the requested depth.
    assert_eq!(label(&run.item(1)), "inconclusive");
    assert_eq!(run.item(1)["record"]["cause"]["kind"], "bound-reached");
    assert_eq!(run.item(1)["record"]["cause"]["depth"], 1);
    assert_eq!(run.exit, 22);
}
