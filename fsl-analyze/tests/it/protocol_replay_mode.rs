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
    assert_eq!(verdict["witness"], json!([0, 3]), "{item}");
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
/// closure `open` the window is pending; batch and incremental assessment
/// agree.
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
        let assess = |assessment: &str| {
            analyze(&request_over(
                &ws,
                &crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body),
                &crate::fill::document(),
                &snapshots,
                json!({"items": [{"claim": "Pay", "kind": "global-conformance",
                                  "recording": recording_identity(),
                                  "assessment": assessment}],
                       "clocks": {"c": {"kind": "timed", "unit": "s"}},
                       "documents": documents_with(snapshots.len(),
                           json!({"file": "recording.json", "identity": recording_identity()}))}),
            ))
            .item(0)
        };
        let window = |item: &Value| {
            verdicts(item)
                .iter()
                .find(|verdict| verdict["obligation"]["kind"] == "window")
                .cloned()
                .unwrap_or_else(|| panic!("a window obligation: {item}"))
        };
        let item = assess("batch");
        assert_eq!(window(&item)["verdict"], expected, "{item}");
        // Batch and incremental assessment give equal verdicts: the single
        // observation's prefix is the whole recording, and its window
        // verdict is the batch one.
        let incremental = assess("incremental");
        let prefixes = incremental["prefixes"].as_array().expect("one prefix");
        assert_eq!(prefixes.len(), 1, "{incremental}");
        assert_eq!(window(&prefixes[0])["verdict"], expected, "{incremental}");
        assert_eq!(
            prefixes[0]["obligations"], item["obligations"],
            "{incremental}"
        );
    }
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

/// FR-895-AC-9: `Seller` sending `Note` after its `Got` is enabled, and the
/// recording settles `proved`.
#[trace("TC-2531", "FR-895-AC-9", "FSpec-FR-059-AC-1")]
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

/// FR-895-AC-10: `Seller`'s local order puts `Note` before its `Got`, so
/// `Got` does not precede `Note` and `Note`'s enabledness is `violated`,
/// cause `unexpected`, though `Got` is in the recording.
#[trace("TC-2531", "FR-895-AC-10", "FSpec-FR-059-AC-1")]
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

/// FR-895-AC-10: the array position of `Got` never makes it precede `Note`:
/// listed first in the array but after `Note` in `Seller`'s local order, it
/// leaves `Note`'s enabledness `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-10", "FSpec-FR-737-AC-2")]
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

/// The labels of `assessments`, one per observation prefix.
fn prefix_labels(
    assessments: &[fsl_analyze::protocol_analysis::recording::RecordingAssessment],
) -> Vec<String> {
    assessments
        .iter()
        .map(|assessment| assessment.global.label.clone())
        .collect()
}

/// Every obligation `violated` in a prefix is `violated` in the next.
fn assert_violations_persist(
    assessments: &[fsl_analyze::protocol_analysis::recording::RecordingAssessment],
) {
    use fsl_analyze::protocol_analysis::recording::RecordingVerdict;
    for pair in assessments.windows(2) {
        for violated in pair[0]
            .obligations
            .iter()
            .filter(|entry| entry.verdict == RecordingVerdict::Violated)
        {
            let next = pair[1]
                .obligations
                .iter()
                .find(|entry| entry.obligation == violated.obligation)
                .unwrap_or_else(|| panic!("{:?} survives", violated.obligation));
            assert_eq!(next.verdict, RecordingVerdict::Violated, "{:?}", next);
        }
    }
}

/// FR-895-AC-11, FSpec FR-059-AC-4: feeding the conforming recording one
/// observation at a time leaves every prefix `inconclusive` while it is open,
/// and the last prefix equals the batch assessment, `proved`.
#[trace("TC-2531", "FR-895-AC-11", "FSpec-FR-059-AC-4")]
#[test]
fn incremental_assessment_of_a_conforming_recording_ends_at_the_batch_verdict() {
    use fsl_analyze::protocol_analysis::recording::{
        assess_recording, assess_recording_incrementally,
    };
    let package = crate::litmus::package_called("analysis-unit", &told_protocol());
    let bytes = told_recording([0, 1, 2, 3, 4, 5], json!([1, 2, 4, 5])).to_string();
    let incremental = assess_recording_incrementally(&package, bytes.as_bytes()).expect("prefixes");
    assert_eq!(
        prefix_labels(&incremental),
        [
            "inconclusive",
            "inconclusive",
            "inconclusive",
            "inconclusive",
            "inconclusive",
            "proved"
        ]
    );
    assert_violations_persist(&incremental);
    let batch = assess_recording(&package, bytes.as_bytes()).expect("batch");
    assert_eq!(incremental.last(), Some(&batch));
}

