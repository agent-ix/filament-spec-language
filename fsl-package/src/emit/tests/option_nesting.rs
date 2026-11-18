// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093: an `Option<Option<T>>` parameter emits as an option type node over
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

/// Emits `source`, asserts it is admitted and returns the wire with the chain
/// `[outer option, inner option, element]` of parameter `x`'s type.
fn nested_option_chain(source: &str) -> (Value, [Value; 3]) {
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
    assert_eq!(outer["semantic_form"], "option");
    let inner = with_id(&wire, &outer["body"]["members"][0]["target"]);
    assert_eq!(inner["semantic_form"], "option", "option over option");
    let element = with_id(&wire, &inner["body"]["members"][0]["target"]);
    let chain = [outer.clone(), inner.clone(), element.clone()];
    (wire, chain)
}

/// FR-093-AC-48 (closest owner; no AC names nested options): `Option<Option<Boolean>>`
/// emits an option node over an option node over `boolean`.
#[trace("FR-093-AC-48")]
#[test]
fn a_nested_boolean_option_emits_an_option_chain_and_is_admitted() {
    let (_, [_, _, element]) = nested_option_chain(
        "function f using v(x: Option<Option<Boolean>>): Boolean pure { present(x) }\n",
    );
    assert_eq!(element["semantic_form"], "boolean");
}

/// FR-093-AC-48 (closest owner): `Option<Option<Int[0, 9]>>` emits an option
/// node over an option node over the bounded `integer_range` domain.
#[trace("FR-093-AC-48")]
#[test]
fn a_nested_bounded_integer_option_emits_an_option_chain_and_is_admitted() {
    let (_, [_, _, element]) = nested_option_chain(
        "function f using v(x: Option<Option<Int[0, 9]>>): Boolean pure { present(x) }\n",
    );
    assert_eq!(element["node_tag"], "bounded_domain");
    assert_eq!(element["semantic_form"], "integer_range");
}
