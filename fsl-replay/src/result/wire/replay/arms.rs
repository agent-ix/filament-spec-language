// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-323 "Replay result rows" for the replay arms beyond a function
//! replay: the observed-temporal, model-graph and protocol-analysis rows.

use base64::Engine;
use fsl_eval::temporal::TemporalOutcome;
use fsl_package::protocol_view::{Port, PortRef};
use fsl_semantics::check::PredicateRole;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::Value;

use super::{limit_wire, Charges, LimitWire};
use crate::model_check::graph_settle::{GraphReplayResult, GraphReplaySettlement};
use crate::protocol_analysis::{ProtocolReplayResult, ProtocolReplaySettlement, WalkDisagreement};
use crate::result::wire::{CauseCodecError, CauseWire};
use crate::spine::{replay_locus_wire, replay_outcome_wire};
use crate::ReachedLimit;
use crate::{
    HybridReplayResult, HybridReplaySettlement, ObservedTemporalKind, ObservedTemporalResult,
    ObservedTemporalSettlement, ReplayResult,
};

const REPRODUCED: &str = "reproduced-with-evaluated-witness";
const INCONCLUSIVE: &str = "inconclusive";

#[derive(Serialize)]
struct HybridWire {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    replay: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    counterexample: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disagreement: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<HybridLimitWire>,
}

#[derive(Serialize)]
struct HybridLimitWire {
    limit: &'static str,
    value: String,
    setting: &'static str,
}

fn static_wire(outcome: &crate::StaticReplay) -> Value {
    match outcome {
        crate::StaticReplay::Consistent => serde_json::json!({"outcome": "consistent"}),
        crate::StaticReplay::Inconsistent { at, check } => serde_json::json!({
            "outcome": "inconsistent",
            "at": at.to_string(),
            "check": check.number(),
        }),
    }
}

fn exact_wire(outcome: &crate::ExactRecord) -> Value {
    match outcome {
        crate::ExactRecord::NotExact => serde_json::json!({"outcome": "not-exact"}),
        crate::ExactRecord::NotReproduced { at, check } => serde_json::json!({
            "outcome": "not-reproduced",
            "at": at.to_string(),
            "check": check.number(),
        }),
    }
}

/// The FR-323 `hybrid` result row.
pub(super) fn hybrid(result: &HybridReplayResult) -> Result<String, CauseCodecError> {
    let (settlement, replay, counterexample, disagreement, limit) = match &result.settlement {
        HybridReplaySettlement::Reproduced {
            replay,
            counterexample: _,
        } => (
            "reproduced-with-evaluated-witness",
            Some(serde_json::json!({
                "static": static_wire(&replay.static_outcome),
                "exact": {"outcome": "reproduced"},
            })),
            result.counterexample_wire.clone(),
            None,
            None,
        ),
        HybridReplaySettlement::ReproducedWithValidatedWitness { replay } => (
            "reproduced-with-validated-witness",
            Some(serde_json::json!({
                "static": static_wire(&replay.static_outcome),
                "exact": {"outcome": "not-exact"},
            })),
            result.counterexample_wire.clone(),
            None,
            None,
        ),
        HybridReplaySettlement::Inconclusive {
            replay,
            disagreement,
        } => {
            let disagreement_type = match disagreement {
                crate::HybridTraceDisagreement::NotReproduced { .. } => "not-reproduced",
                crate::HybridTraceDisagreement::NotExact { .. } => "not-exact",
            };
            (
                INCONCLUSIVE,
                Some(serde_json::json!({
                    "static": static_wire(&replay.static_outcome),
                    "exact": replay.exact.as_ref().map(exact_wire),
                })),
                None,
                Some(serde_json::json!({
                    "cause": "hybrid-trace",
                    "disagreement": {
                        "type": disagreement_type,
                        "counterexample": result.counterexample_wire.clone().unwrap_or(Value::Null),
                    }
                })),
                None,
            )
        }
        HybridReplaySettlement::Incomplete { work_units } => (
            "incomplete",
            None,
            None,
            None,
            Some(HybridLimitWire {
                limit: "work_units",
                value: work_units.to_string(),
                setting: "accounting.work_units",
            }),
        ),
    };
    let row = HybridWire {
        arm: "hybrid",
        settlement,
        category: match &result.settlement {
            HybridReplaySettlement::Reproduced { .. }
            | HybridReplaySettlement::ReproducedWithValidatedWitness { .. } => "success",
            HybridReplaySettlement::Inconclusive { .. } => "inconclusive",
            HybridReplaySettlement::Incomplete { .. } => "incomplete",
        },
        replay,
        counterexample,
        disagreement,
        limit,
    };
    Ok(serde_json::to_string(&row)?)
}

#[derive(Serialize)]
struct ObservedWire {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    position: String,
    outcome: Value,
    undefined_locus: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parity: Option<ParityWire>,
    charges: Charges,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<LimitWire>,
}

