// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-179 (FSpec FR-400): explicit-state reachable-transition relation
//! checking and replay.

use crate::relation_claims_support::{
    initial_world, package_and_relation, package_and_relation_with_document,
    step_with_result_document, subjects, DETERMINISTIC,
};
use filament_exact::Cancel;
use fsl_analyze::model_check::{
    check_model_relation, ModelRelationCheckRequest, ModelRelationOutcome,
};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::Limits;
use fsl_replay::{
    replay_model_relation_tuple, HyperCounterexampleKind, ModelCheckLimit, ModelCheckLimits,
    ModelRelationReplay, ModelTraceSettlement, ModelTraceValue,
};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::Vault;

/// FR-179: the explicit checker exhausts the reachable transition product and
/// proves a relation that is invariant under the checked model's transitions.
#[trace("TC-604", "FR-179-AC-1")]
#[test]
fn a_holding_relation_is_checked_over_reachable_transition_tuples() {
    let (package, relation, document) = package_and_relation(Vault::Secure, DETERMINISTIC);
    let (packages, snapshots, initial) = initial_world(&document);
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Holds { stats, .. } = outcome else {
        panic!("the relation holds: {outcome:?}");
    };
    assert_eq!(stats.transitions, 8);
    assert_eq!(stats.relation_tuples, 64);
}

/// FR-179-AC-3 (TC-604): tuple limits and depth horizons preserve their
/// distinct stopping results.
#[trace("TC-604", "FR-179-AC-3")]
#[test]
fn relation_tuple_limit_and_depth_bound_are_reported() {
    let (package, relation, document) = package_and_relation(Vault::Secure, DETERMINISTIC);
    let (packages, snapshots, initial) = initial_world(&document);
    let limits = ModelCheckLimits {
        max_relation_tuples: 10,
        ..ModelCheckLimits::default()
    };
    let stopped = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits,
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Stopped { limit, stats, .. } = stopped else {
        panic!("the relation tuple limit stops the run: {stopped:?}");
    };
    assert_eq!(
        limit.expect("a named limit").limit,
        ModelCheckLimit::MaxRelationTuples
    );
    assert_eq!(stats.relation_tuples, 10);

    let bounded = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: Some(1),
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the bounded relation check is admitted");
    let ModelRelationOutcome::BoundReached { depth, stats } = bounded else {
        panic!("the relation depth is inconclusive: {bounded:?}");
    };
    assert_eq!(depth, 1);
    assert_eq!(stats.transitions, 4);
    assert_eq!(stats.relation_tuples, 16);
}

/// A graph budget stops relation exploration without settling an incomplete
/// retained graph as holding.
#[trace("TC-604", "FR-179-AC-3")]
#[test]
fn state_budget_exhaustion_does_not_settle_holds() {
    let (package, relation, document) = package_and_relation(Vault::Secure, DETERMINISTIC);
    let (packages, snapshots, initial) = initial_world(&document);
    let limits = ModelCheckLimits {
        limits: Limits::default().with_max_states(1),
        ..ModelCheckLimits::default()
    };
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits,
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the bounded relation check is admitted");
    assert!(
        matches!(outcome, ModelRelationOutcome::Stopped { .. }),
        "a graph budget is inconclusive: {outcome:?}"
    );
}

