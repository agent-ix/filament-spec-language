// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL qualification runner behavior (FR-350..FR-353).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;
use tempfile::TempDir;

fn fspec_dir() -> TempDir {
    xtask::published_fspec::directory("proposals/")
}

fn fixture_dir() -> (TempDir, PathBuf) {
    let fspec = fspec_dir();
    let fixtures = fspec
        .path()
        .join("proposals/conformance-corpus-v1/fixtures");
    (fspec, fixtures)
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(arguments)
        .output()
        .expect("xtask starts")
}

fn utf8(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("process output is UTF-8")
}

fn copy_fixture(temp: &TempDir) -> PathBuf {
    let root = temp.path();
    let (_fspec, fixtures) = fixture_dir();
    fs::create_dir_all(root.join("inputs")).unwrap();
    fs::copy(
        fixtures.join("complete-v1.json"),
        root.join("complete-v1.json"),
    )
    .unwrap();
    fs::copy(
        fixtures.join("inputs/native.native"),
        root.join("inputs/native.native"),
    )
    .unwrap();
    root.to_path_buf()
}

fn write_canonical(path: &Path, value: &Value) {
    let bytes = serde_json_canonicalizer::to_vec(value).expect("canonical corpus");
    fs::write(path, bytes).expect("write canonical corpus");
}

