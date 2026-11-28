// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 UE-6 over the hyper routes: a `step` guard with no value at a
//! reachable state refutes the clause. No item over the subject is witnessed
//! or proved past it.
//!
//! The leaky vault's `step(i)` takes `(h, l)` to `(h, 0)` when `i = h` and to
//! `(h, 1)` otherwise, for `i` in `{0, 1}`. Its guard `1 div (1 - l + h)` has
//! no value exactly at `(0, 1)`, which `step(1)` reaches from `(0, 0)` and
//! `step(0)` never does.

use filament_exact::{Cancel, Undefined};
use fsl_analyze::model_check::ModelCheckRefusal;
use fsl_eval::hyper::{
    read_closed_undefined, ClosedExecution, ClosedInputs, ClosedLocus, ClosedRead, ClosedStep,
    MatchReading, ParameterBinding, StepLabel, StepMatch,
};
use fsl_eval::temporal::UndefinedCause;
use fsl_replay::{
    ExistentialOutcome, HyperCounterexampleKind, HyperLocus, HyperOutcome, ModelCheckLimits,
    ProductClosureClause, ProductClosureRequest, StateGraphMethod, UndefinedLocus,
};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{Vault, World, STARTS};

const QUOTIENT: &str = "function quotient using v(x: Integer): Integer pure { 1 div x }\n";

const GUARDED: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { quotient(1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

const DIVISION: UndefinedCause = UndefinedCause::Kernel(Undefined::DivisionByZero);

fn single_clause(profile: &str, body: &str) -> String {
    format!(
        "hyper Possible using {profile} over behaviours (v: V::Vault) \
         exists trace b of V {{ {body} }}"
    )
}

/// The one-existential item over `starts`, decided with `witness_samples` as
/// asked and settled: the outcome and the label of its terminal record.
fn single(
    profile: &str,
    body: &str,
    starts: &[(i64, i64)],
    samples: bool,
) -> (ExistentialOutcome, String) {
    single_in(GUARDED, profile, body, starts, samples)
}

fn single_in(
    vault: Vault,
    profile: &str,
    body: &str,
    starts: &[(i64, i64)],
    samples: bool,
) -> (ExistentialOutcome, String) {
    let world = World::new_with_kernel_function(
        vault,
        false,
        starts,
        &single_clause(profile, body),
        QUOTIENT,
    );
    let limits = ModelCheckLimits {
        witness_samples: if samples {
            ModelCheckLimits::default().witness_samples
        } else {
            0
        },
        ..ModelCheckLimits::default()
    };
    let run = world
        .run_single(limits, None, &Cancel::new())
        .expect("not refused");
    let instance = &run.instances[0];
    let subjects = [world.subject()];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: ProductClosureClause::Hyper(&world.clause),
        subjects: &subjects,
        meter: UNLIMITED,
    };
    let record = fsl_replay::settle_existential(
        &request,
        instance,
        &StateGraphMethod {
            limits,
            max_depth: None,
            seed: 0,
        },
        &Cancel::new(),
    )
    .expect("settled");
    (
        instance.outcome.clone(),
        format!("{:?}", record.record.label),
    )
}

/// The outcome is `Undefined` at `position` along `steps` steps, naming the
/// `Guard` conjunction, and the settled record is `Refuted`.
fn assert_undefined(run: (ExistentialOutcome, String), position: u64) {
    assert_undefined_in(run, position, "Guard");
}

fn assert_undefined_in(run: (ExistentialOutcome, String), position: u64, clause: &str) {
    let (outcome, label) = run;
    let ExistentialOutcome::Undefined { stem, undefined } = outcome else {
        panic!("expected Undefined, got {outcome:?}");
    };
    assert_eq!(undefined.site.position, position);
    assert_eq!(u64::try_from(stem.steps.len()).unwrap(), position);
    assert_eq!(
        undefined.site.locus,
        UndefinedLocus::Contract(clause.to_owned())
    );
    assert_eq!(undefined.cause, DIVISION);
    assert_eq!(label, "Refuted");
}

/// An infinite-trace witness at `(0, 0)` that loops on `step(0)` is no result
/// when `step(1)` reaches `(0, 1)`: the item is refuted at position 1, with
/// phase 0 sampling on and off.
#[trace("TC-616", "FR-181-AC-15")]
#[test]
fn a_single_infinite_witness_on_another_branch_is_refuted() {
    for samples in [false, true] {
        let run = single("inf", "always holds(v.l @ b = 0)", &[(0, 0)], samples);
        assert_undefined(run, 1);
    }
}

/// Over an event-position clause from `(0, 0)`, `always [0, 2] holds(l = 0)`
/// holds on the `step(0)` execution, and `eventually [0, 2] holds(l = 1)` on
/// the `step(1)` one: neither is a closed witness, since `(0, 1)` is reached
/// at position 1 and its expansion has no value.
#[trace("TC-616", "FR-181-AC-15")]
#[test]
fn a_single_closed_witness_past_the_guard_is_refuted() {
    assert_undefined(
        single("ep", "always [0, 2] holds(v.l @ b = 0)", &[(0, 0)], false),
        1,
    );
    assert_undefined(
        single(
            "ep",
            "eventually [0, 2] holds(v.l @ b = 1)",
            &[(0, 0)],
            false,
        ),
        1,
    );
}

/// An initial state with no value is position 0 with an empty path.
#[trace("TC-616", "FR-181-AC-15")]
#[test]
fn a_single_closed_undefined_initial_state_is_position_zero() {
    assert_undefined(
        single(
            "ep",
            "eventually [0, 2] holds(v.l @ b = 1)",
            &[(0, 1)],
            false,
        ),
        0,
    );
}

/// An undefined postcondition names no transition, and it replays too: the
/// vault's `Div` conjunction divides by the post-state's `l`, which is 0 after
/// `step(0)` from `(0, 0)`, so the initial state's expansion already has no
/// value (ADR-018 UE-6).
#[trace("TC-616", "FR-181-AC-15")]
#[test]
fn an_undefined_postcondition_refutes_and_replays() {
    const POSTCONDITION: Vault = Vault::Custom(
        "post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n\
         post Div using v on V::Vault::step { quotient(self.l) >= 0 }\n",
    );
    assert_undefined_in(
        single_in(
            POSTCONDITION,
            "inf",
            "always holds(v.l @ b = 0)",
            &[(0, 0)],
            false,
        ),
        0,
        "Div",
    );
}

fn hyper(
    declaration: &str,
    starts: &[(i64, i64)],
) -> Result<(HyperOutcome, String), ModelCheckRefusal> {
    let world = World::new_with_kernel_function(GUARDED, false, starts, declaration, QUOTIENT);
    let outcome = world.run(ModelCheckLimits::default(), &Cancel::new())?;
    let subjects = (0..world.clause.aliases().len())
        .map(|_| world.subject())
        .collect::<Vec<_>>();
    let certificate = ProductClosureRequest {
        package: &world.package,
        clause: ProductClosureClause::Hyper(&world.clause),
        subjects: &subjects,
        meter: UNLIMITED,
    };
    let record = fsl_replay::settle_hyper_check(
        &outcome,
        &ModelCheckLimits::default(),
        &[],
        Some(&certificate),
        &Cancel::new(),
        |counterexample| world.replay(counterexample),
    )
    .expect("settled");
    Ok((outcome, format!("{:?}", record.label)))
}

fn universal(profile: &str, matched: &str, body: &str) -> String {
    format!(
        "hyper Same using {profile} over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V {matched} {{ {body} }}"
    )
}

fn forall_exists(profile: &str, matched: &str, body: &str) -> String {
    format!(
        "hyper Some using {profile} over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V {matched} {{ {body} }}"
    )
}

const BOTH_ZERO: &str =
    "match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
     and (if present(b.step.Vault::step.i) then value(b.step.Vault::step.i) else 0) = 0 }";
