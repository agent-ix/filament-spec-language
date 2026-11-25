// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-167 (ADR-022 GE-1, GM-6): phase 1, one canonical breadth-first
//! exploration of the subject for every state-graph item.
//!
//! Initial states are taken in ascending state-key order, equal keys
//! coalescing into one node, and successors in transition-identity bytes,
//! then post-state key bytes, order (FR-101). A node is labelled when it is
//! taken from the frontier, then expanded. Every edge is retained. A node is
//! open when the run did not learn all of its outgoing edges:
//!
//! - `MaxDepth`: it is at the horizon and has a successor, which the run does
//!   not follow;
//! - `ConstraintBoundary`: the selected state constraint is false at it, so it
//!   is not expanded (FR-158);
//! - `UndecidedSuccessor`: its expansion evaluated a contract conjunction
//!   undecided, the successors the expansion did return being retained and
//!   explored; the selected state constraint is undefined at it; or it has no
//!   successor and a refused `terminal when` leaves it undecided;
//! - `NotExpanded`: it was discovered and not expanded, or partly expanded,
//!   when a limit or a cancellation stopped the run.
//!
//! The first node in discovery order where one of the subject's own
//! expressions (a guard, the `terminal when` predicate, the selected state
//! constraint) is undefined is kept with the graph, and refutes every
//! state-graph item (FR-168 "Undefined evaluation").
//!
//! The deadlock-freedom item is decided on the same exploration, as FR-126
//! decides it: the first node in discovery order whose expansion evaluates a
//! contract conjunction undefined, or that is deadlocked, is its violation,
//! with the canonical breadth-first path to it.

use ix_trace_rs::implements;
use std::collections::{BTreeMap, VecDeque};

use filament_exact::Cancel;
use filament_semantic_value::location::ExpressionLocation;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::automaton::{SafetyState, SafetyTranslation};
use fsl_eval::simulation::model::{
    ModelFinding, ModelSimState, ModelSystem, TerminalClass, UndefinedContract,
};
use fsl_eval::simulation::{
    ConstraintValue, ExpansionStop, InitialStateRefusal, SubjectSystem, SymmetryGroup,
    TransitionSystem,
};
use fsl_eval::temporal::UndefinedCause;
use fsl_replay::{
    ClosureCertificate, CounterexampleKind, GraphUndefined, ItemRefusal, ModelCheckLimit,
    ModelCheckLimits, ModelCheckOutcome, ModelStep, OpenCause, ProductStateRef, ReductionCause,
    RunEnd, RunStats, TemporalCounterexample, UndecidedCause, UndefinedEvaluation, UndefinedLocus,
    UndefinedSite,
};

use super::super::ample::Por;
use super::{
    Budgets, ExploredStateGraph, GraphEdge, GraphNode, GraphStats, Halt, InstanceView, NodeIndex,
    PredicateLabel, cancelled, expansion_stop, stopped,
};
use crate::model_check::ModelCheckRefusal;
use crate::model_check::check::{fault, widen};
use crate::model_check::initial::{Refused, admit_initial};

/// Why keying one initial state failed.
enum Keying {
    /// Canonicalising it stopped.
    Stop(ExpansionStop),
    /// Its key has no encoding.
    Refused(ModelCheckRefusal),
}

/// The admitted roots and the declaration-order mapping both phases use.
pub(super) struct Initial {
    nodes: Vec<(Vec<u8>, usize, ModelSimState)>,
    keys: Vec<Vec<u8>>,
    pub(super) starts: Vec<ModelSimState>,
}

