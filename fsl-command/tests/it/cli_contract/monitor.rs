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
use std::io::BufRead;

use super::{monitor, monitor_under, monitor_under_limits, Case, INVALID_UTF8};

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

    // The good records give the events and summary of the stream without the
    // bad record: record numbers after it shift down by one.
    let clean = monitor(
        &path,
        &[records(&[2], false), records(&[9, 3], true)].concat(),
    );
    let (clean_events, clean_summary) = clean.completed(10);
    let good: Vec<Value> = events
        .iter()
        .filter(|event| event["category"] != "refusal")
        .cloned()
        .map(|mut event| {
            if event["record"].as_u64() > Some(1) {
                event["record"] = json!(event["record"].as_u64().unwrap_or(0) - 1);
            }
            event
        })
        .collect();
    assert_eq!(good, clean_events, "events");
    assert_eq!(summary["clauses"], clean_summary["clauses"], "summary");

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

/// QSL-2671, FR-644 Behavior (a malformed record is refused; AC-2 states only the
/// unknown-member case, so it is not tagged): a stream whose first line is blank is one explicit
/// malformed record. It gives a `refusal` event at physical record 0, the
/// following observation is still accepted as the first position, and the
/// summary counts the refused record.
#[test]
#[trace("FR-644")]
fn a_leading_blank_line_is_one_refused_record() {
    let (_case, path) = load();
    let mut input = b"\n".to_vec();
    input.extend_from_slice(&records(&[9], true));
    let stream = monitor(&path, &input);
    let (events, summary) = stream.completed(10);
    let refusals: Vec<&Value> = events
        .iter()
        .filter(|event| event["category"] == "refusal")
        .collect();
    assert_eq!(refusals.len(), 1, "{events:?}");
    assert_eq!(refusals[0]["record"], 0, "{events:?}");
    assert_eq!(
        refusals[0]["disposition"]["cause"], "invalid-request",
        "{events:?}"
    );
    assert_eq!(refusals[0]["disposition"]["member"], "record", "{events:?}");
    let decided: Vec<(u64, String, String)> = changes(events)
        .into_iter()
        .filter(|(_, _, category)| category != "refusal")
        .collect();
    assert_eq!(
        decided,
        vec![
            (1, "StaysLow".to_owned(), "violation".to_owned()),
            (1, "DesiredSeen".to_owned(), "success".to_owned()),
        ]
    );
    assert_eq!(summary["records"], 3, "{summary}");
    assert_eq!(summary["refused"], 1, "{summary}");
}