const A_ZERO: &str =
    "match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
     and b.step.op != stutter }";

/// The outcome is an `undefined` counterexample at `position` naming the
/// `Guard` conjunction of exactly `named`, and its record is `Refuted`.
fn assert_refuted(
    run: Result<(HyperOutcome, String), ModelCheckRefusal>,
    position: u64,
    named: &[&str],
) {
    let (outcome, label) = run.expect("not refused");
    let HyperOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected a counterexample, got {outcome:?}");
    };
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected the undefined kind, got {:?}", counterexample.kind);
    };
    assert_eq!(undefined.position, position);
    assert_eq!(counterexample.trace_position, position);
    assert_eq!(undefined.locus, HyperLocus::Contract("Guard".to_owned()));
    assert_eq!(undefined.cause, DIVISION);
    let variables: Vec<&str> = undefined
        .variables
        .iter()
        .map(|(variable, _)| variable.as_str())
        .collect();
    assert_eq!(variables, named);
    assert_eq!(label, "Refuted");
}

/// The closed forms refute at the state whose guard has no value, with the
/// contract clause as the locus (ADR-023 HV-8).
fn assert_closed_undefined_refuted(run: Result<(HyperOutcome, String), ModelCheckRefusal>) {
    let (outcome, label) = run.expect("not refused");
    let HyperOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected undefined, got {:?}", counterexample.kind);
    };
    assert_eq!(undefined.locus, HyperLocus::Contract("Guard".to_owned()));
    assert_eq!(undefined.cause, DIVISION);
    assert_eq!(label, "Refuted");
}

/// Universal, infinite: the body is decided true at position 0 on every
/// start, yet `(0, 1)` is a component state at position 1. Canonical order
/// tries `a` on `step(0)` first, so the first tuple to reach it has `b` on `step(1)`.
#[trace("TC-602", "FR-176-AC-5")]
#[test]
fn a_universal_infinite_body_decided_at_position_zero_is_refuted_at_the_guard() {
    assert_refuted(
        hyper(
            &universal(
                "inf",
                "",
                "holds(v.h @ a != v.h @ b) or holds(v.l @ a = v.l @ b)",
            ),
            &STARTS,
        ),
        1,
        &["b"],
    );
}