impl Initial {
    /// Admit before sampling or checking symmetry closure. Keep the concrete
    /// starts only when one of those steps needs them.
    pub(super) fn admit(
        system: &ModelSystem<'_>,
        symmetry: &SymmetryGroup,
        max_states: usize,
        retain_starts: bool,
        budgets: Budgets,
    ) -> Result<Result<Self, Halt>, ModelCheckRefusal> {
        let mut keys = Vec::new();
        let mut starts = Vec::new();
        let nodes = admit_initial(
            system.initial_for_check_with_symmetry(max_states, symmetry),
            max_states,
            |_, start| {
                let (state, _) = system
                    .canonicalise(&start, symmetry)
                    .map_err(Keying::Stop)?;
                let (key, _) = system
                    .state_key(&state, fsl_foundation::IdentityLimits::default())
                    .map_err(|refused| Keying::Refused(refused.into()))?;
                keys.push(key.clone());
                if retain_starts {
                    starts.push(start);
                }
                Ok((key, state))
            },
        );
        match nodes {
            Ok(nodes) => Ok(Ok(Self {
                nodes,
                keys,
                starts,
            })),
            Err(Refused::OverBudget) => {
                Ok(Err(stopped(ModelCheckLimit::MaxStates, widen(max_states))))
            }
            Err(Refused::Construction(InitialStateRefusal::OverBudget(_))) => {
                Ok(Err(stopped(ModelCheckLimit::MaxStates, widen(max_states))))
            }
            Err(Refused::Construction(InitialStateRefusal::Encoding(refusal))) => {
                Err(ModelCheckRefusal::Encoding(refusal))
            }
            Err(Refused::Construction(InitialStateRefusal::Expansion(stop))) => {
                Ok(Err(expansion_stop(&stop, budgets)?))
            }
            Err(Refused::Construction(InitialStateRefusal::Construction(refusal))) => {
                Err(ModelCheckRefusal::Protocol(refusal))
            }
            Err(Refused::Keying(Keying::Stop(stop))) => Ok(Err(expansion_stop(&stop, budgets)?)),
            Err(Refused::Keying(Keying::Refused(refusal))) => Err(refusal),
        }
    }
}

/// The explored graph of a run that explored nothing.
pub(super) fn empty(end: RunEnd) -> ExploredStateGraph {
    ExploredStateGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        open: BTreeMap::new(),
        end,
        stats: GraphStats::default(),
        outgoing: Vec::new(),
        parent: Vec::new(),
        root: Vec::new(),
        initial: Vec::new(),
        subject_undefined: None,
    }
}

/// What the deadlock-freedom item found at the first node that decided it.
enum DeadlockEvent {
    Violated(Box<CounterexampleKind>, usize),
    Undecided,
}

/// Phase 1's result: the graph and the deadlock-freedom item's outcome.
pub(super) struct Explored {
    pub(super) graph: ExploredStateGraph,
    pub(super) deadlock: Option<ModelCheckOutcome>,
    pub(super) reduced: bool,
    pub(super) partial_order_reduced: bool,
}

/// Phase 1 of one run.
pub(super) struct Exploration<'r, 'a, 'v> {
    pub(super) system: &'r ModelSystem<'a>,
    pub(super) limits: &'r ModelCheckLimits,
    pub(super) horizon: Option<usize>,
    pub(super) cancel: &'r Cancel,
    pub(super) budgets: Budgets,
    pub(super) views: &'r [Vec<InstanceView<'v>>],
    /// The selected state constraint, which the pre-check resolved.
    pub(super) constraint: Option<&'r str>,
    pub(super) deadlock_freedom: bool,
    pub(super) por: Option<&'r Por<'r, 'a>>,
    pub(super) declared_symmetry: &'r SymmetryGroup,
    pub(super) symmetry: &'r SymmetryGroup,
}

/// The run's working state.
struct Walk {
    graph: ExploredStateGraph,
    states: Vec<ModelSimState>,
    queue: VecDeque<usize>,
    deadlock: Option<DeadlockEvent>,
    horizon_cut: bool,
    reduced: bool,
    partial_order_reduced: bool,
}

impl Walk {
    /// Mark `at` and every queued node `NotExpanded`.
    fn halt(&mut self, at: usize) {
        for node in std::iter::once(at).chain(self.queue.iter().copied()) {
            self.graph
                .open
                .insert(NodeIndex(node), OpenCause::NotExpanded);
        }
    }

