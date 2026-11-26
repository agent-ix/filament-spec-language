// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's checked-expression interpreter for finite untimed state clauses.

use std::collections::BTreeMap;

use filament_checked::node::{Arithmetic, Connective, NodeKind, OrderedKind};
use filament_contract_model::{CheckedFieldScope, CheckedFrameModification};
use filament_exact::{Rational, Value};
use fsl_semantics::check::{
    CheckedGraph, CheckedNode, StateClauseKind, StatePredicate, TimedModel,
};
use ix_trace_rs::implements;

use crate::{ReferenceRefusal, TinyMdp};

const MAX_EXPRESSION_NODES: usize = 100_000;
const MAX_EXPRESSION_DEPTH: usize = 256;
const MAX_FINITE_STATES: usize = 1 << 16;

/// A probabilistic support weight before the reference admits a transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceProbabilityWeight {
    /// A literal exact rational from the checked package.
    Literal(Rational),
    /// Any authored expression, retained only so refusal can name it.
    Expression(String),
}

/// One exact measure value and its literal rational path mass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceMeasureOutcome {
    /// The outcome's exact measure, such as elapsed time or path length.
    pub measure: Rational,
    /// The outcome's probability mass.
    pub probability: Rational,
}

/// Computes the exact cumulative mass whose measure is at most `bound`.
#[implements("FR-1051-AC-17")]
pub fn measure_mass_at_most(
    outcomes: &[ReferenceMeasureOutcome],
    bound: &Rational,
) -> Result<Rational, ReferenceRefusal> {
    if outcomes.is_empty() {
        return Err(unsupported("empty finite measure support"));
    }
    let zero = Rational::from_integer(filament_exact::Integer::zero());
    let one = Rational::from_integer(filament_exact::Integer::one());
    let mut total = zero.clone();
    let mut cumulative = zero;
    for outcome in outcomes {
        if outcome.probability < Rational::from_integer(filament_exact::Integer::zero()) {
            return Err(unsupported("negative path probability"));
        }
        total = total.add(&outcome.probability);
        if outcome.measure <= *bound {
            cumulative = cumulative.add(&outcome.probability);
        }
    }
    if total != one {
        return Err(unsupported("finite measure support does not sum to one"));
    }
    Ok(cumulative)
}

/// Admits only exact literal rational weights for an interpreted transition.
#[implements("FR-1051-AC-19")]
pub fn probability_weight(
    weight: &ReferenceProbabilityWeight,
) -> Result<Rational, ReferenceRefusal> {
    match weight {
        ReferenceProbabilityWeight::Literal(value)
            if value >= &Rational::from_integer(filament_exact::Integer::zero()) =>
        {
            Ok(value.clone())
        }
        ReferenceProbabilityWeight::Literal(_) => {
            Err(unsupported("negative literal probability weight"))
        }
        ReferenceProbabilityWeight::Expression(_) => {
            Err(unsupported("non-literal probability weight"))
        }
    }
}

/// Computes the exact probability of a Boolean predicate over one checked
/// random-parameter support, binding each support value into `parameter_slot`.
#[implements("FR-1051-AC-17")]
pub fn checked_support_probability(
    graph: &CheckedGraph,
    random: &fsl_semantics::check::RandomParameter,
    parameter_slot: usize,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    current_state: &ReferenceFiniteState,
    predicate: CheckedNode<'_>,
) -> Result<Rational, ReferenceRefusal> {
    if parameter_slot >= locals.len() {
        return Err(unsupported("random parameter slot outside checked frame"));
    }
    let mut probability = Rational::from_integer(filament_exact::Integer::zero());
    for support in random.support() {
        let mut bound = locals.to_vec();
        bound[parameter_slot] = support.value.0.clone();
        if evaluate_boolean(graph, predicate, &bound, pre_state, current_state)? {
            probability = probability.add(&support.probability);
        }
    }
    Ok(probability)
}

/// Computes the exact expected integer or rational reward over one checked
/// random-parameter support.
#[implements("FR-1051-AC-17")]
pub fn checked_support_expected_reward(
    graph: &CheckedGraph,
    random: &fsl_semantics::check::RandomParameter,
    parameter_slot: usize,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    current_state: &ReferenceFiniteState,
    reward: CheckedNode<'_>,
) -> Result<Rational, ReferenceRefusal> {
    if parameter_slot >= locals.len() {
        return Err(unsupported("random parameter slot outside checked frame"));
    }
    let mut expected = Rational::from_integer(filament_exact::Integer::zero());
    for support in random.support() {
        let mut bound = locals.to_vec();
        bound[parameter_slot] = support.value.0.clone();
        let reward = match evaluate_finite_value(graph, reward, &bound, pre_state, current_state)? {
            Value::Integer(value) => Rational::from_integer(value),
            Value::Rational(value) => value,
            _ => return Err(unsupported("reward outside integer or rational values")),
        };
        expected = expected.add(&support.probability.mul(&reward));
    }
    Ok(expected)
}

