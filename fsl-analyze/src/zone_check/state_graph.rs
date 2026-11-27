// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-graph forms over the symbolic timed graph (FSpec FR-712).

use std::collections::VecDeque;

use filament_exact::{Cancel, ObjectReference};
use fsl_eval::simulation::model::{ModelSystem, PredicateReading};
use fsl_eval::simulation::ConstraintValue;
use fsl_semantics::check::{
    CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole, StateGraphForm,
};
use ix_trace_rs::implements;

use super::search::{TimedStateGraph, ZoneCheckRefusal};

/// A state-graph decision over a timed zone graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedStateGraphDecision {
    /// The claim holds. `witness_paths` is populated for `possible`; `ef_nodes`
    /// is populated for `always possible`.
    Holds {
        /// The discrete-step witness path for each initial state.
        witness_paths: Vec<Vec<usize>>,
        /// The nodes in the least fixpoint `E(P)`.
        ef_nodes: Vec<usize>,
    },
    /// The claim is refuted by one initial or reachable state-graph node.
    Refuted {
        /// The shortest path from any initial node to the deciding node.
        stem: Vec<usize>,
        /// The deciding node.
        node: usize,
        /// The `E(P)` nodes, when the form is `always possible`.
        ef_nodes: Vec<usize>,
        /// Two distinct paths for a `unique path` refutation; absent for a
        /// possible or always-possible refutation.
        paths: Option<(Vec<usize>, Vec<usize>)>,
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

/// Decide `possible` and `always possible` over the reachable zone graph.
///
/// # Errors
///
/// [`ZoneCheckRefusal`] when evaluating a predicate does not produce a
/// Boolean reading.
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
    "FR-852-AC-27"
)]
pub fn decide_timed_state_graph(
    system: &ModelSystem<'_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let target = match claim.form() {
        StateGraphForm::Possible { target } | StateGraphForm::AlwaysPossible { target, .. } => {
            target
        }
        StateGraphForm::UniquePath { .. } => {
            return decide_unique_path(system, graph, claim, over, cancel);
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
    let ef_nodes = predecessors_of(graph, &p);
    match claim.form() {
        StateGraphForm::Possible { .. } => {
            let mut witness_paths = Vec::with_capacity(graph.initials.len());
            for &initial in &graph.initials {
                let Some(path) = path_to(graph, initial, &p) else {
                    return Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Refuted {
                        stem: Vec::new(),
                        node: initial,
                        ef_nodes: Vec::new(),
                        paths: None,
                    }, undefined));
                };
                witness_paths.push(path);
            }
            Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Holds {
                witness_paths,
                ef_nodes: Vec::new(),
            }, undefined))
        }
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
            for (node, (is_from, can_reach)) in q.iter().zip(&ef_nodes).enumerate() {
                if *is_from && !*can_reach {
                    return Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Refuted {
                        stem: path_from_initial(graph, node),
                        node,
                        ef_nodes: indices(&ef_nodes),
                        paths: None,
                    }, source_undefined));
                }
            }
            Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Holds {
                witness_paths: Vec::new(),
                ef_nodes: indices(&ef_nodes),
            }, source_undefined))
        }
        StateGraphForm::UniquePath { .. } => unreachable!("handled above"),
    }
}

/// Decide a timed graph form while applying the request's selected FR-158
/// state constraint to every reachable node for undefined-evaluation
/// precedence.
#[implements("FR-852-AC-11", "FR-852-AC-19", "FR-852-AC-27")]
pub fn decide_timed_state_graph_with_constraint(
    system: &ModelSystem<'_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    constraint: Option<&str>,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let decision = decide_timed_state_graph(system, graph, claim, over, cancel)?;
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
    for (node, _) in x.iter().enumerate().filter(|(_, is_from)| **is_from) {
        let dfa = determinize(graph, node, &y);
        let words = first_accepted_words(&dfa);
        if words.len() != 1 {
            let paths = match words.as_slice() {
                [first, second, ..] => Some((
                    path_for_word(graph, node, first, &y)
                        .ok_or(ZoneCheckRefusal::Fault("state-graph-first-path"))?,
                    path_for_word(graph, node, second, &y)
                        .ok_or(ZoneCheckRefusal::Fault("state-graph-second-path"))?,
                )),
                _ => None,
            };
            return Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Refuted {
                stem: path_from_initial(graph, node),
                node,
                ef_nodes: Vec::new(),
                paths,
            }, undefined));
        }
    }
    Ok(prefer_undefined(system, graph, TimedStateGraphDecision::Holds {
        witness_paths: Vec::new(),
        ef_nodes: Vec::new(),
    }, undefined))
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
        },
    }
}

#[derive(Clone)]
struct DfaTransition {
    symbol: filament_semantic_value::model_transition::ModelTransition,
    target: usize,
}

struct Dfa {
    transitions: Vec<Vec<DfaTransition>>,
    accepting: Vec<bool>,
}