    /// Keep `at` as the node where one of the subject's expressions is
    /// undefined. Nodes are expanded in discovery order, so the first one
    /// kept is the earliest.
    fn subject_undefined(
        &mut self,
        at: usize,
        locus: Option<ExpressionLocation>,
        transition: Option<ModelTransition>,
        cause: UndefinedCause,
    ) {
        let state_key = &self.graph.nodes[at].key;
        self.graph.subject_undefined.get_or_insert_with(|| {
            (
                NodeIndex(at),
                GraphUndefined {
                    predicate: None,
                    transition,
                    state_key: state_key.clone(),
                    locus,
                    cause,
                },
            )
        });
    }

    /// The deadlock-freedom item's violation by an undefined evaluation at
    /// node `at`.
    fn undefined_violation(
        &self,
        at: usize,
        locus: UndefinedLocus,
        transition: Option<ModelTransition>,
        cause: UndefinedCause,
    ) -> DeadlockEvent {
        let node = &self.graph.nodes[at];
        DeadlockEvent::Violated(
            Box::new(CounterexampleKind::UndefinedEvaluation(
                UndefinedEvaluation {
                    site: UndefinedSite {
                        position: node.depth,
                        state_key: node.key.clone(),
                        locus,
                        transition,
                        time: None,
                    },
                    cause,
                },
            )),
            at,
        )
    }

    fn insert(
        &mut self,
        state: ModelSimState,
        key: fsl_eval::simulation::StateKey,
        depth: usize,
        parent: Option<usize>,
        root: usize,
    ) -> usize {
        let node = self.graph.nodes.len();
        self.graph.nodes.push(GraphNode {
            key,
            depth: widen(depth),
            labels: Vec::new(),
        });
        self.graph.outgoing.push(Vec::new());
        self.graph.parent.push(parent);
        self.graph.root.push(root);
        self.states.push(state);
        self.queue.push_back(node);
        node
    }
}

