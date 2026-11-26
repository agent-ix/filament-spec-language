// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's whole-model runner: reads a checked graph's field declarations
//! and operation clauses, builds the labelled state graph of the declared
//! fragment, and decides a state-graph claim or a bounded probability claim on
//! it by its definition.

use std::collections::VecDeque;

use filament_exact::{Integer, Rational, Value};
use fsl_foundation::bound::TemporalUpper;
use fsl_semantics::check::{
    BinaryTemporal, Bound, CheckedGraph, CheckedInterval, CheckedProbabilisticClaim,
    CheckedStateGraphClaim, CheckedStatePredicate, Event, FormulaNode, FormulaNodeId, ProbForm,
    StateGraphForm, UnaryTemporal,
};
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, NativeValueType, ScalarValueDomain, ValueTypeRef,
};
use ix_trace_rs::implements;

use crate::interpreter::{evaluate_boolean, ReferenceFiniteState, ReferenceTransitionGraph};
use crate::ReferenceRefusal;

/// The pairwise relation reads every ordered pair of valuations.
const MAX_RELATION_STATES: usize = 4096;

fn refuse(construct: impl Into<String>) -> ReferenceRefusal {
    ReferenceRefusal {
        construct: construct.into(),
    }
}

/// One object of the model: its id, its object type's name and the value a
/// clause reads for a reference to it.
#[derive(Clone, Debug)]
pub struct ReferenceObject {
    /// The object's id, the first component of its state keys.
    pub id: String,
    /// The object type's declared name.
    pub type_name: String,
    /// The reference value bound to a clause's receiver or `over` slot.
    pub reference: Value,
}

/// One parameterless operation applied to one receiver object.
#[derive(Clone, Debug)]
pub struct ReferenceOperation {
    /// The operation's declared name.
    pub operation: String,
    /// The id of the receiver object.
    pub receiver: String,
}

/// The labelled state graph a checked model denotes: every valuation of the
/// declared fields, one edge per (source, operation, post-state) the checked
/// clauses allow, and the initial valuations.
#[derive(Clone, Debug)]
pub struct ReferenceModel {
    states: Vec<ReferenceFiniteState>,
    edges: Vec<Vec<(usize, usize)>>,
    initial: Vec<usize>,
}

fn last_segment(node: &str) -> &str {
    node.rsplit('/').next().unwrap_or(node)
}

/// The finite values of one field, from its declared type.
fn field_values(
    package: &DomainPackage,
    value_type: &ValueTypeRef,
) -> Result<Vec<Value>, ReferenceRefusal> {
    match value_type {
        ValueTypeRef::Native(NativeValueType::Boolean) => {
            Ok(vec![Value::Boolean(false), Value::Boolean(true)])
        }
        ValueTypeRef::Package(key) => {
            for record in &package.records {
                match record {
                    DomainPackageRecord::ScalarType(found) if found.key == *key => {
                        let ScalarValueDomain::Integer { lower, upper } = &found.domain else {
                            return Err(refuse("field type that is not finite"));
                        };
                        let lower = i64::try_from(*lower)
                            .map_err(|_| refuse("integer bound outside 64 bits"))?;
                        let upper = i64::try_from(*upper)
                            .map_err(|_| refuse("integer bound outside 64 bits"))?;
                        return Ok((lower..=upper)
                            .map(|value| Value::Integer(Integer::from(value)))
                            .collect());
                    }
                    DomainPackageRecord::Enum(found) if found.key == *key => {
                        return Err(refuse("enumeration field in a state enumeration"));
                    }
                    _ => {}
                }
            }
            Err(refuse("field type that is not finite"))
        }
        _ => Err(refuse("field type that is not finite")),
    }
}

