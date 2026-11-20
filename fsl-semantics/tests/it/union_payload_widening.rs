// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-317-AC-8 (TC-3134): a union-member construction gives a payload
//! argument the verdict a function-call argument gets at the member's
//! declared payload type.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

const UNIT: &str = "profile v = \"filament.value.complete/v1\";\n\
    union Reading { Raw(Integer), Wide(Int[0, 2000]), Tiny(Int[0, 9]) }\n\
    function t using v(a: Int[0, 9]): Boolean pure { true }\n";

/// S1 to S3 of [`UNIT`] followed by `declarations`.
fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{UNIT}{declarations}\n");
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
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
}

/// `declarations` is refused `undefined_expression`/`unproved-range`.
fn assert_unproved_range(declarations: &str) {
    let refusals = check(declarations).expect_err("the unit is refused");
    let first = refusals.first().expect("a refusal");
    assert_eq!(
        first.cause.code(),
        Code::UndefinedExpression,
        "{refusals:?}"
    );
    assert_eq!(first.cause.cause(), Some("unproved-range"), "{refusals:?}");
}

/// A payload argument that does not fit its member's payload type is refused
/// as the function call `t(..)` refuses the same argument, an `if` that
/// narrows it first is admitted as it is for the call, and a payload of
/// another type stays a type mismatch.
#[trace("TC-3134", "FR-317-AC-8")]
#[test]
fn a_union_payload_that_does_not_fit_gets_the_function_call_verdict() {
    for call in ["t(w)", "t(n)"] {
        assert_unproved_range(&format!(
            "function f using v(w: Int[0, 20], n: Integer): Boolean pure {{ {call} }}"
        ));
    }
    for construction in ["Reading::Tiny(w)", "Reading::Tiny(n)"] {
        assert_unproved_range(&format!(
            "function f using v(w: Int[0, 20], n: Integer): Reading pure {{ {construction} }}"
        ));
    }

    check("function f using v(w: Int[0, 20]): Boolean pure { if w <= 9 then t(w) else false }")
        .expect("the call whose argument the condition narrows is admitted");
    check(
        "function f using v(w: Int[0, 20], n: Integer): Reading pure \
         { if w <= 9 then Reading::Tiny(w) else Reading::Raw(n) }",
    )
    .expect("the construction whose argument the condition narrows is admitted");

    let refusals = check("function f using v(): Reading pure { Reading::Tiny(true) }")
        .expect_err("a Boolean is not an integer payload");
    let first = refusals.first().expect("a refusal");
    assert_eq!(first.cause.code(), Code::IllTyped, "{refusals:?}");
    assert_eq!(first.cause.cause(), Some("type-mismatch"), "{refusals:?}");
}
