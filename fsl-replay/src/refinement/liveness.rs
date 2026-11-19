// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 "Enabled and taken through the mapping" (ADR-018 FA-1, ADR-020
//! CO-2): the classes of the abstract fairness set `F_A`, and when a class is
//! enabled at a mapped state or taken by a step.
//!
//! The engine's second phase and replay's lasso check both read the classes
//! here, so a lasso replay never disagrees with the engine about which
//! abstract operation a loop leaves undelivered.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::Cancel;
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::{FairnessConstraint, FairnessGranularity};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{
    Analysis, Constraint, EdgeLabel, FairnessError, Graph, Identity, Vertex, analyze,
};
use fsl_eval::simulation::model::ModelSystem;

use crate::refinement::failure::Taken;

/// One hidden abstract candidate at one position of a concrete lasso.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFairState {
    /// The canonical key of this candidate state.
    pub key: Vec<u8>,
    /// Abstract transitions enabled by this candidate alone.
    pub enabled: Vec<Identity>,
}

/// One candidate-preserving edge of a concrete lasso.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFairStep {
    /// Candidate key at the source position.
    pub pre_state: Vec<u8>,
    /// Abstract transition taken, or `None` for candidate stutter.
    pub transition: Option<Identity>,
    /// Candidate key at the next lasso position.
    pub post_state: Vec<u8>,
}

/// Why a hidden-candidate lasso could not be checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLivenessError {
    /// Candidate rows do not form a closed graph over the supplied positions.
    InvalidGraph,
    /// The fairness graph checker was cancelled or refused its input.
    Fairness(FairnessError),
}

/// Check whether a concrete lasso has a coherent fair hidden-candidate run.
///
/// `positions` contains the candidate states and per-candidate enabled sets
/// at each lasso position. `steps[position]` contains only candidate edges
/// compatible with the concrete edge at that position. The resulting graph
/// preserves candidate continuity; it never unions taken/enabled facts across
/// incompatible candidates.
pub fn candidate_lasso_has_fair_run(
    positions: &[Vec<CandidateFairState>],
    steps: &[Vec<CandidateFairStep>],
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<bool, CandidateLivenessError> {
    if positions.is_empty() || positions.len() != steps.len() {
        return Err(CandidateLivenessError::InvalidGraph);
    }
    let mut graph = Graph::new();
    let mut vertices = BTreeMap::<(usize, Vec<u8>), Vertex>::new();
    for (position, candidates) in positions.iter().enumerate() {
        for candidate in candidates {
            let vertex = graph.add_vertex(candidate.enabled.iter().copied());
            vertices.insert((position, candidate.key.clone()), vertex);
        }
    }
    for (position, rows) in steps.iter().enumerate() {
        let next_position = (position + 1) % positions.len();
        for row in rows {
            let from = vertices
                .get(&(position, row.pre_state.clone()))
                .copied()
                .ok_or(CandidateLivenessError::InvalidGraph)?;
            let to = vertices
                .get(&(next_position, row.post_state.clone()))
                .copied()
                .ok_or(CandidateLivenessError::InvalidGraph)?;
            graph
                .add_edge(
                    from,
                    to,
                    row.transition
                        .map_or(EdgeLabel::Stutter, EdgeLabel::Transition),
                )
                .map_err(|_| CandidateLivenessError::InvalidGraph)?;
        }
    }
    let initial = vertices
        .iter()
        .filter_map(|((position, _), vertex)| (*position == 0).then_some(*vertex));
    let mut reachable = BTreeSet::new();
    let mut queue: VecDeque<_> = initial.collect();
    while let Some(vertex) = queue.pop_front() {
        if !reachable.insert(vertex) {
            continue;
        }
        queue.extend(graph.edges_from(vertex).iter().map(|edge| edge.target));
    }
    let analyses = analyze(&graph, &reachable, constraints, cancel)
        .map_err(CandidateLivenessError::Fairness)?;
    Ok(Analysis::first_fair_in(&analyses).is_some())
}

/// One class of `F_A`: a `whole` constraint's operation, or one transition
/// identity of an `each` constraint's operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractClass {
    constraint: usize,
    operation: String,
    identity: Option<ModelTransition>,
}

impl AbstractClass {
    /// The index of the class's constraint in `F_A`.
    pub fn constraint(&self) -> usize {
        self.constraint
    }

    /// Whether `transition` is of the class.
    pub fn covers(&self, transition: &ModelTransition) -> bool {
        transition.operation() == self.operation
            && self
                .identity
                .as_ref()
                .is_none_or(|identity| identity == transition)
    }

    /// Whether the abstract subject enables the class at a mapped state whose
    /// enabled transition identities are `enabled`.
    pub fn is_enabled_in(&self, enabled: &[ModelTransition]) -> bool {
        enabled.iter().any(|transition| self.covers(transition))
    }

