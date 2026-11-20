// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact evaluation of an expanded expression along a dwell, where every
//! variable is a polynomial in time (FR-450 "Exact replay"): at an instant
//! every numeric subterm is a rational function of time and every decision
//! the sign of a polynomial there, decided by exact real root isolation. A
//! stretch of instants is split at the roots of every polynomial a decision
//! read, so each resulting cell takes one value throughout.

use std::cmp::Ordering;

use filament_exact::Rational;
use fsl_semantics::check::{ArithmeticOp, CompareOp, LogicOp};

use super::expand::{Exp, RangeRole, TermLocus};
use super::poly::{Poly, RatFunc};
use super::poly2::{Poly2, RatFunc2};
use super::real::ExactReal;
use super::{ExactGap, HybridReplayRefusal, UndefinedCause};

/// A capture's value, read throughout its obligation.
#[derive(Clone, Debug)]
pub(crate) enum Constant {
    Number(Rational),
    Boolean(bool),
}

/// The exact arithmetic an evaluation runs in: one real variable (the
/// instant of a dwell), or two (the instant and a quantified post-jump
/// variable).
pub(crate) trait Algebra {
    /// What a variable reads as.
    type Var: Clone;
    /// An exact numeric value: a rational function.
    type Value: Clone;
    /// What a decision's sign is read from.
    type Poly: Clone;
    fn variable(var: &Self::Var) -> Self::Value;
    fn constant(value: &Rational) -> Self::Value;
    fn add(a: &Self::Value, b: &Self::Value) -> Self::Value;
    fn sub(a: &Self::Value, b: &Self::Value) -> Self::Value;
    fn mul(a: &Self::Value, b: &Self::Value) -> Self::Value;
    /// `a / b`, `b` not the zero function.
    fn div(a: &Self::Value, b: &Self::Value) -> Self::Value;
    fn neg(a: &Self::Value) -> Self::Value;
    /// The polynomial that is zero exactly where `value` is zero.
    fn numerator(value: &Self::Value) -> Self::Poly;
    /// The polynomial whose sign is `value`'s sign wherever it is defined.
    fn sign_poly(value: &Self::Value) -> Self::Poly;
}

/// A point at which an evaluation reads the sign of a polynomial.
pub(crate) trait Probe<A: Algebra> {
    fn sign(&self, poly: &A::Poly) -> Ordering;
}

/// Evaluation in one real variable: the instant along a dwell.
pub(crate) struct Uni;

impl Algebra for Uni {
    type Var = Poly;
    type Value = RatFunc;
    type Poly = Poly;

    fn variable(var: &Poly) -> RatFunc {
        RatFunc::poly(var.clone())
    }

    fn constant(value: &Rational) -> RatFunc {
        RatFunc::constant(value.clone())
    }

    fn add(a: &RatFunc, b: &RatFunc) -> RatFunc {
        a.add(b)
    }

    fn sub(a: &RatFunc, b: &RatFunc) -> RatFunc {
        a.sub(b)
    }

    fn mul(a: &RatFunc, b: &RatFunc) -> RatFunc {
        a.mul(b)
    }

    fn div(a: &RatFunc, b: &RatFunc) -> RatFunc {
        a.div(b)
    }

    fn neg(a: &RatFunc) -> RatFunc {
        a.neg()
    }

    fn numerator(value: &RatFunc) -> Poly {
        value.numerator.clone()
    }

    fn sign_poly(value: &RatFunc) -> Poly {
        value.sign_poly()
    }
}

impl Probe<Uni> for ExactReal {
    fn sign(&self, poly: &Poly) -> Ordering {
        self.sign_of(poly)
    }
}

/// Evaluation in two real variables: the instant `t` of a dwell and a
/// quantified post-jump variable `x`.
pub(crate) struct Bi;

impl Algebra for Bi {
    type Var = Poly2;
    type Value = RatFunc2;
    type Poly = Poly2;

    fn variable(var: &Poly2) -> RatFunc2 {
        RatFunc2::of(var.clone())
    }

    fn constant(value: &Rational) -> RatFunc2 {
        RatFunc2::constant(value.clone())
    }

    fn add(a: &RatFunc2, b: &RatFunc2) -> RatFunc2 {
        a.add(b)
    }

    fn sub(a: &RatFunc2, b: &RatFunc2) -> RatFunc2 {
        a.sub(b)
    }

    fn mul(a: &RatFunc2, b: &RatFunc2) -> RatFunc2 {
        a.mul(b)
    }

    fn div(a: &RatFunc2, b: &RatFunc2) -> RatFunc2 {
        a.div(b)
    }

    fn neg(a: &RatFunc2) -> RatFunc2 {
        a.neg()
    }

    fn numerator(value: &RatFunc2) -> Poly2 {
        value.numerator.clone()
    }

    fn sign_poly(value: &RatFunc2) -> Poly2 {
        value.sign_poly()
    }
}

/// What an evaluation reads: each variable, each capture, and each
/// activation variable.
pub(crate) struct Env<'a, A: Algebra = Uni> {
    pub(crate) variables: &'a [A::Var],
    pub(crate) captures: &'a [Constant],
    pub(crate) activation: &'a [A::Var],
}

