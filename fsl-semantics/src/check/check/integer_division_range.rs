// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495: exact quotient corners and metered remainder quotient runs.

use filament_checked::node::IntegerDivisionOperator;
use filament_exact::{DivisionProfile, Integer, NonZeroInteger};
use filament_semantic_value::location::ExpressionLocation;

use super::{refuse, CheckCause, CheckRefusal};
use crate::check::ir::IntegerRange;

type Charge<'a> = &'a mut dyn FnMut() -> Result<(), CheckRefusal>;

fn quotient(
    a: &Integer,
    b: &Integer,
    law: DivisionProfile,
    at: &ExpressionLocation,
) -> Result<Integer, CheckRefusal> {
    let divisor =
        NonZeroInteger::try_from(b).map_err(|_| refuse(at, CheckCause::UnrepresentableBound))?;
    let (q, r) = match law {
        DivisionProfile::Truncating => a.div_rem_truncating(divisor),
        DivisionProfile::Floor | DivisionProfile::Euclidean => a.div_mod_floor(divisor),
    };
    Ok(if law == DivisionProfile::Euclidean && r.is_negative() {
        q.add(&Integer::one())
    } else {
        q
    })
}

fn hull(left: IntegerRange, right: IntegerRange) -> IntegerRange {
    (left.0.min(right.0), left.1.max(right.1))
}

fn negated((lo, hi): IntegerRange) -> IntegerRange {
    (hi.neg(), lo.neg())
}

fn euclidean(
    (a, b): &IntegerRange,
    (lower, upper): &IntegerRange,
    charge: Charge<'_>,
    at: &ExpressionLocation,
) -> Result<IntegerRange, CheckRefusal> {
    super::modulo_interval::exact_interval(a, b, lower, upper, charge).map_err(
        |error| match error {
            super::modulo_interval::ModuloRangeError::Refused(refusal) => refusal,
            super::modulo_interval::ModuloRangeError::Invariant(_) => refuse(
                at,
                CheckCause::InternalFault(Box::new(crate::check::refusal::KeyFault::StageFault(
                    filament_semantic_value::catalog::InternalFault::new(
                        "check.integer-division",
                        "range-invariant",
                    ),
                ))),
            ),
        },
    )
}

