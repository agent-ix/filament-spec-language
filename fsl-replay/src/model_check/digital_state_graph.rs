// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-243's integer-clock reading of a timed state graph.

use std::collections::{HashMap, VecDeque};

use filament_exact::{Cancel, Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{
    ClockValuation, TimedState, TimedStop, TimedSystem, UndecidedReading,
};
use fsl_eval::simulation::StateKey;
use fsl_eval::simulation::TransitionSystem;
use num_bigint::BigInt;
use num_integer::Integer as _;
use ix_trace_rs::implements;

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
    /// Undefined timed evaluations at reachable nodes, in discovery order.
    pub undefined: Vec<DigitalGraphUndefined>,
    /// One exact clock unit in the common denominator scale.
    pub unit: Rational,
}

/// An undefined timed evaluation encountered while exploring a digital node.
#[derive(Clone, Debug)]
pub struct DigitalGraphUndefined {
    /// The graph node where evaluation was undefined.
    pub node: usize,
    /// The timed member, optional transition identity, and cause.
    pub reading: UndecidedReading,
}

/// Explore a closed timed subject on integer clock values.
#[implements(
    "FR-852-AC-12",
    "FR-852-AC-13",
    "FR-852-AC-14",
    "FR-852-AC-15",
    "FR-852-AC-20",
    "FR-852-AC-21",
    "FR-852-AC-22",
    "FR-852-AC-23",
    "FR-852-AC-28",
    "FR-852-AC-29",
    "FR-852-AC-30",
    "FR-852-AC-31"
)]
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
        undefined: Vec::new(),
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
        let steps = match timed.steps(&state) {
            Ok(steps) => steps,
            Err(TimedStop::Undecided(reading)) if reading.cause.is_some() => {
                graph.undefined.push(DigitalGraphUndefined {
                    node: from,
                    reading: *reading,
                });
                Vec::new()
            }
            Err(stop) => return Err(limit_from_stop(stop, max_states)),
        };
        if !graph.undefined.iter().any(|undefined| undefined.node == from) {
            let system = timed.system();
            if let Ok(expansion) = system.successors(state.discrete()) {
                if let Ok(fsl_eval::simulation::model::TerminalClass::Undefined {
                    model,
                    cause,
                }) = system.terminal_class(state.discrete(), !expansion.successors.is_empty())
                {
                    graph.undefined.push(DigitalGraphUndefined {
                        node: from,
                        reading: UndecidedReading {
                            site: fsl_eval::simulation::model::TimedSite::Terminal {
                                model: model.identity().to_owned(),
                            },
                            object: None,
                            cause: Some(cause),
                            transition: None,
                            location: system.terminal_locus(&model),
                        },
                    });
                }
            }
        }
        for (step, next) in steps {
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
