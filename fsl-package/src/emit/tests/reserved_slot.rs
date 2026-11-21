// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488-AC-29, TC-3082 step 4: a function parameter named outside the
//! reserved slot form checks and emits a `value`/`parameter` node of that
//! name.

use super::*;

/// The names of the `value`/`parameter` nodes of `emission`'s graph.
fn parameter_names(emission: &Emission) -> Vec<String> {
    raw_wire(emission)["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .filter_map(|node| node["body"]["members"][0]["value"]["value"].as_str())
        .map(str::to_owned)
        .collect()
}

#[trace("TC-3082", "FR-488-AC-29")]
#[test]
fn a_near_miss_of_the_reserved_slot_form_emits_a_parameter_node_of_that_name() {
    for name in [
        "slot_sv", "op_sa_sb", "rcv_s", "op_sa_x4", "op_s_xC3", "op_s_xc3", "slotted",
    ] {
        let (_, emission) = emit_from_text(&format!(
            "function f using v({name}: Integer): Integer pure {{ {name} }}\n"
        ));
        assert!(
            parameter_names(&emission).iter().any(|found| found == name),
            "{name}: {:?}",
            parameter_names(&emission)
        );
    }
}