/// `match` keeps only the joint steps whose both components take `step(0)`,
/// so no admitted tuple reaches `(0, 1)`: the state is outside the clause's
/// scope (ADR-023 HM-3, HV-8), and the clause holds.
#[trace("TC-602", "FR-176-AC-5")]
#[test]
fn a_state_no_matched_universal_tuple_reaches_is_outside_the_clause() {
    let (outcome, label) = hyper(
        &universal("inf", BOTH_ZERO, "always holds(v.l @ a = v.l @ a)"),
        &[(0, 0)],
    )
    .expect("not refused");
    assert!(matches!(outcome, HyperOutcome::Holds { .. }), "{outcome:?}");
    assert_eq!(label, "Proved");
}

/// The same scope reading for `forall` `exists`: both components stay on
/// `step(0)`.
#[trace("TC-602", "FR-177-AC-7")]
#[test]
fn a_state_no_matched_partner_reaches_is_outside_the_clause() {
    let (outcome, label) = hyper(
        &forall_exists("inf", BOTH_ZERO, "always holds(v.l @ a = v.l @ b)"),
        &[(0, 0)],
    )
    .expect("not refused");
    assert!(matches!(outcome, HyperOutcome::Holds { .. }), "{outcome:?}");
    assert_eq!(label, "Proved");
}

/// `a` stays on `step(0)` but the existential member `b` may take `step(1)`
/// into `(0, 1)`: the member's guard has no value there, which refutes at
/// position 1 and names `b` alone.
#[trace("TC-602", "FR-177-AC-7")]
#[test]
fn an_existential_member_at_the_guard_refutes_the_universal_side() {
    assert_refuted(
        hyper(
            &forall_exists("inf", A_ZERO, "always holds(v.h @ a = v.h @ a)"),
            &[(0, 0)],
        ),
        1,
        &["b"],
    );
}

/// Closed finite executions of length 4 reach `(0, 1)` by `step(1)` from
/// `(0, 0)` and start there from `(0, 1)`: the state's guard has no value, so
/// both closed forms refute there with the contract clause as the locus
/// (ADR-023 HV-8), never proving the clause.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn closed_universal_and_forall_exists_refute_at_the_guard() {
    for start in [(0, 0), (0, 1)] {
        assert_closed_undefined_refuted(hyper(
            &universal("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ a)"),
            &[start],
        ));
        assert_closed_undefined_refuted(hyper(
            &forall_exists("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ b)"),
            &[start],
        ));
    }
}

/// A `match` that has no value on a joint step (the first component's input
/// is 0, which `quotient` cannot divide by) refutes the clause as undefined
/// (ADR-023 HV-8); that step is not a match-excluded one.
fn match_undefined(profile: &str, quantifier: &str, body: &str) -> String {
    format!(
        "hyper Mu using {profile} over behaviours (v: V::Vault) \
         forall trace a of V {quantifier} trace b of V \
         match {{ quotient(if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) >= 0 }} \
         {{ {body} }}"
    )
}

fn match_run(declaration: &str) -> Result<HyperOutcome, ModelCheckRefusal> {
    let world =
        World::new_with_kernel_function(Vault::Secure, false, &[(0, 0)], declaration, QUOTIENT);
    world.run(ModelCheckLimits::default(), &Cancel::new())
}

#[trace("TC-602")]
#[test]
fn an_undefined_match_refutes_the_infinite_universal_and_forall_exists() {
    for quantifier in ["forall", "exists"] {
        let outcome = match_run(&match_undefined(
            "inf",
            quantifier,
            "always holds(v.l @ a = v.l @ a)",
        ))
        .expect("not refused");
        let HyperOutcome::Violated { counterexample, .. } = outcome else {
            panic!("{quantifier}: expected a counterexample, got {outcome:?}");
        };
        let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
            panic!(
                "{quantifier}: expected undefined, got {:?}",
                counterexample.kind
            );
        };
        assert!(
            matches!(undefined.locus, HyperLocus::Match { .. }),
            "{quantifier}: {:?}",
            undefined.locus
        );
    }
}

/// An undefined `match` on a joint step of the closed forms refutes the
/// clause with a `match` locus (ADR-023 HV-8), settled `Refuted`; a `match`
/// the evaluator refuses stays `MatchUndetermined`.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn an_undefined_match_refutes_the_closed_forms() {
    for quantifier in ["forall", "exists"] {
        let declaration = match_undefined(
            "ep",
            quantifier,
            "eventually [0, 2] holds(v.l @ a = v.l @ a)",
        );
        let world = World::new_with_kernel_function(
            Vault::Secure,
            false,
            &[(0, 0)],
            &declaration,
            QUOTIENT,
        );
        let (outcome, label) = settled(&world);
        let HyperOutcome::Violated { counterexample, .. } = outcome else {
            panic!("{quantifier}: expected a counterexample, got {outcome:?}");
        };
        let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
            panic!(
                "{quantifier}: expected undefined, got {:?}",
                counterexample.kind
            );
        };
        assert!(
            matches!(
                undefined.locus,
                HyperLocus::Match { .. } | HyperLocus::ExistentialMatch { .. }
            ),
            "{quantifier}: {:?}",
            undefined.locus
        );
        assert_eq!(label, "Refuted", "{quantifier}");
    }
}

