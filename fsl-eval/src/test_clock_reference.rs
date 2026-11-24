//! A canonical clock-place reference value for the unit tests of the
//! `ClockReference` arms (FR-230-AC-71).

use filament_exact::{ClockPlaceReference, NodeKey, PathSegment, UniverseId, Value};

/// The `place/v2` text of the smallest canonical identity: a one-field path,
/// no collection step and no slots.
fn place_text(field: &str) -> String {
    let components = [
        serde_json::json!([]),
        serde_json::json!("k"),
        serde_json::json!([]),
        serde_json::json!([[field]]),
        serde_json::json!([]),
    ];
    let mut text = "place/v2".to_owned();
    for component in components {
        text.push('/');
        for byte in serde_json::to_vec(&component).expect("canonical component") {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").expect("write to string");
        }
    }
    text
}

/// A clock-place reference value.
pub(crate) fn clock_reference() -> Value {
    clock_reference_of("field")
}

/// The clock-place reference to the clock field `field`.
pub(crate) fn clock_reference_of(field: &str) -> Value {
    let universe = UniverseId::new(
        NodeKey::new("fixture", vec![PathSegment::Text("universe".into())], 1).expect("a key"),
    );
    Value::ClockReference(
        ClockPlaceReference::new(universe, &place_text(field)).expect("a canonical place id"),
    )
}
