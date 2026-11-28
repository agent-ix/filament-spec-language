// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239 "Instances and orbits" and "Undecided nodes in one search"
//! (QSL-3601): EN-6 over a symmetric population combines its orbits as
//! FR-126 "TP-4 items" and FSpec FR-361 state, its run budgets are shared by
//! the orbits, and an undecided node does not end a search that a later
//! violation can still refute.

use filament_checked::key::DeclarationKey;
use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome, ZoneUndecided};
use fsl_replay::TemporalCounterexample;
use fsl_route::reductions::{Reductions, SymmetryDeclaration};
use ix_trace_rs::trace;

use crate::timed_world::{ident, symmetric_document, world_armed_with_mode, World, DENSE, PACKAGE};
use crate::zone_search::{claim_with, violated, Outcome, INFINITE};

/// How far the fixture goes past the initial state `R`, where only `t3` is
/// inflight.
#[derive(Clone, Copy)]
enum Shape {
    /// `lag` alone, giving `late`: no undefined guard anywhere.
    Plain,
    /// `lag` gives `t3.late`: a violation one step from `R`.
    Shallow,
    /// `lag` gives `phase = 2` and `finish` gives `t3.late`: two steps.
    Deep,
    /// [`Shape::Deep`] and `peek`, whose guard is undefined at the `lag`
    /// successor.
    Peek,
}

fn calls() -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: ident("calls"),
    }
}

/// The symmetry declaring `class` over the `Call` population.
fn symmetry(class: &[&str]) -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: calls(),
            classes: vec![class.iter().map(|key| (*key).to_owned()).collect()],
        }],
        ..Reductions::default()
    }
}

/// The orbit fixture: objects `keys`, those `armed` inflight, the claim
/// `always holds(atom)` over `t`.
fn orbits(shape: Shape, atom: &str, keys: &[&str], armed: &[&str]) -> World {
    let aprobe: (&str, &[&str]) = ("aprobe", &["phase"]);
    let operations: Vec<(&str, &[&str])> = match shape {
        Shape::Plain => vec![("lag", &["inflight", "late"])],
        Shape::Shallow => vec![aprobe, ("lag", &["inflight", "late"])],
        Shape::Deep => vec![
            aprobe,
            ("lag", &["inflight", "phase"]),
            ("finish", &["phase", "late"]),
        ],
        Shape::Peek => vec![
            aprobe,
            ("lag", &["inflight", "phase"]),
            ("finish", &["phase", "late"]),
            ("peek", &["retried"]),
        ],
    };
    let lag = match shape {
        Shape::Plain | Shape::Shallow => "not self.inflight and self.late",
        Shape::Deep | Shape::Peek => "not self.inflight and self.phase = 2",
    };
    let deeper = match shape {
        Shape::Plain | Shape::Shallow => "",
        Shape::Deep | Shape::Peek => {
            "pre Finish using v on M::Call::finish { self.phase = 2 }\n\
             post Finished using v on M::Call::finish { self.phase = 3 and self.late }\n"
        }
    };
    let peek = match shape {
        Shape::Peek => {
            "pre Peek using v on M::Call::peek \
                 { not self.inflight and (m2(self.phase) = (self.phase = 2)) }\n\
             post Peeked using v on M::Call::peek { self.retried }\n"
        }
        Shape::Plain | Shape::Shallow | Shape::Deep => "",
    };
    let probing = match shape {
        Shape::Plain => "",
        Shape::Shallow | Shape::Deep | Shape::Peek => {
            "pre Probe using v on M::Call::aprobe \
                 { self.inflight and (m1(self.phase) = (self.phase = 0)) }\n\
             post Probed using v on M::Call::aprobe { self.phase = 1 }\n"
        }
    };
    world_armed_with_mode(
        &symmetric_document(&["x"], &operations),
        INFINITE,
        &format!(
            "{DENSE}\
             function m1 using v(n: Int[0, 3]): Boolean pure {{ 6 mod (1 - n) >= 0 }}\n\
             function m2 using v(n: Int[0, 3]): Boolean pure {{ 6 mod (2 - n) >= 0 }}\n\
             {probing}\
             pre Lag using v on M::Call::lag {{ self.inflight }}\n\
             post Lagged using v on M::Call::lag {{ {lag} }}\n\
             {deeper}{peek}\
             temporal Orbits using inf over (t: M::Call) clock \"model-steps\" on origin \
             {{ always holds({atom}) }}\n"
        ),
        keys,
        armed,
        CheckMode::Kernel,
    )
}

