// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real quantifier elimination over one post-jump variable (FR-450 "Exact
//! replay": the instants of a dwell at which a transition with a
//! non-deterministic reset is enabled). At a section `t = u`, `u` a real
//! algebraic number, every polynomial in the free variable `x` has
//! coefficients in `Q(u)`; its real roots in `x` are isolated by the
//! Sturm-Tarski sequence over those coefficients, whose signs are decided
//! exactly by the sign of a polynomial in `t` at `u`. No floating-point step
//! is taken and no field is constructed: a coefficient is a polynomial in
//! `t`, and zero is decided by [`ExactReal::sign_of`].
//!
//! Work accounting (FR-450 "Exact replay": each work step is charged before
//! it is taken, with no fixed cap). The meter is charged before every
//! pseudo-remainder step of a Sturm or Sturm-Tarski sequence, every start of
//! such a sequence, every candidate tried when a rational that is not a root
//! is searched for, every step of the search for a root-free enclosing
//! interval, every isolation split, every refinement of a root's interval,
//! every comparison of two roots, every sample of the free variable, and (in
//! [`super::poly2::projection`]) every evaluation point of a subresultant
//! interpolation. Exhaustion is returned as the refusal, never a panic.
//! Coefficients are not reduced between remainder steps (the sequences are
//! pseudo-remainder sequences, so coefficient growth is bounded only by the
//! per-step charge), and root isolation separates the distinct roots of
//! `p(u, .)` by counting them with a Sturm-Tarski query, not by a squarefree
//! reduction. Individual arithmetic operations on one coefficient are not
//! charged.

use std::cell::RefCell;
use std::cmp::Ordering;

use filament_exact::Rational;

use super::eval::{eval_in, Bi, Env, Ev, Probe};
use super::expand::Exp;
use super::poly::{midpoint, Poly};
use super::poly2::Poly2;
use super::real::ExactReal;
use super::trace::zero;
use super::{HybridReplayMeter, HybridReplayRefusal};

fn one() -> Rational {
    Rational::from_integer(1_i64.into())
}

/// An end of a stretch of the `x` line.
#[derive(Clone)]
enum Bound {
    Below,
    At(Rational),
    Above,
}

/// The one real root of `poly(u, .)` in `(low, high)`, neither end a root.
#[derive(Clone, Debug)]
pub(crate) struct Root2 {
    poly: Poly2,
    low: Rational,
    high: Rational,
}

/// A point of the `x` line at which a polynomial's sign is read.
#[derive(Clone, Debug)]
pub(crate) enum Sample {
    Rational(Rational),
    Root(Root2),
}

impl Sample {
    /// The point `x` of the line as a sample: a rational, or the one root of
    /// a rational polynomial in an isolating interval.
    pub(crate) fn of(x: &ExactReal) -> Self {
        match x {
            ExactReal::Rational(q) => Self::Rational(q.clone()),
            ExactReal::Root(root) => Self::Root(Root2 {
                poly: Poly2::new(
                    root.coefficients()
                        .iter()
                        .cloned()
                        .map(Poly::constant)
                        .collect(),
                ),
                low: root.lower().clone(),
                high: root.upper().clone(),
            }),
        }
    }
}

/// The line `t = u` over which polynomials in `x` are read.
pub(crate) struct Section<'u, 'm> {
    u: &'u ExactReal,
    meter: RefCell<&'m mut HybridReplayMeter>,
    /// The exhaustion a probe met where it cannot return one.
    failure: RefCell<Option<HybridReplayRefusal>>,
}

impl<'u, 'm> Section<'u, 'm> {
    pub(crate) fn new(u: &'u ExactReal, meter: &'m mut HybridReplayMeter) -> Self {
        Self {
            u,
            meter: RefCell::new(meter),
            failure: RefCell::new(None),
        }
    }

    /// The sign of `p` at the point `(u, at)`. A refusal met on the way is
    /// kept for [`Self::take_failure`], and the sign is then `Equal`.
    pub(crate) fn sign_of(&self, p: &Poly2, at: &Sample) -> Ordering {
        XPoint { section: self, at }.sign(p)
    }

    /// The refusal a [`Self::sign_of`] met, if any.
    pub(crate) fn take_failure(&self) -> Option<HybridReplayRefusal> {
        self.failure.borrow_mut().take()
    }

