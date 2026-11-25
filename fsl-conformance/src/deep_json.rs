// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deep-safe JSON intake for parity corpus documents.

use serde_json::Value;

/// A JSON tree whose final drop is iterative, even for deeply nested data.
pub(crate) struct DeepJson(Value);

impl DeepJson {
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, String> {
        use quire_canonical::{Node, NodeRef};

        let byte_bound = u64::try_from(bytes.len()).map_err(|error| error.to_string())?;
        let document = quire_canonical::read(bytes, byte_bound).map_err(|error| error.to_string())?;
        enum Task<'a> {
            Read(NodeRef<'a>),
            Array(usize),
            Object(Vec<&'a str>),
        }
        let mut tasks = vec![Task::Read(document.root())];
        let mut values = Vec::<DeepJson>::new();
        while let Some(task) = tasks.pop() {
            let value = match task {
                Task::Read(node) => match node.node() {
                    Node::Null => Value::Null,
                    Node::Bool(value) => Value::Bool(value),
                    Node::Number(number) => serde_json::from_str(number.text())
                        .map_err(|error| error.to_string())?,
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
                        .ok_or_else(|| "JSON array children".to_owned())?;
                    Value::Array(
                        values
                            .split_off(start)
                            .into_iter()
                            .map(DeepJson::into_inner)
                            .collect(),
                    )
                }
                Task::Object(names) => {
                    let start = values
                        .len()
                        .checked_sub(names.len())
                        .ok_or_else(|| "JSON object children".to_owned())?;
                    Value::Object(
                        names
                            .into_iter()
                            .zip(values.split_off(start))
                            .map(|(name, value)| (name.to_owned(), value.into_inner()))
                            .collect(),
                    )
                }
            };
            values.push(Self(value));
        }
        values
            .pop()
            .map(Self::into_inner)
            .map(Self)
            .ok_or_else(|| "missing JSON value".to_owned())
    }

    fn into_inner(mut self) -> Value {
        std::mem::take(&mut self.0)
    }
}

impl std::ops::Deref for DeepJson {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for DeepJson {
    fn drop(&mut self) {
        fsl_foundation::node_wire::discard_node_json(std::mem::take(&mut self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::DeepJson;

    #[test]
    fn deep_documents_are_read_and_dropped_without_recursive_json_walks() {
        let depth = 4096;
        let mut bytes = vec![b'['; depth];
        bytes.push(b'0');
        bytes.extend(std::iter::repeat_n(b']', depth));
        let document = DeepJson::read(&bytes).expect("arena-backed JSON reader accepts depth");
        assert!(document.is_array());
        drop(document);
    }

    #[test]
    fn duplicate_member_names_are_refused() {
        assert!(DeepJson::read(br#"{"kind":1,"kind":2}"#).is_err());
    }
}
