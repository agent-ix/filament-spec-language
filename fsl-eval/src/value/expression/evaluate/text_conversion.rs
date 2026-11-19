// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-478's checked text-to-number conversion kernel.

use std::str::FromStr;

use filament_exact::{
    Charge, ChargePoint, Decimal, DecimalOperation, DecimalType, FloatType, IeeeFlags, IeeeValue,
    IeeeWidth, Integer, LimitKind, Meter, Quantity, Rational, RationalDomain, Refusal,
    RoundingMode, ScalarLimits, Undefined, Value, ValueType, evaluate_decimal, exact_to_ieee,
};
use filament_semantic_value::declaration::TypeEnvironment;
use filament_semantic_value::loss::ValueLoss;
use filament_semantic_value::quantity::{UnitScope, UnitTable};
use filament_semantic_value::stop::{Stop, outcome_into_stop};

/// Convert one admitted `Text` or `String` value to its checked numeric target.
pub(super) fn convert(
    types: &TypeEnvironment,
    units: &UnitScope<'_>,
    target: &ValueType,
    source: &Value,
    meter: &mut Meter,
) -> Result<(Value, Option<ValueLoss>), Stop> {
    let (text, input, profile) = match source {
        Value::Text(text) => (
            text.retained(),
            text.payload().as_str(),
            Some(text.text_type().profile()),
        ),
        Value::String(text) => (text.as_str(), text.as_str(), None),
        _ => return Err(Stop::Undefined(Undefined::MalformedNumber)),
    };
    charge_input(input, text, profile, meter)?;

    match target {
        ValueType::Integer => parse_integer(text)
            .map(Value::Integer)
            .map(|value| (value, None))
            .ok_or(malformed()),
        ValueType::Int(domain) => {
            let value = parse_integer(text).ok_or_else(malformed)?;
            if domain.contains(&value) {
                Ok((Value::Integer(value), None))
            } else {
                Err(Stop::Refused(Refusal::IntegerOutOfDomain {
                    target: Box::new(domain.clone()),
                }))
            }
        }
        ValueType::Decimal(domain) => convert_decimal(text, domain, meter),
        ValueType::Rational(domain) => {
            let value = parse_rational(text).ok_or_else(malformed)?;
            if domain.contains(&value) {
                Ok((Value::Rational(value), None))
            } else {
                Err(Stop::Refused(Refusal::RationalOutOfDomain {
                    target: Box::new(domain.clone()),
                }))
            }
        }
        ValueType::Float(target) => convert_float(text, target, meter),
        ValueType::Quantity(unit) => {
            let unit_name = declared_unit_name(types.units(), unit).ok_or_else(malformed)?;
            let (magnitude, written_unit) = parse_quantity(text).ok_or_else(malformed)?;
            if written_unit.is_some_and(|written| written != unit_name) {
                return Err(malformed());
            }
            if units.get(unit.clone()).is_none() {
                return Err(Stop::Undefined(Undefined::MalformedNumber));
            }
            Ok((
                Value::Quantity(Quantity::new(magnitude, unit.clone())),
                None,
            ))
        }
        _ => Err(malformed()),
    }
}

fn charge_input(
    input: &str,
    retained: &str,
    profile: Option<filament_exact::TextProfile>,
    meter: &mut Meter,
) -> Result<(), Stop> {
    meter.charge(Charge::new(ChargePoint::TextInputBytes).size(
        LimitKind::TextInputBytes,
        u64::try_from(input.len()).unwrap_or(u64::MAX),
    ))?;
    let scalars = u64::try_from(input.chars().count()).unwrap_or(u64::MAX);
    meter.charge(
        Charge::new(ChargePoint::TextDecodeScalars).size(LimitKind::TextScalars, scalars),
    )?;
    if profile.is_some_and(|profile| profile.normalization().is_some()) {
        meter.charge(Charge::new(ChargePoint::TextNormalizeInput))?;
        let outputs = u64::try_from(retained.chars().count()).unwrap_or(u64::MAX);
        for emitted in 1..=outputs {
            meter.charge(
                Charge::new(ChargePoint::TextNormalizeOutput)
                    .size(LimitKind::NormalizedScalars, emitted),
            )?;
        }
    }
    Ok(())
}

fn convert_decimal(
    text: &str,
    target: &DecimalType,
    meter: &mut Meter,
) -> Result<(Value, Option<ValueLoss>), Stop> {
    let decimal = parse_decimal(text).ok_or_else(malformed)?;
    let representation = decimal.representation();
    let coefficient = representation.coefficient();
    let scale = representation.scale();
    if scale <= target.max_scale() {
        if !target.contains(&decimal) {
            return Err(Stop::Refused(Refusal::DecimalOutOfDomain {
                target: Box::new(target.clone()),
            }));
        }
        return Ok((Value::Decimal(decimal), None));
    }
    let discarded_nonzero = scale > target.max_scale()
        && coefficient
            .to_string()
            .chars()
            .rev()
            .take((scale - target.max_scale()) as usize)
            .any(|digit| digit != '0');

    if discarded_nonzero && target.rounding() != RoundingMode::Exact {
        meter.charge(
            Charge::new(ChargePoint::DecimalRounding)
                .size(LimitKind::IntegerBits, coefficient.magnitude_bits())
                .size(LimitKind::DecimalDigits, coefficient.decimal_digits()),
        )?;
    }

    let mut unmetered = Meter::new(UNMETERED);
    let result = outcome_into_stop(evaluate_decimal(
        DecimalOperation::Round(&decimal),
        target,
        &mut unmetered,
    ))?;
    let loss = result.loss().cloned().map(ValueLoss::Decimal);
    Ok((Value::Decimal(result.value().clone()), loss))
}

