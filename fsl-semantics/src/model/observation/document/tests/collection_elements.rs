// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native numeric collection element admission.

use super::*;
use filament_exact::{
    CardinalityBound, CollectionType, DecimalType, FloatType, IeeeWidth, RoundingMode, ValueType,
};
use serde_json::json;

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
    ValueType::Collection(CollectionType::new(
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
