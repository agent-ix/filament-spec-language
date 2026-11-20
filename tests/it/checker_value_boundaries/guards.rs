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
    let atoms = [Formula::Present, Formula::A, Formula::B];
    let mut formulas = Vec::new();
    for x in &atoms {
        formulas.push(x.clone());
        formulas.push(Formula::Not(Box::new(x.clone())));
        for y in &atoms {
            formulas.push(Formula::And(Box::new(x.clone()), Box::new(y.clone())));
            formulas.push(Formula::Or(Box::new(x.clone()), Box::new(y.clone())));
            formulas.push(Formula::Implies(Box::new(x.clone()), Box::new(y.clone())));
        }
    }
    formulas.push(Formula::Or(
        Box::new(Formula::And(
            Box::new(Formula::Present),
            Box::new(Formula::A),
        )),
        Box::new(Formula::And(
            Box::new(Formula::Present),
            Box::new(Formula::B),
        )),
    ));
    for formula in formulas {
        let safe = (0..8).all(|bits| {
            !formula.holds(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0) || bits & 1 != 0
        });
        let body = format!("function guarded using v(x: Option<Integer>, a: Boolean, b: Boolean): Boolean pure {{ ({}) implies (value(x) = value(x)) }}",formula.source());
        match check(&body, &DependencyInput::default()) {
            Ok(_) => assert!(safe, "unsound accepted guard {formula:?}"),
            Err(error) => assert_eq!(
                error.code(),
                Code::UndefinedExpression,
                "{formula:?}: {error:?}"
            ),
        }
    }
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
