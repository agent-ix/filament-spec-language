// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-408 "Exact rational bounds": the one logarithm bound the sampler
//! and EN-4 compute with, and the exponential bound a delay law conditioned
//! on a window needs.

use filament_exact::{Integer, NonZeroInteger, Rational};
use filament_semantic_value::catalog::InternalFault;
use ix_trace_rs::implements;

/// The bit precision of every logarithm series tail bound: a series stops
/// once its tail is below `2^-70` (FSpec FR-408).
const TAIL_BITS: u64 = 70;
/// The absolute precision of an exponential bound, below the finest grid
/// value (`2^-256`) a draw takes.
const EXP_BITS: u64 = 512;

const DIVISOR: InternalFault = InternalFault::new("statistical-bounds", "nonzero-divisor");
const LOG_ARGUMENT: InternalFault =
    InternalFault::new("statistical-bounds", "positive-log-argument");
const EXP_ARGUMENT: InternalFault =
    InternalFault::new("statistical-bounds", "non-negative-exponent");

/// A lower and an upper bound on one real number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounds {
    /// At most the true value.
    pub lower: Rational,
    /// At least the true value.
    pub upper: Rational,
}

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn quotient(numerator: &Rational, denominator: &Rational) -> Result<Rational, InternalFault> {
    numerator.div(denominator).ok_or(DIVISOR)
}

/// `2^-bits`.
fn epsilon(bits: u64) -> Rational {
    int(1).divided_by_power_of_two(bits)
}

/// `value · 2^shift`.
fn times_power_of_two(value: &Rational, shift: u64) -> Rational {
    value.mul(&Rational::from_integer(Integer::one().shifted_left(shift)))
}

/// `atanh(u) = Σ u^(2n+1) / (2n + 1)` for `0 <= u < 1`, summed until the
/// geometric tail bound `u^(2n+3) / ((2n + 3)(1 − u²))` is below
/// `2^-bits`: the partial sum is the lower bound and the partial sum plus
/// the tail bound the upper.
fn atanh(u: &Rational, bits: u64) -> Result<Bounds, InternalFault> {
    let square = u.mul(u);
    let complement = int(1).sub(&square);
    let limit = epsilon(bits);
    let mut power = u.clone();
    let mut sum = int(0);
    let mut index: i64 = 0;
    loop {
        sum = sum.add(&quotient(&power, &int(2 * index + 1))?);
        let next = power.mul(&square);
        let tail = quotient(&next, &int(2 * index + 3).mul(&complement))?;
        if tail < limit {
            return Ok(Bounds {
                upper: sum.add(&tail),
                lower: sum,
            });
        }
        power = next;
        index += 1;
    }
}

/// `ln 2 = 2 · atanh(1/3)`, its tail below `2^-bits`.
fn ln_two(bits: u64) -> Result<Bounds, InternalFault> {
    let third = quotient(&int(1), &int(3))?;
    let half = atanh(&third, bits)?;
    Ok(Bounds {
        lower: half.lower.mul(&int(2)),
        upper: half.upper.mul(&int(2)),
    })
}

/// Bounds on `ln x` for a rational `x > 0`: `x = 2^k · y` with
/// `1 <= y < 2`, and `ln x = k · ln 2 + 2 · atanh((y − 1) / (y + 1))`.
/// `ln 2`'s tail is taken below `2^-(71 + bits(|k|))`, so `k · ln 2` adds at
/// most `2^-70` to the width whatever `k` is, and the bound stays within
/// `2^-64`.
///
/// # Errors
///
/// [`InternalFault`] for `x <= 0`.
pub fn ln(x: &Rational) -> Result<Bounds, InternalFault> {
    if *x <= int(0) {
        return Err(LOG_ARGUMENT);
    }
    // `x / 2^k` for the signed `k`: a bit-length difference, so its
    // magnitude fits in `u64`.
    let scale = |exponent: i128| -> Result<Rational, InternalFault> {
        let shift = u64::try_from(exponent.unsigned_abs()).map_err(|_| LOG_ARGUMENT)?;
        Ok(if exponent >= 0 {
            x.divided_by_power_of_two(shift)
        } else {
            times_power_of_two(x, shift)
        })
    };
    let numerator_bits = i128::from(x.numerator().as_big().bits());
    let denominator_bits = i128::from(x.denominator().as_big().bits());
    let mut exponent = numerator_bits - denominator_bits;
    let mut mantissa = scale(exponent)?;
    if mantissa < int(1) {
        exponent -= 1;
        mantissa = scale(exponent)?;
    }
    let u = quotient(&mantissa.sub(&int(1)), &mantissa.add(&int(1)))?;
    let series = atanh(&u, TAIL_BITS)?;
    let exponent_bits = u64::from(128 - exponent.unsigned_abs().leading_zeros());
    let two = ln_two(TAIL_BITS + 1 + exponent_bits)?;
    let k = Rational::from_integer(Integer::from(exponent));
    let (k_lower, k_upper) = if exponent >= 0 {
        (k.mul(&two.lower), k.mul(&two.upper))
    } else {
        (k.mul(&two.upper), k.mul(&two.lower))
    };
    Ok(Bounds {
        lower: k_lower.add(&series.lower.mul(&int(2))),
        upper: k_upper.add(&series.upper.mul(&int(2))),
    })
}

/// `value` rounded down, or up, to a multiple of `2^-bits`.
fn dyadic(value: &Rational, bits: u64, up: bool) -> Result<Rational, InternalFault> {
    let divisor = NonZeroInteger::try_from(value.denominator()).map_err(|_| DIVISOR)?;
    let (mut floor, remainder) = value.numerator().shifted_left(bits).div_mod_floor(divisor);
    if up && !remainder.is_zero() {
        floor = floor.add(&Integer::one());
    }
    Ok(Rational::from_integer(floor).divided_by_power_of_two(bits))
}