fn decide(world: &World, limits: ZoneCheckLimits, reductions: &Reductions) -> Outcome {
    claim_with(world, limits, &Cancel::new(), reductions)
}

/// The operation names of `counterexample`'s prefix, among the fixture's.
fn steps(counterexample: &TemporalCounterexample) -> Vec<&'static str> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let operation = step.transition.operation();
            ["aprobe", "lag", "finish", "peek"]
                .into_iter()
                .find(|name| operation.contains(&format!("Call/{name}\"")))
                .unwrap_or("?")
        })
        .collect()
}

/// The key of the object each step of `counterexample` is received by.
fn receivers(counterexample: &TemporalCounterexample) -> Vec<&'static str> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let receiver = step.transition.receiver().expect("a receiver");
            ["t1", "t2", "t3"]
                .into_iter()
                .find(|key| receiver.contains(&format!("\"identity\":\"{key}\"")))
                .unwrap_or("?")
        })
        .collect()
}

fn bound_to(counterexample: &TemporalCounterexample) -> &str {
    counterexample
        .over_binding
        .as_ref()
        .map(|instance| instance.key.as_str())
        .expect("a bound instance")
}

fn undecided_clause(outcome: &Outcome) -> String {
    match outcome {
        ZoneCheckOutcome::Undecided {
            cause: ZoneUndecided::UndecidedSuccessor(stop),
            ..
        } => format!("{stop:?}"),
        other => panic!("expected Undecided(UndecidedSuccessor), got {other:?}"),
    }
}

/// FR-239-AC-13: the orbit of `t1` ends `Undecided(UndecidedSuccessor)` at the
/// `aprobe` successor, and the orbit of `t3` is decided next: the quiescent
/// `lag` successor has `t3.late`, so the item is refuted by `t3` with the
/// one-step prefix `lag`.
#[trace("TC-694", "FR-239-AC-13", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
#[test]
fn a_refuted_orbit_is_reported_past_an_earlier_undecided_orbit() {
    let world = orbits(Shape::Shallow, "not t.late", &["t1", "t2", "t3"], &["t3"]);
    let counterexample = violated(decide(
        &world,
        ZoneCheckLimits::default(),
        &symmetry(&["t1", "t2"]),
    ));
    assert_eq!(bound_to(&counterexample), "t3");
    assert_eq!(steps(&counterexample), ["lag"]);
    assert_eq!(receivers(&counterexample), ["t3"]);
}

/// FR-239-AC-13: with the refuted orbit first in instance order, it is
/// reported and the orbit after it is not decided.
#[trace("TC-694", "FR-239-AC-13", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
#[test]
fn a_refuted_first_orbit_is_reported() {
    let world = orbits(Shape::Shallow, "not t.late", &["t1", "t2", "t3"], &["t1"]);
    let counterexample = violated(decide(
        &world,
        ZoneCheckLimits::default(),
        &symmetry(&["t2", "t3"]),
    ));
    assert_eq!(bound_to(&counterexample), "t1");
    assert_eq!(steps(&counterexample), ["lag"]);
    assert_eq!(receivers(&counterexample), ["t1"]);
}

