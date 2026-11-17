// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-478: Text and String values convert through their fixed numeric grammar.

use filament_eval::FamilyOutcome;
use filament_exact::{
    admit_text, ChargePoint, DecimalRepresentation, Integer, LimitKind, Meter, Outcome, Rational,
    Refusal, ScalarLimits, TextPayload, TextProfile, TextType, Value,
};
use filament_semantic_value::loss::ValueLoss;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use std::str::FromStr;

const FUNCTIONS: &str = "\
    dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
    function integer_text using v(t: Text[0, 64; unicode-scalars]): Integer pure { convert<Integer>(t) }\n\
    function integer_nfkc using v(t: Text[0, 64; nfkc]): Integer pure { convert<Integer>(t) }\n\
    function integer_nfc using v(t: Text[0, 64; nfc]): Integer pure { convert<Integer>(t) }\n\
    function integer_string using v(t: String): Integer pure { convert<Integer>(t) }\n\
    function bounded_integer_text using v(t: Text[0, 64; unicode-scalars]): Int[0, 99] pure { convert<Int[0, 99]>(t) }\n\
    function decimal_text using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function decimal_string using v(t: String): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function decimal_exact using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; exact] pure { convert<Decimal[0, 1000; 0, 2; exact]>(t) }\n\
    function rational_text using v(t: Text[0, 64; unicode-scalars]): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function rational_string using v(t: String): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function float_text using v(t: Text[0, 64; unicode-scalars]): Float64[nearest-even] pure { convert<Float64[nearest-even]>(t) }\n\
    function float_string using v(t: String): Float64[nearest-even] pure { convert<Float64[nearest-even]>(t) }\n\
    function float_exact using v(t: Text[0, 64; unicode-scalars]): Float64 pure { convert<Float64>(t) }\n\
    function float32_text using v(t: Text[0, 64; unicode-scalars]): Float32[nearest-even] pure { convert<Float32[nearest-even]>(t) }\n\
    function float64_zero using v(t: Text[0, 64; unicode-scalars]): Float64[toward-zero] pure { convert<Float64[toward-zero]>(t) }\n\
    function float64_positive using v(t: Text[0, 64; unicode-scalars]): Float64[toward-positive] pure { convert<Float64[toward-positive]>(t) }\n\
    function float32_zero using v(t: Text[0, 64; unicode-scalars]): Float32[toward-zero] pure { convert<Float32[toward-zero]>(t) }\n\
    function quantity_text using v(t: Text[0, 64; unicode-scalars]): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n\
    function quantity_string using v(t: String): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n";

fn package() -> CheckedPackage {
    super::quantity_literals::package_with(FUNCTIONS)
        .unwrap_or_else(|refusals| panic!("the text conversion package checks: {refusals:?}"))
}

fn text_value(text: &str) -> Value {
    text_value_with_profile(text, TextProfile::UnicodeScalars)
}

fn text_value_with_profile(text: &str, profile: TextProfile) -> Value {
    let ty = TextType::new(0, 64, profile).expect("valid text type");
    let admitted = admit_text(
        &TextPayload::from_utf8(text.as_bytes()).expect("valid UTF-8"),
        &ty,
        &mut Meter::new(super::quantity_literals::UNLIMITED),
    )
    .completed()
    .expect("text is in bounds");
    Value::Text(admitted)
}

fn run_with_limits(
    package: &CheckedPackage,
    name: &str,
    argument: Value,
    limits: ScalarLimits,
) -> (Outcome<Value>, Meter) {
    let (outcome, _, meter) = run_with_details(package, name, argument, limits);
    (outcome, meter)
}

fn run_with_details(
    package: &CheckedPackage,
    name: &str,
    argument: Value,
    limits: ScalarLimits,
) -> (Outcome<Value>, Vec<ValueLoss>, Meter) {
    let mut meter = Meter::new(limits);
    let result = package
        .call(
            &QualifiedName::unqualified(name).expect("function name"),
            vec![argument],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|error| panic!("{name} evaluates: {error:?}"));
    let FamilyOutcome::Evaluated(outcome) = result.outcome else {
        panic!("ordinary evaluation outcome, not {:?}", result.outcome);
    };
    let losses = result.losses.into_iter().map(|each| each.loss).collect();
    (outcome, losses, meter)
}