/// A positive `value` rounded down to at least `bits` significant bits.
pub(crate) fn significant_floor(value: &Rational, bits: u64) -> Result<Rational, InternalFault> {
    let numerator = value.numerator().as_big().bits();
    let denominator = value.denominator().as_big().bits();
    dyadic(
        value,
        bits.saturating_add(denominator.saturating_sub(numerator)),
        false,
    )
}

/// Bounds on `e^-x` for a rational `x >= 0`, within `2^-480`: the
/// alternating series of `e^-y` at `y = x / 2^n <= 1/2`, squared `n` times
/// with each end rounded outward. At `x >= 512`, `e^-x < 2^-512`.
#[implements("FSpec-FR-420-AC-2")]
pub(crate) fn exp_neg(x: &Rational) -> Result<Bounds, InternalFault> {
    let zero = int(0);
    if *x < zero {
        return Err(EXP_ARGUMENT);
    }
    if x.is_zero() {
        return Ok(Bounds {
            lower: int(1),
            upper: int(1),
        });
    }
    if *x >= int(512) {
        return Ok(Bounds {
            lower: zero,
            upper: epsilon(EXP_BITS),
        });
    }
    let half = epsilon(1);
    let mut y = x.clone();
    let mut halvings = 0_u32;
    while y > half {
        y = y.divided_by_power_of_two(1);
        halvings += 1;
    }
    // The terms alternate and fall, so the first omitted one bounds the
    // error of the partial sum.
    let limit = epsilon(EXP_BITS + 16);
    let mut term = int(1);
    let mut sum = int(1);
    let mut index: i64 = 1;
    let error = loop {
        term = quotient(&term.mul(&y), &int(index))?;
        if term < limit {
            break term;
        }
        sum = if index % 2 == 0 {
            sum.add(&term)
        } else {
            sum.sub(&term)
        };
        index += 1;
    };
    let precision = EXP_BITS + 16;
    let mut lower = dyadic(&sum.sub(&error), precision, false)?;
    let mut upper = dyadic(&sum.add(&error), precision, true)?;
    for _ in 0..halvings {
        lower = dyadic(&lower.mul(&lower), precision, false)?;
        upper = dyadic(&upper.mul(&upper), precision, true)?;
    }
    Ok(Bounds { lower, upper })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// `numerator / 10^digits` from a decimal digit string.
    fn decimal(digits: &str, scale: u64) -> Rational {
        let value: Integer = digits.parse().expect("decimal digits");
        Rational::from_integer(value).divided_by_power_of_ten(scale)
    }

    fn within(bounds: &Bounds, truth_lower: &Rational, truth_upper: &Rational) {
        assert!(bounds.lower <= *truth_lower, "lower bound above the value");
        assert!(bounds.upper >= *truth_upper, "upper bound below the value");
        assert!(
            bounds.upper.sub(&bounds.lower) < epsilon(64),
            "bounds wider than 2^-64"
        );
    }

    #[test]
    fn ln_100_brackets_its_true_value_within_two_to_the_minus_64() {
        // ln 100 = 4.60517018598809136803598290936872841520220297725754…
        let lower = decimal("460517018598809136803598290936872841520", 38);
        let upper = decimal("460517018598809136803598290936872841521", 38);
        within(&ln(&int(100)).expect("ln 100"), &lower, &upper);
    }

    #[test]
    fn ln_of_a_fraction_below_one_is_negative_and_bracketed() {
        // ln(3/4) = -0.28768207245178092743921900599382743150350971089776…
        let x = quotient(&int(3), &int(4)).expect("3/4");
        let lower = decimal("-287682072451780927439219005993827431504", 39);
        let upper = decimal("-287682072451780927439219005993827431503", 39);
        within(&ln(&x).expect("ln 3/4"), &lower, &upper);
    }

    #[test]
    fn ln_of_zero_is_an_internal_fault() {
        assert_eq!(ln(&int(0)), Err(LOG_ARGUMENT));
    }

    /// `ln(2^-40) = −40 ln 2`, bracketed within `2^-64` although the binary
    /// exponent is large.
    #[test]
    fn ln_of_a_large_binary_exponent_stays_within_two_to_the_minus_64() {
        // 40 ln 2 = 27.7258872223978123766892848583270627230200053744…
        let lower = decimal("-277258872223978123766892848583270627231", 37);
        let upper = decimal("-277258872223978123766892848583270627230", 37);
        let x = int(1).divided_by_power_of_two(40);
        within(&ln(&x).expect("ln 2^-40"), &lower, &upper);
        let big = Rational::from_integer(Integer::one().shifted_left(200));
        let width = ln(&big).expect("ln 2^200");
        assert!(width.upper.sub(&width.lower) < epsilon(64));
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn exp_neg_brackets_its_true_value() {
        // e^-1 = 0.36787944117144232159552377016146086744581113103176…
        let lower = decimal("367879441171442321595523770161460867445", 39);
        let upper = decimal("367879441171442321595523770161460867446", 39);
        within(&exp_neg(&int(1)).expect("e^-1"), &lower, &upper);
        // e^-10 = 0.0000453999297624848515355915155605506102379180888…
        let lower = decimal("453999297624848515355915155605506102379", 43);
        let upper = decimal("453999297624848515355915155605506102380", 43);
        within(&exp_neg(&int(10)).expect("e^-10"), &lower, &upper);
    }
}
