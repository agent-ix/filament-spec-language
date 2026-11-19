// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native numeric collection element admission.

use super::*;
use filament_exact::{
    CardinalityBound, CollectionType, DecimalType, FloatType, IeeeWidth, RoundingMode, ValueType,
};
use serde_json::json;

#[trace("TC-1171", "FR-106-AC-60", "FR-106-AC-61")]
#[test]
fn native_collection_string_bytes_and_json_members_keep_their_payloads() {
    for (slot, first, second) in [
        (
            ValueType::String,
            json!({"kind":"string","text":"é"}),
            json!({"kind":"string","text":"e\u{301}"}),
        ),
        (
            ValueType::Bytes,
            json!({"kind":"bytes","text":"00ff"}),
            json!({"kind":"bytes","text":"01"}),
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"json_object","text":"{\"a\":1}"}),
            json!({"kind":"json_object","text":"{\"b\":2}"}),
        ),
    ] {
        for (kind, tag) in [
            (CollectionKind::Set, "set"),
            (CollectionKind::Bag, "bag"),
            (CollectionKind::OrderedSet, "ordered-set"),
        ] {
            let value = admitted(
                json!({tag:[first.clone(),second.clone()]}),
                collection(kind, slot.clone()),
            )
            .unwrap();
            let Value::Collection(value) = value else {
                panic!("collection")
            };
            assert_eq!(value.elements().len(), 2);
            let duplicate = admitted(
                json!({tag:[first.clone(),first.clone()]}),
                collection(kind, slot.clone()),
            );
            if kind.is_unique() {
                assert_eq!(duplicate.unwrap_err().catalog.cause(), "duplicate-member");
            } else {
                assert!(duplicate.is_ok());
            }
        }
    }
}

fn decimal_slot() -> ValueType {
    ValueType::Decimal(
        DecimalType::new(
            Integer::from(0),
            Integer::from(10000),
            2,
            2,
            RoundingMode::Exact,
        )
        .unwrap(),
    )
}

#[trace("TC-1171", "FR-106-AC-54", "FR-106-AC-58")]
#[test]
fn native_collection_numeric_payload_defects_name_the_member() {
    for (slot, value, cause, member) in [
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"01","scale":"2"}),
            "invalid-value",
            "coefficient",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1","scale":"01"}),
            "invalid-value",
            "scale",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1","scale":2}),
            "wrong-value-kind",
            "scale",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1"}),
            "missing-member",
            "scale",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":"3FF0000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":"3ff000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary32)),
            json!({"kind":"float32","bits":"3ff0000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":0}),
            "wrong-value-kind",
            "bits",
        ),
    ] {
        let refusal = admitted(
            json!({"set":[value]}),
            collection(CollectionKind::Set, slot),
        )
        .unwrap_err();
        assert_eq!(refusal.catalog.cause(), cause);
        assert_eq!(refusal.path, [PathStep::Index(0)]);
        assert_eq!(
            refusal.fields.get("member").map(String::as_str),
            Some(member)
        );
    }
}

fn admitted(object: serde_json::Value, slot: ValueType) -> Result<Value, AdmissionRecord> {
    let raw = snapshot(&object.to_string());
    let types = TypeEnvironment::default();
    let mut references = References {
        views: &[],
        types: &types,
        admitted: BTreeMap::new(),
        completeness: BTreeMap::new(),
        unresolved: Vec::new(),
        record_keys: None,
        variant_keys: None,
    };
    admit_form(&mut references, raw.form(), &slot, None)
}

fn collection(kind: CollectionKind, element: ValueType) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(0, 6).unwrap()),
    ))
}

