// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-323 "Replay result rows" (FSpec TC-2010): every FR-072
//! replay-result arm writes its `results` row, so a replay outcome's
//! `replayed` result (FR-286) and its text (FR-865) carry every arm.

use filament_contract_model::{CheckedNodeId, CheckedNodePathSegment};
use filament_exact::ScalarLimits;
use filament_semantic_value::catalog::Category;
use fsl_eval::simulation::StateKey;
use fsl_eval::temporal::{
    InsufficientData, InsufficientDataEvaluation, TemporalOutcome, TemporalPosition,
    WindowLimitReached,
};
use fsl_package::protocol_view::{Port, PortRef};
use fsl_replay::protocol_analysis::{
    ProtocolReplayResult, ProtocolReplaySettlement, WalkDisagreement,
};
use fsl_replay::{
    EvaluatedValue, GraphReplayResult, GraphReplaySettlement, InputArmResult, ModelCheckLimit,
    ObservedTemporalKind, ObservedTemporalResult, ReachedLimit, ReplayResult, Verdict,
    WitnessArmResult, WitnessCheck,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn charges() -> ScalarLimits {
    ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 0,
        work_units: 3,
        result_units: 0,
    }
}

/// The `results` row `result` writes.
fn row(result: &ReplayResult) -> Value {
    let text = result
        .to_function_json()
        .expect("every replay-result arm writes its results row");
    serde_json::from_str(&text).expect("the row is JSON")
}

fn assert_closed_row(value: &Value, expected: &[&str]) {
    let object = value.as_object().expect("a result row is an object");
    let mut actual: Vec<_> = object.keys().map(String::as_str).collect();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected, "row members: {value}");
}

fn charges_json() -> Value {
    json!({
        "integer_bits": "0",
        "decimal_digits": "0",
        "scale_expansion": "0",
        "text_input_bytes": "0",
        "text_scalars": "0",
        "normalized_scalars": "0",
        "unit_edges": "0",
        "value_occurrences": "0",
        "work_units": "3",
        "result_units": "0",
    })
}

