// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-graph forms over the symbolic timed graph (FSpec FR-712).
//!
//! The claim's predicates read model state, so they are evaluated per node
//! here; what they decide over the timed states is
//! [`super::state_graph_zones`]'s.

use std::collections::VecDeque;

use filament_exact::{Cancel, ObjectReference};
use fsl_eval::simulation::model::{ModelSystem, PredicateReading, TimedSystem};
use fsl_eval::simulation::ConstraintValue;
use fsl_replay::{ModelPath, ModelStep};
use fsl_semantics::check::{
    CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole, StateGraphForm,
};
use ix_trace_rs::implements;

use super::search::{TimedStateGraph, ZoneCheckLimits, ZoneCheckRefusal};
use super::state_graph_zones::Zones;

/// A state-graph decision over a timed zone graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedStateGraphDecision {
    /// The claim holds. `witness_paths` is populated for `possible`; `ef_nodes`
    /// is populated for `always possible`.
    Holds {
        /// The concretised witness path for each initial state.
        witness_paths: Vec<ModelPath>,
        /// The nodes whose whole zone lies in `E(P)`.
        ef_nodes: Vec<usize>,
    },
    /// The claim is refuted at one initial or reachable timed state.
    Refuted {
        /// The path to the deciding timed state. A trap's path ends at it; a
        /// two-path refutation's may be reached from the end by waiting.
        stem: ModelPath,
        /// The node that holds the deciding timed state.
        node: usize,
        /// The nodes whose whole zone lies in `E(P)`, when the form is
        /// `always possible`.
        ef_nodes: Vec<usize>,
        /// Two distinct paths from the stem's end for a `unique path`
        /// refutation; absent when no path reaches the target.
        paths: Option<(Vec<ModelStep>, Vec<ModelStep>)>,
    },
    /// The claim is violated at `node` only at states its path reaches by
    /// waiting after the last step, which no stem names (FR-852
    /// "Evidence"): neither proved nor refuted.
    Unwritten {
        /// The node that holds the violating states.
        node: usize,
    },
    /// The earliest reachable undefined timed-subject evaluation.
    Undefined {
        /// The shortest path from an initial node to the deciding node.
        stem: Vec<usize>,
        /// The deciding node.
        node: usize,
        /// The timed-subject reading that was undefined.
        reading: fsl_replay::GraphUndefined,
        /// The `E(P)` nodes, when the form is `always possible`.
        ef_nodes: Vec<usize>,
    },
}