/// FR-895-AC-11, FR-895-AC-10: `Seller`'s local order puts `Note` before
/// `Got`. In causal order the first prefix is undecided; the second holds
/// `Note` unordered after `Placed`, so `Note`'s enabledness is `violated`
/// from the second prefix on and every later prefix keeps it, ending at the
/// batch verdict, `refuted`.
#[trace("TC-2531", "FR-895-AC-11", "FSpec-FR-059-AC-4")]
#[test]
fn incremental_assessment_reports_a_violation_from_the_prefix_that_shows_it() {
    use fsl_analyze::protocol_analysis::recording::{
        assess_recording, assess_recording_incrementally, RecordingCause, RecordingVerdict,
    };
    let package = crate::litmus::package_called("analysis-unit", &told_protocol());
    let bytes = told_recording([0, 1, 2, 3, 4, 5], json!([2, 1, 4, 5])).to_string();
    let incremental = assess_recording_incrementally(&package, bytes.as_bytes()).expect("prefixes");
    assert_eq!(
        prefix_labels(&incremental),
        [
            "inconclusive",
            "refuted",
            "refuted",
            "refuted",
            "refuted",
            "refuted"
        ]
    );
    assert_violations_persist(&incremental);
    let note = incremental[1]
        .obligations
        .iter()
        .find(|entry| {
            entry.obligation.kind == "enabledness"
                && entry.obligation.node.as_deref() == Some("Flow::Main::Note")
        })
        .expect("Note enabledness");
    assert_eq!(note.verdict, RecordingVerdict::Violated);
    assert_eq!(note.cause, Some(RecordingCause::Unexpected));
    let batch = assess_recording(&package, bytes.as_bytes()).expect("batch");
    assert_eq!(incremental.last(), Some(&batch));
}

/// One `Flow` replay item with `assessment` set to `assessment`.
fn assessed_item(assessment: &str) -> Value {
    json!([{"claim": "Flow", "kind": "global-conformance",
            "recording": recording_identity(), "assessment": assessment}])
}

/// FR-896-AC-5 (TC-2531): `assessment` `incremental` carries one prefix
/// assessment per observation, the violation shows from the prefix that adds
/// `Note`, and the last prefix equals the item's own obligations; `batch`
/// carries no prefixes; another word, or no recording, refuses the request.
#[trace("TC-2531", "FR-896-AC-5", "FR-895-AC-6")]
#[test]
fn the_incremental_assessment_member_reports_each_prefix() {
    let probe = told_recording([0, 1, 2, 3, 4, 5], json!([2, 1, 4, 5]));
    let entry = json!({"file": "recording.json", "identity": recording_identity()});
    let run = |name: &str, assessment: &str| {
        replay_with(
            name,
            &told_protocol(),
            &probe,
            entry.clone(),
            assessed_item(assessment),
        )
    };

    let item = run("replay-incremental", "incremental").item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    let prefixes = item["prefixes"]
        .as_array()
        .unwrap_or_else(|| panic!("one prefix each: {item}"));
    let labels: Vec<&str> = prefixes
        .iter()
        .map(|prefix| prefix["label"].as_str().expect("a label"))
        .collect();
    assert_eq!(
        labels,
        [
            "inconclusive",
            "refuted",
            "refuted",
            "refuted",
            "refuted",
            "refuted"
        ],
        "{item}"
    );
    let counts: Vec<u64> = prefixes
        .iter()
        .map(|prefix| prefix["observations"].as_u64().expect("a count"))
        .collect();
    assert_eq!(counts, [1, 2, 3, 4, 5, 6], "{item}");
    let note = |prefix: &Value| {
        prefix["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .find(|verdict| {
                verdict["obligation"]["kind"] == "enabledness"
                    && verdict["obligation"]["node"] == "Flow::Main::Note"
            })
            .cloned()
            .expect("a Note enabledness verdict")
    };
    for prefix in &prefixes[1..] {
        let note = note(prefix);
        assert_eq!(note["verdict"], "violated", "{item}");
        assert_eq!(note["cause"], "unexpected", "{item}");
    }
    assert_eq!(
        prefixes[5]["obligations"], item["obligations"],
        "the last prefix is the batch assessment"
    );

    for assessment in ["batch"] {
        let item = run("replay-batch", assessment).item(0);
        assert!(item.get("prefixes").is_none(), "{item}");
        assert_eq!(item["obligations"], prefixes[5]["obligations"], "{item}");
    }
    let none = replay_with(
        "replay-no-assessment",
        &told_protocol(),
        &probe,
        entry.clone(),
        flow_item(recording_identity()),
    )
    .item(0);
    assert!(none.get("prefixes").is_none(), "{none}");
    assert_eq!(none["obligations"], prefixes[5]["obligations"], "{none}");

    let refused = |name: &str, items: Value, names: &str| {
        let run = replay_with(name, &told_protocol(), &probe, entry.clone(), items);
        assert_eq!(run.exit, 20, "{name}");
        let document = run.document();
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid-request",
            "{name}: {document}"
        );
        assert!(document.to_string().contains(names), "{name}: {document}");
    };
    // A word other than `batch` or `incremental` is no value of the member.
    refused(
        "replay-assessment-word",
        assessed_item("streaming"),
        "streaming",
    );
    refused(
        "replay-assessment-no-recording",
        json!([{"claim": "Flow", "kind": "global-conformance",
                "assessment": "incremental"}]),
        "/items/0/assessment",
    );
}

// ---- FR-895 "Enabledness": predecessors through choice, parallel, repeat ----

