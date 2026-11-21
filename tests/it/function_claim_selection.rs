// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-867 (TC-2459): a `function` claim selection whose function does not
//! return `Boolean` is refused at stage `select` naming the function, the
//! expected `Boolean` and the declared result type; a Boolean function runs
//! as a claim and an unknown name is `missing_declaration`.
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{run_file, Output};
use crate::support::config_version_spine::{spine, Case};
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

fn assert_readable_native_result(output: &Output) {
    assert!(
        matches!(
            fsl_replay::spine::read_native_run_result(&output.stdout),
            Ok(fsl_replay::spine::NativeRunResult::Clause(_))
        ),
        "the public reader accepts the command result: {}",
        text(output)
    );
}

fn monitor_item(directory: &Path) -> Value {
    let result =
        fsl_command::command::run(&directory.join("request.json"), &crate::negotiation::Fr290)
            .expect("the function run returns its monitor outcome");
    let outcome = result
        .outcome
        .expect("a function run has a monitor outcome");
    serde_json::to_value(&outcome.items()[0]).expect("the monitor item serializes")
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
    let native: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(
        native["disposition"]["refusal"]["fields"],
        json!({"name": "seven"})
    );
    assert_readable_native_result(&refused);
    assert!(refusal.contains("\"kind\":\"located\""), "{refusal}");
    assert!(
        refusal.contains("function `seven` returns `Integer`")
            && refusal.contains("result is `Boolean`"),
        "{refusal}"
    );
    assert_eq!(
        monitor_item(directory.path())["diagnostic"],
        json!({
            "code": "ill_typed",
            "cause": "type-mismatch",
            "detail": {
                "function": "seven",
                "expected": "Boolean",
                "actual": "Integer",
            },
        })
    );

    let bounded = select(directory.path(), "boundedValue");
    assert_eq!(bounded.status.code(), Some(20), "{bounded:?}");
    let bounded_text = text(&bounded);
    let bounded_native: Value = serde_json::from_slice(&bounded.stdout).unwrap();
    assert_eq!(
        bounded_native["disposition"]["refusal"]["fields"],
        json!({"name": "boundedValue"})
    );
    assert_readable_native_result(&bounded);
    assert!(
        bounded_text.contains("function `boundedValue` returns `Int[0, 3]`"),
        "{bounded_text}"
    );
    assert_eq!(
        monitor_item(directory.path())["diagnostic"]["detail"],
        json!({
            "function": "boundedValue",
            "expected": "Boolean",
            "actual": "Int[0, 3]",
        })
    );

    let tested = select(directory.path(), "yes");
    assert_eq!(tested.status.code(), Some(0), "{tested:?}");
    assert_readable_native_result(&tested);
    let tested_item = monitor_item(directory.path());
    assert_eq!(tested_item["result"], "tested");
    assert!(tested_item.get("diagnostic").is_none());
    assert!(
        text(&tested).contains("\"truth\":true"),
        "{}",
        text(&tested)
    );

    let unknown = select(directory.path(), "nope");
    assert_eq!(unknown.status.code(), Some(20), "{unknown:?}");
    let unknown = text(&unknown);
    assert!(unknown.contains("\"stage\":\"select\""), "{unknown}");
    assert!(unknown.contains("missing_declaration"), "{unknown}");
    assert!(unknown.contains("nope"), "{unknown}");
    assert_readable_native_result(&select(directory.path(), "nope"));
    assert_eq!(
        monitor_item(directory.path())["diagnostic"],
        json!({
            "code": "missing_declaration",
            "cause": "missing-name",
            "detail": {"name": "nope"},
        })
    );
}
