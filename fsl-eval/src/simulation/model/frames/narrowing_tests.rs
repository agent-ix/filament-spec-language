// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Directly solved assignments" and FR-851 (TC-2411): over a rover
//! whose `mode` is `Int[0, 3]` (`0` Driving, `1` Degraded, `2` SafeStop,
//! `3` Failsafe), whose `speed` is `Int[0, max]` and whose `lidarOk` and
//! `stopped` are Booleans, a solved and narrowed application generates only
//! the candidates its postcondition admits and keeps the enumeration's
//! successors, findings and exploration. The enumerated reference form of
//! each clause is the same conjunction compared with `true`, which is no
//! equation and no bound, so it is enumerated.

use ix_trace_rs::trace;
use serde_json::json;

use super::super::application::{Applications, OperationRef};
use super::super::walk::{rows_of, Budget};
use super::super::{explore_model, ModelSystem};
use crate::simulation::explore::{Limits, Outcome};
use crate::simulation::sampled::world::{initial, Model, Operation, World, BOOLEAN};

const PACKAGE: &str = "test/rover";

/// `loseLidar`'s postcondition, with its speed bound `cap`.
fn lose_lidar(cap: u32) -> String {
    format!(
        "self.mode = 1 and not self.lidarOk and self.speed <= {cap} and self.speed <= pre(self.speed)"
    )
}

/// `brake`'s postcondition, with its speed bound `cap`.
fn brake(cap: u32) -> String {
    format!("self.mode = 2 and self.speed <= {cap} and self.speed <= pre(self.speed)")
}

/// The rover with `speed: Int[0, max]`, the clauses `posts` (operation,
/// postcondition), each written as is or, when `enumerated`, as its
/// enumerated reference form, and the initial fields `initial`.
fn rover(max: i64, posts: &[(&str, String)], enumerated: bool, initial: serde_json::Value) -> Model {
    let ident = |name: &str| format!("ix://{PACKAGE}/{name}");
    let operation = |name: &'static str, modifies: Vec<&'static str>| Operation {
        name,
        parameters: Vec::new(),
        modifies,
    };
    let mut body = String::new();
    for (index, (operation, post)) in posts.iter().enumerate() {
        let post = if enumerated {
            format!("({post}) = true")
        } else {
            post.clone()
        };
        body.push_str(&format!(
            "post P{index} using v on M::Rover::{operation} {{ {post} }}\n"
        ));
    }
    body.push_str("pre Driving using v on M::Rover::loseLidar { self.mode = 0 and self.lidarOk }\n");
    body.push_str("pre Degraded using v on M::Rover::brake { self.mode = 1 }\n");
    body.push_str("pre Braked using v on M::Rover::halt { self.mode = 2 and not self.stopped }\n");
    Model {
        package: PACKAGE,
        object: "Rover",
        population: "rovers",
        key: "r1",
        integers: vec![("Mode", 0, 3), ("Speed", 0, max)],
        fields: vec![
            ("mode", ident("Mode")),
            ("speed", ident("Speed")),
            ("lidarOk", BOOLEAN.to_owned()),
            ("stopped", BOOLEAN.to_owned()),
        ],
        operations: vec![
            operation("loseLidar", vec!["mode", "lidarOk", "speed"]),
            operation("brake", vec!["mode", "speed"]),
            operation("halt", vec!["speed", "stopped"]),
        ],
        body,
        initial,
    }
}

fn fields(mode: u32, speed: u32, lidar: bool) -> serde_json::Value {
    json!({
        "mode": {"integer": mode.to_string()},
        "speed": {"integer": speed.to_string()},
        "lidarOk": {"boolean": lidar},
        "stopped": {"boolean": false},
    })
}

/// The applications of `name` from the initial state, and their charge.
fn apply(system: &ModelSystem<'_>, name: &str) -> (Applications, u64) {
    let state = initial(system);
    let [row] = <[_; 1]>::try_from(rows_of(system, &state.state).expect("rows"))
        .unwrap_or_else(|_| panic!("one rover"));
    let operation = OperationRef {
        declaring: row.type_id(),
        name,
        receiver: row.type_id(),
    };
    let mut budget = Budget::new(u64::MAX);
    let applications = system
        .apply_operation_matching(&state, &operation, &|_| true, None, &mut budget)
        .expect("applies");
    (applications, budget.used())
}

