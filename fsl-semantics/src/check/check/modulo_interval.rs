// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact type intervals for Euclidean integer modulus (FR-495).

use filament_exact::{Integer, IntegerInterval, NonZeroInteger, ValueType};

/// An impossible arithmetic or interval invariant failed while deriving a
/// type. The caller turns this into a located checker fault.
pub(crate) struct ModuloRangeInvariant;

/// The exact result type of `left mod right`, charging once for each closed
/// form or quotient run examined by `charge`.
pub(crate) fn result_type(
    left: &ValueType,
    right: &ValueType,
    mut charge: impl FnMut() -> Result<(), crate::check::CheckRefusal>,
) -> Result<ValueType, ModuloRangeError> {
    let left_bounds = bounded(left);
    let right_bounds = bounded(right);

    match (left_bounds, right_bounds) {
        (None, None) => {
            charge().map_err(ModuloRangeError::Refused)?;
            Ok(ValueType::Integer)
        }
        (None, Some((divisor_lower, divisor_upper))) => {
            let maximum = max_abs(divisor_lower, divisor_upper);
            if maximum.is_zero() {
                charge().map_err(ModuloRangeError::Refused)?;
                return Ok(ValueType::Integer);
            }
            charge().map_err(ModuloRangeError::Refused)?;
            interval_type(Integer::zero(), maximum.sub(&Integer::one()))
        }
        (Some((left_lower, left_upper)), Some((right_lower, right_upper))) => {
            let (minimum, maximum) = nonzero_magnitudes(right_lower, right_upper);
            if maximum.is_zero() {
                charge().map_err(ModuloRangeError::Refused)?;
                return Ok(ValueType::Integer);
            }
            let (lower, upper) =
                exact_interval(left_lower, left_upper, &minimum, &maximum, &mut charge)?;
            interval_type(lower, upper)
        }
        (Some((left_lower, left_upper)), None) => {
            if left_lower.is_negative() {
                charge().map_err(ModuloRangeError::Refused)?;
                return Ok(ValueType::Integer);
            }
            let maximum = left_upper.clone();
            if maximum.is_zero() {
                charge().map_err(ModuloRangeError::Refused)?;
                return interval_type(Integer::zero(), Integer::zero());
            }
            let (walk_lower, walk_upper) = exact_interval(
                left_lower,
                left_upper,
                &Integer::one(),
                &maximum,
                &mut charge,
            )?;
            let lower = if left_lower < &walk_lower {
                left_lower.clone()
            } else {
                walk_lower
            };
            let upper = if left_upper > &walk_upper {
                left_upper.clone()
            } else {
                walk_upper
            };
            interval_type(lower, upper)
        }
    }
}

/// Failure while finding a mathematically valid interval.
pub(crate) enum ModuloRangeError {
    /// The checker work ceiling denied a closed form or quotient run.
    Refused(crate::check::CheckRefusal),
    /// An impossible arithmetic or interval invariant failed.
    Invariant(ModuloRangeInvariant),
}

fn bounded(value_type: &ValueType) -> Option<(&Integer, &Integer)> {
    match value_type {
        ValueType::Int(interval) => Some((interval.lower(), interval.upper())),
        ValueType::Integer => None,
        _ => None,
    }
}

fn interval_type(lower: Integer, upper: Integer) -> Result<ValueType, ModuloRangeError> {
    IntegerInterval::new(lower, upper)
        .map(ValueType::Int)
        .map_err(|_| ModuloRangeError::Invariant(ModuloRangeInvariant))
}

fn max_abs(left: &Integer, right: &Integer) -> Integer {
    let left = left.abs();
    let right = right.abs();
    if left >= right {
        left
    } else {
        right
    }
}

/// Magnitudes of every nonzero member of one integer interval.
fn nonzero_magnitudes(lower: &Integer, upper: &Integer) -> (Integer, Integer) {
    let minimum = if lower <= &Integer::zero() && upper >= &Integer::zero() {
        Integer::one()
    } else {
        let lower_abs = lower.abs();
        let upper_abs = upper.abs();
        if lower_abs <= upper_abs {
            lower_abs
        } else {
            upper_abs
        }
    };
    (minimum, max_abs(lower, upper))
}

