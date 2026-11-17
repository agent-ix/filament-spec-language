// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-680-AC-1: collection index typing and presence obligations.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "collection-index"),
        "collection-index.native",
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

fn assert_refused(declarations: &str, code: Code, cause: &str) {
    let refusals = check(declarations).expect_err("the unit is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-31")]
#[test]
#[ignore = "Blocked by IR-1491: checked collection.index has no FSL producer yet"]
fn index_with_a_proved_in_range_position_checks_with_the_element_type() {
    let result = check("function f using v(s: Sequence<Integer>[1, 3]): Integer pure { s[0] }");
    assert!(result.is_ok(), "{result:?}");
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-32")]
#[test]
fn index_with_an_unproved_position_refuses() {
    assert_refused(
        "function f using v(s: Sequence<Integer>[0, 3]): Integer pure { s[0] }",
        Code::UndefinedExpression,
        "unproved-presence",
    );
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-32")]
#[test]
fn index_of_a_set_or_bag_refuses() {
    for kind in ["Set", "Bag"] {
        assert_refused(
            &format!("function f using v(s: {kind}<Integer>[0, 3]): Integer pure {{ s[0] }}"),
            Code::IllTyped,
            "operator-ineligible",
        );
    }
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-32")]
#[test]
fn index_with_a_non_integer_position_refuses() {
    assert_refused(
        "function f using v(s: Sequence<Integer>[1, 3]): Integer pure { s[true] }",
        Code::IllTyped,
        "type-mismatch",
    );
}