    /// Whether a step that took `taken` takes the class. A `stutter` step
    /// takes none.
    pub fn is_taken_by(&self, taken: &Taken) -> bool {
        match taken {
            Taken::Stutter => false,
            Taken::Abstract(transitions) => {
                transitions.iter().any(|transition| self.covers(transition))
            }
            Taken::CandidateSteps(steps) => steps
                .iter()
                .filter_map(|step| step.transition.as_ref())
                .any(|transition| self.covers(transition)),
            Taken::Protocol(transitions) => transitions
                .iter()
                .filter_map(fsl_eval::simulation::protocol::ProtocolTransition::application)
                .any(|transition| self.covers(transition)),
            Taken::ProtocolTraces(traces) => traces.iter().any(|trace| {
                trace
                    .transitions
                    .iter()
                    .filter_map(fsl_eval::simulation::protocol::ProtocolTransition::application)
                    .any(|transition| self.covers(transition))
            }),
        }
    }
}

/// The classes of `fairness`, in its source order and, within an `each`
/// constraint, in the order of `identities`, which are the abstract
/// transition identities the behaviour enables or takes.
///
/// # Errors
///
/// [`InternalFault`] when a constraint names an operation the abstract
/// subject does not declare, which the checked refinement rules out.
pub fn abstract_classes(
    system: &ModelSystem<'_>,
    fairness: &[FairnessConstraint],
    identities: &[ModelTransition],
) -> Result<Vec<AbstractClass>, InternalFault> {
    let mut classes = Vec::new();
    for (index, constraint) in fairness.iter().enumerate() {
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| {
                InternalFault::new(
                    "refinement-liveness",
                    "an-abstract-fairness-names-a-declared-operation",
                )
            })?;
        match constraint.granularity() {
            FairnessGranularity::Whole => classes.push(AbstractClass {
                constraint: index,
                operation: text.to_owned(),
                identity: None,
            }),
            FairnessGranularity::Each => classes.extend(
                identities
                    .iter()
                    .filter(|transition| transition.operation() == text)
                    .map(|transition| AbstractClass {
                        constraint: index,
                        operation: text.to_owned(),
                        identity: Some(transition.clone()),
                    }),
            ),
        }
    }
    Ok(classes)
}

#[cfg(test)]
mod candidate_tests {
    use super::{CandidateFairState, CandidateFairStep, candidate_lasso_has_fair_run};
    use filament_exact::Cancel;
    use fsl_eval::fairness::{Constraint, Identity};
    use ix_trace_rs::trace;

    fn state(key: &[u8], enabled: &[Identity]) -> CandidateFairState {
        CandidateFairState {
            key: key.to_vec(),
            enabled: enabled.to_vec(),
        }
    }

    fn step(pre: &[u8], transition: Option<Identity>, post: &[u8]) -> CandidateFairStep {
        CandidateFairStep {
            pre_state: pre.to_vec(),
            transition,
            post_state: post.to_vec(),
        }
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-6")]
    fn cx1_rejects_a_candidate_graph_with_no_fair_loop() {
        let advance = Identity::new(0);
        let fire = Identity::new(1);
        let positions = vec![vec![
            state(b"0", &[]),
            state(b"1", &[fire]),
            state(b"2", &[fire]),
        ]];
        let steps = vec![vec![
            step(b"0", Some(advance), b"1"),
            step(b"1", Some(fire), b"2"),
            step(b"2", None, b"2"),
        ]];
        assert!(
            !candidate_lasso_has_fair_run(
                &positions,
                &steps,
                &[Constraint::weak([fire])],
                &Cancel::new(),
            )
            .expect("the candidate graph is closed")
        );
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-7")]
    fn cx2_checks_all_fairness_constraints_on_one_candidate_run() {
        let fire1 = Identity::new(1);
        let fire2 = Identity::new(2);
        let positions = vec![vec![
            state(b"A", &[fire1, fire2]),
            state(b"B", &[fire1, fire2]),
        ]];
        let steps = vec![vec![
            step(b"A", Some(fire1), b"A"),
            step(b"B", Some(fire2), b"B"),
        ]];
        assert!(
            !candidate_lasso_has_fair_run(
                &positions,
                &steps,
                &[Constraint::weak([fire1]), Constraint::weak([fire2])],
                &Cancel::new(),
            )
            .expect("the candidate graph is closed")
        );
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-8")]
    fn cx3_keeps_a_candidate_that_never_enables_the_fair_operation() {
        let fire = Identity::new(1);
        let positions = vec![vec![state(b"A", &[fire]), state(b"B", &[])]];
        let steps = vec![vec![step(b"A", None, b"A"), step(b"B", None, b"B")]];
        assert!(
            candidate_lasso_has_fair_run(
                &positions,
                &steps,
                &[Constraint::weak([fire])],
                &Cancel::new(),
            )
            .expect("the candidate graph is closed")
        );
    }
}
