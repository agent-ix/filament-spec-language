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
//! tree, one per leaf, in the leaves' discovery order. The tree is stored
//! once, each state by one shared key and each edge by the positions of its
//! two states and an interned transition identity; a trace is built from it
//! only when the caller asks for that leaf's path ([`Enumeration::path`]).
//!
//! Every expansion, stored state and stored edge is charged to the caller's
//! FR-109 [`Meter`] (`graph.expand`, `graph.result-retain` with the key's
//! bytes, `graph.edge`), so a run whose store would outgrow the request's
//! accounting limits stops `incomplete` naming the exhausted counter
//! ([`EnumerationEnd::Meter`]).
//!
//! [`sampled_coverage`] re-walks sampled traces against the system that drew
//! them and counts what they visited. Both count coverage with one meaning:
//! the distinct state keys visited, the distinct (pre-state, transition
//! identity, post-state) edges traversed, and the distinct visited states at
//! which more than one successor was enabled.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use filament_exact::{length_amount, Charge, ChargePoint, Incomplete, Meter};
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
    /// The FR-109 meter refused a charge: the store would exceed the
    /// request's accounting limits.
    Meter(Incomplete),
    /// The caller cancelled the walk.
    Cancelled,
    /// The system stopped an expansion with this cause.
    Stopped(CatalogCode),
}

/// One discovered state: its key, whether that key is longer than the
/// walk's `max_state_key_bytes` (so no output carries it), its discovery
/// edge, its depth, and whether it has a child in the discovery tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovered<T> {
    /// The state's full canonical key, shared with the walk's index.
    pub key: Rc<StateKey>,
    /// Whether the key exceeds `max_state_key_bytes`.
    pub oversized: bool,
    /// The discovering state's position and the transition taken; `None`
    /// for an initial state.
    pub parent: Option<(usize, T)>,
    /// The number of discovery edges from its initial state.
    pub depth: usize,
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

impl<T> Enumeration<T> {
    /// The leaves of the discovery tree, in discovery order: one trace each.
    pub fn leaves(&self) -> impl Iterator<Item = FrontierPosition> + '_ {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, state)| !state.has_child)
            .map(|(leaf, _)| leaf)
    }

    /// The trace ending at `leaf`: the positions of the states from its
    /// initial state to it and the transitions between them. Built on
    /// request, so a caller meters each trace before it exists.
    pub fn path(&self, leaf: FrontierPosition) -> (Vec<FrontierPosition>, Vec<&T>) {
        let mut positions = vec![leaf];
        let mut transitions = Vec::new();
        let mut at = leaf;
        while let Some((parent, transition)) = self.states.get(at).and_then(|s| s.parent.as_ref()) {
            positions.push(*parent);
            transitions.push(transition);
            at = *parent;
        }
        positions.reverse();
        transitions.reverse();
        (positions, transitions)
    }
}

