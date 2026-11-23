// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{applications, nodes, one_application, package, reference_target};
use filament_contract_model::{CheckedSemanticNodeV2, CheckedTerm};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn parameter_name(node: &Value) -> &str {
    node["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "name")
        .unwrap()["value"]["value"]
        .as_str()
        .expect("parameter name")
}

fn resolved_parameter_name(node: &CheckedSemanticNodeV2) -> &str {
    let CheckedTerm::Aggregate { members, .. } = &node.body else {
        panic!("parameter aggregate");
    };
    members
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value, .. } if name.as_ref() == "name" => match &**value {
                CheckedTerm::Literal { value, .. } => value.as_str(),
                _ => None,
            },
            _ => None,
        })
        .expect("resolved parameter name")
}

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
    for (argument, expected_name) in conjunction_arguments.iter().zip(["a", "b"]) {
        let id = reference_target(argument);
        let node = nodes(&package)
            .iter()
            .find(|node| &node["node_id"] == id)
            .unwrap();
        assert_eq!(node["node_tag"], "value", "{node}");
        assert_eq!(node["semantic_form"], "parameter", "{node}");
        assert_eq!(parameter_name(node), expected_name);
    }
    let arguments = binding["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_eq!(arguments[0]["term"], "binding");
    assert_eq!(arguments[0]["name"], "y");
    let input = reference_target(&arguments[0]["value"]);
    let bound = reference_target(&arguments[1]);
    assert_ne!(input, bound, "let binds a distinct local slot");
    assert_eq!(
        parameter_name(
            nodes(&package)
                .iter()
                .find(|node| &node["node_id"] == input)
                .unwrap()
        ),
        "a"
    );
    assert_eq!(
        parameter_name(
            nodes(&package)
                .iter()
                .find(|node| &node["node_id"] == bound)
                .unwrap()
        ),
        "y"
    );
    assert_eq!(
        nodes(&package)
            .iter()
            .find(|node| &node["node_id"] == bound)
            .unwrap()["node_tag"],
        "value"
    );
}

#[test]
#[trace("FR-093-AC-3")]
fn arithmetic_and_boolean_applications_keep_operator_family_and_operand_order() {
    let package = package(
        "function arithmetic using v(a: Integer, b: Integer): Integer pure { -(a - b) }\nfunction boolean using v(a: Boolean, b: Boolean): Boolean pure { not (a implies b) }",
    );
    let resolved = crate::emit_checked_v2::read_back(&serde_json::to_vec(&package).unwrap(), &[]);
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
        // A node-held id uses the holder's operand slots; a document-held id
        // uses graph indexes. The public reader resolves their structural keys.
        let resolved_application = resolved.resolved_graph().nodes.iter().find(|node| {
            matches!(&node.body, CheckedTerm::Application { operation: found, .. } if found.identity() == Some(operation))
        }).unwrap();
        let CheckedTerm::Application { arguments, .. } = &resolved_application.body else {
            unreachable!("filtered to an application");
        };
        let mut operands = Vec::new();
        for argument in arguments {
            let CheckedTerm::Reference { target } = argument else {
                panic!("{operation} takes an operand reference: {argument:?}");
            };
            operands.push(
                resolved
                    .resolved_graph()
                    .nodes
                    .iter()
                    .find(|node| &node.node_id == target)
                    .unwrap_or_else(|| panic!("resolved {operation} operand {target:?}")),
            );
        }
        match operation {
            "filament.op.integer.sub" | "filament.op.boolean.implies" => {
                assert_eq!(
                    operands
                        .iter()
                        .map(|node| resolved_parameter_name(node))
                        .collect::<Vec<_>>(),
                    ["a", "b"]
                );
            }
            "filament.op.integer.negate" | "filament.op.boolean.not" => {
                let expected = if operation == "filament.op.integer.negate" {
                    "filament.op.integer.sub"
                } else {
                    "filament.op.boolean.implies"
                };
                let CheckedTerm::Application { operation, .. } = &operands[0].body else {
                    panic!("unary operand is the nested binary application");
                };
                assert_eq!(operation.identity(), Some(expected));
            }
            _ => unreachable!("the four explicit fixture operation rows"),
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
fn widening_a_parameter_needs_no_conversion_and_the_resulting_package_admits() {
    let widened = package("function widen using v(x: Int[0, 9]): Int[0, 10] pure { x }");
    assert!(applications(&widened, "filament.op.numeric.narrow").is_empty());
    assert!(applications(&widened, "filament.op.numeric.convert").is_empty());
}

#[test]
#[trace("FR-093-AC-4")]
fn a_bounded_literal_lowers_to_numeric_narrow_over_its_literal() {
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
        "function toRational using v(x: Int[0, 9]): Rational[0, 9; 1, 1] pure { convert<Rational[0, 9; 1, 1]>(x) }\nfunction flattened using v(s: Sequence<Sequence<Int[0, 9]>[0, 2]>[0, 3]): Sequence<Int[0, 9]>[0, 6] pure { flatMap(x in s: x) }",
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
        .filter(|node| node["node_tag"] == "value" && node["semantic_form"] == "parameter")
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

/// FSpec FR-149-AC-30: an annotated `let` binds its binder at the annotation,
/// so `let y: Integer = x in g(y)` passes `y` bare to `g`'s `Integer`
/// parameter and the strict reader admits the package, with no conversion at
/// the `let` or at the call.
#[test]
#[trace("FR-093-AC-4")]
fn an_annotated_let_binds_at_its_annotation_and_the_package_admits() {
    let package = package(
        "function g using v(n: Integer): Boolean pure { true }\n\
         function r using v(x: Int[0, 1000]): Boolean pure { let y: Integer = x in g(y) }",
    );
    assert!(applications(&package, "filament.op.numeric.convert").is_empty());
    assert!(applications(&package, "filament.op.numeric.narrow").is_empty());
    one_application(&package, "filament.op.control.let");
}

/// FSpec FR-322 "The `if` check": an `if` whose branches are a contained
/// literal or a contained integer reads back, with the result typed at the
/// declared type or at the hull of its branches.
#[test]
#[trace("FR-093-AC-4")]
fn an_if_over_contained_integer_branches_admits() {
    let literal_else = package(
        "function a using v(c: Boolean, x: Int[0, 9]): Int[0, 10] pure { if c then 10 else x }",
    );
    one_application(&literal_else, "filament.op.control.if");
    let hull = package(
        "function b using v(c: Boolean, x: Int[0, 9], y: Int[5, 20]): Int[0, 20] pure { if c then x else y }",
    );
    one_application(&hull, "filament.op.control.if");
}
