// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-570: project a checked global protocol onto one role.

use std::collections::BTreeMap;

use filament_contract_model::{CheckedNodeId, CheckedPackageV2};
use fsl_package::protocol_view::{
    clause_activation_trigger, protocol_view, role_replication_maximum, PortRef, ViewRefusal,
};
use fsl_replay::protocol_analysis::{
    node_text, replay_projection_counterexample, Action, AnalysisOutcome, AssessedRoot,
    AssessedRootDraft, BranchMerge, ChoiceBasis, Correspondence, DecisionOwner, GlobalEventKind,
    GlobalStructure, InconclusiveCause, LocalEvent, LocalEventId, LocalEventKind, LocalGraph,
    MergeEvidence, MissingMember, PremiseRefusal, ProjectionBoundsDraft, ProjectionCounterexample,
    ProjectionDecision, ProjectionKeyDraft, ProjectionWitness, ProtocolAnalysisKey,
    ProtocolAnalysisResult, ProtocolPremises, RunBudget, RunLimit,
};
use fsl_replay::spine::StructureError;
use fsl_replay::ModelCheckLimit;

use super::profile::{unsupported_construct_for_profile, Profile};

/// A projection request (PA-5). Key members are optional so that the key
/// builder names the first absent one (PA-13); premises (a) and (b) are
/// read from the package (PA-6).
#[derive(Clone, Debug)]
pub struct ProjectionRequest {
    /// The `protocol_clause` node.
    pub protocol: CheckedNodeId,
    /// The FR-288 qualified definition key.
    pub definition_key: Option<String>,
    /// The role node to project onto.
    pub role: CheckedNodeId,
    /// The profile word.
    pub profile: Option<String>,
    /// The bounds.
    pub bounds: Option<ProjectionBoundsDraft>,
    /// The claim's assessed-subject root (FR-574); not a key member.
    pub assessed_root: Option<AssessedRootDraft>,
    /// The run limits (PA-14); not key members.
    pub budget: RunBudget,
}

