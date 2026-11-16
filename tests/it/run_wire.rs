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

/// The unit of the trace-wire tests: `unit()` without its timed clause.
///
/// These tests exercise the request wire for every trace kind and the carried
/// ledger. The timed clock kind (QSL-2254) and a timed clause over a trace
/// subject (QSL-2136) are other tickets', so the unit and the clock
/// selections here use the clock kinds that exist and clauses that check.
fn trace_unit() -> String {
    let full = unit();
    let kept: Vec<&str> = full
        .lines()
        .filter(|line| !line.starts_with("profile t =") && !line.starts_with("temporal Ack"))
        .collect();
    kept.join("\n") + "\n"
}

/// `clocks()` without the timed `plant` clock.
fn trace_clocks() -> Value {
    let mut selection = clocks();
    selection.as_object_mut().unwrap().remove("plant");
    selection
}

/// [`write`] for a trace-wire test: the unit is [`trace_unit`].
fn write_trace(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), trace_unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

/// [`run_request`] for a trace-wire test: the clocks are [`trace_clocks`].
fn trace_request(temporal: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [], "program": program(), "clocks": trace_clocks(),
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
#[ignore = "QSL-2254, QSL-2136: the unit's timed clause over a record trace subject waits on the timed clock configuration"]
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
#[ignore = "QSL-2254, QSL-2136: the unit's timed clause over a record trace subject waits on the timed clock configuration"]
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
#[ignore = "QSL-2254, QSL-2136: the unit's timed clause over a record trace subject waits on the timed clock configuration"]
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

/// One FR-501-AC-4 input: the levels at stamps 0, 2 and 5, the `(position,
/// atom)` valuations absent, and whether the run is `Indeterminate`.
type TimestampedCase = ([i64; 3], Vec<(u64, u64)>, bool);

/// The sample of `level` as the canonical text a typed position carries.
fn typed_sample(level: i64) -> fsl_replay::DocumentRef {
    fsl_replay::spine::sample_position(&format!("{{\"level\":{{\"integer\":\"{level}\"}}}}"))
}

/// The clause `T` over `stamped` `(stamp, level)` observations, built as a
/// typed value rather than read from a request file: watermark 5, complete
/// through 5, scope and execution `closed`, an authoritative origin and the
/// clause's timestamped-event binding.
fn typed_timestamped(
    stamped: &[(u64, i64)],
    missing: &[(u64, u64)],
) -> fsl_replay::spine::ClauseRunSelection {
    use fsl_replay::spine::{
        ClauseRunSelection, ProgressAssertion, ProgressClosure, ProgressCompleteness,
        TimestampedMissingValuation, TimestampedObservation, TimestampedWindow, TraceBinding,
        TraceInput,
    };
    ClauseRunSelection::SampledTemporal {
        name: "T".to_owned(),
        trace: TraceInput::Timestamped {
            observations: stamped
                .iter()
                .map(|(stamp, level)| TimestampedObservation {
                    document: typed_sample(*level),
                    stamp: *stamp,
                    order_key: None,
                })
                .collect(),
            progress: ProgressAssertion {
                watermark: 5,
                completeness: ProgressCompleteness::Complete { through: 5 },
                scope: ProgressClosure::Closed,
                execution: ProgressClosure::Closed,
            },
            window: Box::new(TimestampedWindow {
                binding: Some(TraceBinding {
                    profile: STAMPED.to_owned(),
                    clock_name: "plant-ms".to_owned(),
                    parameters: serde_json::from_value(trace_clocks()["plant-ms"].clone())
                        .expect("the clock configuration"),
                }),
                authoritative_origin: true,
                cover: None,
                missing_valuations: missing
                    .iter()
                    .map(|(position, atom)| TimestampedMissingValuation {
                        position: *position,
                        atom: *atom,
                    })
                    .collect(),
            }),
        },
        ledger: None,
    }
}

/// FR-640-AC-2, FR-641-AC-1 (TC-1660, TC-1662): the timestamped clause runs
/// over a request-file `timestamped` trace and reports what the same input
/// built as a typed `ClauseRunRequest` reports, byte for byte, with the
/// outcomes FR-501-AC-4 states, and the result carries the ledger.
#[test]
#[trace(
    "TC-1662",
    "FR-641-AC-1",
    "FR-640-AC-2",
    "FR-267-AC-21",
    "FSpec-TC-1163",
    "FSpec-FR-352-AC-18"
)]
fn a_timestamped_trace_runs_from_a_request_file_and_returns_the_ledger() {
    use fsl_replay::spine::{TemporalIndeterminateReason, TemporalVerdict};
    let directory = tempfile::tempdir().unwrap();
    // FR-501-AC-4: stamps 0, 2, 5 with `p` (`level > 0`) false at 0 and 5 and
    // its valuation absent at stamp 2 is `Indeterminate`; true at 5 it is
    // `Completed(true)`.
    let cases: [TimestampedCase; 3] = [
        ([0, 0, 0], vec![(1, 1)], true),
        ([0, 0, 1], vec![(1, 1)], false),
        ([0, 0, 1], vec![], false),
    ];
    for (index, (levels, missing, indeterminate)) in cases.into_iter().enumerate() {
        let observations: Vec<Value> = [0_u64, 2, 5]
            .into_iter()
            .zip(levels)
            .map(|(stamp, level)| json!({"position": sample(level), "stamp": stamp}))
            .collect();
        let missing_members: Vec<Value> = missing
            .iter()
            .map(|(position, atom)| json!({"position": position, "atom": atom}))
            .collect();
        let request = trace_request(json!({"name": "T", "trace": {"timestamped": {
            "observations": observations,
            "progress": {"watermark": 5, "completeness": {"complete": {"through": 5}},
                "scope": "closed", "execution": "closed"},
            "lower_boundary": "authoritative-origin",
            "binding": binding(STAMPED, "plant-ms"),
            "missing": missing_members}}}));
        let path = write_trace(directory.path(), &format!("run-{index}.json"), &request);
        let output = run_file(&path);
        let document = result(&output);
        assert_eq!(document["disposition"]["stage"], "evaluate", "{document}");
        assert_eq!(
            document["ledger"]["format"], "filament.progress-ledger/1",
            "{document}"
        );

        // The same input built as a typed request: the envelope is the
        // file's, the selection is built by hand.
        let prepared = fsl_command::command::prepare_run(&path, Default::default())
            .unwrap_or_else(|error| panic!("the request prepares: {error:?}"));
        let fsl_command::command::PreparedRun::Clause(mut typed) = prepared else {
            panic!("a clause run");
        };
        let stamped: Vec<(u64, i64)> = [0_u64, 2, 5].into_iter().zip(levels).collect();
        typed.selection = typed_timestamped(&stamped, &missing);
        let selection = typed.selection.clone();
        let report = fsl_replay::spine::run_clause(*typed).expect("a report");
        let typed_bytes = fsl_replay::spine::write_clause_run_result_for(&report, Some(&selection))
            .expect("the report writes");
        let typed_document: Value = serde_json::from_slice(&typed_bytes).unwrap();
        assert_eq!(typed_document, document, "run {index}");
        assert_eq!(
            output.status.code(),
            Some(i32::from(report.disposition.exit_code()))
        );
        if indeterminate {
            let fsl_replay::spine::ClauseDisposition::Temporal(TemporalVerdict::Indeterminate {
                at,
                reason: TemporalIndeterminateReason::MissingValuation { position, .. },
            }) = &report.disposition
            else {
                panic!("{:?}", report.disposition);
            };
            assert_eq!((*at, *position), (0, 1));
            assert_eq!(output.status.code(), Some(11));
        } else {
            assert!(
                matches!(
                    report.disposition,
                    fsl_replay::spine::ClauseDisposition::Temporal(TemporalVerdict::Holds)
                ),
                "{:?}",
                report.disposition
            );
            assert_eq!(output.status.code(), Some(0));
        }
    }
}

