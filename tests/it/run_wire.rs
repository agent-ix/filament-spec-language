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
#[trace("TC-1665", "FR-642-AC-5")]
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

/// FR-640-AC-4, AC-6 (TC-1661): a malformed configuration refuses at stage
/// `request` naming the member, and a member named after a model binding is
/// refused as a reserved clock name.
#[test]
#[trace("TC-1661", "FR-640-AC-4", "FR-640-AC-6")]
#[ignore = "QSL-1964: the request `clocks` member is not implemented yet"]
fn malformed_and_reserved_clock_members_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let mut unknown = clocks();
    unknown["events"] = json!({"kind": "wall_clock", "sequence_authority": "events"});
    let mut zero = clocks();
    zero["sample"]["period"] = json!({"kind": "integer", "decimal": "0"});
    for (index, (selection, member)) in [(unknown, "events"), (zero, "sample")]
        .into_iter()
        .enumerate()
    {
        let output = compile_file(&write(
            directory.path(),
            &format!("malformed-{index}.json"),
            &compile_request(selection),
        ));
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "request", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert!(envelope.to_string().contains(member), "{envelope}");
    }
    for (index, name) in ["model-steps", "model-time"].into_iter().enumerate() {
        let mut reserved = clocks();
        reserved[name] = json!({"kind": "event_position", "sequence_authority": "events"});
        let output = compile_file(&write(
            directory.path(),
            &format!("reserved-{index}.json"),
            &compile_request(reserved),
        ));
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "emit", "{envelope}");
        assert_eq!(
            envelope["details"]["cause"], "reserved-clock-name",
            "{envelope}"
        );
        assert_eq!(envelope["details"]["clock"], name, "{envelope}");
    }
}

/// FR-641-AC-4, FR-312-AC-8 (TC-1663, TC-1160): position forms that do not
/// fit the selection, a `timestamped` observation without `stamp`, an empty
/// `timestamped` member and a ledger repeating a binding refuse at stage
/// `request` naming the member; a lasso of samples reaches evaluation.
#[test]
#[trace("TC-1663", "FR-641-AC-4", "TC-1160", "FR-312-AC-8")]
#[ignore = "QSL-2133: record samples and the new trace kinds are not on the run wire yet"]
fn malformed_trace_members_refuse_at_the_request_stage() {
    let directory = tempfile::tempdir().unwrap();
    let over = json!({"reference": {"population": "P", "key": "c1"}});
    let duplicate = json!({"format": "filament.progress-ledger/1", "entries": [
        {"binding": {"package": "p", "clause": "c", "occurrence": 0, "clock": "events",
            "profile": EVENTS}, "watermark": 1, "completeness": "incomplete",
            "scope": "open", "execution": "open", "covered": []},
        {"binding": {"package": "p", "clause": "c", "occurrence": 0, "clock": "events",
            "profile": EVENTS}, "watermark": 2, "completeness": "incomplete",
            "scope": "open", "execution": "open", "covered": []}]});
    std::fs::write(
        directory.path().join("duplicate.json"),
        serde_json::to_vec(&duplicate).unwrap(),
    )
    .unwrap();
    let cases = [
        (
            json!({"name": "Reaches", "trace": {"finite": [json!("s0"), sample(1)]}}),
            "trace",
        ),
        (
            json!({"name": "Reaches", "trace": {"finite": samples(&[1])}, "over": over}),
            "over",
        ),
        (
            json!({"name": "T", "trace": {"timestamped": {
                "observations": [{"position": sample(1)}],
                "progress": progress(1), "lower_boundary": "authoritative-origin",
                "binding": binding(STAMPED, "plant-ms")}}}),
            "stamp",
        ),
        (
            json!({"name": "T", "trace": {"timestamped": {}}}),
            "observations",
        ),
        (
            json!({"name": "E", "trace": {"progressed": {
                "positions": samples(&[1]), "progress": progress(1),
                "lower_boundary": "authoritative-origin", "binding": binding(EVENTS, "events")}},
                "ledger": "duplicate.json"}),
            "ledger",
        ),
    ];
    for (index, (temporal, member)) in cases.into_iter().enumerate() {
        let output = run_file(&write(
            directory.path(),
            &format!("malformed-trace-{index}.json"),
            &run_request(temporal),
        ));
        assert_eq!(output.status.code(), Some(20));
        let envelope = refusal(&output);
        assert_eq!(envelope["stage"], "request", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert!(
            envelope.to_string().contains(member),
            "{member}: {envelope}"
        );
    }
    let lasso = run_request(json!({"name": "Reaches",
        "trace": {"lasso": {"prefix": samples(&[0]), "loop": samples(&[1, 2])}}}));
    let document = result(&run_file(&write(directory.path(), "lasso.json", &lasso)));
    assert_eq!(document["disposition"]["category"], "success", "{document}");
}

/// FR-641-AC-5 (TC-1663): a ledger read from a file and written back by a run
/// that adds nothing reproduces its bytes.
#[test]
#[trace("TC-1663", "FR-641-AC-5")]
#[ignore = "QSL-2133: the `ledger` file is not on the run wire yet"]
fn a_ledger_round_trips_through_a_run_that_changes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let request = |ledger: Option<&str>| {
        let mut temporal = json!({"name": "E", "trace": {"progressed": {
            "positions": samples(&[0, 1]), "progress": progress(3),
            "lower_boundary": "authoritative-origin", "binding": binding(EVENTS, "events")}}});
        if let Some(ledger) = ledger {
            temporal["ledger"] = json!(ledger);
        }
        run_request(temporal)
    };
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &request(None),
    )));
    let bytes = serde_json::to_vec(&first["ledger"]).unwrap();
    std::fs::write(directory.path().join("ledger.json"), &bytes).unwrap();
    let again = result(&run_file(&write(
        directory.path(),
        "again.json",
        &request(Some("ledger.json")),
    )));
    assert_eq!(serde_json::to_vec(&again["ledger"]).unwrap(), bytes);
}

