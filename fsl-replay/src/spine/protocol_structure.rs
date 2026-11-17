// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-3, PA-9: the global event structure of a checked protocol,
//! read from a checked package only through `fsl_package::protocol_view`.
//! It sits in `spine`, the checked route, because it takes a
//! `CheckedPackageV2` (ADR-032 CK-3).

use std::collections::BTreeMap;

use filament_contract_model::{
    CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm, LiteralKind,
};
use fsl_package::protocol_view::{
    ChannelOrdering, ControlBody, EdgeKind, Port, PortRef, ViewRefusal, causal_relation,
    channel_endpoints, channel_ordering, control_body, control_operation, decision_reads,
    fifo_key_reads_message, protocol_view, role_replication_maximum,
};

use crate::protocol_analysis::{
    Comm, GlobalEvent, GlobalEventKind, GlobalStructure, JoinPolicy, OutstandingPolicy, RunBudget,
    RunLimit, StructureInput, UniformFifo,
};

/// Why a structure was not built.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum StructureError {
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// A node the view named is not in the package.
    #[error("the package holds no node {0:?}")]
    UnknownNode(CheckedNodeId),
    /// A run limit stopped the build.
    #[error("the structure build reached {} = {}", .0.limit.as_str(), .0.value)]
    Limit(RunLimit),
}

struct Reader<'p> {
    package: &'p CheckedPackageV2,
    receives: Vec<(CheckedNodeId, CheckedNodeId)>,
    input: StructureInput,
}

