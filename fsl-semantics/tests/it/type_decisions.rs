// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-680: indexing, tuple position reads, source maps, the general `Some`
//! constructor, quantity scaling and the equality-sum refusal (FSpec FR-142,
//! FR-143, FR-144, FR-146, FR-149).

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

const UNITS: &str =
    "dimension T;\nunit s : T = rational(1, 1);\nunit ms : T = rational(1, 1000) * s;\n";

fn unit_text(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}\n"
    )
}

/// S1 to S3 under `CheckMode::Linked`; S1 and S2 must accept the unit.
fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = unit_text(declarations);
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "type-decisions"),
        "type-decisions.native",
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

fn assert_checks(declarations: &str) {
    if let Err(refusals) = check(declarations) {
        panic!("{declarations} checks: {refusals:?}");
    }
}

/// The one refusal of `declarations`, with its code and cause.
fn assert_refused(declarations: &str, code: Code, cause: &str) -> CheckRefusal {
    let refusals = check(declarations).expect_err("the unit is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
    refusal.clone()
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-31")]
#[test]
#[ignore = "QSL-2172: indexing a sequence, ordered set or projected field not implemented yet"]
fn an_index_of_a_sequence_or_ordered_set_reads_the_element_type() {
    assert_checks("function f using v(s: Sequence<Integer>[1, 3]): Integer pure { s[0] }");
    assert_checks(
        "record R { xs: Sequence<Integer>[1, 2]; }\n\
         function g using v(r: R): Integer pure { r.xs[0] }",
    );
    assert_checks(
        "function h using v(o: OrderedSet<Integer>[0, 3]): Integer pure \
         { if size(o) >= 2 then o[1] else 0 }",
    );
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-32")]
#[test]
#[ignore = "QSL-2172: index obligation and index refusals not implemented yet"]
fn an_unproved_or_ineligible_index_is_refused() {
    assert_refused(
        "function f using v(s: Sequence<Integer>[0, 3]): Integer pure { s[0] }",
        Code::UndefinedExpression,
        "unproved-presence",
    );
    assert_refused(
        "function f using v(t: Set<Integer>[0, 3]): Integer pure { t[0] }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(n: Integer): Integer pure { n[0] }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(s: Sequence<Integer>[1, 3]): Integer pure { s[true] }",
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1821", "FR-680-AC-2", "FSpec-FR-143-AC-22")]
#[test]
#[ignore = "QSL-2171: tuple position read p[n] not implemented yet"]
fn a_tuple_position_is_read_with_a_literal_index() {
    assert_checks(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair): Boolean pure { p[0] }\n\
         function g using v(p: Pair): Integer pure { p[1] }",
    );
    assert_refused(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair): Integer pure { p[2] }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair, i: Int[0, 1]): Integer pure { p[i] }",
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1821", "FR-680-AC-2", "FSpec-FR-143-AC-22")]
#[test]
fn tuple_position_index_checks_static_type_constraints() {
    assert_refused(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair): Integer pure { p[2] }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair, i: Int[0, 1]): Integer pure { p[i] }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(n: Integer): Integer pure { n[0] }",
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1821", "FR-680-AC-2", "FSpec-FR-143-AC-22")]
#[test]
fn a_dotted_tuple_position_is_a_syntax_error() {
    let text = unit_text(
        "tuple Pair(Boolean, Integer);\n\
         function f using v(p: Pair): Boolean pure { p.0 }",
    );
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "type-decisions"),
        "type-decisions.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    );
    let admissible = parsed.as_ref().is_ok_and(|parsed| parsed.is_admissible());
    assert!(!admissible, "p.0 is invalid_syntax at S1");
}

#[trace("TC-1822", "FR-680-AC-3", "FSpec-FR-144-AC-33")]
#[test]
#[ignore = "QSL-2171: the source Map<K, V> type and its reads not implemented yet"]
fn a_source_map_type_checks_with_its_guarded_read() {
    assert_checks(
        "function f using v(m: Map<Int[0, 3], Integer>[0, 4], k: Int[0, 3]): Integer pure \
         { if contains(m, k) then m[k] else 0 }",
    );
    assert_refused(
        "function f using v(m: Map<Int[0, 3], Integer>[0, 4], k: Int[0, 3]): Integer pure { m[k] }",
        Code::UndefinedExpression,
        "unproved-presence",
    );
    assert_refused(
        "record Cfg { a: Boolean; }\n\
         function f using v(m: Map<Cfg, Integer>): Boolean pure { true }",
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21", "FSpec-FR-375-AC-17")]
#[test]
#[ignore = "QSL-2169: a Some payload checked against the expected element type not implemented yet"]
fn some_takes_the_expected_bounded_element_type() {
    assert_checks("function f using v(): Option<Int[0, 9]> pure { Some(3) }");
    assert_checks("function g using v(y: Option<Int[0, 9]>): Boolean pure { Some(3) = y }");
    assert_refused(
        "function f using v(): Option<Int[0, 9]> pure { Some(30) }",
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace(
    "TC-1824",
    "FR-680-AC-5",
    "FSpec-FR-142-AC-41",
    "FSpec-TC-1326",
    "FSpec-FR-322-AC-451"
)]
#[test]
#[ignore = "QSL-2168: quantity scaling by an exact number not implemented yet"]
fn a_quantity_scaled_by_a_number_keeps_its_unit() {
    let graph = check(&format!(
        "{UNITS}function f using v(d: Quantity<ms>): Quantity<ms> pure {{ d * 2 }}\n\
         function g using v(d: Quantity<ms>): Quantity<ms> pure {{ 2 * d }}\n\
         function h using v(d: Quantity<ms>): Quantity<ms> pure {{ d / 2 }}"
    ))
    .unwrap_or_else(|refusals| panic!("the scalings check: {refusals:?}"));
    let rendered = format!("{graph:?}");
    assert!(
        rendered.contains("filament.op.quantity.scale"),
        "{rendered}"
    );
    assert!(
        rendered.contains("filament.op.quantity.scale_div"),
        "{rendered}"
    );
}

#[trace("TC-1824", "FR-680-AC-5", "FSpec-FR-142-AC-42")]
#[test]
#[ignore = "QSL-2168: quantity scaling refusals not implemented yet"]
fn an_unproved_or_ineligible_scaling_is_refused() {
    assert_refused(
        &format!("{UNITS}function f using v(d: Quantity<ms>, k: Int[0, 9]): Quantity<ms> pure {{ d / k }}"),
        Code::UndefinedExpression,
        "unproved-nonzero",
    );
    assert_refused(
        &format!("{UNITS}function f using v(d: Quantity<ms>): Boolean pure {{ 2 / d = d }}"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1825", "FR-680-AC-6", "FSpec-FR-149-AC-29")]
#[test]
#[ignore = "QSL-1986: the unproved-range refusal does not name the interval, bound and operand yet"]
fn an_equality_sum_refusal_names_its_bound_and_the_join_is_explicit() {
    let refusal = assert_refused(
        "function f using v(a: Int[0, 10], b: Int[0, 10], c: Int[0, 10]): Boolean pure { c = a + b }",
        Code::UndefinedExpression,
        "unproved-range",
    );
    let rendered = format!("{refusal:?}");
    assert!(rendered.contains("[0, 20]"), "{rendered}");
    assert!(rendered.contains("Int[0, 10]"), "{rendered}");
    assert_checks(
        "function g using v(a: Int[0, 10], b: Int[0, 10], c: Int[0, 10]): Boolean pure \
         { convert<Integer>(c) = a + b }",
    );
}
