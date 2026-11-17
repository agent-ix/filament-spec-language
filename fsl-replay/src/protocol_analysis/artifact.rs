// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-24, PA-25: the projection artifacts, the FR-351 record a
//! decisive counterexample carries, and the counterexample replay walk
//! (PA-20, ADR-018 SM-7).

use std::collections::BTreeMap;

use filament_contract_model::CheckedNodeId;
use filament_exact::ObjectReference;
use fsl_package::protocol_view::PortRef;

use super::{
    node_text, Action, BranchMerge, ChoiceBasis, GlobalStructure, MissingMember,
    ProtocolAnalysisKey,
};
use crate::identity::TracePosition;
use crate::result::{DecidingElement, SeparatingWitnessRecord};
use crate::{ObservationIdentity, RuntimeValuePath, ValuePathSubject};

/// The index of a local event within its graph.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LocalEventId(pub u32);

/// What a local event is (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalEventKind {
    /// A send to `peer`.
    Send {
        /// The receiving role.
        peer: CheckedNodeId,
        /// The message type.
        message_type: CheckedNodeId,
    },
    /// A receive from `peer`.
    Receive {
        /// The sending role.
        peer: CheckedNodeId,
        /// The message type.
        message_type: CheckedNodeId,
    },
    /// A choice the role owns.
    InternalChoice,
    /// A choice the role learns the outcome of.
    ExternalChoice,
    /// The entry of a `parallel` region.
    ParallelRegion,
    /// The exit of a `parallel` region.
    ParallelJoin,
}

/// One event of a local graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEvent {
    /// The event's identity in its graph.
    pub id: LocalEventId,
    /// What it is.
    pub kind: LocalEventKind,
}

/// A role's local graph: its events and the covering edges of its induced
/// order (PA-9).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalGraph {
    /// The events, ascending by id.
    pub events: Vec<LocalEvent>,
    /// The covering edges `(earlier, later)`, ascending.
    pub edges: Vec<(LocalEventId, LocalEventId)>,
    /// Pairs of events that are one behaviour of the role seen in two
    /// branches of a choice it merged (PA-10): alternatives, not concurrent
    /// events. Ascending.
    pub alternatives: Vec<(LocalEventId, LocalEventId)>,
}

/// The total map from each global event of the role to its local event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Correspondence(pub BTreeMap<PortRef, LocalEventId>);

/// How the role stands at one choice, and what merged there (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeEvidence {
    /// The choice event.
    pub choice: PortRef,
    /// The basis.
    pub basis: ChoiceBasis,
    /// The merged pairs of global events, for a [`ChoiceBasis::MustMerge`].
    pub merged: Vec<(PortRef, PortRef)>,
}

/// A sound projection (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionWitness {
    /// The structural key.
    pub key: ProtocolAnalysisKey,
    /// The role's local graph.
    pub local_graph: LocalGraph,
    /// The correspondence table.
    pub correspondence: Correspondence,
    /// One entry per choice, ascending.
    pub merge_evidence: Vec<MergeEvidence>,
}

/// The owner of a projection decision point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionOwner {
    /// A protocol role owns the decision.
    Role(CheckedNodeId),
    /// The environment owns the decision.
    Environment,
}

/// A projection decision point that distinguishes a truthful counterexample
/// (FR-890-AC-5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionDecision {
    /// A choice whose branches do not merge for the projected role.
    Choice {
        /// The choice node.
        choice: CheckedNodeId,
        /// The choice owner.
        owner: DecisionOwner,
        /// The two case body nodes.
        cases: [CheckedNodeId; 2],
        /// Paths from the choice to the divergent actions.
        paths: [Vec<PortRef>; 2],
        /// The divergent actions, or `None` for branch end.
        actions: [Option<Action>; 2],
    },
    /// A replicated role instance is scheduled to act before receiving a
    /// message that pins this event to that instance.
    ActingInstance {
        /// The first send event on a possible instance thread.
        event: PortRef,
        /// Scheduling an acting instance belongs to the environment.
        owner: DecisionOwner,
        /// The two distinct live instances the environment may schedule.
        instances: [u64; 2],
        /// The authored role maximum permitting these instances.
        maximum: u64,
    },
}

/// A minimal counterexample (PA-11, PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCounterexample {
    /// The structural key.
    pub key: ProtocolAnalysisKey,
    /// The projected role.
    pub role: CheckedNodeId,
    /// The decision and its two divergent ways.
    pub decision: ProjectionDecision,
    /// The FR-351 record (PA-24).
    pub separating_witness: SeparatingWitnessRecord,
}

impl ProjectionCounterexample {
    /// The length PA-11 minimises: the sum of the two paths' event counts.
    #[must_use]
    pub fn length(&self) -> usize {
        match &self.decision {
            ProjectionDecision::Choice { paths, .. } => paths[0].len() + paths[1].len(),
            ProjectionDecision::ActingInstance { .. } => 1,
        }
    }