    /// Charge one work step to the meter.
    pub(crate) fn charge(&self) -> Result<(), HybridReplayRefusal> {
        self.meter.borrow_mut().charge()
    }

    /// `p` without the leading coefficients that vanish at `u`.
    fn trimmed(&self, p: &Poly2) -> Poly2 {
        let mut coefficients = p.coefficients().to_vec();
        while coefficients
            .last()
            .is_some_and(|c| self.u.sign_of(c) == Ordering::Equal)
        {
            coefficients.pop();
        }
        Poly2::new(coefficients)
    }

    fn lead_sign(&self, p: &Poly2) -> Ordering {
        p.coefficients()
            .last()
            .map_or(Ordering::Equal, |c| self.u.sign_of(c))
    }

    /// The sign of the trimmed `p` at `bound`.
    fn sign_at(&self, p: &Poly2, bound: &Bound) -> Ordering {
        match bound {
            Bound::At(x) => self.u.sign_of(&p.at_x(x)),
            Bound::Above => self.lead_sign(p),
            Bound::Below => {
                let lead = self.lead_sign(p);
                if p.degree().unwrap_or(0).is_multiple_of(2) {
                    lead
                } else {
                    lead.reverse()
                }
            }
        }
    }

    /// A positive multiple at `u` of `-(a rem b)`, for a trimmed nonzero `b`.
    fn negated_remainder(&self, a: &Poly2, b: &Poly2) -> Result<Poly2, HybridReplayRefusal> {
        let (Some(degree), Some(lead)) = (b.degree(), b.coefficients().last()) else {
            unreachable!("a divisor is nonzero")
        };
        let lead_sign = self.u.sign_of(lead);
        let mut remainder = self.trimmed(a);
        let mut steps = 0_usize;
        while let Some(top) = remainder.degree().filter(|top| *top >= degree) {
            let factor = remainder.coefficients()[top].clone();
            let mut shifted = vec![Poly::zero(); top - degree];
            shifted.extend(b.coefficients().iter().cloned());
            let subtrahend = Poly2::new(shifted).scale(&factor);
            self.charge()?;
            remainder = self.trimmed(&remainder.scale(lead).sub(&subtrahend));
            steps += 1;
        }
        Ok(if lead_sign == Ordering::Less && steps % 2 == 1 {
            remainder
        } else {
            remainder.neg()
        })
    }

    /// The Sturm-Tarski sequence of `p` and `q`.
    fn sequence(&self, p: &Poly2, q: &Poly2) -> Result<Vec<Poly2>, HybridReplayRefusal> {
        self.charge()?;
        let p = self.trimmed(p);
        let second = self.trimmed(&p.derivative().mul(q));
        let mut sequence = vec![p];
        if second.is_zero() {
            return Ok(sequence);
        }
        sequence.push(second);
        loop {
            let len = sequence.len();
            let next = self.negated_remainder(&sequence[len - 2], &sequence[len - 1])?;
            if next.is_zero() {
                return Ok(sequence);
            }
            sequence.push(next);
        }
    }

    fn variations(&self, sequence: &[Poly2], at: &Bound) -> i64 {
        let signs: Vec<Ordering> = sequence
            .iter()
            .map(|p| self.sign_at(p, at))
            .filter(|sign| *sign != Ordering::Equal)
            .collect();
        i64::try_from(signs.windows(2).filter(|w| w[0] != w[1]).count()).unwrap_or(i64::MAX)
    }

    /// The sum of the signs of `q` over the distinct real roots of `p(u, .)`
    /// strictly between `from` and `to`, neither a root of `p(u, .)`.
    fn tarski(
        &self,
        p: &Poly2,
        q: &Poly2,
        from: &Bound,
        to: &Bound,
    ) -> Result<i64, HybridReplayRefusal> {
        let sequence = self.sequence(p, q)?;
        Ok(self.variations(&sequence, from) - self.variations(&sequence, to))
    }

    fn count(&self, p: &Poly2, from: &Bound, to: &Bound) -> Result<i64, HybridReplayRefusal> {
        self.tarski(p, &Poly2::constant(Poly::constant(one())), from, to)
    }