/// The outcome of `world`'s clause and the label it settles with.
fn settled(world: &World) -> (HyperOutcome, String) {
    let outcome = world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("not refused");
    let subjects = (0..world.clause.aliases().len())
        .map(|_| world.subject())
        .collect::<Vec<_>>();
    let certificate = ProductClosureRequest {
        package: &world.package,
        clause: ProductClosureClause::Hyper(&world.clause),
        subjects: &subjects,
        meter: UNLIMITED,
    };
    let record = fsl_replay::settle_hyper_check(
        &outcome,
        &ModelCheckLimits::default(),
        &[],
        Some(&certificate),
        &Cancel::new(),
        |counterexample| world.replay(counterexample),
    )
    .expect("settled");
    let label = format!("{:?}", record.label);
    (outcome, label)
}

/// The refutation `declaration` settles with over `vault` from `starts`: its
/// undefined evaluation, after the label is checked.
fn refuted_undefined(
    vault: Vault,
    starts: &[(i64, i64)],
    declaration: &str,
) -> fsl_replay::HyperUndefined {
    let world = World::new_with_kernel_function(vault, false, starts, declaration, QUOTIENT);
    let (outcome, label) = settled(&world);
    let HyperOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected a counterexample, got {outcome:?}");
    };
    let HyperCounterexampleKind::Undefined(undefined) = counterexample.kind else {
        panic!("expected undefined, got {:?}", counterexample.kind);
    };
    assert_eq!(label, "Refuted");
    undefined
}

/// An atom of the body with no value at a position the executions reach
/// refutes with the atom as the locus, not the guard (ADR-023 HV-8): from
/// `(0, 0)` the body reads `quotient(v.l @ a)` at `l = 0`.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_body_atom_with_no_value_refutes_at_the_atom() {
    let undefined = refuted_undefined(
        Vault::Secure,
        &[(0, 0)],
        &universal("ep", "", "eventually [0, 3] holds(quotient(v.l @ a) >= 0)"),
    );
    assert!(
        matches!(undefined.locus, HyperLocus::Atom(_)),
        "{:?}",
        undefined.locus
    );
}

/// A two-subject clause reaches the guard on one component and every other
/// component holds the prefix of its own: both traces are named, replayed
/// and settled.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_two_subject_clause_refutes_with_every_component_traced() {
    let undefined = refuted_undefined(
        GUARDED,
        &[(0, 0)],
        &universal("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ b)"),
    );
    assert_eq!(undefined.locus, HyperLocus::Contract("Guard".to_owned()));
}

/// Tuples whose executions end at different lengths: the guard of `(0, 0)`
/// fails at `(0, 1)` after `step(1)`, the other component being terminal.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_tuple_with_a_terminal_component_replays() {
    const TERMINAL: Vault = Vault::Custom(
        "pre Guard using v on V::Vault::step { self.h = 0 and quotient(1 - self.l + self.h) >= 0 }\n\
         post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
    );
    let undefined = refuted_undefined(
        TERMINAL,
        &[(0, 0), (1, 0)],
        &universal("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ b)"),
    );
    assert_eq!(undefined.locus, HyperLocus::Contract("Guard".to_owned()));
}

/// The body atom is read on the prefix before the contract is claimed
/// (ADR-018 UE-4): from `(0, 0)` the atom has no value at position 0, though
/// the guard has none at position 1.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_body_atom_is_read_before_the_contract() {
    let undefined = refuted_undefined(
        GUARDED,
        &[(0, 0)],
        &universal("ep", "", "eventually [0, 3] holds(quotient(v.l @ a) >= 0)"),
    );
    assert!(
        matches!(undefined.locus, HyperLocus::Atom(_)),
        "{:?}",
        undefined.locus
    );
    assert_eq!(undefined.position, 0);
}

/// `V` is the secure vault, defined everywhere; `N` is the leaky vault whose
/// guard `NGuard` has no value at `(h, l) = (0, 1)`, which `step(1)` reaches
/// from `(0, 0)`: two different subject models, only `N` undefined.
const N_GUARDED: Vault = Vault::Custom(
    "post Step using v on V::Vault::step { self.l = i }\n\
     pre NGuard using v on N::Vault::step { quotient(1 - self.l + self.h) >= 0 }\n\
     post NStep using v on N::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

fn two_subjects(quantifiers: &str) -> World {
    let declaration = format!(
        "hyper Saves using ep over behaviours (v: V::Vault, w: N::Vault) \
         {quantifiers} {{ eventually [0, 3] holds(v.h @ a = w.h @ b) }}"
    );
    World::with_second_model_kernel_function(N_GUARDED, &[(0, 0)], &declaration, QUOTIENT)
}

/// The two-subject clause refutes at position 1 with `NGuard` as the locus,
/// naming `b` alone, its counterexample's traces taking `lengths` steps, and
/// the refutation replays and settles `Refuted`.
fn assert_n_guard_refutes(quantifiers: &str, lengths: [usize; 2]) {
    let world = two_subjects(quantifiers);
    let (outcome, label) = settled_two(&world);
    let HyperOutcome::Violated { counterexample, .. } = outcome else {
        panic!("expected a counterexample, got {outcome:?}");
    };
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected undefined, got {:?}", counterexample.kind);
    };
    assert_eq!(undefined.locus, HyperLocus::Contract("NGuard".to_owned()));
    assert_eq!(undefined.position, 1);
    let variables: Vec<&str> = undefined
        .variables
        .iter()
        .map(|(variable, _)| variable.as_str())
        .collect();
    assert_eq!(variables, ["b"]);
    let taken: Vec<usize> = counterexample
        .traces
        .iter()
        .map(|trace| trace.steps.len())
        .collect();
    assert_eq!(taken, lengths);
    assert_eq!(label, "Refuted");
}

