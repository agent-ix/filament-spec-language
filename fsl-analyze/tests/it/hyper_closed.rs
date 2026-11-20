// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-173-AC-6 (TC-2243): an event-position hyper clause reaches a verdict
//! through EN-1 over closed finite executions, never `FormNotDecided`, and
//! its counterexample replays as closed traces with no loop.
//!
//! The fixtures are FR-173-AC-6's: two Secure vaults `V` and `N` with the one
//! initial state `h = 1`, `l = 0`, whose `step(i)` sets `l = i` for `i` in
//! `{0, 1}` and which have no terminal state.

use filament_exact::Cancel;
use fsl_replay::{
    settle_hyper_check, ExistentialOutcome, HyperCounterexampleKind, HyperOutcome, HyperStep,
    ModelCheckLimits, ModelTraceSettlement, ModelTraceValue, ResultLabel, TerminalValue,
};
use fsl_route::reductions::Reductions;
use fsl_semantics::check::{HyperForm, TemporalProfile};
use ix_trace_rs::trace;

use crate::hyper::refuted;
use crate::hyper_world::{Vault, World, KEY};

/// The Secure vault twice, `V` and `N`, started from `h = 1`, `l = 0`.
fn two_models(quantifiers: &str, body: &str) -> World {
    let declaration = format!(
        "hyper Saves using ep over behaviours (v: V::Vault, w: N::Vault) \
         {quantifiers} {{ {body} }}"
    );
    let vault = [(KEY, (1, 0))];
    World::with_second_vaults(Vault::Secure, &vault, &vault, &declaration)
}

fn run(world: &World) -> HyperOutcome {
    world
        .run_selecting(Reductions::default())
        .expect("the run is not refused")
}

fn settled(world: &World, outcome: &HyperOutcome) -> (ResultLabel, TerminalValue) {
    let record = settle_hyper_check(
        outcome,
        &ModelCheckLimits::default(),
        &[],
        |counterexample| world.replay(counterexample),
    )
    .expect("the outcome settles");
    (record.label, record.value)
}

const FORALL_FORALL: &str = "forall trace a of V forall trace b of N";
const FORALL_EXISTS: &str = "forall trace a of V exists trace b of N";

/// HP-2: the two vaults' `l` differ, so the clause is refuted by one closed
/// trace of four positions per variable, neither with a loop, and it replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_universal_clause_is_refuted_by_closed_traces() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.l @ a = w.l @ b)");
    assert_eq!(
        world.clause.profile(),
        TemporalProfile::EventPositionFalseExtension
    );
    assert_eq!(world.clause.form(), Some(&HyperForm::Universal));
    let outcome = run(&world);
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert_eq!(counterexample.traces.len(), 2);
    for trace in &counterexample.traces {
        assert_eq!(trace.steps.len(), 3, "four positions, 0 to 3");
        assert!(trace
            .steps
            .iter()
            .all(|step| matches!(step, HyperStep::Model(_))));
    }
    let replayed = world.replay(&counterexample).expect("the replay reads it");
    assert!(
        matches!(
            replayed,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{replayed:?}"
    );
    assert_eq!(settled(&world, &outcome).0, ResultLabel::Refuted);
}

/// HP-2: the two vaults' `h` never change, so the clause holds on every tuple
/// of closed executions and settles `proved`.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_universal_clause_that_holds_settles_proved() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.h @ a = w.h @ b)");
    let outcome = run(&world);
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    let (label, value) = settled(&world, &outcome);
    assert_eq!(label, ResultLabel::Proved);
    assert!(matches!(value, TerminalValue::Proved { .. }), "{value:?}");
}

/// HP-3: every universal trace has an existential partner of the same `l`, so
/// the clause settles `proved`; with `!=` no partner exists at position 0 and
/// the counterexample is a universal closed trace alone, which replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_forall_exists_clause_settles_by_its_witness_search() {
    let held = two_models(FORALL_EXISTS, "always [0, 3] holds(v.l @ a = w.l @ b)");
    assert_eq!(held.clause.form(), Some(&HyperForm::ForallExistsSafety));
    let outcome = run(&held);
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    assert_eq!(settled(&held, &outcome).0, ResultLabel::Proved);

    let broken = two_models(FORALL_EXISTS, "always [0, 3] holds(v.l @ a != w.l @ b)");
    let outcome = run(&broken);
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert_eq!(counterexample.trace_position, 0);
    assert_eq!(counterexample.traces.len(), 1, "the universal trace alone");
    assert_eq!(counterexample.traces[0].steps.len(), 3);
    assert_eq!(settled(&broken, &outcome).0, ResultLabel::Refuted);
}

