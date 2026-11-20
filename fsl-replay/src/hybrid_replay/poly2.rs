// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bivariate polynomials over the exact rationals and the projection of a
//! set of them (FR-450 "Exact replay": quantifier elimination over the
//! post-jump variables a reset leaves free). A `Poly2` is a polynomial in an
//! inner variable `x` whose coefficients are polynomials in an outer
//! parameter `t`. The projection onto `t` of a set of them is the set of
//! `t`-polynomials whose roots bound every cell over which the real roots of
//! the set, their number and their order, are constant (Collins: every
//! coefficient, and every principal subresultant coefficient of each
//! polynomial with its derivative and of each pair).

use filament_exact::Rational;

use super::poly::Poly;
use super::trace::zero;
use super::HybridReplayRefusal;

fn one() -> Rational {
    Rational::from_integer(1_i64.into())
}

/// A polynomial in `x` with coefficients in `t`, lowest degree first, with
/// no trailing zero coefficient.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Poly2(Vec<Poly>);

impl Poly2 {
    pub(crate) fn new(mut coefficients: Vec<Poly>) -> Self {
        while coefficients.last().is_some_and(Poly::is_zero) {
            coefficients.pop();
        }
        Self(coefficients)
    }

    pub(crate) fn zero() -> Self {
        Self(Vec::new())
    }

    /// The polynomial `t`-poly `value`, constant in `x`.
    pub(crate) fn constant(value: Poly) -> Self {
        Self::new(vec![value])
    }

    /// The inner variable `x`.
    pub(crate) fn variable() -> Self {
        Self(vec![Poly::zero(), Poly::constant(one())])
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// The degree in `x`; `None` for zero.
    pub(crate) fn degree(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }

    /// The words of the largest coefficient part: one per 64 bits.
    fn words(&self) -> u64 {
        self.0.iter().map(Poly::words).max().unwrap_or(1)
    }

    /// The work of one arithmetic step on this polynomial.
    pub(crate) fn size(&self) -> u64 {
        self.0.iter().map(Poly::size).fold(0, u64::saturating_add)
    }

    pub(crate) fn coefficients(&self) -> &[Poly] {
        &self.0
    }

    fn coefficient(&self, degree: usize) -> Poly {
        self.0.get(degree).cloned().unwrap_or_else(Poly::zero)
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        let len = self.0.len().max(other.0.len());
        Self::new(
            (0..len)
                .map(|i| self.coefficient(i).add(&other.coefficient(i)))
                .collect(),
        )
    }

    pub(crate) fn neg(&self) -> Self {
        Self(self.0.iter().map(Poly::neg).collect())
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut product = vec![Poly::zero(); self.0.len() + other.0.len() - 1];
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in other.0.iter().enumerate() {
                product[i + j] = product[i + j].add(&a.mul(b));
            }
        }
        Self::new(product)
    }

    /// Every coefficient multiplied by the `t`-polynomial `factor`.
    pub(crate) fn scale(&self, factor: &Poly) -> Self {
        Self::new(self.0.iter().map(|c| c.mul(factor)).collect())
    }

    /// `self` at `x`, a polynomial in `t`.
    pub(crate) fn at_x(&self, x: &Rational) -> Poly {
        self.0
            .iter()
            .rev()
            .fold(Poly::zero(), |value, coefficient| {
                value.scale(x).add(coefficient)
            })
    }

    /// `self` at `t`, a polynomial in `x`.
    pub(crate) fn at_t(&self, t: &Rational) -> Poly {
        Poly::new(self.0.iter().map(|c| c.eval(t)).collect())
    }

    /// `self` with `x + shift` for `x`: the polynomial whose roots in `x`
    /// are `self`'s roots moved down by `shift`.
    pub(crate) fn shifted(&self, shift: &Rational) -> Self {
        let step = Self::new(vec![Poly::constant(shift.clone()), Poly::constant(one())]);
        self.0.iter().rev().fold(Self::zero(), |acc, coefficient| {
            acc.mul(&step).add(&Self::constant(coefficient.clone()))
        })
    }

    /// The derivative in `x`.
    pub(crate) fn derivative(&self) -> Self {
        Self::new(
            self.0
                .iter()
                .enumerate()
                .skip(1)
                .map(|(degree, c)| c.scale(&Rational::from_integer(degree.into())))
                .collect(),
        )
    }

    /// The derivative in `t`.
    pub(crate) fn derivative_t(&self) -> Self {
        Self::new(self.0.iter().map(Poly::derivative).collect())
    }