impl<'p> Reader<'p> {
    fn node(&self, id: &CheckedNodeId) -> Result<&'p CheckedSemanticNodeV2, StructureError> {
        self.package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == id)
            .ok_or_else(|| StructureError::UnknownNode(id.clone()))
    }

    fn comm(&self, channel: &CheckedNodeId) -> Result<Comm, StructureError> {
        let ends = channel_endpoints(self.node(channel)?)?;
        Ok(Comm {
            channel: channel.clone(),
            sender: ends.sender.clone(),
            receiver: ends.receiver.clone(),
            message_type: ends.message_type.clone(),
        })
    }

    fn reference_member(node: &CheckedSemanticNodeV2, member: &str) -> Option<CheckedNodeId> {
        let filament_contract_model::CheckedTerm::Application { arguments, .. } = &node.body else {
            return None;
        };
        arguments.iter().find_map(|argument| match argument {
            filament_contract_model::CheckedTerm::Binding { name, value } if &**name == member => {
                match &**value {
                    filament_contract_model::CheckedTerm::Reference { target } => {
                        Some(target.clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }

    fn term_member<'n>(node: &'n CheckedSemanticNodeV2, member: &str) -> Option<&'n CheckedTerm> {
        let CheckedTerm::Application { arguments, .. } = &node.body else {
            return None;
        };
        arguments.iter().find_map(|argument| match argument {
            CheckedTerm::Binding { name, value } if &**name == member => Some(&**value),
            _ => None,
        })
    }

    fn optional_reference_member(
        node: &CheckedSemanticNodeV2,
        member: &str,
    ) -> Option<CheckedNodeId> {
        let CheckedTerm::Aggregate { members } = Self::term_member(node, member)? else {
            return None;
        };
        match members.as_slice() {
            [] => None,
            [CheckedTerm::Reference { target }] => Some(target.clone()),
            _ => None,
        }
    }

    fn integer(term: &CheckedTerm) -> Option<u64> {
        match term {
            CheckedTerm::Literal {
                value_kind: LiteralKind::Integer,
                value,
                ..
            } => value.as_str()?.parse::<u64>().ok(),
            CheckedTerm::Aggregate { members } => members.iter().find_map(|member| match member {
                CheckedTerm::Binding { name, value } if &**name == "value" => Self::integer(value),
                _ => None,
            }),
            _ => None,
        }
    }

    fn text(term: &CheckedTerm) -> Option<String> {
        match term {
            CheckedTerm::Literal {
                value_kind: LiteralKind::Text,
                value,
                ..
            } => Some(value.as_str()?.to_owned()),
            _ => None,
        }
    }

    fn projection_attributes(
        node: &CheckedSemanticNodeV2,
        operation: fsl_package::protocol_view::ControlOperation,
    ) -> (Option<u64>, Option<(u64, u64, String)>) {
        use fsl_package::protocol_view::ControlOperation as Op;
        match operation {
            Op::Repeat => (
                Self::term_member(node, "maximum").and_then(Self::integer),
                None,
            ),
            Op::Await => {
                let window = Self::term_member(node, "within").and_then(|term| {
                    let CheckedTerm::Aggregate { members } = term else {
                        return None;
                    };
                    let bound = |name| {
                        members.iter().find_map(|member| match member {
                            CheckedTerm::Binding { name: held, value } if &**held == name => {
                                Self::integer(value)
                            }
                            _ => None,
                        })
                    };
                    Some((bound("lower")?, bound("upper")?))
                });
                let clock = Self::term_member(node, "clock").and_then(Self::text);
                (
                    None,
                    window
                        .zip(clock)
                        .map(|((lower, upper), clock)| (lower, upper, clock)),
                )
            }
            _ => (None, None),
        }
    }

    fn owner_of(&self, node: &CheckedSemanticNodeV2) -> Option<CheckedNodeId> {
        let operation = control_operation(node).ok()?;
        match operation {
            fsl_package::protocol_view::ControlOperation::Send
            | fsl_package::protocol_view::ControlOperation::Receive => {
                let channel = Self::reference_member(node, "channel")?;
                let endpoints = channel_endpoints(self.node(&channel).ok()?).ok()?;
                Some(match operation {
                    fsl_package::protocol_view::ControlOperation::Send => endpoints.sender.clone(),
                    _ => endpoints.receiver.clone(),
                })
            }
            fsl_package::protocol_view::ControlOperation::Effect => {
                let attempt = Self::reference_member(node, "of")?;
                self.owner_of(self.node(&attempt).ok()?)
            }
            fsl_package::protocol_view::ControlOperation::Await => {
                let event = Self::reference_member(node, "event")?;
                self.owner_of(self.node(&event).ok()?)
            }
            _ => Self::reference_member(node, "owner")
                .or_else(|| Self::reference_member(node, "by"))
                .or_else(|| Self::reference_member(node, "role")),
        }
    }

    fn walk(
        &mut self,
        id: &CheckedNodeId,
        within: &mut Vec<CheckedNodeId>,
    ) -> Result<(), StructureError> {
        within.push(id.clone());
        let enter = PortRef {
            node: id.clone(),
            port: Port::Enter,
        };
        let exit = PortRef {
            node: id.clone(),
            port: Port::Exit,
        };
        let node = self.node(id)?;
        match control_body(node)? {
            ControlBody::Sequence { children } => {
                for child in children {
                    self.walk(child, within)?;
                }
            }
            ControlBody::Choice { owner, cases } => {
                let reads = decision_reads(self.package, node)?;
                let carriers = self
                    .receives
                    .iter()
                    .filter(|(_, binder)| reads.contains(binder))
                    .map(|(receive, _)| receive.clone())
                    .collect();
                self.input.events.push(GlobalEvent {
                    at: enter.clone(),
                    out: enter,
                    kind: GlobalEventKind::Choice {
                        owner: owner.clone(),
                        cases: cases
                            .iter()
                            .map(|(label, body)| ((*label).to_owned(), (*body).clone()))
                            .collect(),
                        carriers,
                    },
                    within: within.clone(),
                });
                for (_, body) in cases {
                    self.walk(body, within)?;
                }
            }
            ControlBody::Parallel { branches, join } => {
                let bodies: Vec<CheckedNodeId> =
                    branches.iter().map(|(_, body)| (*body).clone()).collect();
                self.input.events.push(GlobalEvent {
                    at: enter.clone(),
                    out: enter,
                    kind: GlobalEventKind::Region {
                        branches: bodies.clone(),
                    },
                    within: within.clone(),
                });
                self.input.events.push(GlobalEvent {
                    at: exit.clone(),
                    out: exit,
                    kind: GlobalEventKind::Join {
                        branches: bodies,
                        policy: match join.policy {
                            fsl_package::protocol_view::ParallelJoinPolicy::All => JoinPolicy::All,
                            fsl_package::protocol_view::ParallelJoinPolicy::Any => JoinPolicy::Any,
                            fsl_package::protocol_view::ParallelJoinPolicy::Quorum => {
                                JoinPolicy::Quorum {
                                    count: join.quorum.expect("checked quorum has count"),
                                }
                            }
                            fsl_package::protocol_view::ParallelJoinPolicy::Predicate => {
                                JoinPolicy::Predicate {
                                    function: join
                                        .predicate
                                        .expect("checked predicate join has function")
                                        .clone(),
                                }
                            }
                        },
                        joined: join
                            .joined
                            .iter()
                            .map(|label| (*label).to_owned())
                            .collect(),
                        outstanding: join.outstanding.map(|word| match word {
                            "cancel" => OutstandingPolicy::Cancel,
                            "continue" => OutstandingPolicy::Continue,
                            _ => unreachable!("checked outstanding policy is cancel or continue"),
                        }),
                    },
                    within: within.clone(),
                });
                for (_, body) in branches {
                    self.walk(body, within)?;
                }
            }
            ControlBody::Send { channel, .. } => {
                let comm = self.comm(channel)?;
                self.input.events.push(GlobalEvent {
                    at: enter,
                    out: exit,
                    kind: GlobalEventKind::Send(comm),
                    within: within.clone(),
                });
            }
            ControlBody::Receive { channel, of, .. } => {
                self.input.receive_of.insert(id.clone(), of.clone());
                let comm = self.comm(channel)?;
                self.input.events.push(GlobalEvent {
                    at: enter,
                    out: exit,
                    kind: GlobalEventKind::Receive(comm),
                    within: within.clone(),
                });
            }
            ControlBody::Other(operation) => {
                if matches!(
                    operation,
                    fsl_package::protocol_view::ControlOperation::Repeat
                        | fsl_package::protocol_view::ControlOperation::Await
                        | fsl_package::protocol_view::ControlOperation::Attempt
                        | fsl_package::protocol_view::ControlOperation::Effect
                        | fsl_package::protocol_view::ControlOperation::Event
                        | fsl_package::protocol_view::ControlOperation::Commit
                        | fsl_package::protocol_view::ControlOperation::Compensate
                        | fsl_package::protocol_view::ControlOperation::Check
                        | fsl_package::protocol_view::ControlOperation::Fence
                ) {
                    let (maximum, window) = Self::projection_attributes(node, operation);
                    self.input.events.push(GlobalEvent {
                        at: enter,
                        out: exit,
                        kind: GlobalEventKind::Control {
                            operation,
                            owner: self.owner_of(node),
                            maximum,
                            window,
                        },
                        within: within.clone(),
                    });
                }
                let children = match operation {
                    fsl_package::protocol_view::ControlOperation::Repeat => {
                        let mut children = Self::reference_member(node, "body")
                            .into_iter()
                            .collect::<Vec<_>>();
                        children.extend(Self::optional_reference_member(node, "exhausted"));
                        children
                    }
                    fsl_package::protocol_view::ControlOperation::Await => [
                        Self::reference_member(node, "event"),
                        Self::reference_member(node, "then"),
                        Self::reference_member(node, "timeout"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                    _ => Vec::new(),
                };
                for child in children {
                    self.walk(&child, within)?;
                }
            }
        }
        within.pop();
        Ok(())
    }
}

impl GlobalStructure {
    /// The structure of the protocol clause `protocol`, from its FR-519
    /// view and causal relation (PA-3, PA-9). The caller has already
    /// refused constructs its selected profile does not admit.
    ///
    /// # Errors
    ///
    /// [`StructureError`] when a view function refuses, a node is absent, or
    /// `budget` stops the build.
    pub fn from_package(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
        budget: RunBudget,
    ) -> Result<Self, StructureError> {
        let view = protocol_view(package, protocol)?;
        let relation = causal_relation(package, protocol)?;
        let mut reader = Reader {
            package,
            receives: Vec::new(),
            input: StructureInput::default(),
        };
        for id in view.controls() {
            if let ControlBody::Receive { binder, .. } = control_body(reader.node(id)?)? {
                reader.receives.push(((*id).clone(), binder.clone()));
            }
        }
        reader.input.edges = relation
            .edges()
            .iter()
            .filter(|edge| edge.kind != EdgeKind::RepeatBack)
            .map(|edge| (edge.from.clone(), edge.to.clone()))
            .collect();
        let mut fifo = BTreeMap::new();
        for id in view.channels() {
            let node = reader.node(id)?;
            if channel_ordering(node)? == ChannelOrdering::Fifo
                && !fifo_key_reads_message(package, node)?
            {
                fifo.insert(
                    (*id).clone(),
                    UniformFifo {
                        channel: (*id).clone(),
                    },
                );
            }
        }
        reader.input.uniform_fifo = fifo.into_values().collect();
        reader.walk(view.run(), &mut Vec::new())?;
        for template in view.compensations() {
            let node = reader.node(template)?;
            let operation = control_operation(node)?;
            if operation == fsl_package::protocol_view::ControlOperation::Compensate {
                let (maximum, window) = Reader::projection_attributes(node, operation);
                let owner = reader.owner_of(node);
                reader.input.events.push(GlobalEvent {
                    at: PortRef {
                        node: (*template).clone(),
                        port: Port::Enter,
                    },
                    out: PortRef {
                        node: (*template).clone(),
                        port: Port::Exit,
                    },
                    kind: GlobalEventKind::Control {
                        operation,
                        owner,
                        maximum,
                        window,
                    },
                    within: vec![(*template).clone()],
                });
            }
        }
        let mut maxima = BTreeMap::new();
        for role in view.roles() {
            let maximum = role_replication_maximum(reader.node(role)?)?;
            if let Some(maximum) = maximum {
                maxima.insert((*role).clone(), maximum);
            }
        }
        let mut structure =
            GlobalStructure::build(reader.input, budget).map_err(StructureError::Limit)?;
        structure.set_role_maxima(maxima);
        Ok(structure)
    }
}

/// Why the protocol analysis replay arm did not run.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolReplayRefusal {
    /// The source is no `ProtocolAnalysis` source.
    #[error(
        "invalid_runtime_input/invalid-value at /replay/source: protocol analysis replay requires a ProtocolAnalysis source"
    )]
    WrongSource,
    /// The evidence's key is not a projection key.
    #[error("invalid_runtime_input/invalid-value: the evidence key is not a projection key")]
    NotAProjectionKey,
    /// The structure did not build.
    #[error(transparent)]
    Structure(#[from] StructureError),
}

/// The `ReplaySource::ProtocolAnalysis` arm (ADR-036 PA-20): re-walk the
/// counterexample's stated paths against the checked graph of `protocol`,
/// check each step is an admitted successor and that the walk ends in the
/// stated divergence. An arm that disagrees settles `inconclusive`.
///
/// # Errors
///
/// [`ProtocolReplayRefusal`] when the source is another arm or the graph
/// does not build.
pub fn replay_protocol_analysis(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
    source: &crate::witness::ReplaySource,
    budget: RunBudget,
) -> Result<crate::result::ReplayResult, ProtocolReplayRefusal> {
    use crate::protocol_analysis::{
        ProtocolAnalysisKey, ProtocolReplayResult, ProtocolReplaySettlement,
        replay_projection_counterexample,
    };
    let crate::witness::ReplaySource::ProtocolAnalysis(evidence) = source else {
        return Err(ProtocolReplayRefusal::WrongSource);
    };
    let ProtocolAnalysisKey::Projection(key) = &evidence.counterexample.key else {
        return Err(ProtocolReplayRefusal::NotAProjectionKey);
    };
    let structure = GlobalStructure::from_package(package, protocol, budget)?;
    let settlement = match replay_projection_counterexample(
        &structure,
        &evidence.counterexample,
        key.bounds.max_depth,
    ) {
        Ok(()) => ProtocolReplaySettlement::Reproduced,
        Err(disagreement) => ProtocolReplaySettlement::Inconclusive(disagreement),
    };
    Ok(crate::result::ReplayResult::ProtocolAnalysis(
        ProtocolReplayResult { settlement },
    ))
}

/// The names a realizability arena's steps resolve to in the protocol clause
/// `protocol` of `package` (ADR-036 PA-16): each control and channel by its
/// declared name, the roles, the `finish` member and the clause.
/// `fsl-eval` names a control by its path through the `run` tree with case
/// labels (`Main/Decide/yes/Ship/PickIt`) and a channel by its qualified name
/// (`Flow::Pick`); the package declares each under the clause by its own
/// name.
///
/// # Errors
///
/// [`ViewRefusal`] when a protocol read refuses.
pub fn arena_names(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
    premises: &crate::protocol_analysis::ProtocolPremises,
) -> Result<crate::protocol_analysis::arena::ArenaNames, ViewRefusal> {
    let view = protocol_view(package, protocol)?;
    let graph = package.resolved_graph();
    let named = |ids: &[&CheckedNodeId]| {
        ids.iter()
            .filter_map(|id| {
                graph
                    .nodes
                    .iter()
                    .find(|node| &node.node_id == *id)
                    .and_then(|node| node.declaration.as_ref())
                    .and_then(|declaration| declaration.qualified_name.last())
                    .map(|segment| (segment.to_string(), (*id).clone()))
            })
            .collect()
    };
    Ok(crate::protocol_analysis::arena::ArenaNames::from_parts(
        named(view.controls()),
        named(view.channels()),
        view.roles().iter().map(|id| (*id).clone()).collect(),
        view.finish().clone(),
        view.clause().clone(),
        premises,
    ))
}