/// FR-645-AC-5 (TC-1671): the refusal of the same trace built in process
/// carries the same reason, without a request pointer.
#[test]
#[trace("TC-1671", "FR-645-AC-5")]
#[ignore = "QSL-2133: trace refusals carry no reason fields yet"]
fn an_in_process_trace_refusal_carries_no_request_pointer() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(
        directory.path(),
        "lasso.json",
        &run_request(json!({"name": "Reaches", "trace": {"lasso": {"prefix": [], "loop": []}}})),
    );
    let prepared = fsl_command::command::prepare_run(&path, Default::default())
        .unwrap_or_else(|error| panic!("the request prepares: {error:?}"));
    let fsl_command::command::PreparedRun::Clause(request) = prepared else {
        panic!("a clause run");
    };
    let report = fsl_replay::spine::run_clause(*request).expect("a report");
    let text = format!("{report:?}");
    assert!(text.contains("empty-lasso-loop"), "{text}");
    assert!(!text.contains("request_pointer"), "{text}");
}

/// The `native-monitor/1` request over `records`, run through `command::run`.
fn monitor_run(
    directory: &Path,
    name: &str,
    request: Value,
    records: &[Value],
) -> crate::command_harness::Output {
    let lines: Vec<String> = records.iter().map(Value::to_string).collect();
    std::fs::write(directory.join(format!("{name}.jsonl")), lines.join("\n")).unwrap();
    let mut request = request;
    request["request"]["records"] = json!(format!("{name}.jsonl"));
    run_file(&write(directory, &format!("{name}.json"), &request))
}

fn monitor_request(clauses: Value, trace: Value) -> Value {
    json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clauses": clauses, "trace": trace}})
}

fn observations(levels: &[i64]) -> Vec<Value> {
    levels
        .iter()
        .map(|level| json!({"observation": sample(*level)}))
        .collect()
}

