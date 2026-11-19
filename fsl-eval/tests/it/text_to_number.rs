// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2241 / FSpec FR-478: checked text-to-number conversion at evaluation.

use std::str::FromStr;

use filament_eval::{Evaluation, FamilyOutcome};
use filament_exact::{
    admit_text, ChargePoint, IeeeFlag, Integer, Meter, Outcome, Rational, Refusal, RoundingMode,
    ScalarLimits, TextProfile, TextType, Undefined, Value,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::loss::ValueLoss;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRef, PackageDimensionRecord, PackageUnitRecord,
};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{admit_ieee_profile, DefinitionRole};
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

const DECLARATIONS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    function convert_integer using v(t: Text[0, 64; unicode-scalars]): Integer pure { convert<Integer>(t) }\n\
    function convert_bounded using v(t: Text[0, 64; unicode-scalars]): Int[0, 99] pure { convert<Int[0, 99]>(t) }\n\
    function convert_decimal_wide using v(t: Text[0, 64; unicode-scalars]): Decimal[-1000, 1000; 0, 18; nearest-even] pure { convert<Decimal[-1000, 1000; 0, 18; nearest-even]>(t) }\n\
    function convert_rational using v(t: Text[0, 64; unicode-scalars]): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function convert_rational_wide using v(t: Text[0, 64; unicode-scalars]): Rational[-1000, 1000; 1, 1000] pure { convert<Rational[-1000, 1000; 1, 1000]>(t) }\n\
    function convert_decimal using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function convert_decimal_exact using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; exact] pure { convert<Decimal[0, 1000; 0, 2; exact]>(t) }\n\
    function convert_float using v(t: Text[0, 64; unicode-scalars]): Float64[nearest-even] pure { convert<Float64[nearest-even]>(t) }\n\
    function convert_float_zero using v(t: Text[0, 64; unicode-scalars]): Float64[toward-zero] pure { convert<Float64[toward-zero]>(t) }\n\
    function convert_float_positive using v(t: Text[0, 64; unicode-scalars]): Float64[toward-positive] pure { convert<Float64[toward-positive]>(t) }\n\
    function convert_float32 using v(t: Text[0, 64; unicode-scalars]): Float32[nearest-even] pure { convert<Float32[nearest-even]>(t) }\n\
    function convert_float32_zero using v(t: Text[0, 64; unicode-scalars]): Float32[toward-zero] pure { convert<Float32[toward-zero]>(t) }\n\
    function convert_float_exact using v(t: Text[0, 64; unicode-scalars]): Float64 pure { convert<Float64>(t) }\n\
    function convert_quantity using v(t: Text[0, 64; unicode-scalars]): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n\
    function convert_nfkc using v(t: Text[0, 64; nfkc]): Integer pure { convert<Integer>(t) }\n\
    function convert_nfc using v(t: Text[0, 64; nfc]): Integer pure { convert<Integer>(t) }\n\
    function convert_string_integer using v(s: String): Integer pure { convert<Integer>(s) }\n\
    function convert_string_decimal using v(s: String): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(s) }\n\
    function convert_string_rational using v(s: String): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(s) }\n\
    function convert_string_float using v(s: String): Float64[nearest-even] pure { convert<Float64[nearest-even]>(s) }\n\
    function convert_string_quantity using v(s: String): Quantity<ms> pure { convert<Quantity<ms>>(s) }\
";

fn compile(source: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    compile_with_models(source, SelectedModels::default())
}

fn compile_with_models(
    source: &str,
    selected: SelectedModels,
) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{source}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2241", "text-to-number"),
        "text-to-number.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the test unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the test unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qsl-2241-runtime-tests".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the package declarations assemble");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the IEEE profile is admitted"),
    );
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

fn package() -> CheckedPackage {
    compile(DECLARATIONS).unwrap_or_else(|refusals| panic!("fixture checks: {refusals:?}"))
}

fn text_value(text: &str, profile: TextProfile) -> Value {
    let text_type = TextType::new(0, 64, profile).expect("valid Text bounds");
    Value::Text(
        admit_text(
            &filament_exact::TextPayload::from_utf8(text.as_bytes()).expect("valid UTF-8"),
            &text_type,
            &mut Meter::new(LIMITS),
        )
        .completed()
        .expect("text is inside its bounds"),
    )
}