fn convert_float(
    text: &str,
    target: &FloatType,
    meter: &mut Meter,
) -> Result<(Value, Option<ValueLoss>), Stop> {
    match parse_float(text).ok_or_else(malformed)? {
        ParsedFloat::Infinity(negative) => {
            Ok((Value::Float(infinity(target.width(), negative)), None))
        }
        ParsedFloat::Nan => Ok((Value::Float(canonical_nan(target.width())), None)),
        ParsedFloat::Finite(value, negative_zero) => {
            round_float(value, negative_zero, target, meter)
        }
        ParsedFloat::Extreme { overflow, negative } => {
            let magnitude = if overflow {
                Rational::from_integer(Integer::power_of_ten(400))
            } else {
                Rational::new(Integer::one(), Integer::power_of_ten(500))
                    .expect("power of ten is nonzero")
            };
            let value = if negative { magnitude.neg() } else { magnitude };
            round_float(value, false, target, meter)
        }
    }
}

fn round_float(
    value: Rational,
    negative_zero: bool,
    target: &FloatType,
    meter: &mut Meter,
) -> Result<(Value, Option<ValueLoss>), Stop> {
    meter.charge(
        Charge::new(ChargePoint::IeeeRound)
            .size(LimitKind::IntegerBits, u64::from(target.width().bits())),
    )?;
    if negative_zero {
        return Ok((Value::Float(negative_zero_value(target.width())), None));
    }
    let mut unmetered = Meter::new(UNMETERED);
    let result = outcome_into_stop(exact_to_ieee(
        &value,
        target.width(),
        target.rounding(),
        &mut unmetered,
    ))?;
    let loss = (result.flags() != IeeeFlags::EMPTY).then_some(ValueLoss::IeeeFlags(result.flags()));
    Ok((Value::Float(result.value()), loss))
}

const UNMETERED: ScalarLimits = ScalarLimits {
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

fn declared_unit_name<'a>(table: &'a UnitTable, unit: &filament_exact::UnitId) -> Option<&'a str> {
    let key = unit.declared_key()?;
    table
        .nominal_node(key)?
        .declaration
        .qualified_declaration
        .last()
        .map(String::as_str)
}

fn parse_integer(text: &str) -> Option<Integer> {
    unsigned_digits(text)?;
    let value = Integer::from_str(text).ok()?;
    (text != "-0").then_some(value)
}

fn parse_decimal(text: &str) -> Option<Decimal> {
    let (coefficient, scale, _) = parse_decimal_parts(text, false)?;
    Some(Decimal::new(coefficient, scale))
}

fn parse_decimal_parts(text: &str, allow_negative_zero: bool) -> Option<(Integer, u32, bool)> {
    let (negative, unsigned) = unsigned_body(text)?;
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) if !fraction.is_empty() => (whole, Some(fraction)),
        Some(_) => return None,
        None => (unsigned, None),
    };
    if !natural(whole) || fraction.is_some_and(|fraction| !fraction.bytes().all(ascii_digit)) {
        return None;
    }
    let digits = fraction.map_or(whole.to_owned(), |fraction| format!("{whole}{fraction}"));
    let mut coefficient = Integer::from_str(&digits).ok()?;
    if negative {
        if coefficient.is_zero() && !allow_negative_zero {
            return None;
        }
        coefficient = coefficient.neg();
    }
    let scale = u32::try_from(fraction.map_or(0, str::len)).ok()?;
    Some((coefficient, scale, negative))
}

fn parse_rational(text: &str) -> Option<Rational> {
    if let Some((numerator, denominator)) = text.split_once('/') {
        if denominator.contains('/') || !positive(denominator) {
            return None;
        }
        let (negative, digits) = unsigned_body(numerator)?;
        if !natural(digits) {
            return None;
        }
        let mut numerator = Integer::from_str(digits).ok()?;
        if negative {
            if numerator.is_zero() {
                return None;
            }
            numerator = numerator.neg();
        }
        let denominator = Integer::from_str(denominator).ok()?;
        return Rational::new(numerator, denominator).ok();
    }
    decimal_to_rational(parse_decimal(text)?)
}

fn parse_quantity(text: &str) -> Option<(Rational, Option<&str>)> {
    let (magnitude, unit) = match text.split_once(' ') {
        Some((magnitude, unit)) if !unit.is_empty() && !unit.contains(' ') => {
            (magnitude, Some(unit))
        }
        Some(_) => return None,
        None => (text, None),
    };
    Some((parse_rational(magnitude)?, unit))
}

