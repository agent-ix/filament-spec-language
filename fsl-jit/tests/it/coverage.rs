// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the coverage the JIT publishes and the disposition of every entry
//! outside it.

use filament_eval::{
    CallFailure, EnvironmentRefusal, ExecutionBackendChoice, FamilyOutcome, UnsupportedCause,
    UntranslatableConstruct,
};
use filament_exact::{Decimal, Integer, Outcome, Rational, Value, ValueType};
use fsl_eval::execution::PreparedEntries;
use fsl_eval::execution::{CoverageOperation, CoverageOperator, CoverageValueType};
use ix_trace_rs::trace;

use crate::common::{int, jitted_call, package_of, prepare, unsupported_names};

#[trace("TC-1380", "FR-560-AC-1")]
#[test]
fn the_published_coverage_lists_the_value_floor() {
    let coverage = fsl_jit::coverage();
    for form in [
        "literal",
        "name",
        "operator",
        "let",
        "if",
        "call",
        "record",
        "collection",
        "population",
    ] {
        assert!(coverage.expression_forms.contains(form), "{form}");
    }
    for value_type in [
        CoverageValueType::Boolean,
        CoverageValueType::Integer,
        CoverageValueType::Rational,
        CoverageValueType::Decimal,
        CoverageValueType::Text,
        CoverageValueType::Record,
        CoverageValueType::Tuple,
        CoverageValueType::Collection,
        CoverageValueType::Reference,
    ] {
        assert!(coverage.value_types.contains(&value_type), "{value_type:?}");
    }
    assert!(!coverage.value_types.contains(&CoverageValueType::Timestamp));
    assert!(!coverage.expression_forms.contains("case"));
    for (operator, value_type, result_type) in [
        (
            CoverageOperator::Add,
            CoverageValueType::Integer,
            CoverageValueType::Integer,
        ),
        (
            CoverageOperator::Multiply,
            CoverageValueType::Rational,
            CoverageValueType::Rational,
        ),
        (
            CoverageOperator::Divide,
            CoverageValueType::Decimal,
            CoverageValueType::Decimal,
        ),
        (
            CoverageOperator::Equal,
            CoverageValueType::Text,
            CoverageValueType::Boolean,
        ),
        (
            CoverageOperator::Equal,
            CoverageValueType::Record,
            CoverageValueType::Boolean,
        ),
        (
            CoverageOperator::Equal,
            CoverageValueType::Collection,
            CoverageValueType::Boolean,
        ),
    ] {
        assert!(coverage.operations.contains(&CoverageOperation {
            operator,
            operand_types: vec![value_type.clone(), value_type],
            result_type,
        }));
    }
    for (operator, operands, result_type) in [
        (
            CoverageOperator::Other("record-field-read".to_owned()),
            vec![CoverageValueType::Record],
            CoverageValueType::Integer,
        ),
        (
            CoverageOperator::Other("contains".to_owned()),
            vec![CoverageValueType::Collection, CoverageValueType::Integer],
            CoverageValueType::Boolean,
        ),
    ] {
        assert!(coverage.operations.contains(&CoverageOperation {
            operator,
            operand_types: operands,
            result_type,
        }));
    }
}

