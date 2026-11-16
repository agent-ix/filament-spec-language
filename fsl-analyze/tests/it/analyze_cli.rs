// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-624 to FR-636 (TC-1584 to TC-1596): one `fsl analyze` per engine family
//! settles to the documented FR-621 item, and every refusal and every unrouted
//! request exits with a named outcome document. Driven through the in-process
//! command adapter (`fsl_command::command::run`), the library half of the
//! driver's `analyze` verb; the driver's own rendering of unrouted verbs and
//! text mode is tested in the driver repository.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, clause, counter_request, label, litmus_request, order_protocol,
    Workspace,
};

const DONE: &str = "terminal Done using v on M when { self.value = 3 };\n";

/// FR-624-AC-1: a malformed document is a named refusal.
#[trace("TC-1584", "FR-624-AC-1")]
#[test]
fn a_malformed_request_is_a_named_refusal() {
    let ws = Workspace::new("cli-malformed");
    let path = ws.write(
        "analyze.json",
        b"{\"format\": \"native-analyze/1\", \"request\": 7}",
    );
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["operation"], "analyze");
    assert_eq!(document["category"], "refusal");
    assert_eq!(document["items"], json!([]));
    assert_eq!(document["diagnostics"][0]["code"], "invalid-request");
}

/// FR-624-AC-2: a checker refusal of the program refuses the request at the
/// checker stage, which FR-286 spells `S3`; FR-624-AC-2 still says `check`, a
/// wording conflict tracked as QSL-2288.
#[trace("TC-1584", "FR-624-AC-2")]
#[test]
fn a_refused_program_refuses_at_its_stage() {
    let ws = Workspace::new("cli-check-refusal");
    let path = counter_request(
        &ws,
        "function Bad using v(): Boolean pure { 1 }\n",
        json!([]),
    );
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    // FR-286 spells the checker stage `S3` in `last_stage` (FR-286-AC-6).
    assert_eq!(run.document()["last_stage"], "S3");
    assert_eq!(run.document()["diagnostics"][0]["code"], "ill_typed");
    assert_eq!(run.document()["items"], json!([]));
}

/// FR-624-AC-1, FSpec FR-522-AC-2: `analyze` is routed through the command
/// adapter. A well-formed request is never answered with the
/// `operation-not-routed` refusal or an empty document; it writes an
/// `analyze` document with one item per requested item.
#[trace("TC-1584", "FR-624-AC-1")]
#[trace("TC-1082", "FSpec-FR-522-AC-2")]
#[test]
fn analyze_is_routed_and_never_refused_as_not_routed() {
    let ws = Workspace::new("cli-routed");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"}]),
    ));
    assert!(!run.stdout.is_empty(), "the outcome is never empty");
    let document = run.document();
    assert_ne!(document["diagnostics"][0]["cause"], "operation-not-routed");
    assert_ne!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    assert_eq!(document["operation"], "analyze");
    assert_eq!(
        document["items"].as_array().map(Vec::len),
        Some(1),
        "{document}"
    );
}

/// FR-625-AC-1, AC-2: one accepting and one refuted explicit-state item.
#[trace("TC-1585", "FR-625-AC-1")]
#[trace("TC-1585", "FR-625-AC-2")]
#[test]
#[ignore = "QSL-2122: explicit-state model-check items are not reachable through fsl analyze yet"]
fn explicit_state_items_prove_and_refute() {
    let ws = Workspace::new("cli-mc");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["basis"]["type"], "exhaustive");
    assert_eq!(label(&run.item(1)), "refuted");
    assert_eq!(run.item(1)["replay"]["reproduced"], true);
    assert_eq!(run.exit, 10);
}

/// FR-625-AC-4: an invariant state clause is an explicit-state item.
#[trace("TC-1585", "FR-625-AC-4")]
#[test]
#[ignore = "QSL-2122: invariant clauses are not reachable through fsl analyze yet"]
fn invariant_items_prove_and_refute() {
    let ws = Workspace::new("cli-invariant");
    let body = "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n\
                invariant AtMostTwo using v on M::Counter at current { self.value <= 2 }\n";
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "AtMostThree"}, {"claim": "AtMostTwo"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "refuted");
}

