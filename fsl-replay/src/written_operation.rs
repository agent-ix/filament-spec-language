// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120's written `operation` of a transition: the one type every writer
//! and reader of that string uses, the one rule every writer spells it by and
//! the one rule it is read back by.

use ix_trace_rs::implements;
use serde::Serialize;
use serde_json::Value;

/// The `operation` string of a written transition (FR-120).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct WrittenOperation(String);

impl WrittenOperation {
    /// The `operation` a writer writes for the identity text `text`. A text
    /// that is no JSON value is no name, and is written as itself.
    pub(crate) fn of(text: &str) -> Self {
        match serde_json::from_str::<Value>(text) {
            Ok(operation) => Self(Self::spelling(&operation, text).to_owned()),
            Err(_) => Self(text.to_owned()),
        }
    }

    /// FR-120's one writer rule, over the identity text `text` and its parsed
    /// value `operation`: a name identity (a JSON string whose content is a
    /// name) is written as that name, and every other identity as `text`
    /// itself. Neither case copies.
    #[implements("FR-120-AC-40")]
    pub(crate) fn spelling<'a>(operation: &'a Value, text: &'a str) -> &'a str {
        match operation.as_str() {
            Some(name) if !Self::is_identity_text(name) => name,
            _ => text,
        }
    }

    /// Whether a written `operation` string is itself an identity text
    /// (FR-120): the JSON text of an object or of a string. Any other string is
    /// a name.
    #[implements("FR-120-AC-39")]
    pub(crate) fn is_identity_text(wire: &str) -> bool {
        matches!(wire.trim_start().bytes().next(), Some(b'{' | b'"'))
            && serde_json::from_str::<serde::de::IgnoredAny>(wire).is_ok()
    }

    /// A string as read from a written transition.
    pub(crate) fn from_wire(wire: String) -> Self {
        Self(wire)
    }

    /// The identity text this string names, by FR-120's reader rule: an
    /// identity text is that text, and a name is its JSON string.
    ///
    /// # Errors
    /// The JSON encoder's, for a name it cannot write.
    #[implements("FR-120-AC-39", "FR-120-AC-40")]
    pub(crate) fn into_identity_text(self) -> Result<String, serde_json::Error> {
        if Self::is_identity_text(&self.0) {
            Ok(self.0)
        } else {
            serde_json::to_string(&self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WrittenOperation;
    use ix_trace_rs::trace;

    // The identity text, what every writer writes for it, and what every
    // reader reads back, all written by hand from FR-120.
    #[trace("TC-471", "FR-120-AC-39", "FR-120-AC-40")]
    #[test]
    fn the_written_operation_reads_back_as_its_identity_text() {
        for (identity, written) in [
            (
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
            ),
            (r#"{"name":"tick"}"#, r#"{"name":"tick"}"#),
            (r#""tick""#, "tick"),
            (r#""Cell::set""#, "Cell::set"),
            (r#""true""#, "true"),
            (r#""{}""#, r#""{}""#),
            (r#""{\"node\":1}""#, r#""{\"node\":1}""#),
        ] {
            let spelled = WrittenOperation::of(identity);
            assert_eq!(spelled, WrittenOperation::from_wire(written.to_owned()));
            assert_eq!(spelled.into_identity_text().expect("text"), identity);
        }
    }
}
