// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-865-AC-17..24 and FR-286-AC-40: each `selection` kind's producer
//! document carries its hint, validates, and its text holds that hint line.

use filament_exact::Identifier;
use filament_semantic_value::catalog::Category;
use fsl_analyze::analyze::{AnalyzeRefusal, ProvisionRefusal};
use fsl_command::command::{self, CheckInputs, RunCause, RunError};
use fsl_foundation::selection::DefinitionRef;
use fsl_foundation::{Code, SourceIdentity};
use fsl_replay::spine::{CompileRefusal, UnitListRefusal};
use fsl_replay::{
    CallSiteRefusal, ImportedFunctionName, LibraryName, Operation, OutcomeDiagnostic,
    OutcomeDocument, ReplayRefusal, ReplayRequestRefusal,
};
use fsl_semantics::library::extension::ExtensionRefusal;
use ix_trace_rs::trace;
use serde_json::Value;

use crate::cli_contract::assert_outcome_schema;

const MODEL: &str = "Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.";
const LIBRARY: &str = "Supply the library in the request's libraries member, or with --library <identity>=<file> on fsl check.";
const SIBLING: &str = "Supply the imported unit as a library, not as a sibling unit.";
const UNIMPORTED: &str =
    "Call a function of a library that the unit imports directly, or add the import.";
const OWNER: &str = "Name a function_owner that the program imports directly, or add the import.";
const BYTES: &str = "Add a byte_provision entry for the named source to the replay request.";
const EXTENSION: &str = "Add the selected extension's definition to the request's extensions member, or remove the selection.";
const SOURCE: &str = "Add the named source to the source provision the analyze call supplies.";
const PACKAGE: &str = "Supply the domain package or library that the package names, in the request's models or libraries member.";

/// The document of one diagnostic.
fn document_of(diagnostic: OutcomeDiagnostic) -> OutcomeDocument {
    OutcomeDocument::new(Operation::Check, None, Category::Refusal)
        .with_diagnostics(vec![diagnostic])
}

/// `document` carries one `missing_import`/`missing-selection` diagnostic of
/// `kind` with `hint`, validates, and its text holds the hint line.
fn assert_kind(kind: &str, hint: &str, document: &OutcomeDocument) {
    let wire: Value = serde_json::from_slice(&document.to_bytes().unwrap()).unwrap();
    assert_outcome_schema(&wire);
    let diagnostic = &wire["diagnostics"][0];
    assert_eq!(diagnostic["code"], "missing_import", "{wire}");
    assert_eq!(diagnostic["cause"], "missing-selection", "{wire}");
    assert_eq!(diagnostic["selection"], kind, "{wire}");
    assert_eq!(diagnostic["hint"], hint, "{wire}");
    let text = fsl_inspect::render_outcome(
        document,
        fsl_inspect::RenderForm::Text,
        &fsl_command::render_sources(&fsl_command::Sources::default()),
        fsl_replay::RenderLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the document renders")
    .value()
    .as_str()
    .to_owned();
    assert!(
        text.lines().any(|line| line == format!("  hint: {hint}")),
        "{kind}: {text}"
    );
}

fn check_source(text: &str) -> OutcomeDocument {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unit.fsl");
    std::fs::write(&path, text).unwrap();
    let outcome = command::check_outcome(&path, &CheckInputs::default());
    let document = outcome.document().clone();
    drop(directory);
    document
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n";

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-17",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_model_producer_text_holds_its_hint() {
    let unit = format!(
        "{HEADER}profile v = \"filament.model.complete/v1\";\n\
         model Config = \"example/config-version\";\n\
         invariant NoCycle using v on Config::ConfigVersion at current {{ not reaches(self, self, parent) }}\n"
    );
    assert_kind("model", MODEL, &check_source(&unit));
}

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-18",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_library_producer_text_holds_its_hint() {
    let unit = format!(
        "{HEADER}profile v = \"filament.value.complete/v1\";\n\
         import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    assert_kind("library", LIBRARY, &check_source(&unit));
}

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-22",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_sibling_unit_producer_text_holds_its_hint() {
    let refusal = CompileRefusal::UnitList(UnitListRefusal::SiblingImport {
        identity: "a-unit".to_owned(),
        unit: Box::new(SourceIdentity::new("test", "a-unit")),
        region: None,
    });
    assert_kind(
        "sibling-unit",
        SIBLING,
        &document_of(OutcomeDiagnostic::from_compile_refusal(&refusal)),
    );
}

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-19",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_unimported_library_producer_text_holds_its_hint() {
    let refusal = CallSiteRefusal::UnknownImport {
        selection: ImportedFunctionName {
            library: LibraryName::new("test/other").unwrap(),
            function: Identifier::new("big").unwrap(),
        },
        package: LibraryName::new("fixture").unwrap(),
    };
    assert_kind(
        "unimported-library",
        UNIMPORTED,
        &document_of(OutcomeDiagnostic::from_call_site_refusal(&refusal)),
    );
}

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-20",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_function_owner_producer_text_holds_its_hint() {
    let refusal = ReplayRefusal::UnknownOwner {
        owner: LibraryName::new("test/other").unwrap(),
        package: LibraryName::new("fixture").unwrap(),
    };
    assert_kind(
        "function-owner",
        OWNER,
        &OutcomeDocument::from_replay_refusal(&refusal),
    );
}

