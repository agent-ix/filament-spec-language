// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Operand index segments" for an analyze result: a document
//! outside the package writes each operand of a node id as the index segment
//! `{"node": i}` naming `semantic_graph.nodes[i]`, and FR-412's
//! `CertificateIdentity` names its owning `model`.
//!
//! The engines build a certificate with the structural node id of its
//! obligation, each operand a nested id. [`hold`] rewrites every such id in
//! one item's certificate through [`WireGraph::wire_value`], the one
//! conversion the request and export routes also use, and gives an identity
//! whose `model` the engine left null the owning model's text. A reader of
//! the result resolves the form back with [`WireGraph::structural_value`].

use filament_exact::node_wire::node_key_from_json;
use fsl_semantics::check::wire_id::{WireGraph, WireIdError};
use serde_json::Value;

/// Why an identity could not be written in the document-held form.
#[derive(Debug, thiserror::Error)]
pub(super) enum HeldIdentityError {
    /// A node id is not a structural key.
    #[error("a certificate node id is not a structural key: {0}")]
    NotAKey(String),
    /// A node id cannot be written against the package graph.
    #[error("a certificate node id has no place in the package graph: {0}")]
    NotInGraph(String),
}

/// Whether `value` is a node id object whose path holds a nested id.
fn is_structural_id(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 3
        && object.contains_key("package")
        && object.contains_key("ordinal")
        && object
            .get("path")
            .and_then(Value::as_array)
            .is_some_and(|path| path.iter().any(|segment| segment.get("package").is_some()))
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

/// Write every node id under `value` in the document-held form of `graph`,
/// and fill each identity's null `model` with `model`.
///
/// # Errors
///
/// [`HeldIdentityError`] for an id that is no structural key or names a node
/// outside `graph`.
pub(super) fn hold(
    value: &mut Value,
    graph: &WireGraph,
    model: Option<&str>,
) -> Result<(), HeldIdentityError> {
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        if is_structural_id(current) {
            let key = node_key_from_json(current)
                .map_err(|error| HeldIdentityError::NotAKey(error.to_string()))?;
            match graph.wire_value(&key) {
                Ok(held) => *current = held,
                // An operand of another package's graph (a composition's
                // claim over a dependency node) has no index in this
                // package's `semantic_graph.nodes`; the id keeps its
                // structural form until FR-322 names that operand.
                Err(WireIdError::OperandNotInGraph { .. }) => {}
                Err(error) => return Err(HeldIdentityError::NotInGraph(error.to_string())),
            }
            continue;
        }
        if is_certificate_identity(current) {
            if let (Some(model), Some(slot)) = (model, current.get_mut("model")) {
                if slot.is_null() {
                    *slot = Value::String(model.to_owned());
                }
            }
        }
        match current {
            Value::Object(object) => pending.extend(object.values_mut()),
            Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
    Ok(())
}

/// Whether `value` holds a node id [`hold`] would rewrite or an identity
/// whose `model` it would fill.
pub(super) fn needs_hold(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        if is_structural_id(current)
            || (is_certificate_identity(current) && current["model"].is_null())
        {
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
