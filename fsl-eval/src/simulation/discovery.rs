// SPDX-License-Identifier: AGPL-3.0-or-later
//! The traced runs a simulation item reports (FSL FR-634, FSpec FR-181):
//! a breadth-first enumeration that keeps its discovery tree, and the
//! coverage of a set of sampled traces.
//!
//! [`enumerate_request`] walks the same canonical order `explore_request`
//! walks (FR-101), coalescing equal state keys. Each explored state other
//! than an initial one keeps its *discovery edge*, the edge by which the
//! walk first reached it, so the discovery edges form a tree rooted at the
//! initial states, and the run's traces are the root-to-leaf paths of that
//! tree, one per leaf, in the leaves' discovery order.
//!
//! [`sampled_coverage`] re-walks sampled traces against the system that drew
//! them and counts what they visited. Both count coverage with one meaning:
//! the distinct state keys visited, the distinct (pre-state, transition
//! identity, post-state) edges traversed, and the distinct visited states at
//! which more than one successor was enabled.

use std::collections::{HashMap, HashSet, VecDeque};

use fsl_foundation::IdentityLimits;

use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{canonical_bytes, EncodingRefusal, StateKey};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated, Root};
use crate::simulation::order::{expand, sorted_initial, Expanded, OrderedSuccessor};
use crate::simulation::trace::Trace;
use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::declaration::TypeEnvironment;

/// The bounds of one enumeration (FSpec FR-181), each a positive count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnumerationBounds {
    /// The most distinct states the walk discovers.
    pub max_states: usize,
    /// The search horizon: no state at this depth is expanded.
    pub max_steps: usize,
    /// The longest canonical state key, in octets, the walk expands.
    pub max_state_key_bytes: u64,
}

/// [`enumerate_request`]'s inputs.
pub struct EnumerationRequest<'r, R: Root> {
    /// The requires-bound roots.
    pub domains: &'r [R],
    /// The type environment the roots are classified under.
    pub types: &'r TypeEnvironment,
    /// The requires-bound pre-check's position budget.
    pub position_limit: u64,
    /// The walk's bounds.
    pub bounds: EnumerationBounds,
    /// The identity limits states and transitions encode under.
    pub identity: IdentityLimits,
}

/// Why an enumeration ended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnumerationEnd {
    /// Every bounded behaviour was explored.
    Exhausted,
    /// `max_states` stopped the walk.
    MaxStates,
    /// The `max_steps` horizon left unexpanded states.
    MaxSteps,
    /// A reached state's key was longer than `max_state_key_bytes`; it is
    /// kept in the frontier by position (FSpec FR-181-AC-30).
    StateKeyLimit,
    /// The caller cancelled the walk.
    Cancelled,
    /// The system stopped an expansion with this cause.
    Stopped(CatalogCode),
}

/// One discovered state: its key, whether that key is longer than the
/// walk's `max_state_key_bytes` (so no output carries it), its discovery
/// edge, and whether it has a child in the discovery tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovered<T> {
    /// The state's full canonical key.
    pub key: StateKey,
    /// Whether the key exceeds `max_state_key_bytes`.
    pub oversized: bool,
    /// The discovering state's position and the transition taken; `None`
    /// for an initial state.
    pub parent: Option<(usize, T)>,
    /// Whether some discovery edge leaves this state.
    pub has_child: bool,
}

/// One entry of the unexplored frontier: a state's position in
/// [`Enumeration::states`].
pub type FrontierPosition = usize;

/// The coverage counts of one run (FR-634 "Outputs").
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Coverage {
    /// The distinct state keys visited.
    pub states: usize,
    /// The distinct (pre-state, transition, post-state) edges traversed.
    pub transitions: usize,
    /// The distinct visited states with more than one enabled successor.
    pub choices: usize,
}

