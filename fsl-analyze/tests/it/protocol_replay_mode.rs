// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-895 (TC-2530, TC-2531, TC-2815): replay mode assesses a recorded
//! protocol execution (FSpec FR-737) against the protocol's obligations.
//! Causal order is each role's local order joined with the correlations; the
//! position of an observation in the `observations` array never orders it,
//! and a pair on a causal cycle is misordered.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, order_protocol, request_over, Run, Workspace,
};

/// The order protocol of [`order_protocol`] with a second channel `Tell`
/// from `Seller` to `Buyer`: `Seller` sends `Note` after `Got` and `Buyer`
/// receives it as `Heard`.
fn told_protocol() -> String {
    order_protocol(true)
        .replace(
            "capacity 4 overflow reject;\n",
            "capacity 4 overflow reject;\n\
             channel Tell from Seller to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n",
        )
        .replace(
            "receive Got via Order of Placed as (got: Ack) { true };\n",
            "receive Got via Order of Placed as (got: Ack) { true };\n\
             send Note via Tell as (note: Ack) { true };\n\
             receive Heard via Tell of Note as (heard: Ack) { true };\n",
        )
}

/// One observation of `kind` at `Flow::Main::<node>` by `role`, carrying the
/// message key `message`.
fn observation(kind: &str, node: &str, role: &str, message: &str) -> Value {
    json!({"kind": kind, "node": format!("Flow::Main::{node}"), "role": role,
           "instance": "w-1",
           "keys": {"message": message, "delivery": format!("d-{message}")},
           "record": {"ok": true}})
}

/// A closed `filament.protocol-recording/1` document of `Flow`.
fn recording(observations: Value, local_order: Value, correlations: Value) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": observations,
        "local_order": local_order,
        "correlations": correlations,
        "closure": {"state": "closed", "premises": []}
    })
}

/// `fsl analyze` of one replay-mode `global-conformance` item of `Flow` in
/// `protocol` over `recording`.
fn replay(name: &str, protocol: &str, recording: &Value) -> Run {
    let ws = Workspace::new(name);
    ws.write("recording.json", recording.to_string().as_bytes());
    analyze(&litmus_request(
        &ws,
        protocol,
        json!({"items": [{"claim": "Flow", "kind": "global-conformance",
                          "recording": {"authority": "test", "identity": "recording"}}],
               "documents": [{"file": "recording.json"}]}),
    ))
}

/// The verdicts of `item`'s obligations.
fn verdicts(item: &Value) -> &Vec<Value> {
    item["obligations"]
        .as_array()
        .expect("one verdict per obligation")
}