#[trace("TC-1380", "FR-560-AC-2")]
#[test]
fn covered_floor_values_and_the_rational_example_use_exact_value_handles() {
    let package = package_of(
        "record Pair { x: Int[0, 9]; }\n\
         function half using v(a: Rational[-9, 9; 1, 9], h: Rational[1, 1; 2, 2]): Rational[-9, 9; 1, 18] pure { a * h }\n\
         function integerEqualsRational using v(a: Int[0, 9], b: Rational[0, 9; 1, 1]): Boolean pure { convert<Rational[0, 9; 1, 1]>(a) = b }\n\
         function text using v(x: Text[0, 8; nfc]): Text[0, 8; nfc] pure { x }\n\
         function before using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a < b }\n\
         function textSize using v(x: Text[0, 8; nfc]): Integer pure { size(x) }\n\
         function textToInteger using v(x: Text[0, 8; nfc]): Int[0, 99] pure { convert<Int[0, 99]>(x) }\n\
         function decimalAdd using v(a: Decimal[-100, 100; 0, 2; nearest-even], b: Decimal[-100, 100; 0, 2; nearest-even]): Decimal[-100, 100; 0, 2; nearest-even] pure { a + b }\n\
         function pair using v(): Pair pure { Pair{x: 1} }\n\
         function pairField using v(p: Pair): Int[0, 9] pure { p.x }\n\
         function makeSeq using v(): Sequence<Int[0, 1]>[1, 2] pure { sequence[0, 1] }\n\
         function hasOne using v(): Boolean pure { contains(makeSeq(), 1) }\n\
         function firstValue using v(s: Sequence<Int[0, 1]>[1, 2]): Int[0, 1] pure { s[0] }\n\
         function dropFirstOne using v(s: Sequence<Int[0, 1]>[1, 2]): Sequence<Int[0, 1]>[0, 1] pure { dropFirst(s) }\n\
         function removeOneValue using v(s: Sequence<Int[0, 1]>[1, 2]): Sequence<Int[0, 1]>[0, 2] pure { removeOne(s, 1) }\n",
    );
    let prepared = prepare(
        &package,
        &[
            "half",
            "integerEqualsRational",
            "text",
            "before",
            "textSize",
            "textToInteger",
            "decimalAdd",
            "pair",
            "pairField",
            "makeSeq",
            "hasOne",
            "firstValue",
            "dropFirstOne",
            "removeOneValue",
        ],
    );
    assert!(
        prepared.unsupported().is_empty(),
        "{:?}",
        prepared.unsupported()
    );

    let half = jitted_call(
        &prepared,
        "half",
        &[
            Value::Rational(Rational::new(3_i64.into(), 1_i64.into()).expect("rational")),
            Value::Rational(Rational::new(1_i64.into(), 2_i64.into()).expect("rational")),
        ],
    )
    .expect("rational multiplication completes");
    assert!(matches!(
        half.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(value)))
            if value == Rational::new(3_i64.into(), 2_i64.into()).expect("rational")
    ));
    let integer_equals_rational = jitted_call(
        &prepared,
        "integerEqualsRational",
        &[
            Value::Integer(Integer::from(2_i64)),
            Value::Rational(Rational::new(2_i64.into(), 1_i64.into()).expect("rational")),
        ],
    )
    .expect("integer-to-rational equality completes");
    assert!(
        matches!(
            integer_equals_rational.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
        ),
        "{integer_equals_rational:?}"
    );

    let text = filament_exact::admit_text(
        &filament_exact::TextPayload::from_utf8(b"value").expect("text payload"),
        &filament_exact::TextType::new(0, 8, filament_exact::TextProfile::Nfc).expect("text type"),
        &mut filament_exact::Meter::new(crate::common::UNLIMITED),
    );
    let Outcome::Completed(text) = text else {
        panic!("the text argument is admitted");
    };
    assert!(matches!(
        jitted_call(&prepared, "text", &[Value::Text(text)])
            .unwrap()
            .outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Text(_)))
    ));
    let text_a = filament_exact::admit_text(
        &filament_exact::TextPayload::from_utf8(b"a").expect("text payload"),
        &filament_exact::TextType::new(0, 8, filament_exact::TextProfile::Nfc).expect("text type"),
        &mut filament_exact::Meter::new(crate::common::UNLIMITED),
    );
    let text_b = filament_exact::admit_text(
        &filament_exact::TextPayload::from_utf8(b"b").expect("text payload"),
        &filament_exact::TextType::new(0, 8, filament_exact::TextProfile::Nfc).expect("text type"),
        &mut filament_exact::Meter::new(crate::common::UNLIMITED),
    );
    let (Outcome::Completed(text_a), Outcome::Completed(text_b)) = (text_a, text_b) else {
        panic!("text ordering arguments are admitted");
    };
    assert!(matches!(
        jitted_call(
            &prepared,
            "before",
            &[Value::Text(text_a.clone()), Value::Text(text_b)],
        )
        .unwrap()
        .outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        jitted_call(&prepared, "textSize", &[Value::Text(text_a.clone())])
            .unwrap()
            .outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(size)))
            if size == Integer::from(1_i64)
    ));
    let digits = filament_exact::admit_text(
        &filament_exact::TextPayload::from_utf8(b"42").expect("text payload"),
        &filament_exact::TextType::new(0, 8, filament_exact::TextProfile::Nfc).expect("text type"),
        &mut filament_exact::Meter::new(crate::common::UNLIMITED),
    );
    let Outcome::Completed(digits) = digits else {
        panic!("the numeric text argument is admitted");
    };
    assert!(matches!(
        jitted_call(&prepared, "textToInteger", &[Value::Text(digits)])
            .unwrap()
            .outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(42_i64)
    ));
    let sum = jitted_call(
        &prepared,
        "decimalAdd",
        &[
            Value::Decimal(Decimal::new(Integer::from(3_i64), 0)),
            Value::Decimal(Decimal::new(Integer::from(2_i64), 0)),
        ],
    )
    .unwrap();
    assert!(matches!(
        sum.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Decimal(value)))
            if value.numerically_equal(&Decimal::new(Integer::from(5_i64), 0))
    ));
    let pair = jitted_call(&prepared, "pair", &[]).unwrap();
    let FamilyOutcome::Evaluated(Outcome::Completed(pair_value @ Value::Composite(_))) =
        pair.outcome
    else {
        panic!("pair construction returns a record");
    };
    assert!(matches!(
        jitted_call(&prepared, "pairField", &[pair_value]).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(1_i64)
    ));
    assert!(matches!(
        jitted_call(&prepared, "makeSeq", &[]).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(_)))
    ));
    assert!(matches!(
        jitted_call(&prepared, "hasOne", &[]).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    let make_seq = jitted_call(&prepared, "makeSeq", &[]).unwrap();
    let FamilyOutcome::Evaluated(Outcome::Completed(sequence @ Value::Collection(_))) =
        make_seq.outcome
    else {
        panic!("sequence construction returns a collection");
    };
    assert!(matches!(
        jitted_call(&prepared, "firstValue", std::slice::from_ref(&sequence)).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(0_i64)
    ));
    let dropped = jitted_call(&prepared, "dropFirstOne", std::slice::from_ref(&sequence)).unwrap();
    assert!(matches!(
        dropped.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(ref value)))
            if value.elements().len() == 1
    ));
    let removed = jitted_call(&prepared, "removeOneValue", &[sequence]).unwrap();
    assert!(matches!(
        removed.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(ref value)))
            if value.elements().len() == 1
    ));
}

