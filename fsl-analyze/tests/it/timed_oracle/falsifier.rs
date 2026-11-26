// SPDX-License-Identifier: AGPL-3.0-or-later
//! The multi-grid dense-time falsifier (FSpec FR-751): it walks the concrete
//! behaviours of a model over rational delays and reads each with the
//! concrete-trace evaluator. It uses only the shared concrete step semantics
//! ([`TimedSystem`]): no zone, no region and no grid of an analysis route.
//! It reads a sample of the dense behaviours, so it can show a claim false
//! and never shows one true; it is a test oracle and settles nothing.
//!
//! Each grid is the multiples of `1/n` up to a bound, for `n` in
//! [`DIVISIONS`]; the grids with `n` above 1 hold delays strictly between
//! the integer boundaries the models compare clocks with.
//!
//! A behaviour ends in one of two ways. At a quiescent state it may idle
//! forever, the stutter step repeating after a positive delay. Where no step
//! is enabled and time cannot pass a bound, it is time-locked: a
//! probabilistic claim reads the run as ended with time frozen
//! ([`Lock::Frozen`]), and a temporal claim does not admit it
//! ([`Lock::Excluded`]), since its stamps do not diverge.

use filament_exact::{Integer, Meter, Rational};
use fsl_eval::model_subject::{ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem, TimedState, TimedSystem};
use fsl_eval::temporal::{
    evaluate_timed_at, AtomReading, LetterStop, MeasureReading, TemporalLetters, TemporalOutcome,
    TemporalPosition, TimedTrace,
};
use fsl_semantics::check::{FormulaNodeId, TemporalClauseNode};

use crate::timed_world::World;

/// The grids' divisions of one time unit.
const DIVISIONS: [i64; 4] = [1, 2, 3, 4];

/// The most discrete steps a behaviour of a corpus model takes.
const DEPTH: usize = 8;

pub(super) fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

pub(super) fn whole(value: i64) -> Rational {
    ratio(value, 1)
}

/// How a time-locked behaviour is read.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Lock {
    /// The run ends with time frozen (probabilistic claims).
    Frozen,
    /// The behaviour is not admitted (temporal claims).
    Excluded,
}

/// What the falsifier walks.
#[derive(Clone)]
pub(super) struct Space {
    /// The largest delay of one move, past every constant of the model.
    pub(super) bound: i64,
    /// The delays between two stutter steps of an idle tail.
    pub(super) idle: Vec<Rational>,
    pub(super) lock: Lock,
    /// Operations whose enabled successors are the equally weighted outcomes
    /// of one draw (`random` with equal weights).
    pub(super) drawn: Vec<&'static str>,
    /// Operations with a one-point `delay` distribution: they fire that long
    /// after the state that enables them is entered.
    pub(super) fixed: Vec<(&'static str, Rational)>,
    /// The delays walked, in place of the grids.
    pub(super) delays: Option<Vec<Rational>>,
    /// The time past which no claim of the model reads a position: a
    /// behaviour that passes it is read as it stands.
    pub(super) horizon: Option<i64>,
}

impl Space {
    pub(super) fn new(bound: i64, lock: Lock) -> Self {
        Self {
            bound,
            idle: vec![ratio(1, 2), whole(1), ratio(7, 3)],
            lock,
            drawn: Vec::new(),
            fixed: Vec::new(),
            delays: None,
            horizon: None,
        }
    }
}

/// How long past its last position a locked run's record is complete: past
/// every window of the corpus.
const CLOSED_AFTER: i64 = 1000;

/// One behaviour: its positions' states and stamps. With `idle`, the last
/// position repeats forever after that delay; without it the run ended in a
/// time-lock and has no later position (FSpec FR-416).
pub(super) struct Behaviour {
    pub(super) states: Vec<ModelSimState>,
    pub(super) stamps: Vec<Rational>,
    idle: Option<Rational>,
}

impl Behaviour {
    pub(super) fn trace(&self) -> TimedTrace {
        let Some(idle) = &self.idle else {
            let last = self.stamps.last().expect("a position");
            return TimedTrace::through(self.stamps.clone(), last.add(&whole(CLOSED_AFTER)))
                .expect("a closed timed trace");
        };
        let last = u64::try_from(self.stamps.len() - 1).expect("a short behaviour");
        TimedTrace::lasso(self.stamps.clone(), last, idle.clone()).expect("a timed lasso")
    }
}

/// The least and greatest expected payoff over the schedulers of the walked
/// behaviours.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Extrema {
    pub(super) min: Rational,
    pub(super) max: Rational,
}

