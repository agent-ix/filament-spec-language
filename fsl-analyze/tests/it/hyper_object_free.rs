// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488-AC-28 (TC-3010), FSpec FR-402-AC-11 (TC-2034): the body wrapper of a
//! hyper clause with no atom slot ranges over the `model`/`model_root` node of
//! its first subject, and the strict reader admits it.

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use ix_trace_rs::trace;
use serde_json::Value;

use crate::hyper_world::{non_interference, Vault, World, STARTS};

/// The emitted wire of `world`'s package.
fn emitted(world: &World) -> (Vec<u8>, Value) {
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits the hyper clause: {refusal:?}"));
    let bytes = emission.package().bytes().to_vec();
    let wire = serde_json::from_slice(&bytes).expect("wire JSON");
    (bytes, wire)
}

fn node<'a>(wire: &'a Value, id: &Value) -> &'a Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("a node {id}"))
}

/// The `over` target of the body wrapper of the one hyperproperty node.
fn wrapper_over(wire: &Value) -> (Value, Value) {
    let hyper = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["node_tag"] == "claim" && node["semantic_form"] == "hyperproperty")
        .expect("a hyperproperty node");
    let wrapper = node(wire, &hyper["body"]["members"][6]["value"]["target"]);
    let over = wrapper["body"]["arguments"][0]["target"].clone();
    (over, wrapper["dependencies"].clone())
}

fn admitted(bytes: &[u8], wire: &Value) -> bool {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    matches!(
        CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence),
        CheckedPackageV2ReadResult::Admitted(_)
    )
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
#[ignore = "QSL-2464 completed; current blocker has no assigned owner: the test's hyper source does not parse at S1 (invalid_syntax: expected identifier)"]
fn an_object_free_hyper_wrapper_ranges_over_the_model_root() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper Whole using inf over behaviours () \
         forall trace a of V forall trace b of V { always holds(true) }",
    );
    let (bytes, wire) = emitted(&world);
    let (over, dependencies) = wrapper_over(&wire);
    let target = node(&wire, &over);
    assert_eq!(
        (
            target["node_tag"].as_str(),
            target["semantic_form"].as_str()
        ),
        (Some("model"), Some("model_root"))
    );
    assert!(dependencies
        .as_array()
        .is_some_and(|deps| deps.contains(&over)));
    assert!(
        admitted(&bytes, &wire),
        "the strict reader admits the package"
    );
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
#[ignore = "QSL-2464 completed; current blocker has no assigned owner: the checked package has no hyperproperty node for this source"]
fn a_model_root_over_on_a_wrapper_that_reads_a_slot_is_refused() {
    let world = World::new(Vault::Secure, false, &STARTS, &non_interference(""));
    let (_, mut wire) = emitted(&world);
    let (over, _) = wrapper_over(&wire);
    assert_eq!(
        node(&wire, &over)["body"]["members"][0]["value"]["value"],
        "slot_sv_sa"
    );
    let root = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["semantic_form"] == "model_root")
        .expect("a model root")["node_id"]
        .clone();
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes");
    let wrapper = nodes
        .iter_mut()
        .find(|node| {
            node["semantic_form"] == "temporal_clause"
                && node["body"]["arguments"][0]["target"] == over
        })
        .expect("the wrapper");
    wrapper["body"]["arguments"][0]["target"] = root.clone();
    if let Some(deps) = wrapper["dependencies"].as_array_mut() {
        deps.push(root);
    }
    let bytes = serde_json::to_vec(&wire).expect("bytes");
    assert!(
        !admitted(&bytes, &wire),
        "a model-root over beside a slot read refuses"
    );
}
