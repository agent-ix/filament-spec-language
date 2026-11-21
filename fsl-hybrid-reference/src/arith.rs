// SPDX-License-Identifier: AGPL-3.0-or-later
//! Outward rounding of enclosure bounds to multiples of `2^-bits` (FR-1019
//! "Arithmetic") and the exact grid arithmetic the refutation search uses.

use filament_exact::{Integer, NonZeroInteger, Rational};
use fsl_replay::{HybridBox, HybridInterval};

fn power(bits: u32) -> Integer {
    Integer::one().shifted_left(u64::from(bits))
}

/// The greatest multiple of `2^-bits` not above `value`.
pub fn floor_to(value: &Rational, bits: u32) -> Rational {
    let scale = power(bits);
    let scaled = value.mul(&Rational::from_integer(scale.clone()));
    let divisor = NonZeroInteger::try_from(scaled.denominator()).expect("a denominator is nonzero");
    let (floor, _) = scaled.numerator().div_mod_floor(divisor);
    Rational::new(floor, scale).expect("a power of two is nonzero")
}

/// The least multiple of `2^-bits` not below `value`.
pub fn ceil_to(value: &Rational, bits: u32) -> Rational {
    let scale = power(bits);
    let scaled = value.mul(&Rational::from_integer(scale.clone()));
    let divisor = NonZeroInteger::try_from(scaled.denominator()).expect("a denominator is nonzero");
    let (floor, rest) = scaled.numerator().div_mod_floor(divisor);
    let up = if rest.is_zero() {
        floor
    } else {
        floor.add(&Integer::one())
    };
    Rational::new(up, scale).expect("a power of two is nonzero")
}

/// `interval` with each bounded end rounded outward.
pub fn round_interval(interval: &HybridInterval, bits: u32) -> HybridInterval {
    HybridInterval {
        lower: interval.lower.as_ref().map(|q| floor_to(q, bits)),
        upper: interval.upper.as_ref().map(|q| ceil_to(q, bits)),
    }
}

/// `bx` rounded outward.
pub fn box_round(bx: &HybridBox, bits: u32) -> HybridBox {
    bx.iter().map(|x| round_interval(x, bits)).collect()
}

fn eighth(width: &Rational) -> Rational {
    width.mul(&Rational::new(Integer::one(), Integer::from(8_i64)).expect("8 is nonzero"))
}

/// `bx` widened by one eighth of each width plus `2^-bits`, rounded outward
/// (FR-1019 "Dwells").
pub fn widen(bx: &HybridBox, bits: u32) -> HybridBox {
    let epsilon = Rational::new(Integer::one(), power(bits)).expect("a power of two is nonzero");
    bx.iter()
        .map(|x| match (&x.lower, &x.upper) {
            (Some(lower), Some(upper)) => {
                let margin = eighth(&upper.sub(lower)).add(&epsilon);
                HybridInterval {
                    lower: Some(floor_to(&lower.sub(&margin), bits)),
                    upper: Some(ceil_to(&upper.add(&margin), bits)),
                }
            }
            _ => x.clone(),
        })
        .collect()
}

/// Whether every end of `bx` is bounded.
pub fn bounded(bx: &HybridBox) -> bool {
    bx.iter().all(|x| x.lower.is_some() && x.upper.is_some())
}

/// The midpoint of a bounded box, per variable.
pub fn midpoint(bx: &HybridBox) -> Option<Vec<Rational>> {
    let half = Rational::new(Integer::one(), Integer::from(2_i64)).ok()?;
    bx.iter()
        .map(|x| Some(x.lower.as_ref()?.add(x.upper.as_ref()?).mul(&half)))
        .collect()
}

/// A bounded box's lower or upper corner.
pub fn corner(bx: &HybridBox, upper: bool) -> Option<Vec<Rational>> {
    bx.iter()
        .map(|x| {
            if upper {
                x.upper.clone()
            } else {
                x.lower.clone()
            }
        })
        .collect()
}

/// The least integer `n` with `n * 2^-bits >= value`, as a non-negative
/// integer (0 for a negative value).
pub fn grid_count(value: &Rational, bits: u32) -> Integer {
    let up = ceil_to(value, bits).mul(&Rational::from_integer(power(bits)));
    if up.numerator().is_negative() {
        Integer::zero()
    } else {
        up.numerator().clone()
    }
}

/// `n * 2^-bits`.
pub fn grid_point(n: &Integer, bits: u32) -> Rational {
    Rational::new(n.clone(), power(bits)).expect("a power of two is nonzero")
}

/// `floor((a + b) / 2)`.
pub fn mid(a: &Integer, b: &Integer) -> Integer {
    let two = Integer::from(2_i64);
    let divisor = NonZeroInteger::try_from(&two).expect("2 is nonzero");
    a.add(b).div_mod_floor(divisor).0
}