/// The enabledness obligation of the observation at `position`, naming `node`.
fn enabledness<'a>(
    assessment: &'a fsl_analyze::protocol_analysis::recording::RecordingAssessment,
    node: &str,
    position: usize,
) -> &'a fsl_analyze::protocol_analysis::recording::RecordingObligation {
    assessment
        .obligations
        .iter()
        .find(|entry| {
            entry.obligation.kind == "enabledness"
                && entry.obligation.node.as_deref() == Some(node)
                && entry.witness == [position]
        })
        .unwrap_or_else(|| panic!("enabledness of {node} at {position}: {assessment:#?}"))
}

/// The checked package of `source`: a `P` unit of the `fill` fixture, else a
/// `Flow` unit of the litmus fixture.
fn package_of(source: &str) -> fsl_package::CheckedPackage {
    if source.contains("protocol P using") {
        crate::fill::package(source)
    } else {
        crate::litmus::package_called("analysis-unit", source)
    }
}

/// Assert `node` at `position` is `satisfied`.
fn assert_enabled(source: &str, recording: &Value, node: &str, position: usize) {
    use fsl_analyze::protocol_analysis::recording::{assess_recording, RecordingVerdict};
    let package = package_of(source);
    let assessed = assess_recording(&package, recording.to_string().as_bytes()).expect("readable");
    let found = enabledness(&assessed, node, position);
    assert_eq!(found.verdict, RecordingVerdict::Satisfied, "{found:#?}");
    assert_eq!(found.cause, None, "{found:#?}");
}

/// Assert `node` at `position` is `violated`, cause `unexpected`.
fn assert_unexpected(source: &str, recording: &Value, node: &str, position: usize) {
    use fsl_analyze::protocol_analysis::recording::{
        assess_recording, RecordingCause, RecordingVerdict,
    };
    let package = package_of(source);
    let assessed = assess_recording(&package, recording.to_string().as_bytes()).expect("readable");
    let found = enabledness(&assessed, node, position);
    assert_eq!(found.verdict, RecordingVerdict::Violated, "{found:#?}");
    assert_eq!(found.cause, Some(RecordingCause::Unexpected), "{found:#?}");
}

/// `Flow` with a `Tell` channel and a step in each case of `Decide`
/// (`Dropped` in `no`, `Told` in `yes`) and a send `Note` after the choice.
fn choice_protocol() -> String {
    let mut source = order_protocol(true)
        .replace(
            "capacity 4 overflow reject;\n",
            "capacity 4 overflow reject;\n\
             channel Tell from Seller to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n",
        )
        .replace(
            "sequence Cancel { }",
            "sequence Cancel { send Dropped via Tell as (dropped: Ack) { true }; }",
        )
        .replace(
            "sequence Ship { }",
            "sequence Ship { send Told via Tell as (told: Ack) { true }; }",
        );
    let finish = source.find("finish Done").expect("finish terminal");
    let close = source[..finish]
        .rfind(" }")
        .expect("the main sequence closes");
    source.insert_str(close, " send Note via Tell as (note: Ack) { true };");
    source
}

/// A closed recording of [`choice_protocol`] from `observations`, `Seller`
/// order `seller`, and the `Placed` to `Got` correlation.
fn choice_recording(observations: Value, seller: Value) -> Value {
    recording(
        observations,
        json!({"Buyer": [0], "Seller": seller}),
        json!([{"send": 0, "receive": 1}]),
    )
}

/// FR-895-AC-12 (TC-2531): after a `choice`, only the steps of the case
/// taken are predecessors; `Told` taken in `yes` and `Dropped` of the untaken
/// `no` never observed, `Note` is enabled.
#[trace("TC-2531", "FR-895-AC-12")]
#[test]
fn a_step_after_a_choice_needs_only_the_taken_case() {
    let observations = json!([
        observation("send", "Placed", "Buyer", "m-1"),
        observation("receive", "Got", "Seller", "m-1"),
        chosen_yes(),
        observation("send", "Decide::yes::Ship::Told", "Seller", "m-2"),
        observation("send", "Note", "Seller", "m-3"),
    ]);
    assert_enabled(
        &choice_protocol(),
        &choice_recording(observations, json!([1, 2, 3, 4])),
        "Flow::Main::Note",
        4,
    );
}

/// FR-895-AC-13 (TC-2531): the same recording without `Told`, a step of the
/// taken case, leaves `Note` `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-13")]
#[test]
fn a_step_after_a_choice_is_unexpected_without_the_taken_cases_steps() {
    let observations = json!([
        observation("send", "Placed", "Buyer", "m-1"),
        observation("receive", "Got", "Seller", "m-1"),
        chosen_yes(),
        observation("send", "Note", "Seller", "m-3"),
    ]);
    assert_unexpected(
        &choice_protocol(),
        &choice_recording(observations, json!([1, 2, 3])),
        "Flow::Main::Note",
        3,
    );
}

/// `Flow` with a `parallel` of two `Buyer` events under `join all` before
/// `Placed`.
fn parallel_protocol() -> String {
    order_protocol(false).replace(
        "run sequence Main {",
        "run sequence Main { parallel Both memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Buyer as (r: Ack) { true }; } } join all [left, right];",
    )
}

