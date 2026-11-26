// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state-graph forms over timed states, held as zones (FSpec FR-712
//! "Symbolic decision").
//!
//! FR-712's forms speak of timed states, and a zone-graph node holds many: an
//! edge says only that some valuation of its source can take it. So nothing
//! here is closed over whole nodes. `possible` is a forward search from each
//! initial valuation. `E(P)` is the least fixpoint of timed and discrete
//! predecessors, a union of zones per model state. `unique path` reads, for
//! each valuation, the transition sequences from it, backwards from `Y` as
//! unions of zones. The forward graph, explored under the aM bounds (`L = U =
//! max(L, U)`), supplies the reachable states: every reachable timed state is
//! bisimilar to a valuation of one of its nodes, and every valuation of a
//! node is reachable.
//!
//! Every predicate reads model state only, so its reading is given per node
//! and is one per model state.
//!
//! The decision draws on the budgets the exploration left (FR-852
//! "Budgets"): a zone it holds counts against `max_symbolic_states` and
//! `max_zone_bytes` with the graph's, and each predecessor or successor it
//! computes, each piece a zone subtraction makes and each pair of zones it
//! intersects count against `max_symbolic_transitions` with the graph's
//! edges.

use std::collections::{BTreeMap, VecDeque};

use filament_exact::{Cancel, Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{GuardedStep, ModelSimState, PlaceConstraint, TimedSystem};
use fsl_replay::{ModelPath, ModelStep};
use ix_trace_rs::implements;
use num_bigint::BigInt;
use num_integer::Integer as _;

use super::concretise::{concretise_from, Concrete, NotConcretised, Stage, Target};
use super::search::{
    apply_step_clocks, constrain_at, intersect, preimage_step_clocks, scaled, subtract, zone_bytes,
    Split, TimedStateGraph, ZoneCheckLimit, ZoneCheckLimits, ZoneCheckRefusal, ZoneStop,
};
use super::state_graph::TimedStateGraphDecision;
use super::Dbm;

/// One step edge between two model states of the graph: one guarded step
/// through one convex part of its guard.
struct Move {
    source: usize,
    step: GuardedStep,
    part: usize,
    target: usize,
}

/// One model state of the graph.
struct State {
    state: ModelSimState,
    invariants: Vec<PlaceConstraint>,
    elapses: bool,
    /// The moves leaving and entering the state, by index.
    leaving: Vec<usize>,
    entering: Vec<usize>,
}

/// `E(P)` as zones, per model state.
type EventuallyZones = Vec<Vec<Dbm>>;

/// The valuations a transition sequence leads from to the target, per model
/// state, and the sequence as its first transition and the rest.
struct Word {
    first: Option<(ModelTransition, usize)>,
    zones: BTreeMap<usize, Vec<Dbm>>,
}

/// A zone the forward search reached, and the move and node that gave it.
struct Reached {
    state: usize,
    zone: Dbm,
    from: Option<(usize, usize)>,
}

/// The timed states of one explored graph, under the run's budgets.
pub(super) struct Zones<'r, 's, 'a> {
    timed: &'r TimedSystem<'s, 'a>,
    graph: &'r TimedStateGraph,
    limits: ZoneCheckLimits,
    cancel: &'r Cancel,
    dim: usize,
    states: Vec<State>,
    /// Each graph node's model state, by index into `states`.
    of_node: Vec<usize>,
    moves: Vec<Move>,
    /// The zones, their bytes and the symbolic work held and done by the
    /// exploration and this decision together.
    stored: u64,
    bytes: u64,
    computed: u64,
}

