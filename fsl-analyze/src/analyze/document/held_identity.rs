// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Operand index segments" for an analyze result: a document
//! outside the package writes each operand of a node id as the index segment
//! `{"node": i}` naming `semantic_graph.nodes[i]`. FR-412's
//! `CertificateIdentity` names its owning `model`, and its `node_id` stays the
//! structural id with every operand nested (FSpec `CertificateNodeId`).
//!
//! The engines build a certificate, a counterexample and a locus with the
//! structural node id of each node, every operand a nested id. [`hold`]
//! rewrites one whole item through [`WireGraph::document_value`], the one
//! conversion the request and export routes also use, against the one graph
//! the item names: the request's own package, or the dependency its `package`
//! member names; a `composition` item writes the ids of a component slot in the
//! component's graph. A node id's own `package` member names its owner and never
//! picks a graph (FSpec FR-322 "Scope"). A reader of the result resolves the
//! form back with [`WireGraph::structural_value`].

use std::collections::BTreeMap;

use fsl_semantics::check::wire_id::{
    certificate_identity, component_key_held, foreign_id_container, key_subject_package,
    peer_graphs, WireGraph,
};
use fsl_semantics::check::ComponentPackages;
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
    value.as_object().is_some_and(certificate_identity)
}

/// Whether `value` holds a node id with a nested operand id outside a
/// certificate identity, or an identity whose `model` is null.
pub(super) fn needs_hold(value: &Value) -> bool {
    let mut pending = vec![value];
    while let Some(current) = pending.pop() {
        if let Value::Object(object) = current {
            if foreign_id_container(object) {
                continue;
            }
        }
        if is_certificate_identity(current) {
            if current["model"].is_null() {
                return true;
            }
            continue;
        }
        let nested = current
            .get("path")
            .and_then(Value::as_array)
            .is_some_and(|path| path.iter().any(|segment| segment.get("package").is_some()));
        if nested {
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

/// The graphs a `composition` item's component ids resolve against (FSpec
/// FR-322 "Scope").
pub(super) struct ComponentScope<'a> {
    /// The package of each `components` entry by name, and of the component
    /// of each guarantee clause.
    pub components: &'a ComponentPackages,
    /// The graph of each admitted dependency package, by identity.
    pub graphs: &'a BTreeMap<String, Result<WireGraph, GraphUnavailable>>,
}

impl ComponentScope<'_> {
    /// The graph of component package `package`, which must be a `components`
    /// entry's package.
    fn graph(&self, package: &str) -> Result<&WireGraph, HeldIdentityError> {
        if !self.components.holds_package(package) {
            return Err(HeldIdentityError::NotInGraph(format!(
                "{package} is the package of no component of the composition"
            )));
        }
        match self.graphs.get(package) {
            Some(Ok(graph)) => Ok(graph),
            Some(Err(cause)) => Err(HeldIdentityError::NoGraph(cause.clone())),
            None => Err(HeldIdentityError::NoGraph(GraphUnavailable::NotEmitted)),
        }
    }
}

/// Write every node id under `value` in the document-held form of `graph`,
/// except each certificate identity's `node_id`, which stays structural, and
/// fill each identity's null `model` with `model`.
///
/// `composition` is the scope of an item whose `request.kind` is
/// `composition` and that has no `package` member: each `component-guarantee`
/// key of its `premises` is written against the graph of the package its
/// subject identity carries, and each `interface-mismatch` `interface`
/// against the graph of the package of its `component` (FSpec FR-322
/// "Scope"). Every other id is written against `graph`.
///
/// # Errors
///
/// [`HeldIdentityError::NotInGraph`] for an id that has no held form.
pub(super) fn hold(
    value: &Value,
    graph: &WireGraph,
    composition: Option<&ComponentScope<'_>>,
    model: Option<&str>,
) -> Result<Value, HeldIdentityError> {
    let mut held = match composition {
        None => graph
            .document_value(value)
            .map_err(HeldIdentityError::NotInGraph)?,
        Some(scope) => hold_composition(value, graph, scope)?,
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

/// The RFC 6901 pointers, in `value`, of the ids a composition item writes
/// against a component graph, each with the package of that graph.
fn component_slots(
    value: &Value,
    scope: &ComponentScope<'_>,
) -> Result<Vec<(String, String, bool)>, HeldIdentityError> {
    let mut slots = Vec::new();
    if let Some(premises) = value["request"]["premises"].as_array() {
        for (at, premise) in premises.iter().enumerate() {
            let Some(key) = premise.get("key") else {
                continue;
            };
            if premise["role"] != "component-guarantee" {
                continue;
            }
            let package = key_subject_package(key).ok_or_else(|| {
                HeldIdentityError::NotInGraph(format!("premise {at} carries no subject identity"))
            })?;
            slots.push((
                format!("/request/premises/{at}/key"),
                package.to_owned(),
                true,
            ));
        }
    }
    let mut pending = vec![(String::new(), value)];
    while let Some((pointer, current)) = pending.pop() {
        match current {
            Value::Object(members) => {
                if members.get("type").and_then(Value::as_str) == Some("interface-mismatch")
                    && members.contains_key("interface")
                {
                    let component = members.get("component").and_then(Value::as_str);
                    let package = component
                        .and_then(|name| scope.components.of_component(name))
                        .ok_or_else(|| {
                            HeldIdentityError::NotInGraph(format!(
                                "an interface-mismatch names no component: {component:?}"
                            ))
                        })?;
                    slots.push((format!("{pointer}/interface"), package.to_owned(), false));
                }
                for (name, member) in members {
                    let name = name.replace('~', "~0").replace('/', "~1");
                    pending.push((format!("{pointer}/{name}"), member));
                }
            }
            Value::Array(items) => {
                for (at, item) in items.iter().enumerate() {
                    pending.push((format!("{pointer}/{at}"), item));
                }
            }
            _ => {}
        }
    }
    Ok(slots)
}

/// [`hold`] for a composition item: the ids of a component slot are held in
/// the component's graph, every other id in `graph`.
fn hold_composition(
    value: &Value,
    graph: &WireGraph,
    scope: &ComponentScope<'_>,
) -> Result<Value, HeldIdentityError> {
    let slots = component_slots(value, scope)?;
    let mut rest = value.clone();
    let mut taken = Vec::with_capacity(slots.len());
    for (pointer, package, key) in slots {
        let Some(slot) = rest.pointer_mut(&pointer) else {
            continue;
        };
        taken.push((pointer, package, key, slot.take()));
    }
    let mut held = graph
        .document_value(&rest)
        .map_err(HeldIdentityError::NotInGraph)?;
    let peers = |clause: &filament_exact::NodeKey| {
        let package = scope.components.of_clause(clause)?;
        match scope.graphs.get(package) {
            Some(Ok(graph)) => Some(graph),
            _ => None,
        }
    };
    for (pointer, package, key, id) in taken {
        let graph = scope.graph(&package)?;
        let written = if key {
            peer_graphs(&id, &peers).and_then(|peers| component_key_held(graph, &id, &peers))
        } else {
            graph.document_value(&id)
        }
        .map_err(HeldIdentityError::NotInGraph)?;
        if let Some(slot) = held.pointer_mut(&pointer) {
            *slot = written;
        }
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn graph(package: &str) -> WireGraph {
        WireGraph::of_package_value(&json!({"semantic_graph": {"nodes": [
            {"node_id": {"package": package, "path": ["leaf"], "ordinal": 0}},
            {"node_id": {"package": package, "path": ["wrap"], "ordinal": 0}}
        ]}}))
        .expect("graph")
    }

    fn id(package: &str, operand_package: &str, leaf: &str) -> Value {
        json!({"package": package, "ordinal": 0, "path": [
            "application",
            {"package": operand_package, "path": [leaf], "ordinal": 0}
        ]})
    }

    fn graphs() -> BTreeMap<String, Result<WireGraph, GraphUnavailable>> {
        BTreeMap::from([("test/part-b".to_owned(), Ok(graph("test/part-b")))])
    }

    fn components() -> ComponentPackages {
        ComponentPackages::new(
            BTreeMap::from([("B".to_owned(), "test/part-b".to_owned())]),
            BTreeMap::new(),
        )
    }

    fn item() -> Value {
        json!({"request": {"kind": "composition", "premises": [{
            "role": "component-guarantee",
            "key": {"claim": {"node": id("test/part-b", "test/part-b", "wrap")},
                "temporal": {"subject": {"identity": "test/part-b using test/m"}}}}]}})
    }

    /// FR-621-AC-17: a component-guarantee key is held in the component's
    /// graph, an own id in the request's graph.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn a_component_key_is_held_in_the_component_graph() {
        let components = components();
        let graphs = graphs();
        let scope = ComponentScope {
            components: &components,
            graphs: &graphs,
        };
        let mut value = item();
        value["claim"] = id("own", "own", "leaf");
        let held = hold(&value, &graph("own"), Some(&scope), None).expect("held");
        assert_eq!(
            held["request"]["premises"][0]["key"]["claim"]["node"]["path"][1],
            json!({"node": 1})
        );
        assert_eq!(held["claim"]["path"][1], json!({"node": 0}));
    }

    /// FR-621-AC-16, FSpec FR-412-AC-34: a certificate identity keeps its
    /// structural `node_id` and takes the owning model, while every other id
    /// of the item is held.
    #[trace("TC-3154", "FR-621-AC-16")]
    #[test]
    fn a_certificate_identity_stays_structural() {
        let structural = id("own", "own", "leaf");
        let identity = json!({"node_id": structural, "model": null, "ordinal": 0});
        let value = json!({
            "request": {"claim": {"node": structural}},
            "certificate": {"identity": identity, "document": {"identity": identity}}});
        let held = hold(&value, &graph("own"), None, Some("test/m")).expect("held");
        assert_eq!(
            held["request"]["claim"]["node"]["path"][1],
            json!({"node": 0})
        );
        let written = json!({"node_id": structural, "model": "test/m", "ordinal": 0});
        assert_eq!(held["certificate"]["identity"], written);
        assert_eq!(held["certificate"]["document"]["identity"], written);
    }

    /// FR-621-AC-17, FSpec FR-322 "Scope": a peer assumption id of a key is
    /// held in the graph of the package of the peer guarantee's component,
    /// not the graph its own `package` member names.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn a_peer_assumption_is_held_in_its_peer_components_graph() {
        let peer = id("elsewhere", "test/part-c", "wrap");
        let clause = filament_exact::node_wire::node_key_from_json(&peer).expect("a node key");
        let components = ComponentPackages::new(
            BTreeMap::from([
                ("B".to_owned(), "test/part-b".to_owned()),
                ("C".to_owned(), "test/part-c".to_owned()),
            ]),
            BTreeMap::from([(clause, "test/part-c".to_owned())]),
        );
        let graphs = BTreeMap::from([
            ("test/part-b".to_owned(), Ok(graph("test/part-b"))),
            ("test/part-c".to_owned(), Ok(graph("test/part-c"))),
        ]);
        let scope = ComponentScope {
            components: &components,
            graphs: &graphs,
        };
        let mut value = item();
        value["request"]["premises"][0]["key"]["temporal"]["subject"]["assumptions"] =
            json!([peer]);
        let held = hold(&value, &graph("own"), Some(&scope), None).expect("held");
        assert_eq!(
            held["request"]["premises"][0]["key"]["temporal"]["subject"]["assumptions"][0]["path"]
                [1],
            json!({"node": 1})
        );
        // A clause no guarantee of the declaration names has no graph.
        let none = ComponentScope {
            components: &self::components(),
            graphs: &graphs,
        };
        assert!(hold(&value, &graph("own"), Some(&none), None).is_err());
    }

    /// FR-621-AC-17: an `interface-mismatch` `interface` is held in the graph
    /// of its `component`.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn an_interface_is_held_in_its_components_graph() {
        let components = components();
        let graphs = graphs();
        let scope = ComponentScope {
            components: &components,
            graphs: &graphs,
        };
        let value = json!({"counterexample": [{"type": "interface-mismatch",
            "component": "B", "interface": id("test/part-b", "test/part-b", "leaf"),
            "premise": 0, "reason": "absent"}]});
        let held = hold(&value, &graph("own"), Some(&scope), None).expect("held");
        assert_eq!(
            held["counterexample"][0]["interface"]["path"][1],
            json!({"node": 0})
        );
    }

    /// FR-621-AC-17: a key of a package no component names has no held form.
    #[trace("TC-3154", "FR-621-AC-17")]
    #[test]
    fn a_key_of_no_component_has_no_held_form() {
        let graphs = graphs();
        let components = ComponentPackages::default();
        let scope = ComponentScope {
            components: &components,
            graphs: &graphs,
        };
        assert!(matches!(
            hold(&item(), &graph("own"), Some(&scope), None),
            Err(HeldIdentityError::NotInGraph(_))
        ));
        assert!(key_subject_package(&item()["request"]["premises"][0]["key"]).is_some());
    }
}