#[test]
#[trace(
    "TC-3199",
    "FR-865-AC-21",
    "FR-865-AC-23",
    "FR-865-AC-24",
    "FR-286-AC-40"
)]
fn the_byte_provision_producer_text_holds_its_hint() {
    let refusal = ReplayRefusal::Request(ReplayRequestRefusal::IncompleteByteProvision {
        requested: "test unit".to_owned(),
        named_by: "/package".to_owned(),
    });
    assert_kind(
        "byte-provision",
        BYTES,
        &OutcomeDocument::from_replay_refusal(&refusal),
    );
}

#[test]
#[trace("TC-3199", "FR-865-AC-23", "FR-865-AC-24", "FR-286-AC-40")]
fn the_extension_definition_producer_text_holds_its_hint() {
    let refusal = CompileRefusal::Extension(ExtensionRefusal::MissingDependency {
        dependent: DefinitionRef::new("agent-ix", "ext.a").unwrap(),
        dependency: DefinitionRef::new("agent-ix", "ext.gone").unwrap(),
    });
    assert_eq!(refusal.code(), Code::MissingImport);
    assert_kind(
        "extension-definition",
        EXTENSION,
        &document_of(OutcomeDiagnostic::from_compile_refusal(&refusal)),
    );
}

#[test]
#[trace("TC-3199", "FR-865-AC-23", "FR-865-AC-24", "FR-286-AC-40")]
fn the_source_producer_text_holds_its_hint() {
    let refusal = AnalyzeRefusal::Provision(ProvisionRefusal {
        authority: "test".to_owned(),
        identity: "counter".to_owned(),
    });
    assert_kind("source", SOURCE, &refusal.document());
}

/// The run's strict-reader package refusal: a missing import has the pair's
/// `missing-selection` cause whether or not the reader named it.
#[test]
#[trace("TC-3199", "FR-865-AC-23", "FR-865-AC-24", "FR-286-AC-40")]
fn the_package_reference_producer_text_holds_its_hint() {
    for cause in [None, Some("missing-selection".to_owned())] {
        let error = RunError {
            cause: RunCause::AnalyzePackage {
                code: Code::MissingImport,
                cause,
                pointer: Some("/semantic_graph/nodes/0".to_owned()),
                message: "missing import test/counter".to_owned(),
            },
            sources: fsl_command::Sources::default(),
        };
        assert_kind(
            "package-reference",
            PACKAGE,
            &error.outcome_document(Operation::Analyze),
        );
    }
}
