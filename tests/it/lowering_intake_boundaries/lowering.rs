// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{applications, nodes, one_application, package, reference_target};
use ix_trace_rs::trace;
use serde_json::{Value, json};

#[test]
#[trace("FR-093-AC-1")]
fn a_conditional_shares_its_literal_but_keeps_three_source_occurrences() {
    let package =
        package("function choose using v(): Boolean pure { if true then true else true }");
    let literals: Vec<_> = nodes(&package)
        .iter()
        .filter(|node| {
            node["node_tag"] == "value"
                && node["semantic_form"] == "literal"
                && node["body"]["value"] == true
        })
        .collect();
    assert_eq!(literals.len(), 1, "one true literal: {literals:?}");
    let literal = literals[0];
    let branch = one_application(&package, "filament.op.control.if");
    assert_eq!(branch["node_tag"], "expression");
    assert_eq!(branch["semantic_form"], "conditional");
    assert_eq!(branch["body"]["operator"], "conditional");
    assert_eq!(branch["body"]["result_type"], literal["semantic_type"]);
    assert_eq!(branch["semantic_type"], literal["semantic_type"]);
    let arguments = branch["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 3);
    for argument in arguments {
        assert_eq!(reference_target(argument), &literal["node_id"]);
    }
    let ordinals: Vec<_> = literal["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|occurrence| occurrence["role"] == "expression")
        .map(|occurrence| occurrence["ordinal"].as_u64().unwrap())
        .collect();
    assert_eq!(ordinals, [0, 1, 2]);
}

#[test]
#[trace("FR-093-AC-2")]
fn local_reads_reference_parameters_and_a_let_keeps_its_bound_slot() {
    let package = package(
        "function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }\nfunction outer using v(a: Boolean): Boolean pure { both(a, true) }\nfunction keep using v(a: Boolean): Boolean pure { let y = a in y }",
    );
    let conjunction = one_application(&package, "filament.op.boolean.and");
    let binding = one_application(&package, "filament.op.control.let");
    assert_ne!(conjunction["node_id"], binding["node_id"]);
    let conjunction_arguments = conjunction["body"]["arguments"].as_array().unwrap();
    assert_eq!(conjunction_arguments.len(), 2);
    for argument in conjunction_arguments {
        let id = reference_target(argument);
        let node = nodes(&package)
            .iter()
            .find(|node| &node["node_id"] == id)
            .unwrap();
        assert_eq!(node["node_tag"], "parameter", "{node}");
    }
    let arguments = binding["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0]["kind"], "binding");
    assert_eq!(arguments[0]["name"], "y");
    let input = reference_target(&arguments[0]["value"]);
    let bound = reference_target(&arguments[1]);
    assert_ne!(input, bound, "let binds a distinct local slot");
    assert_eq!(
        nodes(&package)
            .iter()
            .find(|node| &node["node_id"] == bound)
            .unwrap()["node_tag"],
        "parameter"
    );
}

#[test]
#[trace("FR-093-AC-3")]
fn arithmetic_and_boolean_applications_keep_operator_family_and_operand_order() {
    let package = package(
        "function arithmetic using v(a: Integer, b: Integer): Integer pure { -(a - b) }\nfunction boolean using v(a: Boolean, b: Boolean): Boolean pure { not (a implies b) }",
    );
    for (operation, operator, arity) in [
        ("filament.op.integer.sub", "binary", 2),
        ("filament.op.integer.negate", "unary", 1),
        ("filament.op.boolean.implies", "binary", 2),
        ("filament.op.boolean.not", "unary", 1),
    ] {
        let application = one_application(&package, operation);
        assert_eq!(application["body"]["operator"], operator);
        assert_eq!(
            application["semantic_type"],
            application["body"]["result_type"]
        );
        assert_eq!(
            application["body"]["arguments"].as_array().unwrap().len(),
            arity
        );
        for argument in application["body"]["arguments"].as_array().unwrap() {
            let id = reference_target(argument);
            assert!(nodes(&package).iter().any(|node| &node["node_id"] == id));
        }
    }
    let subtraction = one_application(&package, "filament.op.integer.sub");
    assert_ne!(
        subtraction["body"]["arguments"][0],
        subtraction["body"]["arguments"][1]
    );
}

#[test]
#[trace("FR-093-AC-4")]
fn widening_a_parameter_needs_no_conversion_but_a_bounded_literal_is_narrowed() {
    let widened = package("function widen using v(x: Int[0, 9]): Int[0, 10] pure { x }");
    assert!(applications(&widened, "filament.op.numeric.narrow").is_empty());
    assert!(applications(&widened, "filament.op.numeric.convert").is_empty());
    let narrowed = package("function digit using v(): Int[0, 9] pure { 3 }");
    let narrow = one_application(&narrowed, "filament.op.numeric.narrow");
    assert_eq!(narrow["body"]["operator"], "convert");
    let arguments = narrow["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 1);
    let literal = nodes(&narrowed)
        .iter()
        .find(|node| &node["node_id"] == reference_target(&arguments[0]))
        .unwrap();
    assert_eq!(literal["body"]["value"], json!("3"));
    assert_ne!(literal["semantic_type"], narrow["semantic_type"]);
}

#[test]
#[trace("FR-093-AC-4")]
fn a_rational_conversion_and_flat_map_keep_their_distinct_operations() {
    let package = package(
        "function toRational using v(x: Int[0, 9]): Rational[0, 9; 1, 1] pure { convert<Rational[0, 9; 1, 1]>(x) }\nfunction flatten using v(s: Sequence<Sequence<Int[0, 9]>[0, 2]>[0, 3]): Sequence<Int[0, 9]>[0, 6] pure { flatMap(x in s: x) }",
    );
    assert_eq!(
        one_application(&package, "filament.op.numeric.convert")["body"]["operator"],
        "convert"
    );
    one_application(&package, "filament.op.collection.flat_map");
    assert!(applications(&package, "filament.op.collection.map").is_empty());
}

#[test]
#[trace("FR-093-AC-5")]
fn sibling_quantifiers_reset_their_level_after_a_nested_binder() {
    let package = package(
        "function quantify using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: exists(y in s: x = y)) and exists(z in s: true) }",
    );
    let parameters: Vec<_> = nodes(&package)
        .iter()
        .filter(|node| node["node_tag"] == "parameter")
        .collect();
    let parameter = |name: &str| -> &Value {
        parameters
            .iter()
            .copied()
            .find(|node| {
                node["body"]["members"].as_array().is_some_and(|members| {
                    members
                        .iter()
                        .any(|member| member["name"] == "name" && member["value"]["value"] == name)
                })
            })
            .unwrap_or_else(|| panic!("parameter {name}: {parameters:?}"))
    };
    let level = |name: &str| -> Value {
        parameter(name)["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == "level")
            .unwrap()["value"]["value"]
            .clone()
    };
    assert_eq!(level("x"), json!("1"));
    assert_eq!(level("y"), json!("2"));
    assert_eq!(level("z"), json!("1"));
}
