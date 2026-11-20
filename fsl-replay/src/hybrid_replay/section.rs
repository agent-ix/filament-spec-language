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
//! it is taken, with no fixed cap, so the meter bounds the run time). A step
//! is charged by the size of its operands: per coefficient the square of the
//! 64-bit words of its largest part, 200 to a work unit and at least
//! one unit ([`super::HybridReplayMeter`]). The meter is charged before every
//! pseudo-remainder step of a Sturm or Sturm-Tarski sequence (by the sizes of
//! the remainder and the divisor), every start of such a sequence, every
//! candidate tried when a rational that is not a root is searched for, every
//! step of the search for a root-free enclosing interval, every isolation
//! split, every refinement of a root's interval, every comparison of two
//! roots, every sample of the free variable, every reduction of a coefficient
//! modulo `u`'s polynomial, and (in [`super::poly2::projection`]) every
//! evaluation point of a subresultant interpolation. Every sign of a
//! polynomial in `t` at an irrational `u` is charged by [`ExactReal::sign_with`]:
//! the gcd with `u`'s polynomial, the square-free part, the Sturm sequence
//! (built once) and each bisection of `u`'s interval, the last by the sizes of
//! the sequence and the polynomial and by the degree times the size of the
//! interval's ends, which gain a bit a bisection. Exhaustion is returned as
//! the refusal, never a panic.
//!
//! Coefficients in `t` are reduced modulo `u`'s polynomial after every step,
//! which does not change their sign at `u` and keeps their degree below its
//! own, and each polynomial in `x` is scaled by the positive rational that
//! makes its parts integers of gcd 1, which does not change its signs at `u`
//! and keeps the sequences small. Root isolation separates the distinct roots of
//! `p(u, .)` by counting them with a Sturm-Tarski query, not by a squarefree
//! reduction.

use std::cell::RefCell;
use std::cmp::Ordering;

use filament_exact::Rational;

use super::eval::{eval_in, Bi, Env, Ev, Probe};
use super::expand::Exp;
use super::poly::{midpoint, Poly};
use super::poly2::Poly2;
use super::real::ExactReal;
use super::trace::zero;
use super::{ExactGap, HybridReplayMeter, HybridReplayRefusal};

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
    /// The polynomial `u` is a root of: coefficients in `t` are reduced
    /// modulo it, which leaves their value at `u` unchanged and keeps their
    /// degree below its own.
    modulus: Option<Poly>,
    meter: RefCell<&'m mut HybridReplayMeter>,
    /// The exhaustion a probe met where it cannot return one.
    failure: RefCell<Option<HybridReplayRefusal>>,
}

