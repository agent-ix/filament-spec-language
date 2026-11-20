// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact finite-domain witnesses for checked arithmetic result intervals.
//! The only pending operator is count<N> (QSL-2003); mod is exercised now.

use super::typed_value_case::{body_type, call, compile, exact_integer, inside};
use filament_exact::{Value, ValueType};
use fsl_bench::prop::{
    self,
    value::{Expr, Type, TypedExpression, VALUE_HEADER},
};
use proptest::prelude::{prop_assert, prop_assert_eq, Strategy, TestCaseError};
use proptest::test_runner::TestError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operator {
    Add,
    Subtract,
    Multiply,
}

impl Operator {
    fn apply(self, left: i128, right: i128) -> i128 {
        match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
        }
    }

    fn expression(self, left: Expr, right: Expr) -> Expr {
        match self {
            Self::Add => Expr::Add(Box::new(left), Box::new(right)),
            Self::Subtract => Expr::Subtract(Box::new(left), Box::new(right)),
            Self::Multiply => Expr::Multiply(Box::new(left), Box::new(right)),
        }
    }
}

#[derive(Clone, Debug)]
struct IntervalCase {
    left: (i64, i64),
    right: (i64, i64),
    literal: i64,
    operators: Vec<Operator>,
}

impl IntervalCase {
    fn result(&self, a: i64, b: i64) -> i128 {
        let a = i128::from(a);
        let b = i128::from(b);
        match self.operators.as_slice() {
            [operator] => operator.apply(a, b),
            [first, last] => last.apply(first.apply(a, i128::from(self.literal)), b),
            _ => unreachable!("the strategy creates one or two operators"),
        }
    }

    fn members(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        (self.left.0..=self.left.1)
            .flat_map(move |a| (self.right.0..=self.right.1).map(move |b| (a, b)))
    }

    fn interval(&self) -> (i64, i64) {
        let mut values = self.members().map(|(a, b)| self.result(a, b));
        let first = values.next().expect("both parameter domains are nonempty");
        let (lower, upper) = values.fold((first, first), |(lower, upper), value| {
            (lower.min(value), upper.max(value))
        });
        (
            i64::try_from(lower).expect("bounded reference arithmetic"),
            i64::try_from(upper).expect("bounded reference arithmetic"),
        )
    }

    fn source(&self, lower: i64, upper: i64) -> String {
        let left = Expr::Parameter("a".into());
        let right = Expr::Parameter("b".into());
        let expression = match self.operators.as_slice() {
            [operator] => operator.expression(left, right),
            [first, last] => {
                last.expression(first.expression(left, Expr::Integer(self.literal)), right)
            }
            _ => unreachable!("the strategy creates one or two operators"),
        };
        TypedExpression {
            parameters: vec![
                (
                    "a".into(),
                    Type::BoundedInt {
                        lower: self.left.0,
                        upper: self.left.1,
                    },
                ),
                (
                    "b".into(),
                    Type::BoundedInt {
                        lower: self.right.0,
                        upper: self.right.1,
                    },
                ),
            ],
            result: Type::BoundedInt { lower, upper },
            expression,
        }
        .source()
    }
}

// This strategy supplies interval annotations and finite parameter assignments
// to the landed source AST, rather than replacing its source renderer/config.
fn intervals() -> impl Strategy<Value = IntervalCase> {
    (
        -4_i64..=2,
        0_i64..=3,
        -4_i64..=2,
        0_i64..=3,
        -3_i64..=3,
        proptest::collection::vec(
            proptest::sample::select(vec![Operator::Add, Operator::Subtract, Operator::Multiply]),
            1..=2,
        ),
    )
        .prop_map(|(a, aw, b, bw, literal, operators)| IntervalCase {
            left: (a, a + aw),
            right: (b, b + bw),
            literal,
            operators,
        })
}

fn evaluated_integer(value: Value) -> i128 {
    let Value::Integer(value) = value else {
        panic!("integer source returned {value:?}");
    };
    exact_integer(&value)
}

