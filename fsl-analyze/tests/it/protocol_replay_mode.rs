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
    assert_every_verdict_only_advances(prefixes, &[0, 1, 2, 4, 5, 6, 3]);
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
/// `order` lists the recording's observation indices in causal order, so
/// prefix `k` holds `order[..k]` renumbered by array position. An
/// enabledness obligation is paired across prefixes by its observation's
/// original index, so repeated steps (which share one id) are each checked
/// and a regression cannot be masked by another occurrence becoming decided;
/// every other obligation is compared per id as a multiset.
fn assert_every_verdict_only_advances(prefixes: &[Value], order: &[usize]) {
    let original = |prefix: usize, local: u64| -> usize {
        let mut kept = order[..=prefix].to_vec();
        kept.sort_unstable();
        kept[usize::try_from(local).expect("a local index")]
    };
    // (id, original index of an enabledness observation) to its verdict.
    let keyed = |prefix: usize| {
        let mut verdicts = std::collections::BTreeMap::new();
        let mut tally: std::collections::BTreeMap<String, (usize, usize)> =
            std::collections::BTreeMap::new();
        for entry in prefixes[prefix]["obligations"]
            .as_array()
            .expect("obligations")
        {
            let id = entry["obligation"].to_string();
            let verdict = entry["verdict"].as_str().expect("a verdict").to_owned();
            if entry["obligation"]["kind"] == "enabledness" {
                let local = entry["witness"][0].as_u64().expect("the observation");
                verdicts.insert((id, original(prefix, local)), verdict);
            } else {
                let counts = tally.entry(id).or_default();
                counts.0 += usize::from(verdict == "violated");
                counts.1 += usize::from(verdict != "pending");
            }
        }
        (verdicts, tally)
    };
    for step in 0..prefixes.len().saturating_sub(1) {
        let ((before, before_tally), (after, after_tally)) = (keyed(step), keyed(step + 1));
        for (key, was) in &before {
            let Some(now) = after.get(key) else {
                continue;
            };
            assert!(
                was != "violated" || now == "violated",
                "{key:?}: {was} became {now}"
            );
            assert!(
                was != "satisfied" || now != "pending",
                "{key:?}: {was} became {now}"
            );
        }
        for (id, (violated, decided)) in &before_tally {
            let Some((next_violated, next_decided)) = after_tally.get(id) else {
                continue;
            };
            assert!(
                next_violated >= violated && next_decided >= decided,
                "{id}: violated {violated} -> {next_violated}, decided {decided} -> {next_decided}"
            );
        }
    }
}