#[trace("TC-1380", "FR-560-AC-1")]
#[test]
fn jit_executes_map_filter_count_sum_and_fold() {
    let package = package_of(
        "type Count = Int[0, 2];\n\
         type Total = Integer;\n\
         type Half = Rational[0, 1; 1, 2];\n\
         function mapped using v(s: Sequence<Int[0, 3]>[1, 2]): Sequence<Int[0, 3]>[1, 2] pure { map(x in s: x) }\n\
         function filtered using v(s: Sequence<Int[0, 3]>[1, 2]): Sequence<Int[0, 3]>[0, 2] pure { filter(x in s: x > 0) }\n\
         function counted using v(s: Sequence<Int[0, 3]>[1, 2]): Count pure { count<Count>(x in s: x > 0) }\n\
         function summed using v(s: Sequence<Int[0, 3]>[1, 2]): Total pure { sum<Total>(x in s: x) }\n\
         function quotient using v(): Half pure { 1 / 2 }\n\
         function folded using v(s: Sequence<Int[0, 3]>[1, 2]): Total pure { fold<Total>(acc, x in s: acc + x, identity: 0) }\n",
    );
    let prepared = prepare(
        &package,
        &["mapped", "filtered", "counted", "summed", "quotient", "folded"],
    );
    assert!(prepared.unsupported().is_empty(), "{:?}", prepared.unsupported());
    let sequence_type = filament_exact::CollectionType::new(
        filament_exact::CollectionKind::Sequence,
        ValueType::Int(
            filament_exact::IntegerInterval::new(0_i64.into(), 3_i64.into())
                .expect("the interval is non-empty"),
        ),
        Some(
            filament_exact::CardinalityBound::new(1, 2)
                .expect("the cardinality bound is non-empty"),
        ),
    );
    let sequence = filament_exact::form(
        &sequence_type,
        [1_i64, 2]
            .map(|value| Value::Integer(Integer::from(value)))
            .to_vec(),
        &mut filament_exact::Meter::new(crate::common::UNLIMITED),
    );
    let Outcome::Completed(input @ Value::Collection(_)) = sequence else {
        panic!("the input sequence is admitted");
    };
    for function in ["mapped", "filtered"] {
        assert!(matches!(
            jitted_call(&prepared, function, std::slice::from_ref(&input)).unwrap().outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Collection(_)))
        ));
    }
    assert!(matches!(
        jitted_call(&prepared, "counted", std::slice::from_ref(&input)).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(2_i64)
    ));
    assert!(matches!(
        jitted_call(&prepared, "summed", std::slice::from_ref(&input)).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(3_i64)
    ));
    assert!(matches!(
        jitted_call(&prepared, "quotient", &[]).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(value)))
            if value.numerator() == &Integer::one() && value.denominator() == &Integer::from(2_i64)
    ));
    assert!(matches!(
        jitted_call(&prepared, "folded", &[input]).unwrap().outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value)))
            if value == Integer::from(3_i64)
    ));
}