fn run(package: &CheckedPackage, name: &str, argument: Value) -> Outcome<Value> {
    run_with_limits(package, name, argument, super::quantity_literals::UNLIMITED).0
}

/// FR-478-AC-1/-3/-7/-8/-10: each target family accepts both a profiled Text
/// and an unnormalized String and returns the target's exact or rounded value.
#[trace(
    "FSpec-FR-478-AC-1",
    "FSpec-FR-478-AC-3",
    "FSpec-FR-478-AC-7",
    "FSpec-FR-478-AC-8",
    "FSpec-FR-478-AC-10"
)]
#[test]
fn text_and_string_convert_to_each_numeric_target() {
    let package = package();
    for (name, input, expected) in [
        ("integer_text", text_value("0"), 0_i64),
        ("integer_text", text_value("42"), 42_i64),
        ("integer_string", Value::String("-7".to_owned()), -7_i64),
    ] {
        let Outcome::Completed(Value::Integer(value)) = run(&package, name, input) else {
            panic!("{name} returns an integer");
        };
        assert_eq!(value, Integer::from(expected));
    }
    let Outcome::Completed(Value::Integer(large)) = run(
        &package,
        "integer_text",
        text_value("170141183460469231731687303715884105728"),
    ) else {
        panic!("large integer text remains exact");
    };
    assert_eq!(
        large,
        Integer::from_str("170141183460469231731687303715884105728")
            .expect("the exact integer spelling parses")
    );
    for (name, input, expected) in [
        ("decimal_text", text_value("1.5"), (15_i64, 1_u32)),
        (
            "decimal_string",
            Value::String("1.50".to_owned()),
            (150_i64, 2_u32),
        ),
    ] {
        let Outcome::Completed(Value::Decimal(value)) = run(&package, name, input) else {
            panic!("{name} returns a decimal");
        };
        assert_eq!(
            value.representation(),
            &DecimalRepresentation::new(Integer::from(expected.0), expected.1)
        );
    }
    for (name, input) in [
        ("rational_text", text_value("2/4")),
        ("rational_string", Value::String("0.25".to_owned())),
    ] {
        let Outcome::Completed(Value::Rational(value)) = run(&package, name, input) else {
            panic!("{name} returns a rational");
        };
        let expected = if name == "rational_text" {
            Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap()
        } else {
            Rational::new(Integer::from(1_i64), Integer::from(4_i64)).unwrap()
        };
        assert_eq!(value, expected);
    }
    for (name, input) in [
        ("float_text", text_value("0.25")),
        ("float_string", Value::String("0.1".to_owned())),
    ] {
        let Outcome::Completed(Value::Float(value)) = run(&package, name, input) else {
            panic!("{name} returns a float");
        };
        let expected = if name == "float_text" {
            0x3fd0_0000_0000_0000
        } else {
            0x3fb9_9999_9999_999a
        };
        assert_eq!(value.bits(), expected);
    }
    for (name, input) in [
        ("quantity_text", text_value("3 ms")),
        ("quantity_string", Value::String("3".to_owned())),
    ] {
        let Outcome::Completed(Value::Quantity(value)) = run(&package, name, input) else {
            panic!("{name} returns a quantity");
        };
        assert_eq!(
            value.magnitude(),
            &Rational::from_integer(Integer::from(3_i64))
        );
    }
}

