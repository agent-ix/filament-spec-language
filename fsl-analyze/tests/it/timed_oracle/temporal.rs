// SPDX-License-Identifier: AGPL-3.0-or-later
//! The non-probabilistic rows of the corpus: temporal and state-graph claims
//! under the timed profile, on EN-6 (zones) and EN-1 (digital clocks). A
//! behaviour is admitted when its stamps diverge, so a time-locked or Zeno
//! behaviour is none. Each `_routes` row names the one outcome each route
//! must give; a falsifier-only `_hand` test carries no criterion tag.

use std::cell::Cell;

use filament_exact::{Cancel, Meter, Undefined};
use fsl_analyze::model_check::digital::{check_digital, DigitalLimits, DigitalRequest};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem, TimedState, TimedSystem};
use fsl_eval::simulation::StateKey;
use fsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use fsl_foundation::IdentityLimits;
use fsl_replay::ModelCheckOutcome;
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;
use serde_json::Value;

use super::falsifier::{self, ratio, whole, Lock, Space};
use super::models::{formula, two_step_body};
use super::routes::{self, Answer, Verdict};
use crate::timed_world::{self, World, DENSE};

/// The outcome one route must give for a claim.
#[derive(Clone, Debug)]
enum Expected {
    Proved,
    Refuted,
    /// `inconclusive` with this cause, never proved or refuted.
    Vacuous(&'static str),
    /// EN-1's decline of what it cannot digitise (FR-243), naming the
    /// `not_digitisable` rule.
    NotDigitisable(&'static str),
}

impl Expected {
    fn agrees(&self, verdict: &Verdict) -> bool {
        match (self, verdict) {
            (Self::Proved, Verdict::Proved(_)) | (Self::Refuted, Verdict::Refuted(_)) => true,
            (Self::Vacuous(expected), Verdict::Vacuous(found)) => expected == found,
            (Self::NotDigitisable(rule), Verdict::Declined(found)) => {
                found == &format!("declined:invalid_runtime_input/invalid-value:{rule}")
            }
            _ => false,
        }
    }
}

/// A world over the timed profile `ev` whose `temporal` claims are `claims`.
fn world(clocks: &[&str], operations: &[(&str, &[&str])], body: &str) -> World {
    timed_world::world(
        &timed_world::document(clocks, operations),
        super::models::PROFILES,
        &format!("{DENSE}{body}"),
    )
}

fn assert_none(wrong: Vec<String>) {
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The answers that differ from what their route must give: `en6` for the
/// zone route, `en1` for the digital one.
fn disagreeing(row: &str, en6: &Expected, en1: &Expected, answers: &[Answer]) -> Vec<String> {
    assert!(!answers.is_empty(), "{row}: a route answers");
    answers
        .iter()
        .filter_map(|answer| {
            let expected = match answer.route {
                "en-6" => en6,
                "en-1" => en1,
                route => panic!("{row}: no expectation for {route}"),
            };
            let line = format!(
                "{row} {} expected={expected:?}: {:?}",
                answer.route, answer.verdict
            );
            eprintln!("OBSERVED {line}");
            (!expected.agrees(&answer.verdict)).then_some(line)
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

/// Every claim gets `en6` from the zone route and, from the digital route,
/// the outcome beside it.
fn two_step_routes(claims: &[(&str, &str)], en6: Expected, en1: &[Expected]) {
    assert_eq!(claims.len(), en1.len());
    let world = two_step(claims);
    assert_none(
        claims
            .iter()
            .zip(en1)
            .flat_map(|((name, _), en1)| {
                let row = format!("3446-{name}");
                disagreeing(&row, &en6, en1, &routes::command(&row, &world, name))
            })
            .collect(),
    );
}

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
#[ignore = "QSL-3446: en-1 proves clauses it must decline as not digitisable"]
fn row_3446_empty_window_clauses_routes() {
    // The rule each of `EMPTY` breaks (FR-243 "Behavior").
    let rules = [
        "two-sided-interval",
        "two-sided-interval",
        "second-bounded-operator",
        "two-sided-interval",
    ];
    two_step_routes(
        &EMPTY,
        Expected::Refuted,
        &rules.map(Expected::NotDigitisable),
    );
}

#[trace("FR-243-AC-5")]
#[test]
fn row_3446_true_clauses_routes() {
    two_step_routes(
        &TRUE,
        Expected::Proved,
        &[Expected::Proved, Expected::Proved],
    );
}

#[test]
#[ignore = "QSL-3480: en-1 settles a true eventually-always clause inconclusive replay-parity"]
fn row_3446_nested_true_clause_routes() {
    two_step_routes(&[SETTLED], Expected::Proved, &[Expected::Proved]);
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

#[test]
fn row_3478_fractional_trap_hand() {
    // Phase 1 after `first` at 0.5 ms: `x = y - 0.5`, so `zero` would need
    // `y >= 2.5` and `one` (`x <= 1` and `y >= 2`) `y <= 1.5`: neither runs
    // by `y = 2`. The state has no path to `c.phase >= 2`, so both
    // `always possible` and `unique path` are false.
    assert!(half_millisecond_is_trapped(&trap(NEVER, POSSIBLE)));
    assert!(half_millisecond_is_trapped(&trap(AT_ONE, POSSIBLE)));
}

/// The claim `claim` over the trap on the zone route: false.
fn trap_zones(row: &str, one_guard: &str, claim: &str) {
    trap_route(row, one_guard, claim, "en-6", Expected::Refuted);
}

/// The claim over the trap on the digital route, which must decline the
/// state-graph form naming `rule` (FR-243-AC-6).
fn trap_digital(row: &str, one_guard: &str, claim: &str, rule: &'static str) {
    trap_route(
        row,
        one_guard,
        claim,
        "en-1",
        Expected::NotDigitisable(rule),
    );
}

fn trap_route(row: &str, one_guard: &str, claim: &str, route: &str, expected: Expected) {
    let world = trap(one_guard, claim);
    let answers: Vec<Answer> = routes::command(row, &world, "Done")
        .into_iter()
        .filter(|answer| answer.route == route)
        .collect();
    assert_eq!(answers.len(), 1, "{route} answers");
    assert_none(disagreeing(row, &expected, &expected, &answers));
}

#[test]
#[ignore = "QSL-3478: en-6 proves a false always possible over a partly trapped zone"]
fn row_3478_always_possible_never_one_zones() {
    trap_zones("3478-possible-never-zones", NEVER, POSSIBLE);
}

#[trace("FR-243-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 decides an always possible item it must decline"]
fn row_3478_always_possible_never_one_digital() {
    trap_digital(
        "3478-possible-never-digital",
        NEVER,
        POSSIBLE,
        "always-possible",
    );
}

#[test]
#[ignore = "QSL-3478: en-6 proves a false always possible over a partly trapped zone"]
fn row_3446_always_possible_one_at_one_zones() {
    trap_zones("3446-possible-at-one-zones", AT_ONE, POSSIBLE);
}

#[trace("FR-243-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 proves a false always possible it must decline"]
fn row_3446_always_possible_one_at_one_digital() {
    trap_digital(
        "3446-possible-at-one-digital",
        AT_ONE,
        POSSIBLE,
        "always-possible",
    );
}

#[test]
#[ignore = "QSL-3478: en-6 proves a false unique path over a partly trapped zone"]
fn row_3478_unique_path_never_one_zones() {
    trap_zones("3478-unique-never-zones", NEVER, UNIQUE);
}

#[trace("FR-243-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 decides a unique path item it must decline"]
fn row_3478_unique_path_never_one_digital() {
    trap_digital("3478-unique-never-digital", NEVER, UNIQUE, "unique-path");
}

#[test]
#[ignore = "QSL-3478: en-6 settles unique path failed with established-invariant-broken"]
fn row_3446_unique_path_one_at_one_zones() {
    trap_zones("3446-unique-at-one-zones", AT_ONE, UNIQUE);
}

#[trace("FR-243-AC-6")]
#[test]
#[ignore = "QSL-3446: en-1 proves a false unique path it must decline"]
fn row_3446_unique_path_one_at_one_digital() {
    trap_digital("3446-unique-at-one-digital", AT_ONE, UNIQUE, "unique-path");
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
                let vacuous = Expected::Vacuous("no-admitted-behaviour");
                disagreeing(
                    &row,
                    &vacuous,
                    &vacuous,
                    &routes::command(&row, &world, name),
                )
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
    let vacuous = Expected::Vacuous("no-instance");
    assert_none(disagreeing(
        "3484-no-instance",
        &vacuous,
        &vacuous,
        &routes::command_over("3484", &world, "Never", &[bytes]),
    ));
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
#[test]
fn row_3485_locking_branch_hand_and_routes() {
    let world = locking_branch();
    let space = Space::new(1, Lock::Excluded);
    assert_eq!(on_every_behaviour(&world, &space, "Stays"), Some(false));
    assert_none(disagreeing(
        "3485-defined",
        &Expected::Refuted,
        &Expected::Refuted,
        &routes::command("3485", &world, "Stays"),
    ));
}

fn state_key(system: &ModelSystem<'_>, state: &ModelSimState) -> StateKey {
    system
        .state_key(state, IdentityLimits::default())
        .expect("the state encodes")
        .1
}

/// Reads every atom at the state whose key is `at` as undefined, and every other letter
/// as `inner` does.
struct UndefinedAt<'a> {
    inner: &'a dyn ModelLetters,
    system: &'a ModelSystem<'a>,
    at: StateKey,
    seen: Cell<bool>,
}

impl ModelLetters for UndefinedAt<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if state_key(self.system, state) == self.at {
            self.seen.set(true);
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.inner.atom(atom, instance, state, meter)
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
#[test]
#[ignore = "QSL-3485: en-1 drops the search after an undefined reading at a time lock"]
fn row_3485_undefined_at_a_time_lock_routes() {
    let world = locking_branch();
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("clause letters");
    let initial = timed.initial().remove(0);
    let (_, blocked) = timed
        .steps(&initial)
        .expect("decided steps")
        .into_iter()
        .find(|(step, _)| falsifier::named(step.operation(), "block"))
        .expect("block runs from the initial state");
    assert!(
        timed
            .time_lock(&blocked, 64)
            .expect("a decided time lock")
            .is_some(),
        "the block state is time-locked"
    );
    let wrapped = UndefinedAt {
        inner: &letters,
        system: &system,
        at: state_key(&system, blocked.discrete()),
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
