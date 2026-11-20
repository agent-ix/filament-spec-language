// SPDX-License-Identifier: AGPL-3.0-or-later

use super::check;
use fsl_foundation::Code;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;

#[derive(Clone, Debug)]
enum Formula {
    Present,
    A,
    B,
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Implies(Box<Self>, Box<Self>),
}

impl Formula {
    fn holds(&self, present: bool, a: bool, b: bool) -> bool {
        match self {
            Self::Present => present,
            Self::A => a,
            Self::B => b,
            Self::Not(x) => !x.holds(present, a, b),
            Self::And(x, y) => x.holds(present, a, b) && y.holds(present, a, b),
            Self::Or(x, y) => x.holds(present, a, b) || y.holds(present, a, b),
            Self::Implies(x, y) => !x.holds(present, a, b) || y.holds(present, a, b),
        }
    }

    fn source(&self) -> String {
        match self {
            Self::Present => "present(x)".into(),
            Self::A => "a".into(),
            Self::B => "b".into(),
            Self::Not(x) => format!("not ({})", x.source()),
            Self::And(x, y) => format!("({}) and ({})", x.source(), y.source()),
            Self::Or(x, y) => format!("({}) or ({})", x.source(), y.source()),
            Self::Implies(x, y) => format!("({}) implies ({})", x.source(), y.source()),
        }
    }
}

#[test]
#[trace("TC-053")]
fn accepted_three_variable_guards_never_admit_an_absent_unwrap() {
    use filament_exact::{Integer, OptionValue, Value, ValueType};
    let atoms = [Formula::Present, Formula::A, Formula::B];
    let mut formulas = Vec::new();
    for x in &atoms {
        let required = matches!(x, Formula::Present).then_some("direct-presence");
        formulas.push((x.clone(), required));
        formulas.push((Formula::Not(Box::new(x.clone())), None));
        for y in &atoms {
            formulas.push((Formula::And(Box::new(x.clone()), Box::new(y.clone())), None));
            formulas.push((Formula::Or(Box::new(x.clone()), Box::new(y.clone())), None));
            formulas.push((
                Formula::Implies(Box::new(x.clone()), Box::new(y.clone())),
                None,
            ));
        }
    }
    formulas.push((
        Formula::Or(
            Box::new(Formula::And(
                Box::new(Formula::Present),
                Box::new(Formula::A),
            )),
            Box::new(Formula::And(
                Box::new(Formula::Present),
                Box::new(Formula::B),
            )),
        ),
        Some("common-fact"),
    ));
    formulas.push((
        Formula::And(
            Box::new(Formula::Present),
            Box::new(Formula::Not(Box::new(Formula::Present))),
        ),
        Some("impossible-path"),
    ));
    let formula_count = formulas.len();
    let mut assignment_count = 0;
    let mut accepted = 0;
    let mut refused = 0;
    let mut required_accepts = 0;
    let mut required_evaluations = 0;
    for (formula, required) in formulas {
        let mut safe = true;
        for bits in 0..8 {
            assignment_count += 1;
            if formula.holds(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0) && bits & 1 == 0 {
                safe = false;
            }
        }
        let body = format!("function guarded using v(x: Option<Integer>, a: Boolean, b: Boolean): Boolean pure {{ ({}) implies (value(x) > 0) }}",formula.source());
        match check(&body, &DependencyInput::default()) {
            Ok(checked) => {
                accepted += 1;
                assert!(safe, "unsound accepted guard {formula:?}");
                if required.is_some() {
                    required_accepts += 1;
                    for bits in 0..8 {
                        let present = bits & 1 != 0;
                        let a = bits & 2 != 0;
                        let b = bits & 4 != 0;
                        let payloads: &[Option<i64>] = if present {
                            &[Some(-3), Some(5)]
                        } else {
                            &[None]
                        };
                        for payload in payloads {
                            let x = match payload {
                                Some(value) => OptionValue::present(
                                    ValueType::Integer,
                                    Value::Integer(Integer::from(*value)),
                                )
                                .unwrap(),
                                None => OptionValue::none(ValueType::Integer),
                            };
                            let expected = !formula.holds(present, a, b)
                                || payload.is_some_and(|value| value > 0);
                            let result = super::evaluation::call(
                                &checked,
                                "guarded",
                                vec![x, Value::Boolean(a), Value::Boolean(b)],
                            );
                            assert!(
                                matches!(result, Value::Boolean(actual) if actual == expected),
                                "{required:?}, assignment {bits}, payload {payload:?}: {result:?}"
                            );
                            required_evaluations += 1;
                        }
                    }
                }
            }
            Err(error) => {
                refused += 1;
                assert!(
                    required.is_none(),
                    "mandatory {required:?} control refused: {error:?}"
                );
                assert_eq!(
                    error.code(),
                    Code::UndefinedExpression,
                    "{formula:?}: {error:?}"
                );
            }
        }
    }
    assert_eq!(required_accepts, 3);
    assert_eq!(assignment_count, formula_count * 8);
    assert_eq!(accepted + refused, formula_count);
    assert_eq!(required_evaluations, 36);
    println!("formulas={formula_count}; assignments={assignment_count}; accepted={accepted}; refused={refused}; required_accepts={required_accepts}; required_evaluations={required_evaluations}");
}