/// A counterexample whose trace is not closed (it stops before the horizon at
/// a state that enables steps) is refused by replay.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn replay_refuses_a_trace_that_is_not_closed() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.l @ a = w.l @ b)");
    let mut counterexample = refuted(run(&world));
    for trace in &mut counterexample.traces {
        trace.steps.pop();
    }
    assert!(world.replay(&counterexample).is_err());
}

/// FR-178-AC-1's `align skip { V::Vault::mix }` clause under the event-position
/// profile over `body`.
fn aligned(body: &str) -> String {
    format!(
        "hyper NonInterference using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ (if present(a.step.Vault::step.i) and present(b.step.Vault::step.i) \
         then value(a.step.Vault::step.i) = value(b.step.Vault::step.i) else true) }} \
         {{ {body} }}"
    )
}

/// HP-6: the busy vault from its idle initial states holds `always [0, 2]
/// l @ a = l @ b` on its projection; the leaky vault started mid-operation is
/// refuted by a projected counterexample that replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_projection_aligned_clause_reaches_a_verdict() {
    let body = "always [0, 2] holds(v.l @ a = v.l @ b)";
    let held = World::busy(Vault::Busy, &crate::hyper_projected::IDLE, &aligned(body));
    assert_eq!(held.clause.form(), Some(&HyperForm::ProjectionAligned));
    let outcome = held
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused");
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    assert_eq!(settled(&held, &outcome).0, ResultLabel::Proved);
    let leaky = World::busy(
        Vault::BusyLeaky,
        &crate::hyper_projected::MID_OPERATION,
        &aligned(body),
    );
    let outcome = leaky
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused");
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.traces.len(), 2);
    let replayed = leaky.replay(&counterexample).expect("the replay reads it");
    assert!(
        matches!(replayed, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
    assert_eq!(settled(&leaky, &outcome).0, ResultLabel::Refuted);
}

/// HP-5: `exists trace b of V { eventually [0, 2] holds(v.l @ b = 1) }` is
/// witnessed by one closed trace of three positions with no loop, and the
/// same clause over `v.h @ b = 0` (`h` stays 1) is trapped at the initial
/// state.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_single_existential_clause_is_witnessed_or_trapped() {
    let single = |body: &str| {
        let declaration = format!(
            "hyper Possible using ep over behaviours (v: V::Vault) exists trace b of V {{ {body} }}"
        );
        World::new(Vault::Secure, false, &[(1, 0)], &declaration)
    };
    let witnessed = single("eventually [0, 2] holds(v.l @ b = 1)");
    assert_eq!(witnessed.clause.form(), Some(&HyperForm::SingleExistential));
    let run = witnessed
        .run_single(ModelCheckLimits::default(), None, &Cancel::new())
        .expect("the run is not refused");
    let [instance] = run.instances.as_slice() else {
        panic!("one instance: {run:?}");
    };
    let ExistentialOutcome::ClosedWitnessed { paths } = &instance.outcome else {
        panic!("a closed witness: {instance:?}");
    };
    let [path] = paths.as_slice() else {
        panic!("one witness per initial state: {paths:?}");
    };
    assert_eq!((path.initial, path.steps.len()), (0, 2), "three positions");

    let trapped = single("eventually [0, 2] holds(v.h @ b = 0)");
    let run = trapped
        .run_single(ModelCheckLimits::default(), None, &Cancel::new())
        .expect("the run is not refused");
    let [instance] = run.instances.as_slice() else {
        panic!("one instance: {run:?}");
    };
    assert!(
        matches!(&instance.outcome, ExistentialOutcome::Trapped { stem } if stem.initial == 0 && stem.steps.is_empty()),
        "{instance:?}"
    );
}
