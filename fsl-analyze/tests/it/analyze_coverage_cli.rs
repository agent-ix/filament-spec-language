// SPDX-License-Identifier: AGPL-3.0-or-later
//! Further `fsl analyze` stubs for FR-621, FR-624, FR-625 and FR-627 to
//! FR-634 (TC-1581 to TC-1594), driven through the driver binary
//! (`$FSL_BIN`) from source plus a `native-analyze/1` request.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, clause, counter_request, counter_request_with, label,
    litmus_request, request_members, Workspace,
};

/// FR-621-AC-4: every category and record result is a registered kind.
#[trace("TC-1581", "FR-621-AC-4")]
#[test]
fn every_written_kind_is_registered() {
    let categories = [
        "success",
        "violation",
        "refusal",
        "unsupported",
        "incomplete",
        "pending",
        "indeterminate",
        "inconclusive",
        "internal-failure",
    ];
    let ws = Workspace::new("cov-kinds");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"},
            {"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}}]),
    ));
    let document = run.document();
    assert_eq!(
        document["items"].as_array().map(Vec::len),
        Some(3),
        "{document}"
    );
    assert!(categories.contains(&document["category"].as_str().unwrap_or("")));
    for item in document["items"].as_array().expect("items") {
        assert!(
            categories.contains(&item["category"].as_str().unwrap_or("")),
            "{item}"
        );
    }
}

