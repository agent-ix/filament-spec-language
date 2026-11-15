// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640, FR-641, FR-644, FR-645: clock selections, every temporal trace
//! kind, the carried progress ledger, trace refusals and the monitor stream
//! through the library `command` over request files.
//!
//! Every unit here declares its clauses over a record trace subject, so the
//! traces are inline record samples and no domain package is needed.

use crate::command_harness::{compile_file, run_file};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const VALUE: &str = "filament.value.complete/v1";
const EVENTS: &str = "filament.temporal.event-position.false-extension/v1";
const SAMPLED: &str = "filament.temporal.fixed-sample.false-extension/v1";
const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";
const INFINITE: &str = "filament.temporal.infinite-trace/v1";
const TIMED: &str = "filament.temporal.timed/v1";

/// One unit with a clause per profile over `r: Reading`.
fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"{VALUE}\";\n\
         profile e = \"{EVENTS}\";\n\
         profile f = \"{SAMPLED}\";\n\
         profile w = \"{STAMPED}\";\n\
         profile i = \"{INFINITE}\";\n\
         profile t = \"{TIMED}\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal E using e over (r: Reading) clock \"events\" on origin {{ eventually[0,2] holds(r.level = 2) }}\n\
         temporal S using f over (r: Reading) clock \"sample\" on origin {{ eventually[0,3] holds(r.level = 2) }}\n\
         temporal T using w over (r: Reading) clock \"plant-ms\" on origin {{ eventually[0,5] holds(r.level > 0) }}\n\
         temporal Reaches using i over (r: Reading) clock \"steps\" on origin {{ eventually holds(r.level = 2) }}\n\
         temporal Never using i over (r: Reading) clock \"steps\" on origin {{ always holds(r.level < 2) }}\n\
         temporal Ack using t over (r: Reading) clock \"plant\" on origin {{ eventually[0 ms, 200 ms] holds(r.level = 1) }}\n"
    )
}

fn clocks() -> Value {
    json!({
        "events": {"kind": "event_position", "sequence_authority": "events"},
        "sample": {"kind": "fixed_sample",
            "epoch": {"kind": "integer", "decimal": "0"},
            "period": {"kind": "integer", "decimal": "1"}, "unit": "s"},
        "plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"},
        "plant": {"kind": "timed", "unit": "ms"},
    })
}

fn program() -> Value {
    json!({"source": {"file": "unit.native", "authority": "agent-ix",
        "identity": "test:run-wire", "document": "RunWire", "formal_revision": 1}})
}

fn sample(level: i64) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}}})
}

fn samples(levels: &[i64]) -> Vec<Value> {
    levels.iter().copied().map(sample).collect()
}

/// Writes the unit and `request` (as `name`) into `directory`.
fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn compile_request(clocks: Value) -> Value {
    json!({"format": "native-compile/1", "request": {
        "models": [], "program": program(), "clocks": clocks}})
}

/// A clause-run request over the temporal selection `temporal`.
fn run_request(temporal: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clause": {"selection": {"temporal": temporal}, "observations": {}}}})
}

/// The binding row of `clock` under `profile`, as the trace asserts it.
fn binding(profile: &str, clock: &str) -> Value {
    json!({"profile": profile, "clock": clock, "parameters": clocks()[clock].clone()})
}

fn progress(watermark: u64) -> Value {
    json!({"watermark": watermark, "completeness": {"complete": {"through": watermark}},
        "scope": "open", "execution": "open"})
}

/// The first value under `key` anywhere in `value`, depth first.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members
            .get(key)
            .or_else(|| members.values().find_map(|member| find(member, key))),
        Value::Array(items) => items.iter().find_map(|item| find(item, key)),
        _ => None,
    }
}

fn result(output: &crate::command_harness::Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "expected a result document, got the refusal {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a result document")
}

fn refusal(output: &crate::command_harness::Output) -> Value {
    assert!(output.stdout.is_empty(), "expected a refusal envelope");
    serde_json::from_slice(&output.stderr).expect("a refusal envelope")
}