    /// The choice node when this is a choice counterexample.
    #[must_use]
    pub fn choice(&self) -> Option<&CheckedNodeId> {
        match &self.decision {
            ProjectionDecision::Choice { choice, .. } => Some(choice),
            ProjectionDecision::ActingInstance { .. } => None,
        }
    }

    /// The choice owner when this is a choice counterexample.
    #[must_use]
    pub fn owner(&self) -> Option<&DecisionOwner> {
        match &self.decision {
            ProjectionDecision::Choice { owner, .. } => Some(owner),
            ProjectionDecision::ActingInstance { .. } => None,
        }
    }

    /// The indexed choice case, path and action when this is a choice counterexample.
    #[must_use]
    pub fn choice_side(
        &self,
        side: usize,
    ) -> Option<(&CheckedNodeId, &[PortRef], Option<&Action>)> {
        match &self.decision {
            ProjectionDecision::Choice {
                cases,
                paths,
                actions,
                ..
            } => Some((
                cases.get(side)?,
                paths.get(side)?.as_slice(),
                actions.get(side)?.as_ref(),
            )),
            ProjectionDecision::ActingInstance { .. } => None,
        }
    }
}

/// The claim's assessed-subject root, the one input of the FR-351 record the
/// engine does not derive (FR-574). It is not a key member.
#[derive(Clone, Debug, Default)]
pub struct AssessedRootDraft {
    /// The observation identity.
    pub observation: Option<ObservationIdentity>,
    /// The FR-204 declared object identity.
    pub object: Option<ObjectReference>,
}

/// An assessed-subject root with every component present.
#[derive(Clone, Debug)]
pub struct AssessedRoot {
    observation: ObservationIdentity,
    object: ObjectReference,
}

impl AssessedRoot {
    /// Build the root, refusing the first absent component.
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming `assessed_root`, `assessed_root.observation`
    /// or `assessed_root.object`.
    pub fn new(draft: Option<AssessedRootDraft>) -> Result<Self, MissingMember> {
        let draft = draft.ok_or_else(|| MissingMember::new("assessed_root"))?;
        Ok(Self {
            observation: draft
                .observation
                .ok_or_else(|| MissingMember::new("assessed_root.observation"))?,
            object: draft
                .object
                .ok_or_else(|| MissingMember::new("assessed_root.object"))?,
        })
    }

    /// The FR-351 record of a decisive projection counterexample: the
    /// divergent action's node as `deciding_element`, the FR-207 path of the
    /// root's object with no steps, and the decimal `trace_position`.
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming `deciding_element` when the node does not
    /// encode.
    pub fn record(
        &self,
        deciding: &CheckedNodeId,
        trace_position: usize,
    ) -> Result<SeparatingWitnessRecord, MissingMember> {
        let text = node_text(deciding).map_err(|_| MissingMember::new("deciding_element"))?;
        Ok(SeparatingWitnessRecord {
            quantifier: None,
            deciding_element: DecidingElement::Protocol(text),
            index: None,
            value_path: RuntimeValuePath {
                observation: self.observation.clone(),
                subject: ValuePathSubject::Object(self.object.clone()),
                steps: Vec::new(),
            },
            trace_position: Some(TracePosition::new(trace_position.to_string())),
        })
    }
}

/// The evidence a `ReplaySource::ProtocolAnalysis` request carries: the
/// counterexample to re-walk (PA-20).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAnalysisEvidence {
    /// The counterexample.
    pub counterexample: ProjectionCounterexample,
}

impl ProtocolAnalysisEvidence {
    /// The evidence's measured encoded size.
    #[must_use]
    pub fn measured_bytes(&self) -> usize {
        let ce = &self.counterexample;
        let paths = match &ce.decision {
            ProjectionDecision::Choice { paths, .. } => {
                (paths[0].len() + paths[1].len()) * std::mem::size_of::<PortRef>()
            }
            ProjectionDecision::ActingInstance { .. } => 0,
        };
        std::mem::size_of_val(ce) + paths + ce.separating_witness.measured_bytes()
    }
}

/// Why a counterexample's replay disagrees with the checked graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalkDisagreement {
    /// The named choice is no choice of the graph.
    UnknownChoice,
    /// The role is told the outcome (or owns the choice), so nothing need
    /// merge there.
    NotMustMerge,
    /// A step is not an admitted successor of its predecessor.
    StepNotSuccessor(PortRef),
    /// A path does not end at its stated action, or the action is not the
    /// role's.
    ActionMismatch,
    /// A path leaves the case it is stated for.
    WrongBranch,
    /// The two cases merge, or diverge elsewhere than stated.
    NotTheStatedDivergence,
    /// A branch holds events beyond the horizon.
    Cut,
}

