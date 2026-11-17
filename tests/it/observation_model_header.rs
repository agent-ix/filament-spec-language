// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-868 (TC-2460): an observation document's `model` member is the closed
//! object `{"identity": I}`. A `version` or `digest` member refuses
//! `invalid_runtime_input`/`unknown-member` naming it, because no version or
//! digest of a model exists to compare it with (FSpec FR-154).
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{run_file, Output};
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
#[ignore = "QSL-1999: refusing a version or digest in an observation's model header is not implemented yet"]
fn a_model_header_version_or_digest_is_refused() {
    for (member, value) in [("version", json!("1")), ("digest", json!("0000"))] {
        let directory = tempfile::tempdir().unwrap();
        let mut model = json!({"identity": spine::PACKAGE_IDENTITY});
        model[member] = value;
        let output = run_with_model(directory.path(), model);
        assert_eq!(output.status.code(), Some(20), "{member}: {output:?}");
        let refusal = text(&output);
        assert!(refusal.contains("unknown-member"), "{refusal}");
        assert!(refusal.contains(&format!("model.{member}")), "{refusal}");
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