/// Decide a state-graph claim over the reachable zone graph.
///
/// # Errors
///
/// [`ZoneCheckRefusal`] when evaluating a predicate does not produce a
/// Boolean reading, when a path does not concretise, and
/// [`ZoneCheckRefusal::Stopped`] when `limits` or `cancel` stops the
/// decision, which then has no verdict.
#[implements(
    "FR-852-AC-8",
    "FR-852-AC-9",
    "FR-852-AC-10",
    "FR-852-AC-11",
    "FR-852-AC-16",
    "FR-852-AC-17",
    "FR-852-AC-18",
    "FR-852-AC-19",
    "FR-852-AC-24",
    "FR-852-AC-25",
    "FR-852-AC-26",
    "FR-852-AC-27",
    "FR-852-AC-1", "FR-852-AC-2", "FR-852-AC-3"
)]
pub fn decide_timed_state_graph(
    timed: &TimedSystem<'_, '_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    limits: ZoneCheckLimits,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let system = timed.system();
    let mut zones = Zones::new(timed, graph, limits, cancel)?;
    let target = match claim.form() {
        StateGraphForm::Possible { target } | StateGraphForm::AlwaysPossible { target, .. } => {
            target
        }
        StateGraphForm::UniquePath { .. } => {
            return decide_unique_path(system, &mut zones, graph, claim, over, cancel);
        }
    };
    let mut undefined = Vec::new();
    let p = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            predicate(
                system,
                target,
                claim,
                PredicateRole::Target,
                index,
                &node.state,
                over,
                cancel,
                &mut undefined,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    match claim.form() {
        StateGraphForm::Possible { .. } => Ok(prefer_undefined(
            system,
            graph,
            zones.possible(&p)?,
            undefined,
        )),
        StateGraphForm::AlwaysPossible { from, .. } => {
            let mut source_undefined = undefined;
            let q = graph
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    predicate(
                        system,
                        from,
                        claim,
                        PredicateRole::From,
                        index,
                        &node.state,
                        over,
                        cancel,
                        &mut source_undefined,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(prefer_undefined(
                system,
                graph,
                zones.always_possible(&q, &p)?,
                source_undefined,
            ))
        }
        StateGraphForm::UniquePath { .. } => unreachable!("handled above"),
    }
}

/// Decide a timed graph form while applying the request's selected FR-158
/// state constraint to every reachable node for undefined-evaluation
/// precedence.
#[implements("FR-852-AC-11", "FR-852-AC-19", "FR-852-AC-27")]
pub fn decide_timed_state_graph_with_constraint(
    timed: &TimedSystem<'_, '_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    constraint: Option<&str>,
    limits: ZoneCheckLimits,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let system = timed.system();
    let decision = decide_timed_state_graph(timed, graph, claim, over, limits, cancel)?;
    let Some(constraint) = constraint else {
        return Ok(decision);
    };
    let mut undefined = Vec::new();
    for (node, graph_node) in graph.nodes.iter().enumerate() {
        let ConstraintValue::Undefined { cause, .. } = system
            .constraint_value(&graph_node.state, constraint)
            .map_err(|_| ZoneCheckRefusal::Fault("state-graph-constraint"))?
        else {
            continue;
        };
        let (_, state_key) = system
            .state_key(&graph_node.state, fsl_foundation::IdentityLimits::default())
            .map_err(|_| ZoneCheckRefusal::Fault("state-graph-key"))?;
        undefined.push((
            node,
            fsl_replay::GraphUndefined {
                predicate: None,
                transition: None,
                state_key,
                locus: system.state_clause_locus(constraint),
                cause,
            },
        ));
    }
    Ok(prefer_undefined(system, graph, decision, undefined))
}

fn decide_unique_path(
    system: &ModelSystem<'_>,
    zones: &mut Zones<'_, '_, '_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let StateGraphForm::UniquePath { from, to } = claim.form() else {
        return Err(ZoneCheckRefusal::Fault("state-graph-form"));
    };
    let mut undefined = Vec::new();
    let x = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            predicate(
                system,
                from,
                claim,
                PredicateRole::From,
                index,
                &node.state,
                over,
                cancel,
                &mut undefined,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let y = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            predicate(
                system,
                to,
                claim,
                PredicateRole::Target,
                index,
                &node.state,
                over,
                cancel,
                &mut undefined,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let decision = zones.unique_path(&x, &y)?;
    Ok(prefer_undefined(system, graph, decision, undefined))
}

#[implements("FR-852-AC-9", "FR-852-AC-13", "FR-852-AC-17", "FR-852-AC-25")]
fn prefer_undefined(
    system: &ModelSystem<'_>,
    graph: &TimedStateGraph,
    decision: TimedStateGraphDecision,
    mut candidates: Vec<(usize, fsl_replay::GraphUndefined)>,
) -> TimedStateGraphDecision {
    for entry in &graph.undefined {
        let Some(cause) = entry.reading.cause.clone() else {
            continue;
        };
        let Some(state) = graph.nodes.get(entry.node).map(|node| &node.state) else {
            continue;
        };
        let Ok((_, state_key)) = system.state_key(state, fsl_foundation::IdentityLimits::default()) else {
            continue;
        };
        candidates.push((entry.node, fsl_replay::GraphUndefined {
            predicate: None,
            transition: entry.reading.transition.clone(),
            state_key,
            locus: entry.reading.location.clone(),
            cause,
        }));
    }
    let Some((node, reading)) = candidates.into_iter().min_by_key(|(node, _)| *node) else {
        return decision;
    };
    if matches!(&decision, TimedStateGraphDecision::Refuted { node: refuted, .. } if *refuted < node) {
        return decision;
    }
    TimedStateGraphDecision::Undefined {
        stem: path_from_initial(graph, node),
        node,
        reading,
        ef_nodes: match decision {
            TimedStateGraphDecision::Holds { ef_nodes, .. }
            | TimedStateGraphDecision::Refuted { ef_nodes, .. } => ef_nodes,
            TimedStateGraphDecision::Undefined { ef_nodes, .. } => ef_nodes,
            TimedStateGraphDecision::Unwritten { .. } => Vec::new(),
        },
    }
}

fn predicate(
    system: &ModelSystem<'_>,
    predicate: &CheckedStatePredicate,
    claim: &CheckedStateGraphClaim,
    role: PredicateRole,
    index: usize,
    state: &fsl_eval::simulation::model::ModelSimState,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
    undefined: &mut Vec<(usize, fsl_replay::GraphUndefined)>,
) -> Result<bool, ZoneCheckRefusal> {
    if predicate.is_constant_true() {
        return Ok(true);
    }
    let readings = if let Some(body) = predicate.predicate_body() {
        system.state_predicates_with_cancel(state, &[(body, over)], cancel)
    } else {
        system.state_graph_predicates(state, &[(claim, role, over)])
    }
    .map_err(|stop| ZoneCheckRefusal::Atom(format!("{stop:?}")))?;
    match readings.as_slice() {
        [PredicateReading::True] => Ok(true),
        [PredicateReading::False] => Ok(false),
        [PredicateReading::Undefined { cause, location }] => {
            let (_, state_key) = system
                .state_key(state, fsl_foundation::IdentityLimits::default())
                .map_err(|_| ZoneCheckRefusal::Fault("state-graph-key"))?;
            undefined.push((index, fsl_replay::GraphUndefined {
                predicate: Some(role),
                transition: None,
                state_key,
                locus: location.clone(),
                cause: cause.clone(),
            }));
            Ok(false)
        }
        [PredicateReading::Refused(_)] => Err(ZoneCheckRefusal::Fault("state-graph-predicate")),
        _ => Err(ZoneCheckRefusal::Fault("state-graph-predicate-count")),
    }
}

fn path_from_initial(graph: &TimedStateGraph, target: usize) -> Vec<usize> {
    graph
        .initials
        .iter()
        .find_map(|&initial| {
            let mut parent = vec![None; graph.nodes.len()];
            let mut seen = vec![false; graph.nodes.len()];
            *seen.get_mut(initial)? = true;
            let mut queue = VecDeque::from([initial]);
            while let Some(node) = queue.pop_front() {
                if node == target {
                    return path_from_parent(initial, node, &parent);
                }
                for (edge_index, edge) in graph
                    .edges
                    .iter()
                    .enumerate()
                    .filter(|(_, edge)| edge.from == node)
                {
                    if seen.get(edge.to).copied().unwrap_or(true) {
                        continue;
                    }
                    seen[edge.to] = true;
                    parent[edge.to] = Some((node, edge_index));
                    queue.push_back(edge.to);
                }
            }
            None
        })
        .unwrap_or_default()
}

fn path_from_parent(
    initial: usize,
    mut node: usize,
    parent: &[Option<(usize, usize)>],
) -> Option<Vec<usize>> {
    let mut path = Vec::new();
    while node != initial {
        let (previous, edge) = parent.get(node).copied().flatten()?;
        path.push(edge);
        node = previous;
    }
    path.reverse();
    Some(path)
}
