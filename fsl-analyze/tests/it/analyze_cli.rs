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
#[ignore = "QSL-2301: needs an EN-1 invariant item form (translation, closure certificate, replayed state-clause counterexample) and the record backend member FR-621 states"]
fn invariant_items_prove_and_refute() {
    let ws = Workspace::new("cli-invariant");
    let body = "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n\
                invariant AtMostTwo using v on M::Counter at current { self.value <= 2 }\n";
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "AtMostThree"}, {"claim": "AtMostTwo"},
            {"claim": "AtMostThree", "kind": "temporal-satisfaction"}]),
    ));
    // An invariant keeps its `state-model` family's one kind; over the
    // request's model subject the tie-break routes it to EN-1, never to the
    // contract checker (FR-489).
    assert_eq!(run.item(0)["request"]["kind"], "operation-contract");
    assert_eq!(run.item(0)["record"]["backend"], "en-1");
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["basis"]["type"], "exhaustive");
    assert_eq!(label(&run.item(1)), "refuted");
    assert_eq!(run.item(1)["record"]["backend"], "en-1");
    let refused = run.item(2);
    assert_eq!(
        refused["record"]["diagnostic"]["cause"],
        "inapplicable-kind"
    );
    assert_eq!(
        refused["record"]["diagnostic"]["detail"]["family"], "state-model",
        "{refused}"
    );
}

