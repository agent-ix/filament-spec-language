// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-952 (TC-2662), FSpec FR-770: every emitted `function` node records the
//! mode its body was checked under, and no other node carries the member.

use super::*;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"filament.value.complete/v1\";\n";
/// Defined on every input: `Linked` proves its divisor nonzero.
const GUARDED: &str = "function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure { x mod y }\n";
/// Its divisor may be zero, so only `Kernel` admits it.
const UNGUARDED: &str = "function r using v(x: Int[0, 9]): Integer pure { 10 mod x }\n";

fn emitted(declarations: &str, mode: CheckMode) -> Value {
    let unit = format!("{HEADER}{declarations}");
    let packages = fsl_semantics::model::intake::package_input([]).unwrap();
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(mode, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks under {mode:?}: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    wire(&emission)
}

fn function_modes(wire: &Value) -> Vec<(String, Value)> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "function")
        .map(|node| {
            (
                node["declaration"]["qualified_name"].to_string(),
                node["check_mode"].clone(),
            )
        })
        .collect()
}

/// FR-952-AC-1: a function body checked under `Linked` records `linked`.
#[trace("TC-2662", "FR-952-AC-1")]
#[test]
fn a_linked_function_records_linked() {
    let wire = emitted(GUARDED, CheckMode::Linked);
    let modes = function_modes(&wire);
    assert_eq!(modes.len(), 1, "{modes:?}");
    assert_eq!(modes[0].1, "linked", "{modes:?}");
}

/// FR-952-AC-1: every function body checked under `Kernel` records `kernel`,
/// a body `Linked` would also admit included; the two packages differ.
#[trace("TC-2662", "FR-952-AC-1")]
#[test]
fn a_kernel_function_records_kernel_and_the_packages_differ() {
    let kernel = emitted(&format!("{GUARDED}{UNGUARDED}"), CheckMode::Kernel);
    let modes = function_modes(&kernel);
    assert_eq!(modes.len(), 2, "{modes:?}");
    assert!(modes.iter().all(|(_, mode)| mode == "kernel"), "{modes:?}");
    let linked = emitted(GUARDED, CheckMode::Linked);
    assert_ne!(
        kernel["semantic_graph"], linked["semantic_graph"],
        "the recorded mode is part of the graph"
    );
}

/// FR-952-AC-1: a node that is not a `function` node carries no `check_mode`.
#[trace("TC-2662", "FR-952-AC-1")]
#[test]
fn a_non_function_node_has_no_check_mode() {
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        let wire = emitted(GUARDED, mode);
        let others: Vec<_> = nodes(&wire)
            .iter()
            .filter(|node| node["node_tag"] != "function")
            .collect();
        assert!(!others.is_empty(), "the unit has non-function nodes");
        for node in others {
            assert!(node.get("check_mode").is_none(), "{node}");
        }
    }
}
