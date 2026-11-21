// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sign-invariant cylindrical decomposition with rational projection and
//! triangular algebraic samples. Coordinates are ordered time, then reset
//! variables. Coefficients are never divided by algebraic values: specialized
//! signed pseudo-remainders and Sturm-Tarski queries reduce each sign to the
//! shorter, actual sample prefix, including reducible defining polynomials.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use filament_exact::Rational;

use super::super::poly::{midpoint, square_of_words, Poly};
use super::super::real::ExactReal;
use super::super::trace::zero;
use super::super::{HybridReplayMeter, HybridReplayRefusal};

type Result<T> = std::result::Result<T, HybridReplayRefusal>;

fn one() -> Rational {
    Rational::from_integer(1_i64.into())
}

/// Sparse rational polynomial. Trailing zero exponents and zero terms are
/// absent; coordinate zero is time. Missing coordinates have exponent zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Polynomial(BTreeMap<Vec<usize>, Rational>);

impl Polynomial {
    pub(super) fn constant(value: Rational) -> Self {
        if value.is_zero() {
            Self(BTreeMap::new())
        } else {
            Self(BTreeMap::from([(Vec::new(), value)]))
        }
    }

    pub(super) fn variable(coordinate: usize) -> Self {
        let mut powers = vec![0; coordinate + 1];
        powers[coordinate] = 1;
        Self(BTreeMap::from([(powers, one())]))
    }