impl Extrema {
    fn one(value: Rational) -> Self {
        Self {
            min: value.clone(),
            max: value,
        }
    }

    /// A scheduler's choice between `self` and `other`.
    fn or(choices: impl IntoIterator<Item = Self>) -> Option<Self> {
        choices.into_iter().reduce(|a, b| Self {
            min: a.min.min(b.min),
            max: a.max.max(b.max),
        })
    }

    /// The equally weighted draw over `outcomes`.
    fn mean(outcomes: &[Self]) -> Self {
        let count = whole(i64::try_from(outcomes.len()).expect("a few outcomes"));
        let sum = |of: fn(&Self) -> &Rational| {
            outcomes
                .iter()
                .fold(whole(0), |total, outcome| total.add(of(outcome)))
                .div(&count)
                .expect("at least one outcome")
        };
        Self {
            min: sum(|e| &e.min),
            max: sum(|e| &e.max),
        }
    }
}

/// Whether the operation whose declaration identity text (FR-181) is
/// `operation` is the one named `name`: the identity's last path segment.
pub(super) fn named(operation: &str, name: &str) -> bool {
    operation.contains(&format!("/{name}\""))
}

type Payoff<'p> = dyn Fn(&Behaviour) -> Rational + 'p;

struct Walk<'w, 's, 'a, 'p> {
    timed: &'w TimedSystem<'s, 'a>,
    space: &'w Space,
    grid: Vec<Rational>,
    payoff: &'w Payoff<'p>,
}

impl Walk<'_, '_, '_, '_> {
    fn leaf(
        &self,
        states: &[ModelSimState],
        stamps: &[Rational],
        idle: Option<Rational>,
    ) -> Extrema {
        Extrema::one((self.payoff)(&Behaviour {
            states: states.to_vec(),
            stamps: stamps.to_vec(),
            idle,
        }))
    }

    fn fixed(&self, operation: &str) -> Option<Rational> {
        self.space
            .fixed
            .iter()
            .find(|(name, _)| named(operation, name))
            .map(|(_, delay)| delay.clone())
    }

    /// The extrema over every continuation from `state`, or `None` when no
    /// admitted behaviour continues from it.
    fn from(
        &self,
        state: &TimedState,
        states: &mut Vec<ModelSimState>,
        stamps: &mut Vec<Rational>,
    ) -> Option<Extrema> {
        let now = stamps.last().expect("a position");
        if self.space.horizon.is_some_and(|h| now > &whole(h)) {
            return Some(self.leaf(states, stamps, Some(ratio(7, 3))));
        }
        // Zero-delay steps without end pass no time: no admitted behaviour.
        if states.len() > DEPTH {
            return None;
        }
        let mut choices = Vec::new();
        let (mut stepped, mut forced) = (false, false);
        for delay in &self.grid {
            let Some(delayed) = self.timed.delay(state, delay).expect("a decided delay") else {
                break;
            };
            let steps = self.timed.steps(&delayed).expect("decided steps");
            let mut by_operation: Vec<(String, Vec<TimedState>)> = Vec::new();
            for (step, reached) in steps {
                stepped = true;
                let operation = step.operation().to_owned();
                if let Some(at) = self.fixed(&operation) {
                    forced = true;
                    if &at != delay {
                        continue;
                    }
                }
                let drawn = self.space.drawn.iter().any(|n| named(&operation, n));
                match by_operation
                    .iter_mut()
                    .find(|(o, _)| drawn && *o == operation)
                {
                    Some((_, outcomes)) => outcomes.push(reached),
                    None => by_operation.push((operation, vec![reached])),
                }
            }
            for (_, outcomes) in by_operation {
                let values: Vec<Option<Extrema>> = outcomes
                    .iter()
                    .map(|reached| {
                        states.push(reached.discrete().clone());
                        stamps.push(stamps.last().expect("a position").add(delay));
                        let value = self.from(reached, states, stamps);
                        states.pop();
                        stamps.pop();
                        value
                    })
                    .collect();
                if let Some(values) = values.into_iter().collect::<Option<Vec<_>>>() {
                    choices.push(Extrema::mean(&values));
                }
            }
            if self.space.lock == Lock::Frozen
                && self
                    .timed
                    .time_lock(&delayed, 64)
                    .expect("a decided time lock")
                    .is_some()
            {
                choices.push(self.leaf(states, stamps, None));
            }
        }
        let quiescent = self
            .timed
            .is_quiescent(state.discrete())
            .expect("a decided state");
        if quiescent && !forced {
            for idle in &self.space.idle {
                choices.push(self.leaf(states, stamps, Some(idle.clone())));
            }
        }
        // Time bounded below a strict invariant with nothing enabled: the
        // lock is approached and never reached.
        if !quiescent && !stepped && self.space.lock == Lock::Frozen {
            choices.push(self.leaf(states, stamps, None));
        }
        Extrema::or(choices)
    }
}