/// FSpec FR-323-AC-40: an observed-temporal replay that reproduces its
/// `violated` packet at position 2 writes the `observed-temporal` row with
/// its position and FR-352 outcome; against a packet claiming position 1 it
/// is `inconclusive` with `parity`.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-40")]
#[test]
fn an_observed_temporal_replay_writes_its_row() {
    let violated = || TemporalOutcome::Violated {
        at: TemporalPosition::new(2),
    };
    let reproduced = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(2),
        violated(),
        None,
        charges(),
    ));
    let written = row(&reproduced);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "position",
            "outcome",
            "undefined_locus",
            "charges",
        ],
    );
    assert_eq!(written["arm"], "observed-temporal", "{written}");
    assert_eq!(
        written["settlement"], "reproduced-with-evaluated-witness",
        "{written}"
    );
    assert_eq!(written["category"], "violation", "{written}");
    assert_eq!(written["position"], "2", "{written}");
    assert_eq!(written["outcome"]["kind"], "violated", "{written}");
    assert_eq!(written["outcome"]["position"], 2, "{written}");
    assert!(
        written["outcome"].get("counterexample").is_none(),
        "{written}"
    );
    assert_eq!(written["undefined_locus"], Value::Null, "{written}");
    assert!(written.get("parity").is_none(), "{written}");
    assert_eq!(written["charges"]["work_units"], "3", "{written}");

    let differs = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(1),
        violated(),
        None,
        charges(),
    ));
    let written = row(&differs);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "position",
            "outcome",
            "undefined_locus",
            "parity",
            "charges",
        ],
    );
    assert_eq!(written["settlement"], "inconclusive", "{written}");
    assert_eq!(
        written["parity"]["expected"],
        json!({"kind": "formula"}),
        "{written}"
    );
    assert_eq!(written["parity"]["expected_position"], "1", "{written}");

    let pending = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(1),
        TemporalOutcome::Pending,
        None,
        charges(),
    ));
    let written = row(&pending);
    assert_eq!(written["category"], "pending", "{written}");
    assert_eq!(written["outcome"]["kind"], "pending", "{written}");

    let undefined = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::UndefinedEvaluation {
            locus: fsl_eval::temporal::ObservedLocus::Atom(
                fsl_semantics::check::FormulaNodeId::from_arena_index(4),
            ),
            cause: fsl_eval::temporal::UndefinedCause::Kernel(
                filament_exact::Undefined::DivisionByZero,
            ),
        },
        TemporalPosition::new(1),
        TemporalOutcome::Undefined(fsl_eval::temporal::UndefinedEvaluation {
            at: TemporalPosition::new(1),
            cause: fsl_eval::temporal::UndefinedCause::Kernel(
                filament_exact::Undefined::DivisionByZero,
            ),
        }),
        Some(fsl_semantics::check::FormulaNodeId::from_arena_index(4)),
        charges(),
    ));
    let written = row(&undefined);
    assert_eq!(
        written,
        json!({
            "arm": "observed-temporal",
            "settlement": "reproduced-with-evaluated-witness",
            "category": "violation",
            "position": "1",
            "outcome": {"kind": "undefined", "position": 1, "reason": "division-by-zero"},
            "undefined_locus": 4,
            "charges": charges_json(),
        }),
        "undefined judgment row"
    );

    let insufficient = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(3),
        TemporalOutcome::InsufficientData(InsufficientDataEvaluation {
            at: TemporalPosition::new(3),
            atom: fsl_semantics::check::FormulaNodeId::from_arena_index(5),
            data: InsufficientData {
                count: 2,
                min_count: 4,
            },
        }),
        None,
        charges(),
    ));
    assert_eq!(
        row(&insufficient),
        json!({
            "arm": "observed-temporal",
            "settlement": "inconclusive",
            "category": "undefined",
            "position": "3",
            "outcome": {"kind": "insufficient-data", "position": 3, "count": 2, "min_count": 4},
            "undefined_locus": null,
            "parity": {"expected": {"kind": "formula"}, "expected_position": "3", "actual_locus": null},
            "charges": charges_json(),
        }),
        "insufficient-data judgment row"
    );

    let pending = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(6),
        TemporalOutcome::Pending,
        None,
        charges(),
    ));
    assert_eq!(
        row(&pending),
        json!({
            "arm": "observed-temporal",
            "settlement": "inconclusive",
            "category": "pending",
            "position": "6",
            "outcome": {"kind": "pending"},
            "undefined_locus": null,
            "parity": {"expected": {"kind": "formula"}, "expected_position": "6", "actual_locus": null},
            "charges": charges_json(),
        }),
        "pending judgment row"
    );
}