/// The helper pairs the occurrences of a repeated step by observation: a
/// regressed occurrence is caught even when another occurrence of the same id
/// becomes decided in the same prefix and keeps the counts equal.
#[test]
fn the_monotonicity_check_pairs_occurrences_of_a_repeated_step_by_observation() {
    let id = json!({"kind": "enabledness", "node": "Flow::Main::Placed", "instance": "w-1"});
    let first = |verdict: &str| json!({"obligation": id, "verdict": verdict, "witness": [0]});
    let second = |verdict: &str| json!({"obligation": id, "verdict": verdict, "witness": [1]});
    let advancing = [
        json!({"obligations": [first("satisfied")]}),
        json!({"obligations": [first("satisfied"), second("pending")]}),
        json!({"obligations": [first("satisfied"), second("violated")]}),
    ];
    assert_every_verdict_only_advances(&advancing, &[0, 1, 2]);
    let masked = std::panic::catch_unwind(|| {
        let masked = [
            json!({"obligations": [first("violated")]}),
            json!({"obligations": [first("violated"), second("pending")]}),
            json!({"obligations": [first("satisfied"), second("violated")]}),
        ];
        assert_every_verdict_only_advances(&masked, &[0, 1, 2]);
    });
    assert!(
        masked.is_err(),
        "a violated first occurrence turning satisfied is not masked by the second"
    );
    let went_back = std::panic::catch_unwind(|| {
        let went_back = [
            json!({"obligations": [first("satisfied")]}),
            json!({"obligations": [first("satisfied"), second("satisfied")]}),
            json!({"obligations": [first("satisfied"), second("pending")]}),
        ];
        assert_every_verdict_only_advances(&went_back, &[0, 1, 2]);
    });
    assert!(
        went_back.is_err(),
        "a second occurrence going pending fails"
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
    assert_every_verdict_only_advances(prefixes, &[0, 1, 2]);
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
    assert_every_verdict_only_advances(prefixes, &[0, 1, 2]);

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
    assert_every_verdict_only_advances(prefixes, &[0, 1, 2, 4, 5, 6, 3, 7]);
    assert_eq!(prefixes[7]["obligations"], item["obligations"], "{item}");
}

const PLACED: &str = "Flow::Main::Placed";
const LEFT_STEP: &str = "Flow::Main::Both::left::Left::L";
const RIGHT_STEP: &str = "Flow::Main::Both::right::Right::R";

/// The incremental item of an open recording of `Buyer` observations over
/// `source`.
fn parallel_incremental(name: &str, source: &str, observations: &Value) -> Value {
    let mut recording = parallel_recording(observations.clone());
    recording["closure"] = json!({"state": "open", "premises": []});
    replay_with(
        name,
        source,
        &recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        assessed_item("incremental"),
    )
    .item(0)
}

/// Assert the `node` enabledness verdicts of the incremental `item`'s prefixes,
/// that every obligation only advances over them and that the last prefix is
/// the item's own assessment.
fn assert_prefix_verdicts(item: &Value, node: &str, expected: &[Option<&str>]) {
    let prefixes = item["prefixes"].as_array().expect("prefixes");
    assert_eq!(node_verdicts(prefixes, node), expected, "{item}");
    let order: Vec<usize> = (0..prefixes.len()).collect();
    assert_every_verdict_only_advances(prefixes, &order);
    assert_eq!(
        prefixes[prefixes.len() - 1]["obligations"],
        item["obligations"],
        "the last prefix is the batch assessment"
    );
}

/// [`any_join_protocol`] with `outstanding cancel`.
fn cancel_join_protocol() -> String {
    any_join_protocol().replace("outstanding continue", "outstanding cancel")
}

/// FR-895-AC-29 (TC-2531): under `outstanding cancel` a cancelled branch
/// takes no further step (ADR-027 ST-7). With `join any`, `L`, `Placed`, `R`
/// leaves `R`, observed causally after the cancel, `violated`, cause
/// `unexpected`, in batch and from the third prefix, while `L` and `Placed`
/// stay `satisfied`; under `outstanding continue` the same recording leaves
/// `R` `satisfied`.
#[trace("TC-2531", "FR-895-AC-29")]
#[test]
fn a_cancelled_branch_takes_no_step_after_the_cancel() {
    let left = || branch_event("left", "Left", "L");
    let right = || branch_event("right", "Right", "R");
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let late = json!([left(), placed(), right()]);
    let cancel = cancel_join_protocol();
    assert_enabled(&cancel, &parallel_recording(late.clone()), LEFT_STEP, 0);
    assert_enabled(&cancel, &parallel_recording(late.clone()), PLACED, 1);
    assert_unexpected(&cancel, &parallel_recording(late.clone()), RIGHT_STEP, 2);
    let item = parallel_incremental("replay-cancel-late-branch", &cancel, &late);
    assert_prefix_verdicts(&item, RIGHT_STEP, &[None, None, Some("violated")]);
    assert_prefix_verdicts(&item, LEFT_STEP, &[Some("satisfied"); 3]);
    assert_enabled(
        &any_join_protocol(),
        &parallel_recording(late.clone()),
        RIGHT_STEP,
        2,
    );
    let item = parallel_incremental("replay-continue-late-branch", &any_join_protocol(), &late);
    assert_prefix_verdicts(&item, RIGHT_STEP, &[None, None, Some("satisfied")]);
}

/// `parallel_protocol` with `join predicate(<name>)` over `left` and `right`
/// and the function `<name>` of the two branch completions given by `body`.
fn predicate_join_protocol(name: &str, body: &str, outstanding: &str) -> String {
    let source = parallel_protocol().replace(
        "join all [left, right];",
        &format!("join predicate({name}) [left, right] outstanding {outstanding};"),
    );
    format!(
        "function {name} using v(left: Boolean, right: Boolean): Boolean pure {{ {body} }}\n{source}"
    )
}

/// FR-895-AC-30 (TC-2531): a `predicate` join is reached when its function
/// holds over the branches finished causally before the step (FR-218
/// "Predicate joins"). With `left and right`, `L`, `R`, `Placed` leaves
/// `Placed` `satisfied`, and `L`, `Placed`, `R` leaves it `violated`, cause
/// `unexpected`, since the join was not reached when `Placed` was observed;
/// incrementally the second recording gives `Placed` `pending` then
/// `violated`.
#[trace("TC-2531", "FR-895-AC-30")]
#[test]
fn a_predicate_join_is_reached_when_its_function_holds_over_the_finished_branches() {
    let left = || branch_event("left", "Left", "L");
    let right = || branch_event("right", "Right", "R");
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let both = predicate_join_protocol("both", "left and right", "continue");
    let accepted = json!([left(), right(), placed()]);
    assert_enabled(&both, &parallel_recording(accepted.clone()), PLACED, 2);
    let item = parallel_incremental("replay-predicate-both-accepted", &both, &accepted);
    assert_prefix_verdicts(&item, PLACED, &[None, None, Some("satisfied")]);
    let violated = json!([left(), placed(), right()]);
    assert_unexpected(&both, &parallel_recording(violated.clone()), PLACED, 1);
    let item = parallel_incremental("replay-predicate-both-violated", &both, &violated);
    assert_prefix_verdicts(&item, PLACED, &[None, Some("pending"), Some("violated")]);
}

/// FR-895-AC-31 (TC-2531): a predicate that reads only one branch is reached
/// when that branch has finished, whatever the other did. With `left` as the
/// function, `L`, `Placed`, `R` leaves `Placed` `satisfied` and `R` observed
/// after the cancel `violated`, cause `unexpected`, under `outstanding
/// cancel` and `satisfied` under `outstanding continue`; `R`, `Placed` leaves
/// `Placed` `violated`, cause `unexpected`, once closed, since `L` was never
/// observed.
#[trace("TC-2531", "FR-895-AC-31")]
#[test]
fn a_predicate_over_one_branch_cancels_and_enables_by_that_branch_alone() {
    let left = || branch_event("left", "Left", "L");
    let right = || branch_event("right", "Right", "R");
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let late = json!([left(), placed(), right()]);
    let cancel = predicate_join_protocol("left_only", "left", "cancel");
    assert_enabled(&cancel, &parallel_recording(late.clone()), PLACED, 1);
    assert_unexpected(&cancel, &parallel_recording(late.clone()), RIGHT_STEP, 2);
    let item = parallel_incremental("replay-predicate-left-cancel", &cancel, &late);
    assert_prefix_verdicts(&item, PLACED, &[None, Some("satisfied"), Some("satisfied")]);
    assert_prefix_verdicts(&item, RIGHT_STEP, &[None, None, Some("violated")]);
    let keep = predicate_join_protocol("left_only", "left", "continue");
    assert_enabled(&keep, &parallel_recording(late.clone()), RIGHT_STEP, 2);
    let right_first = json!([right(), placed()]);
    assert_unexpected(&keep, &parallel_recording(right_first.clone()), PLACED, 1);
    let item = parallel_incremental("replay-predicate-left-unfinished", &keep, &right_first);
    assert_prefix_verdicts(&item, PLACED, &[None, Some("pending")]);
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

// ---- FR-895 rules 19 to 21: the join step, bounded predicate joins ---------

/// A `Seller` event at `Flow::Main::Both::right::Right::R`.
fn seller_right() -> Value {
    json!({"kind": "event", "node": RIGHT_STEP, "role": "Seller", "instance": "w-1",
           "keys": {}, "record": {"ok": true}})
}

/// An open recording over `observations`, `Buyer`'s in order at `buyer` and
/// `Seller`'s at `seller`.
fn split_recording(observations: Value, buyer: &[usize], seller: &[usize]) -> Value {
    json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": observations,
        "local_order": {"Buyer": buyer, "Seller": seller},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    })
}

