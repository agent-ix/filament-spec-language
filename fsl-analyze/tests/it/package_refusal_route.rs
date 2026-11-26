// SPDX-License-Identifier: AGPL-3.0-or-later
//! A compiled package the strict reader refuses settles the model-check
//! route with the reader's own typed refusal and that code's exit, never as
//! a runtime invariant (exit 30).

use filament_exact::Cancel;
use fsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, Engines, SourceProvision,
    SubjectBinding,
};
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_semantics::model::intake::PackageInput;
use ix_trace_rs::trace;

use crate::counter::{document, package, universe, Documents, CONTRACT};

/// The Counter unit selects the `test/counter` domain package, and a binding
/// whose `packages` leave that package out makes the strict reader refuse the
/// package it reads back with `missing_import`.
#[test]
#[trace("FR-286-AC-2", "TC-3199", "FR-286-AC-40")]
fn a_package_the_reader_refuses_settles_the_model_check_route_as_its_own_refusal() {
    let document = document(false);
    let checked = package(&document, CONTRACT);
    let mut documents = Documents::new(document);
    let initial = vec![documents.start("c0", &[("c", 0)])];
    let supplied = PackageInput::new();
    let request = AnalyzeRequest {
        binding: SubjectBinding {
            packages: &supplied,
            snapshots: &documents.snapshots,
            initial,
            universes: universe(&["c"]),
        },
        provision: SourceProvision::default(),
        items: Vec::new(),
        certification: Default::default(),
    };
    let failure = analyze(
        &checked,
        &request,
        &Engines::fsl().expect("they register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("the reader refuses the package");
    let StageFailure::Refused(AnalyzeRefusal::Package(refusal)) = failure else {
        panic!("wrong refusal: {failure:?}")
    };
    assert_eq!(refusal.code, Code::MissingImport);
    assert_eq!(refusal.identity.as_deref(), Some("test/counter"));
    assert!(
        refusal
            .pointer
            .as_deref()
            .is_some_and(|pointer| pointer.starts_with("/semantic_graph/nodes/")),
        "{refusal:?}"
    );
    let document = AnalyzeRefusal::Package(refusal).document();
    let schema: serde_json::Value =
        serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .unwrap();
    let whole: serde_json::Value = serde_json::from_slice(&document.to_bytes().unwrap()).unwrap();
    assert!(schema.is_valid(&whole), "{whole}");
    assert_eq!(document.category().exit_code(), 20);
    let diagnostic = document.diagnostics().first().expect("one diagnostic");
    assert_eq!(diagnostic.code(), "missing_import");
    let wire = serde_json::to_value(diagnostic).unwrap();
    assert_eq!(wire["cause"], "missing-selection", "{wire}");
    assert_eq!(wire["selection"], "package-reference", "{wire}");
    assert_eq!(
        wire["hint"],
        "Supply the domain package or library that the package names, in the request's models or libraries member.",
        "{wire}"
    );
}

/// The graph a request reader resolves a document-held id against comes from
/// the same strict read: a package the reader refuses is that refusal, with
/// its own code, and never a resource limit.
#[test]
#[trace("TC-2700", "FR-960-AC-9")]
fn the_document_graph_of_a_package_the_reader_refuses_is_the_readers_refusal() {
    let document = document(false);
    let checked = package(&document, CONTRACT);
    let refusal = fsl_analyze::analyze::DocumentGraphs::of_package(
        &checked,
        &PackageInput::new(),
        AnalyzeLimits::unbounded().emit,
    )
    .expect_err("the reader refuses the package");
    let AnalyzeRefusal::Package(refusal) = refusal else {
        panic!("wrong refusal: {refusal:?}")
    };
    assert_eq!(refusal.code, Code::MissingImport);
}
