// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495-AC-23 (TC-2298): a control node whose leaf is an integer `mod`
//! lowered as `Integer` takes the hull of its leaves' lowered types, and a
//! binder bound to such a node is narrowed when it is read into `Int[..]`.

use filament_contract_model::{
    CapabilityKind, CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm,
    RequirementRecord,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::path::Path;

const NARROW: &str = "filament.op.numeric.narrow";

fn operation_identity(node: &filament_contract_model::CheckedSemanticNodeV2) -> Option<&str> {
    match &node.body {
        CheckedTerm::Application { operation, .. } => operation.identity(),
        _ => None,
    }
}

fn resolved(package: &Value) -> CheckedPackageV2 {
    let bytes = serde_json::to_vec(package).expect("the package remains JSON");
    crate::emit_checked_v2::read_back(&bytes, &[])
}

fn applications_resolved<'a>(
    package: &'a CheckedPackageV2,
    identity: &str,
) -> Vec<&'a CheckedSemanticNodeV2> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .filter(|node| operation_identity(node) == Some(identity))
        .collect()
}

fn the_one_resolved<'a>(
    package: &'a CheckedPackageV2,
    identity: &str,
) -> &'a CheckedSemanticNodeV2 {
    let found = applications_resolved(package, identity);
    let [one] = found.as_slice() else {
        panic!("one resolved {identity} application, found {}", found.len());
    };
    one
}

fn result_type(node: &CheckedSemanticNodeV2) -> &CheckedNodeId {
    let CheckedTerm::Application { result_type, .. } = &node.body else {
        panic!("application node has an application body: {node:?}");
    };
    result_type
}

fn type_node<'a>(
    package: &'a CheckedPackageV2,
    identity: &CheckedNodeId,
) -> &'a CheckedSemanticNodeV2 {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == identity)
        .expect("result type is an admitted graph node")
}

fn assert_integer_range(
    wire: &Value,
    package: &CheckedPackageV2,
    identity: &CheckedNodeId,
    lower: &str,
    upper: &str,
) {
    let typed = type_node(package, identity);
    assert_eq!(typed.node_tag.as_ref(), "bounded_domain", "{typed:?}");
    assert_eq!(typed.semantic_form.as_ref(), "integer_range", "{typed:?}");
    let index = package
        .resolved_graph()
        .nodes
        .iter()
        .position(|node| &node.node_id == identity)
        .expect("result type is an admitted graph node");
    let members = wire["semantic_graph"]["nodes"][index]["body"]["members"]
        .as_array()
        .expect("integer range has named bound members");
    for (name, expected) in [("min", lower), ("max", upper)] {
        let bound = members
            .iter()
            .find(|member| member["name"] == name)
            .unwrap_or_else(|| panic!("integer range has {name}: {members:?}"));
        assert_eq!(bound["value"]["value"], expected, "{bound}");
    }
}

fn parameter_name(node: &CheckedSemanticNodeV2) -> &str {
    let CheckedTerm::Aggregate { members, .. } = &node.body else {
        panic!("a narrow reads a parameter node: {node:?}");
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
        .expect("parameter has a literal name")
}

/// The checked package of one function `body`, compiled through `command`.
fn package(directory: &Path, function: &str) -> Value {
    let function_name = function
        .split_ascii_whitespace()
        .nth(1)
        .expect("the fixture names its function");
    let directory = directory.join(function_name);
    std::fs::create_dir_all(&directory).expect("the fixture workspace is created");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {function}\n"
    );
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": format!("test:control-lowering-{function_name}"), "document": "ControlLowering",
            "formal_revision": 1}}}});
    let path = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let bytes = fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    serde_json::from_slice(&bytes).expect("the package is JSON")
}

/// Every application term in `value` whose operation identity is
/// `identity`, depth first.
fn applications<'a>(value: &'a Value, identity: &str, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(members) => {
            if members
                .get("operation")
                .and_then(|operation| operation.get("identity"))
                .is_some_and(|found| found == identity)
            {
                out.push(value);
            }
            for member in members.values() {
                applications(member, identity, out);
            }
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| applications(item, identity, out)),
        _ => {}
    }
}

fn the_one<'a>(package: &'a Value, identity: &str) -> &'a Value {
    let mut found = Vec::new();
    applications(package, identity, &mut found);
    let [one] = found.as_slice() else {
        panic!("one {identity} application, found {}", found.len());
    };
    one
}