impl<'r, 's, 'a> Zones<'r, 's, 'a> {
    pub(super) fn new(
        timed: &'r TimedSystem<'s, 'a>,
        graph: &'r TimedStateGraph,
        limits: ZoneCheckLimits,
        cancel: &'r Cancel,
    ) -> Result<Self, ZoneCheckRefusal> {
        let key = |state: &ModelSimState| {
            timed
                .system()
                .state_key(state, fsl_foundation::IdentityLimits::default())
                .map(|(key, _)| key)
                .map_err(|_| ZoneCheckRefusal::Fault("state-graph-state-key"))
        };
        let mut keys = BTreeMap::new();
        let mut states = Vec::new();
        let mut of_node = Vec::with_capacity(graph.nodes.len());
        for node in &graph.nodes {
            let next = states.len();
            let index = *keys.entry(key(&node.state)?).or_insert(next);
            if index == next {
                states.push(State {
                    state: node.state.clone(),
                    invariants: timed
                        .invariants(&node.state)
                        .map_err(|_| ZoneCheckRefusal::Fault("state-graph-invariants"))?,
                    elapses: timed.tick_period().is_none()
                        && !timed
                            .is_urgent(&node.state)
                            .map_err(|_| ZoneCheckRefusal::Fault("state-graph-elapses"))?,
                    leaving: Vec::new(),
                    entering: Vec::new(),
                });
            }
            of_node.push(index);
        }
        let mut moves = Vec::new();
        for source in 0..states.len() {
            let steps = timed
                .guarded_steps(&states[source].state)
                .map_err(|_| ZoneCheckRefusal::Fault("state-graph-guarded-steps"))?;
            for step in steps {
                // A post-state no node holds is entered from no reachable
                // valuation.
                let Some(&target) = keys.get(&key(step.post())?) else {
                    continue;
                };
                for part in 0..step.parts().len() {
                    states[source].leaving.push(moves.len());
                    states[target].entering.push(moves.len());
                    moves.push(Move {
                        source,
                        step: step.clone(),
                        part,
                        target,
                    });
                }
            }
        }
        Ok(Self {
            timed,
            graph,
            limits,
            cancel,
            dim: timed.places().len() + 1,
            states,
            of_node,
            moves,
            stored: graph
                .stats
                .symbolic_states
                .saturating_add(graph.stats.divergence_states),
            bytes: graph.stats.zone_bytes,
            computed: graph.stats.retained_edges,
        })
    }

