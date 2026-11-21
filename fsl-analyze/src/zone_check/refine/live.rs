// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143's liveness half over a timed concrete subject (FSpec FR-430,
//! ADR-026 TS-4 and EZ-6): EN-1's second phase on a symbolic refinement
//! product, reading only time-divergent concrete behaviours.
//!
//! A vertex pairs a model state, a refinement position (the safety half's
//! `q`), a cell and a zone over the clock places and, under a dense
//! source, the divergence clock `z`, as FR-240's product does for a claim
//! ([`super::super::liveness`]): the cells split by the guards of `F_C`'s
//! steps, so a concrete constraint's enabledness is exact at every instant
//! (FR-232), a cycle through a reset of `z` has unbounded total delay, and
//! a quiescent state's idle tail stutters on. A discrete edge's position
//! is `check_step`'s; a step `check_step` fails enters a time-locked
//! post-state, since the safety half holds, and takes no behaviour that
//! diverges, so it is not an edge.
//!
//! For each class of `F_A` in order, as for an untimed subject (FR-143),
//! a fair component of the vertices whose position enables the class, over
//! the edges that do not take it, with a reset of `z` and a position on
//! its cycle, is a violation: its loop has positive total delay, which
//! FR-241's exact solution of the loop's delays confirms. The lasso is
//! `Divergence` when the loop takes only abstract stutters and
//! `AbstractUnfair` otherwise.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Integer, Rational};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{
    analyze, Analysis, Constraint, EdgeLabel, FairnessError, Graph, Identity, Strength, Vertex,
};
use fsl_eval::simulation::model::{ModelFinding, ModelSimState, PlaceConstraint};
use fsl_eval::simulation::ExpansionLimit;
use fsl_replay::{
    abstract_classes, candidate_lasso_has_fair_run, check_step, AbstractClass, CandidateFairState,
    CandidateFairStep, ConcreteStep, CounterexampleKind, Lasso, ModelCheckLimit, ReachedLimit,
    RefinementFailure, Taken, TemporalCounterexample, Undetermined,
};
use num_bigint::BigInt;

use super::{
    Judgement, Product, TimedRefinementStop, TimedRefinementUndecided, TimedSafetyOutcome,
};
use crate::model_check::fair_walk::{fairness_constraints, FairModel};
use crate::zone_check::concretise::{concretise, Stage, Target};
use crate::zone_check::liveness::{next_cell, CellAtom, Cells, Sign};
use crate::zone_check::search::{
    constrain_at, scaled, widen, zone_bytes, Halt, Run, ZoneCheckRefusal, ZoneStats, ZoneUndecided,
};
use crate::zone_check::{Bound, Dbm};

/// An outcome that ends the liveness half before it decides: undecided,
/// stopped or a fault (its counts are replaced by the liveness product's).
type Decided = TimedSafetyOutcome<()>;

/// What the liveness half decides.
pub(super) enum Lived {
    /// No class of `F_A` is violated by a fair time-divergent behaviour.
    Holds,
    /// The first violated class's lasso.
    Violated(Box<TemporalCounterexample>),
    /// The half ends with this outcome.
    Ends(Decided),
}

/// One vertex of the product.
#[derive(Clone, Debug)]
struct Node {
    state: ModelSimState,
    /// The refinement position, by its index into the product's.
    q: usize,
    cell: Vec<Sign>,
    /// The zone over the clock places and, under a dense source, `z`.
    zone: Dbm,
    /// Entered by a position: a step, the initial position or a stutter.
    positioned: bool,
    /// Entered by a reset of `z`, or under a tick source by a tick.
    divergent: bool,
    /// In the idle tail: the behaviour takes no further step.
    tail: bool,
    /// The initial state's index, for an initial vertex.
    initial: Option<usize>,
    /// The number of positions from its initial vertex.
    depth: usize,
}

/// How an edge is taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Via {
    Step { step: usize, part: usize },
    Wait,
    Diverge,
    Stutter,
}

#[derive(Clone, Debug)]
struct Arc {
    from: usize,
    to: usize,
    via: Via,
    /// The abstract steps a position takes; `None` for a delay.
    taken: Option<Taken>,
    /// Auxiliary abstract clock operation for a mapped discrete step.
    auxiliary_transform: Option<super::super::concretise::ClockTransform>,
}

/// A position's judgement, by its position, its state's key and its step
/// (`None` for a stutter).
type JudgedKey = (usize, Vec<u8>, Option<usize>);

/// The product of one liveness run.
struct Live {
    nodes: Vec<Node>,
    arcs: Vec<Arc>,
    out: Vec<Vec<usize>>,
    /// The arc that discovered each vertex.
    parent: Vec<Option<usize>>,
    passed: BTreeMap<Vec<u8>, Vec<usize>>,
    cells: Cells,
    dim: usize,
    /// The aM bounds vertices are identified under.
    bounds: Vec<Option<BigInt>>,
    judged: BTreeMap<JudgedKey, Option<(usize, Taken)>>,
    /// What each position's abstract candidates enable over the abstract
    /// clocks, by position; empty with an untimed abstract side.
    views: BTreeMap<usize, AbstractView>,
}

/// What a position's abstract candidates enable over the abstract clocks
/// (FR-899 "Timed subjects"): the guard constants of the `F_A` classes
/// split the cells, and each candidate step's enabling parts decide which
/// cells enable it.
#[derive(Clone, Debug, Default)]
struct AbstractView {
    /// The cell atoms of the `F_A` operations' enabling parts, on the zone's
    /// clocks.
    atoms: Vec<CellAtom>,
    /// Each step of each candidate (its state key), with its enabling parts
    /// (FR-232) on the zone's clocks.
    steps: Vec<(Vec<u8>, ModelTransition, Vec<Vec<PlaceConstraint>>)>,
}

/// A position's step judged: its position after and the abstract steps it
/// takes, `None` when it fails.
enum Moved {
    To(Option<(usize, Taken)>),
    Ends(Decided),
}

impl Product<'_, '_> {
    /// FR-143 over the timed product (FSpec FR-430): the first class of
    /// `F_A` a fair time-divergent behaviour violates, as its lasso.
    ///
    /// # Errors
    ///
    /// [`ZoneUndecided::LassoNotConcretized`] when a violating component
    /// exists and none concretises, or the run's budgets and faults.
    /// The liveness product's own counts come with the decision, whatever
    /// it is: its vertices, its arcs and its zones' bytes.
    pub(super) fn liveness(
        &mut self,
        run: &mut Run<'_, '_, '_>,
    ) -> (Result<Lived, Halt>, ZoneStats) {
        let mut live = match self.live_product(run) {
            Ok(live) => live,
            Err(halt) => return (Err(halt), ZoneStats::default()),
        };
        let decided = self.decide_live(run, &mut live);
        let stats = ZoneStats {
            symbolic_states: widen(live.nodes.len()),
            retained_edges: widen(live.arcs.len()),
            automaton_states: 0,
            zone_bytes: live.nodes.iter().fold(0, |bytes: u64, node| {
                bytes.saturating_add(zone_bytes(&node.zone))
            }),
        };
        (decided, stats)
    }

