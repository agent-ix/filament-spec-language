// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-986 (TC-2785): `null` outside a record field value builds at S2 and is
//! refused `ill_typed`/`type-mismatch` at check.
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

#[trace("TC-2785", "FR-986-AC-1", "FR-986-AC-2", "FSpec-FR-143-AC-20")]
#[test]
#[ignore = "QSL-2165: S2 refuses null as unrepresented; the check-time type error is not implemented yet"]
fn null_outside_a_record_field_is_ill_typed_at_check() {
    for declarations in [
        "function f using v(): Option<Integer> pure { null }",
        "function g using v(a: Option<Integer>): Boolean pure { a = null }",
        "function h using v(): Integer pure { null }",
        "function k using v(): Boolean pure { let x = null in true }",
    ] {
        assert_refused(declarations, Code::IllTyped, "type-mismatch");
    }
}

#[trace("TC-2785", "FR-986-AC-3")]
#[test]
#[ignore = "QSL-2165: the positive half lands with the null refusal; not run as a gate test yet"]
fn a_null_field_and_the_empty_option_spellings_still_check() {
    assert_checks(
        "record P { a: Integer; b: Integer?; }\n\
         function p using v(): P pure { P { a: 1, b: null } }",
    );
    assert_checks("function o using v(): Option<Integer> pure { none }");
    assert_checks("function a using v(): Option<Integer> pure { absent }");
}