    /// `self` with the factor in `t` every coefficient shares removed.
    pub(crate) fn primitive_t(&self) -> Self {
        let content = self.0.iter().fold(Poly::zero(), |common, c| common.gcd(c));
        if content.degree().is_none_or(|degree| degree == 0) {
            return self.clone();
        }
        Self::new(self.0.iter().map(|c| c.exact_div(&content)).collect())
    }

    /// `self` with `x` and `t` exchanged.
    fn transposed(&self) -> Self {
        let width = self.parameter_degree() + 1;
        Self::new(
            (0..width)
                .map(|power| {
                    Poly::new(
                        self.0
                            .iter()
                            .map(|c| c.coefficients().get(power).cloned().unwrap_or_else(zero))
                            .collect(),
                    )
                })
                .collect(),
        )
    }

    /// `self` with the factor in `x` alone that every power of `t` shares
    /// removed: the curves `x = c` that do not move with `t`.
    pub(crate) fn without_vertical_factors(&self) -> Self {
        let across = self.transposed();
        let common = across
            .0
            .iter()
            .fold(Poly::zero(), |common, c| common.gcd(c));
        if common.degree().is_none_or(|degree| degree == 0) {
            return self.clone();
        }
        Self::new(across.0.iter().map(|c| c.exact_div(&common)).collect()).transposed()
    }

    /// The pseudo-remainder of `self` by a nonzero `divisor` in `x`, with the
    /// factor in `t` its steps share removed.
    fn pseudo_remainder(&self, divisor: &Self) -> Self {
        let (Some(degree), Some(lead)) = (divisor.degree(), divisor.0.last()) else {
            return self.clone();
        };
        let mut remainder = self.clone();
        while let Some(top) = remainder.degree().filter(|top| *top >= degree) {
            let factor = remainder.0[top].clone();
            let mut shifted = vec![Poly::zero(); top - degree];
            shifted.extend(divisor.0.iter().cloned());
            let subtrahend = Self::new(shifted).scale(&factor);
            remainder = remainder.scale(lead).sub(&subtrahend).primitive_t();
        }
        remainder
    }

    /// The greatest common divisor in `x` over the rational functions in `t`,
    /// primitive.
    fn gcd_x(&self, other: &Self) -> Self {
        let (mut a, mut b) = (self.primitive_t(), other.primitive_t());
        while !b.is_zero() {
            let remainder = a.pseudo_remainder(&b);
            a = b;
            b = remainder;
        }
        a.primitive_t()
    }

    /// The quotient by a `divisor` that divides `self` exactly in `x` (so, by
    /// Gauss's lemma, with coefficients that are polynomials in `t`).
    fn exact_div_x(&self, divisor: &Self) -> Self {
        let (Some(degree), Some(lead)) = (divisor.degree(), divisor.0.last()) else {
            return self.clone();
        };
        let mut remainder = self.clone();
        let mut quotient = vec![Poly::zero(); self.0.len().saturating_sub(degree)];
        while let Some(top) = remainder.degree().filter(|top| *top >= degree) {
            let factor = remainder.0[top].exact_div(lead);
            quotient[top - degree] = factor.clone();
            let mut shifted = vec![Poly::zero(); top - degree];
            shifted.extend(divisor.0.iter().cloned());
            let before = remainder.degree();
            remainder = remainder.sub(&Self::new(shifted).scale(&factor));
            if remainder.degree() >= before {
                // Not an exact quotient: the leading term did not cancel.
                break;
            }
        }
        Self::new(quotient)
    }

    /// `self` with every repeated factor in `x` taken once: the polynomial
    /// whose real roots in `x` are `self`'s, each simple.
    pub(crate) fn squarefree_x(&self) -> Self {
        let primitive = self.primitive_t();
        let shared = primitive.gcd_x(&primitive.derivative());
        if shared.degree().is_none_or(|degree| degree == 0) {
            return primitive;
        }
        primitive.exact_div_x(&shared).primitive_t()
    }

    /// The largest degree in `t` of a coefficient.
    fn parameter_degree(&self) -> usize {
        self.0.iter().filter_map(Poly::degree).max().unwrap_or(0)
    }
}

/// A quotient of bivariate polynomials, the denominator nonzero. It is kept
/// unreduced: a value read at a point where an earlier division was by zero
/// is never reached, so the denominator is nonzero wherever the value is
/// read, and its sign is the sign of `numerator * denominator` there.
#[derive(Clone, Debug)]
pub(crate) struct RatFunc2 {
    pub(crate) numerator: Poly2,
    pub(crate) denominator: Poly2,
}

