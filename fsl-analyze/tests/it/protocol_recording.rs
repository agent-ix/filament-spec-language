// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-895: replay-mode reading and obligation assessment of protocol recordings.

use fsl_analyze::protocol_analysis::recording::{assess_recording, RecordingVerdict};
use serde_json::{json, Value};

use crate::analyze_cli_support::order_protocol;

fn protocol() -> fsl_package::CheckedPackage {
    crate::litmus::package_called("analysis-unit", &order_protocol(true))
}

fn recording(observations: Value, local_order: Value, closure_state: &str) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": observations,
        "local_order": local_order,
        "correlations": [{"send": 0, "receive": 1}],
        "closure": {"state": closure_state, "premises": []}
    })
}

fn complete_observations() -> Value {
    json!([
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
    ])
}

/// Trace: FR-895-AC-1, FR-895-AC-3.
#[trace("FR-895-AC-1", "FR-895-AC-3")]
#[test]
fn a_closed_recording_assesses_observed_and_unobserved_obligations() {
    let package = protocol();
    let checked = package.graph().protocol("Flow").expect("Flow is checked");
    let closed = recording(
        complete_observations(),
        json!({"Buyer": [0], "Seller": [1, 2, 3]}),
        "closed",
    );
    let assessed = assess_recording(checked, closed.to_string().as_bytes()).expect("readable run");
    assert_eq!(assessed.global.label, "proved");
    assert!(assessed
        .obligations
        .iter()
        .all(|entry| entry.verdict == RecordingVerdict::Satisfied));

    let mut cut = complete_observations();
    cut.as_array_mut().expect("array").truncate(2);
    let open = recording(cut, json!({"Buyer": [0], "Seller": [1]}), "open");
    let assessed = assess_recording(checked, open.to_string().as_bytes()).expect("readable run");
    assert_eq!(assessed.global.label, "inconclusive");
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "control" && entry.verdict == RecordingVerdict::Pending
    }));
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Pending
    }));
}

/// Trace: FR-895-AC-4.
#[trace("FR-895-AC-4")]
#[test]
fn an_undeclared_role_is_a_correspondence_refusal() {
    let package = protocol();
    let checked = package.graph().protocol("Flow").expect("Flow is checked");
    let run = recording(
        json!([{
            "kind": "send", "node": "Flow::Main::Placed", "role": "Courier",
            "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"}
        }]),
        json!({"Courier": [0]}),
        "open",
    );
    let refusal =
        assess_recording(checked, run.to_string().as_bytes()).expect_err("role is absent");
    assert_eq!(refusal.code().cause(), "missing-name");
    assert_eq!(refusal.position(), Some(0));
}
