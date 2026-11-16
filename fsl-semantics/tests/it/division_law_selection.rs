// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-670 and FR-495-AC-17/AC-18: the integer-division law a unit header
//! selects, and the interval and nonzero obligation of `div`, `rem` and
//! `mod` checked from source.

use filament_exact::{DivisionProfile, Integer, IntegerInterval, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, Obligation, PackageDeclarations,
};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

const VALUE: &str = "profile v = \"filament.value.complete/v1\";\n";

fn law_header(alias: &str, law: &str) -> String {
    format!("profile {alias} = \"filament.value.integer-division.{law}/v1\";\n")
}

/// The assembled declarations of a unit with `headers` and `declarations`,
/// with no caller-supplied division law; `Err` renders an assembly refusal.
fn assemble(headers: &str, declarations: &str) -> Result<PackageDeclarations, String> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{headers}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "division-law"),
        "division-law.native",
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
    .map_err(|refusal| format!("{refusal:?}"))
}

fn check(
    headers: &str,
    declarations: &str,
    mode: CheckMode,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    assemble(headers, declarations)
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal}"))
        .check(mode, CheckingLimits::default())
}

/// `(code, cause)` of each refusal.
fn pairs(refusals: &[CheckRefusal]) -> Vec<(String, Option<&'static str>)> {
    refusals
        .iter()
        .map(|refusal| {
            (
                refusal.cause.code().as_str().to_owned(),
                refusal.cause.cause(),
            )
        })
        .collect()
}

fn only_nonzero(refusals: &[CheckRefusal]) {
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        matches!(refusals[0].cause, CheckCause::Unproved(Obligation::Nonzero)),
        "{refusals:?}"
    );
}

const DIV_REM: &str = "function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n\
                       function r using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a rem b }";

#[trace("TC-1780", "FR-670-AC-1", "FSpec-TC-1280", "FSpec-FR-570-AC-1")]
#[test]
#[ignore = "QSL-1989: a header profile naming the integer-division law is not resolved yet"]
fn a_law_header_admits_div_and_rem_under_that_law() {
    for (law, profile) in [
        ("euclidean", DivisionProfile::Euclidean),
        ("floor", DivisionProfile::Floor),
        ("truncating", DivisionProfile::Truncating),
    ] {
        let headers = format!("{VALUE}{}", law_header("d", law));
        let declarations = assemble(&headers, DIV_REM)
            .unwrap_or_else(|refusal| panic!("{law}: assembly succeeds: {refusal}"));
        let selected = declarations
            .integer_division
            .as_ref()
            .unwrap_or_else(|| panic!("{law}: the header selects the law"));
        assert_eq!(selected.profile(), profile);
        declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{law}: {refusals:?}"));
    }
}

#[trace("TC-1780", "FR-670-AC-2", "FSpec-TC-1280", "FSpec-FR-570-AC-2")]
#[test]
fn div_and_rem_with_no_law_refuse_missing_selection() {
    let refusals = check(VALUE, DIV_REM, CheckMode::Linked).expect_err("no law is selected");
    assert_eq!(
        pairs(&refusals),
        [
            ("missing_declaration".to_owned(), Some("missing-selection")),
            ("missing_declaration".to_owned(), Some("missing-selection")),
        ]
    );
    check(
        VALUE,
        "function m using v(a: Int[-7, 7], b: Int[1, 3]): Integer pure { a mod b }",
        CheckMode::Linked,
    )
    .expect("a mod-only unit needs no law");
}

#[trace("TC-1781", "FR-670-AC-3", "FSpec-TC-1280", "FSpec-FR-570-AC-3")]
#[test]
#[ignore = "QSL-1989: two law headers naming different laws are not refused yet"]
fn two_different_law_headers_refuse_conflicting_definition() {
    let headers = format!(
        "{VALUE}{}{}",
        law_header("d1", "floor"),
        law_header("d2", "truncating")
    );
    let refused = match assemble(&headers, DIV_REM) {
        Err(refusal) => refusal,
        Ok(declarations) => format!(
            "{:?}",
            pairs(
                &declarations
                    .check(CheckMode::Linked, CheckingLimits::default())
                    .expect_err("two laws are refused")
            )
        ),
    };
    assert!(
        refused.contains("conflicting-definition") || refused.contains("ConflictingDefinition"),
        "{refused}"
    );
    let same = format!(
        "{VALUE}{}{}",
        law_header("d1", "floor"),
        law_header("d2", "floor")
    );
    let declarations = assemble(&same, DIV_REM).expect("one law named twice");
    assert_eq!(
        declarations
            .integer_division
            .as_ref()
            .map(|law| law.profile()),
        Some(DivisionProfile::Floor)
    );
}

