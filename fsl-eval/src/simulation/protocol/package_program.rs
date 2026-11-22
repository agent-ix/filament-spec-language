// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531/FR-940: the package-native executable control form consumed by
//! protocol model-checking engines. This form retains admitted node
//! identities and slot descriptors; it never rebuilds `CheckedProtocol` or
//! `CheckedBody`.

use std::collections::BTreeMap;

use filament_eval::CallFailure;
use filament_exact::{Meter, NodeKey};
use fsl_foundation::MemoryModelName;
use fsl_package::protocol_view::{
    self, ChannelOrdering, ControlBody, ControlOperation, Delivery, Overflow, ParallelJoinPolicy,
    ProtocolExecutionView, ProtocolNodeId, QueueCapacity,
};
use fsl_semantics::check::{ChannelCapacity, ChannelDelivery, ChannelOverflow, LossSelector};

use super::{MemorySelection, MemorySelectionRefusal, ResolvedMemory};
use crate::simulation::model::{ModelCheckInput, ModelEvaluator, OperationRef, ProtocolDefinition};
use crate::value::{ProtocolSlot, ProtocolSlotEvaluation, ProtocolSuppliedValue, ProtocolSupply};

/// A literal invariant read from its admitted repeat, never a caller-issued proof.
#[derive(Clone, Copy)]
pub(super) struct PackageLoopInvariant<'a> {
    repeat: &'a fsl_package::CheckedNodeId,
    value: bool,
}

impl<'a> PackageLoopInvariant<'a> {
    pub(super) fn value(self) -> bool {
        self.value
    }

    pub(super) fn repeat(self) -> &'a fsl_package::CheckedNodeId {
        self.repeat
    }
}

/// One admitted package control in the normalized execution tree.
#[derive(Clone, Debug)]
pub struct PackageControl {
    /// Package-local protocol node identity.
    pub id: ProtocolNodeId,
    /// The closed control operation.
    pub operation: ControlOperation,
    /// The control's name within its declaration path.
    pub name: String,
    /// The protocol-relative path, without the protocol or `run` prefix.
    pub path: Vec<String>,
    /// The parent control, if this is not the root.
    pub parent: Option<ProtocolNodeId>,
    /// The structural children in execution order.
    pub children: Vec<ProtocolNodeId>,
    /// Labels and body nodes of a parallel's branches or a choice's cases.
    pub branches: Vec<(String, ProtocolNodeId)>,
    pub(super) join: Option<PackageJoin>,
    /// The resolved source default, for a `parallel`.
    pub memory_default: Option<MemoryModelName>,
    /// The operation's admitted predicate slot: a constraint or repeat guard.
    pub constraint: Option<ProtocolSlot>,
    /// The operation-anchor identity of an attempt or recommendation.
    pub operation_anchor: Option<NodeKey>,
    /// The declaring model type resolved by the package evaluator.
    pub operation_declaring: Option<filament_exact::EffectiveId>,
    /// The admitted operation declaration's qualified path.
    pub operation_path: Option<Vec<String>>,
    /// The admitted operation declaration's name.
    pub operation_name: Option<String>,
    /// The acting role node identity, including channel endpoint ownership.
    pub role: Option<fsl_package::CheckedNodeId>,
    /// The FR-531 identity of the acting role node.
    pub role_key: Option<NodeKey>,
    /// The step's result or message-payload binder node identity.
    pub binder: Option<fsl_package::CheckedNodeId>,
    /// The FR-531 identity of the result or payload binder.
    pub binder_key: Option<NodeKey>,
    /// The admitted value type of the result or payload binder.
    pub binder_type: Option<filament_exact::ValueType>,
    /// The admitted channel selected by a send or receive.
    pub(super) channel: Option<fsl_package::CheckedNodeId>,
    /// The admitted send selected by a receive's `of` member.
    pub(super) from: Option<ProtocolNodeId>,
    /// The attempt's explicit memory ordering.
    pub ordering: Option<fsl_semantics::check::AccessOrdering>,
    /// The state clauses selected by the operation call's `contracts` member.
    pub contracts: Vec<fsl_package::CheckedNodeId>,
    /// The FR-531 identities of the selected state clauses.
    pub contract_keys: Vec<NodeKey>,
}

#[derive(Clone, Debug)]
pub(super) struct PackageJoin {
    pub(super) policy: fsl_semantics::check::CheckedJoinPolicy,
    pub(super) joined: Vec<String>,
    pub(super) outstanding: Option<fsl_semantics::check::CheckedOutstanding>,
}

/// An admitted channel and its queue premises, before subject binding.
pub(super) struct PackageChannel {
    pub(super) node: fsl_package::CheckedNodeId,
    pub(super) name: String,
    pub(super) capacity: ChannelCapacity,
    pub(super) overflow: ChannelOverflow,
    pub(super) delivery: ChannelDelivery,
    pub(super) key: Option<PackageChannelKey>,
}

/// A FIFO key and its separate, level-zero payload binder.
pub(super) struct PackageChannelKey {
    pub(super) binder: fsl_package::CheckedNodeId,
    pub(super) slot: ProtocolSlot,
}

/// One authored premise, in source order, selecting canonical end slots.
pub(super) struct PackageRelated<'a> {
    pub(super) relationship: filament_checked::key::DeclarationKey,
    pub(super) authored: &'a fsl_package::ProtocolCorrelationRow,
    pub(super) ends: [ProtocolSlot; 2],
}

/// One static role binding from the admitted package.
#[derive(Clone, Debug)]
pub struct PackageRole {
    /// The role's name.
    pub name: String,
    /// The admitted role node.
    pub node: fsl_package::CheckedNodeId,
    /// The selected model type node.
    pub model: fsl_package::CheckedNodeId,
    /// The selected model type's effective identity.
    pub model_identity: filament_exact::EffectiveId,
    /// The FR-531 declaration identity of the selected model type.
    pub model_key: NodeKey,
    /// Whether this is a replicated role.
    pub replicated: bool,
    /// The declared replication maximum, when present.
    pub maximum: Option<u64>,
}