/// FR-621-AC-5: the machine-mode document is the same bytes on two runs of
/// one request (the library and driver write one serializer's output).
#[trace("TC-1581", "FR-621-AC-5")]
#[test]
fn the_machine_document_is_deterministic_bytes() {
    let ws = Workspace::new("cov-bytes");
    let path = counter_request(&ws, &below_three(), json!([{"claim": "BelowThree"}]));
    let first = analyze(&path);
    let second = analyze(&path);
    assert!(!first.stdout.is_empty());
    assert_eq!(first.document()["operation"], "analyze");
    assert_eq!(first.document()["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(first.stdout, second.stdout);
}

/// FR-621-AC-7: a counterexample value is spelled in FSpec FR-181's typed
/// form.
#[trace("TC-1581", "FR-621-AC-7")]
#[test]
fn counterexample_values_use_the_fr181_typed_form() {
    let ws = Workspace::new("cov-values");
    let run = analyze(&counter_request(
        &ws,
        &below_three(),
        json!([{"claim": "BelowThree"}]),
    ));
    let text = run.item(0)["counterexample"].to_string();
    assert!(text.contains(r#"{"type":"integer","value":"3"}"#), "{text}");
    assert!(!text.contains(r#"{"integer":"3"}"#), "{text}");
}

/// FR-624-AC-3: a binding refusal refuses the whole request.
#[trace("TC-1584", "FR-624-AC-3")]
#[test]
fn a_binding_refusal_refuses_the_request() {
    let ws = Workspace::new("cov-binding");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current", "model": {"identity": "test/counter"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://test/counter/nowhere", "complete": true,
            "objects": []}],
    });
    ws.write("start.json", snapshot.to_string().as_bytes());
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    let code = run.document()["diagnostics"][0]["code"].clone();
    assert!(code.is_string(), "{code}");
    assert_ne!(
        code, "unknown_wire",
        "the request is read, then its binding refuses"
    );
    assert_ne!(code, "invalid-request", "the request is well formed");
    assert_eq!(run.document()["items"], json!([]));
}

/// FR-625-AC-5: an unbounded parameter with no proof bound is unsupported.
#[trace("TC-1585", "FR-625-AC-5")]
#[test]
#[ignore = "QSL-2122: explicit-state items are not reachable through fsl analyze yet"]
fn an_unbounded_domain_is_unsupported() {
    let ws = Workspace::new("cov-unbounded");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    ws.write("model.json", &crate::counter::document(true));
    let run = analyze(&path);
    let item = run.item(0);
    assert_eq!(label(&item), "unsupported");
    assert_eq!(item["record"]["diagnostic"]["cause"], "unbounded-extent");
}

/// FR-627-AC-2, AC-3: weak each refutes where strong each proves, and the
/// refuted lasso replays with its fairness checked.
#[trace("TC-1587", "FR-627-AC-2")]
#[trace("TC-1587", "FR-627-AC-3")]
#[test]
#[ignore = "QSL-2123: liveness items are not reachable through fsl analyze yet"]
fn weak_and_strong_fairness_separate() {
    let weak = clause(
        "Weak",
        "fair weak each M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let strong = clause(
        "Strong",
        "fair strong each M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let unfair = clause("Unfair", "", "eventually holds(c.value = 3)");
    let ws = Workspace::new("cov-fairness");
    // FR-627-AC-1's staying Counter: without fairness the `setTo(0)`
    // self-loop at 0 is a behaviour that never reaches 3.
    let run = analyze(&crate::analyze_cli_support::staying_counter_request(
        &ws,
        &format!("{weak}{strong}{unfair}"),
        json!([{"claim": "Weak"}, {"claim": "Strong"}, {"claim": "Unfair"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "proved");
    let refuted = run.item(2);
    assert_eq!(label(&refuted), "refuted");
    assert_eq!(refuted["replay"]["reproduced"], true);
    // FSpec FR-364's `TemporalCounterexample` names the lasso's cycle `loop`.
    assert!(refuted["counterexample"]["loop"].is_array(), "{refuted}");
}

/// FR-628-AC-2, AC-3: always-possible refutes, and seeded witnesses repeat.
#[trace("TC-1588", "FR-628-AC-2")]
#[trace("TC-1588", "FR-628-AC-3")]
#[test]
#[ignore = "QSL-2125: state-graph items are not reachable through fsl analyze yet"]
fn always_possible_refutes_and_witnesses_repeat() {
    let body = "always possible BackToZero using t0 over (c: M::Counter) { c.value = 0 }\n\
                possible ReachThree using t0 over (c: M::Counter) { c.value = 3 }\n";
    let items = json!([{"claim": "BackToZero"},
        {"claim": "ReachThree", "temporal": {"witness_samples": 3, "seed": 7}}]);
    let ws = Workspace::new("cov-state-graph");
    let path = counter_request(&ws, body, items);
    let first = analyze(&path);
    assert_eq!(label(&first.item(0)), "refuted");
    let second = analyze(&path);
    assert_eq!(
        first.item(1)["record"]["evidence"],
        second.item(1)["record"]["evidence"]
    );
}

/// FR-629-AC-2, AC-3, AC-5: release/acquire message passing, a race, and a
/// selection naming no weak parallel.
#[trace("TC-1589", "FR-629-AC-2")]
#[trace("TC-1589", "FR-629-AC-3")]
#[trace("TC-1589", "FR-629-AC-5")]
#[test]
#[ignore = "QSL-2126: weak-memory items are not reachable through fsl analyze yet"]
fn release_acquire_races_and_selections() {
    let body = crate::litmus::sb_unit(&crate::litmus::sb_run("memory ra", "release", "acquire"));
    let ws = Workspace::new("cov-ra");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"items": [{"claim": "NoBothZero"},
            {"claim": "Both", "temporal": {"item_kind": "race-freedom"}}]}),
    ));
    assert!(matches!(label(&run.item(0)), "proved" | "refuted"));
    assert!(matches!(label(&run.item(1)), "proved" | "refuted"));
    let ws = Workspace::new("cov-ra-selection");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"memory": [{"parallel": "Nowhere", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
}

/// FR-631-AC-2, AC-3: copy-swap reduction and the witness-set budget.
#[trace("TC-1591", "FR-631-AC-2")]
#[trace("TC-1591", "FR-631-AC-3")]
#[test]
#[ignore = "QSL-2128: hyperproperty items are not reachable through fsl analyze yet"]
fn copy_swap_and_witness_budget() {
    use crate::hyper_world::{non_interference, opaque, request_files, Vault};
    let declaration = format!("{}\n{}", non_interference(""), opaque(""));
    let (unit, model, snapshot) = request_files(Vault::Secure, false, (1, 0), &declaration);
    let ws = Workspace::new("cov-hyper");
    let run = analyze(&crate::analyze_cli_support::request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"items": [
            {"claim": "NonInterference", "reductions": {"symmetry": "declared"}},
            {"claim": "Opaque", "temporal": {"limits": {"max_witness_set": 1}}},
        ]}),
    ));
    assert_eq!(run.item(0)["record"]["basis"]["type"], "reduced");
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    assert_eq!(label(&run.item(1)), "incomplete");
    assert_eq!(run.item(1)["record"]["cause"]["limit"], "max_witness_set");
}

/// FR-632-AC-2, AC-3: digital clocks agree with zones, and a clock budget
/// of 0 stops the run.
#[trace("TC-1592", "FR-632-AC-2")]
#[trace("TC-1592", "FR-632-AC-3")]
#[test]
#[ignore = "QSL-2129: timed zone items are not reachable through fsl analyze yet"]
fn digital_clocks_agree_and_the_clock_budget_stops() {
    let claim = "temporal Bound using tt over (c: M::Call) clock \"model-time\" on origin \
                 { always holds(not c.late) }\n";
    let ws = Workspace::new("cov-zone");
    let run = analyze(&crate::analyze_family_cli::rpc_request(
        &ws,
        4,
        claim,
        json!({"items": [{"claim": "Bound"},
            {"claim": "Bound", "options": {"method": "digital-clocks"}},
            {"claim": "Bound", "temporal": {"limits": {"max_clocks": 0}}}]}),
    ));
    assert_eq!(label(&run.item(0)), label(&run.item(1)));
    assert_eq!(label(&run.item(2)), "incomplete");
    assert_eq!(run.item(2)["record"]["cause"]["limit"], "max_clocks");
}

/// FR-633-AC-3: EDF proves `Ctl` in closed form.
#[trace("TC-1593", "FR-633-AC-3")]
#[test]
#[ignore = "QSL-2131: schedulability items are not reachable through fsl analyze yet"]
fn edf_proves_ctl() {
    let ws = Workspace::new("cov-edf");
    let run = analyze(&crate::analyze_family_cli::ctl_request(
        &ws,
        "schedulable CtlEdf on Ctl under edf;\n",
        json!([{"claim": "CtlEdf"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["basis"]["analysis"], "edf");
}

/// FR-634-AC-2, AC-3: a state budget leaves a frontier, and seeded sampling
/// repeats.
#[trace("TC-1594", "FR-634-AC-2")]
#[trace("TC-1594", "FR-634-AC-3")]
#[test]
#[ignore = "QSL-2132: simulation items are not reachable through fsl analyze yet"]
fn simulation_budget_and_seeded_sampling() {
    let ws = Workspace::new("cov-simulation");
    let request = request_members(Some(json!([
        {"kind": "simulation", "simulation": {"mode": "enumerate", "max_states": 2}},
        {"kind": "simulation", "simulation": {"mode": "sample", "seed": 5, "samples": 3}},
    ])));
    let path = counter_request_with(&ws, "", request);
    let first = analyze(&path);
    let stopped = first.item(0);
    assert_eq!(stopped["category"], "incomplete");
    assert!(stopped["frontier"]
        .as_array()
        .is_some_and(|frontier| !frontier.is_empty()));
    let second = analyze(&path);
    let sampled = |run: &crate::analyze_cli_support::Run| -> Value { run.item(1) };
    assert_eq!(sampled(&first), sampled(&second));
}
