// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Operand index segments" for an analyze result: a document
//! outside the package writes each operand of a node id as the index segment
//! `{"node": i}` naming `semantic_graph.nodes[i]`, and FR-412's
//! `CertificateIdentity` names its owning `model`.
//!
//! The engines build a certificate, a counterexample and a locus with the
//! structural node id of each node, every operand a nested id. [`hold`]
//! rewrites one whole item through [`WireGraph::document_value`], the one
//! conversion the request and export routes also use, and gives an identity
//! whose `model` the engine left null the owning model's text. A reader of
//! the result resolves the form back with [`WireGraph::structural_value`].

use fsl_semantics::check::wire_id::WireGraph;
use serde_json::Value;

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
    NoGraph(crate::analyze::GraphUnavailable),
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

/// Whether `value` holds a node id with a nested operand id, or an identity
/// whose `model` is null.
pub(super) fn needs_hold(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
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
/// # Errors
///
/// [`HeldIdentityError::NotInGraph`] for an id that does not resolve in
/// `graph`.
pub(super) fn hold(
    value: &Value,
    graph: &WireGraph,
    model: Option<&str>,
) -> Result<Value, HeldIdentityError> {
    let mut held = match graph.document_value(value) {
        Ok(held) => held,
        Err(_) => hold_each(value, graph)?,
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

/// `value` with each nested node id the graph can name written in the held
/// form. An id with an operand that is no node of `graph` stays structural:
/// the pinned `NodeId` path takes only a string, a `{"node": i}` index and an
/// operand slot, so it has no spelling for a node of another package's graph.
/// A composition's own claim over a component's nodes is such an id; FSpec
/// FR-322 does not yet say how a result names it.
fn hold_each(value: &Value, graph: &WireGraph) -> Result<Value, HeldIdentityError> {
    let mut held = value.clone();
    let mut pending = vec![&mut held];
    while let Some(current) = pending.pop() {
        let nested = current
            .get("path")
            .and_then(Value::as_array)
            .is_some_and(|path| path.iter().any(|segment| segment.get("package").is_some()));
        if nested {
            if let Ok(id) = graph.document_value(current) {
                *current = id;
            }
            continue;
        }
        match current {
            Value::Object(object) => pending.extend(object.values_mut()),
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
    Ok(held)
}
