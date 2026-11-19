// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 model-based run record over a checked counter invariant.

use super::*;
use unicode_normalization::UnicodeNormalization;

use fsl_eval::simulation::model::{
    ModelBasedCounterexample, ModelBasedTestCause, ModelBasedTestDiscardCount,
    ModelBasedTestDiscardReason, ModelBasedTestError, ModelBasedTestEvidence,
    ModelOperationCoverageReason, ModelOperationCoverageReasonKind, ModelOperationCoverageStatus,
    ModelOperationCoverageSummaryStatus, run_model_based_state_invariant,
    run_model_based_state_invariant_with_value_bits, run_model_based_test,
    run_model_based_test_with_value_bits,
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
    assert_eq!(result.counts.generated, 100);
    assert_eq!(result.counts.failing, 0);
    assert_eq!(result.counts.discarded, 0);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested,);
    assert_eq!(result.profile, sampler());
    assert_eq!(result.provenance.len(), 100);
    let first_sample = result.provenance[0]
        .sample
        .as_ref()
        .expect("model cases retain sampled-trace provenance");
    assert_eq!(first_sample.seed, 7);
    assert_eq!(first_sample.trace, 0);
    assert_eq!(result.provenance[0].max_value_bits, None);
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
    let operations = result
        .operations
        .as_ref()
        .expect("state runs carry operation coverage");
    assert_eq!(operations.len(), 3);
    assert_eq!(
        operations
            .iter()
            .map(|entry| entry.operation.name.as_str())
            .collect::<Vec<_>>(),
        ["inc", "jam", "reset"]
    );
    assert_eq!(
        operations[0].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        operations[2].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        operations[1].status,
        fsl_eval::simulation::model::ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Unreachable,
        }
    );
    assert_eq!(
        result.coverage.as_ref().expect("coverage summary").status,
        ModelOperationCoverageSummaryStatus::Partial
    );
    assert_eq!(
        result
            .coverage
            .as_ref()
            .expect("coverage summary")
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
        result.operations.as_ref().unwrap()[2].status,
        fsl_eval::simulation::model::ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::UnreachableWithinBound {
                bounds: vec!["max_trace_length".to_owned()],
            },
        }
    );
}

