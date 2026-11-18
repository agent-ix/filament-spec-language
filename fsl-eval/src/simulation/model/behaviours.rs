// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-931 "Cross-evaluation": every finite behaviour of a model subject of 1
//! to `max_steps` steps, each read as its own closed trace.
//!
//! A behaviour of `j` steps is a path of `j` transitions from an initial
//! state, its `j + 1` states its positions. The behaviours are listed by
//! length, then by initial state in FR-101's admission order, then in FR-101's
//! successor order at each state, so a shorter behaviour is its own entry and
//! never a prefix read of a longer one. A state with no successor ends every
//! path through it. The walk holds one path at a time: memory grows with
//! `max_steps` and the branching at each state on the path, never with the
//! number of behaviours.

use std::ops::ControlFlow;

use filament_exact::{Charge, ChargePoint, Incomplete, Meter};
use filament_semantic_value::model_transition::ModelTransition;

use super::{check_model_bounds, identity_limits, ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::key::StateKey;
use crate::simulation::not_simulated::NotSimulated;
use crate::simulation::order::{expand, sorted_initial, Expanded, OrderedSuccessor};

/// One position of a behaviour: the state and its full canonical key.
#[derive(Clone, Debug)]
pub struct BehaviourState {
    /// The state.
    pub state: ModelSimState,
    /// Its FR-101 state key.
    pub key: StateKey,
}

/// Why [`model_behaviours`] stopped.
#[derive(Clone, Debug)]
pub enum BehavioursEnd<B> {
    /// Every behaviour of 1 to `max_steps` steps was visited.
    Exhausted,
    /// The meter refused an expansion: the request's work budget ran out.
    Meter(Incomplete),
    /// The system stopped an expansion.
    Stopped(ExpansionStop),
    /// The visitor stopped the walk.
    Broken(B),
}

/// One expanded state's ordered successors, consumed in order.
type Frame = std::vec::IntoIter<OrderedSuccessor<ModelTransition, ModelSimState>>;

/// Visit every behaviour of `system` of 1 to `max_steps` steps, in the order
/// this module states, with its states and transitions, lending the visitor
/// `meter`. Each expansion charges `graph.expand`.
///
/// # Errors
///
/// [`NotSimulated`] when a root of the system is unbounded (the requires-bound
/// pre-check, before any state is expanded), or when a state or transition
/// identity has no canonical encoding.
pub fn model_behaviours<B>(
    system: &ModelSystem<'_>,
    max_steps: usize,
    meter: &mut Meter,
    mut visit: impl FnMut(&[BehaviourState], &[ModelTransition], &mut Meter) -> ControlFlow<B>,
) -> Result<BehavioursEnd<B>, NotSimulated> {
    check_model_bounds(system, crate::simulation::BOUND_POSITIONS)?;
    let initial = sorted_initial(system, identity_limits())?;
    for steps in 1..=max_steps {
        for root in &initial {
            let mut path = vec![BehaviourState {
                state: root.state.clone(),
                key: root.key.clone(),
            }];
            let mut transitions: Vec<ModelTransition> = Vec::new();
            let mut frames: Vec<Frame> = match successors(system, &root.state, meter)? {
                Ok(frame) => vec![frame],
                Err(end) => return Ok(end),
            };
            while let Some(frame) = frames.last_mut() {
                let Some(next) = frame.next() else {
                    frames.pop();
                    path.pop();
                    transitions.pop();
                    continue;
                };
                path.push(BehaviourState {
                    state: next.state,
                    key: next.key,
                });
                transitions.push(next.transition);
                if transitions.len() == steps {
                    if let ControlFlow::Break(broken) = visit(&path, &transitions, meter) {
                        return Ok(BehavioursEnd::Broken(broken));
                    }
                    path.pop();
                    transitions.pop();
                    continue;
                }
                let frame = match path.last() {
                    Some(last) => successors(system, &last.state, meter)?,
                    None => continue,
                };
                match frame {
                    Ok(frame) => frames.push(frame),
                    Err(end) => return Ok(end),
                }
            }
        }
    }
    Ok(BehavioursEnd::Exhausted)
}

/// The ordered successors of `state`, charging `graph.expand`, or why the
/// walk ends there.
fn successors<B>(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    meter: &mut Meter,
) -> Result<Result<Frame, BehavioursEnd<B>>, NotSimulated> {
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::GraphExpand)) {
        return Ok(Err(BehavioursEnd::Meter(incomplete)));
    }
    Ok(match expand(system, state, identity_limits())? {
        Expanded::Successors { successors, .. } => Ok(successors.into_iter()),
        Expanded::Stopped(stop) => Err(BehavioursEnd::Stopped(stop)),
    })
}
