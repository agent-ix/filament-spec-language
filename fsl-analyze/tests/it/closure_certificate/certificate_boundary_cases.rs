// SPDX-License-Identifier: AGPL-3.0-or-later
//! Instance-context admission over actual EN-1 certificates.

use super::*;
use fsl_eval::automaton::AutomatonStateKey;
use serde_json::json;

/// Typed reference-shaped and obsolete three-part instance bindings cannot
/// substitute for the canonical two-part instance binding in an engine key.
#[trace("FR-338-AC-9", "TC-1164")]
#[test]
fn noncanonical_instance_bindings_are_rejected_at_the_offered_product_state() {
    let world = world(CONTRACT, &[("a", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let full = certificate(&invariant.run(&world));
    let request = invariant.request(&world, UNLIMITED);
    assert_eq!(check(&request, &full), TemporalCertificateCheck::Accepted);
    let original = &full.states[0];
    let key: serde_json::Value =
        serde_json::from_slice(original.automaton_state.as_bytes()).expect("engine JSON key");
    assert_eq!(key[1][3][0][1], json!(["instance", "a"]));

    for binding in [
        json!(["instance", "Counter", "a"]),
        json!(["instance", {"type": "text", "value": "a"}]),
    ] {
        let mut changed_key = key.clone();
        changed_key[1][3][0][1] = binding;
        let mut changed = full.clone();
        changed.states[0].automaton_state = AutomatonStateKey::imported(
            serde_json::to_vec(&changed_key).expect("canonical array-only key"),
        );
        let at = changed.states[0].clone();
        assert_eq!(
            check(&request, &changed),
            rejected(CertificateRule::ShapeMismatch, at),
            "an admitted model state cannot authenticate an invalid instance context"
        );
    }
}

/// Identical model-state sets do not allow one object's certified monitor
/// context to stand in for the other object's initial product state.
#[trace("FR-338-AC-9", "TC-1164")]
#[test]
fn replacing_one_instances_contexts_by_another_does_not_certify_both() {
    let world = world(CONTRACT, &[("a", 0), ("b", 0)]);
    let invariant = world.invariant("always holds(c.value <= 3)", |v| Some(v <= 3));
    let full = certificate(&invariant.run(&world));
    let request = invariant.request(&world, UNLIMITED);
    let context_of = |state: &ProductStateRef| {
        let key: serde_json::Value =
            serde_json::from_slice(state.automaton_state.as_bytes()).expect("engine key");
        key[1][3][0][1].clone()
    };
    let initial_model = world.state_key(&[0, 0]);
    let missing = full
        .states
        .iter()
        .find(|state| {
            state.model_state == initial_model && context_of(state) == json!(["instance", "b"])
        })
        .expect("b's initial product state")
        .clone();
    let mut only_a = full.clone();
    only_a
        .states
        .retain(|state| context_of(state) == json!(["instance", "a"]));
    assert_eq!(only_a.states.len() * 2, full.states.len());
    assert_eq!(
        check(&request, &only_a),
        rejected(CertificateRule::InitialMissing, missing),
        "all model states under a's context leave b's initial instance uncertified"
    );
}
