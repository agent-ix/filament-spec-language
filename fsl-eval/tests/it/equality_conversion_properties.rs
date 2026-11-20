// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit FR-149 equality conversions, rather than implicit numeric comparison.
//! The reference uses bounded i128 fractions and declared representation grids.
//! Composite float leaves remain outside this suite under IR-1320 / STD-654.

use std::cell::Cell;

use filament_exact::{
    FloatType, IeeeValue, IeeeWidth, IllTyped, IllTypedCause, Meter, OptionValue, Outcome,
    Quantity, Rational, RoundingMode, Value, ValueType,
};
use filament_semantic_value::declaration::{
    admits_equality_conversion, EqualityOperand, EqualityOperator, TypeEnvironment,
};
use filament_semantic_value::enumeration::EnumMemberIndex;
use filament_semantic_value::quantity::UnitScope;
use ix_trace_rs::trace;
use proptest::prelude::{Strategy, TestCaseError};
use proptest::test_runner::{TestError, TestRunner};

use super::{decimal, decimal_type_mode, int, int_type, integer, rational, rational_type, Units};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fraction {
    numerator: i128,
    denominator: i128,
}

impl Fraction {
    fn new(numerator: i64, denominator: i64) -> Self {
        let (mut a, mut b) = (i128::from(numerator).abs(), i128::from(denominator));
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let divisor = a.max(1);
        Self {
            numerator: i128::from(numerator) / divisor,
            denominator: i128::from(denominator) / divisor,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Numeric {
    Integer,
    Int(i64, i64),
    Rational(i64, i64, i64),
    Decimal(i64, i64, u32, u32, RoundingMode),
}

impl Numeric {
    fn value_type(self) -> ValueType {
        match self {
            Self::Integer => ValueType::Integer,
            Self::Int(lo, hi) => int_type(lo, hi),
            Self::Rational(lo, hi, denominator) => rational_type(lo, hi, 1, denominator),
            Self::Decimal(lo, hi, min, max, mode) => {
                decimal_type_mode(lo, hi, u64::from(min), u64::from(max), mode)
            }
        }
    }

    // This reference does not call kernel conversion, membership or equality.
    fn represent(self, number: Fraction) -> Option<Value> {
        let numerator = i64::try_from(number.numerator).expect("bounded reference numerator");
        let denominator = i64::try_from(number.denominator).expect("bounded denominator");
        match self {
            Self::Integer if denominator == 1 => Some(int(numerator)),
            Self::Int(lo, hi) if denominator == 1 && (lo..=hi).contains(&numerator) => {
                Some(int(numerator))
            }
            Self::Rational(lo, hi, max_denominator)
                if (lo..=hi).contains(&numerator) && denominator <= max_denominator =>
            {
                Some(rational(numerator, denominator))
            }
            Self::Decimal(lo, hi, min, max, _) => (min..=max)
                .find(|scale| number.numerator * 10i128.pow(*scale) % number.denominator == 0)
                .and_then(|scale| {
                    // FR-140 chooses the least permitted representation grid;
                    // a later scale cannot rescue out-of-domain coefficients.
                    let coefficient =
                        i64::try_from(number.numerator * 10i128.pow(scale) / number.denominator)
                            .ok()?;
                    (lo..=hi)
                        .contains(&coefficient)
                        .then(|| decimal(coefficient, scale))
                }),
            Self::Integer | Self::Int(..) | Self::Rational(..) => None,
        }
    }

    fn members(self) -> Vec<(Fraction, Value)> {
        let (lo, hi, denominators) = match self {
            Self::Integer => (-31, 31, vec![1]),
            Self::Int(lo, hi) => (lo, hi, vec![1]),
            Self::Rational(lo, hi, denominator) => (lo, hi, vec![1, denominator]),
            Self::Decimal(lo, hi, min, max, _) => {
                (lo, hi, (min..=max).map(|scale| 10i64.pow(scale)).collect())
            }
        };
        let mut coefficients = vec![lo, hi];
        if (lo..=hi).contains(&0) {
            coefficients.push(0);
        }
        if lo < hi {
            coefficients.extend([lo + 1, hi - 1]);
        }
        coefficients.sort_unstable();
        coefficients.dedup();
        let mut result = Vec::new();
        for coefficient in coefficients {
            for denominator in &denominators {
                let number = Fraction::new(coefficient, *denominator);
                // Rational bounds constrain normalized numerators. A raw
                // reducible pair can therefore lie outside this domain.
                if let Some(value) = self.represent(number) {
                    result.push((number, value));
                }
            }
        }
        assert!(!result.is_empty(), "denominator-one boundary members exist");
        result
    }
}

fn parameters() -> impl Strategy<Value = (i64, i64, i64, u32, usize)> {
    (1i64..=30, 1i64..=30, 2i64..=7, 0u32..=2, 0usize..6)
}

fn admitted_rows(
    (negative, positive, denominator, scale, mode): (i64, i64, i64, u32, usize),
) -> Vec<(Numeric, Numeric)> {
    let mut rows = vec![(Numeric::Integer, Numeric::Integer)];
    for (lo, hi) in [
        (-negative, positive),
        (positive, positive + negative),
        (-negative - positive, -negative),
    ] {
        rows.extend(admitted_rows_for_bounds((lo, hi, denominator, scale, mode)));
    }
    rows
}

fn admitted_rows_for_bounds(
    (lo, hi, denominator, scale, mode): (i64, i64, i64, u32, usize),
) -> Vec<(Numeric, Numeric)> {
    let widening = RoundingMode::ALL[(mode + 1) % RoundingMode::ALL.len()];
    let mode = RoundingMode::ALL[mode];
    let integer = Numeric::Int(lo, hi);
    let rational = Numeric::Rational(lo, hi, denominator);
    let decimal = Numeric::Decimal(lo, hi, scale, scale + 1, mode);
    let ten = 10i64.pow(scale);
    vec![
        (integer, integer),
        (integer, Numeric::Integer),
        (integer, Numeric::Int(lo - 1, hi + 1)),
        (integer, Numeric::Rational(lo - 1, hi + 1, denominator)),
        (
            integer,
            Numeric::Decimal(lo * ten, hi * ten, scale, scale + 1, mode),
        ),
        (rational, rational),
        (rational, Numeric::Rational(lo - 1, hi + 1, denominator + 1)),
        (Numeric::Rational(lo, hi, 1), Numeric::Integer),
        (Numeric::Rational(lo, hi, 1), Numeric::Int(lo - 1, hi + 1)),
        (
            Numeric::Rational(lo, hi, 1),
            Numeric::Decimal(lo * ten, hi * ten, scale, scale, mode),
        ),
        (decimal, decimal),
        (
            decimal,
            Numeric::Rational(lo.min(0), hi.max(0), 10i64.pow(scale + 1)),
        ),
        (
            decimal,
            Numeric::Decimal(lo - 1, hi + 1, scale, scale + 2, widening),
        ),
        (
            decimal,
            Numeric::Decimal(lo.min(0) - 1, hi.max(0) + 1, 0, scale + 2, widening),
        ),
        (Numeric::Decimal(lo, hi, 0, 0, mode), Numeric::Integer),
        (Numeric::Decimal(lo, hi, 0, 0, mode), Numeric::Int(lo, hi)),
    ]
}

fn compare(
    environment: &TypeEnvironment,
    units: &Units,
    source_type: &ValueType,
    target_type: &ValueType,
    source: &Value,
    expected: &Value,
    equal: bool,
) -> Result<(), TestCaseError> {
    let scope = UnitScope::new(&units.table);
    proptest::prop_assert!(
        admits_equality_conversion(source_type, target_type, &scope),
        "admission {source_type:?} -> {target_type:?}, source={source:?}"
    );
    let before = format!("{source:?}");
    for (operator, answer) in [
        (EqualityOperator::Equal, equal),
        (EqualityOperator::NotEqual, !equal),
    ] {
        let checked = environment.check_equality(
            operator,
            EqualityOperand::converted(source_type.clone(), target_type.clone()),
            EqualityOperand::typed(target_type.clone()),
            &EnumMemberIndex::default(),
        );
        let checked = checked.map_err(|error| {
            TestCaseError::fail(format!("{source_type:?} -> {target_type:?}: {error:?}"))
        })?;
        proptest::prop_assert_eq!(
            checked.evaluate(
                environment,
                source,
                expected,
                &mut Meter::new(super::UNLIMITED)
            ),
            Outcome::Completed(answer),
            "{:?} -> {:?}, source={:?}, comparison={:?}",
            source_type,
            target_type,
            source,
            expected
        );
    }
    proptest::prop_assert_eq!(
        format!("{source:?}"),
        before,
        "conversion mutated the source representation"
    );
    Ok(())
}

fn preserved_member(
    source: Numeric,
    target: Numeric,
    number: Fraction,
) -> Result<Value, TestCaseError> {
    proptest::prop_assert!(
        source.represent(number).is_some(),
        "invalid source member {source:?} {number:?}"
    );
    target.represent(number).ok_or_else(|| {
        TestCaseError::fail(format!(
            "{source:?} -> {target:?} loses boundary member {number:?}"
        ))
    })
}

/// Provenance: QSL-2057. Each sample exercises every admitted numeric row.
#[trace("FSpec-FR-149-AC-5", "FSpec-FR-149-AC-10")]
#[test]
fn admitted_conversions_lose_nothing() {
    let units = super::units();
    let environment = TypeEnvironment::default().with_units(units.table.clone());
    fsl_bench::prop::run(
        file!(),
        fsl_bench::prop::config(256),
        &parameters(),
        |parameters| {
            for (source, target) in admitted_rows(parameters) {
                let source_type = source.value_type();
                let target_type = target.value_type();
                let members = source.members();
                for (number, value) in &members {
                    let expected = preserved_member(source, target, *number)?;
                    compare(
                        &environment,
                        &units,
                        &source_type,
                        &target_type,
                        value,
                        &expected,
                        true,
                    )?;
                    if let Some((different, _)) =
                        members.iter().find(|(candidate, _)| candidate != number)
                    {
                        let unequal = target
                            .represent(*different)
                            .expect("admitted target member");
                        compare(
                            &environment,
                            &units,
                            &source_type,
                            &target_type,
                            value,
                            &unequal,
                            false,
                        )?;
                    }
                }
            }
            identity_families(&environment, &units, parameters)?;
            Ok(())
        },
    )
    .unwrap();
}

fn identity_families(
    environment: &TypeEnvironment,
    units: &Units,
    parameters: (i64, i64, i64, u32, usize),
) -> Result<(), TestCaseError> {
    let (negative, positive, denominator, scale, mode) = parameters;
    for inner in [
        Numeric::Int(-negative, positive),
        Numeric::Rational(-negative, positive, denominator),
        Numeric::Decimal(
            -negative,
            positive,
            scale,
            scale + 1,
            RoundingMode::ALL[mode],
        ),
    ] {
        let inner_type = inner.value_type();
        let option = ValueType::option(inner_type.clone());
        let none = OptionValue::none(inner_type.clone());
        compare(environment, units, &option, &option, &none, &none, true)?;
        for (_, value) in inner.members() {
            let present = OptionValue::present(inner_type.clone(), value).unwrap();
            compare(
                environment,
                units,
                &option,
                &option,
                &present,
                &present,
                true,
            )?;
            compare(environment, units, &option, &option, &present, &none, false)?;
        }
    }
    for (source_unit, target_unit, multiplier, divisor) in [
        (units.cm.clone(), units.m.clone(), 1, 100),
        (units.m.clone(), units.cm.clone(), 100, 1),
        (units.m.clone(), units.m.clone(), 1, 1),
    ] {
        for magnitude in [-negative, 0, positive] {
            let source = Value::Quantity(Quantity::new(
                Rational::from_integer(integer(magnitude)),
                source_unit.clone(),
            ));
            let expected = Value::Quantity(Quantity::new(
                Rational::new(integer(magnitude * multiplier), integer(divisor)).unwrap(),
                target_unit.clone(),
            ));
            compare(
                environment,
                units,
                &ValueType::Quantity(source_unit.clone()),
                &ValueType::Quantity(target_unit.clone()),
                &source,
                &expected,
                true,
            )?;
            let unequal = Value::Quantity(Quantity::new(
                Rational::new(integer((magnitude + 1) * multiplier), integer(divisor)).unwrap(),
                target_unit.clone(),
            ));
            compare(
                environment,
                units,
                &ValueType::Quantity(source_unit.clone()),
                &ValueType::Quantity(target_unit.clone()),
                &source,
                &unequal,
                false,
            )?;
        }
    }
    for unit in [&units.m, &units.cm] {
        let inner = ValueType::Quantity(unit.clone());
        let option = ValueType::option(inner.clone());
        let none = OptionValue::none(inner.clone());
        compare(environment, units, &option, &option, &none, &none, true)?;
        for magnitude in [-negative, positive] {
            let quantity = Value::Quantity(Quantity::new(
                Rational::from_integer(integer(magnitude)),
                unit.clone(),
            ));
            let present = OptionValue::present(inner.clone(), quantity).unwrap();
            compare(
                environment,
                units,
                &option,
                &option,
                &present,
                &present,
                true,
            )?;
            compare(environment, units, &option, &option, &present, &none, false)?;
        }
    }
    for width in IeeeWidth::ALL {
        let (bits, exponent, fraction) = match width {
            IeeeWidth::Binary32 => (u64::try_from(positive).unwrap(), 0x7f80_0000, 0x007f_ffff),
            IeeeWidth::Binary64 => (
                u64::try_from(positive).unwrap(),
                0x7ff0_0000_0000_0000,
                0x000f_ffff_ffff_ffff,
            ),
        };
        let sign = match width {
            IeeeWidth::Binary32 => 0x8000_0000,
            IeeeWidth::Binary64 => 0x8000_0000_0000_0000,
        };
        let minimum_normal = fraction + 1;
        let one = sign / 2 - minimum_normal;
        for payload in [
            0,
            sign,
            bits,
            sign | bits,
            minimum_normal,
            sign | minimum_normal,
            one | bits,
            sign | one | bits,
            exponent - 1,
            sign | (exponent - 1),
            exponent,
            sign | exponent,
            exponent | fraction,
            sign | exponent | fraction,
        ] {
            let value = match width {
                IeeeWidth::Binary32 => IeeeValue::binary32(u32::try_from(payload).unwrap()),
                IeeeWidth::Binary64 => IeeeValue::binary64(payload),
            };
            let is_nan = payload & exponent == exponent && payload & fraction != 0;
            let value = Value::Float(value);
            for mode in RoundingMode::ALL {
                let value_type = ValueType::Float(FloatType::new(width, mode));
                compare(
                    environment,
                    units,
                    &value_type,
                    &value_type,
                    &value,
                    &value,
                    !is_nan,
                )?;
                if !is_nan {
                    let other_bits = if payload & !sign == 0 { one } else { 0 };
                    let other = match width {
                        IeeeWidth::Binary32 => {
                            IeeeValue::binary32(u32::try_from(other_bits).unwrap())
                        }
                        IeeeWidth::Binary64 => IeeeValue::binary64(other_bits),
                    };
                    compare(
                        environment,
                        units,
                        &value_type,
                        &value_type,
                        &value,
                        &Value::Float(other),
                        false,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn refused(
    environment: &TypeEnvironment,
    units: &Units,
    source: ValueType,
    target: ValueType,
) -> Result<(), TestCaseError> {
    proptest::prop_assert!(
        !admits_equality_conversion(&source, &target, &UnitScope::new(&units.table)),
        "lossy or untabled conversion admitted: {source:?} -> {target:?}"
    );
    for operator in [EqualityOperator::Equal, EqualityOperator::NotEqual] {
        let actual = environment.check_equality(
            operator,
            EqualityOperand::converted(source.clone(), target.clone()),
            EqualityOperand::typed(target.clone()),
            &EnumMemberIndex::default(),
        );
        proptest::prop_assert_eq!(
            actual,
            Err(IllTyped {
                cause: IllTypedCause::TypeMismatch
            }),
            "{:?} -> {:?}",
            source,
            target
        );
    }
    Ok(())
}

/// Provenance: QSL-2057. Every numeric rejection has a real boundary counterexample.
#[trace("FSpec-FR-149-AC-10")]
#[test]
fn lossy_conversions_are_not_admitted() {
    let units = super::units();
    let environment = TypeEnvironment::default().with_units(units.table.clone());
    fsl_bench::prop::run(file!(), fsl_bench::prop::config(256), &parameters(), |p| {
        let (negative, positive, denominator, scale, mode) = p;
        let (lo, hi) = (-negative, positive);
        let mode = RoundingMode::ALL[mode];
        for (source, target, witness) in [
            (
                Numeric::Int(lo, hi),
                Numeric::Int(lo, hi - 1),
                Fraction::new(hi, 1),
            ),
            (
                Numeric::Rational(lo, hi, denominator),
                Numeric::Rational(lo, hi - 1, denominator),
                Fraction::new(hi, 1),
            ),
            (
                Numeric::Rational(lo, hi, denominator),
                Numeric::Rational(lo, hi, denominator - 1),
                Fraction::new(1, denominator),
            ),
            (
                Numeric::Decimal(lo, hi, 0, scale + 1, mode),
                Numeric::Decimal(lo, hi - 1, 0, scale + 1, mode),
                Fraction::new(hi, 1),
            ),
            (
                Numeric::Decimal(lo, hi, 0, scale + 1, mode),
                Numeric::Decimal(lo, hi, 0, scale, mode),
                Fraction::new(1, 10i64.pow(scale + 1)),
            ),
            (
                Numeric::Rational(lo, hi, denominator),
                Numeric::Int(lo, hi),
                Fraction::new(1, denominator),
            ),
            (
                Numeric::Decimal(lo, hi, 0, 1, mode),
                Numeric::Integer,
                Fraction::new(1, 10),
            ),
        ] {
            proptest::prop_assert!(
                source.represent(witness).is_some(),
                "invalid source witness {source:?} {witness:?}"
            );
            proptest::prop_assert!(
                target.represent(witness).is_none(),
                "not a lossy witness {target:?} {witness:?}"
            );
            refused(
                &environment,
                &units,
                source.value_type(),
                target.value_type(),
            )?;
        }
        refused(
            &environment,
            &units,
            ValueType::option(int_type(lo, hi)),
            ValueType::option(int_type(lo, hi - 1)),
        )?;
        refused(
            &environment,
            &units,
            ValueType::Quantity(units.m.clone()),
            ValueType::Quantity(units.s.clone()),
        )?;
        Ok(())
    })
    .unwrap();
}

/// Provenance: QSL-2057. IEEE/exact conversions remain outside the closed table.
#[trace("FSpec-FR-149-AC-10")]
#[test]
fn float_decimal_conversions_are_refused() {
    let units = super::units();
    let environment = TypeEnvironment::default().with_units(units.table.clone());
    for width in IeeeWidth::ALL {
        for (index, mode) in RoundingMode::ALL.into_iter().enumerate() {
            let float = ValueType::Float(FloatType::new(width, mode));
            let decimal = decimal_type_mode(-100, 100, 0, 2, mode);
            refused(&environment, &units, float.clone(), decimal.clone()).unwrap();
            refused(&environment, &units, decimal, float.clone()).unwrap();
            let other_width = match width {
                IeeeWidth::Binary32 => IeeeWidth::Binary64,
                IeeeWidth::Binary64 => IeeeWidth::Binary32,
            };
            refused(
                &environment,
                &units,
                float.clone(),
                ValueType::Float(FloatType::new(other_width, mode)),
            )
            .unwrap();
            let other_mode = RoundingMode::ALL[(index + 1) % RoundingMode::ALL.len()];
            refused(
                &environment,
                &units,
                float,
                ValueType::Float(FloatType::new(width, other_mode)),
            )
            .unwrap();
        }
    }
}

fn lost_member(source_high: i64, target_high: i64, admitted: bool) -> Result<(), TestCaseError> {
    if admitted {
        preserved_member(
            Numeric::Int(0, source_high),
            Numeric::Int(0, target_high),
            Fraction::new(source_high, 1),
        )?;
    }
    Ok(())
}

/// Provenance: QSL-2057. A wrong independent row fails and shrinks to one lost member.
#[trace("FSpec-FR-149-AC-10")]
#[test]
fn wrong_row_is_caught() {
    assert!(lost_member(300, 255, true).is_err());
    let units = super::units();
    let environment = TypeEnvironment::default().with_units(units.table.clone());
    refused(&environment, &units, int_type(0, 300), int_type(0, 255)).unwrap();
    let mut config = fsl_bench::prop::config(1);
    config.failure_persistence = None;
    let calls = Cell::new(0);
    let outcome = TestRunner::new(config).run(&(1i64..=300, 0i64..=255), |(gap, target)| {
        calls.set(calls.get() + 1);
        lost_member(target + gap, target, true)
    });
    let Err(TestError::Fail(message, (gap, target))) = outcome else {
        panic!("the intentionally wrong row must fail");
    };
    assert_eq!((gap, target), (1, 0));
    assert!(calls.get() > 1 && calls.get() <= 258);
    assert!(message.to_string().contains("numerator: 1, denominator: 1"));
}
