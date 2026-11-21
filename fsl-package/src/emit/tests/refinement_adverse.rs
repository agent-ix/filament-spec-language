// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1181 slice 6: the adverse corpus of the refinement emission. Each
//! refusal the V2 refinement stack introduced is driven from source and
//! asserted by code, catalog cause and locus.
//!
//! Covered elsewhere, with code, cause and locus: the memory-row refusals
//! (`refinement/protocol/tests.rs`, FR-432-AC-3/22), the catalog's
//! trigger-binder internal fault (`refinement/internal_list.rs`), the
//! FR-375-AC-21 reader joins (`admission_corpus.rs`).

use filament_semantic_value::checking::CheckMode;
use fsl_foundation::Code;
use fsl_semantics::check::{CheckCause, CheckRefusal};

use super::admission_corpus::{rows_refinement, MAPS};
use super::*;

/// The refusals of `unit` over `documents`, which must not check.
fn refusals(unit: &str, documents: &[Vec<u8>]) -> Vec<CheckRefusal> {
    let packages =
        fsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice)).unwrap();
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(|_| ())
        .expect_err("the unit refuses")
}

/// The source text a span names.
fn spelled(unit: &str, start: usize, end: usize) -> &str {
    &unit[start..end]
}

/// A `history` row and its read keep the refinement out of the emission: it
/// refuses `unsupported_construct`/`not-yet-implemented` once, naming the
/// refinement and located at its whole declaration (FR-135).
#[trace("FR-375-AC-14", "TC-310")]
#[test]
fn rows_without_a_node_form_refuse_the_refinement_at_its_declaration() {
    let step = "step C::ConfigVersion::attemptUpdate -> A::ConfigVersion::attemptUpdate(self);";
    let cases = [(
        "history",
        format!(
            "{MAPS}\n{step}\nhistory C::ConfigVersion.last : Int[0, 1] = 0 \
             {{ on C::ConfigVersion::attemptUpdate : 0; }}"
        ),
        vec!["attemptUpdate"],
    )];
    for (case, rows, operations) in cases {
        let (unit, documents) = rows_refinement(&rows, &operations);
        let found = refusals(&unit, &documents);
        let [refusal] = &found[..] else {
            panic!("{case}: one refusal: {found:?}");
        };
        assert_eq!(
            (refusal.cause.code(), refusal.cause.cause()),
            (Code::UnsupportedConstruct, Some("not-yet-implemented")),
            "{case}"
        );
        let CheckCause::RefinementNotEmitted { name, span } = &refusal.cause else {
            panic!("{case}: {refusal:?}");
        };
        assert_eq!(name, "R", "{case}");
        let declaration = spelled(&unit, span.start, span.end);
        assert!(
            declaration.starts_with("refinement R using") && declaration.ends_with('}'),
            "{case}: {declaration}"
        );
    }
}