/// A package control's attempt placement under a memory-governed parallel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageAttemptContext {
    /// The attempt control.
    pub attempt: ProtocolNodeId,
    /// The outermost parallel that governs this attempt.
    pub parallel: ProtocolNodeId,
    /// The branch label within that parallel.
    pub branch: String,
    /// The attempt's explicit ordering.
    pub ordering: Option<fsl_semantics::check::AccessOrdering>,
}

/// The package-native protocol program needed by selection and execution.
pub struct PackageProtocolProgram<'a> {
    pub(super) related: BTreeMap<ProtocolNodeId, Vec<PackageRelated<'a>>>,
    /// The protocol's intended terminal states, read from its admitted obligation.
    pub(super) terminal: PackageTerminal,
    /// Origin captures in authored initialization order.
    pub(super) captures: Vec<PackageCapture>,
    /// Channels and their admitted queue premises.
    pub(super) channels: Vec<PackageChannel>,
    /// Repeat continuations and bounds read from admitted control members.
    pub(super) loops: BTreeMap<ProtocolNodeId, super::program::Loop<'a>>,
    /// Choice cases and their admitted guard slots, in case-label order.
    pub(super) choices: BTreeMap<ProtocolNodeId, Vec<(ProtocolNodeId, ProtocolSlot)>>,
    /// Await continuations derived from the admitted causal relation.
    pub(super) awaits: BTreeMap<ProtocolNodeId, super::program::Await>,
    /// The admitted protocol name.
    pub name: String,
    /// The `run` root control.
    pub root: ProtocolNodeId,
    /// The checked `over` parameter node.
    pub over: fsl_package::CheckedNodeId,
    /// The FR-531 identity of the `over` parameter node.
    pub over_key: NodeKey,
    /// The admitted value type of the `over` parameter.
    pub over_type: filament_exact::ValueType,
    /// The source/package-independent memory selection result.
    pub memory: ResolvedMemory,
    /// Every control, keyed by the package-local protocol node identity.
    pub controls: BTreeMap<ProtocolNodeId, PackageControl>,
    /// The static and replicated role bindings in package node order.
    pub roles: Vec<PackageRole>,
    /// The admitted control form used to evaluate package expression slots.
    pub form: ProtocolExecutionView<'a>,
    /// The package evaluator selected by the model-check input.
    evaluator: &'a dyn ModelEvaluator,
    /// The admitted `finish` slot, when its constraint is present.
    pub finish: Option<ProtocolSlot>,
    /// The admitted `finish` result binder node.
    pub finish_binder: fsl_package::CheckedNodeId,
    /// The FR-531 identity of the finish result binder.
    pub finish_binder_key: NodeKey,
    /// The admitted value type of the finish result binder.
    pub finish_binder_type: filament_exact::ValueType,
}

/// A protocol terminal declaration read directly from the admitted package.
pub(super) enum PackageTerminal {
    /// No terminal declaration: a stopped live instance is deadlocked.
    None,
    /// Every stopped state is intended.
    Any,
    /// The admitted condition decides whether a stopped state is intended.
    When(ProtocolSlot),
}

pub(super) struct PackageCapture {
    pub(super) name: String,
    pub(super) binder: fsl_package::CheckedNodeId,
    pub(super) value_type: filament_exact::ValueType,
    pub(super) initializer: ProtocolSlot,
}

fn member<'t>(
    term: &'t fsl_package::CheckedTerm,
    name: &str,
) -> Option<&'t fsl_package::CheckedTerm> {
    use fsl_package::CheckedTerm;
    let members = match term {
        CheckedTerm::Application { arguments, .. } => arguments,
        CheckedTerm::Aggregate { members } => members,
        _ => return None,
    };
    members.iter().find_map(|term| match term {
        CheckedTerm::Binding {
            name: bound, value, ..
        } if &**bound == name => Some(&**value),
        _ => None,
    })
}

fn channels(
    input: &ModelCheckInput<'_>,
    form: &ProtocolExecutionView<'_>,
) -> Result<Vec<PackageChannel>, PackageProgramRefusal> {
    for queue in &form.queues {
        let channel = protocol_view::queue_channel(queue)
            .map_err(|_| PackageProgramRefusal::MissingIdentity(queue.node_id.clone()))?;
        if !form.channels.iter().any(|node| &node.node_id == channel) {
            return Err(PackageProgramRefusal::MissingIdentity(
                queue.node_id.clone(),
            ));
        }
    }
    let mut channels = Vec::with_capacity(form.channels.len());
    for channel in &form.channels {
        let absent = || PackageProgramRefusal::MissingIdentity(channel.node_id.clone());
        let key = match protocol_view::channel_ordering(channel).map_err(|_| absent())? {
            ChannelOrdering::Unordered => None,
            ChannelOrdering::Fifo => {
                // Read the payload binder separately; the admitted key slot
                // owns expression evaluation and its binder inventory.
                let binder = member(&channel.body, "ordering")
                    .and_then(|ordering| member(ordering, "binder"))
                    .and_then(|term| match term {
                        fsl_package::CheckedTerm::Reference { target } => Some(target),
                        _ => None,
                    })
                    .ok_or_else(absent)?;
                Some(PackageChannelKey {
                    binder: binder.clone(),
                    slot: input
                        .evaluator
                        .package_protocol_slot(&channel.node_id, "ordering.key", None)
                        .ok_or_else(absent)?,
                })
            }
        };
        let queue = form
            .queues
            .iter()
            .find(|queue| {
                protocol_view::queue_channel(queue).is_ok_and(|id| id == &channel.node_id)
            })
            .ok_or_else(absent)?;
        let capacity = match protocol_view::queue_capacity(queue).map_err(|_| absent())? {
            QueueCapacity::Literal(capacity) => ChannelCapacity::Literal(capacity),
            QueueCapacity::Symbolic(name) => ChannelCapacity::Symbolic(name.to_owned()),
        };
        let overflow = match protocol_view::queue_overflow(queue).map_err(|_| absent())? {
            Overflow::Reject => ChannelOverflow::Reject,
            Overflow::Block => ChannelOverflow::Block,
            Overflow::Loss(selector) => ChannelOverflow::Loss(match selector {
                protocol_view::LossSelector::Head => LossSelector::Head,
                protocol_view::LossSelector::Tail => LossSelector::Tail,
                protocol_view::LossSelector::New => LossSelector::New,
            }),
        };
        let delivery = match protocol_view::delivery(channel).map_err(|_| absent())? {
            Delivery::Unknown => ChannelDelivery::Unknown,
            Delivery::AtMostOnce => ChannelDelivery::AtMostOnce,
            Delivery::AtLeastOnce => ChannelDelivery::AtLeastOnce,
            Delivery::ExactlyOncePremise => ChannelDelivery::ExactlyOncePremise,
            Delivery::Between { lower, upper } => ChannelDelivery::Between { lower, upper },
        };
        let name = channel
            .declaration
            .as_ref()
            .and_then(|declaration| declaration.qualified_name.get(2))
            .ok_or_else(absent)?;
        channels.push(PackageChannel {
            node: channel.node_id.clone(),
            name: format!("{}::{name}", form.name),
            capacity,
            overflow,
            delivery,
            key,
        });
    }
    Ok(channels)
}