/// FR-173-AC-7 over two different subjects, `forall` `forall`: the guard of
/// `N` has no value at the state `b` reaches after one step. The tuple is the
/// joint prefix that reaches the state, one step of each component, so both
/// traces take one step.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_two_subject_universal_clause_refutes_at_the_joint_prefix() {
    assert_n_guard_refutes("forall trace a of V forall trace b of N", [1, 1]);
}

/// FR-177 and ADR-018 UE-6: an existential member's undefined contract
/// conjunction is an undefined evaluation of that member's component. Only
/// `N`, the existential component's subject, has the guard with no value:
/// `forall` `exists` refutes with `NGuard` as the locus, naming `b` alone;
/// `a` is at its initial state, `b` took one step.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn an_existential_members_undefined_guard_refutes_naming_the_member() {
    assert_n_guard_refutes("forall trace a of V exists trace b of N", [0, 1]);
}

/// FR-183 reads, at one joint position, the body's atoms before the contract
/// conjunctions (ADR-018 UE-4). The body is an `or` of one atom per component,
/// and a component past its last position reads false (FR-043), so the atom
/// has no value at position 1 on every tuple that reaches `l = 1`, the state
/// whose guard has no value too: the atom is the locus, at position 1,
/// whichever component took the step.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_closed_atom_at_a_position_is_read_before_that_positions_contract() {
    let undefined = refuted_undefined(
        GUARDED,
        &[(0, 0)],
        &universal(
            "ep",
            "",
            "eventually [0, 3] holds(quotient(1 - v.l @ a) < 0 or quotient(1 - v.l @ b) < 0)",
        ),
    );
    assert!(
        matches!(undefined.locus, HyperLocus::Atom(_)),
        "{:?}",
        undefined.locus
    );
    assert_eq!(undefined.position, 1);
}

/// [`settled`] without a product-closure certificate, for a world of two
/// subject models.
fn settled_two(world: &World) -> (HyperOutcome, String) {
    let outcome = world
        .run_selecting(fsl_route::reductions::Reductions::default())
        .expect("not refused");
    let record = fsl_replay::settle_hyper_check(
        &outcome,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |counterexample| world.replay(counterexample),
    )
    .expect("settled");
    let label = format!("{:?}", record.label);
    (outcome, label)
}

/// The closed execution of `system` that takes `step(i)` for each of
/// `inputs` from its initial state.
fn walk(system: &fsl_eval::simulation::model::ModelSystem<'_>, inputs: &[i64]) -> ClosedExecution {
    use fsl_eval::hyper::StepLabel;
    use fsl_eval::simulation::TransitionSystem;
    let mut states = vec![system
        .initial()
        .into_iter()
        .next()
        .expect("an initial state")];
    let mut steps = Vec::new();
    for input in inputs {
        let last = states.last().expect("a state");
        let expansion = system.successors(last).expect("the state expands");
        let (transition, next) = expansion
            .successors
            .into_iter()
            .find(|(transition, _)| {
                crate::hyper_world::input_of(&StepLabel::Operation(transition)) == Some(*input)
            })
            .expect("the step is enabled");
        let (_, post_state) = system
            .state_key(&next, fsl_foundation::IdentityLimits::default())
            .expect("the state has a key");
        steps.push(ClosedStep {
            transition,
            post_state,
            skipped: false,
        });
        states.push(next);
    }
    ClosedExecution {
        initial: 0,
        states,
        steps,
    }
}

/// What `read_closed_undefined` reads first over `a` taking `a_inputs` and `b`
/// taking `b_inputs`: the locus and the position.
fn first_read(a_inputs: &[i64], b_inputs: &[i64]) -> (ClosedLocus, u64) {
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &universal(
            "ep",
            "",
            "eventually [0, 3] holds(quotient(1 - v.l @ b) < 0)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], a_inputs);
    let b = walk(systems[1], b_inputs);
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &seams,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    match read_closed_undefined(&inputs, &systems, &[&a, &b]) {
        ClosedRead::Undefined(found) => (found.locus, found.position),
        other => panic!("expected an undefined expression, got {other:?}"),
    }
}

/// FR-183 "Undefined evaluation": the joint positions are read in increasing
/// order (ADR-018 UE-4). `b`'s body atom `quotient(1 - v.l @ b)` has no value
/// at position 2, where `b` has taken `step(0)` and `step(1)` to `l = 1`.
/// When `a` has taken `step(1)` to the state whose guard `Guard` has no value
/// at position 1, that contract is read first; when `a` has taken `step(0)`,
/// no contract has no value before position 2, where the atom precedes the
/// contract of `b`'s own state.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn a_contract_at_an_earlier_position_is_read_before_a_later_atom() {
    assert_eq!(
        first_read(&[1], &[0, 1]),
        (ClosedLocus::Contract("Guard".to_owned()), 1)
    );
    let (locus, position) = first_read(&[0], &[0, 1]);
    assert!(matches!(locus, ClosedLocus::Atom(_)), "{locus:?}");
    assert_eq!(position, 2);
}

