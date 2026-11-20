// SPDX-License-Identifier: AGPL-3.0-or-later
//! Current checked-body entry oracles. These exercise the std interpreter;
//! they do not claim an independent no_std checked-term implementation.

use filament_eval::{Evaluation, FamilyOutcome};
use filament_exact::{
    evaluate_integer_arithmetic, AdmittedCharge, Integer, IntegerArithmetic, Meter, Outcome, Value,
    ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{
    evaluate_closed_with_frame, evaluate_protocol_slot, CheckedPackageEvaluation,
};
use fsl_forms::{BinaryOperator, Expression};
use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

use super::quantity_literals::package_with;

#[derive(Debug, PartialEq)]
struct Record {
    outcome: String,
    charges: Vec<AdmittedCharge>,
}

impl Record {
    fn new(outcome: &Outcome<Value>, meter: &Meter) -> Self {
        assert!(
            !meter.charge_log_truncated(),
            "the entire charge log is required"
        );
        Self {
            outcome: format!("{outcome:?}"),
            charges: meter.admitted_charges().to_vec(),
        }
    }
}

fn completed(evaluation: Evaluation) -> Outcome<Value> {
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("expected a kernel outcome: {:?}", evaluation.outcome);
    };
    outcome
}

/// A node's frame is the caller's actual arguments. The independent kernel
/// result and complete charge records detect wrong binding, arithmetic or
/// metering; the two FSL entries are never called independent evaluators.
#[trace("FR-406")]
#[test]
fn closed_and_protocol_bodies_observe_arguments_and_kernel_charges() {
    let package = package_with("function keep using v(x: Integer): Integer pure { x }")
        .expect("the real source package checks");
    let expression = package
        .graph()
        .check_expression(
            vec![
                ("a".to_owned(), ValueType::Integer),
                ("b".to_owned(), ValueType::Integer),
            ],
            &Expression::binary(
                BinaryOperator::Add,
                Expression::name("a".to_owned()),
                Expression::name("b".to_owned()),
            ),
            Some(&ValueType::Integer),
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the real expression checks against the package");
    for (a, b, sum) in [(2_i64, 3_i64, 5_i64), (-8, 3, -5), (0, 0, 0)] {
        let a = Integer::from(a);
        let b = Integer::from(b);
        let arguments = vec![Value::Integer(a.clone()), Value::Integer(b.clone())];
        let mut kernel_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let Outcome::Completed(expected) =
            evaluate_integer_arithmetic(IntegerArithmetic::Add(&a, &b), None, &mut kernel_meter)
        else {
            panic!("the direct unbounded kernel addition completes");
        };
        let expected = Outcome::Completed(Value::Integer(expected));
        assert_eq!(
            format!("{expected:?}"),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(sum)))
            )
        );
        let expected = Record::new(&expected, &kernel_meter);
        assert!(
            !expected.charges.is_empty(),
            "the arithmetic must actually run"
        );
        let mut closed_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let closed = evaluate_closed_with_frame(
            &package,
            expression.root(),
            expression.slots(),
            arguments.clone(),
            expression.formed_units(),
            &mut closed_meter,
        )
        .expect("the checked closed body runs");
        assert_eq!(Record::new(&completed(closed), &closed_meter), expected);
        let mut protocol_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let protocol = evaluate_protocol_slot(
            &package,
            expression.body(),
            arguments.into_iter().map(Some).collect(),
            &ObjectEnvironment::default(),
            &mut protocol_meter,
        )
        .expect("the checked protocol body runs");
        assert_eq!(Record::new(&completed(protocol), &protocol_meter), expected);
        let mut public_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let public = package
            .evaluate(
                &expression,
                vec![Value::Integer(a), Value::Integer(b)],
                &ObjectEnvironment::default(),
                &mut public_meter,
            )
            .expect("the checked expression admits its arguments");
        assert_eq!(Record::new(&completed(public), &public_meter), expected);
        // Negative controls change actual captured data, rather than
        // manufacturing another evaluator or a second claimed implementation.
        let mut changed_result = Record {
            outcome: expected.outcome.clone(),
            charges: expected.charges.clone(),
        };
        changed_result.outcome = "wrong result".to_owned();
        assert_ne!(changed_result, expected);
        let mut changed_charge = Record {
            outcome: expected.outcome.clone(),
            charges: expected.charges.clone(),
        };
        changed_charge.charges.remove(0);
        assert_ne!(changed_charge, expected);
    }
}

#[trace("FR-049-AC-2")]
#[test]
fn checked_expression_argument_refusals_leave_the_evaluation_meter_unchanged() {
    use filament_eval::CallFailure;
    use filament_semantic_value::call::InputRefusal;

    let package = package_with("function keep using v(x: Integer): Integer pure { x }")
        .expect("the source package checks");
    let expression = package
        .graph()
        .check_expression(
            vec![("x".to_owned(), ValueType::Integer)],
            &Expression::binary(
                BinaryOperator::Add,
                Expression::name("x".to_owned()),
                Expression::integer(1_i64),
            ),
            Some(&ValueType::Integer),
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the arithmetic body checks");
    for (arguments, expected) in [
        (
            Vec::new(),
            InputRefusal::Arity {
                declared: 1,
                supplied: 0,
            },
        ),
        (
            vec![
                Value::Integer(Integer::from(1_i64)),
                Value::Integer(Integer::from(2_i64)),
            ],
            InputRefusal::Arity {
                declared: 1,
                supplied: 2,
            },
        ),
        (
            vec![Value::Boolean(true)],
            InputRefusal::WrongValueKind { parameter: 0 },
        ),
    ] {
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let before = meter.clone();
        let actual = package
            .evaluate(
                &expression,
                arguments,
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .expect_err("invalid arguments must be refused before the addition");
        assert_eq!(actual, CallFailure::Input(expected));
        assert_eq!(
            meter, before,
            "admission must leave every evaluation counter and charge unchanged"
        );
    }
}