fn call(package: &CheckedPackage, name: &str, value: Value) -> Evaluation {
    call_with_meter(package, name, value, LIMITS).0
}

fn call_with_meter(
    package: &CheckedPackage,
    name: &str,
    value: Value,
    limits: ScalarLimits,
) -> (Evaluation, Meter) {
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("function name is an identifier"),
            vec![value],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("function evaluation runs");
    (evaluation, meter)
}

fn charge_points(meter: &Meter) -> Vec<ChargePoint> {
    meter
        .admitted_charges()
        .iter()
        .map(|charge| charge.point)
        .collect()
}

fn completed(evaluation: &Evaluation) -> &Value {
    let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome else {
        panic!(
            "expected completed conversion, got {:?}",
            evaluation.outcome
        );
    };
    value
}

fn assert_integer(evaluation: &Evaluation, expected: Integer) {
    let Value::Integer(value) = completed(evaluation) else {
        panic!("expected integer, got {:?}", evaluation.outcome);
    };
    assert_eq!(value, &expected);
}

fn assert_rational(evaluation: &Evaluation, expected: Rational) {
    let Value::Rational(value) = completed(evaluation) else {
        panic!("expected rational, got {:?}", evaluation.outcome);
    };
    assert_eq!(value, &expected);
}

fn undefined(evaluation: &Evaluation, reason: Undefined) {
    assert!(matches!(
        &evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Undefined(found)) if found == &reason
    ));
}

fn ieee_flags(evaluation: &Evaluation) -> Vec<IeeeFlag> {
    evaluation
        .losses
        .iter()
        .find_map(|loss| match &loss.loss {
            ValueLoss::IeeeFlags(flags) => Some(flags.iter().collect()),
            ValueLoss::Decimal(_) | ValueLoss::IeeeExact(_) => None,
        })
        .unwrap_or_default()
}

fn assert_incomplete_at(evaluation: &Evaluation, expected: ChargePoint) {
    let FamilyOutcome::Evaluated(Outcome::Incomplete(record)) = &evaluation.outcome else {
        panic!(
            "expected incomplete at {expected:?}, got {:?}",
            evaluation.outcome
        );
    };
    assert_eq!(record.charge_point, expected);
}