/// The result of one enumeration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Enumeration<T> {
    /// How the walk ended.
    pub end: EnumerationEnd,
    /// Every discovered state, in discovery order.
    pub states: Vec<Discovered<T>>,
    /// The unexplored states, in the order the walk would expand them next.
    pub frontier: Vec<FrontierPosition>,
    /// What the walk visited.
    pub coverage: Coverage,
}

impl<T: Clone> Enumeration<T> {
    /// The discovery-tree traces: for each leaf, in discovery order, the
    /// positions of the states from its initial state to it and the
    /// transitions between them.
    pub fn traces(&self) -> Vec<(Vec<FrontierPosition>, Vec<T>)> {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, state)| !state.has_child)
            .map(|(leaf, _)| {
                let mut positions = vec![leaf];
                let mut transitions = Vec::new();
                let mut at = leaf;
                while let Some((parent, transition)) = &self.states[at].parent {
                    positions.push(*parent);
                    transitions.push(transition.clone());
                    at = *parent;
                }
                positions.reverse();
                transitions.reverse();
                (positions, transitions)
            })
            .collect()
    }
}

/// Distinct successors of one expansion, by transition identity bytes and
/// post-state key.
fn distinct_successors<T, S>(
    successors: &[OrderedSuccessor<T, S>],
    identity: IdentityLimits,
) -> Result<Vec<(Vec<u8>, StateKey)>, EncodingRefusal>
where
    T: quire_canonical::Encode,
{
    let mut seen = Vec::new();
    for successor in successors {
        let entry = (
            canonical_bytes(&successor.transition, identity)?,
            successor.key.clone(),
        );
        if !seen.contains(&entry) {
            seen.push(entry);
        }
    }
    Ok(seen)
}

/// Enumerate `system` breadth-first in FR-101's canonical order within
/// `request`'s bounds, keeping the discovery tree, first refusing an
/// unbounded `domains` request before any `TransitionSystem` method is
/// called. `poll` is called once per state the walk attempts to expand;
/// returning `true` cancels the walk before that state is touched.
///
/// # Errors
///
/// [`NotSimulated`] as `explore_request` returns it.
pub fn enumerate_request<S: TransitionSystem, R: Root>(
    system: &S,
    request: EnumerationRequest<'_, R>,
    mut poll: impl FnMut() -> bool,
) -> Result<Enumeration<S::TransitionId>, NotSimulated> {
    check_requires_bound(request.domains, request.types, request.position_limit)?;
    let bounds = request.bounds;
    let identity = request.identity;
    let oversized = |key: &StateKey| {
        u64::try_from(key.as_bytes().len()).map_or(true, |len| len > bounds.max_state_key_bytes)
    };
    let mut states: Vec<Discovered<S::TransitionId>> = Vec::new();
    let mut known: HashMap<StateKey, usize> = HashMap::new();
    let mut queue: VecDeque<(usize, S::State, usize)> = VecDeque::new();
    let mut held: Vec<FrontierPosition> = Vec::new();
    let mut edges: HashSet<(usize, Vec<u8>, StateKey)> = HashSet::new();
    let mut choices = 0usize;
    let mut end = None;
    for initial in sorted_initial(system, identity).map_err(NotSimulated::from)? {
        if states.len() >= bounds.max_states {
            end = Some(EnumerationEnd::MaxStates);
            break;
        }
        let position = states.len();
        let big = oversized(&initial.key);
        known.insert(initial.key.clone(), position);
        states.push(Discovered {
            key: initial.key,
            oversized: big,
            parent: None,
            has_child: false,
        });
        if big {
            held.push(position);
        } else {
            queue.push_back((position, initial.state, 0));
        }
    }
    let mut frontier = Vec::new();
    while end.is_none() {
        let Some((position, state, depth)) = queue.pop_front() else {
            break;
        };
        if depth >= bounds.max_steps {
            frontier.push(position);
            frontier.extend(queue.iter().map(|(at, _, _)| *at));
            queue.clear();
            end = Some(EnumerationEnd::MaxSteps);
            break;
        }
        if poll() {
            frontier.push(position);
            end = Some(EnumerationEnd::Cancelled);
            break;
        }
        let successors = match expand(system, &state, identity).map_err(NotSimulated::from)? {
            Expanded::Successors { successors, .. } => successors,
            Expanded::Stopped(stop) => {
                frontier.push(position);
                end = Some(EnumerationEnd::Stopped(stop.cause));
                break;
            }
        };
        let distinct = distinct_successors(&successors, identity).map_err(NotSimulated::from)?;
        if distinct.len() > 1 {
            choices += 1;
        }
        let mut stopped = false;
        for successor in successors {
            let bytes =
                canonical_bytes(&successor.transition, identity).map_err(NotSimulated::from)?;
            if known.contains_key(&successor.key) {
                edges.insert((position, bytes, successor.key));
                continue;
            }
            if states.len() >= bounds.max_states {
                frontier.push(position);
                stopped = true;
                break;
            }
            edges.insert((position, bytes, successor.key.clone()));
            let discovered = states.len();
            let big = oversized(&successor.key);
            known.insert(successor.key.clone(), discovered);
            states[position].has_child = true;
            states.push(Discovered {
                key: successor.key,
                oversized: big,
                parent: Some((position, successor.transition)),
                has_child: false,
            });
            if big {
                held.push(discovered);
            } else {
                queue.push_back((discovered, successor.state, depth + 1));
            }
        }
        if stopped {
            end = Some(EnumerationEnd::MaxStates);
        }
    }
    frontier.extend(queue.iter().map(|(at, _, _)| *at));
    frontier.extend(held.iter().copied());
    let end = end.unwrap_or(if held.is_empty() {
        EnumerationEnd::Exhausted
    } else {
        EnumerationEnd::StateKeyLimit
    });
    let coverage = Coverage {
        states: states.len(),
        transitions: edges.len(),
        choices,
    };
    Ok(Enumeration {
        end,
        states,
        frontier,
        coverage,
    })
}

