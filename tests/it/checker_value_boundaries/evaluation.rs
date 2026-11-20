// SPDX-License-Identifier: AGPL-3.0-or-later

use super::check;
use filament_eval::FamilyOutcome;
use filament_exact::{Integer, LimitKind, Meter, Outcome, ScalarLimits, Value, ValueType};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_replay::spine::DependencyInput;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

fn limits(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits: 256,
        decimal_digits: 256,
        scale_expansion: 1_024,
        text_input_bytes: 1_048_576,
        text_scalars: 1_048_576,
        normalized_scalars: 1_048_576,
        unit_edges: 1_024,
        value_occurrences: 2_000_000,
        work_units,
        result_units: 2_000_000,
    }
}

pub(super) fn call(
    checked: &fsl_replay::spine::CheckedUnit,
    function: &str,
    arguments: Vec<Value>,
) -> Value {
    let mut meter = Meter::new(limits(100_000));
    let evaluation = checked
        .package()
        .call(
            &QualifiedName::unqualified(function).unwrap(),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("public call admits");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("expected a completed value: {other:?}"),
    }
}

fn evaluated(body: &str, function: &str) -> Value {
    let checked = check(body, &DependencyInput::default()).expect("well-typed source");
    call(&checked, function, vec![])
}

#[test]
#[trace("TC-1231")]
#[ignore = "QSL-2610: source native constructor/operation name resolution; desired octet/length oracle remains strict"]
fn byte_concatenation_preserves_zero_octets_and_input_order() {
    let value = evaluated("function octets using v(): Boolean pure { concat(Bytes(\"0001\"), Bytes(\"ff00\")) = Bytes(\"0001ff00\") and length(concat(Bytes(\"0001\"), Bytes(\"ff00\"))) = 4 and concat(Bytes(\"0001\"), Bytes(\"ff00\")) != concat(Bytes(\"ff00\"), Bytes(\"0001\")) }", "octets");
    assert!(matches!(value, Value::Boolean(true)));
}

#[test]
#[trace("TC-1231", "TC-992")]
#[ignore = "QSL-2610: source JsonObject constructor name resolution; desired payload-meaning oracle remains strict"]
fn native_json_and_raw_strings_preserve_distinct_payload_meanings() {
    let value = evaluated(
        r#"function meanings using v(): Boolean pure { JsonObject("[null,false,7]") != JsonObject("[false,null,7]") and JsonObject("null") != JsonObject("\"null\"") and "é" != "é" }"#,
        "meanings",
    );
    assert!(matches!(value, Value::Boolean(true)));
}

#[test]
#[trace("TC-1231")]
fn duration_addition_compares_after_explicit_unit_conversion() {
    let value = evaluated("dimension Time; unit s : Time = rational(1,1); unit ms : Time = rational(1,1000) * s; function seconds using v(d: Quantity<s>): Quantity<s> pure { d + rational(1,4) s } function total using v(): Boolean pure { seconds(convert<Quantity<s>>(1750 ms)) = 2 s and seconds(convert<Quantity<s>>(1750 ms)) != 1750 s }", "total");
    assert!(matches!(value, Value::Boolean(true)));
}

#[test]
#[trace("FR-056-AC-23", "TC-992")]
fn raw_strings_keep_canonical_equivalents_distinct_and_never_become_uuid_values() {
    let value = evaluated("function raw using v(): Boolean pure { \"é\" != \"é\" and \"00112233-4455-6677-8899-aabbccddeeff\" = \"00112233-4455-6677-8899-aabbccddeeff\" }","raw");
    assert!(matches!(value, Value::Boolean(true)));
    let error = check(
        "function mismatch using v(text: String, token: UUID): Boolean pure { text = token }",
        &DependencyInput::default(),
    )
    .unwrap_err();
    assert_eq!(error.code(), fsl_foundation::Code::IllTyped, "{error:?}");
}

const COUNTER: &str = "function unpack using v(n: Int[0,100001]): Integer pure decreases(n) { if n = 0 then 7 else unpack(n - 1) + 1 }";

