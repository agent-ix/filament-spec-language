// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 Behavior 16: every run, compile and export route applies each
//! caller override member or refuses it `invalid-request` at stage
//! `request`, naming the member, and writes no file.

use std::collections::BTreeMap;
use std::path::Path;

use fsl_command::command::{self, RequestOverrides, RunError};
use fsl_foundation::{AccountingSetting, Setting};
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use crate::negotiation::Fr290;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function one using v(): Integer pure { 1 }\n";

fn source() -> Value {
    json!({"file": "unit.native", "authority": "agent-ix", "identity": "test:override-members",
        "document": "OverrideMembers", "formal_revision": 1})
}

fn run_envelope(request: Value) -> Value {
    json!({"format": "native-run/1", "request": request})
}

/// A request over the source program.
fn from_source(member: &str, value: Value) -> Value {
    let mut request = json!({"models": [], "program": {"source": source()}});
    request[member] = value;
    run_envelope(request)
}

/// A request over a selected package. No file is at the selection: an
/// override is refused before it is opened.
fn from_package(member: &str, value: Value) -> Value {
    let mut request = json!({"models": [], "package": {"directory": "handoff"}});
    request[member] = value;
    run_envelope(request)
}

fn call() -> Value {
    json!({"function": "one", "arguments": []})
}

fn clause() -> Value {
    json!({"selection": {"temporal": {"name": "Low", "trace": {"finite": []}}},
        "observations": {}})
}

fn export() -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["Low"]})
}

fn evidence() -> Value {
    json!({"package": {}, "obligation": "Low", "tool": "lean", "artifact": "proof.lean"})
}

/// Every `run` route that writes no file, with the name its refusal gives.
fn run_routes() -> Vec<(&'static str, Value)> {
    let synthesis = json!({"declaration": "Pick"});
    let temporal_item = json!({"temporal": {"name": "Low", "trace": {"finite": []}}});
    vec![
        ("synthesis", from_source("synthesis", synthesis.clone())),
        ("package synthesis", from_package("synthesis", synthesis)),
        (
            "model_based_test",
            from_source("model_based_test", json!({})),
        ),
        (
            "refinement_transfer",
            from_source(
                "refinement_transfer",
                json!({"refinement": "R", "abstract": {"model": "M", "initial": []},
                    "concrete": {"model": "M", "initial": []}, "clauses": [], "observations": {}}),
            ),
        ),
        ("call", from_source("call", call())),
        ("package call", from_package("call", call())),
        ("clause", from_source("clause", clause())),
        ("package clause", from_package("clause", clause())),
        (
            "tl_lowering",
            from_source("tl_lowering", json!({"clause": "Low"})),
        ),
        (
            "package tl_lowering",
            from_package("tl_lowering", json!({"clause": "Low"})),
        ),
        (
            "items",
            from_source("items", json!({"selections": [temporal_item]})),
        ),
        (
            "items",
            from_source(
                "items",
                json!({"selections": [{"value_invariants": {"snapshot": "s"}}],
                    "observations": {"s": "s.json"}}),
            ),
        ),
        (
            "verification_plan",
            from_source("verification_plan", json!({})),
        ),
        ("version", json!({"argv": ["version"]})),
        ("check", json!({"argv": ["check", "unit.native"]})),
        (
            "export_evidence",
            from_source("export_evidence", evidence()),
        ),
    ]
}

fn write(directory: &Path, request: &Value) -> std::path::PathBuf {
    std::fs::write(directory.join("unit.native"), UNIT).unwrap();
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn run_error(request: &Value, overrides: &RequestOverrides<'_>) -> Box<RunError> {
    let directory = tempfile::tempdir().unwrap();
    let path = write(directory.path(), request);
    match command::run_with_overrides(&path, overrides, &Fr290) {
        Ok(result) => panic!("{request} ran: {}", result.value),
        Err(error) => error,
    }
}

/// The refusal names `member` and `route`, at stage `request`, exit 20.
fn assert_unapplied(error: &RunError, route: &str, member: &str) {
    let envelope = error.value().expect("a refusal envelope");
    assert_eq!(envelope["code"], "invalid-request", "{route}: {envelope}");
    assert_eq!(envelope["stage"], "request", "{route}: {envelope}");
    assert_eq!(error.category().exit_code(), 20, "{route}: {envelope}");
    let expected = format!("the {member} override cannot be applied to a {route} run");
    assert!(
        envelope["message"].as_str().unwrap().contains(&expected),
        "expected `{expected}`: {envelope}"
    );
}

/// FR-255-AC-36: no route but a compile and an export writes a file, so each
/// other route, from source and from a package, refuses an output override.
#[test]
#[trace("FR-255-AC-36")]
fn every_route_that_writes_no_file_refuses_an_output_override() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("out.json");
    let overrides = RequestOverrides {
        output: Some(&output),
        ..Default::default()
    };
    for (route, request) in run_routes() {
        assert_unapplied(&run_error(&request, &overrides), route, "output");
    }
    // Both envelopes hold every required member, so the override is the only
    // thing refused: it is decided before any member's content is read.
    let monitor = json!({"models": [], "program": {"source": source()}, "clauses": [],
        "trace": {"kind": "finite"}, "records": "records.jsonl"});
    let replay: serde_json::Map<String, Value> = fsl_command::replay_request::MEMBERS
        .iter()
        .map(|member| ((*member).to_owned(), Value::Null))
        .collect();
    for (route, request) in [
        (
            "monitor",
            json!({"format": "native-monitor/1", "request": monitor}),
        ),
        (
            "replay",
            json!({"format": "native-replay/1", "request": replay}),
        ),
    ] {
        assert_unapplied(&run_error(&request, &overrides), route, "output");
    }
    assert!(!output.exists());
}

