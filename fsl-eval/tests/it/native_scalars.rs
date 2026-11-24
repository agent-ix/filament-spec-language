// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056 native scalar kinds evaluated from source text: record values
//! `String` against `UUID` and `Text` (TC-992, FR-056-AC-34) and
//! the `Bytes`, `JsonObject` and `Duration` natives (TC-1231).
//!
//! Each equality vector is run as an `=` / `!=` pair and the result of `!=`
//! must be the negation of `=`, so a constant-true or constant-false
//! evaluator cannot satisfy it. The Bytes and JsonObject source forms are
//! reported unimplemented (QSL-2242) by strict `#[ignore]`d tests that state
//! the specified outcome; nothing here fakes them.

use filament_eval::CallFailure;
use filament_eval::FamilyOutcome;
use filament_exact::{
    FieldValue, Meter, Outcome, ScalarLimits, TextPayload, TextProfile, TextType, Uuid, Value,
};
use filament_semantic_value::call::InputRefusal;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

const UNLIMITED: ScalarLimits = ScalarLimits {
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

fn try_package(functions: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{functions}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "native-scalars"),
        "native-scalars.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("assembly");
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

fn refusals_of(functions: &str) -> Vec<CheckRefusal> {
    match try_package(functions) {
        Err(refusals) => refusals,
        Ok(_) => panic!("the unit must be refused: {functions}"),
    }
}

fn package(functions: &str) -> CheckedPackage {
    try_package(functions).unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"))
}

fn run(
    package: &CheckedPackage,
    name: &str,
    arguments: Vec<Value>,
) -> Result<Outcome<Value>, CallFailure> {
    package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .map(|evaluation| match evaluation.outcome {
            FamilyOutcome::Evaluated(outcome) => outcome,
            other => panic!("a kernel outcome, not {other:?}"),
        })
}

fn truth(package: &CheckedPackage, name: &str, arguments: Vec<Value>) -> bool {
    match run(package, name, arguments).expect("the call runs") {
        Outcome::Completed(Value::Boolean(truth)) => truth,
        other => panic!("{name}: a completed Boolean, not {other:?}"),
    }
}

/// `=` gives `expected`, `!=` its negation.
fn assert_pair(
    package: &CheckedPackage,
    (eq, ne): (&str, &str),
    a: Value,
    b: Value,
    expected: bool,
) {
    assert_eq!(
        truth(package, eq, vec![a.clone(), b.clone()]),
        expected,
        "{eq}"
    );
    assert_eq!(truth(package, ne, vec![a, b]), !expected, "{ne}");
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

fn uuid(text: &str) -> Value {
    Value::Uuid(Uuid::from_canonical_text(text).expect("canonical UUID"))
}

const UUID_A: &str = "00112233-4455-6677-8899-aabbccddeeff";
const UUID_B: &str = "00112233-4455-6677-8899-aabbccddeefe";

const STRING_UUID: &str = "\
    function eqs using v(a: String, b: String): Boolean pure { a = b }\n\
    function nes using v(a: String, b: String): Boolean pure { a != b }\n\
    function equ using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
    function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
    function eqx using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }\n";

fn nfc_text(payload: &str) -> Value {
    let nfc = TextType::new(0, 8, TextProfile::Nfc).expect("a Text type");
    let Outcome::Completed(text) = filament_exact::admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).expect("UTF-8"),
        &nfc,
        &mut Meter::new(UNLIMITED),
    ) else {
        panic!("the payload is admitted");
    };
    Value::Text(text)
}

#[trace("TC-992", "FR-056-AC-34")]
#[test]
fn string_equality_is_the_exact_scalar_sequence_and_text_normalizes() {
    let package = package(STRING_UUID);
    assert_pair(&package, ("eqs", "nes"), string("abc"), string("abc"), true);
    // NFC e-acute against `e` + combining acute: unequal as native String...
    let composed = "\u{e9}";
    let decomposed = "e\u{301}";
    assert_pair(
        &package,
        ("eqs", "nes"),
        string(composed),
        string(decomposed),
        false,
    );
    // ...and equal under `Text[0, 8; nfc]`.
    assert!(truth(
        &package,
        "eqx",
        vec![nfc_text(composed), nfc_text(decomposed)]
    ));
    assert!(!truth(
        &package,
        "eqx",
        vec![nfc_text(composed), nfc_text("e")]
    ));
    // Native strings remain case sensitive.
    assert_pair(&package, ("eqs", "nes"), string("A"), string("a"), false);
}

