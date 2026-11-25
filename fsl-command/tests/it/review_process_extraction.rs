// SPDX-License-Identifier: AGPL-3.0-or-later
//! A synthetic review trace runs from an identity-bound Markdown native fence.

use crate::cli_contract::{analyze, Case, Outcome};
use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const MODEL: &[u8] = include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json");
const PROTOCOL_REQUEST: &str =
    include_str!("../../../examples/protocol-recommendation/analyze.json");
const PROTOCOL_SNAPSHOT: &[u8] =
    include_bytes!("../../../examples/protocol-recommendation/start.json");
const PROFILE: &str = "filament.temporal.timestamped-event.finite-window/v1";

fn native_source() -> String {
    r#"language "ix:native" edition "1-draft";
profile p = "filament.protocol.finite-global/v1";
profile t = "filament.temporal.timestamped-event.finite-window/v1";
model M = "test/counter";

protocol Review using p over (view: M::Counter) on origin {
  observe role Requester on M::Counter;
  role Reviewer on M::Counter;
  channel Receipt from Requester to Reviewer carries Boolean
    ordering unordered delivery unknown capacity 1 overflow reject;
  channel Acknowledgement from Reviewer to Requester carries Boolean
    ordering unordered delivery unknown capacity 1 overflow reject;
  run sequence Main {
    send ReceiptSent via Receipt as (receipt: Boolean) { true };
    receive ReceiptReceived via Receipt of ReceiptSent as (receipt: Boolean) { true };
    send AcknowledgementSent via Acknowledgement as (accepted: Boolean) { true };
    receive AcknowledgementReceived via Acknowledgement of AcknowledgementSent
      as (acknowledged: Boolean) { true };
  }
  finish Closed as (closed: Boolean) { true };
}

record ReviewEvent {
  process_identity: Int;
  repository_identity: Int;
  case_identity: Int;
  source_authority_identity: Int;
  active_source_revision_identity: Int;
  reviewed_source_revision_identity: Int;
  attempt_identity: Int;
  event_identity: Int;
  github_event_identity: Int;
  linear_event_identity: Int;
  relay_event_identity: Int;
  gate_event_identity: Int;
  author_identity: Int;
  reviewer_identity: Int;
  assignment_identity: Int;
  acknowledged: Boolean;
  accepted: Boolean;
  reassigned: Boolean;
  assignment_accepted: Boolean;
}

temporal ReviewAccepted using t over (event: ReviewEvent) clock "review-ms" on origin {
  eventually[0,5] holds(
    event.acknowledged and event.accepted and
    event.process_identity = 11 and event.repository_identity = 12 and
    event.case_identity = 13 and event.source_authority_identity = 17 and
    event.attempt_identity = 15 and event.event_identity = 18 and
    event.github_event_identity = 21 and event.linear_event_identity = 22 and
    event.relay_event_identity = 23 and event.gate_event_identity = 24 and
    event.assignment_identity = 16 and
    event.reviewer_identity != event.author_identity and
    event.reviewed_source_revision_identity = event.active_source_revision_identity and
    (not event.reassigned or event.assignment_accepted)
  )
}
"#
    .to_owned()
}

fn event(variant: EventVariant) -> Value {
    let (
        author,
        reviewer,
        reviewed_revision,
        source_authority,
        event_identity,
        github_event,
        reassigned,
        assignment_accepted,
        acknowledged,
    ) = match variant {
        EventVariant::Qualified => (1, 2, 14, 17, 18, 21, false, false, true),
        EventVariant::Receipt => (1, 2, 14, 17, 18, 21, false, false, false),
        EventVariant::SelfReview => (1, 1, 14, 17, 18, 21, false, false, true),
        EventVariant::StaleRevision => (1, 2, 19, 17, 18, 21, false, false, true),
        EventVariant::UnacceptedReassignment => (1, 2, 14, 17, 18, 21, true, false, true),
        EventVariant::WrongSourceEvent => (1, 2, 14, 17, 18, 99, false, false, true),
        EventVariant::WrongSourceAuthority => (1, 2, 14, 99, 18, 21, false, false, true),
    };
    json!({
        "position": {"sample": {
            "process_identity": {"integer": "11"},
            "repository_identity": {"integer": "12"},
            "case_identity": {"integer": "13"},
            "source_authority_identity": {"integer": source_authority.to_string()},
            "active_source_revision_identity": {"integer": "14"},
            "reviewed_source_revision_identity": {"integer": reviewed_revision.to_string()},
            "attempt_identity": {"integer": "15"},
            "event_identity": {"integer": event_identity.to_string()},
            "github_event_identity": {"integer": github_event.to_string()},
            "linear_event_identity": {"integer": "22"},
            "relay_event_identity": {"integer": "23"},
            "gate_event_identity": {"integer": "24"},
            "author_identity": {"integer": author.to_string()},
            "reviewer_identity": {"integer": reviewer.to_string()},
            "assignment_identity": {"integer": "16"},
            "acknowledged": {"boolean": acknowledged},
            "accepted": {"boolean": true},
            "reassigned": {"boolean": reassigned},
            "assignment_accepted": {"boolean": assignment_accepted}
        }}, "stamp": 3
    })
}