/// Builds an exact one-action MDP from one checked operation and one checked
/// random parameter. Each support value must lead to exactly one enumerated
/// post-state under the checked postconditions.
#[implements("FR-1051-AC-17")]
pub fn checked_random_operation_mdp(
    graph: &CheckedGraph,
    operation: &str,
    parameter: &str,
    locals: &[Value],
    states: Vec<ReferenceFiniteState>,
) -> Result<TinyMdp, ReferenceRefusal> {
    if states.is_empty() || states.len() > MAX_FINITE_STATES {
        return Err(unsupported(
            "finite transition state count outside 1..=2^16",
        ));
    }
    let random = graph
        .probabilistic_model()
        .random_parameters()
        .iter()
        .find(|random| random.operation().name() == operation && random.parameter() == parameter)
        .ok_or_else(|| unsupported("missing checked random support"))?;
    let slot = random
        .position()
        .checked_add(1)
        .ok_or_else(|| unsupported("random parameter slot overflow"))?;
    if slot >= locals.len() {
        return Err(unsupported("random parameter slot outside checked frame"));
    }
    let clauses: Vec<_> = graph
        .state_clauses()
        .iter()
        .filter(|clause| {
            clause
                .operation()
                .is_some_and(|item| item.declaration.name() == operation)
        })
        .collect();
    let preconditions: Vec<_> = clauses
        .iter()
        .copied()
        .filter(|clause| clause.kind() == StateClauseKind::Precondition)
        .collect();
    let postconditions: Vec<_> = clauses
        .iter()
        .copied()
        .filter(|clause| clause.kind() == StateClauseKind::Postcondition)
        .collect();
    if postconditions.is_empty() {
        return Err(unsupported(
            "random operation without a checked postcondition",
        ));
    }
    let mut actions = Vec::with_capacity(states.len());
    for source in &states {
        let mut distribution = BTreeMap::<usize, Rational>::new();
        for support in random.support() {
            let mut bound = locals.to_vec();
            bound[slot] = support.value.0.clone();
            let mut enabled = true;
            for clause in &preconditions {
                enabled &= evaluate_boolean(graph, clause.body(), &bound, source, source)?;
            }
            if !enabled {
                continue;
            }
            let mut destinations = Vec::new();
            for (target_index, target) in states.iter().enumerate() {
                let mut accepted = true;
                for clause in &postconditions {
                    accepted &= evaluate_boolean(graph, clause.body(), &bound, source, target)?;
                }
                accepted &=
                    operation_frame_allows_transition(graph, operation, &bound, source, target)?;
                if accepted {
                    destinations.push(target_index);
                }
            }
            if destinations.len() != 1 {
                return Err(unsupported(if destinations.is_empty() {
                    "random operation next-state outside finite valuation domain"
                } else {
                    "random operation has ambiguous next-state effect"
                }));
            }
            let destination = destinations[0];
            let mass = distribution
                .entry(destination)
                .or_insert_with(|| Rational::from_integer(filament_exact::Integer::zero()));
            *mass = mass.add(&support.probability);
        }
        if distribution.is_empty() {
            return Err(unsupported("random operation disabled at a finite state"));
        }
        actions.push(vec![distribution.into_iter().collect()]);
    }
    TinyMdp::new(actions)
}

/// One concrete finite state, indexed by object identity and field name.
#[derive(Clone, Debug, Default)]
pub struct ReferenceFiniteState {
    fields: BTreeMap<(String, String), Value>,
}

/// A finite transition graph derived from one operation's checked pre/post
/// clauses over an explicitly enumerated state space.
#[derive(Clone, Debug)]
pub struct ReferenceTransitionGraph {
    states: Vec<ReferenceFiniteState>,
    successors: Vec<Vec<usize>>,
}

