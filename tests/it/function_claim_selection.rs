// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-867 (TC-2459): a `function` claim selection whose function does not
//! return `Boolean` is refused at stage `select` naming the function, the
//! expected `Boolean` and the declared result type; a Boolean function runs
//! as a claim and an unknown name is `missing_declaration`.
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{run_file, Output};
use crate::support::config_version_spine::{spine, Case};
use fsl_replay::spine::{
    read_native_run_result, ClauseRunRecord, NativeRunOutcome, NativeRunResult,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const FUNCTIONS: &str = "function seven using v(): Integer pure { 7 }\n\
                         function yes using v(): Boolean pure { true }\n\
                         function boundedValue using v(): Int[0, 3] pure { 3 }\n";

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-867:{file}"),
        "document": document, "formal_revision": 1
    })
}

/// Runs a clause request selecting the function `name` over the healthy
/// snapshot.
fn select(directory: &Path, name: &str) -> Output {
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"function": {"name": name, "arguments": [], "snapshot": "current"}},
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

/// Parses the public native-run-result/2 response through the same reader a
/// downstream consumer uses.
fn read_clause_result(output: &Output) -> Box<ClauseRunRecord> {
    match read_native_run_result(&output.stdout).expect("the command result is readable") {
        NativeRunResult::Clause(record) => record,
        NativeRunResult::CommandError(_) => panic!("a clause result, not a command error"),
        NativeRunResult::ProtocolAnalysis(_) => panic!("a clause result, not protocol analysis"),
    }
}

/// FR-867-AC-1 and AC-2.
#[test]
#[trace("TC-2459", "FR-867-AC-1")]
#[trace("TC-2459", "FR-867-AC-2")]
fn a_non_boolean_function_claim_names_the_function_and_its_type() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = std::fs::read_to_string(directory.path().join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.path().join("spine-unit.native"),
        format!("{unit}{FUNCTIONS}"),
    )
    .unwrap();

    let refused = select(directory.path(), "seven");
    assert_eq!(refused.status.code(), Some(20), "{refused:?}");
    let refusal = text(&refused);
    assert!(refusal.contains("\"stage\":\"select\""), "{refusal}");
    assert!(refusal.contains("ill_typed"), "{refusal}");
    assert!(refusal.contains("type-mismatch"), "{refusal}");
    assert!(refusal.contains("\"function\":\"seven\""), "{refusal}");
    assert!(refusal.contains("\"kind\":\"located\""), "{refusal}");
    assert!(refusal.contains("\"expected\":\"Boolean\""), "{refusal}");
    assert!(refusal.contains("\"actual\":\"Integer\""), "{refusal}");
    assert!(
        refusal.contains("function `seven` returns `Integer`")
            && refusal.contains("result is `Boolean`"),
        "{refusal}"
    );
    let integer_record = read_clause_result(&refused);
    assert_eq!(integer_record.stage(), "select");
    assert_eq!(integer_record.code(), Some("ill_typed"));
    assert_eq!(integer_record.cause(), Some("type-mismatch"));
    let fields = integer_record.fields().expect("the selection name is retained");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields.get("name").map(String::as_str), Some("seven"));

    let bounded = select(directory.path(), "boundedValue");
    assert_eq!(bounded.status.code(), Some(20), "{bounded:?}");
    let bounded = text(&bounded);
    assert!(bounded.contains("\"actual\":\"Int[0, 3]\""), "{bounded}");
    assert!(
        bounded.contains("function `boundedValue` returns `Int[0, 3]`"),
        "{bounded}"
    );

    let tested = select(directory.path(), "yes");
    assert_eq!(tested.status.code(), Some(0), "{tested:?}");
    assert!(
        text(&tested).contains("\"truth\":true"),
        "{}",
        text(&tested)
    );
    let boolean_record = read_clause_result(&tested);
    assert_eq!(boolean_record.stage(), "evaluate");
    assert_eq!(boolean_record.category(), "success");
    assert_eq!(boolean_record.truth(), Some(true));
    assert!(matches!(
        boolean_record.outcome(),
        Some(NativeRunOutcome::Boolean(true))
    ));

    let unknown_output = select(directory.path(), "nope");
    assert_eq!(unknown_output.status.code(), Some(20), "{unknown_output:?}");
    let unknown = text(&unknown_output);
    assert!(unknown.contains("\"stage\":\"select\""), "{unknown}");
    assert!(unknown.contains("missing_declaration"), "{unknown}");
    assert!(unknown.contains("nope"), "{unknown}");
    let unknown_record = read_clause_result(&unknown_output);
    assert_eq!(unknown_record.stage(), "select");
    assert_eq!(unknown_record.code(), Some("missing_declaration"));
    assert_eq!(unknown_record.cause(), Some("missing-name"));
    let fields = unknown_record
        .fields()
        .expect("the unknown selection name is retained");
    assert_eq!(fields.get("name").map(String::as_str), Some("nope"));
}