/// FR-641-AC-2, FR-330-AC-17 (TC-1662): a violated finite record-sample trace
/// of an event-position clause, which the trace-subject route evaluates,
/// writes its counterexample, reads back and exits 10.
#[test]
#[trace("TC-1662", "FR-641-AC-2", "FR-330-AC-17")]
fn a_violated_finite_sample_trace_writes_its_counterexample() {
    let directory = tempfile::tempdir().unwrap();
    let request = trace_request(json!({"name": "E", "trace": {"finite": samples(&[0, 0, 0])}}));
    let output = run_file(&write_trace(directory.path(), "finite.json", &request));
    let document = result(&output);
    assert_eq!(output.status.code(), Some(10), "{document}");
    assert_eq!(
        document["disposition"]["temporal"]["kind"], "violated",
        "{document}"
    );
    assert!(
        find(&document, "counterexample").is_some(),
        "the violation carries its counterexample: {document}"
    );
    fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&document).unwrap())
        .expect("the violated record reads back");
}

/// The result document of clause `name` over one `progressed` sample at
/// level 0 under `profile` and `clock`, asserting `watermark`, complete
/// through `through`, with scope and execution `closure`.
fn progressed_run(
    directory: &Path,
    (name, profile, clock): (&str, &str, &str),
    (watermark, through, closure): (u64, u64, &str),
) -> Value {
    let request = trace_request(json!({"name": name, "trace": {"progressed": {
        "positions": samples(&[0]),
        "progress": {"watermark": watermark, "completeness": {"complete": {"through": through}},
            "scope": closure, "execution": closure},
        "lower_boundary": "authoritative-origin",
        "binding": binding(profile, clock)}}}));
    let document = result(&run_file(&write_trace(
        directory,
        &format!("progressed-{name}-{watermark}-{through}-{closure}.json"),
        &request,
    )));
    assert_eq!(
        document["ledger"]["format"], "filament.progress-ledger/1",
        "{document}"
    );
    document
}