fn count(package: &Value, identity: &str) -> usize {
    let mut found = Vec::new();
    applications(package, identity, &mut found);
    found.len()
}

/// Return an application's result type with its local operand slots resolved
/// to graph node indices, so equality compares referenced nodes rather than
/// per-application slot numbers.
fn result_type_by_node(package: &Value, application: &Value) -> Value {
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("checked graph nodes");
    let node_index = nodes
        .iter()
        .position(|node| {
            let mut found = Vec::new();
            applications(
                node,
                application["operation"]["identity"]
                    .as_str()
                    .expect("application identity"),
                &mut found,
            );
            found
                .iter()
                .any(|candidate| std::ptr::eq(*candidate, application))
        })
        .expect("application belongs to a checked graph node");
    let operands = package["semantic_graph"]["operand_rows"][node_index]
        .as_array()
        .expect("node operand row");
    let mut result_type = application["result_type"].clone();
    let path = result_type["path"]
        .as_array_mut()
        .expect("result type path");
    for segment in path {
        let Some(slot) = segment.get("operand").and_then(Value::as_u64) else {
            continue;
        };
        let target = operands
            .get(slot as usize)
            .and_then(Value::as_u64)
            .expect("node operand slot target");
        *segment = json!({"node": target});
    }
    result_type
}

fn direct_narrow_over<'a>(
    package: &'a CheckedPackageV2,
    operation: &str,
) -> (&'a CheckedSemanticNodeV2, &'a CheckedSemanticNodeV2) {
    let nodes = &package.resolved_graph().nodes;
    let direct: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter_map(|(narrow_index, narrow)| {
            if operation_identity(narrow) != Some(NARROW) {
                return None;
            }
            let CheckedTerm::Application { arguments, .. } = &narrow.body else {
                return None;
            };
            let [CheckedTerm::Reference { target }] = arguments.as_slice() else {
                return None;
            };
            let (operand_index, operand) = nodes
                .iter()
                .enumerate()
                .find(|(_, node)| &node.node_id == target)?;
            (operation_identity(operand) == Some(operation))
                .then_some((narrow_index, operand_index))
        })
        .collect();
    let [(narrow, operand)] = direct.as_slice() else {
        panic!("one narrow directly over {operation} in the resolved graph, found {direct:?}");
    };
    (&nodes[*narrow], &nodes[*operand])
}

