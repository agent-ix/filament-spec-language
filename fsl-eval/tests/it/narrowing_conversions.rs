// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked standalone narrowing preserves exact values and domain backstops.
use filament_eval::{Evaluation, FamilyOutcome};
use filament_exact::{Decimal, Integer, Meter, Outcome, Rational, Refusal, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName, S6aRewardEvaluator};
use fsl_foundation::SourceIdentity;
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

fn package(source: &str, mode: CheckMode) -> CheckedPackage {
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{source}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "narrowing"),
        "narrowing.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "narrowing-test".to_owned(),
        fsl_forms::build_unit(&parsed).unwrap(),
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap();
    CheckedPackage::link(
        declarations
            .check(mode, CheckingLimits::default(), &S6aRewardEvaluator)
            .unwrap(),
    )
}

fn call(package: &CheckedPackage, value: Value) -> Evaluation {
    package
        .call(
            &QualifiedName::unqualified("f").unwrap(),
            vec![value],
            &ObjectEnvironment::default(),
            &mut Meter::new(super::quantity_literals::UNLIMITED),
        )
        .unwrap()
}

#[trace("FR-093-AC-67", "FSpec-FR-149")]
#[test]
fn guarded_narrowing_evaluates_the_same_integer_at_both_endpoints() {
    let package = package("function f using v(n: Integer): Int[0,9] pure { if n >= 0 and n <= 9 then convert<Int[0,9]>(n) else 0 }", CheckMode::Linked);
    for n in [0_i64, 5, 9] {
        let result = call(&package, Value::Integer(Integer::from(n)));
        assert!(result.losses.is_empty());
        match result.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
                assert_eq!(value, Integer::from(n))
            }
            other => panic!("expected unchanged integer {n}: {other:?}"),
        }
    }
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn kernel_narrowing_refuses_both_outside_integer_endpoints() {
    let package = package(
        "function f using v(n: Integer): Int[0,9] pure { convert<Int[0,9]>(n) }",
        CheckMode::Kernel,
    );
    for n in [-1_i64, 10] {
        let result = call(&package, Value::Integer(Integer::from(n)));
        assert!(result.location.is_some());
        match result.outcome {
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { target })) => {
                assert_eq!(target.lower(), &Integer::from(0_i64));
                assert_eq!(target.upper(), &Integer::from(9_i64));
            }
            other => panic!("expected domain refusal for {n}: {other:?}"),
        }
    }
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn rational_narrowing_checks_normalized_parts_and_refuses_each_bad_part() {
    let package = package("function f using v(x: Rational[-20,20;1,20]): Rational[0,10;1,10] pure { convert<Rational[0,10;1,10]>(x) }", CheckMode::Kernel);
    let normalized = Rational::new(Integer::from(-6_i64), Integer::from(-4_i64)).unwrap();
    let result = call(&package, Value::Rational(normalized.clone()));
    assert!(result.losses.is_empty());
    match result.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(value))) => {
            assert_eq!(value, normalized)
        }
        other => panic!("expected 3/2: {other:?}"),
    }
    for (n, d) in [(11_i64, 1_i64), (1, 11), (-1, 1)] {
        let result = call(
            &package,
            Value::Rational(Rational::new(Integer::from(n), Integer::from(d)).unwrap()),
        );
        assert!(
            matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Refused(Refusal::RationalOutOfDomain { .. }))
            ),
            "{n}/{d}: {:?}",
            result.outcome
        );
    }
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn decimal_narrowing_preserves_values_without_rounding_or_clamping() {
    let package = package("function f using v(d: Decimal[-20,20;0,0;nearest-even]): Decimal[0,9;0,0;nearest-even] pure { convert<Decimal[0,9;0,0;nearest-even]>(d) }", CheckMode::Kernel);
    let result = call(
        &package,
        Value::Decimal(Decimal::new(Integer::from(5_i64), 0)),
    );
    assert!(result.losses.is_empty());
    match result.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Decimal(value))) => {
            assert_eq!(value.normalized().coefficient(), &Integer::from(5_i64))
        }
        other => panic!("expected decimal 5: {other:?}"),
    }
    let result = call(
        &package,
        Value::Decimal(Decimal::new(Integer::from(10_i64), 0)),
    );
    assert!(
        matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::DecimalOutOfDomain { .. }))
        ),
        "{:?}",
        result.outcome
    );
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn rational_to_integer_requires_integrality_without_rounding() {
    let bounded = package(
        "function f using v(x: Rational[0,20;1,10]): Int[0,9] pure { convert<Int[0,9]>(x) }",
        CheckMode::Kernel,
    );
    match call(
        &bounded,
        Value::Rational(Rational::from_integer(Integer::from(5_i64))),
    )
    .outcome
    {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
            assert_eq!(value, Integer::from(5_i64))
        }
        other => panic!("expected integer 5: {other:?}"),
    }
    let fraction =
        || Value::Rational(Rational::new(Integer::from(3_i64), Integer::from(2_i64)).unwrap());
    assert!(matches!(
        call(&bounded, fraction()).outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { .. }))
    ));
    let unbounded = package(
        "function f using v(x: Rational[0,20;1,10]): Integer pure { convert<Integer>(x) }",
        CheckMode::Kernel,
    );
    match call(&unbounded, fraction()).outcome {
        FamilyOutcome::FamilyEvaluated(filament_eval::FamilyResult::Refused(cause)) => {
            assert_eq!(cause.catalog_code().code(), "undefined_expression");
            assert_eq!(cause.catalog_code().cause(), "unproved-range");
        }
        other => panic!("expected an unmet integrality obligation: {other:?}"),
    }
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn integer_range_narrowing_uses_rational_and_decimal_target_domains() {
    let rational = package(
        "function f using v(n: Integer): Rational[0,9;1,1] pure { convert<Rational[0,9;1,1]>(n) }",
        CheckMode::Kernel,
    );
    match call(&rational, Value::Integer(Integer::from(5_i64))).outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(value))) => {
            assert_eq!(value, Rational::from_integer(Integer::from(5_i64)))
        }
        other => panic!("expected exact 5/1: {other:?}"),
    }
    assert!(matches!(
        call(&rational, Value::Integer(Integer::from(10_i64))).outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::RationalOutOfDomain { .. }))
    ));
    let decimal = package("function f using v(n: Integer): Decimal[0,9;0,0;nearest-even] pure { convert<Decimal[0,9;0,0;nearest-even]>(n) }", CheckMode::Kernel);
    let result = call(&decimal, Value::Integer(Integer::from(5_i64)));
    assert!(result.losses.is_empty());
    match result.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Decimal(value))) => {
            assert_eq!(value.normalized().coefficient(), &Integer::from(5_i64))
        }
        other => panic!("expected decimal 5: {other:?}"),
    }
    assert!(matches!(
        call(&decimal, Value::Integer(Integer::from(10_i64))).outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::DecimalOutOfDomain { .. }))
    ));
}

