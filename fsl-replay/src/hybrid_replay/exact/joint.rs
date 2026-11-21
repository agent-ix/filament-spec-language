// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": joint real quantifier elimination over the
//! post-jump variables a reset leaves free when one condition of the target
//! invariant or the reset reads two or more of them (`v + w <= 1`).
//!
//! At the instant `u` of a dwell every condition that is linear in the free
//! variables is `sum(c_i * y_i) + c_0 R 0` with each `c` a rational function
//! of time and `R` one of `<` and `<=` (an equation is the two inequalities).
//! The existence of a real point satisfying every condition at `u` is decided
//! exactly by Fourier-Motzkin elimination: the sign of each coefficient at `u`
//! is read exactly ([`super::super::real::ExactReal`]), a variable's lower and
//! upper bounds are combined pairwise with the positive multipliers those
//! signs give, and what is left is a conjunction of conditions on the
//! constants alone. Every sign read is pushed as a decision, so the caller
//! cuts the dwell at the roots of the polynomials read and each cell is read
//! at one sample, as for every other exact decision.
//!
//! A condition that is not linear in the variables it couples (a product of
//! two free variables, a conditional, a disequation) is the one shape this
//! elimination does not decide: it is refused as
//! [`ExactGap::CoupledReset`].
//!
//! Work accounting: the meter is charged before every pairing of a lower and
//! an upper bound, by the size of the two conditions' coefficients, and every
//! sign read at an irrational instant is charged by [`ExactReal::sign_with`].

use std::cmp::Ordering;
use std::collections::BTreeMap;

use fsl_semantics::check::{ArithmeticOp, CompareOp};

use super::super::eval::{eval, Env, Ev};
use super::super::expand::Exp;
use super::super::poly::{Poly, RatFunc};
use super::super::real::ExactReal;
use super::super::{ExactGap, HybridReplayMeter, HybridReplayRefusal};

/// A set of free variables coupled by conditions, and those conditions.
pub(super) struct Joint {
    /// The free variables, by position.
    pub(super) variables: Vec<usize>,
    /// The conditions that read at least one of them.
    pub(super) conjuncts: Vec<Exp>,
    /// The source mode's position and the transition's, for the gap.
    pub(super) mode: usize,
    pub(super) transition: usize,
}

/// `sum(coefficients[i] * y_i) + constant`.
#[derive(Clone)]
struct Linear {
    coefficients: BTreeMap<usize, RatFunc>,
    constant: RatFunc,
}

impl Linear {
    fn constant(value: RatFunc) -> Self {
        Self {
            coefficients: BTreeMap::new(),
            constant: value,
        }
    }

    fn combine(&self, other: &Self, mine: &RatFunc, theirs: &RatFunc) -> Self {
        let mut coefficients = BTreeMap::new();
        for (&variable, c) in &self.coefficients {
            coefficients.insert(variable, c.mul(mine));
        }
        for (&variable, c) in &other.coefficients {
            let scaled = c.mul(theirs);
            let next = match coefficients.remove(&variable) {
                Some(have) => have.add(&scaled),
                None => scaled,
            };
            coefficients.insert(variable, next);
        }
        Self {
            coefficients,
            constant: self.constant.mul(mine).add(&other.constant.mul(theirs)),
        }
    }

    fn scaled(&self, factor: &RatFunc) -> Self {
        self.combine(
            &Self::constant(RatFunc::constant(zero())),
            factor,
            &one_function(),
        )
    }

    fn negated(&self) -> Self {
        self.scaled(&RatFunc::constant(minus_one()))
    }

    fn sum(&self, other: &Self) -> Self {
        self.combine(other, &one_function(), &one_function())
    }

    fn size(&self) -> u64 {
        self.coefficients
            .values()
            .chain(std::iter::once(&self.constant))
            .map(|c| c.numerator.size() + c.denominator.size())
            .sum()
    }
}

fn zero() -> filament_exact::Rational {
    filament_exact::Rational::from_integer(0_i64.into())
}

fn minus_one() -> filament_exact::Rational {
    filament_exact::Rational::from_integer((-1_i64).into())
}