    pub(super) fn time(poly: &Poly) -> Self {
        Self(
            poly.coefficients()
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    (!c.is_zero()).then(|| (if i == 0 { vec![] } else { vec![i] }, c.clone()))
                })
                .collect(),
        )
    }

    fn size(&self) -> u64 {
        self.0
            .iter()
            .map(|(powers, c)| {
                square_of_words(1 + c.max_part_bits() / 64)
                    .saturating_add(u64::try_from(powers.len()).unwrap_or(u64::MAX))
            })
            .fold(0, u64::saturating_add)
            .max(1)
    }

    fn degree(&self, coordinate: usize) -> Option<usize> {
        self.0
            .keys()
            .map(|p| p.get(coordinate).copied().unwrap_or(0))
            .max()
    }

    fn coefficient(&self, coordinate: usize, degree: usize) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(powers, c)| {
                    if powers.get(coordinate).copied().unwrap_or(0) != degree {
                        return None;
                    }
                    let mut powers = powers.clone();
                    if let Some(power) = powers.get_mut(coordinate) {
                        *power = 0;
                    }
                    while powers.last() == Some(&0) {
                        powers.pop();
                    }
                    Some((powers, c.clone()))
                })
                .collect(),
        )
    }

    pub(super) fn add(&self, other: &Self, meter: &mut HybridReplayMeter) -> Result<Self> {
        meter.charge_by(self.size().saturating_add(other.size()))?;
        let mut terms = self.0.clone();
        for (powers, c) in &other.0 {
            let next = terms.get(powers).map_or_else(|| c.clone(), |a| a.add(c));
            if next.is_zero() {
                terms.remove(powers);
            } else {
                terms.insert(powers.clone(), next);
            }
        }
        Ok(Self(terms))
    }

    pub(super) fn neg(&self, meter: &mut HybridReplayMeter) -> Result<Self> {
        meter.charge_by(self.size())?;
        Ok(Self(
            self.0.iter().map(|(p, c)| (p.clone(), c.neg())).collect(),
        ))
    }

    pub(super) fn sub(&self, other: &Self, meter: &mut HybridReplayMeter) -> Result<Self> {
        self.add(&other.neg(meter)?, meter)
    }

    pub(super) fn mul(&self, other: &Self, meter: &mut HybridReplayMeter) -> Result<Self> {
        meter.charge_by(self.size().saturating_mul(other.size()))?;
        let mut terms = BTreeMap::<Vec<usize>, Rational>::new();
        for (a, ac) in &self.0 {
            for (b, bc) in &other.0 {
                let powers = (0..a.len().max(b.len()))
                    .map(|i| a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0))
                    .collect::<Vec<_>>();
                let c = ac.mul(bc);
                let c = terms
                    .get(&powers)
                    .map_or_else(|| c.clone(), |old| old.add(&c));
                if c.is_zero() {
                    terms.remove(&powers);
                } else {
                    terms.insert(powers, c);
                }
            }
        }
        Ok(Self(terms))
    }

    fn derivative(&self, coordinate: usize, meter: &mut HybridReplayMeter) -> Result<Self> {
        meter.charge_by(self.size())?;
        Ok(Self(
            self.0
                .iter()
                .filter_map(|(p, c)| {
                    let degree = p.get(coordinate).copied().unwrap_or(0);
                    if degree == 0 {
                        return None;
                    }
                    let mut powers = p.clone();
                    powers[coordinate] -= 1;
                    while powers.last() == Some(&0) {
                        powers.pop();
                    }
                    Some((powers, c.mul(&Rational::from_integer(degree.into()))))
                })
                .collect(),
        ))
    }

    fn shifted(
        &self,
        coordinate: usize,
        degree: usize,
        meter: &mut HybridReplayMeter,
    ) -> Result<Self> {
        meter.charge_by(self.size())?;
        Ok(Self(
            self.0
                .iter()
                .map(|(p, c)| {
                    let mut powers = p.clone();
                    powers.resize(powers.len().max(coordinate + 1), 0);
                    powers[coordinate] += degree;
                    while powers.last() == Some(&0) {
                        powers.pop();
                    }
                    (powers, c.clone())
                })
                .collect(),
        ))
    }

    fn at(
        &self,
        coordinate: usize,
        value: &Rational,
        meter: &mut HybridReplayMeter,
    ) -> Result<Self> {
        let mut result = Self::constant(zero());
        for degree in (0..=self.degree(coordinate).unwrap_or(0)).rev() {
            meter.charge_by(self.size())?;
            result = result
                .mul(&Self::constant(value.clone()), meter)?
                .add(&self.coefficient(coordinate, degree), meter)?;
        }
        Ok(result)
    }

    /// Positive rational scaling keeps signs and limits rational content in
    /// pseudo-remainder sequences without any quotient-ring assumption.
    fn primitive(&self, meter: &mut HybridReplayMeter) -> Result<Self> {
        meter.charge_by(self.size())?;
        let mut gcd = zero();
        let mut lcm = one();
        for c in self.0.values() {
            gcd = Rational::from_integer(gcd.numerator().gcd(&c.numerator().abs()));
            let denominator = Rational::from_integer(c.denominator().clone());
            let shared = Rational::from_integer(lcm.numerator().gcd(denominator.numerator()));
            lcm = lcm
                .mul(&denominator)
                .div(&shared)
                .expect("a denominator gcd is nonzero");
        }
        match lcm.div(&gcd) {
            Some(factor) => self.mul(&Self::constant(factor), meter),
            None => Ok(self.clone()),
        }
    }

    fn univariate(&self) -> Poly {
        let degree = self.degree(0).unwrap_or(0);
        Poly::new(
            (0..=degree)
                .map(|i| {
                    self.0
                        .get(&if i == 0 { vec![] } else { vec![i] })
                        .cloned()
                        .unwrap_or_else(zero)
                })
                .collect(),
        )
    }
}

/// An isolated root is defined over all earlier coordinates, not an
/// independent algebraic number. The rational ends are not defining roots.
#[derive(Clone, Debug)]
struct Root {
    polynomial: Polynomial,
    low: Rational,
    high: Rational,
}

#[derive(Clone, Debug)]
enum Coordinate {
    Rational(Rational),
    Root(Root),
}

#[derive(Clone)]
enum Bound {
    Below,
    At(Rational),
    Above,
}

pub(super) struct Kernel<'a> {
    pub(super) time: &'a ExactReal,
    pub(super) meter: &'a mut HybridReplayMeter,
    prefix: Vec<Coordinate>,
}

impl<'a> Kernel<'a> {
    pub(super) fn new(time: &'a ExactReal, meter: &'a mut HybridReplayMeter) -> Self {
        Self {
            time,
            meter,
            prefix: Vec::new(),
        }
    }