/// FR-752-AC-6: size-bounded operation results stop at a value cut and keep
/// the corresponding operation unreachable only within that bound.
#[trace("TC-2956", "FR-752-AC-6")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-12")]
#[test]
fn unbounded_model_roots_report_value_bound_cuts_in_operation_coverage() {
    let make_world = || {
        let clauses = "pre CanAdd using v on M::Counter::add { true }\n\
                       post Add using v on M::Counter::add { self.value = pre(self.value) + amount }\n\
                       pre CanBig using v on M::Counter::big { self.value >= 300 }\n\
                       post Big using v on M::Counter::big { true }\n\
                       invariant Safe using v on M::Counter at current { true }";
        let mut amount_type = bound_type("Amount", 100);
        amount_type["constraints"][0]["operands"]["value"] = json!("1");
        let mut document: Json = serde_json::from_slice(&counters_document(
            vec![
                operation(
                    "add",
                    &[("amount", &ident("Amount"))],
                    None,
                    frame(&["value"], false, false),
                ),
                operation("big", &[], None, frame(&[], false, false)),
            ],
            false,
        ))
        .expect("fixture JSON");
        let counter = document["types"]
            .as_array_mut()
            .expect("types array")
            .iter_mut()
            .find(|ty| ty["identity"] == ident("Counter"))
            .expect("Counter declaration");
        counter["fields"] = json!([wire_field(
            "value",
            "ix://filament/native/Integer",
            "required"
        )]);
        document["types"]
            .as_array_mut()
            .expect("types array")
            .push(amount_type);
        build_world_document(
            serde_json::to_vec(&document).expect("fixture serializes"),
            clauses,
            false,
            false,
        )
        .expect("unbounded integer model and clauses check")
    };
    let run = |bits| {
        let mut world = make_world();
        let bytes = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "s0"},
            "observation": "current",
            "model": world.header.clone(),
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": ident("counters"), "complete": true,
                "objects": [{"key": "c1", "type": ident("Counter"),
                    "fields": {"value": {"integer": "0"}}}]}],
        })
        .to_string()
        .into_bytes();
        let reference = fsl_semantics::model::observation::document_reference(&bytes)
            .expect("a document names itself");
        world.snapshots.insert(reference, bytes);
        let initial = DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
        };
        let system = system(&world, vec![initial], &["c1"], UNLIMITED);
        run_model_based_state_invariant_with_value_bits(
            &system,
            "Safe",
            &sampler(),
            7,
            100,
            8,
            Some(bits),
            10_000,
            Limits::default()
                .with_max_states(1_000)
                .with_max_transitions(50_000),
        )
        .expect("the size-bounded native model run")
    };

    let narrow = run(8);
    assert!(narrow.provenance.value_bound_cuts.count >= 1);
    assert_eq!(narrow.provenance.max_value_bits, Some(8));
    let big = narrow
        .operations
        .as_ref()
        .expect("state clauses carry operation coverage")
        .iter()
        .find(|entry| entry.operation.name == "big")
        .expect("big is declared");
    assert_eq!(
        big.status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::UnreachableWithinBound {
                bounds: vec!["max_value_bits".to_owned()],
            },
        }
    );

    let wide = run(10);
    let big = wide
        .operations
        .as_ref()
        .expect("state clauses carry operation coverage")
        .iter()
        .find(|entry| entry.operation.name == "big")
        .expect("big is declared");
    assert!(
        matches!(big.status, ModelOperationCoverageStatus::Covered { .. }),
        "the seeded wide-bound suite must actually apply big"
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
    let ModelBasedCounterexample::Trace(counterexample) =
        result.counterexample.expect("a replayed failure")
    else {
        panic!("a state failure retains a trace counterexample");
    };
    assert_eq!(counterexample.trace.steps.len(), 4);
    assert_eq!(
        result.operations.as_ref().unwrap()[1].status,
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

/// FR-190-AC-9: zero case and trace limits refuse before sampling.
#[trace("TC-2102", "FR-752-AC-1")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-9")]
#[test]
fn zero_model_based_limits_are_rejected_before_sampling() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    for (max_cases, max_trace_length, member) in [(0, 8, "max_cases"), (1, 0, "max_trace_length")] {
        assert!(matches!(
            run_model_based_state_invariant(
                &system,
                "Safe",
                &sampler(),
                7,
                max_cases,
                max_trace_length,
                10_000,
                Limits::default()
                    .with_max_states(100)
                    .with_max_transitions(1_000),
            ),
            Err(ModelBasedTestError::InvalidLimit { member: found }) if found == member
        ));
    }
}

/// FR-752-AC-5: `max_value_bits` is constrained to 1..=64 and is unknown
/// when every generation root already has a finite declared domain.
#[trace("TC-2102", "FR-752-AC-5")]
#[test]
fn max_value_bits_rejects_invalid_ranges_and_finite_only_models() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    for bits in [0, 65] {
        assert!(matches!(
            run_model_based_state_invariant_with_value_bits(
                &system,
                "Safe",
                &sampler(),
                7,
                20,
                8,
                Some(bits),
                10_000,
                Limits::default()
                    .with_max_states(100)
                    .with_max_transitions(1_000),
            ),
            Err(ModelBasedTestError::InvalidLimit {
                member: "max_value_bits"
            })
        ));
    }
    assert!(matches!(
        run_model_based_state_invariant_with_value_bits(
            &system,
            "Safe",
            &sampler(),
            7,
            20,
            8,
            Some(8),
            10_000,
            Limits::default()
                .with_max_states(100)
                .with_max_transitions(1_000),
        ),
        Err(ModelBasedTestError::UnknownValueBits)
    ));
}