#[trace("TC-992", "FR-056-AC-34")]
#[test]
fn uuid_equality_is_the_canonical_128_bits() {
    let package = package(STRING_UUID);
    assert_pair(&package, ("equ", "neu"), uuid(UUID_A), uuid(UUID_A), true);
    assert_pair(&package, ("equ", "neu"), uuid(UUID_A), uuid(UUID_B), false);
}

#[trace("TC-992", "FR-056-AC-34")]
#[test]
fn string_against_uuid_is_ill_typed_statically_and_wrong_kind_at_runtime() {
    // Static: the cross-type equality has no Boolean result.
    let refusals =
        refusals_of("function mixed using v(s: String, u: UUID): Boolean pure { s = u }\n");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause.code(), fsl_foundation::Code::IllTyped);
    assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
    // Runtime: each wrong native kind is refused as `wrong-value-kind` at its
    // parameter; neither is coerced to the other.
    let package = package(STRING_UUID);
    let wrong_uuid = run(&package, "equ", vec![string(UUID_A), uuid(UUID_A)]);
    assert!(
        matches!(
            wrong_uuid,
            Err(CallFailure::Input(InputRefusal::WrongValueKind {
                parameter: 0
            }))
        ),
        "{wrong_uuid:?}"
    );
    let wrong_string = run(&package, "eqs", vec![string("x"), uuid(UUID_A)]);
    assert!(
        matches!(
            wrong_string,
            Err(CallFailure::Input(InputRefusal::WrongValueKind {
                parameter: 1
            }))
        ),
        "{wrong_string:?}"
    );
}

// ---- Record values --------------------------------------------------------

const RECORDS: &str = "\
    record Street { name: Integer; }\n\
    record Address { street: Option<Street>; }\n\
    record Account { address: Address; }\n\
    function built using v(n: Int[0, 9]): Account pure { Account { address: Address { street: Some(Street { name: n }) } } }\n\
    function bare using v(): Account pure { Account { address: Address { street: none } } }\n\
    function named using v(a: Account): Boolean pure { present(a.address.street) and value(a.address.street).name = 7 }\n\
    function seven using v(): Boolean pure { named(built(7)) }\n\
    function eight using v(): Boolean pure { named(built(8)) }\n\
    function none_named using v(): Boolean pure { named(bare()) }\n\
    function equal_built using v(): Boolean pure { built(7) = built(7) }\n\
    function unequal_built using v(): Boolean pure { built(7) != built(8) }\n\
    function absent_differs using v(): Boolean pure { built(7) != bare() }\n";

#[test]
fn a_record_value_field_and_its_nested_optional_record_read_and_compare() {
    let package = package(RECORDS);
    assert!(truth(&package, "seven", vec![]));
    assert!(
        !truth(&package, "eight", vec![]),
        "the nested name decides the read"
    );
    assert!(
        !truth(&package, "none_named", vec![]),
        "an absent nested record is not present"
    );
    assert!(truth(&package, "equal_built", vec![]));
    assert!(truth(&package, "unequal_built", vec![]));
    assert!(truth(&package, "absent_differs", vec![]));
    // The result is a record value with its declared slots, not a reference.
    let Outcome::Completed(Value::Composite(account)) =
        run(&package, "bare", vec![]).expect("the call runs")
    else {
        panic!("an Account record value");
    };
    assert_eq!(account.slots().len(), 1);
    assert!(matches!(
        &account.slots()[0],
        FieldValue::Present(Value::Composite(_))
    ));
}

// ---- TC-1231: Duration, Bytes and JsonObject -------------------------------

const TIME: &str = "dimension Time; unit s : Time = rational(1, 1); \
                    unit ms : Time = rational(1, 1000) * s;\n";