/// A request the engine refuses before any analysis.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProjectionRefusal {
    /// A key member or the assessed-subject root is absent.
    #[error(transparent)]
    Missing(#[from] MissingMember),
    /// The profile word is not a supported profile.
    #[error(
        "invalid_runtime_input/invalid-value: the member `profile` is `{0}`, not `bounded` or `bounded-static`"
    )]
    InvalidProfile(String),
    /// The role is not a role of the clause.
    #[error("invalid_runtime_input/invalid-value: the member `role` names no role of the clause")]
    UnknownRole(CheckedNodeId),
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// Premises (a) and (b) were not built.
    #[error(transparent)]
    Premises(#[from] PremiseRefusal),
    /// The structure contradicts itself: an engine fault, never a result.
    #[error("the protocol structure is inconsistent: {0}")]
    Inconsistent(&'static str),
    /// The structure did not build for a reason other than a run limit.
    #[error(transparent)]
    Structure(StructureError),
}

/// The settlement of one projection (PA-20).
pub type ProjectionOutcome = ProtocolAnalysisResult<ProjectionWitness, ProjectionCounterexample>;

fn result(
    key: ProtocolAnalysisKey,
    outcome: AnalysisOutcome<ProjectionWitness, ProjectionCounterexample>,
) -> ProjectionOutcome {
    ProtocolAnalysisResult { key, outcome }
}

/// Project the protocol onto a role (FR-570).
///
/// # Errors
///
/// [`ProjectionRefusal`] for a request the engine refuses at admission.
pub fn project(
    package: &CheckedPackageV2,
    request: ProjectionRequest,
) -> Result<ProjectionOutcome, ProjectionRefusal> {
    let premises = ProtocolPremises::from_view(package, &request.protocol)?;
    let role_text = node_text(&request.role).map_err(|_| MissingMember::new("role"))?;
    let key = ProtocolAnalysisKey::projection(ProjectionKeyDraft {
        definition_key: request.definition_key,
        role: Some(role_text),
        profile: request.profile.clone(),
        channels: Some(premises.channels),
        choices: Some(premises.choices),
        bounds: request.bounds,
    })?;
    let ProtocolAnalysisKey::Projection(held) = &key else {
        unreachable!("the projection builder returns a projection key");
    };
    let max_depth = held.bounds.max_depth;
    let profile = request.profile.as_deref().and_then(Profile::from_wire);
    let Some(profile) = profile else {
        return Err(ProjectionRefusal::InvalidProfile(
            request.profile.unwrap_or_default(),
        ));
    };
    let root = AssessedRoot::new(request.assessed_root)?;
    let view = protocol_view(package, &request.protocol)?;
    if !view.roles().iter().any(|role| **role == request.role) {
        return Err(ProjectionRefusal::UnknownRole(request.role));
    }
    if let Some(cause) = unsupported_construct_for_profile(package, &request.protocol, profile)? {
        return Ok(result(key, AnalysisOutcome::Unsupported(cause)));
    }
    let replication_maximum = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == request.role)
        .map(role_replication_maximum)
        .transpose()?
        .flatten();
    let structure = match GlobalStructure::from_package(package, &request.protocol, request.budget)
    {
        Ok(structure) => structure,
        Err(StructureError::Limit(limit)) => {
            return Ok(result(key, AnalysisOutcome::Failed(limit)));
        }
        Err(other) => return Err(ProjectionRefusal::Structure(other)),
    };
    let role = &request.role;

    if replication_maximum.is_some_and(|maximum| maximum >= 2) {
        let role_actions: Vec<_> = structure
            .events()
            .iter()
            .filter_map(|event| {
                structure
                    .action_of(&event.at, role)
                    .map(|action| (event, action))
            })
            .collect();
        if let Some((event, _)) = role_actions.iter().find(|(candidate, action)| {
            action.direction == fsl_replay::protocol_analysis::Direction::Send
                && structure
                    .depth(&candidate.at)
                    .is_some_and(|depth| depth <= max_depth)
                && !role_actions.iter().any(|(prior, _)| {
                    prior.at != candidate.at && structure.precedes(&prior.at, &candidate.at)
                })
        }) {
            let position = structure
                .depth(&event.at)
                .map(|depth| usize::try_from(depth.saturating_sub(1)).unwrap_or(usize::MAX))
                .unwrap_or_default();
            let separating_witness = root.record(&event.at.node, position)?;
            return Ok(result(
                key.clone(),
                AnalysisOutcome::Refuted(ProjectionCounterexample {
                    key,
                    role: role.clone(),
                    decision: ProjectionDecision::ActingInstance {
                        event: event.at.clone(),
                        owner: DecisionOwner::Environment,
                        instances: [0, 1],
                        maximum: replication_maximum.unwrap_or(0),
                    },
                    separating_witness,
                }),
            ));
        }
    }

    let mut cut = structure.is_cut(max_depth);
    // Each comparison of two runs is a transition against the run limit,
    // beyond the covering edges the structure already holds.
    let mut merge_calls = request
        .budget
        .max_transitions
        .saturating_sub(structure.transition_count());
    let mut evidence = Vec::new();
    let mut best: Option<Candidate> = None;
    for event in structure.events() {
        let GlobalEventKind::Choice { cases, .. } = &event.kind else {
            continue;
        };
        if structure
            .depth(&event.at)
            .is_some_and(|depth| depth > max_depth)
        {
            continue;
        }
        let Some(basis) = structure.classify(role, &event.at) else {
            continue;
        };
        let mut merged = Vec::new();
        if basis == ChoiceBasis::MustMerge {
            for (first, (_, case_first)) in cases.iter().enumerate() {
                for (_, case_second) in &cases[first + 1..] {
                    match structure.merge(
                        role,
                        case_first,
                        case_second,
                        max_depth,
                        &mut merge_calls,
                    ) {
                        BranchMerge::Merged(pairs) => merged.extend(pairs),
                        BranchMerge::Cut => cut = true,
                        BranchMerge::Exhausted => {
                            return Ok(result(
                                key,
                                AnalysisOutcome::Failed(RunLimit {
                                    limit: ModelCheckLimit::MaxTransitions,
                                    value: request.budget.max_transitions,
                                }),
                            ));
                        }
                        BranchMerge::Diverged(divergences) => {
                            for divergence in divergences {
                                let side = |action: Option<Action>, case: &CheckedNodeId| {
                                    let path = match &action {
                                        Some(action) => structure
                                            .shortest_path(&event.at, &action.at)
                                            .ok_or_else(|| {
                                                ProjectionRefusal::Inconsistent(
                                                    "a divergent action does not follow its choice",
                                                )
                                            })?,
                                        None => Vec::new(),
                                    };
                                    Ok::<_, ProjectionRefusal>(Side {
                                        action,
                                        case: case.clone(),
                                        path,
                                    })
                                };
                                let candidate = Candidate::new(
                                    event.at.clone(),
                                    side(divergence.first, case_first)?,
                                    side(divergence.second, case_second)?,
                                );
                                if best
                                    .as_ref()
                                    .is_none_or(|held| candidate.rank() < held.rank())
                                {
                                    best = Some(candidate);
                                }
                            }
                        }
                    }
                }
            }
        }
        merged.sort();
        merged.dedup();
        evidence.push(MergeEvidence {
            choice: event.at.clone(),
            basis,
            merged,
        });
    }

    if let Some(candidate) = best {
        let counterexample = candidate.finish(&structure, key.clone(), role, &root)?;
        let outcome = match replay_projection_counterexample(&structure, &counterexample, max_depth)
        {
            Ok(()) => AnalysisOutcome::Refuted(counterexample),
            Err(_) => AnalysisOutcome::Inconclusive(InconclusiveCause::ReplayDisagreed),
        };
        return Ok(result(key, outcome));
    }
    if cut {
        return Ok(result(
            key,
            AnalysisOutcome::Inconclusive(InconclusiveCause::Horizon),
        ));
    }
    let activation = clause_activation_trigger(package, &request.protocol)?;
    match witness(
        &structure,
        key.clone(),
        role,
        request.budget,
        evidence,
        activation,
    ) {
        Ok(witness) => Ok(result(key, AnalysisOutcome::Proved(witness))),
        Err(limit) => Ok(result(key, AnalysisOutcome::Failed(limit))),
    }
}