/// FR-752-AC-5 / FSpec FR-190-AC-8: an unbounded operation parameter requires
/// the requested size limit, which appears in every sampled-case provenance.
#[trace("TC-2102", "FR-752-AC-5")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-8")]
#[test]
fn an_unbounded_integer_parameter_uses_and_records_max_value_bits() {
    let mut world = world_with_level_bound(
        vec![operation(
            "observe",
            &[("sample", "ix://filament/native/Integer")],
            None,
            frame(&[], false, false),
        )],
        "invariant Safe using v on M::Counter at current { self.value <= 5 }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    assert!(matches!(
        run_model_based_state_invariant_with_value_bits(
            &system,
            "Safe",
            &sampler(),
            7,
            8,
            2,
            None,
            10_000,
            Limits::default()
                .with_max_states(100)
                .with_max_transitions(1_000),
        ),
        Err(ModelBasedTestError::MissingValueBits)
    ));

    let result = run_model_based_state_invariant_with_value_bits(
        &system,
        "Safe",
        &sampler(),
        7,
        8,
        2,
        Some(3),
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the bounded integer operation input");
    assert_eq!(result.max_value_bits, Some(3));
    assert_eq!(result.provenance.max_value_bits, Some(3));
    assert_eq!(result.provenance.size_bounded.len(), 1);
    assert_eq!(result.provenance.size_bounded[0].values, "15");
    assert_eq!(result.provenance.len(), 8);
    assert!(
        result
            .provenance
            .iter()
            .all(|case| case.max_value_bits == Some(3))
    );
    let bounded_domain = IntegerInterval::new((-7_i64).into(), 7_i64.into())
        .expect("three size bits bound the parameter to [-7, 7]");
    let partitions: Vec<_> = result
        .partitions
        .iter()
        .filter(|partition| partition.domain == bounded_domain)
        .map(|partition| (partition.kind, partition.value.to_string()))
        .collect();
    assert_eq!(
        partitions,
        [
            (ModelBoundaryKind::LowerRefusal, "-8".to_owned()),
            (ModelBoundaryKind::LowerEndpoint, "-7".to_owned()),
            (ModelBoundaryKind::UpperEndpoint, "7".to_owned()),
            (ModelBoundaryKind::UpperRefusal, "8".to_owned()),
        ]
    );
}

