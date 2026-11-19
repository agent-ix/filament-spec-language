// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit conversion classes use declared bounds; guards prove obligations.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn check(source: &str, mode: CheckMode) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{source}");
    let parsed = fsl_cst::parse(SourceIdentity::new("test", "narrowing"), "narrowing.native", text.as_bytes(), fsl_cst::Limits::default()).unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    PackageDeclarations::assemble(parsed.source().reference().clone(), crate::TEST_PACKAGE_NAME.to_owned(), fsl_forms::build_unit(&parsed).unwrap(), SelectedModels::default(), Vec::new()).unwrap().check(mode, CheckingLimits::default(), &crate::reward_evaluator::ZeroRewardEvaluator)
}

fn assert_operation(source: &str, identity: &str, mode: CheckMode) {
    let graph = check(source, mode).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    let operations: Vec<_> = graph.semantic_graph().nodes().filter_map(|node| {
        match node.body() {
            fsl_semantics::check::BodyTerm::Application(application) => Some(application.operation.identity().to_owned()),
            _ => None,
        }
    }).collect();
    assert!(operations.iter().any(|operation| operation == identity), "expected {identity}, got {operations:?}");
}

fn assert_unproved(source: &str) {
    let errors = check(source, CheckMode::Linked).expect_err("narrowing needs proof");
    assert_eq!(errors[0].cause.code(), Code::UndefinedExpression, "{errors:?}");
    assert_eq!(errors[0].cause.cause(), Some("unproved-range"), "{errors:?}");
}

#[trace("FR-093-AC-4", "FSpec-FR-149")]
#[test]
fn guarded_int_narrowing_checks_and_lowers_to_numeric_narrow() {
    assert_operation("function f using v(q: Int[0,20]): Int[0,9] pure { if q <= 9 then convert<Int[0,9]>(q) else 0 }", "filament.op.numeric.narrow", CheckMode::Linked);
}

#[trace("FR-096", "FSpec-FR-149")]
#[test]
fn unguarded_int_narrowing_refuses_unproved_range() {
    assert_unproved("function f using v(q: Int[0,20]): Int[0,9] pure { convert<Int[0,9]>(q) }");
}
