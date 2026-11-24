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
    analyze, Analysis, Constraint, EdgeLabel, FairnessError, Graph, Identity, Strength, Vertex,
};
use fsl_eval::simulation::model::ModelSystem;

use crate::refinement::failure::Taken;

/// One hidden abstract candidate at one position of a concrete lasso.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFairState {
    /// Stable ordinal of this candidate in the position's canonical block order.
    pub ordinal: usize,
    /// The canonical key of this candidate state.
    pub key: Vec<u8>,
    /// Abstract transitions enabled by this candidate alone.
    pub enabled: Vec<Identity>,
}

/// One candidate-preserving edge of a concrete lasso.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFairStep {
    /// Candidate ordinal at the source position.
    pub pre_candidate: usize,
    /// Candidate key at the source position.
    pub pre_state: Vec<u8>,
    /// Abstract transition taken, or `None` for candidate stutter.
    pub transition: Option<Identity>,
    /// Candidate ordinal at the next lasso position.
    pub post_candidate: usize,
    /// Candidate key at the next lasso position.
    pub post_state: Vec<u8>,
    /// The steps after `transition` when the edge runs through intermediate
    /// configurations (a protocol trace's internal closure), in order; empty
    /// for a single step.
    pub interior: Vec<CandidateFairHop>,
}

/// One further step of a candidate edge that passes through intermediate
/// configurations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFairHop {
    /// Abstract transitions enabled at the configuration this hop leaves.
    pub enabled: Vec<Identity>,
    /// The abstract transition the hop takes, or `None` for a step of no class.
    pub transition: Option<Identity>,
}

/// Why a hidden-candidate lasso could not be checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLivenessError {
    /// Candidate rows do not form a closed graph over the supplied positions.
    InvalidGraph,
    /// The fairness graph checker was cancelled or refused its input.
    Fairness(FairnessError),
}

/// Resolve a candidate row's destination ordinal from its occurrence among
/// ordered relation rows with the same post-state. Duplicate keys retain
/// destination order even when distinct source keys converge to them.
pub fn candidate_edge_target_ordinal(
    source: &[CandidateFairState],
    target: &[CandidateFairState],
    pre_candidate: usize,
    post_state: &[u8],
    post_occurrence: usize,
) -> Result<usize, CandidateLivenessError> {
    source
        .iter()
        .find(|candidate| candidate.ordinal == pre_candidate)
        .ok_or(CandidateLivenessError::InvalidGraph)?;
    let matches: Vec<usize> = target
        .iter()
        .filter(|candidate| candidate.key == post_state)
        .map(|candidate| candidate.ordinal)
        .collect();
    if matches.len() == 1 {
        return Ok(matches[0]);
    }
    matches
        .get(post_occurrence)
        .copied()
        .ok_or(CandidateLivenessError::InvalidGraph)
}

/// Resolve the destination block of a stutter row. Stuttering preserves a
/// candidate's key, so its occurrence is counted among prior source rows
/// with that same key that also survive at the target.
pub fn candidate_stutter_target_ordinal(
    source: &[CandidateFairState],
    target: &[CandidateFairState],
    pre_candidate: usize,
    surviving_candidates: &[usize],
) -> Result<usize, CandidateLivenessError> {
    let source_index = source
        .iter()
        .position(|candidate| candidate.ordinal == pre_candidate)
        .ok_or(CandidateLivenessError::InvalidGraph)?;
    let source_candidate = &source[source_index];
    let post_occurrence = source[..=source_index]
        .iter()
        .filter(|candidate| {
            candidate.key == source_candidate.key
                && surviving_candidates.contains(&candidate.ordinal)
        })
        .count()
        .saturating_sub(1);
    candidate_edge_target_ordinal(
        source,
        target,
        pre_candidate,
        &source_candidate.key,
        post_occurrence,
    )
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
    candidate_lasso_has_fair_run_inner(positions, steps, None, constraints, cancel)
}