#[derive(Clone, Copy)]
enum EventVariant {
    Qualified,
    Receipt,
    SelfReview,
    StaleRevision,
    UnacceptedReassignment,
    WrongSourceEvent,
    WrongSourceAuthority,
}

fn request(
    directory: &Path,
    variant: EventVariant,
    stamp: u64,
    incomplete: bool,
    pending: bool,
) -> std::path::PathBuf {
    let source = native_source();
    let markdown =
        format!("# Synthetic process\n\n### review_acceptance\n```ix:native\n{source}```\n");
    std::fs::write(directory.join("process.md"), markdown).expect("Markdown source writes");
    std::fs::write(directory.join("counter.json"), MODEL).expect("model writes");

    let mut observation = event(variant);
    observation["stamp"] = json!(stamp);
    let completeness = if incomplete {
        json!("incomplete")
    } else {
        json!({"complete": {"through": if pending { 3 } else { 5 }}})
    };
    let watermark = if pending {
        3
    } else if incomplete {
        5
    } else {
        6
    };
    let scope = if pending { "open" } else { "closed" };
    let clocks = json!({"review-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {
            "file": "counter.json", "authority": "synthetic", "identity": "synthetic:counter",
            "document": "Counter", "formal_revision": 1
        }}],
        "program": {
            "source": {"file": "process.md", "authority": "synthetic",
                "identity": "synthetic:review-process", "document": "ReviewProcess", "formal_revision": 1},
            "extraction": {"body": {"authority": "synthetic",
                "identity": "synthetic:review-process#review_acceptance", "document": "ReviewAcceptance"}},
            "clauses": [{"clause": "review_acceptance", "package": "test/review-process"}]
        },
        "clocks": clocks,
        "clause": {"selection": {"temporal": {"name": "ReviewAccepted", "trace": {"timestamped": {
            "observations": [observation],
            "progress": {"watermark": watermark, "completeness": completeness,
                "scope": scope, "execution": "closed"},
            "lower_boundary": "authoritative-origin",
            "binding": {"profile": PROFILE, "clock": "review-ms", "parameters": clocks["review-ms"]},
            "missing": []
        }} }}, "observations": {}}
    }});
    assert_eq!(
        request["request"]["program"]["extraction"]["body"]["identity"],
        "synthetic:review-process#review_acceptance"
    );
    assert_eq!(
        request["request"]["models"][0]["source"]["identity"],
        "synthetic:counter"
    );
    let path = directory.join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn run(path: &Path) -> command::RunResult {
    command::run(path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("{}", error.value().expect("error document")))
}

fn result(
    directory: &Path,
    variant: EventVariant,
    stamp: u64,
    incomplete: bool,
    pending: bool,
) -> command::RunResult {
    run(&request(directory, variant, stamp, incomplete, pending))
}

fn analyze_embedded_protocol(body: &[u8]) -> Outcome {
    let case = Case::new();
    case.write("review.native", body);
    case.write("counter.json", MODEL);
    let mut snapshot: Value = serde_json::from_slice(PROTOCOL_SNAPSHOT).expect("snapshot fixture");
    snapshot["identity"] =
        json!({"authority": "synthetic", "identity": "synthetic:review-process-start"});
    case.json("start.json", &snapshot);

    let mut request: Value =
        serde_json::from_str(PROTOCOL_REQUEST).expect("protocol request fixture");
    request["request"]["program"]["source"] = json!({
        "file": "review.native", "authority": "synthetic",
        "identity": "synthetic:review-process#review_acceptance",
        "document": "ReviewAcceptance", "formal_revision": 1
    });
    request["request"]["models"][0]["source"] = json!({
        "file": "counter.json", "authority": "synthetic", "identity": "synthetic:counter",
        "document": "Counter", "formal_revision": 1
    });
    request["request"]["documents"][0]["file"] = json!("start.json");
    request["request"]["subject"]["initial_states"] =
        json!([{"authority": "synthetic", "identity": "synthetic:review-process-start"}]);
    request["request"]["subject"]["universes"] =
        json!([{"authority": "synthetic", "identity": "synthetic:review-process-start"}]);
    request["request"]["subject"]["protocol"]["roles"] = json!({
        "Requester": {"type": "model-binding", "selection": "test/counter"},
        "Reviewer": {"type": "model-binding", "selection": "test/counter"}
    });
    assert_eq!(
        request["request"]["program"]["source"]["identity"],
        "synthetic:review-process#review_acceptance"
    );
    assert_eq!(
        request["request"]["models"][0]["source"]["identity"],
        "synthetic:counter"
    );
    analyze(&case.json("analyze.json", &request))
}