#[derive(Serialize)]
struct ParityWire {
    expected: ExpectedWire,
    expected_position: String,
    actual_locus: Option<usize>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ExpectedWire {
    Formula,
    UndefinedEvaluation { locus: Value, cause: &'static str },
}

fn expected_wire(expected: &ObservedTemporalKind) -> Result<ExpectedWire, CauseCodecError> {
    Ok(match expected {
        ObservedTemporalKind::Formula => ExpectedWire::Formula,
        ObservedTemporalKind::UndefinedEvaluation { locus, cause } => {
            ExpectedWire::UndefinedEvaluation {
                locus: replay_locus_wire(locus)?,
                cause: crate::undefined_reason(cause),
            }
        }
    })
}

/// The `observed-temporal` row of `result`.
pub(super) fn observed(
    row: &ReplayResult,
    result: &ObservedTemporalResult,
) -> Result<String, CauseCodecError> {
    let outcome = replay_outcome_wire(result.outcome())?;
    let category = if matches!(result.outcome(), TemporalOutcome::Pending) {
        "pending"
    } else {
        row.category().as_str()
    };
    let parity = result
        .parity()
        .map(|parity| {
            Ok::<_, CauseCodecError>(ParityWire {
                expected: expected_wire(parity.expected())?,
                expected_position: parity.expected_position().index().to_string(),
                actual_locus: parity.actual_locus().map(|locus| locus.index()),
            })
        })
        .transpose()?;
    let wire = ObservedWire {
        arm: "observed-temporal",
        settlement: match result.settlement() {
            ObservedTemporalSettlement::ReproducedWithEvaluatedWitness => REPRODUCED,
            ObservedTemporalSettlement::Inconclusive => INCONCLUSIVE,
        },
        category,
        position: result.position().index().to_string(),
        outcome,
        undefined_locus: result.undefined_locus().map(|locus| locus.index()),
        parity,
        charges: Charges::from(result.charges()),
        limit: result.limit().map(limit_wire),
    };
    Ok(serde_json::to_string(&wire)?)
}

#[derive(Serialize)]
struct GraphWire {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    evaluated_predicates: Vec<PredicateWire>,
    closure_nodes: Option<String>,
    first_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disagreement: Option<CauseWire<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<ReachedWire>,
}

#[derive(Serialize)]
struct PredicateWire {
    role: &'static str,
    value: bool,
}

#[derive(Serialize)]
struct ReachedWire {
    limit: &'static str,
    value: String,
    setting: &'static str,
}

impl From<&ReachedLimit> for ReachedWire {
    fn from(reached: &ReachedLimit) -> Self {
        Self {
            limit: reached.limit.as_str(),
            value: reached.value.to_string(),
            setting: reached.limit.setting(),
        }
    }
}

/// The `model-graph` row of `result`.
pub(super) fn model_graph(
    row: &ReplayResult,
    result: &GraphReplayResult,
) -> Result<String, CauseCodecError> {
    let wire = GraphWire {
        arm: "model-graph",
        settlement: match &result.settlement {
            GraphReplaySettlement::Reproduced => REPRODUCED,
            GraphReplaySettlement::Inconclusive(_) => INCONCLUSIVE,
            GraphReplaySettlement::Stopped(_) => "incomplete",
        },
        category: row.category().as_str(),
        evaluated_predicates: result
            .evaluated_predicates
            .iter()
            .map(|(role, value)| PredicateWire {
                role: match role {
                    PredicateRole::Target if result.unique_path => "to",
                    PredicateRole::Target => "target",
                    PredicateRole::From => "from",
                },
                value: *value,
            })
            .collect(),
        closure_nodes: result.closure_nodes.map(|nodes| nodes.to_string()),
        first_target: result
            .first_target
            .as_ref()
            .map(|key| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes())),
        disagreement: match &result.settlement {
            GraphReplaySettlement::Inconclusive(cause) => Some(CauseWire::of(cause, None)?),
            GraphReplaySettlement::Reproduced | GraphReplaySettlement::Stopped(_) => None,
        },
        limit: match &result.settlement {
            GraphReplaySettlement::Stopped(reached) => Some(ReachedWire::from(reached)),
            GraphReplaySettlement::Reproduced | GraphReplaySettlement::Inconclusive(_) => None,
        },
    };
    Ok(serde_json::to_string(&wire)?)
}

#[derive(Serialize)]
struct ProtocolWire<'a> {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    disagreement: Option<WalkWire<'a>>,
}

#[derive(Serialize)]
struct WalkWire<'a> {
    kind: &'static str,
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<PortWire<'a>>,
}

#[derive(Serialize)]
struct PortWire<'a> {
    node: &'a filament_contract_model::CheckedNodeId,
    port: &'static str,
}

impl<'a> From<&'a PortRef> for PortWire<'a> {
    fn from(port: &'a PortRef) -> Self {
        Self {
            node: &port.node,
            port: match port.port {
                Port::Enter => "enter",
                Port::Exit => "exit",
            },
        }
    }
}

impl<'a> From<&'a WalkDisagreement> for WalkWire<'a> {
    fn from(disagreement: &'a WalkDisagreement) -> Self {
        let (reason, port) = match disagreement {
            WalkDisagreement::UnknownChoice => ("unknown-choice", None),
            WalkDisagreement::NotMustMerge => ("not-must-merge", None),
            WalkDisagreement::StepNotSuccessor(port) => {
                ("step-not-successor", Some(PortWire::from(port)))
            }
            WalkDisagreement::ActionMismatch => ("action-mismatch", None),
            WalkDisagreement::WrongBranch => ("wrong-branch", None),
            WalkDisagreement::NotTheStatedDivergence => ("not-the-stated-divergence", None),
            WalkDisagreement::Cut => ("cut", None),
        };
        Self {
            kind: "walk",
            reason,
            port,
        }
    }
}

/// The `protocol-analysis` row of `result`.
pub(super) fn protocol_analysis(
    row: &ReplayResult,
    result: &ProtocolReplayResult,
) -> Result<String, CauseCodecError> {
    let wire = ProtocolWire {
        arm: "protocol-analysis",
        settlement: match &result.settlement {
            ProtocolReplaySettlement::Reproduced => REPRODUCED,
            ProtocolReplaySettlement::Inconclusive(_) => INCONCLUSIVE,
        },
        category: row.category().as_str(),
        disagreement: match &result.settlement {
            ProtocolReplaySettlement::Reproduced => None,
            ProtocolReplaySettlement::Inconclusive(walk) => Some(WalkWire::from(walk)),
        },
    };
    Ok(serde_json::to_string(&wire)?)
}
