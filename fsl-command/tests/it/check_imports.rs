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
/// Provenance: QSL-2423.
fn malformed_check_source_refusal_uses_the_supplied_path_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("malformed.fsl");
    std::fs::write(&path, b"\xff").unwrap();

    let outcome = command::check_outcome(&path, &CheckInputs::default());
    let document = document(&outcome);
    assert_eq!(document["category"], "refusal", "{document}");
    let locus = &document["diagnostics"][0]["locus"];
    assert_eq!(locus["kind"], "region", "{document}");
    assert_eq!(locus["source"]["authority"], "filament", "{document}");
    assert_eq!(
        locus["source"]["identity"],
        path.to_string_lossy().as_ref(),
        "{document}"
    );
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
    assert_eq!(document["category"], "refusal", "{document}");
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

    assert_eq!(document["category"], "refusal", "{document}");
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

/// FR-866-AC-7 / QSL-2657: the check source uses its S1 ceiling, while only
/// selected dependent documents consume `dependent_bytes`.
#[test]
#[trace("TC-2457", "FR-866-AC-7")]
fn check_dependent_bytes_excludes_source_and_counts_selected_library() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("check.fsl");
    let source_only = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function Eligible using v(n: Int[0, 12]): Boolean pure { n >= 3 }\n";
    std::fs::write(&source_path, source_only).unwrap();
    let no_dependents = CheckInputs {
        dependent_bytes: 0,
        ..CheckInputs::default()
    };
    let bare = document(&command::check_outcome(&source_path, &no_dependents));
    assert_eq!(bare["category"], "success", "{bare}");
    assert_eq!(bare["last_stage"], "S4", "{bare}");
    assert!(bare["diagnostics"].as_array().unwrap().is_empty());

    let library_path = directory.path().join("geometry.fsl");
    let library = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
    std::fs::write(&library_path, library).unwrap();
    let library_bytes = std::fs::metadata(&library_path).unwrap().len() as usize;
    let importing_source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         import \"test/geometry\" as g;\n\
         function Eligible using v(n: Int[0, 12]): Boolean pure {{ n >= 3 }}\n"
    );
    let selected_library = || CheckInputs {
        libraries: vec![("test/geometry".to_owned(), library_path.clone())],
        dependent_bytes: library_bytes,
        ..CheckInputs::default()
    };
    // The unpadded FR-866-AC-2 program at N succeeds at S4.
    std::fs::write(&source_path, &importing_source).unwrap();
    let unpadded = document(&command::check_outcome(&source_path, &selected_library()));
    assert_eq!(unpadded["category"], "success", "{unpadded}");
    assert_eq!(unpadded["last_stage"], "S4", "{unpadded}");
    assert!(unpadded["diagnostics"].as_array().unwrap().is_empty());

    // Trailing whitespace pads the checked source past the library size, so
    // only the selected library can be what `dependent_bytes` counts.
    let importing_source = format!("{importing_source}{}", " ".repeat(library_bytes + 1));
    std::fs::write(&source_path, &importing_source).unwrap();
    assert!(importing_source.len() > library_bytes);
    let exact = document(&command::check_outcome(&source_path, &selected_library()));
    assert_eq!(exact["category"], "success", "{exact}");
    assert_eq!(exact["last_stage"], "S4", "{exact}");

    let below = CheckInputs {
        dependent_bytes: library_bytes - 1,
        ..selected_library()
    };
    let below = document(&command::check_outcome(&source_path, &below));
    assert_eq!(below["category"], "incomplete", "{below}");
    assert_eq!(below["last_stage"], Value::Null, "{below}");
    assert_eq!(below["diagnostics"].as_array().unwrap().len(), 1, "{below}");
    let diagnostic = &below["diagnostics"][0];
    assert_eq!(diagnostic["code"], "resource_exhausted", "{below}");
    assert_eq!(diagnostic["cause"], Value::Null, "{below}");
    assert_eq!(diagnostic["locus"], Value::Null, "{below}");
    let message = diagnostic["message"].as_str().unwrap();
    assert!(message.contains("dependent_bytes"), "{below}");
    assert!(
        message.contains(&(library_bytes - 1).to_string()),
        "{below}"
    );
    assert!(message.contains("--dependent-bytes"), "{below}");
}

/// FR-866 / FR-869 / QSL-2657: a check source over `s1.input_bytes` refuses at
/// S1 with `stage_limit_exceeded` (the shape compile gives the same source),
/// independent of `dependent_bytes`; at the ceiling it succeeds.
#[test]
#[trace("TC-2457", "FR-866")]
fn check_source_over_s1_input_bytes_refuses_at_s1_like_compile() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function Eligible using v(n: Int[0, 12]): Boolean pure { n >= 3 }\n";
    let (_directory, path) = source(text);
    let mut inputs = CheckInputs {
        dependent_bytes: 0,
        ..CheckInputs::default()
    };
    inputs.limits.source.source_bytes = text.len() - 1;
    let over = document(&command::check_outcome(&path, &inputs));
    assert_eq!(over["category"], "incomplete", "{over}");
    assert_eq!(over["last_stage"], "S1", "{over}");
    assert_eq!(over["diagnostics"].as_array().unwrap().len(), 1, "{over}");
    assert_eq!(
        over["diagnostics"][0]["code"], "stage_limit_exceeded",
        "{over}"
    );
    assert!(over.to_string().contains("s1.input_bytes"), "{over}");

    inputs.limits.source.source_bytes = text.len();
    let fits = document(&command::check_outcome(&path, &inputs));
    assert_eq!(fits["category"], "success", "{fits}");
    assert_eq!(fits["last_stage"], "S4", "{fits}");
}