/// Provenance: QSL-2056. Every member of each small parameter product is run.
#[test]
fn p3_accepted_functions_stay_inside_the_declared_interval() {
    prop::run(
        "p3_accepted_functions_stay_inside_the_declared_interval",
        prop::config(256),
        &intervals(),
        |input| {
            let (lower, upper) = input.interval();
            let source = input.source(lower, upper);
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            let signature = package.graph().callable("f").expect("checked function");
            let checked = body_type(&package, "f");
            for (a, b) in input.members() {
                let value = call(
                    &package,
                    "f",
                    vec![Value::Integer(a.into()), Value::Integer(b.into())],
                )
                .map_err(|error| TestCaseError::fail(format!("{error}\na={a},b={b}\n{source}")))?;
                prop_assert!(
                    inside(&value, signature.result),
                    "{source}\na={a},b={b}\nchecked={:?}\nvalue={value:?}",
                    signature.result
                );
                prop_assert!(
                    inside(&value, &checked),
                    "{source}\na={a},b={b}\nnode={checked:?}\nvalue={value:?}"
                );
                prop_assert_eq!(
                    evaluated_integer(value),
                    input.result(a, b),
                    "{}\na={},b={}",
                    source,
                    a,
                    b
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2056. The rejected result interval excludes a real extremum.
#[test]
fn p3_unproved_range_refusals_are_real() {
    prop::run(
        "p3_unproved_range_refusals_are_real",
        prop::config(256),
        &(intervals(), proptest::bool::ANY),
        |(input, excludes_lower)| {
            let (minimum, maximum) = input.interval();
            let declared = if excludes_lower {
                (minimum + 1, maximum + 1)
            } else {
                (minimum - 1, maximum - 1)
            };
            let source = input.source(declared.0, declared.1);
            let refusals = match compile(&source) {
                Ok(_) => {
                    return Err(TestCaseError::fail(format!(
                        "unsound range admitted\n{source}"
                    )))
                }
                Err(refusals) => refusals,
            };
            prop_assert!(
                refusals
                    .iter()
                    .any(|refusal| refusal.cause.cause() == Some("unproved-range")),
                "wrong refusal: {refusals:?}\n{source}"
            );
            prop_assert!(
                input
                    .members()
                    .any(|(a, b)| input.result(a, b) < i128::from(declared.0)
                        || input.result(a, b) > i128::from(declared.1)),
                "refusal has no actual parameter witness\n{source}"
            );
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2056. A wrong independent upper bound is caught by real runs.
#[test]
fn weakened_rule_is_caught() {
    let mut config = prop::config(256);
    config.failure_persistence = None;
    let result = prop::run("weakened_rule_is_caught", config, &intervals(), |input| {
        if input.operators.last() != Some(&Operator::Add) {
            return Ok(());
        }
        let (lower, upper) = input.interval();
        let source = input.source(lower, upper);
        let package = compile(&source)
            .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
        for (a, b) in input.members() {
            let value = call(
                &package,
                "f",
                vec![Value::Integer(a.into()), Value::Integer(b.into())],
            )
            .map_err(|error| TestCaseError::fail(format!("{error}\n{source}")))?;
            let actual = evaluated_integer(value);
            prop_assert!(
                actual < i128::from(upper),
                "weakened + interval misses value={actual}; a={a}, b={b}\n{source}"
            );
        }
        Ok(())
    });
    let Err(TestError::Fail(reason, minimal)) = result else {
        panic!("the weakened rule must be rejected");
    };
    assert_eq!(minimal.operators, [Operator::Add]);
    assert!(reason.to_string().contains("function f"), "{reason}");
    assert!(reason.to_string().contains("a="), "{reason}");
}

fn modulo_source(a: (i64, i64), b: (i64, i64), result: &str) -> String {
    format!(
        "{VALUE_HEADER}function f using v(a:Int[{},{}],b:Int[{},{}]): {result} pure {{ a mod b }}",
        a.0, a.1, b.0, b.1
    )
}

/// Provenance: QSL-2056. QSL-2002 is complete, so mod is active coverage.
#[test]
fn p4_mod_with_a_zero_free_divisor_stays_in_range() {
    let strategy = (1_i64..=6, proptest::bool::ANY, -8_i64..=8);
    prop::run(
        "p4_mod_with_a_zero_free_divisor_stays_in_range",
        prop::config(256),
        &strategy,
        |(maximum, negative, numerator)| {
            let divisor = if negative {
                (-maximum, -1)
            } else {
                (1, maximum)
            };
            let source = modulo_source((-8, 8), divisor, &format!("Int[0,{}]", maximum - 1));
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            let checked = body_type(&package, "f");
            let ValueType::Int(bounds) = &checked else {
                return Err(TestCaseError::fail(format!(
                    "mod is not given a bounded integer type: {checked:?}\n{source}"
                )));
            };
            prop_assert_eq!(exact_integer(bounds.lower()), 0);
            prop_assert_eq!(exact_integer(bounds.upper()), i128::from(maximum - 1));
            for b in divisor.0..=divisor.1 {
                let value = call(
                    &package,
                    "f",
                    vec![Value::Integer(numerator.into()), Value::Integer(b.into())],
                )
                .map_err(|error| {
                    TestCaseError::fail(format!("{error}\na={numerator},b={b}\n{source}"))
                })?;
                prop_assert!(
                    inside(&value, &checked),
                    "{value:?} outside {checked:?}\n{source}"
                );
                prop_assert_eq!(
                    evaluated_integer(value),
                    i128::from(numerator).rem_euclid(i128::from(b)),
                    "{}\na={},b={}",
                    source,
                    numerator,
                    b
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2056. The whole divisor type includes zero, not only a sample.
#[test]
fn p4_mod_with_a_zero_divisor_is_refused_at_check() {
    prop::run(
        "p4_mod_with_a_zero_divisor_is_refused_at_check",
        prop::config(256),
        &(-6_i64..=0, 0_i64..=6),
        |(lower, upper)| {
            let source = modulo_source((-8, 8), (lower, upper), "Integer");
            let refusals = match compile(&source) {
                Ok(_) => {
                    return Err(TestCaseError::fail(format!(
                        "zero-divisor domain admitted\n{source}"
                    )))
                }
                Err(refusals) => refusals,
            };
            prop_assert!(
                refusals
                    .iter()
                    .any(|refusal| refusal.cause.cause() == Some("unproved-nonzero")),
                "{refusals:?}\n{source}"
            );
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2056. Strict desired check-time refusal; not passing coverage.
#[test]
#[ignore = "QSL-2003: count<N> must prove its declared upper bound at check"]
fn p4_count_over_the_bound_is_refused_at_check() {
    let source = format!("{VALUE_HEADER}type N=Int[0,1]; function f using v(s:Sequence<Integer>[0,4]): N pure {{ count<N>(x in s: true) }}");
    let refusals = match compile(&source) {
        Ok(_) => panic!("count may exceed N but checked: {source}"),
        Err(refusals) => refusals,
    };
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.cause.cause() == Some("unproved-range")),
        "{refusals:?}"
    );
}
