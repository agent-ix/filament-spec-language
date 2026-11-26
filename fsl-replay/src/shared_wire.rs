// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-037's FSL wire module: the one declared type of each value more than
//! one format writes and reads (FSpec FR-331's State locus, FR-181's source
//! transition identity and FR-413's claim bound).
//!
//! Each type has one `Serialize` writer and one strict `read`, which names the
//! member at fault and why. Each format maps that [`WireDefect`] to its own
//! refusal codes, and runs its own ordered member checks over the type's
//! `MEMBERS` first where its refusal order is specified.

mod bound;
mod state;
mod transition;

use serde::de::{Deserializer, Error as _, MapAccess, Visitor};
use serde_json::{Map, Value};

pub use bound::{BoundForm, ProbabilisticBound};
pub use state::{state_key_from_text, state_key_text, StateLocus};
pub use transition::TransitionIdentity;

/// Why a shared wire value is not one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WireDefectKind {
    /// A required member is absent.
    #[error("a required member is absent")]
    Missing,
    /// A member the value does not define.
    #[error("the member is not defined")]
    UnknownMember,
    /// The value is of another JSON kind.
    #[error("the value is of another JSON kind")]
    WrongKind,
    /// The value is of the right kind and is no canonical spelling.
    #[error("the value is no canonical spelling")]
    Invalid,
    /// A spelling the closed set does not list.
    #[error("the spelling is not listed")]
    UnknownVariant,
}

/// A shared wire value's refusal: the JSON pointer of the member at fault,
/// relative to the value, and why.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{kind} at `{pointer}`")]
pub struct WireDefect {
    /// The relative JSON pointer, empty for the value itself.
    pub pointer: String,
    /// Why.
    pub kind: WireDefectKind,
}

impl WireDefect {
    fn new(pointer: impl Into<String>, kind: WireDefectKind) -> Self {
        Self {
            pointer: pointer.into(),
            kind,
        }
    }
}

/// `value` as an object holding exactly the `required` members and any of the
/// `optional` ones.
fn closed_object<'v>(
    value: &'v Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'v Map<String, Value>, WireDefect> {
    let map = value
        .as_object()
        .ok_or_else(|| WireDefect::new("", WireDefectKind::WrongKind))?;
    if let Some(name) = required.iter().find(|name| !map.contains_key(**name)) {
        return Err(WireDefect::new(format!("/{name}"), WireDefectKind::Missing));
    }
    match map
        .keys()
        .find(|name| !required.contains(&name.as_str()) && !optional.contains(&name.as_str()))
    {
        Some(name) => Err(WireDefect::new(
            format!("/{name}"),
            WireDefectKind::UnknownMember,
        )),
        None => Ok(map),
    }
}

fn member<'v>(map: &'v Map<String, Value>, name: &str) -> Result<&'v Value, WireDefect> {
    map.get(name)
        .ok_or_else(|| WireDefect::new(format!("/{name}"), WireDefectKind::Missing))
}

/// The object a deserializer holds, refusing a member written twice, which a
/// [`Value`] would silently keep once.
fn unique_object<'de, D: Deserializer<'de>>(decoder: D) -> Result<Value, D::Error> {
    struct Object;
    impl<'de> Visitor<'de> for Object {
        type Value = Value;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an object")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
            let mut map = Map::new();
            while let Some((name, value)) = access.next_entry::<String, Value>()? {
                if map.contains_key(&name) {
                    return Err(A::Error::custom(format!("duplicate member `{name}`")));
                }
                map.insert(name, value);
            }
            Ok(Value::Object(map))
        }
    }
    decoder.deserialize_map(Object)
}
