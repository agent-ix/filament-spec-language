// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-243's digital-clock route of EN-1: an authored temporal clause over a
//! closed timed subject decided by explicit-state search over the subject's
//! integer-clock reading (ADR-028 TA-1 to TA-3), the cross-check of EN-6's
//! zone verdict a caller selects with `options.method` `digital-clocks`
//! (FR-632). EN-6 never digitises (FR-239); this route is EN-1's.
//!
//! **The closed condition (TA-2).** Before any state is explored, every
//! clock constraint of the subject's guards and time invariants and every
//! timed interval of the claim must be non-strict; a strict one declines
//! the item naming it ([`DigitalRefusal::Strict`]). On closed subjects and
//! claims digitisation decides the dense-time verdict (Henzinger, Manna and
//! Pnueli), and every delay a counterexample takes is a valid dense delay.
//!
//! **The reading (TA-3).** Every constant of the model and of the claim is
//! scaled by their common denominator to whole units. A timed state is the
//! discrete state with each clock at a whole number of units, capped one
//! unit past the largest constant, where every constraint reads it as it
//! reads any larger value. A delay move advances every clock by one unit and
//! is admissible where the subject's time invariants hold after it and the
//! state is not urgent (FR-231 "Delay"); a discrete move is FR-231's step at
//! the state's clocks.
//!
//! **The product.** The clause's negation automaton read over positions
//! ([`fsl_eval::automaton::position_automaton`]), untimed or FR-242's timed
//! automaton, whose clocks are read on the same units and advance with each
//! delay, is multiplied with the reading as EN-6's liveness product
//! multiplies it with zones (FR-240): a vertex entered by a step, the
//! initial position or an idle-tail stutter is a position; a vertex entered
//! by a delay is divergent, since a cycle through one lets time advance by
//! at least a unit each repetition; at a quiescent state the behaviour may
//! stutter forever (FR-231's idle tail). A fair cycle through every
//! acceptance set, a position and a divergent vertex is a violation, found
//! by FR-126's fairness kernels; none, over every `over` instance, is a
//! proof with basis `exhaustive` (FR-235). Before any claim product, the
//! timed outcome layer ([`admitted_start`]) classifies a subject with no
//! initial state (`no-initial-state`) and one that admits no fair
//! time-divergent behaviour (`inconclusive` `no-admitted-behaviour`, FR-232
//! and FR-240 "Vacuity"). An undefined reading on a time-locked state
//! refutes nothing and the search goes on ([`admitted_refutation`]).
//!
//! **Counterexamples.** The canonical lasso is written with its exact
//! delays, whole units as rationals, its loop repeated until each clock it
//! never resets is past every constant at its entry, so FR-237's replay
//! closes it; an idle tail ends in the terminal stutter.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Cancel, Integer, Meter, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::automaton::{
    position_automaton, Atom, AutomatonError, ClockedEdge, Letter, PositionAutomaton, StateId,
};
use fsl_eval::fairness::{analyze, Analysis, EdgeLabel, FairnessError, Graph, Identity, Vertex};
use fsl_eval::model_subject::{ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{
    admitted_refutation, admitted_start, claim_start, ClockValuation, GuardedStep, ModelSimState,
    TimedState, TimedStop, TimedSystem, TimedVacuity,
};
use fsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use fsl_foundation::source::Span;
use fsl_replay::{
    CounterexampleKind, IncompleteCause, Lasso, ModelCheckLimit, ModelCheckOutcome, ModelStep,
    NonZeno, ReachedLimit, RunStats, TemporalCounterexample, TimedPoint, UndefinedEvaluation,
    UndefinedLocus, UndefinedSite,
};
use fsl_semantics::check::{
    CaptureSite, CheckedInterval, CheckedTemporalClause, ClockOp, FormulaNode,
};
use ix_trace_rs::implements;
use num_bigint::BigInt;
use num_integer::Integer as _;

use super::fair_walk::{fairness_constraints, FairModel};

/// The budgets of one digital-clock run (FR-625's limits).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DigitalLimits {
    /// The most product vertices the run stores.
    pub max_states: u64,
    /// The most product edges the run explores.
    pub max_transitions: u64,
    /// The most claim-automaton states the run materialises.
    pub max_automaton_states: u64,
}

/// One digital-clock item: an authored clause over a timed subject.
pub struct DigitalRequest<'r, 's, 'a, L> {
    /// The timed subject (FR-230).
    pub subject: &'r TimedSystem<'s, 'a>,
    /// The authored clause.
    pub clause: &'r CheckedTemporalClause,
    /// The clause's letters over the subject's model states.
    pub letters: &'r L,
    /// The run's budgets.
    pub limits: DigitalLimits,
}

/// EN-1's digital-clock outcome and its non-Zeno premise, if the result was
/// decided by the liveness SCC phase (FR-235).
#[derive(Clone, Debug)]
pub struct DigitalCheckOutcome {
    /// The ordinary EN-1 result.
    pub outcome: ModelCheckOutcome,
    /// The liveness premise accompanying a settled result.
    pub non_zeno: Option<NonZeno>,
}

/// What a strict element the closed condition refuses is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Strict {
    /// A strict clock constraint of a guard or a time invariant, by its
    /// source span.
    Constraint(Span),
    /// A strict (open or half-open) timed interval of the claim, spelled in
    /// the clause's time unit.
    Interval(String),
}

/// Why the digital-clock route decided nothing and settles no outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DigitalRefusal {
    /// TA-2: the subject or the claim is not closed; no state is explored.
    Strict(Strict),
    /// The clause has no claim automaton the route reads: a punctual
    /// interval started at many positions, or more automaton clocks than the
    /// construction builds (as EN-6 reads them, `UnsupportedForm`).
    UnsupportedForm,
    /// A `holds` atom stopped with no reading.
    Atom(String),
    /// A broken invariant of the route.
    Fault(&'static str),
}

/// Why a run stopped early: an outcome to return, or a refusal.
enum Halt {
    Outcome(Box<ModelCheckOutcome>),
    Refusal(DigitalRefusal),
}

