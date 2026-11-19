// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-623 (TC-1583) through `fsl analyze`: what a reduction selection does to
//! an item's states, its counterexample, its fairness and its refusals. The
//! Counter fixture has two interchangeable counters `a` and `b`, with the
//! `counters` population annotated `symmetric`.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, clause, counter_request_with, counter_snapshot, label, request_members, Workspace,
};
use crate::counter::{document, symmetric_document};

fn request(ws: &Workspace, body: &str, items: Value, symmetric: bool) -> std::path::PathBuf {
    let path = counter_request_with(ws, body, request_members(Some(items)));
    let model = if symmetric {
        symmetric_document()
    } else {
        document(false)
    };
    ws.write("model.json", &model);
    ws.write("start.json", &counter_snapshot(&[("a", 0), ("b", 0)]));
    path
}

fn states(item: &Value) -> u64 {
    item["exploration"]["states"]
        .as_str()
        .and_then(|text| text.parse().ok())
        .expect("an exploration state count")
}

/// FR-623-AC-1, AC-5: symmetry gives the unreduced verdict over fewer states.
#[trace("TC-1583", "FR-623-AC-1")]
#[trace("TC-1583", "FR-623-AC-5")]
#[test]
fn symmetry_reduces_and_agrees() {
    let ws = Workspace::new("red-symmetry");
    // A deadlock-freedom item binds no instance, so the counters' symmetry is
    // not broken by a bound `over` object.
    let derived = json!({"item_kind": "deadlock-freedom"});
    let run = analyze(&request(
        &ws,
        "terminal Done using v on M when { self.value = 3 };\n",
        json!([{"claim": "M", "temporal": derived},
            {"claim": "M", "temporal": derived, "reductions": {"symmetry": "declared"}}]),
        true,
    ));
    let (plain, reduced) = (run.item(0), run.item(1));
    assert_eq!(label(&plain), "proved");
    assert_eq!(label(&reduced), "proved");
    assert!(states(&reduced) < states(&plain), "{plain} {reduced}");
    assert_eq!(reduced["record"]["basis"]["type"], "reduced");
    assert_eq!(reduced["record"]["certification"], "uncertified");
}

/// FR-623-AC-1: a refuted claim under partial-order reduction returns a
/// counterexample that replays against the unreduced model.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
fn a_partial_order_counterexample_is_concrete() {
    let ws = Workspace::new("red-por");
    let run = analyze(&request(
        &ws,
        &clause("Fails", "", "always holds(c.value <= 2)"),
        json!([{"claim": "Fails", "reductions": {"partial_order": true}}]),
        true,
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted");
    assert_eq!(item["replay"]["reproduced"], true);
    assert_eq!(run.exit, 10);
}

/// Each-fairness liveness under symmetry agrees with the unreduced verdict.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
fn each_fairness_under_symmetry_agrees() {
    let ws = Workspace::new("red-each");
    let fair = clause(
        "Fair",
        "fair weak each M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let run = analyze(&request(
        &ws,
        &fair,
        json!([{"claim": "Fair"},
            {"claim": "Fair", "reductions": {"symmetry": "declared"}}]),
        true,
    ));
    assert_eq!(label(&run.item(0)), label(&run.item(1)));
}

/// A symmetry declaration over a population the unit does not annotate
/// `symmetric` is a typed refusal of that item, never silently ignored; the
/// other item still runs.
#[trace("TC-1583", "FR-623-AC-4")]
#[test]
fn an_unsound_symmetry_is_refused_not_ignored() {
    let ws = Workspace::new("red-unsound");
    let explicit = json!({"symmetry": [{"population": "counters", "classes": [["a", "b"]]}]});
    let run = analyze(&request(
        &ws,
        &clause("Holds", "", "always holds(c.value <= 3)"),
        json!([{"claim": "Holds", "reductions": explicit}, {"claim": "Holds"}]),
        false,
    ));
    assert_eq!(label(&run.item(0)), "declined");
    assert_eq!(run.item(0)["record"]["cause"]["kind"], "invalid-input");
    assert_eq!(label(&run.item(1)), "proved");
}

/// A state constraint never proves a property past its boundary: the claim
/// that holds everywhere is not `proved` once the constraint prunes states.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
fn a_constraint_never_proves_past_its_boundary() {
    let ws = Workspace::new("red-constraint");
    let body = format!(
        "invariant Low using v on M::Counter at current {{ self.value <= 1 }}\n{}",
        clause("Holds", "", "always holds(c.value <= 3)")
    );
    let run = analyze(&request(
        &ws,
        &body,
        json!([{"claim": "Holds", "reductions": {"constraint": "Low"}}]),
        false,
    ));
    let item = run.item(0);
    assert_ne!(label(&item), "proved", "{item}");
}

/// FR-621 "The record's `backend` member": a malformed reduction selection
/// is a disposition, so its record is `settled_by` `disposition` with no
/// `backend`, though the item was negotiated to a backend.
#[trace("TC-2296", "FR-621-AC-10")]
#[test]
fn a_malformed_reduction_is_settled_by_disposition() {
    let ws = Workspace::new("red-malformed");
    let run = analyze(&request(
        &ws,
        &clause("Holds", "", "always holds(c.value <= 3)"),
        json!([{"claim": "Holds", "reductions": {"partial_order": "yes"}}]),
        true,
    ));
    let record = &run.item(0)["record"];
    assert_eq!(record["result"], "declined", "{record}");
    assert_eq!(record["settled_by"], "disposition", "{record}");
    assert!(record.get("backend").is_none(), "{record}");
}
