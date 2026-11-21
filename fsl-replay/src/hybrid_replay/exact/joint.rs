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
//! an upper bound, by the size of the two conditions' coefficients.

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

    /// `exp` as a linear form in the free variables.
    fn form(
        &self,
        exp: &Exp,
        env: &Env<'_>,
        at: &ExactReal,
        decisions: &mut Vec<Poly>,
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
                match self.form($e, env, at, decisions)? {
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
                let zero = at.sign_of(&divisor) == Ordering::Equal;
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
    ) -> Result<Option<Vec<Condition>>, HybridReplayRefusal> {
        let Exp::Compare(op, a, b) = conjunct else {
            return Err(self.gap());
        };
        let (Read::Form(a), Read::Form(b)) = (
            self.form(a, env, at, decisions)?,
            self.form(b, env, at, decisions)?,
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
            match self.conditions(conjunct, env, at, decisions)? {
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
                let sign = at.sign_of(&poly);
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
            let sign = at.sign_of(&poly);
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
