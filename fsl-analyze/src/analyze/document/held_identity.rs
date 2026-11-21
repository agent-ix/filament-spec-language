// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Operand index segments" for an analyze result: a document
//! outside the package writes each operand of a node id as the index segment
//! `{"node": i}` naming `semantic_graph.nodes[i]`, and FR-412's
//! `CertificateIdentity` names its owning `model`.
//!
//! The engines build a certificate, a counterexample and a locus with the
//! structural node id of each node, every operand a nested id. [`hold`]
//! rewrites one whole item through [`WireGraph::document_value`], the one
//! conversion the request and export routes also use, against the one graph
//! the item names: the request's own package, or the dependency its `package`
//! member names. A node id's own `package` member names its owner and never
//! picks a graph (FSpec FR-322 "Scope"). A reader of the result resolves the
//! form back with [`WireGraph::structural_value`].

use std::collections::BTreeMap;

use filament_exact::{node_wire::node_key_from_json, NodeKey, PathSegment};
use fsl_semantics::check::wire_id::{foreign_id_container, WireGraph};
use serde_json::Value;

use crate::analyze::GraphUnavailable;

/// Why an item could not be written in the document-held form.
#[derive(Debug, thiserror::Error)]
pub enum HeldIdentityError {
    /// A node id is no structural key, or names an operand that is no node of
    /// the graph the item is written against.
    #[error("a node id of the result has no place in its package graph: {0}")]
    NotInGraph(String),
    /// The item holds node ids and its package has no graph to write them
    /// against.
    #[error("the item holds node ids and its package has no graph: {0:?}")]
    NoGraph(GraphUnavailable),
}

/// Whether `value` is a FR-412 `{node_id, model, ordinal}` identity.
fn is_certificate_identity(value: &Value) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == 3
            && object.contains_key("node_id")
            && object.contains_key("model")
            && object.contains_key("ordinal")
    })
}

/// Whether `object` is a node id: exactly `package`, `path` and `ordinal`.
fn is_id(object: &serde_json::Map<String, Value>) -> bool {
    object.len() == 3
        && object.contains_key("package")
        && object.contains_key("path")
        && object.contains_key("ordinal")
}

/// Whether `value` holds a node id with a nested operand id, or an identity
/// whose `model` is null.
pub(super) fn needs_hold(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        if let Value::Object(object) = current {
            if foreign_id_container(object) {
                continue;
            }
        }
        let nested = current
            .get("path")
            .and_then(Value::as_array)
            .is_some_and(|path| path.iter().any(|segment| segment.get("package").is_some()));
        if nested || (is_certificate_identity(current) && current["model"].is_null()) {
            return true;
        }
        match current {
            Value::Object(object) => pending.extend(object.values()),
            Value::Array(items) => pending.extend(items.iter()),
            _ => {}
        }
    }
    false
}

/// Write every node id under `value` in the document-held form of `graph`,
/// and fill each identity's null `model` with `model`.
///
/// `composition` is the dependency map of an item whose `request.kind` is
/// `composition` and that has no `package` member: until FSpec FR-322 names a
/// segment for a node of a dependency's graph, such an item keeps in
/// structural form exactly the ids that have no held form in `graph` because
/// the id or one of its operands is a node of a direct dependency.
///
/// # Errors
///
/// [`HeldIdentityError::NotInGraph`] for an id that has no held form for any
/// other reason.
pub(super) fn hold(
    value: &Value,
    graph: &WireGraph,
    composition: Option<&BTreeMap<String, Result<WireGraph, GraphUnavailable>>>,
    model: Option<&str>,
) -> Result<Value, HeldIdentityError> {
    let mut held = match composition {
        None => graph
            .document_value(value)
            .map_err(HeldIdentityError::NotInGraph)?,
        Some(dependencies) => hold_composition(value, graph, dependencies)?,
    };
    if let Some(model) = model {
        let mut pending = vec![&mut held];
        while let Some(current) = pending.pop() {
            if is_certificate_identity(current) && current["model"].is_null() {
                current["model"] = Value::String(model.to_owned());
            }
            match current {
                Value::Object(object) => pending.extend(object.values_mut()),
                Value::Array(items) => pending.extend(items.iter_mut()),
                _ => {}
            }
        }
    }
    Ok(held)
}