    /// `possible`: from each initial valuation, a forward search for a state
    /// where `p` holds.
    #[implements("FR-852-AC-1", "FR-852-AC-32")]
    pub(super) fn possible(
        &mut self,
        p: &[bool],
    ) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
        let p = self.per_state(p);
        let mut witness_paths = Vec::with_capacity(self.graph.initials.len());
        for (initial, &node) in self.graph.initials.iter().enumerate() {
            let initial = u32::try_from(initial)
                .map_err(|_| ZoneCheckRefusal::Fault("state-graph-initial"))?;
            let state = self.of_node[node];
            let mut zone = Dbm::zero(self.dim);
            let scale = self.graph.scale.clone();
            self.settle(state, &mut zone, &scale)?;
            let (reached, found) = self.search(vec![(state, zone)], Some(&p))?;
            let Some(mut at) = found else {
                return Ok(TimedStateGraphDecision::Refuted {
                    stem: ModelPath {
                        initial,
                        steps: Vec::new(),
                    },
                    node,
                    ef_nodes: Vec::new(),
                    paths: None,
                });
            };
            let mut moves = Vec::new();
            while let Some((taken, parent)) = reached[at].from {
                moves.push(taken);
                at = parent;
            }
            moves.reverse();
            let steps = self
                .concretise(&moves, &self.zero_clocks(), &scale, &Target::Any)?
                .ok_or(ZoneCheckRefusal::Fault("state-graph-witness"))?
                .prefix;
            witness_paths.push(ModelPath { initial, steps });
            // This initial state's search is dropped before the next.
            for held in &reached {
                self.release_zone(&held.zone);
            }
        }
        Ok(TimedStateGraphDecision::Holds {
            witness_paths,
            ef_nodes: Vec::new(),
        })
    }

    /// `always possible`: every valuation of a node where `q` holds is in
    /// `E(P)`, the backward fixpoint from the states where `p` holds.
    #[implements("FR-852-AC-2", "FR-852-AC-32", "FSpec-FR-712-AC-2")]
    pub(super) fn always_possible(
        &mut self,
        q: &[bool],
        p: &[bool],
    ) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
        let p = self.per_state(p);
        let ef = self.eventually(&p)?;
        let mut ef_nodes = Vec::new();
        let mut trapped = None;
        let graph = self.graph;
        for (node, from) in q.iter().enumerate() {
            let outside = self.outside(&graph.nodes[node].zone, &ef[self.of_node[node]])?;
            if outside.is_empty() {
                ef_nodes.push(node);
            } else if trapped.is_none() && *from {
                trapped = Some((node, outside));
            }
        }
        let Some((node, outside)) = trapped else {
            return Ok(TimedStateGraphDecision::Holds {
                witness_paths: Vec::new(),
                ef_nodes,
            });
        };
        let (stem, _) = self.stem(node, &outside)?;
        Ok(TimedStateGraphDecision::Refuted {
            stem,
            node,
            ef_nodes,
            paths: None,
        })
    }

    /// `unique path`: from every valuation of a node where `x` holds, one
    /// transition sequence leads to a state where `y` holds.
    #[implements("FR-852-AC-3", "FR-852-AC-32", "FSpec-FR-712-AC-3")]
    pub(super) fn unique_path(
        &mut self,
        x: &[bool],
        y: &[bool],
    ) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
        let sources: Vec<usize> = (0..x.len()).filter(|&node| x[node]).collect();
        let y = self.per_state(y);
        // A valuation with no sequence.
        let ef = self.eventually(&y)?;
        let graph = self.graph;
        for &node in &sources {
            let outside = self.outside(&graph.nodes[node].zone, &ef[self.of_node[node]])?;
            if !outside.is_empty() {
                let (stem, _) = self.stem(node, &outside)?;
                return Ok(TimedStateGraphDecision::Refuted {
                    stem,
                    node,
                    ef_nodes: Vec::new(),
                    paths: None,
                });
            }
        }
        // A valuation with two. Sequences are read backwards from `Y`,
        // shortest first, each as the valuations it leads from. Only those
        // that still meet the states reachable from an `X` state are
        // extended; while the claim holds they are pairwise disjoint there,
        // so there are finitely many.
        let seeds = sources
            .iter()
            .map(|&node| (self.of_node[node], self.graph.nodes[node].zone.clone()))
            .collect();
        let (reachable, _) = self.search(seeds, None)?;
        let mut words = vec![Word {
            first: None,
            zones: BTreeMap::new(),
        }];
        for (state, _) in y.iter().enumerate().filter(|(_, holds)| **holds) {
            let zone = self.within_invariants(state)?;
            if !zone.is_empty() {
                self.charge_zone(&zone)?;
                words[0].zones.insert(state, vec![zone]);
            }
        }
        let mut seen: BTreeMap<usize, Vec<(Dbm, usize)>> = BTreeMap::new();
        let mut at = 0;
        while at < words.len() {
            self.poll()?;
            for &node in &sources {
                let Some(zones) = words[at].zones.get(&self.of_node[node]) else {
                    continue;
                };
                let mut fresh = Vec::new();
                for zone in zones {
                    let mut here = zone.clone();
                    intersect(&mut here, &self.graph.nodes[node].zone)?;
                    if here.is_empty() {
                        continue;
                    }
                    for (other, word) in seen.get(&node).into_iter().flatten() {
                        self.charge_step()?;
                        let mut both = here.clone();
                        intersect(&mut both, other)?;
                        if !both.is_empty() {
                            let word = *word;
                            return self.two_paths(node, both, &words, (word, at));
                        }
                    }
                    fresh.push(here);
                }
                for zone in fresh {
                    self.charge_zone(&zone)?;
                    seen.entry(node).or_default().push((zone, at));
                }
            }
            for (transition, zones) in self.extend(&words[at])? {
                if !self.meets(&zones, &reachable)? {
                    for zone in zones.values().flatten() {
                        self.release_zone(zone);
                    }
                    continue;
                }
                words.push(Word {
                    first: Some((transition, at)),
                    zones,
                });
            }
            at += 1;
        }
        Ok(TimedStateGraphDecision::Holds {
            witness_paths: Vec::new(),
            ef_nodes: Vec::new(),
        })
    }

    /// The sequences one transition longer than `word`, by that transition.
    fn extend(
        &mut self,
        word: &Word,
    ) -> Result<BTreeMap<ModelTransition, BTreeMap<usize, Vec<Dbm>>>, ZoneCheckRefusal> {
        let mut longer: BTreeMap<ModelTransition, BTreeMap<usize, Vec<Dbm>>> = BTreeMap::new();
        for (&state, zones) in &word.zones {
            for taken in self.states[state].entering.clone() {
                let source = self.moves[taken].source;
                let transition = self.moves[taken].step.transition().clone();
                for zone in zones {
                    let before = self.predecessor(taken, zone)?;
                    if before.is_empty() {
                        continue;
                    }
                    let stored = longer
                        .entry(transition.clone())
                        .or_default()
                        .entry(source)
                        .or_default();
                    for part in self.outside(&before, stored)? {
                        self.charge_zone(&part)?;
                        stored.push(part);
                    }
                }
            }
        }
        Ok(longer)
    }

    fn per_state(&self, nodes: &[bool]) -> Vec<bool> {
        let mut states = vec![false; self.states.len()];
        for (&state, &holds) in self.of_node.iter().zip(nodes) {
            states[state] |= holds;
        }
        states
    }

    fn zero_clocks(&self) -> Vec<Rational> {
        vec![Rational::from_integer(Integer::zero()); self.dim - 1]
    }

    /// Every valuation of `state` within its time invariants.
    fn within_invariants(&self, state: usize) -> Result<Dbm, ZoneCheckRefusal> {
        let mut zone = Dbm::universe(self.dim);
        constrain_at(&mut zone, &self.states[state].invariants, &self.graph.scale)?;
        Ok(zone)
    }

    fn poll(&self) -> Result<(), ZoneCheckRefusal> {
        if self.cancel.poll() {
            return Err(ZoneCheckRefusal::Stopped(ZoneStop::Cancelled));
        }
        Ok(())
    }

    /// Charge one held zone against `max_symbolic_states` and
    /// `max_zone_bytes`.
    #[implements("FR-852-AC-33")]
    fn charge_zone(&mut self, zone: &Dbm) -> Result<(), ZoneCheckRefusal> {
        self.stored = self.stored.saturating_add(1);
        let limit = self.limits.max_symbolic_states;
        if self.stored > limit {
            return Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxSymbolicStates(limit),
            )));
        }
        self.bytes = self.bytes.saturating_add(zone_bytes(zone));
        let limit = self.limits.max_zone_bytes;
        if self.bytes > limit {
            return Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxZoneBytes(limit),
            )));
        }
        Ok(())
    }

    /// Give back a charged zone that is dropped.
    fn release_zone(&mut self, zone: &Dbm) {
        self.stored = self.stored.saturating_sub(1);
        self.bytes = self.bytes.saturating_sub(zone_bytes(zone));
    }

    /// The parts of `zone` outside every zone of `held`.
    #[implements("FR-852-AC-33")]
    fn outside(&mut self, zone: &Dbm, held: &[Dbm]) -> Result<Vec<Dbm>, ZoneCheckRefusal> {
        let room = self.limits.max_symbolic_states.saturating_sub(self.stored);
        let states = self.limits.max_symbolic_states;
        subtract(zone, held, &mut |split| match split {
            Split::Read => self.poll(),
            Split::Piece(pieces) => {
                if u64::try_from(pieces).map_or(true, |pieces| pieces > room) {
                    return Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(
                        ZoneCheckLimit::MaxSymbolicStates(states),
                    )));
                }
                self.charge_step()
            }
        })
    }

    /// Whether some zone of `zones` meets one the forward search reached at
    /// its state.
    #[implements("FR-852-AC-33")]
    fn meets(
        &mut self,
        zones: &BTreeMap<usize, Vec<Dbm>>,
        reached: &[Reached],
    ) -> Result<bool, ZoneCheckRefusal> {
        for node in reached {
            for zone in zones.get(&node.state).into_iter().flatten() {
                self.charge_step()?;
                let mut both = zone.clone();
                intersect(&mut both, &node.zone)?;
                if !both.is_empty() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    #[implements("FR-852-AC-33")]
    fn charge_step(&mut self) -> Result<(), ZoneCheckRefusal> {
        self.poll()?;
        self.computed = self.computed.saturating_add(1);
        let limit = self.limits.max_symbolic_transitions;
        if self.computed > limit {
            return Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxSymbolicTransitions(limit),
            )));
        }
        Ok(())
    }

    fn part(&self, taken: usize) -> Result<&[PlaceConstraint], ZoneCheckRefusal> {
        let moved = &self.moves[taken];
        moved
            .step
            .parts()
            .get(moved.part)
            .map(Vec::as_slice)
            .ok_or(ZoneCheckRefusal::Fault("state-graph-part"))
    }

    /// The valuations of `taken`'s source from which, after a delay its
    /// invariants allow, the move lands in `landing`.
    fn predecessor(&mut self, taken: usize, landing: &Dbm) -> Result<Dbm, ZoneCheckRefusal> {
        self.charge_step()?;
        let moved = &self.moves[taken];
        let source = &self.states[moved.source];
        let scale = &self.graph.scale;
        let mut zone = landing.clone();
        constrain_at(&mut zone, &self.states[moved.target].invariants, scale)?;
        let mut zone = preimage_step_clocks(&zone, &moved.step, scale)?;
        constrain_at(&mut zone, self.part(taken)?, scale)?;
        constrain_at(&mut zone, &source.invariants, scale)?;
        if source.elapses && !zone.is_empty() {
            zone.down();
            constrain_at(&mut zone, &source.invariants, scale)?;
        }
        Ok(zone)
    }

    /// The valuations `taken` reaches from `zone`, a zone of its source, at
    /// the instant of the step.
    fn landing(
        &mut self,
        taken: usize,
        zone: &Dbm,
        scale: &BigInt,
    ) -> Result<Dbm, ZoneCheckRefusal> {
        self.charge_step()?;
        let moved = &self.moves[taken];
        let mut next = zone.clone();
        constrain_at(&mut next, self.part(taken)?, scale)?;
        if !next.is_empty() {
            apply_step_clocks(&mut next, &moved.step, scale)?;
            constrain_at(&mut next, &self.states[moved.target].invariants, scale)?;
        }
        Ok(next)
    }

    /// `zone` closed under the delays `state` allows.
    fn settle(&self, state: usize, zone: &mut Dbm, scale: &BigInt) -> Result<(), ZoneCheckRefusal> {
        let state = &self.states[state];
        constrain_at(zone, &state.invariants, scale)?;
        if state.elapses && !zone.is_empty() {
            zone.up();
            constrain_at(zone, &state.invariants, scale)?;
        }
        Ok(())
    }

    /// The forward zone search from `seeds`, covered under the aM bounds,
    /// breadth-first: every zone it stored, and, with a `goal`, the first
    /// whose state satisfies it, where the search stops.
    fn search(
        &mut self,
        seeds: Vec<(usize, Dbm)>,
        goal: Option<&[bool]>,
    ) -> Result<(Vec<Reached>, Option<usize>), ZoneCheckRefusal> {
        let scale = self.graph.scale.clone();
        let mut reached: Vec<Reached> = Vec::new();
        let mut at_state: Vec<Vec<usize>> = self.states.iter().map(|_| Vec::new()).collect();
        for (state, zone) in seeds {
            if zone.is_empty() {
                continue;
            }
            self.charge_zone(&zone)?;
            at_state[state].push(reached.len());
            reached.push(Reached {
                state,
                zone,
                from: None,
            });
            if goal.is_some_and(|goal| goal[state]) {
                let found = reached.len() - 1;
                return Ok((reached, Some(found)));
            }
        }
        let mut next_node = 0;
        while next_node < reached.len() {
            let state = reached[next_node].state;
            let zone = reached[next_node].zone.clone();
            for taken in self.states[state].leaving.clone() {
                let target = self.moves[taken].target;
                let mut next = self.landing(taken, &zone, &scale)?;
                if next.is_empty() {
                    continue;
                }
                self.settle(target, &mut next, &scale)?;
                for (index, place) in self.timed.places().iter().enumerate() {
                    if !self.timed.live(place, &self.states[target].state) {
                        next.free(index + 1)?;
                    }
                }
                if next.is_empty() {
                    continue;
                }
                let mut covered = false;
                for &other in &at_state[target] {
                    if next.alu_covered_by(
                        &reached[other].zone,
                        &self.graph.lower,
                        &self.graph.upper,
                    )? {
                        covered = true;
                        break;
                    }
                }
                if covered {
                    continue;
                }
                self.charge_zone(&next)?;
                at_state[target].push(reached.len());
                reached.push(Reached {
                    state: target,
                    zone: next,
                    from: Some((taken, next_node)),
                });
                if goal.is_some_and(|goal| goal[target]) {
                    let found = reached.len() - 1;
                    return Ok((reached, Some(found)));
                }
            }
            next_node += 1;
        }
        Ok((reached, None))
    }

    /// `E(P)`: the least fixpoint of the timed and discrete predecessors of
    /// the states where `p` holds, per model state. It terminates because a
    /// predecessor of a union of regions is one.
    #[implements("FR-852-AC-2")]
    fn eventually(&mut self, p: &[bool]) -> Result<EventuallyZones, ZoneCheckRefusal> {
        let mut ef: EventuallyZones = self.states.iter().map(|_| Vec::new()).collect();
        let mut queue = VecDeque::new();
        for (state, _) in p.iter().enumerate().filter(|(_, holds)| **holds) {
            let zone = self.within_invariants(state)?;
            if !zone.is_empty() {
                self.charge_zone(&zone)?;
                queue.push_back((state, ef[state].len()));
                ef[state].push(zone);
            }
        }
        while let Some((state, index)) = queue.pop_front() {
            let landing = ef[state][index].clone();
            for taken in self.states[state].entering.clone() {
                let source = self.moves[taken].source;
                let before = self.predecessor(taken, &landing)?;
                if before.is_empty() {
                    continue;
                }
                for zone in self.outside(&before, &ef[source])? {
                    self.charge_zone(&zone)?;
                    queue.push_back((source, ef[source].len()));
                    ef[source].push(zone);
                }
            }
        }
        Ok(ef)
    }

    /// The moves from an initial node to `node` along the nodes that created
    /// one another, whose zones are exact, and the initial state's index.
    fn tree_path(&self, node: usize) -> Result<(u32, Vec<usize>), ZoneCheckRefusal> {
        let mut at = node;
        let mut moves = Vec::new();
        while let Some((parent, step_index, part)) = self.graph.parents.get(at).copied().flatten() {
            let source = self.of_node[parent];
            let steps = self
                .timed
                .guarded_steps(&self.states[source].state)
                .map_err(|_| ZoneCheckRefusal::Fault("state-graph-guarded-steps"))?;
            let step = steps
                .get(step_index)
                .ok_or(ZoneCheckRefusal::Fault("state-graph-step-index"))?;
            let taken = self.states[source]
                .leaving
                .iter()
                .copied()
                .find(|&taken| {
                    let moved = &self.moves[taken];
                    moved.part == part
                        && moved.target == self.of_node[at]
                        && moved.step.transition() == step.transition()
                })
                .ok_or(ZoneCheckRefusal::Fault("state-graph-parent-move"))?;
            moves.push(taken);
            at = parent;
        }
        moves.reverse();
        let initial = self
            .graph
            .initials
            .iter()
            .position(|&root| root == at)
            .ok_or(ZoneCheckRefusal::Fault("state-graph-path-initial"))?;
        let initial =
            u32::try_from(initial).map_err(|_| ZoneCheckRefusal::Fault("state-graph-initial"))?;
        Ok((initial, moves))
    }

    /// FR-241's delays for `moves` from the clock values `start`, or `None`
    /// when no run takes them to `target`.
    fn concretise(
        &self,
        moves: &[usize],
        start: &[Rational],
        scale: &BigInt,
        target: &Target,
    ) -> Result<Option<Concrete>, ZoneCheckRefusal> {
        let stages: Vec<_> = moves
            .iter()
            .map(|&taken| {
                let moved = &self.moves[taken];
                let source = &self.states[moved.source];
                Stage {
                    invariants: source.invariants.clone(),
                    elapses: source.elapses,
                    target: self.states[moved.target].invariants.clone(),
                    step: Some((moved.step.clone(), moved.part)),
                    position: Vec::new(),
                    position_resets: Vec::new(),
                    before: None,
                    after: None,
                    auxiliary_carries: Vec::new(),
                    auxiliary_resets: Vec::new(),
                    auxiliary_advances: Vec::new(),
                    piece: None,
                    silent: false,
                    fixed: None,
                }
            })
            .collect();
        match concretise_from(self.timed, start, &stages, target, scale) {
            Ok(concrete) => Ok(Some(concrete)),
            Err(NotConcretised::Infeasible) => Ok(None),
            Err(_) => Err(ZoneCheckRefusal::Fault("state-graph-path-concretisation")),
        }
    }

    /// A path to `node` that ends in one of `zones`, parts of the node's
    /// zone, or, when no step lands there, at a valuation a delay takes
    /// there; with the clock values at its end.
    fn stem(
        &self,
        node: usize,
        zones: &[Dbm],
    ) -> Result<(ModelPath, Vec<Rational>), ZoneCheckRefusal> {
        let (initial, moves) = self.tree_path(node)?;
        let state = &self.states[self.of_node[node]];
        let mut candidates = zones.to_vec();
        if state.elapses {
            for zone in zones {
                let mut before = zone.clone();
                before.down();
                constrain_at(&mut before, &state.invariants, &self.graph.scale)?;
                candidates.push(before);
            }
        }
        let start = self.zero_clocks();
        for candidate in candidates {
            self.poll()?;
            let target = Target::Within(candidate);
            if let Some(concrete) = self.concretise(&moves, &start, &self.graph.scale, &target)? {
                return Ok((
                    ModelPath {
                        initial,
                        steps: concrete.prefix,
                    },
                    concrete.last_clocks,
                ));
            }
        }
        Err(ZoneCheckRefusal::Fault("state-graph-stem"))
    }

    /// The `unique path` refutation at `both`, valuations of `node` from
    /// which the two sequences `found` both lead to `Y`.
    fn two_paths(
        &mut self,
        node: usize,
        both: Dbm,
        words: &[Word],
        found: (usize, usize),
    ) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
        let (stem, clocks) = self.stem(node, &[both])?;
        let state = self.of_node[node];
        let first = self.follow(state, &clocks, words, found.0)?;
        let second = self.follow(state, &clocks, words, found.1)?;
        Ok(TimedStateGraphDecision::Refuted {
            stem,
            node,
            ef_nodes: Vec::new(),
            paths: Some((first, second)),
        })
    }

    /// The run of `word`'s transitions from `state` at the clock values
    /// `clocks`, with exact delays.
    fn follow(
        &mut self,
        state: usize,
        clocks: &[Rational],
        words: &[Word],
        mut word: usize,
    ) -> Result<Vec<ModelStep>, ZoneCheckRefusal> {
        // A scale at which the stem's end valuation is a point of integers.
        let mut factor = BigInt::from(1);
        for value in clocks {
            let units = value.numerator().as_big() * &self.graph.scale;
            let denominator = value.denominator().as_big();
            factor = factor.lcm(&(denominator / denominator.gcd(&units)));
        }
        let scale = &self.graph.scale * &factor;
        let mut zone = Dbm::zero(self.dim);
        for (place, value) in clocks.iter().enumerate() {
            zone.reset(place + 1, &scaled(value, &scale))?;
        }
        self.settle(state, &mut zone, &scale)?;
        let mut at = state;
        let mut moves = Vec::new();
        while let Some((transition, rest)) = words
            .get(word)
            .and_then(|word| word.first.as_ref())
            .cloned()
        {
            let mut chosen = None;
            'moves: for taken in self.states[at].leaving.clone() {
                if self.moves[taken].step.transition() != &transition {
                    continue;
                }
                let target = self.moves[taken].target;
                let landed = self.landing(taken, &zone, &scale)?;
                if landed.is_empty() {
                    continue;
                }
                for onward in words[rest].zones.get(&target).into_iter().flatten() {
                    self.poll()?;
                    // The rest's zones are closed under delay predecessors,
                    // so one that a later instant is in holds the landing
                    // too.
                    let mut onward = onward.clone();
                    onward.scale_by(&factor);
                    intersect(&mut onward, &landed)?;
                    if !onward.is_empty() {
                        self.settle(target, &mut onward, &scale)?;
                        chosen = Some((taken, onward));
                        break 'moves;
                    }
                }
            }
            let (taken, next) = chosen.ok_or(ZoneCheckRefusal::Fault("state-graph-word"))?;
            moves.push(taken);
            at = self.moves[taken].target;
            zone = next;
            word = rest;
        }
        Ok(self
            .concretise(&moves, clocks, &scale, &Target::Any)?
            .ok_or(ZoneCheckRefusal::Fault("state-graph-word-concretisation"))?
            .prefix)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use num_bigint::BigInt;

    use super::super::search::{subtract, Split, ZoneCheckLimit, ZoneCheckRefusal, ZoneStop};
    use super::super::{Bound, Dbm};

    /// `low <= x <= low + 1` over one clock.
    fn unit(low: i64) -> Dbm {
        let mut zone = Dbm::universe(2);
        zone.constrain(1, 0, Bound::le(BigInt::from(low + 1)))
            .expect("an upper bound");
        zone.constrain(0, 1, Bound::le(BigInt::from(-low)))
            .expect("a lower bound");
        zone
    }

    #[trace("FR-852-AC-33")]
    #[test]
    fn a_wide_union_is_subtracted_without_recursion_and_within_its_budget() {
        let stopped = || {
            ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(
                100,
            )))
        };
        // 100,000 held zones, none meeting the point: each is read in turn
        // and nothing is split.
        let held: Vec<Dbm> = (0..100_000).map(|index| unit(2 * index + 1)).collect();
        let point = Dbm::zero(2);
        let rest = subtract(&point, &held, &mut |_| Ok::<_, ZoneCheckRefusal>(()));
        assert_eq!(rest, Ok(vec![point]));
        // 12,000 held zones that each split the rest stop at the cap.
        let held: Vec<Dbm> = (0..12_000).map(|index| unit(2 * index + 1)).collect();
        let rest = subtract(&Dbm::universe(2), &held, &mut |split| match split {
            Split::Piece(pieces) if pieces > 100 => Err(stopped()),
            Split::Piece(_) | Split::Read => Ok(()),
        });
        assert_eq!(rest, Err(stopped()));
    }
}
