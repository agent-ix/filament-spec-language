// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-983 (TC-2778, TC-2779): `Some` is the present-option constructor in
//! every position; every other call name stays an ordinary call.
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

/// FR-983-AC-1: `Some` checks as the present-option constructor in a function
/// and a predicate body.
#[trace("TC-2778", "FR-983-AC-1")]
#[test]
fn some_checks_in_function_and_predicate_bodies() {
    assert_checks(
        "function s using v(b: Int[0, 9]): Option<Int[0, 9]> pure { Some(b) }\n\
         function p using v(b: Integer): Boolean pure { present(Some(b)) }",
    );
}

/// FR-983-AC-2: `some`, `Something` and an unknown `Somex` are ordinary
/// calls.
#[trace("TC-2778", "FR-983-AC-2")]
#[test]
fn calls_named_like_some_are_ordinary_calls() {
    assert_checks(
        "function some using v(x: Integer): Integer pure { x }\n\
         function g using v(): Integer pure { some(1) }",
    );
    assert_checks(
        "function Something using v(x: Integer): Integer pure { x }\n\
         function h using v(): Integer pure { Something(1) }",
    );
    let refusals = check("function k using v(): Integer pure { Somex(1) }")
        .expect_err("an unresolved call refuses");
    assert!(format!("{refusals:?}").contains("Somex"), "{refusals:?}");
}

/// FR-983-AC-3: another argument count refuses at the call.
#[trace("TC-2779", "FR-983-AC-3", "FSpec-TC-1005", "FSpec-FR-501-AC-4")]
#[test]
fn some_with_another_argument_count_refuses() {
    for body in ["Some(b, b)", "Some()"] {
        assert_refused(
            &format!("function w using v(b: Integer): Option<Integer> pure {{ {body} }}"),
            Code::IllTyped,
            "type-mismatch",
        );
    }
}
