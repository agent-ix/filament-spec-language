// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact existential reset constraints. Symbolic execution preserves division
//! holes and left-to-right Boolean evaluation in a finite sign decision DAG.
//! Every branch's polynomials are collected before CAD projection or witness
//! search, so an early witness cannot hide a later time-cell boundary.

use std::cmp::Ordering;
use std::rc::Rc;

use filament_exact::Rational;
use fsl_semantics::check::{ArithmeticOp, CompareOp, LogicOp};

use super::super::eval::{Constant, Env};
use super::super::expand::Exp;
use super::super::poly::Poly;
use super::super::real::ExactReal;
use super::super::{ExactGap, HybridReplayMeter, HybridReplayRefusal};
use super::cad::{self, Kernel, Polynomial};

type Result<T> = std::result::Result<T, HybridReplayRefusal>;
type Node = Rc<Decision>;

pub(super) struct Joint {
    pub(super) variables: Vec<usize>,
    pub(super) conjuncts: Vec<Exp>,
}

#[derive(Clone)]
struct Fraction {
    numerator: Polynomial,
    denominator: Polynomial,
}

impl Fraction {
    fn of(numerator: Polynomial) -> Self {
        Self {
            numerator,
            denominator: Polynomial::constant(Rational::from_integer(1_i64.into())),
        }
    }

    fn arithmetic(
        &self,
        op: ArithmeticOp,
        b: &Self,
        meter: &mut HybridReplayMeter,
    ) -> Result<Self> {
        let (numerator, denominator) = match op {
            ArithmeticOp::Add | ArithmeticOp::Sub => {
                let a = self.numerator.mul(&b.denominator, meter)?;
                let b_top = b.numerator.mul(&self.denominator, meter)?;
                let numerator = match op {
                    ArithmeticOp::Add => a.add(&b_top, meter)?,
                    ArithmeticOp::Sub => a.sub(&b_top, meter)?,
                    ArithmeticOp::Mul | ArithmeticOp::Div => unreachable!("an additive operation"),
                };
                (numerator, self.denominator.mul(&b.denominator, meter)?)
            }
            ArithmeticOp::Mul => (
                self.numerator.mul(&b.numerator, meter)?,
                self.denominator.mul(&b.denominator, meter)?,
            ),
            ArithmeticOp::Div => (
                self.numerator.mul(&b.denominator, meter)?,
                self.denominator.mul(&b.numerator, meter)?,
            ),
        };
        Ok(Self {
            numerator,
            denominator,
        })
    }
}

/// Undefined leaves represent hypothetical assignments with no value. They
/// never satisfy the existential; actual trace evaluation retains its own
/// existing cause and locus through the ordinary replay evaluator.
enum Decision {
    Number(Fraction),
    Boolean(bool),
    Undefined,
    Sign {
        polynomial: Polynomial,
        less: Node,
        equal: Node,
        greater: Node,
    },
}

impl Decision {
    fn boolean(value: bool) -> Node {
        Rc::new(Self::Boolean(value))
    }

    fn select(
        condition: &Node,
        yes: &Node,
        no: &Node,
        meter: &mut HybridReplayMeter,
    ) -> Result<Node> {
        meter.charge()?;
        Ok(match condition.as_ref() {
            Self::Boolean(true) => yes.clone(),
            Self::Boolean(false) => no.clone(),
            Self::Undefined => condition.clone(),
            Self::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
            Self::Sign {
                polynomial,
                less,
                equal,
                greater,
            } => Rc::new(Self::Sign {
                polynomial: polynomial.clone(),
                less: Self::select(less, yes, no, meter)?,
                equal: Self::select(equal, yes, no, meter)?,
                greater: Self::select(greater, yes, no, meter)?,
            }),
        })
    }

