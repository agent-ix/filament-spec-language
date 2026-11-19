// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531/FR-940: the package-native executable control form consumed by
//! protocol model-checking engines. This form retains admitted node
//! identities and slot descriptors; it never rebuilds `CheckedProtocol` or
//! `CheckedBody`.

use std::collections::BTreeMap;

use filament_exact::NodeKey;
use fsl_foundation::MemoryModelName;
use fsl_package::protocol_view::{
    ControlBody, ControlOperation, ParallelJoinPolicy, ProtocolExecutionView, ProtocolNodeId,
};

use super::{MemorySelection, MemorySelectionRefusal, ResolvedMemory};
use crate::simulation::model::{ModelCheckInput, ProtocolDefinition};
use crate::value::ProtocolSlot;

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
    /// Branch labels for a `parallel`, in package order.
    pub branches: Vec<(String, ProtocolNodeId)>,
    /// The resolved source default, for a `parallel`.
    pub memory_default: Option<MemoryModelName>,
    /// The operation's admitted constraint slot, for expression controls.
    pub constraint: Option<ProtocolSlot>,
    /// The operation's operation-anchor identity, for an `attempt`.
    pub operation_anchor: Option<NodeKey>,
    /// The attempt's role node identity.
    pub role: Option<fsl_package::CheckedNodeId>,
    /// The attempt's explicit memory ordering.
    pub ordering: Option<fsl_forms::AccessOrdering>,
    /// The state clauses selected by an attempt's `contracts` member.
    pub contracts: Vec<fsl_package::CheckedNodeId>,
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
    /// Whether this is a replicated role.
    pub replicated: bool,
    /// The declared replication maximum, when present.
    pub maximum: Option<u64>,
}

/// The package-native protocol program needed by selection and execution.
pub struct PackageProtocolProgram<'a> {
    /// The admitted protocol name.
    pub name: String,
    /// The `run` root control.
    pub root: ProtocolNodeId,
    /// The source/package-independent memory selection result.
    pub memory: ResolvedMemory,
    /// Every control, keyed by the package-local protocol node identity.
    pub controls: BTreeMap<ProtocolNodeId, PackageControl>,
    /// The static and replicated role bindings in package node order.
    pub roles: Vec<PackageRole>,
    /// The admitted control form used to evaluate package expression slots.
    pub form: ProtocolExecutionView<'a>,
    /// The admitted `finish` slot, when its constraint is present.
    pub finish: Option<ProtocolSlot>,
}

/// Why a package protocol could not become an executable program.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageProgramRefusal {
    /// No protocol of the requested name is in the admitted package.
    #[error("invalid_runtime_input/invalid-value: no protocol {name}")]
    UnknownProtocol { name: String },
    /// The request's memory selection did not resolve over the package form.
    #[error(transparent)]
    Memory(#[from] MemorySelectionRefusal),
    /// The package form lacks a model-evaluator identity for a referenced node.
    #[error("the package has no evaluator identity for node {0:?}")]
    MissingIdentity(fsl_package::CheckedNodeId),
    /// The execution engine does not step this checked package operation yet.
    #[error("unsupported_construct/not-yet-implemented: package control {0:?}")]
    Unsupported(ControlOperation),
    /// A node referenced by the control tree has no package execution entry.
    #[error("the package control tree references an absent control")]
    UnknownControl,
}

impl<'a> PackageProtocolProgram<'a> {
    /// Build the package-native protocol program from the selected route's
    /// model-check input and memory bindings.
    pub fn new(
        input: &ModelCheckInput<'a>,
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
        Self::from_form(input, form, memory)
    }

    fn from_form(
        input: &ModelCheckInput<'a>,
        form: ProtocolExecutionView<'a>,
        memory: ResolvedMemory,
    ) -> Result<Self, PackageProgramRefusal> {
        if !form.channels.is_empty() || !form.queues.is_empty() {
            return Err(PackageProgramRefusal::Unsupported(
                ControlOperation::Channel,
            ));
        }
        if !form.protocol.compensations().is_empty() {
            return Err(PackageProgramRefusal::Unsupported(
                ControlOperation::Compensate,
            ));
        }
        if form.protocol.terminal().is_some() {
            return Err(PackageProgramRefusal::Unsupported(
                ControlOperation::Terminal,
            ));
        }
        let id_by_node: BTreeMap<_, _> = form
            .controls
            .iter()
            .map(|control| (control.node.node_id.clone(), control.id))
            .collect();
        let root = id_by_node
            .get(form.protocol.run())
            .copied()
            .ok_or(PackageProgramRefusal::UnknownControl)?;
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
        let mut controls = BTreeMap::new();
        for control in &form.controls {
            let name = control.path.last().cloned().unwrap_or_default();
            let (children, branches, role, operation_anchor, ordering, contracts) =
                match &control.body {
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
                        Vec::new(),
                    ),
                    ControlBody::Parallel { branches, join } => {
                        if join.policy != ParallelJoinPolicy::All {
                            return Err(PackageProgramRefusal::Unsupported(control.operation));
                        }
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
                        (children, branches, None, None, None, Vec::new())
                    }
                    ControlBody::Attempt {
                        role,
                        operation,
                        ordering,
                        contracts,
                        ..
                    } => {
                        let anchor =
                            input.evaluator.package_node_key(operation).ok_or_else(|| {
                                PackageProgramRefusal::MissingIdentity(operation.clone())
                            })?;
                        (
                            Vec::new(),
                            Vec::new(),
                            Some((*role).clone()),
                            Some(anchor),
                            *ordering,
                            contracts
                                .iter()
                                .map(|contract| (*contract).clone())
                                .collect(),
                        )
                    }
                    _ => return Err(PackageProgramRefusal::Unsupported(control.operation)),
                };
            let constraint = if matches!(&control.body, ControlBody::Attempt { .. }) {
                Some(
                    input
                        .evaluator
                        .package_protocol_slot(&control.node.node_id, "constraint", None)
                        .ok_or_else(|| {
                            PackageProgramRefusal::MissingIdentity(control.node.node_id.clone())
                        })?,
                )
            } else {
                None
            };
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
                    memory_default: control.memory,
                    constraint,
                    operation_anchor,
                    role,
                    ordering,
                    contracts,
                },
            );
        }
        let roles = form
            .roles
            .iter()
            .map(|role| {
                Ok(PackageRole {
                    name: role.path.last().cloned().unwrap_or_default(),
                    node: role.node.node_id.clone(),
                    model: role.model.clone(),
                    replicated: role.replicated,
                    maximum: role.maximum,
                })
            })
            .collect::<Result<Vec<_>, PackageProgramRefusal>>()?;
        let finish = Some(
            input
                .evaluator
                .package_protocol_slot(form.protocol.finish(), "constraint", None)
                .ok_or_else(|| {
                    PackageProgramRefusal::MissingIdentity(form.protocol.finish().clone())
                })?,
        );
        Ok(Self {
            name: form.name.clone(),
            root,
            memory,
            controls,
            roles,
            form,
            finish,
        })
    }
}