/// Whether one expansion's successors hold more than one distinct
/// (transition identity, post-state) pair: a choice point.
fn is_choice<T, S>(
    successors: &[OrderedSuccessor<T, S>],
    identity: IdentityLimits,
) -> Result<bool, EncodingRefusal>
where
    T: quire_canonical::Encode,
{
    let mut seen: HashSet<(Vec<u8>, &StateKey)> = HashSet::new();
    for successor in successors {
        seen.insert((
            canonical_bytes(&successor.transition, identity)?,
            &successor.key,
        ));
        if seen.len() > 1 {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The walk's store: the discovered states, the index from key to
/// position, the interned transition identities and the traversed edges.
struct Store<T> {
    states: Vec<Discovered<T>>,
    known: HashMap<Rc<StateKey>, usize>,
    transitions: HashMap<Vec<u8>, usize>,
    edges: HashSet<(usize, usize, usize)>,
}

impl<T> Store<T> {
    /// Record a newly reached state, charging its retained key.
    fn discover(
        &mut self,
        meter: &mut Meter,
        key: StateKey,
        oversized: bool,
        parent: Option<(usize, T)>,
        depth: usize,
    ) -> Result<usize, Incomplete> {
        meter.charge(
            Charge::new(ChargePoint::GraphResultRetain)
                .results(length_amount(key.as_bytes().len())),
        )?;
        let key = Rc::new(key);
        let position = self.states.len();
        if let Some((parent, _)) = &parent {
            if let Some(state) = self.states.get_mut(*parent) {
                state.has_child = true;
            }
        }
        self.known.insert(Rc::clone(&key), position);
        self.states.push(Discovered {
            key,
            oversized,
            parent,
            depth,
            has_child: false,
        });
        Ok(position)
    }

    /// Record the edge `pre --transition--> post`, charging it once.
    fn edge(
        &mut self,
        meter: &mut Meter,
        pre: usize,
        transition: Vec<u8>,
        post: usize,
    ) -> Result<(), Incomplete> {
        let next = self.transitions.len();
        let interned = *self.transitions.entry(transition).or_insert(next);
        if !self.edges.contains(&(pre, interned, post)) {
            meter.charge(Charge::new(ChargePoint::GraphEdge))?;
            self.edges.insert((pre, interned, post));
        }
        Ok(())
    }
}

/// Enumerate `system` breadth-first in FR-101's canonical order within
/// `request`'s bounds, keeping the discovery tree and charging `meter`,
/// first refusing an unbounded `domains` request before any
/// `TransitionSystem` method is called. `poll` is called once per state the
/// walk attempts to expand; returning `true` cancels the walk before that
/// state is touched.
///
/// # Errors
///
/// [`NotSimulated`] as `explore_request` returns it.
pub fn enumerate_request<S: TransitionSystem, R: Root>(
    system: &S,
    request: EnumerationRequest<'_, R>,
    meter: &mut Meter,
    mut poll: impl FnMut() -> bool,
) -> Result<Enumeration<S::TransitionId>, NotSimulated> {
    check_requires_bound(request.domains, request.types, request.position_limit)?;
    let bounds = request.bounds;
    let identity = request.identity;
    let oversized = |key: &StateKey| {
        !u64::try_from(key.as_bytes().len()).is_ok_and(|len| len <= bounds.max_state_key_bytes)
    };
    let mut store: Store<S::TransitionId> = Store {
        states: Vec::new(),
        known: HashMap::new(),
        transitions: HashMap::new(),
        edges: HashSet::new(),
    };
    let mut queue: VecDeque<(usize, S::State)> = VecDeque::new();
    let mut held: Vec<FrontierPosition> = Vec::new();
    let mut frontier = Vec::new();
    let mut choices = 0usize;
    let mut end = None;
    for initial in sorted_initial(system, identity).map_err(NotSimulated::from)? {
        if store.states.len() >= bounds.max_states {
            end = Some(EnumerationEnd::MaxStates);
            break;
        }
        let big = oversized(&initial.key);
        let position = match store.discover(meter, initial.key, big, None, 0) {
            Ok(position) => position,
            Err(incomplete) => {
                end = Some(EnumerationEnd::Meter(incomplete));
                break;
            }
        };
        if big {
            held.push(position);
        } else {
            queue.push_back((position, initial.state));
        }
    }
    while end.is_none() {
        let Some((position, state)) = queue.pop_front() else {
            break;
        };
        let depth = store.states.get(position).map_or(0, |state| state.depth);
        if depth >= bounds.max_steps {
            frontier.push(position);
            end = Some(EnumerationEnd::MaxSteps);
            break;
        }
        if poll() {
            frontier.push(position);
            end = Some(EnumerationEnd::Cancelled);
            break;
        }
        if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::GraphExpand)) {
            frontier.push(position);
            end = Some(EnumerationEnd::Meter(incomplete));
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
        if is_choice(&successors, identity).map_err(NotSimulated::from)? {
            choices += 1;
        }
        for successor in successors {
            let bytes =
                canonical_bytes(&successor.transition, identity).map_err(NotSimulated::from)?;
            if let Some(post) = store.known.get(&successor.key).copied() {
                if let Err(incomplete) = store.edge(meter, position, bytes, post) {
                    end = Some(EnumerationEnd::Meter(incomplete));
                    break;
                }
                continue;
            }
            if store.states.len() >= bounds.max_states {
                end = Some(EnumerationEnd::MaxStates);
                break;
            }
            let big = oversized(&successor.key);
            let discovered = match store.discover(
                meter,
                successor.key,
                big,
                Some((position, successor.transition)),
                depth + 1,
            ) {
                Ok(discovered) => discovered,
                Err(incomplete) => {
                    end = Some(EnumerationEnd::Meter(incomplete));
                    break;
                }
            };
            if let Err(incomplete) = store.edge(meter, position, bytes, discovered) {
                end = Some(EnumerationEnd::Meter(incomplete));
                break;
            }
            if big {
                held.push(discovered);
            } else {
                queue.push_back((discovered, successor.state));
            }
        }
        if end.is_some() {
            frontier.push(position);
        }
    }
    frontier.extend(queue.iter().map(|(at, _)| *at));
    frontier.extend(held.iter().copied());
    let end = end.unwrap_or(if held.is_empty() {
        EnumerationEnd::Exhausted
    } else {
        EnumerationEnd::StateKeyLimit
    });
    let coverage = Coverage {
        states: store.states.len(),
        transitions: store.edges.len(),
        choices,
    };
    Ok(Enumeration {
        end,
        states: store.states,
        frontier,
        coverage,
    })
}

/// The position of `key` among `keys`, interning it on first sight.
fn index(key: &StateKey, keys: &mut HashMap<StateKey, usize>) -> usize {
    let next = keys.len();
    *keys.entry(key.clone()).or_insert(next)
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
    let mut keys: HashMap<StateKey, usize> = HashMap::new();
    let mut transitions: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut edges: HashSet<(usize, usize, usize)> = HashSet::new();
    let mut choices: HashSet<usize> = HashSet::new();
    for trace in traces {
        let Some(start) = initial.iter().find(|state| state.key == trace.initial) else {
            continue;
        };
        let mut at = index(&start.key, &mut keys);
        let mut state = None;
        let mut steps = trace.steps.iter();
        loop {
            let current = state.as_ref().unwrap_or(&start.state);
            let Expanded::Successors { successors, .. } = expand(system, current, identity)? else {
                break;
            };
            if is_choice(&successors, identity)? {
                choices.insert(at);
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
            let bytes = canonical_bytes(&next.transition, identity)?;
            let next_transition = transitions.len();
            let transition = *transitions.entry(bytes).or_insert(next_transition);
            let post = index(&next.key, &mut keys);
            edges.insert((at, transition, post));
            at = post;
            state = Some(next.state);
        }
    }
    Ok(Coverage {
        states: keys.len(),
        transitions: edges.len(),
        choices: choices.len(),
    })
}