impl Exploration<'_, '_, '_> {
    /// Explore the subject.
    pub(super) fn run(&self, initial: Initial) -> Result<Explored, ModelCheckRefusal> {
        let mut walk = Walk {
            graph: empty(RunEnd::Completed),
            states: Vec::new(),
            queue: VecDeque::new(),
            deadlock: None,
            horizon_cut: false,
            reduced: !self.symmetry.is_trivial(),
            partial_order_reduced: false,
        };
        let Initial {
            nodes,
            keys,
            starts: _,
        } = initial;
        let mut known = BTreeMap::new();
        for (key, index, state) in nodes {
            let node = walk.insert(
                state,
                fsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                0,
                None,
                index,
            );
            known.insert(key, node);
        }
        let of_initial = keys
            .iter()
            .map(|key| {
                known
                    .get(key)
                    .copied()
                    .map(NodeIndex)
                    .ok_or_else(|| fault("every-initial-state-has-a-node"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        walk.graph.initial = of_initial;
        while let Some(at) = walk.queue.pop_front() {
            match self.visit(&mut walk, at, &mut known)? {
                Ok(()) => {}
                Err(halt) => {
                    walk.halt(at);
                    walk.graph.end = halt.end();
                    break;
                }
            }
        }
        self.finish(walk)
    }

    /// Label and expand node `at`.
    #[implements(
        "FR-167-AC-7",
        "FR-168-AC-11",
        "FR-168-AC-12",
        "FR-168-AC-13",
        "FR-168-AC-16"
    )]
    fn visit(
        &self,
        walk: &mut Walk,
        at: usize,
        known: &mut BTreeMap<Vec<u8>, usize>,
    ) -> Result<Result<(), Halt>, ModelCheckRefusal> {
        if let Some(halt) = cancelled(self.cancel) {
            return Ok(Err(halt));
        }
        let mut keys = Vec::new();
        let mut predicates = Vec::new();
        for view in self.views.iter().flatten() {
            for (role, claim) in &view.predicates {
                if let Some(claim) = claim {
                    keys.push((view.item, view.instance, *role));
                    predicates.push((*claim, *role, view.over.as_ref()));
                }
            }
        }
        let readings = match self
            .system
            .state_graph_predicates(&walk.states[at], &predicates)
        {
            Ok(readings) => readings,
            Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
        };
        let labels = keys
            .into_iter()
            .zip(readings)
            .map(|((item, instance, role), reading)| PredicateLabel {
                item,
                instance,
                role,
                reading,
            })
            .collect();
        walk.graph.nodes[at].labels = labels;
        if let Some(name) = self.constraint {
            match self.system.constraint_value(&walk.states[at], name) {
                Ok(ConstraintValue::Holds) => {}
                // FR-158 "Boundary states": stored and labelled, never
                // expanded, and never a deadlock.
                Ok(ConstraintValue::Boundary) => {
                    walk.graph
                        .open
                        .insert(NodeIndex(at), OpenCause::ConstraintBoundary);
                    return Ok(Ok(()));
                }
                Ok(ConstraintValue::Undefined { cause, .. }) => {
                    walk.graph
                        .open
                        .insert(NodeIndex(at), OpenCause::UndecidedSuccessor);
                    walk.subject_undefined(
                        at,
                        self.system.state_clause_locus(name),
                        None,
                        cause.clone(),
                    );
                    if self.deadlock_freedom && walk.deadlock.is_none() {
                        let locus = UndefinedLocus::Constraint(name.to_owned());
                        walk.deadlock = Some(walk.undefined_violation(at, locus, None, cause));
                    }
                    return Ok(Ok(()));
                }
                Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
            }
        }
        let expansion = match self.system.successors(&walk.states[at]) {
            Ok(expansion) => expansion,
            Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
        };
        let undefined = expansion
            .findings
            .iter()
            .find_map(ModelFinding::undefined_contract);
        let undecided = expansion
            .findings
            .iter()
            .any(ModelFinding::is_refused_contract);
        if undefined.is_some() || undecided {
            walk.graph
                .open
                .insert(NodeIndex(at), OpenCause::UndecidedSuccessor);
        }
        // An undefined guard is no disabled transition: it refutes.
        let guard = expansion
            .findings
            .iter()
            .filter_map(ModelFinding::undefined_contract)
            .find_map(|contract| Some((contract.clause, contract.guard_of?, contract.cause)));
        if let Some((clause, transition, cause)) = guard {
            let locus = self.system.state_clause_locus(&clause);
            walk.subject_undefined(at, locus, Some(transition), cause);
        }
        let pending = self.deadlock_freedom && walk.deadlock.is_none();
        // FR-124: `terminal when` is read only where no transition is
        // enabled, which an undecided contract leaves unknown.
        let terminal = if expansion.successors.is_empty() && undefined.is_none() && !undecided {
            match self.system.terminal_class(&walk.states[at], false) {
                Ok(class) => Some(class),
                Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
            }
        } else {
            None
        };
        match &terminal {
            // A refused declaration may intend the state, so the node stays
            // undecided, whatever another reads (FR-124, FR-167).
            Some(TerminalClass::Undecided { .. }) => {
                walk.graph
                    .open
                    .insert(NodeIndex(at), OpenCause::UndecidedSuccessor);
            }
            Some(TerminalClass::Undefined { model, cause }) => {
                let locus = self.system.terminal_locus(model);
                walk.subject_undefined(at, locus, None, cause.clone());
            }
            _ => {}
        }
        if pending {
            walk.deadlock = deadlock_event(walk, at, undefined, undecided, terminal);
        }
        let depth = self.depth_of(walk, at)?;
        if self.horizon.is_some_and(|horizon| depth >= horizon) {
            if !expansion.successors.is_empty() {
                walk.horizon_cut = true;
                walk.graph
                    .open
                    .entry(NodeIndex(at))
                    .or_insert(OpenCause::MaxDepth);
            }
            return Ok(Ok(()));
        }
        let mut successors = Vec::with_capacity(expansion.successors.len());
        for (transition, state) in expansion.successors {
            let order = self.system.transition_key(&transition)?;
            let (state, permutation) = match self.system.canonicalise(&state, self.symmetry) {
                Ok(canonical) => canonical,
                Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
            };
            let (key, _) = self
                .system
                .state_key(&state, fsl_foundation::IdentityLimits::default())?;
            successors.push((order, key, transition, state, permutation));
        }
        successors.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        let root = walk.graph.root[at];
        let selected = if let Some(por) = self.por {
            let mut enabled: Vec<_> = successors
                .iter()
                .map(|(_, _, transition, _, _)| transition.clone())
                .collect();
            enabled.sort();
            let fresh = |transition: &ModelTransition| {
                successors
                    .iter()
                    .find(|(_, _, candidate, _, _)| candidate == transition)
                    .is_some_and(|(_, key, _, _, _)| !known.contains_key(key))
            };
            match por.ample(&walk.states[at], &enabled, &fresh) {
                Ok(selected) => Some(selected),
                Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
            }
        } else {
            None
        };
        if selected
            .as_ref()
            .is_some_and(|chosen| chosen.len() < successors.len())
        {
            walk.reduced = true;
            walk.partial_order_reduced = true;
        }
        for (_, key, transition, state, permutation) in successors {
            if selected
                .as_ref()
                .is_some_and(|chosen| !chosen.contains(&transition))
            {
                continue;
            }
            if walk.graph.edges.len() >= self.limits.limits.max_transitions {
                return Ok(Err(stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                )));
            }
            let edge = walk.graph.edges.len();
            let target = match known.get(&key) {
                Some(node) => *node,
                None => {
                    if walk.graph.nodes.len() >= self.limits.limits.max_states {
                        return Ok(Err(stopped(
                            ModelCheckLimit::MaxStates,
                            widen(self.limits.limits.max_states),
                        )));
                    }
                    let node = walk.insert(
                        state,
                        fsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                        depth.saturating_add(1),
                        Some(edge),
                        root,
                    );
                    known.insert(key, node);
                    node
                }
            };
            walk.graph.edges.push(GraphEdge {
                source: NodeIndex(at),
                transition,
                target: NodeIndex(target),
                permutation,
            });
            walk.graph.outgoing[at].push(edge);
        }
        Ok(Ok(()))
    }

    fn depth_of(&self, walk: &Walk, at: usize) -> Result<usize, ModelCheckRefusal> {
        usize::try_from(walk.graph.nodes[at].depth).map_err(|_| fault("node-depth-fits-usize"))
    }

    /// The graph's statistics and the deadlock-freedom item's outcome.
    fn finish(&self, mut walk: Walk) -> Result<Explored, ModelCheckRefusal> {
        walk.graph.stats = GraphStats {
            nodes: widen(walk.graph.nodes.len()),
            edges: widen(walk.graph.edges.len()),
            depth: walk
                .graph
                .nodes
                .iter()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0),
        };
        let deadlock = if self.deadlock_freedom {
            Some(self.deadlock(&walk)?)
        } else {
            None
        };
        Ok(Explored {
            graph: walk.graph,
            deadlock,
            reduced: walk.reduced,
            partial_order_reduced: walk.partial_order_reduced,
        })
    }

    /// The deadlock-freedom item's outcome, as FR-126 decides it.
    fn deadlock(&self, walk: &Walk) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let graph = &walk.graph;
        let stats = RunStats {
            product_states: graph.stats.nodes,
            product_edges: graph.stats.edges,
            stutter_edges: 0,
            automaton_states: 0,
            depth: graph.stats.depth,
            bound_limited: 0,
            message_bound_limited: 0,
            instance_limited: 0,
        };
        Ok(match &walk.deadlock {
            Some(DeadlockEvent::Violated(kind, at)) => ModelCheckOutcome::Violated {
                counterexample: Box::new(TemporalCounterexample {
                    initial_state: graph.root[*at],
                    prefix: graph.path_to(NodeIndex(*at)),
                    over_binding: None,
                    fairness: Vec::new(),
                    kind: kind.as_ref().clone(),
                    lasso: None,
                    blocked: Vec::new(),
                    refinement: None,
                    final_delay: None,
                    closure_stutters: Vec::new(),
                }),
                stats,
            },
            Some(DeadlockEvent::Undecided) => ModelCheckOutcome::Undecided {
                cause: UndecidedCause::UndecidedSuccessor,
                stats,
            },
            None => match &graph.end {
                RunEnd::Stopped { cause, limit } => ModelCheckOutcome::Stopped {
                    cause: cause.clone(),
                    limit: *limit,
                    stats,
                },
                // FR-158: a boundary state settles before any bound.
                RunEnd::Completed if graph.boundary_states() > 0 => ModelCheckOutcome::Reduction {
                    cause: ReductionCause::ConstraintReached {
                        boundary_states: graph.boundary_states(),
                    },
                    stats,
                },
                RunEnd::Completed if walk.horizon_cut => ModelCheckOutcome::BoundReached {
                    depth: widen(self.horizon.unwrap_or(0)),
                    stats,
                },
                RunEnd::Completed if walk.reduced => ModelCheckOutcome::HoldsReduced {
                    reductions: super::applied_reductions(
                        walk.partial_order_reduced,
                        self.declared_symmetry,
                        self.symmetry,
                        &[],
                    ),
                    stats,
                },
                RunEnd::Completed => ModelCheckOutcome::Holds {
                    certificate: fsl_replay::ProofCertificate::Closure(deadlock_certificate(
                        graph,
                    )?),
                    stats,
                },
            },
        })
    }
}