/// A closed recording of [`parallel_protocol`], all `Buyer` observations in
/// array order.
fn parallel_recording(observations: Value) -> Value {
    let count = observations.as_array().expect("observations").len();
    json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": observations,
        "local_order": {"Buyer": (0..count).collect::<Vec<_>>()},
        "correlations": [],
        "closure": {"state": "closed", "premises": [
            {"kind": "execution", "node": "Flow::Done"},
            {"kind": "branch", "node": "Flow::Main::Both::left"},
            {"kind": "branch", "node": "Flow::Main::Both::right"}
        ]}
    })
}

/// A `Buyer` event at `Flow::Main::Both::<branch>::<name>`.
fn branch_event(branch: &str, body: &str, name: &str) -> Value {
    json!({"kind": "event", "node": format!("Flow::Main::Both::{branch}::{body}::{name}"),
           "role": "Buyer", "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// FR-895-AC-14 (TC-2531): after a `parallel` under `join all`, the steps of
/// every branch are predecessors: `Placed` after `L` and `R` is enabled.
#[trace("TC-2531", "FR-895-AC-14")]
#[test]
fn a_step_after_an_all_join_needs_every_branch() {
    let observations = json!([
        branch_event("left", "Left", "L"),
        branch_event("right", "Right", "R"),
        observation("send", "Placed", "Buyer", "m-1"),
    ]);
    assert_enabled(
        &parallel_protocol(),
        &parallel_recording(observations),
        "Flow::Main::Placed",
        2,
    );
}

/// FR-895-AC-15 (TC-2531): `Placed` after `L` alone, `R` never observed in a
/// closed recording, is `violated`, cause `unexpected`; and `R` alone is
/// enabled, since a branch's steps have no order against a sibling branch.
#[trace("TC-2531", "FR-895-AC-15")]
#[test]
fn a_step_after_an_all_join_is_unexpected_without_a_branch_and_branches_are_unordered() {
    let observations = json!([
        branch_event("left", "Left", "L"),
        observation("send", "Placed", "Buyer", "m-1"),
    ]);
    assert_unexpected(
        &parallel_protocol(),
        &parallel_recording(observations),
        "Flow::Main::Placed",
        1,
    );
    assert_enabled(
        &parallel_protocol(),
        &parallel_recording(json!([branch_event("right", "Right", "R")])),
        "Flow::Main::Both::right::Right::R",
        0,
    );
}

/// `Flow` whose `Placed`, `Got` and `Decide` run in a `repeat` `Loop` of at
/// most two iterations.
fn repeat_protocol() -> String {
    let mut source = order_protocol(true).replace(
        "run sequence Main {",
        "run sequence Main { repeat Loop by Seller visible () max 2 while { true } sequence Body {",
    );
    let finish = source.find("finish Done").expect("finish terminal");
    let close = source[..finish]
        .rfind(" }")
        .expect("the main sequence closes");
    source.replace_range(close..close + 2, " } exhausted sequence Out { } }");
    source
}

/// A closed recording of [`repeat_protocol`].
fn repeat_recording(
    observations: Value,
    seller: Value,
    buyer: Value,
    correlations: Value,
) -> Value {
    json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": observations,
        "local_order": {"Buyer": buyer, "Seller": seller},
        "correlations": correlations,
        "closure": {"state": "closed", "premises": [
            {"kind": "execution", "node": "Flow::Done"},
            {"kind": "branch", "node": "Flow::Main::Loop::Body::Decide::no"},
            {"kind": "branch", "node": "Flow::Main::Loop::Body::Decide::yes"}
        ]}
    })
}

/// An observation at `Flow::Main::Loop::Body::<node>`.
fn looped(kind: &str, node: &str, role: &str, message: &str) -> Value {
    json!({"kind": kind, "node": format!("Flow::Main::Loop::Body::{node}"), "role": role,
           "instance": "w-1",
           "keys": {"message": message, "delivery": format!("d-{message}")},
           "record": {"ok": true}})
}

/// FR-895-AC-16 (TC-2531): in a `repeat`, the second `Got` is enabled by the
/// second `Placed`.
#[trace("TC-2531", "FR-895-AC-16")]
#[test]
fn an_iteration_is_enabled_by_its_own_earlier_steps() {
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
    ]);
    assert_enabled(
        &repeat_protocol(),
        &repeat_recording(
            observations,
            json!([1, 3]),
            json!([0, 2]),
            json!([{"send": 0, "receive": 1}, {"send": 2, "receive": 3}]),
        ),
        "Flow::Main::Loop::Body::Got",
        3,
    );
}

/// FR-895-AC-17 (TC-2531): a second `Got` with no second `Placed` is
/// `violated`, cause `unexpected`, though the first iteration's `Placed`
/// precedes it.
#[trace("TC-2531", "FR-895-AC-17")]
#[test]
fn an_earlier_iteration_does_not_enable_a_later_one() {
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped("receive", "Got", "Seller", "m-2"),
    ]);
    assert_unexpected(
        &repeat_protocol(),
        &repeat_recording(
            observations,
            json!([1, 2]),
            json!([0]),
            json!([{"send": 0, "receive": 1}]),
        ),
        "Flow::Main::Loop::Body::Got",
        2,
    );
}

