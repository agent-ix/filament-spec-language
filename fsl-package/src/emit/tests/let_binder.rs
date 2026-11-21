// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-68 (TC-3137), FSpec-FR-322-AC-512: a `let` application names its
//! binder's `value`/`parameter` node among its `dependencies`, whether or not
//! its body reads the binder, and the binder's annotation is part of the
//! `let` node's identity.

use super::*;

/// The emitted wire of `declarations`, with nothing omitted.
fn emitted_wire(declarations: &str) -> Value {
    let (_, package) = package_from_text(declarations);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, [], "nothing is omitted");
    wire(&emission)
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
    let narrow = emitted_wire(&source("Int[0, 1000]"));
    let wide = emitted_wire(&source("Int[0, 2000]"));
    let narrow_let = the_let(&narrow);
    assert_eq!(
        dependent_parameters(&narrow, narrow_let, "y").len(),
        1,
        "the unread binder is a dependency"
    );
    assert_ne!(
        narrow_let["node_id"],
        the_let(&wide)["node_id"],
        "the binder's annotation is part of the let's identity"
    );
}

/// FR-093-AC-68: each `#ieee_operand` `let` of an IEEE ordering depends on
/// its own binder node.
#[trace("FR-093-AC-68", "FSpec-FR-322-AC-512", "TC-3137")]
#[test]
fn each_ieee_operand_let_depends_on_its_binder_node() {
    let wire = emitted_wire(
        "function f using v(p: Float64[nearest-even], q: Float64[nearest-even]): Boolean pure \
         { p < q }\n",
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
        binders.push(parameter_member(binder, "level").clone());
    }
    binders.sort_by_key(Value::to_string);
    assert_eq!(binders, [json!("2"), json!("3")], "each at its own level");
}