#[test]
fn missing_fairness_and_window_limit_judgments_write_complete_rows() {
    use filament_semantic_value::fairness::{
        FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
    };

    let constraint = FairnessConstraint::new(
        FairnessKind::Weak,
        FairnessOperation::new(
            filament_exact::EffectiveId::new(
                None,
                filament_exact::OriginalDeclarationKey::new("fixture", "Type"),
            ),
            "send".to_owned(),
        ),
        FairnessGranularity::Each,
    );
    let missing_fairness = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(5),
        TemporalOutcome::MissingFairnessPremise {
            constraint: constraint.clone(),
        },
        None,
        charges(),
    ));
    assert_eq!(
        row(&missing_fairness),
        json!({
            "arm": "observed-temporal",
            "settlement": "inconclusive",
            "category": "unsupported",
            "position": "5",
            "outcome": {
                "kind": "missing-fairness-premise",
                "constraint": {
                    "granularity": "each",
                    "kind": "weak",
                    "operation": {
                        "declaring_type": {
                            "owner_effective_type": null,
                            "original": {"package": "fixture", "node": "Type"}
                        },
                        "name": "send"
                    }
                }
            },
            "undefined_locus": null,
            "parity": {"expected": {"kind": "formula"}, "expected_position": "5", "actual_locus": null},
            "charges": charges_json(),
        }),
        "missing-fairness judgment row"
    );

    let window_limit = ReplayResult::ObservedTemporal(ObservedTemporalResult::settle(
        ObservedTemporalKind::Formula,
        TemporalPosition::new(8),
        TemporalOutcome::WindowLimitReached(WindowLimitReached {
            limit: 16,
            value: 17,
            setting: "monitor.max_window_values",
            position: TemporalPosition::new(8),
        }),
        None,
        charges(),
    ));
    assert_eq!(
        row(&window_limit),
        json!({
            "arm": "observed-temporal",
            "settlement": "inconclusive",
            "category": "incomplete",
            "position": "8",
            "outcome": {
                "kind": "window-limit-reached",
                "limit": "max_window_values",
                "value": "16",
                "setting": "monitor.max_window_values"
            },
            "undefined_locus": null,
            "parity": {"expected": {"kind": "formula"}, "expected_position": "8", "actual_locus": null},
            "charges": charges_json(),
        }),
        "window-limit judgment row"
    );
}

/// FSpec FR-323-AC-41: a reproduced state-graph trap replay writes the
/// `model-graph` row with its predicates and closure; a closure stopped at
/// `max_states` 1 is `incomplete` with its `limit`.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-41")]
#[test]
fn a_state_graph_replay_writes_its_row() {
    let reproduced = ReplayResult::ModelGraph(GraphReplayResult {
        settlement: GraphReplaySettlement::Reproduced,
        evaluated_predicates: vec![(fsl_semantics::check::PredicateRole::From, true)],
        closure_nodes: Some(4),
        first_target: None,
        unique_path: false,
    });
    let written = row(&reproduced);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "evaluated_predicates",
            "closure_nodes",
            "first_target",
        ],
    );
    assert_eq!(written["arm"], "model-graph", "{written}");
    assert_eq!(
        written["settlement"], "reproduced-with-evaluated-witness",
        "{written}"
    );
    assert_eq!(written["category"], "success", "{written}");
    assert_eq!(
        written["evaluated_predicates"],
        json!([{"role": "from", "value": true}]),
        "{written}"
    );
    assert_eq!(written["closure_nodes"], "4", "{written}");

    let unique = ReplayResult::ModelGraph(GraphReplayResult {
        settlement: GraphReplaySettlement::Reproduced,
        evaluated_predicates: vec![
            (fsl_semantics::check::PredicateRole::From, true),
            (fsl_semantics::check::PredicateRole::Target, true),
        ],
        closure_nodes: None,
        first_target: None,
        unique_path: true,
    });
    let written = row(&unique);
    assert_eq!(
        written["evaluated_predicates"],
        json!([{"role": "from", "value": true}, {"role": "to", "value": true}]),
        "{written}"
    );
    assert_eq!(written["first_target"], Value::Null, "{written}");
    assert!(written.get("charges").is_none(), "{written}");

    let stopped = ReplayResult::ModelGraph(GraphReplayResult {
        settlement: GraphReplaySettlement::Stopped(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        }),
        evaluated_predicates: vec![],
        closure_nodes: None,
        first_target: None,
        unique_path: false,
    });
    let written = row(&stopped);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "evaluated_predicates",
            "closure_nodes",
            "first_target",
            "limit",
        ],
    );
    assert_eq!(written["settlement"], "incomplete", "{written}");
    assert_eq!(written["category"], "incomplete", "{written}");
    assert_eq!(written["limit"]["limit"], "max_states", "{written}");
    assert_eq!(written["limit"]["value"], "1", "{written}");

    let disagrees = ReplayResult::ModelGraph(GraphReplayResult {
        settlement: GraphReplaySettlement::Inconclusive(fsl_replay::DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Success),
            replayed: Verdict::from_category(Category::Violation),
        }),
        evaluated_predicates: vec![],
        closure_nodes: None,
        first_target: Some(StateKey::from_canonical_bytes(vec![1, 2, 3])),
        unique_path: false,
    });
    assert_eq!(
        row(&disagrees),
        json!({
            "arm": "model-graph",
            "settlement": "inconclusive",
            "category": "inconclusive",
            "evaluated_predicates": [],
            "closure_nodes": null,
            "first_target": "AQID",
            "disagreement": {
                "cause": "no-value",
                "proved": "success",
                "replayed": "violation"
            }
        }),
        "inconclusive model-graph judgment row"
    );
}

