// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-623 (TC-1583): a reduction selected in an analyze item never changes
//! the verdict, and a selection the preservation table does not admit is a
//! typed refusal of that item alone. The selections are enumerated over every
//! combination rather than sampled.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, clause, counter_request_with, label, request_members, Workspace,
};
use crate::counter::{document, symmetric_document};

/// The Counter request over two interchangeable counters `a` and `b`.
fn two_counters(ws: &Workspace, body: &str, items: Value) -> std::path::PathBuf {
    let path = counter_request_with(ws, body, request_members(Some(items)));
    ws.write("model.json", &symmetric_document());
    ws.write(
        "start.json",
        &crate::analyze_cli_support::counter_snapshot(&[("a", 0), ("b", 0)]),
    );
    path
}

fn selections() -> Vec<Value> {
    let symmetry = json!([{"population": "counters", "classes": [["a", "b"]]}]);
    vec![
        json!({}),
        json!({"symmetry": symmetry}),
        json!({"partial_order": true}),
        json!({"symmetry": symmetry, "partial_order": true}),
    ]
}

/// FR-623-AC-1: every selection gives the unreduced verdict.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
fn every_selection_keeps_the_verdict() {
    for (name, formula, expected) in [
        ("Holds", "always holds(c.value <= 3)", "proved"),
        ("Fails", "always holds(c.value <= 2)", "refuted"),
    ] {
        let items: Vec<Value> = selections()
            .into_iter()
            .map(|reductions| json!({"claim": name, "reductions": reductions}))
            .collect();
        let ws = Workspace::new(&format!("reductions-{name}"));
        let run = analyze(&two_counters(
            &ws,
            &clause(name, "", formula),
            Value::Array(items),
        ));
        for index in 0..selections().len() {
            assert_eq!(
                label(&run.item(index)),
                expected,
                "{name} selection {index}"
            );
        }
    }
}

/// FR-623-AC-3: partial-order reduction on a bounded MLTL clause is refused
/// for that item only.
#[trace("TC-1583", "FR-623-AC-3")]
#[trace("TC-1080", "FSpec-FR-520-AC-4")]
#[test]
fn a_non_preserving_selection_refuses_its_item_only() {
    let body = "temporal Window using t0 over (c: M::Counter) clock \"model-steps\" on origin \
                { eventually[0, 3] holds(c.value = 3) }\n"
        .to_owned()
        + &clause("Holds", "", "always holds(c.value <= 3)");
    let ws = Workspace::new("reductions-mltl");
    let run = analyze(&two_counters(
        &ws,
        &body,
        json!([{"claim": "Window", "reductions": {"partial_order": true}},
            {"claim": "Holds"}]),
    ));
    let refused = run.item(0);
    assert_eq!(label(&refused), "inconclusive");
    assert_eq!(
        refused["record"]["cause"]["kind"],
        "reduction-not-preserving"
    );
    assert_eq!(refused["record"]["cause"]["reduction"], "partial-order");
    assert_eq!(label(&run.item(1)), "proved");
    assert_eq!(run.exit, 22);
}

/// FR-623-AC-4: an unknown constraint name and an undeclared symmetry refuse.
#[trace("TC-1583", "FR-623-AC-4")]
#[test]
fn malformed_selections_are_refused_typed() {
    let ws = Workspace::new("reductions-malformed");
    let request = two_counters(
        &ws,
        &clause("Holds", "", "always holds(c.value <= 3)"),
        json!([{"claim": "Holds", "reductions": {"constraint": "NoSuchConstraint"}},
            {"claim": "Holds", "reductions": {"symmetry": "declared"}}]),
    );
    // The unit annotates no population `symmetric`, so `"declared"` has no class.
    ws.write("model.json", &document(false));
    let run = analyze(&request);
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["code"],
        "invalid_runtime_input"
    );
    assert_eq!(label(&run.item(1)), "declined");
    assert_eq!(run.item(1)["record"]["cause"]["kind"], "invalid-input");
}