#[test]
#[trace("FR-262-AC-1", "TC-734")]
fn recursive_work_fuel_is_exact_at_the_last_required_charge() {
    std::thread::Builder::new().stack_size(512 * 1024).spawn(|| {
        let checked = check(COUNTER, &DependencyInput::default()).expect("decreasing recursion checks");
        let function = QualifiedName::unqualified("unpack").unwrap();
        let objects = ObjectEnvironment::default();
        let invoke = |meter: &mut Meter| checked.package().call(&function, vec![Value::Integer(Integer::from(100_000_i64))], &objects, meter).expect("public invocation admits");
        let mut meter = Meter::new(limits(20_000_000));
        let evaluation = invoke(&mut meter);
        assert!(matches!(evaluation.outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref n))) if *n == Integer::from(100_007_i64)), "{evaluation:?}");
        let work = meter.consumed(LimitKind::WorkUnits);
        assert!(work > 100_000, "recursive calls and body work must charge");
        let mut short = Meter::new(limits(work - 1));
        let stopped = invoke(&mut short);
        let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = stopped.outcome else { panic!("fuel exhaustion must be typed: {stopped:?}") };
        assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
        assert_eq!(incomplete.limit, work - 1);
        assert!(incomplete.consumed <= work - 1);
        assert!(incomplete.next_charge > Integer::zero());
        let mut exact = Meter::new(limits(work));
        let completed = invoke(&mut exact);
        assert!(matches!(completed.outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref n))) if *n == Integer::from(100_007_i64)), "{completed:?}");
        assert_eq!(exact.consumed(LimitKind::WorkUnits), work);
    }).unwrap().join().unwrap();
}

#[test]
#[trace("FR-262-AC-2", "TC-735")]
fn nested_value_type_clone_compare_hash_debug_and_drop_use_bounded_stack() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let mut value_type = ValueType::Integer;
            for _ in 0..100_000 {
                value_type = ValueType::Option(Arc::new(value_type));
            }
            let cloned = value_type.clone();
            assert_eq!(value_type, cloned);
            let mut left = DefaultHasher::new();
            let mut right = DefaultHasher::new();
            value_type.hash(&mut left);
            cloned.hash(&mut right);
            assert_eq!(left.finish(), right.finish());
            let debug = format!("{value_type:?}");
            assert!(debug.starts_with("Option("));
            assert!(debug.contains("Integer"));
            assert_eq!(debug.matches("Option(").count(), 100_000);
            drop(cloned);
            drop(value_type);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
#[trace("FR-262-AC-2", "TC-735")]
fn recursively_built_record_values_compare_clone_and_drop_at_real_depth() {
    std::thread::Builder::new().stack_size(512 * 1024).spawn(|| {
        let source = "record Link { head: Int[0,9]; tail: Option<Link>; } function chain using v(n: Int[0,100000]): Option<Link> pure decreases(n) { if n = 0 then none else Some(Link { head: 7, tail: chain(n - 1) }) }";
        let checked = check(source,&DependencyInput::default()).expect("guarded recursive record type checks");
        let depth = 100_000_u64;
        let mut accounting = limits(20_000_000);
        // Result retention charges each successively larger returned record
        // and option. This cumulative bound is quadratic even though the
        // final value has only linear live occurrences.
        accounting.result_units = 4 * depth * (depth + 1);
        let mut meter = Meter::new(accounting);
        let evaluation = checked.package().call(&QualifiedName::unqualified("chain").unwrap(),vec![Value::Integer(Integer::from(depth))],&ObjectEnvironment::default(),&mut meter).expect("call admits");
        let FamilyOutcome::Evaluated(Outcome::Completed(value)) = evaluation.outcome else {panic!("recursive construction must complete: {evaluation:?}")};
        let mut cursor = &value;
        let mut length = 0;
        loop {
            let Value::Option(option) = cursor else {panic!("chain is an option")};
            let Some(payload) = option.payload() else {break};
            let Value::Composite(link) = payload else {panic!("present chain is a record")};
            assert!(matches!(&link.slots()[0],filament_exact::FieldValue::Present(Value::Integer(head)) if *head == Integer::from(7_i64)));
            let filament_exact::FieldValue::Present(tail) = &link.slots()[1] else {panic!("required optional-valued tail has a value")};
            cursor = tail;
            length += 1;
        }
        assert_eq!(length,100_000);
        let cloned = value.clone();
        let mut comparison = Meter::new(limits(20_000_000));
        assert!(matches!(filament_exact::planned_equality(&value,&cloned,&mut comparison),Outcome::Completed(true)));
        drop(cloned);
        drop(value);
    }).unwrap().join().unwrap();
}
