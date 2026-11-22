// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Directly solved assignments" and FR-851 (TC-2411): over a rover
//! whose `mode` is `Int[0, 3]` (`0` Driving, `1` Degraded, `2` SafeStop,
//! `3` Failsafe), whose `speed` is `Int[0, max]` and whose `lidarOk` and
//! `stopped` are Booleans, a solved and narrowed application generates only
//! the candidates its postcondition admits and keeps the enumeration's
//! successors, findings and exploration. The enumerated reference form of
//! each clause is the same conjunction compared with `true`, which is no
//! equation and no bound, so it is enumerated.

use filament_exact::{Integer, Value};
use filament_semantic_value::model_transition::ModelTransition;
use ix_trace_rs::trace;
use serde_json::json;
use std::collections::BTreeMap;

use super::super::application::{Applications, OperationRef};
use super::super::walk::{rows_of, Budget};
use super::super::{explore_model, ModelSystem, StepEffect};
use crate::simulation::expansion::ExpansionLimit;
use crate::simulation::explore::{Limits, Outcome};
use crate::simulation::sampled::world::{initial, Model, Operation, World, BOOLEAN};
use crate::simulation::TransitionSystem;

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
fn rover(
    max: i64,
    posts: &[(&str, String)],
    enumerated: bool,
    initial: serde_json::Value,
) -> Model {
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
    body.push_str(
        "pre Driving using v on M::Rover::loseLidar { self.mode = 0 and self.lidarOk }\n",
    );
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

/// A query of [`cell_with`]: its name, result type reference, precondition
/// and body.
type Query = (&'static str, &'static str, &'static str, &'static str);

/// FR-851-AC-5's `nonZero(): Boolean`: an empty frame, body `true` and
/// precondition `self.a != 0`, so `self.nonZero()` is `Undefined` with
/// reason `precondition-false` on every candidate with `a` 0.
const NON_ZERO: Query = ("nonZero", BOOLEAN, "self.a != 0", "true");

/// `mirror(): A`, whose body `3 - self.a` is defined exactly when its
/// precondition `self.b != 5` holds.
const MIRROR: Query = ("mirror", "ix://test/rover/A", "self.b != 5", "3 - self.a");

/// FR-851-AC-5's `Cell`: `a: Int[0, 3]` and `b: Int[0, 99]`, at `a` 1 and
/// `b` 50, with the query `nonZero()` and the operation `set()`, which
/// modifies both fields and has the clause `post A { post }`, or its
/// enumerated reference form.
fn cell(post: &str, enumerated: bool) -> World {
    cell_with(post, enumerated, 50, &[NON_ZERO])
}

/// The `Cell` at `b` with the queries `queries` and `set()`'s clause
/// `post A { post }`, or its enumerated reference form.
fn cell_with(post: &str, enumerated: bool, b: u32, queries: &[Query]) -> World {
    cell_with_bounds(post, enumerated, 3, 1, b, queries)
}

/// A Cell fixture with a caller-selected A domain and initial values.
fn cell_with_bounds(
    post: &str,
    enumerated: bool,
    max_a: u32,
    a: u32,
    b: u32,
    queries: &[Query],
) -> World {
    cell_with_precondition(post, enumerated, max_a, a, b, queries, None)
}

fn cell_with_precondition(
    post: &str,
    enumerated: bool,
    max_a: u32,
    a: u32,
    b: u32,
    queries: &[Query],
    precondition: Option<&str>,
) -> World {
    let ident = |name: &str| format!("ix://{PACKAGE}/{name}");
    let post = if enumerated {
        format!("({post}) = true")
    } else {
        post.to_owned()
    };
    let operation = |name: &'static str, modifies: Vec<&'static str>| Operation {
        name,
        parameters: Vec::new(),
        modifies,
    };
    let mut operations: Vec<Operation> = queries
        .iter()
        .map(|(name, ..)| operation(name, Vec::new()))
        .collect();
    operations.push(operation("set", vec!["a", "b"]));
    let mut body: String = queries
        .iter()
        .map(|(name, _, pre, value)| {
            format!(
                "pre {name}Pre using v on M::Cell::{name} {{ {pre} }}\n\
                 body {name}Body using v on M::Cell::{name}() {{ {value} }}\n"
            )
        })
        .collect();
    if let Some(precondition) = precondition {
        body.push_str(&format!(
            "pre Range using v on M::Cell::set {{ {precondition} }}\n"
        ));
    }
    body.push_str(&format!("post A using v on M::Cell::set {{ {post} }}\n"));
    let results: Vec<(&str, &str)> = queries
        .iter()
        .map(|(name, result, ..)| (*name, *result))
        .collect();
    World::with_queries(
        &Model {
            package: PACKAGE,
            object: "Cell",
            population: "cells",
            key: "k1",
            integers: vec![("A", 0, i64::from(max_a)), ("B", 0, 99)],
            fields: vec![("a", ident("A")), ("b", ident("B"))],
            operations,
            body,
            initial: json!({"a": {"integer": a.to_string()}, "b": {"integer": b.to_string()}}),
        },
        &results,
    )
}

/// Compare actual canonical successors, transition identities and effects,
/// without depending on the candidate generator's iteration order.
type ApplicationView = BTreeMap<Vec<u8>, (ModelTransition, Option<StepEffect>)>;

fn keyed_applications(system: &ModelSystem<'_>, applications: &Applications) -> ApplicationView {
    let keyed: BTreeMap<_, _> = applications
        .successors
        .iter()
        .map(|applied| {
            let effect = applied
                .state
                .reached_by()
                .expect("a completed assignment records its step effect");
            (
                system.state_key(&applied.state).expect("a canonical key").0,
                (applied.transition.clone(), Some(effect.clone())),
            )
        })
        .collect();
    assert_eq!(
        keyed.len(),
        applications.successors.len(),
        "the original fixture has no duplicate state keys"
    );
    keyed
}

fn integer_field(state: &super::super::ModelSimState, name: &str) -> Integer {
    let Value::Integer(value) = crate::simulation::sampled::world::field(state, name) else {
        panic!("{name} is an integer field")
    };
    value
}

#[trace("FR-120-AC-29")]
#[test]
fn direct_swap_reads_both_pre_values_and_observes_the_candidate_budget_boundary() {
    let post = "self.a = pre(self.b) and self.b = pre(self.a)";
    let solved = cell_with_bounds(post, false, 99, 3, 5, &[]);
    let general = cell_with_bounds(post, true, 99, 3, 5, &[]);
    let mut solved_system = solved.system();
    let mut general_system = general.system();
    solved_system.probability_work =
        Some(filament_exact::WorkScope::new(solved_system.meter_limits()));
    general_system.probability_work = Some(filament_exact::WorkScope::new(
        general_system.meter_limits(),
    ));
    let (fast, fast_charge) = apply(&solved_system, "set");
    let (full, full_charge) = apply(&general_system, "set");
    assert_eq!(fast_charge, 2);
    assert_eq!(full_charge, 10_001);
    assert_eq!(fast.successors.len(), 1);
    let state = &fast.successors[0].state;
    assert_eq!(integer_field(state, "a"), Integer::from(5_i64));
    assert_eq!(integer_field(state, "b"), Integer::from(3_i64));
    assert_eq!(
        keyed_applications(&solved_system, &fast),
        keyed_applications(&general_system, &full)
    );
    assert!(fast.findings.is_empty());
    assert!(full.findings.is_empty());
    for (budget, completes) in [(1, false), (2, true)] {
        let mut system = solved.system_within(budget);
        system.probability_work = Some(filament_exact::WorkScope::new(system.meter_limits()));
        let expansion = system.successors(&initial(&system));
        if completes {
            assert_eq!(
                expansion.expect("the exact charge fits").successors.len(),
                1
            );
        } else {
            let stopped = expansion.expect_err("one missing candidate charge must stop");
            assert_eq!(stopped.limit, Some(ExpansionLimit::MaxCandidates));
            assert_eq!(
                stopped.cause,
                filament_semantic_value::catalog::CatalogCode::new(
                    "resource_exhausted",
                    "insufficient-next-charge"
                )
            );
        }
    }
}

/// FR-577 permits the explicit precondition to prove the increment's range;
/// the post remains a direct assignment in both generator comparisons.
#[trace("FR-120-AC-30")]
#[test]
fn guarded_partial_assignment_preserves_unsolved_values_effects_and_charges() {
    let post = "self.a = pre(self.a) + 1";
    let solved = cell_with_precondition(post, false, 99, 3, 5, &[], Some("self.a <= 98"));
    let general = cell_with_precondition(post, true, 99, 3, 5, &[], Some("self.a <= 98"));
    assert_partial_assignment(solved, general);
}

fn assert_partial_assignment(solved: World, general: World) {
    let mut solved_system = solved.system();
    let mut general_system = general.system();
    solved_system.probability_work =
        Some(filament_exact::WorkScope::new(solved_system.meter_limits()));
    general_system.probability_work = Some(filament_exact::WorkScope::new(
        general_system.meter_limits(),
    ));
    let (fast, fast_charge) = apply(&solved_system, "set");
    let (full, full_charge) = apply(&general_system, "set");
    assert_eq!(fast_charge, 101);
    assert_eq!(full_charge, 10_001);
    assert_eq!(fast.successors.len(), 100);
    let mut bs: Vec<_> = fast
        .successors
        .iter()
        .map(|applied| {
            assert_eq!(integer_field(&applied.state, "a"), Integer::from(4_i64));
            let Value::Integer(b) = crate::simulation::sampled::world::field(&applied.state, "b")
            else {
                panic!("b is an integer")
            };
            b
        })
        .collect();
    bs.sort();
    bs.dedup();
    assert_eq!(bs, (0_i64..100).map(Integer::from).collect::<Vec<_>>());
    assert_eq!(
        keyed_applications(&solved_system, &fast),
        keyed_applications(&general_system, &full)
    );
    assert!(fast.findings.is_empty());
    assert!(full.findings.is_empty());
    let system = general.system_within(300);
    assert_eq!(
        system
            .successors(&initial(&system))
            .expect_err("the enumerated form cannot fit300")
            .limit,
        Some(ExpansionLimit::MaxCandidates)
    );
    let system = solved.system_within(300);
    assert_eq!(
        system
            .successors(&initial(&system))
            .expect("the solved form fits300")
            .successors
            .len(),
        100
    );
}

#[trace("FR-120-AC-30")]
#[test]
fn guarded_assignment_and_reference_preserve_full_canonical_exploration() {
    let post = "self.a = pre(self.a) + 1";
    let solved = cell_with_precondition(post, false, 3, 0, 5, &[], Some("self.a <= 2"));
    let general = cell_with_precondition(post, true, 3, 0, 5, &[], Some("self.a <= 2"));
    assert_increment_exploration(solved, general);
}

fn assert_increment_exploration(solved: World, general: World) {
    let explore = |world: &World| {
        explore_model(&world.system(), 10_000, Limits::default(), || false)
            .expect("the finite Cell family admits")
    };
    let fast = explore(&solved);
    let full = explore(&general);
    assert_eq!(fast, full);
    let Outcome::Exhaustive(stats) = fast.outcome else {
        panic!("the bounded increment family must finish")
    };
    assert_eq!(stats.states, 301);
    assert_eq!(stats.transitions, 20_100);
    assert_eq!(stats.depth, 3);
    assert!(fast.findings.is_empty());
}

/// The one finding a candidate of `applications` records for the
/// postcondition `A`, with the reason it is undefined.
fn undetermined(applications: &Applications) -> Vec<String> {
    applications
        .findings
        .iter()
        .map(|finding| format!("{finding:?}"))
        .filter(|finding| finding.contains("ContractUndetermined"))
        .collect()
}

/// FR-851-AC-5: a dispatched call as the first conjunct leaves the defined
/// prefix empty, so neither `a` nor `b` is narrowed: `set` generates all 400
/// candidates and charges 401, records one `ContractUndetermined` for `A`
/// (`precondition-false`) on each of the 100 candidates with `a` 0, and
/// keeps the 11 successors with `a` 2 and `b` 0 to 10, exactly as forced
/// full enumeration does. The expansion, which also applies `nonZero`
/// (its argument vector, its one unchanged candidate and its first result
/// pair), charges 404: it completes under 404 and stops naming
/// `MaxCandidates` under 403.
#[trace("TC-2411", "FR-851-AC-5")]
#[test]
fn an_undefined_leading_conjunct_narrows_nothing() {
    let post = "self.nonZero() and self.a = 2 and self.b <= 10";
    let narrowed = cell(post, false);
    let enumerated = cell(post, true);
    let (solved, solved_charge) = apply(&narrowed.system(), "set");
    let (full, full_charge) = apply(&enumerated.system(), "set");
    assert_eq!(solved_charge, 401);
    assert_eq!(full_charge, 401);
    assert_eq!(solved.successors.len(), 11);
    let findings = undetermined(&solved);
    assert_eq!(findings.len(), 100, "{:?}", solved.findings);
    assert!(
        findings
            .iter()
            .all(|finding| finding.contains("PreconditionFalse")),
        "{findings:?}"
    );
    assert_eq!(view(&solved), view(&full));

    let expand = |world: &World, max: u64| {
        let system = world.system_within(max);
        let state = initial(&system);
        system
            .successors(&state)
            .map(|expansion| format!("{expansion:?}"))
    };
    let complete = expand(&narrowed, 404).expect("the expansion fits 404 candidates");
    assert_eq!(
        Some(complete),
        expand(&enumerated, 1_000_000).ok(),
        "the narrowed expansion is the enumerated one"
    );
    assert_eq!(
        expand(&narrowed, 403).err().map(|stop| stop.limit),
        Some(Some(ExpansionLimit::MaxCandidates))
    );
}

/// FR-851-AC-5: with the call last, `a` is solved and `b` narrowed to 0 to
/// 10: `set` generates 11 candidates, charges 12 and records no finding, as
/// its forced full enumeration records none, since every candidate with `a`
/// 0 stops at `self.a = 2`.
#[trace("TC-2411", "FR-851-AC-5")]
#[test]
fn an_undefined_trailing_conjunct_keeps_the_prefix_narrowing() {
    let post = "self.a = 2 and self.b <= 10 and self.nonZero()";
    let (solved, solved_charge) = apply(&cell(post, false).system(), "set");
    let (full, full_charge) = apply(&cell(post, true).system(), "set");
    assert_eq!(solved_charge, 12);
    assert_eq!(full_charge, 401);
    assert_eq!(solved.successors.len(), 11);
    assert!(solved.findings.is_empty(), "{:?}", solved.findings);
    assert_eq!(view(&solved), view(&full));
}

/// FR-120 "Eligible": an equation whose `e` calls a dispatched query on the
/// pre-state is solved when the call is defined. At `a` 1 and `b` 50,
/// `pre(self.mirror())` is 2 and `pre(self.b)` is 50, so `set` generates the
/// one candidate `a` 2, `b` 50 and charges 2, where its enumerated reference
/// form charges 401; both keep that one successor and record no finding.
#[trace("TC-1418", "FR-120-AC-32")]
#[test]
fn an_equation_over_a_defined_pre_state_query_is_solved() {
    let post = "self.a = pre(self.mirror()) and self.b = pre(self.b)";
    let (solved, solved_charge) = apply(&cell_with(post, false, 50, &[MIRROR]).system(), "set");
    let (full, full_charge) = apply(&cell_with(post, true, 50, &[MIRROR]).system(), "set");
    assert_eq!(solved_charge, 2);
    assert_eq!(full_charge, 401);
    assert_eq!(solved.successors.len(), 1);
    assert!(solved.findings.is_empty(), "{:?}", solved.findings);
    assert_eq!(view(&solved), view(&full));
}

/// FR-120-AC-32: at `b` 5 the query's precondition is false on the
/// pre-state, so `pre(self.mirror())` is `Undefined` (`precondition-false`)
/// and the application is not solved: it gives its enumerated reference
/// form's successors (none), its `ContractUndetermined` finding for `A` on
/// every candidate, and its charge.
#[trace("TC-1418", "FR-120-AC-32")]
#[test]
fn an_equation_over_an_undefined_pre_state_query_is_enumerated() {
    let post = "self.a = pre(self.mirror()) and self.b = pre(self.b)";
    let (unsolved, unsolved_charge) = apply(&cell_with(post, false, 5, &[MIRROR]).system(), "set");
    let (full, full_charge) = apply(&cell_with(post, true, 5, &[MIRROR]).system(), "set");
    assert_eq!(unsolved_charge, 401);
    assert_eq!(full_charge, 401);
    assert!(unsolved.successors.is_empty());
    let findings = undetermined(&unsolved);
    assert_eq!(findings.len(), 400, "{:?}", unsolved.findings);
    assert!(findings
        .iter()
        .all(|finding| finding.contains("PreconditionFalse")));
    assert_eq!(view(&unsolved), view(&full));
}
