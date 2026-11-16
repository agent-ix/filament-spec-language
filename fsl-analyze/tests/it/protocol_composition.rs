// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 (TC-1596): a supplied finite protocol trace replays through
//! `fsl analyze`, and channel capacity under a parallel join is analyzed.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Workspace};

fn trace_document(identity: &str, steps: &[&str]) -> Value {
    json!({"format": "filament.protocol-trace/v1",
        "identity": {"authority": "test", "identity": identity},
        "steps": steps.iter().map(|step| json!({"step": step})).collect::<Vec<_>>()})
}

/// FR-636-AC-3: an admitted trace proves; a trace starting with a disabled
/// step is declined at that step.
#[trace("TC-1596", "FR-636-AC-3")]
#[test]
#[ignore = "QSL-2224: supplied finite protocol traces are not accepted by fsl analyze yet"]
fn a_supplied_trace_replays_or_is_declined_at_its_step() {
    for (identity, steps, expected) in [
        ("good", &["Placed", "Got", "Ship"][..], "proved"),
        ("stale", &["Got"][..], "declined"),
    ] {
        let ws = Workspace::new(&format!("protocol-trace-{identity}"));
        let path = litmus_request(
            &ws,
            &order_protocol(true),
            json!({"items": [{"claim": "Flow", "kind": "finite-replay",
                "trace": {"authority": "test", "identity": identity}}]}),
        );
        ws.write(
            "trace.json",
            trace_document(identity, steps).to_string().as_bytes(),
        );
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
        document["request"]["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": "trace.json"}));
        ws.write("analyze.json", document.to_string().as_bytes());
        let run = analyze(&path);
        let item = run.item(0);
        assert_eq!(label(&item), expected, "{identity}");
        if expected == "declined" {
            assert_eq!(item["record"]["cause"], "invalid-input");
            assert_eq!(item["record"]["diagnostic"]["payload"]["position"], 0);
        }
    }
}

/// FR-636-AC-4: a parallel join over a capacity-1 channel overflows, and a
/// message budget stops the run incomplete.
#[trace("TC-1596", "FR-636-AC-4")]
#[test]
#[ignore = "QSL-2224: protocol analysis items are not reachable through fsl analyze yet"]
fn parallel_join_overflow_and_message_budget() {
    let body = "record Ack { ok: Boolean; }\n\
        protocol Burst using pp over (k: M::Core) on origin {\n\
        role Buyer on M::Cell;\nrole Seller on M::Cell;\n\
        channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 1 overflow reject;\n\
        run parallel Both { branch left sequence L { send A via Order as (a: Ack) { true }; } \
        branch right sequence R { send B via Order as (b: Ack) { true }; } } join all [left, right];\n\
        finish Done as (zz: Boolean) { true };\n}\n";
    let ws = Workspace::new("protocol-overflow");
    let run = analyze(&litmus_request(
        &ws,
        body,
        json!({"items": [
            {"claim": "Burst", "kind": "global-conformance"},
            {"claim": "Burst", "kind": "global-conformance",
             "temporal": {"limits": {"max_messages": 1}}},
        ]}),
    ));
    assert_eq!(label(&run.item(0)), "refuted");
    assert_eq!(label(&run.item(1)), "incomplete");
    assert_eq!(run.item(1)["record"]["cause"]["limit"], "max_messages");
}

/// Run one bare `finite-replay` item of the order fixture over `trace`.
fn bare_replay(identity: &str, trace: Value) -> Value {
    let ws = Workspace::new(&format!("bare-replay-{identity}"));
    let path = litmus_request(
        &ws,
        &order_protocol(true),
        json!({"items": [{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": identity}}]}),
    );
    ws.write("trace.json", trace.to_string().as_bytes());
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    document["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "trace.json"}));
    ws.write("analyze.json", document.to_string().as_bytes());
    analyze(&path).item(0)
}

/// FR-941-AC-1: a bare protocol claim is proved over a supplied trace when
/// every step replays, a prefix that stops before the choice included.
#[trace("TC-2621", "FR-941-AC-1")]
#[test]
#[ignore = "QSL-2272: the finite-replay engine for supplied protocol traces is not merged yet"]
fn a_bare_claim_is_proved_when_every_step_replays() {
    for (identity, steps) in [
        ("whole", &["Placed", "Got", "Ship"][..]),
        ("prefix", &["Placed", "Got"][..]),
    ] {
        let item = bare_replay(identity, trace_document(identity, steps));
        assert_eq!(label(&item), "proved", "{identity}: {item}");
        assert_eq!(
            item["record"]["steps"],
            steps.len(),
            "{identity}: the replayed step count"
        );
    }
}

/// FR-941-AC-2 and AC-3: the trace document is read before replay, so a
/// shape defect at a later step is reported over a stale earlier step; a
/// stale first step of a well-formed trace is declined at position 0.
#[trace("TC-2621", "FR-941-AC-2", "FR-941-AC-3")]
#[test]
#[ignore = "QSL-2272: the finite-replay engine for supplied protocol traces is not merged yet"]
fn a_trace_is_read_before_it_is_replayed() {
    let mut malformed = trace_document("malformed", &["Got", "Placed"]);
    malformed["steps"][1]["x"] = json!(1);
    let item = bare_replay("malformed", malformed);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["cause"], "invalid-input");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unknown-member",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["payload"]["position"], 1,
        "{item}"
    );

    let item = bare_replay("stale", trace_document("stale", &["Got"]));
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["payload"]["position"], 0,
        "{item}"
    );
}
