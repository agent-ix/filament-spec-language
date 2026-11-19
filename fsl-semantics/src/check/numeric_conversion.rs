// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-149's first-match conversion classes, based only on declared types.
use filament_exact::{Integer, ValueType};
use filament_semantic_value::declaration::admits_equality_conversion;
use filament_semantic_value::quantity::{UnitScope, UnitTable};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Class {
    Exact,
    ScaleReduction,
    RationalNarrowing,
    RangeNarrowing,
}

pub(super) fn classify(source: &ValueType, target: &ValueType) -> Option<Class> {
    use ValueType::{Decimal, Int, Integer as UnboundedInteger, Rational};
    if !matches!(source, UnboundedInteger | Int(_) | Rational(_) | Decimal(_))
        || !matches!(target, UnboundedInteger | Int(_) | Rational(_) | Decimal(_))
    {
        return None;
    }
    if admits_equality_conversion(source, target, &UnitScope::new(&UnitTable::default())) {
        return Some(Class::Exact);
    }
    match (source, target) {
        (Decimal(from), Decimal(to)) if from.max_scale() > to.max_scale() => {
            Some(Class::ScaleReduction)
        }
        (Rational(from), Decimal(_)) if from.denominator().upper() > &Integer::one() => {
            Some(Class::ScaleReduction)
        }
        (Rational(_), Rational(_)) => Some(Class::RationalNarrowing),
        (UnboundedInteger | Int(_), _)
        | (Decimal(_), Rational(_) | Decimal(_))
        | (Rational(_), UnboundedInteger | Int(_))
        | (Rational(_), Decimal(_)) => Some(Class::RangeNarrowing),
        (Decimal(from), UnboundedInteger | Int(_)) if from.max_scale() == 0 => {
            Some(Class::RangeNarrowing)
        }
        _ => None,
    }
}