#[trace("FR-093-AC-67", "FR-096-AC-8", "FSpec-FR-149")]
#[test]
fn scale_zero_decimal_to_integer_is_exact_and_bounded_narrowing_checks_its_domain() {
    let exact = package(
        "function f using v(d: Decimal[-20,20;0,0;exact]): Integer pure { convert<Integer>(d) }",
        CheckMode::Linked,
    );
    let narrowed = package(
        "function f using v(d: Decimal[-20,20;0,0;exact]): Int[0,9] pure { convert<Int[0,9]>(d) }",
        CheckMode::Kernel,
    );
    for n in [-1_i64, 0, 5, 9, 10] {
        let decimal = || Value::Decimal(Decimal::new(Integer::from(n), 0));
        let result = call(&exact, decimal());
        assert!(result.losses.is_empty());
        match result.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
                assert_eq!(value, Integer::from(n));
            }
            other => panic!("expected exact integer {n}: {other:?}"),
        }
        let result = call(&narrowed, decimal());
        assert!(result.losses.is_empty());
        if (0..=9).contains(&n) {
            match result.outcome {
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
                    assert_eq!(value, Integer::from(n));
                }
                other => panic!("expected narrowed integer {n}: {other:?}"),
            }
        } else {
            match result.outcome {
                FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain {
                    target,
                })) => {
                    assert_eq!(target.lower(), &Integer::from(0_i64));
                    assert_eq!(target.upper(), &Integer::from(9_i64));
                }
                other => panic!("expected integer domain refusal for {n}: {other:?}"),
            }
        }
    }
}
