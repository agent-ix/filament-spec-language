// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3534 route probes: a `step` guard with no value at a reachable state.
//! ADR-018 UE-6 / ADR-022 RU-5: every item over the subject is refuted as an
//! undefined evaluation; none is witnessed or proved.

use filament_exact::Cancel;
use fsl_replay::{
    ExistentialOutcome, HyperCounterexampleKind, HyperOutcome, ModelCheckLimits,
    ProductClosureClause, ProductClosureRequest, StateGraphMethod,
};

use crate::counter::UNLIMITED;
use crate::hyper_world::{Vault, World, STARTS};

const QUOTIENT: &str = "function quotient using v(x: Integer): Integer pure { 1 div x }\n";

/// Leaky `step` whose guard has no value at `(h, l) = (0, 1)`, which
/// `step(1)` reaches from `(0, 0)`.
const GUARDED: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { quotient(1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

fn short<T: std::fmt::Debug>(value: &T) -> String {
    let mut text = format!("{value:?}");
    text.truncate(260);
    text
}

fn single_clause(profile: &str, body: &str) -> String {
    format!(
        "hyper Possible using {profile} over behaviours (v: V::Vault) \
         exists trace b of V {{ {body} }}"
    )
}

fn single(profile: &str, body: &str, starts: &[(i64, i64)], samples: bool) {
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
    );
    let record = record.map(|record| short(&format!("{:?}", record.record)));
    println!("OBSERVED outcome: {}", short(&instance.outcome));
    println!("OBSERVED record: {record:?}");
    assert!(
        matches!(instance.outcome, ExistentialOutcome::Undefined { .. }),
        "expected Undefined (UE-6); outcome {} ; record {record:?}",
        short(&instance.outcome)
    );
}

#[test]
fn probe_single_inf_witness_on_other_branch() {
    single("inf", "always holds(v.l @ b = 0)", &[(0, 0)], false);
}

#[test]
fn probe_single_inf_witness_on_other_branch_sampled() {
    single("inf", "always holds(v.l @ b = 0)", &[(0, 0)], true);
}

#[test]
fn probe_single_closed_witness_on_other_branch() {
    single("ep", "always [0, 2] holds(v.l @ b = 0)", &[(0, 0)], false);
}

#[test]
fn probe_single_closed_witness_through_undefined_state() {
    single("ep", "eventually [0, 2] holds(v.l @ b = 1)", &[(0, 0)], false);
}

#[test]
fn probe_single_closed_undefined_initial_state() {
    single("ep", "eventually [0, 2] holds(v.h @ b = 0)", &[(0, 1)], false);
}

fn hyper(declaration: &str, starts: &[(i64, i64)]) {
    let world =
        World::new_with_kernel_function(GUARDED, false, starts, declaration, QUOTIENT);
    let outcome = world
        .run(ModelCheckLimits::default(), &Cancel::new());
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(refusal) => panic!("expected an undefined counterexample; refused {refusal:?}"),
    };
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
    );
    let record = record.map(|record| format!("label {:?} value {}", record.label, short(&record.value)));
    println!("OBSERVED outcome: {}", short(&outcome));
    println!("OBSERVED record: {record:?}");
    assert!(
        matches!(&outcome, HyperOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, HyperCounterexampleKind::Undefined(_))),
        "expected an undefined counterexample (UE-6); outcome {} ; record {record:?}",
        short(&outcome)
    );
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

/// Universal, infinite trace, a body decided at position 0.
#[test]
fn probe_universal_inf_body_decided_at_position_zero() {
    hyper(
        &universal("inf", "", "holds(v.h @ a != v.h @ b) or holds(v.l @ a = v.l @ b)"),
        &STARTS,
    );
}

/// Universal, infinite trace, `match` keeps every tuple at `step(0)`, so the
/// state with no value is on no matched tuple.
#[test]
fn probe_universal_inf_match_goes_around() {
    hyper(&universal("inf", BOTH_ZERO, "always holds(v.l @ a = v.l @ a)"), &[(0, 0)]);
}

#[test]
fn probe_universal_closed_all_tuples() {
    hyper(&universal("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ a)"), &[(0, 0)]);
}

#[test]
fn probe_universal_closed_initial_undefined() {
    hyper(&universal("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ a)"), &[(0, 1)]);
}

#[test]
fn probe_forall_exists_inf_partner_goes_around() {
    hyper(
        &forall_exists("inf", A_ZERO, "always holds(v.h @ a = v.h @ a)"),
        &[(0, 0)],
    );
}

#[test]
fn probe_forall_exists_inf_match_goes_around() {
    hyper(
        &forall_exists("inf", BOTH_ZERO, "always holds(v.l @ a = v.l @ b)"),
        &[(0, 0)],
    );
}

#[test]
fn probe_forall_exists_closed() {
    hyper(
        &forall_exists("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ b)"),
        &[(0, 0)],
    );
}

#[test]
fn probe_forall_exists_closed_initial_undefined() {
    hyper(
        &forall_exists("ep", "", "eventually [0, 3] holds(v.h @ a = v.h @ b)"),
        &[(0, 1)],
    );
}
