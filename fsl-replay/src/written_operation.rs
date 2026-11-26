// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120's written `operation` of a transition: the one type, and the one
//! rule, every writer and reader of that string uses.

use ix_trace_rs::implements;
use serde::Serialize;
use serde_json::Value;

/// The `operation` string of a written transition (FR-120).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct WrittenOperation(String);

impl WrittenOperation {
    /// The identity text `text` itself, as every arm but the task-source arm
    /// writes it.
    pub(crate) fn identity(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// What a task-source writer writes for the identity text `text`, whose
    /// parsed value is `operation`.
    pub(crate) fn task_source(operation: &Value, text: &str) -> Self {
        Self(Self::task_source_spelling(operation, text).to_owned())
    }

    /// The one `operation` string a task-source writer writes (FR-120): the
    /// identity text `text` itself, except that a name (a JSON string whose
    /// content is not itself an identity text) is written as that name, which
    /// the reader rule restores. Neither case copies.
    #[implements("FR-120-AC-40")]
    pub(crate) fn task_source_spelling<'a>(operation: &'a Value, text: &'a str) -> &'a str {
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

    /// The declaration identity text this string names, by FR-120's reader
    /// rule: an identity text is that text, and a name is its JSON string.
    ///
    /// # Errors
    /// The JSON encoder's, for a name it cannot write.
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

    // The identity text, what a task-source writer writes for it, and what
    // every reader reads back, all written by hand from FR-120.
    #[trace("TC-471", "FR-120-AC-39", "FR-120-AC-40")]
    #[test]
    fn the_written_operation_reads_back_as_its_identity_text() {
        for (identity, written) in [
            (
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
                r#"{"node":"ix://a/b/C/d","package":"a/b"}"#,
            ),
            (r#""Cell::set""#, "Cell::set"),
            (r#""{\"node\":1}""#, r#""{\"node\":1}""#),
        ] {
            let value = serde_json::from_str(identity).expect("JSON");
            let spelled = WrittenOperation::task_source(&value, identity);
            assert_eq!(spelled, WrittenOperation::from_wire(written.to_owned()));
            assert_eq!(spelled.into_identity_text().expect("text"), identity);
        }
    }
}
