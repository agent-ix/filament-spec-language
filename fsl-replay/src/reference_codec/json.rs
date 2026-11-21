// SPDX-License-Identifier: AGPL-3.0-or-later
//! Convert the shared stack-backed JSON admission without recursive serde reads.

use super::{Admission, ModelKeyError, Provision};
use quire_canonical::{Items, Members, Node, NodeRef, ReadError};
use serde_json::{Map, Value};

pub(super) const MALFORMED: &str = "malformed admitted JSON";
pub(super) const DEPTH: &str = "caller JSON depth";

pub(super) struct DeepJson(pub(super) Option<Value>);
impl Drop for DeepJson {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            fsl_foundation::node_wire::discard_node_json(value);
        }
    }
}

pub(super) struct DeepMap(pub(super) Map<String, Value>);
impl std::ops::Deref for DeepMap {
    type Target = Map<String, Value>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for DeepMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for DeepMap {
    fn drop(&mut self) {
        fsl_foundation::node_wire::discard_node_json(Value::Object(std::mem::take(&mut self.0)));
    }
}
pub(super) struct DeepArray(pub(super) Vec<Value>);
impl std::ops::Deref for DeepArray {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for DeepArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for DeepArray {
    fn drop(&mut self) {
        fsl_foundation::node_wire::discard_node_json(Value::Array(std::mem::take(&mut self.0)));
    }
}

enum Input<'d> {
    Parsed(NodeRef<'d>),
    Borrowed(&'d Value),
}
enum Array<'d> {
    Parsed(Items<'d>),
    Borrowed(std::slice::Iter<'d, Value>),
}
impl Array<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Parsed(items) => items.len(),
            Self::Borrowed(items) => items.len(),
        }
    }
}
impl<'d> Iterator for Array<'d> {
    type Item = Input<'d>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Parsed(items) => items.next().map(Input::Parsed),
            Self::Borrowed(items) => items.next().map(Input::Borrowed),
        }
    }
}
enum Object<'d> {
    Parsed(Members<'d>),
    Borrowed(serde_json::map::Iter<'d>),
}
impl<'d> Iterator for Object<'d> {
    type Item = (&'d str, Input<'d>);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Parsed(items) => items.next().map(|(key, value)| (key, Input::Parsed(value))),
            Self::Borrowed(items) => items
                .next()
                .map(|(key, value)| (key.as_str(), Input::Borrowed(value))),
        }
    }
}
enum Kind<'d> {
    Null,
    Bool(bool),
    Number(&'d str),
    NativeNumber(&'d serde_json::Number),
    Text(&'d str),
    Array(Array<'d>),
    Object(Object<'d>),
}
impl<'d> Input<'d> {
    fn kind(self) -> Kind<'d> {
        match self {
            Self::Parsed(node) => match node.node() {
                Node::Null => Kind::Null,
                Node::Bool(value) => Kind::Bool(value),
                Node::Number(value) => Kind::Number(value.text()),
                Node::String(value) => Kind::Text(value),
                Node::Array(value) => Kind::Array(Array::Parsed(value)),
                Node::Object(value) => Kind::Object(Object::Parsed(value)),
            },
            Self::Borrowed(node) => match node {
                Value::Null => Kind::Null,
                Value::Bool(value) => Kind::Bool(*value),
                Value::Number(value) => Kind::NativeNumber(value),
                Value::String(value) => Kind::Text(value),
                Value::Array(value) => Kind::Array(Array::Borrowed(value.iter())),
                Value::Object(value) => Kind::Object(Object::Borrowed(value.iter())),
            },
        }
    }
}
enum Task<'d> {
    Read(Input<'d>, usize),
    Array(Array<'d>, DeepJson, usize),
    Object(Object<'d>, DeepJson, Option<&'d str>, usize),
}

impl Admission<'_> {
    pub(super) fn push<T>(&self, list: &mut Vec<T>, value: T) -> Result<(), ModelKeyError> {
        self.work(1)?;
        if list.len() == list.capacity() {
            let bytes = std::mem::size_of::<T>();
            self.reserve(bytes)?;
            list.try_reserve_exact(1).map_err(|_| {
                ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Allocation {
                    requested: bytes,
                })
            })?;
        }
        list.push(value);
        Ok(())
    }

    fn json_text(&self, text: &str) -> Result<String, ModelKeyError> {
        self.work(text.len())?;
        self.reserve(text.len())?;
        let mut owned = String::new();
        owned.try_reserve_exact(text.len()).map_err(|_| {
            ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Allocation {
                requested: text.len(),
            })
        })?;
        owned.push_str(text);
        Ok(owned)
    }

    pub(super) fn json_value(&self, bytes: &[u8]) -> Result<Value, ModelKeyError> {
        self.work(bytes.len())?;
        let document =
            quire_canonical::read_with_reservation(bytes, u64::MAX, &mut Provision(self)).map_err(
                |error| match error {
                    ReadError::Allocation { requested } => {
                        ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Allocation {
                            requested,
                        })
                    }
                    ReadError::ReservationDenied { .. } => self
                        .failure
                        .borrow_mut()
                        .take()
                        .unwrap_or(ModelKeyError::Denied),
                    ReadError::Limit(_) => ModelKeyError::SizeOverflow,
                    ReadError::Malformed { .. }
                    | ReadError::NumberOutOfRange { .. }
                    | ReadError::DuplicateName { .. } => ModelKeyError::Unsupported(MALFORMED),
                },
            )?;
        self.build_json(Input::Parsed(document.root()), usize::MAX)
    }

    pub(super) fn borrowed_json(
        &self,
        value: &Value,
        max_depth: usize,
    ) -> Result<Value, ModelKeyError> {
        self.build_json(Input::Borrowed(value), max_depth)
    }

    fn build_json(&self, input: Input<'_>, max_depth: usize) -> Result<Value, ModelKeyError> {
        let mut tasks = Vec::new();
        self.push(&mut tasks, Task::Read(input, 0))?;
        let mut result = DeepJson(None);
        while let Some(task) = tasks.pop() {
            self.work(1)?;
            match task {
                Task::Read(node, depth) => {
                    if depth > max_depth {
                        return Err(ModelKeyError::Unsupported(DEPTH));
                    }
                    match node.kind() {
                        Kind::Null => result.0 = Some(Value::Null),
                        Kind::Bool(value) => result.0 = Some(Value::Bool(value)),
                        Kind::NativeNumber(number) => {
                            result.0 = Some(Value::Number(number.clone()))
                        }
                        Kind::Number(number) => {
                            result.0 = Some(Value::Number(
                                number
                                    .parse()
                                    .map_err(|_| ModelKeyError::Unsupported(MALFORMED))?,
                            ))
                        }
                        Kind::Text(text) => result.0 = Some(Value::String(self.json_text(text)?)),
                        Kind::Array(items) => {
                            let bytes = items
                                .len()
                                .checked_mul(std::mem::size_of::<Value>())
                                .ok_or(ModelKeyError::SizeOverflow)?;
                            self.reserve(bytes)?;
                            let mut values = Vec::new();
                            values.try_reserve_exact(items.len()).map_err(|_| {
                                ModelKeyError::Encoding(
                                    fsl_eval::simulation::EncodingRefusal::Allocation {
                                        requested: bytes,
                                    },
                                )
                            })?;
                            self.push(
                                &mut tasks,
                                Task::Array(items, DeepJson(Some(Value::Array(values))), depth),
                            )?;
                        }
                        Kind::Object(members) => self.push(
                            &mut tasks,
                            Task::Object(
                                members,
                                DeepJson(Some(Value::Object(Map::new()))),
                                None,
                                depth,
                            ),
                        )?,
                    }
                }
                Task::Array(mut items, mut out, depth) => {
                    if let Some(value) = result.0.take() {
                        out.0
                            .as_mut()
                            .and_then(Value::as_array_mut)
                            .expect("array frame")
                            .push(value);
                    }
                    match items.next() {
                        Some(child) => {
                            self.push(&mut tasks, Task::Array(items, out, depth))?;
                            self.push(&mut tasks, Task::Read(child, depth.saturating_add(1)))?;
                        }
                        None => result.0 = out.0.take(),
                    }
                }
                Task::Object(mut members, mut out, name, depth) => {
                    if let Some(name) = name {
                        self.reserve(
                            std::mem::size_of::<(String, Value)>()
                                .checked_mul(4)
                                .ok_or(ModelKeyError::SizeOverflow)?,
                        )?;
                        let key = self.json_text(name)?;
                        let value = result.0.take().expect("completed object child");
                        out.0
                            .as_mut()
                            .and_then(Value::as_object_mut)
                            .expect("object frame")
                            .insert(key, value);
                    }
                    match members.next() {
                        Some((name, child)) => {
                            self.push(&mut tasks, Task::Object(members, out, Some(name), depth))?;
                            self.push(&mut tasks, Task::Read(child, depth.saturating_add(1)))?;
                        }
                        None => result.0 = out.0.take(),
                    }
                }
            }
        }
        result.0.take().ok_or(ModelKeyError::Unsupported(MALFORMED))
    }
}