#[trace("TC-1231", "FR-091-AC-56")]
#[test]
fn durations_are_quantities_of_seconds_and_compare_after_explicit_conversion() {
    let package = package(&format!(
        "{TIME}\
         function same_half using v(): Boolean pure {{ rational(3, 2) s = rational(3, 2) s }}\n\
         function differs using v(): Boolean pure {{ rational(3, 2) s = rational(2, 1) s }}\n\
         function via_ms using v(): Boolean pure {{ rational(3, 2) s = convert<Quantity<s>>(1500 ms) }}\n\
         function via_other_ms using v(): Boolean pure {{ rational(3, 2) s = convert<Quantity<s>>(1501 ms) }}\n\
         function before using v(a: Quantity<s>, b: Quantity<s>): Boolean pure {{ a < b }}\n\
         function summed using v(a: Quantity<s>, b: Quantity<s>): Quantity<s> pure {{ a + b }}\n\
         function sums using v(): Boolean pure {{ summed(rational(1, 2) s, rational(1, 1) s) = rational(3, 2) s }}\n"
    ));
    assert!(truth(&package, "same_half", vec![]));
    assert!(!truth(&package, "differs", vec![]));
    assert!(truth(&package, "via_ms", vec![]));
    assert!(!truth(&package, "via_other_ms", vec![]));
    assert!(truth(&package, "sums", vec![]), "FR-094 quantity addition");
    // D4: comparing a Quantity<s> with a Quantity<ms> without conversion is
    // refused statically.
    let refusals = refusals_of(&format!(
        "{TIME}function mixed using v(d: Quantity<s>, m: Quantity<ms>): Boolean pure {{ d = m }}\n"
    ));
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        (refusals[0].cause.code(), refusals[0].cause.cause()),
        (fsl_foundation::Code::IllTyped, Some("type-mismatch"))
    );
}

/// E1 to E13 and the L literals over `Bytes`.
#[trace("TC-1231")]
#[test]
#[ignore = "QSL-2242: native Bytes source type, literal and eq/length/concat operations do not exist yet"]
fn bytes_equality_length_concat_and_literals() {
    let package = package(
        "function eqb using v(a: Bytes, b: Bytes): Boolean pure { a = b }\n\
         function neb using v(a: Bytes, b: Bytes): Boolean pure { a != b }\n\
         function lenb using v(a: Bytes): Integer pure { length(a) }\n\
         function catb using v(a: Bytes, b: Bytes): Bytes pure { concat(a, b) }\n\
         function lit using v(): Boolean pure { eqb(Bytes(\"00ff\"), Bytes(\"00ff\")) }\n",
    );
    let bytes = |octets: &[u8]| Value::Bytes(octets.to_vec());
    assert_pair(
        &package,
        ("eqb", "neb"),
        bytes(&[0, 255]),
        bytes(&[0, 255]),
        true,
    );
    assert_pair(
        &package,
        ("eqb", "neb"),
        bytes(&[0, 255]),
        bytes(&[0, 254]),
        false,
    );
    assert_pair(&package, ("eqb", "neb"), bytes(&[]), bytes(&[0]), false);
    let joined = run(&package, "catb", vec![bytes(&[0, 255]), bytes(&[16])]).unwrap();
    assert!(
        matches!(joined, Outcome::Completed(Value::Bytes(ref octets)) if octets == &[0, 255, 16]),
        "{joined:?}"
    );
    assert!(truth(&package, "lit", vec![]));
}

/// E14 to E21 and the L literals over `JsonObject`.
#[trace("TC-1231")]
#[test]
#[ignore = "QSL-2242: native JsonObject source type, literal and equality operations do not exist yet"]
fn json_object_equality_and_literals() {
    let package = package(
        "function eqj using v(x: JsonObject, y: JsonObject): Boolean pure { x = y }\n\
         function nej using v(x: JsonObject, y: JsonObject): Boolean pure { x != y }\n\
         function lit using v(): Boolean pure { JsonObject(\"{\\\"a\\\":1}\") = JsonObject(\"{\\\"a\\\":1}\") }\n",
    );
    assert!(truth(&package, "lit", vec![]));
}