fn one_function() -> RatFunc {
    RatFunc::constant(filament_exact::Rational::from_integer(1_i64.into()))
}

/// What a term reads as at an instant.
enum Read {
    Form(Linear),
    /// A subterm with no value at the instant.
    Undefined,
}

/// A condition `form < 0` (strict) or `form <= 0`.
struct Condition {
    form: Linear,
    strict: bool,
}

impl Joint {
    fn gap(&self) -> HybridReplayRefusal {
        ExactGap::CoupledReset {
            mode: self.mode,
            transition: self.transition,
        }
        .into()
    }

    /// The sign of `poly` at `at`, charged as [`super::super::section::Section`]
    /// charges its sign reads.
    fn sign(
        at: &ExactReal,
        poly: &Poly,
        meter: &mut HybridReplayMeter,
    ) -> Result<Ordering, HybridReplayRefusal> {
        at.sign_with(poly, &mut |weight| meter.charge_by(weight))
    }

    /// `exp` as a linear form in the free variables.
    fn form(
        &self,
        exp: &Exp,
        env: &Env<'_>,
        at: &ExactReal,
        decisions: &mut Vec<Poly>,
        meter: &mut HybridReplayMeter,
    ) -> Result<Read, HybridReplayRefusal> {
        if !exp.any_node(|node| matches!(node, Exp::Variable(i) if self.variables.contains(i))) {
            return Ok(match eval(exp, env, at, decisions)? {
                Ev::Number(value) => Read::Form(Linear::constant(value)),
                Ev::Undefined { .. } => Read::Undefined,
                Ev::Boolean(_) => return Err(ExactGap::IllKindedTerm.into()),
            });
        }
        macro_rules! form {
            ($e:expr) => {
                match self.form($e, env, at, decisions, meter)? {
                    Read::Form(form) => form,
                    Read::Undefined => return Ok(Read::Undefined),
                }
            };
        }
        Ok(Read::Form(match exp {
            Exp::Variable(i) => Linear {
                coefficients: BTreeMap::from([(*i, one_function())]),
                constant: RatFunc::constant(zero()),
            },
            Exp::Negate(a) => form!(a).negated(),
            Exp::Arithmetic(ArithmeticOp::Add, a, b, _) => form!(a).sum(&form!(b)),
            Exp::Arithmetic(ArithmeticOp::Sub, a, b, _) => form!(a).sum(&form!(b).negated()),
            Exp::Arithmetic(ArithmeticOp::Mul, a, b, _) => {
                let (a, b) = (form!(a), form!(b));
                match (a.coefficients.is_empty(), b.coefficients.is_empty()) {
                    (true, _) => b.scaled(&a.constant),
                    (_, true) => a.scaled(&b.constant),
                    _ => return Err(self.gap()),
                }
            }
            Exp::Arithmetic(ArithmeticOp::Div, a, b, _) => {
                let (a, b) = (form!(a), form!(b));
                if !b.coefficients.is_empty() {
                    return Err(self.gap());
                }
                let divisor = b.constant.numerator.clone();
                let zero = Self::sign(at, &divisor, meter)? == Ordering::Equal;
                decisions.push(divisor);
                if zero {
                    return Ok(Read::Undefined);
                }
                let inverse = one_function().div(&b.constant);
                a.scaled(&inverse)
            }
            _ => return Err(self.gap()),
        }))
    }

    /// `conjunct` as the conditions it states at `at`, or `None` when a
    /// subterm of it has no value there.
    fn conditions(
        &self,
        conjunct: &Exp,
        env: &Env<'_>,
        at: &ExactReal,
        decisions: &mut Vec<Poly>,
        meter: &mut HybridReplayMeter,
    ) -> Result<Option<Vec<Condition>>, HybridReplayRefusal> {
        let Exp::Compare(op, a, b) = conjunct else {
            return Err(self.gap());
        };
        let (Read::Form(a), Read::Form(b)) = (
            self.form(a, env, at, decisions, meter)?,
            self.form(b, env, at, decisions, meter)?,
        ) else {
            return Ok(None);
        };
        let difference = a.sum(&b.negated());
        let condition = |form: Linear, strict: bool| Condition { form, strict };
        Ok(Some(match op {
            CompareOp::Lt => vec![condition(difference, true)],
            CompareOp::Le => vec![condition(difference, false)],
            CompareOp::Gt => vec![condition(difference.negated(), true)],
            CompareOp::Ge => vec![condition(difference.negated(), false)],
            CompareOp::Eq => vec![
                condition(difference.negated(), false),
                condition(difference, false),
            ],
            CompareOp::Ne => return Err(self.gap()),
        }))
    }