/// FR-478-AC-2/-4/-7/-9/-10: malformed spellings and mismatched unit names
/// are undefined; bounded values outside their declared domain are refused.
#[trace(
    "FSpec-FR-478-AC-2",
    "FSpec-FR-478-AC-4",
    "FSpec-FR-478-AC-7",
    "FSpec-FR-478-AC-9",
    "FSpec-FR-478-AC-10"
)]
#[test]
fn malformed_out_of_domain_and_wrong_unit_outcomes_are_typed() {
    let package = package();
    for text in ["", "+5", "05", "-0", " 5", "1e3", "1.0", "2/1", "٥"] {
        assert!(
            matches!(
                run(&package, "integer_text", text_value(text)),
                Outcome::Undefined(_)
            ),
            "{text:?}"
        );
    }
    let Outcome::Completed(Value::Integer(normalized)) = run(
        &package,
        "integer_nfkc",
        text_value_with_profile("５", TextProfile::Nfkc),
    ) else {
        panic!("NFKC fullwidth digit normalizes to an integer");
    };
    assert_eq!(normalized, Integer::from(5_i64));
    assert!(matches!(
        run(
            &package,
            "integer_nfc",
            text_value_with_profile("５", TextProfile::Nfc)
        ),
        Outcome::Undefined(_)
    ));
    assert!(matches!(
        run(&package, "integer_string", Value::String("５".to_owned())),
        Outcome::Undefined(_)
    ));
    for text in [".5", "5.", "-0.00", "1.2.3", "1,5", "1e3", "3/4"] {
        assert!(
            matches!(
                run(&package, "decimal_text", text_value(text)),
                Outcome::Undefined(_)
            ),
            "decimal {text:?}"
        );
    }
    for text in ["1/0", "1/-2", "-0/3", "+1/2", "01/2", "1/02", "1e1"] {
        assert!(
            matches!(
                run(&package, "rational_text", text_value(text)),
                Outcome::Undefined(_)
            ),
            "rational {text:?}"
        );
    }
    for text in [
        "1E3", "1e+3", "1e03", "1e-0", ".5", "5.", "1e", "Infinity", "NaN", "-nan", "+inf", "0x1p3",
    ] {
        assert!(
            matches!(
                run(&package, "float_text", text_value(text)),
                Outcome::Undefined(_)
            ),
            "float {text:?}"
        );
    }
    assert!(matches!(
        run(&package, "bounded_integer_text", text_value("100")),
        Outcome::Refused(Refusal::IntegerOutOfDomain { .. })
    ));
    assert!(matches!(
        run(&package, "bounded_integer_text", text_value("-1")),
        Outcome::Refused(Refusal::IntegerOutOfDomain { .. })
    ));
    assert!(matches!(
        run(&package, "rational_text", text_value("1/10")),
        Outcome::Refused(Refusal::RationalOutOfDomain { .. })
    ));
    assert!(matches!(
        run(&package, "decimal_exact", text_value("10.01")),
        Outcome::Refused(Refusal::DecimalOutOfDomain { .. })
    ));
    assert!(matches!(
        run(&package, "quantity_text", text_value("3 s")),
        Outcome::Undefined(_)
    ));
    for text in ["3ms", "3  ms", "3 MS", "ms"] {
        assert!(
            matches!(
                run(&package, "quantity_text", text_value(text)),
                Outcome::Undefined(_)
            ),
            "quantity {text:?}"
        );
    }
    for text in ["0.5", "2/4 ms"] {
        let Outcome::Completed(Value::Quantity(value)) =
            run(&package, "quantity_text", text_value(text))
        else {
            panic!("{text:?} converts to an exact quantity");
        };
        assert_eq!(
            value.magnitude(),
            &Rational::new(Integer::from(1_i64), Integer::from(2_i64))
                .expect("one half has a nonzero denominator")
        );
    }
}