/// The coverage of `traces`, each drawn from `system`: the run re-walks each
/// trace from its initial state, expanding every state on it to count its
/// enabled successors.
///
/// # Errors
///
/// [`EncodingRefusal`] when a state or transition identity has no RFC 8785
/// encoding; a trace that names no initial state or no successor of `system`
/// counts only the prefix that re-walks.
pub fn sampled_coverage<S: TransitionSystem, F>(
    system: &S,
    traces: &[Trace<S::TransitionId, F>],
    identity: IdentityLimits,
) -> Result<Coverage, EncodingRefusal> {
    let initial = sorted_initial(system, identity)?;
    let mut states: HashSet<StateKey> = HashSet::new();
    let mut edges: HashSet<(StateKey, Vec<u8>, StateKey)> = HashSet::new();
    let mut choices: HashSet<StateKey> = HashSet::new();
    for trace in traces {
        let Some(start) = initial.iter().find(|state| state.key == trace.initial) else {
            continue;
        };
        let mut key = start.key.clone();
        let mut state = None;
        states.insert(key.clone());
        let mut steps = trace.steps.iter();
        loop {
            let at = state.as_ref().unwrap_or(&start.state);
            let Expanded::Successors { successors, .. } = expand(system, at, identity)? else {
                break;
            };
            if distinct_successors(&successors, identity)?.len() > 1 {
                choices.insert(key.clone());
            }
            let Some(step) = steps.next() else {
                break;
            };
            let Some(next) = successors
                .into_iter()
                .filter(|successor| successor.transition == step.transition)
                .nth(step.ordinal)
            else {
                break;
            };
            edges.insert((
                key.clone(),
                canonical_bytes(&next.transition, identity)?,
                next.key.clone(),
            ));
            key = next.key;
            states.insert(key.clone());
            state = Some(next.state);
        }
    }
    Ok(Coverage {
        states: states.len(),
        transitions: edges.len(),
        choices: choices.len(),
    })
}
