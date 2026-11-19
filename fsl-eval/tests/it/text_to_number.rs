// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2241 / FSpec FR-478: checked text-to-number conversion at evaluation.

use std::str::FromStr;

use filament_eval::{Evaluation, FamilyOutcome};
use filament_exact::{
    Integer, Meter, Outcome, Rational, Refusal, ScalarLimits, TextProfile, TextType, Undefined,
    Value, admit_text,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::loss::ValueLoss;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{DefinitionRole, admit_ieee_profile};
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
    function integer using v(t: Text[0, 64; unicode-scalars]): Integer pure { convert<Integer>(t) }\n\
    function bounded using v(t: Text[0, 64; unicode-scalars]): Int[0, 99] pure { convert<Int[0, 99]>(t) }\n\
    function rational using v(t: Text[0, 64; unicode-scalars]): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function decimal using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function decimal_exact using v(t: Text[0, 64; unicode-scalars]): Decimal[0, 1000; 0, 2; exact] pure { convert<Decimal[0, 1000; 0, 2; exact]>(t) }\n\
    function float using v(t: Text[0, 64; unicode-scalars]): Float64[nearest-even] pure { convert<Float64[nearest-even]>(t) }\n\
    function quantity using v(t: Text[0, 64; unicode-scalars]): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n\
    function nfkc using v(t: Text[0, 64; nfkc]): Integer pure { convert<Integer>(t) }\n\
    function nfc using v(t: Text[0, 64; nfc]): Integer pure { convert<Integer>(t) }\n\
    function string_integer using v(s: String): Integer pure { convert<Integer>(s) }\
";

fn compile(source: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
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
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the package declarations assemble");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the IEEE profile is admitted"),
    );
    declarations
        .check(CheckMode::Linked, CheckingLimits::default())
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
    package
        .call(
            &QualifiedName::unqualified(name).expect("function name is an identifier"),
            vec![value],
            &ObjectEnvironment::default(),
            &mut Meter::new(LIMITS),
        )
        .expect("function evaluation runs")
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

fn undefined(evaluation: &Evaluation, reason: Undefined) {
    assert!(matches!(
        &evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Undefined(found)) if found == &reason
    ));
}

#[trace("TC-964", "FR-478-AC-1", "FR-478-AC-3")]
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
            "integer",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert_eq!(completed(&evaluation), &Value::Integer(expected));
    }

    let rational = call(
        &package,
        "rational",
        text_value("2/4", TextProfile::UnicodeScalars),
    );
    assert_eq!(
        completed(&rational),
        &Value::Rational(Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap())
    );

    let decimal = call(
        &package,
        "decimal",
        text_value("1.50", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&decimal), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(150_i64) && value.representation().scale() == 2)
    );
    let decimal = call(
        &package,
        "decimal",
        text_value("1.5", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&decimal), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(15_i64) && value.representation().scale() == 1)
    );
    let rounded = call(
        &package,
        "decimal",
        text_value("1.555", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&rounded), Value::Decimal(value) if value.representation().coefficient() == &Integer::from(156_i64) && value.representation().scale() == 2)
    );
    assert!(
        matches!(rounded.losses.as_slice(), [loss] if matches!(loss.loss, ValueLoss::Decimal(_)))
    );

    let strict_inexact = call(
        &package,
        "decimal_exact",
        text_value("1.555", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        &strict_inexact.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::InexactDecimal { .. }))
    ));
    let strict_zeroes = call(
        &package,
        "decimal_exact",
        text_value("1.500", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&strict_zeroes),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(150_i64) && value.representation().scale() == 2
    ));
    let strict_retained = call(
        &package,
        "decimal_exact",
        text_value("1.5", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&strict_retained),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(15_i64) && value.representation().scale() == 1
    ));
    let strict_outside = call(
        &package,
        "decimal_exact",
        text_value("10.01", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        strict_outside.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::DecimalOutOfDomain { .. }))
    ));
    let tie = call(
        &package,
        "decimal",
        text_value("1.565", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        completed(&tie),
        Value::Decimal(value) if value.representation().coefficient() == &Integer::from(156_i64) && value.representation().scale() == 2
    ));
}

#[trace("TC-964", "FR-478-AC-2", "FR-478-AC-4")]
#[test]
fn malformed_numeric_text_is_undefined_and_profile_normalization_is_used() {
    let package = package();
    for text in ["", "+5", "05", "-0", " 5", "1e3", "٣", "５"] {
        undefined(
            &call(
                &package,
                "integer",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
    assert_eq!(
        completed(&call(&package, "nfkc", text_value("５", TextProfile::Nfkc),)),
        &Value::Integer(Integer::from(5_i64))
    );
    undefined(
        &call(&package, "nfc", text_value("５", TextProfile::Nfc)),
        Undefined::MalformedNumber,
    );
    undefined(
        &call(&package, "string_integer", Value::String("５".to_owned())),
        Undefined::MalformedNumber,
    );
    assert_eq!(
        completed(&call(
            &package,
            "string_integer",
            Value::String("42".to_owned()),
        )),
        &Value::Integer(Integer::from(42_i64))
    );
    for text in [".5", "5.", "-0.00", "1.2.3", "1,5", "1e3", "3/4"] {
        undefined(
            &call(
                &package,
                "decimal_exact",
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
                "rational",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}

#[trace("TC-964", "FR-478-AC-1", "FR-478-AC-10")]
#[test]
fn range_and_quantity_unit_outcomes_are_typed() {
    let package = package();
    let outside = call(
        &package,
        "bounded",
        text_value("100", TextProfile::UnicodeScalars),
    );
    assert!(matches!(
        outside.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IntegerOutOfDomain { .. }))
    ));
    for text in ["1/10", "10"] {
        let outside = call(
            &package,
            "rational",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(matches!(
            outside.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::RationalOutOfDomain { .. }))
        ));
    }

    let quantity = call(
        &package,
        "quantity",
        text_value("3 ms", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&quantity), Value::Quantity(value) if value.magnitude() == &Rational::from_integer(Integer::from(3_i64)))
    );
    for text in ["0.5", "2/4 ms"] {
        let quantity = call(
            &package,
            "quantity",
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
                "quantity",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}

#[trace("TC-964", "FR-478-AC-8", "FR-478-AC-11")]
#[test]
fn finite_ieee_text_rounds_once_to_the_target_width() {
    let package = package();
    let evaluation = call(
        &package,
        "float",
        text_value("0.1", TextProfile::UnicodeScalars),
    );
    assert!(
        matches!(completed(&evaluation), Value::Float(value) if value.bits() == 0x3fb9_9999_9999_999a)
    );
    let evaluation = call(
        &package,
        "float",
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
            "float",
            text_value(text, TextProfile::UnicodeScalars),
        );
        assert!(
            matches!(completed(&evaluation), Value::Float(value) if value.bits() == bits),
            "unexpected bits for {text}: {:?}",
            evaluation.outcome
        );
    }
    for text in [
        "1E3", "1e+3", "1e03", "1e-0", ".5", "5.", "1e", "Infinity", "NaN", "-nan", "+inf", "0x1p3",
    ] {
        undefined(
            &call(
                &package,
                "float",
                text_value(text, TextProfile::UnicodeScalars),
            ),
            Undefined::MalformedNumber,
        );
    }
}
