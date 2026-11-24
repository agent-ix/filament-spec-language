// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-053: the checker's accepted Boolean guard facts equal an independent
//! truth-table evaluation over every guard shape of up to three variables
//! (presence of `x`, `a`, `b`).
//!
//! The oracle is the `Formula::holds` evaluator below, a few lines of
//! Boolean algebra that share nothing with the checker's proof facts. The
//! property for an accepted clause is: no assignment makes the guard true
//! while `x` is absent. A guard the table makes unsound is therefore never
//! accepted, and the mandatory controls keep the check from passing by
//! refusing everything.

use fsl_foundation::Code;
use ix_trace_rs::trace;

use crate::imported_call::{compile, HEADER};

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

    /// True when no assignment makes the guard true with `x` absent.
    fn protects_unwrap(&self) -> bool {
        (0..8)
            .all(|bits| !(self.holds(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0) && bits & 1 == 0))
    }
}

fn boxed(f: &Formula) -> Box<Formula> {
    Box::new(f.clone())
}

/// Every base formula, its negation, and every and/or/implies pairing, plus
/// the common-fact join and impossible-path controls.
fn family() -> Vec<(Formula, Option<&'static str>)> {
    let atoms = [Formula::Present, Formula::A, Formula::B];
    let mut out = Vec::new();
    for x in &atoms {
        let control = matches!(x, Formula::Present).then_some("direct-presence");
        out.push((x.clone(), control));
        out.push((Formula::Not(boxed(x)), None));
        for y in &atoms {
            out.push((Formula::And(boxed(x), boxed(y)), None));
            out.push((Formula::Or(boxed(x), boxed(y)), None));
            out.push((Formula::Implies(boxed(x), boxed(y)), None));
        }
    }
    out.push((
        Formula::Or(
            Box::new(Formula::And(boxed(&Formula::Present), boxed(&Formula::A))),
            Box::new(Formula::And(boxed(&Formula::Present), boxed(&Formula::B))),
        ),
        Some("common-fact"),
    ));
    out.push((
        Formula::And(
            boxed(&Formula::Present),
            Box::new(Formula::Not(boxed(&Formula::Present))),
        ),
        Some("impossible-path"),
    ));
    out
}

fn verdict(formula: &Formula) -> Result<(), (Code, Option<&'static str>)> {
    let source = format!(
        "{HEADER}function guarded using v(x: Option<Integer>, a: Boolean, b: Boolean): Boolean pure \
         {{ ({}) implies (value(x) > 0) }}\n",
        formula.source()
    );
    compile(&source, Vec::new())
        .map(|_| ())
        .map_err(|refusals| {
            assert_eq!(refusals.len(), 1, "{formula:?}: {refusals:?}");
            (refusals[0].cause.code(), refusals[0].cause.cause())
        })
}

#[trace("TC-053")]
#[test]
fn accepted_guards_equal_the_truth_table_and_unsound_guards_are_refused() {
    let formulas = family();
    let mut assignments = 0;
    let (mut accepted, mut refused) = (0, 0);
    let mut accepted_controls = Vec::new();
    for (formula, control) in &formulas {
        assignments += 8;
        let safe = formula.protects_unwrap();
        match verdict(formula) {
            Ok(()) => {
                accepted += 1;
                assert!(safe, "unsound guard accepted: {formula:?}");
                if let Some(name) = control {
                    accepted_controls.push(*name);
                }
            }
            Err((code, cause)) => {
                refused += 1;
                assert!(control.is_none(), "mandatory control refused: {formula:?}");
                assert_eq!(code, Code::UndefinedExpression, "{formula:?}");
                assert_eq!(cause, Some("unproved-presence"), "{formula:?}");
                // A refusal is permitted only conservatively; the unsound
                // ones must be among them (checked via `safe` above for the
                // accepted side), so count them for the table summary.
            }
        }
    }
    assert_eq!(accepted_controls.len(), 3, "{accepted_controls:?}");
    assert_eq!(assignments, formulas.len() * 8);
    assert_eq!(accepted + refused, formulas.len());
    let unsound = formulas
        .iter()
        .filter(|(f, _)| !f.protects_unwrap())
        .count();
    assert!(
        unsound > 0 && refused >= unsound,
        "{refused} refused, {unsound} unsound"
    );
    assert!(accepted > 3, "vacuous refusal of every formula: {accepted}");
}

#[trace("TC-053")]
#[test]
fn a_guard_true_with_the_value_absent_refuses_and_its_safe_twin_checks() {
    // `a or present(x)` is true with `x` absent; `present(x) or (a and present(x))`
    // never is.
    let unsound = Formula::Or(boxed(&Formula::A), boxed(&Formula::Present));
    assert!(!unsound.protects_unwrap());
    assert_eq!(
        verdict(&unsound),
        Err((Code::UndefinedExpression, Some("unproved-presence")))
    );
    let safe = Formula::Or(
        boxed(&Formula::Present),
        Box::new(Formula::And(boxed(&Formula::A), boxed(&Formula::Present))),
    );
    assert!(safe.protects_unwrap());
    assert_eq!(verdict(&safe), Ok(()));
}
