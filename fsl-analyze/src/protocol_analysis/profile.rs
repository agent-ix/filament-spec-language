// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-575: profile admission for `bounded-static` and `bounded`.

use filament_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2};
use fsl_package::protocol_view::{
    clause_activation, control_body, control_operation, protocol_view, queue_capacity,
    role_is_replicated, ClauseActivation, ControlBody, ControlOperation, ParallelJoinPolicy,
    QueueCapacity, ViewRefusal,
};
use fsl_replay::protocol_analysis::{UnsupportedCause, UnsupportedSlug};

/// A strategy or projection profile V1 decides (FR-575).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// `bounded-static`: static roles, literal capacities, sequence, send,
    /// receive, choice and `parallel` with `join all` (PA-21).
    BoundedStatic,
    /// `bounded`: the bounded profile, including replicated role classes.
    Bounded,
}

impl Profile {
    /// The profile a request word names, `None` for any other word.
    #[ix_trace_rs::string_edge]
    #[must_use]
    pub fn from_wire(word: &str) -> Option<Self> {
        match word {
            "bounded-static" => Some(Self::BoundedStatic),
            "bounded" => Some(Self::Bounded),
            _ => None,
        }
    }
}

fn node<'p>(
    package: &'p CheckedPackageV2,
    id: &CheckedNodeId,
) -> Result<&'p CheckedSemanticNodeV2, ViewRefusal> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .ok_or_else(|| ViewRefusal::UnknownProtocol { node: id.clone() })
}

fn cause(slug: UnsupportedSlug, node: &CheckedNodeId, detail: Option<&str>) -> UnsupportedCause {
    UnsupportedCause {
        slug,
        node: node.clone(),
        detail: detail.map(ToOwned::to_owned),
    }
}

/// The first construct of the clause `protocol` outside the profile, in the
/// order activation, roles, queues, compensations, then the controls in
/// layout order (FR-575). `None` for a clause that stays inside it.
///
/// # Errors
///
/// [`ViewRefusal`] when a view function refuses.
pub fn unsupported_construct(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
) -> Result<Option<UnsupportedCause>, ViewRefusal> {
    unsupported_construct_for_profile(package, protocol, Profile::BoundedStatic)
}

/// The first construct outside `profile` in the order activation, roles,
/// queues, compensations, then controls in layout order.
///
/// # Errors
///
/// [`ViewRefusal`] when a view function refuses.
pub fn unsupported_construct_for_profile(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
    profile: Profile,
) -> Result<Option<UnsupportedCause>, ViewRefusal> {
    let view = protocol_view(package, protocol)?;
    // `bounded` carries instance activation through the arena and partitions
    // observations by instance; `bounded-static` has no instance dimension.
    if profile == Profile::BoundedStatic
        && clause_activation(node(package, view.clause())?)? == ClauseActivation::Each
    {
        return Ok(Some(cause(
            UnsupportedSlug::ActivationOnEach,
            view.clause(),
            None,
        )));
    }
    if profile == Profile::BoundedStatic {
        for role in view.roles() {
            if role_is_replicated(node(package, role)?)? {
                return Ok(Some(cause(
                    UnsupportedSlug::DynamicRolePopulation,
                    role,
                    None,
                )));
            }
        }
    }
    if profile == Profile::BoundedStatic {
        for queue in view.queues() {
            if let QueueCapacity::Symbolic(name) = queue_capacity(node(package, queue)?)? {
                return Ok(Some(cause(
                    UnsupportedSlug::SymbolicQueueCapacity,
                    queue,
                    Some(name),
                )));
            }
        }
    }
    if let Some(template) = view.compensations().first() {
        return Ok(Some(cause(UnsupportedSlug::Compensation, template, None)));
    }
    // The specific causes first, in layout order; a control the profile's
    // list does not include is reported only when nothing more specific is.
    let mut other = None;
    for id in view.controls() {
        let control = node(package, id)?;
        let found = match control_operation(control)? {
            ControlOperation::Repeat => Some(cause(UnsupportedSlug::Repeat, id, None)),
            ControlOperation::Await => Some(cause(UnsupportedSlug::TimedAwait, id, None)),
            ControlOperation::Compensate => Some(cause(UnsupportedSlug::Compensation, id, None)),
            ControlOperation::Check
            | ControlOperation::Attempt
            | ControlOperation::Effect
            | ControlOperation::Event
            | ControlOperation::Commit
            | ControlOperation::Fence => {
                other.get_or_insert_with(|| cause(UnsupportedSlug::UnsupportedControl, id, None));
                None
            }
            _ => match control_body(control)? {
                ControlBody::Parallel { join, .. }
                    if profile == Profile::BoundedStatic
                        && join.policy != ParallelJoinPolicy::All =>
                {
                    Some(cause(
                        UnsupportedSlug::UnsupportedJoin,
                        id,
                        Some(join.policy.as_str()),
                    ))
                }
                ControlBody::Parallel { join, .. }
                    if profile == Profile::BoundedStatic && join.outstanding.is_some() =>
                {
                    Some(cause(
                        UnsupportedSlug::UnsupportedJoin,
                        id,
                        Some("outstanding"),
                    ))
                }
                _ => None,
            },
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(other)
}
