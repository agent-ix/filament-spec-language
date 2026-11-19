// SPDX-License-Identifier: AGPL-3.0-or-later
//! A JSON document read with duplicate member names refused and no nesting
//! ceiling but its bytes: the reader of every untrusted request tree whose
//! depth only its byte count bounds (FR-336 replay requests, FR-350 corpus
//! documents).

use serde::Deserialize;
use serde_json::Value;

/// A JSON tree that is released without recursing, however deeply it nests.
pub(crate) struct Deep(Value);

impl Deep {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(mut self) -> Value {
        std::mem::take(&mut self.0)
    }
}

impl std::ops::Deref for Deep {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Deep {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for Deep {
    fn drop(&mut self) {
        fsl_foundation::node_wire::discard_node_json(std::mem::take(&mut self.0));
    }
}

/// Read `bytes` as one JSON document, refusing a repeated member name.
/// Numbers keep their original lexeme; nesting costs input bytes, not a
/// fixed depth. Release a deep result with
/// [`fsl_foundation::node_wire::discard_node_json`].
///
/// # Errors
///
/// The reader's message for malformed JSON or a duplicate member name.
pub fn read(bytes: &[u8]) -> std::result::Result<Value, String> {
    use quire_canonical::{Node, NodeRef, ReadError};

    // The file was admitted under the caller's byte ceiling. The shared
    // reader's arena and every conversion stack are bounded by those bytes;
    // neither introduces a second, hidden nesting ceiling.
    let byte_bound = u64::try_from(bytes.len()).map_err(|error| error.to_string())?;
    let document = quire_canonical::read(bytes, byte_bound).map_err(|error| match error {
        ReadError::DuplicateName { offset, .. } => {
            // The owner reports the start of the later member's JSON
            // string. Decode only that scalar to preserve its actual name,
            // including escapes, without walking the untrusted tree again.
            let name = bytes.get(offset..).and_then(|bytes| {
                String::deserialize(&mut serde_json::Deserializer::from_slice(bytes)).ok()
            });
            match name {
                Some(name) => format!("duplicate object key {name} at byte {offset}"),
                None => format!("duplicate object key at byte {offset}"),
            }
        }
        other => other.to_string(),
    })?;
    enum Task<'a> {
        Read(NodeRef<'a>),
        Array(usize),
        Object(Vec<&'a str>),
    }
    let mut tasks = vec![Task::Read(document.root())];
    let mut values = Vec::<Deep>::new();
    while let Some(task) = tasks.pop() {
        let value = match task {
            Task::Read(node) => match node.node() {
                Node::Null => Value::Null,
                Node::Bool(value) => Value::Bool(value),
                // The shared reader also exposes an f64 for canonical
                // writing. Decode the original scalar lexeme instead: a
                // qualification adapter must not round a supplied integer.
                Node::Number(number) => {
                    serde_json::from_str(number.text()).map_err(|error| error.to_string())?
                }
                Node::String(value) => Value::String(value.to_owned()),
                Node::Array(items) => {
                    let items = items.collect::<Vec<_>>();
                    tasks.push(Task::Array(items.len()));
                    tasks.extend(items.into_iter().rev().map(Task::Read));
                    continue;
                }
                Node::Object(members) => {
                    let members = members.collect::<Vec<_>>();
                    tasks.push(Task::Object(
                        members.iter().map(|(name, _)| *name).collect(),
                    ));
                    tasks.extend(
                        members
                            .into_iter()
                            .rev()
                            .map(|(_, value)| Task::Read(value)),
                    );
                    continue;
                }
            },
            Task::Array(length) => {
                let start = values
                    .len()
                    .checked_sub(length)
                    .ok_or("JSON array children")?;
                Value::Array(
                    values
                        .split_off(start)
                        .into_iter()
                        .map(Deep::into_inner)
                        .collect(),
                )
            }
            Task::Object(names) => {
                let start = values
                    .len()
                    .checked_sub(names.len())
                    .ok_or("JSON object children")?;
                Value::Object(
                    names
                        .into_iter()
                        .zip(values.split_off(start))
                        .map(|(name, value)| (name.to_owned(), value.into_inner()))
                        .collect(),
                )
            }
        };
        values.push(Deep::new(value));
    }
    values
        .pop()
        .map(Deep::into_inner)
        .ok_or_else(|| "missing JSON value".to_owned())
}