/// FR-895-AC-32 (TC-2531): the join step cancels, and it may fire at any
/// point after its policy holds (ADR-027 CI-3), so a branch step taken before
/// a step after the `parallel` is legitimate. Under `join any … outstanding
/// cancel`, `L`, `R`, `Placed` leaves `R` `satisfied`; `L`, `Placed`, `R`
/// leaves it `violated`, cause `unexpected`; `R` of `Seller`, unordered with
/// `Placed`, is not flagged; `join predicate(left_only)` behaves the same.
#[trace("TC-2531", "FR-895-AC-32")]
#[test]
fn a_branch_step_is_after_the_cancel_only_once_a_step_after_the_parallel_precedes_it() {
    let left = || branch_event("left", "Left", "L");
    let right = || branch_event("right", "Right", "R");
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let predicate = predicate_join_protocol("left_only", "left", "cancel");
    for (source, name) in [(cancel_join_protocol(), "any"), (predicate, "predicate")] {
        // The policy holds after `L` and no later step is observed: `R` is
        // not after the cancel, since the join step may fire later.
        let policy_only = json!([left(), right()]);
        assert_enabled(&source, &parallel_recording(policy_only), RIGHT_STEP, 1);
        let before = json!([left(), right(), placed()]);
        assert_enabled(&source, &parallel_recording(before.clone()), RIGHT_STEP, 1);
        assert_enabled(&source, &parallel_recording(before.clone()), PLACED, 2);
        let item = parallel_incremental(&format!("replay-cancel-before-{name}"), &source, &before);
        assert_prefix_verdicts(
            &item,
            RIGHT_STEP,
            &[None, Some("satisfied"), Some("satisfied")],
        );

        let after = json!([left(), placed(), right()]);
        assert_unexpected(&source, &parallel_recording(after.clone()), RIGHT_STEP, 2);
        let item = parallel_incremental(&format!("replay-cancel-after-{name}"), &source, &after);
        assert_prefix_verdicts(&item, RIGHT_STEP, &[None, None, Some("violated")]);

        // `R` by `Seller` is unordered with `Placed`: no step after the
        // `parallel` causally precedes it.
        let split_source = source.replace("event R by Buyer", "event R by Seller");
        let concurrent = json!([left(), placed(), seller_right()]);
        let recording = split_recording(concurrent, &[0, 1], &[2]);
        let package = package_of(&split_source);
        let assessed = fsl_analyze::protocol_analysis::recording::assess_recording(
            &package,
            recording.to_string().as_bytes(),
        )
        .expect("readable");
        let found = enabledness(&assessed, RIGHT_STEP, 2);
        assert_eq!(
            found.verdict,
            fsl_analyze::protocol_analysis::recording::RecordingVerdict::Satisfied,
            "{name}: {found:#?}"
        );
    }
}

/// The nested `parallel` fixture: `Inner` (`join any … outstanding cancel`
/// over `L` and `R`) then `M` in branch `a` of `Outer`, and `N` in branch
/// `b`.
fn nested_cancel_protocol() -> String {
    order_protocol(false).replace(
        "run sequence Main {",
        "run sequence Main { parallel Outer memory sc { branch a sequence A { parallel Inner memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Buyer as (r: Ack) { true }; } } join any [left, right] outstanding cancel; event M by Buyer as (m: Ack) { true }; } branch b sequence B { event N by Buyer as (n: Ack) { true }; } } join all [a, b];",
    )
}