/// FR-177 reads every existential partner of a universal tuple: one whose
/// atom has no value at position 1 refutes, and a partner that holds does not
/// witness the tuple, wherever it stands in the list. The clause reads
/// `quotient(1 - w.l @ b)` from position 1: the partner that never leaves
/// `l = 0` holds there, the partner that takes `step(1)` reaches `l = 1`,
/// where the quotient has no value.
fn partner_search(partners_first: bool) -> (usize, u64) {
    use fsl_eval::hyper::{search_partner, PartnerSearch};
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &forall_exists(
            "ep",
            "",
            "eventually [1, 3] holds(quotient(1 - v.l @ b) >= 1)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[0, 0]);
    let holding = walk(systems[1], &[0, 0]);
    let undefined = walk(systems[1], &[1]);
    let partners = if partners_first {
        vec![holding, undefined]
    } else {
        vec![undefined, holding]
    };
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &seams,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    match search_partner(&inputs, &[&a], &[partners.as_slice()], 100) {
        PartnerSearch::Undefined {
            found,
            existentials,
        } => {
            assert!(
                matches!(found.locus, ClosedLocus::Atom(_)),
                "{:?}",
                found.locus
            );
            (existentials[0], found.position)
        }
        other => panic!("expected an undefined partner, got {other:?}"),
    }
}

#[trace("TC-2243", "FR-177-AC-13")]
#[test]
fn a_holding_partner_before_an_undefined_one_does_not_witness() {
    assert_eq!(partner_search(true), (1, 1));
}

#[trace("TC-2243", "FR-177-AC-13")]
#[test]
fn a_holding_partner_after_an_undefined_one_does_not_witness() {
    assert_eq!(partner_search(false), (0, 1));
}

/// ADR-018 UE-4 and FR-177: the search for the shortest undefined prefix is
/// charged one unit of the request's state limit for each state it expands,
/// from its own count, and a search that passes the limit refuses with the
/// limit stop rather than report the walk's longer prefix. Over the vault
/// from `(0, 0)`, `step(0)` returns to the initial state and `step(1)` reaches
/// `l = 1`, where the guard has no value: the search expands the initial state
/// and the `l = 1` state, two states, and finds the one-step prefix `[1]`
/// that is shorter than the walk's three-step prefix `[0, 0, 1]`.
#[trace("TC-2243")]
#[test]
fn the_shortest_undefined_search_is_charged_to_the_limit() {
    use fsl_eval::hyper::{shortest_undefined, ClosedStop};
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &universal(
            "ep",
            "",
            "eventually [0, 3] holds(quotient(1 - v.l @ a) < 0)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let system = systems[0];
    let walked = walk(system, &[0, 0]);
    let mut longer = walked.clone();
    let reached = walk(system, &[1]);
    longer.states.push(reached.states[1].clone());
    longer.steps.push(reached.steps[0].clone());
    let skips = std::collections::BTreeSet::new();
    let cancel = Cancel::new();
    match shortest_undefined(system, 3, 1, &skips, &cancel, &longer) {
        Err(ClosedStop::Executions { limit }) => assert_eq!(limit, 1),
        other => panic!(
            "expected the limit stop, got {:?}",
            other.map(|found| found.map(|f| f.0))
        ),
    }
    let found = shortest_undefined(system, 3, 2, &skips, &cancel, &longer)
        .expect("within the limit")
        .expect("a shorter prefix exists");
    assert_eq!(found.0.steps.len(), 1);
}

/// A `match` that reads `μ_E` as holding on every step but `step(0)` of the
/// existential component, whose reading is incomplete: the stand-in for a
/// partner whose `match` the evaluator cannot finish.
struct IncompleteOnZero<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for IncompleteOnZero<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        _meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        if steps.get(1).and_then(crate::hyper_world::input_of) == Some(0) {
            filament_exact::Meter::new(crate::counter::ZERO)
                .charge(filament_exact::Charge::new(
                    filament_exact::ChargePoint::GraphExpand,
                ))
                .map_err(|incomplete| fsl_eval::hyper::MatchIncomplete {
                    conjunct: 0,
                    incomplete,
                })?;
        }
        Ok(MatchReading::Holds)
    }
}