impl ReferenceTransitionGraph {
    /// Derives an operation relation by evaluating its checked clauses over
    /// every pre/post state pair. When no postcondition exists, the operation
    /// is interpreted as a stutter; unsupported clause nodes refuse by name.
    #[implements("FR-1051-AC-17")]
    pub fn from_operation(
        graph: &CheckedGraph,
        operation: &str,
        locals: &[Value],
        states: Vec<ReferenceFiniteState>,
    ) -> Result<Self, ReferenceRefusal> {
        if states.is_empty() || states.len() > MAX_FINITE_STATES {
            return Err(unsupported(
                "finite transition state count outside 1..=2^16",
            ));
        }
        let clauses: Vec<_> = graph
            .state_clauses()
            .iter()
            .filter(|clause| {
                clause
                    .operation()
                    .is_some_and(|item| item.declaration.name() == operation)
            })
            .collect();
        if clauses.is_empty() {
            return Err(unsupported("operation without checked clauses"));
        }
        let preconditions: Vec<_> = clauses
            .iter()
            .copied()
            .filter(|clause| clause.kind() == StateClauseKind::Precondition)
            .collect();
        let postconditions: Vec<_> = clauses
            .iter()
            .copied()
            .filter(|clause| clause.kind() == StateClauseKind::Postcondition)
            .collect();
        let mut successors = vec![Vec::new(); states.len()];
        for (source_index, source) in states.iter().enumerate() {
            let mut enabled = true;
            for clause in &preconditions {
                enabled &= evaluate_boolean(graph, clause.body(), locals, source, source)?;
            }
            if !enabled {
                continue;
            }
            for (target_index, target) in states.iter().enumerate() {
                let accepted = if postconditions.is_empty() {
                    source_index == target_index
                } else {
                    let mut accepted = true;
                    for clause in &postconditions {
                        accepted &= evaluate_boolean(graph, clause.body(), locals, source, target)?;
                    }
                    accepted
                };
                if accepted
                    && operation_frame_allows_transition(graph, operation, locals, source, target)?
                {
                    successors[source_index].push(target_index);
                }
            }
        }
        Ok(Self { states, successors })
    }

    /// The enumerated states in their deterministic input order.
    pub fn states(&self) -> &[ReferenceFiniteState] {
        &self.states
    }

    /// The successor indices of one state.
    pub fn successors(&self, state: usize) -> Option<&[usize]> {
        self.successors.get(state).map(Vec::as_slice)
    }

    /// Whether any path from `initial` reaches one of the target states.
    #[implements("FR-1051-AC-17")]
    pub fn can_reach(&self, initial: usize, targets: &[bool]) -> Result<bool, ReferenceRefusal> {
        if initial >= self.states.len() || targets.len() != self.states.len() {
            return Err(unsupported(
                "reachability target set outside finite state space",
            ));
        }
        let mut seen = vec![false; self.states.len()];
        let mut pending = vec![initial];
        while let Some(state) = pending.pop() {
            if seen[state] {
                continue;
            }
            seen[state] = true;
            if targets[state] {
                return Ok(true);
            }
            pending.extend(self.successors[state].iter().copied());
        }
        Ok(false)
    }
}