#[trace(
    "FSpec-TC-964",
    "FSpec-FR-478-AC-1",
    "FSpec-FR-478-AC-6",
    "FSpec-FR-478-AC-7"
)]
#[test]
fn exact_numeric_targets_read_their_whole_ascii_grammar() {
    let package = package();
    for (text, expected) in [
        ("0", Integer::zero()),
        ("42", Integer::from(42_i64)),
        ("-7", Integer::from(-7_i64)),
        (
            "170141183460469231731687303715884105728",
            Integer::from_str("170141183460469231731687303715884105728").unwrap(),
        ),
    ] {
        let evaluation = call(
            &package,
            "convert_integer",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert_integer(&evaluation, expected);
    }

    let negative_bounded = call(
        &package,
        "convert_bounded",
        text_value("-1", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        negative_bounded.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { .. }))
    ));

    for (text, coefficient, scale) in [("1.50", 150_i64, 2), ("-0.5", -5_i64, 1), ("3", 3_i64, 0)] {
        let decimal = call(
            &package,
            "convert_decimal_wide",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(matches!(
            completed(&decimal),
            Value::Decimal(value)
                if value.representation().coefficient() == &Integer::from(coefficient)
                    && value.representation().scale() == scale
        ));
    }

    let rational = call(
        &package,
        "convert_rational",
        text_value("2/4", TextProfile::UnicodeScalars),
    );
    assert_rational(
        &rational,
        Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap(),
    );
    let wide_rational = call(
        &package,
        "convert_rational_wide",
        text_value("6/4", TextProfile::UnicodeScalars),
    );
    assert_rational(
        &wide_rational,
        Rational::new(Integer::from(3_i64), Integer::from(2_i64)).unwrap(),
    );

    let decimal = call(
        &package,
        "convert_decimal",
        text_value("1.50", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&decimal), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(150_i64) && value.representation().scale() == 2)
    );
    let decimal = call(
        &package,
        "convert_decimal",
        text_value("1.5", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&decimal), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(15_i64) && value.representation().scale() == 1)
    );
    let rounded = call(
        &package,
        "convert_decimal",
        text_value("1.555", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&rounded), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(156_i64) && value.representation().scale() == 2)
    );
    assert!(matches!(
        rounded.losses.as_slice(),
        [loss] if matches!(&loss.loss, ValueLoss::Decimal(decimal_loss)
            if decimal_loss.exact() == Rational::new(Integer::from(311_i64), Integer::from(200_i64)).unwrap()
                && decimal_loss.mode() == RoundingMode::NearestEven)
    ));

    let strict_inexact = call(
        &package,
        "convert_decimal_exact",
        text_value("1.555", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        &strict_inexact.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::InexactDecimal { .. }))
    ));
    let strict_zeroes = call(
        &package,
        "convert_decimal_exact",
        text_value("1.500", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&strict_zeroes),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(150_i64) && value.representation().scale() == 2
    ));
    let strict_retained = call(
        &package,
        "convert_decimal_exact",
        text_value("1.5", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&strict_retained),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(15_i64) && value.representation().scale() == 1
    ));
    let strict_outside = call(
        &package,
        "convert_decimal_exact",
        text_value("10.01", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        strict_outside.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::DecimalOutOfDomain { .. }))
    ));
    let tie = call(
        &package,
        "convert_decimal",
        text_value("1.565", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&tie),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(156_i64) && value.representation().scale() == 2
    ));
}

#[trace(
    "FSpec-TC-964",
    "FSpec-FR-478-AC-4",
    "FSpec-FR-478-AC-5",
    "FSpec-FR-478-AC-6",
    "FSpec-FR-478-AC-9",
    "FSpec-FR-478-AC-11"
)]
#[test]
fn conversion_charges_input_then_target_work_and_stops_on_denial() {
    let package = package();
    let (integer, meter) = call_with_meter(
        &package,
        "convert_integer",
        text_value("42", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert_integer(&integer, Integer::from(42_i64));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars
        ]
    );

    let (malformed, meter) = call_with_meter(
        &package,
        "convert_integer",
        text_value("abc", TextProfile::UnicodeScalars),
        LIMITS,
    );
    undefined(&malformed, Undefined::MalformedNumber);
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars
        ],
        "grammar failure follows input and decode charges"
    );

    let (outside, meter) = call_with_meter(
        &package,
        "convert_bounded",
        text_value("100", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert!(matches!(
        outside.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { .. }))
    ));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars
        ],
        "out-of-domain refusal follows input and decode charges"
    );

    let (rounded, meter) = call_with_meter(
        &package,
        "convert_decimal",
        text_value("1.555", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert!(matches!(
        meter.admitted_charges().last(),
        Some(charge)
            if charge.point == ChargePoint::DecimalRounding
                && charge.integer_bits == Some(11)
                && charge.decimal_digits == Some(4)
    ));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::DecimalRounding
        ]
    );
    assert!(matches!(completed(&rounded), Value::Decimal(_)));

    let (strict, meter) = call_with_meter(
        &package,
        "convert_decimal_exact",
        text_value("1.555", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert!(matches!(
        strict.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::InexactDecimal { .. }))
    ));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars
        ],
        "strict exact refuses before the decimal.rounding charge"
    );

    let (rounded_float, meter) = call_with_meter(
        &package,
        "convert_float",
        text_value("0.1", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::IeeeRound
        ]
    );
    assert_eq!(
        meter
            .admitted_charges()
            .last()
            .and_then(|charge| charge.integer_bits),
        Some(64)
    );
    assert!(matches!(completed(&rounded_float), Value::Float(_)));
    let (infinity, meter) = call_with_meter(
        &package,
        "convert_float",
        text_value("inf", TextProfile::UnicodeScalars),
        LIMITS,
    );
    assert!(matches!(completed(&infinity), Value::Float(_)));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars
        ],
        "infinity does not charge ieee.round"
    );

    let (string_float, meter) = call_with_meter(
        &package,
        "convert_string_float",
        Value::String("0.1".to_owned()),
        LIMITS,
    );
    assert!(matches!(completed(&string_float), Value::Float(_)));
    assert_eq!(
        charge_points(&meter),
        [
            ChargePoint::FunctionCall,
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::IeeeRound
        ]
    );
    assert!(meter.admitted_charges().iter().all(|charge| !matches!(
        charge.point,
        ChargePoint::TextNormalizeInput | ChargePoint::TextNormalizeOutput
    )));

    for (name, value, limits, denied) in [
        (
            "convert_integer",
            text_value("42", TextProfile::UnicodeScalars),
            ScalarLimits {
                text_input_bytes: 0,
                ..LIMITS
            },
            ChargePoint::TextInputBytes,
        ),
        (
            "convert_decimal",
            text_value("1.555", TextProfile::UnicodeScalars),
            ScalarLimits {
                decimal_digits: 3,
                ..LIMITS
            },
            ChargePoint::DecimalRounding,
        ),
        (
            "convert_float",
            text_value("0.1", TextProfile::UnicodeScalars),
            ScalarLimits {
                integer_bits: 63,
                ..LIMITS
            },
            ChargePoint::IeeeRound,
        ),
    ] {
        let (evaluation, meter) = call_with_meter(&package, name, value, limits);
        assert_incomplete_at(&evaluation, denied);
        assert_eq!(
            charge_points(&meter).last().copied(),
            Some(match denied {
                ChargePoint::TextInputBytes => ChargePoint::FunctionCall,
                ChargePoint::DecimalRounding | ChargePoint::IeeeRound => {
                    ChargePoint::TextDecodeScalars
                }
                _ => unreachable!("the expected denial point is explicit"),
            }),
            "a denied charge is not recorded as admitted"
        );
    }
}

