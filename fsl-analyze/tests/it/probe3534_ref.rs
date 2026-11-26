// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3534 refinement probes: a reachable `step` guard with no value is
//! refuted as an undefined evaluation (ADR-020 RE-5, ADR-018 UE-6).

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_replay::{CounterexampleKind, ModelCheckLimits, SafetyOutcome};

use crate::refinement_fixture::{cas_refines_counter, Variant, World, UNLIMITED};

const Q: &str = "function q using v(x: Integer): Integer pure { 6 mod x }\n";

fn probe(contracts: &str) {
    let world = World::with_mode(
        Variant::CAS,
        &format!("{Q}{contracts}{}", cas_refines_counter()),
        CheckMode::Kernel,
    );
    let outcome = check_refinement(
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
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
    .safety;
    let mut text = format!("{outcome:?}");
    text.truncate(300);
    println!("OBSERVED: {text}");
    assert!(
        matches!(&outcome, SafetyOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, CounterexampleKind::UndefinedEvaluation(_))),
        "expected an undefined evaluation (RE-5): {text}"
    );
}

#[test]
fn probe_refinement_concrete_stutter_guard() {
    probe(
        "pre PeekPre using v on Impl::Counter::peek { q(2 - self.value) >= 0 }\n\
         post PeekPost using v on Impl::Counter::peek { self.value = pre(self.value) }\n",
    );
}

#[test]
fn probe_refinement_abstract_guard() {
    probe(
        "pre AbsGuard using v on Spec::Counter::inc { q(1 - self.value) >= 0 }\n\
         post AbsPost using v on Spec::Counter::inc { true }\n",
    );
}
