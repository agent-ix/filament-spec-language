// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-323 "Replay result rows" (FSpec TC-2010): every FR-072
//! replay-result arm writes its `results` row, so a replay outcome's
//! `replayed` result (FR-286) and its text (FR-865) carry every arm.

use filament_exact::ScalarLimits;
use fsl_eval::temporal::{TemporalOutcome, TemporalPosition};
use fsl_replay::protocol_analysis::{
    ProtocolReplayResult, ProtocolReplaySettlement, WalkDisagreement,
};
use fsl_replay::{
    GraphReplayResult, GraphReplaySettlement, ModelCheckLimit, ObservedTemporalKind,
    ObservedTemporalResult, ReachedLimit, ReplayResult,
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

/// FSpec FR-323-AC-40: an observed-temporal replay that reproduces its
/// `violated` packet at position 2 writes the `observed-temporal` row with
/// its position and FR-352 outcome; against a packet claiming position 1 it
/// is `inconclusive` with `parity`.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-40")]
#[test]
#[ignore = "QSL-2292: the observed-temporal results row not implemented yet"]
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
    assert_eq!(written["arm"], "observed-temporal", "{written}");
    assert_eq!(
        written["settlement"], "reproduced-with-evaluated-witness",
        "{written}"
    );
    assert_eq!(written["category"], "violation", "{written}");
    assert_eq!(written["position"], "2", "{written}");
    assert_eq!(written["outcome"]["kind"], "violated", "{written}");
    assert_eq!(written["outcome"]["position"], 2, "{written}");
    assert!(written["outcome"].get("counterexample").is_none(), "{written}");
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
    assert_eq!(written["settlement"], "inconclusive", "{written}");
    assert_eq!(
        written["parity"]["expected"],
        json!({"kind": "formula"}),
        "{written}"
    );
    assert_eq!(written["parity"]["expected_position"], "1", "{written}");
}

/// FSpec FR-323-AC-41: a reproduced state-graph trap replay writes the
/// `model-graph` row with its predicates and closure; a closure stopped at
/// `max_states` 1 is `incomplete` with its `limit`.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-41")]
#[test]
#[ignore = "QSL-2292: the model-graph results row not implemented yet"]
fn a_state_graph_replay_writes_its_row() {
    let reproduced = ReplayResult::ModelGraph(GraphReplayResult {
        settlement: GraphReplaySettlement::Reproduced,
        evaluated_predicates: vec![(fsl_semantics::check::PredicateRole::From, true)],
        closure_nodes: Some(4),
        first_target: None,
    });
    let written = row(&reproduced);
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
    });
    let written = row(&stopped);
    assert_eq!(written["settlement"], "incomplete", "{written}");
    assert_eq!(written["category"], "incomplete", "{written}");
    assert_eq!(written["limit"]["limit"], "max_states", "{written}");
    assert_eq!(written["limit"]["value"], "1", "{written}");
}

/// FSpec FR-323-AC-42: a reproduced protocol-analysis replay writes the
/// three-member `protocol-analysis` row; a walk that leaves its case is
/// `inconclusive` with its walk disagreement.
#[trace("FSpec-TC-2010", "FSpec-FR-323-AC-42")]
#[test]
#[ignore = "QSL-2292: the protocol-analysis results row not implemented yet"]
fn a_protocol_analysis_replay_writes_its_row() {
    let reproduced = ReplayResult::ProtocolAnalysis(ProtocolReplayResult {
        settlement: ProtocolReplaySettlement::Reproduced,
    });
    assert_eq!(
        row(&reproduced),
        json!({"arm": "protocol-analysis",
            "settlement": "reproduced-with-evaluated-witness", "category": "success"})
    );

    let left = ReplayResult::ProtocolAnalysis(ProtocolReplayResult {
        settlement: ProtocolReplaySettlement::Inconclusive(WalkDisagreement::WrongBranch),
    });
    let written = row(&left);
    assert_eq!(written["settlement"], "inconclusive", "{written}");
    assert_eq!(written["category"], "inconclusive", "{written}");
    assert_eq!(
        written["disagreement"],
        json!({"kind": "walk", "reason": "wrong-branch"}),
        "{written}"
    );
}