/// The deadlock-freedom item's event at node `at`, if `at` decides it: an
/// undefined contract conjunction, then a refused one, then the node's
/// FR-124 class when it is terminal (ADR-018 DL-7, UE-6, V-6).
#[implements("FR-168-AC-16")]
fn deadlock_event(
    walk: &Walk,
    at: usize,
    undefined: Option<UndefinedContract>,
    undecided: bool,
    terminal: Option<TerminalClass>,
) -> Option<DeadlockEvent> {
    if let Some(contract) = undefined {
        let locus = UndefinedLocus::Contract(contract.clause);
        return Some(walk.undefined_violation(at, locus, contract.guard_of, contract.cause));
    }
    if undecided {
        return Some(DeadlockEvent::Undecided);
    }
    match terminal? {
        TerminalClass::NotTerminal | TerminalClass::Intended => None,
        TerminalClass::Deadlocked => Some(DeadlockEvent::Violated(
            Box::new(CounterexampleKind::Deadlock),
            at,
        )),
        TerminalClass::Undefined { model, cause } => {
            Some(walk.undefined_violation(at, UndefinedLocus::Terminal(model), None, cause))
        }
        TerminalClass::Undecided { .. } => Some(DeadlockEvent::Undecided),
    }
}

/// FR-126's closure certificate of a deadlock-freedom proof: every explored
/// model state under the item's carried root, sorted.
fn deadlock_certificate(
    graph: &ExploredStateGraph,
) -> Result<ClosureCertificate, ModelCheckRefusal> {
    let key = SafetyTranslation::deadlock_freedom()
        .map_err(|_| ModelCheckRefusal::Item(ItemRefusal::DeadlockContext))?
        .key(SafetyState::Carried)?;
    let mut states: Vec<ProductStateRef> = graph
        .nodes
        .iter()
        .map(|node| ProductStateRef {
            model_state: node.key.clone(),
            automaton_state: key.clone(),
        })
        .collect();
    states.sort();
    states.dedup();
    Ok(ClosureCertificate { states })
}

impl ExploredStateGraph {
    /// The canonical breadth-first path's steps to `node`.
    pub(super) fn path_to(&self, node: NodeIndex) -> Vec<ModelStep> {
        let mut steps = Vec::new();
        let mut cursor = node.0;
        while let Some(edge) = self.parent.get(cursor).copied().flatten() {
            let Some(edge) = self.edges.get(edge) else {
                break;
            };
            steps.push(ModelStep {
                transition: edge.transition.clone(),
                post_state: self.nodes[cursor].key.clone(),
                delay: None,
            });
            cursor = edge.source.0;
        }
        steps.reverse();
        steps
    }

    /// The boundary states of the selected state constraint the run stored
    /// (FR-158).
    pub(super) fn boundary_states(&self) -> u64 {
        widen(
            self.open
                .values()
                .filter(|cause| **cause == OpenCause::ConstraintBoundary)
                .count(),
        )
    }

    /// The initial-state index of `node`'s breadth-first tree root.
    pub(super) fn root_of(&self, node: NodeIndex) -> usize {
        self.root.get(node.0).copied().unwrap_or(0)
    }
}