/// The extrema of `payoff` over every grid.
pub(super) fn extrema(
    system: &ModelSystem<'_>,
    space: &Space,
    payoff: &Payoff<'_>,
) -> Option<Extrema> {
    let timed = TimedSystem::new(system, "M").expect("a timed subject");
    Extrema::or(DIVISIONS.iter().filter_map(|&n| {
        let walk = Walk {
            timed: &timed,
            space,
            grid: space
                .delays
                .clone()
                .unwrap_or_else(|| (0..=space.bound * n).map(|k| ratio(k, n)).collect()),
            payoff,
        };
        Extrema::or(timed.initial().iter().filter_map(|initial| {
            walk.from(
                initial,
                &mut vec![initial.discrete().clone()],
                &mut vec![whole(0)],
            )
        }))
    }))
}

fn node<'w>(world: &'w World, name: &str) -> &'w TemporalClauseNode {
    world
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
        .unwrap_or_else(|| panic!("the unit declares {name}"))
}

/// The one object the corpus clauses range over.
fn instance(system: &ModelSystem<'_>, node: &TemporalClauseNode) -> ModelInstance {
    system
        .instances(&node.clause().over().1)
        .expect("a bounded population")
        .into_iter()
        .next()
        .expect("one object")
}

struct Letters<'b, L> {
    letters: &'b L,
    instance: &'b ModelInstance,
    states: &'b [ModelSimState],
}

impl<L: ModelLetters> TemporalLetters for Letters<'_, L> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(position.index()).expect("a represented position");
        Ok(self
            .letters
            .atom(atom, self.instance, &self.states[index], meter)?)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }
}

/// The value at position 0 of the formula the temporal declaration `name`
/// states, over `behaviour`: 1 where it holds and 0 where it is violated.
pub(super) fn holds(
    world: &World,
    system: &ModelSystem<'_>,
    name: &str,
    behaviour: &Behaviour,
) -> Rational {
    let node = node(world, name);
    let model = system.clause_letters(node).expect("the clause has letters");
    let instance = instance(system, node);
    let outcome = evaluate_timed_at(
        node.clause(),
        &behaviour.trace(),
        TemporalPosition::new(0),
        &mut Letters {
            letters: &model,
            instance: &instance,
            states: &behaviour.states,
        },
        &mut Meter::new(system.meter_limits()),
    )
    .expect("the formula evaluates");
    match outcome {
        TemporalOutcome::Holds => whole(1),
        TemporalOutcome::Violated { .. } => whole(0),
        other => panic!("a lasso decides {name}: {other:?}"),
    }
}

/// The truth at each position of the state predicate the temporal
/// declaration `name` states, a formula that is one `holds`.
pub(super) fn truths(
    world: &World,
    system: &ModelSystem<'_>,
    name: &str,
    behaviour: &Behaviour,
) -> Vec<bool> {
    let node = node(world, name);
    let model = system.clause_letters(node).expect("the clause has letters");
    let instance = instance(system, node);
    let root = node
        .clause()
        .formula()
        .node_id(node.clause().formula().root().index())
        .expect("the root has an identity");
    behaviour
        .states
        .iter()
        .map(|state| {
            match model
                .atom(
                    root,
                    &instance,
                    state,
                    &mut Meter::new(system.meter_limits()),
                )
                .expect("the predicate reads")
            {
                AtomReading::True => true,
                AtomReading::False => false,
                other => panic!("{name} has a truth value: {other:?}"),
            }
        })
        .collect()
}

/// A measure reading as an amount: `None` for a behaviour that does not
/// activate it, and `Some(None)` for a censored one.
pub(super) fn amount(reading: MeasureReading) -> Option<Option<Rational>> {
    match reading {
        MeasureReading::NotActivated => None,
        MeasureReading::Censored => Some(None),
        MeasureReading::Value(value) => Some(Some(value)),
        other => panic!("a lasso decides a measure: {other:?}"),
    }
}