/// FR-495-AC-23: the `if` over a `mod` leaf in an unbounded position is
/// typed `Integer` as its leaf is; in a bounded position each leaf is
/// narrowed; a `let` binder bound to the `if` is narrowed at its read.
#[test]
#[trace("TC-2298", "FR-495-AC-23")]
fn a_control_node_takes_the_hull_of_its_leaves_lowered_types() {
    let directory = tempfile::tempdir().unwrap();
    let ifi = package(
        directory.path(),
        "function ifi using v(a: Integer, b: Int[1, 12], c: Boolean): Integer pure \
         { if c then a mod b else 0 }",
    );
    let modulo = the_one(&ifi, "filament.op.integer.mod");
    let branch = the_one(&ifi, "filament.op.control.if");
    assert_eq!(
        result_type_by_node(&ifi, branch),
        result_type_by_node(&ifi, modulo),
        "{ifi}"
    );
    let resolved_ifi = resolved(&ifi);
    let modulo = the_one_resolved(&resolved_ifi, "filament.op.integer.mod");
    let branch = the_one_resolved(&resolved_ifi, "filament.op.control.if");
    assert_eq!(result_type(branch), result_type(modulo), "{ifi}");
    assert_eq!(
        type_node(&resolved_ifi, result_type(branch))
            .semantic_form
            .as_ref(),
        "integer",
        "{ifi}"
    );
    assert_eq!(count(&ifi, NARROW), 0, "{ifi}");

    let ifb = package(
        directory.path(),
        "function ifb using v(a: Integer, b: Int[1, 12], c: Boolean): Int[0, 11] pure \
         { if c then a mod b else 0 }",
    );
    let narrow = the_one(&ifb, NARROW);
    let branch = the_one(&ifb, "filament.op.control.if");
    assert_eq!(
        result_type_by_node(&ifb, branch),
        result_type_by_node(&ifb, narrow),
        "{ifb}"
    );
    assert_ne!(
        result_type_by_node(&ifb, branch),
        result_type_by_node(&ifb, the_one(&ifb, "filament.op.integer.mod")),
        "{ifb}"
    );
    let resolved_ifb = resolved(&ifb);
    let (narrow, modulo) = direct_narrow_over(&resolved_ifb, "filament.op.integer.mod");
    let branch = the_one_resolved(&resolved_ifb, "filament.op.control.if");
    assert_eq!(result_type(branch), result_type(narrow), "{ifb}");
    assert_ne!(result_type(branch), result_type(modulo), "{ifb}");
    assert_integer_range(&ifb, &resolved_ifb, result_type(branch), "0", "11");
    assert_eq!(
        type_node(&resolved_ifb, result_type(modulo))
            .semantic_form
            .as_ref(),
        "integer",
        "{ifb}"
    );

    let letif = package(
        directory.path(),
        "function letif using v(a: Integer, b: Int[1, 12], c: Boolean): Int[0, 11] pure \
         { let x = if c then a mod b else 0 in x }",
    );
    assert_eq!(count(&letif, NARROW), 1, "{letif}");
    let branch = the_one(&letif, "filament.op.control.if");
    assert_eq!(
        result_type_by_node(&letif, branch),
        result_type_by_node(&letif, the_one(&letif, "filament.op.integer.mod")),
        "{letif}"
    );
    let resolved_letif = resolved(&letif);
    let branch = the_one_resolved(&resolved_letif, "filament.op.control.if");
    let modulo = the_one_resolved(&resolved_letif, "filament.op.integer.mod");
    assert_eq!(result_type(branch), result_type(modulo), "{letif}");
    let narrows = applications_resolved(&resolved_letif, NARROW);
    let [narrow] = narrows.as_slice() else {
        panic!("exactly one let-body narrow: {narrows:?}");
    };
    let CheckedTerm::Application { arguments, .. } = &narrow.body else {
        unreachable!("numeric.narrow is an application");
    };
    let [CheckedTerm::Reference { target }] = arguments.as_slice() else {
        panic!("the let-body narrow has one reference operand: {arguments:?}");
    };
    let read = resolved_letif
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == target)
        .expect("let-body narrow references its binder");
    assert_eq!(parameter_name(read), "x", "{letif}");
    assert_ne!(target, &modulo.node_id, "the mod is not directly narrowed");
    let row = resolved_letif
        .requirement_items()
        .iter()
        .find(|row| row.claim == modulo.node_id && row.kind == CapabilityKind::ValueValidity)
        .expect("the mod application has a value-validity requirement record");
    let Some(RequirementRecord::ValueValidity(record)) = row.record.as_ref() else {
        panic!("the mod requirement has a value-validity record: {row:?}");
    };
    assert_eq!(record.application, modulo.node_id, "{record:?}");
    assert_eq!(record.conversion, None, "{record:?}");
    assert_integer_range(&letif, &resolved_letif, &record.result_bound, "0", "11");

    let invalid_directory = directory.path().join("invalid");
    std::fs::create_dir_all(&invalid_directory).unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"filament.value.complete/v1\";\n\
                function narrow using v(a: Integer, b: Int[1, 12], c: Boolean): Int[0, 5] pure \
                { let x = if c then a mod b else 0 in x }\n";
    std::fs::write(invalid_directory.join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:control-lowering-invalid", "document": "ControlLowering",
            "formal_revision": 1}}}});
    let request_path = invalid_directory.join("compile.json");
    std::fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let error =
        fsl_command::command::compile(&request_path).expect_err("an unproved range refuses");
    let value = error.value().unwrap();
    assert_eq!(value["code"], "undefined_expression", "{value}");
    assert_eq!(value["cause"], "unproved-range", "{value}");
    let read_offset = unit.rfind("in x").expect("the let has a final x read") + 3;
    assert_eq!(value["details"]["path"], "unit.native", "{value}");
    assert_eq!(
        value["details"]["span"]["start"]["byte"], read_offset,
        "{value}"
    );
    assert_eq!(
        value["details"]["span"]["end"]["byte"],
        read_offset + 1,
        "{value}"
    );
}