/// [`repeat_protocol`] with an event `After` of `Seller` after the `repeat`.
fn repeat_then_protocol() -> String {
    let mut source = repeat_protocol();
    let finish = source.find("finish Done").expect("finish terminal");
    let close = source[..finish]
        .rfind(" }")
        .expect("the main sequence closes");
    source.insert_str(close, " event After by Seller as (af: Ack) { true };");
    source
}

/// The `After` event of [`repeat_then_protocol`].
fn after() -> Value {
    json!({"kind": "event", "node": "Flow::Main::After", "role": "Seller",
           "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// The `Decide` case observation `case` (`yes` or `no`) of `Seller` in the
/// loop body.
fn looped_case(case: &str) -> Value {
    json!({"kind": "case", "node": format!("Flow::Main::Loop::Body::Decide::{case}"),
           "role": "Seller", "instance": "w-1", "keys": {}})
}

/// A closed recording of [`repeat_then_protocol`] from `observations`:
/// `Buyer` sends at `buyer`, `Seller` observations at `seller`, and
/// correlations `(send, receive)`.
fn after_recording(
    observations: Value,
    buyer: &[usize],
    seller: &[usize],
    correlations: &[(usize, usize)],
) -> Value {
    repeat_recording(
        observations,
        json!(seller),
        json!(buyer),
        Value::Array(
            correlations
                .iter()
                .map(|(send, receive)| json!({"send": send, "receive": receive}))
                .collect(),
        ),
    )
}

/// FR-895-AC-20 (TC-2531): a step after a `repeat` run zero times is enabled,
/// and one after two iterations is enabled by the last iteration's steps.
#[trace("TC-2531", "FR-895-AC-20")]
#[test]
fn a_step_after_a_repeat_needs_its_last_iteration() {
    let source = repeat_then_protocol();
    assert_enabled(
        &source,
        &after_recording(json!([after()]), &[], &[0], &[]),
        "Flow::Main::After",
        0,
    );
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
        after(),
    ]);
    assert_enabled(
        &source,
        &after_recording(observations, &[0, 3], &[1, 2, 4, 5, 6], &[(0, 1), (3, 4)]),
        "Flow::Main::After",
        6,
    );
}

/// FR-895-AC-21 (TC-2531): the second iteration's `Got` missing leaves
/// `After` `violated`, cause `unexpected`, once closed, though the first
/// iteration's `Got` precedes it, and `pending` while the closure is open.
#[trace("TC-2531", "FR-895-AC-21")]
#[test]
fn an_earlier_iteration_does_not_enable_a_step_after_the_repeat() {
    use fsl_analyze::protocol_analysis::recording::{assess_recording, RecordingVerdict};
    let source = repeat_then_protocol();
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped_case("yes"),
        after(),
    ]);
    let closed = after_recording(observations, &[0], &[1, 2, 3, 4], &[(0, 1)]);
    assert_unexpected(&source, &closed, "Flow::Main::After", 4);
    let mut open = closed;
    open["closure"] = json!({"state": "open", "premises": []});
    let assessed =
        assess_recording(&package_of(&source), open.to_string().as_bytes()).expect("readable");
    assert_eq!(
        enabledness(&assessed, "Flow::Main::After", 4).verdict,
        RecordingVerdict::Pending
    );
}

/// FR-895-AC-22 (TC-2531): iterations that take different cases each judge
/// their own: `yes` in the first iteration and `no` in the last leave `After`
/// enabled; with the last iteration's `Got` missing it is `violated`.
#[trace("TC-2531", "FR-895-AC-22")]
#[test]
fn iterations_taking_different_cases_each_use_their_own() {
    let source = repeat_then_protocol();
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("no"),
        after(),
    ]);
    assert_enabled(
        &source,
        &after_recording(observations, &[0, 3], &[1, 2, 4, 5, 6], &[(0, 1), (3, 4)]),
        "Flow::Main::After",
        6,
    );
    let without_got = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped_case("no"),
        after(),
    ]);
    assert_unexpected(
        &source,
        &after_recording(without_got, &[0, 3], &[1, 2, 4, 5], &[(0, 1)]),
        "Flow::Main::After",
        5,
    );
}

/// [`repeat_then_protocol`] with an event `Shipped` in the `yes` case.
fn repeat_case_step_protocol() -> String {
    repeat_then_protocol().replace(
        "sequence Ship { }",
        "sequence Ship { event Shipped by Seller as (sh: Ack) { true }; }",
    )
}