#[trace("TC-1781", "FR-670-AC-4", "FSpec-TC-1280", "FSpec-FR-570-AC-4")]
#[test]
#[ignore = "QSL-1989: a law alias in a using position is not refused wrong-selection-role yet"]
fn a_law_alias_in_using_refuses_wrong_selection_role() {
    let headers = format!("{VALUE}{}", law_header("d", "floor"));
    let outcome = assemble(
        &headers,
        "function q using d(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }",
    )
    .and_then(|declarations| {
        declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .map_err(|refusals| format!("{:?}", pairs(&refusals)))
    });
    let refused = outcome.expect_err("a law alias governs no declaration");
    assert!(refused.contains("wrong-selection-role"), "{refused}");
}

#[trace("TC-1783", "FR-495-AC-17", "FSpec-TC-1281", "FSpec-FR-147-AC-13")]
#[test]
#[ignore = "QSL-1962: Linked records no nonzero obligation for an integer mod yet"]
fn a_mod_divisor_that_may_be_zero_refuses_unproved_nonzero() {
    let source = "function m0 using v(a: Int[0, 100], b: Int[0, 12]): Int[0, 11] pure { a mod b }";
    only_nonzero(&check(VALUE, source, CheckMode::Linked).expect_err("b may be zero"));
    check(VALUE, source, CheckMode::Kernel).expect("Kernel records no nonzero obligation");
}

#[trace("TC-1783", "FR-495-AC-17", "FSpec-TC-1281", "FSpec-FR-147-AC-12")]
#[test]
fn a_mod_node_is_typed_by_its_divisor_interval() {
    let graph = check(
        VALUE,
        "function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }\n\
         function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }\n\
         function m4 using v(a: Int[0, 2], b: Int[5, 5]): Int[0, 2] pure { a mod b }\n\
         function m5 using v(a: Int[0, 2], b: Int[5, 6]): Int[0, 2] pure { a mod b }\n\
         function s3 using v(a: Int[-7, -7], b: Int[3, 3]): Int[2, 2] pure { a mod b }",
        CheckMode::Linked,
    )
    .expect("modulo's typed interval admits each bounded result");
    for (index, lower, upper) in [
        (0, 0_i64, 11_i64),
        (1, 0_i64, 11_i64),
        (2, 0_i64, 2_i64),
        (3, 0_i64, 2_i64),
        (4, 2_i64, 2_i64),
    ] {
        let function = graph
            .function_state(index)
            .unwrap_or_else(|| panic!("function at index {index} is checked"));
        let expected = ValueType::Int(IntegerInterval::spanning(
            Integer::from(lower),
            Integer::from(upper),
        ));
        assert_eq!(function.body.value_type(), &expected, "{}", function.name);
    }

    let refusals = check(
        VALUE,
        "function m3 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 10] pure { a mod b }",
        CheckMode::Linked,
    )
    .expect_err("[0, 11] is not inside [0, 10]");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        matches!(
            refusals[0].cause,
            CheckCause::Unproved(Obligation::Range { .. })
        ),
        "{refusals:?}"
    );
}

#[trace(
    "TC-1783",
    "FR-495-AC-18",
    "FSpec-TC-1281",
    "FSpec-FR-147-AC-12",
    "FSpec-FR-147-AC-13"
)]
#[test]
#[ignore = "QSL-1962: div and rem from source need the header law and their interval and nonzero obligation"]
fn div_and_rem_carry_their_interval_and_nonzero_obligation_under_each_law() {
    for law in ["euclidean", "floor", "truncating"] {
        let headers = format!("{VALUE}{}", law_header("d", law));
        let wide = "function r1 using v(a: Int[-100, 100], b: Int[1, 12]): Int[-11, 11] pure { a rem b }\n\
                    function q1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 100] pure { a div b }";
        check(&headers, wide, CheckMode::Linked)
            .unwrap_or_else(|refusals| panic!("{law}: {refusals:?}"));
        let narrow = check(
            &headers,
            "function r2 using v(a: Int[-100, 100], b: Int[1, 12]): Int[0, 11] pure { a rem b }",
            CheckMode::Linked,
        );
        assert_eq!(narrow.is_ok(), law != "truncating", "{law}: {narrow:?}");
        for zero in [
            "function r0 using v(a: Int[-100, 100], b: Int[0, 12]): Int[-11, 11] pure { a rem b }",
            "function q0 using v(a: Int[0, 100], b: Int[0, 12]): Int[0, 100] pure { a div b }",
        ] {
            only_nonzero(&check(&headers, zero, CheckMode::Linked).expect_err(zero));
        }
    }
}
