// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-623 (TC-1583) through `fsl analyze`: what a reduction selection does to
//! an item's states, its counterexample, its fairness and its refusals. The
//! Counter fixture has two interchangeable counters `a` and `b`, with the
//! `counters` population annotated `symmetric`.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Workspace, analyze, clause, counter_request_with, counter_snapshot, label,
    package_selection, request_members,
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

/// FSpec FR-383's pure-call reads preserve the verdict and concrete replay
/// for a genuine admitted atom. QSL-2829 implements the missing atom rule.
#[trace("TC-1583", "FR-623-AC-1")]
#[test]
#[ignore = "QSL-2829: partial-order reduction has no atom footprint for function calls yet"]
fn an_atom_pure_call_preserves_the_verdict_and_replayed_refutations() {
    let ws = Workspace::new("pure-call-por");
    let body = "function same_truth using v(x: Boolean): Boolean pure { x }\n".to_owned()
        + &clause("Fails", "", "always holds(same_truth(c.value <= 2))");
    let run = analyze(&request(
        &ws,
        &body,
        json!([
            {"claim": "Fails"},
            {"claim": "Fails", "reductions": {"partial_order": true}}
        ]),
        true,
    ));
    for index in [0, 1] {
        let item = run.item(index);
        assert_eq!(label(&item), "refuted");
        assert_eq!(item["replay"]["reproduced"], true);
    }
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

/// TC-581: a request naming Low runs AC-1's boundary counterexample and
/// retains the selected constraint's node key in its request echo. A request naming the
/// invariant Wide by name or package node key refuses before any state is
/// expanded, and a constraint anchored at an operation refuses during source
/// compilation.
#[trace("TC-581", "FR-158-AC-6")]
#[test]
fn constraint_request_key_truth_and_invalid_selections_are_typed() {
    let body = format!(
        "constraint Low using v on M::Counter at current {{ self.value <= 1 }}\n\
         invariant Wide using v on M::Counter at current {{ self.value <= 3 }}\n{}",
        clause("Claim", "", "always holds(c.value <= 1)")
    );
    let ws = Workspace::new("fr158-constraint-wire-key");
    let selection = request_members(Some(json!([
        {"claim": "Claim", "reductions": {"constraint": "Low"}}
    ])));
    let _ = counter_request_with(&ws, &body, selection.clone());
    ws.write(
        "start.json",
        &counter_snapshot(&[("a", 0), ("b", 0), ("c", 0)]),
    );
    let (selected, package_bytes) = package_selection(&ws, &selection);
    assert!(
        !package_bytes.is_empty(),
        "the compiled request has package bytes"
    );
    let package: Value = serde_json::from_slice(&package_bytes).expect("the package is JSON");
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("the package emits semantic nodes");
    let wide_key = nodes
        .iter()
        .find(|node| {
            node["node_tag"] == "state"
                && node["semantic_form"] == "state_clause"
                && node["body"]["operation"]["member"]["clause"] == "invariant"
        })
        .expect("Wide emits an invariant state-clause node")["node_id"]
        .clone();
    let package_path = ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": selected})
            .to_string()
            .as_bytes(),
    );
    let valid = analyze(&package_path).item(0);
    assert_eq!(label(&valid), "refuted", "{valid}");
    assert_eq!(valid["record"]["result"], "refuted", "{valid}");
    assert_eq!(valid["replay"]["reproduced"], true, "{valid}");
    assert_eq!(
        valid["request"]["reductions"]["constraint"],
        json!({
            "ordinal": 0,
            "package": "analyze-unit",
            "path": [
                "state",
                "state_clause",
                "constraint",
                {"node": 8},
                {"node": 22}
            ]
        }),
        "the request echoes Low's canonical node key: {valid}"
    );
    let run_invalid = |name: &str, constraint: Value| {
        let ws = Workspace::new(name);
        let path = counter_request_with(
            &ws,
            &body,
            request_members(Some(json!([
                {"claim": "Claim", "reductions": {"constraint": constraint}}
            ]))),
        );
        ws.write(
            "start.json",
            &counter_snapshot(&[("a", 0), ("b", 0), ("c", 0)]),
        );
        analyze(&path).item(0)
    };
    let item = run_invalid("state-constraint-invalid", json!("Absent"));
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["name"], "Absent",
        "{item}"
    );
    assert!(
        item["exploration"].is_null(),
        "no state was expanded: {item}"
    );

    let invalid_package = |name: &str, constraint: Value| {
        let ws = Workspace::new(name);
        let selection = request_members(Some(json!([
            {"claim": "Claim", "reductions": {"constraint": constraint}}
        ])));
        let _ = counter_request_with(&ws, &body, selection.clone());
        let (selected, _) = package_selection(&ws, &selection);
        ws.write(
            "analyze-package.json",
            json!({"format": "native-analyze/1", "request": selected})
                .to_string()
                .as_bytes(),
        )
    };
    for (name, key) in [
        ("state-constraint-invariant-name", json!("Wide")),
        ("state-constraint-invariant-key", wide_key.clone()),
    ] {
        let invalid = analyze(&invalid_package(name, key.clone())).item(0);
        assert_eq!(
            invalid["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{invalid}"
        );
        assert_eq!(
            invalid["record"]["diagnostic"]["cause"], "invalid-value",
            "{invalid}"
        );
        let diagnostic_name = &invalid["record"]["diagnostic"]["detail"]["name"];
        if name == "state-constraint-invariant-key" {
            assert_eq!(
                diagnostic_name,
                &json!({
                    "ordinal": 0,
                    "package": "analyze-unit",
                    "path": [
                        "state",
                        "state_clause",
                        "invariant",
                        {"node": 8},
                        {"node": 23}
                    ]
                }),
                "the refusal names the canonical node key for Wide: {invalid}"
            );
        } else {
            assert_eq!(diagnostic_name, &key, "{invalid}");
        }
        assert!(
            invalid["exploration"].is_null(),
            "no state was expanded: {invalid}"
        );
    }

    let bad_ws = Workspace::new("state-constraint-operation-anchor");
    let bad = "constraint Bad using v on M::Counter::bump { true }\n";
    let _ = counter_request_with(&bad_ws, bad, request_members(None));
    let source_request = request_members(None);
    let compile_request = json!({"format": "native-compile/1", "request": {
        "models": source_request["models"], "program": source_request["program"],
        "handoff": {"directory": HANDOFF, "package_name": "bad-constraint",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}
    }});
    let compile_path = bad_ws.write("compile.json", compile_request.to_string().as_bytes());
    let refusal = match fsl_command::command::compile_outcome(&compile_path) {
        Err(refusal) => refusal,
        Ok(_) => panic!("Bad cannot anchor a state constraint at an operation"),
    };
    let diagnostic: Value = serde_json::from_slice(
        &refusal
            .outcome_document(fsl_replay::Operation::Package)
            .to_bytes()
            .expect("the typed refusal serializes"),
    )
    .expect("the refusal document is JSON");
    let source_diagnostic = &diagnostic["diagnostics"][0];
    assert_eq!(source_diagnostic["code"], "invalid_syntax", "{diagnostic}");
    assert_eq!(source_diagnostic["locus"]["kind"], "region", "{diagnostic}");
    assert!(
        source_diagnostic["locus"]["source"].is_object(),
        "{diagnostic}"
    );
    assert!(
        source_diagnostic["locus"]["start"].as_u64().is_some(),
        "{diagnostic}"
    );
    assert!(
        source_diagnostic["locus"]["end"].as_u64().is_some(),
        "{diagnostic}"
    );
    assert!(
        source_diagnostic["locus"]["end"].as_u64() > source_diagnostic["locus"]["start"].as_u64(),
        "{diagnostic}"
    );
}

