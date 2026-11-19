// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit conversion classes use declared bounds; guards prove obligations.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn check(source: &str, mode: CheckMode) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{source}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "narrowing"),
        "narrowing.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        fsl_forms::build_unit(&parsed).unwrap(),
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap()
    .check(
        mode,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
}

fn assert_operation(source: &str, identity: &str, mode: CheckMode) -> CheckedGraph {
    let graph = check(source, mode).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    let operations: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.body() {
            fsl_semantics::check::BodyTerm::Application(application) => {
                let operation = application.operation.identity();
                if operation == identity {
                    assert_eq!(application.arguments.len(), 1);
                    assert!(application.operation.member().is_some());
                    if identity != "filament.op.numeric.convert_rounding" {
                        assert!(application.operation.mode().is_none());
                    }
                }
                Some(operation.to_owned())
            }
            _ => None,
        })
        .collect();
    assert!(
        operations.iter().any(|operation| operation == identity),
        "expected {identity}, got {operations:?}"
    );
    if identity.ends_with(".narrow") {
        assert!(
            !operations
                .iter()
                .any(|operation| operation == "filament.op.numeric.convert"),
            "explicit narrowing must retain its class: {operations:?}"
        );
    }
    graph
}

fn assert_parameter_narrow(graph: &CheckedGraph) {
    use fsl_semantics::check::{BodyTerm, LeafTerm, MemberTerm};
    let parameter = graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "parameter")
        .expect("one parameter")
        .key();
    assert!(graph.semantic_graph().nodes().any(|node| {
        let BodyTerm::Application(application) = node.body() else { return false; };
        application.operation.identity() == "filament.op.numeric.narrow"
            && matches!(application.arguments.as_slice(), [MemberTerm::Leaf(LeafTerm::Reference { target })] if target.admitted() == Some(&parameter))
    }), "the authored narrowing must wrap the parameter, independently of the else literal");
}