#[test]
#[trace("TC-880", "FR-353-AC-1")]
fn qualify_runs_fspec_formatter_fixture_and_reports_a_qualified_scope() {
    let (_fspec, fixture) = fixture_dir();
    let output = run(&["qualify", "--corpus", fixture.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(0), "{}", utf8(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["scope"], serde_json::json!(["V1-TOOL-004"]));
    assert_eq!(report["counts"]["passed"], 1);
    assert_eq!(report["coverage"]["passed"], 1);
    assert_eq!(report["coverage"]["in_scope"], 1);
    assert_eq!(
        report["capabilities"][0]["outcome"], "passed",
        "the fixture's formatter vector is executed through FSL"
    );
}

/// FR-353-AC-3: a caller-selected passing capability is qualified under its
/// named scope, without claiming complete-v1; FR-567 still gates the same run
/// on the lifecycle matrix and linked-backend parity part.
#[test]
#[trace("TC-881", "FR-353-AC-3")]
fn qualify_scoped_passing_capability_keeps_all_gate_parts() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&corpus_file).unwrap()).unwrap();

    // TC-880 step 2's control corpus: retain the real passing formatter vector,
    // and add a second capability whose real formatter result disagrees with
    // its deliberately changed expected value. First prove that capability
    // fails in the default scope, then select only the passing capability.
    let mut failing_coverage = value["coverage"]["primary"][0].clone();
    failing_coverage["capability_id"] = Value::String("V1-TOOL-005".into());
    value["coverage"]["primary"]
        .as_array_mut()
        .unwrap()
        .push(failing_coverage);
    let mut failing_rollup = value["coverage"]["consumer_rollups"][0].clone();
    failing_rollup["capability_ids"] = serde_json::json!(["V1-TOOL-004", "V1-TOOL-005"]);
    value["coverage"]["consumer_rollups"] = serde_json::json!([failing_rollup]);
    let mut failing_vector = value["vectors"][0].clone();
    failing_vector["id"] = Value::String("V1-TOOL-005-failing-format".into());
    failing_vector["capability_id"] = Value::String("V1-TOOL-005".into());
    failing_vector["expected"]["value"] =
        Value::String("deliberately incorrect expected formatter output\n".into());
    value["vectors"].as_array_mut().unwrap().push(failing_vector);
    write_canonical(&corpus_file, &value);

    let control = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);
    assert_eq!(control.status.code(), Some(1), "{}", utf8(&control.stderr));
    let control_report: Value = serde_json::from_slice(&control.stdout).expect("JSON report");
    assert_eq!(control_report["verdict"], "not-qualified");
    assert_eq!(control_report["scope"], serde_json::json!(["V1-TOOL-004", "V1-TOOL-005"]));
    assert_eq!(control_report["capabilities"][1]["outcome"], "failed");

    let output = run(&[
        "qualify",
        "--corpus",
        corpus.to_str().unwrap(),
        "--capability",
        "V1-TOOL-004",
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", utf8(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["scope"], serde_json::json!(["V1-TOOL-004"]));
    assert!(report["qualification"].is_null());
    assert!(!utf8(&output.stdout).contains("complete-v1"));
    let parts = report["parts"].as_array().expect("the gate parts");
    let lifecycle = parts
        .iter()
        .find(|part| part["part"] == "lifecycle-matrix")
        .expect("the lifecycle matrix remains in the verdict");
    let parity = parts
        .iter()
        .find(|part| part["part"] == "parity")
        .expect("backend parity remains in the verdict");
    assert_eq!(lifecycle["result"], "passed");
    assert_eq!(parity["result"], "no-backend");
}

#[test]
#[trace("TC-875", "FR-350-AC-1")]
fn qualify_compiles_a_language_vector_through_the_public_entry_point() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let source_path = corpus.join("inputs/native.native");
    let source_bytes = fs::read(&source_path).unwrap();
    let identity = fsl_foundation::SourceIdentity::new("agent-ix", "inputs/native.native");
    let compiled = fsl_replay::compile_package(
        fsl_replay::CompileUnit {
            source: identity,
            package_name: "qualification".to_owned(),
            path: "inputs/native.native",
            bytes: &source_bytes,
        },
        std::iter::empty::<&[u8]>(),
        &fsl_replay::DependencyInput::default(),
        fsl_replay::StageLimits::default(),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("the compile fixture is admitted");
    let package: Value = serde_json::from_slice(compiled.bytes()).unwrap();
    let mut value: Value =
        serde_json::from_slice(&fs::read(corpus.join("complete-v1.json")).unwrap()).unwrap();
    value["coverage"]["primary"][0]["applicability"]["consumer_types"] =
        serde_json::json!(["language"]);
    value["vectors"][0]["applicability"]["consumer_types"] = serde_json::json!(["language"]);
    value["vectors"][0]["subject"]["consumer_type"] = Value::String("language".into());
    value["vectors"][0]["subject"]["operation"] = Value::String("compile".into());
    value["vectors"][0]["expected"]["value"] = package;
    write_canonical(&corpus.join("complete-v1.json"), &value);

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(0), "{}", utf8(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["counts"]["passed"], 1);
    assert_eq!(report["capabilities"][0]["outcome"], "passed");
}

#[test]
#[trace("TC-881", "FR-353-AC-4")]
fn qualify_refuses_a_schema_invalid_corpus_before_execution() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&corpus_file).unwrap()).unwrap();
    value["vectors"][0]
        .as_object_mut()
        .unwrap()
        .remove("expected");
    fs::write(&corpus_file, serde_json::to_vec(&value).unwrap()).unwrap();

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "a refusal has no verdict or report"
    );
    assert!(utf8(&output.stderr).contains("expected"));
}

#[test]
#[trace("TC-876", "FR-350-AC-4", "FR-353-AC-2")]
fn missing_vector_input_is_reported_as_tool_failure_and_exit_one() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&corpus_file).unwrap()).unwrap();
    value["vectors"][0]["input"]["files"][0]["path"] =
        Value::String("inputs/missing.native".into());
    write_canonical(&corpus_file, &value);

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("completed report");
    assert_eq!(report["verdict"], "not-qualified");
    assert_eq!(report["counts"]["tool-failure"], 1);
    assert_eq!(report["capabilities"][0]["outcome"], "tool-failure");
    assert_eq!(
        report["capabilities"][0]["vectors"][0]["outcome"],
        "tool-failure"
    );
}

#[test]
#[trace("TC-881", "FR-353-AC-4")]
fn duplicate_object_keys_refuse_before_a_report() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let bytes = fs::read(&corpus_file).unwrap();
    let source = String::from_utf8(bytes).unwrap();
    let duplicate = source.replacen(
        "\"contract_version\":",
        "\"contract_version\":\"filament.conformance-corpus/v1\",\"contract_version\":",
        1,
    );
    fs::write(&corpus_file, duplicate).unwrap();

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(utf8(&output.stderr).contains("duplicate object key contract_version"));
}
