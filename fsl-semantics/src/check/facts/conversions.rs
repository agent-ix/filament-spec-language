// SPDX-License-Identifier: AGPL-3.0-or-later
//! Membership proofs for explicit numeric narrowing, separate from classification.
use super::{Definedness, End, Facts, Interval};
use crate::check::ir::CheckedNode;
use crate::check::numeric_conversion::{classify, Class};
use crate::check::refusal::{CheckRefusal, Obligation, OperandText};
use filament_checked::node::{CheckedLiteral, NodeKind};
use filament_exact::{
    ArithmeticOperator, DecimalType, Integer, IntegerInterval, NonZeroInteger, RationalDomain,
    RoundingMode, Value, ValueType,
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
        // FR-093 "Explicit numeric narrowing": a decimal operand is proved by
        // the declared ranges of its typed operands under exact interval
        // arithmetic, never by the type the sum is written at.
        if let ValueType::Decimal(target) = node.value_type() {
            if DecimalBounds::of(operand).is_some_and(|bounds| bounds.within(target)) {
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

/// An exact decimal value `numerator / 10^scale`, the endpoint of a
/// [`DecimalBounds`] interval.
#[derive(Clone)]
struct Scaled {
    numerator: Integer,
    scale: u32,
}

impl Scaled {
    /// This value and `other` over their larger common scale.
    fn aligned(&self, other: &Self) -> (Integer, Integer, u32) {
        let scale = self.scale.max(other.scale);
        let lift = |value: &Self| {
            value
                .numerator
                .mul(&Integer::power_of_ten(u64::from(scale - value.scale)))
        };
        (lift(self), lift(other), scale)
    }

    fn add(&self, other: &Self) -> Self {
        let (a, b, scale) = self.aligned(other);
        Self {
            numerator: a.add(&b),
            scale,
        }
    }

    fn sub(&self, other: &Self) -> Self {
        let (a, b, scale) = self.aligned(other);
        Self {
            numerator: a.sub(&b),
            scale,
        }
    }

    fn mul(&self, other: &Self) -> Self {
        Self {
            numerator: self.numerator.mul(&other.numerator),
            scale: self.scale.saturating_add(other.scale),
        }
    }

    fn at_least(&self, other: &Self) -> bool {
        let (a, b, _) = self.aligned(other);
        a >= b
    }
}

/// The exact value interval of a decimal-typed node: a literal is a point, a
/// `+`, `-` or `*` combines its operands' intervals exactly, and any other
/// node spans the values its declared `Decimal[..]` type admits.
struct DecimalBounds {
    lower: Scaled,
    upper: Scaled,
}

impl DecimalBounds {
    fn of(node: CheckedNode<'_>) -> Option<Self> {
        match node.kind() {
            NodeKind::Literal(CheckedLiteral(Value::Decimal(value))) => {
                let point = Scaled {
                    numerator: value.representation().coefficient().clone(),
                    scale: value.representation().scale(),
                };
                Some(Self {
                    lower: point.clone(),
                    upper: point,
                })
            }
            NodeKind::Decimal {
                operator,
                left,
                right,
                ..
            } => {
                let (left, right) = (Self::of(node.at(*left))?, Self::of(node.at(*right))?);
                let bounds = match operator {
                    ArithmeticOperator::Add => Some(Self {
                        lower: left.lower.add(&right.lower),
                        upper: left.upper.add(&right.upper),
                    }),
                    ArithmeticOperator::Subtract => Some(Self {
                        lower: left.lower.sub(&right.upper),
                        upper: left.upper.sub(&right.lower),
                    }),
                    ArithmeticOperator::Multiply => {
                        let corners = [
                            left.lower.mul(&right.lower),
                            left.lower.mul(&right.upper),
                            left.upper.mul(&right.lower),
                            left.upper.mul(&right.upper),
                        ];
                        let mut lower = corners[0].clone();
                        let mut upper = corners[0].clone();
                        for corner in &corners[1..] {
                            if lower.at_least(corner) {
                                lower = corner.clone();
                            }
                            if corner.at_least(&upper) {
                                upper = corner.clone();
                            }
                        }
                        Some(Self { lower, upper })
                    }
                    // A quotient has no exact decimal interval.
                    ArithmeticOperator::Divide => None,
                }?;
                // A node whose own type rounds stores a grid value at its maximum
                // scale, which lies between the exact bounds rounded outward onto
                // that grid.
                Some(match node.value_type() {
                    ValueType::Decimal(own) if own.rounding() != RoundingMode::Exact => {
                        bounds.rounded_outward(own.max_scale())?
                    }
                    _ => bounds,
                })
            }
            _ => match node.value_type() {
                ValueType::Decimal(declared) => Some(Self::declared(declared)),
                _ => None,
            },
        }
    }

    /// These bounds with the lower rounded down and the upper rounded up onto
    /// the grid of multiples of `10^-scale`.
    fn rounded_outward(self, scale: u32) -> Option<Self> {
        let onto_grid = |value: Scaled, up: bool| -> Option<Scaled> {
            if value.scale <= scale {
                return Some(value);
            }
            let power = Integer::power_of_ten(u64::from(value.scale - scale));
            let divisor = NonZeroInteger::try_from(&power).ok()?;
            let numerator = if up {
                value.numerator.neg().div_mod_floor(divisor).0.neg()
            } else {
                value.numerator.div_mod_floor(divisor).0
            };
            Some(Scaled { numerator, scale })
        };
        Some(Self {
            lower: onto_grid(self.lower, false)?,
            upper: onto_grid(self.upper, true)?,
        })
    }

    /// The values a declared `Decimal[..]` type admits: its coefficient
    /// bounds read at its smallest and its largest scale.
    fn declared(declared: &DecimalType) -> Self {
        let at = |coefficient: &Integer, scale: u32| Scaled {
            numerator: coefficient.clone(),
            scale,
        };
        let (min, max) = (declared.min_scale(), declared.max_scale());
        let lowers = [at(declared.lower(), min), at(declared.lower(), max)];
        let uppers = [at(declared.upper(), min), at(declared.upper(), max)];
        let [lower_a, lower_b] = lowers;
        let [upper_a, upper_b] = uppers;
        Self {
            lower: if lower_a.at_least(&lower_b) {
                lower_b
            } else {
                lower_a
            },
            upper: if upper_a.at_least(&upper_b) {
                upper_a
            } else {
                upper_b
            },
        }
    }

    /// Whether every value of this interval is a member of `target` at the
    /// target's smallest and its largest scale.
    fn within(&self, target: &DecimalType) -> bool {
        [target.min_scale(), target.max_scale()]
            .into_iter()
            .all(|scale| {
                let lower = Scaled {
                    numerator: target.lower().clone(),
                    scale,
                };
                let upper = Scaled {
                    numerator: target.upper().clone(),
                    scale,
                };
                self.lower.at_least(&lower) && upper.at_least(&self.upper)
            })
    }
}
