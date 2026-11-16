// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-670 and FR-495-AC-17/AC-18: the integer-division law a unit header
//! selects, and the interval and nonzero obligation of `div`, `rem` and
//! `mod` checked from source.

use filament_exact::DivisionProfile;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, CheckingLimitKind, Obligation, PackageDeclarations,
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
    .map_err(|refusal| {
        let codes = refusal.errors.iter().map(|error| error.cause.catalog_code().to_string()).collect::<Vec<_>>().join(", ");
        format!("{codes}: {refusal:?}")
    })
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

#[test]
#[trace("TC-2188", "FR-767-AC-1")]
fn a_large_remainder_keeps_its_exact_range_or_reports_the_work_limit() {
    let headers = format!("{VALUE}{}", law_header("d", "euclidean"));
    let body = "function big using v(a: Int[1000000007,1000000007], b: Int[2,1000000006]): Int[1,500000003] pure { a rem b }";
    assemble(&headers, body).unwrap().check(CheckMode::Linked, CheckingLimits::default()).unwrap();
    let denied = assemble(&headers, body).unwrap().check(CheckMode::Linked, CheckingLimits::default().with_work_budget(10000)).unwrap_err();
    assert_eq!(denied.len(), 1, "{denied:?}");
    let CheckCause::ResourceExhausted(limit) = &denied[0].cause else { panic!("{denied:?}") };
    assert_eq!(limit.kind, fsl_semantics::check::CheckingLimitKind::WorkBudget);
    assert_eq!(limit.limit, 10000);
    assert_eq!(limit.actual, 10001);
    let too_narrow = body.replace("Int[1,500000003] pure", "Int[2,500000003] pure");
    let refused = assemble(&headers, &too_narrow).unwrap().check(CheckMode::Linked, CheckingLimits::default()).unwrap_err();
    assert_eq!(pairs(&refused), [("undefined_expression".to_owned(), Some("unproved-range"))]);
}

#[trace("TC-1780", "FR-670-AC-1", "FSpec-TC-1280", "FSpec-FR-570-AC-1")]
#[test]
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
#[ignore = "QSL-2002: a mod node is typed Integer instead of its remainder interval"]
fn a_mod_node_is_typed_by_its_divisor_interval() {
    for source in [
        "function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }",
        "function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }",
    ] {
        check(VALUE, source, CheckMode::Linked)
            .unwrap_or_else(|refusals| panic!("{source}: {refusals:?}"));
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

/// `true` when `refusals` is one `unproved-range` refusal.
fn only_range(refusals: &[CheckRefusal]) -> bool {
    refusals.len() == 1
        && matches!(
            refusals[0].cause,
            CheckCause::Unproved(Obligation::Range { .. })
        )
}

/// `function f using v(<parameters>): Int[<lo>, <hi>] pure { <body> }`.
fn into(parameters: &str, lo: i64, hi: i64, body: &str) -> String {
    format!("function f using v({parameters}): Int[{lo}, {hi}] pure {{ {body} }}")
}

/// The remainder checks into `Int[lo, hi]` and refuses `unproved-range` into
/// `Int[lo + 1, hi]` and `Int[lo, hi - 1]`: the interval is exact.
fn exact(headers: &str, parameters: &str, body: &str, lo: i64, hi: i64) {
    let fits = into(parameters, lo, hi, body);
    check(headers, &fits, CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("{fits}: {refusals:?}"));
    for narrow in [
        into(parameters, lo + 1, hi, body),
        into(parameters, lo, hi - 1, body),
    ] {
        let refusals = check(headers, &narrow, CheckMode::Linked)
            .expect_err("the narrowed interval misses a remainder");
        assert!(only_range(&refusals), "{narrow}: {refusals:?}");
    }
}

#[trace("TC-2210", "FR-495-AC-19", "FSpec-TC-1660", "FSpec-FR-147-AC-14")]
#[test]
#[ignore = "QSL-2002: the exact smallest remainder interval is not computed yet"]
fn a_remainder_node_carries_its_exact_smallest_interval() {
    for (parameters, lo, hi) in [
        ("a: Int[0, 2], b: Int[5, 6]", 0, 2),
        ("a: Int[91, 91], b: Int[2, 12]", 0, 7),
        ("a: Int[97, 97], b: Int[2, 12]", 1, 9),
        ("a: Int[-7, -7], b: Int[3, 5]", 1, 3),
    ] {
        exact(VALUE, parameters, "a mod b", lo, hi);
    }
    let truncating = format!("{VALUE}{}", law_header("d", "truncating"));
    exact(
        &truncating,
        "a: Int[-7, -7], b: Int[3, 5]",
        "a rem b",
        -3,
        -1,
    );
    let floor = format!("{VALUE}{}", law_header("d", "floor"));
    exact(&floor, "a: Int[7, 7], b: Int[-5, -3]", "a rem b", -3, -1);

    let guarded = "if b != 0 then a mod b else 0";
    let bounded = into("a: Int[0, 9], b: Integer", 0, 9, guarded);
    check(VALUE, &bounded, CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("{bounded}: {refusals:?}"));
    let unbounded = into("a: Int[-1, 9], b: Integer", 0, 1000, guarded);
    let refusals = check(VALUE, &unbounded, CheckMode::Linked)
        .expect_err("a negative dividend by an unbounded divisor is unbounded");
    assert!(only_range(&refusals), "{unbounded}: {refusals:?}");
}

#[trace("TC-2211", "FR-495-AC-20", "FSpec-TC-1660", "FSpec-FR-147-AC-14")]
#[test]
#[ignore = "QSL-2002: the remainder run walk and its work-ceiling charge are not implemented yet"]
fn the_remainder_walk_is_bounded_by_the_checking_work_ceiling() {
    let parameters = "a: Int[1000000007, 1000000007], b: Int[2, 1000000006]";
    let big = into(parameters, 1, 500_000_003, "a mod b");
    check(VALUE, &big, CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the default ceiling fits the walk: {refusals:?}"));
    let raised = into(parameters, 2, 500_000_003, "a mod b");
    let refusals =
        check(VALUE, &raised, CheckMode::Linked).expect_err("the minimum remainder is 1");
    assert!(only_range(&refusals), "{refusals:?}");

    let refusals = assemble(VALUE, &big)
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal}"))
        .check(
            CheckMode::Linked,
            CheckingLimits::default().with_work_budget(10_000),
        )
        .expect_err("the walk needs more than 10000 runs");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let CheckCause::ResourceExhausted(limit) = &refusals[0].cause else {
        panic!("the work stage limit, never a wider type: {refusals:?}");
    };
    assert_eq!(limit.kind, CheckingLimitKind::WorkBudget, "{limit:?}");
    assert_eq!(limit.limit, 10_000, "{limit:?}");
    // Located at the `mod`, the root of `f`'s body.
    assert!(
        matches!(
            &refusals[0].location.origin,
            filament_semantic_value::location::ExpressionOwner::Body { function, .. }
                if function == "f"
        ),
        "{refusals:?}"
    );
    assert!(refusals[0].location.path().is_empty(), "{refusals:?}");
}