    /// A rational strictly between `low` and `high` at which `p(u, .)` does
    /// not vanish.
    fn nonroot_between(
        &self,
        p: &Poly2,
        low: &Rational,
        high: &Rational,
    ) -> Result<Rational, HybridReplayRefusal> {
        let width = high.sub(low);
        for denominator in 2_usize.. {
            for numerator in 1..denominator {
                let fraction = Rational::from_integer(numerator.into())
                    .div(&Rational::from_integer(denominator.into()))
                    .unwrap_or_else(|| unreachable!("a denominator is nonzero"));
                let candidate = low.add(&width.mul(&fraction));
                self.charge()?;
                if self.u.sign_of(&p.at_x(&candidate)) != Ordering::Equal {
                    return Ok(candidate);
                }
            }
        }
        unreachable!("a nonzero polynomial has finitely many roots")
    }

    /// A rational at or above `start`, stepping up, at which `p(u, .)` does
    /// not vanish.
    fn nonroot_from(
        &self,
        p: &Poly2,
        start: Rational,
        step: &Rational,
    ) -> Result<Rational, HybridReplayRefusal> {
        let mut x = start;
        self.charge()?;
        while self.u.sign_of(&p.at_x(&x)) == Ordering::Equal {
            self.charge()?;
            x = x.add(step);
        }
        Ok(x)
    }