/// `Shipped` of the loop body's `yes` case.
fn shipped() -> Value {
    json!({"kind": "event", "node": "Flow::Main::Loop::Body::Decide::yes::Ship::Shipped",
           "role": "Seller", "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// FR-895-AC-23 (TC-2531): a step of the case taken in the last iteration is
/// a predecessor of `After`, and a step of a case taken only in an earlier
/// iteration is not.
#[trace("TC-2531", "FR-895-AC-23")]
#[test]
fn a_case_step_in_the_last_iteration_precedes_the_step_after_the_repeat() {
    let source = repeat_case_step_protocol();
    let first = [
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        shipped(),
    ];
    let second = |case: &str, extra: Vec<Value>| {
        let mut observations = first.to_vec();
        observations.extend([
            looped("send", "Placed", "Buyer", "m-2"),
            looped("receive", "Got", "Seller", "m-2"),
            looped_case(case),
        ]);
        observations.extend(extra);
        observations.push(after());
        observations
    };
    let seller = |count: usize| {
        (0..count)
            .filter(|index| *index != 0 && *index != 4)
            .collect::<Vec<_>>()
    };
    // `yes` twice, `Shipped` in both: enabled.
    let both = second("yes", vec![shipped()]);
    let count = both.len();
    assert_enabled(
        &source,
        &after_recording(json!(both), &[0, 4], &seller(count), &[(0, 1), (4, 5)]),
        "Flow::Main::After",
        count - 1,
    );
    // `yes` twice, the last `Shipped` missing: violated.
    let missing = second("yes", vec![]);
    let count = missing.len();
    assert_unexpected(
        &source,
        &after_recording(json!(missing), &[0, 4], &seller(count), &[(0, 1), (4, 5)]),
        "Flow::Main::After",
        count - 1,
    );
    // `yes` then `no`: the first iteration's `Shipped` is not required.
    let other = second("no", vec![]);
    let count = other.len();
    assert_enabled(
        &source,
        &after_recording(json!(other), &[0, 4], &seller(count), &[(0, 1), (4, 5)]),
        "Flow::Main::After",
        count - 1,
    );
}

/// [`repeat_then_protocol`] whose `exhausted` path holds an event `X`.
fn repeat_exhausted_protocol() -> String {
    repeat_then_protocol().replace(
        "exhausted sequence Out { }",
        "exhausted sequence Out { event X by Seller as (x: Ack) { true }; }",
    )
}

/// `X` of the `exhausted` path.
fn exhausted_x() -> Value {
    json!({"kind": "event", "node": "Flow::Main::Loop::Out::X", "role": "Seller",
           "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// FR-895-AC-24 (TC-2531): the `exhausted` path is an alternative exit run
/// once after the last iteration, not part of an iteration: after two
/// iterations and `X`, `After` and `X` are enabled; `X` without the last
/// iteration's `Got` is `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-24")]
#[test]
fn the_exhausted_path_follows_the_last_iteration_and_precedes_the_step_after() {
    let source = repeat_exhausted_protocol();
    let full = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
        exhausted_x(),
        after(),
    ]);
    let recording = after_recording(full, &[0, 3], &[1, 2, 4, 5, 6, 7], &[(0, 1), (3, 4)]);
    assert_enabled(&source, &recording, "Flow::Main::Loop::Out::X", 6);
    assert_enabled(&source, &recording, "Flow::Main::After", 7);
    let without_got = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-2"),
        looped_case("yes"),
        exhausted_x(),
    ]);
    assert_unexpected(
        &source,
        &after_recording(without_got, &[0, 3], &[1, 2, 4, 5], &[(0, 1)]),
        "Flow::Main::Loop::Out::X",
        5,
    );
}

/// [`parallel_protocol`] under `join any`.
fn any_join_protocol() -> String {
    parallel_protocol().replace(
        "join all [left, right];",
        "join any [left, right] outstanding continue;",
    )
}

/// FR-895-AC-25 (TC-2531): after a `parallel` under `join any`, the branches
/// whose steps were all observed are predecessors, and an unobserved branch
/// is not: `Placed` after `L` alone is enabled; `L` observed after `Placed`
/// leaves `Placed` `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-25")]
#[test]
fn a_step_after_an_any_join_needs_the_observed_branches() {
    let source = any_join_protocol();
    assert_enabled(
        &source,
        &parallel_recording(json!([
            branch_event("left", "Left", "L"),
            observation("send", "Placed", "Buyer", "m-1"),
        ])),
        "Flow::Main::Placed",
        1,
    );
    assert_unexpected(
        &source,
        &parallel_recording(json!([
            observation("send", "Placed", "Buyer", "m-1"),
            branch_event("left", "Left", "L"),
        ])),
        "Flow::Main::Placed",
        0,
    );
}

/// `P` with `Start`, two `await`s `W1` and `W2` after it, and `End`.
fn await_protocol() -> String {
    let wait = |name: &str, seen: &str, binder: &str| {
        format!(
            "await {name} after Start using t0 clock \"c\" within [0,1] \
             match event {seen} by w as ({binder}: Boolean) {{ true }}; \
             then sequence T{name} {{ }} timeout sequence O{name} {{ }} "
        )
    };
    let run = format!(
        "sequence Main {{ event Start by w as (st: Boolean) {{ true }}; {}{}event End by w as (en: Boolean) {{ true }}; }}",
        wait("W1", "Seen1", "s1"),
        wait("W2", "Seen2", "s2"),
    );
    crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d")
}

/// A closed recording of [`await_protocol`]: events by `w` in local order.
fn await_recording(nodes: &[&str]) -> Value {
    let observations: Vec<Value> = nodes
        .iter()
        .map(|node| {
            json!({"kind": "event", "node": format!("P::Main::{node}"), "role": "w",
                   "instance": "w-1", "keys": {}, "record": {"ok": true}})
        })
        .collect();
    json!({
        "format": "filament.protocol-recording/1", "protocol": "P",
        "observations": observations,
        "local_order": {"w": (0..nodes.len()).collect::<Vec<_>>()},
        "correlations": [],
        "closure": {"state": "closed", "premises": [{"kind": "execution", "node": "P::Done"}]}
    })
}

/// FR-895-AC-18 (TC-2531): `await` siblings are unordered and supply no
/// predecessors: the event matched by `W2` and `End` are enabled with `Start`
/// observed and `W1`'s match never observed.
#[trace("TC-2531", "FR-895-AC-18")]
#[test]
fn await_siblings_are_not_predecessors_of_each_other() {
    let source = await_protocol();
    let recording = await_recording(&["Start", "W2::Seen2", "End"]);
    assert_enabled(&source, &recording, "P::Main::W2::Seen2", 1);
    assert_enabled(&source, &recording, "P::Main::End", 2);
}

/// FR-895-AC-19 (TC-2531): without `Start`, the step before the `await`s,
/// the event matched by `W2` and `End` are `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-19")]
#[test]
fn a_step_after_awaits_still_needs_the_steps_before_them() {
    let source = await_protocol();
    let recording = await_recording(&["W2::Seen2", "End"]);
    assert_unexpected(&source, &recording, "P::Main::W2::Seen2", 0);
    assert_unexpected(&source, &recording, "P::Main::End", 1);
}

/// FR-895-AC-26 (TC-2531): incremental prefixes follow the recording's causal
/// order, never the array position. `After` listed before the second
/// iteration in the array is a causal successor of it, so no prefix holds
/// `After` before its last iteration is complete: it is absent from every
/// prefix but the last, where it is `satisfied`, and no obligation ever
/// moves from `violated`, or from `satisfied` to `pending`.
#[trace("TC-2531", "FR-895-AC-26")]
#[test]
fn incremental_prefixes_follow_causal_order_so_a_verdict_never_returns_to_pending() {
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        after(),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
    ]);
    let recording = after_recording(observations, &[0, 4], &[1, 2, 5, 6, 3], &[(0, 1), (4, 5)]);
    let item = replay_with(
        "replay-causal-prefixes",
        &repeat_then_protocol(),
        &recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        assessed_item("incremental"),
    )
    .item(0);
    let prefixes = item["prefixes"].as_array().expect("prefixes");
    assert_eq!(prefixes.len(), 7, "{item}");
    let after_verdicts: Vec<Option<&str>> = prefixes
        .iter()
        .map(|prefix| {
            prefix["obligations"]
                .as_array()
                .expect("obligations")
                .iter()
                .find(|verdict| {
                    verdict["obligation"]["kind"] == "enabledness"
                        && verdict["obligation"]["node"] == "Flow::Main::After"
                })
                .map(|verdict| verdict["verdict"].as_str().expect("a verdict"))
        })
        .collect();
    assert_eq!(
        after_verdicts,
        [None, None, None, None, None, None, Some("satisfied")],
        "{item}"
    );
    assert_every_verdict_only_advances(prefixes);
    for prefix in &prefixes[..6] {
        for channel in prefix["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .filter(|entry| entry["obligation"]["kind"] == "channel-premise")
        {
            assert_eq!(channel["verdict"], "pending", "{prefix}");
        }
    }
    assert_eq!(
        prefixes[6]["obligations"], item["obligations"],
        "the last prefix is the batch assessment"
    );
}

/// Every obligation of every prefix keeps its verdict's direction in the
/// next prefix: `pending` becomes a decision, `satisfied` may become
/// `violated`, nothing leaves `violated` and nothing returns to `pending`.
///
/// Obligations of repeated steps share one id, and a prefix may add an
/// observation anywhere in the array, so obligations are compared per id as
/// a multiset: the number of `violated` verdicts and the number of decided
/// verdicts under one id never fall from one prefix to the next.
fn assert_every_verdict_only_advances(prefixes: &[Value]) {
    let tally = |prefix: &Value| {
        let mut tally: std::collections::BTreeMap<String, (usize, usize)> =
            std::collections::BTreeMap::new();
        for entry in prefix["obligations"].as_array().expect("obligations") {
            let counts = tally.entry(entry["obligation"].to_string()).or_default();
            if entry["verdict"] == "violated" {
                counts.0 += 1;
            }
            if entry["verdict"] != "pending" {
                counts.1 += 1;
            }
        }
        tally
    };
    for pair in prefixes.windows(2) {
        let (before, after) = (tally(&pair[0]), tally(&pair[1]));
        for (id, (violated, decided)) in &before {
            let Some((next_violated, next_decided)) = after.get(id) else {
                continue;
            };
            assert!(
                next_violated >= violated && next_decided >= decided,
                "{id}: violated {violated} -> {next_violated}, decided {decided} -> {next_decided}"
            );
        }
    }
}

/// The helper fails a repeated step whose second obligation, sharing the
/// first's id, goes back to `pending` while the first stays decided.
#[test]
fn the_monotonicity_check_covers_every_occurrence_of_a_repeated_step() {
    let id = json!({"kind": "enabledness", "node": "Flow::Main::Placed", "instance": "w-1"});
    let prefix = |first: &str, second: &str| {
        json!({"obligations": [
            {"obligation": id, "verdict": first},
            {"obligation": id, "verdict": second},
        ]})
    };
    assert_every_verdict_only_advances(&[
        prefix("satisfied", "pending"),
        prefix("satisfied", "satisfied"),
    ]);
    let regressed = std::panic::catch_unwind(|| {
        assert_every_verdict_only_advances(&[
            prefix("satisfied", "satisfied"),
            prefix("satisfied", "pending"),
        ]);
    });
    assert!(
        regressed.is_err(),
        "a second occurrence going pending fails"
    );
    let reopened = std::panic::catch_unwind(|| {
        assert_every_verdict_only_advances(&[
            prefix("violated", "pending"),
            prefix("satisfied", "pending"),
        ]);
    });
    assert!(
        reopened.is_err(),
        "a violated occurrence leaving violated fails"
    );
}

/// `After`'s enabledness verdict in each prefix, `None` where the prefix has
/// no such obligation yet.
fn after_verdicts(prefixes: &[Value]) -> Vec<Option<&str>> {
    node_verdicts(prefixes, "Flow::Main::After")
}

/// The enabledness verdict of `node` in each prefix, `None` where the prefix
/// has no such obligation yet.
fn node_verdicts<'a>(prefixes: &'a [Value], node: &str) -> Vec<Option<&'a str>> {
    prefixes
        .iter()
        .map(|prefix| {
            prefix["obligations"]
                .as_array()
                .expect("obligations")
                .iter()
                .find(|verdict| {
                    verdict["obligation"]["kind"] == "enabledness"
                        && verdict["obligation"]["node"] == node
                })
                .map(|verdict| verdict["verdict"].as_str().expect("a verdict"))
        })
        .collect()
}