impl RatFunc2 {
    pub(crate) fn of(numerator: Poly2) -> Self {
        Self {
            numerator,
            denominator: Poly2::constant(Poly::constant(one())),
        }
    }

    pub(crate) fn constant(value: Rational) -> Self {
        Self::of(Poly2::constant(Poly::constant(value)))
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self {
            numerator: self
                .numerator
                .mul(&other.denominator)
                .add(&other.numerator.mul(&self.denominator)),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    pub(crate) fn neg(&self) -> Self {
        Self {
            numerator: self.numerator.neg(),
            denominator: self.denominator.clone(),
        }
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self {
            numerator: self.numerator.mul(&other.numerator),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    pub(crate) fn div(&self, other: &Self) -> Self {
        Self {
            numerator: self.numerator.mul(&other.denominator),
            denominator: self.denominator.mul(&other.numerator),
        }
    }

    pub(crate) fn sign_poly(&self) -> Poly2 {
        self.numerator.mul(&self.denominator)
    }
}

/// The work of the determinant of a `size` square matrix whose entries have
/// parts of `words` words: `size^3` entry operations on parts that grow to
/// `size * words` words, and the same again to interpolate `points` values.
fn determinant_work(size: usize, words: u64, points: usize) -> u64 {
    let size = u64::try_from(size).unwrap_or(u64::MAX);
    let points = u64::try_from(points).unwrap_or(u64::MAX);
    let grown = size.saturating_mul(words);
    size.saturating_mul(size)
        .saturating_mul(size)
        .saturating_mul(grown)
        .saturating_add(points.saturating_mul(grown))
}

/// The determinant of a square rational matrix, by Gaussian elimination.
fn determinant(mut rows: Vec<Vec<Rational>>) -> Rational {
    let size = rows.len();
    let mut value = one();
    for column in 0..size {
        let Some(pivot) = (column..size).find(|&r| !rows[r][column].is_zero()) else {
            return zero();
        };
        if pivot != column {
            rows.swap(pivot, column);
            value = value.neg();
        }
        let lead = rows[column][column].clone();
        value = value.mul(&lead);
        for r in column + 1..size {
            let Some(factor) = rows[r][column].div(&lead) else {
                unreachable!("a pivot is nonzero")
            };
            if factor.is_zero() {
                continue;
            }
            let pivot_row = rows[column].clone();
            for (cell, above) in rows[r].iter_mut().zip(&pivot_row).skip(column) {
                *cell = cell.sub(&factor.mul(above));
            }
        }
    }
    value
}

/// The polynomial of degree below `values.len()` taking `values[i]` at the
/// integer `i`.
fn interpolate(values: &[Rational]) -> Poly {
    let mut total = Poly::zero();
    for (i, value) in values.iter().enumerate() {
        if value.is_zero() {
            continue;
        }
        let mut term = Poly::constant(value.clone());
        for j in 0..values.len() {
            if i == j {
                continue;
            }
            let at = |k: usize| Rational::from_integer(k.into());
            let Some(inverse) = one().div(&at(i).sub(&at(j))) else {
                unreachable!("the points are distinct")
            };
            term = term
                .mul(&Poly::new(vec![at(j).neg(), one()]))
                .scale(&inverse);
        }
        total = total.add(&term);
    }
    total
}

/// The principal subresultant coefficient of order `order` of `a` and `b`
/// (order 0 is the resultant) as a polynomial in `t`, by evaluation at
/// enough integers and interpolation. `order` is below both degrees.
fn principal_subresultant(
    a: &Poly2,
    b: &Poly2,
    order: usize,
    charge: &mut dyn FnMut(u64) -> Result<(), HybridReplayRefusal>,
) -> Result<Poly, HybridReplayRefusal> {
    let (Some(m), Some(n)) = (a.degree(), b.degree()) else {
        return Ok(Poly::zero());
    };
    let size = m + n - 2 * order;
    let bound = (n - order) * a.parameter_degree() + (m - order) * b.parameter_degree();
    let mut values: Vec<Rational> = Vec::with_capacity(bound + 1);
    let words = a.words().max(b.words());
    for point in 0..=bound {
        charge(determinant_work(size, words, bound + 1))?;
        let t = Rational::from_integer(point.into());
        let (at_a, at_b) = (a.at_t(&t), b.at_t(&t));
        let mut rows = Vec::with_capacity(size);
        for (shifts, polynomial) in [(n - order, &at_a), (m - order, &at_b)] {
            for shift in (0..shifts).rev() {
                rows.push(
                    (0..size)
                        .map(|column| {
                            // Column `c` is the power `m + n - order - 1 - c`.
                            let power = m + n - order - 1 - column;
                            power
                                .checked_sub(shift)
                                .and_then(|degree| polynomial.coefficients().get(degree))
                                .cloned()
                                .unwrap_or_else(zero)
                        })
                        .collect(),
                );
            }
        }
        values.push(determinant(rows));
    }
    Ok(interpolate(&values))
}

/// The resultant in `t` of the polynomial `f` in `t` and `p`, as a
/// polynomial in `x`: it vanishes at every `x` at which `p(t, x)` and `f`
/// have a common root `t`, so its roots hold the roots in `x` of `p(u, .)`
/// for every root `u` of `f`. The degree in `t` of `p` is taken formally,
/// so the result may differ from the resultant by a nonzero constant.
pub(crate) fn eliminate(
    f: &Poly,
    p: &Poly2,
    charge: &mut dyn FnMut(u64) -> Result<(), HybridReplayRefusal>,
) -> Result<Poly, HybridReplayRefusal> {
    let (Some(n), Some(dx)) = (f.degree(), p.degree()) else {
        return Ok(Poly::zero());
    };
    let m = p.parameter_degree();
    if m == 0 {
        return Ok(p.at_t(&zero()));
    }
    if n == 0 {
        return Ok(Poly::zero());
    }
    let size = m + n;
    let mut values: Vec<Rational> = Vec::with_capacity(n * dx + 1);
    let words = f.words().max(p.words());
    for point in 0..=n * dx {
        charge(determinant_work(size, words, n * dx + 1))?;
        let at = p.at_x(&Rational::from_integer(point.into()));
        let mut rows = Vec::with_capacity(size);
        for (shifts, polynomial) in [(m, f), (n, &at)] {
            for shift in (0..shifts).rev() {
                rows.push(
                    (0..size)
                        .map(|column| {
                            let power = size - 1 - column;
                            power
                                .checked_sub(shift)
                                .and_then(|degree| polynomial.coefficients().get(degree))
                                .cloned()
                                .unwrap_or_else(zero)
                        })
                        .collect(),
                );
            }
        }
        values.push(determinant(rows));
    }
    Ok(interpolate(&values))
}

/// The values of `t` at which a root curve of `p` has a horizontal tangent
/// or `p` has a vertical one: the resultant in `x` of `p` and its derivative
/// in `t` (zero when they share a factor). Between two of its roots every
/// simple root of `p` is a monotone function of `t`. `p` is squarefree in
/// `x` and free of vertical factors.
pub(crate) fn stationary(
    p: &Poly2,
    charge: &mut dyn FnMut(u64) -> Result<(), HybridReplayRefusal>,
) -> Result<Poly, HybridReplayRefusal> {
    let along = p.derivative_t();
    match (p.degree(), along.degree()) {
        (Some(m), Some(n)) if m > 0 && n > 0 => principal_subresultant(p, &along, 0, charge),
        (Some(m), Some(0)) => {
            let value = along.0[0].clone();
            let mut power = Poly::constant(one());
            for _ in 0..m {
                power = power.mul(&value);
            }
            Ok(power)
        }
        _ => Ok(Poly::zero()),
    }
}

/// The nonconstant polynomials in `t` whose roots bound every cell over
/// which the real roots in `x` of every polynomial of `set`, their number
/// and their order, are constant.
pub(crate) fn projection(
    set: &[Poly2],
    charge: &mut dyn FnMut(u64) -> Result<(), HybridReplayRefusal>,
) -> Result<Vec<Poly>, HybridReplayRefusal> {
    let mut projected: Vec<Poly> = Vec::new();
    let mut keep = |poly: Poly| {
        if poly.degree().unwrap_or(0) > 0 && !projected.contains(&poly) {
            projected.push(poly);
        }
    };
    for f in set {
        for coefficient in f.coefficients() {
            keep(coefficient.clone());
        }
        let derivative = f.derivative();
        if let (Some(m), Some(_)) = (f.degree(), derivative.degree()) {
            for order in 0..m.saturating_sub(1) {
                keep(principal_subresultant(f, &derivative, order, charge)?);
            }
        }
    }
    for (i, f) in set.iter().enumerate() {
        for g in &set[i + 1..] {
            if let (Some(m), Some(n)) = (f.degree(), g.degree()) {
                for order in 0..m.min(n) {
                    keep(principal_subresultant(f, g, order, charge)?);
                }
            }
        }
    }
    Ok(projected)
}