impl<'u, 'm> Section<'u, 'm> {
    pub(crate) fn new(u: &'u ExactReal, meter: &'m mut HybridReplayMeter) -> Self {
        Self {
            u,
            modulus: match u {
                ExactReal::Rational(_) => None,
                ExactReal::Root(root) => Some(Poly::new(root.coefficients().to_vec())),
            },
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

    /// Charge a step that works on `weight` words of operand, one per
    /// coefficient and per 64 bits of the largest, to the meter.
    pub(crate) fn charge_by(&self, weight: u64) -> Result<(), HybridReplayRefusal> {
        self.meter.borrow_mut().charge_by(weight)
    }

    /// `p` reduced modulo `u`'s polynomial: the same value at `u`.
    fn reduce(&self, p: &Poly) -> Result<Poly, HybridReplayRefusal> {
        match &self.modulus {
            Some(modulus) if p.degree() >= modulus.degree() => {
                self.charge_by(p.size().saturating_mul(modulus.size()))?;
                Ok(p.rem(modulus))
            }
            _ => Ok(p.clone()),
        }
    }

    /// The sign at `u` of the polynomial `p` in `t`; the work is charged
    /// before it is done, down to each bisection of `u`'s interval.
    fn sign_u(&self, p: &Poly) -> Result<Ordering, HybridReplayRefusal> {
        let p = self.reduce(p)?;
        self.u.sign_with(&p, &mut |weight| self.charge_by(weight))
    }

    /// `p` with every coefficient reduced modulo `u`'s polynomial, without
    /// the leading coefficients that vanish at `u`, and scaled by the
    /// positive rational that makes its parts integers of gcd 1 (a positive
    /// multiple has the same sign at `u`, and the sequences built from it
    /// stay small).
    fn trimmed(&self, p: &Poly2) -> Result<Poly2, HybridReplayRefusal> {
        let mut coefficients = Vec::with_capacity(p.coefficients().len());
        for c in p.coefficients() {
            coefficients.push(self.reduce(c)?);
        }
        while let Some(last) = coefficients.last() {
            if self.sign_u(last)? != Ordering::Equal {
                break;
            }
            coefficients.pop();
        }
        // The least positive multiple with integer parts of gcd 1.
        self.charge_by(
            coefficients
                .iter()
                .map(Poly::size)
                .fold(0, u64::saturating_add)
                .saturating_mul(2),
        )?;
        let mut divisor = zero();
        let mut multiple = one();
        for part in coefficients.iter().flat_map(|c| c.coefficients()) {
            let numerator = part.numerator().abs();
            divisor = Rational::from_integer(divisor.numerator().gcd(&numerator));
            let denominator = Rational::from_integer(part.denominator().clone());
            let shared = Rational::from_integer(multiple.numerator().gcd(denominator.numerator()));
            multiple = multiple
                .mul(&denominator)
                .div(&shared)
                .unwrap_or_else(|| unreachable!("a gcd is nonzero"));
        }
        if let Some(factor) = multiple.div(&divisor) {
            if factor != one() {
                coefficients = coefficients.iter().map(|c| c.scale(&factor)).collect();
            }
        }
        Ok(Poly2::new(coefficients))
    }

    fn lead_sign(&self, p: &Poly2) -> Result<Ordering, HybridReplayRefusal> {
        match p.coefficients().last() {
            Some(c) => self.sign_u(c),
            None => Ok(Ordering::Equal),
        }
    }

    /// The sign of the trimmed `p` at `bound`.
    fn sign_at(&self, p: &Poly2, bound: &Bound) -> Result<Ordering, HybridReplayRefusal> {
        match bound {
            Bound::At(x) => {
                self.charge_by(p.size())?;
                self.sign_u(&p.at_x(x))
            }
            Bound::Above => self.lead_sign(p),
            Bound::Below => {
                let lead = self.lead_sign(p)?;
                Ok(if p.degree().unwrap_or(0).is_multiple_of(2) {
                    lead
                } else {
                    lead.reverse()
                })
            }
        }
    }

    /// A positive multiple at `u` of `-(a rem b)`, for a trimmed nonzero `b`.
    fn negated_remainder(&self, a: &Poly2, b: &Poly2) -> Result<Poly2, HybridReplayRefusal> {
        let (Some(degree), Some(lead)) = (b.degree(), b.coefficients().last()) else {
            unreachable!("a divisor is nonzero")
        };
        let lead_sign = self.sign_u(lead)?;
        let mut remainder = self.trimmed(a)?;
        let mut steps = 0_usize;
        while let Some(top) = remainder.degree().filter(|top| *top >= degree) {
            let factor = remainder.coefficients()[top].clone();
            let mut shifted = vec![Poly::zero(); top - degree];
            shifted.extend(b.coefficients().iter().cloned());
            let subtrahend = Poly2::new(shifted).scale(&factor);
            // The step multiplies the remainder by the divisor's lead and
            // subtracts the divisor scaled by the remainder's lead.
            self.charge_by(
                remainder
                    .size()
                    .saturating_mul(lead.size())
                    .saturating_add(subtrahend.size()),
            )?;
            remainder = self.trimmed(&remainder.scale(lead).sub(&subtrahend))?;
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
        self.charge_by(p.size().saturating_mul(q.size()))?;
        let p = self.trimmed(p)?;
        let second = self.trimmed(&p.derivative().mul(q))?;
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

    fn variations(&self, sequence: &[Poly2], at: &Bound) -> Result<i64, HybridReplayRefusal> {
        let mut signs: Vec<Ordering> = Vec::new();
        for p in sequence {
            let sign = self.sign_at(p, at)?;
            if sign != Ordering::Equal {
                signs.push(sign);
            }
        }
        Ok(i64::try_from(signs.windows(2).filter(|w| w[0] != w[1]).count()).unwrap_or(i64::MAX))
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
        Ok(self.variations(&sequence, from)? - self.variations(&sequence, to)?)
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
                self.charge_by(p.size())?;
                if self.sign_u(&p.at_x(&candidate))? != Ordering::Equal {
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
        self.charge_by(p.size())?;
        while self.sign_u(&p.at_x(&x))? == Ordering::Equal {
            self.charge_by(p.size())?;
            x = x.add(step);
        }
        Ok(x)
    }

    /// The distinct real roots in `x` of `p(u, .)`, ascending. A polynomial
    /// that vanishes at `u` for every `x` has none.
    pub(crate) fn roots(&self, p: &Poly2) -> Result<Vec<Root2>, HybridReplayRefusal> {
        let p = self.trimmed(p)?;
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

    /// Rationals that separate the ascending distinct `roots`: one below the
    /// first, one between each two and one above the last.
    pub(crate) fn separators(&self, roots: &[Root2]) -> Result<Vec<Rational>, HybridReplayRefusal> {
        let (Some(first), Some(last)) = (roots.first(), roots.last()) else {
            return Ok(Vec::new());
        };
        let mut separators = vec![first.low.sub(&one())];
        for pair in roots.windows(2) {
            separators.push(self.between(&pair[0], &pair[1])?);
        }
        separators.push(last.high.add(&one()));
        Ok(separators)
    }

    /// The real number `root` is, given `over`, a rational polynomial one
    /// of whose roots it is (the resultant of `u`'s polynomial and the
    /// polynomial `root` is a root of): `root`'s interval is refined until
    /// it holds one root of `over` and neither end is one.
    pub(crate) fn real_root(
        &self,
        root: &Root2,
        over: &Poly,
    ) -> Result<ExactReal, HybridReplayRefusal> {
        self.charge_by(over.gcd_work(over))?;
        let over = over.square_free();
        if over.degree().unwrap_or(0) == 0 {
            return Err(ExactGap::IllKindedTerm.into());
        }
        // The Sturm sequence is built once for every interval tried.
        self.charge_by(over.gcd_work(&over))?;
        let sequence = over.sturm();
        let mut root = root.clone();
        loop {
            let ends = 1 + root.low.max_part_bits().max(root.high.max_part_bits()) / 64;
            let work = sequence
                .iter()
                .map(|p| p.horner_work(ends))
                .fold(0, u64::saturating_add);
            self.charge_by(work.saturating_mul(4))?;
            let apart = over.sign_at(&root.low) != Ordering::Equal
                && over.sign_at(&root.high) != Ordering::Equal;
            if apart && over.roots_between_by(&sequence, &root.low, &root.high) == 1 {
                let found =
                    over.roots_in_by(&root.low, &root.high, &mut |work| self.charge_by(work))?;
                let one = found
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| unreachable!("one root lies in the interval"));
                return Ok(one);
            }
            self.refine(&mut root)?;
        }
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
            Sample::Rational(x) => self
                .section
                .charge_by(poly.size())
                .and_then(|()| self.section.sign_u(&poly.at_x(x)))
                .unwrap_or_else(|refusal| {
                    *self.section.failure.borrow_mut() = Some(refusal);
                    Ordering::Equal
                }),
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
