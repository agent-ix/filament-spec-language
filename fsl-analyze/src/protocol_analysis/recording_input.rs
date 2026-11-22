// SPDX-License-Identifier: AGPL-3.0-or-later
//! Recording facts retain either source bodies or admitted package slots.

use fsl_eval::simulation::model::ModelCheckInput;
use fsl_eval::simulation::protocol::{PackageProtocolProgram, ResolvedMemory};
use fsl_eval::value::ProtocolSlot;
use fsl_package::protocol_view::{self, ControlBody, ControlOperation};
use fsl_semantics::check::{self, CheckedBody, CheckedControl, CheckedControlTree, ProtocolNodeId};

pub(super) enum Body<'a> {
    Source(&'a CheckedBody),
    Package(ProtocolSlot),
}

impl<'a> From<&'a CheckedBody> for Body<'a> {
    fn from(body: &'a CheckedBody) -> Self {
        Self::Source(body)
    }
}

impl<'a> From<&Body<'a>> for Body<'a> {
    fn from(body: &Body<'a>) -> Self {
        match body { Body::Source(body) => Self::Source(body), Body::Package(slot) => Self::Package(slot.clone()) }
    }
}

pub(super) struct NodeBody<'a> {
    pub node: ProtocolNodeId,
    pub body: Body<'a>,
}

pub(super) struct Case<'a> {
    pub name: String,
    pub guard: Body<'a>,
}

pub(super) struct Choice<'a> {
    pub node: ProtocolNodeId,
    pub cases: Vec<Case<'a>>,
}

pub(super) struct Slots<'a> {
    pub bodies: Vec<NodeBody<'a>>,
    pub choices: Vec<Choice<'a>>,
    pub repeats: Vec<&'a check::CheckedRepeat>,
    pub binders: Vec<check::CheckedSlotBinder>,
    pub package_binders: Vec<fsl_package::CheckedNodeId>,
}

pub(super) enum Ordering<'a> {
    Unordered,
    Fifo { key: Body<'a> },
}

pub(super) struct Channel<'a> {
    pub name: String,
    pub ordering: Ordering<'a>,
    pub delivery: check::ChannelDelivery,
    pub capacity: check::ChannelCapacity,
    pub overflow: check::ChannelOverflow,
}

pub(super) struct Role {
    pub name: String,
    pub replicated: bool,
}

pub(super) struct Obligations<'a> {
    pub compensations: Vec<&'a check::CheckedCompensation>,
}

/// The assessment's metadata; this is never a rebuilt source protocol.
pub(super) struct Protocol<'a> {
    pub name: String,
    pub on_each: bool,
    pub obligations: Obligations<'a>,
    pub controls: CheckedControlTree,
    pub roles: Vec<Role>,
    pub owners: Vec<check::CheckedOwner>,
    pub vias: Vec<check::CheckedVia>,
    pub edges: Vec<check::CausalEdge>,
    pub finish: Option<(ProtocolNodeId, String)>,
    pub slots: Slots<'a>,
    pub channels: Vec<Channel<'a>>,
    pub requirements: Vec<&'a check::CheckedRequirement>,
    pub awaits: Vec<&'a check::CheckedAwait>,
    pub parallels: Vec<&'a check::CheckedParallel>,
}