impl From<DigitalRefusal> for Halt {
    fn from(refusal: DigitalRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

/// FR-243's digital-clock route over `request`.
///
/// # Errors
///
/// [`DigitalRefusal::Strict`] before any search when the subject or the
/// claim is not closed; the other [`DigitalRefusal`]s as they state.
pub fn check_digital<L: ModelLetters>(
    request: &DigitalRequest<'_, '_, '_, L>,
    cancel: &Cancel,
) -> Result<DigitalCheckOutcome, DigitalRefusal> {
    if let Some(strict) = strict_element(request.subject, request.clause) {
        return Err(DigitalRefusal::Strict(strict));
    }
    let mut automaton = position_automaton(request.clause, request.limits.max_automaton_states)
        .map_err(|error| match error {
            AutomatonError::StateLimit { .. } => DigitalRefusal::Fault("automaton-initial"),
            _ => DigitalRefusal::UnsupportedForm,
        })?;
    let mut run = Digital::new(request, &automaton, cancel)?;
    let outcome = match run.decide(&mut automaton) {
        Ok(outcome) => outcome,
        Err(Halt::Outcome(outcome)) => *outcome,
        Err(Halt::Refusal(refusal)) => return Err(refusal),
    };
    let non_zeno = match &outcome {
        ModelCheckOutcome::HoldsReduced { .. } => run.non_zeno,
        ModelCheckOutcome::Violated { counterexample, .. }
            if counterexample.lasso.is_some()
                && matches!(&counterexample.kind, CounterexampleKind::Formula) =>
        {
            Some(NonZeno::Checked)
        }
        ModelCheckOutcome::Undecided {
            cause: fsl_replay::UndecidedCause::NoAdmittedBehaviour,
            ..
        } => Some(NonZeno::Refuted),
        ModelCheckOutcome::Undecided { .. }
        | ModelCheckOutcome::Violated { .. }
        | ModelCheckOutcome::Holds { .. }
        | ModelCheckOutcome::Reduction { .. }
        | ModelCheckOutcome::Memory { .. }
        | ModelCheckOutcome::RecoveryView { .. }
        | ModelCheckOutcome::RefutedIgnoringConstraints { .. }
        | ModelCheckOutcome::BoundReached { .. }
        | ModelCheckOutcome::Stopped { .. } => None,
    };
    Ok(DigitalCheckOutcome { outcome, non_zeno })
}

/// TA-2: the first strict clock constraint of the subject, in declaration
/// order, then the first strict timed interval of the claim.
fn strict_element(timed: &TimedSystem<'_, '_>, clause: &CheckedTemporalClause) -> Option<Strict> {
    let constraint = crate::exact_probabilistic::model_constraints(timed)
        .into_iter()
        .find(|constraint| matches!(constraint.op(), ClockOp::Less | ClockOp::Greater));
    if let Some(constraint) = constraint {
        return Some(Strict::Constraint(constraint.span()));
    }
    clause.formula().nodes().iter().find_map(|node| match node {
        FormulaNode::Unary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        }
        | FormulaNode::Binary {
            interval: Some(CheckedInterval::Timed(interval)),
            ..
        } if !interval.lower_closed() || (interval.is_bounded() && !interval.upper_closed()) => {
            let upper = interval
                .upper()
                .map_or_else(|| "*".to_owned(), rational_text);
            Some(Strict::Interval(format!(
                "{}{}, {}{}",
                if interval.lower_closed() { '[' } else { '(' },
                rational_text(interval.lower()),
                upper,
                if interval.upper_closed() { ']' } else { ')' },
            )))
        }
        _ => None,
    })
}

fn rational_text(value: &Rational) -> String {
    if value.is_integer() {
        value.numerator().as_big().to_string()
    } else {
        format!(
            "{}/{}",
            value.numerator().as_big(),
            value.denominator().as_big()
        )
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// One vertex of the digital product.
#[derive(Clone, Debug)]
struct DVertex {
    /// The timed state, each clock capped.
    state: TimedState,
    /// The claim automaton's state.
    q: StateId,
    /// The automaton's clocks, each capped.
    clocks: ClockValuation,
    /// Entered by a position: a step, the initial position or a stutter.
    positioned: bool,
    /// Entered by a delay of one unit.
    divergent: bool,
    /// In the idle tail: the behaviour takes no further step.
    tail: bool,
    /// The initial state's index, for an initial vertex.
    initial: Option<usize>,
}

/// How an edge of the product is taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Via {
    /// The step at this index of the source's steps.
    Step(usize),
    /// A delay of one unit.
    Delay,
    /// An idle-tail stutter position.
    Stutter,
}

#[derive(Clone, Copy, Debug)]
struct DEdge {
    from: usize,
    to: usize,
    via: Via,
}

/// A position's letter, or the first condition undefined there.
#[derive(Clone, Debug)]
enum Reading {
    Defined(Letter),
    Undefined(UndefinedLocus, UndefinedCause),
}

/// One instance's product.
#[derive(Default)]
struct Product {
    vertices: Vec<DVertex>,
    edges: Vec<DEdge>,
    out: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
    index: BTreeMap<Vec<u8>, usize>,
}

/// An undefined position the product reached: the vertex it is read at
/// (or the initial state's index) and what is undefined.
struct UndefinedAt {
    path: Vec<usize>,
    initial: usize,
    locus: UndefinedLocus,
    cause: UndefinedCause,
    instance: ModelInstance,
}

struct Digital<'q, 'r, 's, 'a, L> {
    request: &'q DigitalRequest<'r, 's, 'a, L>,
    cancel: &'q Cancel,
    /// One unit, `1 / scale`.
    unit: Rational,
    /// The model clocks' cap: one unit past the largest constant.
    cap: Rational,
    /// The automaton clocks' cap.
    automaton_cap: Rational,
    /// The largest constant of the model and the claim, which every clock
    /// a loop never resets passes at its entry.
    ceiling: Rational,
    stats: RunStats,
    non_zeno: Option<NonZeno>,
    steps: BTreeMap<Vec<u8>, Vec<(GuardedStep, TimedState)>>,
    /// Each searched state's [`Digital::diverges`].
    divergence: BTreeMap<Vec<u8>, bool>,
}

impl<'q, 'r, 's, 'a, L: ModelLetters> Digital<'q, 'r, 's, 'a, L> {
    fn new(
        request: &'q DigitalRequest<'r, 's, 'a, L>,
        automaton: &PositionAutomaton,
        cancel: &'q Cancel,
    ) -> Result<Self, DigitalRefusal> {
        let timed = request.subject;
        let mut model_constants: Vec<Rational> =
            crate::exact_probabilistic::model_constraints(timed)
                .into_iter()
                .map(|constraint| constraint.bound().clone())
                .collect();
        for resets in timed.model().resets() {
            for reset in resets.resets() {
                model_constants.push(reset.value().clone());
            }
        }
        let claim_constants = automaton.all_constants();
        let scale = model_constants
            .iter()
            .chain(&claim_constants)
            .fold(BigInt::from(1), |scale, value| {
                scale.lcm(value.denominator().as_big())
            });
        let unit = Rational::new(Integer::one(), Integer::from_big(scale))
            .map_err(|_| DigitalRefusal::Fault("digital-scale"))?;
        let most = |values: &[Rational]| values.iter().cloned().max().unwrap_or_else(zero);
        let cap = most(&model_constants).add(&unit);
        let automaton_cap = most(&claim_constants).add(&unit);
        let ceiling = most(&model_constants).max(most(&claim_constants));
        Ok(Self {
            request,
            cancel,
            unit,
            cap,
            automaton_cap,
            ceiling,
            stats: RunStats::default(),
            non_zeno: None,
            steps: BTreeMap::new(),
            divergence: BTreeMap::new(),
        })
    }

    fn timed(&self) -> &'r TimedSystem<'s, 'a> {
        self.request.subject
    }

    fn poll(&self) -> Result<(), Halt> {
        if self.cancel.poll() {
            return Err(self.cancelled());
        }
        Ok(())
    }

    fn cancelled(&self) -> Halt {
        let source = self
            .cancel
            .cause()
            .unwrap_or(filament_exact::CancelCause::Requested);
        self.stopped(IncompleteCause::Cancelled { source }, None)
    }

    fn stopped(&self, cause: IncompleteCause, limit: Option<ReachedLimit>) -> Halt {
        Halt::Outcome(Box::new(ModelCheckOutcome::Stopped {
            cause,
            limit,
            stats: self.stats,
        }))
    }

    fn limit(&self, limit: ModelCheckLimit, value: u64) -> Halt {
        self.stopped(
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    }

    fn timed_stop<T>(&self, result: Result<T, TimedStop>) -> Result<T, Halt> {
        result.map_err(|stop| match stop {
            TimedStop::MaxClocks { max_clocks } => {
                self.limit(ModelCheckLimit::MaxClocks, max_clocks)
            }
            _ => Halt::Outcome(Box::new(ModelCheckOutcome::Undecided {
                cause: fsl_replay::UndecidedCause::UndecidedSuccessor,
                stats: self.stats,
            })),
        })
    }

    fn automaton_stop<T>(&self, result: Result<T, AutomatonError>) -> Result<T, Halt> {
        result.map_err(|error| match error {
            AutomatonError::StateLimit { limit } => {
                self.limit(ModelCheckLimit::MaxAutomatonStates, limit)
            }
            _ => DigitalRefusal::UnsupportedForm.into(),
        })
    }

    fn capped(values: &ClockValuation, cap: &Rational) -> ClockValuation {
        ClockValuation::new(
            values
                .values()
                .iter()
                .map(|value| {
                    if value > cap {
                        cap.clone()
                    } else {
                        value.clone()
                    }
                })
                .collect(),
        )
    }

    fn capped_state(&self, state: &TimedState) -> TimedState {
        TimedState::new(
            state.discrete().clone(),
            Self::capped(state.clocks(), &self.cap),
        )
    }

    /// The discrete state's canonical bytes.
    fn discrete_key(&self, state: &ModelSimState) -> Result<Vec<u8>, Halt> {
        self.timed()
            .system()
            .state_key(state, fsl_foundation::IdentityLimits::default())
            .map(|(bytes, _)| bytes)
            .map_err(|_| DigitalRefusal::Fault("state-key").into())
    }

    /// The steps from the capped timed state `state`, computed once.
    fn steps_of(&mut self, state: &TimedState) -> Result<Vec<(GuardedStep, TimedState)>, Halt> {
        let mut key = self.discrete_key(state.discrete())?;
        push_clocks(&mut key, state.clocks());
        if let Some(known) = self.steps.get(&key) {
            return Ok(known.clone());
        }
        let found = self.timed_stop(self.timed().steps(state))?;
        self.steps.insert(key, found.clone());
        Ok(found)
    }

    /// The vertex a one-unit delay from `state` reaches, capped, or `None`
    /// where the delay is not admissible.
    fn delayed(&self, state: &TimedState) -> Result<Option<TimedState>, Halt> {
        let delayed = self.timed_stop(self.timed().delay(state, &self.unit))?;
        Ok(delayed.map(|state| self.capped_state(&state)))
    }

    fn quiescent(&self, state: &TimedState) -> Result<bool, Halt> {
        let discrete = state.discrete();
        Ok(self.timed_stop(self.timed().is_quiescent(discrete))?
            && !self.timed_stop(self.timed().is_urgent(discrete))?)
    }

    /// The letter at `state` for `instance`, `initial` at position 0.
    fn reading(
        &self,
        instance: &ModelInstance,
        automaton: &PositionAutomaton,
        universal: bool,
        state: &ModelSimState,
        initial: bool,
    ) -> Result<Reading, Halt> {
        if universal {
            return Ok(Reading::Defined(Letter::default()));
        }
        let letters = self.request.letters;
        let limits = self.timed().system().meter_limits();
        let atom_stop = |stop| DigitalRefusal::Atom(format!("{stop:?}"));
        let mut letter = Letter::default();
        let when = automaton.when();
        if let Some(when) = when {
            let mut meter = Meter::new(limits);
            match letters
                .activation(instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                AtomReading::True => letter = letter.with(when),
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    return Ok(Reading::Undefined(UndefinedLocus::Activation, cause));
                }
            }
        }
        let capturing = match self.request.clause.capture_site() {
            CaptureSite::Never => false,
            CaptureSite::Origin => initial,
            CaptureSite::Activations => when.is_none_or(|when| letter.holds(when)),
        };
        if capturing {
            let mut meter = Meter::new(limits);
            if let CaptureReading::Undefined { capture, cause } = letters
                .captures(instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                return Ok(Reading::Undefined(UndefinedLocus::Capture(capture), cause));
            }
        }
        for (index, atom) in automaton.atoms().iter().enumerate() {
            let mut meter = Meter::new(limits);
            match letters
                .atom(*atom, instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                AtomReading::True => {
                    let atom = Atom::new(index).map_err(|_| DigitalRefusal::Fault("atom-index"))?;
                    letter = letter.with(atom);
                }
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    return Ok(Reading::Undefined(UndefinedLocus::Atom(*atom), cause));
                }
            }
        }
        Ok(Reading::Defined(letter))
    }