/// Partial TC-581 coverage: the constrained and unconstrained requests keep
/// the same claim identity while receiving their own independent outcomes.
#[trace("TC-581")]
#[test]
fn a_constraint_does_not_cross_settle_the_same_claim_without_a_constraint() {
    let ws = Workspace::new("fr158-independent-method-results");
    let body = format!(
        "constraint Low using v on M::Counter at current {{ self.value <= 1 }}\n{}",
        clause("Claim", "", "always holds(c.value <= 3)")
    );
    let selections = json!([
        {"claim": "Claim", "reductions": {"constraint": "Low"}},
        {"claim": "Claim"}
    ]);
    let path = counter_request_with(&ws, &body, request_members(Some(selections.clone())));
    ws.write(
        "start.json",
        &counter_snapshot(&[("a", 0), ("b", 0), ("c", 0)]),
    );
    let run = analyze(&path);
    let constrained = run.item(0);
    let unconstrained = run.item(1);

    assert_ne!(selections[0], selections[1], "the method requests differ");
    assert_ne!(constrained["request_index"], unconstrained["request_index"]);
    assert_eq!(
        constrained["request"]["claim"],
        unconstrained["request"]["claim"]
    );
    assert!(constrained["request"]["reductions"]["constraint"].is_object());
    assert!(unconstrained["request"]["reductions"].is_null());
    assert_eq!(label(&constrained), "inconclusive", "{constrained}");
    assert_eq!(
        constrained["record"]["cause"]["kind"], "constraint-reached",
        "{constrained}"
    );
    assert_eq!(label(&unconstrained), "proved", "{unconstrained}");
    assert_eq!(
        unconstrained["record"]["result"], "proved",
        "{unconstrained}"
    );
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
