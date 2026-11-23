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

/// The `Seller`'s observation that `Decide` took its `yes` case.
fn chosen_yes() -> Value {
    json!({"kind": "case", "node": "Flow::Main::Decide::yes", "role": "Seller",
           "instance": "w-1", "keys": {}})
}

/// The observation that the protocol reached `Flow::Done`.
fn finished() -> Value {
    json!({"kind": "finish", "node": "Flow::Done", "role": "Seller",
           "instance": "w-1", "keys": {}, "record": {"zz": true}})
}

/// The closure premises of a recording of `Pay` in state `closure`: none are
/// required while it is open.
fn pay_premises(closure: &str) -> Value {
    if closure == "closed" {
        json!([{"kind": "execution", "node": "Pay::Done"}])
    } else {
        json!([])
    }
}

/// The closure premises of a closed recording of `Flow`: the execution
/// premise at `Flow::Done` and one branch premise per case (FR-990-AC-6).
fn flow_premises() -> Value {
    json!([
        {"kind": "execution", "node": "Flow::Done"},
        {"kind": "branch", "node": "Flow::Main::Decide::no"},
        {"kind": "branch", "node": "Flow::Main::Decide::yes"},
    ])
}

/// A closed `filament.protocol-recording/1` document of `Flow`.
fn recording(observations: Value, local_order: Value, correlations: Value) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": observations,
        "local_order": local_order,
        "correlations": correlations,
        "closure": {"state": "closed", "premises": flow_premises()}
    })
}

/// The document identity the replay requests bind `recording.json` to.
fn recording_identity() -> Value {
    json!({"authority": "test", "identity": "recording"})
}

/// The `documents` member of a request over `snapshots` snapshot files
/// (named `snapshot-<n>.json` by `request_over`) and `recording.json`
/// supplied as `entry` (FR-1032).
fn documents_with(snapshots: usize, entry: Value) -> Value {
    let mut documents: Vec<Value> = (0..snapshots)
        .map(|n| json!({"file": format!("snapshot-{n}.json")}))
        .collect();
    documents.push(entry);
    Value::Array(documents)
}

/// `fsl analyze` of the replay-mode `items` of `Flow` in `protocol` over
/// `recording`, supplied as `entry` among the litmus snapshots.
fn replay_with(name: &str, protocol: &str, recording: &Value, entry: Value, items: Value) -> Run {
    let ws = Workspace::new(name);
    ws.write("recording.json", recording.to_string().as_bytes());
    let snapshots = crate::litmus::Documents::new().snapshots.len();
    analyze(&litmus_request(
        &ws,
        protocol,
        json!({"items": items, "documents": documents_with(snapshots, entry)}),
    ))
}

/// `fsl analyze` of one replay-mode `global-conformance` item of `Flow` in
/// `protocol` over `recording`, bound to [`recording_identity`] by its
/// `documents` entry (FR-1032).
fn replay(name: &str, protocol: &str, recording: &Value) -> Run {
    replay_with(
        name,
        protocol,
        recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        json!([{"claim": "Flow", "kind": "global-conformance",
                "recording": recording_identity()}]),
    )
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
#[ignore = "QSL-2885: spec gap, FR-895-AC-2 does not order the two witness indices of a misordered causal-order obligation; protocol_recording::receive_before_send_is_a_causal_order_violation asserts [target, source] = [0, 3]"]
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
        .filter(|verdict| {
            verdict["verdict"] == "violated"
                && verdict["cause"] == "misordered"
                && verdict["obligation"]["kind"] == "causal-order"
                && verdict["obligation"]["node"] == "Flow::Main::Placed -> Flow::Main::Got"
        })
        .collect();
    let [verdict] = misordered[..] else {
        panic!("one misordered obligation: {item}");
    };
    assert_eq!(verdict["witness"], json!([3, 0]), "{item}");
    assert_eq!(item["counterexample"]["trace_position"], 0, "{item}");
}