impl ReferenceModel {
    /// Reads the field domains of `objects` from the graph's packages,
    /// enumerates every valuation, and derives each listed operation's edges
    /// from its checked clauses. `initial` lists the starting valuations.
    pub fn explore(
        graph: &CheckedGraph,
        objects: &[ReferenceObject],
        operations: &[ReferenceOperation],
        initial: &[ReferenceFiniteState],
    ) -> Result<Self, ReferenceRefusal> {
        crate::validate_tiny_fragment(graph)?;
        let mut domains = Vec::new();
        for object in objects {
            for package in graph
                .selected_model_views()
                .values()
                .map(|view| view.domain_package())
                .chain(graph.source_packages())
            {
                for record in &package.records {
                    let DomainPackageRecord::FieldMember(field) = record else {
                        continue;
                    };
                    if last_segment(&field.owner.node) != object.type_name {
                        continue;
                    }
                    domains.push((
                        object.id.clone(),
                        last_segment(&field.key.node).to_owned(),
                        field_values(package, &field.value_type)?,
                    ));
                }
            }
        }
        domains.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
        domains.dedup_by(|left, right| (&left.0, &left.1) == (&right.0, &right.1));
        let states = ReferenceFiniteState::enumerate(domains)?;
        if states.len() > MAX_RELATION_STATES {
            return Err(refuse(
                "pairwise transition relation over more than 4096 valuations",
            ));
        }
        let mut edges = vec![Vec::new(); states.len()];
        for (index, operation) in operations.iter().enumerate() {
            let receiver = objects
                .iter()
                .find(|object| object.id == operation.receiver)
                .ok_or_else(|| refuse("operation receiver is not an object of the model"))?;
            let relation = ReferenceTransitionGraph::from_operation(
                graph,
                &operation.operation,
                std::slice::from_ref(&receiver.reference),
                states.clone(),
            )?;
            for (source, outgoing) in edges.iter_mut().enumerate() {
                for destination in relation.successors(source).unwrap_or_default() {
                    outgoing.push((index, *destination));
                }
            }
        }
        let initial = initial
            .iter()
            .map(|start| {
                states
                    .iter()
                    .position(|state| state.same_valuation(start))
                    .ok_or_else(|| refuse("initial valuation outside the declared domains"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            states,
            edges,
            initial,
        })
    }

    /// The enumerated valuations.
    pub fn states(&self) -> &[ReferenceFiniteState] {
        &self.states
    }

    /// The outgoing `(operation index, post-state)` edges of a valuation.
    pub fn edges(&self, state: usize) -> Option<&[(usize, usize)]> {
        self.edges.get(state).map(Vec::as_slice)
    }

    /// The initial valuations' indices.
    pub fn initial(&self) -> &[usize] {
        &self.initial
    }

    fn labels(
        &self,
        graph: &CheckedGraph,
        predicate: &CheckedStatePredicate,
        instance: &Value,
    ) -> Result<Vec<bool>, ReferenceRefusal> {
        match predicate {
            CheckedStatePredicate::ConstantTrue => Ok(vec![true; self.states.len()]),
            CheckedStatePredicate::Checked(body) => self
                .states
                .iter()
                .map(|state| {
                    evaluate_boolean(
                        graph,
                        body.body(),
                        std::slice::from_ref(instance),
                        state,
                        state,
                    )
                })
                .collect(),
            CheckedStatePredicate::Package(_) => {
                Err(refuse("state-graph predicate without a source body"))
            }
        }
    }

    /// Decides one state-graph claim by ADR-022 GM-4 and GM-5, for every
    /// instance of its `over` type: `instances` lists their reference values.
    #[implements("FR-1051")]
    pub fn decide_state_graph_claim(
        &self,
        graph: &CheckedGraph,
        claim: &CheckedStateGraphClaim,
        instances: &[Value],
    ) -> Result<bool, ReferenceRefusal> {
        if instances.is_empty() {
            return Err(refuse("state-graph claim over no instance"));
        }
        let successors: Vec<Vec<usize>> = self
            .edges
            .iter()
            .map(|edges| edges.iter().map(|(_, destination)| *destination).collect())
            .collect();
        let reachable = reachable_from(&successors, &self.initial);
        let mut holds = true;
        for instance in instances {
            holds &= match claim.form() {
                StateGraphForm::Possible { target } => {
                    let labels = self.labels(graph, target, instance)?;
                    let can_reach = can_reach_target(&successors, &labels);
                    self.initial.iter().all(|state| can_reach[*state])
                }
                StateGraphForm::AlwaysPossible { from, target } => {
                    let from = self.labels(graph, from, instance)?;
                    let can_reach =
                        can_reach_target(&successors, &self.labels(graph, target, instance)?);
                    (0..self.states.len())
                        .all(|state| !(reachable[state] && from[state]) || can_reach[state])
                }
                StateGraphForm::UniquePath { from, to } => {
                    let from = self.labels(graph, from, instance)?;
                    let to = self.labels(graph, to, instance)?;
                    (0..self.states.len()).all(|state| {
                        !(reachable[state] && from[state])
                            || has_exactly_one_path(&self.edges, &to, state)
                    })
                }
            };
        }
        Ok(holds)
    }
}

/// The nodes reachable from `start` by zero or more edges.
fn reachable_from(successors: &[Vec<usize>], start: &[usize]) -> Vec<bool> {
    let mut seen = vec![false; successors.len()];
    let mut pending: VecDeque<usize> = start.iter().copied().collect();
    while let Some(node) = pending.pop_front() {
        if std::mem::replace(&mut seen[node], true) {
            continue;
        }
        pending.extend(successors[node].iter().copied());
    }
    seen
}

/// ADR-022 GM-4's `E(P)`: the nodes from which some path of zero or more
/// edges reaches a node where `target` holds, the least fixpoint of
/// `Z = P union pre(Z)`.
pub fn can_reach_target(successors: &[Vec<usize>], target: &[bool]) -> Vec<bool> {
    let mut inside = target.to_vec();
    loop {
        let mut grew = false;
        for (node, outgoing) in successors.iter().enumerate() {
            if !inside[node] && outgoing.iter().any(|next| inside[*next]) {
                inside[node] = true;
                grew = true;
            }
        }
        if !grew {
            return inside;
        }
    }
}

/// ADR-022 GM-5: whether `start` has exactly one path to `to`. A path is an
/// edge sequence ending at the first node where `to` holds; `edges` lists
/// each node's `(transition, post-state)` pairs. A cycle on a path from
/// `start` to `to` gives unboundedly many paths.
pub fn has_exactly_one_path(edges: &[Vec<(usize, usize)>], to: &[bool], start: usize) -> bool {
    if to[start] {
        return true;
    }
    let count = edges.len();
    // Interior nodes: not `to`, reachable from `start` without passing `to`,
    // and able to reach `to` without passing it first.
    let mut forward = vec![false; count];
    let mut pending = vec![start];
    while let Some(node) = pending.pop() {
        if std::mem::replace(&mut forward[node], true) {
            continue;
        }
        pending.extend(
            edges[node]
                .iter()
                .map(|(_, next)| *next)
                .filter(|next| !to[*next]),
        );
    }
    let mut backward = vec![false; count];
    loop {
        let mut grew = false;
        for node in 0..count {
            if !to[node]
                && !backward[node]
                && edges[node]
                    .iter()
                    .any(|(_, next)| to[*next] || backward[*next])
            {
                backward[node] = true;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let interior: Vec<bool> = (0..count).map(|n| forward[n] && backward[n]).collect();
    if !interior[start] {
        return false;
    }
    // Paths from each interior node, capped at two; an edge into an interior
    // node that is still being counted closes a cycle.
    let mut paths: Vec<Option<u8>> = vec![None; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<(usize, usize, u8)> = vec![(start, 0, 0)];
    on_stack[start] = true;
    while let Some((node, cursor, total)) = stack.pop() {
        if cursor == edges[node].len() {
            on_stack[node] = false;
            paths[node] = Some(total);
            if let Some(parent) = stack.last_mut() {
                parent.2 = parent.2.saturating_add(total).min(2);
            }
            continue;
        }
        let next = edges[node][cursor].1;
        stack.push((node, cursor + 1, total));
        if to[next] {
            let parent = stack.last_mut().expect("just pushed");
            parent.2 = parent.2.saturating_add(1).min(2);
        } else if interior[next] {
            if on_stack[next] {
                return false;
            }
            match paths[next] {
                Some(known) => {
                    let parent = stack.last_mut().expect("just pushed");
                    parent.2 = parent.2.saturating_add(known).min(2);
                }
                None => {
                    on_stack[next] = true;
                    stack.push((next, 0, 0));
                }
            }
        }
    }
    paths[start] == Some(1)
}

/// The paths a bounded claim may enumerate before the reference refuses.
const MAX_PATHS: usize = 1 << 20;

/// Decides a bounded-profile probability claim over a single-action chain by
/// enumerating every path to the claim's horizon and summing the exact mass
/// of the paths on which the formula holds, then comparing with the bound.
/// `states` are the chain's valuations, indexed as `chain` indexes them.
#[implements("FR-1051")]
pub fn decide_bounded_probability_claim(
    graph: &CheckedGraph,
    claim: &CheckedProbabilisticClaim,
    chain: &crate::TinyMdp,
    states: &[ReferenceFiniteState],
    initial: usize,
    instance: &Value,
) -> Result<bool, ReferenceRefusal> {
    let ProbForm::Probability {
        bound,
        event: Event::Formula {
            formula, horizon, ..
        },
    } = &claim.form
    else {
        return Err(refuse(
            "probabilistic claim form outside bounded probability",
        ));
    };
    if chain.actions.iter().any(|actions| actions.len() != 1) {
        return Err(refuse("scheduler choice in a bounded probability claim"));
    }
    if initial >= states.len() || states.len() != chain.actions.len() {
        return Err(refuse("initial state outside the chain"));
    }
    let horizon = usize::try_from(*horizon).map_err(|_| refuse("horizon beyond the platform"))?;
    let zero = Rational::from_integer(Integer::zero());
    let mut mass = zero.clone();
    let mut paths = 0_usize;
    let mut pending = vec![(vec![initial], Rational::from_integer(Integer::one()))];
    while let Some((path, weight)) = pending.pop() {
        if path.len() == horizon + 1 {
            paths += 1;
            if paths > MAX_PATHS {
                return Err(refuse("more than 2^20 paths to the horizon"));
            }
            let trace: Vec<&ReferenceFiniteState> = path.iter().map(|s| &states[*s]).collect();
            if formula_holds(
                graph,
                formula,
                formula.formula().root(),
                0,
                &trace,
                instance,
            )? {
                mass = mass.add(&weight);
            }
            continue;
        }
        let last = *path.last().expect("a path has its initial state");
        for (destination, probability) in &chain.actions[last][0] {
            let mut next = path.clone();
            next.push(*destination);
            pending.push((next, weight.mul(probability)));
        }
    }
    Ok(match bound {
        Bound::AtLeast(least) => mass >= *least,
        Bound::AtMost(most) => mass <= *most,
    })
}

/// Whether the formula node holds at position `at` of a finite trace.
fn formula_holds(
    graph: &CheckedGraph,
    checked: &fsl_semantics::check::CheckedTemporalFormula,
    node: FormulaNodeId,
    at: usize,
    trace: &[&ReferenceFiniteState],
    instance: &Value,
) -> Result<bool, ReferenceRefusal> {
    let recurse = |child: FormulaNodeId, position: usize| {
        formula_holds(graph, checked, child, position, trace, instance)
    };
    let window = |interval: &Option<CheckedInterval>| match interval {
        Some(CheckedInterval::Steps(steps)) => {
            let lower = usize::try_from(steps.lower())
                .map_err(|_| refuse("interval bound beyond the platform"))?;
            let upper = match steps.upper() {
                TemporalUpper::Finite(upper) => usize::try_from(upper)
                    .map_err(|_| refuse("interval bound beyond the platform"))?,
                TemporalUpper::Open => return Err(refuse("open interval in a bounded claim")),
            };
            Ok((
                at.saturating_add(lower),
                at.saturating_add(upper).min(trace.len() - 1),
            ))
        }
        _ => Err(refuse("operator without a step interval")),
    };
    match &checked.formula().nodes()[node.index()] {
        FormulaNode::Constant(value) => Ok(*value),
        FormulaNode::Holds { .. } => {
            let predicate = checked
                .condition(node.index())
                .ok_or_else(|| refuse("holds atom without a checked condition"))?;
            let body = predicate
                .body()
                .ok_or_else(|| refuse("claim predicate without a source body"))?;
            evaluate_boolean(
                graph,
                body,
                std::slice::from_ref(instance),
                trace[at],
                trace[at],
            )
        }
        FormulaNode::Not(operand) => Ok(!recurse(*operand, at)?),
        FormulaNode::And(left, right) => Ok(recurse(*left, at)? && recurse(*right, at)?),
        FormulaNode::Or(left, right) => Ok(recurse(*left, at)? || recurse(*right, at)?),
        FormulaNode::Implies(left, right) => Ok(!recurse(*left, at)? || recurse(*right, at)?),
        FormulaNode::Unary {
            operator: UnaryTemporal::Eventually,
            interval,
            operand,
        } => {
            let (first, last) = window(interval)?;
            for position in first..=last {
                if recurse(*operand, position)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval,
            operand,
        } => {
            let (first, last) = window(interval)?;
            for position in first..=last {
                if !recurse(*operand, position)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        FormulaNode::Binary {
            operator: BinaryTemporal::Until,
            interval,
            left,
            right,
        } => {
            let (first, last) = window(interval)?;
            for position in first..=last {
                if recurse(*right, position)? {
                    let mut held = true;
                    for before in at..position {
                        held &= recurse(*left, before)?;
                    }
                    if held {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
        _ => Err(refuse("formula node outside the reference fragment")),
    }
}
#[cfg(test)]
mod tests {
    use super::{can_reach_target, has_exactly_one_path};
    use ix_trace_rs::trace;

    fn graph(rows: &[&[usize]]) -> Vec<Vec<(usize, usize)>> {
        rows.iter()
            .map(|row| row.iter().copied().enumerate().collect())
            .collect()
    }

    /// Diamond 0 -> {1, 2} -> 3: two distinct paths from 0 to 3, one from
    /// either middle node.
    #[trace("FR-1051")]
    #[test]
    fn a_diamond_has_two_paths_and_each_branch_one() {
        let edges = graph(&[&[1, 2], &[3], &[3], &[]]);
        let to = [false, false, false, true];
        assert!(!has_exactly_one_path(&edges, &to, 0));
        assert!(has_exactly_one_path(&edges, &to, 1));
        assert!(has_exactly_one_path(&edges, &to, 2));
        assert!(has_exactly_one_path(&edges, &to, 3));
    }

    /// 0 -> 1 -> 2 with a self-loop on 1: infinitely many paths from 0.
    /// Without the loop exactly one.
    #[trace("FR-1051")]
    #[test]
    fn a_cycle_on_the_path_gives_unboundedly_many_paths() {
        let to = [false, false, true];
        assert!(!has_exactly_one_path(&graph(&[&[1], &[1, 2], &[]]), &to, 0));
        assert!(has_exactly_one_path(&graph(&[&[1], &[2], &[]]), &to, 0));
    }

    /// A path ends at its first target node, so a target beyond another
    /// target is not a second path; a node with no path has zero, not one.
    #[trace("FR-1051")]
    #[test]
    fn a_path_stops_at_the_first_target_and_zero_paths_is_not_one() {
        let edges = graph(&[&[1], &[2], &[]]);
        assert!(has_exactly_one_path(&edges, &[false, true, true], 0));
        assert!(!has_exactly_one_path(
            &graph(&[&[], &[]]),
            &[false, true],
            0
        ));
    }

    /// 0 -> 1 -> 2 and 3 alone: `E(P)` for `P = {2}` is {0, 1, 2}.
    #[trace("FR-1051")]
    #[test]
    fn backward_fixpoint_reaches_exactly_the_predecessors_of_the_target() {
        let successors = vec![vec![1], vec![2], vec![], vec![3]];
        assert_eq!(
            can_reach_target(&successors, &[false, false, true, false]),
            vec![true, true, true, false]
        );
    }
}
