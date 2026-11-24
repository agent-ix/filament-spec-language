// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103: a domain-package operation's kept `requires` and `ensures` are
//! checked as Boolean clauses (TC-993) over the ConfigVersion package.

use filament_semantic_value::call::WrongSnapshotCause;
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use filament_semantic_value::location::{ExpressionOwner as Origin, OperationClauseKind};
use fsl_semantics::check::{CheckCause, CheckRefusal};
use fsl_semantics::model::refusal::{MemberPath, MemberRoot};
use ix_trace_rs::trace;
use serde_json::json;

use super::model_operations::{
    admit_and_assemble_with_body, config_version_document_with_operations, empty_frame, operation,
    operation_parameter, PACKAGE_IDENTITY,
};

/// The one function the unit declares: a unit needs a declaration, and no
/// unit state clause names the operation.
const UNIT_BODY: &str = "function same using v(a: Boolean): Boolean pure { a }";

/// Checks a package whose `bump` operation holds `requires` and `ensures`
/// (`language`, `text`) and no unit clause.
fn check(
    requires: &[(&str, &str)],
    ensures: &[(&str, &str)],
) -> Result<fsl_semantics::check::CheckedGraph, Vec<CheckRefusal>> {
    let mut bump = operation("bump", json!([]), None, empty_frame());
    let clauses = |entries: &[(&str, &str)]| {
        entries
            .iter()
            .map(|(language, text)| {
                json!({
                    "language": language,
                    "text": text,
                    "origin": bump["origin"].clone(),
                })
            })
            .collect::<Vec<_>>()
    };
    let (pre, post) = (clauses(requires), clauses(ensures));
    bump["pre"] = json!(pre);
    bump["post"] = json!(post);
    let document = config_version_document_with_operations(vec![bump]);
    admit_and_assemble_with_body(&document, UNIT_BODY)
        .expect("the package assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
}

fn check_optional_parameter_clause(
    clause: &str,
) -> Result<fsl_semantics::check::CheckedGraph, Vec<CheckRefusal>> {
    check_optional_parameter_clause_with_type(clause, "ix://filament/native/Integer")
}

fn check_optional_parameter_clause_with_type(
    clause: &str,
    type_ref: &str,
) -> Result<fsl_semantics::check::CheckedGraph, Vec<CheckRefusal>> {
    let mut bump = operation(
        "bump",
        json!([operation_parameter("bump", "amount", type_ref)]),
        None,
        empty_frame(),
    );
    bump["params"][0]["presence"] = json!("optional");
    bump["pre"] = json!([{
        "language": "quire",
        "text": clause,
        "origin": bump["origin"].clone(),
    }]);
    let document = config_version_document_with_operations(vec![bump]);
    admit_and_assemble_with_body(&document, UNIT_BODY)
        .expect("the optional-parameter package assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
}

fn bump_node() -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/bump")
}

/// Clauses that check yield a package, add no state clause, and a clause in
/// another language is kept and neither parsed nor checked.
#[trace("FR-103-AC-7", "FR-103-AC-8", "TC-993")]
#[test]
fn kept_operation_clauses_that_check_add_no_state_clause() {
    let graph = check(
        &[
            ("quire", "self.versionNumber >= 0"),
            ("acme:rules", "not an expression ((("),
        ],
        &[("quire", "self.versionNumber = pre(self.versionNumber)")],
    )
    .expect("a Boolean requires and an ensures that reads pre check");
    assert!(
        graph.state_clauses().is_empty(),
        "a kept clause adds no checked state clause"
    );
}

/// A `requires` that reads `pre(e)` refuses `wrong_snapshot`/
/// `forbidden-pre-read`, located at `Operation.pre[k]` and naming the
/// operation.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn a_requires_that_reads_pre_refuses_at_its_position() {
    let refusals = check(
        &[
            ("quire", "self.versionNumber >= 0"),
            ("quire", "pre(self.versionNumber) = self.versionNumber"),
        ],
        &[],
    )
    .expect_err("the second requires reads pre");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert!(matches!(
        refusal.cause.as_ref(),
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    ));
    assert_eq!(refusal.operation_node(), Some(bump_node().as_str()));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(1))
    );
    assert!(matches!(
        &refusal.location.origin,
        Origin::ModelOperationClause {
            kind: OperationClauseKind::Requires,
            position: 1,
            ..
        }
    ));
}

/// An `ensures` that does not check is located at `Operation.post[k]`.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn an_ensures_that_is_not_boolean_refuses_at_its_position() {
    let refusals =
        check(&[], &[("quire", "self.versionNumber")]).expect_err("an ensures must be Boolean");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(refusal.operation_node(), Some(bump_node().as_str()));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("post").index(0))
    );
}

/// A kept clause names no `using` profile, so the unit's selected
/// complete-value profile plays no part: integer `div` refuses
/// `profile_prohibited_operator`.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn a_kept_clause_is_checked_under_the_profile_independent_defaults() {
    let refusals = check(&[("quire", "self.versionNumber div 2 = 0")], &[])
        .expect_err("integer div is profile prohibited in a kept clause");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert!(matches!(
        refusal.cause.as_ref(),
        CheckCause::ProfileProhibitedOperator(_)
    ));
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(0))
    );
}

/// FR-103-AC-12: an optional operation parameter is typed as an option, so
/// its presence can be checked directly.
#[trace("FR-103-AC-12")]
#[test]
fn present_over_an_optional_parameter_checks() {
    check_optional_parameter_clause("present(amount)")
        .expect("presence of the optional parameter checks");
}

/// FR-103-AC-12: a presence guard proves the value of an optional parameter.
#[trace("FR-103-AC-12")]
#[test]
fn guarded_value_of_an_optional_parameter_checks() {
    check_optional_parameter_clause("present(amount) implies value(amount) >= 0")
        .expect("the guard proves the optional parameter is present");
}

/// FR-103-AC-12: reading an optional parameter without a presence proof
/// refuses at the value expression with `unproved-presence`.
#[trace("FR-103-AC-12")]
#[test]
fn value_of_an_optional_parameter_without_a_presence_guard_refuses_unproved_presence() {
    let refusals = check_optional_parameter_clause("value(amount) >= 0")
        .expect_err("an unguarded value read must refuse");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(
        refusal.cause.code(),
        fsl_foundation::diagnostic::Code::UndefinedExpression
    );
    assert_eq!(refusal.cause.cause(), Some("unproved-presence"));
    assert_eq!(refusal.location.path(), vec![0]);
}

/// FR-103-AC-12: `deref` requires a reference option, so it cannot be applied
/// to an optional integer parameter.
#[trace("FR-103-AC-12")]
#[test]
fn deref_over_an_optional_parameter_refuses_type_mismatch() {
    let refusals = check_optional_parameter_clause("deref(amount) >= 0")
        .expect_err("deref of an optional integer must refuse");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(
        refusal.cause.code(),
        fsl_foundation::diagnostic::Code::IllTyped
    );
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    assert_eq!(refusal.location.path(), vec![0]);
}

/// FR-103-AC-12: after `value` establishes the reference, `deref` over an
/// optional reference parameter checks under the same presence guard.
#[trace("FR-103-AC-12")]
#[test]
fn deref_value_of_an_optional_reference_parameter_checks() {
    check_optional_parameter_clause_with_type(
        "present(amount) implies deref(value(amount)).versionNumber >= 0",
        &format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
    )
    .expect("the guarded option value dereferences as a model reference");
}