/// FR-895-AC-7 (TC-2815): a recording that lists `Got` before `Placed` in its
/// array, with no causal cycle, conforms and settles `proved`.
#[trace("TC-2815", "FR-895-AC-7", "FSpec-FR-737-AC-2")]
#[test]
fn array_order_alone_never_misorders_a_recording() {
    let run = replay(
        "replay-array-order",
        &order_protocol(true),
        &recording(
            json!([
                observation("receive", "Got", "Seller", "m-1"),
                observation("send", "Placed", "Buyer", "m-1"),
                chosen_yes(),
                finished(),
            ]),
            json!({"Seller": [0, 2, 3], "Buyer": [1]}),
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
/// closure `open` the window is pending. The batch and incremental agreement
/// of FR-895-AC-6 awaits the spec ruling on the incremental surface.
#[trace("TC-2531", "FR-895-AC-6", "FSpec-FR-059-AC-4")]
#[test]
fn a_late_compensation_attempt_violates_its_window_and_an_unread_one_is_pending() {
    let body =
        crate::protocol_compensation::pay("true").replace("within [0, 10]", "within [0, 120]");
    let mut documents = crate::fill::Documents::new();
    documents.start("start", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.into_values().collect();
    let attempt = |clocks: Value| {
        json!({"kind": "compensation-attempt", "node": "Pay::Main::Paid", "role": "w",
               "instance": "w-1",
               "keys": {"compensation_attempt": "ca-1", "registration": "r-1"},
               "outcome": "successful", "clocks": clocks})
    };
    for (clocks, closure, expected) in [
        (json!({"c": "130"}), "closed", "violated"),
        (json!({}), "open", "pending"),
    ] {
        let ws = Workspace::new(&format!("replay-window-{expected}"));
        let recording = json!({
            "format": "filament.protocol-recording/1",
            "protocol": "Pay",
            "observations": [attempt(clocks)],
            "local_order": {"w": [0]},
            "correlations": [],
            "closure": {"state": closure, "premises": pay_premises(closure)}
        });
        ws.write("recording.json", recording.to_string().as_bytes());
        let run = analyze(&request_over(
            &ws,
            &crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body),
            &crate::fill::document(),
            &snapshots,
            json!({"items": [{"claim": "Pay", "kind": "global-conformance",
                              "recording": recording_identity()}],
                   "clocks": {"c": {"kind": "timed", "unit": "s"}},
                   "documents": documents_with(snapshots.len(),
                       json!({"file": "recording.json", "identity": recording_identity()}))}),
        ));
        let item = run.item(0);
        let found = verdicts(&item)
            .iter()
            .find(|verdict| verdict["obligation"]["kind"] == "window")
            .unwrap_or_else(|| panic!("a window obligation: {item}"));
        assert_eq!(found["verdict"], expected, "{item}");
    }
    // FR-895-AC-6's "batch and incremental assessment give equal verdicts"
    // is not asserted here: `fsl analyze` exposes no incremental assessment
    // to compare against, and re-running prefixes through the same request
    // equals the batch run by construction. It waits on the spec ruling on
    // the incremental surface.
}

/// A closed recording of the `Tell` protocol: `Placed`, `Got`, `Note`,
/// `Heard`, the `yes` case and `Done`, with `Seller`'s local order
/// `seller_order` over those indices and `observations` listing them in
/// the given array order.
fn told_recording(array_order: [usize; 6], seller_order: Value) -> Value {
    let canonical = [
        observation("send", "Placed", "Buyer", "m-1"),
        observation("receive", "Got", "Seller", "m-1"),
        observation("send", "Note", "Seller", "m-2"),
        observation("receive", "Heard", "Buyer", "m-2"),
        chosen_yes(),
        finished(),
    ];
    // `array_order[n]` is the canonical index listed at array position `n`.
    let at = |canonical_index: usize| {
        array_order
            .iter()
            .position(|listed| *listed == canonical_index)
            .expect("every observation is listed")
    };
    let remap = |order: &Value| -> Value {
        order
            .as_array()
            .expect("an order")
            .iter()
            .map(|index| json!(at(index.as_u64().expect("an index") as usize)))
            .collect()
    };
    recording(
        Value::Array(
            array_order
                .iter()
                .map(|index| canonical[*index].clone())
                .collect(),
        ),
        json!({"Seller": remap(&seller_order), "Buyer": remap(&json!([0, 3]))}),
        json!([
            {"send": at(0), "receive": at(1)},
            {"send": at(2), "receive": at(3)},
        ]),
    )
}

/// The enabledness verdict of `Flow::Main::Note` in `item`.
fn note_enabledness(item: &Value) -> &Value {
    verdicts(item)
        .iter()
        .find(|verdict| {
            verdict["obligation"]["kind"] == "enabledness"
                && verdict["obligation"]["node"] == "Flow::Main::Note"
        })
        .unwrap_or_else(|| panic!("a Note enabledness verdict: {item}"))
}

/// FR-895-AC-5: `Seller` sending `Note` after its `Got` is enabled, and the
/// recording settles `proved`.
#[trace("TC-2531", "FR-895-AC-5", "FSpec-FR-059-AC-1")]
#[test]
fn a_send_after_its_earlier_own_receive_is_enabled() {
    let run = replay(
        "replay-told-accepted",
        &told_protocol(),
        &told_recording([0, 1, 2, 3, 4, 5], json!([1, 2, 4, 5])),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(note_enabledness(&item)["verdict"], "satisfied", "{item}");
}

/// FR-895-AC-5: `Seller`'s local order puts `Note` before its `Got`, so
/// `Got` does not precede `Note` and `Note`'s enabledness is `violated`,
/// cause `unexpected`, though `Got` is in the recording.
#[trace("TC-2531", "FR-895-AC-5", "FSpec-FR-059-AC-1")]
#[test]
fn a_send_before_its_earlier_own_receive_is_unexpected() {
    let run = replay(
        "replay-told-before",
        &told_protocol(),
        &told_recording([0, 1, 2, 3, 4, 5], json!([2, 1, 4, 5])),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    let note = note_enabledness(&item);
    assert_eq!(note["verdict"], "violated", "{item}");
    assert_eq!(note["cause"], "unexpected", "{item}");
}

/// FR-895-AC-5: the array position of `Got` never makes it precede `Note`:
/// listed first in the array but after `Note` in `Seller`'s local order, it
/// leaves `Note`'s enabledness `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-5", "FSpec-FR-737-AC-2")]
#[test]
fn an_earlier_own_receive_listed_first_but_ordered_later_is_unexpected() {
    let run = replay(
        "replay-told-listed-first",
        &told_protocol(),
        &told_recording([1, 0, 2, 3, 4, 5], json!([2, 1, 4, 5])),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    let note = note_enabledness(&item);
    assert_eq!(note["verdict"], "violated", "{item}");
    assert_eq!(note["cause"], "unexpected", "{item}");
}

// ---- FR-1032: the recording's document identity ---------------------------

/// FR-895-AC-7's closed, conforming recording of `Flow`.
fn conforming_recording() -> Value {
    recording(
        json!([
            observation("receive", "Got", "Seller", "m-1"),
            observation("send", "Placed", "Buyer", "m-1"),
            chosen_yes(),
            finished(),
        ]),
        json!({"Seller": [0, 2, 3], "Buyer": [1]}),
        json!([{"send": 1, "receive": 0}]),
    )
}

/// One replay-mode `Flow` item naming `recording`.
fn flow_item(recording: Value) -> Value {
    json!([{"claim": "Flow", "kind": "global-conformance", "recording": recording}])
}

/// FR-1032-AC-1/AC-2, FSpec FR-821-AC-1/AC-2: a recording that declares no
/// identity is bound by its `documents` entry, assessed, and its identity is
/// echoed; declaring the identity instead gives the same result, and binding
/// it under another identity changes no premise identity or verdict.
#[trace("TC-2922", "FR-1032-AC-1")]
#[trace("TC-2922", "FR-1032-AC-2")]
#[trace("FSpec-TC-2001", "FSpec-FR-821-AC-1")]
#[trace("FSpec-TC-2001", "FSpec-FR-821-AC-2")]
#[test]
fn a_recording_is_bound_by_its_stated_or_declared_identity() {
    let stated = replay(
        "replay-identity-stated",
        &order_protocol(true),
        &conforming_recording(),
    );
    let stated = stated.item(0);
    assert_eq!(label(&stated), "proved", "{stated}");
    assert_eq!(stated["recording"], recording_identity(), "{stated}");

    let mut declaring = conforming_recording();
    declaring["identity"] = recording_identity();
    let declared = replay_with(
        "replay-identity-declared",
        &order_protocol(true),
        &declaring,
        json!({"file": "recording.json"}),
        flow_item(recording_identity()),
    )
    .item(0);
    for member in ["obligations", "closure", "global", "recording"] {
        assert_eq!(stated[member], declared[member], "{member}: {declared}");
    }

    let other = json!({"authority": "test", "identity": "other"});
    let rebound = replay_with(
        "replay-identity-other",
        &order_protocol(true),
        &conforming_recording(),
        json!({"file": "recording.json", "identity": other}),
        flow_item(other.clone()),
    )
    .item(0);
    assert_eq!(
        stated["closure"]["premises"], rebound["closure"]["premises"],
        "{rebound}"
    );
    assert_eq!(stated["obligations"], rebound["obligations"], "{rebound}");
    assert_eq!(rebound["recording"], other, "{rebound}");
}

/// FR-1032-AC-3/AC-4, FSpec FR-821-AC-3/AC-4: a malformed, contradicted,
/// missing or duplicated recording identity, and an item `recording` that
/// names nothing, a snapshot, or sits on another kind, refuse the request.
#[trace("TC-2922", "FR-1032-AC-3")]
#[trace("TC-2922", "FR-1032-AC-4")]
#[trace("FSpec-TC-2001", "FSpec-FR-821-AC-3")]
#[trace("FSpec-TC-2001", "FSpec-FR-821-AC-4")]
#[test]
fn a_bad_recording_identity_refuses_the_request() {
    let snapshots = crate::litmus::Documents::new().snapshots.len();
    let at_documents = format!("/documents/{snapshots}");
    let refused = |name: &str, recording: &Value, entry: Value, items: Value, pointer: &str| {
        let run = replay_with(name, &order_protocol(true), recording, entry, items);
        assert_eq!(run.exit, 20, "{name}");
        let document = run.document();
        assert_eq!(document["items"], json!([]), "{name}: {document}");
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid-request",
            "{name}: {document}"
        );
        assert!(
            document.to_string().contains(pointer),
            "{name} names {pointer}: {document}"
        );
        document
    };
    let plain = conforming_recording();
    let mut declaring = conforming_recording();
    declaring["identity"] = json!({"authority": "test", "identity": "declared"});

    refused(
        "identity-malformed",
        &plain,
        json!({"file": "recording.json", "identity": {"authority": "test"}}),
        flow_item(recording_identity()),
        &format!("{at_documents}/identity"),
    );
    let document = refused(
        "identity-contradicted",
        &declaring,
        json!({"file": "recording.json", "identity": recording_identity()}),
        flow_item(recording_identity()),
        &format!("{at_documents}/identity"),
    );
    assert!(document.to_string().contains("declared"), "{document}");
    refused(
        "identity-missing",
        &plain,
        json!({"file": "recording.json"}),
        flow_item(recording_identity()),
        &at_documents,
    );

    // The litmus snapshots each declare an identity; take the first.
    let first: Value = crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .next()
        .map(|bytes| serde_json::from_slice::<Value>(&bytes).unwrap()["identity"].clone())
        .expect("a litmus snapshot");
    refused(
        "identity-duplicated",
        &plain,
        json!({"file": "recording.json", "identity": first}),
        flow_item(first.clone()),
        &at_documents,
    );

    let bound = json!({"file": "recording.json", "identity": recording_identity()});
    refused(
        "recording-unprovisioned",
        &plain,
        bound.clone(),
        flow_item(json!({"authority": "test", "identity": "absent"})),
        "/items/0/recording",
    );
    refused(
        "recording-is-a-snapshot",
        &plain,
        bound.clone(),
        flow_item(first),
        "/items/0/recording",
    );
    refused(
        "recording-on-another-kind",
        &plain,
        bound,
        json!([{"claim": "Flow", "kind": "temporal-satisfaction",
                "recording": recording_identity()}]),
        "/items/0/recording",
    );
}
