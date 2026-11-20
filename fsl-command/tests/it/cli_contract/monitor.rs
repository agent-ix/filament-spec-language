// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2473: `fsl monitor <request>` at the command surface the driver calls
//! (`command::monitor` over the `--input` file or stdin, split into lines as
//! the driver splits them): a decided and a pending stream, an empty
//! stream, a stream cut mid-record, a malformed record, whole-request
//! refusals, cancellation, and odd input. Every event and summary line is
//! checked against its FR-644 schema.

use filament_exact::{Cancel, CancelCause};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::{monitor, monitor_under, Case, INVALID_UTF8};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile t = \"filament.temporal.infinite-trace/v1\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    record Load { bins: Int[0, 15]; }\n\
    temporal DesiredSeen using t over (s: Load) clock \"steps\" on origin {\n\
        eventually holds(s.bins = 9)\n\
    }\n\
    temporal StaysLow using t over (s: Load) clock \"steps\" on origin {\n\
        always holds(s.bins <= 6)\n\
    }\n";

fn request() -> Value {
    json!({"format": "native-monitor/1", "request": {
        "models": [], "program": {"source": {"file": "load.fsl", "authority": "monitor-contracts",
            "identity": "load", "document": "LoadMonitor", "formal_revision": 1}},
        "clauses": [{"name": "StaysLow"}, {"name": "DesiredSeen"}], "trace": {"kind": "finite"}}})
}

fn case_with(source: &[u8], request: &Value) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("load.fsl", source);
    let path = case.json("monitor.json", request);
    (case, path)
}

fn load() -> (Case, std::path::PathBuf) {
    case_with(SOURCE.as_bytes(), &request())
}

fn records(values: &[u8], end: bool) -> Vec<u8> {
    let mut lines: Vec<String> = values
        .iter()
        .map(|value| {
            json!({"observation": {"sample": {"bins": {"integer": value.to_string()}}}}).to_string()
        })
        .collect();
    if end {
        lines.push(json!({"end": true}).to_string());
    }
    lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        .into_bytes()
}

fn changes(events: &[Value]) -> Vec<(u64, String, String)> {
    events
        .iter()
        .map(|event| {
            (
                event["record"].as_u64().unwrap_or(u64::MAX),
                event["clause"].as_str().unwrap_or("?").to_owned(),
                event["category"].as_str().unwrap_or("?").to_owned(),
            )
        })
        .collect()
}

/// FR-644-AC-1, AC-7: one event per status change, a decisive violation at
/// the record that settles it, then one summary; exit 10.
#[test]
#[trace("FR-644-AC-1", "FR-644-AC-7", "FR-644-AC-12")]
fn a_decided_stream_writes_status_changes_and_exits_10() {
    let (_case, path) = load();
    let stream = monitor(&path, &records(&[2, 4, 9, 3], true));
    let (events, summary) = stream.completed(10);
    let expected: Vec<(u64, String, String)> = [
        (0, "StaysLow", "pending"),
        (0, "DesiredSeen", "pending"),
        (2, "StaysLow", "violation"),
        (2, "DesiredSeen", "success"),
    ]
    .into_iter()
    .map(|(record, clause, category)| (record, clause.to_owned(), category.to_owned()))
    .collect();
    assert_eq!(changes(events), expected);
    assert_eq!(summary["records"], 5, "{summary}");
    assert_eq!(summary["refused"], 0, "{summary}");
}

/// FR-285-AC-9, FR-286-AC-13: a good finite prefix stays `pending`, exit 0;
/// an empty stream and a stream holding only `{"end": true}` are pending
/// summaries, not refusals.
#[test]
#[trace("FR-285-AC-9", "FR-286-AC-13")]
fn pending_and_empty_streams_exit_0() {
    let (_case, path) = load();
    for (input, count) in [
        (records(&[2, 4, 3], true), 4),
        (records(&[2, 4, 3], false), 3),
        (records(&[], true), 1),
        (Vec::new(), 0),
    ] {
        let stream = monitor(&path, &input);
        let (_, summary) = stream.completed(0);
        assert_eq!(summary["records"], count, "{summary}");
    }
}

