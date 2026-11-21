// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-243's integer-clock reading of a timed state graph.

use std::collections::{HashMap, VecDeque};

use filament_exact::{Cancel, Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ClockValuation, TimedState, TimedStop, TimedSystem};
use fsl_eval::simulation::StateKey;
use num_bigint::BigInt;
use num_integer::Integer as _;

/// A limit that stopped integer-clock graph expansion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalGraphLimit {
    /// Maximum number of states.
    States(u64),
    /// Maximum number of transitions, including one-unit delay edges.
    Transitions(u64),
    /// Maximum live clocks.
    Clocks(u64),
    /// Caller cancellation.
    Cancelled,
}

/// One edge of an integer-clock graph; `None` is a unit delay edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalGraphEdge {
    /// Source node.
    pub from: usize,
    /// Destination node.
    pub to: usize,
    /// Discrete transition, or none for one unit of delay.
    pub transition: Option<ModelTransition>,
}

/// The finite timed graph over whole clock units, with clocks capped one unit
/// above the largest model constant.
#[derive(Clone, Debug)]
pub struct DigitalStateGraph {
    /// The timed states in discovery order.
    pub nodes: Vec<TimedState>,
    /// Delay and discrete edges in discovery order.
    pub edges: Vec<DigitalGraphEdge>,
    /// Initial node indices.
    pub initials: Vec<usize>,
    /// One exact clock unit in the common denominator scale.
    pub unit: Rational,
}

/// Explore a closed timed subject on integer clock values.
pub fn explore_digital_state_graph(
    timed: &TimedSystem<'_, '_>,
    max_states: u64,
    max_transitions: u64,
    cancel: &Cancel,
) -> Result<DigitalStateGraph, DigitalGraphLimit> {
    let (unit, cap) = grid(timed);
    let mut graph = DigitalStateGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        initials: Vec::new(),
        unit: unit.clone(),
    };
    let mut indices = HashMap::<StateKey, usize>::new();
    let mut queue = VecDeque::new();
    for initial in timed.initial() {
        let state = capped(initial, &cap);
        let index = insert_state(
            timed,
            state,
            max_states,
            &mut graph,
            &mut indices,
            &mut queue,
        )?;
        graph.initials.push(index);
    }
    while let Some(from) = queue.pop_front() {
        if cancel.poll() {
            return Err(DigitalGraphLimit::Cancelled);
        }
        let state = graph.nodes[from].clone();
        if let Some(next) = timed
            .delay(&state, &unit)
            .map_err(|stop| limit_from_stop(stop, max_states))?
        {
            let next = capped(next, &cap);
            if timed
                .state_key(&next)
                .map_err(|_| DigitalGraphLimit::States(max_states))?
                != timed
                    .state_key(&state)
                    .map_err(|_| DigitalGraphLimit::States(max_states))?
            {
                let to = insert_state(
                    timed,
                    next,
                    max_states,
                    &mut graph,
                    &mut indices,
                    &mut queue,
                )?;
                insert_edge(from, to, None, max_transitions, &mut graph.edges)?;
            }
        }
        for (step, next) in timed
            .steps(&state)
            .map_err(|stop| limit_from_stop(stop, max_states))?
        {
            let to = insert_state(
                timed,
                capped(next, &cap),
                max_states,
                &mut graph,
                &mut indices,
                &mut queue,
            )?;
            insert_edge(
                from,
                to,
                Some(step.transition().clone()),
                max_transitions,
                &mut graph.edges,
            )?;
        }
    }
    Ok(graph)
}

fn insert_state(
    timed: &TimedSystem<'_, '_>,
    state: TimedState,
    max_states: u64,
    graph: &mut DigitalStateGraph,
    indices: &mut HashMap<StateKey, usize>,
    queue: &mut VecDeque<usize>,
) -> Result<usize, DigitalGraphLimit> {
    let key = timed
        .state_key(&state)
        .map_err(|_| DigitalGraphLimit::States(max_states))?;
    if let Some(&index) = indices.get(&key) {
        return Ok(index);
    }
    if u64::try_from(graph.nodes.len()).unwrap_or(u64::MAX) >= max_states {
        return Err(DigitalGraphLimit::States(max_states));
    }
    let index = graph.nodes.len();
    graph.nodes.push(state);
    indices.insert(key, index);
    queue.push_back(index);
    Ok(index)
}

fn insert_edge(
    from: usize,
    to: usize,
    transition: Option<ModelTransition>,
    max_transitions: u64,
    edges: &mut Vec<DigitalGraphEdge>,
) -> Result<(), DigitalGraphLimit> {
    if u64::try_from(edges.len()).unwrap_or(u64::MAX) >= max_transitions {
        return Err(DigitalGraphLimit::Transitions(max_transitions));
    }
    edges.push(DigitalGraphEdge {
        from,
        to,
        transition,
    });
    Ok(())
}

fn capped(state: TimedState, cap: &Rational) -> TimedState {
    let clocks = ClockValuation::new(
        state
            .clocks()
            .values()
            .iter()
            .map(|value| value.clone().min(cap.clone()))
            .collect(),
    );
    TimedState::new(state.discrete().clone(), clocks)
}

fn grid(timed: &TimedSystem<'_, '_>) -> (Rational, Rational) {
    let model = timed.model();
    let mut constants: Vec<Rational> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .flat_map(|part| {
            part.clocks()
                .iter()
                .map(|constraint| constraint.bound().clone())
        })
        .chain(
            model
                .guards()
                .iter()
                .flat_map(|guard| guard.parts())
                .flat_map(|part| part.families())
                .flat_map(|family| family.parts())
                .flat_map(|part| {
                    part.clocks()
                        .iter()
                        .map(|constraint| constraint.bound().clone())
                }),
        )
        .chain(model.invariants().iter().flat_map(|invariant| {
            invariant
                .bounds()
                .iter()
                .map(|constraint| constraint.bound().clone())
        }))
        .chain(
            model
                .resets()
                .iter()
                .flat_map(|resets| resets.resets().iter().map(|reset| reset.value().clone())),
        )
        .collect();
    let scale = constants.iter().fold(BigInt::from(1), |scale, value| {
        scale.lcm(value.denominator().as_big())
    });
    let unit = Rational::new(Integer::one(), Integer::from_big(scale))
        .unwrap_or_else(|_| Rational::from_integer(Integer::one()));
    let largest = constants
        .drain(..)
        .max()
        .unwrap_or_else(|| Rational::from_integer(Integer::zero()));
    let cap = largest.add(&unit);
    (unit, cap)
}

fn limit_from_stop(stop: TimedStop, max_states: u64) -> DigitalGraphLimit {
    match stop {
        TimedStop::MaxClocks { max_clocks } => DigitalGraphLimit::Clocks(max_clocks),
        _ => DigitalGraphLimit::States(max_states),
    }
}