/// FR-643-AC-2, FR-643-AC-5 (TC-1666, TC-1667): a timestamped stream with
/// progress after each observation ends with the batch run's verdict and
/// ledger; a stamp at the retained watermark is refused and the stream goes on.
#[test]
#[trace("TC-1666", "FR-643-AC-2", "TC-1667", "FR-643-AC-5")]
#[ignore = "QSL-1977: the monitor command does not run a record stream yet"]
fn a_timestamped_stream_matches_the_batch_run_and_refuses_a_regressing_stamp() {
    let directory = tempfile::tempdir().unwrap();
    let trace = json!({"kind": "timestamped", "lower_boundary": "authoritative-origin",
        "binding": binding(STAMPED, "plant-ms")});
    let mut records = Vec::new();
    for (stamp, level) in [(0, 0), (2, 0), (5, 1)] {
        records.push(json!({"observation": sample(level), "stamp": stamp}));
        records.push(json!({"progress": progress(stamp)}));
    }
    records.insert(4, json!({"observation": sample(1), "stamp": 2}));
    records.push(json!({"end": "closed"}));
    let output = monitor_run(
        directory.path(),
        "stamped",
        monitor_request(json!([{"name": "T"}]), trace),
        &records,
    );
    let run = result(&output);
    let events = run["events"].as_array().expect("events");
    assert!(
        events
            .iter()
            .any(|event| event["record"] == 4 && event["category"] == "refusal"),
        "the regressing stamp is refused: {run}"
    );
    let batch = request_batch(directory.path());
    assert_eq!(
        run["summary"]["clauses"][0]["disposition"], batch["disposition"],
        "{run}"
    );
    assert_eq!(run["summary"]["ledger"], batch["ledger"], "{run}");
}

/// The batch timestamped run FR-641-AC-1 states, open at the end.
fn request_batch(directory: &Path) -> Value {
    let observations: Vec<Value> = [(0, 0), (2, 0), (5, 1)]
        .into_iter()
        .map(|(stamp, level)| json!({"position": sample(level), "stamp": stamp}))
        .collect();
    let request = run_request(json!({"name": "T", "trace": {"timestamped": {
        "observations": observations, "progress": progress(5),
        "lower_boundary": "authoritative-origin", "binding": binding(STAMPED, "plant-ms")}}}));
    result(&run_file(&write(directory, "batch.json", &request)))
}