/// FR-239-AC-13 and arch ruling Q9: the orbit `{t2, t3}`, declared as
/// `[t3, t2]`, is bound to its first member in instance order, `t2`.
#[trace("TC-694", "FR-239-AC-13", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
#[test]
fn an_orbit_is_bound_to_its_first_member_in_instance_order() {
    let world = orbits(Shape::Plain, "not t.late", &["t2", "t3"], &["t2", "t3"]);
    let counterexample = violated(decide(
        &world,
        ZoneCheckLimits::default(),
        &symmetry(&["t3", "t2"]),
    ));
    assert_eq!(bound_to(&counterexample), "t2");
    assert_eq!(steps(&counterexample), ["lag"]);
    assert_eq!(receivers(&counterexample), ["t2"]);
}

/// FR-239-AC-14: `max_symbolic_states` is one budget for the run. The orbit
/// of `t1` stores 6 or 7 of the 7 states (3 in the search, 3 or 4 in its
/// admitted-behaviour check) and ends undecided; the orbit of `t3` is denied
/// its second state, which ends the run, so the item is the first not-proved
/// orbit's `Undecided`, not the `Violated` `t3` gives under a budget of its
/// own.
#[trace("TC-694", "FR-239-AC-14", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
#[test]
fn the_symbolic_state_budget_is_shared_by_the_orbits() {
    let world = orbits(Shape::Shallow, "not t.late", &["t1", "t2", "t3"], &["t3"]);
    let limits = ZoneCheckLimits {
        max_symbolic_states: 7,
        ..ZoneCheckLimits::default()
    };
    let outcome = decide(&world, limits, &symmetry(&["t1", "t2"]));
    assert!(
        undecided_clause(&outcome).contains("\"Probe\""),
        "{outcome:?}"
    );
}

/// FR-239-AC-15: the search meets the undecided `aprobe` successor first in
/// breadth-first order, leaves it unexpanded and goes on; the `lag`
/// successor expands to the quiescent `finish` successor, where `t3.late`
/// holds.
#[trace("TC-694", "FR-239-AC-15")]
#[test]
fn a_later_violation_wins_over_an_undecided_node() {
    let world = orbits(Shape::Deep, "not t.late", &["t1", "t2", "t3"], &["t3"]);
    let counterexample = violated(decide(
        &world,
        ZoneCheckLimits::default(),
        &Reductions::default(),
    ));
    assert_eq!(bound_to(&counterexample), "t3");
    assert_eq!(steps(&counterexample), ["lag", "finish"]);
    assert_eq!(receivers(&counterexample), ["t3", "t3"]);
}

/// FR-239-AC-15: with no violation, the search stores the four symbolic
/// states `R`, `aprobe`-successor, `lag`-successor and `finish`-successor,
/// and ends `Undecided(UndecidedSuccessor)`, never `Holds`.
#[trace("TC-694", "FR-239-AC-15")]
#[test]
fn an_undecided_node_leaves_the_search_going_and_the_item_undecided() {
    let world = orbits(Shape::Deep, "not t.retried", &["t1", "t2", "t3"], &["t3"]);
    let outcome = decide(&world, ZoneCheckLimits::default(), &Reductions::default());
    undecided_clause(&outcome);
    let ZoneCheckOutcome::Undecided { stats, .. } = outcome else {
        panic!("expected an undecided item");
    };
    assert_eq!(stats.symbolic_states, 4);
}

/// FR-239-AC-15: of the undecided nodes, `aprobe`'s successor and `lag`'s
/// successor (whose `peek` guard is undefined), both one step from `R`, the
/// first in breadth-first order gives the item's cause: the guard `Probe`.
#[trace("TC-694", "FR-239-AC-15")]
#[test]
fn the_first_undecided_cause_in_breadth_first_order_is_kept() {
    let world = orbits(Shape::Peek, "not t.retried", &["t1", "t2", "t3"], &["t3"]);
    let outcome = decide(&world, ZoneCheckLimits::default(), &Reductions::default());
    let cause = undecided_clause(&outcome);
    assert!(cause.contains("\"Probe\""), "{cause}");
    assert!(!cause.contains("\"Peek\""), "{cause}");
}