/// FR-641-AC-2 (TC-1662): a `progressed` trace of a fixed-sample clause and of
/// an event-position clause reports the outcomes FR-504-AC-1 and AC-2 state,
/// and the result carries a `ledger` member. A fixed-sample deadline settles
/// through silence; an event-position deadline waits for the next event.
#[test]
#[trace("TC-1662", "FR-641-AC-2", "FR-504-AC-1", "FR-504-AC-2", "TC-1299")]
fn a_progressed_trace_settles_a_fixed_sample_deadline_through_silence() {
    let directory = tempfile::tempdir().unwrap();
    let fixed = ("S", SAMPLED, "sample");
    let events = ("E", EVENTS, "events");

    // FR-504-AC-1: watermark 0 leaves `eventually[0,3]` pending on tick 3.
    let pending = progressed_run(directory.path(), fixed, (0, 0, "open"));
    assert_eq!(
        pending["disposition"]["temporal"]["kind"], "pending-window",
        "{pending}"
    );
    assert_eq!(
        pending["disposition"]["temporal"]["deadline"], 3,
        "{pending}"
    );
    // Watermark 3, with no new observation, settles it as false whether the
    // scope is open or closed (the basis is TC-1299's own table).
    let open = progressed_run(directory.path(), fixed, (3, 3, "open"));
    assert_eq!(
        open["disposition"]["temporal"]["kind"], "violated",
        "{open}"
    );
    // The violation carries its counterexample over the samples, and the
    // record reads back.
    fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&open).unwrap())
        .expect("the violated record reads back");
    let closed = progressed_run(directory.path(), fixed, (3, 3, "closed"));
    assert_eq!(
        closed["disposition"]["temporal"]["kind"], "violated",
        "{closed}"
    );

    // FR-504-AC-2: under event position the same trace does not advance in
    // silence, and a closed scope reads it by false extension.
    let next = progressed_run(directory.path(), events, (3, 3, "open"));
    assert_eq!(
        next["disposition"]["temporal"]["kind"], "pending-next-event",
        "{next}"
    );
    let extended = progressed_run(directory.path(), events, (0, 0, "closed"));
    assert_eq!(
        extended["disposition"]["temporal"]["kind"], "violated",
        "{extended}"
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
        trace_request(temporal)
    };
    let first = result(&run_file(&write_trace(
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

    let regress = result(&run_file(&write_trace(
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

    let advance = result(&run_file(&write_trace(
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
        let request = trace_request(json!({"name": "Reaches", "trace": trace}));
        let output = run_file(&write_trace(
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

/// The refusal a trace kind the clause's profile does not read reports for the
/// request file `path`: stage `admit`, the reason, the profile, the expected
/// and the supplied kinds, and a message naming the clause and the profile.
fn assert_kind_mismatch(
    path: &Path,
    (name, profile, expected, supplied): (&str, &str, Value, &str),
) {
    let output = run_file(path);
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

/// FR-645-AC-3 (TC-1671): a trace kind the clause's profile does not read
/// refuses at stage `admit` naming the profile and the expected and supplied
/// kinds.
#[test]
#[trace("TC-1671", "FR-645-AC-3")]
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
    ]
    .into_iter()
    .enumerate()
    {
        let request = trace_request(json!({"name": name, "trace": trace}));
        let path = write_trace(directory.path(), &format!("kind-{index}.json"), &request);
        assert_kind_mismatch(&path, (name, profile, expected, supplied));
    }
}

/// FR-645-AC-4, FR-642-AC-5 (TC-1671, TC-1665): a timed clause over a trace
/// without stamps is the documented case: positions are not time.
#[test]
#[trace(
    "TC-1671",
    "FR-645-AC-4",
    "FR-642-AC-5",
    "FSpec-TC-1161",
    "FSpec-FR-541-AC-3"
)]
#[trace("TC-1665", "FR-642-AC-5")]
#[ignore = "QSL-2254/QSL-2136: timed clock kind and timed clause over a trace subject"]
fn a_timed_clause_over_untimed_samples_refuses_with_profile_and_kinds() {
    let directory = tempfile::tempdir().unwrap();
    let request = run_request(json!({"name": "Ack", "trace": {"finite": samples(&[0, 1])}}));
    let path = write(directory.path(), "kind-timed.json", &request);
    assert_kind_mismatch(&path, ("Ack", TIMED, json!(["timed"]), "finite"));
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
fn the_monitor_stream_equals_a_clause_run_over_each_prefix() {
    let directory = tempfile::tempdir().unwrap();
    let levels = [0, 1, 2, 1];
    let mut records: Vec<String> = levels
        .iter()
        .map(|level| json!({"observation": sample(*level)}).to_string())
        .collect();
    records.push(json!({"end": true}).to_string());
    std::fs::write(directory.path().join("records.jsonl"), records.join("\n")).unwrap();
    let request = json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": trace_clocks(),
        "clauses": [{"name": "Reaches"}, {"name": "Never"}],
        "trace": {"kind": "finite"},
        "records": "records.jsonl"}});
    let output = run_file(&write_trace(directory.path(), "monitor.json", &request));
    assert_eq!(output.status.code(), Some(10));
    let run = result(&output);
    assert_eq!(run["format"], "filament.monitor-run/1", "{run}");
    let events = run["events"].as_array().expect("events");

    for clause in ["Reaches", "Never"] {
        let mut status = Value::Null;
        let mut decided = 0;
        let batch_of = |upto: usize| {
            let prefix = trace_request(json!({"name": clause,
                "trace": {"finite": samples(&levels[..=upto])}}));
            result(&run_file(&write_trace(
                directory.path(),
                &format!("{clause}-{upto}.json"),
                &prefix,
            )))
        };
        for record in 0..levels.len() {
            if let Some(event) = events
                .iter()
                .find(|event| event["record"] == record && event["clause"] == clause)
            {
                status = event["disposition"].clone();
                decided = record;
                // Only `Never` settles, at record 2 (FR-644-AC-5).
                assert_eq!(
                    event["settled"],
                    clause == "Never" && record >= 2,
                    "{clause} settled at record {record}"
                );
                // `settlement` is present exactly when `settled` is true.
                if clause == "Never" && record >= 2 {
                    assert_eq!(
                        event["settlement"],
                        json!({"step": "3", "trigger": "decided",
                            "basis": batch_of(2)["basis"], "horizon": "2", "position": "0"}),
                        "{event}"
                    );
                } else {
                    assert!(event.get("settlement").is_none(), "{event}");
                }
            }
            // A settled result never changes (FR-643-AC-3), so the status
            // equals the clause run at the record that decided it; its
            // category equals the clause run over every sample so far.
            let batch = batch_of(record);
            assert_eq!(
                status["category"], batch["disposition"]["category"],
                "{clause} after record {record}"
            );
            assert_eq!(
                status,
                batch_of(decided)["disposition"],
                "{clause} as decided at record {decided}"
            );
        }
        let summary = run["summary"]["clauses"]
            .as_array()
            .and_then(|clauses| clauses.iter().find(|entry| entry["clause"] == clause))
            .expect("a summary entry");
        assert_eq!(summary["disposition"], status, "{clause} summary");
        assert_eq!(
            summary["settled"],
            clause == "Never",
            "{clause} settled in the summary"
        );
        if clause == "Never" {
            assert_eq!(summary["settlement"]["step"], "3", "{summary}");
            assert_eq!(summary["settlement"]["trigger"], "decided", "{summary}");
        } else {
            assert_eq!(
                summary["settlement"],
                json!({"step": "4", "trigger": "finish"}),
                "{summary}"
            );
        }
        assert!(
            run["summary"]["ledger"].is_null(),
            "a finite summary has no ledger: {run}"
        );
    }
}

/// FR-644-AC-7 (TC-2340): a `finite` stream of `E` over levels 0, 1, 2 gives
/// at record 2 an event settled with `B(3)`'s disposition and a settlement
/// naming the deciding reading; the earlier events carry none, and the
/// summary repeats the settlement and has no ledger.
#[test]
#[trace("TC-2340", "FR-644-AC-7")]
fn a_settled_event_carries_the_reading_that_decided_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut records = observations(&[0, 1, 2]);
    records.push(json!({"end": true}));
    let output = monitor_run(
        directory.path(),
        "settlement",
        monitor_request(json!([{"name": "E"}]), json!({"kind": "finite"})),
        &records,
    );
    let run = result(&output);
    let events = run["events"].as_array().expect("events");
    let batch = result(&run_file(&write_trace(
        directory.path(),
        "batch.json",
        &trace_request(json!({"name": "E", "trace": {"finite": samples(&[0, 1, 2])}})),
    )));
    let settlement = json!({"step": "3", "trigger": "decided",
        "basis": "decisive-witness", "horizon": "2", "position": "2"});
    for event in events {
        if event["record"] == 2 {
            assert_eq!(event["settled"], true, "{event}");
            assert_eq!(event["disposition"], batch["disposition"], "{event}");
            assert_eq!(event["disposition"]["category"], "success", "{event}");
            assert_eq!(event["basis"], "closed-scope", "{event}");
            assert!(
                event["disposition"].get("finite_window").is_none(),
                "{event}"
            );
            assert_eq!(event["settlement"], settlement, "{event}");
        } else {
            assert_eq!(event["settled"], false, "{event}");
            assert!(event.get("settlement").is_none(), "{event}");
        }
    }
    let summary = &run["summary"];
    assert_eq!(summary["clauses"][0]["settlement"], settlement, "{summary}");
    assert!(summary["ledger"].is_null(), "{summary}");
}

/// FR-640-AC-4, AC-6 (TC-1661): a malformed configuration refuses at stage
/// `request` naming the member, and a member named after a model binding is
/// refused as a reserved clock name.
#[test]
#[trace("TC-1661", "FR-640-AC-4", "FR-640-AC-6")]
#[ignore = "QSL-2254, QSL-2136: the unit's timed clause over a record trace subject waits on the timed clock configuration"]
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
        let output = run_file(&write_trace(
            directory.path(),
            &format!("malformed-trace-{index}.json"),
            &trace_request(temporal),
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
    let lasso = trace_request(json!({"name": "Reaches",
        "trace": {"lasso": {"prefix": samples(&[0]), "loop": samples(&[1, 2])}}}));
    let document = result(&run_file(&write_trace(
        directory.path(),
        "lasso.json",
        &lasso,
    )));
    assert_eq!(document["disposition"]["category"], "success", "{document}");
}

/// FR-641-AC-5 (TC-1663): a ledger read from a file and written back by a run
/// that adds nothing reproduces its bytes.
#[test]
#[trace("TC-1663", "FR-641-AC-5")]
fn a_ledger_round_trips_through_a_run_that_changes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let request = |ledger: Option<&str>| {
        let mut temporal = json!({"name": "E", "trace": {"progressed": {
            "positions": samples(&[0, 1]), "progress": progress(3),
            "lower_boundary": "authoritative-origin", "binding": binding(EVENTS, "events")}}});
        if let Some(ledger) = ledger {
            temporal["ledger"] = json!(ledger);
        }
        trace_request(temporal)
    };
    let first = result(&run_file(&write_trace(
        directory.path(),
        "first.json",
        &request(None),
    )));
    let bytes = serde_json::to_vec(&first["ledger"]).unwrap();
    std::fs::write(directory.path().join("ledger.json"), &bytes).unwrap();
    let again = result(&run_file(&write_trace(
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
fn an_in_process_trace_refusal_carries_no_request_pointer() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_trace(
        directory.path(),
        "lasso.json",
        &trace_request(json!({"name": "Reaches", "trace": {"lasso": {"prefix": [], "loop": []}}})),
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
    run_file(&write_trace(directory, &format!("{name}.json"), &request))
}

fn monitor_request(clauses: Value, trace: Value) -> Value {
    json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": trace_clocks(),
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
    records.push(json!({"end": true}));
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
    let request = trace_request(json!({"name": "T", "trace": {"timestamped": {
        "observations": observations, "progress": progress(5),
        "lower_boundary": "authoritative-origin", "binding": binding(STAMPED, "plant-ms")}}}));
    result(&run_file(&write_trace(directory, "batch.json", &request)))
}

/// FR-643-AC-3, FR-644-AC-1 (TC-1666, TC-1668): a settled violation is
/// reported once, at the observation that decides it, and never changes.
#[test]
#[trace("TC-1666", "FR-643-AC-3", "TC-1668", "FR-644-AC-1")]
fn a_settled_result_is_reported_once_and_never_changes() {
    let directory = tempfile::tempdir().unwrap();
    let mut records = observations(&[0, 1, 2, 1, 0]);
    records.push(json!({"end": true}));
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
fn a_checkpointed_stream_resumes_to_the_same_events() {
    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(
        json!([{"name": "Reaches"}, {"name": "Never"}]),
        json!({"kind": "finite"}),
    );
    let mut whole = observations(&[0, 1, 2, 1, 0]);
    whole.push(json!({"end": true}));
    let full = result(&monitor_run(
        directory.path(),
        "whole",
        request.clone(),
        &whole,
    ));

    let mut head = observations(&[0, 1]);
    head.push(json!({"checkpoint": "state.json"}));
    head.push(json!({"end": true}));
    monitor_run(directory.path(), "head", request.clone(), &head);
    let mut resumed_request = request;
    resumed_request["request"]["state"] = json!("state.json");
    let mut tail = observations(&[2, 1, 0]);
    tail.push(json!({"end": true}));
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
fn a_bad_record_is_an_event_and_a_bad_request_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(json!([{"name": "Never"}]), json!({"kind": "finite"}));
    let mut good = observations(&[0, 1, 2]);
    good.push(json!({"end": true}));
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
#[ignore = "QSL-2254, QSL-2136: the unit's timed clause over a record trace subject waits on the timed clock configuration"]
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

/// FR-767-AC-10 (TC-2196): a `native-monitor/1` request's `integer_division`
/// member supplies the law its program's `div` needs.
#[test]
#[trace("TC-2196", "FR-767-AC-10")]
#[ignore = "QSL-1989: the native-monitor/1 integer_division member is not implemented yet"]
fn a_monitor_request_supplies_the_integer_division_law() {
    let directory = tempfile::tempdir().unwrap();
    let mut records = observations(&[0, 1, 2]);
    records.push(json!({"end": true}));
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

/// FR-645, QSL-2138: a window aggregate over a record trace subject is read
/// only by the closed finite trace of the event-position profile. A
/// `progressed` sample trace is not that route, so the run refuses with a
/// typed refusal naming the gap, never an internal fault.
#[test]
fn a_window_aggregate_over_a_progressed_sample_trace_refuses_and_never_faults() {
    let directory = tempfile::tempdir().unwrap();
    let unit = trace_unit()
        + "temporal Count using e over (r: Reading) clock \"events\" on origin \
           { always[0,2] (count where holds(r.level > 1) over past[0,2] min_count 0 <= 5) }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    for trace in [
        json!({"progressed": {
            "positions": samples(&[0, 2, 3]),
            "progress": progress(3),
            "lower_boundary": "authoritative-origin",
            "binding": binding(EVENTS, "events")}}),
        json!({"finite": samples(&[0, 2, 3])}),
    ] {
        let path = directory.path().join("count.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&trace_request(json!({"name": "Count", "trace": trace}))).unwrap(),
        )
        .unwrap();
        let output = run_file(&path);
        let document = result(&output);
        let category = &document["disposition"]["category"];
        assert_ne!(category, "internal-failure", "{document}");
        if trace.get("progressed").is_some() {
            assert_eq!(output.status.code(), Some(20), "{document}");
            assert_eq!(
                find(&document, "reason"),
                Some(&json!("unsupported-window-aggregate")),
                "{document}"
            );
            let message = find(&document, "message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            assert!(message.contains("QSL-2138"), "{document}");
        }
    }
}

/// FR-642, FR-641-AC-4 (TC-1664): a `timed` trace of exact rational stamps
/// reaches FR-330 and a timed clause is answered: `Ack` holds when level 1
/// is read at stamp 0.
#[test]
#[trace("TC-1664", "FR-641-AC-4", "FR-642-AC-1")]
#[ignore = "QSL-2254: timed clock kind"]
fn a_timed_trace_answers_a_timed_clause() {
    let directory = tempfile::tempdir().unwrap();
    let request = run_request(json!({"name": "Ack", "trace": {"timed": {
        "prefix": [{"position": sample(1), "stamp": "0"}]}}}));
    let output = run_file(&write(directory.path(), "timed.json", &request));
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    let document = result(&output);
    assert_eq!(document["disposition"]["truth"], true, "{document}");
}

/// FR-644-AC-1, AC-4 (TC-1668, TC-1669): `command::monitor` hands the sink
/// each event before reading the next record, gives the events and summary
/// the batch form gives, and refuses a request naming no clause before any
/// record is read.
#[test]
#[trace("TC-1668", "FR-644-AC-1", "TC-1669", "FR-644-AC-4")]
fn command_monitor_streams_events_to_its_sink() {
    use fsl_command::command::{monitor, MonitorEvent, RunLimits};

    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(
        json!([{"name": "Reaches"}, {"name": "Never"}]),
        json!({"kind": "finite"}),
    );
    let path = write_trace(directory.path(), "stream.json", &request);
    let mut lines: Vec<String> = observations(&[0, 1, 2, 1, 0])
        .iter()
        .map(Value::to_string)
        .collect();
    lines.push(json!({"end": true}).to_string());
    let mut seen: Vec<MonitorEvent> = Vec::new();
    let handed = std::cell::Cell::new(0_usize);
    let records = lines.iter().enumerate().map(|(index, line)| {
        // The events of records 0 to 2 are with the sink before record 3 is read.
        if index == 3 {
            assert!(handed.get() >= 3, "{}", handed.get());
        }
        Ok(line.clone().into_bytes())
    });
    let summary = monitor(
        &path,
        records,
        &mut |event| {
            handed.set(handed.get() + 1);
            seen.push(event.clone());
        },
        RunLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the stream runs");
    assert_eq!(summary.exit_code, 10);
    assert!(
        seen.iter()
            .any(|event| event.as_value()["clause"] == "Never"
                && event.as_value()["category"] == "violation"
                && event.as_value()["record"] == 2),
        "{seen:?}"
    );
    assert_eq!(
        summary.as_value()["clauses"][1]["category"],
        "violation",
        "{}",
        summary.as_value()
    );

    let missing = write_trace(
        directory.path(),
        "missing.json",
        &monitor_request(json!([{"name": "Absent"}]), json!({"kind": "finite"})),
    );
    let mut nothing = 0;
    let refused = monitor(
        &missing,
        lines.iter().map(|line| Ok(line.clone().into_bytes())),
        &mut |_| nothing += 1,
        RunLimits::default(),
        &filament_exact::Cancel::new(),
    );
    let error = refused.expect_err("an unknown clause is refused");
    assert_eq!(error.category().exit_code(), 20);
    assert!(error.to_string().contains("Absent"), "{error}");
    assert_eq!(nothing, 0);
}

/// FR-644-AC-6 (TC-2324), refusal half: `{"end": "closed"}`, `{"end": "open"}`
/// and `{"through": X}` on a finite stream each give a `refusal` event naming
/// `end` or `through`, and leave the session unchanged.
#[test]
#[trace("TC-2324", "FR-644-AC-6")]
fn end_with_a_closure_and_through_are_refused_as_events() {
    let directory = tempfile::tempdir().unwrap();
    let request = monitor_request(json!([{"name": "Never"}]), json!({"kind": "finite"}));
    let mut clean = observations(&[0, 1, 2]);
    clean.push(json!({"end": true}));
    let clean = result(&monitor_run(
        directory.path(),
        "clean",
        request.clone(),
        &clean,
    ));
    let mut noisy = observations(&[0, 1]);
    noisy.push(json!({"end": "closed"}));
    noisy.push(json!({"through": 250}));
    noisy.push(json!({"end": "open"}));
    noisy.extend(observations(&[2]));
    noisy.push(json!({"end": true}));
    let noisy = result(&monitor_run(directory.path(), "noisy", request, &noisy));
    let members: Vec<(u64, String)> = noisy["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter(|event| event["category"] == "refusal")
        .map(|event| {
            (
                event["record"].as_u64().expect("a record"),
                event["disposition"]["member"]
                    .as_str()
                    .expect("a member")
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(
        members,
        vec![
            (2, "end".to_owned()),
            (3, "through".to_owned()),
            (4, "end".to_owned())
        ],
        "{noisy}"
    );
    assert_eq!(
        noisy["summary"]["clauses"], clean["summary"]["clauses"],
        "{noisy}"
    );
}

/// FR-643, FR-504 (TC-2321): a fixed-sample clause on a `Progressed` stream
/// reads under the session's latest assertion. After one sample, a
/// watermark-only assertion W 3 complete through 3 settles `S`
/// (`eventually[0,3]`) `violation` on its own, naming the horizon 3; W 2
/// leaves it pending.
#[test]
#[trace("TC-2321", "FR-643-AC-7", "FR-504")]
fn a_watermark_assertion_alone_settles_a_fixed_sample_deadline() {
    let directory = tempfile::tempdir().unwrap();
    for (name, watermark, settles) in [("deadline", 3, true), ("short", 2, false)] {
        let trace = json!({"kind": "progressed", "lower_boundary": "authoritative-origin",
            "binding": binding(SAMPLED, "sample")});
        let records = [
            json!({"observation": sample(0)}),
            json!({"progress": progress(watermark)}),
            json!({"end": true}),
        ];
        let output = monitor_run(
            directory.path(),
            name,
            monitor_request(json!([{"name": "S"}]), trace),
            &records,
        );
        let run = result(&output);
        let event = run["events"]
            .as_array()
            .and_then(|events| events.iter().rev().find(|event| event["clause"] == "S"))
            .expect("an event for S");
        assert_eq!(event["settled"], settles, "W {watermark}: {run}");
        if settles {
            assert_eq!(event["category"], "violation", "{event}");
            assert_eq!(event["record"], 1, "{event}");
            assert_eq!(event["settlement"]["trigger"], "decided", "{event}");
            assert_eq!(
                event["settlement"]["basis"], "decisive-counterexample",
                "{event}"
            );
            assert_eq!(event["settlement"]["horizon"], "3", "{event}");
        } else {
            assert!(event.get("settlement").is_none(), "{event}");
        }
    }
}