const SOURCE: &str = "\
 function ok using v(a: Int[0, 9]): Integer pure { a + 1 }\n\
 function stamped using v(t: Timestamp): Boolean pure { true }\n\
 function text using v(t: Text[0, 4; nfd]): Boolean pure { true }\n\
 function wide using v(a: Int[0, 9223372036854775807], b: Int[0, 9223372036854775807]): Integer pure { a + b }\n";

#[trace("TC-1381")]
#[test]
fn a_parameter_of_an_unlisted_type_is_listed_with_that_type_and_the_rest_are_prepared() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["ok", "stamped", "ok"]);
    assert_eq!(unsupported_names(&prepared), vec!["stamped".to_owned()]);
    let (_, entry) = &prepared.unsupported()[0];
    assert_eq!(entry.backend, ExecutionBackendChoice::Jit);
    let UnsupportedCause::Untranslatable { construct, .. } = &entry.cause else {
        panic!("expected Untranslatable, got {:?}", entry.cause);
    };
    assert_eq!(
        *construct,
        UntranslatableConstruct::ValueType(Box::new(ValueType::Timestamp))
    );
    let called = jitted_call(&prepared, "stamped", &[]);
    assert!(matches!(called, Err(CallFailure::Unsupported(_))));
    assert!(jitted_call(&prepared, "ok", &[int(4)]).is_ok());
}

#[trace("TC-1381")]
#[test]
fn an_operation_whose_result_does_not_fit_64_bits_is_listed_unsupported() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["wide"]);
    assert_eq!(unsupported_names(&prepared), vec!["wide".to_owned()]);
}

/// TC-1381 FR-560-AC-6: host executable-memory refusal is attached to every
/// selected entry and returned through the normal unsupported call result.
#[trace("TC-1381", "FR-560-AC-6")]
#[test]
fn executable_memory_refusal_lists_and_rejects_every_selected_entry() {
    let package = package_of(
        "function one using v(): Integer pure { 1 }\n\
         function two using v(): Integer pure { 2 }\n\
         function three using v(): Integer pure { one() + two() }\n",
    );
    let entries = crate::common::select(&["one", "two", "three"]);
    let jit = fsl_jit::Jit;
    let denied = jit
        .prepare_with_executable_memory_provider_for_test(
            &package,
            &entries,
            &fsl_eval::execution::PrepareLimits::default(),
            &filament_exact::Cancel::new(),
            || false,
        )
        .expect("a denied executable-memory request is a prepared refusal");

    assert_eq!(denied.unsupported().len(), 3);
    for (selection, unsupported) in denied.unsupported() {
        assert_eq!(unsupported.backend, ExecutionBackendChoice::Jit);
        assert_eq!(
            unsupported.cause,
            UnsupportedCause::EnvironmentRefused(EnvironmentRefusal::ExecutableMemoryRefused)
        );
        let catalog = unsupported.catalog_code();
        assert_eq!(catalog.code(), "unimplemented_capability");
        assert_eq!(catalog.cause(), "environment-refused");
        let name = selection.name.to_string();
        let failure = crate::common::jitted_call(&denied, &name, &[])
            .expect_err("a denied executable-memory entry is unsupported");
        assert_eq!(failure.category().exit_code(), 21);
        let CallFailure::Unsupported(actual) = failure else {
            panic!("denied JIT call returns the typed unsupported failure");
        };
        assert_eq!(actual, *unsupported);
    }

    let allowed = jit
        .prepare_with_executable_memory_provider_for_test(
            &package,
            &entries,
            &fsl_eval::execution::PrepareLimits::default(),
            &filament_exact::Cancel::new(),
            || true,
        )
        .expect("an allowed executable-memory request translates the same entries");
    assert!(allowed.unsupported().is_empty());
    for name in ["one", "two", "three"] {
        assert!(crate::common::jitted_call(&allowed, name, &[]).is_ok());
    }
}
