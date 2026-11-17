// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-135-AC-6 (TC-540): reference lifting reaches nested collection elements.

use fsl_foundation::Code;
use ix_trace_rs::trace;

use super::refinement_declarations::{
    NESTED_IMPL, NESTED_SPEC, check_with, nested_models, nested_refinement, only, refused,
};

/// FR-135-AC-6: a sequence of concrete references maps element by element to
/// the abstract sequence; whole populations and top-level references remain
/// accepted, while a malformed sequence row names its field in the refusal.
#[trace("TC-540", "FR-135-AC-6")]
#[test]
fn nested_reference_rows_are_lifted_and_malformed_rows_are_refused() {
    let (spec, concrete) = nested_models();
    let models = [
        ("Q", NESTED_SPEC, &spec[..]),
        ("R", NESTED_IMPL, &concrete[..]),
    ];
    let accepted = check_with(
        "",
        &models,
        &nested_refinement("head = self; chain = self.chain; cells = R::slots;"),
    )
    .unwrap_or_else(|found| panic!("nested mapping checks: {found:?}"));
    assert_eq!(accepted.refinements().len(), 1);

    let malformed = refused(
        &models,
        &nested_refinement("head = self; chain = self.size; cells = R::slots;"),
    );
    let row = only(&malformed);
    assert_eq!(
        row.code_and_cause(),
        (Code::IllTyped, Some("type-mismatch"))
    );
    assert_eq!(row.at, "self.size");
    assert!(
        format!("{:?}", row.refusal).contains("Queue.chain"),
        "{row:?}"
    );
}