/// The exact Euclidean remainder interval for a finite dividend range and a
/// finite positive divisor-magnitude range.
fn exact_interval(
    lower: &Integer,
    upper: &Integer,
    divisor_lower: &Integer,
    divisor_upper: &Integer,
    charge: &mut impl FnMut() -> Result<(), crate::check::CheckRefusal>,
) -> Result<(Integer, Integer), ModuloRangeError> {
    let width = upper.sub(lower).add(&Integer::one());
    if &width >= divisor_upper {
        charge().map_err(ModuloRangeError::Refused)?;
        return Ok((Integer::zero(), divisor_upper.sub(&Integer::one())));
    }
    if !lower.is_negative() && upper < divisor_lower {
        charge().map_err(ModuloRangeError::Refused)?;
        return Ok((lower.clone(), upper.clone()));
    }

    let mut current = divisor_upper.clone();
    let mut result_lower: Option<Integer> = None;
    let mut result_upper: Option<Integer> = None;
    loop {
        let lower_quotient = floor_quotient(lower, &current)?;
        let upper_quotient = floor_quotient(upper, &current)?;
        let lower_run = quotient_run_lower(lower, &lower_quotient)?;
        let upper_run = quotient_run_lower(upper, &upper_quotient)?;
        let run_lower = max_integer(divisor_lower, &max_integer(&lower_run, &upper_run));

        charge().map_err(ModuloRangeError::Refused)?;
        let (run_result_lower, run_result_upper) = if lower_quotient == upper_quotient {
            let lower_at_start = lower.sub(&lower_quotient.mul(&run_lower));
            let lower_at_end = lower.sub(&lower_quotient.mul(&current));
            let upper_at_start = upper.sub(&upper_quotient.mul(&run_lower));
            let upper_at_end = upper.sub(&upper_quotient.mul(&current));
            (
                min_integer(&lower_at_start, &lower_at_end),
                max_integer(&upper_at_start, &upper_at_end),
            )
        } else {
            (Integer::zero(), current.sub(&Integer::one()))
        };
        include_minimum(&mut result_lower, run_result_lower);
        include_maximum(&mut result_upper, run_result_upper);

        if result_lower.as_ref() == Some(&Integer::zero())
            && result_upper.as_ref() == Some(&divisor_upper.sub(&Integer::one()))
        {
            break;
        }
        if run_lower == *divisor_lower {
            break;
        }
        current = run_lower.sub(&Integer::one());
    }

    match (result_lower, result_upper) {
        (Some(lower), Some(upper)) => Ok((lower, upper)),
        _ => Err(ModuloRangeError::Invariant(ModuloRangeInvariant)),
    }
}

fn floor_quotient(
    value: &Integer,
    positive_divisor: &Integer,
) -> Result<Integer, ModuloRangeError> {
    let divisor = NonZeroInteger::try_from(positive_divisor)
        .map_err(|_| ModuloRangeError::Invariant(ModuloRangeInvariant))?;
    Ok(value.div_mod_floor(divisor).0)
}

/// Smallest positive divisor magnitude for which `floor(value / d)` remains
/// equal to `quotient` as `d` descends from the current run's upper endpoint.
fn quotient_run_lower(value: &Integer, quotient: &Integer) -> Result<Integer, ModuloRangeError> {
    if quotient.is_zero() {
        return Ok(value.add(&Integer::one()));
    }
    if quotient.is_negative() {
        let divisor = NonZeroInteger::try_from(quotient)
            .map_err(|_| ModuloRangeError::Invariant(ModuloRangeInvariant))?;
        let (quotient, remainder) = value.div_rem_truncating(divisor);
        return Ok(if remainder.is_zero() {
            quotient
        } else {
            quotient.add(&Integer::one())
        });
    }
    let next_quotient = quotient.add(&Integer::one());
    let divisor = NonZeroInteger::try_from(&next_quotient)
        .map_err(|_| ModuloRangeError::Invariant(ModuloRangeInvariant))?;
    let quotient_at_boundary = value.div_mod_floor(divisor).0;
    Ok(quotient_at_boundary.add(&Integer::one()))
}

fn max_integer(left: &Integer, right: &Integer) -> Integer {
    if left >= right {
        left.clone()
    } else {
        right.clone()
    }
}

fn min_integer(left: &Integer, right: &Integer) -> Integer {
    if left <= right {
        left.clone()
    } else {
        right.clone()
    }
}

fn include_minimum(bound: &mut Option<Integer>, value: Integer) {
    match bound {
        Some(current) if value < *current => *current = value,
        Some(_) => {}
        None => *bound = Some(value),
    }
}

fn include_maximum(bound: &mut Option<Integer>, value: Integer) {
    match bound {
        Some(current) if value > *current => *current = value,
        Some(_) => {}
        None => *bound = Some(value),
    }
}
