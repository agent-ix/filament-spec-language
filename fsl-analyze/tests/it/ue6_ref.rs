// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3534 refinement probes: a reachable `step` guard with no value is
//! refuted as an undefined evaluation (ADR-020 RE-5, ADR-018 UE-6).

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_replay::{CounterexampleKind, ModelCheckLimits, RefinementFailure, SafetyOutcome};

const POSITION_PEEK: u64 = 4;
const POSITION_ABSTRACT: u64 = 4;
use ix_trace_rs::trace;

use crate::refinement_fixture::{cas_refines_counter, Variant, World, UNLIMITED};

const Q: &str = "function q using v(x: Integer): Integer pure { 6 mod x }\n";

/// The refinement's safety outcome over the counter pair with `contracts`.
fn outcome_of(contracts: &str) -> SafetyOutcome {
    let world = World::with_mode(
        Variant::CAS,
        &format!("{Q}{contracts}{}", cas_refines_counter()),
        CheckMode::Kernel,
    );
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            retained: None,
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
    .safety
}

/// The undefined contract the refinement is refuted with: the failure's
/// position and clause, after the counterexample's kind is checked.
fn refuted_with(contracts: &str) -> (u64, String) {
    let outcome = outcome_of(contracts);
    let SafetyOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a refutation (RE-5): {outcome:?}");
    };
    assert!(
        matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_)),
        "an undefined evaluation: {:?}",
        counterexample.kind
    );
    let Some(RefinementFailure::UndefinedContract { position, clause }) =
        counterexample.refinement.clone()
    else {
        panic!("the failure is an undefined contract: {:?}", counterexample.refinement);
    };
    (position, clause)
}

#[trace("FR-142-AC-9")]
#[test]
fn ue6_refinement_concrete_stutter_guard() {
    let (position, clause) = refuted_with(
        "pre PeekPre using v on Impl::Counter::peek { q(2 - self.value) >= 0 }\n\
         post PeekPost using v on Impl::Counter::peek { self.value = pre(self.value) }\n",
    );
    assert_eq!((position, clause.as_str()), (POSITION_PEEK, "PeekPre"));
}

#[trace("FR-142-AC-9")]
#[test]
fn ue6_refinement_abstract_guard() {
    let (position, clause) = refuted_with(
        "pre AbsGuard using v on Spec::Counter::inc { q(1 - self.value) >= 0 }\n\
         post AbsPost using v on Spec::Counter::inc { true }\n",
    );
    assert_eq!((position, clause.as_str()), (POSITION_ABSTRACT, "AbsGuard"));
}