struct Side {
    action: Option<Action>,
    case: CheckedNodeId,
    path: Vec<PortRef>,
}

struct Candidate {
    choice: PortRef,
    a: Side,
    b: Side,
}

impl Candidate {
    /// Path A is the path to an action when the other side has ended, else
    /// the canonically first path (FR-570).
    fn new(choice: PortRef, first: Side, second: Side) -> Self {
        let swap = match (&first.action, &second.action) {
            (None, Some(_)) => true,
            (Some(_), None) => false,
            _ => second.path < first.path,
        };
        if swap {
            Self {
                choice,
                a: second,
                b: first,
            }
        } else {
            Self {
                choice,
                a: first,
                b: second,
            }
        }
    }

    /// PA-11: length, then choice node id, then the two paths.
    fn rank(&self) -> (usize, &PortRef, &Vec<PortRef>, &Vec<PortRef>) {
        (
            self.a.path.len() + self.b.path.len(),
            &self.choice,
            &self.a.path,
            &self.b.path,
        )
    }

    fn finish(
        self,
        structure: &GlobalStructure,
        key: ProtocolAnalysisKey,
        role: &CheckedNodeId,
        root: &AssessedRoot,
    ) -> Result<ProjectionCounterexample, ProjectionRefusal> {
        let owner = match structure.event(&self.choice).map(|event| &event.kind) {
            Some(GlobalEventKind::Choice { owner, .. }) => DecisionOwner::Role(owner.clone()),
            _ => DecisionOwner::Environment,
        };
        let deciding = self
            .a
            .action
            .as_ref()
            .map_or_else(|| self.choice.node.clone(), |action| action.at.node.clone());
        let separating_witness = root.record(&deciding, self.a.path.len().saturating_sub(1))?;
        Ok(ProjectionCounterexample {
            key,
            role: role.clone(),
            decision: ProjectionDecision::Choice {
                choice: self.choice.node,
                owner,
                cases: [self.a.case, self.b.case],
                paths: [self.a.path, self.b.path],
                actions: [self.a.action, self.b.action],
            },
            separating_witness,
        })
    }
}