/// FSpec FR-323-AC-42: a reproduced protocol-analysis replay writes the
/// three-member `protocol-analysis` row; a walk that leaves its case is
/// `inconclusive` with its walk disagreement.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-42")]
#[test]
fn a_protocol_analysis_replay_writes_its_row() {
    let reproduced = ReplayResult::ProtocolAnalysis(ProtocolReplayResult {
        settlement: ProtocolReplaySettlement::Reproduced,
    });
    let written = row(&reproduced);
    assert_closed_row(&written, &["arm", "settlement", "category"]);
    assert_eq!(
        written,
        json!({"arm": "protocol-analysis",
            "settlement": "reproduced-with-evaluated-witness", "category": "success"})
    );

    let left = ReplayResult::ProtocolAnalysis(ProtocolReplayResult {
        settlement: ProtocolReplaySettlement::Inconclusive(WalkDisagreement::WrongBranch),
    });
    let written = row(&left);
    assert_closed_row(&written, &["arm", "settlement", "category", "disagreement"]);
    assert_eq!(written["settlement"], "inconclusive", "{written}");
    assert_eq!(written["category"], "inconclusive", "{written}");
    assert_eq!(
        written["disagreement"],
        json!({"kind": "walk", "reason": "wrong-branch"}),
        "{written}"
    );

    let step_not_successor = ReplayResult::ProtocolAnalysis(ProtocolReplayResult {
        settlement: ProtocolReplaySettlement::Inconclusive(WalkDisagreement::StepNotSuccessor(
            PortRef {
                node: CheckedNodeId {
                    package: "fixture".into(),
                    path: vec![CheckedNodePathSegment::Name("step".into())],
                    ordinal: 2,
                },
                port: Port::Exit,
            },
        )),
    });
    assert_eq!(
        row(&step_not_successor),
        json!({
            "arm": "protocol-analysis",
            "settlement": "inconclusive",
            "category": "inconclusive",
            "disagreement": {
                "kind": "walk",
                "reason": "step-not-successor",
                "port": {
                    "node": {
                        "package": "fixture",
                        "path": ["step"],
                        "ordinal": 2
                    },
                    "port": "exit"
                }
            }
        }),
        "step-not-successor protocol-analysis judgment row"
    );
}

/// FSpec FR-323-AC-39: the function replay rows use each arm's closed member
/// set; `input` carries no record value, and `witness` always carries it.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-39")]
#[test]
fn function_replay_rows_are_closed_per_arm() {
    let success = Verdict::from_category(Category::Success);
    let input = ReplayResult::Input(InputArmResult::settle(
        success,
        success,
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        charges(),
    ));
    let written = row(&input);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "value",
            "charges",
            "resolved_regions",
            "record",
        ],
    );
    assert_eq!(written["record"], Value::Null, "{written}");
    assert!(written.get("disagreement").is_none(), "{written}");

    let witness = ReplayResult::Witness(WitnessArmResult::settle(
        success,
        success,
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        WitnessCheck::Agrees(None),
        Vec::new(),
        charges(),
    ));
    let written = row(&witness);
    assert_closed_row(
        &written,
        &[
            "arm",
            "settlement",
            "category",
            "value",
            "charges",
            "resolved_regions",
            "record",
        ],
    );
    assert_eq!(written["record"], Value::Null, "{written}");
}
