// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 model-based run record over a checked counter invariant.

use super::*;

use fsl_eval::simulation::model::{
    run_model_based_state_invariant, run_model_based_test, ModelBasedTestError,
    ModelBasedTestEvidence, ModelOperationCoverageReason, ModelOperationCoverageReasonKind,
    ModelOperationCoverageStatus, ModelOperationCoverageSummaryStatus,
};

const INC: &str = "pre CanInc using v on M::Counter::inc { self.value < 5 }\n\
                   post Inc using v on M::Counter::inc { self.value = pre(self.value) + 1 }";
const RESET: &str = "pre CanReset using v on M::Counter::reset { self.value = 5 }\n\
                     post Reset using v on M::Counter::reset { self.value = 0 }";
const JAM: &str = "pre CanJam using v on M::Counter::jam { self.value > 5 }\n\
                   post Jam using v on M::Counter::jam { self.value = 0 }";

fn counter_run_world(invariant: &str, expression: &str) -> World {
    let clauses = format!(
        "{INC}\n{RESET}\n{JAM}\ninvariant {invariant} using v on M::Counter at current {{ {expression} }}"
    );
    world_with_level_bound(
        vec![
            operation("inc", &[], None, frame(&["value"], false, false)),
            operation("reset", &[], None, frame(&["value"], false, false)),
            operation("jam", &[], None, frame(&["value"], false, false)),
        ],
        &clauses,
        5,
    )
}

fn counter_temporal_world(formula: &str) -> World {
    let clauses = format!(
        "{INC}\n{RESET}\n{JAM}\n\
         temporal ModelProperty using ep over (c: M::Counter) clock \"model-steps\" on origin {{ {formula} }}"
    );
    let mut document: Json = serde_json::from_slice(&counters_document(
        vec![
            operation("inc", &[], None, frame(&["value"], false, false)),
            operation("reset", &[], None, frame(&["value"], false, false)),
            operation("jam", &[], None, frame(&["value"], false, false)),
        ],
        false,
    ))
    .expect("fixture JSON");
    let level = document["types"]
        .as_array_mut()
        .expect("types array")
        .iter_mut()
        .find(|ty| ty["identity"] == ident("Level"))
        .expect("Level declaration");
    *level = bound_type("Level", 5);
    build_world_document_with_header(
        serde_json::to_vec(&document).expect("fixture serializes"),
        &clauses,
        "profile ep = \"filament.temporal.event-position.false-extension/v1\";\n",
        false,
        false,
    )
    .expect("the temporal fixture checks")
}

/// FR-752-AC-4 / FSpec FR-190-AC-11: passing cases cover enabled operations,
/// while exhaustive graph reachability proves jam unreachable.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-11")]
#[test]
fn passing_run_reports_native_operation_coverage() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let result = run_model_based_state_invariant(
        &system,
        "Safe",
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the checked invariant and bounded model run");
    let repeated = run_model_based_state_invariant(
        &system,
        "Safe",
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the same model-based request is reproducible");

    assert_eq!(result.counts.passing, 100);
    assert_eq!(result.counts.failing, 0);
    assert_eq!(result.counts.discarded, 0);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(result.profile, sampler());
    assert_eq!(result.provenance.len(), 100);
    assert_eq!(result.provenance[0].seed, 7);
    assert_eq!(result.provenance[0].trace, 0);
    assert_eq!(result.partitions, repeated.partitions);
    assert_eq!(result.operations, repeated.operations);
    let counter_domain = IntegerInterval::new(0_i64.into(), 5_i64.into())
        .expect("the fixture's counter domain is finite");
    let counter_partitions: Vec<_> = result
        .partitions
        .iter()
        .filter(|partition| partition.domain == counter_domain)
        .map(|partition| (partition.kind, partition.value.to_string()))
        .collect();
    assert_eq!(
        counter_partitions,
        [
            (ModelBoundaryKind::LowerRefusal, "-1".to_owned()),
            (ModelBoundaryKind::LowerEndpoint, "0".to_owned()),
            (ModelBoundaryKind::UpperEndpoint, "5".to_owned()),
            (ModelBoundaryKind::UpperRefusal, "6".to_owned()),
        ]
    );
    assert!(result.counterexample.is_none());
    assert_eq!(result.operations.len(), 3);
    assert_eq!(
        result
            .operations
            .iter()
            .map(|entry| entry.operation.name.as_str())
            .collect::<Vec<_>>(),
        ["inc", "jam", "reset"]
    );
    assert_eq!(
        result.operations[0].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        result.operations[2].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        result.operations[1].status,
        fsl_eval::simulation::model::ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Unreachable,
        }
    );
    assert_eq!(
        result.coverage.status,
        ModelOperationCoverageSummaryStatus::Partial
    );
    assert_eq!(
        result
            .coverage
            .uncovered
            .get(&ModelOperationCoverageReasonKind::Unreachable),
        Some(&1)
    );
}