    /// The automaton transitions a position reads `letter` by from `q` at
    /// the automaton clocks `clocks`: each enabled one's target and clocks
    /// after its resets.
    fn reads(
        &self,
        automaton: &mut PositionAutomaton,
        universal: bool,
        q: StateId,
        letter: Letter,
        clocks: &ClockValuation,
    ) -> Result<Vec<(StateId, ClockValuation)>, Halt> {
        let edges = if universal {
            vec![ClockedEdge {
                target: q,
                guard: Vec::new(),
                resets: Vec::new(),
            }]
        } else {
            self.automaton_stop(automaton.successors(q, letter))?
        };
        Ok(edges
            .into_iter()
            .filter(|edge| edge.guard.iter().all(|constraint| constraint.holds(clocks)))
            .map(|edge| {
                let mut values = clocks.values().to_vec();
                for &clock in &edge.resets {
                    if let Some(value) = values.get_mut(clock) {
                        *value = zero();
                    }
                }
                (edge.target, ClockValuation::new(values))
            })
            .collect())
    }

    /// Store `vertex` unless an equal one is stored; the new vertex's index,
    /// or `None` with the edge recorded to the equal one.
    fn add_vertex(
        &mut self,
        product: &mut Product,
        vertex: DVertex,
        from: Option<(usize, Via)>,
    ) -> Result<Option<usize>, Halt> {
        let mut key = self.discrete_key(vertex.state.discrete())?;
        push_clocks(&mut key, vertex.state.clocks());
        key.extend_from_slice(
            &u64::try_from(vertex.q.index())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        push_clocks(&mut key, &vertex.clocks);
        key.push(u8::from(vertex.positioned));
        key.push(u8::from(vertex.divergent));
        key.push(u8::from(vertex.tail));
        key.push(u8::from(vertex.initial.is_some()));
        if let Some(&held) = product.index.get(&key) {
            if let Some((from, via)) = from {
                self.add_edge(product, from, held, via)?;
            }
            return Ok(None);
        }
        let limits = self.request.limits;
        if self.stats.product_states >= limits.max_states {
            return Err(self.limit(ModelCheckLimit::MaxStates, limits.max_states));
        }
        self.stats.product_states += 1;
        let index = product.vertices.len();
        product.vertices.push(vertex);
        product.out.push(Vec::new());
        product.parent.push(None);
        product.index.insert(key, index);
        if let Some((from, via)) = from {
            self.add_edge(product, from, index, via)?;
            product.parent[index] = Some(product.edges.len() - 1);
        }
        Ok(Some(index))
    }

    fn add_edge(
        &mut self,
        product: &mut Product,
        from: usize,
        to: usize,
        via: Via,
    ) -> Result<(), Halt> {
        let limits = self.request.limits;
        if self.stats.product_edges >= limits.max_transitions {
            return Err(self.limit(ModelCheckLimit::MaxTransitions, limits.max_transitions));
        }
        self.stats.product_edges += 1;
        if via == Via::Stutter {
            self.stats.stutter_edges += 1;
        }
        product.out[from].push(product.edges.len());
        product.edges.push(DEdge { from, to, via });
        Ok(())
    }

    /// The edges from an initial vertex to `vertex`.
    fn path_to(product: &Product, vertex: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut at = vertex;
        while let Some(edge) = product.parent[at] {
            path.push(edge);
            at = product.edges[edge].from;
        }
        path.reverse();
        path
    }

    /// Build one instance's product breadth first: the first undefined
    /// position it reaches on a state time can diverge from, if any. An
    /// undefined position on a time-locked state is left out of the product
    /// and the rest is built.
    fn build(
        &mut self,
        product: &mut Product,
        automaton: &mut PositionAutomaton,
        universal: bool,
        instance: &ModelInstance,
    ) -> Result<Option<UndefinedAt>, Halt> {
        let start = self.automaton_stop(automaton.initial())?;
        let zeros = ClockValuation::new(vec![zero(); automaton.clock_count()]);
        let mut queue = VecDeque::new();
        for (index, initial) in self.timed().initial().iter().enumerate() {
            self.poll()?;
            let state = self.capped_state(initial);
            match self.reading(instance, automaton, universal, state.discrete(), true)? {
                Reading::Undefined(locus, cause) => {
                    let diverges = self.diverges(&state)?;
                    let found = admitted_refutation(diverges, || {
                        Ok::<_, Halt>(UndefinedAt {
                            path: Vec::new(),
                            initial: index,
                            locus,
                            cause,
                            instance: instance.clone(),
                        })
                    })?;
                    if found.is_some() {
                        return Ok(found);
                    }
                }
                Reading::Defined(letter) => {
                    for (q, clocks) in self.reads(automaton, universal, start, letter, &zeros)? {
                        let vertex = DVertex {
                            state: state.clone(),
                            q,
                            clocks,
                            positioned: true,
                            divergent: false,
                            tail: false,
                            initial: Some(index),
                        };
                        queue.extend(self.add_vertex(product, vertex, None)?);
                    }
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            match self.expand(product, automaton, universal, instance, at)? {
                Ok(created) => queue.extend(created),
                Err(undefined) => return Ok(Some(undefined)),
            }
        }
        // The idle tails after every stepping vertex.
        for at in 0..product.vertices.len() {
            self.poll()?;
            let vertex = product.vertices[at].clone();
            if !vertex.tail && self.quiescent(&vertex.state)? {
                match self.stutter(product, automaton, universal, instance, at)? {
                    Ok(created) => queue.extend(created),
                    Err(undefined) => return Ok(Some(undefined)),
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            match self.expand(product, automaton, universal, instance, at)? {
                Ok(created) => queue.extend(created),
                Err(undefined) => return Ok(Some(undefined)),
            }
        }
        Ok(None)
    }

    /// The vertices `at`'s edges create, or the undefined position a step
    /// from it reaches.
    fn expand(
        &mut self,
        product: &mut Product,
        automaton: &mut PositionAutomaton,
        universal: bool,
        instance: &ModelInstance,
        at: usize,
    ) -> Result<Result<Vec<usize>, UndefinedAt>, Halt> {
        let vertex = product.vertices[at].clone();
        let mut created = Vec::new();
        if vertex.tail {
            match self.stutter(product, automaton, universal, instance, at)? {
                Ok(next) => created.extend(next),
                Err(undefined) => return Ok(Err(undefined)),
            }
        } else {
            for (index, (_, post)) in self.steps_of(&vertex.state)?.into_iter().enumerate() {
                self.poll()?;
                let post = self.capped_state(&post);
                match self.reading(instance, automaton, universal, post.discrete(), false)? {
                    Reading::Undefined(locus, cause) => {
                        let diverges = self.diverges(&post)?;
                        let found = admitted_refutation(diverges, || {
                            let mut path = Self::path_to(product, at);
                            let initial = root(product, &path, at)?;
                            // The step to the undefined position closes the path.
                            product.edges.push(DEdge {
                                from: at,
                                to: at,
                                via: Via::Step(index),
                            });
                            path.push(product.edges.len() - 1);
                            Ok::<_, Halt>(UndefinedAt {
                                path,
                                initial,
                                locus,
                                cause,
                                instance: instance.clone(),
                            })
                        })?;
                        if let Some(found) = found {
                            return Ok(Err(found));
                        }
                    }
                    Reading::Defined(letter) => {
                        for (q, clocks) in
                            self.reads(automaton, universal, vertex.q, letter, &vertex.clocks)?
                        {
                            let next = DVertex {
                                state: post.clone(),
                                q,
                                clocks,
                                positioned: true,
                                divergent: false,
                                tail: false,
                                initial: None,
                            };
                            created.extend(self.add_vertex(
                                product,
                                next,
                                Some((at, Via::Step(index))),
                            )?);
                        }
                    }
                }
            }
        }
        if let Some(later) = self.delayed(&vertex.state)? {
            let clocks = Self::capped(&vertex.clocks.delayed(&self.unit), &self.automaton_cap);
            let next = DVertex {
                state: later,
                clocks,
                positioned: false,
                divergent: true,
                initial: None,
                ..vertex
            };
            created.extend(self.add_vertex(product, next, Some((at, Via::Delay)))?);
        }
        Ok(Ok(created))
    }

    /// The idle-tail stutters from `at`: the state's letter read again.
    fn stutter(
        &mut self,
        product: &mut Product,
        automaton: &mut PositionAutomaton,
        universal: bool,
        instance: &ModelInstance,
        at: usize,
    ) -> Result<Result<Vec<usize>, UndefinedAt>, Halt> {
        let vertex = product.vertices[at].clone();
        let letter = match self.reading(
            instance,
            automaton,
            universal,
            vertex.state.discrete(),
            false,
        )? {
            Reading::Defined(letter) => letter,
            Reading::Undefined(locus, cause) => {
                let mut path = Self::path_to(product, at);
                let initial = root(product, &path, at)?;
                let edge = product.edges.len();
                self.add_edge(product, at, at, Via::Stutter)?;
                path.push(edge);
                return Ok(Err(UndefinedAt {
                    path,
                    initial,
                    locus,
                    cause,
                    instance: instance.clone(),
                }));
            }
        };
        let mut created = Vec::new();
        for (q, clocks) in self.reads(automaton, universal, vertex.q, letter, &vertex.clocks)? {
            let next = DVertex {
                q,
                clocks,
                positioned: true,
                divergent: false,
                tail: true,
                initial: None,
                ..vertex.clone()
            };
            created.extend(self.add_vertex(product, next, Some((at, Via::Stutter)))?);
        }
        Ok(Ok(created))
    }

    /// The product as the fairness kernels read it.
    fn fair_model(
        &mut self,
        product: &Product,
        automaton: &PositionAutomaton,
        universal: bool,
        include_divergence: bool,
    ) -> Result<FairModel, Halt> {
        let mut transitions: Vec<ModelTransition> = Vec::new();
        let mut graph = Graph::new();
        let mut handles = Vec::with_capacity(product.vertices.len());
        let mut steps = Vec::with_capacity(product.vertices.len());
        for vertex in &product.vertices {
            let found = self.steps_of(&vertex.state)?;
            let mut enabled = Vec::with_capacity(found.len());
            for (step, _) in &found {
                enabled.push(number(&mut transitions, step.transition()));
            }
            handles.push(graph.add_vertex(enabled));
            steps.push(found);
        }
        let fault = |_: FairnessError| Halt::from(DigitalRefusal::Fault("fairness-graph"));
        for &index in product.out.iter().flatten() {
            let edge = product.edges[index];
            let label = match edge.via {
                Via::Step(index) => {
                    let (step, _) = steps[edge.from]
                        .get(index)
                        .ok_or(DigitalRefusal::Fault("edge-step"))?;
                    EdgeLabel::Transition(number(&mut transitions, step.transition()))
                }
                Via::Delay | Via::Stutter => EdgeLabel::Stutter,
            };
            graph
                .add_edge(handles[edge.from], handles[edge.to], label)
                .map_err(fault)?;
        }
        let sets = if universal {
            0
        } else {
            automaton.acceptance_set_count()
        };
        for set in 0..sets {
            let members: Vec<Vertex> = handles
                .iter()
                .zip(&product.vertices)
                .filter(|(_, vertex)| automaton.is_accepting_in(set, vertex.q))
                .map(|(handle, _)| *handle)
                .collect();
            graph.add_acceptance_set(members).map_err(fault)?;
        }
        if include_divergence {
            let members: Vec<Vertex> = handles
                .iter()
                .zip(&product.vertices)
                .filter(|(_, vertex)| vertex.divergent || vertex.tail)
                .map(|(handle, _)| *handle)
                .collect();
            graph.add_acceptance_set(members).map_err(fault)?;
        }
        let positioned: Vec<Vertex> = handles
            .iter()
            .zip(&product.vertices)
            .filter(|(_, vertex)| vertex.positioned)
            .map(|(handle, _)| *handle)
            .collect();
        graph.add_acceptance_set(positioned).map_err(fault)?;
        let constraints = fairness_constraints(
            self.timed().system(),
            self.request.clause.fairness(),
            &transitions,
            |transition| {
                transitions
                    .iter()
                    .position(|held| held == transition)
                    .map(|index| Identity::new(u64::try_from(index).unwrap_or(u64::MAX)))
                    .ok_or_else(|| {
                        super::ModelCheckRefusal::Fault(
                            filament_semantic_value::catalog::InternalFault::new(
                                "digital-clocks",
                                "a-constrained-transition-is-numbered",
                            ),
                        )
                    })
            },
        )
        .map_err(|_| DigitalRefusal::Fault("fairness-constraints"))?;
        Ok(FairModel { graph, constraints })
    }

    /// The first fair accepting component's lasso edges: its entry and the
    /// loop's product edges.
    fn fair_cycle(
        &mut self,
        product: &Product,
        model: &FairModel,
    ) -> Result<Option<(usize, Vec<usize>)>, Halt> {
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses =
            analyze(&model.graph, &all, &model.constraints, self.cancel).map_err(|error| {
                match error {
                    FairnessError::Cancelled => self.cancelled(),
                    _ => DigitalRefusal::Fault("fairness-analysis").into(),
                }
            })?;
        let Some(component) = Analysis::fair_in(&analyses).next() else {
            return Ok(None);
        };
        let Some(&entry) = component.vertices().first() else {
            return Err(DigitalRefusal::Fault("empty-component").into());
        };
        let members: BTreeSet<Vertex> = component.vertices().iter().copied().collect();
        let walk = model
            .greedy_loop(&members, component.removed(), entry)
            .map_err(|_| DigitalRefusal::Fault("component-loop"))?;
        let mut cycle = Vec::with_capacity(walk.len());
        for (from, edge) in &walk {
            let position = model
                .graph
                .edges_from(*from)
                .iter()
                .position(|held| held == edge)
                .ok_or(DigitalRefusal::Fault("walk-edge"))?;
            let index = graph_edges(product, from.index())
                .get(position)
                .copied()
                .ok_or(DigitalRefusal::Fault("walk-edge"))?;
            cycle.push(index);
        }
        Ok(Some((entry.index(), cycle)))
    }

    /// The decision over every instance (FR-240, FR-126). A would-be proof
    /// is classified first, once, by the timed outcome layer: a subject with
    /// no admitted behaviour has no counterexample either, so no claim
    /// product is built over it.
    #[implements("FR-125-AC-7", "FR-232-AC-3")]
    fn decide(&mut self, automaton: &mut PositionAutomaton) -> Result<ModelCheckOutcome, Halt> {
        let system = self.timed().system();
        let initial_states = self.timed().initial().len();
        let over = system.claim_instances(&self.request.clause.over().1);
        let instances = match claim_start(initial_states, over)
            .map_err(|_| DigitalRefusal::Fault("instances"))?
        {
            Ok(instances) => instances,
            Err(vacuity) => return Ok(self.vacuous(vacuity)),
        };
        let start = admitted_start(initial_states, || {
            let admitted = self.admits_behaviour(automaton, instances.first())?;
            Ok::<_, Halt>(admitted.then_some(()))
        })?;
        if let Err(vacuity) = start {
            return Ok(self.vacuous(vacuity));
        }
        for instance in instances.as_slice() {
            let mut product = Product::default();
            let built = self.build(&mut product, automaton, false, instance)?;
            let automaton_states: BTreeSet<usize> = product
                .vertices
                .iter()
                .map(|vertex| vertex.q.index())
                .collect();
            self.stats.automaton_states = self
                .stats
                .automaton_states
                .max(u64::try_from(automaton_states.len()).unwrap_or(u64::MAX));
            if let Some(undefined) = built {
                let found = self.undefined(&product, undefined)?;
                return Ok(self.violated(found));
            }
            let model = self.fair_model(&product, automaton, false, true)?;
            if let Some((entry, cycle)) = self.fair_cycle(&product, &model)? {
                let found = self.lasso(&product, instance, entry, &cycle)?;
                self.non_zeno = Some(NonZeno::Checked);
                return Ok(self.violated(found));
            }
            let without_divergence = self.fair_model(&product, automaton, false, false)?;
            if self.fair_cycle(&product, &without_divergence)?.is_some() {
                self.non_zeno = Some(NonZeno::Assumed);
            }
        }
        if self.non_zeno.is_none() {
            self.non_zeno = Some(NonZeno::NotRequired);
        }
        Ok(ModelCheckOutcome::HoldsReduced {
            reductions: Vec::new(),
            stats: self.stats,
        })
    }

    /// FR-240 and FR-232 "Vacuity", this route's evidence: whether the
    /// subject read by no claim has a fair time-divergent cycle or reaches a
    /// quiescent state, with or without a declared fairness. The product
    /// depends on no instance's letters.
    fn admits_behaviour(
        &mut self,
        automaton: &mut PositionAutomaton,
        instance: &ModelInstance,
    ) -> Result<bool, Halt> {
        let mut product = Product::default();
        self.build(&mut product, automaton, true, instance)?;
        if product.vertices.iter().any(|vertex| vertex.tail) {
            return Ok(true);
        }
        let model = self.fair_model(&product, automaton, true, true)?;
        Ok(self.fair_cycle(&product, &model)?.is_some())
    }

    fn vacuous(&self, vacuity: TimedVacuity) -> ModelCheckOutcome {
        ModelCheckOutcome::Undecided {
            cause: vacuity.into(),
            stats: self.stats,
        }
    }

    fn violated(&self, counterexample: TemporalCounterexample) -> ModelCheckOutcome {
        ModelCheckOutcome::Violated {
            counterexample: Box::new(counterexample),
            stats: self.stats,
        }
    }

    /// The concrete steps of `edges` from the initial state `initial`, with
    /// exact clocks: the steps, the reached state and its time stamp.
    fn concrete(
        &mut self,
        product: &Product,
        initial: usize,
        edges: &[usize],
    ) -> Result<(Vec<ModelStep>, TimedState, Rational), Halt> {
        let mut state = self
            .timed()
            .initial()
            .get(initial)
            .cloned()
            .ok_or(DigitalRefusal::Fault("initial-state"))?;
        let mut delay = zero();
        let mut stamp = zero();
        let mut steps = Vec::new();
        for &edge in edges {
            let edge = product.edges[edge];
            match edge.via {
                Via::Delay => {
                    delay = delay.add(&self.unit);
                    stamp = stamp.add(&self.unit);
                    state = TimedState::new(
                        state.discrete().clone(),
                        state.clocks().delayed(&self.unit),
                    );
                }
                Via::Step(index) => {
                    let source = &product.vertices[edge.from].state;
                    let (step, _) = self
                        .steps_of(source)?
                        .get(index)
                        .cloned()
                        .ok_or(DigitalRefusal::Fault("edge-step"))?;
                    state = TimedState::new(step.post().clone(), step.reset(state.clocks()));
                    let post_state = self
                        .timed()
                        .state_key(&state)
                        .map_err(|_| DigitalRefusal::Fault("state-key"))?;
                    steps.push(ModelStep {
                        transition: step.transition().clone(),
                        post_state,
                        delay: Some(std::mem::replace(&mut delay, zero())),
                    });
                }
                Via::Stutter => {}
            }
        }
        Ok((steps, state, stamp))
    }

    /// FR-241 over digital clocks: the lasso of the stem to `entry` and the
    /// loop `cycle`.
    fn lasso(
        &mut self,
        product: &Product,
        instance: &ModelInstance,
        entry: usize,
        cycle: &[usize],
    ) -> Result<TemporalCounterexample, Halt> {
        let stem = Self::path_to(product, entry);
        let initial = root(product, &stem, entry)?;
        let counterexample = |prefix, lasso| TemporalCounterexample {
            initial_state: initial,
            prefix,
            over_binding: Some(instance.clone()),
            fairness: self.request.clause.fairness().to_vec(),
            kind: CounterexampleKind::Formula,
            lasso: Some(lasso),
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
            closure_stutters: Vec::new(),
        };
        if product.vertices[entry].tail {
            // The stem to the quiescent state, then its stutter forever.
            let tail = stem
                .iter()
                .position(|&edge| product.edges[edge].via == Via::Stutter)
                .unwrap_or(stem.len());
            let (prefix, _, _) = self.concrete(product, initial, &stem[..tail])?;
            let entry = prefix.len();
            return Ok(counterexample(
                prefix,
                Lasso {
                    entry,
                    terminal_stutter: true,
                    terminal_stutter_delay: None,
                },
            ));
        }
        // Start the loop just after a step, so its entry is a position.
        let first = cycle
            .iter()
            .position(|&edge| matches!(product.edges[edge].via, Via::Step(_)))
            .ok_or(DigitalRefusal::Fault("loop-position"))?;
        let mut stem = stem;
        stem.extend_from_slice(&cycle[..=first]);
        let rotated: Vec<usize> = cycle[first + 1..]
            .iter()
            .chain(&cycle[..=first])
            .copied()
            .collect();
        let units = rotated
            .iter()
            .filter(|&&edge| product.edges[edge].via == Via::Delay)
            .count();
        let total = self.unit.mul(&Rational::from_integer(Integer::from(units)));
        // Copies of the loop until each clock it never resets is past the
        // ceiling at the last copy's entry.
        let rounds = self
            .ceiling
            .div(&total)
            .ok_or(DigitalRefusal::Fault("loop-delay"))?;
        let rounds = rounds.numerator().as_big() / rounds.denominator().as_big();
        let copies = usize::try_from(rounds)
            .unwrap_or(usize::MAX)
            .saturating_add(2);
        let mut edges = stem;
        for _ in 1..copies {
            edges.extend_from_slice(&rotated);
        }
        let (head, _, _) = self.concrete(product, initial, &edges)?;
        let entry = head.len();
        edges.extend_from_slice(&rotated);
        let (prefix, _, _) = self.concrete(product, initial, &edges)?;
        Ok(counterexample(
            prefix,
            Lasso {
                entry,
                terminal_stutter: false,
                terminal_stutter_delay: None,
            },
        ))
    }

    /// The counterexample of an undefined position the product reached
    /// (FR-241).
    fn undefined(
        &mut self,
        product: &Product,
        undefined: UndefinedAt,
    ) -> Result<TemporalCounterexample, Halt> {
        let UndefinedAt {
            path,
            initial,
            locus,
            cause,
            instance,
        } = undefined;
        let (prefix, reached, stamp) = self.concrete(product, initial, &path)?;
        let (discrete_key, _) = self
            .timed()
            .system()
            .state_key(
                reached.discrete(),
                fsl_foundation::IdentityLimits::default(),
            )
            .map_err(|_| DigitalRefusal::Fault("state-key"))?;
        let clocks: Vec<Rational> = self
            .timed()
            .places()
            .iter()
            .zip(reached.clocks().values())
            .filter(|(place, _)| self.timed().live(place, reached.discrete()))
            .map(|(_, value)| value.clone())
            .collect();
        Ok(TemporalCounterexample {
            initial_state: initial,
            kind: CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                site: UndefinedSite {
                    position: u64::try_from(prefix.len()).unwrap_or(u64::MAX),
                    state_key: fsl_eval::simulation::StateKey::from_canonical_bytes(discrete_key),
                    locus,
                    transition: None,
                    time: Some(Box::new(TimedPoint { stamp, clocks })),
                },
                cause,
            }),
            prefix,
            over_binding: Some(instance),
            fairness: self.request.clause.fairness().to_vec(),
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
            closure_stutters: Vec::new(),
        })
    }

    /// Whether `start` has a time-divergent continuation: it reaches a
    /// quiescent state, or a cycle through a delay. This route's evidence
    /// for the time-lock rule, searched once per state.
    fn diverges(&mut self, start: &TimedState) -> Result<bool, Halt> {
        let mut from = self.discrete_key(start.discrete())?;
        push_clocks(&mut from, start.clocks());
        if let Some(&known) = self.divergence.get(&from) {
            return Ok(known);
        }
        let found = self.search_divergence(start)?;
        self.divergence.insert(from, found);
        Ok(found)
    }

    fn search_divergence(&mut self, start: &TimedState) -> Result<bool, Halt> {
        let mut index: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        let mut states: Vec<TimedState> = Vec::new();
        let mut edges: Vec<(usize, usize, bool)> = Vec::new();
        let mut queue = VecDeque::new();
        let key = |run: &Self, state: &TimedState| -> Result<Vec<u8>, Halt> {
            let mut key = run.discrete_key(state.discrete())?;
            push_clocks(&mut key, state.clocks());
            Ok(key)
        };
        index.insert(key(self, start)?, 0);
        states.push(start.clone());
        queue.push_back(0_usize);
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let state = states[at].clone();
            if self.quiescent(&state)? {
                return Ok(true);
            }
            let mut next: Vec<(TimedState, bool)> = self
                .steps_of(&state)?
                .into_iter()
                .map(|(_, post)| (self.capped_state(&post), false))
                .collect();
            if let Some(later) = self.delayed(&state)? {
                next.push((later, true));
            }
            for (post, delay) in next {
                let found = key(self, &post)?;
                let to = match index.get(&found) {
                    Some(&to) => to,
                    None => {
                        let limits = self.request.limits;
                        if u64::try_from(states.len()).unwrap_or(u64::MAX) >= limits.max_states {
                            return Err(self.limit(ModelCheckLimit::MaxStates, limits.max_states));
                        }
                        index.insert(found, states.len());
                        states.push(post);
                        queue.push_back(states.len() - 1);
                        states.len() - 1
                    }
                };
                edges.push((at, to, delay));
            }
        }
        // A delay edge inside a strongly connected component.
        let mut graph = Graph::new();
        let handles: Vec<Vertex> = (0..states.len())
            .map(|_| graph.add_vertex(Vec::new()))
            .collect();
        let fault = |_: FairnessError| Halt::from(DigitalRefusal::Fault("divergence-graph"));
        for &(from, to, _) in &edges {
            graph
                .add_edge(handles[from], handles[to], EdgeLabel::Stutter)
                .map_err(fault)?;
        }
        graph
            .add_acceptance_set(
                edges
                    .iter()
                    .filter(|(_, _, delay)| *delay)
                    .map(|&(_, to, _)| handles[to]),
            )
            .map_err(fault)?;
        let all: BTreeSet<Vertex> = graph.vertices().collect();
        let analyses = analyze(&graph, &all, &[], self.cancel).map_err(|error| match error {
            FairnessError::Cancelled => self.cancelled(),
            _ => DigitalRefusal::Fault("divergence-analysis").into(),
        })?;
        let found = Analysis::first_fair_in(&analyses).is_some();
        Ok(found)
    }
}

/// The product edges out of vertex `from`, in the order the fairness graph
/// holds them: an undefined position's closing edge is never added.
fn graph_edges(product: &Product, from: usize) -> Vec<usize> {
    product.out.get(from).cloned().unwrap_or_default()
}

/// The initial state's index of the path `path` to `at`.
fn root(product: &Product, path: &[usize], at: usize) -> Result<usize, DigitalRefusal> {
    let first = path.first().map_or(at, |&edge| product.edges[edge].from);
    product.vertices[first]
        .initial
        .ok_or(DigitalRefusal::Fault("path-root"))
}

/// Number `transition` among `transitions`, adding it when new.
fn number(transitions: &mut Vec<ModelTransition>, transition: &ModelTransition) -> Identity {
    let index = match transitions.iter().position(|held| held == transition) {
        Some(index) => index,
        None => {
            transitions.push(transition.clone());
            transitions.len() - 1
        }
    };
    Identity::new(u64::try_from(index).unwrap_or(u64::MAX))
}

/// Append each clock value of `clocks` to `key`.
fn push_clocks(key: &mut Vec<u8>, clocks: &ClockValuation) {
    for value in clocks.values() {
        key.extend_from_slice(value.numerator().as_big().to_string().as_bytes());
        key.push(b'/');
        key.extend_from_slice(value.denominator().as_big().to_string().as_bytes());
        key.push(b';');
    }
    key.push(b'|');
}