/// FR-640-AC-1 (TC-1660): `clocks` configures every trace-subject clock by
/// name and the package carries one binding row per clock-bearing clause.
#[test]
#[trace("TC-1660", "FR-640-AC-1", "FSpec-TC-1160", "FSpec-FR-540-AC-1")]
#[ignore = "QSL-1964: the request `clocks` member is not implemented yet"]
fn clocks_member_compiles_event_position_fixed_sample_and_timestamped_clauses() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), "compile.json", &compile_request(clocks()));
    let output = compile_file(&path);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    for (clock, kind) in [
        ("events", "event_position"),
        ("sample", "fixed_sample"),
        ("plant-ms", "timestamped_event"),
        ("plant", "timed"),
    ] {
        let row = rows
            .iter()
            .find(|row| row["clock_name"] == clock)
            .unwrap_or_else(|| panic!("a row for {clock}"));
        assert_eq!(row["clock"]["kind"], kind, "{row}");
        assert_eq!(row["clock"], clocks()[clock], "{row}");
    }
}

/// FR-640-AC-5 (TC-1660): the selection enters the package identity, and
/// member order does not.
#[test]
#[trace("TC-1660", "FR-640-AC-5", "FSpec-TC-1160", "FSpec-FR-540-AC-3")]
#[ignore = "QSL-1964: the request `clocks` member is not implemented yet"]
fn clock_configuration_changes_package_identity_and_member_order_does_not() {
    let directory = tempfile::tempdir().unwrap();
    let base = compile_file(&write(
        directory.path(),
        "a.json",
        &compile_request(clocks()),
    ));
    let mut other = clocks();
    other["events"]["sequence_authority"] = json!("elsewhere");
    let changed = compile_file(&write(directory.path(), "b.json", &compile_request(other)));
    let reordered: serde_json::Map<String, Value> = clocks()
        .as_object()
        .unwrap()
        .iter()
        .rev()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let same = compile_file(&write(
        directory.path(),
        "c.json",
        &compile_request(Value::Object(reordered)),
    ));
    assert_eq!(base.status.code(), Some(0));
    assert_ne!(base.stdout, changed.stdout);
    assert_eq!(base.stdout, same.stdout);
}

/// FR-640-AC-3 (TC-1661): a missing, an unused and a mismatched selection
/// each refuse at stage `emit` with their cause and the clock's name.
#[test]
#[trace("TC-1661", "FR-640-AC-3", "FSpec-TC-1160", "FSpec-FR-540-AC-2")]
#[ignore = "QSL-1964: clock selection refusals are not implemented yet"]
fn missing_unused_and_mismatched_clock_selections_refuse_naming_the_clock() {
    let directory = tempfile::tempdir().unwrap();
    let mut missing = clocks();
    missing.as_object_mut().unwrap().remove("plant-ms");
    let mut unused = clocks();
    unused["spare"] = json!({"kind": "timestamped_event", "timestamp_unit": "ms"});
    let mut mismatched = clocks();
    mismatched["events"] = json!({"kind": "timestamped_event", "timestamp_unit": "ms"});
    for (index, (selection, cause, clock)) in [
        (missing, "missing-clock-selection", "plant-ms"),
        (unused, "unused-clock-selection", "spare"),
        (mismatched, "clock-kind-mismatch", "events"),
    ]
    .into_iter()
    .enumerate()
    {
        let path = write(
            directory.path(),
            &format!("compile-{index}.json"),
            &compile_request(selection),
        );
        let output = compile_file(&path);
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "emit", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert_eq!(envelope["details"]["cause"], cause, "{envelope}");
        assert_eq!(envelope["details"]["clock"], clock, "{envelope}");
        assert!(
            envelope["message"]
                .as_str()
                .is_some_and(|message| message.contains(clock)),
            "{envelope}"
        );
    }
}

/// FR-640-AC-2, FR-641-AC-1 (TC-1660, TC-1662): the timestamped clause runs
/// over a request-file `timestamped` trace and the result carries the ledger.
#[test]
#[trace(
    "TC-1662",
    "FR-641-AC-1",
    "FR-640-AC-2",
    "FR-267-AC-21",
    "FSpec-TC-1163",
    "FSpec-FR-352-AC-18"
)]
#[ignore = "QSL-2133: the `timestamped` trace kind is not on the run wire yet"]
fn a_timestamped_trace_runs_from_a_request_file_and_returns_the_ledger() {
    let directory = tempfile::tempdir().unwrap();
    let observations: Vec<Value> = [(0, 0), (2, 0), (5, 1)]
        .into_iter()
        .map(|(stamp, level)| json!({"position": sample(level), "stamp": stamp}))
        .collect();
    let request = run_request(json!({"name": "T", "trace": {"timestamped": {
        "observations": observations,
        "progress": {"watermark": 5, "completeness": {"complete": {"through": 5}},
            "scope": "closed", "execution": "closed"},
        "lower_boundary": "authoritative-origin",
        "binding": binding(STAMPED, "plant-ms")}}}));
    let output = run_file(&write(directory.path(), "run.json", &request));
    let document = result(&output);
    assert_eq!(document["disposition"]["stage"], "evaluate", "{document}");
    assert_eq!(output.status.code(), Some(0), "{document}");
    assert_eq!(
        document["ledger"]["format"], "filament.progress-ledger/1",
        "{document}"
    );
}

