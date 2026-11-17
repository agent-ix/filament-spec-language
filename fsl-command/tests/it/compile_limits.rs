// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-869-AC-1 and AC-2: compile accepts caller intake and stage limits.

use std::path::Path;

use fsl_command::command::{self, RequestOverrides, RunLimits};
use fsl_replay::spine::SpineLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const PROGRAM: &[u8] = include_bytes!("../../../tests/fixtures/spine-compile.native");

fn write_request(directory: &Path, source: &[u8]) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), source).unwrap();
    let path = directory.join("compile.json");
    let request = json!({"format":"native-compile/1", "request": {
        "models": [], "program": {"source": {"file":"program.native",
            "authority":"agent-ix", "identity":"test:fr-869", "document":"Spine",
            "formal_revision":1}}
    }});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn refusal(error: &command::RunError) -> Value {
    error.value().expect("refusal envelope")
}

#[test]
#[trace("TC-2461", "FR-869-AC-1")]
fn compile_request_byte_limit_is_caller_configurable() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_request(directory.path(), PROGRAM);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.resize(1_048_577, b' ');
    std::fs::write(&path, &bytes).unwrap();

    let error = command::compile_outcome(&path)
        .err()
        .expect("default limit refuses");
    let failure = refusal(&error);
    assert_eq!(failure["details"]["limit"], "request_bytes", "{failure}");
    assert_eq!(failure["details"]["maximum"], 1_048_576, "{failure}");

    let outcome = command::compile_outcome_with_limits_and_overrides(
        &path,
        RunLimits {
            request_bytes: bytes.len(),
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    )
    .unwrap();
    assert!(!outcome.package_bytes().is_empty());
}

#[test]
#[trace("TC-2461", "FR-869-AC-2")]
fn compile_stage_and_output_limits_come_from_request_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let nested = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function seven using v(): Int[0, 9] pure {{ {}7{} }}\n",
        "(".repeat(6_000),
        ")".repeat(6_000)
    );
    let path = write_request(directory.path(), nested.as_bytes());
    let error = command::compile_outcome(&path)
        .err()
        .expect("default limit refuses");
    let failure = refusal(&error);
    assert_eq!(failure["code"], "stage_limit_exceeded", "{failure}");
    assert!(
        failure["message"].as_str().unwrap().contains("s1.nodes"),
        "{failure}"
    );

    let mut limits = SpineLimits::default();
    limits.source.tokens = 1_000_000;
    limits.source.nodes = 1_000_000;
    limits.source.work_units = 1_000_000;
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .unwrap();
    let mut limits = limits;
    limits.package.output_bytes = outcome.package_bytes().len() as u64 - 1;
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let error =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .err()
            .expect("output limit refuses");
    let failure = refusal(&error);
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .contains("emit.output_bytes"),
        "{failure}"
    );
}

/// Trace: FR-865-AC-5.
#[test]
fn compile_outcome_retains_its_admitted_source_for_rendering() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_request(directory.path(), PROGRAM);
    let outcome = command::compile_outcome(&path).unwrap();

    assert_eq!(outcome.sources().len(), 1);
    let source = outcome.sources().as_slice().first().expect("one source");
    assert_eq!(source.path(), "program.native");
    assert_eq!(source.text().as_bytes(), PROGRAM);
    let rendered = fsl_command::render_sources(outcome.sources());
    assert!(format!("{rendered:?}").contains("source_count: 1"));
}