impl<'a> Protocol<'a> {
    pub fn source(source: &'a check::CheckedProtocol) -> Self {
        Self {
            name: source.name.clone(),
            on_each: matches!(source.obligations.activation, check::CheckedActivation::OnEach { .. }),
            obligations: Obligations { compensations: source.obligations.compensations.iter().collect() },
            controls: source.controls.clone(),
            roles: source.roles.iter().map(|role| Role { name: role.name.clone(), replicated: role.replicated }).collect(),
            owners: source.owners.clone(),
            vias: source.vias.clone(),
            edges: source.edges.clone(),
            finish: source.finish.clone(),
            slots: Slots {
                bodies: source.slots.bodies.iter().map(|body| NodeBody { node: body.node, body: Body::Source(&body.body) }).collect(),
                choices: source.slots.choices.iter().map(|choice| Choice { node: choice.node, cases: choice.cases.iter().map(|case| Case { name: case.name.clone(), guard: Body::Source(&case.guard) }).collect() }).collect(),
                repeats: source.slots.repeats.iter().collect(),
                binders: source.slots.binders.clone(),
                package_binders: Vec::new(),
            },
            channels: source.channels.iter().map(|channel| Channel {
                name: channel.name.clone(),
                ordering: match &channel.ordering {
                    check::CheckedOrdering::Unordered => Ordering::Unordered,
                    check::CheckedOrdering::Fifo { key, .. } => Ordering::Fifo { key: Body::Source(key) },
                },
                delivery: channel.delivery,
                capacity: channel.capacity.clone(),
                overflow: channel.overflow,
            }).collect(),
            requirements: source.requirements.iter().collect(),
            awaits: source.awaits.iter().collect(),
            parallels: source.parallels.iter().collect(),
        }
    }

