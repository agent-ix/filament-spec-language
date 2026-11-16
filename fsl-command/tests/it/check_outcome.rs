// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-866: the source-only check API takes model and imported-library inputs.

use fsl_command::command::{self, CheckInputs};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const MODEL: &[u8] = include_bytes!("../../../examples/config-version/model.semantic-ir.json");
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    model Config = \"example/config-version\";\n\
    invariant NoCycle using v on Config::ConfigVersion at current {\n\
        not reaches(self, self, parent)\n\
    }\n";
const LIBRARY: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

fn write_source(directory: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn outcome(path: &Path, inputs: &CheckInputs) -> Value {
    serde_json::from_slice(
        &command::check_outcome(path, inputs)
            .unwrap()
            .to_bytes()
            .unwrap(),
    )
    .unwrap()
}

/// FR-866-AC-1 (TC-2457): check selects its domain package and returns the
/// same success shape as the compile pipeline through S4.
#[test]
#[trace("TC-2457", "FR-866-AC-1")]
fn check_outcome_selects_a_model_and_stops_at_s4() {
    let directory = tempfile::tempdir().unwrap();
    let source = write_source(directory.path(), "unit.native", UNIT.as_bytes());
    let model = write_source(directory.path(), "model.json", MODEL);
    let mut inputs = CheckInputs::default();
    inputs.models.push(("example/config-version".into(), model));

    let result = outcome(&source, &inputs);
    assert_eq!(result["operation"], "check", "{result}");
    assert_eq!(result["category"], "success", "{result}");
    assert_eq!(result["last_stage"], "S4", "{result}");
    assert_eq!(result["diagnostics"], json!([]), "{result}");
}

/// FR-866-AC-2 (TC-2457): check resolves and links an imported library; the
/// missing-library result matches a compile request with no library entry.
#[test]
#[trace("TC-2457", "FR-866-AC-2")]
fn check_outcome_supplies_an_imported_library() {
    let directory = tempfile::tempdir().unwrap();
    let source_bytes = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         import \"test/geometry\" as g;\n\
         function u using v(x: Int[0, 9]): Boolean pure {{ g::f(x) }}\n"
    );
    let source = write_source(directory.path(), "unit.native", source_bytes.as_bytes());
    let library = write_source(directory.path(), "geometry.native", LIBRARY.as_bytes());
    let mut inputs = CheckInputs::default();
    inputs.libraries.push(("test/geometry".into(), library));

    let result = outcome(&source, &inputs);
    assert_eq!(result["category"], "success", "{result}");
    assert_eq!(result["last_stage"], "S4", "{result}");

    let mut without_library = CheckInputs::default();
    without_library.dependent_bytes = 0;
    let missing = command::check_outcome(&source, &without_library)
        .unwrap()
        .to_bytes()
        .unwrap();
    let missing: Value = serde_json::from_slice(&missing).unwrap();
    assert_eq!(missing["category"], "refusal", "{missing}");
    assert!(missing.to_string().contains("test/geometry"), "{missing}");
}

/// FR-866-AC-3 (TC-2457): a missing selected model is an I1 missing-import
/// refusal, and its diagnostic names both ways to supply the model.
#[test]
#[trace("TC-2457", "FR-866-AC-3")]
fn missing_model_names_the_command_option_and_request_member() {
    let directory = tempfile::tempdir().unwrap();
    let source = write_source(directory.path(), "unit.native", UNIT.as_bytes());
    let result = outcome(&source, &CheckInputs::default());
    assert_eq!(result["category"], "refusal", "{result}");
    assert_eq!(
        result["diagnostics"][0]["code"], "missing_import",
        "{result}"
    );
    assert!(
        result.to_string().contains("example/config-version"),
        "{result}"
    );
    assert!(result.to_string().contains("--model"), "{result}");
    assert!(result.to_string().contains("models"), "{result}");
}

/// FR-866-AC-4 (TC-2457): unused supplied model/library identities refuse
/// before opening their paths.
#[test]
#[trace("TC-2457", "FR-866-AC-4")]
fn unused_check_inputs_refuse_before_reading_files() {
    let directory = tempfile::tempdir().unwrap();
    let source = write_source(
        directory.path(),
        "unit.native",
        b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Boolean pure { true }\n",
    );
    let mut models = CheckInputs::default();
    models.models.push((
        "example/unused".into(),
        directory.path().join("missing.json"),
    ));
    let error = command::check_outcome(&source, &models).expect_err("the model is unused");
    let refusal = error.value().unwrap();
    assert!(refusal.to_string().contains("--model"), "{refusal}");
    assert!(refusal.to_string().contains("example/unused"), "{refusal}");

    let mut libraries = CheckInputs::default();
    libraries.libraries.push((
        "test/unused".into(),
        directory.path().join("missing.native"),
    ));
    let error = command::check_outcome(&source, &libraries).expect_err("the library is unused");
    let refusal = error.value().unwrap();
    assert!(refusal.to_string().contains("--library"), "{refusal}");
    assert!(refusal.to_string().contains("test/unused"), "{refusal}");
}
