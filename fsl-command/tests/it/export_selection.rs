// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026/FR-1015/FR-1022: required export selection shares one bounded intake.

use std::path::Path;

use fsl_command::command::{
    export_with_limits_and_overrides, run_with_limits, ExportSelection, RequestOverrides, RunLimits,
};
use fsl_replay::Operation;
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn run(path: &Path, selection: ExportSelection, limits: RunLimits) -> Value {
    let (exit, document) = match export_with_limits_and_overrides(
        path,
        selection,
        limits,
        &RequestOverrides::default(),
    ) {
        Ok(result) => (
            result.exit_code,
            serde_json::to_value(result.outcome.expect("selected export lifecycle outcome"))
                .unwrap(),
        ),
        Err(error) => (
            error.category().exit_code(),
            serde_json::to_value(error.outcome_document(Operation::Export)).unwrap(),
        ),
    };
    json!({"exit": exit, "document": document})
}

fn request(source: &Path) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": source, "authority": "export-contracts",
            "identity": "contracts:bin-capacity", "document": "BinCapacity", "formal_revision": 1}},
        "export": {"profile": "filament.output.tla2/v1", "obligations": ["Capacity"]}}})
}

/// Required selection is checked before opening a program or producing a target.
#[test]
#[trace("FR-026", "FR-1015", "FR-1022", "TC-104")]
fn wrong_or_missing_required_selection_refuses_before_source_io() {
    let directory = tempfile::tempdir().unwrap();
    for (selection, body, required) in [
        (ExportSelection::Ordinary, json!({}), "export"),
        (
            ExportSelection::Evidence,
            json!({"export": {}}),
            "export_evidence",
        ),
        (
            ExportSelection::Ordinary,
            json!({"export_evidence": {}}),
            "export",
        ),
    ] {
        let path = directory.path().join("request.json");
        let mut input = request(&directory.path().join("absent.native"));
        input["request"].as_object_mut().unwrap().remove("export");
        input["request"]
            .as_object_mut()
            .unwrap()
            .extend(body.as_object().unwrap().clone());
        std::fs::write(&path, input.to_string()).unwrap();
        let output = run(&path, selection, RunLimits::default());
        assert_eq!(output["exit"], 20, "{output}");
        assert!(output["document"]["result"].is_null(), "{output}");
        assert!(output.to_string().contains(required), "{output}");
        assert!(!output.to_string().contains("No such file"), "{output}");
    }
}

/// Raw duplicate and closed-envelope defects precede both selection and source I/O.
#[test]
#[trace("FR-026", "TC-104")]
fn raw_admission_refuses_before_required_selection() {
    let directory = tempfile::tempdir().unwrap();
    for (bytes, expected) in [
        (
            br#"{"format":"native-run/1","request":{"models":[],"models":[{}]}}"#.as_slice(),
            "duplicate object key models",
        ),
        (
            br#"{"format":"native-run/1","request":{},"extra":true}"#.as_slice(),
            "extra",
        ),
    ] {
        let path = directory.path().join("request.json");
        std::fs::write(&path, bytes).unwrap();
        let output = run(&path, ExportSelection::Ordinary, RunLimits::default());
        assert_eq!(output["exit"], 20, "{output}");
        assert!(output["document"]["result"].is_null());
        assert!(output.to_string().contains(expected), "{output}");
    }
}

/// A source with no selected obligation still reaches the ordinary export record.
#[test]
#[trace("FR-026", "FR-1015", "TC-104")]
fn a_regular_export_reaches_its_typed_obligation_record() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("unit.native");
    std::fs::write(&source, "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction Ready using v(): Boolean pure { true }\n").unwrap();
    let path = directory.path().join("request.json");
    std::fs::write(&path, request(&source).to_string()).unwrap();
    let output = run(&path, ExportSelection::Ordinary, RunLimits::default());
    assert_eq!(output["exit"], 21, "{output}");
    assert_eq!(output["document"]["result"]["kind"], "exported", "{output}");
    assert!(output.to_string().contains("not-an-obligation"), "{output}");
    assert!(output.to_string().contains("Capacity"), "{output}");
}

