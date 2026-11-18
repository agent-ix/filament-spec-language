// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-952 (TC-2662) and FSpec FR-770 (FSpec TC-1860): every function node
//! records its check mode, and a Kernel world applies `Kernel` to every
//! function body it reaches while declared clauses stay `Linked`.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_foundation::RequestIndex;
use fsl_replay::{ModelCheckLimits, TerminalValue};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{counter_request, request_members, Workspace};
use crate::prob_world::{self, World};

/// FR-952-AC-1, FSpec FR-770-AC-1: a Linked compile writes `linked` on every
/// function node.
#[trace("TC-2662", "FR-952-AC-1")]
#[trace("FSpec-TC-1860", "FSpec-FR-770-AC-1")]
#[test]
fn every_function_node_records_its_check_mode() {
    let body = "function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure { x mod y }\n";
    let items = json!([]);
    let ws = Workspace::new("check-mode-linked");
    let _ = counter_request(&ws, body, items.clone());
    let request = request_members(Some(items));
    let compile = json!({"format": "native-compile/1", "request": {
        "models": request["models"], "program": request["program"]}});
    let path = ws.write("compile.json", compile.to_string().as_bytes());
    let package = fsl_command::command::compile(&path).expect("the program compiles");
    let document: Value = serde_json::from_slice(&package).expect("a v2 JSON document");
    let functions: Vec<&Value> = document["semantic_graph"]["nodes"]
        .as_array()
        .expect("the node collection")
        .iter()
        .filter(|node| node["node_tag"] == "function")
        .collect();
    assert!(!functions.is_empty(), "the unit declares a function");
    for node in functions {
        assert_eq!(node["check_mode"], "linked", "{node}");
    }
}

/// FR-952-AC-3: under a Kernel world, a function body called from the
/// probability path is checked under `Kernel` and is undefined at a zero
/// divisor at run time; the same division in the declared clause still
/// refuses `unproved-nonzero` at S3.
#[trace("TC-2662", "FR-952-AC-3")]
#[test]
#[ignore = "QSL-2329: Kernel check mode is not propagated to function bodies on the probability path"]
fn a_kernel_world_applies_kernel_to_function_bodies_only() {
    let called = "function z using vc(p: Int[0, 1]): Boolean pure { 1 mod p = 0 }\n\
                  probabilistic Z using ev on M over (c: M::Coin) under Even on origin \
                  { probability >= 0.5 [ always[0,3] holds(z(c.v)) ] }";
    let world = World::try_with_mode(&prob_world::coin(called), CheckMode::Kernel)
        .unwrap_or_else(|refusals| panic!("the function body checks under Kernel: {refusals:#?}"));
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(
        RequestIndex::new(0),
        world.request("Z", ModelCheckLimits::default()),
    );
    let records = analyze_items(&engines, vec![item], &Cancel::new());
    let terminal = records[0].terminal().expect("a terminal record");
    assert_eq!(terminal.value(), &TerminalValue::Refuted);
    assert!(
        format!("{:?}", records[0].probabilistic_counterexample).contains("DivisionByZero"),
        "z(0) is undefined at run time"
    );
    let declared = "probabilistic Z using ev on M over (c: M::Coin) under Even on origin \
                    { probability >= 0.5 [ always[0,3] holds(1 mod c.v = 0) ] }";
    let refusals = World::try_with_mode(&prob_world::coin(declared), CheckMode::Kernel)
        .err()
        .expect("a declared clause stays Linked");
    assert!(format!("{refusals:?}").contains("Nonzero"), "{refusals:#?}");
}

/// FR-952-AC-5 (TC-2975): a `native-compile/1` request carrying
/// `"check_mode": "kernel"`, a `handoff` and an `output` checks `g` and `r`
/// under `Kernel` and the Counter's declared precondition under `Linked`
/// (it admits); the package the handoff holds is the `output` file, so both
/// carry `kernel` on every function body's node, `g`'s and `r`'s included.
/// With `"check_mode": "linked"` the same request refuses `r`
/// `undefined_expression`/`unproved-nonzero` and writes neither.
#[trace("TC-2975", "FR-952-AC-5")]
#[test]
fn a_kernel_compile_writes_a_kernel_checked_handoff() {
    let body = "function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure { x mod y }\n\
                function r using v(x: Int[0, 9]): Integer pure { 10 mod x }\n";
    let compile = |name: &str, mode: &str| {
        let ws = Workspace::new(name);
        let _ = counter_request(&ws, body, json!([]));
        let request = request_members(None);
        let compile = json!({"format": "native-compile/1", "request": {
            "models": request["models"], "program": request["program"],
            "check_mode": mode,
            "output": {"package": "out.package.json"},
            "handoff": {"directory": "handoff", "package_name": "counter",
                "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}}});
        let path = ws.write("compile.json", compile.to_string().as_bytes());
        let outcome = fsl_command::command::compile_outcome(&path);
        (ws, outcome)
    };
    let modes = |package: &Value| -> Vec<(String, Value)> {
        package["semantic_graph"]["nodes"]
            .as_array()
            .expect("the node collection")
            .iter()
            .filter(|node| node["node_tag"] == "function")
            .map(|node| {
                let name = node["declaration"]["qualified_name"].to_string();
                (name, node["check_mode"].clone())
            })
            .collect()
    };

    let (ws, outcome) = compile("check-mode-kernel-handoff", "kernel");
    outcome.unwrap_or_else(|error| panic!("the Kernel compile succeeds: {error}"));
    let output = std::fs::read(ws.path("out.package.json")).expect("the output file");
    let held = std::fs::read(ws.path("handoff/checked-package.json")).expect("the handoff");
    assert_eq!(held, output, "the handoff holds the output package");
    let package: Value = serde_json::from_slice(&output).expect("a v2 JSON document");
    // Every function node here is a function body (`Noop`, `g`, `r`); the
    // Counter's `CanInc` precondition is a declared clause, a `state` node
    // checked under `Linked`, which carries no `check_mode`.
    let modes = modes(&package);
    let mut names: Vec<&str> = modes.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["[\"Noop\"]", "[\"g\"]", "[\"r\"]"], "{modes:?}");
    assert!(modes.iter().all(|(_, mode)| mode == "kernel"), "{modes:?}");

    let (ws, outcome) = compile("check-mode-linked-handoff", "linked");
    let error = outcome.err().expect("Linked refuses `10 mod x`");
    let text = error.to_string();
    assert!(text.contains("unproved-nonzero"), "{text}");
    assert!(!ws.path("out.package.json").exists());
    assert!(!ws.path("handoff").exists());
}
