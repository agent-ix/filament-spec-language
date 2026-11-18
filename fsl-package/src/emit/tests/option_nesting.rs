// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-1: an `Option<Option<T>>` parameter emits as an option type node over
//! an option type node, and the emitted package is admitted by the pinned IR
//! reader (FSL's full I2 read).

use super::*;

/// The node whose id is `id`.
fn with_id<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// Emits `source`, asserts it is admitted and returns the element node of
/// parameter `x`'s type after asserting the option chain above it.
fn nested_option_element(source: &str) -> Value {
    let (_, package) = package_from_text(source);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the pinned IR reader admits the emitted package");
    };
    let wire = wire(&emission);
    let parameter = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "parameter")
        .expect("the parameter node");
    let outer = with_id(&wire, &parameter["semantic_type"]);
    assert_option_node(outer);
    let inner = with_id(&wire, &outer["body"]["members"][0]["target"]);
    assert_option_node(inner);
    with_id(&wire, &inner["body"]["members"][0]["target"]).clone()
}

/// A `composite_type`/`option` node typed by itself.
fn assert_option_node(node: &Value) {
    assert_eq!(node["node_tag"], "composite_type");
    assert_eq!(node["semantic_form"], "option");
    assert_eq!(node["semantic_type"], node["node_id"]);
}

/// FR-092-AC-1 (Type nodes: `Option<T>` is `composite_type`/`option` referencing
/// `T`): `Option<Option<Boolean>>` emits an option node over an option node over `boolean`.
#[trace("FR-092-AC-1", "TC-413")]
#[test]
fn a_nested_boolean_option_emits_an_option_chain_and_is_admitted() {
    let element = nested_option_element(
        "function f using v(x: Option<Option<Boolean>>): Boolean pure { present(x) }\n",
    );
    assert_eq!(element["semantic_form"], "boolean");
}

/// FR-092-AC-1: `Option<Option<Int[0, 9]>>` emits an option
/// node over an option node over the bounded `integer_range` domain.
#[trace("FR-092-AC-1", "TC-413")]
#[test]
fn a_nested_bounded_integer_option_emits_an_option_chain_and_is_admitted() {
    let element = nested_option_element(
        "function f using v(x: Option<Option<Int[0, 9]>>): Boolean pure { present(x) }\n",
    );
    assert_eq!(element["node_tag"], "bounded_domain");
    assert_eq!(element["semantic_form"], "integer_range");
}
