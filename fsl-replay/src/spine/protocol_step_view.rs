// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-578: the linked-package entry of the protocol step view. It reads the
//! clause's dynamic constructs and step names from the linked package and
//! hands the protocol subject to the view in `refinement`.

use filament_contract_model::{CheckedNodeId, CheckedPackageV2};
use fsl_eval::model_subject::ProtocolSubject;
use fsl_package::protocol_view::{
    clause_activation, protocol_view, role_is_replicated, ClauseActivation, ViewRefusal,
};

use crate::protocol_analysis::ProtocolPremises;
use crate::refinement::{view_over, ProtocolStepView, StepViewRefusal, UnsteppedConstruct};

/// The constructs of the clause `protocol` that `fsl-eval` steps only up to
/// a bound: `activation on each`, then each replicated role.
fn dynamic_constructs(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
) -> Result<Vec<UnsteppedConstruct>, ViewRefusal> {
    let view = protocol_view(package, protocol)?;
    let node = |id: &CheckedNodeId| {
        package
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| &node.node_id == id)
            .ok_or_else(|| ViewRefusal::UnknownProtocol { node: id.clone() })
    };
    let mut found = Vec::new();
    if clause_activation(node(view.clause())?)? == ClauseActivation::Each {
        found.push(UnsteppedConstruct::ActivationOnEach(view.clause().clone()));
    }
    for role in view.roles() {
        if role_is_replicated(node(role)?)? {
            found.push(UnsteppedConstruct::ReplicatedRole((*role).clone()));
        }
    }
    Ok(found)
}

/// FR-578: the step view of the protocol clause `protocol` of the linked
/// package `package`, over the protocol subject `subject`, exploring at most
/// `max_states` states.
///
/// # Errors
///
/// [`StepViewRefusal`] when the protocol cannot be read, the subject does
/// not admit, an expansion stops, a state is undetermined or cut, a step's
/// node has no unique name, or the protocol reaches more than `max_states`
/// states.
pub fn protocol_step_view(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
    subject: &ProtocolSubject<'_>,
    max_states: usize,
) -> Result<ProtocolStepView, StepViewRefusal> {
    let unstepped = dynamic_constructs(package, protocol)?;
    let names = super::arena_names(package, protocol, &ProtocolPremises::default())?;
    view_over(unstepped, &names, subject, max_states)
}
