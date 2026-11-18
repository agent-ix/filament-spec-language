// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-323 "Replay result rows" for the replay arms beyond a function
//! replay: the observed-temporal, model-graph and protocol-analysis rows.

use base64::Engine;
use fsl_eval::temporal::{ObservedLocus, TemporalOutcome};
use fsl_package::protocol_view::{Port, PortRef};
use fsl_semantics::check::PredicateRole;
use quire_canonical::{to_vec, Limits};
use serde::Serialize;
use serde_json::value::RawValue;

use super::{limit_wire, Charges, LimitWire};
use crate::model_check::graph_settle::{GraphReplayResult, GraphReplaySettlement};
use crate::protocol_analysis::{ProtocolReplayResult, ProtocolReplaySettlement, WalkDisagreement};
use crate::result::wire::{CauseCodecError, CauseWire};
use crate::ReachedLimit;
use crate::{
    ObservedTemporalKind, ObservedTemporalResult, ObservedTemporalSettlement, ReplayResult,
};

const REPRODUCED: &str = "reproduced-with-evaluated-witness";
const INCONCLUSIVE: &str = "inconclusive";
const MAX_WINDOW_VALUES: &str = "max_window_values";

#[derive(Serialize)]
struct ObservedWire {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    position: String,
    outcome: OutcomeWire,
    undefined_locus: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parity: Option<ParityWire>,
    charges: Charges,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<LimitWire>,
}

/// FSpec FR-352's `disposition.temporal` row of an outcome's kind, without
/// the `counterexample` member a replay row never carries.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum OutcomeWire {
    Holds,
    Violated {
        position: u64,
    },
    Undefined {
        position: u64,
    },
    InsufficientData {
        position: u64,
        count: u64,
        min_count: u64,
    },
    Pending,
    MissingFairnessPremise {
        constraint: Box<RawValue>,
    },
    Incomplete,
    WindowLimitReached {
        limit: &'static str,
        value: u64,
        setting: &'static str,
    },
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
    UndefinedEvaluation {
        locus: LocusWire,
        cause: &'static str,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum LocusWire {
    FormulaNode(usize),
    Guard(&'static str),
    Capture(String),
}

fn outcome_wire(outcome: &TemporalOutcome) -> Result<OutcomeWire, CauseCodecError> {
    Ok(match outcome {
        TemporalOutcome::Holds => OutcomeWire::Holds,
        TemporalOutcome::Violated { at } => OutcomeWire::Violated {
            position: at.index(),
        },
        TemporalOutcome::Undefined(undefined) => OutcomeWire::Undefined {
            position: undefined.at.index(),
        },
        TemporalOutcome::InsufficientData(short) => OutcomeWire::InsufficientData {
            position: short.at.index(),
            count: short.data.count,
            min_count: short.data.min_count,
        },
        TemporalOutcome::Pending => OutcomeWire::Pending,
        TemporalOutcome::MissingFairnessPremise { constraint } => {
            let bytes = to_vec(constraint, Limits::new(u64::MAX))?;
            OutcomeWire::MissingFairnessPremise {
                constraint: serde_json::from_slice(&bytes)?,
            }
        }
        TemporalOutcome::Incomplete(_) => OutcomeWire::Incomplete,
        TemporalOutcome::WindowLimitReached(reached) => OutcomeWire::WindowLimitReached {
            limit: MAX_WINDOW_VALUES,
            value: reached.limit,
            setting: reached.setting,
        },
    })
}

fn expected_wire(expected: &ObservedTemporalKind) -> ExpectedWire {
    match expected {
        ObservedTemporalKind::Formula => ExpectedWire::Formula,
        ObservedTemporalKind::UndefinedEvaluation { locus, cause } => {
            ExpectedWire::UndefinedEvaluation {
                locus: match locus {
                    ObservedLocus::Atom(atom) => LocusWire::FormulaNode(atom.index()),
                    ObservedLocus::Activation => LocusWire::Guard("on-each"),
                    ObservedLocus::Capture(name) => LocusWire::Capture(name.clone()),
                },
                cause: crate::undefined_reason(cause),
            }
        }
    }
}

/// The `observed-temporal` row of `result`.
pub(super) fn observed(
    row: &ReplayResult,
    result: &ObservedTemporalResult,
) -> Result<String, CauseCodecError> {
    let wire = ObservedWire {
        arm: "observed-temporal",
        settlement: match result.settlement() {
            ObservedTemporalSettlement::ReproducedWithEvaluatedWitness => REPRODUCED,
            ObservedTemporalSettlement::Inconclusive => INCONCLUSIVE,
        },
        category: row.category().as_str(),
        position: result.position().index().to_string(),
        outcome: outcome_wire(result.outcome())?,
        undefined_locus: result.undefined_locus().map(|locus| locus.index()),
        parity: result.parity().map(|parity| ParityWire {
            expected: expected_wire(parity.expected()),
            expected_position: parity.expected_position().index().to_string(),
            actual_locus: parity.actual_locus().map(|locus| locus.index()),
        }),
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
                    PredicateRole::Target => "target",
                    PredicateRole::From => "from",
                    PredicateRole::To => "to",
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
