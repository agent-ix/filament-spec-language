// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty checks through the command adapter and its wire document.

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};
use crate::hyper_world::{non_interference, opaque, request_files, Vault};

#[trace("TC-1591", "FR-631-AC-1", "FR-183-AC-1")]
#[test]
fn analyze_hyper_refuted_with_pair() {
    let (unit, model, snapshot) = request_files(Vault::Leaky, false, (1, 0), &non_interference(""));
    let ws = Workspace::new("hyper-wire-refuted");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"items": [{"claim": "NonInterference"}]}),
    ));
    assert_eq!(run.exit, 10, "{}", run.document());
    let item = run.item(0);
    assert_eq!(label(&item), "refuted");
    let found = &item["counterexample"];
    assert_eq!(found["type"], "hyper-counterexample");
    assert_eq!(found["kind"], "lockstep");
    assert_eq!(found["traces"].as_array().map(Vec::len), Some(2));
    assert_eq!(found["traces"][0]["variable"], "a");
    assert_eq!(found["traces"][1]["variable"], "b");
    assert_eq!(item["replay"]["reproduced"], true);
}

#[trace("TC-1591", "FR-631-AC-3", "FR-184-AC-2")]
#[test]
fn analyze_hyper_witness_budget_is_incomplete() {
    let (unit, model, snapshot) = request_files(Vault::Secure, false, (1, 0), &opaque(""));
    let ws = Workspace::new("hyper-wire-witness-budget");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"items": [{"claim": "Opaque", "temporal": {"limits": {"max_witness_set": 1}}}]}),
    ));
    assert_eq!(label(&run.item(0)), "incomplete", "{}", run.document());
    assert_eq!(run.item(0)["record"]["cause"]["limit"], "max_witness_set");
    assert_eq!(run.exit, 22);
}