/// FR-626-AC-1, AC-2: deadlock freedom respects intended terminal states.
#[trace("TC-1586", "FR-626-AC-1")]
#[trace("TC-1586", "FR-626-AC-2")]
#[test]
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
fn weak_fairness_decides_liveness() {
    let fair = clause(
        "Fair",
        "fair weak M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let unfair = clause("Unfair", "", "eventually holds(c.value = 3)");
    let ws = Workspace::new("cli-liveness");
    // Over the inc-only Counter the one behaviour reaches 3 and stutters only
    // at its terminal state (FR-125-AC-2), so `Unfair` holds there; the
    // staying Counter adds the `setTo(0)` self-loop at 0 that only fairness
    // on `inc` excludes.
    let run = analyze(&crate::analyze_cli_support::staying_counter_request(
        &ws,
        &format!("{fair}{unfair}"),
        json!([{"claim": "Fair"}, {"claim": "Unfair"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted");
    let cycle = refuted["counterexample"]["loop"]
        .as_array()
        .expect("a lasso loop");
    assert!(!cycle.is_empty(), "{refuted}");
    // Every loop step is the `setTo(0)` self-loop taken at `value = 0`: its
    // FSpec FR-181 transition identity names `setTo`, and its post state
    // holds `value` 0 (`Stay` admits only `n = 0` there), so the cycle
    // never takes `inc`.
    for step in cycle {
        let transition = step["transition"].to_string();
        assert!(transition.contains("setTo"), "a setTo step: {step}");
        assert!(
            step["post_state"]["state"]
                .to_string()
                .contains(r#""name":"value","value":{"type":"integer","value":"0"}"#),
            "at value = 0: {step}"
        );
    }
    let ws = Workspace::new("cli-liveness-inc-only");
    let run = analyze(&counter_request(&ws, &unfair, json!([{"claim": "Unfair"}])));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-627-AC-4: a `finite-replay` item naming a temporal clause is
/// inapplicable to its `temporal-trace` family (FR-507).
#[trace("TC-1587", "FR-627-AC-4")]
#[test]
fn a_finite_replay_item_over_a_temporal_clause_is_inapplicable() {
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
    // `finite-replay` applies to protocol claims only (FR-507): a temporal
    // clause is evaluated over a supplied trace by the clause run, where
    // FR-132 settles the missing premise.
    assert_eq!(run.exit, 20);
    let item = run.item(0);
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "inapplicable-kind",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["family"], "temporal-trace",
        "{item}"
    );
}

/// FR-628-AC-1: a possible claim proves with a witness and refutes.
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
fn possible_claims_prove_and_refute() {
    // A target outside `Int[0, 3]` refuses at check (FR-104), so the
    // unreachable target is `value = 0` from a counter started at 1.
    let body = "possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n\
                possible ReachZero using v over (c: M::Counter) { c.value = 0 }\n";
    let ws = Workspace::new("cli-state-graph");
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "ReachThree"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    let ws = Workspace::new("cli-state-graph-from-one");
    let run = analyze(&crate::analyze_cli_support::request_over(
        &ws,
        &crate::counter::unit_text(&format!("{}{body}", crate::counter::CONTRACT)),
        &crate::counter::document(false),
        &[crate::analyze_cli_support::counter_snapshot(&[("a", 1)])],
        json!({"items": [{"claim": "ReachZero"}]}),
    ));
    assert_eq!(label(&run.item(0)), "refuted");
}

/// FR-629-AC-4: a liveness item over a release/acquire parallel always has
/// a terminal record, with a catalog code when it is not decided.
#[trace("TC-1589", "FR-629-AC-4")]
#[test]
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
#[ignore = "QSL-2296 (from QSL-2132): FR-634 does not say which traces an enumerate item reports nor what coverage.choices counts, and fsl-eval exploration returns no traces (spec question open)"]
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
    assert_eq!(item["coverage"]["choices"], "0");
    assert_eq!(item["traces"].as_array().map(Vec::len), Some(1));
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

/// FR-626-AC-1: the refuted deadlock item's counterexample is a finite prefix
/// whose last state is the stuck state `value = 3`, and its replay reproduced.
#[trace("TC-1586", "FR-626-AC-1")]
#[test]
fn a_deadlock_counterexample_ends_at_the_stuck_state_and_replays() {
    let ws = Workspace::new("cli-deadlock-ce");
    let run = analyze(&counter_request(
        &ws,
        "",
        json!([{"claim": "M", "temporal": {"item_kind": "deadlock-freedom"}}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted");
    assert_eq!(item["counterexample"]["kind"], "deadlock");
    assert_eq!(item["replay"]["reproduced"], true);
    let prefix = item["counterexample"]["prefix"]
        .as_array()
        .expect("a finite prefix");
    assert_eq!(prefix.len(), 3, "inc runs three times from value 0");
    let last = prefix.last().expect("a last step")["post_state"]["state"].to_string();
    assert!(last.contains(r#""name":"value","value":{"type":"integer","value":"3"}"#));
    assert_eq!(run.exit, 10);
}

/// FR-626-AC-1: a derived item named by its model alias is the one derived
/// item of the subject, and a claim naming no model alias is `unknown-claim`.
#[trace("TC-1586", "FR-626-AC-1")]
#[test]
fn a_derived_item_over_a_name_that_is_no_model_alias_is_an_unknown_claim() {
    let ws = Workspace::new("cli-deadlock-unknown");
    let run = analyze(&counter_request(
        &ws,
        "",
        json!([{"claim": "Nothing", "temporal": {"item_kind": "deadlock-freedom"}}]),
    ));
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["cause"],
        "unknown-claim"
    );
}

/// FR-896-AC-1, AC-2 (TC-2532): a `global-conformance` item with no
/// recording is decided over every behaviour of the protocol and its bound
/// role implementations: the order protocol conforms, and without its
/// receive it is refuted with the deadlock.
#[trace("TC-2532", "FR-896-AC-1", "FR-896-AC-2")]
#[test]
#[ignore = "QSL-2209: a global-conformance item settles unsupported with a pending obligation instead of being decided"]
fn global_conformance_is_decided_over_every_behaviour() {
    for (receive, expected) in [(true, "proved"), (false, "refuted")] {
        let ws = Workspace::new(&format!("cli-conformance-{receive}"));
        let run = analyze(&litmus_request(
            &ws,
            &order_protocol(receive),
            json!({"items": [{"claim": "Flow", "kind": "global-conformance"}]}),
        ));
        assert_eq!(label(&run.item(0)), expected, "receive {receive}");
    }
}

/// FR-1032-AC-1: bind the recording under its request-level identity and
/// echo that identity in the settled item without claiming independent replay.
#[trace("TC-2922", "FR-1032-AC-1", "FR-990-AC-3")]
#[test]
fn a_recording_result_echoes_its_provisioned_identity_without_replay_subject() {
    let ws = Workspace::new("cli-replay-mode");
    let recording = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [
            {"kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "case", "node": "Flow::Main::Decide::yes", "role": "Seller",
             "instance": "w-1", "keys": {}},
            {"kind": "finish", "node": "Flow::Done", "role": "Seller",
             "instance": "w-1", "keys": {}, "record": {"zz": true}}
        ],
        "local_order": {"Seller": [1, 2, 3], "Buyer": [0]},
        "correlations": [{"send": 0, "receive": 1}],
        "closure": {"state": "closed", "premises": [
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    ws.write("recording.json", recording.to_string().as_bytes());
    let run = analyze(&litmus_request(
        &ws,
        &order_protocol(true),
        json!({"items": [{"claim": "Flow", "kind": "global-conformance",
                   "recording": {"authority": "test", "identity": "recording"}}],
         "documents": [{"file": "recording.json", "identity": {
             "authority": "test", "identity": "recording"
         }}]}),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    let verdicts = item["obligations"]
        .as_array()
        .expect("one verdict per obligation");
    assert!(
        verdicts
            .iter()
            .all(|verdict| verdict["verdict"] == "satisfied"),
        "{item}"
    );
    assert_eq!(item["closure"]["state"], "closed");
    assert_eq!(item["global"]["complete"], true);
    assert!(
        item.get("replay").is_none(),
        "recording bytes are not an independent replay: {item}"
    );
    assert_eq!(item["recording"], json!({"authority":"test", "identity":"recording"}));

    let mut declared = recording;
    declared["identity"] = json!({"authority":"test", "identity":"recording"});
    let declared_ws = Workspace::new("cli-replay-mode-declared");
    declared_ws.write("recording.json", declared.to_string().as_bytes());
    let declared_run = analyze(&litmus_request(
        &declared_ws,
        &order_protocol(true),
        json!({"items": [{"claim": "Flow", "kind": "global-conformance",
                   "recording": {"authority": "test", "identity": "recording"}}],
         "documents": [{"file": "recording.json"}]}),
    ));
    let declared_item = declared_run.item(0);
    assert_eq!(label(&declared_item), "proved", "{declared_item}");
    assert_eq!(declared_item["obligations"], item["obligations"]);
    assert_eq!(declared_item["closure"], item["closure"]);
    assert_eq!(declared_item["global"], item["global"]);
}