/// FR-895-AC-2 (TC-2530): `Placed` and `Got` on a causal cycle through the
/// `Tell` correlation give the causal-order obligation of the pair
/// `violated`, `misordered`, naming both, and the item is refuted at `Got`.
#[trace("TC-2530", "FR-895-AC-2", "FSpec-FR-737-AC-1")]
#[test]
#[ignore = "QSL-2210: no replay-mode executor assesses a recorded protocol execution"]
fn a_pair_on_a_causal_cycle_is_misordered() {
    let run = replay(
        "replay-cycle",
        &told_protocol(),
        &recording(
            json!([
                observation("receive", "Got", "Seller", "m-1"),
                observation("send", "Note", "Seller", "m-2"),
                observation("receive", "Heard", "Buyer", "m-2"),
                observation("send", "Placed", "Buyer", "m-1"),
            ]),
            json!({"Seller": [0, 1], "Buyer": [2, 3]}),
            json!([{"send": 3, "receive": 0}, {"send": 1, "receive": 2}]),
        ),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    let misordered: Vec<&Value> = verdicts(&item)
        .iter()
        .filter(|verdict| verdict["verdict"] == "violated" && verdict["cause"] == "misordered")
        .collect();
    let [verdict] = misordered[..] else {
        panic!("one misordered obligation: {item}");
    };
    assert_eq!(verdict["observations"], json!([3, 0]), "{item}");
    assert_eq!(item["record"]["trace_position"], 0, "{item}");
}

/// FR-895-AC-7 (TC-2815): a recording that lists `Got` before `Placed` in its
/// array, with no causal cycle, conforms and settles `proved`.
#[trace("TC-2815", "FR-895-AC-7", "FSpec-FR-737-AC-2")]
#[test]
#[ignore = "QSL-2210: no replay-mode executor assesses a recorded protocol execution"]
fn array_order_alone_never_misorders_a_recording() {
    let mut finish = observation("finish", "Done", "Buyer", "m-0");
    finish["node"] = json!("Flow::Done");
    let run = replay(
        "replay-array-order",
        &order_protocol(true),
        &recording(
            json!([
                observation("receive", "Got", "Seller", "m-1"),
                observation("send", "Placed", "Buyer", "m-1"),
                observation("event", "Decide::yes::Ship", "Seller", "m-0"),
                finish,
            ]),
            json!({"Seller": [0, 2], "Buyer": [1, 3]}),
            json!([{"send": 1, "receive": 0}]),
        ),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert!(
        verdicts(&item)
            .iter()
            .all(|verdict| verdict["verdict"] == "satisfied"),
        "{item}"
    );
}

/// FR-895-AC-5 (TC-2531): `Seller` sending `Note` before any `Got` is a step
/// the protocol does not enable: its enabledness obligation is `violated`,
/// cause `unexpected`, and the recording is assessed, not refused.
#[trace("TC-2531", "FR-895-AC-5", "FSpec-FR-059-AC-1")]
#[test]
#[ignore = "QSL-2210: no replay-mode executor assesses a recorded protocol execution"]
fn an_unexpected_send_violates_its_enabledness_obligation() {
    let run = replay(
        "replay-unexpected",
        &told_protocol(),
        &recording(
            json!([observation("send", "Note", "Seller", "m-2")]),
            json!({"Seller": [0], "Buyer": []}),
            json!([]),
        ),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert!(
        verdicts(&item)
            .iter()
            .any(|verdict| verdict["obligation"]["kind"] == "enabledness"
                && verdict["verdict"] == "violated"
                && verdict["cause"] == "unexpected"),
        "{item}"
    );
}

/// FR-895-AC-6 (TC-2531): a compensation `within [0, 120]` on clock `c`
/// whose first attempt reads 130 is a violated window; with no reading and
/// closure `open` the window is pending; batch and incremental assessment
/// agree.
#[trace("TC-2531", "FR-895-AC-6", "FSpec-FR-059-AC-4")]
#[test]
#[ignore = "QSL-2210: no replay-mode executor assesses a recorded protocol execution"]
fn a_late_compensation_attempt_violates_its_window_and_an_unread_one_is_pending() {
    let body =
        crate::protocol_compensation::pay("true").replace("within [0, 10]", "within [0, 120]");
    let mut documents = crate::fill::Documents::new();
    documents.start("start", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.into_values().collect();
    let attempt = |clocks: Value| {
        json!({"kind": "compensation-attempt", "node": "Pay::Refund", "role": "w",
               "instance": "w-1",
               "keys": {"compensation_attempt": "ca-1", "registration": "r-1"},
               "record": {"fwd": {"v": 1}}, "outcome": "succeeded", "clocks": clocks})
    };
    for (clocks, closure, expected) in [
        (json!({"c": "130"}), "closed", "violated"),
        (json!({}), "open", "pending"),
    ] {
        let mut window = None;
        for mode in ["batch", "incremental"] {
            let ws = Workspace::new(&format!("replay-window-{expected}-{mode}"));
            let recording = json!({
                "format": "filament.protocol-recording/1",
                "protocol": "Pay",
                "observations": [attempt(clocks.clone())],
                "local_order": {"w": [0]},
                "correlations": [],
                "closure": {"state": closure, "premises": []}
            });
            ws.write("recording.json", recording.to_string().as_bytes());
            let run = analyze(&request_over(
                &ws,
                &crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body),
                &crate::fill::document(),
                &snapshots,
                json!({"items": [{"claim": "Pay", "kind": "global-conformance",
                                  "recording": {"authority": "test", "identity": "recording"},
                                  "assessment": mode}],
                       "documents": [{"file": "recording.json"}]}),
            ));
            let item = run.item(0);
            let found = verdicts(&item)
                .iter()
                .find(|verdict| verdict["obligation"]["kind"] == "window")
                .cloned()
                .unwrap_or_else(|| panic!("a window obligation: {item}"));
            assert_eq!(found["verdict"], expected, "{mode}: {item}");
            match &window {
                None => window = Some(found),
                Some(batch) => assert_eq!(batch, &found, "batch equals incremental"),
            }
        }
    }
}