    /// The distinct real roots in `x` of `p(u, .)`, ascending. A polynomial
    /// that vanishes at `u` for every `x` has none.
    pub(crate) fn roots(&self, p: &Poly2) -> Result<Vec<Root2>, HybridReplayRefusal> {
        let p = self.trimmed(p);
        if p.degree().unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }
        let total = self.count(&p, &Bound::Below, &Bound::Above)?;
        let step = Rational::new(1_i64.into(), 7_i64.into())
            .unwrap_or_else(|_| unreachable!("7 is not zero"));
        let mut radius = one();
        let (mut low, mut high);
        loop {
            self.charge()?;
            high = self.nonroot_from(&p, radius.clone(), &step)?;
            low = self.nonroot_from(&p, radius.neg(), &step)?;
            if self.count(&p, &Bound::At(low.clone()), &Bound::At(high.clone()))? == total {
                break;
            }
            radius = radius.mul(&Rational::from_integer(2_i64.into()));
        }
        let mut roots = Vec::new();
        self.isolate(&p, low, high, total, &mut roots)?;
        Ok(roots)
    }

    fn isolate(
        &self,
        p: &Poly2,
        low: Rational,
        high: Rational,
        count: i64,
        out: &mut Vec<Root2>,
    ) -> Result<(), HybridReplayRefusal> {
        match count {
            0 => {}
            1 => out.push(Root2 {
                poly: p.clone(),
                low,
                high,
            }),
            _ => {
                self.charge()?;
                let middle = self.nonroot_between(p, &low, &high)?;
                let left = self.count(p, &Bound::At(low.clone()), &Bound::At(middle.clone()))?;
                self.isolate(p, low, middle.clone(), left, out)?;
                self.isolate(p, middle, high, count - left, out)?;
            }
        }
        Ok(())
    }

    /// Halve `root`'s isolating interval.
    fn refine(&self, root: &mut Root2) -> Result<(), HybridReplayRefusal> {
        self.charge()?;
        let middle = self.nonroot_between(&root.poly, &root.low, &root.high)?;
        let left = self.count(
            &root.poly,
            &Bound::At(root.low.clone()),
            &Bound::At(middle.clone()),
        )?;
        if left == 1 {
            root.high = middle;
        } else {
            root.low = middle;
        }
        Ok(())
    }

    /// The sign of `p` at the root `root`.
    fn sign_at_root(&self, p: &Poly2, root: &Root2) -> Result<Ordering, HybridReplayRefusal> {
        let value = self.tarski(
            &root.poly,
            p,
            &Bound::At(root.low.clone()),
            &Bound::At(root.high.clone()),
        )?;
        Ok(value.cmp(&0))
    }

    /// The order of two roots, refining their intervals as far as it takes.
    fn compare(&self, a: &Root2, b: &Root2) -> Result<Ordering, HybridReplayRefusal> {
        let (mut a, mut b) = (a.clone(), b.clone());
        loop {
            self.charge()?;
            if a.high <= b.low {
                return Ok(Ordering::Less);
            }
            if b.high <= a.low {
                return Ok(Ordering::Greater);
            }
            // A root on the other polynomial is the other's root exactly
            // when its interval ends up inside the other's, and apart from
            // it otherwise; a root off it is not the other's root, so the
            // two intervals end up apart.
            if self.sign_at_root(&b.poly, &a)? == Ordering::Equal {
                if a.low >= b.low && a.high <= b.high {
                    return Ok(Ordering::Equal);
                }
                self.refine(&mut a)?;
            } else {
                self.refine(&mut a)?;
                self.refine(&mut b)?;
            }
        }
    }

    /// A rational strictly between the roots `a` and `b`, `a` below `b`.
    fn between(&self, a: &Root2, b: &Root2) -> Result<Rational, HybridReplayRefusal> {
        let (mut a, mut b) = (a.clone(), b.clone());
        while a.high > b.low {
            self.refine(&mut a)?;
            self.refine(&mut b)?;
        }
        Ok(midpoint(&a.high, &b.low))
    }

    /// Add `root` to the ascending `roots` unless one is equal to it.
    fn insert(&self, roots: &mut Vec<Root2>, root: Root2) -> Result<bool, HybridReplayRefusal> {
        let mut position = roots.len();
        for (index, known) in roots.iter().enumerate() {
            match self.compare(&root, known)? {
                Ordering::Equal => return Ok(false),
                Ordering::Less => {
                    position = index;
                    break;
                }
                Ordering::Greater => {}
            }
        }
        roots.insert(position, root);
        Ok(true)
    }

    /// One sample per cell of the `x` line cut at `roots`, and one per root,
    /// ascending.
    fn samples(&self, roots: &[Root2]) -> Result<Vec<Sample>, HybridReplayRefusal> {
        let (Some(first), Some(last)) = (roots.first(), roots.last()) else {
            return Ok(vec![Sample::Rational(zero())]);
        };
        let mut samples = vec![Sample::Rational(first.low.sub(&one()))];
        for (index, root) in roots.iter().enumerate() {
            samples.push(Sample::Root(root.clone()));
            samples.push(Sample::Rational(match roots.get(index + 1) {
                Some(next) => self.between(root, next)?,
                None => last.high.add(&one()),
            }));
        }
        Ok(samples)
    }

    /// Whether some real `x` makes the Boolean `body` true, and every
    /// polynomial in `(t, x)` the evaluation read, once every real root of
    /// those at `u` has been used to cut the line.
    pub(crate) fn exists(
        &self,
        body: &Exp,
        env: &Env<'_, Bi>,
    ) -> Result<(bool, Vec<Poly2>), HybridReplayRefusal> {
        let mut read: Vec<Poly2> = Vec::new();
        let mut roots: Vec<Root2> = Vec::new();
        loop {
            let mut found = false;
            let mut grew = false;
            for sample in self.samples(&roots)? {
                self.charge()?;
                let mut decisions = Vec::new();
                let point = XPoint {
                    section: self,
                    at: &sample,
                };
                let value = eval_in::<Bi, _>(body, env, &point, &mut decisions);
                if let Some(refusal) = self.failure.borrow_mut().take() {
                    return Err(refusal);
                }
                found |= matches!(value?, Ev::Boolean(true));
                for decision in decisions {
                    if read.contains(&decision) {
                        continue;
                    }
                    for root in self.roots(&decision)? {
                        grew |= self.insert(&mut roots, root)?;
                    }
                    read.push(decision);
                }
            }
            if !grew {
                return Ok((found, read));
            }
        }
    }
}

/// A point `(u, x)` of the plane.
struct XPoint<'a, 'u, 'm> {
    section: &'a Section<'u, 'm>,
    at: &'a Sample,
}

impl Probe<Bi> for XPoint<'_, '_, '_> {
    fn sign(&self, poly: &Poly2) -> Ordering {
        match self.at {
            Sample::Rational(x) => self.section.u.sign_of(&poly.at_x(x)),
            Sample::Root(root) => self
                .section
                .sign_at_root(poly, root)
                .unwrap_or_else(|refusal| {
                    *self.section.failure.borrow_mut() = Some(refusal);
                    Ordering::Equal
                }),
        }
    }
}