/// [`hold`] for a composition item: each id is held in `graph`, except an id
/// that names a node of a direct dependency, which stays whole and structural.
fn hold_composition(
    value: &Value,
    graph: &WireGraph,
    dependencies: &BTreeMap<String, Result<WireGraph, GraphUnavailable>>,
) -> Result<Value, HeldIdentityError> {
    let mut held = value.clone();
    let mut pending = vec![&mut held];
    while let Some(current) = pending.pop() {
        match current {
            Value::Object(object) if foreign_id_container(object) => {}
            Value::Object(object) if is_id(object) => {
                let id = Value::Object(std::mem::take(object));
                match graph.document_value(&id) {
                    Ok(held) => *current = held,
                    Err(error) => {
                        let key = node_key_from_json(&id)
                            .map_err(|error| HeldIdentityError::NotInGraph(error.to_string()))?;
                        if !names_dependency(&key, graph, dependencies) {
                            return Err(HeldIdentityError::NotInGraph(error));
                        }
                        *current = id;
                    }
                }
            }
            Value::Object(object) => pending.extend(object.values_mut()),
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
    Ok(held)
}

/// Whether `key`'s package, or the package of an operand that is no node of
/// `graph`, is a direct dependency.
fn names_dependency(
    key: &NodeKey,
    graph: &WireGraph,
    dependencies: &BTreeMap<String, Result<WireGraph, GraphUnavailable>>,
) -> bool {
    if dependencies.contains_key(key.package()) {
        return true;
    }
    let mut pending = vec![key.clone()];
    while let Some(key) = pending.pop() {
        for segment in key.path() {
            let PathSegment::Node(operand) = segment else {
                continue;
            };
            if graph.index_of(&operand).is_some() {
                continue;
            }
            if dependencies.contains_key(operand.package()) {
                return true;
            }
            pending.push(operand);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn graph(package: &str) -> WireGraph {
        WireGraph::of_package_value(&json!({"semantic_graph": {"nodes": [
            {"node_id": {"package": package, "path": ["leaf"], "ordinal": 0}}
        ]}}))
        .expect("graph")
    }

    fn id(package: &str, operand_package: &str) -> Value {
        json!({"package": package, "ordinal": 0, "path": [
            "application",
            {"package": operand_package, "path": ["leaf"], "ordinal": 0}
        ]})
    }

    fn dependencies() -> BTreeMap<String, Result<WireGraph, GraphUnavailable>> {
        BTreeMap::from([("test/part-b".to_owned(), Ok(graph("test/part-b")))])
    }

    /// FR-621-AC-17: a composition item keeps the id with a dependency operand
    /// nested and holds every other id.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn a_composition_item_keeps_a_dependency_operand_id_nested() {
        let item = json!({
            "claim": id("own", "test/part-b"),
            "other": {"package": "own", "ordinal": 0, "path": [
                "application", {"package": "own", "path": ["leaf"], "ordinal": 0}]},
        });
        let held = hold(&item, &graph("own"), Some(&dependencies()), None).expect("held");
        assert_eq!(held["claim"], item["claim"]);
        assert_eq!(held["other"]["path"][1], json!({"node": 0}));
    }

    /// FR-621-AC-17: an item that is no composition, or that names a foreign
    /// operand no dependency owns, fails instead of staying nested.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn an_own_package_item_with_a_foreign_operand_fails() {
        let item = json!({"claim": id("own", "test/part-b")});
        assert!(matches!(
            hold(&item, &graph("own"), None, None),
            Err(HeldIdentityError::NotInGraph(_))
        ));
        assert!(matches!(
            hold(&item, &graph("own"), Some(&BTreeMap::new()), None),
            Err(HeldIdentityError::NotInGraph(_))
        ));
    }
}