    pub(super) fn sign(&mut self, p: &Polynomial) -> Result<Ordering> {
        self.sign_over(p, self.prefix.len())
    }

    fn sign_over(&mut self, p: &Polynomial, depth: usize) -> Result<Ordering> {
        self.meter.charge_by(p.size())?;
        if p.0.is_empty() {
            return Ok(Ordering::Equal);
        }
        if depth == 0 {
            return self
                .time
                .sign_with(&p.univariate(), &mut |w| self.meter.charge_by(w));
        }
        if p.degree(depth) == Some(0) {
            return self.sign_over(p, depth - 1);
        }
        match self.prefix[depth - 1].clone() {
            Coordinate::Rational(q) => {
                let p = p.at(depth, &q, self.meter)?;
                self.sign_over(&p, depth - 1)
            }
            Coordinate::Root(root) => {
                let value = self.tarski(
                    &root.polynomial,
                    p,
                    depth,
                    &Bound::At(root.low),
                    &Bound::At(root.high),
                )?;
                Ok(value.cmp(&0))
            }
        }
    }

    /// Specialization can lower the degree at every remainder, and can
    /// nullify an entire fiber. Drop those coefficients before all degree,
    /// parity, leading-sign and infinity decisions.
    fn trimmed(&mut self, p: &Polynomial, coordinate: usize) -> Result<Polynomial> {
        let mut p = p.clone();
        while let Some(degree) = p.degree(coordinate) {
            self.meter.charge_by(p.size())?;
            if self.sign_over(&p.coefficient(coordinate, degree), coordinate - 1)?
                != Ordering::Equal
            {
                break;
            }
            p.0.retain(|powers, _| powers.get(coordinate).copied().unwrap_or(0) != degree);
        }
        p.primitive(self.meter)
    }

    fn sign_at(&mut self, p: &Polynomial, coordinate: usize, bound: &Bound) -> Result<Ordering> {
        match bound {
            Bound::At(q) => {
                let p = p.at(coordinate, q, self.meter)?;
                self.sign_over(&p, coordinate - 1)
            }
            Bound::Below | Bound::Above => {
                let Some(degree) = p.degree(coordinate) else {
                    return Ok(Ordering::Equal);
                };
                let lead = self.sign_over(&p.coefficient(coordinate, degree), coordinate - 1)?;
                Ok(if matches!(bound, Bound::Below) && degree % 2 == 1 {
                    lead.reverse()
                } else {
                    lead
                })
            }
        }
    }

    /// Positive multiple of the negative Euclidean remainder. Each actual
    /// cancellation multiplies by the specialized divisor lead; correct
    /// its sign using the ACTUAL number of steps, including degree drops.
    fn negated_remainder(
        &mut self,
        a: &Polynomial,
        b: &Polynomial,
        coordinate: usize,
    ) -> Result<Polynomial> {
        let mut remainder = self.trimmed(a, coordinate)?;
        let degree = b.degree(coordinate).expect("a PRS divisor is nonzero");
        let lead = b.coefficient(coordinate, degree);
        let negative = self.sign_over(&lead, coordinate - 1)? == Ordering::Less;
        let mut odd = false;
        while let Some(top) = remainder.degree(coordinate).filter(|top| *top >= degree) {
            self.meter
                .charge_by(remainder.size().saturating_mul(b.size()))?;
            let factor = remainder.coefficient(coordinate, top);
            let subtrahend = b
                .shifted(coordinate, top - degree, self.meter)?
                .mul(&factor, self.meter)?;
            remainder = remainder
                .mul(&lead, self.meter)?
                .sub(&subtrahend, self.meter)?;
            remainder = self.trimmed(&remainder, coordinate)?;
            odd = !odd;
        }
        if negative && odd {
            Ok(remainder)
        } else {
            remainder.neg(self.meter)
        }
    }