fn assert_unproved(source: &str) {
    let errors = check(source, CheckMode::Linked).expect_err("narrowing needs proof");
    assert_eq!(
        errors[0].cause.code(),
        Code::UndefinedExpression,
        "{errors:?}"
    );
    assert_eq!(
        errors[0].cause.cause(),
        Some("unproved-range"),
        "{errors:?}"
    );
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn guarded_int_narrowing_checks_and_lowers_to_numeric_narrow() {
    let graph = assert_operation("function f using v(q: Int[0,20]): Int[0,9] pure { if q <= 9 then convert<Int[0,9]>(q) else 0 }", "filament.op.numeric.narrow", CheckMode::Linked);
    assert_parameter_narrow(&graph);
}

#[trace("FR-093-AC-67", "FR-096", "FSpec-FR-149")]
#[test]
fn unguarded_int_narrowing_refuses_unproved_range() {
    assert_unproved("function f using v(q: Int[0,20]): Int[0,9] pure { convert<Int[0,9]>(q) }");
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn integer_source_under_guard_narrows() {
    let graph = assert_operation("function f using v(n: Integer): Int[0,9] pure { if n >= 0 and n <= 9 then convert<Int[0,9]>(n) else 0 }", "filament.op.numeric.narrow", CheckMode::Linked);
    assert_parameter_narrow(&graph);
    assert_unproved("function f using v(n: Integer): Int[0,9] pure { convert<Int[0,9]>(n) }");
    assert_unproved("function f using v(n: Integer): Int[0,9] pure { if n <= 9 then convert<Int[0,9]>(n) else 0 }");
    assert_unproved("function f using v(n: Integer): Int[0,9] pure { if n >= 0 then convert<Int[0,9]>(n) else 0 }");
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn rational_narrowing_lowers_to_rational_narrow() {
    assert_operation("function f using v(): Rational[0,10;1,10] pure { let x: Rational[-20,20;1,20] = rational(-6,-4) in convert<Rational[0,10;1,10]>(x) }", "filament.op.rational.narrow", CheckMode::Linked);
    assert_unproved("function f using v(x: Rational[0,20;1,10]): Rational[0,10;1,10] pure { convert<Rational[0,10;1,10]>(x) }");
    assert_unproved("function f using v(): Rational[0,10;1,10] pure { let x: Rational[0,20;1,20] = rational(1,11) in convert<Rational[0,10;1,10]>(x) }");
    assert_unproved("function f using v(): Rational[0,10;1,10] pure { let x: Rational[0,20;1,20] = rational(11,1) in convert<Rational[0,10;1,10]>(x) }");
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn decimal_to_integer_is_exact_or_range_narrowing() {
    assert_operation(
        "function f using v(d: Decimal[0,9;0,0;exact]): Integer pure { convert<Integer>(d) }",
        "filament.op.numeric.convert",
        CheckMode::Linked,
    );
    assert_operation("function f using v(): Int[0,9] pure { let d: Decimal[-20,20;0,0;exact] = decimal(5,0) in convert<Int[0,9]>(d) }", "filament.op.numeric.narrow", CheckMode::Linked);
    assert_unproved(
        "function f using v(d: Decimal[-20,20;0,0;exact]): Int[0,9] pure { convert<Int[0,9]>(d) }",
    );
    // The current FR-149 table admits only scale-zero decimal sources into integers.
    let errors = check(
        "function f using v(d: Decimal[0,900;2,2;exact]): Integer pure { convert<Integer>(d) }",
        CheckMode::Linked,
    )
    .unwrap_err();
    assert_eq!(errors[0].cause.code(), Code::IllTyped);
    assert_eq!(errors[0].cause.cause(), Some("operator-ineligible"));
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn range_narrowing_covers_integer_rational_and_decimal_targets() {
    for (source, target, value) in [
        ("Int[0,20]", "Rational[0,9;1,1]", "5"),
        ("Integer", "Decimal[0,9;0,0;nearest-even]", "5"),
        ("Rational[0,20;1,1]", "Int[0,9]", "rational(5,1)"),
        (
            "Rational[0,20;1,1]",
            "Decimal[0,9;0,0;exact]",
            "rational(5,1)",
        ),
        (
            "Decimal[0,20;0,0;exact]",
            "Rational[0,9;1,1]",
            "decimal(5,0)",
        ),
        (
            "Decimal[0,20;0,0;exact]",
            "Rational[5,9;1,1]",
            "decimal(5,0)",
        ),
        (
            "Decimal[0,900;2,2;exact]",
            "Rational[0,9;1,10]",
            "decimal(500,2)",
        ),
        (
            "Decimal[0,90;1,1;exact]",
            "Decimal[1,9;0,1;exact]",
            "decimal(5,1)",
        ),
        (
            "Decimal[0,20;0,0;nearest-even]",
            "Decimal[0,9;0,0;nearest-even]",
            "decimal(5,0)",
        ),
    ] {
        assert_operation(&format!("function f using v(): {target} pure {{ let x: {source} = {value} in convert<{target}>(x) }}"), "filament.op.numeric.narrow", CheckMode::Linked);
        assert_unproved(&format!(
            "function f using v(x: {source}): {target} pure {{ convert<{target}>(x) }}"
        ));
    }
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn scale_reduction_remains_rounding_and_exact_widening_remains_exact() {
    assert_operation("function f using v(d: Decimal[0,900;2,2;exact]): Decimal[0,9;0,0;nearest-even] pure { convert<Decimal[0,9;0,0;nearest-even]>(d) }", "filament.op.numeric.convert_rounding", CheckMode::Linked);
    assert_operation("function f using v(x: Rational[0,9;1,10]): Decimal[0,90;1,1;nearest-even] pure { convert<Decimal[0,90;1,1;nearest-even]>(x) }", "filament.op.numeric.convert_rounding", CheckMode::Linked);
    assert_operation(
        "function f using v(x: Int[0,9]): Int[0,20] pure { convert<Int[0,20]>(x) }",
        "filament.op.numeric.convert",
        CheckMode::Linked,
    );
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn direct_equality_narrowing_stays_refused_but_a_let_conversion_checks() {
    let errors = check("function f using v(n: Int[0,20], m: Int[0,9]): Boolean pure { if n <= 9 then convert<Int[0,9]>(n) = m else false }", CheckMode::Linked).unwrap_err();
    assert_eq!(errors[0].cause.code(), Code::IllTyped);
    assert_operation("function f using v(n: Int[0,20], m: Int[0,9]): Boolean pure { if n <= 9 then let x = convert<Int[0,9]>(n) in x = m else false }", "filament.op.numeric.narrow", CheckMode::Linked);
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn float_to_integer_narrowing_stays_refused() {
    let errors = check(
        "function f using v(f: Float64): Int[0,9] pure { convert<Int[0,9]>(f) }",
        CheckMode::Linked,
    )
    .unwrap_err();
    assert_eq!(errors[0].cause.code(), Code::IllTyped);
}

#[trace("FR-093-AC-67", "FR-096", "FSpec-FR-149")]
#[test]
fn historical_narrowing_corpus_now_reports_unproved_range() {
    for source in [
        include_str!("../../../tests/corpus/values/conv-narrow-2007.native"),
        include_str!("../../../tests/corpus/values/conv-narrow-noguard.native"),
        include_str!("../../../tests/corpus/values/conv-rat-narrow.native"),
    ] {
        let declarations = source
            .split_once("filament.value.complete/v1\";")
            .unwrap()
            .1;
        assert_unproved(declarations);
    }
}

#[trace("FR-093-AC-67", "FR-062-AC-13", "FSpec-FR-149")]
#[test]
fn explicit_narrowing_of_an_application_keys_the_direct_conversion_context() {
    use filament_exact::{Integer, ValueType};
    use fsl_semantics::check::{BodyTerm, LeafTerm, MemberTerm};
    let graph = check("function f using v(n: Integer): Int[0,9] pure { if n >= 0 and n <= 8 then convert<Int[0,9]>(n + 1) else 0 }", CheckMode::Linked).unwrap();
    let additions: Vec<_> = graph.requirements().iter().filter(|(key, _)| {
        matches!(graph.semantic_graph().node(key.node().clone()).unwrap().body(), BodyTerm::Application(application) if application.operation.identity() == "filament.op.integer.add")
    }).collect();
    assert_eq!(additions.len(), 1);
    let (key, record) = additions[0];
    let ValueType::Int(bound) = record.result_bound().value_type() else {
        panic!("the explicit narrow supplies the bound");
    };
    assert_eq!(bound.lower(), &Integer::from(0_i64));
    assert_eq!(bound.upper(), &Integer::from(9_i64));
    let context = record
        .conversion_context()
        .expect("the direct explicit conversion occurrence");
    let narrow = graph.semantic_graph().node(context.node().clone()).unwrap();
    let BodyTerm::Application(application) = narrow.body() else {
        panic!("a narrow application");
    };
    assert_eq!(
        application.operation.identity(),
        "filament.op.numeric.narrow"
    );
    assert_eq!(
        record.result_bound().node(),
        narrow.semantic_type().unwrap()
    );
    assert!(
        matches!(application.arguments.as_slice(), [MemberTerm::Leaf(LeafTerm::Reference { target })] if target.admitted() == Some(key.node()))
    );
    assert!(!graph.requirements().iter().any(|(key, _)| matches!(graph.semantic_graph().node(key.node().clone()).unwrap().body(), BodyTerm::Application(application) if application.operation.identity() == "filament.op.numeric.narrow")));
}
