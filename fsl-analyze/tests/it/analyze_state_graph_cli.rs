// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-628 (TC-1588) through `fsl analyze`: `possible`, `always possible` and
//! `unique path` claims over the Counter fixture, whose `value` counts from 0
//! to 3 and then stops.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, counter_request, label, Workspace};

const CLAIMS: &str = "possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n\
    possible NeverAbove using v over (c: M::Counter) { c.value > 3 }\n\
    always possible BackToZero using v over (c: M::Counter) { c.value = 0 }\n\
    always possible StillReachThree using v over (c: M::Counter) { c.value = 3 }\n\
    unique path OnePath using v over (c: M::Counter) from (c.value = 0) to (c.value = 3);\n";

fn run(name: &str, items: Value) -> crate::analyze_cli_support::Run {
    analyze(&counter_request(&Workspace::new(name), CLAIMS, items))
}

fn steps(path: &Value) -> usize {
    path["steps"].as_array().map_or(usize::MAX, Vec::len)
}

/// FR-628-AC-1: `possible (value = 3)` proves with a three-step witness path
/// that replays, and `possible (value > 3)` refutes.
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
fn a_possible_claim_proves_with_a_replayed_witness_path_and_refutes() {
    let run = run(
        "sg-possible",
        json!([{"claim": "ReachThree"}, {"claim": "NeverAbove"}]),
    );
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved");
    assert_eq!(proved["record"]["certification"], "certified");
    assert_eq!(proved["record"]["evidence"]["arm"], "witness");
    assert_eq!(steps(&proved["record"]["evidence"]["paths"][0]["path"]), 3);
    assert_eq!(proved["record"]["method"], "explicit-state");
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted");
    assert_eq!(refuted["counterexample"]["arm"], "trap");
    assert_eq!(refuted["replay"]["reproduced"], true);
    assert_eq!(run.exit, 10);
}

/// FR-628-AC-2: `always possible (value = 0)` refutes with a path to
/// `value = 1`, from which no step returns to 0.
#[trace("TC-1588", "FR-628-AC-2")]
#[test]
fn an_always_possible_claim_refutes_with_a_path_to_a_trap() {
    let run = run(
        "sg-always",
        json!([{"claim": "BackToZero"}, {"claim": "StillReachThree"}]),
    );
    let refuted = run.item(0);
    assert_eq!(label(&refuted), "refuted");
    assert_eq!(refuted["counterexample"]["arm"], "trap");
    assert_eq!(steps(&refuted["counterexample"]["stem"]), 1);
    assert_eq!(refuted["replay"]["reproduced"], true);
    assert_eq!(label(&run.item(1)), "proved");
    assert_eq!(
        run.item(1)["record"]["basis"]["type"],
        "exhaustive",
        "{}",
        run.item(1)
    );
}

/// FR-628-AC-3: seeded witness sampling is reproducible, and the item echoes
/// the sampling members it ran under.
#[trace("TC-1588", "FR-628-AC-3")]
#[test]
fn sampled_witnesses_repeat_under_one_seed() {
    let items = json!([{"claim": "ReachThree", "temporal": {"witness_samples": 3, "seed": 7}}]);
    let ws = Workspace::new("sg-sampled");
    let path = counter_request(&ws, CLAIMS, items);
    let (first, second) = (analyze(&path), analyze(&path));
    let evidence = &first.item(0)["record"]["evidence"];
    assert_eq!(evidence, &second.item(0)["record"]["evidence"]);
    assert_eq!(evidence["sampled"][0]["source"]["type"], "sampled");
    assert_eq!(evidence["sampled"][0]["source"]["seed"], "7");
    assert_eq!(first.item(0)["record"]["seed"], 7);
    assert_eq!(first.item(0)["record"]["witness_samples"], 3);
    assert_eq!(first.item(0)["request"]["temporal"]["witness_samples"], 3);
}

/// FR-628 "Refusals": `witness_samples` on a claim that is not `possible`
/// refuses the item `invalid-request`, naming the member.
#[trace("TC-1588", "FR-628-AC-3")]
#[test]
fn witness_samples_on_another_form_is_refused_naming_the_member() {
    let run = run(
        "sg-samples-refused",
        json!([{"claim": "BackToZero", "temporal": {"witness_samples": 3}},
            {"claim": "ReachThree", "temporal": {"seed": "not-a-count"}},
            {"claim": "ReachThree"}]),
    );
    for index in [0, 1] {
        let item = run.item(index);
        assert_eq!(
            item["record"]["diagnostic"]["code"],
            "invalid_runtime_input"
        );
        assert!(item["record"]["diagnostic"]["detail"]["member"]
            .as_str()
            .is_some_and(|member| member.starts_with("temporal.")));
    }
    assert_eq!(label(&run.item(2)), "proved");
}

/// FSpec FR-390: a `unique path` claim holds when its one path is the only
/// one, and a budget that stops the exploration settles `incomplete`.
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
fn a_unique_path_claim_is_decided_and_a_budget_stops_it() {
    let run = run(
        "sg-unique",
        json!([{"claim": "OnePath"},
            {"claim": "StillReachThree", "temporal": {"limits": {"max_states": 2}}},
            {"claim": "ReachThree", "temporal": {"limits": {"max_states": 2}}}]),
    );
    assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
    let stopped = run.item(1);
    assert_eq!(label(&stopped), "incomplete", "{stopped}");
    assert_eq!(stopped["record"]["cause"]["kind"], "limit-reached");
    // A sampled witness found before the budget stops the run does not decide
    // the claim: its well-definedness is unchecked (FR-166, FR-169).
    assert_eq!(label(&run.item(2)), "inconclusive", "{}", run.item(2));
}

/// FSpec FR-390: negotiation routes a state-graph item only to a backend whose
/// manifest lists the claim's form, so a backend that lists none is
/// `unsupported`.
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
fn a_backend_that_decides_no_state_graph_form_is_unsupported() {
    let run = run(
        "sg-unsupported",
        json!([{"claim": "ReachThree", "backend": "en-6"}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "unsupported", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"],
        "unsupported-requested-capability"
    );
    assert_eq!(run.exit, 21);
}
