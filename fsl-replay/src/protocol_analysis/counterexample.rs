// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-331 and FR-286's complete protocol-analysis counterexample body.

use filament_contract_model::CheckedNodeId;
use fsl_eval::simulation::protocol::{send_record_json, transition_json, StepKind};
use fsl_package::protocol_view::{Port, PortRef};
use serde::Serialize;
use serde_json::Value;

use super::{
    Action, DecisionKind, DecisionOwner, Direction, Play, ProjectionCounterexample, ProjectionDecision,
    ProtocolAnalysisKey, RealizabilityCounterexample, StrategyAction,
};
use crate::result::{ProtocolWitnessWire, SeparatingWitnessRecord};
use crate::CounterexampleWireError;

mod key;
use key::KeyWire;

/// The native decisive evidence of a protocol-analysis refutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolAnalysisCounterexample {
    /// The realizability arena's refuting evidence.
    Realizability(Box<RealizabilityCounterexample>),
    /// A projection's refuting decision.
    Projection(Box<ProjectionCounterexample>),
}

impl ProtocolAnalysisCounterexample {
    /// The decisive witness, including the assessed root of the refutation.
    #[must_use]
    pub fn separating_witness(&self) -> &SeparatingWitnessRecord {
        match self {
            Self::Realizability(found) => found.separating_witness(),
            Self::Projection(found) => &found.separating_witness,
        }
    }

    /// The native analysis key of the refutation.
    #[must_use]
    pub fn key(&self) -> &ProtocolAnalysisKey {
        match self {
            Self::Realizability(found) => match found.as_ref() {
                RealizabilityCounterexample::IndistinguishableHistories { key, .. }
                | RealizabilityCounterexample::EnvironmentDefeat { key, .. } => key,
            },
            Self::Projection(found) => &found.key,
        }
    }

    /// Spell the complete FR-331 body using the native identity codecs.
    ///
    /// # Errors
    /// Returns an error when a native member has no canonical wire spelling.
    pub fn to_wire(&self) -> Result<Value, CounterexampleWireError> {
        match self {
            Self::Realizability(found) => found.to_wire(),
            Self::Projection(found) => found.to_wire(),
        }
    }
}

#[derive(Serialize)]
struct Body<'a> {
    #[serde(rename = "type")]
    wire_type: BodyType,
    #[serde(flatten)]
    evidence: Evidence<'a>,
}

#[derive(Serialize)]
enum BodyType {
    #[serde(rename = "protocol-analysis-counterexample")]
    ProtocolAnalysis,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Evidence<'a> {
    IndistinguishableHistories {
        key: KeyWire<'a>,
        role: &'a CheckedNodeId,
        history_a: PlayWire,
        history_b: PlayWire,
        action_a: StrategyActionWire<'a>,
        action_b: StrategyActionWire<'a>,
        obligation: &'a CheckedNodeId,
        separating_witness: ProtocolWitnessWire,
    },
    EnvironmentDefeat {
        key: KeyWire<'a>,
        defeats_per_branch: Vec<BranchWire>,
        obligation: &'a CheckedNodeId,
        separating_witness: ProtocolWitnessWire,
    },
    Projection {
        key: KeyWire<'a>,
        role: &'a CheckedNodeId,
        decision: DecisionWire<'a>,
        separating_witness: ProtocolWitnessWire,
    },
}

fn witness(
    record: &SeparatingWitnessRecord,
) -> Result<ProtocolWitnessWire, CounterexampleWireError> {
    ProtocolWitnessWire::of(record)
        .map_err(|_| CounterexampleWireError::Unspelled("protocol separating witness"))
}

fn body(evidence: Evidence<'_>) -> Result<Value, CounterexampleWireError> {
    serde_json::to_value(Body {
        wire_type: BodyType::ProtocolAnalysis,
        evidence,
    })
    .map_err(|_| CounterexampleWireError::Unspelled("protocol analysis counterexample"))
}

impl RealizabilityCounterexample {
    /// Spell the full realizability counterexample, separately from FR-352's record.
    ///
    /// # Errors
    /// Returns an error when a native member has no canonical wire spelling.
    pub fn to_wire(&self) -> Result<Value, CounterexampleWireError> {
        body(match self {
            Self::IndistinguishableHistories {
                key,
                role,
                history_a,
                history_b,
                action_a,
                action_b,
                obligation,
                separating_witness,
            } => Evidence::IndistinguishableHistories {
                key: KeyWire::of(key),
                role,
                history_a: play(history_a)?,
                history_b: play(history_b)?,
                action_a: strategy_action(action_a),
                action_b: strategy_action(action_b),
                obligation,
                separating_witness: witness(separating_witness)?,
            },
            Self::EnvironmentDefeat {
                key,
                defeats_per_branch,
                obligation,
                separating_witness,
            } => Evidence::EnvironmentDefeat {
                key: KeyWire::of(key),
                defeats_per_branch: defeats_per_branch
                    .iter()
                    .map(|branch| {
                        Ok(BranchWire {
                            play: play(&branch.play)?,
                        })
                    })
                    .collect::<Result<_, CounterexampleWireError>>()?,
                obligation,
                separating_witness: witness(separating_witness)?,
            },
        })
    }
}