/// FR-478-AC-6/-8/-9: decimal targets retain exact spellings or round only
/// when needed; IEEE targets preserve special values and enforce exact mode.
#[trace("FSpec-FR-478-AC-6", "FSpec-FR-478-AC-8", "FSpec-FR-478-AC-9")]
#[test]
fn decimal_and_ieee_targets_apply_their_declared_modes() {
    let package = package();
    for (spelling, expected) in [("1.5", (15_i64, 1_u32)), ("1.500", (150_i64, 2_u32))] {
        let Outcome::Completed(Value::Decimal(value)) =
            run(&package, "decimal_exact", text_value(spelling))
        else {
            panic!("{spelling:?} is exactly representable");
        };
        assert_eq!(
            value.representation(),
            &DecimalRepresentation::new(Integer::from(expected.0), expected.1)
        );
    }
    assert!(matches!(
        run(&package, "decimal_exact", text_value("1.555")),
        Outcome::Refused(Refusal::InexactDecimal { .. })
    ));
    let (rounded_outcome, rounded_losses, _) = run_with_details(
        &package,
        "decimal_text",
        text_value("1.555"),
        super::quantity_literals::UNLIMITED,
    );
    let Outcome::Completed(Value::Decimal(rounded)) = rounded_outcome else {
        panic!("nearest-even decimal rounds");
    };
    assert_eq!(
        rounded.representation(),
        &DecimalRepresentation::new(Integer::from(156_i64), 2)
    );
    let [ValueLoss::Decimal(decimal_loss)] = rounded_losses.as_slice() else {
        panic!("one decimal loss record, not {rounded_losses:?}");
    };
    assert_eq!(decimal_loss.exact_numerator(), &Integer::from(311_i64));
    assert_eq!(decimal_loss.exact_denominator(), Integer::from(200_i64));
    assert_eq!(decimal_loss.rounded_coefficient(), &Integer::from(156_i64));
    assert_eq!(decimal_loss.rounded_scale(), 2);
    assert_eq!(decimal_loss.mode().as_str(), "nearest-even");
    let Outcome::Completed(Value::Decimal(tie)) =
        run(&package, "decimal_text", text_value("1.565"))
    else {
        panic!("nearest-even decimal rounds ties");
    };
    assert_eq!(
        tie.representation(),
        &DecimalRepresentation::new(Integer::from(156_i64), 2)
    );

    let Outcome::Completed(Value::Float(exact)) = run(&package, "float_exact", text_value("0.25"))
    else {
        panic!("0.25 is exact as Float64");
    };
    assert_eq!(exact.bits(), 0x3fd0_0000_0000_0000);
    let Outcome::Refused(Refusal::IeeeNotExact {
        would_be: inexact_flags,
        ..
    }) = run(&package, "float_exact", text_value("0.1"))
    else {
        panic!("strict exact refuses inexact 0.1");
    };
    assert!(inexact_flags.contains(filament_exact::IeeeFlag::Inexact));
    let Outcome::Refused(Refusal::IeeeNotExact {
        would_be: overflow_flags,
        ..
    }) = run(&package, "float_exact", text_value("1e309"))
    else {
        panic!("strict exact refuses overflowing 1e309");
    };
    assert!(overflow_flags.contains(filament_exact::IeeeFlag::Overflow));
    assert!(overflow_flags.contains(filament_exact::IeeeFlag::Inexact));
    let (inexact_float, float_losses, _) = run_with_details(
        &package,
        "float_text",
        text_value("0.1"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(inexact_float, Outcome::Completed(Value::Float(_))));
    let [ValueLoss::IeeeFlags(flags)] = float_losses.as_slice() else {
        panic!("one inexact flag loss, not {float_losses:?}");
    };
    assert_eq!(
        flags.iter().collect::<Vec<_>>(),
        [filament_exact::IeeeFlag::Inexact]
    );
    for (name, spelling, bits) in [
        ("float_text", "inf", 0x7ff0_0000_0000_0000),
        ("float_text", "nan", 0x7ff8_0000_0000_0000),
        ("float_text", "1e309", 0x7ff0_0000_0000_0000),
        ("float_text", "-0", 0x8000_0000_0000_0000),
        ("float_text", "1e-400", 0),
        ("float32_text", "0.1", 0x0000_0000_3dcc_cccd),
    ] {
        let Outcome::Completed(Value::Float(value)) = run(&package, name, text_value(spelling))
        else {
            panic!("{spelling:?} converts to a float");
        };
        assert_eq!(value.bits(), bits, "{name} parses {spelling}");
    }
    let Outcome::Completed(Value::Float(toward_zero)) =
        run(&package, "float64_zero", text_value("0.1"))
    else {
        panic!("toward-zero Float64 converts");
    };
    assert_eq!(toward_zero.bits(), 0x3fb9_9999_9999_9999);
    let Outcome::Completed(Value::Float(toward_positive)) =
        run(&package, "float64_positive", text_value("1e-400"))
    else {
        panic!("toward-positive Float64 converts");
    };
    assert_eq!(toward_positive.bits(), 1);
    let Outcome::Completed(Value::Float(float32_zero)) =
        run(&package, "float32_zero", text_value("0.1"))
    else {
        panic!("toward-zero Float32 converts");
    };
    assert_eq!(float32_zero.bits(), 0x3dcc_cccc);
}

/// FR-478-AC-5/-6/-8/-9/-11: conversion charges the operand first, then only
/// the target-specific rounding point when the grammar and value require it.
#[trace(
    "FSpec-FR-478-AC-5",
    "FSpec-FR-478-AC-6",
    "FSpec-FR-478-AC-8",
    "FSpec-FR-478-AC-9",
    "FSpec-FR-478-AC-11"
)]
#[test]
fn conversion_charges_follow_operand_then_target_order() {
    let package = package();
    let (integer, meter) = run_with_limits(
        &package,
        "integer_text",
        text_value("42"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(integer, Outcome::Completed(Value::Integer(_))));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
        ]
    );

    let (string_integer, meter) = run_with_limits(
        &package,
        "integer_string",
        Value::String("42".to_owned()),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(
        string_integer,
        Outcome::Completed(Value::Integer(_))
    ));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
        ]
    );

    let (normalized_integer, meter) = run_with_limits(
        &package,
        "integer_nfkc",
        text_value_with_profile("５", TextProfile::Nfkc),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(
        normalized_integer,
        Outcome::Completed(Value::Integer(_))
    ));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::TextNormalizeInput,
            ChargePoint::TextNormalizeOutput,
        ]
    );

    let (rounded_decimal, meter) = run_with_limits(
        &package,
        "decimal_text",
        text_value("1.555"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(
        rounded_decimal,
        Outcome::Completed(Value::Decimal(_))
    ));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::DecimalRounding,
        ]
    );

    let (exact_decimal, meter) = run_with_limits(
        &package,
        "decimal_exact",
        text_value("1.555"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(
        exact_decimal,
        Outcome::Refused(Refusal::InexactDecimal { .. })
    ));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
        ]
    );

    let (exact_float, meter) = run_with_limits(
        &package,
        "float_exact",
        text_value("0.25"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(exact_float, Outcome::Completed(Value::Float(_))));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::IeeeRound,
        ]
    );

    let (strict_inexact, meter) = run_with_limits(
        &package,
        "float_exact",
        text_value("0.1"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(
        strict_inexact,
        Outcome::Refused(Refusal::IeeeNotExact { .. })
    ));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::IeeeRound,
        ]
    );

    let (special_float, meter) = run_with_limits(
        &package,
        "float_text",
        text_value("inf"),
        super::quantity_literals::UNLIMITED,
    );
    assert!(matches!(special_float, Outcome::Completed(Value::Float(_))));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
        ]
    );

    let mut denied_bytes = super::quantity_literals::UNLIMITED;
    denied_bytes.text_input_bytes = 0;
    let (denied, meter) = run_with_limits(&package, "integer_text", text_value("42"), denied_bytes);
    assert!(matches!(&denied, Outcome::Incomplete(_)));
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>(),
        [ChargePoint::FunctionCall]
    );
    let incomplete = match &denied {
        Outcome::Incomplete(incomplete) => incomplete,
        _ => unreachable!(),
    };
    assert_eq!(incomplete.limit_kind, LimitKind::TextInputBytes);
}