/// A `Buyer` event at the nested fixture's `path`.
fn nested_event(path: &str) -> Value {
    json!({"kind": "event", "node": format!("Flow::Main::Outer::{path}"), "role": "Buyer",
           "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// FR-895-AC-33 (TC-2531): in a nested `parallel` the cancel is the inner
/// join's: `L`, `M`, `R` leaves `R`, after `M` of the sequence after `Inner`,
/// `violated`, cause `unexpected`; `L`, `R`, `M` leaves it `satisfied`; a step
/// of the other outer branch (`N`) is never after the inner cancel.
#[trace("TC-2531", "FR-895-AC-33")]
#[test]
fn a_nested_parallel_cancels_at_its_own_join() {
    let source = nested_cancel_protocol();
    let inner_left = || nested_event("a::A::Inner::left::Left::L");
    let inner_right = || nested_event("a::A::Inner::right::Right::R");
    let after_inner = || nested_event("a::A::M");
    let other = || nested_event("b::B::N");
    let right_node = "Flow::Main::Outer::a::A::Inner::right::Right::R";
    let open = |observations: Value| {
        let count = observations.as_array().expect("observations").len();
        json!({
            "format": "filament.protocol-recording/1", "protocol": "Flow",
            "observations": observations,
            "local_order": {"Buyer": (0..count).collect::<Vec<_>>()},
            "correlations": [],
            "closure": {"state": "open", "premises": []}
        })
    };
    assert_unexpected(
        &source,
        &open(json!([inner_left(), after_inner(), inner_right()])),
        right_node,
        2,
    );
    assert_enabled(
        &source,
        &open(json!([inner_left(), inner_right(), after_inner()])),
        right_node,
        1,
    );
    assert_enabled(
        &source,
        &open(json!([inner_left(), other(), inner_right()])),
        right_node,
        2,
    );
}

/// A protocol whose `parallel` joins `labels` branches by a predicate
/// reading the first only, each branch holding one `Buyer` event `E<i>`.
fn many_label_protocol(labels: usize) -> String {
    let branches: String = (0..labels)
        .map(|i| {
            format!(
                "branch b{i} sequence S{i} {{ event E{i} by Buyer as (v{i}: Ack) {{ true }}; }} "
            )
        })
        .collect();
    let joined: Vec<String> = (0..labels).map(|i| format!("b{i}")).collect();
    let parameters: Vec<String> = (0..labels).map(|i| format!("b{i}: Boolean")).collect();
    let source = order_protocol(false).replace(
        "run sequence Main {",
        &format!(
            "run sequence Main {{ parallel Many memory sc {{ {branches}}} join predicate(head_of) [{}] outstanding continue;",
            joined.join(", ")
        ),
    );
    format!(
        "function head_of using v({}): Boolean pure {{ b0 }}\n{source}",
        parameters.join(", ")
    )
}

/// FR-895-AC-34 (TC-2531): a predicate join names at most 10 labels; at the
/// bound the recording is assessed, and above it, at 11 and at 64 labels, the
/// assessment is refused `stage_limit_exceeded`/`work-budget-exceeded`,
/// naming the join and the bound, before any evaluation.
#[trace("TC-2531", "FR-895-AC-34")]
#[test]
fn a_predicate_join_over_too_many_labels_is_refused_naming_the_bound() {
    use fsl_analyze::protocol_analysis::recording::{assess_recording, MAX_PREDICATE_JOIN_LABELS};
    assert_eq!(MAX_PREDICATE_JOIN_LABELS, 10);
    let empty = json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": [], "local_order": {}, "correlations": [],
        "closure": {"state": "open", "premises": []}
    })
    .to_string();
    let at_bound = package_of(&many_label_protocol(10));
    assert!(assess_recording(&at_bound, empty.as_bytes()).is_ok());
    for labels in [11, 64] {
        let package = package_of(&many_label_protocol(labels));
        let refusal = assess_recording(&package, empty.as_bytes())
            .expect_err("a predicate join over too many labels is refused");
        assert!(refusal.is_limit(), "{labels}: {refusal}");
        assert_eq!(refusal.cause(), "predicate-join-label-bound");
        assert_eq!(
            refusal.code().to_string(),
            "stage_limit_exceeded/work-budget-exceeded"
        );
        let text = refusal.to_string();
        assert!(
            text.contains("Flow::Main::Many")
                && text.contains(&format!("names {labels} labels"))
                && text.contains("at most 10"),
            "{text}"
        );
    }
}

/// FR-895-AC-35 (TC-2531): a predicate that is never true leaves the join
/// unreached, so every branch's steps stay predecessors: with `left and not
/// left`, `L`, `Placed` leaves `Placed` `pending` in every open prefix and
/// `violated`, cause `unexpected`, once the recording is closed.
#[trace("TC-2531", "FR-895-AC-35")]
#[test]
fn a_predicate_that_is_never_true_leaves_the_join_unreached() {
    use fsl_analyze::protocol_analysis::recording::{assess_recording, RecordingVerdict};
    let source = predicate_join_protocol("contradiction", "left and not left", "continue");
    let observations = json!([
        branch_event("left", "Left", "L"),
        observation("send", "Placed", "Buyer", "m-1")
    ]);
    let item = parallel_incremental("replay-predicate-never", &source, &observations);
    assert_prefix_verdicts(&item, PLACED, &[None, Some("pending")]);
    let closed = parallel_recording(observations);
    assert_unexpected(&source, &closed, PLACED, 1);
    let mut open = closed;
    open["closure"] = json!({"state": "open", "premises": []});
    let assessed =
        assess_recording(&package_of(&source), open.to_string().as_bytes()).expect("readable");
    assert_eq!(
        enabledness(&assessed, PLACED, 1).verdict,
        RecordingVerdict::Pending
    );
}

/// FR-895-AC-34 (TC-2531): on the wire a predicate join over 11 labels
/// settles the item `declined`, `stage_limit_exceeded`, naming the join and the bound, whether the
/// assessment is batch or incremental.
#[trace("TC-2531", "FR-895-AC-34")]
#[test]
fn a_refused_predicate_join_settles_the_item_declined() {
    let recording = json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": [], "local_order": {}, "correlations": [],
        "closure": {"state": "open", "premises": []}
    });
    for assessment in ["batch", "incremental"] {
        let item = replay_with(
            &format!("replay-label-bound-{assessment}"),
            &many_label_protocol(11),
            &recording,
            json!({"file": "recording.json", "identity": recording_identity()}),
            assessed_item(assessment),
        )
        .item(0);
        assert_eq!(label(&item), "declined", "{assessment}: {item}");
        let text = item.to_string();
        assert!(
            text.contains("stage_limit_exceeded")
                && text.contains("predicate-join-label-bound")
                && text.contains("Flow::Main::Many"),
            "{assessment}: {item}"
        );
    }
}