impl ProjectionCounterexample {
    /// Spell the full native projection decision, separately from FR-352's record.
    ///
    /// # Errors
    /// Returns an error when a native member has no canonical wire spelling.
    pub fn to_wire(&self) -> Result<Value, CounterexampleWireError> {
        let decision = match &self.decision {
            ProjectionDecision::Choice {
                point,
                choice,
                owner,
                cases,
                paths,
                actions,
            } => {
                let body = ChoiceWire {
                    choice,
                    owner: owner_wire(owner),
                    cases,
                    paths: [
                        paths[0].iter().map(port).collect(),
                        paths[1].iter().map(port).collect(),
                    ],
                    actions: [
                        actions[0].as_ref().map(action),
                        actions[1].as_ref().map(action),
                    ],
                };
                match point {
                    DecisionKind::Choice => DecisionWire::Choice(body),
                    DecisionKind::Repeat => DecisionWire::Repeat(body),
                }
            }
            ProjectionDecision::ActingInstance {
                event,
                owner,
                instances,
                maximum,
            } => DecisionWire::ActingInstance {
                event: port(event),
                owner: owner_wire(owner),
                instances: instances.map(|instance| instance.to_string()),
                maximum: maximum.to_string(),
            },
        };
        body(Evidence::Projection {
            key: KeyWire::of(&self.key),
            role: &self.role,
            decision,
            separating_witness: witness(&self.separating_witness)?,
        })
    }
}

#[derive(Serialize)]
struct PlayWire {
    initial: String,
    steps: Vec<StepWire>,
}

#[derive(Serialize)]
struct StepWire {
    successor: String,
    transition: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    send_record: Option<Value>,
}

#[derive(Serialize)]
struct BranchWire {
    play: PlayWire,
}

fn ordinal(value: usize) -> Result<String, CounterexampleWireError> {
    u64::try_from(value)
        .map(|value| value.to_string())
        .map_err(|_| CounterexampleWireError::Unspelled("protocol play ordinal"))
}

fn play(play: &Play) -> Result<PlayWire, CounterexampleWireError> {
    let steps = play
        .steps
        .iter()
        .map(|step| {
            if (step.transition.kind() == StepKind::Send) != step.transition.send_record().is_some()
            {
                return Err(CounterexampleWireError::Unspelled("send record placement"));
            }
            Ok(StepWire {
                successor: ordinal(step.successor)?,
                transition: transition_json(&step.transition)
                    .ok_or(CounterexampleWireError::Unspelled("protocol transition"))?,
                send_record: step
                    .transition
                    .send_record()
                    .map(|record| {
                        send_record_json(record)
                            .ok_or(CounterexampleWireError::Unspelled("send record"))
                    })
                    .transpose()?,
            })
        })
        .collect::<Result<_, CounterexampleWireError>>()?;
    Ok(PlayWire {
        initial: ordinal(play.initial)?,
        steps,
    })
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum StrategyActionWire<'a> {
    Send { node: &'a CheckedNodeId },
    Receive { peer: &'a CheckedNodeId },
    End,
}

fn strategy_action(action: &StrategyAction) -> StrategyActionWire<'_> {
    match action {
        StrategyAction::Send { node } => StrategyActionWire::Send { node },
        StrategyAction::Receive { peer } => StrategyActionWire::Receive { peer },
        StrategyAction::End => StrategyActionWire::End,
    }
}

#[derive(Serialize)]
struct ChoiceWire<'a> {
    choice: &'a CheckedNodeId,
    owner: OwnerWire<'a>,
    cases: &'a [CheckedNodeId; 2],
    paths: [Vec<PortWire<'a>>; 2],
    actions: [Option<ActionWire<'a>>; 2],
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum DecisionWire<'a> {
    Choice(ChoiceWire<'a>),
    Repeat(ChoiceWire<'a>),
    ActingInstance {
        event: PortWire<'a>,
        owner: OwnerWire<'a>,
        instances: [String; 2],
        maximum: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum OwnerWire<'a> {
    Role { role: &'a CheckedNodeId },
    Environment,
}

fn owner_wire(owner: &DecisionOwner) -> OwnerWire<'_> {
    match owner {
        DecisionOwner::Role(role) => OwnerWire::Role { role },
        DecisionOwner::Environment => OwnerWire::Environment,
    }
}

#[derive(Serialize)]
struct PortWire<'a> {
    node: &'a CheckedNodeId,
    port: PortWireKind,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum PortWireKind {
    Enter,
    Exit,
}

fn port(at: &PortRef) -> PortWire<'_> {
    PortWire {
        node: &at.node,
        port: match at.port {
            Port::Enter => PortWireKind::Enter,
            Port::Exit => PortWireKind::Exit,
        },
    }
}

#[derive(Serialize)]
struct ActionWire<'a> {
    direction: DirectionWire,
    peer: &'a CheckedNodeId,
    message_type: &'a CheckedNodeId,
    at: PortWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum DirectionWire {
    Send,
    Receive,
}

fn action(action: &Action) -> ActionWire<'_> {
    ActionWire {
        direction: match action.direction {
            Direction::Send => DirectionWire::Send,
            Direction::Receive => DirectionWire::Receive,
        },
        peer: &action.peer,
        message_type: &action.message_type,
        at: port(&action.at),
    }
}