fn determinize(graph: &TimedStateGraph, start: usize, target: &[bool]) -> Dfa {
    let mut subsets = vec![vec![start]];
    let mut transitions = vec![Vec::new()];
    let mut accepting = vec![false];
    let mut at = 0;
    while at < subsets.len() {
        let subset = subsets[at].clone();
        let mut symbols: Vec<(
            filament_semantic_value::model_transition::ModelTransition,
            Vec<usize>,
        )> = Vec::new();
        for node in subset {
            accepting[at] |= target.get(node).copied().unwrap_or(false);
            for edge in graph.edges.iter().filter(|edge| edge.from == node) {
                let Some((_, destinations)) = symbols
                    .iter_mut()
                    .find(|(symbol, _)| symbol == &edge.transition)
                else {
                    symbols.push((edge.transition.clone(), vec![edge.to]));
                    continue;
                };
                destinations.push(edge.to);
            }
        }
        for (symbol, mut destinations) in symbols {
            destinations.sort_unstable();
            destinations.dedup();
            let next = if let Some(found) = subsets.iter().position(|old| old == &destinations) {
                found
            } else {
                let found = subsets.len();
                subsets.push(destinations);
                transitions.push(Vec::new());
                accepting.push(false);
                found
            };
            transitions[at].push(DfaTransition {
                symbol,
                target: next,
            });
        }
        at += 1;
    }
    Dfa {
        transitions,
        accepting,
    }
}

fn first_accepted_words(
    dfa: &Dfa,
) -> Vec<Vec<filament_semantic_value::model_transition::ModelTransition>> {
    let mut can_accept = dfa.accepting.clone();
    loop {
        let mut changed = false;
        for (from, outgoing) in dfa.transitions.iter().enumerate() {
            if !can_accept[from]
                && outgoing
                    .iter()
                    .any(|edge| can_accept.get(edge.target).copied().unwrap_or(false))
            {
                can_accept[from] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let max_len = dfa.transitions.len().saturating_mul(2).saturating_add(2);
    let mut queue = VecDeque::from([(0_usize, Vec::new())]);
    let mut words = Vec::new();
    while let Some((state, word)) = queue.pop_front() {
        if dfa.accepting.get(state).copied().unwrap_or(false) {
            words.push(word.clone());
            if words.len() == 2 {
                break;
            }
        }
        if word.len() >= max_len {
            continue;
        }
        for edge in dfa.transitions.get(state).into_iter().flatten() {
            if !can_accept.get(edge.target).copied().unwrap_or(false) {
                continue;
            }
            let mut next = word.clone();
            next.push(edge.symbol.clone());
            queue.push_back((edge.target, next));
        }
    }
    words
}

fn path_for_word(
    graph: &TimedStateGraph,
    start: usize,
    word: &[filament_semantic_value::model_transition::ModelTransition],
    target: &[bool],
) -> Option<Vec<usize>> {
    let mut queue = VecDeque::from([(start, 0_usize, Vec::new())]);
    let mut seen = vec![vec![false; word.len() + 1]; graph.nodes.len()];
    seen.get_mut(start)?.get_mut(0).map(|seen| *seen = true)?;
    while let Some((node, at, path)) = queue.pop_front() {
        if at == word.len() {
            if target.get(node).copied().unwrap_or(false) {
                return Some(path);
            }
            continue;
        }
        for (edge_index, edge) in graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node && edge.transition == word[at])
        {
            let next_at = at + 1;
            if seen
                .get(edge.to)
                .and_then(|positions| positions.get(next_at))
                .copied()
                .unwrap_or(true)
            {
                continue;
            }
            seen[edge.to][next_at] = true;
            let mut next_path = path.clone();
            next_path.push(edge_index);
            queue.push_back((edge.to, next_at, next_path));
        }
    }
    None
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

fn predecessors_of(graph: &TimedStateGraph, target: &[bool]) -> Vec<bool> {
    let mut reaches = target.to_vec();
    let mut incoming = vec![Vec::new(); graph.nodes.len()];
    for edge in &graph.edges {
        if let Some(nodes) = incoming.get_mut(edge.to) {
            nodes.push(edge.from);
        }
    }
    let mut queue: VecDeque<usize> = reaches
        .iter()
        .enumerate()
        .filter_map(|(index, found)| found.then_some(index))
        .collect();
    while let Some(node) = queue.pop_front() {
        for &predecessor in incoming.get(node).into_iter().flatten() {
            if let Some(found) = reaches.get_mut(predecessor) {
                if !*found {
                    *found = true;
                    queue.push_back(predecessor);
                }
            }
        }
    }
    reaches
}

fn path_to(graph: &TimedStateGraph, initial: usize, target: &[bool]) -> Option<Vec<usize>> {
    if target.get(initial).copied().unwrap_or(false) {
        return Some(Vec::new());
    }
    let mut parent = vec![None; graph.nodes.len()];
    let mut seen = vec![false; graph.nodes.len()];
    *seen.get_mut(initial)? = true;
    let mut queue = VecDeque::from([initial]);
    while let Some(node) = queue.pop_front() {
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
            if target.get(edge.to).copied().unwrap_or(false) {
                return path_from_parent(initial, edge.to, &parent);
            }
            queue.push_back(edge.to);
        }
    }
    None
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

fn indices(bits: &[bool]) -> Vec<usize> {
    bits.iter()
        .enumerate()
        .filter_map(|(index, yes)| yes.then_some(index))
        .collect()
}