    fn tarski(
        &mut self,
        p: &Polynomial,
        q: &Polynomial,
        coordinate: usize,
        low: &Bound,
        high: &Bound,
    ) -> Result<i64> {
        self.meter.charge_by(p.size().saturating_mul(q.size()))?;
        let p = self.trimmed(p, coordinate)?;
        let second = p.derivative(coordinate, self.meter)?.mul(q, self.meter)?;
        let second = self.trimmed(&second, coordinate)?;
        let mut sequence = vec![p];
        if !second.0.is_empty() {
            sequence.push(second);
        }
        while sequence.len() > 1 {
            let len = sequence.len();
            let next =
                self.negated_remainder(&sequence[len - 2], &sequence[len - 1], coordinate)?;
            if next.0.is_empty() {
                break;
            }
            sequence.push(next);
        }
        let mut variation = |bound: &Bound| -> Result<i64> {
            let mut previous = Ordering::Equal;
            let mut changes = 0_i64;
            for p in &sequence {
                let sign = self.sign_at(p, coordinate, bound)?;
                if sign != Ordering::Equal {
                    if previous != Ordering::Equal && previous != sign {
                        changes += 1;
                    }
                    previous = sign;
                }
            }
            Ok(changes)
        };
        Ok(variation(low)? - variation(high)?)
    }

    fn count(
        &mut self,
        p: &Polynomial,
        coordinate: usize,
        low: &Bound,
        high: &Bound,
    ) -> Result<i64> {
        self.tarski(p, &Polynomial::constant(one()), coordinate, low, high)
    }

    fn nonroot_between(
        &mut self,
        p: &Polynomial,
        coordinate: usize,
        low: &Rational,
        high: &Rational,
    ) -> Result<Rational> {
        let width = high.sub(low);
        // The candidate sequence converges to low and therefore eventually
        // avoids the finitely many roots. Its rational size is metered.
        let mut denominator = Rational::from_integer(2_i64.into());
        loop {
            self.meter.charge_by(
                p.size()
                    .saturating_add(square_of_words(1 + denominator.max_part_bits() / 64)),
            )?;
            let candidate = low.add(&width.div(&denominator).expect("a positive denominator"));
            if self.sign_at(p, coordinate, &Bound::At(candidate.clone()))? != Ordering::Equal {
                return Ok(candidate);
            }
            denominator = denominator.add(&one());
        }
    }