/// An exact value at one instant.
#[derive(Clone, Debug)]
pub(crate) enum Ev<V = RatFunc> {
    Number(V),
    Boolean(bool),
    /// The expression has no value there.
    Undefined {
        locus: TermLocus,
        cause: UndefinedCause,
    },
}

/// The value of `exp` at the instant `at`, pushing every polynomial whose
/// sign a decision read.
pub(crate) fn eval(
    exp: &Exp,
    env: &Env<'_>,
    at: &ExactReal,
    decisions: &mut Vec<Poly>,
) -> Result<Ev, ExactGap> {
    eval_in::<Uni, ExactReal>(exp, env, at, decisions)
}

/// [`eval`] in the arithmetic `A`, at the point `at`.
pub(crate) fn eval_in<A: Algebra, P: Probe<A>>(
    exp: &Exp,
    env: &Env<'_, A>,
    at: &P,
    decisions: &mut Vec<A::Poly>,
) -> Result<Ev<A::Value>, ExactGap> {
    macro_rules! number {
        ($e:expr) => {
            match eval_in::<A, P>($e, env, at, decisions)? {
                Ev::Number(value) => value,
                undefined @ Ev::Undefined { .. } => return Ok(undefined),
                Ev::Boolean(_) => return Err(ExactGap::IllKindedTerm),
            }
        };
    }
    macro_rules! truth {
        ($e:expr) => {
            match eval_in::<A, P>($e, env, at, decisions)? {
                Ev::Boolean(value) => value,
                undefined @ Ev::Undefined { .. } => return Ok(undefined),
                Ev::Number(_) => return Err(ExactGap::IllKindedTerm),
            }
        };
    }
    let sign = |value: &A::Value, decisions: &mut Vec<A::Poly>| {
        let poly = A::sign_poly(value);
        let sign = at.sign(&poly);
        decisions.push(poly);
        sign
    };
    Ok(match exp {
        Exp::Variable(i) => Ev::Number(A::variable(
            env.variables.get(*i).ok_or(ExactGap::IllKindedTerm)?,
        )),
        Exp::Activation(i) => Ev::Number(A::variable(
            env.activation.get(*i).ok_or(ExactGap::IllKindedTerm)?,
        )),
        Exp::Capture(i) => match env.captures.get(*i).ok_or(ExactGap::IllKindedTerm)? {
            Constant::Number(value) => Ev::Number(A::constant(value)),
            Constant::Boolean(value) => Ev::Boolean(*value),
        },
        Exp::Parameter(_) | Exp::Recursive(_) => return Err(ExactGap::RecursiveCall),
        Exp::Number(value) => Ev::Number(A::constant(value)),
        Exp::Boolean(value) => Ev::Boolean(*value),
        Exp::Negate(a) => Ev::Number(A::neg(&number!(a))),
        Exp::Arithmetic(op, a, b, locus) => {
            let (a, b) = (number!(a), number!(b));
            Ev::Number(match op {
                ArithmeticOp::Add => A::add(&a, &b),
                ArithmeticOp::Sub => A::sub(&a, &b),
                ArithmeticOp::Mul => A::mul(&a, &b),
                ArithmeticOp::Div => {
                    let divisor = A::numerator(&b);
                    let zero = at.sign(&divisor) == Ordering::Equal;
                    decisions.push(divisor);
                    if zero {
                        return Ok(Ev::Undefined {
                            locus: locus.clone(),
                            cause: UndefinedCause::DivisionByZero,
                        });
                    }
                    A::div(&a, &b)
                }
            })
        }
        Exp::Compare(op, a, b) => {
            let difference = A::sub(&number!(a), &number!(b));
            let s = sign(&difference, decisions);
            Ev::Boolean(match op {
                CompareOp::Eq => s == Ordering::Equal,
                CompareOp::Ne => s != Ordering::Equal,
                CompareOp::Lt => s == Ordering::Less,
                CompareOp::Le => s != Ordering::Greater,
                CompareOp::Gt => s == Ordering::Greater,
                CompareOp::Ge => s != Ordering::Less,
            })
        }
        Exp::Not(a) => Ev::Boolean(!truth!(a)),
        Exp::Logic(op, a, b) => {
            let left = truth!(a);
            match (op, left) {
                (LogicOp::And, false) => Ev::Boolean(false),
                (LogicOp::Or, true) | (LogicOp::Implies, false) => Ev::Boolean(true),
                (LogicOp::And | LogicOp::Or | LogicOp::Implies, _) => Ev::Boolean(truth!(b)),
            }
        }
        Exp::Conditional(c, a, b, _) => {
            if truth!(c) {
                eval_in::<A, P>(a, env, at, decisions)?
            } else {
                eval_in::<A, P>(b, env, at, decisions)?
            }
        }
        Exp::Ranged {
            value,
            range,
            role,
            locus,
        } => {
            let value = number!(value);
            let mut inside = true;
            if let Some(lower) = &range.lower {
                inside &= sign(&A::sub(&value, &A::constant(lower)), decisions) != Ordering::Less;
            }
            if let Some(upper) = &range.upper {
                inside &= sign(&A::sub(&A::constant(upper), &value), decisions) != Ordering::Less;
            }
            if !inside {
                return Ok(Ev::Undefined {
                    locus: locus.clone(),
                    cause: match role {
                        RangeRole::Parameter {
                            function,
                            parameter,
                        } => UndefinedCause::OutsideParameterRange {
                            function: *function,
                            parameter: *parameter,
                        },
                        RangeRole::Result { function } => UndefinedCause::OutsideResultRange {
                            function: *function,
                        },
                    },
                });
            }
            Ev::Number(value)
        }
    })
}