#[trace(
    "FSpec-TC-964",
    "FSpec-FR-478-AC-2",
    "FSpec-FR-478-AC-7",
    "FSpec-FR-478-AC-11"
)]
#[test]
fn malformed_numeric_text_is_undefined_and_profile_normalization_is_used() {
    let package = package();
    for text in ["", "+5", "05", "-0", " 5", "1e3", "٣", "５"] {
        undefined(
            &call(
                &package,
                "convert_integer",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
    assert_integer(
        &call(
            &package,
            "convert_nfkc",
            text_value("５", TextProfile::Nfkc),
        ),
        Integer::from(5_i64),
    );
    undefined(
        &call(&package, "convert_nfc", text_value("５", TextProfile::Nfc)),
        Undefined::MalformedNumber,
    );
    undefined(
        &call(
            &package,
            "convert_string_integer",
            Value::String("５".to_owned()),
        ),
        Undefined::MalformedNumber,
    );
    assert_integer(
        &call(
            &package,
            "convert_string_integer",
            Value::String("42".to_owned()),
        ),
        Integer::from(42_i64),
    );
    assert!(matches!(
        completed(&call(
            &package,
            "convert_string_rational",
            Value::String("2/4".to_owned()),
        )),
        Value::Rational(value) if value == &Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap()
    ));
    assert!(matches!(
        completed(&call(
            &package,
            "convert_string_decimal",
            Value::String("1.50".to_owned()),
        )),
        Value::Decimal(value)
            if value.representation().coefficient() == &Integer::from(150_i64)
                && value.representation().scale() == 2
    ));
    assert!(matches!(
        completed(&call(
            &package,
            "convert_string_float",
            Value::String("0.1".to_owned()),
        )),
        Value::Float(value) if value.bits() == 0x3fb9_9999_9999_999a
    ));
    assert!(matches!(
        completed(&call(
            &package,
            "convert_string_quantity",
            Value::String("3 ms".to_owned()),
        )),
        Value::Quantity(value)
            if value.magnitude() == &Rational::from_integer(Integer::from(3_i64))
    ));
    undefined(
        &call(
            &package,
            "convert_string_float",
            Value::String("５".to_owned()),
        ),
        Undefined::MalformedNumber,
    );
    for text in [".5", "5.", "-0.00", "1.2.3", "1,5", "1e3", "3/4"] {
        undefined(
            &call(
                &package,
                "convert_decimal_exact",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
    for text in [
        "1/0", "1/-2", "-0/3", "+1/2", "1 / 2", "01/2", "1/02", "1.5/2", "1e1",
    ] {
        undefined(
            &call(
                &package,
                "convert_rational",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}

#[trace(
    "FSpec-TC-964",
    "FSpec-FR-478-AC-1",
    "FSpec-FR-478-AC-7",
    "FSpec-FR-478-AC-10"
)]
#[test]
fn range_and_quantity_unit_outcomes_are_typed() {
    let package = package();
    let outside = call(
        &package,
        "convert_bounded",
        text_value("100", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        outside.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { .. }))
    ));
    for text in ["1/10", "10"] {
        let outside = call(
            &package,
            "convert_rational",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(matches!(
            outside.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::RationalOutOfDomain { .. }))
        ));
    }

    let quantity = call(
        &package,
        "convert_quantity",
        text_value("3 ms", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&quantity), Value::Quantity(value) if value.magnitude() == &Rational::from_integer(Integer::from(3_i64)))
    );
    for text in ["0.5", "2/4 ms"] {
        let quantity = call(
            &package,
            "convert_quantity",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(
            matches!(completed(&quantity), Value::Quantity(value) if value.magnitude() == &Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap())
        );
    }

    for text in ["3 s", "3ms", "3  ms", "3 MS", "ms"] {
        undefined(
            &call(
                &package,
                "convert_quantity",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}

#[trace("FSpec-TC-964", "FSpec-FR-478-AC-10")]
#[test]
fn a_domain_package_quantity_uses_its_short_unit_name() {
    let time = filament_checked::key::DeclarationKey::fixture("dimension/Time");
    let ms = filament_checked::key::DeclarationKey::fixture("unit/ms");
    let mut model = DomainPackage::new(
        DomainPackageRef {
            identity: "acme/rpc".to_owned(),
        },
        vec![],
    );
    model.dimensions.push(PackageDimensionRecord {
        key: time.clone(),
        name: "Time".to_owned(),
        base: true,
        terms: vec![],
    });
    model.units.push(PackageUnitRecord {
        key: ms,
        name: "ms".to_owned(),
        dimension: time,
        target: None,
        scale: "1".to_owned(),
        offset: "0".to_owned(),
    });
    let selected = SelectedModels::fixture(
        "M",
        fsl_foundation::Span { start: 0, end: 1 },
        model,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the domain package units admit");
    let package = compile_with_models(
        "model M = \"acme/rpc\";\n\
         function package_quantity using v(t: Text[0, 64; unicode-scalars]): Quantity<M::ms> pure { convert<Quantity<M::ms>>(t) }",
        selected,
    )
    .unwrap_or_else(|refusals| panic!("package unit conversion checks: {refusals:?}"));
    let short_name = call(
        &package,
        "package_quantity",
        text_value("3 ms", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&short_name),
        Value::Quantity(value)
            if value.magnitude() == &Rational::from_integer(Integer::from(3_i64))
    ));
    undefined(
        &call(
            &package,
            "package_quantity",
            text_value("3 M::ms", TextProfile::UnicodeScalars),
        ),
        Undefined::MalformedNumber,
    );
}

#[trace("FSpec-TC-964", "FSpec-FR-478-AC-8", "FSpec-FR-478-AC-9")]
#[test]
fn finite_ieee_text_rounds_once_to_the_target_width() {
    let package = package();
    let evaluation = call(
        &package,
        "convert_float",
        text_value("0.1", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&evaluation), Value::Float(value) if value.bits() == 0x3fb9_9999_9999_999a)
    );
    assert!(matches!(
        evaluation.losses.as_slice(),
        [loss] if matches!(loss.loss, ValueLoss::IeeeFlags(flags) if flags.contains(IeeeFlag::Inexact))
    ));
    assert_eq!(ieee_flags(&evaluation), [IeeeFlag::Inexact]);
    let evaluation = call(
        &package,
        "convert_float",
        text_value("2.5e-1", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&evaluation), Value::Float(value) if value.bits() == 0x3fd0_0000_0000_0000)
    );
    for (text, bits) in [
        ("-0", 0x8000_0000_0000_0000),
        ("1e309", 0x7ff0_0000_0000_0000),
        ("1e-400", 0),
        ("inf", 0x7ff0_0000_0000_0000),
        ("-inf", 0xfff0_0000_0000_0000),
        ("nan", 0x7ff8_0000_0000_0000),
    ] {
        let evaluation = call(
            &package,
            "convert_float",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(
            matches!(completed(&evaluation), Value::Float(value) if value.bits() == bits),
            "unexpected bits for {text}: {:?}",
            evaluation.outcome
        );
    }
    let overflow = call(
        &package,
        "convert_float",
        text_value("1e309", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&overflow),
        Value::Float(value) if value.bits() == 0x7ff0_0000_0000_0000
    ));
    assert_eq!(
        ieee_flags(&overflow),
        [IeeeFlag::Overflow, IeeeFlag::Inexact]
    );
    let underflow = call(
        &package,
        "convert_float",
        text_value("1e-400", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&underflow),
        Value::Float(value) if value.bits() == 0
    ));
    assert_eq!(
        ieee_flags(&underflow),
        [IeeeFlag::Underflow, IeeeFlag::Inexact]
    );
    let toward_zero_overflow = call(
        &package,
        "convert_float_zero",
        text_value("1e309", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&toward_zero_overflow),
        Value::Float(value) if value.bits() == 0x7fef_ffff_ffff_ffff
    ));
    assert_eq!(
        ieee_flags(&toward_zero_overflow),
        [IeeeFlag::Overflow, IeeeFlag::Inexact]
    );
    let toward_zero_inexact = call(
        &package,
        "convert_float_zero",
        text_value("0.1", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&toward_zero_inexact),
        Value::Float(value) if value.bits() == 0x3fb9_9999_9999_9999
    ));
    assert_eq!(ieee_flags(&toward_zero_inexact), [IeeeFlag::Inexact]);
    let toward_positive_underflow = call(
        &package,
        "convert_float_positive",
        text_value("1e-400", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&toward_positive_underflow),
        Value::Float(value) if value.bits() == 1
    ));
    assert_eq!(
        ieee_flags(&toward_positive_underflow),
        [IeeeFlag::Underflow, IeeeFlag::Inexact]
    );
    for (name, bits) in [
        ("convert_float32", 0x3dcc_cccd),
        ("convert_float32_zero", 0x3dcc_cccc),
    ] {
        let result = call(
            &package,
            name,
            text_value("0.1", TextProfile::UnicodeScalars),
        );
        assert!(matches!(
            completed(&result),
            Value::Float(value) if value.bits() == bits
        ));
        assert_eq!(ieee_flags(&result), [IeeeFlag::Inexact], "{name}");
    }
    let exact_finite = call(
        &package,
        "convert_float_exact",
        text_value("0.25", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&exact_finite), Value::Float(value) if value.bits() == 0x3fd0_0000_0000_0000)
    );
    let exact_inexact = call(
        &package,
        "convert_float_exact",
        text_value("0.1", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        exact_inexact.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IeeeNotExact {
            would_be,
            ..
        })) if would_be.iter().collect::<Vec<_>>() == [IeeeFlag::Inexact]
    ));
    let exact_overflow = call(
        &package,
        "convert_float_exact",
        text_value("1e309", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        exact_overflow.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IeeeNotExact {
            would_be,
            ..
        })) if would_be.iter().collect::<Vec<_>>() == [IeeeFlag::Overflow, IeeeFlag::Inexact]
    ));
    for text in [
        "1E3", "1e+3", "1e03", "1e-0", ".5", "5.", "1e", "Infinity", "NaN", "-nan", "+inf", "0x1p3",
    ] {
        undefined(
            &call(
                &package,
                "convert_float",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}