    /// Whether some real point of the free variables satisfies every
    /// condition at the instant `at`.
    pub(super) fn exists(
        &self,
        env: &Env<'_>,
        at: &ExactReal,
        decisions: &mut Vec<Poly>,
        meter: &mut HybridReplayMeter,
    ) -> Result<bool, HybridReplayRefusal> {
        let mut conditions = Vec::new();
        for conjunct in &self.conjuncts {
            meter.charge()?;
            match self.conditions(conjunct, env, at, decisions, meter)? {
                Some(read) => conditions.extend(read),
                None => return Ok(false),
            }
        }
        for &variable in &self.variables {
            let mut rest = Vec::new();
            let mut lower = Vec::new();
            let mut upper = Vec::new();
            for mut condition in conditions {
                let Some(coefficient) = condition.form.coefficients.get(&variable) else {
                    rest.push(condition);
                    continue;
                };
                let poly = coefficient.sign_poly();
                let sign = Self::sign(at, &poly, meter)?;
                decisions.push(poly);
                match sign {
                    Ordering::Equal => {
                        condition.form.coefficients.remove(&variable);
                        rest.push(condition);
                    }
                    Ordering::Greater => upper.push(condition),
                    Ordering::Less => lower.push(condition),
                }
            }
            for low in &lower {
                for high in &upper {
                    meter.charge_by(low.form.size().saturating_mul(high.form.size()))?;
                    let (Some(low_c), Some(high_c)) = (
                        low.form.coefficients.get(&variable),
                        high.form.coefficients.get(&variable),
                    ) else {
                        continue;
                    };
                    let mut form = low.form.combine(&high.form, high_c, &low_c.neg());
                    form.coefficients.remove(&variable);
                    rest.push(Condition {
                        form,
                        strict: low.strict || high.strict,
                    });
                }
            }
            conditions = rest;
        }
        for condition in conditions {
            let poly = condition.form.constant.sign_poly();
            let sign = Self::sign(at, &poly, meter)?;
            decisions.push(poly);
            let holds = if condition.strict {
                sign == Ordering::Less
            } else {
                sign != Ordering::Greater
            };
            if !holds {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use filament_exact::Rational;

    use super::super::super::expand::TermLocus;
    use super::*;

    fn n(value: i64) -> Rational {
        Rational::from_integer(value.into())
    }

    fn num(value: i64) -> Exp {
        Exp::Number(n(value))
    }

    fn time() -> Exp {
        Exp::Variable(2)
    }

    fn times(a: Exp, b: Exp) -> Exp {
        Exp::Arithmetic(
            ArithmeticOp::Mul,
            Box::new(a),
            Box::new(b),
            TermLocus::default(),
        )
    }

    fn plus(a: Exp, b: Exp) -> Exp {
        Exp::Arithmetic(
            ArithmeticOp::Add,
            Box::new(a),
            Box::new(b),
            TermLocus::default(),
        )
    }

    fn compare(op: CompareOp, a: Exp, b: Exp) -> Exp {
        Exp::Compare(op, Box::new(a), Box::new(b))
    }

    /// Whether the free variables `0` and `1` have a joint solution of
    /// `conjuncts` at `at`, and the polynomials the decision read; variable
    /// `2` is the instant itself.
    fn joint_at(conjuncts: Vec<Exp>, at: &ExactReal) -> (bool, Vec<Poly>) {
        let t = Poly::new(vec![n(0), n(1)]);
        let solution = [Poly::zero(), Poly::zero(), t];
        let env = Env {
            variables: &solution,
            captures: &[],
            activation: &solution,
        };
        let joint = Joint {
            variables: vec![0, 1],
            conjuncts,
            mode: 0,
            transition: 0,
        };
        let mut decisions = Vec::new();
        let mut meter = HybridReplayMeter::new(u64::MAX);
        let found = joint
            .exists(&env, at, &mut decisions, &mut meter)
            .expect("an unmetered linear elimination");
        (found, decisions)
    }

    fn sqrt_two() -> ExactReal {
        Poly::new(vec![n(-2), n(0), n(1)])
            .roots_in(&n(1), &n(2))
            .into_iter()
            .next()
            .expect("sqrt(2) is isolated in (1, 2)")
    }

    /// A coefficient `t` of the free variable `y0` is zero at `t = 0`: the
    /// condition `t * y0 >= 1` then states `1 <= 0` and has no solution, where
    /// it would vanish if a zero coefficient counted as a bound or dropped
    /// the condition; at `t = 1` it is `y0 >= 1`, which has one. The sign of
    /// the coefficient and of the remaining constant are read as decisions.
    #[test]
    fn a_zero_coefficient_leaves_its_constant_condition_to_decide() {
        let condition = compare(CompareOp::Ge, times(time(), Exp::Variable(0)), num(1));
        let (at_zero, read) = joint_at(vec![condition.clone()], &ExactReal::Rational(n(0)));
        assert!(!at_zero);
        assert!(read.contains(&Poly::new(vec![n(0), n(-1)])), "{read:?}");
        assert!(read.contains(&Poly::constant(n(1))), "{read:?}");
        let (at_one, _) = joint_at(vec![condition], &ExactReal::Rational(n(1)));
        assert!(at_one);
    }

    /// A strict and a non-strict condition on one variable combine strictly:
    /// `y0 >= 1` with `y0 < 1` has no solution, nor does `y0 > 1` with
    /// `y0 <= 1`, while `y0 >= 1` with `y0 <= 1` has one.
    #[test]
    fn a_strict_bound_makes_the_pairing_strict() {
        let y = || Exp::Variable(0);
        let at = ExactReal::Rational(n(0));
        let found = |first: CompareOp, second: CompareOp| {
            joint_at(
                vec![compare(first, y(), num(1)), compare(second, y(), num(1))],
                &at,
            )
            .0
        };
        assert!(!found(CompareOp::Ge, CompareOp::Lt));
        assert!(!found(CompareOp::Gt, CompareOp::Le));
        assert!(found(CompareOp::Ge, CompareOp::Le));
    }

    /// At the irrational instant `t = sqrt 2` the coefficient signs and the
    /// constants are read exactly: `y0 >= t`, `y1 >= t`, `y0 + y1 <= 2 t` is
    /// satisfied only at the point `y0 = y1 = t`, so `<` makes it
    /// unsatisfiable; `y0 >= t`, `y1 >= 1`, `y0 + y1 <= 3` needs `t + 1 <= 3`,
    /// true at `sqrt 2`, and `y0 + y1 <= 2` needs `t + 1 <= 2`, false there.
    #[test]
    fn elimination_at_an_irrational_instant_is_exact() {
        let at = sqrt_two();
        let sum = || plus(Exp::Variable(0), Exp::Variable(1));
        let both_at_least_t = || {
            vec![
                compare(CompareOp::Ge, Exp::Variable(0), time()),
                compare(CompareOp::Ge, Exp::Variable(1), time()),
            ]
        };
        let twice = || times(num(2), time());
        let mut closed = both_at_least_t();
        closed.push(compare(CompareOp::Le, sum(), twice()));
        let (found, read) = joint_at(closed, &at);
        assert!(found);
        assert!(!read.is_empty());
        let mut open = both_at_least_t();
        open.push(compare(CompareOp::Lt, sum(), twice()));
        assert!(!joint_at(open, &at).0);
        let bounds = |ceiling: i64| {
            vec![
                compare(CompareOp::Ge, Exp::Variable(0), time()),
                compare(CompareOp::Ge, Exp::Variable(1), num(1)),
                compare(CompareOp::Le, sum(), num(ceiling)),
            ]
        };
        assert!(joint_at(bounds(3), &at).0);
        assert!(!joint_at(bounds(2), &at).0);
    }
}