    /// The initial package recording slice: origin-activated static roles,
    /// unordered bounded channels, sequence, choice, message and check.
    /// Other forms remain explicitly pending before an assessment starts.
    pub fn package(input: &ModelCheckInput<'a>, name: &str) -> Option<Self> {
        let form = input.evaluator.package_protocol(name)?;
        if !form.protocol.compensations().is_empty()
            || !form.protocol.requirements().is_empty()
            || form.roles.iter().any(|role| role.replicated)
            || form.controls.iter().any(|control| !matches!(control.operation,
                ControlOperation::Sequence | ControlOperation::Choice | ControlOperation::Send
                | ControlOperation::Receive | ControlOperation::Check))
        {
            return None;
        }
        let admitted = input.admitted_package()?;
        let clause = admitted.resolved_graph().nodes.iter().find(|node| &node.node_id == form.protocol.clause())?;
        if protocol_view::clause_activation(clause).ok()? != protocol_view::ClauseActivation::Origin {
            return None;
        }
        if !matches!(member(&clause.body, "captures"), Some(fsl_package::CheckedTerm::Aggregate { members }) if members.is_empty()) {
            return None;
        }
        let memory = ResolvedMemory::resolve_package(&form, &[]).ok()?;
        let program = PackageProtocolProgram::with_memory(input, form, memory).ok()?;
        let mut controls = Vec::new();
        let mut ids = std::collections::BTreeMap::new();
        let mut bodies = Vec::new();
        let mut choices = Vec::new();
        let mut owners = Vec::new();
        let mut binders = vec![check::CheckedSlotBinder { name: "over".to_owned(), value_type: program.over_type.clone(), node: None, ranged: false }];
        let mut package_binders = vec![program.over.clone()];
        let mut pending: Vec<(ProtocolNodeId, Option<Vec<String>>)> = vec![(program.root, None)];
        while let Some((id, parent_case)) = pending.pop() {
            let control = program.controls.get(&id)?;
            if let Some(path) = parent_case {
                let case_id = ProtocolNodeId(controls.len());
                controls.push(CheckedControl {
                    node: case_id, kind: fsl_forms::ProtocolNodeKind::Case,
                    name: path.last()?.clone(), path, children: vec![ProtocolNodeId(controls.len() + 1)], observe_only: false,
                });
            }
            let node = ProtocolNodeId(controls.len());
            ids.insert(id, node);
            let kind = match control.operation {
                ControlOperation::Sequence => fsl_forms::ProtocolNodeKind::Sequence,
                ControlOperation::Choice => fsl_forms::ProtocolNodeKind::Choice,
                ControlOperation::Send => fsl_forms::ProtocolNodeKind::Send,
                ControlOperation::Receive => fsl_forms::ProtocolNodeKind::Receive,
                ControlOperation::Check => fsl_forms::ProtocolNodeKind::Check,
                _ => return None,
            };
            controls.push(CheckedControl { node, kind, name: control.name.clone(), path: control.path.clone(), children: Vec::new(), observe_only: false });
            if let Some(slot) = &control.constraint {
                bodies.push(NodeBody { node, body: Body::Package(slot.clone()) });
            }
            if let (Some(binder), Some(value_type)) = (&control.binder, &control.binder_type) {
                package_binders.push(binder.clone());
                binders.push(check::CheckedSlotBinder { name: control.name.clone(), value_type: value_type.clone(), node: Some(node), ranged: true });
            }
            let source_control = program.form.controls.iter().find(|candidate| candidate.id == id)?;
            if let ControlBody::Choice { owner, cases } = &source_control.body {
                let role = program.roles.iter().position(|role| &role.node == *owner)?;
                owners.push(check::CheckedOwner { node, role });
                choices.push(Choice { node, cases: cases.iter().enumerate().map(|(index, (label, _))| {
                    Some(Case { name: (*label).to_owned(), guard: Body::Package(input.evaluator.package_protocol_slot(&source_control.node.node_id, "cases.guard", Some(index))?) })
                }).collect::<Option<Vec<_>>>()? });
                let mut branches = control.branches.iter().map(|(label, child)| {
                    let native = program.form.controls.iter().find(|candidate| candidate.id == *child)?;
                    let region = admitted.source_map().iter().filter(|entry| entry.node_id == native.node.node_id)
                        .flat_map(|entry| &entry.regions).min_by_key(|region| region.start)?;
                    Some((label, child, region.start))
                }).collect::<Option<Vec<_>>>()?;
                branches.sort_by_key(|(_, _, start)| *start);
                for (label, child, _) in branches.into_iter().rev() {
                    let mut path = control.path.clone();
                    path.push(label.clone());
                    // Local assessment ids come from the derived control tree;
                    // they never become package node identities or evidence.
                    pending.push((*child, Some(path)));
                }
            } else {
                pending.extend(control.children.iter().rev().map(|child| (*child, None)));
            }
        }
        let children = controls.iter().map(|control| {
            if control.kind == fsl_forms::ProtocolNodeKind::Case {
                return Some(control.children.clone());
            }
            let candidate = program.controls.values().find(|candidate| ids.get(&candidate.id) == Some(&control.node))?;
            if candidate.operation == ControlOperation::Choice {
                candidate.branches.iter().map(|(label, _)| {
                    let mut path = candidate.path.clone();
                    path.push(label.clone());
                    controls.iter().find(|control| control.path == path).map(|control| control.node)
                }).collect()
            } else {
                candidate.children.iter().map(|child| ids.get(child).copied()).collect()
            }
        }).collect::<Option<Vec<Vec<ProtocolNodeId>>>>()?;
        for (control, children) in controls.iter_mut().zip(children) {
            control.children = children;
        }
        let finish_id = ProtocolNodeId(controls.len());
        if let Some(slot) = &program.finish {
            bodies.push(NodeBody { node: finish_id, body: Body::Package(slot.clone()) });
        }
        package_binders.push(program.finish_binder.clone());
        binders.push(check::CheckedSlotBinder { name: "finish".to_owned(), value_type: program.finish_binder_type.clone(), node: Some(finish_id), ranged: true });
        let mut vias = Vec::new();
        let mut edges = Vec::new();
        for control in &program.form.controls {
            let node = *ids.get(&control.id)?;
            match &control.body {
                ControlBody::Send { channel, .. } | ControlBody::Receive { channel, .. } => {
                    let channel_node = program.form.channels.iter().find(|candidate| &candidate.node_id == *channel)?;
                    let (_, path) = protocol_view::declaration(channel_node).ok()?;
                    vias.push(check::CheckedVia { node, channel: path.iter().map(AsRef::as_ref).collect::<Vec<_>>().join("::") });
                    let endpoints = protocol_view::channel_endpoints(channel_node).ok()?;
                    let owner = if matches!(control.body, ControlBody::Send { .. }) { endpoints.sender } else { endpoints.receiver };
                    owners.push(check::CheckedOwner { node, role: program.roles.iter().position(|role| &role.node == owner)? });
                    if let ControlBody::Receive { of, .. } = &control.body {
                        let source = program.form.controls.iter().find(|candidate| &candidate.node.node_id == *of)?;
                        edges.push(check::CausalEdge { kind: check::EdgeKind::Message,
                            from: check::PortRef { node: *ids.get(&source.id)?, port: check::Port::Exit },
                            to: check::PortRef { node, port: check::Port::Enter } });
                    }
                }
                _ => {}
            }
        }
        edges.sort_by_key(|edge| edge.to.node);
        let mut channels = Vec::new();
        for channel in &program.form.channels {
            if protocol_view::channel_ordering(channel).ok()? != protocol_view::ChannelOrdering::Unordered {
                return None;
            }
            let queue = program.form.queues.iter().find(|queue| protocol_view::queue_channel(queue).ok() == Some(&channel.node_id))?;
            let protocol_view::QueueCapacity::Literal(capacity) = protocol_view::queue_capacity(queue).ok()? else { return None; };
            let delivery = match protocol_view::delivery(channel).ok()? {
                protocol_view::Delivery::Unknown => check::ChannelDelivery::Unknown,
                protocol_view::Delivery::AtMostOnce => check::ChannelDelivery::AtMostOnce,
                protocol_view::Delivery::AtLeastOnce => check::ChannelDelivery::AtLeastOnce,
                protocol_view::Delivery::ExactlyOncePremise => check::ChannelDelivery::ExactlyOncePremise,
                protocol_view::Delivery::Between { lower, upper } => check::ChannelDelivery::Between { lower, upper },
            };
            let overflow = match protocol_view::queue_overflow(queue).ok()? {
                protocol_view::Overflow::Reject => check::ChannelOverflow::Reject,
                protocol_view::Overflow::Block => check::ChannelOverflow::Block,
                _ => return None,
            };
            channels.push(Channel { name: protocol_view::declaration(channel).ok()?.1.iter().map(AsRef::as_ref).collect::<Vec<_>>().join("::"), ordering: Ordering::Unordered, delivery, capacity: check::ChannelCapacity::Literal(capacity), overflow });
        }
        Some(Self {
            name: name.to_owned(), on_each: false, obligations: Obligations { compensations: Vec::new() },
            controls: CheckedControlTree { controls },
            roles: program.roles.iter().map(|role| Role { name: role.name.clone(), replicated: role.replicated }).collect(),
            owners, vias, edges, finish: Some((finish_id, protocol_view::declaration(program.form.finish.node).ok()?.1.last()?.to_string())),
            slots: Slots { bodies, choices, repeats: Vec::new(), binders, package_binders },
            channels, requirements: Vec::new(), awaits: Vec::new(), parallels: Vec::new(),
        })
    }
}

fn member<'a>(term: &'a fsl_package::CheckedTerm, name: &str) -> Option<&'a fsl_package::CheckedTerm> {
    use fsl_package::CheckedTerm;
    let members = match term { CheckedTerm::Application { arguments, .. } => arguments, CheckedTerm::Aggregate { members } => members, _ => return None };
    members.iter().find_map(|term| match term { CheckedTerm::Binding { name: found, value, .. } if found.as_ref() == name => Some(value.as_ref()), _ => None })
}

pub(super) enum Input<'a> {
    Source(&'a fsl_package::CheckedPackage),
    Package(ModelCheckInput<'a>),
}

impl Input<'_> {
    pub fn types(&self) -> &filament_semantic_value::declaration::TypeEnvironment {
        match self {
            Self::Source(package) => package.graph().scope().types(),
            Self::Package(input) => input.types,
        }
    }

}