#[test]
#[trace("FR-1050-AC-1")]
fn an_embedded_review_trace_keeps_source_identity_and_repeats_deterministically() {
    let first_dir = tempfile::tempdir().expect("first case directory");
    let first = result(first_dir.path(), EventVariant::Qualified, 3, false, false);
    let first_doc = first.value.as_value();
    assert_eq!(first.exit_code, 0, "{first_doc}");
    assert_eq!(
        first_doc["disposition"]["category"], "success",
        "{first_doc}"
    );
    assert_eq!(first_doc["disposition"]["truth"], true, "{first_doc}");
    assert_eq!(
        first_doc["extraction"]["original"]["identity"],
        "synthetic:review-process"
    );
    assert_eq!(
        first_doc["extraction"]["body"]["identity"],
        "synthetic:review-process#review_acceptance"
    );
    assert_eq!(
        first_doc["extraction"]["binding"]["clause"],
        "review_acceptance"
    );
    let body = first_doc["extraction"]["body"]["bytes"]
        .as_array()
        .expect("body bytes are retained")
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().expect("byte is an integer")).expect("u8"))
        .collect::<Vec<_>>();
    let body = String::from_utf8(body).expect("body is UTF-8");
    assert!(
        body.contains("protocol Review"),
        "the selected body declares its protocol"
    );
    assert!(
        body.contains("model M = \"test/counter\""),
        "the protocol model identity is retained"
    );
    for field in [
        "process_identity",
        "repository_identity",
        "case_identity",
        "source_authority_identity",
        "active_source_revision_identity",
        "reviewed_source_revision_identity",
        "attempt_identity",
        "event_identity",
        "github_event_identity",
        "linear_event_identity",
        "relay_event_identity",
        "gate_event_identity",
        "assignment_identity",
    ] {
        assert!(body.contains(field), "{field} absent from extracted body");
    }
    let protocol = analyze_embedded_protocol(body.as_bytes());
    let protocol_document = protocol.expect("analyze", "violation", 10);
    assert_eq!(protocol_document["items"][0]["claim_name"], "Review");
    assert_eq!(protocol_document["items"][0]["replay"]["reproduced"], true);

    let second_dir = tempfile::tempdir().expect("second case directory");
    let second = result(second_dir.path(), EventVariant::Qualified, 3, false, false);
    assert_eq!(first_doc, second.value.as_value());
}

#[test]
#[trace("FR-1050-AC-2", "FR-1050-AC-3", "FR-1050-AC-5", "FR-1050-AC-6")]
fn receipt_late_self_review_stale_revision_and_unaccepted_reassignment_do_not_qualify() {
    for (variant, stamp, description) in [
        (EventVariant::Receipt, 3, "receipt is not acknowledgement"),
        (
            EventVariant::Qualified,
            6,
            "late acknowledgement misses the window",
        ),
        (
            EventVariant::SelfReview,
            3,
            "self-review is not independent",
        ),
        (
            EventVariant::StaleRevision,
            3,
            "stale source revision does not qualify",
        ),
        (
            EventVariant::UnacceptedReassignment,
            3,
            "unaccepted reassignment does not qualify",
        ),
        (
            EventVariant::WrongSourceEvent,
            3,
            "unbound source event identity does not qualify",
        ),
        (
            EventVariant::WrongSourceAuthority,
            3,
            "unbound source authority identity does not qualify",
        ),
    ] {
        let directory = tempfile::tempdir().expect("case directory");
        let outcome = result(directory.path(), variant, stamp, false, false);
        let document = outcome.value.as_value();
        assert_eq!(outcome.exit_code, 10, "{description}: {document}");
        assert_eq!(
            document["disposition"]["category"], "violation",
            "{description}: {document}"
        );
        assert_eq!(
            document["disposition"]["truth"], false,
            "{description}: {document}"
        );
    }
}

#[test]
#[trace("FR-1050-AC-4")]
fn incomplete_review_trace_has_no_completed_truth() {
    let directory = tempfile::tempdir().expect("case directory");
    let outcome = result(directory.path(), EventVariant::Receipt, 3, true, false);
    let document = outcome.value.as_value();
    assert_eq!(outcome.exit_code, 11, "{document}");
    assert_eq!(
        document["disposition"]["category"], "indeterminate",
        "{document}"
    );
    assert_eq!(document["disposition"]["truth"], Value::Null, "{document}");
    assert_eq!(
        document["temporal"]["cause"]["kind"], "incomplete-input",
        "{document}"
    );

    let pending_directory = tempfile::tempdir().expect("pending case directory");
    let pending = result(
        pending_directory.path(),
        EventVariant::Receipt,
        3,
        false,
        true,
    );
    let pending_doc = pending.value.as_value();
    assert_eq!(
        pending_doc["disposition"]["category"], "pending",
        "{pending_doc}"
    );
    assert_eq!(
        pending_doc["disposition"]["truth"],
        Value::Null,
        "{pending_doc}"
    );
    assert_eq!(pending_doc["temporal"]["deadline"], 5, "{pending_doc}");
}
