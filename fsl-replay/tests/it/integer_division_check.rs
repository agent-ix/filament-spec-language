// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-767: explicit laws reach lifecycle checking and Markdown bodies.

use filament_exact::Cancel;
#[cfg(feature = "quire-extraction")]
use filament_semantic_value::catalog::Category;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction q using v(a: Int[-7,7], b: Int[1,3]): Int[-7,7] pure { a div b }\nfunction r using v(a: Int[-7,7], b: Int[1,3]): Int[-7,7] pure { a rem b }\n";

#[test]
#[trace("TC-2197", "FR-767-AC-11")]
fn check_operation_takes_the_supplied_law() {
    let identity = SourceIdentity::new("agent-ix", "division-check");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.fsl",
            bytes: UNIT.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .unwrap()
        .into_value();
    let dependencies = DependencyInput::default();
    let law = fsl_replay::admit_integer_division(
        &[fsl_replay::DefinitionRole::IntegerDivisionFloor.reference()],
        None,
    )
    .unwrap();
    let checked = spine::check_with_integer_division(
        &parsed,
        "division-check".to_owned(),
        &models,
        &dependencies,
        &law,
        limits,
        &cancel,
    )
    .unwrap()
    .into_value();
    let emitted = spine::package(&checked, limits.package, &cancel)
        .unwrap()
        .into_value();
    let wire: serde_json::Value = serde_json::from_slice(emitted.package().bytes()).unwrap();
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let laws: Vec<_> = nodes
        .iter()
        .filter(|node| {
            matches!(
                node["body"]["operation"]["identity"].as_str(),
                Some("filament.op.integer.div" | "filament.op.integer.rem")
            )
        })
        .map(|node| {
            node["body"]["operation"]["laws"][0]["definition"]["identity"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(laws, vec!["filament.value.integer-division.floor/v1"; 2]);
    let error = spine::check(
        &parsed,
        "division-check".to_owned(),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .unwrap_err();
    let refusal = spine::refusal_or_fault(error).unwrap();
    assert_eq!(refusal.code(), fsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-selection"));
}

#[cfg(feature = "quire-extraction")]
#[test]
#[trace("TC-2197", "FR-767-AC-11")]
fn every_fence_takes_the_supplied_law() {
    let text = format!(
        "## Invariants\n### First\n```ix:native\n{UNIT}```\n### Second\n```ix:native\n{UNIT}```\n"
    );
    let original = fsl_foundation::Source::read(
        SourceIdentity::new("agent-ix", "doc"),
        "spec.md",
        text.as_bytes(),
        text.len(),
    )
    .unwrap();
    let packages = Default::default();
    let dependencies = DependencyInput::default();
    let law = fsl_replay::admit_integer_division(
        &[fsl_replay::DefinitionRole::IntegerDivisionFloor.reference()],
        None,
    )
    .unwrap();
    let mut request = spine::NativeFenceRequest {
        original: &original,
        packages: &packages,
        dependencies: &dependencies,
        extensions: None,
        integer_division: Some(&law),
    };
    let report = spine::check_native_fences(&request, Default::default(), &Cancel::new())
        .unwrap()
        .into_value();
    assert_eq!(report.fences.len(), 2);
    assert_eq!(
        report
            .fences
            .iter()
            .map(|fence| fence.category)
            .collect::<Vec<_>>(),
        vec![Category::Success; 2]
    );
    assert_eq!(report.fences[0].body.identity, "doc#First");
    assert_eq!(report.fences[1].body.identity, "doc#Second");
    let document: serde_json::Value = serde_json::from_slice(&fsl_replay::OutcomeDocument::from_native_fences(&report).to_bytes().unwrap()).unwrap();
    assert_eq!(document["operation"], "check");
    assert_eq!(document["items"].as_array().unwrap().len(), 2);
    assert_eq!(document["items"][0]["unit"]["body"]["identity"], "doc#First");
    request.integer_division = None;
    let report = spine::check_native_fences(&request, Default::default(), &Cancel::new())
        .unwrap()
        .into_value();
    assert_eq!(report.fences.len(), 2);
    for fence in report.fences {
        assert_eq!(fence.category, Category::Refusal);
        assert!(fence
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.diagnostic.cause() == Some("missing-selection")));
        assert!(!fence.diagnostics[0].original_regions.is_empty());
    }
}

#[cfg(feature = "quire-extraction")]
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
fn duplicate_and_headless_fences_refuse_independently_and_cancel_returns_no_report() {
    let text = format!("```ix:native\n{UNIT}```\n## Invariants\n### Duplicate\n```ix:native\n{UNIT}```\n```ix:native\n{UNIT}```\n### Good\n```ix:native\n{UNIT}```\n```text\nignored\n```\n");
    let original = fsl_foundation::Source::read(
        SourceIdentity::new("agent-ix", "doc"),
        "spec.md",
        text.as_bytes(),
        text.len(),
    )
    .unwrap();
    let packages = Default::default();
    let dependencies = DependencyInput::default();
    let law = fsl_replay::admit_integer_division(
        &[fsl_replay::DefinitionRole::IntegerDivisionFloor.reference()],
        None,
    )
    .unwrap();
    let request = spine::NativeFenceRequest {
        original: &original,
        packages: &packages,
        dependencies: &dependencies,
        extensions: None,
        integer_division: Some(&law),
    };
    let first = spine::check_native_fences(&request, Default::default(), &Cancel::new())
        .unwrap()
        .into_value();
    assert_eq!(
        first,
        spine::check_native_fences(&request, Default::default(), &Cancel::new())
            .unwrap()
            .into_value()
    );
    assert_eq!(first.fences.len(), 4);
    assert_eq!(
        first.fences[0].diagnostics[0].diagnostic.cause(),
        Some("missing-name")
    );
    for fence in &first.fences[1..3] {
        assert_eq!(fence.actual, Some(2));
        assert_eq!(
            fence.diagnostics[0].diagnostic.cause(),
            Some("extraction-clause-count")
        );
    }
    assert_eq!(first.fences[3].category, Category::Success);
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    assert!(matches!(
        spine::check_native_fences(&request, Default::default(), &cancel),
        Err(fsl_foundation::diagnostic::StageFailure::Cancelled(
            filament_exact::CancelCause::Requested
        ))
    ));
}