/// FR-895-AC-28 (TC-2531): under `join any … outstanding continue` the other
/// branch keeps running after the join (ADR-027 ST-7), so a branch counts as
/// a predecessor only when its steps were observed causally before the step.
/// `L`, `Placed`, `R` is `satisfied` for `Placed` in batch and from the
/// second prefix on; `R`, `L`, `Placed` (both branches before) stays
/// `satisfied`; under `join all`, `L`, `Placed`, `R` is `violated`, cause
/// `unexpected`.
#[trace("TC-2531", "FR-895-AC-28")]
#[test]
fn a_branch_finishing_after_an_any_join_does_not_violate_the_step() {
    let left = || branch_event("left", "Left", "L");
    let right = || branch_event("right", "Right", "R");
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let incremental = |name: &str, source: &str, observations: Value| {
        let mut recording = parallel_recording(observations);
        recording["closure"] = json!({"state": "open", "premises": []});
        replay_with(
            name,
            source,
            &recording,
            json!({"file": "recording.json", "identity": recording_identity()}),
            assessed_item("incremental"),
        )
        .item(0)
    };
    let source = any_join_protocol();

    let late = incremental(
        "replay-any-join-late-branch",
        &source,
        json!([left(), placed(), right()]),
    );
    let prefixes = late["prefixes"].as_array().expect("prefixes");
    assert_eq!(
        node_verdicts(prefixes, "Flow::Main::Placed"),
        [None, Some("satisfied"), Some("satisfied")],
        "{late}"
    );
    assert_every_verdict_only_advances(prefixes);
    assert_enabled(
        &source,
        &parallel_recording(json!([left(), placed(), right()])),
        "Flow::Main::Placed",
        1,
    );

    let early = incremental(
        "replay-any-join-both-before",
        &source,
        json!([right(), left(), placed()]),
    );
    let prefixes = early["prefixes"].as_array().expect("prefixes");
    assert_eq!(
        node_verdicts(prefixes, "Flow::Main::Placed"),
        [None, None, Some("satisfied")],
        "{early}"
    );
    assert_every_verdict_only_advances(prefixes);

    assert_unexpected(
        &parallel_protocol(),
        &parallel_recording(json!([left(), placed(), right()])),
        "Flow::Main::Placed",
        1,
    );
}