    fn binary(a: &Node, b: &Node, op: Operation, meter: &mut HybridReplayMeter) -> Result<Node> {
        meter.charge()?;
        let split = |polynomial: &Polynomial,
                     less: &Node,
                     equal: &Node,
                     greater: &Node,
                     left: bool,
                     meter: &mut HybridReplayMeter|
         -> Result<Node> {
            let mut apply = |child: &Node| {
                if left {
                    Self::binary(child, b, op, meter)
                } else {
                    Self::binary(a, child, op, meter)
                }
            };
            Ok(Rc::new(Self::Sign {
                polynomial: polynomial.clone(),
                less: apply(less)?,
                equal: apply(equal)?,
                greater: apply(greater)?,
            }))
        };
        match a.as_ref() {
            Self::Undefined => return Ok(a.clone()),
            Self::Sign {
                polynomial,
                less,
                equal,
                greater,
            } => return split(polynomial, less, equal, greater, true, meter),
            Self::Number(_) | Self::Boolean(_) => {}
        }
        match b.as_ref() {
            Self::Undefined => return Ok(b.clone()),
            Self::Sign {
                polynomial,
                less,
                equal,
                greater,
            } => return split(polynomial, less, equal, greater, false, meter),
            Self::Number(_) | Self::Boolean(_) => {}
        }
        let (Self::Number(a), Self::Number(b)) = (a.as_ref(), b.as_ref()) else {
            return Err(ExactGap::IllKindedTerm.into());
        };
        Ok(match op {
            Operation::Arithmetic(operation) => {
                let value = Rc::new(Self::Number(a.arithmetic(operation, b, meter)?));
                if operation == ArithmeticOp::Div {
                    Rc::new(Self::Sign {
                        polynomial: b.numerator.clone(),
                        less: value.clone(),
                        equal: Rc::new(Self::Undefined),
                        greater: value,
                    })
                } else {
                    value
                }
            }
            Operation::Compare(operation) => {
                let difference = a.arithmetic(ArithmeticOp::Sub, b, meter)?;
                let polynomial = difference.numerator.mul(&difference.denominator, meter)?;
                let holds = |sign| {
                    Self::boolean(match operation {
                        CompareOp::Eq => sign == Ordering::Equal,
                        CompareOp::Ne => sign != Ordering::Equal,
                        CompareOp::Lt => sign == Ordering::Less,
                        CompareOp::Le => sign != Ordering::Greater,
                        CompareOp::Gt => sign == Ordering::Greater,
                        CompareOp::Ge => sign != Ordering::Less,
                    })
                };
                Rc::new(Self::Sign {
                    polynomial,
                    less: holds(Ordering::Less),
                    equal: holds(Ordering::Equal),
                    greater: holds(Ordering::Greater),
                })
            }
        })
    }

    fn collect(
        node: &Node,
        set: &mut Vec<Polynomial>,
        visited: &mut std::collections::BTreeSet<usize>,
        meter: &mut HybridReplayMeter,
    ) -> Result<()> {
        meter.charge()?;
        // Rc identity avoids traversing shared continuations repeatedly.
        if !visited.insert(Rc::as_ptr(node) as usize) {
            return Ok(());
        }
        match node.as_ref() {
            Self::Sign {
                polynomial,
                less,
                equal,
                greater,
            } => {
                if !set.contains(polynomial) {
                    set.push(polynomial.clone());
                }
                for child in [less, equal, greater] {
                    Self::collect(child, set, visited, meter)?;
                }
            }
            Self::Number(_) | Self::Boolean(_) | Self::Undefined => {}
        }
        Ok(())
    }