/// FR-644-AC-2: a malformed record and a record cut mid-line each give one
/// `refusal` event naming the record, and the stream goes on.
#[test]
#[trace("FR-644-AC-2")]
fn malformed_and_truncated_records_are_refusal_events() {
    let (_case, path) = load();
    let mut malformed = records(&[2], false);
    malformed.extend_from_slice(
        b"{\"observation\": {\"sample\": {\"bins\": {\"integer\": \"4\"}}}, \"extra\": 1}\n",
    );
    malformed.extend_from_slice(&records(&[9, 3], true));
    let stream = monitor(&path, &malformed);
    let (events, summary) = stream.completed(10);
    assert_eq!(summary["refused"], 1, "{summary}");
    assert!(
        events
            .iter()
            .any(|event| event["record"] == 1 && event["category"] == "refusal"),
        "{events:?}"
    );

    let mut truncated = records(&[2, 4], false);
    truncated.extend_from_slice(b"{\"observation\": {\"sam");
    let stream = monitor(&path, &truncated);
    let (events, summary) = stream.completed(0);
    assert_eq!(summary["refused"], 1, "{summary}");
    assert!(
        events
            .iter()
            .any(|event| event["record"] == 2 && event["category"] == "refusal"),
        "{events:?}"
    );
}

/// `--input` against stdin (driver FR-025 Inputs): the same records give
/// byte-identical lines whichever stream carries them; the harness hands
/// both as the same byte stream, so this checks the run is a function of
/// the records alone.
#[test]
#[trace("FR-644", "FR-286-AC-26")]
fn the_same_records_give_the_same_bytes() {
    let (_case, path) = load();
    let input = records(&[2, 4, 9, 3], true);
    let first = monitor(&path, &input);
    let second = monitor(&path, &input);
    assert_eq!(first.bytes(), second.bytes());
}

/// FR-644-AC-4: a clause the program lacks, an unknown request member and a
/// run request handed to monitor each refuse before any record is read,
/// exit 20, with no event.
#[test]
#[trace("FR-644-AC-4")]
fn whole_request_defects_refuse_before_the_stream() {
    let mut absent = request();
    absent["request"]["clauses"] = json!([{"name": "Absent"}]);
    let mut unknown = request();
    unknown["request"]["extra"] = json!(true);
    let mut wrong = request();
    wrong["format"] = json!("native-run/1");
    for document in [absent, unknown, wrong] {
        let (_case, path) = case_with(SOURCE.as_bytes(), &document);
        let stream = monitor(&path, &records(&[2], true));
        stream.refused().expect("monitor", "refusal", 20);
    }
}

/// FR-624 (SIGINT/SIGTERM end the stream as a cancelled run): a handle
/// cancelled before the call gives exit 22.
#[test]
#[trace("FR-624")]
fn a_cancelled_stream_exits_22() {
    let (_case, path) = load();
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let stream = monitor_under(&path, &records(&[2, 4], false), &cancel);
    let exit = match &stream {
        super::Stream::Lines { exit, .. } => *exit,
        super::Stream::Refused(outcome) => outcome.exit,
    };
    assert_eq!(exit, 22, "{stream:?}");
}

/// FR-001: invalid UTF-8 in the program, and a missing, a directory and an
/// empty request operand, each refuse before the stream, exit 20.
#[test]
#[trace("FR-001-AC-3", "FR-001")]
fn odd_input_refuses_before_the_stream() {
    let (case, path) = case_with(INVALID_UTF8, &request());
    monitor(&path, &records(&[2], true))
        .refused()
        .expect("monitor", "refusal", 20);
    std::fs::create_dir_all(case.path("folder")).unwrap();
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        case.write("empty.json", b""),
    ] {
        let stream = monitor(&path, &records(&[2], true));
        let outcome = stream.refused();
        outcome.expect("monitor", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}",
            outcome.document
        );
    }
}