/// FR-626-AC-1, AC-2: deadlock freedom respects intended terminal states.
#[trace("TC-1586", "FR-626-AC-1")]
#[trace("TC-1586", "FR-626-AC-2")]
#[test]
#[ignore = "QSL-2122: deadlock-freedom items are not reachable through fsl analyze yet"]
fn deadlock_freedom_respects_terminal_states() {
    let item = json!([{"claim": "M", "temporal": {"item_kind": "deadlock-freedom"}}]);
    let ws = Workspace::new("cli-deadlock");
    let run = analyze(&counter_request(&ws, "", item.clone()));
    assert_eq!(label(&run.item(0)), "refuted");
    let ws = Workspace::new("cli-deadlock-terminal");
    let run = analyze(&counter_request(&ws, DONE, item));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-626-AC-3: time-lock freedom over an untimed subject is inapplicable.
#[trace("TC-1586", "FR-626-AC-3")]
#[test]
#[ignore = "QSL-2122: time-lock-freedom items are not reachable through fsl analyze yet"]
fn time_lock_freedom_over_an_untimed_subject_is_inapplicable() {
    let ws = Workspace::new("cli-time-lock");
    let run = analyze(&counter_request(
        &ws,
        "",
        json!([{"claim": "M", "temporal": {"item_kind": "time-lock-freedom"}}]),
    ));
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["cause"],
        "inapplicable-kind"
    );
}

