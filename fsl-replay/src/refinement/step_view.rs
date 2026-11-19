// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-578: a read-only view of a linked protocol's steps for a
//! protocol-level refinement decided outside FSL.
//!
//! The view is the protocol's reachable state graph under the one `fsl-eval`
//! protocol transition system, read through [`AbstractProtocolSystem`]: no
//! step rule is stated here. Each step keeps its `fsl-eval` transition, its
//! kind, its control's and channel's checked node ids and its binder record;
//! the view decides nothing about hiding, refinement or composition.
//!
//! A protocol the system steps only up to a bound is never a view: a
//! construct `fsl-eval` refuses to step, an `activation on each` clause and a
//! replicated role are returned as [`UnsteppedConstruct`]s with no steps, and
//! a state a method bound cut, or the caller's state limit, refuses the view.

use std::collections::{BTreeMap, VecDeque};

use filament_contract_model::CheckedNodeId;
use fsl_eval::model_subject::ProtocolSubject;
use fsl_eval::simulation::model::{ModelFinding, ModelSystem, TerminalClass};
use fsl_eval::simulation::protocol::{
    ProtocolFairnessError, ProtocolRefusal, ProtocolTransition, StepKind, Unsupported,
};
use fsl_eval::simulation::{EncodingRefusal, ExpansionStop};
use fsl_package::protocol_view::ViewRefusal;
use fsl_semantics::model::observation::AdmissionFailure;

use super::protocol_system::{AbstractProtocolSystem, ProtocolAssumptionFairness};
use crate::protocol_analysis::arena::ArenaNames;

/// A state of a [`ProtocolSteps`] view, by its index in discovery order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateIndex(pub usize);

/// One labelled step of a protocol view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolStep {
    /// The state it leaves.
    pub from: StateIndex,
    /// The state it reaches.
    pub to: StateIndex,
    /// Its kind.
    pub kind: StepKind,
    /// The checked node of the control it steps, or of the clause's
    /// `finish` member for a `finish` step; `None` for a step of no
    /// control (a memory step, a `spawn`, a `retire`, an `activate`, a
    /// compensation step).
    pub node: Option<CheckedNodeId>,
    /// For a `send` or a `receive`, the checked node of its channel.
    pub channel: Option<CheckedNodeId>,
    /// Its binder record in FR-181's typed canonical bytes; `None` for a
    /// step with no record binder.
    pub record: Option<Vec<u8>>,
    /// The exact `fsl-eval` transition, for replay.
    pub transition: ProtocolTransition,
}

/// The reachable state graph of a protocol `fsl-eval` steps exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSteps {
    /// The number of states; every [`StateIndex`] is below it.
    pub states: usize,
    /// The initial states, ascending.
    pub initial: Vec<StateIndex>,
    /// The states with no step that a terminal declaration intends,
    /// ascending. A state with no step outside this list is a deadlock.
    pub final_states: Vec<StateIndex>,
    /// Every step, by source state, then in `fsl-eval`'s canonical
    /// successor order.
    pub steps: Vec<ProtocolStep>,
    /// The assumptions the protocol's behaviour rests on: its derived
    /// scheduler and memory fairness and the operation fairness of each
    /// directly required temporal clause (FR-148).
    pub assumptions: Vec<ProtocolAssumptionFairness>,
}

/// A construct the view does not step exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsteppedConstruct {
    /// The protocol clause is activated `on each` trigger: `fsl-eval` steps
    /// its instances only up to the live-instance bound.
    ActivationOnEach(CheckedNodeId),
    /// The role has a replicated population, which `fsl-eval` steps only
    /// over the request's finite universe.
    ReplicatedRole(CheckedNodeId),
    /// A construct `fsl-eval` refuses to step.
    Construct(Unsupported),
    /// Another `unsupported_construct` refusal of the protocol system: a
    /// weak-access shape, a recovery view or a node footprint.
    Refused(ProtocolRefusal),
}

