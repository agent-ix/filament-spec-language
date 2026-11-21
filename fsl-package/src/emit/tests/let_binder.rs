// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-68 (TC-3137), FSpec-FR-322-AC-512: a `let` application names its
//! binder's `value`/`parameter` node among its `dependencies`, whether or not
//! its body reads the binder, and the binder's annotation is part of the
//! `let` node's identity.

use super::*;

/// Emit and read `declarations`, with nothing omitted.
fn emitted_package(declarations: &str) -> Emission {
    let (_, package) = package_from_text(declarations);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, [], "nothing is omitted");
    assert!(
        matches!(read_back(&emission), Read::Verified { .. }),
        "IR admits the emitted binder link and its dependencies"
    );
    emission
}

/// The emitted wire of `declarations`, with nothing omitted.
fn emitted_wire(declarations: &str) -> Value {
    wire(&emitted_package(declarations))
}

/// The one `filament.op.control.let` application of `wire`.
fn the_let(wire: &Value) -> &Value {
    let lets: Vec<_> = nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "filament.op.control.let")
        .collect();
    let [only] = lets.as_slice() else {
        panic!("one let application, got {}", lets.len());
    };
    only
}

/// The node whose id is `id`.
fn node_with<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// The literal a `value`/`parameter` node's `member` binding holds.
fn parameter_member<'w>(node: &'w Value, member: &str) -> &'w Value {
    &node["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == member)
        .unwrap_or_else(|| panic!("no {member} member in {node}"))["value"]["value"]
}

/// The `value`/`parameter` nodes `application` depends on that are named
/// `name`.
fn dependent_parameters<'w>(wire: &'w Value, application: &Value, name: &str) -> Vec<&'w Value> {
    application["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| node_with(wire, id))
        .filter(|node| {
            node["semantic_form"] == "parameter" && parameter_member(node, "name") == name
        })
        .collect()
}

/// FR-093-AC-68: a `let` whose body reads its binder depends on the binder
/// node, which is the node the body reads and is not the parameter `x`.
#[trace("FR-093-AC-68", "FSpec-FR-322-AC-512", "TC-3137")]
#[test]
fn a_let_depends_on_the_binder_node_its_body_reads() {
    let wire = emitted_wire(
        "function f using v(x: Int[0, 1000]): Integer pure { let y: Integer = x in y }\n",
    );
    let application = the_let(&wire);
    let binders = dependent_parameters(&wire, application, "y");
    let [binder] = binders.as_slice() else {
        panic!("the let depends on one binder node: {binders:?}");
    };
    assert_eq!(
        application["body"]["arguments"][0]["binder"], binder["node_id"],
        "the binding explicitly links the binder node"
    );
    assert_eq!(
        application["body"]["arguments"][1]["target"], binder["node_id"],
        "the body reads the linked node"
    );
    assert_eq!(
        parameter_member(binder, "level"),
        &json!("1"),
        "the binder is at level 1, after the parameter"
    );
    assert!(
        dependent_parameters(&wire, application, "x")
            .iter()
            .all(|node| node["node_id"] != binder["node_id"]),
        "the parameter x is not the binder"
    );
}

/// FR-093-AC-68: a binder the body never reads is still a dependency of its
/// `let`, and its annotation alone changes the `let` node's identity.
#[trace("FR-093-AC-68", "FSpec-FR-322-AC-512", "TC-3137")]
#[test]
fn an_unread_binder_is_a_dependency_and_its_annotation_is_part_of_the_identity() {
    let source = |annotation: &str| {
        format!(
            "function f using v(x: Int[0, 999]): Boolean pure \
             {{ let y: {annotation} = x in true }}\n"
        )
    };
    let emission = emitted_package(&source("Int[0, 1000]"));
    let wider_emission = emitted_package(&source("Int[0, 2000]"));
    let wider = wire(&wider_emission);
    let wire = wire(&emission);
    let mut identities = Vec::new();
    for (document, maximum) in [(&wire, "1000"), (&wider, "2000")] {
        let application = the_let(document);
        let binders = dependent_parameters(document, application, "y");
        let [binder] = binders.as_slice() else {
            panic!("the unread binder is a dependency: {binders:?}");
        };
        assert_eq!(
            application["body"]["arguments"][0]["binder"], binder["node_id"],
            "the unread binder remains explicitly linked"
        );
        assert_eq!(parameter_member(binder, "level"), &json!("1"));
        let annotation = node_with(document, &binder["semantic_type"]);
        assert_eq!(annotation["node_tag"], "bounded_domain");
        assert_eq!(annotation["semantic_form"], "integer_range");
        let base = node_with(document, &annotation["semantic_type"]);
        assert_eq!(base["node_tag"], "scalar_type");
        assert_eq!(base["semantic_form"], "integer");
        assert_eq!(parameter_member(annotation, "min"), &json!("0"));
        assert_eq!(parameter_member(annotation, "max"), &json!(maximum));
        assert_eq!(annotation["body"]["members"].as_array().unwrap().len(), 2);

        // Operand indices are local to each emitted graph. Resolve them before
        // comparing structural identities across independently emitted packages.
        let graph = WireGraph::of_node_values(nodes(document)).expect("the emitted graph");
        let key = |node: &Value| {
            graph
                .key_of_value(&node["node_id"])
                .expect("a structural key")
        };
        identities.push((key(application), key(binder), key(annotation)));
    }
    assert_ne!(
        identities[0].2, identities[1].2,
        "the annotation type changes"
    );
    assert_ne!(
        identities[0].1, identities[1].1,
        "the linked binder identity changes"
    );
    assert_ne!(
        identities[0].0, identities[1].0,
        "the binder's annotation is part of the let's structural identity"
    );
    assert_ne!(
        emission.package.bytes(),
        wider_emission.package.bytes(),
        "the canonical emitted packages retain the annotation distinction"
    );
}

/// FR-093-AC-68: each `#ieee_operand` `let` of an IEEE ordering depends on
/// its own binder node.
#[trace("FR-093-AC-68", "FSpec-FR-322-AC-512", "TC-3137")]
#[test]
fn each_ieee_operand_let_depends_on_its_binder_node() {
    let wire = emitted_wire(
        "function f using v(p: Float64[nearest-even], q: Float64[nearest-even]): Boolean pure \
         { -p < -q }\n",
    );
    let lets: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "filament.op.control.let")
        .collect();
    assert_eq!(lets.len(), 2, "one let per operand");
    let mut binders = Vec::new();
    for application in lets {
        let found = dependent_parameters(&wire, application, "#ieee_operand");
        let [binder] = found.as_slice() else {
            panic!("the let depends on one operand binder: {found:?}");
        };
        assert_eq!(
            application["body"]["arguments"][0]["binder"], binder["node_id"],
            "each IEEE operand binding links its own binder node"
        );
        binders.push(parameter_member(binder, "level").clone());
    }
    binders.sort_by_key(Value::to_string);
    assert_eq!(binders, [json!("2"), json!("3")], "each at its own level");
}