    fn roots(&mut self, p: &Polynomial, coordinate: usize) -> Result<Vec<Root>> {
        let p = self.trimmed(p, coordinate)?;
        if p.degree(coordinate).unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }
        let total = self.count(&p, coordinate, &Bound::Below, &Bound::Above)?;
        let mut radius = one();
        let (low, high) = loop {
            self.meter.charge_by(
                p.size()
                    .saturating_add(square_of_words(1 + radius.max_part_bits() / 64)),
            )?;
            let low = radius.neg();
            let high = radius.clone();
            if self.sign_at(&p, coordinate, &Bound::At(low.clone()))? != Ordering::Equal
                && self.sign_at(&p, coordinate, &Bound::At(high.clone()))? != Ordering::Equal
                && self.count(
                    &p,
                    coordinate,
                    &Bound::At(low.clone()),
                    &Bound::At(high.clone()),
                )? == total
            {
                break (low, high);
            }
            radius = radius.mul(&Rational::from_integer(2_i64.into()));
        };
        let mut pending = vec![(low, high, total)];
        let mut roots = Vec::new();
        while let Some((low, high, count)) = pending.pop() {
            self.meter.charge()?;
            match count {
                0 => {}
                1 => roots.push(Root {
                    polynomial: p.clone(),
                    low,
                    high,
                }),
                _ => {
                    let middle = self.nonroot_between(&p, coordinate, &low, &high)?;
                    let left = self.count(
                        &p,
                        coordinate,
                        &Bound::At(low.clone()),
                        &Bound::At(middle.clone()),
                    )?;
                    pending.push((middle.clone(), high, count - left));
                    pending.push((low, middle, left));
                }
            }
        }
        Ok(roots)
    }

    fn refine(&mut self, root: &mut Root, coordinate: usize) -> Result<()> {
        self.meter.charge_by(root.polynomial.size())?;
        let middle = self.nonroot_between(&root.polynomial, coordinate, &root.low, &root.high)?;
        if self.count(
            &root.polynomial,
            coordinate,
            &Bound::At(root.low.clone()),
            &Bound::At(middle.clone()),
        )? == 1
        {
            root.high = middle;
        } else {
            root.low = middle;
        }
        Ok(())
    }

    fn compare(&mut self, a: &Root, b: &Root, coordinate: usize) -> Result<Ordering> {
        let (mut a, mut b) = (a.clone(), b.clone());
        loop {
            self.meter
                .charge_by(a.polynomial.size().saturating_add(b.polynomial.size()))?;
            if a.high <= b.low {
                return Ok(Ordering::Less);
            }
            if b.high <= a.low {
                return Ok(Ordering::Greater);
            }
            if self.tarski(
                &a.polynomial,
                &b.polynomial,
                coordinate,
                &Bound::At(a.low.clone()),
                &Bound::At(a.high.clone()),
            )? == 0
            {
                if a.low >= b.low && a.high <= b.high {
                    return Ok(Ordering::Equal);
                }
                self.refine(&mut a, coordinate)?;
            } else {
                self.refine(&mut a, coordinate)?;
                self.refine(&mut b, coordinate)?;
            }
        }
    }

    fn samples(&mut self, set: &[Polynomial], coordinate: usize) -> Result<Vec<Coordinate>> {
        let mut roots: Vec<Root> = Vec::new();
        for p in set {
            for root in self.roots(p, coordinate)? {
                let mut index = 0;
                let mut duplicate = false;
                while let Some(other) = roots.get(index) {
                    match self.compare(&root, other, coordinate)? {
                        Ordering::Equal => {
                            duplicate = true;
                            break;
                        }
                        Ordering::Less => break,
                        Ordering::Greater => index += 1,
                    }
                }
                if !duplicate {
                    roots.insert(index, root);
                }
            }
        }
        if roots.is_empty() {
            return Ok(vec![Coordinate::Rational(zero())]);
        }
        let mut samples = vec![Coordinate::Rational(roots[0].low.sub(&one()))];
        for (i, root) in roots.iter().enumerate() {
            self.meter.charge()?;
            samples.push(Coordinate::Root(root.clone()));
            if let Some(next) = roots.get(i + 1) {
                let (mut a, mut b) = (root.clone(), next.clone());
                while a.high > b.low {
                    self.refine(&mut a, coordinate)?;
                    self.refine(&mut b, coordinate)?;
                }
                samples.push(Coordinate::Rational(midpoint(&a.high, &b.low)));
            } else {
                samples.push(Coordinate::Rational(root.high.add(&one())));
            }
        }
        Ok(samples)
    }

    pub(super) fn exists(
        &mut self,
        levels: &[Vec<Polynomial>],
        predicate: &mut impl FnMut(&mut Self) -> Result<bool>,
    ) -> Result<bool> {
        if self.prefix.len() == levels.len() {
            return predicate(self);
        }
        let coordinate = self.prefix.len() + 1;
        for sample in self.samples(&levels[coordinate - 1], coordinate)? {
            self.meter.charge()?;
            self.prefix.push(sample);
            let found = self.exists(levels, predicate)?;
            self.prefix.pop();
            if found {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Division-free determinant over the polynomial coefficient ring. Expansion
/// is expensive but complete; every minor and arithmetic operation is charged
/// before execution, so caller WorkUnits bound it without a degree cutoff.
fn determinant(rows: &[Vec<Polynomial>], meter: &mut HybridReplayMeter) -> Result<Polynomial> {
    meter.charge()?;
    let Some(first) = rows.first() else {
        return Ok(Polynomial::constant(one()));
    };
    let mut value = Polynomial::constant(zero());
    for (column, entry) in first.iter().enumerate() {
        if entry.0.is_empty() {
            continue;
        }
        meter.charge_by(entry.size())?;
        let minor = rows[1..]
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(i, p)| (i != column).then(|| p.clone()))
                    .collect()
            })
            .collect::<Vec<_>>();
        let term = entry.mul(&determinant(&minor, meter)?, meter)?;
        value = if column % 2 == 0 {
            value.add(&term, meter)?
        } else {
            value.sub(&term, meter)?
        };
    }
    Ok(value)
}

