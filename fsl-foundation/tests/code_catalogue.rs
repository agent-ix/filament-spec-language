// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010-AC-8: stable native codes round-trip and match their documentation.

use fsl_foundation::diagnostic::Code;
use std::collections::BTreeSet;

const DOCUMENTATION: &str = include_str!("../../docs/native-error-codes.md");
const CST_DIAGNOSTICS: &str = include_str!("../../fsl-cst/src/diagnostic.rs");

/// FR-010-AC-8: every stable code has exactly one documented catalogue row,
/// and documentation cannot retain a code that the Rust catalogue removed.
#[test]
fn documented_rows_match_the_code_catalogue_in_both_directions() {
    let documented = DOCUMENTATION
        .lines()
        .filter_map(|line| line.strip_prefix("| "))
        .filter_map(|row| row.split_once(" |"))
        .map(|(code, _)| code)
        .filter(|code| *code != "Code" && *code != "---")
        .collect::<BTreeSet<_>>();
    let implemented = Code::all()
        .iter()
        .map(|code| code.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        documented, implemented,
        "documented and implemented codes differ"
    );
}

/// FR-010-AC-8: each code in the complete vocabulary resolves to itself and
/// has a defined outcome category.
#[test]
fn every_code_round_trips_through_the_catalogue() {
    for code in Code::all() {
        assert_eq!(Code::from_code(code.as_str()), Some(*code), "{code}");
        let _ = code.category();
    }
}

/// FR-301/FR-285: exit class is a function of the stable code. This table
/// covers the complete diagnostic refusal corpus, including unsupported,
/// incomplete and internal-failure codes.
#[test]
fn every_code_maps_to_its_exit_class() {
    let cases = [
        (Code::IoError, 20),
        (Code::InvalidRequest, 20),
        (Code::InvalidIdentifier, 20),
        (Code::OutputFailure, 20),
        (Code::UnsupportedProjection, 21),
        (Code::InvalidProjectionCorrespondence, 20),
        (Code::ExtractionRequiresRun, 20),
        (Code::ExtractionPackageConflict, 20),
        (Code::ExtractionClauseCount, 20),
        (Code::InvalidQuireContext, 20),
        (Code::InvalidSourceIdentity, 20),
        (Code::InvalidSourceMap, 20),
        (Code::InvalidUtf8, 20),
        (Code::InvalidSyntax, 20),
        (Code::UnsupportedConstruct, 21),
        (Code::UnrepresentableConstraint, 20),
        (Code::UnknownLanguage, 20),
        (Code::UnknownEdition, 20),
        (Code::UnknownProfile, 20),
        (Code::InvalidPackage, 20),
        (Code::UnknownWire, 20),
        (Code::UnknownRequiredFeature, 21),
        (Code::ResourceExhausted, 22),
        (Code::MissingImport, 20),
        (Code::StaleDependency, 20),
        (Code::AmbiguousDeclaration, 20),
        (Code::MissingDeclaration, 20),
        (Code::ConflictingDeclaration, 20),
        (Code::InvalidModelBinding, 20),
        (Code::InvalidRuntimeInput, 20),
        (Code::WrongSnapshot, 20),
        (Code::IllTyped, 20),
        (Code::UndefinedExpression, 20),
        (Code::DanglingReference, 20),
        (Code::IncompletePopulation, 22),
        (Code::UnavailableObservation, 22),
        (Code::PopulationDeltaMismatch, 20),
        (Code::FrameViolation, 20),
        (Code::Cancelled, 22),
        (Code::RuntimeInvariant, 30),
        (Code::AmbiguousDispatch, 20),
        (Code::ForeignReference, 20),
        (Code::CardinalityOutOfBound, 20),
        (Code::DuplicateSelection, 20),
        (Code::StageLimitExceeded, 22),
        (Code::NoncanonicalWire, 20),
        (Code::UnimplementedCapability, 21),
        (Code::InvalidProbabilisticModel, 20),
        (Code::InvalidTimedModel, 20),
    ];

    assert_eq!(cases.len(), Code::all().len(), "table covers every code");
    for (code, exit_code) in cases {
        assert!(Code::all().contains(&code), "{code} is in the catalogue");
        assert_eq!(
            code.category().exit_code(),
            exit_code,
            "{} must map to exit {exit_code}",
            code.as_str()
        );
    }
}

/// FR-010-AC-8: every typed complete-source cause has a case in the
/// diagnostic mapping test, including each syntax cause used by recovery.
#[test]
fn every_complete_cause_appears_in_a_diagnostic_test() {
    let variants = CST_DIAGNOSTICS
        .split_once("pub enum CompleteCause {")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| {
            body.lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let first = line.split_whitespace().next()?;
                    let name = first.split('(').next()?.trim_end_matches(',');
                    name.chars()
                        .next()
                        .filter(|character| character.is_ascii_uppercase())
                        .map(|_| name)
                })
                .collect::<Vec<_>>()
        })
        .expect("CompleteCause enum is present");
    let diagnostic_tests = CST_DIAGNOSTICS
        .split_once("#[cfg(test)]\nmod tests")
        .map(|(_, tests)| tests)
        .expect("diagnostic tests are present");

    for variant in variants {
        assert!(
            diagnostic_tests.contains(&format!("CompleteCause::{variant}")),
            "CompleteCause::{variant} has no diagnostic test case"
        );
    }
}

/// FR-031-AC-6 (TC-109): a run request has no `package` member (FR-493), so
/// `extraction-package-conflict` has no producer and is retired from the
/// catalogue and its documentation; `invalid-quire-context` stays, produced
/// by the clause-run request reader.
#[ix_trace_rs::trace("TC-109", "FR-031-AC-6", "FR-031-AC-7")]
#[test]
#[ignore = "QSL-2203: extraction-package-conflict is retired with the request package member; not implemented yet"]
fn extraction_package_conflict_is_retired_and_invalid_quire_context_stays() {
    assert_eq!(Code::from_code("extraction-package-conflict"), None);
    assert!(!DOCUMENTATION.contains("| extraction-package-conflict |"));
    assert_eq!(
        Code::from_code("invalid-quire-context"),
        Some(Code::InvalidQuireContext)
    );
}