#[trace(
    "TC-1171",
    "FR-106-AC-53",
    "FR-106-AC-54",
    "FR-106-AC-55",
    "FR-106-AC-56"
)]
#[trace("FSpec-FR-144-AC-17", "FSpec-FR-144-AC-18")]
#[test]
fn native_collection_numeric_equivalence_controls_duplicates_and_bags() {
    for (slot, first, equal) in [
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"150","scale":"2"}),
            json!({"kind":"decimal","coefficient":"15","scale":"1"}),
        ),
        (
            rational_slot(),
            json!({"kind":"rational","numerator":"2","denominator":"4"}),
            json!({"kind":"rational","numerator":"1","denominator":"2"}),
        ),
    ] {
        for (kind, tag) in [
            (CollectionKind::Set, "set"),
            (CollectionKind::OrderedSet, "ordered-set"),
        ] {
            let refusal = admitted(
                json!({tag:[first.clone(), equal.clone()]}),
                collection(kind, slot.clone()),
            )
            .unwrap_err();
            assert_eq!(refusal.catalog.cause(), "duplicate-member");
            assert_eq!(refusal.path, [PathStep::Index(1)]);
            assert_eq!(
                refusal.fields.get("earlier-index").map(String::as_str),
                Some("0")
            );
        }
        let bag = admitted(
            json!({"bag":[first, equal]}),
            collection(CollectionKind::Bag, slot),
        )
        .unwrap();
        let Value::Collection(bag) = bag else {
            panic!("collection")
        };
        assert_eq!(bag.elements().len(), 2);
        assert_eq!(
            filament_exact::compare_keys(&bag.elements()[0], &bag.elements()[1]),
            Some(std::cmp::Ordering::Equal)
        );
    }
}

#[trace("TC-1171", "FR-106-AC-53", "FR-106-AC-54")]
#[test]
fn native_collection_decimal_retains_ordered_spelling_and_checks_value_membership() {
    let decimal = |coefficient: &str, scale: &str| json!({"kind":"decimal","coefficient":coefficient,"scale":scale});
    let value = admitted(decimal("150", "2"), decimal_slot()).unwrap();
    let Value::Decimal(value) = value else {
        panic!("decimal")
    };
    assert_eq!(value.coefficient().to_string(), "150");
    assert_eq!(value.scale(), 2);
    assert!(admitted(decimal("1000", "3"), decimal_slot()).is_ok());
    for (coefficient, scale) in [("1", "3"), ("10001", "2"), ("-1", "0")] {
        let refusal = admitted(decimal(coefficient, scale), decimal_slot()).unwrap_err();
        assert_eq!(refusal.catalog.cause(), "invalid-value");
        assert_eq!(
            refusal.fields.get("expected").map(String::as_str),
            Some("Decimal[0, 10000; 2, 2]")
        );
    }
    let nested = collection(
        CollectionKind::Bag,
        collection(CollectionKind::Sequence, decimal_slot()),
    );
    let value = admitted(
        json!({"bag":[{"sequence":[decimal("150","2"),decimal("100","2")]}]}),
        nested,
    )
    .unwrap();
    let Value::Collection(outer) = value else {
        panic!("bag")
    };
    let Value::Collection(inner) = &outer.elements()[0] else {
        panic!("sequence")
    };
    let Value::Decimal(first) = &inner.elements()[0] else {
        panic!("decimal")
    };
    assert_eq!(
        (first.coefficient().to_string(), first.scale()),
        ("15".to_owned(), 1)
    );
}

#[trace("TC-1171", "FR-106-AC-57", "FR-106-AC-58")]
#[test]
fn native_collection_float_bits_preserve_zero_sign_and_nan_payload() {
    for (width, tag, bits) in [
        (
            IeeeWidth::Binary32,
            "float32",
            ["00000000", "80000000", "7fc00001", "7fc00002"],
        ),
        (
            IeeeWidth::Binary64,
            "float64",
            [
                "0000000000000000",
                "8000000000000000",
                "7ff8000000000001",
                "7ff8000000000002",
            ],
        ),
    ] {
        let slot = ValueType::Float(FloatType::exact(width));
        let values = bits.map(|bits| json!({"kind":tag,"bits":bits}));
        let value = admitted(
            json!({"set":values}),
            collection(CollectionKind::Set, slot.clone()),
        )
        .unwrap();
        let Value::Collection(value) = value else {
            panic!("collection")
        };
        assert_eq!(value.elements().len(), 4);
        let refusal = admitted(
            json!({"set":[values[2],values[2]]}),
            collection(CollectionKind::Set, slot),
        )
        .unwrap_err();
        assert_eq!(refusal.catalog.cause(), "duplicate-member");
    }
}