/// [`repeat_protocol`] with a step `P` and then a `parallel` `join any [left,
/// right] outstanding cancel` of `L` and `R` first in the loop body; `P` runs
/// once in every iteration, so it fixes the iteration of the steps after it.
fn cancel_in_repeat_protocol() -> String {
    repeat_protocol().replace(
        "sequence Body {",
        "sequence Body { event P by Buyer as (p: Ack) { true }; parallel Both memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Buyer as (r: Ack) { true }; } } join any [left, right] outstanding cancel;",
    )
}

/// An event of the body's `parallel` at `Flow::Main::Loop::Body::Both::<path>`.
fn body_event(path: &str) -> Value {
    json!({"kind": "event", "node": format!("Flow::Main::Loop::Body::Both::{path}"),
           "role": "Buyer", "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// The enabledness verdicts of every observation of `node` in `prefix`.
fn node_verdicts_in(prefix: &Value, node: &str) -> Vec<String> {
    prefix["obligations"]
        .as_array()
        .expect("obligations")
        .iter()
        .filter(|verdict| {
            verdict["obligation"]["kind"] == "enabledness" && verdict["obligation"]["node"] == node
        })
        .map(|verdict| verdict["verdict"].as_str().expect("a verdict").to_owned())
        .collect()
}

/// An open incremental assessment of `observations` over [`cancel_in_repeat_protocol`]:
/// `Buyer`'s observations at `buyer` in order, `Seller`'s at `seller`, and
/// the correlations `(send, receive)`.
fn cancel_in_repeat_item(
    name: &str,
    observations: Value,
    buyer: &[usize],
    seller: &[usize],
    correlations: &[(usize, usize)],
) -> Value {
    let mut recording = repeat_recording(
        observations,
        json!(seller),
        json!(buyer),
        Value::Array(
            correlations
                .iter()
                .map(|(send, receive)| json!({"send": send, "receive": receive}))
                .collect(),
        ),
    );
    recording["closure"] = json!({"state": "open", "premises": []});
    replay_with(
        name,
        &cancel_in_repeat_protocol(),
        &recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        assessed_item("incremental"),
    )
    .item(0)
}

const BODY_LEFT: &str = "Flow::Main::Loop::Body::Both::left::Left::L";
const BODY_RIGHT: &str = "Flow::Main::Loop::Body::Both::right::Right::R";

/// FR-895-AC-37 (TC-2531): a `parallel` with an `outstanding cancel` join in
/// a `repeat` body cancels per iteration: the first iteration's `Placed`
/// does not put the second iteration's `L` and `R` after a cancel. The
/// recording `P`, `L`, `R`, `Placed`, `Got`, `yes` twice (`P` runs once per
/// iteration, so it fixes the iteration) leaves every `L` and `R` `satisfied`
/// in batch and in every prefix; with the second `R` after the second
/// `Placed` it is `violated`, cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-37")]
#[test]
fn a_cancel_in_a_repeat_body_cancels_per_iteration() {
    let left = || body_event("left::Left::L");
    let right = || body_event("right::Right::R");
    let lead = || loop_event("P");
    let observations = json!([
        lead(),
        left(),
        right(),
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        lead(),
        left(),
        right(),
        looped("send", "Placed", "Buyer", "m-2"),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
    ]);
    let item = cancel_in_repeat_item(
        "replay-cancel-repeat-legit",
        observations,
        &[0, 1, 2, 3, 6, 7, 8, 9],
        &[4, 5, 10, 11],
        &[(3, 4), (9, 10)],
    );
    let prefixes = item["prefixes"].as_array().expect("prefixes");
    assert_eq!(prefixes.len(), 12, "{item}");
    for prefix in prefixes {
        for node in [BODY_LEFT, BODY_RIGHT] {
            assert!(
                node_verdicts_in(prefix, node)
                    .iter()
                    .all(|verdict| verdict == "satisfied"),
                "{node}: {prefix}"
            );
        }
    }
    assert_eq!(
        node_verdicts_in(&prefixes[11], BODY_RIGHT),
        ["satisfied"; 2]
    );
    let order: Vec<usize> = (0..12).collect();
    assert_every_verdict_only_advances(prefixes, &order);
    assert_eq!(prefixes[11]["obligations"], item["obligations"], "{item}");

    // The second `R` after the second `Placed`: the second iteration's join
    // has fired, and the first `R`, `L` stay satisfied.
    let after_own_placed = json!([
        lead(),
        left(),
        right(),
        looped("send", "Placed", "Buyer", "m-1"),
        looped("receive", "Got", "Seller", "m-1"),
        looped_case("yes"),
        lead(),
        left(),
        looped("send", "Placed", "Buyer", "m-2"),
        right(),
        looped("receive", "Got", "Seller", "m-2"),
        looped_case("yes"),
    ]);
    let item = cancel_in_repeat_item(
        "replay-cancel-repeat-violated",
        after_own_placed,
        &[0, 1, 2, 3, 6, 7, 8, 9],
        &[4, 5, 10, 11],
        &[(3, 4), (8, 10)],
    );
    let last = &item["prefixes"][11];
    assert_eq!(
        node_verdicts_in(last, BODY_LEFT),
        ["satisfied"; 2],
        "{item}"
    );
    assert_eq!(
        node_verdicts_in(last, BODY_RIGHT),
        ["satisfied", "violated"],
        "{item}"
    );
}

/// `Flow` whose `parallel` `join any [left, right] outstanding cancel` holds
/// a `repeat` of `L` in branch `left`, then `Placed` after it.
fn repeat_in_cancel_protocol() -> String {
    order_protocol(false).replace(
        "run sequence Main {",
        "run sequence Main { parallel Both memory sc { branch left sequence Left { repeat Loop by Buyer visible () max 2 while { true } sequence Body { event L by Buyer as (l: Ack) { true }; } exhausted sequence Out { } } branch right sequence Right { event R by Buyer as (r: Ack) { true }; } } join any [left, right] outstanding cancel;",
    )
}

/// FR-895-AC-38 (TC-2531): a `repeat` below the cancelling `parallel` does
/// not change the cancel: the steps after the `parallel` are matched whatever
/// the iteration of a repeat inside its branches. `L`, `L`, `Placed` leaves
/// both `L` `satisfied`; `L`, `Placed`, `L` leaves the second `L` `violated`,
/// cause `unexpected`.
#[trace("TC-2531", "FR-895-AC-38")]
#[test]
fn a_repeat_inside_a_cancelling_branch_keeps_the_cancel_after_the_parallel() {
    let source = repeat_in_cancel_protocol();
    let node = "Flow::Main::Both::left::Left::Loop::Body::L";
    let step = || {
        json!({"kind": "event", "node": node, "role": "Buyer", "instance": "w-1",
               "keys": {}, "record": {"ok": true}})
    };
    let placed = || observation("send", "Placed", "Buyer", "m-1");
    let open = |observations: Value| {
        let count = observations.as_array().expect("observations").len();
        json!({
            "format": "filament.protocol-recording/1", "protocol": "Flow",
            "observations": observations,
            "local_order": {"Buyer": (0..count).collect::<Vec<_>>()},
            "correlations": [],
            "closure": {"state": "open", "premises": []}
        })
    };
    assert_enabled(&source, &open(json!([step(), step(), placed()])), node, 1);
    assert_unexpected(&source, &open(json!([step(), placed(), step()])), node, 2);
}

/// [`repeat_protocol`] whose body begins with `lead` (an event `P`, or
/// nothing) and an inner `repeat` of the cancelling `parallel` of `L` and
/// `R`; `Placed`, `Got` and the `Decide` choice follow the inner `repeat`.
fn nested_repeat_cancel_protocol(lead: &str) -> String {
    repeat_protocol().replace(
        "sequence Body {",
        &format!("sequence Body {{ {lead}repeat Inner by Buyer visible () max 2 while {{ true }} sequence IBody {{ parallel Both memory sc {{ branch left sequence Left {{ event L by Buyer as (l: Ack) {{ true }}; }} branch right sequence Right {{ event R by Buyer as (r: Ack) {{ true }}; }} }} join any [left, right] outstanding cancel; }} exhausted sequence IOut {{ }}"),
    )
}

const NESTED_LEFT: &str = "Flow::Main::Loop::Body::Inner::IBody::Both::left::Left::L";
const NESTED_RIGHT: &str = "Flow::Main::Loop::Body::Inner::IBody::Both::right::Right::R";

/// A `Buyer` event at `Flow::Main::Loop::Body::<path>`.
fn loop_event(path: &str) -> Value {
    json!({"kind": "event", "node": format!("Flow::Main::Loop::Body::{path}"),
           "role": "Buyer", "instance": "w-1", "keys": {}, "record": {"ok": true}})
}

/// The open incremental item of `observations` over `source`.
fn nested_cancel_item(
    name: &str,
    source: &str,
    observations: Value,
    buyer: &[usize],
    seller: &[usize],
    correlations: &[(usize, usize)],
) -> Value {
    let mut recording = repeat_recording(
        observations,
        json!(seller),
        json!(buyer),
        Value::Array(
            correlations
                .iter()
                .map(|(send, receive)| json!({"send": send, "receive": receive}))
                .collect(),
        ),
    );
    recording["closure"] = json!({"state": "open", "premises": []});
    let run = replay_with(
        name,
        source,
        &recording,
        json!({"file": "recording.json", "identity": recording_identity()}),
        assessed_item("incremental"),
    )
    ;
    eprintln!("DBG {}", run.document());
    run.item(0)
}

/// FR-895-AC-39 (TC-2531): a cancelling `parallel` in an inner `repeat` in an
/// outer `repeat` cancels per iteration of each. With `Placed`, `Got` and a
/// `choice` after the inner `repeat` in the outer body, `L`, `R`, `L`, `R`
/// twice per outer iteration leaves every `L` and `R` `satisfied` in batch and
/// every prefix, with or without a step `P` before the inner `repeat` (which
/// fixes the outer iteration of an inner step); with `P`, the last `R` of the
/// second outer iteration after that iteration's `Placed` is `violated`,
/// cause `unexpected`, while the first iteration's `Placed` does not make
/// the second iteration's steps so.
#[trace("TC-2531", "FR-895-AC-39")]
#[test]
fn a_cancel_in_nested_repeats_cancels_per_iteration_of_each() {
    let step = |path: &str| loop_event(path);
    let left = || step("Inner::IBody::Both::left::Left::L");
    let right = || step("Inner::IBody::Both::right::Right::R");
    let placed = |message: &str| looped("send", "Placed", "Buyer", message);
    let got = |message: &str| looped("receive", "Got", "Seller", message);
    let assert_all_satisfied = |item: &Value, count: usize| {
        let prefixes = item["prefixes"].as_array().expect("prefixes");
        assert_eq!(prefixes.len(), count, "{item}");
        for prefix in prefixes {
            for node in [NESTED_LEFT, NESTED_RIGHT] {
                assert!(
                    node_verdicts_in(prefix, node)
                        .iter()
                        .all(|verdict| verdict == "satisfied"),
                    "{node}: {prefix}"
                );
            }
        }
        let order: Vec<usize> = (0..count).collect();
        assert_every_verdict_only_advances(prefixes, &order);
        assert_eq!(
            prefixes[count - 1]["obligations"],
            item["obligations"],
            "{item}"
        );
    };

    // With `P`: the outer iteration of an inner step is the number of `P`
    // observed before it.
    let with_p = nested_repeat_cancel_protocol("event P by Buyer as (p: Ack) { true }; ");
    let legit = json!([
        step("P"),
        left(),
        right(),
        left(),
        right(),
        placed("m-1"),
        got("m-1"),
        looped_case("yes"),
        step("P"),
        left(),
        right(),
        left(),
        right(),
        placed("m-2"),
        got("m-2"),
        looped_case("yes"),
    ]);
    let item = nested_cancel_item(
        "replay-nested-cancel-legit",
        &with_p,
        legit,
        &[0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13],
        &[6, 7, 14, 15],
        &[(5, 6), (13, 14)],
    );
    assert_all_satisfied(&item, 16);
    let last = &item["prefixes"][15];
    assert_eq!(
        node_verdicts_in(last, NESTED_RIGHT),
        ["satisfied"; 4],
        "{item}"
    );

    // The last `R` of the second outer iteration after its own `Placed`.
    let violated = json!([
        step("P"),
        left(),
        right(),
        left(),
        right(),
        placed("m-1"),
        got("m-1"),
        looped_case("yes"),
        step("P"),
        left(),
        right(),
        left(),
        placed("m-2"),
        right(),
        got("m-2"),
        looped_case("yes"),
    ]);
    let item = nested_cancel_item(
        "replay-nested-cancel-violated",
        &with_p,
        violated,
        &[0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13],
        &[6, 7, 14, 15],
        &[(5, 6), (12, 14)],
    );
    let last = &item["prefixes"][15];
    assert_eq!(
        node_verdicts_in(last, NESTED_LEFT),
        ["satisfied"; 4],
        "{item}"
    );
    assert_eq!(
        node_verdicts_in(last, NESTED_RIGHT),
        ["satisfied", "satisfied", "satisfied", "violated"],
        "{item}"
    );

    // Without `P` the outer iteration of an inner step is not determined, so
    // the first iteration's `Placed` does not cancel the second iteration.
    let without_p = nested_repeat_cancel_protocol("");
    let legit = json!([
        left(),
        right(),
        left(),
        right(),
        placed("m-1"),
        got("m-1"),
        looped_case("yes"),
        left(),
        right(),
        left(),
        right(),
        placed("m-2"),
        got("m-2"),
        looped_case("yes"),
    ]);
    let item = nested_cancel_item(
        "replay-nested-cancel-probe-n",
        &without_p,
        legit,
        &[0, 1, 2, 3, 4, 7, 8, 9, 10, 11],
        &[5, 6, 12, 13],
        &[(4, 5), (11, 12)],
    );
    assert_all_satisfied(&item, 14);
}

/// Every `nodes` verdict of the `count` prefixes of `item` is `satisfied`,
/// verdicts only advance, and the last prefix is the batch assessment.
fn assert_nodes_satisfied(item: &Value, nodes: &[&str], count: usize) {
    let prefixes = item["prefixes"]
        .as_array()
        .unwrap_or_else(|| panic!("prefixes: {item}"));
    assert_eq!(prefixes.len(), count, "{item}");
    for prefix in prefixes {
        for node in nodes {
            assert!(
                node_verdicts_in(prefix, node)
                    .iter()
                    .all(|verdict| verdict == "satisfied"),
                "{node}: {:?}",
                prefixes
                    .iter()
                    .map(|p| node_verdicts_in(p, node))
                    .collect::<Vec<_>>()
            );
        }
    }
    let order: Vec<usize> = (0..count).collect();
    assert_every_verdict_only_advances(prefixes, &order);
    assert_eq!(
        prefixes[count - 1]["obligations"],
        item["obligations"],
        "{item}"
    );
}

/// FR-895-AC-40 (TC-2531): an iteration is taken only from a step that runs
/// once in every iteration. When a cancelled branch skips an iteration's
/// `R`, `P`, `L`, `Placed`, `Got`, `yes`, `P`, `L`, `R`, `Placed`, `Got`,
/// `yes` leaves the second iteration's `R` `satisfied` in batch and in every
/// prefix, as it does under `outstanding continue`.
#[trace("TC-2531", "FR-895-AC-40")]
#[test]
fn a_skipped_branch_step_does_not_shift_the_iteration() {
    let observations = || {
        json!([
            loop_event("P"),
            body_event("left::Left::L"),
            looped("send", "Placed", "Buyer", "m-1"),
            looped("receive", "Got", "Seller", "m-1"),
            looped_case("yes"),
            loop_event("P"),
            body_event("left::Left::L"),
            body_event("right::Right::R"),
            looped("send", "Placed", "Buyer", "m-2"),
            looped("receive", "Got", "Seller", "m-2"),
            looped_case("yes"),
        ])
    };
    for outstanding in ["cancel", "continue"] {
        let source = cancel_in_repeat_protocol()
            .replace("outstanding cancel", &format!("outstanding {outstanding}"));
        let item = nested_cancel_item(
            &format!("replay-skipped-branch-{outstanding}"),
            &source,
            observations(),
            &[0, 1, 2, 5, 6, 7, 8],
            &[3, 4, 9, 10],
            &[(2, 3), (8, 9)],
        );
        assert_nodes_satisfied(&item, &[BODY_LEFT, BODY_RIGHT], 11);
        assert_eq!(
            node_verdicts_in(&item["prefixes"][10], BODY_RIGHT),
            ["satisfied"],
            "{outstanding}: {item}"
        );
    }
}

/// [`nested_repeat_cancel_protocol`] with `P` and a second `repeat` of a step
/// `X` after the inner `repeat` that holds the cancelling `parallel`.
fn second_repeat_cancel_protocol() -> String {
    nested_repeat_cancel_protocol("event P by Buyer as (p: Ack) { true }; ").replace(
        "exhausted sequence IOut { }",
        "exhausted sequence IOut { } repeat Second by Buyer visible () max 2 while { true } sequence SBody { event X by Buyer as (x: Ack) { true }; } exhausted sequence SOut { }",
    )
}

/// FR-895-AC-41 (TC-2531): steps inside a `repeat` do not count toward an
/// iteration. With `P`, an inner `repeat` of the cancelling `parallel` and
/// another inner `repeat` of `X` in the body, `P`, `L`, `R`, `X`, `Placed`,
/// `Got`, `yes` twice leaves every `L` and `R` `satisfied` in batch and in
/// every prefix, as under `outstanding continue`.
#[trace("TC-2531", "FR-895-AC-41")]
#[test]
fn a_step_inside_an_inner_repeat_does_not_count_toward_the_iteration() {
    let observations = || {
        let round = |message: &str| {
            [
                loop_event("P"),
                loop_event("Inner::IBody::Both::left::Left::L"),
                loop_event("Inner::IBody::Both::right::Right::R"),
                loop_event("Second::SBody::X"),
                looped("send", "Placed", "Buyer", message),
                looped("receive", "Got", "Seller", message),
                looped_case("yes"),
            ]
        };
        let mut all = Vec::new();
        all.extend(round("m-1"));
        all.extend(round("m-2"));
        Value::Array(all)
    };
    for outstanding in ["cancel", "continue"] {
        let source = second_repeat_cancel_protocol()
            .replace("outstanding cancel", &format!("outstanding {outstanding}"));
        let item = nested_cancel_item(
            &format!("replay-inner-repeat-step-{outstanding}"),
            &source,
            observations(),
            &[0, 1, 2, 3, 4, 7, 8, 9, 10, 11],
            &[5, 6, 12, 13],
            &[(4, 5), (11, 12)],
        );
        assert_nodes_satisfied(&item, &[NESTED_LEFT, NESTED_RIGHT], 14);
    }
}

/// FR-895-AC-42 (TC-2531): the cases of a `choice` do not count toward an
/// iteration. With a `choice` before the inner `repeat` of the cancelling
/// `parallel`, taking `a` and then `b`, `a`, `L`, `R`, `Placed`, `Got`, `yes`,
/// `b`, `L`, `R`, `Placed`, `Got`, `yes` leaves every `L` and `R` `satisfied`
/// in batch and in every prefix, as under `outstanding continue`.
#[trace("TC-2531", "FR-895-AC-42")]
#[test]
fn a_choice_before_an_inner_repeat_does_not_count_toward_the_iteration() {
    let pick = |case: &str| {
        json!({"kind": "case", "node": format!("Flow::Main::Loop::Body::Pick::{case}"),
               "role": "Buyer", "instance": "w-1", "keys": {}})
    };
    let observations = || {
        let round = |case: &str, message: &str| {
            [
                pick(case),
                loop_event("Inner::IBody::Both::left::Left::L"),
                loop_event("Inner::IBody::Both::right::Right::R"),
                looped("send", "Placed", "Buyer", message),
                looped("receive", "Got", "Seller", message),
                looped_case("yes"),
            ]
        };
        let mut all = Vec::new();
        all.extend(round("a", "m-1"));
        all.extend(round("b", "m-2"));
        Value::Array(all)
    };
    for outstanding in ["continue"] {
        let source = nested_repeat_cancel_protocol(
            "choice Pick by Buyer visible () { case a when { true } sequence CA { } case b when { false } sequence CB { } } ",
        )
        .replace("outstanding cancel", &format!("outstanding {outstanding}"));
        let item = nested_cancel_item(
            &format!("replay-choice-before-repeat-{outstanding}"),
            &source,
            observations(),
            &[0, 1, 2, 3, 6, 7, 8, 9],
            &[4, 5, 10, 11],
            &[(3, 4), (9, 10)],
        );
        assert_nodes_satisfied(&item, &[NESTED_LEFT, NESTED_RIGHT], 12);
    }
}