/// A stretch of a segment over which every evaluated expression takes one
/// value: the instant `low` when `point`, else the open interval
/// `(low, high)`.
#[derive(Clone, Debug)]
pub(crate) struct Cell {
    pub(crate) low: ExactReal,
    pub(crate) high: ExactReal,
    pub(crate) point: bool,
    pub(crate) values: Vec<Ev>,
}

/// A rational at or below `x`.
pub(super) fn floor_bound(x: &ExactReal) -> Rational {
    match x {
        ExactReal::Rational(q) => q.clone(),
        ExactReal::Root(root) => root.lower().clone(),
    }
}

/// A rational at or above `x`.
pub(super) fn ceiling_bound(x: &ExactReal) -> Rational {
    match x {
        ExactReal::Rational(q) => q.clone(),
        ExactReal::Root(root) => root.upper().clone(),
    }
}

/// What `cells` splits a stretch for: the values of some expressions at an
/// instant and every polynomial whose sign the evaluation read.
pub(crate) trait Evaluate {
    fn at(&mut self, at: &ExactReal) -> Result<(Vec<Ev>, Vec<Poly>), HybridReplayRefusal>;

    /// Charge `work` words of operand to the meter the evaluator works
    /// under, if it has one, before a step on them is taken.
    fn charge(&mut self, work: u64) -> Result<(), HybridReplayRefusal> {
        let _ = work;
        Ok(())
    }
}

/// The expressions `exps` evaluated directly in `env`.
struct Direct<'a> {
    exps: &'a [Exp],
    env: &'a Env<'a>,
}

impl Evaluate for Direct<'_> {
    fn at(&mut self, at: &ExactReal) -> Result<(Vec<Ev>, Vec<Poly>), HybridReplayRefusal> {
        let mut decisions = Vec::new();
        let values = self
            .exps
            .iter()
            .map(|exp| eval(exp, self.env, at, &mut decisions))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((values, decisions))
    }
}

/// The cells of `[low, high]` (`[low, high)` when `!high_closed`), in
/// order, over which each of `exps` takes one value.
pub(crate) fn cells(
    exps: &[Exp],
    env: &Env<'_>,
    low: &ExactReal,
    high: &ExactReal,
    high_closed: bool,
) -> Result<Vec<Cell>, HybridReplayRefusal> {
    cells_of(&mut Direct { exps, env }, low, high, high_closed)
}

/// The instant `at` as a cell of its own.
fn point_cell(
    evaluator: &mut impl Evaluate,
    at: &ExactReal,
    done: &mut Vec<Cell>,
) -> Result<(), HybridReplayRefusal> {
    let (values, _) = evaluator.at(at)?;
    done.push(Cell {
        low: at.clone(),
        high: at.clone(),
        point: true,
        values,
    });
    Ok(())
}

/// The cells of `[low, high]` (`[low, high)` when `!high_closed`), in
/// order, over which `evaluator`'s values are constant.
pub(crate) fn cells_of(
    evaluator: &mut impl Evaluate,
    low: &ExactReal,
    high: &ExactReal,
    high_closed: bool,
) -> Result<Vec<Cell>, HybridReplayRefusal> {
    let mut done = Vec::new();
    point_cell(evaluator, low, &mut done)?;
    let mut pending = Vec::new();
    if low < high {
        if high_closed {
            point_cell(evaluator, high, &mut done)?;
        }
        pending.push((low.clone(), high.clone()));
    }
    while let Some((a, b)) = pending.pop() {
        let sample = ExactReal::Rational(a.between(&b));
        let (values, decisions) = evaluator.at(&sample)?;
        let mut splits: Vec<ExactReal> = Vec::new();
        for decision in &decisions {
            let found =
                decision.roots_in_by(&floor_bound(&a), &ceiling_bound(&b), &mut |work| {
                    evaluator.charge(work)
                })?;
            for root in found {
                if root > a && root < b {
                    splits.push(root);
                }
            }
        }
        if splits.is_empty() {
            done.push(Cell {
                low: a,
                high: b,
                point: false,
                values,
            });
            continue;
        }
        splits.sort();
        splits.dedup();
        let mut from = a;
        for split in splits {
            point_cell(evaluator, &split, &mut done)?;
            pending.push((from, split.clone()));
            from = split;
        }
        pending.push((from, b));
    }
    done.sort_by(|x, y| x.low.cmp(&y.low).then(y.point.cmp(&x.point)));
    Ok(done)
}