/// QSL-2671, FR-644 Behavior: a blank or whitespace-only line between records is
/// refused too, and leaves the session as the stream without it. An empty
/// stdin stream (not an absent source) stays a legitimate empty trace.
#[test]
#[trace("FR-644")]
fn blank_and_whitespace_lines_are_refused_and_absence_is_not() {
    let (_case, path) = load();
    let mut input = records(&[2], false);
    input.extend_from_slice(b"\n   \n");
    input.extend_from_slice(&records(&[4], true));
    let stream = monitor(&path, &input);
    let (events, summary) = stream.completed(0);
    let refused: Vec<u64> = events
        .iter()
        .filter(|event| event["category"] == "refusal")
        .filter_map(|event| event["record"].as_u64())
        .collect();
    assert_eq!(refused, vec![1, 2], "{events:?}");
    assert_eq!(summary["refused"], 2, "{summary}");

    let clean = monitor(&path, &[records(&[2], false), records(&[4], true)].concat());
    let (clean_events, clean_summary) = clean.completed(0);
    let good: Vec<Value> = events
        .iter()
        .filter(|event| event["category"] != "refusal")
        .cloned()
        .map(|mut event| {
            if event["record"].as_u64() > Some(2) {
                event["record"] = json!(event["record"].as_u64().unwrap_or(0) - 2);
            }
            event
        })
        .collect();
    assert_eq!(good, clean_events, "events");
    assert_eq!(summary["clauses"], clean_summary["clauses"], "summary");

    let stream = monitor(&path, b"");
    let (events, summary) = stream.completed(0);
    assert_eq!(summary["refused"], 0, "{summary}");
    assert!(
        events.iter().all(|event| event["category"] != "refusal"),
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

/// FR-644-AC-16/17 and FR-255-AC-24: target defects normalize before records,
/// a target requires caller activation capacity, and that setting is
/// operation-unavailable when the request omits its target.
#[test]
#[trace("TC-721", "FR-644-AC-16", "FR-644-AC-17", "FR-255-AC-24")]
fn target_validation_and_activation_setting_precede_the_stream() {
    let target = json!({
        "counter": "tick",
        "unit": "step",
        "width_bits": 16,
        "uncertainty": {"kind": "rational", "numerator": "0", "denominator": "1"},
        "max_event_rate": null,
        "max_reading_gap": 1
    });
    let input = records(&[2], true);

    let mut malformed = request();
    malformed["request"]["target"] = target.clone();
    malformed["request"]["target"]["unknown"] = json!(true);
    let (_case, path) = case_with(SOURCE.as_bytes(), &malformed);
    let stream = monitor(&path, &input);
    let refusal = stream.refused();
    assert_eq!(refusal.exit, 20, "{}", refusal.document);
    assert_eq!(
        refusal.document["diagnostics"][0]["message"],
        "invalid request member /request/target: invalid MonitorTarget",
        "{}",
        refusal.document
    );
    assert!(
        refusal
            .document
            .to_string()
            .contains("invalid MonitorTarget"),
        "{}",
        refusal.document
    );
    assert_eq!(
        refusal.document["diagnostics"][0]["details"]["path"], "/request/target",
        "{}",
        refusal.document
    );

    let mut missing_setting = request();
    missing_setting["request"]["target"] = target;
    let (_case, path) = case_with(SOURCE.as_bytes(), &missing_setting);
    let stream = monitor(&path, &input);
    let refusal = stream.refused();
    assert_eq!(refusal.exit, 20, "{}", refusal.document);
    assert!(
        refusal
            .document
            .to_string()
            .contains("monitor.max_activations is required"),
        "{}",
        refusal.document
    );
    assert_eq!(
        refusal.document["diagnostics"][0]["details"]["path"],
        "/request/target"
    );

    let mut unavailable = fsl_command::command::CommandLimits::monitor(
        fsl_command::command::MonitorPreparationLimits {
            work: 16_777_216,
            storage: 268_435_456,
        },
    );
    unavailable
        .set(
            fsl_foundation::SettingName::Stage(fsl_foundation::Setting::MonitorMaxActivations),
            8,
        )
        .expect("caller can set activation capacity");
    let mut target_without_handoff = request();
    target_without_handoff["request"]["target"] = json!({
        "counter": "tick",
        "unit": "step",
        "width_bits": 16,
        "uncertainty": {"kind": "rational", "numerator": "0", "denominator": "1"},
        "max_event_rate": null,
        "max_reading_gap": 1
    });
    let (_case, path) = case_with(SOURCE.as_bytes(), &target_without_handoff);
    let stream = monitor_under_limits(&path, &input, &Cancel::new(), &unavailable);
    let refusal = stream.refused();
    assert_eq!(
        refusal.document["diagnostics"][0]["details"]["path"],
        "/request/target"
    );
    assert!(
        refusal
            .document
            .to_string()
            .contains("target-preparation handoff"),
        "{}",
        refusal.document
    );
    let (_case, path) = load();
    let stream = monitor_under_limits(&path, &input, &Cancel::new(), &unavailable);
    let refusal = stream.refused();
    assert_eq!(refusal.exit, 20, "{}", refusal.document);
    assert!(
        refusal
            .document
            .to_string()
            .contains("unavailable without target"),
        "{}",
        refusal.document
    );
}

/// FR-644-AC-17: target admission requires object syntax and a duplicate-free
/// rational before the driver handoff is invoked.
#[test]
#[trace("TC-721", "FR-644-AC-17")]
fn malformed_target_objects_refuse_before_driver_handoff() {
    let target = json!({
        "counter": "tick",
        "unit": "step",
        "width_bits": 16,
        "uncertainty": {"kind": "rational", "numerator": "0", "denominator": "1"},
        "max_event_rate": null,
        "max_reading_gap": 1
    });
    let mut request_with_target = request();
    request_with_target["request"]["target"] = target;
    let valid_request = request_with_target.to_string();
    let positional = valid_request.replace(
        r#"{"counter":"tick","max_event_rate":null,"max_reading_gap":1,"uncertainty":{"denominator":"1","kind":"rational","numerator":"0"},"unit":"step","width_bits":16}"#,
        r#"["tick","step",16,{"kind":"rational","numerator":"0","denominator":"1"},null,1]"#,
    );
    let duplicate_rate = valid_request.replace(
        r#""max_event_rate":null"#,
        r#""max_event_rate":{"kind":"rational","numerator":"0","numerator":"0","denominator":"1"}"#,
    );
    assert_ne!(
        positional, valid_request,
        "positional target fixture changed"
    );
    assert_ne!(
        duplicate_rate, valid_request,
        "duplicate rate fixture changed"
    );

    let mut caller = fsl_command::command::CommandLimits::monitor(
        fsl_command::command::MonitorPreparationLimits {
            work: 17,
            storage: 29,
        },
    );
    caller
        .set(
            fsl_foundation::SettingName::Stage(fsl_foundation::Setting::MonitorMaxActivations),
            8,
        )
        .expect("caller can set activation capacity");
    let options = fsl_analyze::provider::ProviderOptions::default();
    for malformed in [positional, duplicate_rate] {
        let case = Case::new();
        case.write("load.fsl", SOURCE.as_bytes());
        let path = case.write("monitor.json", malformed.as_bytes());
        let prepared = std::cell::Cell::new(false);
        let mut preparer = |_: &fsl_package::CheckedPackage,
                            _: &fsl_replay::MonitorTarget,
                            _: &fsl_command::command::CommandLimits| {
            prepared.set(true);
            Ok(())
        };
        let error = fsl_command::command::monitor_with_target_preparer(
            &path,
            std::io::Cursor::new(records(&[2], true)).split(b'\n'),
            &mut |_| {},
            fsl_command::command::MonitorProviders {
                negotiate: &crate::negotiation::Fr290,
                providers: Vec::new(),
                options: &options,
            },
            fsl_command::command::RunLimits::default(),
            &caller,
            &Cancel::new(),
            &mut preparer,
        )
        .expect_err("malformed target must be refused");
        assert_eq!(
            error.to_string(),
            "invalid request member /request/target: invalid MonitorTarget"
        );
        assert!(!prepared.get(), "malformed target must not reach driver");
    }
}

#[trace("TC-721", "FR-644-AC-16", "FR-644-AC-17", "FR-255-AC-24")]
#[test]
fn accepted_target_and_activation_bound_reach_driver_preparer() {
    let mut input = request();
    input["request"]["target"] = json!({
        "counter": "tick",
        "unit": "step",
        "width_bits": 16,
        "uncertainty": {"kind": "rational", "numerator": "0", "denominator": "1"},
        "max_event_rate": null,
        "max_reading_gap": 1
    });
    let (_case, path) = case_with(SOURCE.as_bytes(), &input);
    let mut caller = fsl_command::command::CommandLimits::monitor(
        fsl_command::command::MonitorPreparationLimits {
            work: 17,
            storage: 29,
        },
    );
    caller
        .set(
            fsl_foundation::SettingName::Stage(fsl_foundation::Setting::MonitorMaxActivations),
            8,
        )
        .expect("caller can set activation capacity");
    let prepared = std::cell::RefCell::new(None);
    let mut preparer = |_: &fsl_package::CheckedPackage,
                        target: &fsl_replay::MonitorTarget,
                        caller: &fsl_command::command::CommandLimits| {
        let bounds = caller.preparation();
        *prepared.borrow_mut() = Some((
            target.counter.clone(),
            target.max_activations,
            bounds.work,
            bounds.storage,
        ));
        Ok(())
    };
    let mut events = Vec::new();
    let options = fsl_analyze::provider::ProviderOptions::default();
    let summary = fsl_command::command::monitor_with_target_preparer(
        &path,
        std::io::Cursor::new(records(&[2], true)).split(b'\n'),
        &mut |event| events.push(event.as_value().clone()),
        fsl_command::command::MonitorProviders {
            negotiate: &crate::negotiation::Fr290,
            providers: Vec::new(),
            options: &options,
        },
        fsl_command::command::RunLimits::default(),
        &caller,
        &Cancel::new(),
        &mut preparer,
    )
    .expect("the driver handoff admits the target");
    assert_eq!(summary.exit_code, 0);
    assert!(!events.is_empty());
    assert_eq!(
        *prepared.borrow(),
        Some(("tick".to_owned(), 8, 17, 29)),
        "the driver receives target and caller preparation bounds"
    );
}

#[trace("TC-721", "FR-644-AC-16", "FR-644-AC-17", "FR-255-AC-24")]
#[test]
fn timed_target_reaches_driver_preparer_before_timed_trace_refusal() {
    let mut input = request();
    input["request"]["trace"]["kind"] = json!("timed");
    input["request"]["target"] = json!({
        "counter": "tick",
        "unit": "step",
        "width_bits": 16,
        "uncertainty": {"kind": "rational", "numerator": "0", "denominator": "1"},
        "max_event_rate": null,
        "max_reading_gap": 1
    });
    let (_case, path) = case_with(SOURCE.as_bytes(), &input);
    let mut caller = fsl_command::command::CommandLimits::monitor(
        fsl_command::command::MonitorPreparationLimits {
            work: 17,
            storage: 29,
        },
    );
    caller
        .set(
            fsl_foundation::SettingName::Stage(fsl_foundation::Setting::MonitorMaxActivations),
            8,
        )
        .expect("caller can set activation capacity");
    let prepared = std::cell::Cell::new(false);
    let mut preparer = |_: &fsl_package::CheckedPackage,
                        target: &fsl_replay::MonitorTarget,
                        caller: &fsl_command::command::CommandLimits| {
        assert_eq!(target.counter, "tick");
        assert_eq!(target.max_activations, 8);
        assert_eq!(caller.preparation().work, 17);
        prepared.set(true);
        Ok(())
    };
    let pulled = std::cell::Cell::new(0);
    let records = std::io::Cursor::new(records(&[2], true))
        .split(b'\n')
        .map(|line| {
            pulled.set(pulled.get() + 1);
            line
        });
    let options = fsl_analyze::provider::ProviderOptions::default();
    let error = fsl_command::command::monitor_with_target_preparer(
        &path,
        records,
        &mut |_| {},
        fsl_command::command::MonitorProviders {
            negotiate: &crate::negotiation::Fr290,
            providers: Vec::new(),
            options: &options,
        },
        fsl_command::command::RunLimits::default(),
        &caller,
        &Cancel::new(),
        &mut preparer,
    )
    .expect_err("timed traces remain unsupported after target handoff");
    assert!(
        error
            .to_string()
            .contains("timed trace is read but not yet run"),
        "{error}"
    );
    assert!(prepared.get(), "driver target preparation runs first");
    assert_eq!(pulled.get(), 0, "the refusal precedes record reads");
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
    let (events, summary) = stream.completed(22);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(summary["cancelled"], "requested", "{summary}");
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

/// The batch form: the `records` file of a `native-monitor/1` request through
/// `command::run`, as `(events, summary)`.
fn batch(file: &[u8]) -> (Vec<Value>, Value) {
    let mut document = request();
    document["request"]["records"] = json!("records.jsonl");
    let (case, path) = case_with(SOURCE.as_bytes(), &document);
    case.write("records.jsonl", file);
    let result = fsl_command::command::run_with_limits_and_overrides(
        &path,
        fsl_command::command::RunLimits::default(),
        &fsl_command::command::RequestOverrides::default(),
        &crate::negotiation::Fr290,
    )
    .expect("the batch run completes");
    let run = result.value.as_value().clone();
    assert_eq!(run["format"], "filament.monitor-run/1", "{run}");
    (
        run["events"].as_array().expect("events").clone(),
        run["summary"].clone(),
    )
}

/// QSL-2671, FR-644 Inputs ("a file of records gives the events a live stream
/// of the same records gives"): a records file that ends in a newline and has
/// no `end` record gives no spurious refusal for the terminator, and one with
/// a trailing blank line refuses that line as one blank record, each exactly
/// as the same bytes run as a stream.
#[test]
#[trace("FR-644")]
fn a_records_file_gives_the_events_of_the_same_stream() {
    let (_case, path) = load();
    let terminated = records(&[2, 4], false);
    let mut blank_tail = terminated.clone();
    blank_tail.push(b'\n');
    for (file, refused) in [(terminated, 0), (blank_tail, 1)] {
        let (events, summary) = batch(&file);
        let stream = monitor(&path, &file);
        let (stream_events, stream_summary) = stream.completed(0);
        assert_eq!(summary["refused"], refused, "{summary}");
        assert_eq!(events, stream_events, "events");
        assert_eq!(summary["records"], stream_summary["records"], "{summary}");
        assert_eq!(summary["clauses"], stream_summary["clauses"], "{summary}");
    }
}

const QSL_2750_SOURCE: &str = r#"language "ix:native" edition "1-draft";
profile w = "filament.temporal.timestamped-event.finite-window/v1";
record Response { latency: Quantity<ms>; }
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
temporal MissingQuantile using w over (r: Response) clock "plant-ms" on origin {
    always[0,300000] (quantile 0.95 of r.latency over past[0 ms,300 s] <= 5 ms)
}
"#;

fn qsl_2750_case() -> (Case, std::path::PathBuf, Value) {
    let mut request = request();
    request["request"]["clauses"] = json!([{"name":"MissingQuantile"}]);
    let clock = json!({"kind":"timestamped_event","timestamp_unit":"ms"});
    request["request"]["clocks"] = json!({"plant-ms":clock});
    request["request"]["trace"] = json!({"kind":"timestamped","lower_boundary":"authoritative-origin",
        "binding":{"profile":"filament.temporal.timestamped-event.finite-window/v1",
            "clock":"plant-ms","parameters":clock}});
    let (case, path) = case_with(QSL_2750_SOURCE.as_bytes(), &request);
    (case, path, request)
}

fn qsl_2750_records(missing: bool, empty: bool) -> Vec<Value> {
    let mut records = Vec::new();
    for index in 0_u64..20 {
        let value = match index {
            18 => 6,
            19 => 7,
            _ => 1,
        };
        let mut record = json!({"observation":{"sample":{"latency":{"kind":"quantity","unit":"ms",
            "magnitude":{"kind":"rational","numerator":value.to_string(),"denominator":"1"}}}},"stamp":index});
        if missing && index == 18 {
            // The only aggregate is node 1, below the outer always node,
            // as in the native-run fixture; the batch request checks it too.
            record["missing"] = json!([{"position":18,"atom":1}]);
        } else if empty {
            record["missing"] = json!([]);
        }
        records.push(record);
    }
    records.push(json!({"progress":{"watermark":300000,
        "completeness":{"complete":{"through":300000}},"scope":"closed","execution":"closed"}}));
    records.push(json!({"end":true}));
    records
}

fn qsl_2750_lines(records: &[Value]) -> Vec<u8> {
    records
        .iter()
        .map(|record| format!("{record}\n"))
        .collect::<String>()
        .into_bytes()
}

#[test]
#[trace("FR-644-AC-13", "FR-643-AC-13", "FR-194-AC-17")]
fn qsl_2750_stream_missing_window_matches_the_real_batch_run() {
    let (case, path, request) = qsl_2750_case();
    let records = qsl_2750_records(true, false);
    let stream = monitor(&path, &qsl_2750_lines(&records));
    let (events, summary) = stream.completed(11);
    assert_eq!(summary["refused"], 0, "{summary}");
    let mut batch = request.clone();
    batch["format"] = json!("native-run/1");
    batch["request"].as_object_mut().unwrap().remove("clauses");
    batch["request"].as_object_mut().unwrap().remove("trace");
    batch["request"]["clause"] = json!({"selection":{"temporal":{"name":"MissingQuantile",
        "trace":{"timestamped":{"observations":records.iter().take(20).map(|record|
            json!({"position":record["observation"],"stamp":record["stamp"]})).collect::<Vec<_>>(),
            "progress":records[20]["progress"],"lower_boundary":"authoritative-origin",
            "binding":request["request"]["trace"]["binding"],"missing":[{"position":18,"atom":1}]}}}},
        "observations":{}});
    let batch_path = case.json("batch-run.json", &batch);
    let batch = super::run(
        &batch_path,
        fsl_command::command::RunLimits::default(),
        &fsl_command::command::RequestOverrides::default(),
    );
    let batch = batch.expect("monitor", "indeterminate", 11);
    assert_eq!(
        batch["items"][0]["cause"]["kind"], "missing-valuation",
        "{batch}"
    );
    assert_eq!(batch["items"][0]["cause"]["position"], 18, "{batch}");
    let report = &summary["clauses"][0]["disposition"];
    assert!(report.to_string().contains("missing-valuation"), "{report}");
    assert!(
        events
            .iter()
            .any(|event| event["category"] == "indeterminate"),
        "{events:?}"
    );
    let omitted = monitor(&path, &qsl_2750_lines(&qsl_2750_records(false, false)));
    let empty = monitor(&path, &qsl_2750_lines(&qsl_2750_records(false, true)));
    omitted.completed(10);
    empty.completed(10);
    assert_eq!(omitted.bytes(), empty.bytes());
}

#[test]
#[trace("FR-644-AC-14")]
fn qsl_2750_malformed_missing_entries_are_refused_atomically() {
    let (_case, path, _) = qsl_2750_case();
    let records = qsl_2750_records(false, false);
    let clean = monitor(&path, &qsl_2750_lines(&records));
    let (clean_events, clean_summary) = clean.completed(10);
    let bad = [
        "null",
        "{}",
        "[{}]",
        "[{\"position\":1}]",
        "[{\"position\":1,\"atom\":1,\"extra\":0}]",
        "[{\"position\":1,\"atom\":1,\"atom\":1}]",
        "[{\"position\":null,\"atom\":1}]",
        "[{\"position\":1,\"atom\":null}]",
        "[{\"position\":-1,\"atom\":1}]",
        "[{\"position\":1,\"atom\":-1}]",
        "[{\"position\":1.5,\"atom\":1}]",
        "[{\"position\":1,\"atom\":1.5}]",
        "[{\"position\":\"1\",\"atom\":1}]",
        "[{\"position\":1,\"atom\":\"1\"}]",
        "[{\"position\":18446744073709551616,\"atom\":1}]",
        "[{\"position\":1,\"atom\":18446744073709551616}]",
    ];
    let mut lines = qsl_2750_lines(&records[..1]);
    let mut malformed = Vec::new();
    for missing in bad {
        malformed.push(format!(
            "{{\"observation\":{},\"stamp\":1,\"missing\":{missing}}}",
            records[1]["observation"]
        ));
    }
    malformed.push(format!(
        "{{\"observation\":{},\"stamp\":1,\"missing\":[],\"missing\":[]}}",
        records[1]["observation"]
    ));
    malformed.push("{\"progress\":{},\"missing\":[]}".to_owned());
    for line in &malformed {
        lines.extend_from_slice(format!("{line}\n").as_bytes());
    }
    lines.extend_from_slice(&qsl_2750_lines(&records[1..]));
    let stream = monitor(&path, &lines);
    let (events, summary) = stream.completed(10);
    let refusals = events
        .iter()
        .filter(|event| event["category"] == "refusal")
        .collect::<Vec<_>>();
    assert_eq!(refusals.len(), malformed.len(), "{events:?}");
    assert_eq!(summary["refused"], u64::try_from(malformed.len()).unwrap());
    for refusal in refusals {
        let disposition = &refusal["disposition"];
        assert_eq!(disposition["stage"], "request");
        assert_eq!(disposition["cause"], "invalid-request");
        assert!(
            matches!(disposition["member"].as_str(),Some(member) if member.starts_with("missing") || member=="atom"),
            "{disposition}"
        );
    }
    let strip_record = |event: &Value| {
        let mut value = event.clone();
        value.as_object_mut().unwrap().remove("record");
        value
    };
    assert_eq!(
        events
            .iter()
            .filter(|event| event["category"] != "refusal")
            .map(strip_record)
            .collect::<Vec<_>>(),
        clean_events.iter().map(strip_record).collect::<Vec<_>>()
    );
    assert_eq!(summary["clauses"], clean_summary["clauses"]);
    assert_eq!(summary["ledger"], clean_summary["ledger"]);
}

#[test]
#[trace("FR-644-AC-14")]
fn qsl_2750_empty_missing_is_refused_on_other_trace_kinds() {
    let (_case, path) = load();
    let mut with_member = json!({"observation":{"sample":{"bins":{"integer":"2"}}},"missing":[]})
        .to_string()
        .into_bytes();
    with_member.push(b'\n');
    with_member.extend(records(&[2, 4, 9], true));
    let stream = monitor(&path, &with_member);
    let (events, summary) = stream.completed(10);
    assert_eq!(summary["refused"], 1);
    assert_eq!(events[0]["disposition"]["member"], "missing");
    let clean = monitor(&path, &records(&[2, 4, 9], true));
    let (_, clean_summary) = clean.completed(10);
    assert_eq!(summary["clauses"], clean_summary["clauses"]);
}