/// FR-177: the verdict on a universal tuple does not depend on the order of
/// its partners. One partner whose `match` is incomplete (`step(0)`) does not
/// hide another whose atom has no value at position 1 (`step(1)`, where
/// `l = 1`): the search reports the undefined partner whichever comes first.
fn partner_search_with_incomplete(incomplete_first: bool) -> (usize, u64) {
    use fsl_eval::hyper::{search_partner, PartnerSearch};
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        "hyper Part using ep over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { eventually [1, 3] holds(quotient(1 - v.l @ b) >= 1) }",
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let incomplete = walk(systems[1], &[0]);
    let undefined = walk(systems[1], &[1]);
    let partners = if incomplete_first {
        vec![incomplete, undefined]
    } else {
        vec![undefined, incomplete]
    };
    let matcher = IncompleteOnZero(world.seams());
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &matcher,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    match search_partner(&inputs, &[&a], &[partners.as_slice()], 100) {
        PartnerSearch::Undefined {
            found,
            existentials,
        } => (existentials[0], found.position),
        other => panic!("expected an undefined partner, got {other:?}"),
    }
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn an_incomplete_partner_before_an_undefined_one_does_not_hide_it() {
    assert_eq!(partner_search_with_incomplete(true), (1, 1));
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn an_incomplete_partner_after_an_undefined_one_does_not_hide_it() {
    assert_eq!(partner_search_with_incomplete(false), (0, 1));
}

/// A `match` that refuses `μ_E` at a joint step whose existential input is 1
/// unless the previous step read was input 0: a partner `[1]` is refused at
/// joint 1, a partner `[0, 1]` is read through to joint 2.
struct RefusedAtOne<'w> {
    seams: crate::hyper_world::Seams<'w>,
    last: std::cell::Cell<Option<i64>>,
}

impl StepMatch for RefusedAtOne<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        self.seams.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        _meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        let input = steps.get(1).and_then(crate::hyper_world::input_of);
        let previous = self.last.replace(input);
        if input == Some(1) && previous != Some(0) {
            filament_exact::Meter::new(crate::counter::ZERO)
                .charge(filament_exact::Charge::new(
                    filament_exact::ChargePoint::GraphExpand,
                ))
                .map_err(|incomplete| fsl_eval::hyper::MatchIncomplete {
                    conjunct: 0,
                    incomplete,
                })?;
        }
        Ok(MatchReading::Holds)
    }
}

/// ADR-023 HV-8 HP-3 and FR-177 "Ranking findings": the finding at the least
/// joint position is the tuple's. Partner `[1]` is refused at joint 1 and
/// partner `[0, 1]` has an atom with no value at position 2 (`l = 1` there),
/// so by hand the refused stop ranks first and the tuple is `Undecided`,
/// whichever partner is listed first.
fn partner_search_refused_before_undefined(refused_first: bool) -> String {
    use fsl_eval::hyper::search_partner;
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        "hyper Part using ep over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { eventually [2, 3] holds(quotient(1 - v.l @ b) >= 1) }",
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let refused = walk(systems[1], &[1]);
    let undefined = walk(systems[1], &[0, 1]);
    let partners = if refused_first {
        vec![refused, undefined]
    } else {
        vec![undefined, refused]
    };
    let matcher = RefusedAtOne {
        seams: world.seams(),
        last: std::cell::Cell::new(None),
    };
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &matcher,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    format!(
        "{:?}",
        search_partner(&inputs, &[&a], &[partners.as_slice()], 100)
    )
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn a_partner_refused_at_joint_1_ranks_before_an_undefined_one_at_joint_2() {
    let settled = partner_search_refused_before_undefined(true);
    assert!(
        settled.starts_with("Stopped(MatchUndetermined") && settled.contains("position: 1"),
        "{settled}"
    );
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn an_undefined_partner_at_joint_2_listed_first_does_not_outrank_a_refusal_at_joint_1() {
    let settled = partner_search_refused_before_undefined(false);
    assert!(
        settled.starts_with("Stopped(MatchUndetermined") && settled.contains("position: 1"),
        "{settled}"
    );
}

/// FR-177 "Ranking findings": a refused `match` at joint 1 does not hide a
/// body atom with no value at position 0 (`l = 0` at the initial state, so
/// `1 div l` has no value). By hand the atom at 0 is the least finding.
#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn a_refused_match_does_not_hide_a_body_atom_undefined_at_an_earlier_position() {
    use fsl_eval::hyper::search_partner;
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &forall_exists(
            "ep",
            "match { a.step.Vault::step.i = b.step.Vault::step.i }",
            "eventually [0, 3] holds(quotient(v.l @ b) >= 0)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let partners = vec![walk(systems[1], &[0])];
    let matcher = IncompleteOnZero(world.seams());
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &matcher,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    let settled = format!(
        "{:?}",
        search_partner(&inputs, &[&a], &[partners.as_slice()], 100)
    );
    assert!(
        settled.starts_with("Undefined") && settled.contains("position: 0"),
        "{settled}"
    );
}

/// `μ_E` refused at input 1 whatever the previous step: partner `[0, 1]`
/// stops at joint 2, partner `[1]` at joint 1.
struct RefusedAtEveryOne<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for RefusedAtEveryOne<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        _meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        if steps.get(1).and_then(crate::hyper_world::input_of) == Some(1) {
            filament_exact::Meter::new(crate::counter::ZERO)
                .charge(filament_exact::Charge::new(
                    filament_exact::ChargePoint::GraphExpand,
                ))
                .map_err(|incomplete| fsl_eval::hyper::MatchIncomplete {
                    conjunct: 0,
                    incomplete,
                })?;
        }
        Ok(MatchReading::Holds)
    }
}