    /// The empty liveness product of `run`'s subject.
    fn live_product(&self, run: &Run<'_, '_, '_>) -> Result<Live, Halt> {
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        let mut constrained = BTreeSet::new();
        for constraint in &concrete_fairness {
            let operation = constraint.operation();
            let text = run
                .timed
                .system()
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("fairness-operation"))?;
            constrained.insert(text.to_owned());
        }
        let dense = run.timed.tick_period().is_none();
        // The liveness product stores the same concrete + auxiliary clock
        // valuation as EN-6, followed by its divergence clock.
        let dim = run.dim() + usize::from(dense);
        let mut bounds: Vec<Option<BigInt>> = run
            .lower
            .iter()
            .zip(&run.upper)
            .map(|(lower, upper)| lower.clone().max(upper.clone()))
            .collect();
        if dense {
            bounds.push(Some(run.scale.clone()));
        }
        Ok(Live {
            nodes: Vec::new(),
            arcs: Vec::new(),
            out: Vec::new(),
            parent: Vec::new(),
            passed: BTreeMap::new(),
            cells: Cells::new(constrained),
            dim,
            bounds,
            judged: BTreeMap::new(),
            views: BTreeMap::new(),
        })
    }

    /// The decision over `live`, built here.
    fn decide_live(&mut self, run: &mut Run<'_, '_, '_>, live: &mut Live) -> Result<Lived, Halt> {
        if let Some(ends) = self.build(run, live)? {
            return Ok(Lived::Ends(ends));
        }
        // The transition identities each vertex enables, by vertex: from
        // the position's untimed expansion, which also reports a stop or
        // an undetermined successor, and under a timed abstract side from
        // the candidates' guarded steps at the vertex's cell (FR-899
        // "Timed subjects").
        let mut untimed = BTreeMap::new();
        let mut enabled: Vec<Vec<ModelTransition>> = Vec::with_capacity(live.nodes.len());
        for at in 0..live.nodes.len() {
            let q = live.nodes[at].q;
            if let std::collections::btree_map::Entry::Vacant(slot) = untimed.entry(q) {
                match self.abstract_enabled(run, q)? {
                    Ok(here) => {
                        slot.insert(here);
                    }
                    Err(ends) => return Ok(Lived::Ends(ends)),
                }
            }
            if run.auxiliary_timed.is_some() {
                let view = self.abstract_view(run, live, q)?;
                let mut here = Vec::new();
                for (_, transition) in Self::enabled_at_zone(run, &view, &live.nodes[at].zone)? {
                    if !here.contains(&transition) {
                        here.push(transition);
                    }
                }
                enabled.push(here);
            } else {
                enabled.push(untimed.get(&q).cloned().unwrap_or_default());
            }
        }
        let mut identities = BTreeMap::new();
        for transition in enabled
            .iter()
            .flatten()
            .chain(live.arcs.iter().flat_map(|arc| {
                arc.taken.iter().flat_map(|taken| match taken {
                    Taken::Abstract(transitions) => transitions.iter().collect::<Vec<_>>(),
                    Taken::CandidateSteps(steps) => steps
                        .iter()
                        .filter_map(|step| step.transition.as_ref())
                        .collect(),
                    Taken::Stutter | Taken::Protocol(_) | Taken::ProtocolTraces(_) => Vec::new(),
                })
            }))
        {
            let key = self
                .abstract_system
                .transition_key(transition)
                .map_err(|_| ZoneCheckRefusal::Fault("abstract-transition-key"))?;
            identities.entry(key).or_insert_with(|| transition.clone());
        }
        let identities: Vec<ModelTransition> = identities.into_values().collect();
        let abstract_fairness: Vec<_> = self
            .refinement
            .abstract_operation_fairness()
            .cloned()
            .collect();
        let classes = match abstract_classes(self.abstract_system, &abstract_fairness, &identities)
        {
            Ok(classes) => classes,
            Err(fault) => {
                return Ok(Lived::Ends(Decided::Fault {
                    fault,
                    stats: run.stats(),
                }));
            }
        };
        let hidden = self
            .refinement
            .objects()
            .iter()
            .any(|object| !object.hidden.is_empty());
        let mut not_concretised = false;
        for class in &classes {
            let (model, within, kept) = self.fair_model(run, live, class, &enabled)?;
            let analyses = analyze(&model.graph, &within, &model.constraints, run.cancel)
                .map_err(|error| fairness(run, error))?;
            for component in Analysis::fair_in(&analyses) {
                run.poll()?;
                let Some(&entry) = component.vertices().first() else {
                    return Err(ZoneCheckRefusal::Fault("empty-component").into());
                };
                let members: BTreeSet<Vertex> = component.vertices().iter().copied().collect();
                let walk = model
                    .greedy_loop(&members, component.removed(), entry)
                    .map_err(|_| ZoneCheckRefusal::Fault("component-loop"))?;
                let mut cycle = Vec::with_capacity(walk.len());
                for (from, edge) in &walk {
                    let position = model
                        .graph
                        .edges_from(*from)
                        .iter()
                        .position(|held| held == edge)
                        .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                    let index = kept
                        .get(from.index())
                        .and_then(|out| out.get(position))
                        .copied()
                        .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                    cycle.push(index);
                }
                let mut named = class.constraint();
                if hidden {
                    match self.candidate_loop_has_fair_run(
                        run,
                        live,
                        &cycle,
                        FairScope::Through(class.constraint()),
                    )? {
                        // The fair run answers for the whole component only
                        // when the component is that one loop; another loop
                        // through it may still violate, so decide nothing.
                        Ok(true) if single_loop(&model.graph, &members) => continue,
                        Ok(true) => {
                            not_concretised = true;
                            continue;
                        }
                        Ok(false) => {
                            named = match self.first_unfair_constraint(run, live, &cycle)? {
                                Ok(index) => index,
                                Err(ends) => return Ok(Lived::Ends(ends)),
                            };
                        }
                        Err(ends) => return Ok(Lived::Ends(ends)),
                    }
                }
                if let Some(found) = self.lasso(run, live, class, named, entry.index(), &cycle)? {
                    return Ok(Lived::Violated(Box::new(found)));
                }
                not_concretised = true;
            }
        }
        if not_concretised {
            return Err(run.undecided(ZoneUndecided::LassoNotConcretized));
        }
        Ok(Lived::Holds)
    }

    /// Explore the product breadth first, then the idle tails.
    fn build(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
    ) -> Result<Option<Decided>, Halt> {
        let concrete = run.timed.system();
        let mut queue = VecDeque::new();
        for (index, initial) in run.timed.initial().iter().enumerate() {
            run.poll()?;
            let state = initial.discrete().clone();
            let verdict = fsl_replay::check_initial(
                self.refinement,
                concrete,
                self.abstract_system,
                index,
                state.state(),
            );
            let q = match self.judge(verdict, run)? {
                Judgement::Passes(q, _) => q,
                // A failing initial state is time-locked, since the safety
                // half holds: no behaviour from it diverges.
                Judgement::Fails(_) => continue,
                Judgement::Ends(ends) => return Ok(Some(ends)),
            };
            let mut zero = Dbm::zero(live.dim);
            self.auxiliary_invariants(run, q, &mut zero)?;
            let extra = self.abstract_view(run, live, q)?.atoms;
            for (cell, piece) in run.split(&mut live.cells, &state, &extra, &zero)? {
                let Some(zone) = run.close(&mut live.cells, &state, &extra, piece, &cell)? else {
                    continue;
                };
                let node = Node {
                    state: state.clone(),
                    q,
                    cell,
                    zone,
                    positioned: true,
                    divergent: false,
                    tail: false,
                    initial: Some(index),
                    depth: 0,
                };
                if let Some(new) = add_node(run, live, node, None)? {
                    queue.push_back(new);
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            run.poll()?;
            match self.expand_live(run, live, at)? {
                Ok(created) => queue.extend(created),
                Err(ends) => return Ok(Some(ends)),
            }
        }
        // The idle tails come after every stepping vertex.
        for at in 0..live.nodes.len() {
            run.poll()?;
            let node = live.nodes[at].clone();
            let idles = !node.tail
                && match run.timed.tick_period() {
                    Some(_) => run.steps_for(&node.state)?.is_empty(),
                    None => {
                        run.timed(run.timed.is_quiescent(&node.state))?
                            && run.elapses(&node.state)?
                    }
                };
            if idles {
                match self.stutter(run, live, at)? {
                    Ok(created) => queue.extend(created),
                    Err(ends) => return Ok(Some(ends)),
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            run.poll()?;
            match self.expand_live(run, live, at)? {
                Ok(created) => queue.extend(created),
                Err(ends) => return Ok(Some(ends)),
            }
        }
        Ok(None)
    }

    /// The vertices `at`'s edges create, in canonical order.
    fn expand_live(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        at: usize,
    ) -> Result<Result<Vec<usize>, Decided>, Halt> {
        let node = live.nodes[at].clone();
        let dense = run.timed.tick_period().is_none();
        let scale = run.scale.clone();
        let mut created = Vec::new();
        if node.tail {
            match self.stutter(run, live, at)? {
                Ok(more) => created.extend(more),
                Err(ends) => return Ok(Err(ends)),
            }
        }
        let steps = if node.tail {
            Vec::new()
        } else {
            run.steps_for(&node.state)?
        };
        for (index, step) in steps.iter().enumerate() {
            let target = run.timed(run.timed.invariants(step.post()))?;
            let tick =
                run.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
            for (part_index, part) in step.parts().iter().enumerate() {
                run.poll()?;
                let mut next = node.zone.clone();
                constrain_at(&mut next, part, &scale)?;
                if next.is_empty() {
                    continue;
                }
                for (place, value) in step.resets() {
                    next.reset(place + 1, &scaled(value, &scale))?;
                }
                constrain_at(&mut next, &target, &scale)?;
                if next.is_empty() {
                    continue;
                }
                let post = step.post().clone();
                let moved =
                    self.moved(run, live, &node, Some((index, step.transition())), &post)?;
                let (q, taken) = match moved {
                    Moved::To(Some(moved)) => moved,
                    Moved::To(None) => continue,
                    Moved::Ends(ends) => return Ok(Err(ends)),
                };
                let (failure, auxiliary_transform) =
                    self.auxiliary_step(run, node.q, q, &taken, &mut next)?;
                if failure.is_some() || next.is_empty() {
                    continue;
                }
                let extra = self.abstract_view(run, live, q)?.atoms;
                for (cell, piece) in run.split(&mut live.cells, &post, &extra, &next)? {
                    let Some(entered) = run.close(&mut live.cells, &post, &extra, piece, &cell)?
                    else {
                        continue;
                    };
                    let next = Node {
                        state: post.clone(),
                        q,
                        cell,
                        zone: entered,
                        positioned: true,
                        divergent: !dense && tick,
                        tail: false,
                        initial: None,
                        depth: node.depth + 1,
                    };
                    let arc = Arc {
                        from: at,
                        to: 0,
                        via: Via::Step {
                            step: index,
                            part: part_index,
                        },
                        taken: Some(taken.clone()),
                        auxiliary_transform: auxiliary_transform.clone(),
                    };
                    created.extend(add_node(run, live, next, Some(arc))?);
                }
            }
        }
        if dense && run.elapses(&node.state)? {
            // A delay into each next cell.
            let invariants = run.timed(run.timed.invariants(&node.state))?;
            let mut later = node.zone.clone();
            later.up();
            constrain_at(&mut later, &invariants, &scale)?;
            self.auxiliary_invariants(run, node.q, &mut later)?;
            let extra = self.abstract_view(run, live, node.q)?.atoms;
            for (cell, piece) in run.split(&mut live.cells, &node.state, &extra, &later)? {
                if !next_cell(&node.cell, &cell) {
                    continue;
                }
                let Some(entered) =
                    run.close(&mut live.cells, &node.state, &extra, piece, &cell)?
                else {
                    continue;
                };
                let next = Node {
                    cell,
                    zone: entered,
                    positioned: false,
                    divergent: false,
                    initial: None,
                    ..node.clone()
                };
                let arc = Arc {
                    from: at,
                    to: 0,
                    via: Via::Wait,
                    taken: None,
                    auxiliary_transform: None,
                };
                created.extend(add_node(run, live, next, Some(arc))?);
            }
        }
        if dense {
            // The divergence clock's reset once a unit has passed.
            let z = live.dim - 1;
            let mut reset = node.zone.clone();
            reset.constrain(0, z, Bound::le(-scale.clone()))?;
            if !reset.is_empty() {
                reset.reset(z, &BigInt::from(0))?;
                let extra = extra_atoms(live, node.q);
                if let Some(entered) =
                    run.close(&mut live.cells, &node.state, &extra, reset, &node.cell)?
                {
                    let next = Node {
                        zone: entered,
                        positioned: false,
                        divergent: true,
                        initial: None,
                        ..node.clone()
                    };
                    let arc = Arc {
                        from: at,
                        to: 0,
                        via: Via::Diverge,
                        taken: None,
                        auxiliary_transform: None,
                    };
                    created.extend(add_node(run, live, next, Some(arc))?);
                }
            }
        }
        Ok(Ok(created))
    }

    /// The idle-tail stutter from `at`: the state and zone kept, the
    /// position `check_step` gives a stutter.
    fn stutter(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        at: usize,
    ) -> Result<Result<Vec<usize>, Decided>, Halt> {
        let node = live.nodes[at].clone();
        let dense = run.timed.tick_period().is_none();
        let (q, taken) = match self.moved(run, live, &node, None, &node.state)? {
            Moved::To(Some(moved)) => moved,
            Moved::To(None) => return Ok(Ok(Vec::new())),
            Moved::Ends(ends) => return Ok(Err(ends)),
        };
        let next = Node {
            q,
            positioned: true,
            divergent: !dense,
            tail: true,
            initial: None,
            depth: node.depth + 1,
            ..node.clone()
        };
        let arc = Arc {
            from: at,
            to: 0,
            via: Via::Stutter,
            taken: Some(taken),
            auxiliary_transform: None,
        };
        if run.auxiliary_timed.is_none() {
            return Ok(Ok(add_node(run, live, next, Some(arc))?
                .into_iter()
                .collect()));
        }
        // The stutter's position reads other abstract guards: its zone is
        // split again by them, each piece a vertex.
        let extra = self.abstract_view(run, live, q)?.atoms;
        let mut created = Vec::new();
        for (cell, piece) in run.split(&mut live.cells, &node.state, &extra, &node.zone)? {
            let Some(zone) = run.close(&mut live.cells, &node.state, &extra, piece, &cell)? else {
                continue;
            };
            let next = Node {
                cell,
                zone,
                ..next.clone()
            };
            created.extend(add_node(run, live, next, Some(arc.clone()))?);
        }
        Ok(Ok(created))
    }

    /// `check_step` on the step `transition` (by its index; `None` for a
    /// stutter) from `node` to `post`, once per position, state and step.
    fn moved(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        node: &Node,
        transition: Option<(usize, &ModelTransition)>,
        post: &ModelSimState,
    ) -> Result<Moved, Halt> {
        let key = (
            node.q,
            run.key(&node.state)?,
            transition.map(|(index, _)| index),
        );
        if let Some(moved) = live.judged.get(&key) {
            return Ok(Moved::To(moved.clone()));
        }
        let result = check_step(
            self.refinement,
            run.timed.system(),
            self.abstract_system,
            &self.positions[node.q],
            &ConcreteStep {
                position: widen(node.depth.saturating_add(1)),
                transition: transition.map(|(_, transition)| transition),
                post,
            },
        );
        let moved = match self.judge(result.verdict, run)? {
            Judgement::Passes(q, taken) => Some((q, taken)),
            // A failing step enters a time-locked post-state, since the
            // safety half holds (FSpec FR-430): no behaviour through it
            // diverges.
            Judgement::Fails(_) => None,
            Judgement::Ends(ends) => return Ok(Moved::Ends(ends)),
        };
        live.judged.insert(key, moved.clone());
        Ok(Moved::To(moved))
    }

    /// The abstract transition identities enabled from position `q`'s
    /// mapped states (ADR-020 CO-2).
    fn abstract_enabled(
        &self,
        run: &Run<'_, '_, '_>,
        q: usize,
    ) -> Result<Result<Vec<ModelTransition>, Decided>, Halt> {
        let mut here = Vec::new();
        for candidate in &self.positions[q].abstract_states {
            run.poll()?;
            let expansion = match self.abstract_system.expand_state(candidate) {
                Ok(expansion) => expansion,
                Err(stop) if stop.is_internal_failure() => {
                    return Ok(Err(Decided::Fault {
                        fault: InternalFault::new("refinement-liveness", "abstract-expansion"),
                        stats: run.stats(),
                    }));
                }
                Err(stop) => {
                    let reached = match stop.limit {
                        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
                            limit: ModelCheckLimit::EvaluationMeter { counter },
                            value: fsl_replay::meter_bound(&self.abstract_system.meter(), counter),
                        },
                        Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
                            limit: ModelCheckLimit::MaxCandidates,
                            value: self.abstract_system.max_candidates(),
                        },
                    };
                    return Ok(Err(Decided::Stopped {
                        cause: TimedRefinementStop::Step(reached),
                        stats: run.stats(),
                    }));
                }
            };
            if expansion
                .findings
                .iter()
                .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
            {
                return Ok(Err(Decided::Undecided {
                    cause: TimedRefinementUndecided::Undetermined(Undetermined::UndecidedSuccessor),
                    stats: run.stats(),
                }));
            }
            here.extend(
                expansion
                    .successors
                    .into_iter()
                    .map(|(transition, _)| transition),
            );
        }
        Ok(Ok(here))
    }

    /// What position `q`'s abstract candidates enable over the abstract
    /// clocks, computed once (FR-899 "Timed subjects").
    fn abstract_view(
        &self,
        run: &Run<'_, '_, '_>,
        live: &mut Live,
        q: usize,
    ) -> Result<AbstractView, Halt> {
        if let Some(view) = live.views.get(&q) {
            return Ok(view.clone());
        }
        let view = self.build_view(run, q)?;
        live.views.insert(q, view.clone());
        Ok(view)
    }

    /// [`Self::abstract_view`], not stored: each step of each candidate of
    /// position `q` with its enabling parts (FR-232) on the zone's clocks,
    /// and the guard constants of the `F_A` operations' steps as cell atoms.
    /// Empty with an untimed abstract side.
    fn build_view(&self, run: &Run<'_, '_, '_>, q: usize) -> Result<AbstractView, Halt> {
        let mut view = AbstractView::default();
        let Some(timed) = run.auxiliary_timed else {
            return Ok(view);
        };
        let offset = run.timed.places().len();
        let mut constrained = BTreeSet::new();
        for constraint in self.refinement.abstract_operation_fairness() {
            let operation = constraint.operation();
            let text = self
                .abstract_system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("abstract-fairness-operation"))?;
            constrained.insert(text.to_owned());
        }
        let position = self
            .positions
            .get(q)
            .ok_or(ZoneCheckRefusal::Fault("abstract-view-position"))?;
        let mut atoms = BTreeSet::new();
        for candidate in &position.abstract_states {
            run.poll()?;
            let key = self
                .abstract_system
                .state_key_of(candidate)
                .map_err(|_| ZoneCheckRefusal::Fault("abstract-view-state-key"))?
                .0;
            let state = timed
                .system()
                .pre_state(candidate.clone(), timed.system().declared_anchor());
            for step in run.timed(timed.guarded_steps(&state))? {
                let parts: Vec<Vec<PlaceConstraint>> = run
                    .enabling_parts_in(timed, &step)?
                    .iter()
                    .map(|part| super::auxiliary_constraints(offset, part))
                    .collect();
                if constrained.contains(step.operation()) {
                    for constraint in parts.iter().flatten() {
                        atoms.insert((
                            constraint.place() + 1,
                            scaled(constraint.bound(), &run.scale),
                        ));
                    }
                }
                view.steps
                    .push((key.clone(), step.transition().clone(), parts));
            }
        }
        view.atoms = atoms.into_iter().collect();
        Ok(view)
    }

    /// The candidate steps of `view` enabled at the instants of `zone`, a
    /// cell of the position (so each is enabled at every instant of it or
    /// at none): the candidate's state key and the transition identity.
    fn enabled_at_zone(
        run: &Run<'_, '_, '_>,
        view: &AbstractView,
        zone: &Dbm,
    ) -> Result<Vec<(Vec<u8>, ModelTransition)>, Halt> {
        let mut here: Vec<(Vec<u8>, ModelTransition)> = Vec::new();
        for (key, transition, parts) in &view.steps {
            run.poll()?;
            for part in parts {
                let mut at = zone.clone();
                constrain_at(&mut at, part, &run.scale)?;
                if !at.is_empty() {
                    let held = (key.clone(), transition.clone());
                    if !here.contains(&held) {
                        here.push(held);
                    }
                    break;
                }
            }
        }
        Ok(here)
    }

    fn candidate_loop_has_fair_run(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &Live,
        cycle: &[usize],
        scope: FairScope,
    ) -> Result<Result<bool, Decided>, Halt> {
        let mut positions = Vec::with_capacity(cycle.len());
        let mut transitions = BTreeSet::<ModelTransition>::new();
        for &arc_index in cycle {
            run.poll()?;
            let arc = live
                .arcs
                .get(arc_index)
                .ok_or(ZoneCheckRefusal::Fault("candidate-loop-arc"))?;
            let source = live
                .nodes
                .get(arc.from)
                .ok_or(ZoneCheckRefusal::Fault("candidate-loop-source"))?;
            let mut candidates = Vec::new();
            let timed_enabled = if run.auxiliary_timed.is_some() {
                let view = live
                    .views
                    .get(&source.q)
                    .ok_or(ZoneCheckRefusal::Fault("abstract-view-cached"))?;
                Some(Self::enabled_at_zone(run, view, &source.zone)?)
            } else {
                None
            };
            for candidate in &self.positions[source.q].abstract_states {
                run.poll()?;
                let (key, _) = self
                    .abstract_system
                    .state_key_of(candidate)
                    .map_err(|_| ZoneCheckRefusal::Fault("candidate-loop-state-key"))?;
                let expansion = match self.abstract_system.expand_state(candidate) {
                    Ok(expansion) => expansion,
                    Err(stop) if stop.is_internal_failure() => {
                        return Err(ZoneCheckRefusal::Fault("candidate-loop-expansion").into());
                    }
                    Err(stop) => {
                        let reached = match stop.limit {
                            Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
                                limit: ModelCheckLimit::EvaluationMeter { counter },
                                value: fsl_replay::meter_bound(
                                    &self.abstract_system.meter(),
                                    counter,
                                ),
                            },
                            Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
                                limit: ModelCheckLimit::MaxCandidates,
                                value: self.abstract_system.max_candidates(),
                            },
                        };
                        return Ok(Err(Decided::Stopped {
                            cause: TimedRefinementStop::Step(reached),
                            stats: run.stats(),
                        }));
                    }
                };
                if expansion
                    .findings
                    .iter()
                    .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
                {
                    return Ok(Err(Decided::Undecided {
                        cause: TimedRefinementUndecided::Undetermined(
                            Undetermined::UndecidedSuccessor,
                        ),
                        stats: run.stats(),
                    }));
                }
                // Under a timed abstract side the candidate's steps enabled at
                // the source vertex's cell, not its untimed successors.
                let enabled: Vec<_> = match &timed_enabled {
                    Some(held) => {
                        let mut here: Vec<ModelTransition> = Vec::new();
                        for (candidate_key, transition) in held {
                            if *candidate_key == key && !here.contains(transition) {
                                here.push(transition.clone());
                            }
                        }
                        here
                    }
                    None => expansion
                        .successors
                        .into_iter()
                        .map(|(transition, _)| transition)
                        .collect(),
                };
                transitions.extend(enabled.iter().cloned());
                candidates.push((key, enabled));
            }
            positions.push(candidates);
            match arc.taken.as_ref() {
                Some(Taken::CandidateSteps(rows)) => {
                    transitions.extend(rows.iter().filter_map(|row| row.transition.clone()));
                }
                Some(Taken::Stutter) | None => {}
                Some(Taken::Abstract(_)) => {
                    return Err(ZoneCheckRefusal::Fault(
                        "hidden-model-edge-retains-candidate-steps",
                    )
                    .into());
                }
                Some(Taken::Protocol(_) | Taken::ProtocolTraces(_)) => {
                    return Err(ZoneCheckRefusal::Fault(
                        "hidden-model-edge-is-not-a-protocol-edge",
                    )
                    .into());
                }
            }
        }
        let numbered: BTreeMap<_, _> = transitions
            .into_iter()
            .enumerate()
            .map(|(index, transition)| (transition, Identity::new(widen(index))))
            .collect();
        let fair_positions = positions
            .iter()
            .map(|candidates| {
                candidates
                    .iter()
                    .map(|(key, enabled)| {
                        Ok(CandidateFairState {
                            key: key.clone(),
                            enabled: enabled
                                .iter()
                                .map(|transition| {
                                    numbered.get(transition).copied().ok_or(
                                        ZoneCheckRefusal::Fault(
                                            "enabled-candidate-transition-numbered",
                                        ),
                                    )
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>, ZoneCheckRefusal>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let fair_steps = cycle
            .iter()
            .enumerate()
            .map(|(index, &arc_index)| {
                let arc = &live.arcs[arc_index];
                let source = &positions[index];
                let rows = match arc.taken.as_ref() {
                    Some(Taken::CandidateSteps(rows)) => rows
                        .iter()
                        .map(|row| {
                            Ok(CandidateFairStep {
                                pre_state: row.pre_state.clone(),
                                transition: row
                                    .transition
                                    .as_ref()
                                    .map(|transition| {
                                        numbered.get(transition).copied().ok_or(
                                            ZoneCheckRefusal::Fault(
                                                "taken-candidate-transition-numbered",
                                            ),
                                        )
                                    })
                                    .transpose()?,
                                post_state: row.post_state.clone(),
                                interior: Vec::new(),
                            })
                        })
                        .collect::<Result<Vec<_>, ZoneCheckRefusal>>()?,
                    Some(Taken::Stutter) | None => source
                        .iter()
                        .map(|(key, _)| CandidateFairStep {
                            pre_state: key.clone(),
                            transition: None,
                            post_state: key.clone(),
                            interior: Vec::new(),
                        })
                        .collect(),
                    _ => return Err(ZoneCheckRefusal::Fault("candidate-loop-taken-shape")),
                };
                Ok(rows)
            })
            .collect::<Result<Vec<_>, ZoneCheckRefusal>>()?;
        let fairness_constraints: Vec<_> = self
            .refinement
            .abstract_operation_fairness()
            .cloned()
            .collect();
        let mut constraints = Vec::new();
        for (index, constraint) in fairness_constraints.iter().enumerate() {
            if !scope.covers(index) {
                continue;
            }
            let operation = constraint.operation();
            let name = self
                .abstract_system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("candidate-fairness-operation"))?;
            let covered: Vec<_> = numbered
                .iter()
                .filter_map(|(transition, identity)| {
                    (transition.operation() == name).then_some(*identity)
                })
                .collect();
            let strength = Strength::from(constraint.kind());
            match constraint.granularity() {
                FairnessGranularity::Whole => {
                    constraints.push(Constraint::new(strength, covered));
                }
                FairnessGranularity::Each => constraints.extend(
                    covered
                        .into_iter()
                        .map(|identity| Constraint::new(strength, [identity])),
                ),
            }
        }
        match candidate_lasso_has_fair_run(&fair_positions, &fair_steps, &constraints, run.cancel) {
            Ok(fair) => Ok(Ok(fair)),
            Err(error) => Err(fairness(
                run,
                match error {
                    fsl_replay::CandidateLivenessError::Fairness(error) => error,
                    fsl_replay::CandidateLivenessError::InvalidGraph => FairnessError::EmptyLoop,
                },
            )),
        }
    }

    /// FR-899's naming rule for a loop no candidate run satisfies `F_A` along:
    /// the first constraint, in source order, that every candidate cycle
    /// violates on its own; when each is satisfiable alone, the first
    /// constraint whose prefix no candidate cycle satisfies jointly. Replay
    /// applies the same rule.
    fn first_unfair_constraint(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &Live,
        cycle: &[usize],
    ) -> Result<Result<usize, Decided>, Halt> {
        let count = self.refinement.abstract_operation_fairness().count();
        for index in 0..count {
            match self.candidate_loop_has_fair_run(run, live, cycle, FairScope::Only(index))? {
                Ok(false) => return Ok(Ok(index)),
                Ok(true) => {}
                Err(ends) => return Ok(Err(ends)),
            }
        }
        for index in 0..count {
            match self.candidate_loop_has_fair_run(run, live, cycle, FairScope::Through(index))? {
                Ok(false) => return Ok(Ok(index)),
                Ok(true) => {}
                Err(ends) => return Ok(Err(ends)),
            }
        }
        Err(ZoneCheckRefusal::Fault("an-unsatisfied-candidate-fairness-prefix-exists").into())
    }

    /// The product as the fairness kernels read it for `class`: each
    /// vertex with the concrete steps enabled at its instants, the edges
    /// that do not take `class`, divergence and positions as acceptance
    /// sets; the vertices whose position enables `class`; and the arcs
    /// kept from each vertex, in the graph's order.
    #[allow(
        clippy::type_complexity,
        reason = "the model, the vertices and the kept arcs"
    )]
    fn fair_model(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        class: &AbstractClass,
        enabled: &[Vec<ModelTransition>],
    ) -> Result<(FairModel, BTreeSet<Vertex>, Vec<Vec<usize>>), Halt> {
        let hidden = self
            .refinement
            .objects()
            .iter()
            .any(|object| !object.hidden.is_empty());
        let mut transitions: Vec<ModelTransition> = Vec::new();
        let mut number = |transition: &ModelTransition| -> Identity {
            let index = match transitions.iter().position(|held| held == transition) {
                Some(index) => index,
                None => {
                    transitions.push(transition.clone());
                    transitions.len() - 1
                }
            };
            Identity::new(widen(index))
        };
        let mut graph = Graph::new();
        let mut handles = Vec::with_capacity(live.nodes.len());
        for node in &live.nodes {
            let mut here = Vec::new();
            for step in &run.steps_for(&node.state)? {
                if run.enabled_at(&node.zone, step)? {
                    here.push(number(step.transition()));
                }
            }
            handles.push(graph.add_vertex(here));
        }
        let mut kept = vec![Vec::new(); live.nodes.len()];
        for (from, out) in live.out.iter().enumerate() {
            for &index in out {
                let arc = &live.arcs[index];
                if !hidden
                    && arc
                        .taken
                        .as_ref()
                        .is_some_and(|taken| class.is_taken_by(taken))
                {
                    continue;
                }
                let label = match arc.via {
                    Via::Step { step, .. } => {
                        let steps = run.steps_for(&live.nodes[from].state)?;
                        let step = steps
                            .get(step)
                            .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                        EdgeLabel::Transition(number(step.transition()))
                    }
                    Via::Wait | Via::Diverge | Via::Stutter => EdgeLabel::Stutter,
                };
                graph
                    .add_edge(handles[from], handles[arc.to], label)
                    .map_err(|_| ZoneCheckRefusal::Fault("fairness-edge"))?;
                kept[from].push(index);
            }
        }
        for flag in [
            (|node: &Node| node.divergent) as fn(&Node) -> bool,
            |node: &Node| node.positioned,
        ] {
            let set: Vec<Vertex> = handles
                .iter()
                .zip(&live.nodes)
                .filter(|(_, node)| flag(node))
                .map(|(handle, _)| *handle)
                .collect();
            graph
                .add_acceptance_set(set)
                .map_err(|_| ZoneCheckRefusal::Fault("acceptance-set"))?;
        }
        let constraint = self
            .refinement
            .abstract_operation_fairness()
            .nth(class.constraint())
            .ok_or(ZoneCheckRefusal::Fault("class-constraint"))?;
        let strong = !hidden && constraint.kind() == FairnessKind::Strong;
        let class_enabled: Vec<Vertex> = handles
            .iter()
            .zip(enabled)
            .filter(|(_, here)| class.is_enabled_in(here))
            .map(|(handle, _)| *handle)
            .collect();
        if strong {
            graph
                .add_acceptance_set(class_enabled.iter().copied())
                .map_err(|_| ZoneCheckRefusal::Fault("strong-abstract-acceptance"))?;
        }
        let within: BTreeSet<Vertex> = if hidden || strong {
            handles.iter().copied().collect()
        } else {
            class_enabled.iter().copied().collect()
        };
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        let constraints = fairness_constraints(
            run.timed.system(),
            &concrete_fairness,
            &transitions,
            |transition| {
                transitions
                    .iter()
                    .position(|held| held == transition)
                    .map(|index| Identity::new(widen(index)))
                    .ok_or_else(|| {
                        crate::model_check::ModelCheckRefusal::Fault(InternalFault::new(
                            "zone-check",
                            "a-constrained-transition-is-numbered",
                        ))
                    })
            },
        )
        .map_err(|_| ZoneCheckRefusal::Fault("fairness-constraints"))?;
        Ok((FairModel { graph, constraints }, within, kept))
    }

    /// FR-241: the lasso of the stem to `entry` and the loop `cycle` of
    /// arcs from it, which violates `class`, or `None` when it does not
    /// concretise.
    fn lasso(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        _class: &AbstractClass,
        named: usize,
        entry: usize,
        cycle: &[usize],
    ) -> Result<Option<TemporalCounterexample>, Halt> {
        let taking = cycle.iter().any(|&arc| {
            live.arcs[arc]
                .taken
                .as_ref()
                .is_some_and(|taken| *taken != Taken::Stutter)
        });
        let constraint = self
            .refinement
            .abstract_operation_fairness()
            .nth(named)
            .ok_or(ZoneCheckRefusal::Fault("class-constraint"))?
            .clone();
        let hidden = self
            .refinement
            .objects()
            .iter()
            .any(|object| !object.hidden.is_empty());
        let failure = if hidden {
            RefinementFailure::HiddenUnfair {
                constraint: constraint.clone(),
            }
        } else if taking || constraint.kind() == FairnessKind::Strong {
            RefinementFailure::AbstractUnfair { constraint }
        } else {
            RefinementFailure::Divergence {
                constraint: Some(constraint),
                protocol_class: None,
            }
        };
        let counterexample = |initial, prefix, lasso| TemporalCounterexample {
            initial_state: initial,
            prefix,
            over_binding: None,
            fairness: self
                .refinement
                .concrete_operation_fairness()
                .cloned()
                .collect(),
            kind: CounterexampleKind::Formula,
            lasso: Some(lasso),
            refinement: Some(failure.clone()),
            blocked: Vec::new(),
            final_delay: None,
        };
        let places = run.dim() - 1;
        let mut stem = path_to(live, entry);
        if live.nodes[entry].tail {
            // The idle tail: the stem to the quiescent state, then its
            // stutter forever (FR-236).
            let tail = stem
                .iter()
                .position(|&arc| live.arcs[arc].via == Via::Stutter)
                .ok_or(ZoneCheckRefusal::Fault("idle-entry"))?;
            // With no step before the tail, the quiescent state is initial.
            let quiescent = live.arcs[stem[tail]].from;
            let (initial, stages) = stages_along(run, live, &stem[..tail], quiescent)?;
            let Ok(concrete) = concretise(run.timed, places, &stages, &Target::Any, &run.scale)
            else {
                return Ok(None);
            };
            let entry = concrete.prefix.len();
            return Ok(Some(counterexample(
                initial,
                concrete.prefix,
                Lasso {
                    entry,
                    terminal_stutter: true,
                    terminal_stutter_delay: None,
                },
            )));
        }
        // The loop starts at a position: the entry when a position enters
        // it, otherwise just after the cycle's first step.
        let mut looped: Vec<usize> = if live.nodes[entry].positioned {
            cycle.to_vec()
        } else {
            let first = cycle
                .iter()
                .position(|&arc| matches!(live.arcs[arc].via, Via::Step { .. }))
                .ok_or(ZoneCheckRefusal::Fault("loop-position"))?;
            stem.extend_from_slice(&cycle[..=first]);
            cycle[first + 1..]
                .iter()
                .chain(&cycle[..=first])
                .copied()
                .collect()
        };
        // The loop rolled back over the stem while the stem's last step is
        // the loop's last, from a position of equal state, refinement
        // position and cell: the shortest stem of the same loop.
        while let (Some(&last_stem), Some(&last_loop)) = (stem.last(), looped.last()) {
            let (left, right) = (&live.arcs[last_stem], &live.arcs[last_loop]);
            let (from, to) = (&live.nodes[left.from], &live.nodes[right.from]);
            let same = matches!(left.via, Via::Step { .. })
                && left.via == right.via
                && left.to == right.to
                && from.positioned
                && from.q == to.q
                && from.cell == to.cell
                && run.key(&from.state)? == run.key(&to.state)?;
            if !same {
                break;
            }
            stem.pop();
            looped.pop();
            looped.insert(0, last_stem);
        }
        let start = live.arcs[looped[0]].from;
        let (initial, stem_stages) = stages_along(run, live, &stem, start)?;
        let mut cycle_stages = Vec::with_capacity(looped.len());
        for &arc in &looped {
            if live.arcs[arc].via != Via::Diverge {
                cycle_stages.push(stage(run, live, arc)?);
            }
        }
        // The loop's unreset clocks stand past every constant of either
        // model (FR-236).
        let ceiling = run.auxiliary_timed.map_or_else(
            || run.timed.max_constant(),
            |auxiliary| run.timed.max_constant().max(auxiliary.max_constant()),
        );
        Ok(run
            .timed_lasso(places, &ceiling, stem_stages, &cycle_stages)?
            .map(|(prefix, entry)| {
                counterexample(
                    initial,
                    prefix,
                    Lasso {
                        entry,
                        terminal_stutter: false,
                        terminal_stutter_delay: None,
                    },
                )
            }))
    }
}

/// Store `node` unless one with equal state, position, cell and flags
/// holds an aM-equivalent zone; the new vertex, or `None` with `arc`
/// recorded to the equivalent one.
fn add_node(
    run: &mut Run<'_, '_, '_>,
    live: &mut Live,
    node: Node,
    arc: Option<Arc>,
) -> Result<Option<usize>, Halt> {
    let mut key = run.key(&node.state)?;
    key.extend_from_slice(&widen(node.q).to_be_bytes());
    key.extend(node.cell.iter().map(|sign| *sign as u8));
    key.push(u8::from(node.positioned));
    key.push(u8::from(node.divergent));
    key.push(u8::from(node.tail));
    if let Some(stored) = live.passed.get(&key) {
        for &candidate in stored {
            let held = &live.nodes[candidate].zone;
            let same = node.zone.alu_covered_by(held, &live.bounds, &live.bounds)?
                && held.alu_covered_by(&node.zone, &live.bounds, &live.bounds)?;
            if same {
                if let Some(arc) = arc {
                    add_arc(
                        run,
                        live,
                        Arc {
                            to: candidate,
                            ..arc
                        },
                    )?;
                }
                return Ok(None);
            }
        }
    }
    run.charge_vertex(zone_bytes(&node.zone))?;
    run.escape_state();
    let index = live.nodes.len();
    live.nodes.push(node);
    live.out.push(Vec::new());
    live.parent.push(None);
    live.passed.entry(key).or_default().push(index);
    if let Some(arc) = arc {
        add_arc(run, live, Arc { to: index, ..arc })?;
        live.parent[index] = Some(live.arcs.len() - 1);
    }
    Ok(Some(index))
}

fn add_arc(run: &mut Run<'_, '_, '_>, live: &mut Live, arc: Arc) -> Result<(), Halt> {
    run.charge_edge()?;
    run.escape_edge();
    live.out[arc.from].push(live.arcs.len());
    live.arcs.push(arc);
    Ok(())
}

/// The arcs from an initial vertex to `node`.
fn path_to(live: &Live, node: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut at = node;
    while let Some(arc) = live.parent[at] {
        path.push(arc);
        at = live.arcs[arc].from;
    }
    path.reverse();
    path
}

/// The initial state's index and the stages of the arcs `path`, which
/// starts at an initial vertex and ends at `end`; resets of `z` take no
/// stage.
fn stages_along(
    run: &mut Run<'_, '_, '_>,
    live: &mut Live,
    path: &[usize],
    end: usize,
) -> Result<(usize, Vec<Stage>), Halt> {
    let root = path.first().map_or(end, |&arc| live.arcs[arc].from);
    let initial = live.nodes[root]
        .initial
        .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
    let mut stages = Vec::with_capacity(path.len());
    for &arc in path {
        if live.arcs[arc].via != Via::Diverge {
            stages.push(stage(run, live, arc)?);
        }
    }
    Ok((initial, stages))
}

/// The abstract cell atoms of position `q`, as the build cached them.
fn extra_atoms(live: &Live, q: usize) -> Vec<CellAtom> {
    live.views
        .get(&q)
        .map_or_else(Vec::new, |view| view.atoms.clone())
}

/// The stage of `arc`, a step or a delay into the next cell.
fn stage(run: &mut Run<'_, '_, '_>, live: &mut Live, arc: usize) -> Result<Stage, Halt> {
    let arc = live.arcs[arc].clone();
    let source = live.nodes[arc.from].clone();
    let target = live.nodes[arc.to].clone();
    let invariants = run.timed(run.timed.invariants(&source.state))?;
    let elapses = run.elapses(&source.state)?;
    let source_atoms = extra_atoms(live, source.q);
    let target_atoms = extra_atoms(live, target.q);
    Ok(match arc.via {
        Via::Step { step, part } => {
            let steps = run.steps_for(&source.state)?;
            let step = steps
                .get(step)
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
            let post = step.post().clone();
            Stage {
                invariants,
                elapses,
                target: run.timed(run.timed.invariants(&post))?,
                before: run.cell_zone(
                    &mut live.cells,
                    &source.state,
                    &source_atoms,
                    &source.cell,
                )?,
                after: None,
                auxiliary_carries: arc
                    .auxiliary_transform
                    .as_ref()
                    .map_or_else(Vec::new, |transform| transform.carries.clone()),
                auxiliary_resets: arc
                    .auxiliary_transform
                    .as_ref()
                    .map_or_else(Vec::new, |transform| transform.resets.clone()),
                piece: run.cell_zone(&mut live.cells, &post, &target_atoms, &target.cell)?,
                step: Some((step, part)),
                position: Vec::new(),
                position_resets: Vec::new(),
                silent: false,
                fixed: None,
            }
        }
        Via::Wait => Stage {
            invariants: invariants.clone(),
            elapses,
            target: invariants,
            before: None,
            after: None,
            auxiliary_carries: Vec::new(),
            auxiliary_resets: Vec::new(),
            piece: run.cell_zone(&mut live.cells, &target.state, &target_atoms, &target.cell)?,
            step: None,
            position: Vec::new(),
            position_resets: Vec::new(),
            silent: true,
            fixed: None,
        },
        Via::Diverge | Via::Stutter => return Err(ZoneCheckRefusal::Fault("stage-edge").into()),
    })
}

fn fairness(run: &Run<'_, '_, '_>, error: FairnessError) -> Halt {
    match error {
        FairnessError::Cancelled => run.stop(crate::zone_check::search::ZoneStop::Cancelled),
        _ => ZoneCheckRefusal::Fault("fairness-graph").into(),
    }
}

/// Which abstract fairness constraints a candidate-run check demands: the
/// first `through + 1` jointly, or one alone.
#[derive(Clone, Copy)]
enum FairScope {
    Through(usize),
    Only(usize),
}

impl FairScope {
    fn covers(self, constraint: usize) -> bool {
        match self {
            Self::Through(last) => constraint <= last,
            Self::Only(only) => constraint == only,
        }
    }
}

/// Whether every vertex of `members` has exactly one edge staying inside
/// `members`, so the component's only loop is its one cycle. Edges the
/// filter removed are counted, which only makes a component fail this test.
fn single_loop(graph: &Graph, members: &BTreeSet<Vertex>) -> bool {
    members.iter().all(|vertex| {
        graph
            .edges_from(*vertex)
            .iter()
            .filter(|edge| members.contains(&edge.target))
            .count()
            == 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-899 "Timed subjects": a fair candidate run on one greedy loop
    /// answers for a component only when the component is that one loop. A
    /// ring is one loop; a ring with a second loop through one of its
    /// vertices is a component whose other loop is not examined.
    #[trace("TC-2536")]
    #[test]
    fn only_a_component_that_is_one_loop_is_answered_by_its_greedy_loop() {
        let mut graph = Graph::new();
        let first = graph.add_vertex([]);
        let second = graph.add_vertex([]);
        graph
            .add_edge(first, second, EdgeLabel::Stutter)
            .expect("known vertices");
        graph
            .add_edge(second, first, EdgeLabel::Stutter)
            .expect("known vertices");
        let members: BTreeSet<Vertex> = [first, second].into_iter().collect();
        assert!(single_loop(&graph, &members));

        graph
            .add_edge(first, first, EdgeLabel::Stutter)
            .expect("known vertices");
        assert!(!single_loop(&graph, &members));
    }
}