/// Deep selected values, conversion inputs and the root-owned control all
/// return typed refusals from a real child process under the byte ceiling.
#[test]
#[trace("FR-026-AC-3", "TC-104")]
fn deeply_nested_export_inputs_refuse_without_aborting() {
    use std::process::{Command, Stdio};
    const INPUT: &str = "FSL_DEEP_EXPORT_CHILD_INPUT";
    const OUTPUT: &str = "FSL_DEEP_EXPORT_CHILD_OUTPUT";
    if let Some(input) = std::env::var_os(INPUT) {
        let result = run(
            Path::new(&input),
            ExportSelection::Ordinary,
            RunLimits::default(),
        );
        std::fs::write(std::env::var_os(OUTPUT).unwrap(), result.to_string()).unwrap();
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let deep = format!("{}0{}", "[".repeat(16_000), "]".repeat(16_000));
    let export = r#"{"profile":"filament.output.tla2/v1","obligations":["Low"]}"#;
    let selected =
        format!(r#"{{"profile":"filament.output.tla2/v1","obligations":["Low"],"z":{deep}}}"#);
    let cases = [
        (format!(r#""export":{selected}"#), "/request/export/z"),
        (
            format!(r#""z":{selected}"#),
            "invalid request: unknown field `z`",
        ),
        (
            format!(r#""export":{export},"package":{{"z":{deep}}}"#),
            "/request/package/source_package",
        ),
        (format!(r#""export":{export},"z":{deep}"#), "unknown field"),
        (
            format!(
                r#""export":{{"profile":"filament.output.tla2/v1","obligations":["Low"],"protocols":[{{"protocol":"P","roles":{{"R":{{"type":"T","selection":{deep}}}}}}}]}}"#
            ),
            "/request/export/protocols/0",
        ),
        (
            format!(
                r#""export":{{"profile":"filament.output.tla2/v1","obligations":["Low"],"universes":[{{"authority":"a","identity":{deep}}}]}}"#
            ),
            "/request/export/universes/0",
        ),
    ];
    for (index, (members, expected)) in cases.into_iter().enumerate() {
        let bytes = format!(r#"{{"format":"native-run/1","request":{{"models":[],{members}}}}}"#);
        assert!(bytes.len() < RunLimits::default().request_bytes);
        let input = directory.path().join(format!("deep-{index}.json"));
        let output = directory.path().join(format!("result-{index}.json"));
        std::fs::write(&input, bytes).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "export_selection::deeply_nested_export_inputs_refuse_without_aborting",
                "--test-threads=1",
            ])
            .env(INPUT, input)
            .env(OUTPUT, &output)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "case {index} must return a typed refusal, not abort: {child:?}"
        );
        let result: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
        assert_eq!(result["exit"], 20, "case {index}: {result}");
        assert_eq!(result["document"]["category"], "refusal", "{result}");
        assert!(result["document"]["result"].is_null(), "{result}");
        assert!(!result["document"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(
            result.to_string().contains(expected),
            "case {index}: {result}"
        );
        if index == 0 {
            assert_eq!(
                result["document"]["diagnostics"][0]["code"],
                "invalid_runtime_input"
            );
            assert_eq!(
                result["document"]["diagnostics"][0]["cause"],
                "unknown-member"
            );
        }
    }
}

/// The run and export entries read the whole request tree before they name a
/// request's defects. A deep unknown member beside absent required members
/// refuses at each, on a 2 MiB stack, from the everyday bar of 1,000 levels up
/// to 100,000. It runs in a child process, so an overflow fails this test
/// alone.
#[test]
#[trace("FR-026-AC-3", "TC-104")]
fn deeply_nested_requests_refuse_at_every_entry_on_a_small_stack() {
    use std::process::{Command, Stdio};
    const INPUT: &str = "FSL_DEEP_ENTRY_CHILD_INPUT";
    const OUTPUT: &str = "FSL_DEEP_ENTRY_CHILD_OUTPUT";
    if let Some(input) = std::env::var_os(INPUT) {
        let output = std::env::var_os(OUTPUT).unwrap();
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                let path = Path::new(&input);
                let export = run(path, ExportSelection::Ordinary, RunLimits::default());
                let error = run_with_limits(path, RunLimits::default(), &crate::negotiation::Fr290)
                    .expect_err("run refuses");
                let observed = json!({
                    "export": export,
                    "run": {"exit": error.category().exit_code(), "cause": error.cause.to_string()},
                });
                std::fs::write(output, observed.to_string()).unwrap();
            })
            .expect("spawn the small-stack control")
            .join()
            .expect("the entries return");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    for depth in [1_000, 16_000, 100_000] {
        let deep = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        let bytes = format!(r#"{{"format":"native-run/1","request":{{"models":[],"z":{deep}}}}}"#);
        assert!(bytes.len() < RunLimits::default().request_bytes);
        let input = directory.path().join(format!("request-{depth}.json"));
        let output = directory.path().join(format!("result-{depth}.json"));
        std::fs::write(&input, bytes).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "export_selection::deeply_nested_requests_refuse_at_every_entry_on_a_small_stack",
                "--test-threads=1",
            ])
            .env(INPUT, &input)
            .env(OUTPUT, &output)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "depth {depth} must return a typed refusal, not abort: {child:?}"
        );
        let observed: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
        let export = &observed["export"];
        assert_eq!(export["exit"], 20, "depth {depth}: {export}");
        assert_eq!(export["document"]["category"], "refusal", "{export}");
        assert!(export["document"]["result"].is_null(), "{export}");
        assert!(export.to_string().contains("unknown field `z`"), "{export}");
        assert_eq!(observed["run"]["exit"], 20, "depth {depth}: {observed}");
        assert!(
            observed["run"]["cause"]
                .as_str()
                .is_some_and(|cause| cause.contains("recursion limit exceeded")),
            "depth {depth}: {observed}"
        );
    }
}

/// An explicit streamed operand receives the same bounded, required-selection intake.
#[cfg(unix)]
#[test]
#[trace("FR-026", "FR-1015", "TC-104")]
fn an_explicit_stdin_export_is_read_once_with_regular_file_parity() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    const CHILD: &str = "FSL_EXPORT_INTAKE_CHILD_RESULT";
    const LIMIT: &str = "FSL_EXPORT_INTAKE_CHILD_LIMIT";
    if let Some(output) = std::env::var_os(CHILD) {
        let maximum = std::env::var(LIMIT).unwrap().parse().unwrap();
        let result = run(
            Path::new("/dev/stdin"),
            ExportSelection::Ordinary,
            RunLimits {
                request_bytes: maximum,
                ..RunLimits::default()
            },
        );
        std::fs::write(output, result.to_string()).unwrap();
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("unit.native");
    std::fs::write(&source, "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction Ready using v(): Boolean pure { true }\n").unwrap();
    let original = request(&source).to_string().into_bytes();
    for (index, bytes, maximum, expected_exit, expected_text) in [
        (0, original.clone(), 1_048_576, 21, "not-an-obligation"),
        (
            1,
            br#"{"format":"native-run/1","request":{}}"#.to_vec(),
            1_048_576,
            20,
            "export",
        ),
        (2, original, 64, 22, "request"),
    ] {
        let output_path = directory.path().join(format!("child-{index}.json"));
        let regular_path = directory.path().join(format!("request-{index}.json"));
        std::fs::write(&regular_path, &bytes).unwrap();
        let regular = run(
            &regular_path,
            ExportSelection::Ordinary,
            RunLimits {
                request_bytes: maximum,
                ..RunLimits::default()
            },
        );
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "export_selection::an_explicit_stdin_export_is_read_once_with_regular_file_parity",
                "--test-threads=1",
            ])
            .env(CHILD, &output_path)
            .env(LIMIT, maximum.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&bytes).unwrap();
        let status = child.wait_with_output().unwrap();
        assert!(status.status.success(), "{status:?}");
        let streamed: Value = serde_json::from_slice(&std::fs::read(output_path).unwrap()).unwrap();
        assert_eq!(streamed["exit"], expected_exit, "{streamed}");
        assert!(streamed.to_string().contains(expected_text), "{streamed}");
        assert_eq!(streamed, regular);
    }
}