fn witness(
    structure: &GlobalStructure,
    key: ProtocolAnalysisKey,
    role: &CheckedNodeId,
    budget: RunBudget,
    merge_evidence: Vec<MergeEvidence>,
    activation: Option<CheckedNodeId>,
) -> Result<ProjectionWitness, RunLimit> {
    let mut kept = structure.role_events(role);
    kept.sort_by(|left, right| {
        structure
            .depth(&left.at)
            .cmp(&structure.depth(&right.at))
            .then_with(|| left.at.cmp(&right.at))
    });
    let mut correspondence = BTreeMap::new();
    let mut events = Vec::new();
    for (position, event) in kept.iter().enumerate() {
        let id = LocalEventId(u32::try_from(position).unwrap_or(u32::MAX));
        let kind = match &event.kind {
            GlobalEventKind::Send(_) | GlobalEventKind::Receive(_) => {
                let action = structure.action_of(&event.at, role);
                match action {
                    Some(action)
                        if action.direction == fsl_replay::protocol_analysis::Direction::Send =>
                    {
                        LocalEventKind::Send {
                            peer: action.peer,
                            message_type: action.message_type,
                        }
                    }
                    Some(action) => LocalEventKind::Receive {
                        peer: action.peer,
                        message_type: action.message_type,
                    },
                    None => continue,
                }
            }
            GlobalEventKind::Choice { owner, .. } if owner == role => {
                LocalEventKind::InternalChoice
            }
            GlobalEventKind::Choice { .. } => LocalEventKind::ExternalChoice,
            GlobalEventKind::Region { .. } => LocalEventKind::ParallelRegion,
            GlobalEventKind::Join {
                policy,
                joined,
                outstanding,
                ..
            } => LocalEventKind::ParallelJoin {
                policy: policy.clone(),
                joined: joined.clone(),
                outstanding: *outstanding,
            },
            GlobalEventKind::Control {
                operation,
                maximum,
                window,
                record_type,
                ..
            } => LocalEventKind::ControlAction {
                operation: *operation,
                maximum: *maximum,
                window: window.clone(),
                record_type: record_type.clone(),
            },
        };
        correspondence.insert(event.at.clone(), id);
        events.push(LocalEvent { id, kind });
    }
    let ids: Vec<Option<LocalEventId>> = kept
        .iter()
        .map(|event| correspondence.get(&event.at).copied())
        .collect();
    let ats: Vec<&PortRef> = kept.iter().map(|event| &event.at).collect();
    let mut edges: Vec<(LocalEventId, LocalEventId)> = structure
        .induced_cover(&ats, budget.max_transitions)?
        .into_iter()
        .filter_map(|(from, to)| Some((ids[from]?, ids[to]?)))
        .collect();
    edges.sort();
    let mut alternatives: Vec<(LocalEventId, LocalEventId)> = merge_evidence
        .iter()
        .flat_map(|held| &held.merged)
        .filter_map(|(first, second)| {
            let (a, b) = (correspondence.get(first)?, correspondence.get(second)?);
            Some((*a.min(b), *a.max(b)))
        })
        .collect();
    alternatives.sort();
    alternatives.dedup();
    if let Some(trigger) = activation {
        let shift = |id: &mut LocalEventId| id.0 = id.0.saturating_add(1);
        for event in &mut events {
            shift(&mut event.id);
        }
        for id in correspondence.values_mut() {
            shift(id);
        }
        for (from, to) in &mut edges {
            shift(from);
            shift(to);
        }
        for (first, second) in &mut alternatives {
            shift(first);
            shift(second);
        }
        let mut incoming = vec![false; events.len()];
        for (_, to) in &edges {
            if let Some(marked) = incoming.get_mut(to.0.saturating_sub(1) as usize) {
                *marked = true;
            }
        }
        let mut activation_edges: Vec<_> = incoming
            .iter()
            .enumerate()
            .filter_map(|(at, has_incoming)| {
                (!has_incoming).then_some((LocalEventId(0), LocalEventId(at as u32 + 1)))
            })
            .collect();
        edges.append(&mut activation_edges);
        edges.sort();
        events.insert(
            0,
            LocalEvent {
                id: LocalEventId(0),
                kind: LocalEventKind::Activate { trigger },
            },
        );
    }
    let global_only = structure
        .events()
        .iter()
        .filter_map(|event| match &event.kind {
            GlobalEventKind::Control { operation, .. }
                if matches!(
                    operation,
                    fsl_package::protocol_view::ControlOperation::Check
                        | fsl_package::protocol_view::ControlOperation::Fence
                ) =>
            {
                Some(event.at.clone())
            }
            _ => None,
        })
        .collect();
    Ok(ProjectionWitness {
        key,
        local_graph: LocalGraph {
            events,
            edges,
            alternatives,
        },
        correspondence: Correspondence(correspondence, global_only),
        merge_evidence,
    })
}
