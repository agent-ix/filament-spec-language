// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original protocol source programs and authoritative parts-saga journeys.

use filament_exact::Cancel;
use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn saga() -> PathBuf {
    let root = std::env::var_os("FSL_EXAMPLES_DIR")
        .map(PathBuf::from)
        .expect("FSL_EXAMPLES_DIR must point to the authoritative filament-examples checkout");
    root.join("examples/parts-saga")
}

fn corpus(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus/protocol")
        .join(file)
}

fn source(file: &Path) -> Value {
    json!({"file":file,"authority":"test","identity":"test:protocol-corpus",
        "document":"ProtocolCorpus","formal_revision":1})
}

fn compile_request(file: &Path) -> Value {
    json!({"format":"native-compile/1","request":{
        "program":{"source":source(file)},
        "models":[{"format":"semantic-ir/2.0.0",
            "source":source(&saga().join("model/parts-saga.semantic-ir.json"))}]}})
}

fn write(directory: &Path, request: &Value) -> PathBuf {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn compile(directory: &Path, file: &Path) {
    let output = command::compile_outcome(&write(directory, &compile_request(file)))
        .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
    assert!(!output.package_bytes().is_empty(), "{}", file.display());
}

fn refusal(directory: &Path, file: &Path, code: &str, cause: &str) {
    let error = match command::compile_outcome(&write(directory, &compile_request(file))) {
        Ok(_) => panic!("{} must refuse {code}/{cause}", file.display()),
        Err(error) => error,
    };
    let artifact = error.artifact_value().unwrap();
    assert_eq!(artifact["code"], code, "{}: {artifact}", file.display());
    assert_eq!(artifact["cause"], cause, "{}: {artifact}", file.display());
}

#[test]
#[trace("FR-218", "FR-219", "FR-220", "FR-224", "FR-884")]
fn original_protocol_declarations_communications_and_refusals() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read_to_string(corpus("original/exchange.native")).unwrap();
    compile(directory.path(), &corpus("original/exchange.native"));
    // Each mutation retains the same accepted model, subject and surrounding source.
    for (name, from, to, code, cause) in [
        ("duplicate-role", "role Receiver on Factory::MachiningStation;",
         "role Receiver on Factory::MachiningStation; role Receiver on Factory::MachiningStation;",
         "ambiguous_declaration", "ambiguous-name"),
        ("record-role", "role Receiver on Factory::MachiningStation;", "role Receiver on Note;",
         "invalid_model_binding", "malformed-declaration"),
        ("object-message", "carries Note", "carries Factory::Job",
         "invalid_model_binding", "malformed-declaration"),
        ("wrong-channel", "receive Seen via Queue", "receive Seen via Absent",
         "missing_declaration", "missing-name"),
        ("effect-of-send", "receive Seen via Queue of Sent as (got: Note) { true };",
         "effect Seen of Sent as (got: Note) { true };", "ill_typed", "type-mismatch"),
    ] {
        assert!(original.contains(from));
        let file = directory.path().join(format!("{name}.native"));
        std::fs::write(&file, original.replacen(from, to, 1)).unwrap();
        refusal(directory.path(), &file, code, cause);
    }
}

#[test]
#[trace(
    "FR-218", "FR-219", "FR-220", "FR-223", "FR-224", "FR-225", "FR-226", "FR-227", "FR-228",
    "FR-229", "FR-230", "FR-231", "FR-232", "FR-233"
)]
fn authoritative_parts_saga_protocol_constructs_compile_from_their_own_repository() {
    let directory = tempfile::tempdir().unwrap();
    for file in [
        "part-flow-untimed.fsl",
        "part-flow.fsl",
        "inspection.fsl",
        "protocols/declaration/declaration-minimal.fsl",
        "protocols/declaration/declaration-on-each.fsl",
        "protocols/declaration/declaration-terminal-scheduling.fsl",
        "protocols/roles-and-channels/roles-replicated.fsl",
        "protocols/roles-and-channels/channels-loss-head.fsl",
        "protocols/roles-and-channels/channels-fifo-by-part.fsl",
        "protocols/steps/steps-all-kinds.fsl",
        "protocols/control-flow/control-parallel-all.fsl",
        "protocols/control-flow/control-parallel-any-cancel.fsl",
        "protocols/control-flow/control-repeat-progress.fsl",
        "protocols/retries-and-timeouts/timeouts-await.fsl",
    ] {
        compile(directory.path(), &saga().join("rules").join(file));
    }
}

#[test]
#[trace("FR-218", "FR-220", "FR-224", "FR-225", "FR-226", "FR-231", "FR-233")]
fn authoritative_parts_saga_rejects_invalid_protocols_at_the_right_boundary() {
    let directory = tempfile::tempdir().unwrap();
    for (file, code, cause) in [
        (
            "protocols/declaration/declaration-no-finish.fsl",
            "invalid_syntax",
            "unexpected-token",
        ),
        (
            "protocols/roles-and-channels/channels-object-payload.fsl",
            "invalid_model_binding",
            "malformed-declaration",
        ),
        (
            "protocols/steps/steps-effect-of-send.fsl",
            "ill_typed",
            "type-mismatch",
        ),
        (
            "protocols/steps/steps-receive-other-channel.fsl",
            "ill_typed",
            "type-mismatch",
        ),
        (
            "protocols/control-flow/control-repeat-no-progress.fsl",
            "unsupported_construct",
            "declaration-form",
        ),
        (
            "protocols/control-flow/control-join-omits-branch.fsl",
            "invalid_package",
            "invalid-value",
        ),
        (
            "protocols/compensation/compensation-zero-attempts.fsl",
            "ill_typed",
            "type-mismatch",
        ),
    ] {
        refusal(
            directory.path(),
            &saga().join("rules").join(file),
            code,
            cause,
        );
    }
}

/// Resolve file references without reproducing an example's source or model.
fn authoritative_request(path: &Path) -> Value {
    let mut request: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let base = path.parent().unwrap();
    let program = &mut request["request"]["program"]["source"]["file"];
    *program = json!(base.join(program.as_str().unwrap()));
    for model in request["request"]["models"].as_array_mut().unwrap() {
        let file = &mut model["source"]["file"];
        *file = json!(base.join(file.as_str().unwrap()));
    }
    for file in request["request"]["clause"]["observations"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        *file = json!(base.join(file.as_str().unwrap()));
    }
    request
}

#[test]
#[trace("FR-330", "FR-52")]
fn parts_saga_observations_distinguish_tracked_untracked_and_third_rework() {
    let directory = tempfile::tempdir().unwrap();
    for (name, exit, result) in [
        ("jobs-rework-tracked", 0, "tested"),
        ("jobs-rework-untracked", 10, "refuted"),
        ("jobs-third-rework", 10, "refuted"),
    ] {
        let request = authoritative_request(&saga().join(format!("requests/{name}.json")));
        let output = command::run(
            &write(directory.path(), &request),
            &crate::negotiation::Fr290,
            &Cancel::new(),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        let native = output.value.as_value();
        fsl_replay::spine::read_native_run_result(&serde_json::to_vec(native).unwrap())
            .unwrap_or_else(|error| panic!("{name}: strict native result: {error}"));
        assert_eq!(output.exit_code, exit, "{name}: {native}");
        let lifecycle: Value =
            serde_json::from_slice(&output.outcome.unwrap().to_bytes().unwrap()).unwrap();
        assert_eq!(
            lifecycle["items"][0]["result"], result,
            "{name}: {lifecycle}"
        );
    }
}