/// FR-752-AC-4 / FSpec FR-190-AC-12: the bounded graph never turns a missing
/// sampled invocation into an unreachable claim.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-12")]
#[test]
fn bounded_run_reports_unreachable_within_bound() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let result = run_model_based_state_invariant(
        &system,
        "Safe",
        &sampler(),
        7,
        100,
        3,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the bounded reachability run");
    assert_eq!(result.counts.passing, 100);
    assert_eq!(
        result.operations[2].status,
        fsl_eval::simulation::model::ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::UnreachableWithinBound,
        }
    );
}

/// FR-752-AC-4 / FSpec FR-190-AC-13: a passing run stays tested; a failing
/// case is minimized and retained beside operation coverage.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-13")]
#[test]
fn failing_run_retains_replayable_counterexample_without_losing_coverage() {
    let mut world = counter_run_world("Low", "self.value <= 3");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let result = run_model_based_state_invariant(
        &system,
        "Low",
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the failing invariant run and native shrink");

    assert_eq!(result.counts.failing, 1);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Refuted);
    let counterexample = result.counterexample.expect("a replayed failure");
    assert_eq!(counterexample.trace.steps.len(), 4);
    assert_eq!(
        result.operations[1].status,
        fsl_eval::simulation::model::ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Unreachable,
        }
    );
}

#[trace("TC-2956", "FR-752-AC-4")]
#[test]
fn missing_clause_is_rejected_instead_of_reported_tested() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let result = run_model_based_state_invariant(
        &system,
        "Missing",
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    );

    assert!(matches!(
        result,
        Err(ModelBasedTestError::InvalidClause { clause }) if clause == "Missing"
    ));
}

/// FR-752 evaluates temporal claims through the checked clause's profile and
/// the native model letters at each position.
#[trace("TC-2102", "FR-752-AC-1")]
#[test]
fn temporal_clause_uses_its_checked_profile_and_model_letters() {
    let mut world = counter_temporal_world("always[0,5] holds(c.value <= 5)");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let node = world
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "ModelProperty")
        .expect("the checked temporal clause");
    let result = run_model_based_test(
        &system,
        &node.identity(),
        &sampler(),
        42,
        1,
        5,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the temporal evaluator reads the sampled model trace");

    assert_eq!(result.clause, "ModelProperty");
    assert_eq!(result.counts.passing, 1);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert!(result.counterexample.is_none());
}

/// FR-752-AC-1: temporal capture observations use the model state at the
/// clause's activation position.
#[trace("TC-2102", "FR-752-AC-1")]
#[test]
fn temporal_clause_reads_captures_from_the_activation_state() {
    let mut world = counter_temporal_world(
        "capture snapshot: Int[0,5] = c.value; always[0,0] holds(c.value = snapshot)",
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let node = world
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "ModelProperty")
        .expect("the checked temporal clause");
    let result = run_model_based_test(
        &system,
        &node.identity(),
        &sampler(),
        42,
        1,
        0,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the temporal evaluator binds the activation capture");

    assert_eq!(result.counts.passing, 1);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
}

#[trace("TC-2102", "FR-752-AC-2")]
#[test]
fn temporal_violation_shrinks_and_retains_the_same_clause_failure() {
    let mut world = counter_temporal_world("always[0,5] holds(c.value < 5)");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let node = world
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "ModelProperty")
        .expect("the checked temporal clause");
    let result = run_model_based_test(
        &system,
        &node.identity(),
        &sampler(),
        42,
        1,
        5,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the temporal evaluator finds and shrinks the violation");

    assert_eq!(result.counts.failing, 1);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Refuted);
    assert!(result
        .counterexample
        .expect("a native-replayed temporal failure")
        .trace
        .steps
        .is_empty());
}