/// Check a fair candidate run that also traverses at least one marked edge
/// infinitely often.
pub fn candidate_lasso_has_fair_run_with_recurring_edge(
    positions: &[Vec<CandidateFairState>],
    steps: &[Vec<CandidateFairStep>],
    recurring: &[Vec<bool>],
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<bool, CandidateLivenessError> {
    candidate_lasso_has_fair_run_inner(positions, steps, Some(recurring), constraints, cancel)
}

fn candidate_lasso_has_fair_run_inner(
    positions: &[Vec<CandidateFairState>],
    steps: &[Vec<CandidateFairStep>],
    recurring: Option<&[Vec<bool>]>,
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<bool, CandidateLivenessError> {
    if positions.is_empty() || positions.len() != steps.len() {
        return Err(CandidateLivenessError::InvalidGraph);
    }
    if recurring.is_some_and(|edges| {
        edges.len() != steps.len()
            || edges
                .iter()
                .zip(steps)
                .any(|(marked, rows)| marked.len() != rows.len())
    }) {
        return Err(CandidateLivenessError::InvalidGraph);
    }
    let recurring_identity = Identity::new(u64::MAX);
    let mut graph = Graph::new();
    let mut vertices = BTreeMap::<(usize, usize), (Vec<u8>, Vertex)>::new();
    for (position, candidates) in positions.iter().enumerate() {
        for candidate in candidates {
            let enabled = candidate
                .enabled
                .iter()
                .copied()
                .chain(recurring.is_some().then_some(recurring_identity));
            let vertex = graph.add_vertex(enabled);
            if vertices
                .insert(
                    (position, candidate.ordinal),
                    (candidate.key.clone(), vertex),
                )
                .is_some()
            {
                return Err(CandidateLivenessError::InvalidGraph);
            }
        }
    }
    for (position, rows) in steps.iter().enumerate() {
        let next_position = (position + 1) % positions.len();
        for (row_index, row) in rows.iter().enumerate() {
            let from = vertices
                .get(&(position, row.pre_candidate))
                .filter(|(key, _)| *key == row.pre_state)
                .map(|(_, vertex)| *vertex)
                .ok_or(CandidateLivenessError::InvalidGraph)?;
            let to = vertices
                .get(&(next_position, row.post_candidate))
                .filter(|(key, _)| *key == row.post_state)
                .map(|(_, vertex)| *vertex)
                .ok_or(CandidateLivenessError::InvalidGraph)?;
            // The edge runs through its intermediate configurations, each a
            // vertex enabling what the protocol enables there.
            let label = |transition: Option<Identity>| {
                transition.map_or(EdgeLabel::Stutter, EdgeLabel::Transition)
            };
            let mut source = from;
            let mut leaving = label(row.transition);
            for hop in &row.interior {
                let middle = graph.add_vertex(hop.enabled.iter().copied());
                graph
                    .add_edge(source, middle, leaving)
                    .map_err(|_| CandidateLivenessError::InvalidGraph)?;
                source = middle;
                leaving = label(hop.transition);
            }
            let is_recurring = recurring
                .and_then(|edges| edges.get(position))
                .and_then(|edges| edges.get(row_index))
                .copied()
                .unwrap_or(false);
            if recurring.is_some() {
                let middle = graph.add_vertex([recurring_identity]);
                graph
                    .add_edge(source, middle, leaving)
                    .map_err(|_| CandidateLivenessError::InvalidGraph)?;
                graph
                    .add_edge(
                        middle,
                        to,
                        if is_recurring {
                            EdgeLabel::Transition(recurring_identity)
                        } else {
                            EdgeLabel::Stutter
                        },
                    )
                    .map_err(|_| CandidateLivenessError::InvalidGraph)?;
            } else {
                graph
                    .add_edge(source, to, leaving)
                    .map_err(|_| CandidateLivenessError::InvalidGraph)?;
            }
        }
    }
    let initial = vertices
        .iter()
        .filter_map(|((position, _), (_, vertex))| (*position == 0).then_some(*vertex));
    let mut reachable = BTreeSet::new();
    let mut queue: VecDeque<_> = initial.collect();
    while let Some(vertex) = queue.pop_front() {
        if !reachable.insert(vertex) {
            continue;
        }
        queue.extend(graph.edges_from(vertex).iter().map(|edge| edge.target));
    }
    let mut constraints = constraints.to_vec();
    if recurring.is_some() {
        constraints.push(Constraint::new(Strength::Strong, [recurring_identity]));
    }
    let analyses = analyze(&graph, &reachable, &constraints, cancel)
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
            Taken::Stutter | Taken::Failing => false,
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
    use super::{
        candidate_edge_target_ordinal, candidate_stutter_target_ordinal,
        candidate_lasso_has_fair_run,
        candidate_lasso_has_fair_run_with_recurring_edge, CandidateFairHop, CandidateFairState,
        CandidateFairStep,
    };
    use filament_exact::Cancel;
    use fsl_eval::fairness::{Constraint, Identity};
    use ix_trace_rs::trace;

    fn state(ordinal: usize, key: &[u8], enabled: &[Identity]) -> CandidateFairState {
        CandidateFairState {
            ordinal,
            key: key.to_vec(),
            enabled: enabled.to_vec(),
        }
    }

    fn step(
        pre_candidate: usize,
        pre: &[u8],
        transition: Option<Identity>,
        post_candidate: usize,
        post: &[u8],
    ) -> CandidateFairStep {
        CandidateFairStep {
            pre_candidate,
            pre_state: pre.to_vec(),
            transition,
            post_candidate,
            post_state: post.to_vec(),
            interior: Vec::new(),
        }
    }

    /// A one-candidate loop whose edge runs through one intermediate
    /// configuration enabling `middle`, the second hop taking `taken`.
    fn through(middle: &[Identity], taken: Option<Identity>) -> Vec<Vec<CandidateFairStep>> {
        vec![vec![CandidateFairStep {
            pre_candidate: 0,
            pre_state: b"c".to_vec(),
            transition: None,
            post_candidate: 0,
            post_state: b"c".to_vec(),
            interior: vec![CandidateFairHop {
                enabled: middle.to_vec(),
                transition: taken,
            }],
        }]]
    }

    #[trace("FSpec-FR-740-AC-5")]
    #[test]
    fn timed_candidate_liveness_requires_a_recurring_abstract_tick_edge() {
        let positions = vec![vec![state(0, b"c", &[])]];
        let steps = vec![vec![step(0, b"c", None, 0, b"c")]];
        let constraints: Vec<Constraint> = Vec::new();
        let cancel = Cancel::new();
        assert!(!candidate_lasso_has_fair_run_with_recurring_edge(
            &positions,
            &steps,
            &[vec![false]],
            &constraints,
            &cancel,
        )
        .expect("candidate graph checks"));
        assert!(candidate_lasso_has_fair_run_with_recurring_edge(
            &positions,
            &steps,
            &[vec![true]],
            &constraints,
            &cancel,
        )
        .expect("candidate graph checks"));
        let tick = Identity::new(7);
        let interior = through(&[], Some(tick));
        assert!(candidate_lasso_has_fair_run_with_recurring_edge(
            &positions,
            &interior,
            &[vec![true]],
            &[Constraint::strong([tick])],
            &cancel,
        )
        .expect("the interior tick hop witnesses both fairness and recurring liveness"));
        let no_interior_tick = through(&[tick], None);
        assert!(!candidate_lasso_has_fair_run_with_recurring_edge(
            &positions,
            &no_interior_tick,
            &[vec![true]],
            &[Constraint::strong([tick])],
            &cancel,
        )
        .expect("the candidate graph is closed"));
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-6")]
    fn cx1_rejects_a_candidate_graph_with_no_fair_loop() {
        let advance = Identity::new(0);
        let fire = Identity::new(1);
        let positions = vec![vec![
            state(0, b"0", &[]),
            state(1, b"1", &[fire]),
            state(2, b"2", &[fire]),
        ]];
        let steps = vec![vec![
            step(0, b"0", Some(advance), 1, b"1"),
            step(1, b"1", Some(fire), 2, b"2"),
            step(2, b"2", None, 2, b"2"),
        ]];
        assert!(!candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::weak([fire])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-7")]
    fn cx2_checks_all_fairness_constraints_on_one_candidate_run() {
        let fire1 = Identity::new(1);
        let fire2 = Identity::new(2);
        let positions = vec![vec![
            state(0, b"A", &[fire1, fire2]),
            state(1, b"B", &[fire1, fire2]),
        ]];
        let steps = vec![vec![
            step(0, b"A", Some(fire1), 0, b"A"),
            step(1, b"B", Some(fire2), 1, b"B"),
        ]];
        assert!(!candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::weak([fire1]), Constraint::weak([fire2])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-8")]
    fn cx3_keeps_a_candidate_that_never_enables_the_fair_operation() {
        let fire = Identity::new(1);
        let positions = vec![vec![state(0, b"A", &[fire]), state(1, b"B", &[])]];
        let steps = vec![vec![
            step(0, b"A", None, 0, b"A"),
            step(1, b"B", None, 1, b"B"),
        ]];
        assert!(candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::weak([fire])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
    }

    #[test]
    #[trace("FSpec-FR-740-AC-5", "FR-899-AC-6")]
    fn duplicate_candidate_keys_keep_enabled_and_taken_rows_separate() {
        let first = Identity::new(11);
        let owed = Identity::new(12);
        let other = Identity::new(13);
        let positions = vec![vec![
            state(0, b"same", &[first]),
            state(1, b"same", &[owed, other]),
        ]];
        let steps = vec![vec![
            step(0, b"same", Some(first), 0, b"same"),
            step(1, b"same", Some(other), 1, b"same"),
        ]];
        assert!(candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::strong([first]), Constraint::strong([owed])],
            &Cancel::new(),
        )
        .expect("duplicate-key candidate graph is closed"));
    }

    #[trace("FSpec-FR-740-AC-5", "FR-899-AC-6")]
    #[test]
    fn converging_candidate_rows_keep_ordered_duplicate_post_blocks() {
        let owed = Identity::new(21);
        let source = vec![state(0, b"A", &[owed]), state(1, b"B", &[])];
        let target = vec![
            state(0, b"same", &[]),
            state(1, b"same", &[]),
            state(2, b"same", &[]),
        ];
        // Candidate A has two transition alternatives into equal-key post
        // blocks; candidate B converges into the third block. The target
        // ordinal follows relation row order, including row multiplicity.
        let ordered = [(1, b"same".as_slice()), (0, b"same"), (0, b"same")];
        let forward = ordered
            .iter()
            .enumerate()
            .map(|(row_index, (pre_candidate, post_state))| {
                let occurrence = ordered
                    .iter()
                    .enumerate()
                    .filter(|(prior_index, (prior_candidate, prior_post))| {
                        prior_post == post_state
                            && (*prior_candidate, *prior_index) <= (*pre_candidate, row_index)
                    })
                    .count()
                    - 1;
                let post_candidate = candidate_edge_target_ordinal(
                    &source,
                    &target,
                    *pre_candidate,
                    post_state,
                    occurrence,
                )
                .expect("row maps to its ordered duplicate target");
                step(
                    *pre_candidate,
                    if *pre_candidate == 0 { b"A" } else { b"B" },
                    None,
                    post_candidate,
                    post_state,
                )
            })
            .collect();
        let reverse = vec![
            step(0, b"same", None, 0, b"A"),
            step(1, b"same", None, 0, b"A"),
            step(2, b"same", None, 1, b"B"),
        ];
        assert!(candidate_lasso_has_fair_run(
            &[source, target],
            &[forward, reverse],
            &[Constraint::strong([owed])],
            &Cancel::new(),
        )
        .expect("converging duplicate-key candidate graph is closed"));
    }

    #[trace("FSpec-FR-740-AC-5", "FR-899-AC-6")]
    #[test]
    fn stutter_rows_rank_duplicate_keys_across_mixed_candidate_keys() {
        let source = vec![
            state(0, b"A", &[]),
            state(1, b"B", &[]),
            state(2, b"A", &[]),
        ];
        let target = vec![state(0, b"A", &[]), state(1, b"A", &[])];
        assert_eq!(
            candidate_stutter_target_ordinal(&source, &target, 0, &[0, 2]),
            Ok(0)
        );
        assert_eq!(
            candidate_stutter_target_ordinal(&source, &target, 2, &[0, 2]),
            Ok(1)
        );
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-10")]
    fn a_strong_class_enabled_only_inside_a_trace_edge_is_owed() {
        // The class is enabled at the intermediate configuration of every
        // loop iteration and no hop takes it: strong fairness is violated.
        let owed = Identity::new(1);
        let positions = vec![vec![state(0, b"c", &[])]];
        let steps = through(&[owed], None);
        assert!(!candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::strong([owed])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
        // A hop that takes it discharges the obligation.
        let steps = through(&[owed], Some(owed));
        assert!(candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::strong([owed])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
    }

    #[test]
    #[trace("TC-2536", "FR-899-AC-10")]
    fn a_weak_class_is_enabled_everywhere_only_when_every_configuration_enables_it() {
        let owed = Identity::new(1);
        // Enabled at the start and the intermediate configuration, never
        // taken: weak fairness is violated.
        let positions = vec![vec![state(0, b"c", &[owed])]];
        let steps = through(&[owed], None);
        assert!(!candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::weak([owed])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
        // Disabled at the intermediate configuration: the class is not
        // enabled at every position, so the run is fair.
        let steps = through(&[], None);
        assert!(candidate_lasso_has_fair_run(
            &positions,
            &steps,
            &[Constraint::weak([owed])],
            &Cancel::new(),
        )
        .expect("the candidate graph is closed"));
    }
}
