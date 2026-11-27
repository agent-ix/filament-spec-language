// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418's DBM entry (`DbmBound`), which the zone certificate's zones
//! and LU bounds and FR-331's task-search evidence zones all carry.

use filament_exact::Integer;
use serde_json::{json, Value};

use super::{closed_object, member, WireDefect, WireDefectKind};

/// The spelling of an absent bound.
const INFINITY: &str = "infinity";

/// One DBM entry: `"infinity"`, or `{bound, strict}` with a canonical integer
/// string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbmBound(pub Option<(Integer, bool)>);

impl DbmBound {
    /// The finite entry's members.
    pub const MEMBERS: [&'static str; 2] = ["bound", "strict"];

    /// The written entry.
    pub fn value(&self) -> Value {
        match &self.0 {
            Some((bound, strict)) => json!({"bound": bound.to_string(), "strict": strict}),
            None => json!(INFINITY),
        }
    }

    /// The entry `value` holds.
    ///
    /// # Errors
    /// [`WireDefect`] naming the member at fault.
    #[ix_trace_rs::string_edge]
    pub fn read(value: &Value) -> Result<Self, WireDefect> {
        if value.as_str() == Some(INFINITY) {
            return Ok(Self(None));
        }
        let map = closed_object(value, &Self::MEMBERS, &[])?;
        let bound = member(map, "bound")?
            .as_str()
            .ok_or_else(|| WireDefect::new("/bound", WireDefectKind::WrongKind))?
            .parse::<Integer>()
            .map_err(|_| WireDefect::new("/bound", WireDefectKind::Invalid))?;
        let strict = member(map, "strict")?
            .as_bool()
            .ok_or_else(|| WireDefect::new("/strict", WireDefectKind::WrongKind))?;
        Ok(Self(Some((bound, strict))))
    }
}

#[cfg(test)]
mod tests {
    use filament_exact::Integer;
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::DbmBound;
    use crate::shared_wire::{WireDefect, WireDefectKind};

    // FSpec FR-418 `DbmBound`: an `IntegerString` bound with its strictness,
    // or `infinity`.
    #[trace("FSpec-FR-418")]
    #[test]
    fn a_dbm_entry_is_infinity_or_a_canonical_bound() {
        for (entry, written) in [
            (DbmBound(None), json!("infinity")),
            (
                DbmBound(Some((Integer::from(-4_i64), true))),
                json!({"bound": "-4", "strict": true}),
            ),
        ] {
            assert_eq!(entry.value(), written);
            assert_eq!(DbmBound::read(&written), Ok(entry));
        }
        for (value, pointer, kind) in [
            (json!("inf"), "", WireDefectKind::WrongKind),
            (json!({"bound": "0"}), "/strict", WireDefectKind::Missing),
            (
                json!({"bound": 0, "strict": false}),
                "/bound",
                WireDefectKind::WrongKind,
            ),
            (
                json!({"bound": "01", "strict": false}),
                "/bound",
                WireDefectKind::Invalid,
            ),
            (
                json!({"bound": "-0", "strict": false}),
                "/bound",
                WireDefectKind::Invalid,
            ),
            (
                json!({"bound": "0", "strict": 0}),
                "/strict",
                WireDefectKind::WrongKind,
            ),
        ] {
            assert_eq!(
                DbmBound::read(&value),
                Err(WireDefect {
                    pointer: pointer.to_owned(),
                    kind
                }),
                "{value}"
            );
        }
    }
}