/// FR-627-AC-1: weak fairness proves a liveness item that fails without it.
#[trace("TC-1587", "FR-627-AC-1")]
#[test]
#[ignore = "QSL-2123: liveness items are not reachable through fsl analyze yet"]
fn weak_fairness_decides_liveness() {
    let fair = clause(
        "Fair",
        "fair weak M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let unfair = clause("Unfair", "", "eventually holds(c.value = 3)");
    let ws = Workspace::new("cli-liveness");
    let run = analyze(&counter_request(
        &ws,
        &format!("{fair}{unfair}"),
        json!([{"claim": "Fair"}, {"claim": "Unfair"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "refuted");
}

/// FR-627-AC-4: fairness over a supplied trace is a missing premise.
#[trace("TC-1587", "FR-627-AC-4")]
#[test]
#[ignore = "QSL-2123: finite-replay items with fairness are not reachable through fsl analyze yet"]
fn strong_fairness_over_a_supplied_trace_is_refused() {
    let strong = clause(
        "Strong",
        "fair strong M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let ws = Workspace::new("cli-fair-trace");
    let trace = json!({"format": "filament.protocol-trace/v1", "identity":
        {"authority": "test", "identity": "recorded"}, "steps": []});
    ws.write("recorded.json", trace.to_string().as_bytes());
    let mut request = crate::analyze_cli_support::request_members(Some(json!([
        {"claim": "Strong", "kind": "finite-replay",
         "trace": {"authority": "test", "identity": "recorded"}}
    ])));
    request["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "recorded.json"}));
    let run = analyze(&crate::analyze_cli_support::counter_request_with(
        &ws, &strong, request,
    ));
    assert_eq!(run.exit, 21);
    let item = run.item(0);
    assert_eq!(label(&item), "unsupported");
    assert_eq!(item["record"]["cause"]["type"], "missing-fairness-premise");
}

/// FR-628-AC-1: a possible claim proves with a witness and refutes.
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
#[ignore = "QSL-2125: state-graph items are not reachable through fsl analyze yet"]
fn possible_claims_prove_and_refute() {
    let body = "possible ReachThree using t0 over (c: M::Counter) { c.value = 3 }\n\
                possible ReachFour using t0 over (c: M::Counter) { c.value = 4 }\n";
    let ws = Workspace::new("cli-state-graph");
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "ReachThree"}, {"claim": "ReachFour"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "refuted");
}

/// FR-629-AC-4: a liveness item over a release/acquire parallel always has
/// a terminal record, with a catalog code when it is not decided.
#[trace("TC-1589", "FR-629-AC-4")]
#[test]
#[ignore = "QSL-2218: liveness over a release/acquire parallel is not reachable through fsl analyze yet"]
fn liveness_over_a_ra_parallel_is_settled_by_name() {
    let body = crate::litmus::sb_unit(&crate::litmus::sb_run("memory ra", "release", "acquire"))
        + "temporal Live using t0 over (k: M::Core) clock \"model-steps\" on origin \
           { eventually holds(k.r1 = 0) }\n";
    let ws = Workspace::new("cli-ra-liveness");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"items": [{"claim": "Live"}]}),
    ));
    let item = run.item(0);
    let result = label(&item);
    if !matches!(result, "proved" | "refuted") {
        assert!(item["record"]["diagnostic"]["code"].is_string(), "{item}");
    }
}

/// FR-629-AC-1: the store-buffering claim holds under SC and fails under TSO.
#[trace("TC-1589", "FR-629-AC-1")]
#[test]
#[ignore = "QSL-2126: weak-memory items are not reachable through fsl analyze yet"]
fn store_buffering_separates_sc_and_tso() {
    for (memory, expected) in [("memory sc", "proved"), ("memory tso", "refuted")] {
        let body = crate::litmus::sb_unit(&crate::litmus::sb_run(memory, "release", "acquire"));
        let ws = Workspace::new(&format!("cli-sb-{}", memory.replace(' ', "-")));
        let run = analyze(&litmus_request(
            &ws,
            &body,
            json!({"items": [{"claim": "NoBothZero"}]}),
        ));
        assert_eq!(label(&run.item(0)), expected, "{memory}");
    }
}

/// FR-634-AC-1: enumerating the Counter subject reports coverage and traces.
#[trace("TC-1594", "FR-634-AC-1")]
#[test]
#[ignore = "QSL-2132: simulation items are not reachable through fsl analyze yet"]
fn simulation_enumerates_the_counter_subject() {
    let ws = Workspace::new("cli-simulation");
    let run = analyze(&counter_request(
        &ws,
        "",
        json!([{"kind": "simulation", "simulation": {"mode": "enumerate"}}]),
    ));
    let item = run.item(0);
    assert_eq!(item["category"], "success");
    assert_eq!(item["coverage"]["states"], "4");
    assert_eq!(item["coverage"]["transitions"], "3");
    assert_eq!(item["frontier"], json!([]));
    assert!(item.get("record").is_none());
}

/// FR-635-AC-3: an unregistered backend is an invalid request.
#[trace("TC-1595", "FR-635-AC-3")]
#[test]
#[ignore = "QSL-2132: backend selection is not reachable through fsl analyze yet"]
fn an_unregistered_backend_is_refused() {
    let ws = Workspace::new("cli-backend");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded", "backend": "no-such-backend"}]),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["cause"],
        "unknown-backend"
    );
    let registry: Vec<Value> = run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .map(|entry| entry["backend"].clone())
        .collect();
    assert!(registry.contains(&json!("en-1")) && registry.contains(&json!("en-7")));
}

/// FR-636-AC-1: the order protocol's deadlock item proves, and refutes
/// without the receive.
#[trace("TC-1596", "FR-636-AC-1")]
#[test]
#[ignore = "QSL-2224: protocol analysis items are not reachable through fsl analyze yet"]
fn protocol_deadlock_proves_and_refutes() {
    for (receive, expected) in [(true, "proved"), (false, "refuted")] {
        let ws = Workspace::new(&format!("cli-protocol-{receive}"));
        let run = analyze(&litmus_request(
            &ws,
            &order_protocol(receive),
            json!({"items": [{"claim": "Flow", "temporal": {"item_kind": "deadlock-freedom"}}]}),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), expected, "receive {receive}");
        if !receive {
            assert!(item["counterexample"]["witness"].is_object(), "{item}");
            assert_eq!(item["replay"]["reproduced"], true);
        }
    }
}