/// Why a package protocol could not become an executable program.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageProgramRefusal {
    /// No protocol of the requested name is in the admitted package.
    #[error("invalid_runtime_input/invalid-value: no protocol {name}")]
    UnknownProtocol {
        /// The protocol name requested by the caller.
        name: String,
    },
    /// The request's memory selection did not resolve over the package form.
    #[error(transparent)]
    Memory(#[from] MemorySelectionRefusal),
    /// The package form lacks a model-evaluator identity for a referenced node.
    #[error("the package has no evaluator identity for node {0:?}")]
    MissingIdentity(fsl_package::CheckedNodeId),
    /// A binder's exact type could not be read from the admitted package.
    #[error("package parameter type projection failed at {node:?}: {failure:?}")]
    ParameterType {
        /// The package whose admitted binder is being read.
        package: String,
        /// The admitted binder whose type could not be projected.
        node: fsl_package::CheckedNodeId,
        /// The original typed selection or projection refusal.
        failure: Box<fsl_package::BareSelectionFailure>,
    },
    /// The evaluator cannot expose the admitted binders of a selected slot.
    #[error("the package has no admitted binder inventory for slot {node:?}.{member}")]
    MissingSlotBinders {
        /// The admitted identity of the node that owns the slot.
        node: NodeKey,
        /// The node member containing the slot.
        member: String,
    },
    /// The execution engine does not step this checked package operation yet.
    #[error("unsupported_construct/not-yet-implemented: package control {0:?}")]
    Unsupported(ControlOperation),
    /// A selected control has correlation premises with no executable package form.
    #[error("unsupported_construct/not-yet-implemented: package correlation at {node:?}")]
    Correlation {
        /// The admitted control whose `related` group is nonempty.
        node: fsl_package::CheckedNodeId,
    },
    /// Nonempty correlation lacks the producer's authored clause evidence.
    #[error(transparent)]
    CorrelationProvenance(fsl_package::UnavailableCorrelationProvenance),
    /// A repeat carries proof expressions without executable package readers.
    #[error("unsupported_construct/not-yet-implemented: package repeat proof at {node:?}")]
    RepeatProof {
        /// The admitted repeat whose proof is nonempty.
        node: fsl_package::CheckedNodeId,
    },
    /// A replicated role's lifetime needs a package evaluator or scope form.
    #[error(
        "unsupported_construct/not-yet-implemented: package role lifetime {lifetime} at {node:?}"
    )]
    RoleLifetime {
        /// The admitted role node.
        node: fsl_package::CheckedNodeId,
        /// The admitted lifetime, `scope` or `until`.
        lifetime: String,
    },
    /// A node referenced by the control tree has no package execution entry.
    #[error("the package control tree references an absent control")]
    UnknownControl,
    /// An attempt has no package-resolved operation.
    #[error("the package attempt {0:?} has no resolved operation")]
    MissingOperation(ProtocolNodeId),
    /// An attempt's role is absent from the package protocol.
    #[error("the package protocol has no role node {0:?}")]
    MissingRole(fsl_package::CheckedNodeId),
}

impl<'a> PackageProtocolProgram<'a> {
    /// Build the package-native protocol program from the selected route's
    /// model-check input and memory bindings.
    pub fn new(
        input: &'a ModelCheckInput<'a>,
        name: &str,
        selection: &[MemorySelection],
    ) -> Result<Self, PackageProgramRefusal> {
        let definition = input.protocol_definition(name).ok_or_else(|| {
            PackageProgramRefusal::UnknownProtocol {
                name: name.to_owned(),
            }
        })?;
        let ProtocolDefinition::Package(form) = definition else {
            return Err(PackageProgramRefusal::UnknownProtocol {
                name: name.to_owned(),
            });
        };
        let memory = ResolvedMemory::resolve_package(&form, selection)?;
        Self::with_memory(input, *form, memory)
    }

    /// Build the package-native execution form from an already resolved
    /// subject memory binding.
    pub fn with_memory(
        input: &ModelCheckInput<'a>,
        form: ProtocolExecutionView<'a>,
        memory: ResolvedMemory,
    ) -> Result<Self, PackageProgramRefusal> {
        Self::from_form(input, form, memory)
    }