enum ParsedFloat {
    Infinity(bool),
    Nan,
    Finite(Rational, bool),
    Extreme { overflow: bool, negative: bool },
}

fn parse_float(text: &str) -> Option<ParsedFloat> {
    if text == "inf" {
        return Some(ParsedFloat::Infinity(false));
    }
    if text == "-inf" {
        return Some(ParsedFloat::Infinity(true));
    }
    if text == "nan" {
        return Some(ParsedFloat::Nan);
    }
    let (mantissa, exponent, extreme_exponent) = match text.split_once('e') {
        Some((mantissa, exponent)) if !exponent.contains('e') => {
            let (exponent_negative, digits) = unsigned_body(exponent)?;
            if !natural(digits) || (exponent_negative && digits == "0") {
                return None;
            }
            match u64::from_str(digits)
                .ok()
                .filter(|exponent| *exponent <= 100_000_000)
            {
                Some(exponent) => (mantissa, Some((exponent_negative, exponent)), None),
                None => (mantissa, None, Some(exponent_negative)),
            }
        }
        Some(_) => return None,
        None => (text, None, None),
    };
    let (coefficient, scale, negative) = parse_decimal_parts(mantissa, true)?;
    if coefficient.is_zero() {
        return Some(ParsedFloat::Finite(
            Rational::from_integer(Integer::zero()),
            negative,
        ));
    }
    if let Some(exponent_negative) = extreme_exponent {
        return Some(ParsedFloat::Extreme {
            overflow: !exponent_negative,
            negative,
        });
    }
    let value = if let Some((exponent_negative, exponent)) = exponent {
        let exp = i128::from(exponent);
        let signed_exp = if exponent_negative { -exp } else { exp };
        let adjusted =
            i128::from(coefficient.decimal_digits()) - 1 - i128::from(scale) + signed_exp;
        if adjusted > 400 {
            return Some(ParsedFloat::Extreme {
                overflow: true,
                negative: coefficient.is_negative(),
            });
        }
        if adjusted < -500 {
            return Some(ParsedFloat::Extreme {
                overflow: false,
                negative: coefficient.is_negative(),
            });
        }
        let shift = signed_exp - i128::from(scale);
        if shift >= 0 {
            Rational::from_integer(
                coefficient.mul(&Integer::power_of_ten(u64::try_from(shift).ok()?)),
            )
        } else {
            Rational::new(
                coefficient,
                Integer::power_of_ten(u64::try_from(shift.unsigned_abs()).ok()?),
            )
            .ok()?
        }
    } else {
        decimal_to_rational(Decimal::new(coefficient, scale))?
    };
    Some(ParsedFloat::Finite(value, false))
}

fn decimal_to_rational(decimal: Decimal) -> Option<Rational> {
    let representation = decimal.representation();
    Rational::new(
        representation.coefficient().clone(),
        Integer::power_of_ten(u64::from(representation.scale())),
    )
    .ok()
}

fn unsigned_digits(text: &str) -> Option<&str> {
    let (_, digits) = unsigned_body(text)?;
    natural(digits).then_some(digits)
}

fn unsigned_body(text: &str) -> Option<(bool, &str)> {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |digits| (true, digits));
    (!digits.starts_with('+') && !digits.is_empty()).then_some((negative, digits))
}

fn natural(text: &str) -> bool {
    text == "0"
        || (text
            .as_bytes()
            .first()
            .is_some_and(|digit| (b'1'..=b'9').contains(digit))
            && text.bytes().all(ascii_digit))
}

fn positive(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|digit| (b'1'..=b'9').contains(digit))
        && text.bytes().all(ascii_digit)
}

fn ascii_digit(byte: u8) -> bool {
    (b'0'..=b'9').contains(&byte)
}

fn malformed() -> Stop {
    Stop::Undefined(Undefined::MalformedNumber)
}

fn infinity(width: IeeeWidth, negative: bool) -> IeeeValue {
    match width {
        IeeeWidth::Binary32 => {
            IeeeValue::binary32(if negative { 0xff80_0000 } else { 0x7f80_0000 })
        }
        IeeeWidth::Binary64 => IeeeValue::binary64(if negative {
            0xfff0_0000_0000_0000
        } else {
            0x7ff0_0000_0000_0000
        }),
    }
}

fn canonical_nan(width: IeeeWidth) -> IeeeValue {
    match width {
        IeeeWidth::Binary32 => IeeeValue::binary32(0x7fc0_0000),
        IeeeWidth::Binary64 => IeeeValue::binary64(0x7ff8_0000_0000_0000),
    }
}

fn negative_zero_value(width: IeeeWidth) -> IeeeValue {
    match width {
        IeeeWidth::Binary32 => IeeeValue::binary32(0x8000_0000),
        IeeeWidth::Binary64 => IeeeValue::binary64(0x8000_0000_0000_0000),
    }
}