    fn holds(node: &Node, kernel: &mut Kernel<'_>) -> Result<bool> {
        kernel.meter.charge()?;
        match node.as_ref() {
            Self::Boolean(value) => Ok(*value),
            Self::Undefined => Ok(false),
            Self::Number(_) => Err(ExactGap::IllKindedTerm.into()),
            Self::Sign {
                polynomial,
                less,
                equal,
                greater,
            } => {
                let child = match kernel.sign(polynomial)? {
                    Ordering::Less => less,
                    Ordering::Equal => equal,
                    Ordering::Greater => greater,
                };
                Self::holds(child, kernel)
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Arithmetic(ArithmeticOp),
    Compare(CompareOp),
}

impl Joint {
    fn lower(&self, exp: &Exp, env: &Env<'_>, meter: &mut HybridReplayMeter) -> Result<Node> {
        meter.charge()?;
        let number = |p| Rc::new(Decision::Number(Fraction::of(p)));
        Ok(match exp {
            Exp::Number(q) => number(Polynomial::constant(q.clone())),
            Exp::Boolean(b) => Decision::boolean(*b),
            Exp::Variable(i) => {
                if let Some(coordinate) = self.variables.iter().position(|v| v == i) {
                    number(Polynomial::variable(coordinate + 1))
                } else {
                    number(Polynomial::time(
                        env.variables.get(*i).ok_or(ExactGap::IllKindedTerm)?,
                    ))
                }
            }
            Exp::Activation(i) => number(Polynomial::time(
                env.activation.get(*i).ok_or(ExactGap::IllKindedTerm)?,
            )),
            Exp::Capture(i) => match env.captures.get(*i).ok_or(ExactGap::IllKindedTerm)? {
                Constant::Number(q) => number(Polynomial::constant(q.clone())),
                Constant::Boolean(b) => Decision::boolean(*b),
            },
            Exp::Parameter(_) | Exp::Recursive(_) => return Err(ExactGap::RecursiveCall.into()),
            Exp::Negate(a) => Decision::binary(
                &number(Polynomial::constant(Rational::from_integer(0_i64.into()))),
                &self.lower(a, env, meter)?,
                Operation::Arithmetic(ArithmeticOp::Sub),
                meter,
            )?,
            Exp::Not(a) => Decision::select(
                &self.lower(a, env, meter)?,
                &Decision::boolean(false),
                &Decision::boolean(true),
                meter,
            )?,
            Exp::Arithmetic(op, a, b, _) => Decision::binary(
                &self.lower(a, env, meter)?,
                &self.lower(b, env, meter)?,
                Operation::Arithmetic(*op),
                meter,
            )?,
            Exp::Compare(op, a, b) => Decision::binary(
                &self.lower(a, env, meter)?,
                &self.lower(b, env, meter)?,
                Operation::Compare(*op),
                meter,
            )?,
            Exp::Logic(op, a, b) => {
                let a = self.lower(a, env, meter)?;
                let b = self.lower(b, env, meter)?;
                match op {
                    LogicOp::And => Decision::select(&a, &b, &Decision::boolean(false), meter)?,
                    LogicOp::Or => Decision::select(&a, &Decision::boolean(true), &b, meter)?,
                    LogicOp::Implies => Decision::select(&a, &b, &Decision::boolean(true), meter)?,
                }
            }
            Exp::Conditional(c, a, b, _) => Decision::select(
                &self.lower(c, env, meter)?,
                &self.lower(a, env, meter)?,
                &self.lower(b, env, meter)?,
                meter,
            )?,
            Exp::Ranged { value, range, .. } => {
                let value = self.lower(value, env, meter)?;
                let mut inside = Decision::boolean(true);
                for (bound, op) in [(&range.lower, CompareOp::Ge), (&range.upper, CompareOp::Le)] {
                    if let Some(bound) = bound {
                        let check = Decision::binary(
                            &value,
                            &number(Polynomial::constant(bound.clone())),
                            Operation::Compare(op),
                            meter,
                        )?;
                        inside =
                            Decision::select(&inside, &check, &Decision::boolean(false), meter)?;
                    }
                }
                Decision::select(&inside, &value, &Rc::new(Decision::Undefined), meter)?
            }
        })
    }

    pub(super) fn exists(
        &self,
        env: &Env<'_>,
        at: &ExactReal,
        decisions: &mut Vec<Poly>,
        meter: &mut HybridReplayMeter,
    ) -> Result<bool> {
        let mut body = Decision::boolean(true);
        for conjunct in &self.conjuncts {
            let part = self.lower(conjunct, env, meter)?;
            body = Decision::select(&body, &part, &Decision::boolean(false), meter)?;
        }
        let mut set = Vec::new();
        Decision::collect(
            &body,
            &mut set,
            &mut std::collections::BTreeSet::new(),
            meter,
        )?;
        let (levels, projected) = cad::project(set, self.variables.len(), meter)?;
        decisions.extend(projected);
        Kernel::new(at, meter).exists(&levels, &mut |kernel| Decision::holds(&body, kernel))
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

#[cfg(test)]
mod nonlinear_tests {
    use super::super::super::expand::TermLocus;
    use super::*;
    use fsl_semantics::check::LogicOp;

    fn n(i: i64) -> Rational {
        Rational::from_integer(i.into())
    }
    fn q(i: i64) -> Exp {
        Exp::Number(n(i))
    }
    fn v(i: usize) -> Exp {
        Exp::Variable(i)
    }
    fn op(op: ArithmeticOp, a: Exp, b: Exp) -> Exp {
        Exp::Arithmetic(op, Box::new(a), Box::new(b), TermLocus::default())
    }
    fn mul(a: Exp, b: Exp) -> Exp {
        op(ArithmeticOp::Mul, a, b)
    }
    fn cmp(op: CompareOp, a: Exp, b: Exp) -> Exp {
        Exp::Compare(op, Box::new(a), Box::new(b))
    }
    fn logic(op: LogicOp, a: Exp, b: Exp) -> Exp {
        Exp::Logic(op, Box::new(a), Box::new(b))
    }
    fn run(
        parts: Vec<Exp>,
        dimensions: usize,
        at: &ExactReal,
        bound: u64,
    ) -> (Result<bool>, Vec<Poly>, u64) {
        let mut variables = vec![Poly::zero(); dimensions];
        variables.push(Poly::new(vec![n(0), n(1)]));
        let env = Env {
            variables: &variables,
            activation: &variables,
            captures: &[],
        };
        let joint = Joint {
            variables: (0..dimensions).collect(),
            conjuncts: parts,
        };
        let mut decisions = Vec::new();
        let mut meter = HybridReplayMeter::new(bound);
        let result = joint.exists(&env, at, &mut decisions, &mut meter);
        (result, decisions, meter.consumed)
    }

    fn product_parts(dimensions: usize, comparison: CompareOp) -> Vec<Exp> {
        let mut parts = (0..dimensions)
            .map(|i| cmp(CompareOp::Ge, v(i), v(dimensions)))
            .collect::<Vec<_>>();
        let product = (1..dimensions).fold(v(0), |a, i| mul(a, v(i)));
        parts.push(cmp(comparison, mul(q(2), product), q(1)));
        parts
    }

    /// Trace: FR-450
    /// Positive lower bounds force the product minimum t^d. The analytical
    /// boundary is the positive root of 2t^d-1, including its exact section.
    #[test]
    fn cad_product_boundary_in_two_and_three_dimensions() {
        for dimensions in [2, 3] {
            let mut coefficients = vec![n(-1)];
            coefficients.resize(dimensions, n(0));
            coefficients.push(n(2));
            let polynomial = Poly::new(coefficients);
            let at = polynomial
                .roots_in(&n(0), &n(1))
                .into_iter()
                .next()
                .expect("one positive boundary");
            let closed = run(
                product_parts(dimensions, CompareOp::Le),
                dimensions,
                &at,
                u64::MAX,
            );
            assert_eq!(closed.0, Ok(true), "dimension {dimensions}");
            assert!(
                closed
                    .1
                    .iter()
                    .any(|p| at.sign_of(p) == Ordering::Equal && p.degree().unwrap_or(0) > 0),
                "projection must expose the irrational boundary"
            );
            assert_eq!(
                run(
                    product_parts(dimensions, CompareOp::Lt),
                    dimensions,
                    &at,
                    u64::MAX
                )
                .0,
                Ok(false)
            );
            assert_eq!(
                run(
                    product_parts(dimensions, CompareOp::Le),
                    dimensions,
                    &ExactReal::Rational(n(1)),
                    u64::MAX
                )
                .0,
                Ok(false)
            );
        }
    }

    /// Trace: FR-450
    #[test]
    fn cad_denominator_holes_and_short_circuit_branches() {
        let quotient = op(
            ArithmeticOp::Div,
            op(ArithmeticOp::Sub, v(0), v(1)),
            op(ArithmeticOp::Sub, v(0), v(1)),
        );
        let hole = cmp(CompareOp::Eq, v(0), v(1));
        let quotient_one = cmp(CompareOp::Eq, quotient.clone(), q(1));
        let at = ExactReal::Rational(n(0));
        assert_eq!(
            run(vec![hole.clone(), quotient_one.clone()], 2, &at, u64::MAX).0,
            Ok(false)
        );
        assert_eq!(
            run(
                vec![
                    hole.clone(),
                    logic(LogicOp::Or, Exp::Boolean(true), quotient_one.clone())
                ],
                2,
                &at,
                u64::MAX
            )
            .0,
            Ok(true)
        );
        assert_eq!(
            run(
                vec![
                    hole.clone(),
                    logic(LogicOp::Or, quotient_one.clone(), Exp::Boolean(true))
                ],
                2,
                &at,
                u64::MAX
            )
            .0,
            Ok(false)
        );
        let branch = Exp::Conditional(
            Box::new(hole),
            Box::new(Exp::Boolean(true)),
            Box::new(quotient_one),
            TermLocus::default(),
        );
        assert_eq!(run(vec![branch], 2, &at, u64::MAX).0, Ok(true));
    }

    /// Trace: FR-450
    #[test]
    fn cad_meter_repeats_and_denies_the_last_actual_charge() {
        let parts = product_parts(2, CompareOp::Le);
        let at = ExactReal::Rational(n(0));
        let first = run(parts.clone(), 2, &at, u64::MAX);
        let repeated = run(parts.clone(), 2, &at, u64::MAX);
        assert_eq!(first, repeated);
        assert_eq!(first.0, Ok(true));
        let work = first.2;
        assert!(work > 0);
        let exact = run(parts.clone(), 2, &at, work);
        assert_eq!(exact.0, Ok(true));
        assert_eq!(exact.2, work);
        assert!(matches!(
            run(parts, 2, &at, work - 1).0,
            Err(HybridReplayRefusal::ResourceExhausted { .. })
        ));
    }
}
