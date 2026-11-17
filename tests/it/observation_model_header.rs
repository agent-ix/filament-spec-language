// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-868 (TC-2460): an observation document's `model` member is the closed
//! object `{"identity": I}`. A `version` or `digest` member refuses
//! `invalid_runtime_input`/`unknown-member` naming it, because no version or
//! digest of a model exists to compare it with (FSpec FR-154).
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{compile_file, run_file, Output};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-868:{file}"),
        "document": document, "formal_revision": 1
    })
}

/// Runs FR-108's healthy `ParentOrder` request with the current snapshot's
/// `model` member replaced by `model`.
fn run_with_model(directory: &Path, model: Value) -> Output {
    spine::write(directory, Case::Healthy).unwrap();
    let path = directory.join("spine-snapshot.json");
    let mut snapshot: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    snapshot["model"] = model;
    std::fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    let child = json!({"population": format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
        "key": "child"});
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"clause": {"name": "ParentOrder", "observation": "current",
                "self": child, "anchor": {"kind": "handler", "name": "validate"}}},
            "observations": {"current": "spine-snapshot.json"},
        },
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

/// Runs the healthy snapshot case after replacing its model header with raw
/// JSON, preserving the member order supplied by the test.
fn run_with_raw_model(directory: &Path, model: &str) -> Output {
    spine::write(directory, Case::Healthy).unwrap();
    let path = directory.join("spine-snapshot.json");
    let snapshot = std::fs::read_to_string(&path).unwrap();
    let snapshot = snapshot.replace(
        r#""model":{"identity":"example/config-version"}"#,
        &format!(r#""model":{model}"#),
    );
    std::fs::write(&path, snapshot).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"clause": {"name": "ParentOrder", "observation": "current",
                "self": {"population": format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
                    "key": "child"}, "anchor": {"kind": "handler", "name": "validate"}}},
            "observations": {"current": "spine-snapshot.json"},
        },
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

fn run_with_invocation_model(directory: &Path) -> Output {
    spine::write(directory, Case::Unchanged).unwrap();
    let path = directory.join("spine-invocation.json");
    let mut invocation: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    invocation["model"]["version"] = json!("1");
    std::fs::write(&path, serde_json::to_vec(&invocation).unwrap()).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"clause": {"name": "VersionUnchanged", "observation": "invocation"}},
            "observations": {
                "pre": "spine-pre-snapshot.json", "post": "spine-post-snapshot.json",
                "invocation": "spine-invocation.json"
            },
        },
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// FR-868-AC-1 and AC-2.
#[test]
#[trace("TC-2460", "FR-868-AC-1")]
#[trace("TC-2460", "FR-868-AC-2")]
fn a_model_header_version_or_digest_is_refused() {
    for (member, value) in [("version", json!("1")), ("digest", json!("0000"))] {
        let directory = tempfile::tempdir().unwrap();
        let mut model = json!({"identity": spine::PACKAGE_IDENTITY});
        model[member] = value;
        let output = run_with_model(directory.path(), model);
        assert_eq!(output.status.code(), Some(20), "{member}: {output:?}");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["disposition"]["stage"], "admit", "{report}");
        assert_eq!(
            report["disposition"]["refusal"]["cause"], "unknown-member",
            "{report}"
        );
        assert!(
            report.to_string().contains(&format!("model.{member}")),
            "{report}"
        );
    }

    let directory = tempfile::tempdir().unwrap();
    let output = run_with_model(
        directory.path(),
        json!({"identity": spine::PACKAGE_IDENTITY}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let directory = tempfile::tempdir().unwrap();
    let output = run_with_model(directory.path(), json!({"identity": "example/other"}));
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(text(&output).contains("wrong-model-selection"));
}

/// FR-868-AC-1: when both forbidden members are present, admission reports
/// the first one in the source document's member order.
#[test]
#[trace("TC-2460", "FR-868-AC-1")]
fn a_model_header_reports_the_first_unknown_member_in_document_order() {
    let directory = tempfile::tempdir().unwrap();
    let output = run_with_raw_model(
        directory.path(),
        r#"{"identity":"example/config-version","version":"1","digest":"0000"}"#,
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["disposition"]["stage"], "admit", "{report}");
    assert_eq!(
        report["disposition"]["refusal"]["cause"], "unknown-member",
        "{report}"
    );
    assert!(report.to_string().contains("model.version"), "{report}");
    assert!(!report.to_string().contains("model.digest"), "{report}");
}

/// FR-868-AC-3: invocation documents use the same identity-only model header.
#[test]
#[trace("TC-2460", "FR-868-AC-3")]
fn an_invocation_model_version_is_refused_at_admission() {
    let directory = tempfile::tempdir().unwrap();
    let output = run_with_invocation_model(directory.path());
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["disposition"]["stage"], "admit", "{report}");
    assert_eq!(
        report["disposition"]["refusal"]["cause"], "unknown-member",
        "{report}"
    );
    assert!(report.to_string().contains("model.version"), "{report}");
}

/// FR-868-AC-3: a compile request's closed `models` entry rejects an
/// unsupported member before selected files are read.
#[test]
#[trace("TC-2460", "FR-868-AC-3")]
fn a_compile_model_entry_rejects_an_unknown_member() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "version": "1", "source":
            source("missing-model.json", "SpineModel")}],
        "program": {"source": source("missing-unit.native", "SpineUnit")},
    }});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = compile_file(&path);
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(report["code"], "invalid-request", "{report}");
    assert_eq!(report["stage"], "request", "{report}");
    assert!(report.to_string().contains("version"), "{report}");
}