/// FR-752-AC-7: the deterministic domain-boundary prefix is ordered by
/// parameter declaration, before any seeded assignments, and consumes the
/// generated-case limit (including refusal discards).
#[trace("TC-2102", "FR-752-AC-7")]
#[test]
fn value_function_boundaries_precede_seeded_cases_in_declared_parameter_order() {
    let mut world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function Limit using v(x: Int[0,5], delta: Int[1,6]): Boolean pure { x + delta <= 11 }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .function_identity("Limit")
        .expect("checked value function");

    let result = run_model_based_test_with_value_bits(
        &system,
        &identity,
        &sampler(),
        7,
        8,
        8,
        None,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native value-function oracle and deterministic boundary prefix");

    let assignments: Vec<Vec<String>> = result
        .provenance
        .iter()
        .map(|case| {
            case.assignment
                .as_ref()
                .expect("value cases retain their typed assignments")
                .iter()
                .map(|value| match value {
                    filament_exact::Value::Integer(value) => value.to_string(),
                    other => panic!("expected integer boundary value, got {other:?}"),
                })
                .collect()
        })
        .collect();
    assert_eq!(
        assignments,
        [
            vec!["-1", "1"],
            vec!["0", "1"],
            vec!["5", "1"],
            vec!["6", "1"],
            vec!["0", "0"],
            vec!["0", "1"],
            vec!["0", "6"],
            vec!["0", "7"],
        ]
    );
    assert_eq!(result.counts.generated, 8);
    assert_eq!(result.counts.discarded, 4);
    assert_eq!(result.counts.passing, 4);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(result.operations, None);
    assert_eq!(result.coverage, None);
}

/// FSpec FR-190-AC-14 / FR-752-AC-5: an unbounded integer root uses the
/// request's size domain and emits its pinned size partitions before draws.
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-14")]
#[trace("TC-2102", "FR-752-AC-5")]
#[test]
fn unbounded_value_function_uses_size_order_partitions_before_seeded_draws() {
    let mut world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function Square using v(x: Integer): Boolean pure { x * x >= 0 }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .function_identity("Square")
        .expect("checked value function");

    let result = run_model_based_test_with_value_bits(
        &system,
        &identity,
        &sampler(),
        7,
        20,
        8,
        Some(8),
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native predicate evaluation over the size-bounded root");

    let values: Vec<String> = result
        .provenance
        .iter()
        .take(5)
        .map(|case| {
            let [filament_exact::Value::Integer(value)] = case
                .assignment
                .as_ref()
                .expect("value case assignment")
                .as_slice()
            else {
                panic!("expected one integer parameter")
            };
            value.to_string()
        })
        .collect();
    assert_eq!(values, ["0", "-1", "1", "-255", "255"]);
    assert_eq!(result.max_value_bits, Some(8));
    assert_eq!(result.provenance.max_value_bits, Some(8));
    assert_eq!(result.provenance.size_bounded.len(), 1);
    assert_eq!(result.provenance.size_bounded[0].values, "511");
    assert_eq!(result.provenance.value_bound_cuts.count, 0);
    assert_eq!(result.counts.generated, 20);
    assert_eq!(result.counts.passing, 20);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(result.checks.evaluated, 20);
    assert_eq!(result.discarded_by.len(), 4);
    assert_eq!(
        result.partition_coverage.status,
        ModelOperationCoverageSummaryStatus::Complete
    );
    let limited = run_model_based_test_with_value_bits(
        &system,
        &identity,
        &sampler(),
        7,
        1,
        8,
        Some(8),
        10_000,
        Limits::default(),
    )
    .expect("a case budget may end within the partition prefix");
    assert_eq!(limited.counts.generated, 1);
    assert_eq!(limited.counts.passing, 1);
    assert_eq!(limited.partitions.len(), 5);
    assert_eq!(limited.partition_coverage.uncovered["not-generated"], 4);
    assert!(result.partitions.iter().all(|partition| {
        !matches!(
            partition.kind,
            ModelBoundaryKind::LowerRefusal | ModelBoundaryKind::UpperRefusal
        )
    }));
}

/// FR-190-AC-21: Text normalization witnesses consume the deterministic
/// partition prefix before a sampler-indexed exact-domain draw.
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-21")]
#[test]
fn text_value_draw_uses_the_exact_unicode_scalar_domain_index() {
    let mut world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function TextValue using v(t: Text[0,2; unicode-scalars]): Boolean pure { true }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .function_identity("TextValue")
        .expect("checked Text value function");
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        11,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native Text value evaluation");

    let [filament_exact::Value::Text(value)] = result.provenance.cases[10]
        .assignment
        .as_ref()
        .expect("seeded Text assignment")
        .as_slice()
    else {
        panic!("expected a Text value");
    };
    assert_eq!(value.retained(), "\u{d51a9}\u{87ac9}");
    assert_eq!(result.counts.generated, 11);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
}

/// FR-190-AC-14: Text partitions retain the declared domain regardless of
/// the optional integer size bound.
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-14")]
#[test]
fn text_partitions_are_independent_of_max_value_bits() {
    let mut world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function TextValue using v(x: Integer, t: Text[0,4; unicode-scalars]): Boolean pure { true }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .function_identity("TextValue")
        .expect("checked value function");
    let run = |bits| {
        run_model_based_test_with_value_bits(
            &system,
            &identity,
            &sampler(),
            7,
            30,
            8,
            Some(bits),
            10_000,
            Limits::default()
                .with_max_states(100)
                .with_max_transitions(1_000),
        )
        .expect("native bounded value function")
    };
    let wide = run(8);
    let narrow = run(1);
    let text_partitions = |result: &fsl_eval::simulation::model::ModelBasedTestResult| {
        result
            .partitions
            .iter()
            .filter(|partition| partition.path == [1])
            .map(|partition| {
                (
                    partition.kind,
                    partition.value.to_string(),
                    partition.text_value.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let text = text_partitions(&wide);
    assert_eq!(text, text_partitions(&narrow));
    assert_eq!(text.len(), 12);
    assert_eq!(
        text.iter().map(|(kind, _, _)| *kind).collect::<Vec<_>>(),
        [
            ModelBoundaryKind::TextEmpty,
            ModelBoundaryKind::TextSingleton,
            ModelBoundaryKind::TextUpperEndpoint,
            ModelBoundaryKind::UpperRefusal,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
        ]
    );
    assert_eq!(wide.provenance.size_bounded.len(), 1);
    assert_eq!(wide.provenance.size_bounded[0].values, "511");
    assert_eq!(narrow.provenance.size_bounded[0].values, "3");
    assert_eq!(wide.counts.generated, 30);
    assert_eq!(wide.counts.passing, 29);
    assert_eq!(wide.counts.discarded, 1);
    assert_eq!(wide.evidence, ModelBasedTestEvidence::Tested);
}

/// FR-752-AC-9 / FR-190-AC-19: Text boundaries precede normalized seeded
/// draws, while the unicode-scalars profile keeps all witness spellings.
#[trace("TC-2102", "FR-752-AC-9")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-19")]
#[test]
fn text_value_cases_keep_normalization_partitions_and_seeded_members() {
    let mut nfc_world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function Label using v(t: Text[0,4; nfc]): Boolean pure { size(t) <= 4 }",
        5,
    );
    let initial = nfc_world.start("s0", &[Obj("c1", 0, 0, None)]);
    let nfc_system = system(&nfc_world, vec![initial], &["c1"], UNLIMITED);
    let identity = nfc_world
        .package
        .graph()
        .function_identity("Label")
        .expect("checked NFC value function");
    let nfc_result = run_model_based_test(
        &nfc_system,
        &identity,
        &sampler(),
        7,
        30,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native NFC Text value evaluation");

    assert_eq!(nfc_result.counts.generated, 30);
    assert_eq!(nfc_result.counts.discarded, 1);
    assert_eq!(nfc_result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(
        nfc_result
            .partitions
            .iter()
            .map(|partition| partition.kind)
            .collect::<Vec<_>>(),
        [
            ModelBoundaryKind::TextEmpty,
            ModelBoundaryKind::TextSingleton,
            ModelBoundaryKind::TextUpperEndpoint,
            ModelBoundaryKind::UpperRefusal,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
            ModelBoundaryKind::TextNormalization,
        ]
    );
    for case in &nfc_result.provenance.cases[9..] {
        let [Value::Text(text)] = case
            .assignment
            .as_ref()
            .expect("a seeded Text case has its assignment")
            .as_slice()
        else {
            panic!("expected a Text assignment");
        };
        assert!(text.retained().chars().eq(text.retained().chars().nfc()));
        assert!(text.retained().chars().count() <= 4);
    }

    let mut scalar_world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function ScalarLabel using v(t: Text[0,4; unicode-scalars]): Boolean pure { true }",
        5,
    );
    let initial = scalar_world.start("s0", &[Obj("c1", 0, 0, None)]);
    let scalar_system = system(&scalar_world, vec![initial], &["c1"], UNLIMITED);
    let identity = scalar_world
        .package
        .graph()
        .function_identity("ScalarLabel")
        .expect("checked scalar Text value function");
    let scalar_result = run_model_based_test(
        &scalar_system,
        &identity,
        &sampler(),
        7,
        12,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native unicode-scalars Text value evaluation");
    let witnesses: Vec<String> = scalar_result.provenance.cases[4..12]
        .iter()
        .map(|case| {
            let [Value::Text(text)] = case
                .assignment
                .as_ref()
                .expect("a witness has its assignment")
                .as_slice()
            else {
                panic!("expected a Text assignment");
            };
            text.retained().to_owned()
        })
        .collect();
    assert_eq!(
        witnesses,
        [
            "\u{e9}",
            "e\u{301}",
            "d\u{307}\u{323}",
            "d\u{323}\u{307}",
            "\u{ac00}",
            "\u{1100}\u{1161}",
            "\u{212b}",
            "\u{fb01}",
        ]
    );
}

/// FR-752-AC-5: size-domain shrinking walks in size order and native
/// re-evaluation retains the least failing value.
#[trace("TC-2102", "FR-752-AC-5")]
#[test]
fn unbounded_value_failure_shrinks_to_the_least_failing_integer() {
    let mut world = world_with_level_bound(
        vec![operation("observe", &[], None, frame(&[], false, false))],
        "function Below using v(x: Integer): Boolean pure { x < 200 }",
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .function_identity("Below")
        .expect("checked value function");

    let result = run_model_based_test_with_value_bits(
        &system,
        &identity,
        &sampler(),
        7,
        20,
        8,
        Some(8),
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native value evaluation and size-ordered shrinking");

    assert_eq!(result.evidence, ModelBasedTestEvidence::Refuted);
    let ModelBasedCounterexample::ValueAssignment(counterexample) = result
        .counterexample
        .expect("native-replayed value counterexample")
    else {
        panic!("a value clause retains its parameter assignment");
    };
    let [filament_exact::Value::Integer(value)] = counterexample.as_slice() else {
        panic!("expected one integer parameter");
    };
    assert_eq!(value.to_string(), "200");
}

/// FR-752-AC-7: native contract dispatch evaluates a selected precondition
/// independently, and a false precondition remains a passing check point.
#[trace("TC-2102", "FR-752-AC-7")]
#[test]
fn a_model_based_precondition_treats_false_as_a_passing_point() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .state_clause("CanReset")
        .expect("checked precondition")
        .identity();
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        40,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native precondition oracle");

    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(result.counts.generated, 40);
    assert!(result.counts.passing > 0);
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::PreconditionFalse].count,
        0
    );
}

/// FR-752-AC-6: a completed run with no evaluated check point is inconclusive
/// and retains every generated discard in its result and exit category.
#[trace("TC-2102", "FR-752-AC-6")]
#[test]
fn a_run_with_only_no_check_point_discards_is_inconclusive_exit_22() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .state_clause("Jam")
        .expect("checked postcondition")
        .identity();
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        20,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the native precondition oracle");

    assert_eq!(result.counts.generated, 20);
    assert_eq!(result.counts.passing, 0);
    assert_eq!(result.counts.failing, 0);
    assert_eq!(result.counts.discarded, 20);
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::DomainRefusal].count,
        2
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::NoCheckPoint].count,
        18
    );
    assert_eq!(result.evidence, ModelBasedTestEvidence::Inconclusive);
    assert_eq!(result.cause, Some(ModelBasedTestCause::NoEvaluatedCase));
    assert_eq!(result.exit_code(), 22);
}

/// FR-752-AC-7: the selected postcondition does not constrain its own
/// generated operation step, then a false native result is retained.
#[trace("TC-2102", "FR-752-AC-7")]
#[test]
fn a_model_based_postcondition_is_checked_on_its_applied_step() {
    let mut world = counter_run_world("Safe", "self.value <= 5");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .state_clause("Reset")
        .expect("checked postcondition")
        .identity();
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        40,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("native postcondition oracle");

    assert_eq!(result.evidence, ModelBasedTestEvidence::Refuted);
    assert_eq!(result.counts.generated, 5);
    assert_eq!(result.counts.failing, 1);
    assert!(result.counterexample.is_some());
}

/// FR-752's deterministic contract prefix emits field partitions before
/// parameter partitions, counts refusals, and preserves disabled arguments.
#[trace("FSpec-TC-2102", "FR-752-AC-7")]
#[test]
fn contract_cases_start_with_ordered_domain_partitions() {
    let clauses = "pre CanInc using v on M::Counter::inc { self.value + delta <= 5 }\n\
                   post Inc using v on M::Counter::inc { self.value <= 5 }";
    let op = operation(
        "inc",
        &[("delta", &ident("By"))],
        None,
        frame(&["value"], false, false),
    );
    let mut document: Json =
        serde_json::from_slice(&counters_document(vec![op], false)).expect("fixture JSON");
    let level = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == ident("Level"))
        .unwrap();
    *level = bound_type("Level", 5);
    let mut by_type = bound_type("By", 6);
    by_type["constraints"][0]["operands"]["value"] = json!("1");
    document["types"].as_array_mut().unwrap().push(by_type);
    let mut world = build_world_document(
        serde_json::to_vec(&document).expect("fixture serializes"),
        clauses,
        false,
        false,
    )
    .expect("the model and contract check");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .state_clause("Inc")
        .expect("checked postcondition")
        .identity();
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the native postcondition partition run");

    let repeated = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the same native run");
    assert_eq!(result.counts, repeated.counts);
    assert_eq!(result.discarded_by, repeated.discarded_by);
    assert_eq!(result.checks, repeated.checks);
    assert_eq!(result.partitions, repeated.partitions);
    assert_eq!(result.partition_coverage, repeated.partition_coverage);
    assert!(result.checks.evaluated >= result.counts.passing);
    assert!(result.checks.skipped.values().all(|count| *count == 0));
    assert_eq!(result.partition_coverage.uncovered["precondition-false"], 1);
    assert!(
        !result
            .partition_coverage
            .uncovered
            .contains_key("no-check-point")
    );

    let bounded = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(1)
            .with_max_transitions(1_000),
    )
    .expect("a coverage stop does not stop the suite");
    assert_eq!(bounded.result, ModelBasedTestEvidence::Tested);
    assert_eq!(
        bounded.counts.generated,
        bounded.counts.passing + bounded.counts.failing + bounded.counts.discarded
    );
    assert_eq!(bounded.counts.generated, 100);
    assert!(
        bounded
            .partition_coverage
            .uncovered
            .contains_key("undetermined")
    );

    let expected = [
        (vec![0, 2], "-1".to_owned(), ModelBoundaryKind::LowerRefusal),
        (vec![0, 2], "0".to_owned(), ModelBoundaryKind::LowerEndpoint),
        (vec![0, 2], "5".to_owned(), ModelBoundaryKind::UpperEndpoint),
        (vec![0, 2], "6".to_owned(), ModelBoundaryKind::UpperRefusal),
        (
            vec![1, 0, 0],
            "0".to_owned(),
            ModelBoundaryKind::LowerRefusal,
        ),
        (
            vec![1, 0, 0],
            "1".to_owned(),
            ModelBoundaryKind::LowerEndpoint,
        ),
        (
            vec![1, 0, 0],
            "6".to_owned(),
            ModelBoundaryKind::UpperEndpoint,
        ),
        (
            vec![1, 0, 0],
            "7".to_owned(),
            ModelBoundaryKind::UpperRefusal,
        ),
    ];
    let actual: Vec<_> = result
        .partitions
        .iter()
        .map(|partition| {
            (
                partition.path.clone(),
                partition.value.to_string(),
                partition.kind,
            )
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(result.counts.generated, 100);
    assert_eq!(result.counts.discarded, 6);
    assert_eq!(result.counts.passing, 94);
    assert_eq!(result.counts.failing, 0);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Tested);
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::DomainRefusal].count,
        4
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::PreconditionFalse].count,
        1
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::NoCheckPoint].count,
        1
    );
    assert_eq!(
        result.partitions[0].status,
        Some(fsl_eval::simulation::model::ModelPartitionStatus::Refused { first_case: 0 })
    );
    assert_eq!(
        result.partitions[6].status,
        Some(
            fsl_eval::simulation::model::ModelPartitionStatus::Uncovered {
                reason:
                    fsl_eval::simulation::model::ModelPartitionUncoveredReason::PreconditionFalse,
                bounds: Vec::new(),
            }
        )
    );
    assert_eq!(
        result.partitions[1].status,
        Some(fsl_eval::simulation::model::ModelPartitionStatus::Covered { first_case: 2 })
    );
    assert_eq!(
        result.partitions[5].status,
        Some(fsl_eval::simulation::model::ModelPartitionStatus::Covered { first_case: 2 })
    );
}

