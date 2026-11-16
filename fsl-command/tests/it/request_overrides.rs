// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640: the `clocks` request member supplied beside the request file, as the
//! driver's `--clocks` does, through every public compile and run entry.

use std::path::{Path, PathBuf};

use fsl_command::command::{self, RequestOverrides, RunLimits};
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

const PROGRAM: &[u8] = include_bytes!("../../../tests/fixtures/spine-run.native");
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
    record Reading { level: Int[0, 9]; }\n\
    temporal E using e over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n";

fn source() -> Value {
    json!({"file": "unit.native", "authority": "agent-ix", "identity": "test:overrides",
        "document": "Overrides", "formal_revision": 1})
}

fn clocks() -> Value {
    json!({"events": {"kind": "event_position", "sequence_authority": "events"}})
}

fn raw(value: &Value) -> Box<RawValue> {
    RawValue::from_string(value.to_string()).unwrap()
}

fn write(directory: &Path, name: &str, unit: &[u8], request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn compile_request(clocks: Option<Value>) -> Value {
    let mut request = json!({"models": [], "program": {"source": source()}});
    if let Some(clocks) = clocks {
        request["clocks"] = clocks;
    }
    json!({"format": "native-compile/1", "request": request})
}

/// A function-call run of the spine fixture; its `clocks` name no clock the
/// program declares, so the run refuses the selection by name.
fn run_request(clocks: Option<Value>) -> Value {
    let mut request = json!({"models": [], "program": {"source": source()}, "call": {
        "function": "lt", "arguments": [{"parameter": "a", "value": 2}, {"parameter": "b", "value": 5}],
        "accounting": {"work_units": 100_000}}});
    if let Some(clocks) = clocks {
        request["clocks"] = clocks;
    }
    json!({"format": "native-run/1", "request": request})
}

fn refusal_of(error: &command::RunError) -> Value {
    error.value().expect("refusal envelope")
}

fn unused() -> Value {
    json!({"spare": {"kind": "event_position", "sequence_authority": "spare"}})
}

/// FR-640: an override gives the package the member would give.
#[test]
#[trace("TC-2244", "FR-640-AC-13")]
fn compile_entries_take_the_override_as_the_member() {
    let directory = tempfile::tempdir().unwrap();
    let member = write(
        directory.path(),
        "member.json",
        UNIT.as_bytes(),
        &compile_request(Some(clocks())),
    );
    let bare = write(
        directory.path(),
        "bare.json",
        UNIT.as_bytes(),
        &compile_request(None),
    );
    let expected = command::compile(&member).unwrap();
    let supplied = raw(&clocks());
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..Default::default()
    };

    let bytes = command::compile_with_overrides(&bare, &overrides).unwrap();
    assert_eq!(bytes, expected);
    let outcome = command::compile_outcome_with_overrides(&bare, &overrides).unwrap();
    assert_eq!(outcome.package_bytes(), expected);
    let package: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(package.to_string().contains("event_position"), "{package}");

    // The path-only entry is the same call with no override.
    assert!(command::compile(&bare).is_err());
    assert_eq!(
        command::compile_with_overrides(&member, &RequestOverrides::default()).unwrap(),
        expected
    );
}

/// FR-640: the run entries read the override as the member.
#[test]
#[trace("TC-2244", "FR-640-AC-13")]
fn run_entries_take_the_override_as_the_member() {
    let directory = tempfile::tempdir().unwrap();
    let member = write(
        directory.path(),
        "member.json",
        PROGRAM,
        &run_request(Some(unused())),
    );
    let bare = write(directory.path(), "bare.json", PROGRAM, &run_request(None));
    let supplied = raw(&unused());
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..Default::default()
    };

    let expected = refusal_of(&command::run(&member).unwrap_err());
    assert_eq!(expected["cause"], "unused-clock-selection", "{expected}");
    for error in [
        command::run_with_overrides(&bare, &overrides).unwrap_err(),
        command::run_with_limits_and_overrides(&bare, RunLimits::default(), &overrides)
            .unwrap_err(),
    ] {
        assert_eq!(refusal_of(&error), expected);
    }
    // Preparation reads the override without executing: a well-formed one is
    // admitted, a malformed one refuses at stage `request`.
    assert!(command::prepare_run_with_overrides(&bare, RunLimits::default(), &overrides).is_ok());
    let malformed = raw(&json!({"spare": {"kind": "wall_clock"}}));
    let refused = command::prepare_run_with_overrides(
        &bare,
        RunLimits::default(),
        &RequestOverrides {
            clocks: Some(&malformed),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(refusal_of(&refused)["stage"], "request");
    // Without the override the bare request runs.
    assert_eq!(command::run(&bare).unwrap().exit_code, 0);
}

/// FR-640: both the member and the override refuse `invalid-request` at
/// stage `request`, naming the conflict, on every entry.
#[test]
#[trace("TC-2245", "FR-640-AC-14")]
fn a_member_and_an_override_together_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let compile = write(
        directory.path(),
        "compile.json",
        UNIT.as_bytes(),
        &compile_request(Some(clocks())),
    );
    let run = write(
        directory.path(),
        "run.json",
        PROGRAM,
        &run_request(Some(unused())),
    );
    let supplied = raw(&clocks());
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..Default::default()
    };
    let errors = [
        command::compile_with_overrides(&compile, &overrides).unwrap_err(),
        command::compile_outcome_with_overrides(&compile, &overrides)
            .err()
            .unwrap(),
        command::run_with_overrides(&run, &overrides).unwrap_err(),
        command::prepare_run_with_overrides(&run, RunLimits::default(), &overrides)
            .err()
            .unwrap(),
    ];
    for error in errors {
        let envelope = refusal_of(&error);
        assert_eq!(envelope["stage"], "request", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        let message = envelope["message"].as_str().unwrap();
        assert!(
            message.contains("clocks") && message.contains("--clocks"),
            "{message}"
        );
        assert_eq!(error.category().exit_code(), 20);
    }
}

/// FR-640: an override with an unknown kind or a repeated name refuses as the
/// member would, at stage `request`.
#[test]
#[trace("TC-2244", "FR-640-AC-13")]
fn a_malformed_override_refuses_as_a_malformed_member_does() {
    let directory = tempfile::tempdir().unwrap();
    let bare = write(
        directory.path(),
        "bare.json",
        UNIT.as_bytes(),
        &compile_request(None),
    );
    let unknown = raw(&json!({"events": {"kind": "wall_clock", "sequence_authority": "events"}}));
    let repeated = RawValue::from_string(
        r#"{"events":{"kind":"event_position","sequence_authority":"a"},"events":{"kind":"event_position","sequence_authority":"b"}}"#
            .to_owned(),
    )
    .unwrap();
    for (supplied, member) in [(unknown, "events"), (repeated, "events")] {
        let overrides = RequestOverrides {
            clocks: Some(&supplied),
            ..Default::default()
        };
        let error = command::compile_with_overrides(&bare, &overrides).unwrap_err();
        let envelope = refusal_of(&error);
        assert_eq!(envelope["stage"], "request", "{envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert!(
            envelope["message"].as_str().unwrap().contains(member),
            "{envelope}"
        );
    }
}