/// The replay walk of a projection counterexample: it re-walks both paths
/// against the structure, checks each step is an admitted successor, and
/// that the walk ends in the stated divergence (PA-20).
///
/// # Errors
///
/// [`WalkDisagreement`] naming what disagrees.
pub fn replay_projection_counterexample(
    structure: &GlobalStructure,
    counterexample: &ProjectionCounterexample,
    max_depth: u64,
) -> Result<(), WalkDisagreement> {
    let ce = counterexample;
    match &ce.decision {
        ProjectionDecision::Choice {
            choice,
            owner: DecisionOwner::Role(_),
            cases,
            paths,
            actions,
        } => {
            let choice_port = PortRef {
                node: choice.clone(),
                port: fsl_package::protocol_view::Port::Enter,
            };
            match structure.classify(&ce.role, &choice_port) {
                None => return Err(WalkDisagreement::UnknownChoice),
                Some(ChoiceBasis::MustMerge) => {}
                Some(_) => return Err(WalkDisagreement::NotMustMerge),
            }
            for (path, action, case) in [
                (&paths[0], &actions[0], &cases[0]),
                (&paths[1], &actions[1], &cases[1]),
            ] {
                let mut previous = choice_port.clone();
                for step in path {
                    if !structure.successors(&previous).contains(&step) {
                        return Err(WalkDisagreement::StepNotSuccessor(step.clone()));
                    }
                    let inside = structure
                        .event(step)
                        .is_some_and(|event| event.within.contains(case));
                    if !inside {
                        return Err(WalkDisagreement::WrongBranch);
                    }
                    previous = step.clone();
                }
                match action {
                    Some(action) => {
                        if path.last() != Some(&action.at)
                            || structure.action_of(&action.at, &ce.role).as_ref() != Some(action)
                        {
                            return Err(WalkDisagreement::ActionMismatch);
                        }
                    }
                    None if !path.is_empty() => return Err(WalkDisagreement::ActionMismatch),
                    None => {}
                }
            }
            let mut unlimited = u64::MAX;
            match structure.merge(&ce.role, &cases[0], &cases[1], max_depth, &mut unlimited) {
                BranchMerge::Cut | BranchMerge::Exhausted => Err(WalkDisagreement::Cut),
                BranchMerge::Merged(_) => Err(WalkDisagreement::NotTheStatedDivergence),
                BranchMerge::Diverged(found) => {
                    if found.iter().any(|divergence| {
                        (&divergence.first, &divergence.second) == (&actions[0], &actions[1])
                    }) {
                        Ok(())
                    } else {
                        Err(WalkDisagreement::NotTheStatedDivergence)
                    }
                }
            }
        }
        ProjectionDecision::Choice { .. } => Err(WalkDisagreement::NotMustMerge),
        ProjectionDecision::ActingInstance {
            event,
            owner: DecisionOwner::Environment,
            instances,
            maximum,
        } => {
            if maximum < 2
                || structure
                    .depth(event)
                    .map_or(true, |depth| depth > max_depth)
                || structure.role_maximum(&ce.role) != Some(*maximum)
                || instances.iter().any(|instance| *instance >= *maximum)
                || instances[0] == instances[1]
                || structure
                    .action_of(event, &ce.role)
                    .map_or(true, |action| action.direction != super::Direction::Send)
            {
                return Err(WalkDisagreement::ActionMismatch);
            }
            let has_prior_role_event = structure.events().iter().any(|candidate| {
                candidate.at != *event
                    && structure.action_of(&candidate.at, &ce.role).is_some()
                    && structure.precedes(&candidate.at, event)
            });
            if has_prior_role_event {
                return Err(WalkDisagreement::NotTheStatedDivergence);
            }
            Ok(())
        }
        ProjectionDecision::ActingInstance { .. } => Err(WalkDisagreement::ActionMismatch),
    }
}

/// How a counterexample replay settled (ADR-018 SM-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolReplaySettlement {
    /// The walk reproduced the stated divergence.
    Reproduced,
    /// The walk disagrees with the checked graph.
    Inconclusive(WalkDisagreement),
}

/// The result of the `ReplaySource::ProtocolAnalysis` arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolReplayResult {
    /// The settlement.
    pub settlement: ProtocolReplaySettlement,
}

impl ProtocolReplayResult {
    /// The category the result carries: a reproduced walk is a success, a
    /// disagreeing one `inconclusive`.
    #[must_use]
    pub fn category(&self) -> filament_semantic_value::catalog::Category {
        use filament_semantic_value::catalog::Category;
        match self.settlement {
            ProtocolReplaySettlement::Reproduced => Category::Success,
            ProtocolReplaySettlement::Inconclusive(_) => Category::Inconclusive,
        }
    }
}
