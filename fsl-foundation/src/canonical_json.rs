// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286: the shared RFC 8785 encoder for FSL JSON documents.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::cmp::Ordering;

/// Why a JSON document could not be encoded canonically.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalJsonError {
    /// An integer does not fit the exact IEEE 754 integer range.
    #[error("integer {value} at `{pointer}` is outside the exact JSON integer range")]
    UnsafeInteger {
        /// The RFC 6901 pointer to the integer, empty for the root value.
        pointer: String,
        /// The integer's decimal spelling.
        value: String,
    },
    /// The value could not be converted to JSON.
    #[error("value is not JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The RFC 8785 encoder refused the JSON value.
    #[error("RFC 8785 encoding failed: {0}")]
    Encode(#[from] quire_canonical::Error),
}

/// Encode `value` as RFC 8785 canonical JSON.
///
/// Object members are ordered by UTF-16 code units at every depth. Integers
/// outside the inclusive range `-(2^53 - 1)..=(2^53 - 1)` are refused with
/// their RFC 6901 pointer.
///
/// # Errors
///
/// Returns [`CanonicalJsonError::UnsafeInteger`] for an integer outside the
/// exact range, or a serialization/encoding error for values that cannot be
/// represented as canonical JSON.
pub fn encode<T: serde::Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, CanonicalJsonError> {
    let json = serde_json::to_value(value)?;
    reject_unsafe_integer(&json)?;
    Ok(quire_canonical::to_vec(
        &json,
        quire_canonical::Limits::new(u64::MAX),
    )?)
}

fn reject_unsafe_integer(root: &serde_json::Value) -> Result<(), CanonicalJsonError> {
    let mut pending = vec![(root, String::new())];
    while let Some((value, pointer)) = pending.pop() {
        match value {
            serde_json::Value::Number(number) => {
                if let Some(integer) = number.as_i64() {
                    if integer.unsigned_abs() > 9_007_199_254_740_991 {
                        return Err(CanonicalJsonError::UnsafeInteger {
                            pointer,
                            value: integer.to_string(),
                        });
                    }
                } else if let Some(integer) = number.as_u64() {
                    if integer > 9_007_199_254_740_991 {
                        return Err(CanonicalJsonError::UnsafeInteger {
                            pointer,
                            value: integer.to_string(),
                        });
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for (index, child) in values.iter().enumerate().rev() {
                    pending.push((child, child_pointer(&pointer, &index.to_string())));
                }
            }
            serde_json::Value::Object(members) => {
                let mut ordered: Vec<_> = members.iter().collect();
                ordered.sort_unstable_by(|(left, _), (right, _)| utf16_cmp(left, right));
                for (key, child) in ordered.into_iter().rev() {
                    pending.push((child, child_pointer(&pointer, key)));
                }
            }
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {
            }
        }
    }
    Ok(())
}

fn child_pointer(parent: &str, token: &str) -> String {
    let mut pointer = String::with_capacity(parent.len() + token.len() + 1);
    pointer.push_str(parent);
    pointer.push('/');
    for character in token.chars() {
        match character {
            '~' => pointer.push_str("~0"),
            '/' => pointer.push_str("~1"),
            other => pointer.push(other),
        }
    }
    pointer
}

fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// FR-286-AC-26 (TC-3111): object keys use UTF-16 code-unit order.
    #[trace("TC-3111", "FR-286-AC-26")]
    #[test]
    fn object_keys_use_utf16_code_unit_order() {
        let value = serde_json::json!({"\u{E000}": 1, "\u{1F600}": 2});
        let encoded = encode(&value).expect("the value is canonical JSON");
        assert_eq!(encoded, r#"{"😀":2,"":1}"#.as_bytes(),);
    }

    /// FR-286-AC-32 (TC-3117): unsafe integers refuse with their JSON pointer.
    #[trace("TC-3117", "FR-286-AC-32")]
    #[test]
    fn unsafe_integer_refusal_names_its_pointer() {
        let value = serde_json::json!({"n": 9_007_199_254_740_992_u64});
        let error = encode(&value).expect_err("the integer exceeds the exact range");
        assert!(matches!(
            error,
            CanonicalJsonError::UnsafeInteger { ref pointer, .. } if pointer == "/n"
        ));

        let boundary = serde_json::json!({"n": 9_007_199_254_740_991_u64});
        assert!(encode(&boundary).is_ok());
    }
}
