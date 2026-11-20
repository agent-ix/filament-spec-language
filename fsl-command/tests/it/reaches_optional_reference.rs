// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-104, FR-365: an invariant that reaches over an optional-reference
//! field evaluates in a `native-run/1` clause run, with the target read from
//! the optional reference.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::{json, Value};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile m = \"filament.model.complete/v1\";\n\
     model M = \"example/config-version\";\n";

const SNAPSHOT: &str = r#"{"format":"filament.state.snapshot/v1","identity":{"authority":"agent-ix","identity":"test:reaches:current"},"observation":"current","anchor":{"kind":"handler","name":"validate"},"model":{"identity":"example/config-version"},"populations":[{"population":"ix://example/config-version/config_history","complete":true,"objects":[{"key":"root","type":"ix://example/config-version/ConfigVersion","fields":{"versionNumber":{"integer":"1"},"parent":{"absent":{}}}},{"key":"child","type":"ix://example/config-version/ConfigVersion","fields":{"versionNumber":{"integer":"2"},"parent":{"present":{"reference":{"population":"ix://example/config-version/config_history","key":"root"}}}}}]}]}"#;

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:reaches:{file}"),
        "document": file, "formal_revision": 1})
}

/// Runs the invariant `body` over the child object of the snapshot: the exit
/// code and the result document.
fn run(body: &str, key: &str) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let dir: &Path = directory.path();
    std::fs::write(
        dir.join("unit.native"),
        format!("{HEADER}invariant Q using m on M::ConfigVersion at current {{ {body} }}\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("model.semantic-ir.json"),
        crate::config_version_spine::spine::DOMAIN_PACKAGE,
    )
    .unwrap();
    std::fs::create_dir(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/current.json"), SNAPSHOT).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": {
            "file": "model.semantic-ir.json", "authority": "agent-ix",
            "identity": "example/config-version", "document": "ConfigVersion",
            "formal_revision": 1}}],
        "clause": {
            "selection": {"clause": {"name": "Q", "observation": "current",
                "self": {"population": "ix://example/config-version/config_history", "key": key},
                "anchor": {"kind": "handler", "name": "validate"}}},
            "observations": {"current": "data/current.json"}},
        "program": {"source": source("unit.native")}}});
    let path = dir.join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

/// FR-104, FR-365: the child reaches its optional-reference parent over the
/// one `parent` edge, so the invariant is true, and the reversed control, the
/// parent reaching the child, is false.
#[test]
#[trace("FR-104-AC-9", "FR-365-AC-7")]
fn reaches_over_an_optional_reference_target_evaluates() {
    let (exit, value) = run(
        "if present(self.parent) then reaches(self,value(self.parent),parent) else false",
        "child",
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(value["disposition"]["stage"], "evaluate", "{value}");
    assert_eq!(
        value["disposition"]["outcome"]["value"],
        json!({"kind": "boolean", "value": true}),
        "{value}"
    );
    let (exit, control) = run(
        "if present(self.parent) then reaches(value(self.parent),self,parent) else false",
        "child",
    );
    assert_eq!(exit, 10, "{control}");
    assert_eq!(
        control["disposition"]["outcome"]["value"],
        json!({"kind": "boolean", "value": false}),
        "{control}"
    );
}