/// Text count-table work is charged before any caller-sized allocation.
#[trace("TC-3068", "FR-752-AC-10")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-22")]
#[test]
fn text_domain_work_limits_settle_a_record_before_any_case() {
    for (maximum, profile, work, expected) in [
        (
            1_099_511_627_776_u64,
            "nfc",
            1_000_000,
            ModelBasedTestEvidence::Incomplete,
        ),
        (
            1_099_511_627_776,
            "unicode-scalars",
            1_000_000,
            ModelBasedTestEvidence::Incomplete,
        ),
        (4, "nfc", 2_559, ModelBasedTestEvidence::Incomplete),
        (4, "nfc", 1_000_000, ModelBasedTestEvidence::Tested),
    ] {
        let mut world = world_with_level_bound(
            vec![operation("observe", &[], None, frame(&[], false, false))],
            &format!(
                "function Label using v(t: Text[0,{maximum}; {profile}]): Boolean pure {{ size(t) >= 0 }}"
            ),
            5,
        );
        let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
        let mut meter = UNLIMITED;
        meter.work_units = work;
        let system = system(&world, vec![initial], &["c1"], meter);
        let identity = world
            .package
            .graph()
            .function_identity("Label")
            .expect("value clause");
        let result = run_model_based_test(
            &system,
            &identity,
            &sampler(),
            7,
            30,
            8,
            10_000,
            Limits::default(),
        )
        .expect("resource exhaustion is a run record");
        assert_eq!(result.limits.accounting.work_units, work);
        assert_eq!(result.limits.positions, 10_000);
        assert_eq!(result.limits.exploration, Limits::default());
        assert_eq!(result.result, expected);
        assert!(result.provenance.size_bounded.is_empty());
        if expected == ModelBasedTestEvidence::Incomplete {
            assert_eq!(result.exit_code(), 22);
            assert_eq!(result.counts.generated, 0);
            assert_eq!(result.checks.evaluated, 0);
            assert_eq!(
                result.cause,
                Some(ModelBasedTestCause::LimitReached {
                    limit: filament_exact::LimitKind::WorkUnits,
                    value: work,
                    setting: "accounting.work_units".to_owned(),
                })
            );
        } else {
            assert_eq!(result.counts.generated, 30);
            assert_eq!(result.counts.discarded, 1);
            assert_eq!(result.checks.evaluated, 29);
            assert!(
                result
                    .discarded_by
                    .values()
                    .filter(|entry| entry.count == 0)
                    .all(|entry| entry.first_case.is_none())
            );
        }
    }
}