#[test]
#[trace("TC-025", "TC-026")]
fn an_unrelated_optional_presence_cannot_discharge_the_unwrap() {
    let dependencies = DependencyInput::default();
    check("function guarded using v(x: Option<Integer>, y: Option<Integer>): Boolean pure { present(x) implies (value(x) = value(x)) }", &dependencies).unwrap();
    let error = check("function guarded using v(x: Option<Integer>, y: Option<Integer>): Boolean pure { present(y) implies (value(x) = value(x)) }", &dependencies).unwrap_err();
    assert_eq!(error.code(), Code::UndefinedExpression, "{error:?}");
}

#[test]
#[trace("TC-025")]
fn short_circuit_order_protects_only_the_following_optional_arithmetic() {
    let dependencies = DependencyInput::default();
    for body in [
        "present(x) and (value(x) + 1 > value(x))",
        "not present(x) or (value(x) + 1 > value(x))",
        "if present(x) then value(x) + 1 > value(x) else true",
    ] {
        check(
            &format!("function sequenced using v(x: Option<Integer>): Boolean pure {{ {body} }}"),
            &dependencies,
        )
        .unwrap_or_else(|error| panic!("{body}: {error:?}"));
    }
    for body in [
        "(value(x) + 1 > value(x)) and present(x)",
        "(present(x) or a) implies (value(x) + 1 > value(x))",
    ] {
        let error = check(&format!("function sequenced using v(x: Option<Integer>, a: Boolean): Boolean pure {{ {body} }}"),&dependencies).unwrap_err();
        assert_eq!(error.code(), Code::UndefinedExpression, "{body}: {error:?}");
    }
}

#[test]
#[trace("FR-104-AC-16")]
fn a_let_binding_does_not_borrow_decimal_context_from_its_later_use() {
    use filament_exact::IllTypedCause;
    use fsl_replay::spine::CompileRefusal;
    use fsl_semantics::check::CheckCause;
    let prefix = "type Amount = Decimal[-10000,10000;3,3;exact]; ";
    check(&format!("{prefix}function contextual using v(): Amount pure {{ decimal(125,2) + decimal(375,2) }}"),&DependencyInput::default()).unwrap();
    let error = check(&format!("{prefix}function unbound using v(): Amount pure {{ let total = decimal(125,2) + decimal(375,2) in total }}"),&DependencyInput::default()).unwrap_err();
    assert_eq!(error.code(), Code::IllTyped, "{error:?}");
    let refusals = match error.as_ref() {
        CompileRefusal::Check { refusals, .. }
        | CompileRefusal::CheckDiagnostics { refusals, .. } => refusals,
        other => panic!("literal ambiguity belongs to checking: {other:?}"),
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::AmbiguousLiteral)
        )),
        "{error:?}"
    );
}

#[test]
#[trace("TC-027")]
fn a_strict_finite_guard_discharges_only_the_admitted_successor_range() {
    let dependencies = DependencyInput::default();
    check(
        "function successor using v(x: Int[0,7]): Int[0,7] pure { if x < 7 then x + 1 else 7 }",
        &dependencies,
    )
    .expect("strict upper guard proves successor membership");
    for body in ["x + 1", "if x <= 7 then x + 1 else 7"] {
        let error = check(
            &format!("function successor using v(x: Int[0,7]): Int[0,7] pure {{ {body} }}"),
            &dependencies,
        )
        .unwrap_err();
        assert_eq!(error.code(), Code::UndefinedExpression, "{body}: {error:?}");
    }
}
