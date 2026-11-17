// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-866: check admits the explicit model/library selections through bounded intake.

use fsl_command::command::{self, CheckInputs};
use ix_trace_rs::trace;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn source(text: &str) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("check.fsl");
    std::fs::write(&path, text).unwrap();
    (directory, path)
}

fn document(outcome: &command::CheckOutcome) -> Value {
    serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap()
}

#[test]
#[trace("TC-2457", "FR-866-AC-1", "FR-865-AC-5")]
fn check_outcome_admits_model_and_retains_source() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        model Config = \"example/config-version\";\n\
        invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }\n";
    let (_directory, path) = source(text);
    let model = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/config-version/model.semantic-ir.json");
    let inputs = CheckInputs {
        models: vec![("example/config-version".to_owned(), model)],
        ..CheckInputs::default()
    };

    let outcome = command::check_outcome(&path, &inputs);
    let document = document(&outcome);
    assert_eq!(document["operation"], "check", "{document}");
    assert_eq!(document["category"], "success", "{document}");
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert!(document["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(outcome.sources().len(), 2);
    assert!(outcome
        .sources()
        .as_slice()
        .iter()
        .any(|source| source.path() == path.to_string_lossy()));
    let rendered_sources = fsl_command::render_sources(outcome.sources());
    assert!(format!("{rendered_sources:?}").contains("source_count: 2"));
}

#[test]
#[trace("TC-2457", "FR-866-AC-4")]
fn check_rejects_an_unselected_model_before_reading_its_file() {
    let (_directory, path) = source(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function seven using v(): Int[0, 9] pure { 7 }\n",
    );
    let inputs = CheckInputs {
        models: vec![("example/other".to_owned(), PathBuf::from("missing.json"))],
        ..CheckInputs::default()
    };

    let document = document(&command::check_outcome(&path, &inputs));
    assert_eq!(document["operation"], "check", "{document}");
    assert_eq!(document["category"], "refused", "{document}");
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    assert!(document["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("--model example/other"));
}

#[test]
#[trace("TC-2457", "FR-866-AC-3")]
fn check_reports_a_missing_model_selection_at_admission() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        model Config = \"example/config-version\";\n\
        invariant NoCycle using v on Config::ConfigVersion at current { not reaches(self, self, parent) }\n";
    let (_directory, path) = source(text);
    let document = document(&command::check_outcome(&path, &CheckInputs::default()));

    assert_eq!(document["category"], "refused", "{document}");
    assert_eq!(document["last_stage"], "I1", "{document}");
    assert_eq!(
        document["diagnostics"][0]["code"], "missing_import",
        "{document}"
    );
    assert_eq!(
        document["diagnostics"][0]["cause"], "missing-selection",
        "{document}"
    );
    let message = document["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains("example/config-version"), "{document}");
    assert!(message.contains("--model"), "{document}");
    assert!(message.contains("models"), "{document}");
}