/// A linked protocol's step view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolStepView {
    /// Every reachable state and step.
    Steps(ProtocolSteps),
    /// The constructs that keep the view from being exact, in the order
    /// activation, roles in view order, then the `fsl-eval` refusal. No
    /// step is returned.
    Unsupported(Vec<UnsteppedConstruct>),
}

/// Why no view was built.
#[derive(Debug, thiserror::Error)]
pub enum StepViewRefusal {
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// The subject's model system did not admit.
    #[error("the protocol subject's model did not admit")]
    Admission(Box<AdmissionFailure>),
    /// The protocol system refused the subject for a reason other than an
    /// unsupported construct.
    #[error(transparent)]
    Protocol(ProtocolRefusal),
    /// An expansion stopped: a budget ran out or an evaluation broke.
    #[error("the protocol expansion stopped: {}/{}", .0.cause.code(), .0.cause.cause())]
    Stopped(ExpansionStop),
    /// A reached state's findings or terminal class are undetermined.
    #[error("state {0:?} is undetermined")]
    Undetermined(StateIndex),
    /// The memory bound or the live-instance bound left a state unexpanded.
    #[error("a method bound cut state {0:?}")]
    Cut(StateIndex),
    /// The protocol reaches more than the caller's state limit.
    #[error("the protocol reaches more than {max_states} states")]
    StateLimit {
        /// The limit.
        max_states: usize,
    },
    /// A step names a control or channel the linked package does not hold
    /// under exactly one name.
    #[error("the step at `{0}` names no node of the linked package")]
    UnnamedNode(String),
    /// A state has no canonical key.
    #[error("a state has no canonical key: {0:?}")]
    Encoding(EncodingRefusal),
    /// The protocol's assumptions did not resolve.
    #[error(transparent)]
    Fairness(#[from] ProtocolFairnessError),
    /// A state is not a state of the admitted system.
    #[error("a protocol state belongs to no admitted system")]
    Fault,
}

impl From<ExpansionStop> for StepViewRefusal {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

/// FR-578: the step view over the protocol subject `subject`, whose clause's
/// steps resolve to the linked package's nodes through `names`
/// ([`crate::spine::arena_names`]), after the dynamic constructs
/// `unstepped` the linked package holds, exploring at most `max_states`
/// states. The linked-package entry is `fsl_analyze`'s
/// `protocol_analysis::protocol_step_view`.
///
/// # Errors
///
/// [`StepViewRefusal`] when the subject does not admit, an expansion stops,
/// a state is undetermined or cut, a step's node has no unique name, or the
/// protocol reaches more than `max_states` states.
pub fn step_view_over(
    mut unstepped: Vec<UnsteppedConstruct>,
    names: &ArenaNames,
    subject: &ProtocolSubject<'_>,
    max_states: usize,
) -> Result<ProtocolStepView, StepViewRefusal> {
    let model = ModelSystem::new(subject.model.simulation.clone())
        .map_err(|failure| StepViewRefusal::Admission(Box::new(failure)))?;
    let system = match AbstractProtocolSystem::new(model, subject) {
        Ok(system) => system,
        Err(ProtocolRefusal::Unsupported(construct)) => {
            unstepped.push(UnsteppedConstruct::Construct(construct));
            return Ok(ProtocolStepView::Unsupported(unstepped));
        }
        Err(refusal)
            if refusal.code() == fsl_foundation::diagnostic::Code::UnsupportedConstruct =>
        {
            unstepped.push(UnsteppedConstruct::Refused(refusal));
            return Ok(ProtocolStepView::Unsupported(unstepped));
        }
        // A replicated role the subject gives no population is still the
        // dynamic construct, not a request fault.
        Err(_) if !unstepped.is_empty() => return Ok(ProtocolStepView::Unsupported(unstepped)),
        Err(refusal) => return Err(StepViewRefusal::Protocol(refusal)),
    };
    if !unstepped.is_empty() {
        return Ok(ProtocolStepView::Unsupported(unstepped));
    }
    explore(&system, names, max_states).map(ProtocolStepView::Steps)
}

fn explore(
    system: &AbstractProtocolSystem<'_>,
    names: &ArenaNames,
    max_states: usize,
) -> Result<ProtocolSteps, StepViewRefusal> {
    let mut index: BTreeMap<Vec<u8>, StateIndex> = BTreeMap::new();
    let mut pending = VecDeque::new();
    let mut admit = |state, pending: &mut VecDeque<_>| -> Result<StateIndex, StepViewRefusal> {
        let (key, _) = system
            .state_key(&state)
            .map_err(StepViewRefusal::Encoding)?;
        if let Some(found) = index.get(&key) {
            return Ok(*found);
        }
        if index.len() == max_states {
            return Err(StepViewRefusal::StateLimit { max_states });
        }
        let at = StateIndex(index.len());
        index.insert(key, at);
        pending.push_back((at, state));
        Ok(at)
    };
    let mut initial = Vec::new();
    for state in system.initial() {
        initial.push(admit(state, &mut pending)?);
    }
    initial.sort();
    initial.dedup();
    let mut final_states = Vec::new();
    let mut steps = Vec::new();
    // States are expanded in discovery order, so the steps are appended in
    // ascending source index.
    while let Some((at, state)) = pending.pop_front() {
        let ((expansion, terminal), cut) = system
            .successors_cut_and_terminal(&state)?
            .ok_or(StepViewRefusal::Fault)?;
        if cut.memory || cut.instances {
            return Err(StepViewRefusal::Cut(at));
        }
        if expansion.findings.iter().any(undetermined) {
            return Err(StepViewRefusal::Undetermined(at));
        }
        match terminal {
            TerminalClass::Intended => final_states.push(at),
            TerminalClass::NotTerminal | TerminalClass::Deadlocked => {}
            TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                return Err(StepViewRefusal::Undetermined(at))
            }
        }
        for (transition, successor) in expansion.successors {
            let to = admit(successor, &mut pending)?;
            steps.push(step(system, names, at, to, transition)?);
        }
    }
    Ok(ProtocolSteps {
        states: index.len(),
        initial,
        final_states,
        steps,
        assumptions: system.assumption_fairness()?,
    })
}

/// Whether `finding` leaves its state's behaviour undetermined: an
/// undecided evaluation, or a weak-access shape `fsl-eval` reports with no
/// successor. A violated invariant is decided and leaves the state served.
fn undetermined(finding: &ModelFinding) -> bool {
    match finding {
        ModelFinding::InvariantUndetermined { .. }
        | ModelFinding::ProtocolExpressionUndetermined { .. }
        | ModelFinding::ContractUndetermined { .. }
        | ModelFinding::WeakAccessShape(_) => true,
        ModelFinding::InvariantViolated { .. }
        | ModelFinding::WeakAccessDeferred { .. }
        | ModelFinding::WeakAccessShared { .. } => false,
    }
}

fn step(
    system: &AbstractProtocolSystem<'_>,
    names: &ArenaNames,
    from: StateIndex,
    to: StateIndex,
    transition: ProtocolTransition,
) -> Result<ProtocolStep, StepViewRefusal> {
    let unnamed = |what: &str| StepViewRefusal::UnnamedNode(what.to_owned());
    // A `finish` step names the clause's `finish` member, not a control.
    let node = match (transition.kind(), transition.node()) {
        (StepKind::Finish, _) => Some(names.finish().clone()),
        (_, Some(path)) => Some(names.node(path).map_err(|_| unnamed(path))?.clone()),
        (_, None) => None,
    };
    let channel = match transition.kind() {
        StepKind::Send => transition.send_record().map(|record| record.channel()),
        StepKind::Receive => transition.received().map(|(channel, _, _)| channel),
        _ => None,
    };
    let channel = match channel {
        Some(qualified) => Some(
            names
                .channel_node(qualified)
                .map_err(|_| unnamed(qualified))?
                .clone(),
        ),
        None => None,
    };
    Ok(ProtocolStep {
        from,
        to,
        kind: transition.kind(),
        node,
        channel,
        record: system.binder_record_bytes(&transition)?,
        transition,
    })
}