/// Checks state changes against the admitted operation effect row. The tiny
/// fragment supports field writes only; object and relationship effects refuse.
#[implements("FR-1051-AC-17")]
fn operation_frame_allows_transition(
    graph: &CheckedGraph,
    operation: &str,
    locals: &[Value],
    before: &ReferenceFiniteState,
    after: &ReferenceFiniteState,
) -> Result<bool, ReferenceRefusal> {
    let rows = graph
        .operation_effects()
        .ok_or_else(|| unsupported("operation without checked effects"))?;
    let mut matching = rows
        .iter()
        .filter(|row| row.operation.as_ref() == operation);
    let row = matching
        .next()
        .ok_or_else(|| unsupported("operation without a checked effect row"))?;
    if matching.next().is_some() {
        return Err(unsupported("operation name with ambiguous checked effects"));
    }
    let effects = &row.effect;
    if !effects.creates.is_empty()
        || !effects.deletes.is_empty()
        || !effects.deletes_scoped.is_empty()
    {
        return Err(unsupported("object creation or deletion effect"));
    }
    let mut writes = Vec::new();
    for modification in &effects.modifies {
        match modification {
            CheckedFrameModification::Field { name, scope, .. } => {
                writes.push((name.as_ref(), scope))
            }
            CheckedFrameModification::Relationship { .. } => {
                return Err(unsupported("relationship effect"));
            }
        }
    }
    let receiver = match locals.first() {
        Some(Value::Reference(reference)) => Some(reference.object().as_str()),
        _ => None,
    };
    if before.fields.keys().ne(after.fields.keys()) {
        return Err(unsupported("state shape change without object effect"));
    }
    for ((object, field), value) in &before.fields {
        if after
            .fields
            .get(&(object.clone(), field.clone()))
            .is_some_and(|after| finite_scalar_equal(value, after))
        {
            continue;
        }
        let granted = writes.iter().any(|(name, scope)| {
            name == field
                && match scope {
                    CheckedFieldScope::EveryObject => true,
                    CheckedFieldScope::Receiver => receiver == Some(object.as_str()),
                }
        });
        if !granted {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A finite prefix followed by a nonempty loop, representing an infinite trace.
#[derive(Clone, Debug)]
pub struct ReferenceLasso {
    states: Vec<ReferenceFiniteState>,
    loop_start: usize,
}

/// A finite sequence of state observations at exact rational times.
#[derive(Clone, Debug)]
pub struct ReferenceTimedTrace {
    observations: Vec<(Rational, ReferenceFiniteState)>,
}

impl ReferenceTimedTrace {
    /// Builds a nonempty timed path with nondecreasing observation times.
    pub fn new(
        observations: Vec<(Rational, ReferenceFiniteState)>,
    ) -> Result<Self, ReferenceRefusal> {
        let zero = Rational::from_integer(filament_exact::Integer::zero());
        if observations.is_empty()
            || observations.iter().any(|(time, _)| time < &zero)
            || observations
                .windows(2)
                .any(|window| window[0].0 > window[1].0)
        {
            return Err(unsupported("invalid timed trace observations"));
        }
        Ok(Self { observations })
    }

    /// Evaluates a closed eventually window relative to `origin`. A jump
    /// from before the window to after it does not invent an observation at
    /// an intermediate time.
    #[implements("FR-1051-AC-17")]
    pub fn eventually_within(
        &self,
        graph: &CheckedGraph,
        predicate: CheckedNode<'_>,
        locals: &[Value],
        origin: &Rational,
        lower: &Rational,
        upper: &Rational,
    ) -> Result<bool, ReferenceRefusal> {
        if lower > upper {
            return Err(unsupported("inverted timed observation window"));
        }
        for (time, state) in &self.observations {
            let elapsed = time.sub(origin);
            if elapsed >= *lower
                && elapsed <= *upper
                && evaluate_boolean(graph, predicate, locals, state, state)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl ReferenceLasso {
    /// Creates a lasso whose repeated suffix begins at `loop_start`.
    pub fn new(
        states: Vec<ReferenceFiniteState>,
        loop_start: usize,
    ) -> Result<Self, ReferenceRefusal> {
        if states.is_empty() || loop_start >= states.len() {
            return Err(unsupported("lasso has no nonempty loop"));
        }
        Ok(Self { states, loop_start })
    }

    /// Whether a checked state predicate holds at some position of the
    /// represented infinite trace.
    #[implements("FR-1051-AC-17")]
    pub fn eventually(
        &self,
        graph: &CheckedGraph,
        predicate: CheckedNode<'_>,
        locals: &[Value],
    ) -> Result<bool, ReferenceRefusal> {
        for state in &self.states {
            if evaluate_boolean(graph, predicate, locals, state, state)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a checked state predicate holds at every position of the
    /// represented infinite trace.
    #[implements("FR-1051-AC-17")]
    pub fn always(
        &self,
        graph: &CheckedGraph,
        predicate: CheckedNode<'_>,
        locals: &[Value],
    ) -> Result<bool, ReferenceRefusal> {
        for state in &self.states {
            if !evaluate_boolean(graph, predicate, locals, state, state)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The first state of the trace.
    pub fn initial(&self) -> &ReferenceFiniteState {
        &self.states[0]
    }

    /// The repeated suffix's first state index.
    pub fn loop_start(&self) -> usize {
        self.loop_start
    }
}

/// One exact clock valuation in a checked timed model's clock order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceClockState {
    clocks: Vec<Rational>,
}

impl ReferenceClockState {
    /// Creates a nonnegative rational valuation with one value per clock.
    pub fn new(model: &TimedModel, clocks: Vec<Rational>) -> Result<Self, ReferenceRefusal> {
        let zero = Rational::from_integer(filament_exact::Integer::zero());
        if clocks.len() != model.clocks().len() || clocks.iter().any(|value| value < &zero) {
            return Err(unsupported(
                "clock valuation has wrong dimension or negative value",
            ));
        }
        Ok(Self { clocks })
    }

    /// Returns this state's exact clock values in checked clock order.
    pub fn values(&self) -> &[Rational] {
        &self.clocks
    }

    /// Evaluates the indexed checked guard when it is one clock-only
    /// conjunction. Any other guard feature is refused by name.
    #[implements("FR-1051-AC-17")]
    pub fn satisfies_guard(
        &self,
        model: &TimedModel,
        guard_index: usize,
    ) -> Result<bool, ReferenceRefusal> {
        let guard = model
            .guards()
            .get(guard_index)
            .ok_or_else(|| unsupported("missing checked clock guard"))?;
        if guard.parts().len() != 1 {
            return Err(unsupported("disjunctive timed guard"));
        }
        let part = &guard.parts()[0];
        if part.data().is_some()
            || !part.presences().is_empty()
            || !part.element_presences().is_empty()
            || !part.elements().is_empty()
            || !part.families().is_empty()
        {
            return Err(unsupported(
                "timed guard with data or quantified clock access",
            ));
        }
        let mut satisfied = true;
        for constraint in part.clocks() {
            if !constraint.via().is_empty()
                || !constraint.slots().is_empty()
                || constraint.parameter().is_some()
            {
                return Err(unsupported("indirect or parameterized clock constraint"));
            }
            let value = self
                .clocks
                .get(constraint.clock().index())
                .ok_or_else(|| unsupported("clock constraint outside valuation"))?;
            satisfied &= clock_comparison(value, constraint.op(), constraint.bound());
        }
        Ok(satisfied)
    }

    /// Evaluates one checked conjunction of clock bounds from a time invariant.
    #[implements("FR-1051-AC-17")]
    pub fn satisfies_invariant(
        &self,
        model: &TimedModel,
        invariant_index: usize,
    ) -> Result<bool, ReferenceRefusal> {
        let invariant = model
            .invariants()
            .get(invariant_index)
            .ok_or_else(|| unsupported("missing checked time invariant"))?;
        if invariant.alternatives().iter().any(|alternative| {
            !alternative.presences().is_empty()
                || !matches!(
                    alternative.predicate().body(),
                    Some(node) if matches!(node.kind(), NodeKind::Literal(literal) if matches!(literal.0, Value::Boolean(true)))
                )
        }) {
            return Err(unsupported("time invariant with a non-constant data predicate"));
        }
        let mut satisfied = true;
        for constraint in invariant.bounds() {
            if !constraint.via().is_empty()
                || !constraint.slots().is_empty()
                || constraint.parameter().is_some()
            {
                return Err(unsupported("indirect or parameterized clock invariant"));
            }
            let value = self
                .clocks
                .get(constraint.clock().index())
                .ok_or_else(|| unsupported("clock invariant outside valuation"))?;
            satisfied &= clock_comparison(value, constraint.op(), constraint.bound());
        }
        Ok(satisfied)
    }

    /// Applies the indexed checked reset clause when all its resets are to zero.
    #[implements("FR-1051-AC-17")]
    pub fn apply_zero_resets(
        &self,
        model: &TimedModel,
        reset_index: usize,
    ) -> Result<Self, ReferenceRefusal> {
        let resets = model
            .resets()
            .get(reset_index)
            .ok_or_else(|| unsupported("missing checked clock reset"))?;
        let mut clocks = self.clocks.clone();
        for reset in resets.resets() {
            if !reset.value().is_zero()
                || reset.parameter().is_some()
                || reset.selection().is_some()
            {
                return Err(unsupported("nonzero or selected clock reset"));
            }
            let value = clocks
                .get_mut(reset.clock().index())
                .ok_or_else(|| unsupported("clock reset outside valuation"))?;
            *value = Rational::from_integer(filament_exact::Integer::zero());
        }
        Ok(Self { clocks })
    }

    /// Advances every clock by the same exact nonnegative dense-time delay.
    pub fn delay(&self, delta: &Rational) -> Result<Self, ReferenceRefusal> {
        if delta < &Rational::from_integer(filament_exact::Integer::zero()) {
            return Err(unsupported("negative clock delay"));
        }
        Ok(Self {
            clocks: self.clocks.iter().map(|clock| clock.add(delta)).collect(),
        })
    }
}

fn clock_comparison(
    value: &Rational,
    operator: fsl_semantics::check::ClockOp,
    bound: &Rational,
) -> bool {
    use fsl_semantics::check::ClockOp;
    match operator {
        ClockOp::Less => value < bound,
        ClockOp::LessOrEqual => value <= bound,
        ClockOp::Equal => value == bound,
        ClockOp::GreaterOrEqual => value >= bound,
        ClockOp::Greater => value > bound,
    }
}

impl ReferenceFiniteState {
    /// Creates a state from the supplied `(object, field, value)` entries.
    pub fn new(
        fields: impl IntoIterator<Item = (String, String, Value)>,
    ) -> Result<Self, ReferenceRefusal> {
        let mut state = Self::default();
        for (object, field, value) in fields {
            if !finite_state_value(&value) {
                return Err(unsupported("non-finite or non-admitted state field value"));
            }
            let key = (object, field);
            if state.fields.insert(key, value).is_some() {
                return Err(ReferenceRefusal {
                    construct: "duplicate finite-state field".to_owned(),
                });
            }
        }
        Ok(state)
    }

    /// Reads a field value from this state.
    pub fn field(&self, object: &str, field: &str) -> Option<&Value> {
        self.fields.get(&(object.to_owned(), field.to_owned()))
    }

    /// Enumerates the Cartesian product of finite scalar field domains in
    /// deterministic field order, refusing products larger than 2^16.
    #[implements("FR-1051-AC-17")]
    pub fn enumerate(
        fields: Vec<(String, String, Vec<Value>)>,
    ) -> Result<Vec<Self>, ReferenceRefusal> {
        let mut seen = std::collections::BTreeSet::new();
        let mut valuation_count = 1_usize;
        for (object, field, values) in &fields {
            if !seen.insert((object.clone(), field.clone())) {
                return Err(unsupported("duplicate finite-state field domain"));
            }
            if values.is_empty() || values.iter().any(|value| !finite_state_value(value)) {
                return Err(unsupported("empty or non-finite field domain"));
            }
            valuation_count = valuation_count
                .checked_mul(values.len())
                .filter(|count| *count <= MAX_FINITE_STATES)
                .ok_or_else(|| unsupported("finite state space exceeds 2^16 valuations"))?;
        }
        let mut states = vec![Self::default()];
        for (object, field, values) in fields {
            let mut expanded = Vec::with_capacity(valuation_count);
            for state in states {
                for value in &values {
                    let mut next = state.clone();
                    next.fields
                        .insert((object.clone(), field.clone()), value.clone());
                    expanded.push(next);
                }
            }
            states = expanded;
        }
        Ok(states)
    }
}

/// Interprets a checked Boolean expression over explicit current and pre-state
/// values. The admitted expression fragment is literals, locals, field reads,
/// integer comparisons, equality, conjunction, negation, and integer addition
/// or subtraction by a literal. Every other checked node refuses by name.
#[implements("FR-1051-AC-17")]
pub fn evaluate_boolean(
    graph: &CheckedGraph,
    expression: CheckedNode<'_>,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    current_state: &ReferenceFiniteState,
) -> Result<bool, ReferenceRefusal> {
    match evaluate_finite_value(graph, expression, locals, pre_state, current_state)? {
        Value::Boolean(value) => Ok(value),
        _ => Err(ReferenceRefusal {
            construct: "checked claim expression is not Boolean".to_owned(),
        }),
    }
}

/// Evaluates one finite scalar value from the checked graph.
#[implements("FR-1051-AC-17")]
pub fn evaluate_finite_value(
    graph: &CheckedGraph,
    expression: CheckedNode<'_>,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    current_state: &ReferenceFiniteState,
) -> Result<Value, ReferenceRefusal> {
    let mut budget = MAX_EXPRESSION_NODES;
    let value = evaluate_value(
        graph,
        expression,
        locals,
        pre_state,
        current_state,
        0,
        &mut budget,
    )?;
    if finite_scalar(&value) {
        Ok(value)
    } else {
        Err(unsupported(
            "checked expression produced a non-finite value",
        ))
    }
}

fn evaluate_value(
    graph: &CheckedGraph,
    expression: CheckedNode<'_>,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    active_state: &ReferenceFiniteState,
    depth: usize,
    budget: &mut usize,
) -> Result<Value, ReferenceRefusal> {
    if depth > MAX_EXPRESSION_DEPTH {
        return Err(ReferenceRefusal {
            construct: "checked expression exceeds the reference depth bound".to_owned(),
        });
    }
    *budget = budget.checked_sub(1).ok_or_else(|| ReferenceRefusal {
        construct: "checked expression exceeds the reference node bound".to_owned(),
    })?;
    let child = |node, state: &ReferenceFiniteState, budget: &mut usize| {
        evaluate_value(
            graph,
            expression.at(node),
            locals,
            pre_state,
            state,
            depth + 1,
            budget,
        )
    };
    match expression.kind() {
        NodeKind::Literal(literal) => match &literal.0 {
            Value::Boolean(_) | Value::Integer(_) | Value::Rational(_) | Value::Enum(_) => {
                Ok(literal.0.clone())
            }
            _ => Err(unsupported("non-finite literal value")),
        },
        NodeKind::Coerce(operand, interval) => {
            let value = child(*operand, active_state, budget)?;
            let Value::Integer(integer) = &value else {
                return Err(unsupported("non-integer finite coercion"));
            };
            if integer < interval.lower() || integer > interval.upper() {
                return Err(unsupported("integer outside checked finite interval"));
            }
            Ok(value)
        }
        NodeKind::Local(slot) => locals
            .get(*slot)
            .cloned()
            .ok_or_else(|| unsupported("local slot outside checked frame")),
        NodeKind::Attribute {
            reference, field, ..
        } => {
            let reference = child(*reference, active_state, budget)?;
            let Value::Reference(reference) = reference else {
                return Err(unsupported("attribute read without an object reference"));
            };
            active_state
                .field(reference.object().as_str(), &field.name)
                .cloned()
                .ok_or_else(|| unsupported("field absent from finite state"))
        }
        NodeKind::Order(operator, kind, left, right) => {
            let left = child(*left, active_state, budget)?;
            let right = child(*right, active_state, budget)?;
            let ordering = match (kind, &left, &right) {
                (OrderedKind::Integers, Value::Integer(left), Value::Integer(right)) => {
                    left.cmp(right)
                }
                (OrderedKind::Rationals, Value::Rational(left), Value::Rational(right)) => {
                    left.cmp(right)
                }
                (OrderedKind::Enums, Value::Enum(left), Value::Enum(right)) => {
                    left.rank().cmp(&right.rank())
                }
                _ => return Err(unsupported("non-finite ordered comparison")),
            };
            Ok(Value::Boolean(operator.holds(ordering)))
        }
        NodeKind::Equality(operator, _, left, right) => {
            let left = child(*left, active_state, budget)?;
            let right = child(*right, active_state, budget)?;
            if !finite_scalar(&left) || !finite_scalar(&right) {
                return Err(unsupported("equality outside finite scalar values"));
            }
            if std::mem::discriminant(&left) != std::mem::discriminant(&right) {
                return Err(unsupported("equality across incompatible scalar types"));
            }
            let equal = finite_scalar_equal(&left, &right);
            Ok(Value::Boolean(match operator {
                filament_semantic_value::declaration::EqualityOperator::Equal => equal,
                filament_semantic_value::declaration::EqualityOperator::NotEqual => !equal,
            }))
        }
        NodeKind::Connective(Connective::And, _, left, right) => {
            let left = child(*left, active_state, budget)?;
            let Value::Boolean(left) = left else {
                return Err(unsupported("non-Boolean conjunction operand"));
            };
            let right = child(*right, active_state, budget)?;
            let Value::Boolean(right) = right else {
                return Err(unsupported("non-Boolean conjunction operand"));
            };
            Ok(Value::Boolean(left && right))
        }
        NodeKind::Connective(Connective::Or, ..) => Err(unsupported("disjunction")),
        NodeKind::Connective(Connective::Implies, ..) => Err(unsupported("implication")),
        NodeKind::Not(operand) => {
            let value = child(*operand, active_state, budget)?;
            let Value::Boolean(value) = value else {
                return Err(unsupported("non-Boolean negation operand"));
            };
            Ok(Value::Boolean(!value))
        }
        NodeKind::Arithmetic(Arithmetic::Add, left, right)
        | NodeKind::Arithmetic(Arithmetic::Subtract, left, right) => {
            let is_subtract = matches!(
                expression.kind(),
                NodeKind::Arithmetic(Arithmetic::Subtract, ..)
            );
            let left_is_literal = matches!(
                expression.at(*left).kind(),
                NodeKind::Literal(literal) if matches!(literal.0, Value::Integer(_))
            );
            let right_is_literal = matches!(
                expression.at(*right).kind(),
                NodeKind::Literal(literal) if matches!(literal.0, Value::Integer(_))
            );
            if left_is_literal == right_is_literal {
                return Err(unsupported("non-linear integer term"));
            }
            if is_subtract && left_is_literal {
                return Err(unsupported("literal-minus-state subtraction"));
            }
            let left_value = child(*left, active_state, budget)?;
            let right_value = child(*right, active_state, budget)?;
            match (left_value, right_value) {
                (Value::Integer(left), Value::Integer(right)) => {
                    Ok(Value::Integer(if is_subtract {
                        left.sub(&right)
                    } else {
                        left.add(&right)
                    }))
                }
                _ => Err(unsupported("integer arithmetic on non-integer values")),
            }
        }
        NodeKind::Pre(operand) => child(*operand, pre_state, budget),
        kind => Err(unsupported(node_name(kind))),
    }
}

fn finite_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Boolean(_) | Value::Integer(_) | Value::Rational(_) | Value::Enum(_)
    )
}

fn finite_state_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Boolean(_) | Value::Integer(_) | Value::Enum(_)
    )
}

fn node_name<Child, Literal>(kind: &NodeKind<Child, Literal>) -> &'static str {
    match kind {
        NodeKind::Literal(_) => "literal",
        NodeKind::ReferenceLiteral { .. } => "reference literal",
        NodeKind::Coerce(..) => "integer coercion",
        NodeKind::Local(_) => "local read",
        NodeKind::Let { .. } => "let binding",
        NodeKind::If { .. } => "conditional expression",
        NodeKind::Arithmetic(Arithmetic::Multiply, ..) => "multiplication",
        NodeKind::Arithmetic(..) => "integer arithmetic",
        NodeKind::IntegerDivision { .. } => "integer division",
        NodeKind::Negate(_) => "integer negation",
        NodeKind::Divide { .. } => "rational division",
        NodeKind::Rational { .. } | NodeKind::RationalNegate(..) => "rational arithmetic",
        NodeKind::Decimal { .. } | NodeKind::DecimalNegate(..) => "decimal arithmetic",
        NodeKind::Ieee(..)
        | NodeKind::IeeeSquareRoot(..)
        | NodeKind::IeeeFloat32Fma(..)
        | NodeKind::IeeeFloat64Fma(..) => "floating-point operation",
        NodeKind::Quantity(..)
        | NodeKind::QuantityScale { .. }
        | NodeKind::QuantityScaleDiv { .. }
        | NodeKind::QuantityNegate(..)
        | NodeKind::QuantityPower(..) => "quantity operation",
        NodeKind::ConvertDecimal(..)
        | NodeKind::ConvertScalar(..)
        | NodeKind::ConvertText(..)
        | NodeKind::IeeeToRational(..) => "numeric conversion",
        NodeKind::Order(..) => "ordered comparison",
        NodeKind::Equality(..) | NodeKind::IeeeNumericEquality(..) => "equality comparison",
        NodeKind::IeeeTotalOrder(..) => "floating-point ordering",
        NodeKind::IeeeBitIdentical(..) => "floating-point identity comparison",
        NodeKind::Connective(Connective::And, ..) => "conjunction",
        NodeKind::Connective(Connective::Or, ..) => "disjunction",
        NodeKind::Connective(Connective::Implies, ..) => "implication",
        NodeKind::Not(_) => "negation",
        NodeKind::Field { .. } => "record field read",
        NodeKind::Attribute { .. } => "model field read",
        NodeKind::TupleProject { .. } => "tuple projection",
        NodeKind::Index { .. } => "collection indexing",
        NodeKind::Navigate { .. } => "relationship navigation",
        NodeKind::Present(_) | NodeKind::Value(_) | NodeKind::OptionSome(_) => "optional value",
        NodeKind::PlaceReference { .. }
        | NodeKind::PlacePresence { .. }
        | NodeKind::SlotPresence { .. } => "clock place access",
        NodeKind::Call { .. } => "called function",
        NodeKind::ImportedCall { .. } => "imported function call",
        NodeKind::Tuple { .. } | NodeKind::Record { .. } | NodeKind::UnionValue { .. } => {
            "composite construction"
        }
        NodeKind::Collection { .. } | NodeKind::ConvertCollection { .. } => "collection operation",
        NodeKind::Query { .. }
        | NodeKind::Fold { .. }
        | NodeKind::Flatten(_)
        | NodeKind::Size(_) => "collection query",
        NodeKind::Contains(..) => "collection membership",
        NodeKind::BytesLength(_) | NodeKind::BytesConcat(..) => "byte operation",
        NodeKind::DropFirst(_) | NodeKind::RemoveOne(..) => "collection mutation",
        NodeKind::AllInstances { .. }
        | NodeKind::NamedPopulation { .. }
        | NodeKind::PopulationObjects { .. }
        | NodeKind::Lookup { .. } => "population operation",
        NodeKind::Pre(_) => "pre-state expression",
        NodeKind::Reaches { .. } | NodeKind::ReachesEnd { .. } => "reachability predicate",
        NodeKind::ProtocolReference { .. } => "protocol reference",
        NodeKind::Dispatch { .. } => "dispatched function call",
        NodeKind::Case { .. } => "union case selection",
        NodeKind::History { .. } => "history read",
        NodeKind::Seq { .. } => "sequence construction",
    }
}

fn finite_scalar_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Boolean(left), Value::Boolean(right)) => left == right,
        (Value::Integer(left), Value::Integer(right)) => left == right,
        (Value::Rational(left), Value::Rational(right)) => left == right,
        (Value::Enum(left), Value::Enum(right)) => left == right,
        _ => false,
    }
}

fn unsupported(construct: impl Into<String>) -> ReferenceRefusal {
    ReferenceRefusal {
        construct: construct.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{probability_weight, ReferenceFiniteState, ReferenceProbabilityWeight};
    use filament_exact::{Integer, Rational, Value};
    use ix_trace_rs::trace;

    #[trace("FR-1051-AC-17")]
    #[test]
    fn finite_boolean_fields_enumerate_the_complete_product() {
        let states = ReferenceFiniteState::enumerate(vec![
            (
                "a".to_owned(),
                "left".to_owned(),
                vec![Value::Boolean(false), Value::Boolean(true)],
            ),
            (
                "a".to_owned(),
                "right".to_owned(),
                vec![
                    Value::Integer(Integer::from(0_i64)),
                    Value::Integer(Integer::from(1_i64)),
                ],
            ),
        ])
        .expect("the four finite valuations fit the checked state cap");
        assert_eq!(states.len(), 4);
        assert!(ReferenceFiniteState::enumerate(vec![(
            "a".to_owned(),
            "large".to_owned(),
            (0..=65_536)
                .map(|value| Value::Integer(Integer::from(value as i64)))
                .collect(),
        )])
        .is_err());
        assert!(ReferenceFiniteState::enumerate(vec![(
            "a".to_owned(),
            "rational".to_owned(),
            vec![Value::Rational(Rational::from_integer(Integer::one()))],
        )])
        .is_err());
    }

    #[trace("FR-1051-AC-19")]
    #[test]
    fn refuses_a_non_literal_probability_weight_without_a_value() {
        let refusal = probability_weight(&ReferenceProbabilityWeight::Expression(
            "self.weight".to_owned(),
        ))
        .expect_err("state-dependent transition weights are outside the fragment");
        assert_eq!(refusal.construct, "non-literal probability weight");
    }
}
