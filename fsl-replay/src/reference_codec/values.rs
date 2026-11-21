// SPDX-License-Identifier: AGPL-3.0-or-later
//! Postorder typed conversion with the caller's actual depth policy.

use super::{json::DeepJson, ReferenceCodec, ReferenceCodecError, ReferenceFailure, Scope};
use serde_json::Value;

enum Task<'v> {
    Visit(&'v Value, usize, String),
    Finish(&'v Value, usize, String, usize),
}

fn path(scope: &mut Scope<'_, '_>) -> Result<String, ReferenceCodecError> {
    let len = scope.path.len();
    scope.work(len)?;
    scope.reserve(len)?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(len)
        .map_err(|_| scope.allocation(len))?;
    owned.push_str(&scope.path);
    Ok(owned)
}

pub(super) fn push<T>(
    scope: &mut Scope<'_, '_>,
    list: &mut Vec<T>,
    value: T,
) -> Result<(), ReferenceCodecError> {
    scope.work(1)?;
    if list.len() == list.capacity() {
        let bytes = std::mem::size_of::<T>();
        scope.reserve(bytes)?;
        list.try_reserve_exact(1)
            .map_err(|_| scope.allocation(bytes))?;
    }
    list.push(value);
    Ok(())
}

impl<'c, 'm, 'r> ReferenceCodec<'c, 'm, 'r> {
    pub(super) fn value(
        &self,
        value: &Value,
        restore: bool,
        depth: usize,
        scope: &mut Scope<'_, 'm>,
    ) -> Result<Value, ReferenceCodecError> {
        let mut tasks = Vec::new();
        let root_path = path(scope)?;
        push(scope, &mut tasks, Task::Visit(value, depth, root_path))?;
        let mut built: Vec<DeepJson> = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(input, depth, at) => {
                    scope.path = at;
                    scope.work(1)?;
                    if depth > scope.max_depth {
                        return Err(scope.fail(ReferenceFailure::DepthLimit));
                    }
                    let mut children = Vec::new();
                    let kind = input
                        .get("type")
                        .and_then(Value::as_str)
                        .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
                    if kind == "option" && input.get("value").is_some_and(|value| !value.is_null())
                    {
                        scope.member("value", |scope| {
                            let at = path(scope)?;
                            push(scope, &mut children, (&input["value"], at))
                        })?;
                    }
                    let field = match kind {
                        "set" | "bag" | "sequence" | "ordered-set" => Some("elements"),
                        "tuple" | "union" => Some("components"),
                        "record" => Some("fields"),
                        "map" => Some("entries"),
                        _ => None,
                    };
                    if let Some(field) = field {
                        scope.member(field, |scope| {
                            let array = input[field]
                                .as_array()
                                .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
                            for (index, child) in array.iter().enumerate() {
                                scope.reserve(20)?;
                                let index = index.to_string();
                                scope.member(&index, |scope| {
                                    if matches!(field, "entries" | "fields") {
                                        let names: &[&str] = if field == "entries" {
                                            &["key", "value"]
                                        } else {
                                            &["name", "value"]
                                        };
                                        let row = scope.closed(child, names)?;
                                        for name in names.iter().filter(|name| **name != "name") {
                                            if field == "fields" && row[*name].is_null() {
                                                continue;
                                            }
                                            scope.member(name, |scope| {
                                                let at = path(scope)?;
                                                push(scope, &mut children, (&row[*name], at))
                                            })?;
                                        }
                                    } else {
                                        let at = path(scope)?;
                                        push(scope, &mut children, (child, at))?;
                                    }
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })?;
                    }
                    let parent = path(scope)?;
                    push(
                        scope,
                        &mut tasks,
                        Task::Finish(input, depth, parent, children.len()),
                    )?;
                    for (child, at) in children.into_iter().rev() {
                        push(
                            scope,
                            &mut tasks,
                            Task::Visit(child, depth.saturating_add(1), at),
                        )?;
                    }
                }
                Task::Finish(input, depth, at, count) => {
                    scope.path = at;
                    let bytes = count
                        .checked_mul(std::mem::size_of::<DeepJson>())
                        .ok_or_else(|| scope.stopped(super::ModelKeyError::SizeOverflow))?;
                    scope.reserve(bytes)?;
                    let mut children = Vec::new();
                    children
                        .try_reserve_exact(count)
                        .map_err(|_| scope.allocation(bytes))?;
                    for _ in 0..count {
                        children.push(built.pop().expect("converted child"));
                    }
                    let mut children = children
                        .into_iter()
                        .rev()
                        .map(|mut value| value.0.take().expect("converted value"));
                    let output =
                        self.value_with_children(input, restore, depth, scope, &mut children)?;
                    push(scope, &mut built, DeepJson(Some(output)))?;
                }
            }
        }
        Ok(built
            .pop()
            .expect("converted root")
            .0
            .take()
            .expect("root value"))
    }
}