    fn from_form(
        input: &ModelCheckInput<'a>,
        form: ProtocolExecutionView<'a>,
        memory: ResolvedMemory,
    ) -> Result<Self, PackageProgramRefusal> {
        let admitted =
            input
                .admitted_package()
                .ok_or_else(|| PackageProgramRefusal::UnknownProtocol {
                    name: form.name.clone(),
                })?;
        let channels = channels(input, &form)?;
        if !form.protocol.compensations().is_empty() {
            return Err(PackageProgramRefusal::Unsupported(
                ControlOperation::Compensate,
            ));
        }
        let terminal = match form.protocol.terminal() {
            None => PackageTerminal::None,
            Some(id) => {
                let mode = admitted
                    .resolved_graph()
                    .nodes
                    .iter()
                    .find(|node| &node.node_id == id)
                    .and_then(|node| match &node.body {
                        fsl_package::CheckedTerm::Application { arguments, .. } => {
                            arguments.iter().find_map(|argument| match argument {
                                fsl_package::CheckedTerm::Binding { name, value, .. }
                                    if &**name == "mode" =>
                                {
                                    match value.literal_read().map(|literal| literal.value) {
                                        Some(fsl_package::CheckedLiteralValue::Text(mode)) => {
                                            Some(mode)
                                        }
                                        _ => None,
                                    }
                                }
                                _ => None,
                            })
                        }
                        _ => None,
                    });
                match mode {
                    Some("any") => PackageTerminal::Any,
                    Some("when") => PackageTerminal::When(
                        input
                            .evaluator
                            .package_protocol_slot(id, "condition", None)
                            .ok_or_else(|| PackageProgramRefusal::MissingIdentity(id.clone()))?,
                    ),
                    _ => return Err(PackageProgramRefusal::MissingIdentity(id.clone())),
                }
            }
        };
        let clause = admitted
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == form.protocol.clause())
            .ok_or_else(|| {
                PackageProgramRefusal::MissingIdentity(form.protocol.clause().clone())
            })?;
        let Some(fsl_package::CheckedTerm::Aggregate {
            members: capture_rows,
        }) = member(&clause.body, "captures")
        else {
            return Err(PackageProgramRefusal::MissingIdentity(
                clause.node_id.clone(),
            ));
        };
        let captures = capture_rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let Some(fsl_package::CheckedTerm::Reference { target: binder }) =
                    member(row, "binder")
                else {
                    return Err(PackageProgramRefusal::MissingIdentity(
                        clause.node_id.clone(),
                    ));
                };
                let binder_node = admitted
                    .resolved_graph()
                    .nodes
                    .iter()
                    .find(|node| &node.node_id == binder)
                    .ok_or_else(|| PackageProgramRefusal::MissingIdentity(binder.clone()))?;
                let name = member(&binder_node.body, "name")
                    .and_then(|term| term.literal_read())
                    .and_then(|literal| match literal.value {
                        fsl_package::CheckedLiteralValue::Text(name) => Some(name.to_owned()),
                        _ => None,
                    })
                    .ok_or_else(|| PackageProgramRefusal::MissingIdentity(binder.clone()))?;
                let value_type = input
                    .evaluator
                    .package_protocol_parameter_type(binder)
                    .map_err(|failure| PackageProgramRefusal::ParameterType {
                        package: admitted.name().to_owned(),
                        node: binder.clone(),
                        failure: Box::new(failure),
                    })?;
                let initializer = input
                    .evaluator
                    .package_protocol_slot(&clause.node_id, "captures", Some(index))
                    .ok_or_else(|| {
                        PackageProgramRefusal::MissingIdentity(clause.node_id.clone())
                    })?;
                Ok(PackageCapture {
                    name,
                    binder: binder.clone(),
                    value_type,
                    initializer,
                })
            })
            .collect::<Result<Vec<_>, PackageProgramRefusal>>()?;
        let id_by_node: BTreeMap<_, _> = form
            .controls
            .iter()
            .map(|control| (control.node.node_id.clone(), control.id))
            .collect();
        let root = id_by_node
            .get(form.protocol.run())
            .copied()
            .ok_or(PackageProgramRefusal::UnknownControl)?;
        let mut loops = BTreeMap::new();
        for control in form
            .controls
            .iter()
            .filter(|control| control.operation == ControlOperation::Repeat)
        {
            let absent = || PackageProgramRefusal::MissingIdentity(control.node.node_id.clone());
            let body = &control.node.body;
            let Some(fsl_package::CheckedTerm::Aggregate { members: proof }) =
                member(body, "proof")
            else {
                return Err(absent());
            };
            let proof = if proof.is_empty() {
                None
            } else {
                let invariant = member(body, "proof")
                    .and_then(|proof| member(proof, "invariant"))
                    .and_then(|term| match term {
                        fsl_package::CheckedTerm::Reference { target } => {
                            (admitted.node_kind(target)
                                == Some(fsl_package::CheckedNodeKind::Value(
                                    fsl_package::ValueForm::Literal,
                                )))
                            .then(|| admitted.term(target))
                            .flatten()
                        }
                        _ => Some(term),
                    })
                    .and_then(|term| term.literal_read())
                    .and_then(|literal| match literal.value {
                        fsl_package::CheckedLiteralValue::Boolean(value) => Some(value),
                        _ => None,
                    })
                    .ok_or_else(|| PackageProgramRefusal::RepeatProof {
                        node: control.node.node_id.clone(),
                    })?;
                // Admission owns both typed terms. An invariant item reads
                // only its invariant, as the source proof product does.
                Some(super::program::LoopProof::Package(PackageLoopInvariant {
                    repeat: &control.node.node_id,
                    value: invariant,
                }))
            };
            let reference = |name| match member(body, name) {
                Some(fsl_package::CheckedTerm::Reference { target }) => {
                    id_by_node.get(target).copied().ok_or_else(absent)
                }
                _ => Err(absent()),
            };
            let Some(fsl_package::CheckedTerm::Aggregate { members: maximum }) =
                member(body, "maximum")
            else {
                return Err(absent());
            };
            let max = match maximum.as_slice() {
                [] => None,
                [maximum] => match maximum.literal_read().map(|literal| literal.value) {
                    Some(fsl_package::CheckedLiteralValue::Integer(value)) => {
                        Some(value.to_string().parse::<u64>().map_err(|_| absent())?)
                    }
                    _ => return Err(absent()),
                },
                _ => return Err(absent()),
            };
            let Some(fsl_package::CheckedTerm::Aggregate { members: exhausted }) =
                member(body, "exhausted")
            else {
                return Err(absent());
            };
            let exhausted = match exhausted.as_slice() {
                [] => None,
                [fsl_package::CheckedTerm::Reference { target }] => {
                    Some(*id_by_node.get(target).ok_or_else(absent)?)
                }
                _ => return Err(absent()),
            };
            let post_test = match member(body, "test") {
                None => false,
                Some(test) => match test.literal_read().map(|literal| literal.value) {
                    Some(fsl_package::CheckedLiteralValue::Text("post")) => true,
                    _ => return Err(absent()),
                },
            };
            loops.insert(
                control.id,
                super::program::Loop {
                    body: reference("body")?,
                    exhausted,
                    max,
                    post_test,
                    guard: super::program::LoopGuard::Package,
                    proof,
                },
            );
        }
        let mut awaits = BTreeMap::new();
        if form
            .controls
            .iter()
            .any(|control| control.operation == ControlOperation::Await)
        {
            let relation = protocol_view::causal_relation(admitted, form.protocol.clause())
                .map_err(|_| {
                    PackageProgramRefusal::MissingIdentity(form.protocol.clause().clone())
                })?;
            let edge_to = |kind: protocol_view::EdgeKind,
                           from: &fsl_package::CheckedNodeId,
                           port: protocol_view::Port| {
                relation
                    .edges()
                    .iter()
                    .find(|edge| {
                        edge.kind == kind
                            && &edge.from.node == from
                            && edge.from.port == port
                            && edge.to.port == protocol_view::Port::Enter
                    })
                    .and_then(|edge| id_by_node.get(&edge.to.node).map(|id| (&edge.to.node, *id)))
                    .ok_or(PackageProgramRefusal::UnknownControl)
            };
            for control in form
                .controls
                .iter()
                .filter(|control| control.operation == ControlOperation::Await)
            {
                let (event_node, event) = edge_to(
                    protocol_view::EdgeKind::AwaitSuccess,
                    &control.node.node_id,
                    protocol_view::Port::Enter,
                )?;
                let (_, then) = edge_to(
                    protocol_view::EdgeKind::AwaitSuccess,
                    event_node,
                    protocol_view::Port::Exit,
                )?;
                let (_, timeout) = edge_to(
                    protocol_view::EdgeKind::AwaitTimeout,
                    &control.node.node_id,
                    protocol_view::Port::Enter,
                )?;
                awaits.insert(
                    control.id,
                    super::program::Await {
                        event,
                        then,
                        timeout,
                    },
                );
            }
        }
        let path_parent = |control: &fsl_package::protocol_view::ProtocolControlView<'_>| {
            form.controls
                .iter()
                .filter(|candidate| {
                    candidate.id != control.id
                        && candidate.path.len() < control.path.len()
                        && control.path.starts_with(&candidate.path)
                })
                .max_by_key(|candidate| candidate.path.len())
                .map(|candidate| candidate.id)
        };
        let mut choices = BTreeMap::new();
        let mut controls = BTreeMap::new();
        let mut related = BTreeMap::new();
        for control in &form.controls {
            if control.operation == ControlOperation::Recommend
                && matches!(member(&control.node.body, "related"), Some(fsl_package::CheckedTerm::Aggregate { members }) if !members.is_empty())
            {
                return Err(PackageProgramRefusal::Correlation {
                    node: control.node.node_id.clone(),
                });
            }
            if matches!(
                control.operation,
                ControlOperation::Send | ControlOperation::Receive | ControlOperation::Attempt
            ) {
                let Some(fsl_package::CheckedTerm::Aggregate { members }) =
                    member(&control.node.body, "related")
                else {
                    return Err(PackageProgramRefusal::MissingIdentity(
                        control.node.node_id.clone(),
                    ));
                };
                if !members.is_empty()
                    && memory
                        .parallels()
                        .iter()
                        .any(|parallel| parallel.model.is_weak())
                {
                    return Err(PackageProgramRefusal::Correlation {
                        node: control.node.node_id.clone(),
                    });
                }
                let clauses = admitted
                    .protocol_correlation_clauses(&control.node.node_id)
                    .map_err(|failure| match failure {
                        fsl_package::ProtocolCorrelationReadError::Unavailable(cause) => {
                            PackageProgramRefusal::CorrelationProvenance(cause)
                        }
                        fsl_package::ProtocolCorrelationReadError::NotAnEligibleControl => {
                            PackageProgramRefusal::UnknownControl
                        }
                    })?;
                let mut premises = Vec::with_capacity(clauses.len());
                for clause in clauses {
                    let missing =
                        || PackageProgramRefusal::MissingIdentity(control.node.node_id.clone());
                    let index = usize::try_from(clause.canonical_index).map_err(|_| missing())?;
                    let record = members.get(index).ok_or_else(missing)?;
                    let fsl_package::CheckedTerm::Reference { target, .. } =
                        member(record, "relationship").ok_or_else(missing)?
                    else {
                        return Err(missing());
                    };
                    let relationship = admitted
                        .resolved_graph()
                        .nodes
                        .iter()
                        .find(|node| &node.node_id == target)
                        .ok_or_else(missing)?;
                    let Some(fsl_package::CheckedNodeOwner::Model { identity, node }) =
                        relationship.owner.as_ref()
                    else {
                        return Err(missing());
                    };
                    let end = |member| {
                        input
                            .evaluator
                            .package_protocol_slot(&control.node.node_id, member, Some(index))
                            .ok_or_else(missing)
                    };
                    premises.push(PackageRelated {
                        relationship: filament_checked::key::DeclarationKey {
                            package: identity.to_string(),
                            node: node.to_string(),
                        },
                        authored: clause,
                        ends: [end("related.from")?, end("related.to")?],
                    });
                }
                if !premises.is_empty() {
                    related.insert(control.id, premises);
                }
            }
            let name = control.path.last().cloned().unwrap_or_default();
            let channel = match &control.body {
                ControlBody::Send { channel, .. } | ControlBody::Receive { channel, .. } => {
                    Some((*channel).clone())
                }
                _ => None,
            };
            let from = match &control.body {
                ControlBody::Receive { of, .. } => Some(
                    id_by_node
                        .get(*of)
                        .copied()
                        .ok_or(PackageProgramRefusal::UnknownControl)?,
                ),
                _ => None,
            };
            let join = match &control.body {
                ControlBody::Parallel { join, .. } => {
                    use fsl_semantics::check::{CheckedJoinPolicy, CheckedOutstanding};
                    let policy = match join.policy {
                        ParallelJoinPolicy::All => CheckedJoinPolicy::All,
                        ParallelJoinPolicy::Any => CheckedJoinPolicy::Any,
                        ParallelJoinPolicy::Quorum | ParallelJoinPolicy::Predicate => {
                            return Err(PackageProgramRefusal::Unsupported(control.operation));
                        }
                    };
                    let outstanding = match join.outstanding {
                        None => None,
                        Some("cancel") => Some(CheckedOutstanding::Cancel),
                        Some("continue") => Some(CheckedOutstanding::Continue),
                        Some(_) => return Err(PackageProgramRefusal::UnknownControl),
                    };
                    Some(PackageJoin {
                        policy,
                        joined: join
                            .joined
                            .iter()
                            .map(|label| (*label).to_owned())
                            .collect(),
                        outstanding,
                    })
                }
                _ => None,
            };
            let (
                children,
                branches,
                role,
                binder,
                operation_anchor,
                operation_declaring,
                operation_path,
                operation_name,
                ordering,
                contracts,
            ) = match &control.body {
                ControlBody::Choice { owner, cases } => {
                    let branches = cases
                        .iter()
                        .map(|(label, body)| {
                            id_by_node
                                .get(*body)
                                .copied()
                                .map(|id| ((*label).to_owned(), id))
                                .ok_or(PackageProgramRefusal::UnknownControl)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let guards = branches
                        .iter()
                        .enumerate()
                        .map(|(index, (_, body))| {
                            let guard = input
                                .evaluator
                                .package_protocol_slot(
                                    &control.node.node_id,
                                    "cases.guard",
                                    Some(index),
                                )
                                .ok_or_else(|| {
                                    PackageProgramRefusal::MissingIdentity(
                                        control.node.node_id.clone(),
                                    )
                                })?;
                            Ok((*body, guard))
                        })
                        .collect::<Result<Vec<_>, PackageProgramRefusal>>()?;
                    choices.insert(control.id, guards);
                    (
                        branches.iter().map(|(_, body)| *body).collect(),
                        branches,
                        Some((*owner).clone()),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                    )
                }
                ControlBody::Sequence { children } => (
                    children
                        .iter()
                        .map(|child| {
                            id_by_node
                                .get(*child)
                                .copied()
                                .ok_or(PackageProgramRefusal::UnknownControl)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    Vec::new(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Vec::new(),
                ),
                ControlBody::Parallel { branches, .. } => {
                    let branches = branches
                        .iter()
                        .map(|(label, body)| {
                            id_by_node
                                .get(*body)
                                .copied()
                                .map(|id| ((*label).to_owned(), id))
                                .ok_or(PackageProgramRefusal::UnknownControl)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let children = branches.iter().map(|(_, id)| *id).collect();
                    (
                        children,
                        branches,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                    )
                }
                ControlBody::Attempt {
                    role,
                    operation,
                    contracts,
                    ..
                }
                | ControlBody::Recommend {
                    role,
                    operation,
                    contracts,
                    ..
                } => {
                    let anchor = input.evaluator.package_node_key(operation).ok_or_else(|| {
                        PackageProgramRefusal::MissingIdentity((*operation).clone())
                    })?;
                    let (operation_declaring, operation_name) = input
                        .evaluator
                        .package_protocol_operation(operation)
                        .ok_or_else(|| {
                            PackageProgramRefusal::MissingIdentity((*operation).clone())
                        })?;
                    // The package identity row supplies the declaring type
                    // and operation name; the operation anchor itself has no
                    // declaration-qualified path.
                    (
                        Vec::new(),
                        Vec::new(),
                        Some((*role).clone()),
                        match &control.body {
                            ControlBody::Attempt { binder, .. } => Some((*binder).clone()),
                            _ => None,
                        },
                        Some(anchor),
                        Some(operation_declaring),
                        None,
                        Some(operation_name),
                        match &control.body {
                            ControlBody::Attempt { ordering, .. } => *ordering,
                            _ => None,
                        },
                        contracts
                            .iter()
                            .map(|contract| (*contract).clone())
                            .collect(),
                    )
                }
                ControlBody::Send { binder, .. } | ControlBody::Receive { binder, .. } => (
                    Vec::new(),
                    Vec::new(),
                    None,
                    Some((*binder).clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    Vec::new(),
                ),
                ControlBody::Check { .. } => (
                    Vec::new(),
                    Vec::new(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    Vec::new(),
                ),
                ControlBody::Other(ControlOperation::Repeat) => {
                    let repeat = loops
                        .get(&control.id)
                        .ok_or(PackageProgramRefusal::UnknownControl)?;
                    (
                        std::iter::once(repeat.body)
                            .chain(repeat.exhausted)
                            .collect(),
                        Vec::new(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                    )
                }
                ControlBody::Other(ControlOperation::Await) => {
                    let awaited = awaits
                        .get(&control.id)
                        .ok_or(PackageProgramRefusal::UnknownControl)?;
                    (
                        vec![awaited.event, awaited.then, awaited.timeout],
                        Vec::new(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                    )
                }
                _ => return Err(PackageProgramRefusal::Unsupported(control.operation)),
            };
            let constraint = if control.operation == ControlOperation::Repeat
                || matches!(
                    &control.body,
                    ControlBody::Attempt { .. }
                        | ControlBody::Recommend { .. }
                        | ControlBody::Check { .. }
                        | ControlBody::Send { .. }
                        | ControlBody::Receive { .. }
                ) {
                Some(
                    input
                        .evaluator
                        .package_protocol_slot(
                            &control.node.node_id,
                            if control.operation == ControlOperation::Repeat {
                                "guard"
                            } else if control.operation == ControlOperation::Check {
                                "condition"
                            } else {
                                "constraint"
                            },
                            None,
                        )
                        .ok_or_else(|| {
                            PackageProgramRefusal::MissingIdentity(control.node.node_id.clone())
                        })?,
                )
            } else {
                None
            };
            let contract_keys = contracts
                .iter()
                .map(|contract| {
                    input
                        .evaluator
                        .package_node_key(contract)
                        .ok_or_else(|| PackageProgramRefusal::MissingIdentity(contract.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let role = match &control.body {
                ControlBody::Send { channel, .. } | ControlBody::Receive { channel, .. } => {
                    let channel = form
                        .channels
                        .iter()
                        .find(|node| &node.node_id == *channel)
                        .ok_or_else(|| {
                            PackageProgramRefusal::MissingIdentity((*channel).clone())
                        })?;
                    let ends = protocol_view::channel_endpoints(channel).map_err(|_| {
                        PackageProgramRefusal::MissingIdentity(channel.node_id.clone())
                    })?;
                    Some(
                        if control.operation == ControlOperation::Send {
                            ends.sender
                        } else {
                            ends.receiver
                        }
                        .clone(),
                    )
                }
                _ => role,
            };
            let role_key = role
                .as_ref()
                .map(|role| {
                    input
                        .evaluator
                        .package_node_key(role)
                        .ok_or_else(|| PackageProgramRefusal::MissingIdentity(role.clone()))
                })
                .transpose()?;
            let binder_key = binder
                .as_ref()
                .map(|binder| {
                    input
                        .evaluator
                        .package_node_key(binder)
                        .ok_or_else(|| PackageProgramRefusal::MissingIdentity(binder.clone()))
                })
                .transpose()?;
            let binder_type = binder
                .as_ref()
                .map(|binder| {
                    input
                        .evaluator
                        .package_protocol_parameter_type(binder)
                        .map_err(|failure| PackageProgramRefusal::ParameterType {
                            package: admitted.name().to_owned(),
                            node: binder.clone(),
                            failure: Box::new(failure),
                        })
                })
                .transpose()?;
            controls.insert(
                control.id,
                PackageControl {
                    id: control.id,
                    operation: control.operation,
                    name,
                    path: control.path.clone(),
                    parent: path_parent(control),
                    children,
                    branches,
                    join,
                    memory_default: control.memory,
                    constraint,
                    operation_anchor,
                    operation_declaring,
                    operation_path,
                    operation_name,
                    role,
                    role_key,
                    binder,
                    binder_key,
                    binder_type,
                    channel,
                    from,
                    ordering,
                    contracts,
                    contract_keys,
                },
            );
        }
        let roles = form
            .roles
            .iter()
            .map(|role| {
                let model_key = input
                    .evaluator
                    .package_node_key(role.model)
                    .ok_or_else(|| PackageProgramRefusal::MissingIdentity(role.model.clone()))?;
                let model_identity = input
                    .evaluator
                    .package_protocol_role_model(role.model)
                    .ok_or_else(|| PackageProgramRefusal::MissingIdentity(role.model.clone()))?;
                if role.replicated {
                    let absent =
                        || PackageProgramRefusal::MissingIdentity(role.node.node_id.clone());
                    let replication = member(&role.node.body, "replication").ok_or_else(absent)?;
                    let text = |name| {
                        member(replication, name)
                            .and_then(|term| term.literal_read())
                            .and_then(|literal| match literal.value {
                                fsl_package::CheckedLiteralValue::Text(text) => Some(text),
                                _ => None,
                            })
                    };
                    match text("lifetime") {
                        Some("workflow") => {}
                        Some(lifetime @ ("scope" | "until")) => {
                            return Err(PackageProgramRefusal::RoleLifetime {
                                node: role.node.node_id.clone(),
                                lifetime: lifetime.to_owned(),
                            })
                        }
                        _ => return Err(absent()),
                    }
                    if text("population").is_none_or(str::is_empty) || role.maximum.is_none() {
                        return Err(absent());
                    }
                    let Some(fsl_package::CheckedTerm::Reference { target: binder }) =
                        member(replication, "binder")
                    else {
                        return Err(absent());
                    };
                    let binder_type = input
                        .evaluator
                        .package_protocol_parameter_type(binder)
                        .map_err(|failure| PackageProgramRefusal::ParameterType {
                            package: admitted.name().to_owned(),
                            node: binder.clone(),
                            failure: Box::new(failure),
                        })?;
                    if binder_type != filament_exact::ValueType::Reference(model_identity.clone()) {
                        return Err(absent());
                    }
                }
                Ok(PackageRole {
                    name: role.path.last().cloned().unwrap_or_default(),
                    node: role.node.node_id.clone(),
                    model: role.model.clone(),
                    model_identity,
                    model_key,
                    replicated: role.replicated,
                    maximum: role.maximum,
                })
            })
            .collect::<Result<Vec<_>, PackageProgramRefusal>>()?;
        let finish = form.finish.constraint.map(|_| {
            input
                .evaluator
                .package_protocol_slot(&form.finish.node.node_id, "constraint", None)
                .ok_or_else(|| {
                    PackageProgramRefusal::MissingIdentity(form.finish.node.node_id.clone())
                })
        });
        let finish = finish.transpose()?;
        let over = form.protocol.over().clone();
        let over_key = input
            .evaluator
            .package_node_key(&over)
            .ok_or_else(|| PackageProgramRefusal::MissingIdentity(over.clone()))?;
        let over_type = input
            .evaluator
            .package_protocol_parameter_type(&over)
            .map_err(|failure| PackageProgramRefusal::ParameterType {
                package: admitted.name().to_owned(),
                node: over.clone(),
                failure: Box::new(failure),
            })?;
        let finish_binder = form.finish.binder.clone();
        let finish_binder_key = input
            .evaluator
            .package_node_key(&finish_binder)
            .ok_or_else(|| PackageProgramRefusal::MissingIdentity(finish_binder.clone()))?;
        let finish_binder_type = input
            .evaluator
            .package_protocol_parameter_type(&finish_binder)
            .map_err(|failure| PackageProgramRefusal::ParameterType {
                package: admitted.name().to_owned(),
                node: finish_binder.clone(),
                failure: Box::new(failure),
            })?;
        Ok(Self {
            related,
            terminal,
            captures,
            channels,
            loops,
            choices,
            awaits,
            name: form.name.clone(),
            root,
            over,
            over_key,
            over_type,
            memory,
            controls,
            roles,
            form,
            evaluator: input.evaluator,
            finish,
            finish_binder,
            finish_binder_key,
            finish_binder_type,
        })
    }

    /// Evaluate one admitted package predicate slot for a control. Controls
    /// without a constraint or guard have no expression to evaluate. The supplied
    /// binder values and population provenance are checked by the package
    /// evaluator against the slot's admitted identity.
    pub fn evaluate_control_constraint(
        &self,
        control: ProtocolNodeId,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<Option<ProtocolSlotEvaluation>, CallFailure> {
        let Some(slot) = self
            .controls
            .get(&control)
            .and_then(|control| control.constraint.as_ref())
        else {
            return Ok(None);
        };
        self.evaluator
            .evaluate_package_protocol_slot(slot, supply, meter)
            .map(Some)
    }

    /// Evaluate one admitted predicate slot using the existing package evaluator.
    pub(super) fn evaluate_slot(
        &self,
        slot: &ProtocolSlot,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        self.evaluator
            .evaluate_package_protocol_slot(slot, supply, meter)
    }

    /// Evaluate the admitted package `finish` expression when present in the
    /// execution program.
    pub fn evaluate_finish(
        &self,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<Option<ProtocolSlotEvaluation>, CallFailure> {
        let Some(slot) = self.finish.as_ref() else {
            return Ok(None);
        };
        self.evaluator
            .evaluate_package_protocol_slot(slot, supply, meter)
            .map(Some)
    }

    /// Resolve an admitted attempt or recommendation to its model operation input.
    pub(crate) fn operation_ref(
        &self,
        control: ProtocolNodeId,
    ) -> Result<OperationRef<'_>, PackageProgramRefusal> {
        let call = self
            .controls
            .get(&control)
            .filter(|control| {
                matches!(
                    control.operation,
                    ControlOperation::Attempt | ControlOperation::Recommend
                )
            })
            .ok_or(PackageProgramRefusal::MissingOperation(control))?;
        let declaring = call
            .operation_declaring
            .clone()
            .ok_or(PackageProgramRefusal::MissingOperation(control))?;
        let name = call
            .operation_name
            .as_deref()
            .ok_or(PackageProgramRefusal::MissingOperation(control))?;
        let role_id = call
            .role
            .as_ref()
            .ok_or(PackageProgramRefusal::MissingOperation(control))?;
        let role = self
            .roles
            .iter()
            .find(|candidate| candidate.node == *role_id)
            .ok_or_else(|| PackageProgramRefusal::MissingRole(role_id.clone()))?;
        Ok(OperationRef {
            declaring,
            name,
            receiver: role.model_identity.clone(),
        })
    }

    /// Build a package evaluator supply from the protocol's admitted binder
    /// nodes. Values stay keyed by package node identity until the evaluator
    /// checks them against the selected slot.
    pub fn supply<'obs>(
        &self,
        binders: impl IntoIterator<Item = (fsl_package::CheckedNodeId, ProtocolSuppliedValue)>,
        observation: Option<
            &'obs filament_semantic_value::admission::ObservationAdmission<
                fsl_package::AdmittedProtocolObservation<'obs>,
            >,
        >,
    ) -> Result<ProtocolSupply<'obs>, PackageProgramRefusal> {
        let binders = binders
            .into_iter()
            .map(|(node, value)| {
                self.evaluator
                    .package_node_key(&node)
                    .map(|key| (key, value))
                    .ok_or(PackageProgramRefusal::MissingIdentity(node))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProtocolSupply {
            binders,
            observation,
        })
    }

    /// Build a supply containing exactly the binders selected by `slot`.
    /// FR-390 rejects both missing and unrelated caller-supplied binders, so
    /// package execution projects the live protocol frame through the
    /// evaluator's admitted slot inventory before calling it.
    pub fn supply_for_slot<'obs>(
        &self,
        slot: &ProtocolSlot,
        binders: impl IntoIterator<Item = (fsl_package::CheckedNodeId, ProtocolSuppliedValue)>,
        observation: Option<
            &'obs filament_semantic_value::admission::ObservationAdmission<
                fsl_package::AdmittedProtocolObservation<'obs>,
            >,
        >,
    ) -> Result<ProtocolSupply<'obs>, PackageProgramRefusal> {
        let required = self.slot_binders(slot)?;
        let required = required
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut selected = Vec::new();
        for (node, value) in binders {
            let key = self
                .evaluator
                .package_node_key(&node)
                .ok_or_else(|| PackageProgramRefusal::MissingIdentity(node.clone()))?;
            if required.contains(&key) {
                selected.push((node, value));
            }
        }
        self.supply(selected, observation)
    }

    /// The admitted binder reads of one selected expression slot.
    pub(super) fn slot_binders(
        &self,
        slot: &ProtocolSlot,
    ) -> Result<Vec<NodeKey>, PackageProgramRefusal> {
        self.evaluator
            .package_protocol_slot_binders(slot)
            .ok_or_else(|| PackageProgramRefusal::MissingSlotBinders {
                node: slot.node.clone(),
                member: slot.member.clone(),
            })
    }

    /// The outermost `parallel` whose memory model governs `control`, if the
    /// control is within a parallel.
    pub fn governing_parallel(&self, control: ProtocolNodeId) -> Option<ProtocolNodeId> {
        let mut ancestors = Vec::new();
        let mut cursor = Some(control);
        while let Some(id) = cursor {
            let node = self.controls.get(&id)?;
            if node.operation == ControlOperation::Parallel {
                ancestors.push(id);
            }
            cursor = node.parent;
        }
        ancestors.reverse();
        ancestors.into_iter().next()
    }

    /// The resolved memory model governing `control`.
    pub fn governing_memory(&self, control: ProtocolNodeId) -> Option<MemoryModelName> {
        let parallel = self.governing_parallel(control)?;
        let descriptor = self.controls.get(&parallel)?;
        let path = std::iter::once(self.name.as_str())
            .chain(descriptor.path.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join("::");
        self.memory.model_of(&path)
    }

    /// The resolved model of every `parallel`, including nested controls
    /// governed by their outermost ancestor.
    pub fn governing_models(
        &self,
    ) -> Result<BTreeMap<ProtocolNodeId, MemoryModelName>, MemorySelectionRefusal> {
        self.memory.governing_package(&self.form)
    }

    /// The attempts nested in weak memory-governed branches, in package
    /// control order. This is the package-native input to EN-1 access and
    /// race prechecks.
    pub fn weak_attempt_contexts(&self) -> Vec<PackageAttemptContext> {
        self.controls
            .values()
            .filter(|control| control.operation == ControlOperation::Attempt)
            .filter_map(|attempt| {
                let memory = self.governing_memory(attempt.id)?;
                memory.is_weak().then_some(())?;
                let parallel = self.governing_parallel(attempt.id)?;
                let parallel_control = self.controls.get(&parallel)?;
                let mut child = attempt.id;
                while self.controls.get(&child)?.parent != Some(parallel) {
                    child = self.controls.get(&child)?.parent?;
                }
                let branch = parallel_control
                    .branches
                    .iter()
                    .find_map(|(label, body)| (*body == child).then(|| label.clone()))?;
                Some(PackageAttemptContext {
                    attempt: attempt.id,
                    parallel,
                    branch,
                    ordering: attempt.ordering,
                })
            })
            .collect()
    }
}