fn principal_subresultant(
    a: &Polynomial,
    b: &Polynomial,
    coordinate: usize,
    order: usize,
    meter: &mut HybridReplayMeter,
) -> Result<Polynomial> {
    let (m, n) = (
        a.degree(coordinate).unwrap_or(0),
        b.degree(coordinate).unwrap_or(0),
    );
    let size = m + n - 2 * order;
    meter.charge_by(
        a.size()
            .saturating_add(b.size())
            .saturating_mul(u64::try_from(size).unwrap_or(u64::MAX)),
    )?;
    let mut rows = Vec::new();
    for (shifts, p) in [(n - order, a), (m - order, b)] {
        for shift in (0..shifts).rev() {
            rows.push(
                (0..size)
                    .map(|column| {
                        (m + n - order - 1 - column).checked_sub(shift).map_or_else(
                            || Polynomial::constant(zero()),
                            |power| p.coefficient(coordinate, power),
                        )
                    })
                    .collect(),
            );
        }
    }
    determinant(&rows, meter)
}

fn keep(set: &mut Vec<Polynomial>, p: Polynomial, meter: &mut HybridReplayMeter) -> Result<()> {
    let p = p.primitive(meter)?;
    if p.0.keys().all(Vec::is_empty) {
        return Ok(());
    }
    let negative =
        p.0.values()
            .next()
            .is_some_and(|c| c.numerator().is_negative());
    let p = if negative { p.neg(meter)? } else { p };
    meter.charge_by(
        p.size()
            .saturating_mul(u64::try_from(set.len()).unwrap_or(u64::MAX).max(1)),
    )?;
    if !set.contains(&p) {
        set.push(p);
    }
    Ok(())
}

/// Full Collins projection: every coefficient, all positive-degree reducta,
/// all derivative PSCs and all PSCs of distinct reductum pairs. In particular
/// coefficient-only constraints survive and nullified fibers are supported.
pub(super) fn project(
    mut set: Vec<Polynomial>,
    dimensions: usize,
    meter: &mut HybridReplayMeter,
) -> Result<(Vec<Vec<Polynomial>>, Vec<Poly>)> {
    let mut levels = vec![Vec::new(); dimensions];
    for coordinate in (1..=dimensions).rev() {
        let mut unique = Vec::new();
        for p in set {
            keep(&mut unique, p, meter)?;
        }
        levels[coordinate - 1] = unique.clone();
        let mut projected = Vec::new();
        let mut reducta = Vec::new();
        for p in &unique {
            meter.charge_by(p.size())?;
            let mut reductum = p.clone();
            for degree in (0..=p.degree(coordinate).unwrap_or(0)).rev() {
                keep(&mut projected, p.coefficient(coordinate, degree), meter)?;
                if degree > 0 && reductum.degree(coordinate) == Some(degree) {
                    keep(&mut reducta, reductum.clone(), meter)?;
                }
                reductum
                    .0
                    .retain(|powers, _| powers.get(coordinate).copied().unwrap_or(0) != degree);
            }
        }
        for h in &reducta {
            let derivative = h.derivative(coordinate, meter)?;
            for order in 0..derivative.degree(coordinate).unwrap_or(0) {
                keep(
                    &mut projected,
                    principal_subresultant(h, &derivative, coordinate, order, meter)?,
                    meter,
                )?;
            }
        }
        for (i, h) in reducta.iter().enumerate() {
            for g in &reducta[i + 1..] {
                for order in 0..h
                    .degree(coordinate)
                    .unwrap_or(0)
                    .min(g.degree(coordinate).unwrap_or(0))
                {
                    keep(
                        &mut projected,
                        principal_subresultant(h, g, coordinate, order, meter)?,
                        meter,
                    )?;
                }
            }
        }
        set = projected;
    }
    Ok((levels, set.iter().map(Polynomial::univariate).collect()))
}
