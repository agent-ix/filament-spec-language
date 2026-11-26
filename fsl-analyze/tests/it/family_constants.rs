// SPDX-License-Identifier: AGPL-3.0-or-later
//! Every reading of a timed model's clock constants includes those inside a
//! quantifier family (FR-230 "Family parts"). Each model holds one element's
//! clock `x`, compared with 5 only inside a family, and each expected value
//! is worked out by hand.

use std::num::NonZeroU64;

use filament_exact::Rational;
use fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem, TimedSystem};
use fsl_eval::simulation::Limits;
use fsl_replay::{
    CounterexampleKind, Lasso, ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue,
    ReplayRefusal, TemporalCounterexample,
};
use ix_trace_rs::trace;

use crate::timed_world::{family_world, World};
use crate::zone_replay::{payload, replay, walk, whole};
use crate::zone_search::{INFINITE, TIMED};
use crate::zone_support::exact;

/// `ping` (`y >= 1`) resets `y`, and `work`, enabled once `x >= 5`, sets
/// `retried`. Nothing resets `x`.
fn retrying_family(fairness: &str) -> World {
    family_world(
        &["ts"],
        &["y"],
        &[("ping", &["y"]), ("work", &["retried"])],
        INFINITE,
        &format!(
            "pre Ping using v on M::Call::ping {{ self.y >= 1 ms }}\n\
             post Pinged using v on M::Call::ping {{ self.y = 0 ms }}\n\
             pre Work using v on M::Call::work {{ not self.retried and \
             forall(e in self.ts: e.x >= 5 ms) }}\n\
             post Worked using v on M::Call::work {{ self.retried }}\n\
             temporal Retries using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(c.retried) }}\n"
        ),
    )
}

/// A `ping` after `stem`, then the loop of one `ping` after 1.
fn pinging(world: &World, stem: Rational) -> TemporalCounterexample {
    payload(
        world,
        walk(world, &[("ping", stem), ("ping", whole(1))]),
        CounterexampleKind::Formula,
        Some(Lasso {
            entry: 1,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
    )
}

/// FR-236 "Timed lasso": a loop entered at `x = 2`, which it never resets,
/// is not above 5, the model's largest constant, so it does not close.
/// Entered at `x = 6` it closes, and `work` stays enabled and untaken, so
/// under `fair weak work` it is unfair.
#[trace("TC-692", "FR-236-AC-3")]
#[test]
fn a_loop_below_a_quantifier_family_constant_does_not_close() {
    let free = retrying_family("");
    let below = replay(&free, &pinging(&free, whole(2)));
    let Err(ReplayRefusal::ModelTrace(refusal)) = &below else {
        panic!("expected an open loop, got {below:?}");
    };
    assert_eq!(
        **refusal,
        ModelTraceRefusal::LoopOpen { entry: 1, steps: 2 }
    );
    let above = replay(&free, &pinging(&free, whole(6))).expect("replays");
    assert!(
        matches!(above, ModelTraceSettlement::Reproduced { .. }),
        "{above:?}"
    );
    let fair = retrying_family("fair weak M::Call::work;");
    let unfair = replay(&fair, &pinging(&fair, whole(6)));
    assert!(
        matches!(unfair, Err(ReplayRefusal::UnfairLasso { .. })),
        "{unfair:?}"
    );
}

/// FR-237's non-local time-lock replay explores the closure under the
/// model's constants. `tick` (`z >= 1`, `x <= 5`) resets `z`, which the
/// invariant holds at most 1, so every behaviour stops by `x = 6`: the
/// initial state is time-locked.
#[trace("TC-692", "FR-237-AC-4")]
#[test]
fn a_time_lock_past_a_quantifier_family_constant_reproduces() {
    let world = family_world(
        &["ts"],
        &["z"],
        &[("tick", &["z"])],
        INFINITE,
        "pre Tick using v on M::Call::tick { self.z >= 1 ms and \
         forall(e in self.ts: e.x <= 5 ms) }\n\
         post Ticked using v on M::Call::tick { self.z = 0 ms }\n\
         time invariant Brief when { true } { self.z <= 1 ms }\n\
         temporal NoLate using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    );
    let system = world.system();
    // Two live clocks: `z` and one element's `x`.
    let max_clocks = NonZeroU64::MIN.saturating_add(1);
    let timed = TimedSystem::with_max_clocks(&system, "M", max_clocks).expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    let counterexample = TemporalCounterexample {
        over_binding: None,
        fairness: Vec::new(),
        final_delay: Some(exact(1, 2)),
        closure_stutters: Vec::new(),
        ..payload(&world, Vec::new(), CounterexampleKind::TimeLock, None)
    };
    let (settlement, _) = fsl_replay::replay_timed_counterexample(
        &timed,
        &counterexample,
        None,
        &letters,
        system.meter_limits(),
        &Limits::default(),
    )
    .expect("replays");
    assert_eq!(
        settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: None,
        }
    );
}

/// FR-419's task product keeps each source clock exactly up to the largest
/// constant it is compared with: 5 for the element's `x`.
#[test]
fn a_task_automaton_ceiling_reads_a_quantifier_family() {
    let world = family_world(
        &["ts"],
        &[],
        &[("fire_a", &["phase"]), ("work", &["phase"])],
        TIMED,
        "pre Work using v on M::Call::work { forall(e in self.ts: e.x >= 5 ms) }\n\
         post Worked using v on M::Call::work { self.phase = 1 }\n\
         taskset One { task a periodic 100 ms deadline 100 ms bcet 1 ms wcet 1 ms priority 1 lo; }\n\
         schedulable OneFp on One under fixed_priority;\n\
         post Fire_a using v on M::Call::fire_a { self.phase = pre(self.phase) }\n\
         release One.a on M::Call::fire_a;\n",
    );
    let system = world.system();
    let timed =
        TimedSystem::with_max_clocks(&system, "M", NonZeroU64::MIN).expect("a timed subject");
    let product = TaskProductSystem::new(timed, "OneFp").expect("a checked task product");
    let automaton = TaskAutomaton::new(&product);
    assert_eq!(automaton.clocks()[0].ceiling, whole(5));
}