/// FR-752-AC-7 / FSpec FR-190-AC-16: an unrelated Jam postcondition records
/// the false inc precondition partition and treats every other missing check
/// point as a discard.
#[trace("TC-2102", "FR-752-AC-7")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-16")]
#[test]
fn jam_postcondition_discards_unrelated_inc_partition_cases() {
    let clauses = "pre CanInc using v on M::Counter::inc { self.value + delta <= 5 }\n\
                   post Inc using v on M::Counter::inc { self.value <= 5 }\n\
                   pre CanJam using v on M::Counter::jam { self.value > 5 }\n\
                   post Jam using v on M::Counter::jam { self.value = 0 }";
    let operations = vec![
        operation(
            "inc",
            &[("delta", &ident("By"))],
            None,
            frame(&["value"], false, false),
        ),
        operation("jam", &[], None, frame(&["value"], false, false)),
    ];
    let mut document: Json =
        serde_json::from_slice(&counters_document(operations, false)).expect("fixture JSON");
    let level = document["types"]
        .as_array_mut()
        .expect("types array")
        .iter_mut()
        .find(|ty| ty["identity"] == ident("Level"))
        .expect("Level declaration");
    *level = bound_type("Level", 5);
    let mut by_type = bound_type("By", 6);
    by_type["constraints"][0]["operands"]["value"] = json!("1");
    document["types"]
        .as_array_mut()
        .expect("types array")
        .push(by_type);
    let mut world = build_world_document(
        serde_json::to_vec(&document).expect("fixture serializes"),
        clauses,
        false,
        false,
    )
    .expect("the model and contracts check");
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1"], UNLIMITED);
    let identity = world
        .package
        .graph()
        .state_clause("Jam")
        .expect("checked Jam postcondition")
        .identity();
    let result = run_model_based_test(
        &system,
        &identity,
        &sampler(),
        7,
        100,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the native Jam postcondition run");

    assert_eq!(result.counts.generated, 100);
    assert_eq!(result.counts.passing, 0);
    assert_eq!(result.counts.failing, 0);
    assert_eq!(result.counts.discarded, 100);
    assert_eq!(result.evidence, ModelBasedTestEvidence::Inconclusive);
    assert_eq!(result.cause, Some(ModelBasedTestCause::NoEvaluatedCase));
    assert_eq!(result.exit_code(), 22);
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::DomainRefusal],
        ModelBasedTestDiscardCount {
            count: 4,
            first_case: Some(0),
        }
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::PreconditionFalse],
        ModelBasedTestDiscardCount {
            count: 1,
            first_case: Some(6),
        }
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::InvariantFalse],
        ModelBasedTestDiscardCount {
            count: 0,
            first_case: None,
        }
    );
    assert_eq!(
        result.discarded_by[&ModelBasedTestDiscardReason::NoCheckPoint],
        ModelBasedTestDiscardCount {
            count: 95,
            first_case: Some(1),
        }
    );

    let inc_precondition = world
        .package
        .graph()
        .state_clause("CanInc")
        .expect("checked inc precondition")
        .identity();
    let precondition_result = run_model_based_test(
        &system,
        &inc_precondition,
        &sampler(),
        7,
        8,
        8,
        10_000,
        Limits::default()
            .with_max_states(100)
            .with_max_transitions(1_000),
    )
    .expect("the false inc precondition is an evaluated point");
    assert_eq!(
        precondition_result.discarded_by[&ModelBasedTestDiscardReason::PreconditionFalse].count,
        0
    );
    assert_eq!(
        precondition_result.partitions[6].status,
        Some(fsl_eval::simulation::model::ModelPartitionStatus::Covered { first_case: 6 })
    );
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
        1,
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
    let ModelBasedCounterexample::Trace(counterexample) = result
        .counterexample
        .expect("a native-replayed temporal failure")
    else {
        panic!("a temporal failure retains a trace counterexample");
    };
    assert!(counterexample.trace.steps.is_empty());
}