/// FR-641-AC-2, AC-3 (TC-1662, TC-1663): a `progressed` event-position
/// trace returns a ledger; carried into the next request, a regressing
/// watermark is refused with the ledger unchanged and a later one advances.
#[test]
#[trace(
    "TC-1663",
    "FR-641-AC-2",
    "FR-641-AC-3",
    "FSpec-TC-1162",
    "FSpec-FR-114-AC-5"
)]
#[ignore = "QSL-2133: the `progressed` trace kind and the `ledger` file are not on the run wire yet"]
fn a_carried_ledger_refuses_a_regressing_watermark_and_advances_on_a_later_one() {
    let directory = tempfile::tempdir().unwrap();
    let progressed = |watermark: u64, ledger: Option<&str>| {
        let mut temporal = json!({"name": "E", "trace": {"progressed": {
            "positions": samples(&[0, 1]),
            "progress": progress(watermark),
            "lower_boundary": "authoritative-origin",
            "binding": binding(EVENTS, "events")}}});
        if let Some(ledger) = ledger {
            temporal["ledger"] = json!(ledger);
        }
        run_request(temporal)
    };
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &progressed(3, None),
    )));
    let ledger = first["ledger"].clone();
    assert_eq!(ledger["format"], "filament.progress-ledger/1", "{first}");
    std::fs::write(
        directory.path().join("ledger.json"),
        serde_json::to_vec(&ledger).unwrap(),
    )
    .unwrap();

    let regress = result(&run_file(&write(
        directory.path(),
        "regress.json",
        &progressed(1, Some("ledger.json")),
    )));
    assert_eq!(regress["disposition"]["stage"], "evaluate", "{regress}");
    assert_eq!(regress["disposition"]["category"], "refusal", "{regress}");
    assert_eq!(
        find(&regress, "contradicted_dimension"),
        Some(&json!("Watermark")),
        "{regress}"
    );
    assert_eq!(
        regress["ledger"], ledger,
        "the refused run returns the ledger unchanged"
    );

    let advance = result(&run_file(&write(
        directory.path(),
        "advance.json",
        &progressed(9, Some("ledger.json")),
    )));
    assert_ne!(advance["disposition"]["category"], "refusal", "{advance}");
    assert_eq!(advance["ledger"]["entries"][0]["watermark"], 9, "{advance}");
}

/// The trace refusal fields FR-645 adds, read from a result document.
fn trace_refusal(document: &Value) -> (&Value, &Value, &Value) {
    let reason = find(document, "reason").expect("a reason field");
    let pointer = find(document, "request_pointer").expect("a request pointer");
    let message = find(document, "message").expect("a message");
    (reason, pointer, message)
}