/// FR-255-AC-35: a route that reads no clocks refuses a clocks override.
#[test]
#[trace("FR-255-AC-35")]
fn a_route_that_reads_no_clocks_refuses_a_clocks_override() {
    let clocks = RawValue::from_string(
        json!({"events": {"kind": "event_position", "sequence_authority": "events"}}).to_string(),
    )
    .unwrap();
    let overrides = RequestOverrides {
        clocks: Some(&clocks),
        ..Default::default()
    };
    for (route, request) in [
        (
            "synthesis",
            from_source("synthesis", json!({"declaration": "Pick"})),
        ),
        (
            "model_based_test",
            from_source("model_based_test", json!({})),
        ),
        ("package clause", from_package("clause", clause())),
        ("export", from_source("export", export())),
        ("package export", from_package("export", export())),
        ("check", json!({"argv": ["check", "unit.native"]})),
        ("version", json!({"argv": ["version"]})),
    ] {
        assert_unapplied(&run_error(&request, &overrides), route, "clocks");
    }
}

/// FR-255-AC-37, FR-255-AC-38: a setting no part of a route reads is refused
/// by name: `export.output_bytes` on a call and a clause run, a source stage
/// setting on a package run, an observation or stage setting on an export of
/// evidence, a `check` and a `version`.
#[test]
#[trace("FR-255-AC-37", "FR-255-AC-38")]
fn a_route_refuses_a_setting_it_never_reads() {
    for (setting, route, request) in [
        (
            Setting::ExportOutputBytes,
            "call",
            from_source("call", call()),
        ),
        (
            Setting::ExportOutputBytes,
            "clause",
            from_source("clause", clause()),
        ),
        (
            Setting::ProgressCoveredPositions,
            "call",
            from_source("call", call()),
        ),
        (
            Setting::S1Tokens,
            "package call",
            from_package("call", call()),
        ),
        (
            Setting::S1Tokens,
            "package synthesis",
            from_package("synthesis", json!({"declaration": "Pick"})),
        ),
        (
            Setting::S1Tokens,
            "check",
            json!({"argv": ["check", "unit.native"]}),
        ),
        (Setting::S1Tokens, "version", json!({"argv": ["version"]})),
        (
            Setting::S3Nodes,
            "export_evidence",
            from_source("export_evidence", evidence()),
        ),
    ] {
        let limits = BTreeMap::from([(setting, 1)]);
        let overrides = RequestOverrides {
            limits: Some(&limits),
            ..Default::default()
        };
        assert_unapplied(&run_error(&request, &overrides), route, setting.name());
    }
}

/// FR-255-AC-38: no export, `check` or `version` evaluates under an
/// accounting counter, so each refuses one.
#[test]
#[trace("FR-255-AC-37", "FR-255-AC-38")]
fn a_route_that_charges_no_counter_refuses_one() {
    let accounting = BTreeMap::from([(AccountingSetting::WorkUnits, 5)]);
    let overrides = RequestOverrides {
        accounting: Some(&accounting),
        ..Default::default()
    };
    for (route, request) in [
        ("export", from_source("export", export())),
        ("package export", from_package("export", export())),
        (
            "export_evidence",
            from_source("export_evidence", evidence()),
        ),
        ("check", json!({"argv": ["check", "unit.native"]})),
        ("version", json!({"argv": ["version"]})),
    ] {
        assert_unapplied(
            &run_error(&request, &overrides),
            route,
            "accounting.work_units",
        );
    }
}

/// FR-255-AC-38: a compile evaluates nothing, so it refuses an accounting
/// counter and an observation setting; a model unit compile reads only its
/// own declarations, so it refuses a stage setting.
#[test]
#[trace("FR-255-AC-38")]
fn a_compile_refuses_what_it_never_reads() {
    let directory = tempfile::tempdir().unwrap();
    let path = write(
        directory.path(),
        &json!({"format": "native-compile/1",
            "request": {"models": [], "program": {"source": source()}}}),
    );
    command::compile(&path).expect("the control compile completes");
    let accounting = BTreeMap::from([(AccountingSetting::WorkUnits, 5)]);
    let observation = BTreeMap::from([(Setting::ObservationInputBytes, 1)]);
    for (overrides, member) in [
        (
            RequestOverrides {
                accounting: Some(&accounting),
                ..Default::default()
            },
            "accounting.work_units",
        ),
        (
            RequestOverrides {
                limits: Some(&observation),
                ..Default::default()
            },
            "observation.input_bytes",
        ),
    ] {
        let error = command::compile_with_overrides(&path, &overrides).unwrap_err();
        assert_unapplied(&error, "compile", member);
    }
}

/// A source `synthesis` and a direct `model_based_test` read no clocks, so a
/// request's own `clocks` is refused naming it.
#[test]
#[trace("FR-255-AC-35")]
fn a_request_clocks_member_on_a_route_that_reads_none_is_refused() {
    for (member, value) in [
        ("synthesis", json!({"declaration": "Pick"})),
        (
            "model_based_test",
            json!({"clause": "Low", "seed": 0, "max_cases": 1}),
        ),
    ] {
        let mut request = from_source(member, value);
        request["request"]["clocks"] =
            json!({"events": {"kind": "event_position", "sequence_authority": "events"}});
        let error = run_error(&request, &RequestOverrides::default());
        let envelope = error.value().unwrap();
        assert_eq!(envelope["code"], "invalid-request", "{member}: {envelope}");
        assert_eq!(error.category().exit_code(), 20, "{envelope}");
        assert!(envelope.to_string().contains("clocks"), "{envelope}");
    }
}
