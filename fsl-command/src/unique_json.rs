// SPDX-License-Identifier: AGPL-3.0-or-later
//! A JSON document read with duplicate member names refused and no nesting
//! ceiling but its bytes: the reader of every untrusted request tree whose
//! depth only its byte count bounds (FR-336 replay requests, FR-350 corpus
//! documents).

use serde::Deserialize;
use serde_json::Value;

/// A JSON tree whose copy, serialization, debug view and release never
/// recurse over it, however deeply it nests (ADR-030 D-1: those of a
/// [`Value`] recurse once per level). Serialization writes through
/// [`serde_json`]'s raw value, so it is for a JSON text writer, not for a
/// conversion back to a bare [`Value`].
pub struct Deep(Value);

impl Deep {
    /// Own `value`.
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self(value)
    }

    /// Copy `value`, a tree borrowed from anywhere, into a new owner.
    #[must_use]
    pub fn copy(value: &Value) -> Self {
        enum Task<'a> {
            Read(&'a Value),
            Array(usize),
            Object(Vec<&'a str>),
        }
        let mut tasks = vec![Task::Read(value)];
        let mut values = Vec::<Self>::new();
        while let Some(task) = tasks.pop() {
            // Every close task follows exactly its immutable source's
            // children, so the saturating start is that count.
            let copied = match task {
                Task::Read(Value::Array(items)) => {
                    tasks.push(Task::Array(items.len()));
                    tasks.extend(items.iter().rev().map(Task::Read));
                    continue;
                }
                Task::Read(Value::Object(members)) => {
                    tasks.push(Task::Object(members.keys().map(String::as_str).collect()));
                    tasks.extend(members.values().rev().map(Task::Read));
                    continue;
                }
                Task::Read(scalar) => scalar.clone(),
                Task::Array(count) => Value::Array(
                    values
                        .split_off(values.len().saturating_sub(count))
                        .into_iter()
                        .map(Self::into_inner)
                        .collect(),
                ),
                Task::Object(names) => {
                    let children = values.split_off(values.len().saturating_sub(names.len()));
                    Value::Object(
                        names
                            .into_iter()
                            .zip(children)
                            .map(|(name, value)| (name.to_owned(), value.into_inner()))
                            .collect(),
                    )
                }
            };
            values.push(Self(copied));
        }
        values.pop().unwrap_or(Self(Value::Null))
    }

    /// The compact JSON text of `value`, byte for byte what
    /// [`serde_json::to_vec`] writes, with the framing walked on an explicit
    /// stack.
    ///
    /// # Errors
    ///
    /// The JSON writer's error for a scalar it cannot write.
    pub fn text_of(value: &Value) -> serde_json::Result<Vec<u8>> {
        enum Task<'a> {
            Value(&'a Value),
            Name(&'a str),
            Byte(u8),
        }
        let mut bytes = Vec::new();
        let mut pending = vec![Task::Value(value)];
        while let Some(task) = pending.pop() {
            match task {
                Task::Byte(byte) => bytes.push(byte),
                Task::Name(name) => serde_json::to_writer(&mut bytes, name)?,
                Task::Value(Value::Array(items)) => {
                    bytes.push(b'[');
                    pending.push(Task::Byte(b']'));
                    for (index, value) in items.iter().enumerate().rev() {
                        pending.push(Task::Value(value));
                        if index != 0 {
                            pending.push(Task::Byte(b','));
                        }
                    }
                }
                Task::Value(Value::Object(members)) => {
                    bytes.push(b'{');
                    pending.push(Task::Byte(b'}'));
                    for (index, (name, value)) in members.iter().enumerate().rev() {
                        pending.push(Task::Value(value));
                        pending.push(Task::Byte(b':'));
                        pending.push(Task::Name(name));
                        if index != 0 {
                            pending.push(Task::Byte(b','));
                        }
                    }
                }
                Task::Value(scalar) => serde_json::to_writer(&mut bytes, scalar)?,
            }
        }
        Ok(bytes)
    }

    /// Take the tree out. The caller then releases it without recursing, with
    /// [`fsl_foundation::node_wire::discard_node_json`].
    #[must_use]
    pub fn into_inner(mut self) -> Value {
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

impl Clone for Deep {
    fn clone(&self) -> Self {
        Self::copy(&self.0)
    }
}

/// The shape only: a container is named with its length, never walked.
impl std::fmt::Debug for Deep {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Value::Array(items) => formatter
                .debug_struct("JsonArray")
                .field("length", &items.len())
                .finish(),
            Value::Object(members) => formatter
                .debug_struct("JsonObject")
                .field("length", &members.len())
                .finish(),
            scalar => std::fmt::Debug::fmt(scalar, formatter),
        }
    }
}

impl serde::Serialize for Deep {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let bytes = Self::text_of(&self.0).map_err(serde::ser::Error::custom)?;
        let text = String::from_utf8(bytes).map_err(serde::ser::Error::custom)?;
        serde_json::value::RawValue::from_string(text)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Deep {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        // RawValue skips the arbitrary JSON subtree iteratively; `read` then
        // verifies uniqueness and builds the owned tree.
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        read(raw.get().as_bytes()).map_err(serde::de::Error::custom)
    }
}

impl Drop for Deep {
    fn drop(&mut self) {
        fsl_foundation::node_wire::discard_node_json(std::mem::take(&mut self.0));
    }
}

/// Read `bytes` as one JSON document, refusing a repeated member name.
/// Numbers keep their original lexeme; nesting costs input bytes, not a
/// fixed depth. The result releases its tree without recursing, so a caller
/// that only asks whether the document reads may drop it at once.
///
/// # Errors
///
/// The reader's message for malformed JSON or a duplicate member name.
pub fn read(bytes: &[u8]) -> std::result::Result<Deep, String> {
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
    values.pop().ok_or_else(|| "missing JSON value".to_owned())
}