/// FR-645-AC-1, AC-2 (TC-1670): an empty lasso loop and an empty finite
/// trace each refuse with a reason, the request pointer, a message and the
/// clause's locus.
#[test]
#[trace("TC-1670", "FR-645-AC-1", "FR-645-AC-2")]
#[ignore = "QSL-2133: trace refusals carry no reason, message or locus yet"]
fn empty_traces_refuse_with_a_reason_message_and_locus() {
    let directory = tempfile::tempdir().unwrap();
    for (index, (trace, reason, pointer)) in [
        (
            json!({"lasso": {"prefix": [], "loop": []}}),
            "empty-lasso-loop",
            "selection.temporal.trace.lasso.loop",
        ),
        (
            json!({"finite": []}),
            "empty-finite-trace",
            "selection.temporal.trace.finite",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let request = run_request(json!({"name": "Reaches", "trace": trace}));
        let output = run_file(&write(
            directory.path(),
            &format!("empty-{index}.json"),
            &request,
        ));
        assert_eq!(output.status.code(), Some(20));
        let document = result(&output);
        assert_eq!(document["disposition"]["stage"], "evaluate", "{document}");
        assert_eq!(document["disposition"]["category"], "refusal", "{document}");
        let (found, found_pointer, message) = trace_refusal(&document);
        assert_eq!(found, reason, "{document}");
        assert_eq!(found_pointer, pointer, "{document}");
        assert!(
            message.as_str().is_some_and(|text| !text.is_empty()),
            "{document}"
        );
        assert!(find(&document, "locus").is_some(), "{document}");
    }
}

/// FR-645-AC-3, AC-4, FR-642-AC-5 (TC-1671, TC-1665): a trace kind the
/// clause's profile does not read refuses at stage `admit` naming the
/// profile and the expected and supplied kinds. A timed clause over a trace
/// without stamps is the documented case: positions are not time.
#[test]
#[trace(
    "TC-1671",
    "FR-645-AC-3",
    "FR-645-AC-4",
    "FR-642-AC-5",
    "FSpec-TC-1161",
    "FSpec-FR-541-AC-3"
)]
#[ignore = "QSL-2133: trace-kind-mismatch refusals carry no fields yet"]
fn a_trace_kind_the_profile_does_not_read_refuses_with_profile_and_kinds() {
    let directory = tempfile::tempdir().unwrap();
    for (index, (name, trace, profile, expected, supplied)) in [
        (
            "E",
            json!({"lasso": {"prefix": [], "loop": samples(&[0, 1])}}),
            EVENTS,
            json!(["finite", "progressed"]),
            "lasso",
        ),
        (
            "T",
            json!({"finite": samples(&[0, 1])}),
            STAMPED,
            json!(["timestamped"]),
            "finite",
        ),
        (
            "Ack",
            json!({"finite": samples(&[0, 1])}),
            TIMED,
            json!(["timed"]),
            "finite",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let request = run_request(json!({"name": name, "trace": trace}));
        let output = run_file(&write(
            directory.path(),
            &format!("kind-{index}.json"),
            &request,
        ));
        assert_eq!(output.status.code(), Some(20));
        let document = result(&output);
        assert_eq!(document["disposition"]["stage"], "admit", "{document}");
        assert_eq!(
            find(&document, "reason"),
            Some(&json!("trace-kind-mismatch")),
            "{document}"
        );
        assert_eq!(
            find(&document, "profile"),
            Some(&json!(profile)),
            "{document}"
        );
        assert_eq!(find(&document, "expected"), Some(&expected), "{document}");
        assert_eq!(
            find(&document, "supplied"),
            Some(&json!(supplied)),
            "{document}"
        );
        let message = find(&document, "message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            message.contains(profile) && message.contains(name),
            "{document}"
        );
    }
}

/// FR-644-AC-5, FR-643-AC-1 (TC-1668, TC-1666): a `native-monitor/1`
/// request over a record file gives, after each observation, the status a
/// clause run over the samples received so far gives, and a summary equal
/// to the run over all of them.
#[test]
#[trace(
    "TC-1668",
    "FR-644-AC-5",
    "FR-643-AC-1",
    "FSpec-TC-1162",
    "FSpec-FR-114-AC-5"
)]
#[ignore = "QSL-1977: the monitor command does not run a record stream yet"]
fn the_monitor_stream_equals_a_clause_run_over_each_prefix() {
    let directory = tempfile::tempdir().unwrap();
    let levels = [0, 1, 2, 1];
    let mut records: Vec<String> = levels
        .iter()
        .map(|level| json!({"observation": sample(*level)}).to_string())
        .collect();
    records.push(json!({"end": "open"}).to_string());
    std::fs::write(directory.path().join("records.jsonl"), records.join("\n")).unwrap();
    let request = json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clauses": [{"name": "Reaches"}, {"name": "Never"}],
        "trace": {"kind": "finite"},
        "records": "records.jsonl"}});
    let output = run_file(&write(directory.path(), "monitor.json", &request));
    assert_eq!(output.status.code(), Some(10));
    let run = result(&output);
    assert_eq!(run["format"], "filament.monitor-run/1", "{run}");
    let events = run["events"].as_array().expect("events");

    for clause in ["Reaches", "Never"] {
        let mut status = Value::Null;
        for record in 0..levels.len() {
            if let Some(event) = events
                .iter()
                .find(|event| event["record"] == record && event["clause"] == clause)
            {
                status = event["disposition"].clone();
            }
            let prefix = run_request(json!({"name": clause,
                "trace": {"finite": samples(&levels[..=record])}}));
            let batch = result(&run_file(&write(
                directory.path(),
                &format!("{clause}-{record}.json"),
                &prefix,
            )));
            assert_eq!(
                status, batch["disposition"],
                "{clause} after record {record}"
            );
        }
        let summary = run["summary"]["clauses"]
            .as_array()
            .and_then(|clauses| clauses.iter().find(|entry| entry["clause"] == clause))
            .expect("a summary entry");
        assert_eq!(summary["disposition"], status, "{clause} summary");
    }
}