/// With two stopped partners the one that stopped at the least joint position
/// settles, whichever is listed first (a keep-first search settles joint 2
/// when `[0, 1]` is listed first).
fn two_stopped_partners(late_first: bool) -> String {
    use fsl_eval::hyper::search_partner;
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &forall_exists(
            "ep",
            "match { a.step.Vault::step.i = b.step.Vault::step.i }",
            "holds(v.l @ b >= 0)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let late = walk(systems[1], &[0, 1]);
    let early = walk(systems[1], &[1]);
    let partners = if late_first {
        vec![late, early]
    } else {
        vec![early, late]
    };
    let matcher = RefusedAtEveryOne(world.seams());
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &seams,
        matcher: &matcher,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    format!(
        "{:?}",
        search_partner(&inputs, &[&a], &[partners.as_slice()], 100)
    )
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn of_two_stopped_partners_the_earlier_stop_settles_whichever_is_listed_first() {
    for late_first in [true, false] {
        let settled = two_stopped_partners(late_first);
        assert!(
            settled.starts_with("Stopped(MatchUndetermined") && settled.contains("position: 1"),
            "{late_first}: {settled}"
        );
    }
}

/// The body's `holds` atom runs out of meter on every state of the wrapped
/// execution (the stand-in for a body whose evaluation stops where the
/// evaluator reports no position).
struct IncompleteOn<'w> {
    seams: crate::hyper_world::Seams<'w>,
    states: &'w [fsl_eval::simulation::model::ModelSimState],
}

impl fsl_eval::hyper::HyperLetters for IncompleteOn<'_> {
    fn atom(
        &self,
        atom: fsl_semantics::check::FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&fsl_eval::simulation::model::ModelSimState],
        meter: &mut filament_exact::Meter,
    ) -> Result<fsl_eval::temporal::AtomReading, fsl_eval::model_subject::AtomStop> {
        if states
            .get(1)
            .is_some_and(|state| self.states.iter().any(|own| std::ptr::eq(own, *state)))
        {
            filament_exact::Meter::new(crate::counter::ZERO)
                .charge(filament_exact::Charge::new(
                    filament_exact::ChargePoint::GraphExpand,
                ))
                .map_err(fsl_eval::model_subject::AtomStop::Incomplete)?;
        }
        self.seams.atom(atom, instance, states, meter)
    }
}

/// FR-177 "Ranking findings": a body stop at no reported position ranks at 0,
/// so it leaves the tuple `Undecided` against an undefined partner at
/// position 2, whichever is listed first, and is never a refutation.
fn body_stop_before_later_undefined(stopped_first: bool) -> String {
    use fsl_eval::hyper::search_partner;
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &forall_exists(
            "ep",
            "",
            "eventually [2, 3] holds(quotient(1 - v.l @ b) >= 1)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let stopped = walk(systems[1], &[0, 0, 0]);
    let undefined = walk(systems[1], &[0, 1]);
    let partners = if stopped_first {
        vec![stopped, undefined]
    } else {
        vec![undefined, stopped]
    };
    let index = if stopped_first { 0 } else { 1 };
    let letters = IncompleteOn {
        seams: world.seams(),
        states: &partners[index].states,
    };
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &letters,
        matcher: &seams,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    format!(
        "{:?}",
        search_partner(&inputs, &[&a], &[partners.as_slice()], 100)
    )
}

#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn a_body_meter_stop_ranks_first_and_never_yields_to_a_later_undefined_partner() {
    for stopped_first in [true, false] {
        let settled = body_stop_before_later_undefined(stopped_first);
        assert!(
            settled.starts_with("Stopped(Incomplete") && settled.contains("position: 0"),
            "{stopped_first}: {settled}"
        );
    }
}

/// `μ_E` has no value on every joint step.
struct UndefinedMatch<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for UndefinedMatch<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut filament_exact::Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        Ok(MatchReading::Undefined {
            conjunct: 0,
            cause: DIVISION,
        })
    }
}

/// FR-177 "Ranking findings": `μ_E` has no value at joint 1 and the body's
/// evaluation then runs out of meter at no reported position (rank 0). By hand
/// the stop at position 0 is before the undefined `match` at position 1, so the
/// tuple is `Undecided`, never refuted at joint 1.
#[trace("TC-2243", "FR-177-AC-14")]
#[test]
fn a_body_meter_stop_ranks_before_an_undefined_match_at_joint_1() {
    use fsl_eval::hyper::search_partner;
    let world = World::new_with_kernel_function(
        GUARDED,
        false,
        &[(0, 0)],
        &forall_exists(
            "ep",
            "match { a.step.Vault::step.i = b.step.Vault::step.i }",
            "holds(v.l @ b >= 0)",
        ),
        QUOTIENT,
    );
    let seams = world.seams();
    let systems = seams.per_trace();
    let a = walk(systems[0], &[]);
    let partners = vec![walk(systems[1], &[0])];
    let matcher = UndefinedMatch(world.seams());
    let letters = IncompleteOn {
        seams: world.seams(),
        states: &partners[0].states,
    };
    let instance = crate::hyper_world::instance();
    let cancel = Cancel::new();
    let inputs = ClosedInputs {
        clause: &world.clause,
        instance: &instance,
        letters: &letters,
        matcher: &matcher,
        meter: UNLIMITED,
        cancel: &cancel,
    };
    let settled = format!(
        "{:?}",
        search_partner(&inputs, &[&a], &[partners.as_slice()], 100)
    );
    assert!(
        settled.starts_with("Stopped(Incomplete") && settled.contains("position: 0"),
        "{settled}"
    );
}
