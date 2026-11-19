// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 operation coverage from model-graph reach information and evaluated
//! model-based cases.

use fsl_eval::simulation::model::{
    ModelOperation, ModelOperationCase, ModelOperationCoverageReason, ModelOperationCoverageStatus,
    ModelOperationCoverageSummaryStatus, model_operation_coverage,
};
use ix_trace_rs::trace;

fn operations() -> Vec<ModelOperation> {
    ["inc", "reset", "jam"]
        .into_iter()
        .map(|name| ModelOperation {
            identity: format!("operation:{name}"),
            name: name.to_owned(),
        })
        .collect()
}

/// FR-752-AC-4 (FSpec FR-190-AC-11): operation coverage preserves declared
/// order and distinguishes covered cases from exhaustively unreachable ops.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-11")]
#[test]
fn operation_coverage_reports_unreachable_operations_in_declaration_order() {
    let result = model_operation_coverage(
        &operations(),
        &[ModelOperationCase {
            case_index: 0,
            operations: vec!["operation:inc".into(), "operation:reset".into()],
        }],
        &[(
            "operation:jam".into(),
            ModelOperationCoverageReason::Unreachable,
        )],
    )
    .expect("every uncovered declaration has graph reach information");

    assert_eq!(result.status, ModelOperationCoverageSummaryStatus::Partial);
    assert_eq!(
        result
            .operations
            .iter()
            .map(|entry| entry.operation.name.as_str())
            .collect::<Vec<_>>(),
        ["inc", "reset", "jam"]
    );
    assert_eq!(
        result.operations[0].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        result.operations[1].status,
        ModelOperationCoverageStatus::Covered { first_case: 0 }
    );
    assert_eq!(
        result.operations[2].status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Unreachable,
        }
    );
    assert_eq!(
        result
            .uncovered
            .get(&fsl_eval::simulation::model::ModelOperationCoverageReasonKind::Unreachable),
        Some(&1)
    );
    let wire = serde_json::to_value(&result).expect("coverage has a JSON result shape");
    assert_eq!(wire["operations"][2]["status"], "uncovered");
    assert_eq!(wire["operations"][2]["reason"], "unreachable");
    assert_eq!(wire["status"], "partial");
    assert!(wire.get("verdict").is_none());
}

/// FR-752-AC-4 (FSpec FR-190-AC-12): a horizon-limited absence is not
/// misreported as exhaustive unreachability.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-12")]
#[test]
fn operation_coverage_preserves_unreachable_within_bound_reason() {
    let result = model_operation_coverage(
        &operations(),
        &[ModelOperationCase {
            case_index: 0,
            operations: vec!["operation:inc".into()],
        }],
        &[
            (
                "operation:reset".into(),
                ModelOperationCoverageReason::UnreachableWithinBound {
                    bounds: vec!["max_trace_length".to_owned()],
                },
            ),
            (
                "operation:jam".into(),
                ModelOperationCoverageReason::Unreachable,
            ),
        ],
    )
    .expect("uncovered declarations have graph reach information");

    assert_eq!(
        result.operations[1].status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::UnreachableWithinBound {
                bounds: vec!["max_trace_length".to_owned()],
            },
        }
    );
    let wire = serde_json::to_value(&result).expect("coverage has a JSON result shape");
    assert_eq!(
        wire["operations"][1]["bounds"],
        serde_json::json!(["max_trace_length"])
    );
    assert_eq!(
        result.operations[2].status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Unreachable,
        }
    );
}

/// FR-752-AC-4 (FSpec FR-190-AC-13): coverage is deterministic, does not
/// decide the model run's verdict, and preserves undetermined reach limits.
#[trace("TC-2956", "FR-752-AC-4")]
#[trace("FSpec-TC-2004", "FSpec-FR-190-AC-13")]
#[test]
fn operation_coverage_is_deterministic_and_keeps_verdict_out_of_scope() {
    let declared = operations();
    let cases = [ModelOperationCase {
        case_index: 3,
        operations: vec!["operation:inc".into()],
    }];
    let reachability = [
        (
            "operation:reset".into(),
            ModelOperationCoverageReason::NotGenerated {
                enabled_at: vec!["operation:inc".into(); 5],
            },
        ),
        (
            "operation:jam".into(),
            ModelOperationCoverageReason::Undetermined {
                limit: fsl_eval::simulation::model::ModelOperationCoverageLimit::States,
                value: 20,
            },
        ),
    ];
    let first = model_operation_coverage(&declared, &cases, &reachability)
        .expect("all uncovered declarations have graph reach information");
    let second = model_operation_coverage(&declared, &cases, &reachability)
        .expect("the same inputs produce a result");

    assert_eq!(first, second);
    assert_eq!(
        first.operations[1].status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::NotGenerated {
                enabled_at: vec!["operation:inc".into(); 5],
            },
        }
    );
    assert_eq!(
        first.operations[2].status,
        ModelOperationCoverageStatus::NotCovered {
            reason: ModelOperationCoverageReason::Undetermined {
                limit: fsl_eval::simulation::model::ModelOperationCoverageLimit::States,
                value: 20,
            },
        }
    );
    assert_eq!(first.limits.undetermined, 1);
}
