// SPDX-License-Identifier: AGPL-3.0-or-later
//! Membership proofs for explicit numeric narrowing, separate from classification.
use super::{Definedness, End, Facts, Interval};
use crate::check::ir::CheckedNode;
use crate::check::numeric_conversion::{classify, Class};
use crate::check::refusal::{CheckRefusal, Obligation, OperandText};
use filament_checked::node::{CheckedLiteral, NodeKind};
use filament_exact::{
    DecimalType, Integer, IntegerInterval, NonZeroInteger, RationalDomain, Value, ValueType,
};
use filament_semantic_value::declaration::admits_equality_conversion;
use filament_semantic_value::quantity::{UnitScope, UnitTable};

impl Definedness<'_> {
    pub(super) fn numeric_literal(&self, mut node: CheckedNode<'_>) -> Option<Value> {
        loop {
            match node.kind() {
                NodeKind::Literal(CheckedLiteral(
                    value @ (Value::Integer(_) | Value::Rational(_) | Value::Decimal(_)),
                )) => return Some(value.clone()),
                NodeKind::Local(slot) => return self.numeric_literals.get(slot).cloned(),
                NodeKind::ConvertScalar(_, operand) => node = node.at(*operand),
                NodeKind::Coerce(operand, _) => node = node.at(*operand),
                _ => return None,
            }
        }
    }

    pub(super) fn discharge_conversion(
        &self,
        node: CheckedNode<'_>,
        operand: CheckedNode<'_>,
        facts: &Facts,
    ) -> Result<(), CheckRefusal> {
        if !matches!(
            classify(operand.value_type(), node.value_type()),
            Some(Class::RangeNarrowing | Class::RationalNarrowing)
        ) {
            return Ok(());
        }
        // An integer guard refines the value interval, never the conversion class.
        let interval = self.interval(operand, facts);
        let proved = match (&interval.lower, &interval.upper) {
            (End::Finite(lower), End::Finite(upper)) => {
                IntegerInterval::new(lower.clone(), upper.clone())
                    .ok()
                    .map(ValueType::Int)
            }
            _ => None,
        };
        let literal = self.numeric_literal(operand);
        // Membership uses the exact converted value; the equality conversion
        // table conservatively bounds decimal numerator and denominator ranges.
        if let (Some(Value::Decimal(value)), ValueType::Rational(target)) =
            (&literal, node.value_type())
        {
            if target.contains(&value.representation().to_rational()) {
                return Ok(());
            }
        }
        if let (Some(Value::Decimal(value)), ValueType::Decimal(target)) =
            (&literal, node.value_type())
        {
            if target.contains(value) {
                return Ok(());
            }
        }
        let literal_type = literal.and_then(|value| match value {
            Value::Integer(value) => IntegerInterval::new(value.clone(), value)
                .ok()
                .map(ValueType::Int),
            Value::Rational(value) => {
                let numerator =
                    IntegerInterval::new(value.numerator().clone(), value.numerator().clone())
                        .ok()?;
                let denominator =
                    IntegerInterval::new(value.denominator().clone(), value.denominator().clone())
                        .ok()?;
                RationalDomain::new(numerator, denominator)
                    .ok()
                    .map(ValueType::Rational)
            }
            Value::Decimal(value) => {
                let value = value.normalized();
                if value.scale() == 0 {
                    return IntegerInterval::new(
                        value.coefficient().clone(),
                        value.coefficient().clone(),
                    )
                    .ok()
                    .map(ValueType::Int);
                }
                DecimalType::new(
                    value.coefficient().clone(),
                    value.coefficient().clone(),
                    u64::from(value.scale()),
                    u64::from(value.scale()),
                    filament_exact::RoundingMode::Exact,
                )
                .ok()
                .map(ValueType::Decimal)
            }
            _ => None,
        });
        let units = UnitTable::default();
        let units = UnitScope::new(&units);
        if literal_type
            .or(proved)
            .as_ref()
            .is_some_and(|source| admits_equality_conversion(source, node.value_type(), &units))
        {
            return Ok(());
        }
        if matches!(node.value_type(), ValueType::Rational(_)) {
            return Err(Self::refuse(node, Obligation::RationalRange));
        }
        let required = match node.value_type() {
            ValueType::Int(target) => Interval::of_domain(target),
            ValueType::Decimal(target) => decimal_hull(target),
            _ => Interval::unbounded(),
        };
        Err(Self::refuse(
            node,
            Obligation::Range {
                required: Box::new(required.proved()),
                proved: Box::new(interval.proved()),
                operand: OperandText::default(),
            },
        ))
    }
}

/// The integer interval spanning every value a `Decimal` target admits: its
/// coefficient bounds read at the smallest and the largest membership scale,
/// rounded outward.
fn decimal_hull(target: &DecimalType) -> Interval {
    let at = |scale: u32| {
        let power = Integer::power_of_ten(u64::from(scale));
        let divisor = NonZeroInteger::try_from(&power).ok()?;
        let lower = target.lower().div_mod_floor(divisor).0;
        let upper = target.upper().neg().div_mod_floor(divisor).0.neg();
        Some((lower, upper))
    };
    match (at(target.min_scale()), at(target.max_scale())) {
        (Some((low_a, high_a)), Some((low_b, high_b))) => Interval {
            lower: End::Finite(low_a.min(low_b)),
            upper: End::Finite(high_a.max(high_b)),
        },
        _ => Interval::unbounded(),
    }
}
