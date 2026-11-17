// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-980 (TC-2771, TC-2772): an annotated `let` checks its initializer at
//! the declared type and gives the binder that type.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

/// S1 to S3 of `header` and `declarations` under `mode`; S1 and S2 must
/// accept the unit.
fn check_in(
    header: &str,
    declarations: &str,
    mode: CheckMode,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{header}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "w17"),
        "w17.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(mode, CheckingLimits::default())
}

const V: &str = "profile v = \"filament.value.complete/v1\";\n";

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_in(V, declarations, CheckMode::Linked)
}

fn assert_checks(declarations: &str) {
    if let Err(refusals) = check(declarations) {
        panic!("{declarations} checks: {refusals:?}");
    }
}

/// The first refusal of `declarations` has `code` and `cause`.
fn assert_refused_with(
    header: &str,
    declarations: &str,
    code: Code,
    cause: Option<&str>,
) -> CheckRefusal {
    let refusals =
        check_in(header, declarations, CheckMode::Linked).expect_err("the unit is refused");
    let first = refusals.first().expect("a refusal").clone();
    assert_eq!(first.cause.code(), code, "{declarations}: {refusals:?}");
    if cause.is_some() {
        assert_eq!(first.cause.cause(), cause, "{declarations}: {refusals:?}");
    }
    first
}

fn assert_refused(declarations: &str, code: Code, cause: &str) -> CheckRefusal {
    assert_refused_with(V, declarations, code, Some(cause))
}

/// FR-980-AC-3: the annotation carries the range obligation of an arithmetic
/// initializer and keeps it where the unannotated form loses it.
#[trace("TC-2771", "FR-980-AC-3", "FSpec-TC-1004", "FSpec-FR-501-AC-3")]
#[test]
fn an_annotated_let_gives_its_declared_type_to_the_bound_expression() {
    assert_checks("function f using v(y: Int[0,2]): Integer pure { let x: Int[0,3] = y + 1 in x }");
    assert_refused(
        "function f using v(y: Int[0,3]): Integer pure { let x: Int[0,3] = y + 1 in x }",
        Code::UndefinedExpression,
        "unproved-range",
    );
    assert_checks(
        "function g using v(p: Int[0, 9], q: Int[0, 9]): Int[0, 81] pure \
         { let line: Int[0, 81] = p * q in line }",
    );
    assert_refused(
        "function g using v(p: Int[0, 9], q: Int[0, 9]): Int[0, 81] pure { let line = p * q in line }",
        Code::UndefinedExpression,
        "unproved-range",
    );
}

/// FR-980-AC-4: a non-conforming initializer, an option at its annotation
/// and an unresolved annotation.
#[trace("TC-2771", "FR-980-AC-4")]
#[test]
fn an_annotated_let_refuses_an_initializer_that_does_not_conform() {
    assert_refused(
        "function b using v(): Boolean pure { let x: Boolean = 1 in x }",
        Code::IllTyped,
        "type-mismatch",
    );
    assert_checks(
        "function o using v(): Boolean pure { let o: Option<Int[0, 9]> = none in present(o) }",
    );
    let refusals = check("function u using v(): Integer pure { let x: Undeclared = 1 in x }")
        .expect_err("an unresolved annotation refuses");
    assert!(
        format!("{refusals:?}").contains("Undeclared"),
        "{refusals:?}"
    );
}

/// FR-980-AC-5: the formatted source checks as the original does.
#[trace("TC-2772", "FR-980-AC-5")]
#[test]
#[ignore = "QSL-1974 integration: the canonical formatter is not on this branch yet"]
fn a_formatted_annotated_let_checks_as_the_original() {
    let body = "function f using v(y: Int[0,2]): Integer pure { let x:Int[0,3]=y+1 in x }";
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{V}{body}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "w17"),
        "w17.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let formatted = fsl_cst::format::format(&parsed).expect("formats");
    let declarations = formatted
        .split_once("filament.value.complete/v1\";\n")
        .expect("the header")
        .1;
    assert_checks(body);
    assert_checks(declarations);
}