/// FR-179: a violated relation returns a transition-pair counterexample
/// whose final transitions are checked by the relation-specific replay path.
#[trace("TC-604", "FR-179-AC-2")]
#[test]
fn a_violated_relation_returns_a_replayable_step_tuple() {
    let nondeterministic = "post Step using v on V::Vault::step { self.l = i or self.l = 1 - i }\n";
    let (package, relation, document) =
        package_and_relation(Vault::Custom(nondeterministic), DETERMINISTIC);
    let (packages, snapshots, initial) = initial_world(&document);
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Refuted { counterexample, .. } = outcome else {
        panic!("the transition relation is refuted: {outcome:?}");
    };
    assert_eq!(
        counterexample.kind,
        fsl_replay::HyperCounterexampleKind::StepTuple
    );
    let subject_values = subjects(&package, &packages, &snapshots, &initial);
    let subject = subject_values.first().expect("one alias subject");
    let systems = [
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
    ];
    let replay = replay_model_relation_tuple(
        &ModelRelationReplay {
            relation: &relation,
            package: &package,
            systems: systems.iter().collect(),
            meter: UNLIMITED,
            cancel: &Cancel::new(),
        },
        &counterexample,
    )
    .expect("the step tuple replays");
    assert!(matches!(
        replay,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
}

/// FR-179 binds result values even when distinct results reach the same state.
#[trace("TC-604", "FR-179-AC-2")]
#[test]
fn relation_tuple_witness_preserves_coalesced_operation_results() {
    let relation_text = "relation EqualResult using v over \
        (x: V::Vault::step, y: V::Vault::step) { x.result = y.result }";
    let (package, relation, document) = package_and_relation_with_document(
        step_with_result_document(),
        Vault::Custom("post Step using v on V::Vault::step { result = i and self.l = 0 }\n"),
        relation_text,
    );
    let (packages, snapshots, initial) = initial_world(&document);
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Refuted {
        counterexample,
        stats,
    } = outcome
    else {
        panic!("different result pairs refute equality: {outcome:?}");
    };
    assert_eq!(stats.transitions, 4);
    assert_eq!(
        counterexample.traces[0].results.last().cloned().flatten(),
        Some(r#"{"type":"integer","value":"0"}"#.to_owned())
    );
    assert_eq!(
        counterexample.traces[1].results.last().cloned().flatten(),
        Some(r#"{"type":"integer","value":"1"}"#.to_owned())
    );

    let subject_values = subjects(&package, &packages, &snapshots, &initial);
    let subject = subject_values.first().expect("one alias subject");
    let systems = [
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
    ];
    let replay = replay_model_relation_tuple(
        &ModelRelationReplay {
            relation: &relation,
            package: &package,
            systems: systems.iter().collect(),
            meter: UNLIMITED,
            cancel: &Cancel::new(),
        },
        &counterexample,
    )
    .expect("the result-specific transition tuple replays");
    assert!(matches!(
        replay,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
}

/// FR-179-AC-4: the first undefined tuple is a refutation and replays with
/// its division-by-zero cause intact.
#[trace("TC-610", "FR-179-AC-4")]
#[test]
fn an_undefined_relation_tuple_is_replayed_as_undefined() {
    let relation =
        "relation Div using v over (x: V::Vault::step, y: V::Vault::step) { 1 mod x.i >= y.i }";
    let (package, relation, document) = package_and_relation(Vault::Secure, relation);
    let (packages, snapshots, initial) = initial_world(&document);
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Refuted { counterexample, .. } = outcome else {
        panic!("division by zero refutes the relation: {outcome:?}");
    };
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("undefined tuple has an Undefined counterexample: {counterexample:?}");
    };
    assert_eq!(undefined.locus, fsl_replay::HyperLocus::RelationBody);
    assert!(matches!(
        undefined.cause,
        fsl_eval::temporal::UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    ));
    assert_eq!(counterexample.traces[0].steps.len(), 1);

    let subject_values = subjects(&package, &packages, &snapshots, &initial);
    let subject = subject_values.first().expect("one alias subject");
    let systems = [
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
    ];
    let replay = replay_model_relation_tuple(
        &ModelRelationReplay {
            relation: &relation,
            package: &package,
            systems: systems.iter().collect(),
            meter: UNLIMITED,
            cancel: &Cancel::new(),
        },
        &counterexample,
    )
    .expect("the undefined tuple replays");
    assert!(matches!(
        replay,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::HyperUndefined(_),
            ..
        }
    ));
}

/// FR-179-AC-5: the first false tuple wins over later undefined tuples in
/// canonical transition order.
#[trace("TC-611", "FR-179-AC-5")]
#[test]
fn an_earlier_false_tuple_wins_over_later_undefined_tuples() {
    let relation = "relation Mod using v over (x: V::Vault::step, y: V::Vault::step) { 3 mod (2 - 2 * x.i) > y.i }";
    let (package, relation, document) = package_and_relation(Vault::Secure, relation);
    let (packages, snapshots, initial) = initial_world(&document);
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: subjects(&package, &packages, &snapshots, &initial),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the relation check is admitted");
    let ModelRelationOutcome::Refuted { counterexample, .. } = outcome else {
        panic!("the first false tuple refutes the relation: {outcome:?}");
    };
    assert_eq!(counterexample.kind, HyperCounterexampleKind::StepTuple);
    assert_eq!(counterexample.traces[0].steps.len(), 1);

    let subject_values = subjects(&package, &packages, &snapshots, &initial);
    let subject = subject_values.first().expect("one alias subject");
    let systems = [
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
        ModelSystem::new(subject.simulation.clone()).expect("a model system"),
    ];
    let replay = replay_model_relation_tuple(
        &ModelRelationReplay {
            relation: &relation,
            package: &package,
            systems: systems.iter().collect(),
            meter: UNLIMITED,
            cancel: &Cancel::new(),
        },
        &counterexample,
    )
    .expect("the false tuple replays");
    assert!(matches!(
        replay,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
}