/// Exact finite `div`/`rem` result interval, or no finite interval.
pub(super) fn infer(
    operator: IntegerDivisionOperator,
    law: DivisionProfile,
    dividend: Option<IntegerRange>,
    divisor: Option<IntegerRange>,
    charge: Charge<'_>,
    at: &ExpressionLocation,
) -> Result<Option<IntegerRange>, CheckRefusal> {
    let zero = Integer::zero();
    let one = Integer::one();
    if dividend
        .as_ref()
        .is_some_and(|(a, b)| a.is_zero() && b.is_zero())
        && operator == IntegerDivisionOperator::Rem
    {
        charge()?;
        return Ok(Some((zero.clone(), zero)));
    }
    let Some((lo, hi)) = divisor else {
        charge()?;
        if operator == IntegerDivisionOperator::Div {
            if let Some((a, b)) = &dividend {
                let cutoff = a.abs().max(b.abs()).add(&one);
                return infer(
                    operator,
                    law,
                    dividend,
                    Some((cutoff.neg(), cutoff)),
                    charge,
                    at,
                );
            }
        }
        return Ok(match (operator, law, dividend) {
            (IntegerDivisionOperator::Rem, DivisionProfile::Truncating, Some((a, b))) => {
                Some((a.min(zero.clone()), b.max(zero)))
            }
            (IntegerDivisionOperator::Rem, DivisionProfile::Euclidean, Some((a, b)))
                if !a.is_negative() =>
            {
                Some((zero, b))
            }
            _ => None,
        });
    };
    let mut signs = Vec::new();
    if lo.is_negative() {
        signs.push((lo.clone(), hi.clone().min(one.neg())));
    }
    if hi > zero {
        signs.push((lo.clone().max(one.clone()), hi.clone()));
    }
    if signs.is_empty() {
        return Ok(None);
    }
    let Some(dividend) = dividend else {
        if operator == IntegerDivisionOperator::Div {
            return Ok(None);
        }
        charge()?;
        let magnitude = lo.abs().max(hi.abs()).sub(&one);
        return Ok(Some(match law {
            DivisionProfile::Euclidean => (zero, magnitude),
            DivisionProfile::Truncating => (magnitude.neg(), magnitude),
            DivisionProfile::Floor => (
                if lo.is_negative() {
                    lo.abs().sub(&one).neg()
                } else {
                    zero.clone()
                },
                if hi > zero { hi.sub(&one) } else { zero },
            ),
        }));
    };
    let mut result = None;
    for (lo, hi) in signs {
        let found = match operator {
            IntegerDivisionOperator::Div => {
                charge()?;
                let mut values = Vec::with_capacity(4);
                for a in [&dividend.0, &dividend.1] {
                    for b in [&lo, &hi] {
                        values.push(quotient(a, b, law, at)?);
                    }
                }
                let minimum = values
                    .iter()
                    .min()
                    .cloned()
                    .ok_or_else(|| refuse(at, CheckCause::UnrepresentableBound))?;
                let maximum = values
                    .into_iter()
                    .max()
                    .ok_or_else(|| refuse(at, CheckCause::UnrepresentableBound))?;
                (minimum, maximum)
            }
            IntegerDivisionOperator::Rem => {
                let magnitudes = (lo.abs().min(hi.abs()), lo.abs().max(hi.abs()));
                match law {
                    DivisionProfile::Euclidean => euclidean(&dividend, &magnitudes, charge, at)?,
                    DivisionProfile::Floor if lo.is_negative() => negated(euclidean(
                        &negated(dividend.clone()),
                        &magnitudes,
                        charge,
                        at,
                    )?),
                    DivisionProfile::Floor => euclidean(&dividend, &magnitudes, charge, at)?,
                    DivisionProfile::Truncating if dividend.1.is_negative() => negated(euclidean(
                        &negated(dividend.clone()),
                        &magnitudes,
                        charge,
                        at,
                    )?),
                    DivisionProfile::Truncating if !dividend.0.is_negative() => {
                        euclidean(&dividend, &magnitudes, charge, at)?
                    }
                    DivisionProfile::Truncating => hull(
                        negated(euclidean(
                            &(zero.clone(), dividend.0.neg()),
                            &magnitudes,
                            charge,
                            at,
                        )?),
                        euclidean(&(zero.clone(), dividend.1.clone()), &magnitudes, charge, at)?,
                    ),
                }
            }
            IntegerDivisionOperator::Mod => return Ok(None),
        };
        result = Some(result.map_or(found.clone(), |previous| hull(previous, found)));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{DivisionMember, IntegerDomain, Meter, Outcome, ScalarLimits};
    use ix_trace_rs::trace;

    const LIMITS: ScalarLimits = ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    };

    #[test]
    #[trace("TC-1284", "FR-495-AC-1")]
    fn finite_division_intervals_equal_the_hull_of_every_kernel_result() {
        let at =
            ExpressionLocation::root(filament_semantic_value::location::ExpressionOwner::Body {
                function: "range".to_owned(),
                index: 0,
            });
        for law in DivisionProfile::ALL {
            let role = match law {
                DivisionProfile::Truncating => {
                    crate::value::DefinitionRole::IntegerDivisionTruncating
                }
                DivisionProfile::Floor => crate::value::DefinitionRole::IntegerDivisionFloor,
                DivisionProfile::Euclidean => {
                    crate::value::DefinitionRole::IntegerDivisionEuclidean
                }
            };
            let selected = crate::value::admit_integer_division(&[role.reference()], None).unwrap();
            for a in -4_i64..=4 {
                for b in a..=4 {
                    for l in -3_i64..=3 {
                        for u in l..=3 {
                            if l == 0 && u == 0 {
                                continue;
                            }
                            for (operator, member) in [
                                (IntegerDivisionOperator::Div, DivisionMember::Quotient),
                                (IntegerDivisionOperator::Rem, DivisionMember::Remainder),
                            ] {
                                let found = infer(
                                    operator,
                                    law,
                                    Some((Integer::from(a), Integer::from(b))),
                                    Some((Integer::from(l), Integer::from(u))),
                                    &mut || Ok(()),
                                    &at,
                                )
                                .unwrap()
                                .unwrap();
                                let mut values = Vec::new();
                                for x in a..=b {
                                    for y in l..=u {
                                        if y == 0 {
                                            continue;
                                        }
                                        let actual = crate::value::divide(
                                            &selected,
                                            member,
                                            &Integer::from(x),
                                            &Integer::from(y),
                                            &IntegerDomain::Mathematical,
                                            &mut Meter::new(LIMITS),
                                        );
                                        let Outcome::Completed(value) = actual else {
                                            panic!("{actual:?}")
                                        };
                                        values.push(value);
                                    }
                                }
                                assert_eq!(
                                    found,
                                    (
                                        values.iter().min().unwrap().clone(),
                                        values.into_iter().max().unwrap()
                                    ),
                                    "{law:?} {operator:?}: [{a},{b}] by [{l},{u}]"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