/// FR-643-AC-3, FR-644-AC-1 (TC-1666, TC-1668): a settled violation is
/// reported once, at the observation that decides it, and never changes.
#[test]
#[trace("TC-1666", "FR-643-AC-3", "TC-1668", "FR-644-AC-1")]
#[ignore = "QSL-1977: the monitor command does not run a record stream yet"]
fn a_settled_result_is_reported_once_and_never_changes() {
    let directory = tempfile::tempdir().unwrap();
    let mut records = observations(&[0, 1, 2, 1, 0]);
    records.push(json!({"end": "open"}));
    let output = monitor_run(
        directory.path(),
        "settled",
        monitor_request(json!([{"name": "Never"}]), json!({"kind": "finite"})),
        &records,
    );
    assert_eq!(output.status.code(), Some(10));
    let run = result(&output);
    let events: Vec<&Value> = run["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter(|event| event["clause"] == "Never")
        .collect();
    let settled: Vec<&&Value> = events
        .iter()
        .filter(|event| event["category"] == "violation")
        .collect();
    assert_eq!(settled.len(), 1, "{run}");
    assert_eq!(settled[0]["record"], 2, "{run}");
    assert!(
        events
            .iter()
            .all(|event| event["record"].as_u64() <= Some(2)),
        "{run}"
    );
}

/// FR-643-AC-4, FR-644-AC-3 (TC-1667, TC-1669): a checkpoint record and a
/// resumed request give the later events of the uninterrupted stream.
#[test]
#[trace("TC-1667", "FR-643-AC-4", "TC-1669", "FR-644-AC-3")]
#[ignore = "QSL-1977: the monitor command does not checkpoint or resume yet"]
fn a_checkpointed_stream_resumes_to_the_same_events() {
    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(
        json!([{"name": "Reaches"}, {"name": "Never"}]),
        json!({"kind": "finite"}),
    );
    let mut whole = observations(&[0, 1, 2, 1, 0]);
    whole.push(json!({"end": "open"}));
    let full = result(&monitor_run(
        directory.path(),
        "whole",
        request.clone(),
        &whole,
    ));

    let mut head = observations(&[0, 1]);
    head.push(json!({"checkpoint": "state.json"}));
    head.push(json!({"end": "open"}));
    monitor_run(directory.path(), "head", request.clone(), &head);
    let mut resumed_request = request;
    resumed_request["request"]["state"] = json!("state.json");
    let mut tail = observations(&[2, 1, 0]);
    tail.push(json!({"end": "open"}));
    let resumed = result(&monitor_run(
        directory.path(),
        "tail",
        resumed_request,
        &tail,
    ));
    assert_eq!(
        resumed["summary"]["clauses"], full["summary"]["clauses"],
        "{resumed}"
    );
}

/// FR-644-AC-2, AC-4 (TC-1668, TC-1669): a malformed record is reported as a
/// refusal event and the stream goes on; a request naming a missing clause
/// returns the command-error envelope before any record.
#[test]
#[trace("TC-1668", "FR-644-AC-2", "TC-1669", "FR-644-AC-4")]
#[ignore = "QSL-1977: the monitor command does not run a record stream yet"]
fn a_bad_record_is_an_event_and_a_bad_request_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(json!([{"name": "Never"}]), json!({"kind": "finite"}));
    let mut good = observations(&[0, 1, 2]);
    good.push(json!({"end": "open"}));
    let clean = result(&monitor_run(
        directory.path(),
        "clean",
        request.clone(),
        &good,
    ));
    let mut bad = good.clone();
    bad.insert(1, json!({"observation": sample(0), "colour": "red"}));
    let noisy = result(&monitor_run(directory.path(), "noisy", request, &bad));
    let refusals: Vec<&Value> = noisy["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter(|event| event["category"] == "refusal")
        .collect();
    assert_eq!(refusals.len(), 1, "{noisy}");
    assert_eq!(refusals[0]["record"], 1, "{noisy}");
    assert_eq!(
        noisy["summary"]["clauses"], clean["summary"]["clauses"],
        "{noisy}"
    );

    let missing = monitor_request(json!([{"name": "Absent"}]), json!({"kind": "finite"}));
    let output = monitor_run(directory.path(), "missing", missing, &good);
    assert_eq!(output.status.code(), Some(20));
    let envelope = refusal(&output);
    assert!(envelope.to_string().contains("Absent"), "{envelope}");
}

/// FR-640-AC-8 (TC-2182): the `clocks` object rebuilt from a package's
/// `temporal_bindings` rows compiles the same program to the same bytes; it
/// is the selection list the replay recompile builds from the package.
#[test]
#[trace("TC-2182", "FR-640-AC-8")]
#[ignore = "QSL-1964: the request `clocks` member is not implemented yet"]
fn clocks_rebuilt_from_package_rows_compile_the_same_package() {
    let directory = tempfile::tempdir().unwrap();
    let first = compile_file(&write(
        directory.path(),
        "first.json",
        &compile_request(clocks()),
    ));
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let package: Value = serde_json::from_slice(&first.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    let mut rebuilt = serde_json::Map::new();
    for row in rows {
        let name = row["clock_name"].as_str().expect("a clock name").to_owned();
        rebuilt.insert(name, row["clock"].clone());
    }
    let second = compile_file(&write(
        directory.path(),
        "second.json",
        &compile_request(Value::Object(rebuilt)),
    ));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
}

/// FR-765-AC-10 (TC-2191): a `native-monitor/1` request's `integer_division`
/// member supplies the law its program's `div` needs.
#[test]
#[trace("TC-2191", "FR-765-AC-10")]
#[ignore = "QSL-1989: the native-monitor/1 integer_division member is not implemented yet"]
fn a_monitor_request_supplies_the_integer_division_law() {
    let directory = tempfile::tempdir().unwrap();
    let mut records = observations(&[0, 1, 2]);
    records.push(json!({"end": "open"}));
    let lines: Vec<String> = records.iter().map(Value::to_string).collect();
    std::fs::write(directory.path().join("half.jsonl"), lines.join("\n")).unwrap();
    let run = |law: Option<&str>| {
        let mut request = monitor_request(json!([{"name": "Half"}]), json!({"kind": "finite"}));
        request["request"]["records"] = json!("half.jsonl");
        if let Some(law) = law {
            request["request"]["integer_division"] = json!(law);
        }
        let path = directory.path().join("half.json");
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        std::fs::write(
            directory.path().join("unit.native"),
            format!(
                "{}function half using v(a: Int[0, 9]): Int[0, 9] pure {{ a div 2 }}\n\
                 temporal Half using i over (r: Reading) clock \"steps\" on origin {{ always holds(half(r.level) < 5) }}\n",
                unit()
            ),
        )
        .unwrap();
        run_file(&path)
    };

    let output = run(Some("floor"));
    let monitored = result(&output);
    assert_eq!(monitored["format"], "filament.monitor-run/1", "{monitored}");

    let output = run(None);
    assert_eq!(output.status.code(), Some(20));
    let refused = refusal(&output);
    assert_eq!(
        find(&refused, "cause").and_then(Value::as_str),
        Some("missing-selection"),
        "{refused}"
    );
}
