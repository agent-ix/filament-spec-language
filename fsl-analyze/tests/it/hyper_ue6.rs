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
use fsl_analyze::model_check::{HyperRefusal, ModelCheckRefusal};
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
    let world = World::new_with_kernel_function(
        GUARDED,
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
    let (outcome, label) = run;
    let ExistentialOutcome::Undefined { stem, undefined } = outcome else {
        panic!("expected Undefined, got {outcome:?}");
    };
    assert_eq!(undefined.site.position, position);
    assert_eq!(u64::try_from(stem.steps.len()).unwrap(), position);
    assert_eq!(
        undefined.site.locus,
        UndefinedLocus::Contract("Guard".to_owned())
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
            "eventually [0, 2] holds(v.h @ b = 0)",
            &[(0, 1)],
            false,
        ),
        0,
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

const BOTH_ZERO: &str = "match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
     and (if present(b.step.Vault::step.i) then value(b.step.Vault::step.i) else 0) = 0 }";
const A_ZERO: &str = "match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
     and b.step.op != stutter }";

/// The outcome is an `undefined` counterexample at `position` naming the
/// `Guard` conjunction of exactly `named`, and its record is `Refuted`.
fn assert_refuted(run: Result<(HyperOutcome, String), ModelCheckRefusal>, position: u64, named: &[&str]) {
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

fn assert_closed_undefined_refused(run: Result<(HyperOutcome, String), ModelCheckRefusal>) {
    assert!(
        matches!(
            run,
            Err(ModelCheckRefusal::Hyper(HyperRefusal::ClosedUndefined))
        ),
        "expected the ClosedUndefined refusal, got {run:?}"
    );
}

/// Universal, infinite: the body is decided true at position 0 on every
/// start, yet `(0, 1)` is a component state at position 1. Canonical order
/// tries `a` on `step(0)` first, so the first tuple to reach it has `b` on `step(1)`.
#[trace("TC-602", "FR-176-AC-4")]
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
#[trace("TC-602", "FR-176-AC-4")]
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

/// Closed finite executions of length 4 reach `(0, 1)` by `step(1)` from `(0, 0)`
/// and start there from `(0, 1)`: no execution ends at a state whose guard has no
/// value, so the clause is not proved.
#[trace("TC-2243", "FR-173-AC-7")]
#[test]
fn closed_universal_and_forall_exists_do_not_prove_past_the_guard() {
    for start in [(0, 0), (0, 1)] {
        assert_closed_undefined_refused(hyper(
            &universal(
                "ep",
                "",
                "eventually [0, 3] holds(v.h @ a = v.h @ a)",
            ),
            &[start],
        ));
        assert_closed_undefined_refused(hyper(
            &forall_exists(
                "ep",
                "",
                "eventually [0, 3] holds(v.h @ a = v.h @ b)",
            ),
            &[start],
        ));
    }
}
