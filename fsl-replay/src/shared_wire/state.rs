// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-331's State locus `{key}` and the one spelling of a state key.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use fsl_eval::simulation::StateKey;
use serde::de::{Deserialize, Deserializer, Error as _};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

use super::{closed_object, member, unique_object, WireDefect, WireDefectKind};

/// The wire spelling of a state key: the unpadded base64url of its canonical
/// octets (FSpec FR-181, FR-331).
pub fn state_key_text(key: &StateKey) -> String {
    URL_SAFE_NO_PAD.encode(key.as_bytes())
}

/// The state key `text` spells, `None` where `text` is not the canonical
/// unpadded base64url spelling of any octets.
pub fn state_key_from_text(text: &str) -> Option<StateKey> {
    URL_SAFE_NO_PAD
        .decode(text)
        .ok()
        .map(StateKey::from_canonical_bytes)
}

/// FSpec FR-331's State locus `{key}`. Reading it checks the spelling, not
/// membership in a model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateLocus {
    key: StateKey,
}

impl StateLocus {
    /// The locus's members.
    pub const MEMBERS: [&'static str; 1] = ["key"];

    /// The state's key.
    pub fn key(&self) -> &StateKey {
        &self.key
    }

    /// The state's key, taken.
    pub fn into_key(self) -> StateKey {
        self.key
    }

    /// The written `key` member.
    pub fn key_text(&self) -> String {
        state_key_text(&self.key)
    }

    /// The locus `value` holds.
    ///
    /// # Errors
    /// [`WireDefect`] for anything but `{key}` with a canonical unpadded
    /// base64url `key`.
    pub fn read(value: &Value) -> Result<Self, WireDefect> {
        let map = closed_object(value, &Self::MEMBERS, &[])?;
        let text = member(map, "key")?
            .as_str()
            .ok_or_else(|| WireDefect::new("/key", WireDefectKind::WrongKind))?;
        state_key_from_text(text)
            .map(|key| Self { key })
            .ok_or_else(|| WireDefect::new("/key", WireDefectKind::Invalid))
    }
}

impl From<StateKey> for StateLocus {
    fn from(key: StateKey) -> Self {
        Self { key }
    }
}

impl From<&StateKey> for StateLocus {
    fn from(key: &StateKey) -> Self {
        Self { key: key.clone() }
    }
}

impl Serialize for StateLocus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("StateLocus", 1)?;
        object.serialize_field("key", &self.key_text())?;
        object.end()
    }
}

impl<'de> Deserialize<'de> for StateLocus {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::read(&unique_object(decoder)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use fsl_eval::simulation::StateKey;
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::StateLocus;
    use crate::shared_wire::{WireDefect, WireDefectKind};

    // FSpec FR-331: `[0,[]]` is the octets `5b 30 2c 5b 5d 5d`, whose unpadded
    // base64url is `WzAsW11d`.
    #[trace("FR-290-AC-9")]
    #[test]
    fn a_state_locus_is_the_key_object_both_ways() {
        let locus = StateLocus::from(StateKey::from_canonical_bytes(b"[0,[]]".to_vec()));
        let written = serde_json::to_value(&locus).expect("writes");
        assert_eq!(written, json!({"key": "WzAsW11d"}));
        assert_eq!(StateLocus::read(&written), Ok(locus.clone()));
        assert_eq!(
            serde_json::from_str::<StateLocus>(r#"{"key":"WzAsW11d"}"#).expect("reads"),
            locus
        );
    }

    #[trace("FR-290-AC-9")]
    #[test]
    fn a_state_locus_refuses_every_other_shape() {
        let defect = |pointer: &str, kind| {
            Err(WireDefect {
                pointer: pointer.to_owned(),
                kind,
            })
        };
        for (value, expected) in [
            (json!("WzAsW11d"), defect("", WireDefectKind::WrongKind)),
            (json!({}), defect("/key", WireDefectKind::Missing)),
            (
                json!({"key": "AA", "extra": 1}),
                defect("/extra", WireDefectKind::UnknownMember),
            ),
            (json!({"key": 1}), defect("/key", WireDefectKind::WrongKind)),
            // Padded, and with a non-zero unused trailing bit.
            (
                json!({"key": "AA=="}),
                defect("/key", WireDefectKind::Invalid),
            ),
            (
                json!({"key": "AB"}),
                defect("/key", WireDefectKind::Invalid),
            ),
        ] {
            assert_eq!(StateLocus::read(&value), expected, "{value}");
        }
        assert!(serde_json::from_str::<StateLocus>(r#"{"key":"AA","key":"AA"}"#).is_err());
    }
}
