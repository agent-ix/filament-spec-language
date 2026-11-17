// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093 and FR-140: `Some(value)` is a checked present option value.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use filament_semantic_value::location::ExpressionOwner as Origin;
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckCause, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn check(
    declarations: &str,
) -> Result<fsl_semantics::check::CheckedGraph, Vec<fsl_semantics::check::CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "option-values"),
        "option-values.native",
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
    .check(CheckMode::Linked, CheckingLimits::default())
}

fn assert_checks(declaration: &str) {
    check(declaration).unwrap_or_else(|refusals| panic!("{declaration}: {refusals:?}"));
}

fn assert_refused(declaration: &str, code: Code, cause: &str) {
    let refusals = check(declaration).expect_err(declaration);
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal for {declaration}: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_literal_checks_as_an_option_int_result() {
    assert_checks("function f using v(): Option<Int[0, 9]> pure { Some(3) }");
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_literal_checks_as_an_equality_operand() {
    assert_checks(
        "function f using v(x: Option<Int[0, 9]>): Boolean pure { x = Some(3) }",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_literal_out_of_range_refuses() {
    assert_refused(
        "function f using v(): Option<Int[0, 9]> pure { Some(30) }",
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_of_a_decimal_literal_payload_checks_against_a_decimal_option() {
    assert_checks(
        "function f using v(): Option<Decimal[0, 9; 0, 0; exact]> pure { Some(decimal(3, 0)) }",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_with_a_payload_of_the_wrong_kind_refuses() {
    assert_refused(
        "function f using v(): Option<Int[0, 9]> pure { Some(true) }",
        Code::IllTyped,
        "type-mismatch",
    );
}

/// TC-1273's S3 negative vector: Some(Integer) does not match Option<Boolean>.
#[trace("TC-1273")]
#[test]
fn some_with_a_payload_outside_the_element_type_refuses_at_the_call() {
    let error = check("function bad using v(x: Integer): Option<Boolean> pure { Some(x) }")
        .expect_err("the payload is not a Boolean");
    let [refusal] = error.as_slice() else {
        panic!("one typed refusal: {error:?}");
    };
    assert_eq!(refusal.cause.code(), Code::IllTyped);
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    assert!(matches!(
        &refusal.cause,
        CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
    ));
    assert!(matches!(
        &refusal.location.origin,
        Origin::Body { function, index: 0, .. } if function == "bad"
    ));
    assert!(
        refusal.location.path().is_empty(),
        "Some(x) is the root body expression: {:?}",
        refusal.location
    );
}

/// FSpec FR-143-AC-20 and FR-986-AC-1..3: `null` reaches S3 as an expression
/// and is refused as ill-typed, including when compared with an option value.
#[trace(
    "TC-2214",
    "FSpec-FR-143-AC-20",
    "FSpec-FR-986-AC-1",
    "FSpec-FR-986-AC-2",
    "FSpec-FR-986-AC-3"
)]
#[test]
fn a_null_expression_is_ill_typed_at_s3() {
    for declaration in [
        "function bad using v(): Option<Integer> pure { null }",
        "function bad using v(a: Option<Integer>): Boolean pure { a = null }",
    ] {
        let error = check(declaration).expect_err("null is not an option value");
        let [refusal] = error.as_slice() else {
            panic!("one typed refusal for {declaration}: {error:?}");
        };
        assert_eq!(refusal.cause.code(), Code::IllTyped, "{declaration}");
        assert_eq!(
            refusal.cause.cause(),
            Some("type-mismatch"),
            "{declaration}"
        );
        assert!(matches!(
            &refusal.cause,
            CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
        ));
        assert!(matches!(
            &refusal.location.origin,
            Origin::Body { function, index: 0, .. } if function == "bad"
        ));
    }
}

/// FR-091-AC-76 (TC-2214): `none` is a second spelling of `absent`, and
/// `result` builds everywhere and is judged by check, not refused at S2.
#[trace("TC-2214", "FR-091-AC-76")]
#[test]
fn none_and_result_build_and_check_decides_their_position() {
    for declaration in [
        "function o using v(): Option<Integer> pure { none }",
        "function a using v(): Option<Integer> pure { absent }",
    ] {
        check(declaration)
            .unwrap_or_else(|refusals| panic!("{declaration}: the empty option: {refusals:?}"));
    }
    for (declaration, code, cause) in [
        (
            "function n using v(): Integer pure { none }",
            Code::IllTyped,
            "type-mismatch",
        ),
        (
            "function l using v(): Boolean pure { let x = none in true }",
            Code::IllTyped,
            "ambiguous-literal",
        ),
        (
            "function r using v(): Integer pure { result }",
            Code::WrongSnapshot,
            "wrong-anchor",
        ),
    ] {
        let refusals = check(declaration).expect_err(declaration);
        assert!(
            refusals.iter().any(|refusal| refusal.cause.code() == code
                && refusal.cause.cause() == Some(cause)),
            "{declaration}: {refusals:?}"
        );
        assert!(
            refusals
                .iter()
                .all(|refusal| refusal.cause.code() != Code::UnsupportedConstruct),
            "{declaration}: {refusals:?}"
        );
    }
}
