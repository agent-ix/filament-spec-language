// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-181's source transition identity
//! `{type, operation, receiver, arguments}`: a thin wrapper over
//! [`ModelTransition`], its one writer and its one strict reader.

use std::borrow::Cow;

use filament_semantic_value::model_transition::ModelTransition;
use ix_trace_rs::implements;
use serde::de::{Deserialize, Deserializer, Error as _};
use serde::ser::{Error as _, Serialize, SerializeStruct, Serializer};
use serde_json::Value;

use super::{closed_object, member, unique_object, WireDefect, WireDefectKind};
use crate::written_operation::WrittenOperation;

const KIND: &str = "transition";

/// A written source or model transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionIdentity<'a>(Cow<'a, ModelTransition>);

impl<'a> TransitionIdentity<'a> {
    /// The identity's members.
    pub const MEMBERS: [&'static str; 4] = ["type", "operation", "receiver", "arguments"];

    /// The written form of `identity`.
    pub fn borrowed(identity: &'a ModelTransition) -> Self {
        Self(Cow::Borrowed(identity))
    }

    /// The written form of `identity`, held.
    pub fn owned(identity: ModelTransition) -> TransitionIdentity<'static> {
        TransitionIdentity(Cow::Owned(identity))
    }

    /// The model transition.
    pub fn into_model(self) -> ModelTransition {
        self.0.into_owned()
    }

    /// The transition `value` holds. The operation is one string, read by
    /// FR-120's reader rule; an object operation is refused. The receiver is
    /// null or an object and the arguments an array; a format that checks them
    /// against FSpec's `TypedValue` does so after this read.
    ///
    /// # Errors
    /// [`WireDefect`] naming the member at fault.
    pub fn read(value: &Value) -> Result<TransitionIdentity<'static>, WireDefect> {
        Self::read_within(value, quire_canonical::Limits::new(u64::MAX))
    }

    /// [`Self::read`], admitting each identity text under `limits`.
    ///
    /// # Errors
    /// [`WireDefect`] naming the member at fault.
    #[implements("FR-120-AC-39", "FR-120-AC-40")]
    pub fn read_within(
        value: &Value,
        limits: quire_canonical::Limits,
    ) -> Result<TransitionIdentity<'static>, WireDefect> {
        let refuse = |pointer: &str, kind| WireDefect::new(pointer, kind);
        let map = closed_object(value, &Self::MEMBERS, &[])?;
        match member(map, "type")?.as_str() {
            Some(KIND) => {}
            Some(_) => return Err(refuse("/type", WireDefectKind::UnknownVariant)),
            None => return Err(refuse("/type", WireDefectKind::WrongKind)),
        }
        let operation = match member(map, "operation")?.as_str() {
            Some("") => return Err(refuse("/operation", WireDefectKind::Invalid)),
            Some(operation) => WrittenOperation::from_wire(operation.to_owned())
                .into_identity_text()
                .map_err(|_| refuse("/operation", WireDefectKind::Invalid))?,
            None => return Err(refuse("/operation", WireDefectKind::WrongKind)),
        };
        let text = |value: &Value, pointer: &str| {
            serde_json::to_string(value).map_err(|_| refuse(pointer, WireDefectKind::Invalid))
        };
        let receiver = match member(map, "receiver")? {
            Value::Null => None,
            receiver @ Value::Object(_) => Some(text(receiver, "/receiver")?),
            _ => return Err(refuse("/receiver", WireDefectKind::WrongKind)),
        };
        let arguments = member(map, "arguments")?
            .as_array()
            .ok_or_else(|| refuse("/arguments", WireDefectKind::WrongKind))?
            .iter()
            .map(|argument| text(argument, "/arguments"))
            .collect::<Result<Vec<_>, _>>()?;
        ModelTransition::new(operation, receiver, arguments, limits)
            .map(Self::owned)
            .map_err(|_| refuse("", WireDefectKind::Invalid))
    }
}

impl Serialize for TransitionIdentity<'_> {
    #[implements("FR-120-AC-40")]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = |text: &str| serde_json::from_str::<Value>(text).map_err(S::Error::custom);
        let receiver = self.0.receiver().map(value).transpose()?;
        let arguments = self
            .0
            .arguments()
            .iter()
            .map(|argument| value(argument))
            .collect::<Result<Vec<_>, _>>()?;
        let mut object = serializer.serialize_struct("TransitionIdentity", 4)?;
        object.serialize_field("type", KIND)?;
        object.serialize_field("operation", &WrittenOperation::of(self.0.operation()))?;
        object.serialize_field("receiver", &receiver)?;
        object.serialize_field("arguments", &arguments)?;
        object.end()
    }
}

impl<'de> Deserialize<'de> for TransitionIdentity<'_> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::read(&unique_object(decoder)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use filament_semantic_value::model_transition::ModelTransition;
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::TransitionIdentity;
    use crate::shared_wire::{WireDefect, WireDefectKind};

    fn model(operation: &str) -> ModelTransition {
        ModelTransition::new(
            operation.to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("a canonical identity")
    }

    fn written(operation: Value) -> Value {
        json!({"type": "transition", "operation": operation, "receiver": null, "arguments": []})
    }

    // The identity text and the written `operation`, by hand from FR-120. A
    // name that begins `ix://` is a name like any other.
    #[trace("TC-471", "FR-120-AC-39", "FR-120-AC-40")]
    #[test]
    fn a_transition_identity_reads_back_as_it_was_written() {
        for (identity, operation) in [
            (
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
            ),
            (r#""tick""#, "tick"),
            (r#""ix://a/b/C/d""#, "ix://a/b/C/d"),
        ] {
            let transition = model(identity);
            let wire =
                serde_json::to_value(TransitionIdentity::borrowed(&transition)).expect("writes");
            assert_eq!(wire, written(json!(operation)));
            let read = TransitionIdentity::read(&wire).expect("reads").into_model();
            assert_eq!(read.operation(), identity);
            assert_eq!(read, transition);
        }
    }

    #[trace("TC-471", "FR-120-AC-39")]
    #[test]
    fn a_transition_identity_refuses_the_removed_object_operation() {
        for (value, pointer, kind) in [
            (
                written(json!({"name": "tick"})),
                "/operation",
                WireDefectKind::WrongKind,
            ),
            (
                written(json!(true)),
                "/operation",
                WireDefectKind::WrongKind,
            ),
            (written(json!("")), "/operation", WireDefectKind::Invalid),
            (
                json!({"type": "transition", "operation": "tick", "receiver": 1,
                    "arguments": []}),
                "/receiver",
                WireDefectKind::WrongKind,
            ),
            (
                json!({"type": "other", "operation": "tick", "receiver": null,
                    "arguments": []}),
                "/type",
                WireDefectKind::UnknownVariant,
            ),
            (
                json!({"type": "transition", "operation": "tick", "receiver": null}),
                "/arguments",
                WireDefectKind::Missing,
            ),
        ] {
            assert_eq!(
                TransitionIdentity::read(&value).map(|read| read.into_model()),
                Err(WireDefect {
                    pointer: pointer.to_owned(),
                    kind,
                }),
                "{value}"
            );
        }
    }
}
