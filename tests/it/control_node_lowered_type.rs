// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495-AC-23 (TC-2298): a control node whose leaf is an integer `mod`
//! lowered as `Integer` takes the hull of its leaves' lowered types, and a
//! binder bound to such a node is narrowed when it is read into `Int[..]`.

use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const NARROW: &str = "filament.op.numeric.narrow";

/// The checked package of one function `body`, compiled through `command`.
fn package(directory: &Path, function: &str) -> Value {
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {function}\n"
    );
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:control-lowering", "document": "ControlLowering",
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

/// FR-495-AC-23: the `if` over a `mod` leaf in an unbounded position is
/// typed `Integer` as its leaf is; in a bounded position each leaf is
/// narrowed; a `let` binder bound to the `if` is narrowed at its read.
#[test]
#[trace("TC-2298", "FR-495-AC-23")]
#[ignore = "QSL-2300: a control node's lowered type is not the hull of its leaves' lowered types yet"]
fn a_control_node_takes_the_hull_of_its_leaves_lowered_types() {
    let directory = tempfile::tempdir().unwrap();
    let ifi = package(
        directory.path(),
        "function ifi using v(a: Integer, b: Int[1, 12], c: Boolean): Integer pure \
         { if c then a mod b else 0 }",
    );
    let modulo = the_one(&ifi, "filament.op.integer.mod");
    let branch = the_one(&ifi, "filament.op.control.if");
    assert_eq!(branch["result_type"], modulo["result_type"], "{ifi}");
    assert_eq!(count(&ifi, NARROW), 0, "{ifi}");

    let ifb = package(
        directory.path(),
        "function ifb using v(a: Integer, b: Int[1, 12], c: Boolean): Int[0, 11] pure \
         { if c then a mod b else 0 }",
    );
    let narrow = the_one(&ifb, NARROW);
    let branch = the_one(&ifb, "filament.op.control.if");
    assert_eq!(branch["result_type"], narrow["result_type"], "{ifb}");
    assert_ne!(
        branch["result_type"],
        the_one(&ifb, "filament.op.integer.mod")["result_type"],
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
        branch["result_type"],
        the_one(&letif, "filament.op.integer.mod")["result_type"],
        "{letif}"
    );

    let unit = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"filament.value.complete/v1\";\n\
                function narrow using v(a: Integer, b: Int[1, 12], c: Boolean): Int[0, 5] pure \
                { let x = if c then a mod b else 0 in x }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let error = fsl_command::command::compile(&directory.path().join("compile.json"))
        .expect_err("an unproved range refuses");
    let value = error.value().unwrap();
    assert_eq!(value["code"], "undefined_expression", "{value}");
    assert!(value.to_string().contains("unproved-range"), "{value}");
}
