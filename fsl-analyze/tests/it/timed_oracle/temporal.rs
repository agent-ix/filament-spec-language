// SPDX-License-Identifier: AGPL-3.0-or-later
//! The non-probabilistic rows of the corpus: temporal and state-graph claims
//! under the timed profile, on EN-6 (zones) and EN-1 (digital clocks). A
//! behaviour is admitted when its stamps diverge, so a time-locked or Zeno
//! behaviour is none.

use std::cell::Cell;

use filament_exact::{Cancel, Meter, Undefined};
use fsl_analyze::model_check::digital::{check_digital, DigitalLimits, DigitalRequest};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, TimedState, TimedSystem};
use fsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use fsl_replay::ModelCheckOutcome;
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;
use serde_json::Value;

use super::falsifier::{self, ratio, whole, Lock, Space};
use super::models::{formula, two_step_body};
use super::routes::{self, Answer, Verdict};
use crate::timed_world::{self, World, DENSE};

/// What a claim's dense meaning is.
#[derive(Clone, Copy, Debug)]
enum Meaning {
    True,
    False,
    /// `inconclusive` with this cause, never proved or refuted.
    Vacuous(&'static str),
}

/// A world over the timed profile `ev` whose `temporal` claims are `claims`.
fn world(clocks: &[&str], operations: &[(&str, &[&str])], body: &str) -> World {
    timed_world::world(
        &timed_world::document(clocks, operations),
        super::models::PROFILES,
        &format!("{DENSE}{body}"),
    )
}

/// Every answer agrees with `meaning`, or is an honest decline.
fn assert_answers(row: &str, meaning: Meaning, answers: &[Answer]) {
    assert_none(disagreeing(row, meaning, answers));
}

fn assert_none(wrong: Vec<String>) {
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The answers that neither agree with `meaning` nor honestly decline.
fn disagreeing(row: &str, meaning: Meaning, answers: &[Answer]) -> Vec<String> {
    answers
        .iter()
        .filter_map(|answer| {
            let line = format!(
                "{row} {} meaning={meaning:?}: {:?}",
                answer.route, answer.verdict
            );
            eprintln!("OBSERVED {line}");
            let agrees = match (&answer.verdict, meaning) {
                (Verdict::Proved(_), Meaning::True) | (Verdict::Refuted(_), Meaning::False) => true,
                (Verdict::Vacuous(found), Meaning::Vacuous(expected)) => *found == expected,
                (Verdict::Unsettled(_), _) => true,
                _ => false,
            };
            (!agrees).then_some(line)
        })
        .collect()
}

/// Whether every admitted behaviour the grids walk satisfies the temporal
/// declaration `name`; `None` when they walk none.
fn on_every_behaviour(world: &World, space: &Space, name: &str) -> Option<bool> {
    let system = world.system();
    falsifier::extrema(&system, space, &|b| {
        falsifier::holds(world, &system, name, b)
    })
    .map(|found| found.min == whole(1))
}

// --- QSL-3446: temporal claims over the two-step subject --------------------

/// `first` at `t` in [0, 1] and `second` at `t + 2`. With `t` strictly
/// inside (0, 1) no position is in [1, 2] ms, so each of these is false
/// (FR-243-AC-5).
const EMPTY: [(&str, &str); 4] = [
    ("Eventually", "eventually[1 ms, 2 ms] holds(c.phase >= 0)"),
    (
        "Until",
        "holds(c.phase >= 0) until[1 ms, 2 ms] holds(c.phase >= 0)",
    ),
    (
        "Nested",
        "eventually[0 ms, 0 ms] eventually[1 ms, 2 ms] holds(c.phase >= 0)",
    ),
    (
        "Untimed",
        "eventually (holds(c.phase = 0) and eventually[1 ms, 2 ms] holds(c.phase >= 0))",
    ),
];

/// `first` is by 1 ms, `second` 2 ms after it: each of these is true.
const TRUE: [(&str, &str); 2] = [
    ("Early", "eventually[0 ms, 2 ms] holds(c.phase >= 1)"),
    (
        "Response",
        "always (holds(c.phase = 1) implies eventually[0 ms, 2 ms] holds(c.phase = 2))",
    ),
];

/// `second` is by 3 ms and `phase` stays 2: true.
const SETTLED: (&str, &str) = (
    "Settled",
    "eventually[0 ms, 4 ms] always[0 ms, 1 ms] holds(c.phase = 2)",
);

fn two_step(claims: &[(&str, &str)]) -> World {
    let declarations: String = claims
        .iter()
        .map(|(name, form)| formula(name, form))
        .collect();
    world(
        &["x"],
        &[("first", &["phase", "x"]), ("second", &["phase"])],
        &two_step_body(&declarations),
    )
}

fn two_step_routes(claims: &[(&str, &str)], meaning: Meaning) {
    let world = two_step(claims);
    assert_none(
        claims
            .iter()
            .flat_map(|(name, _)| {
                let row = format!("3446-{name}");
                disagreeing(&row, meaning, &routes::command(&row, &world, name))
            })
            .collect(),
    );
}

#[trace("FR-243-AC-5")]
#[test]
fn row_3446_two_step_clauses_hand() {
    let space = Space::new(3, Lock::Excluded);
    let all: Vec<(&str, &str)> = EMPTY.into_iter().chain(TRUE).chain([SETTLED]).collect();
    let world = two_step(&all);
    let found: Vec<(&str, Option<bool>)> = all
        .iter()
        .map(|(name, _)| (*name, on_every_behaviour(&world, &space, name)))
        .collect();
    let expected: Vec<(&str, Option<bool>)> = all
        .iter()
        .enumerate()
        .map(|(at, (name, _))| (*name, Some(at >= EMPTY.len())))
        .collect();
    assert_eq!(found, expected);
}

#[trace("FR-243-AC-5")]
#[test]
#[ignore = "QSL-3446: en-1 proves clauses over empty windows that are false"]
fn row_3446_empty_window_clauses_routes() {
    two_step_routes(&EMPTY, Meaning::False);
}

#[trace("FR-243-AC-5")]
#[test]
fn row_3446_true_clauses_routes() {
    two_step_routes(&TRUE, Meaning::True);
}

#[trace("FR-243-AC-5")]
#[test]
#[ignore = "QSL-3480: en-1 settles a true eventually-always clause inconclusive replay-parity"]
fn row_3446_nested_true_clause_routes() {
    two_step_routes(&[SETTLED], Meaning::True);
}

// --- QSL-3446, QSL-3478: state-graph claims over a fractional trap ----------

const POSSIBLE: &str = "always possible Done using v over (c: M::Call) { c.phase >= 2 }\n";
const UNIQUE: &str =
    "unique path Done using v over (c: M::Call) from (c.phase = 1) to (c.phase >= 2);\n";

/// `first` at a time `t` in [0, 1] resets `x`; `y` reads the time. Phase 1
/// ends by `y = 2 ms`, where `zero` needs `x >= 2 ms` (`t = 0`).
fn trap(one_guard: &str, claim: &str) -> World {
    world(
        &["x", "y"],
        &[
            ("first", &["phase", "x"]),
            ("zero", &["phase"]),
            ("one", &["phase"]),
        ],
        &format!(
            "pre First using v on M::Call::first {{ self.phase = 0 }}\n\
             post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
             pre Zero using v on M::Call::zero {{ self.phase = 1 and self.y >= 2 ms and self.x >= 2 ms }}\n\
             post ZeroDone using v on M::Call::zero {{ self.phase = 2 }}\n\
             pre One using v on M::Call::one {{ self.phase = 1 and {one_guard} }}\n\
             post OneDone using v on M::Call::one {{ self.phase = 3 }}\n\
             time invariant Early when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
             time invariant Late when {{ self.phase = 1 }} {{ self.y <= 2 ms }}\n\
             {claim}{}",
            formula("Moved", "holds(c.phase >= 2)")
        ),
    )
}

/// `one` is never enabled.
const NEVER: &str = "false";
/// `one` is enabled only when `first` ran at 1 ms.
const AT_ONE: &str = "self.y >= 2 ms and self.x <= 1 ms";

/// A timed state in phase 1, `first` having run at half a millisecond, from
/// which no walked delay and step reaches phase 2 or 3: a reachable state
/// with no path to `c.phase >= 2`.
fn half_millisecond_is_trapped(world: &World) -> bool {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let initial = timed.initial().remove(0);
    let delayed = timed
        .delay(&initial, &ratio(1, 2))
        .expect("a decided delay")
        .expect("half a millisecond is admissible");
    let (_, reached): (_, TimedState) = timed.steps(&delayed).expect("decided steps").remove(0);
    (0..=16).all(|quarter| {
        timed
            .delay(&reached, &ratio(quarter, 4))
            .expect("a decided delay")
            .is_none_or(|later| timed.steps(&later).expect("decided steps").is_empty())
    })
}

#[trace("FR-852-AC-2", "FR-852-AC-3")]
#[test]
fn row_3478_fractional_trap_hand() {
    // Phase 1 after `first` at 0.5 ms: `x = y - 0.5`, so `zero` would need
    // `y >= 2.5` and `one` (`x <= 1` and `y >= 2`) `y <= 1.5`: neither runs
    // by `y = 2`. The state has no path to `c.phase >= 2`, so both
    // `always possible` and `unique path` are false.
    assert!(half_millisecond_is_trapped(&trap(NEVER, POSSIBLE)));
    assert!(half_millisecond_is_trapped(&trap(AT_ONE, POSSIBLE)));
}

/// The claim `claim` over the trap, on the route `route`: en-6 or en-1.
fn trap_route(row: &str, one_guard: &str, claim: &str, route: &str) {
    let world = trap(one_guard, claim);
    let answers: Vec<Answer> = routes::command(row, &world, "Done")
        .into_iter()
        .filter(|answer| answer.route == route)
        .collect();
    assert_eq!(answers.len(), 1, "{route} answers");
    assert_answers(row, Meaning::False, &answers);
}

#[trace("FR-852-AC-2")]
#[test]
#[ignore = "QSL-3478: en-6 proves a false always possible over a partly trapped zone"]
fn row_3478_always_possible_never_one_zones() {
    trap_route("3478-possible-never-zones", NEVER, POSSIBLE, "en-6");
}

#[trace("FR-852-AC-6")]
#[test]
fn row_3478_always_possible_never_one_digital() {
    trap_route("3478-possible-never-digital", NEVER, POSSIBLE, "en-1");
}

#[trace("FR-852-AC-2")]
#[test]
#[ignore = "QSL-3478: en-6 proves a false always possible over a partly trapped zone"]
fn row_3446_always_possible_one_at_one_zones() {
    trap_route("3446-possible-at-one-zones", AT_ONE, POSSIBLE, "en-6");
}

#[trace("FR-852-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 proves a false always possible over a fractional trap"]
fn row_3446_always_possible_one_at_one_digital() {
    trap_route("3446-possible-at-one-digital", AT_ONE, POSSIBLE, "en-1");
}

#[trace("FR-852-AC-3")]
#[test]
#[ignore = "QSL-3478: en-6 proves a false unique path over a partly trapped zone"]
fn row_3478_unique_path_never_one_zones() {
    trap_route("3478-unique-never-zones", NEVER, UNIQUE, "en-6");
}

#[trace("FR-852-AC-6")]
#[test]
fn row_3478_unique_path_never_one_digital() {
    trap_route("3478-unique-never-digital", NEVER, UNIQUE, "en-1");
}

#[trace("FR-852-AC-3")]
#[test]
#[ignore = "QSL-3478: en-6 settles unique path failed with established-invariant-broken"]
fn row_3446_unique_path_one_at_one_zones() {
    trap_route("3446-unique-at-one-zones", AT_ONE, UNIQUE, "en-6");
}

#[trace("FR-852-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 proves a false unique path over a fractional trap"]
fn row_3446_unique_path_one_at_one_digital() {
    trap_route("3446-unique-at-one-digital", AT_ONE, UNIQUE, "en-1");
}

// --- QSL-3483, QSL-3484: no behaviour and no instance -----------------------

fn stall(claims: &str) -> World {
    world(
        &["x"],
        &[("ping", &[])],
        &format!("time invariant Stall when {{ true }} {{ self.x <= 1 ms }}\n{claims}"),
    )
}

/// `ping` changes nothing and time cannot pass 1 ms: no behaviour's stamps
/// diverge (FR-232-AC-3).
fn stall_claims() -> World {
    stall(
        &(formula("Safe", "always holds(true)")
            + &formula("Live", "always eventually holds(true)")),
    )
}

#[trace("FR-232-AC-3")]
#[test]
fn row_3483_stalled_subject_hand() {
    let mut space = Space::new(1, Lock::Excluded);
    space.delays = Some(vec![whole(0), whole(1)]);
    assert_eq!(on_every_behaviour(&stall_claims(), &space, "Safe"), None);
}

#[trace("FR-232-AC-3", "FR-240-AC-4")]
#[test]
fn row_3483_stalled_subject_routes() {
    let world = stall_claims();
    assert_none(
        ["Safe", "Live"]
            .iter()
            .flat_map(|name| {
                let row = format!("3483-{name}");
                let vacuous = Meaning::Vacuous("no-admitted-behaviour");
                disagreeing(&row, vacuous, &routes::command(&row, &world, name))
            })
            .collect(),
    );
}

/// With no object under the `over` type, a claim has no instance
/// (FR-125-AC-13); `always holds(false)` is then neither proved nor refuted.
#[trace("FR-125-AC-13")]
#[test]
#[ignore = "QSL-3484: en-6 and en-1 prove a claim over no instance"]
fn row_3484_no_instance_routes() {
    let world = stall(&formula("Never", "always holds(false)"));
    let own = world.snapshots.values().next().expect("one snapshot");
    let mut snapshot: Value = serde_json::from_slice(own).expect("JSON");
    snapshot["populations"][0]["objects"] = Value::Array(Vec::new());
    let bytes = serde_json::to_vec(&snapshot).expect("serializes");
    assert_answers(
        "3484-no-instance",
        Meaning::Vacuous("no-instance"),
        &routes::command_over("3484", &world, "Never", &[bytes]),
    );
}

// --- QSL-3485: an undefined letter on a branch no behaviour passes ----------

fn locking_branch() -> World {
    world(
        &["x"],
        &[("block", &["phase"]), ("go", &["phase"])],
        &format!(
            "pre Block using v on M::Call::block {{ self.phase = 0 }}\n\
             post Blocked using v on M::Call::block {{ self.phase = 2 }}\n\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Went using v on M::Call::go {{ self.phase = 1 }}\n\
             time invariant Locked when {{ self.phase = 2 }} {{ self.x <= 0 ms }}\n{}",
            formula("Stays", "always holds(c.phase = 0)")
        ),
    )
}

/// `go` leads to a quiescent state with `phase` 1, so the claim is false.
#[trace("FR-632-AC-2")]
#[test]
fn row_3485_locking_branch_hand_and_routes() {
    let world = locking_branch();
    let space = Space::new(1, Lock::Excluded);
    assert_eq!(on_every_behaviour(&world, &space, "Stays"), Some(false));
    assert_answers(
        "3485-defined",
        Meaning::False,
        &routes::command("3485", &world, "Stays"),
    );
}

/// Reads the first false atom as undefined, and every other letter as
/// `inner` does.
struct UndefinedAtFirstFalseAtom<'a> {
    inner: &'a dyn ModelLetters,
    seen: Cell<bool>,
}

impl ModelLetters for UndefinedAtFirstFalseAtom<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let reading = self.inner.atom(atom, instance, state, meter)?;
        if matches!(reading, AtomReading::False) && !self.seen.replace(true) {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        Ok(reading)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.inner.activation(instance, state, meter)
    }

    fn captures(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<CaptureReading, AtomStop> {
        self.inner.captures(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

/// The undefined letter is at the time-locked `block` state, which no
/// admitted behaviour passes, so the violation through `go` stands.
#[trace("FR-632-AC-2")]
#[test]
#[ignore = "QSL-3485: en-1 drops the search after an undefined reading at a time lock"]
fn row_3485_undefined_at_a_time_lock_routes() {
    let world = locking_branch();
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("clause letters");
    let wrapped = UndefinedAtFirstFalseAtom {
        inner: &letters,
        seen: Cell::new(false),
    };
    let outcome = check_digital(
        &DigitalRequest {
            subject: &timed,
            clause: node.clause(),
            letters: &wrapped,
            limits: DigitalLimits {
                max_states: u64::MAX,
                max_transitions: u64::MAX,
                max_automaton_states: u64::MAX,
            },
        },
        &Cancel::new(),
    )
    .expect("the digital route accepts");
    eprintln!("OBSERVED 3485-undefined en-1: {:?}", outcome.outcome);
    assert!(wrapped.seen.get(), "the undefined reading was reached");
    assert!(
        matches!(outcome.outcome, ModelCheckOutcome::Violated { .. }),
        "{:?}",
        outcome.outcome
    );
}
