// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051: the definitional reference runs a whole checked model to a truth
//! value for a state-graph claim. The expected values are ADR-022 section 7's
//! hand-derived verdicts, never the analyzer's.

use filament_exact::{Integer, Value};
use fsl_definitional_reference::{
    ReferenceFiniteState, ReferenceModel, ReferenceObject, ReferenceOperation,
};
use ix_trace_rs::trace;

use crate::graph_world::World;

fn state(object: &str, fields: &[(&str, i64)]) -> ReferenceFiniteState {
    ReferenceFiniteState::new(fields.iter().map(|(field, value)| {
        (
            object.to_owned(),
            (*field).to_owned(),
            Value::Integer(Integer::from(*value)),
        )
    }))
    .expect("the hand-written start is a finite valuation")
}

/// The reference's verdict on claim `name` of `world`, whose one object `key`
/// of type `type_name` starts at `start` and which runs `operations`.
fn verdict(
    world: &World,
    key: &str,
    type_name: &str,
    operations: &[&str],
    start: &[(&str, i64)],
    name: &str,
) -> bool {
    let graph = world.package.graph();
    let reference = Value::Reference(world.reference(key));
    let objects = [ReferenceObject {
        id: key.to_owned(),
        type_name: type_name.to_owned(),
        reference: reference.clone(),
    }];
    let operations: Vec<_> = operations
        .iter()
        .map(|operation| ReferenceOperation {
            operation: (*operation).to_owned(),
            receiver: key.to_owned(),
        })
        .collect();
    let model = ReferenceModel::explore(graph, &objects, &operations, &[state(key, start)])
        .expect("the model is inside the reference fragment");
    model
        .decide_state_graph_claim(graph, world.claim(name), &[reference])
        .expect("the claim is inside the reference fragment")
}

/// ADR-022 7.2: from `Mid` the game can reach `Lost`, from which no operation
/// leaves, so `CanStillWin` is false; with `restart` (`Lost -> Start`) every
/// reachable state still reaches `Won`, so it is true.
#[trace("FR-1051")]
#[test]
fn reference_decides_always_possible_by_the_backward_fixpoint() {
    use crate::graph_world::{game_world, CAN_STILL_WIN};
    let claim = "CanStillWin";
    let play = &["play", "win", "lose"];
    let trapped = game_world(false, false, CAN_STILL_WIN);
    assert!(!verdict(
        &trapped,
        "g",
        "Game",
        play,
        &[("phase", 0)],
        claim
    ));
    let restart = game_world(false, true, CAN_STILL_WIN);
    assert!(verdict(
        &restart,
        "g",
        "Game",
        &["play", "win", "lose", "restart"],
        &[("phase", 0)],
        claim
    ));
}

/// `possible` holds when every initial state reaches the target: phase 3 is
/// reachable from `Start`; a phase that is both 2 and 3 is reachable from
/// nowhere.
#[trace("FR-1051")]
#[test]
fn reference_decides_possible_from_every_initial_state() {
    use crate::graph_world::game_world;
    let world = game_world(
        false,
        false,
        "possible Wins using v over (x: G::Game) { x.phase = 3 }\n\
         possible Both using v over (x: G::Game) { x.phase = 2 and x.phase = 3 }\n",
    );
    let play = &["play", "win", "lose"];
    assert!(verdict(&world, "g", "Game", play, &[("phase", 0)], "Wins"));
    assert!(!verdict(&world, "g", "Game", play, &[("phase", 0)], "Both"));
}

/// ADR-022 7.3: from `a = 0, b = 0` the steps may come in either order, two
/// distinct paths to `done`; sequencing `stepB` after `stepA` leaves one.
#[trace("FR-1051")]
#[test]
fn reference_decides_unique_path_by_counting_edge_sequences() {
    use crate::graph_world::job_world;
    let steps = &["stepA", "stepB", "finish"];
    let start = &[("a", 0), ("b", 0), ("done", 0)];
    assert!(!verdict(
        &job_world(false, false),
        "j",
        "Job",
        steps,
        start,
        "InOneWay"
    ));
    assert!(verdict(
        &job_world(true, false),
        "j",
        "Job",
        steps,
        start,
        "InOneWay"
    ));
}