/// The successors' transitions, arguments and post-states, and the
/// findings.
fn view(applications: &Applications) -> (Vec<String>, String) {
    let successors = applications
        .successors
        .iter()
        .map(|applied| {
            format!(
                "{:?} {:?} {:?}",
                applied.transition, applied.arguments, applied.state
            )
        })
        .collect();
    (successors, format!("{:?}", applications.findings))
}

/// FR-851-AC-1: from a driving rover at speed 800, `loseLidar` solves
/// `mode` and `lidarOk` and narrows `speed` to `[0, 300]`: it charges its
/// one argument vector and 301 candidates, where the enumeration charges
/// 4 * 2 * 2001, and both give the same successors in the same order and
/// the same findings.
#[trace("TC-2411", "FR-851-AC-1")]
#[test]
fn lose_lidar_generates_one_candidate_per_admitted_speed() {
    let posts = [("loseLidar", lose_lidar(300))];
    let narrowed = World::new(&rover(2000, &posts, false, fields(0, 800, true)));
    let enumerated = World::new(&rover(2000, &posts, true, fields(0, 800, true)));
    let (solved, solved_charge) = apply(&narrowed.system(), "loseLidar");
    let (full, full_charge) = apply(&enumerated.system(), "loseLidar");
    assert_eq!(solved_charge, 1 + 301);
    assert_eq!(full_charge, 1 + 4 * 2 * 2001);
    assert_eq!(solved.successors.len(), 301);
    assert_eq!(view(&solved), view(&full));
}

/// FR-851-AC-1: from a degraded rover at speed 300, `brake` solves `mode`
/// and narrows `speed` to `[0, 100]`: 101 candidates against 4 * 2001.
#[trace("TC-2411", "FR-851-AC-1")]
#[test]
fn brake_generates_one_candidate_per_admitted_speed() {
    let posts = [("brake", brake(100))];
    let narrowed = World::new(&rover(2000, &posts, false, fields(1, 300, false)));
    let enumerated = World::new(&rover(2000, &posts, true, fields(1, 300, false)));
    let (solved, solved_charge) = apply(&narrowed.system(), "brake");
    let (full, full_charge) = apply(&enumerated.system(), "brake");
    assert_eq!(solved_charge, 1 + 101);
    assert_eq!(full_charge, 1 + 4 * 2001);
    assert_eq!(view(&solved), view(&full));
}

/// FR-851 "Narrowing": a bound below the field's range admits no
/// candidate, so the application charges only its argument vector and has
/// no successor and no finding, as the enumeration has.
#[trace("TC-2411", "FR-851-AC-1")]
#[test]
fn an_empty_interval_generates_nothing() {
    let posts = [("brake", "self.mode = 2 and self.speed < 0".to_owned())];
    let narrowed = World::new(&rover(2000, &posts, false, fields(1, 300, false)));
    let enumerated = World::new(&rover(2000, &posts, true, fields(1, 300, false)));
    let (solved, solved_charge) = apply(&narrowed.system(), "brake");
    let (full, full_charge) = apply(&enumerated.system(), "brake");
    assert_eq!(solved_charge, 1);
    assert_eq!(full_charge, 1 + 4 * 2001);
    assert!(solved.successors.is_empty());
    assert_eq!(view(&solved), view(&full));
}

/// FR-851-AC-1 (scaled): exploring a rover with `loseLidar`, `brake` and
/// `halt` with narrowing and with forced full enumeration gives equal
/// `Exploration`s: the same state keys in the same order, the same
/// transitions and the same findings.
#[trace("TC-2411", "FR-851-AC-1")]
#[test]
fn narrowing_keeps_the_exploration() {
    let posts = [
        ("loseLidar", lose_lidar(30)),
        ("brake", brake(10)),
        ("halt", "self.speed = 0 and self.stopped".to_owned()),
    ];
    let narrowed = World::new(&rover(80, &posts, false, fields(0, 60, true)));
    let enumerated = World::new(&rover(80, &posts, true, fields(0, 60, true)));
    let explore = |world: &World| {
        explore_model(&world.system(), 10_000, Limits::default(), || false).expect("bounded")
    };
    let solved = explore(&narrowed);
    assert_eq!(solved, explore(&enumerated));
    let Outcome::Exhaustive(stats) = &solved.outcome else {
        panic!("the exploration completes: {:?}", solved.outcome);
    };
    assert!(stats.states > 31, "{stats:?}");
}