/// FR-895-AC-27 (TC-2531): an iteration begun concurrently with `After` does
/// not reopen it. The AC-26 recording with a third `Placed` by `Buyer` after
/// the second, unordered with `After`, closure `open`, leaves `After`
/// `satisfied` in the seventh prefix and the eighth: the last iteration is
/// the one begun causally before `After`.
#[trace("TC-2531", "FR-895-AC-27")]
#[test]
fn an_iteration_begun_concurrently_with_the_step_does_not_reopen_it() {
    let observations = json!([
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        after(),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
        looped("send", "Placed", "Buyer", "m-3"),
    ]);
    let mut recording = after_recording(
        observations,
        &[0, 4, 7],
        &[1, 2, 5, 6, 3],
        &[(0, 1), (4, 5)],
    );
    recording["closure"] = json!({"state": "open", "premises": []});
    let item = replay_with(
        "replay-concurrent-iteration",
        &repeat_then_protocol(),
        &recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        assessed_item("incremental"),
    )
    .item(0);
    let prefixes = item["prefixes"].as_array().expect("prefixes");
    assert_eq!(prefixes.len(), 8, "{item}");
    assert_eq!(
        after_verdicts(prefixes),
        [
            None,
            None,
            None,
            None,
            None,
            None,
            Some("satisfied"),
            Some("satisfied")
        ],
        "{item}"
    );
    assert_every_verdict_only_advances(prefixes);
    assert_eq!(prefixes[7]["obligations"], item["obligations"], "{item}");
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
