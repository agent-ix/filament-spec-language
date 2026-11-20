// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-197: exact backward induction for the bounded acyclic product.

use std::collections::{BTreeSet, VecDeque};

use filament_exact::{Integer, Rational};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::Zero as _;

use super::rows::Evaluation;
use super::unbounded::{
    Graph, UnboundedError, UnboundedObjective, maximal_end_components_from,
    policy_iteration,
};
use super::{
    ExactProbLimits, MonitorVerdict, ProbEdge, ProbProduct, ProbStateId, ProductAction, ProductKind,
};
use fsl_semantics::check::Bound;

/// The scheduler objective for a bounded claim over every scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FhObjective {
    /// Choose the least expected value at every state.
    Min,
    /// Choose the greatest expected value at every state.
    Max,
}

/// An integer multiple of `2^-exponent` (FR-197 AR-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dyadic {
    /// The signed integer multiple.
    pub mantissa: BigInt,
    /// The power-of-two denominator.
    pub exponent: u32,
}

impl Dyadic {
    /// The exact rational denoted by this dyadic endpoint.
    pub fn rational(&self) -> Rational {
        Rational::from_integer(Integer::from_big(self.mantissa.clone()))
            .divided_by_power_of_two(u64::from(self.exponent))
    }
}

/// A finite-horizon value vector and its memoryless deterministic policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FhResult {
    /// Exact rational values, one per product state.
    Exact {
        /// Values in product discovery order.
        values: Vec<Rational>,
        /// The selected action-group index for each state, when it has a
        /// choice, then the row choice selected at each row of each
        /// `FreeDelays` state (FR-204; [`super::free_delays_choice`]).
        policy: Vec<Option<usize>>,
    },
    /// Outward-rounded values after the exact rational budget was exceeded.
    Bounds {
        /// Lower bounds in product discovery order.
        lower: Vec<Dyadic>,
        /// Upper bounds in product discovery order.
        upper: Vec<Dyadic>,
        /// The precision used.
        precision_bits: u32,
        /// The selected action-group index for each state, when it has a
        /// choice, then the row choice selected at each row of each
        /// `FreeDelays` state (FR-204; [`super::free_delays_choice`]).
        policy: Vec<Option<usize>>,
    },
    /// The initial value still straddles the threshold at maximum precision.
    PrecisionBudget {
        /// The product initial state whose interval straddles the threshold.
        initial: ProbStateId,
        /// The lower endpoint.
        lower: Dyadic,
        /// The upper endpoint.
        upper: Dyadic,
    },
}

/// Why the graph cannot be evaluated as a finite-horizon product.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FhError {
    /// A product state or edge refers outside the state vector.
    #[error("the probabilistic product contains an invalid state reference")]
    InvalidState,
    /// An undecided state has no outgoing action.
    #[error("an undecided probabilistic product state has no action")]
    MissingAction,
    /// A product edge has a negative probability or a choice does not sum to 1.
    #[error("a probabilistic product choice has invalid probability mass")]
    InvalidProbability,
    /// The purported finite-horizon product contains a cycle.
    #[error("the probabilistic product is cyclic")]
    Cycle,
    /// Precision limits are inconsistent or cannot be represented.
    #[error("the precision limits are invalid")]
    InvalidPrecision,
}

/// Decide a bounded formula event on an already-built product (FR-197).
///
/// The terminal values come from FR-196's decided monitor states. Under every
/// scheduler, a lower probability bound selects the minimum and an upper
/// probability bound selects the maximum; a workload product has one action.
///
/// # Errors
/// Returns [`FhError`] when the product is malformed or its graph is cyclic.
pub fn decide_bounded_event(
    product: &ProbProduct,
    bound: &Bound,
    limits: &ExactProbLimits,
) -> Result<FhResult, FhError> {
    let mut terminal = vec![None; product.states.len()];
    for (state, verdict) in &product.decided {
        let value = match verdict {
            MonitorVerdict::Accept => rational_one(),
            MonitorVerdict::Reject => rational_zero(),
        };
        *terminal
            .get_mut(state.index())
            .ok_or(FhError::InvalidState)? = Some(value);
    }
    let (objective, threshold) = match bound {
        Bound::AtLeast(threshold) => (FhObjective::Min, threshold),
        Bound::AtMost(threshold) => (FhObjective::Max, threshold),
    };
    backward_induction_with_policy_limit(
        product,
        &terminal,
        objective,
        threshold,
        limits.max_rational_bits,
        limits.precision_bits,
        limits.max_precision_bits,
        limits.max_policy_iterations,
    )
}

/// Evaluate a finite-horizon product in exact rationals first, then in
/// outward-rounded dyadic intervals if the rational-size budget is exceeded.
///
/// `terminal` contains a value for each decided state and `None` for each
/// undecided state. `threshold` is used only to decide whether precision must
/// be doubled; exact values are returned regardless of which side they fall on.
/// Product edge order is the canonical tie-break order for the witness policy.
///
/// A `FreeDelays` state is decided by FR-204's row-factored operator: the
/// values are the product's states', and the policy's entries after them
/// select each row's choice ([`super::free_delays_choice`]).
///
/// # Errors
/// Returns [`FhError`] for malformed or cyclic products, or inconsistent
/// precision limits.
pub fn backward_induction(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    threshold: &Rational,
    max_rational_bits: u64,
    precision_bits: u32,
    max_precision_bits: u32,
) -> Result<FhResult, FhError> {
    backward_induction_with_policy_limit(
        product,
        terminal,
        objective,
        threshold,
        max_rational_bits,
        precision_bits,
        max_precision_bits,
        u64::MAX,
    )
}

fn backward_induction_with_policy_limit(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    threshold: &Rational,
    max_rational_bits: u64,
    precision_bits: u32,
    max_precision_bits: u32,
    max_policy_iterations: u64,
) -> Result<FhResult, FhError> {
    if terminal.len() != product.states.len()
        || precision_bits == 0
        || max_precision_bits < precision_bits
    {
        return Err(FhError::InvalidPrecision);
    }
    let evaluation = Evaluation::of(product).map_err(|_| FhError::InvalidProbability)?;
    let terminal = evaluation.extend(terminal).ok_or(FhError::InvalidState)?;
    if let Some(result) =
        collapse_zero_time_components(
            &evaluation.product,
            &terminal,
            objective,
            max_rational_bits,
            max_policy_iterations,
        )?
    {
        return Ok(FhResult::Exact {
            values: evaluation.restrict(result.0),
            policy: result.1,
        });
    }
    let result = induction(
        &evaluation.product,
        &terminal,
        objective,
        threshold,
        max_rational_bits,
        (precision_bits, max_precision_bits),
    )?;
    Ok(match result {
        FhResult::Exact { values, policy } => FhResult::Exact {
            values: evaluation.restrict(values),
            policy,
        },
        FhResult::Bounds {
            lower,
            upper,
            precision_bits,
            policy,
        } => FhResult::Bounds {
            lower: evaluation.restrict(lower),
            upper: evaluation.restrict(upper),
            precision_bits,
            policy,
        },
        budget @ FhResult::PrecisionBudget { .. } => budget,
    })
}

/// Evaluate zero-time end components before ordinary backward induction.
/// Every component state has the same value: the optimal exact expectation of
/// an exit action after the scheduler uses internal zero-time actions to reach
/// its chosen exit. A component without an exit is a terminal time-lock.
fn collapse_zero_time_components(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    max_rational_bits: u64,
    max_policy_iterations: u64,
) -> Result<Option<(Vec<Rational>, Vec<Option<usize>>)>, FhError> {
    let graph = Graph::new(product).map_err(|error| match error {
        UnboundedError::InvalidState => FhError::InvalidState,
        _ => FhError::InvalidProbability,
    })?;
    let zero_choices = graph
        .choices
        .iter()
        .map(|choices| {
            choices
                .iter()
                .filter(|choice| choice.edges.iter().all(|edge| edge.elapsed.is_zero()))
                .cloned()
                .collect()
        })
        .collect();
    let zero_graph = Graph {
        product,
        choices: zero_choices,
    };
    let components = maximal_end_components_from(
        &zero_graph,
        (0..product.states.len()).collect::<BTreeSet<_>>(),
    )
    .map_err(|_| FhError::InvalidState)?;
    if components.is_empty() {
        if !has_cycle(&zero_graph)? {
            return Ok(None);
        }
        let graph = Graph::new(product).map_err(|error| match error {
            UnboundedError::InvalidState => FhError::InvalidState,
            _ => FhError::InvalidProbability,
        })?;
        let policy = graph
            .choices
            .iter()
            .map(|choices| (!choices.is_empty()).then_some(0))
            .collect();
        let objective = match objective {
            FhObjective::Min => UnboundedObjective::Min,
            FhObjective::Max => UnboundedObjective::Max,
        };
        let (values, policy, _) = policy_iteration(
            &graph,
            terminal,
            objective,
            max_policy_iterations,
            policy,
            max_rational_bits,
        )
        .map_err(|error| match error {
            UnboundedError::InvalidState => FhError::InvalidState,
            UnboundedError::MissingAction => FhError::MissingAction,
            UnboundedError::InvalidProbability | UnboundedError::MaxRationalBits => {
                FhError::InvalidProbability
            }
            UnboundedError::Singular
            | UnboundedError::MaxPolicyIterations
            | UnboundedError::MissingReward
            | UnboundedError::ZeroWeightComponent => FhError::Cycle,
        })?;
        return Ok(Some((values, policy)));
    }
    let mut solver = ZeroTimeSolver {
        product,
        terminal,
        objective,
        max_rational_bits,
        component_of: vec![None; product.states.len()],
        components,
        values: vec![None; product.states.len()],
        policy: vec![None; product.states.len()],
        visiting: vec![false; product.states.len()],
        solved_components: vec![false; product.states.len()],
    };
    for (component, states) in solver.components.iter().enumerate() {
        for state in states {
            let Some(slot) = solver.component_of.get_mut(*state) else {
                return Err(FhError::InvalidState);
            };
            *slot = Some(component);
        }
    }
    for state in 0..product.states.len() {
        solver.value(state)?;
    }
    let values = solver
        .values
        .into_iter()
        .map(|value| value.ok_or(FhError::InvalidState))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some((values, solver.policy)))
}

fn has_cycle(graph: &Graph<'_>) -> Result<bool, FhError> {
    let count = graph.product.states.len();
    let mut indegree = vec![0_usize; count];
    for choices in &graph.choices {
        for choice in choices {
            for edge in &choice.edges {
                let degree = indegree.get_mut(edge.to.index()).ok_or(FhError::InvalidState)?;
                *degree = degree.checked_add(1).ok_or(FhError::InvalidState)?;
            }
        }
    }
    let mut ready = VecDeque::new();
    for (state, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(state);
        }
    }
    let mut visited = 0_usize;
    while let Some(state) = ready.pop_front() {
        visited = visited.checked_add(1).ok_or(FhError::InvalidState)?;
        for choice in graph.choices.get(state).ok_or(FhError::InvalidState)? {
            for edge in &choice.edges {
                let degree = indegree.get_mut(edge.to.index()).ok_or(FhError::InvalidState)?;
                *degree = degree.checked_sub(1).ok_or(FhError::InvalidState)?;
                if *degree == 0 {
                    ready.push_back(edge.to.index());
                }
            }
        }
    }
    Ok(visited != count)
}

struct ZeroTimeSolver<'p> {
    product: &'p ProbProduct,
    terminal: &'p [Option<Rational>],
    objective: FhObjective,
    max_rational_bits: u64,
    component_of: Vec<Option<usize>>,
    components: Vec<Vec<usize>>,
    values: Vec<Option<Rational>>,
    policy: Vec<Option<usize>>,
    visiting: Vec<bool>,
    solved_components: Vec<bool>,
}

impl ZeroTimeSolver<'_> {
    fn value(&mut self, state: usize) -> Result<Rational, FhError> {
        if let Some(value) = self.values.get(state).and_then(Clone::clone) {
            return Ok(value);
        }
        if self.visiting.get(state).copied().unwrap_or(true) {
            return Err(FhError::Cycle);
        }
        let Some(component) = self.component_of.get(state).copied().flatten() else {
            return self.ordinary_value(state);
        };
        self.component_value(component)?;
        self.values
            .get(state)
            .and_then(Clone::clone)
            .ok_or(FhError::InvalidState)
    }

    fn ordinary_value(&mut self, state: usize) -> Result<Rational, FhError> {
        if let Some(value) = self.values.get(state).and_then(Clone::clone) {
            return Ok(value);
        }
        if self.visiting.get(state).copied().unwrap_or(true) {
            return Err(FhError::Cycle);
        }
        if let Some(slot) = self.visiting.get_mut(state) {
            *slot = true;
        }
        let value = if let Some(value) = self.terminal.get(state).cloned().flatten() {
            value
        } else {
            let state_id = ProbStateId(state);
            let groups = action_groups(self.product, state_id)?;
            if groups.is_empty() {
                return Err(FhError::MissingAction);
            }
            let mut candidates = Vec::with_capacity(groups.len());
            for (choice_index, edges) in groups.iter().enumerate() {
                let value = self.expectation(edges)?;
                candidates.push((choice_index, value));
            }
            let (choice, value) = self.extreme(candidates).ok_or(FhError::InvalidState)?;
            if let Some(slot) = self.policy.get_mut(state) {
                *slot = Some(choice);
            }
            value
        };
        if !rational_fits(&value, self.max_rational_bits) {
            return Err(FhError::InvalidProbability);
        }
        if let Some(slot) = self.values.get_mut(state) {
            *slot = Some(value.clone());
        }
        if let Some(slot) = self.visiting.get_mut(state) {
            *slot = false;
        }
        Ok(value)
    }

    fn component_value(&mut self, component: usize) -> Result<(), FhError> {
        if self
            .solved_components
            .get(component)
            .copied()
            .unwrap_or(false)
        {
            return Ok(());
        }
        let members = self
            .components
            .get(component)
            .cloned()
            .ok_or(FhError::InvalidState)?;
        let member_set = members.iter().copied().collect::<BTreeSet<_>>();
        for state in &members {
            if let Some(slot) = self.visiting.get_mut(*state) {
                *slot = true;
            }
        }
        let mut exits = Vec::new();
        for state in &members {
            let groups = action_groups(self.product, ProbStateId(*state))?;
            for (choice_index, edges) in groups.iter().enumerate() {
                let internal = edges.iter().all(|edge| {
                    edge.probability.is_zero()
                        || (edge.elapsed.is_zero() && member_set.contains(&edge.to.index()))
                });
                if internal {
                    continue;
                }
                let (exit_value, internal_mass) = self.exit_expectation(edges, &member_set)?;
                let denominator = rational_one().sub(&internal_mass);
                if denominator.is_zero() {
                    continue;
                }
                let value = exit_value.div(&denominator).ok_or(FhError::InvalidProbability)?;
                if !rational_fits(&value, self.max_rational_bits) {
                    return Err(FhError::InvalidProbability);
                }
                exits.push((*state, choice_index, value));
            }
        }
        let best = match self.objective {
            FhObjective::Min => exits.iter().min_by(|left, right| left.2.cmp(&right.2)),
            FhObjective::Max => exits.iter().max_by(|left, right| left.2.cmp(&right.2)),
        }
        .cloned();
        let value = best
            .as_ref()
            .map(|exit| exit.2.clone())
            .unwrap_or_else(rational_zero);
        for state in &members {
            if let Some(slot) = self.values.get_mut(*state) {
                *slot = Some(value.clone());
            }
            if let Some(slot) = self.visiting.get_mut(*state) {
                *slot = false;
            }
        }
        if let Some((exit_state, exit_choice, _)) = best {
            if let Some(slot) = self.policy.get_mut(exit_state) {
                *slot = Some(exit_choice);
            }
            expand_zero_time_component_policy(
                self.product,
                &members,
                exit_state,
                &mut self.policy,
            )?;
        }
        if let Some(slot) = self.solved_components.get_mut(component) {
            *slot = true;
        }
        Ok(())
    }

    fn exit_expectation(
        &mut self,
        edges: &[&super::ProbEdge],
        members: &BTreeSet<usize>,
    ) -> Result<(Rational, Rational), FhError> {
        let mut outside = rational_zero();
        let mut internal = rational_zero();
        for edge in edges {
            if edge.probability.is_zero() {
                continue;
            }
            if members.contains(&edge.to.index()) {
                internal = internal.add(&edge.probability);
            } else {
                let value = self.value(edge.to.index())?;
                outside = outside.add(&edge.probability.mul(&value));
            }
        }
        Ok((outside, internal))
    }

    fn expectation(&mut self, edges: &[&super::ProbEdge]) -> Result<Rational, FhError> {
        let mut mass = rational_zero();
        let mut value = rational_zero();
        for edge in edges {
            mass = mass.add(&edge.probability);
            value = value.add(&edge.probability.mul(&self.value(edge.to.index())?));
        }
        if mass != rational_one() {
            return Err(FhError::InvalidProbability);
        }
        Ok(value)
    }

    fn extreme(&self, candidates: Vec<(usize, Rational)>) -> Option<(usize, Rational)> {
        match self.objective {
            FhObjective::Min => candidates
                .into_iter()
                .min_by(|left, right| left.1.cmp(&right.1)),
            FhObjective::Max => candidates
                .into_iter()
                .max_by(|left, right| left.1.cmp(&right.1)),
        }
    }
}

/// Expand the selected MEC exit into a proper policy: every other member
/// follows the first canonical internal action on a shortest path to the
/// exit owner (FR-198/ADR-028 UR-7).
fn expand_zero_time_component_policy(
    product: &ProbProduct,
    component: &[usize],
    owner: usize,
    policy: &mut [Option<usize>],
) -> Result<(), FhError> {
    let members = component.iter().copied().collect::<BTreeSet<_>>();
    let mut distance = vec![None; product.states.len()];
    *distance.get_mut(owner).ok_or(FhError::InvalidState)? = Some(0_usize);
    loop {
        let mut changed = false;
        for state in component {
            if distance.get(*state).copied().flatten().is_some() {
                continue;
            }
            let groups = action_groups(product, ProbStateId(*state))?;
            let nearest = groups
                .iter()
                .filter(|edges| {
                    edges.iter().all(|edge| {
                        edge.probability.is_zero()
                            || (edge.elapsed.is_zero() && members.contains(&edge.to.index()))
                    })
                })
                .flat_map(|edges| {
                    edges
                        .iter()
                        .filter(|edge| !edge.probability.is_zero())
                        .map(|edge| edge.to.index())
                })
                .filter_map(|target| distance.get(target).copied().flatten().map(|d| d + 1))
                .min();
            if let Some(nearest) = nearest {
                *distance.get_mut(*state).ok_or(FhError::InvalidState)? = Some(nearest);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for state in component {
        if *state == owner {
            continue;
        }
        let current = distance
            .get(*state)
            .copied()
            .flatten()
            .ok_or(FhError::InvalidState)?;
        let groups = action_groups(product, ProbStateId(*state))?;
        let selected = groups
            .iter()
            .position(|edges| {
                edges.iter().all(|edge| {
                    edge.probability.is_zero()
                        || (edge.elapsed.is_zero() && members.contains(&edge.to.index()))
                }) && edges.iter().any(|edge| {
                    !edge.probability.is_zero()
                        && distance
                            .get(edge.to.index())
                            .copied()
                            .flatten()
                            .is_some_and(|next| next < current)
                })
            })
            .ok_or(FhError::InvalidState)?;
        *policy.get_mut(*state).ok_or(FhError::InvalidState)? = Some(selected);
    }
    Ok(())
}

fn induction(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    threshold: &Rational,
    max_rational_bits: u64,
    (precision_bits, max_precision_bits): (u32, u32),
) -> Result<FhResult, FhError> {
    let order = topological_order(product)?;
    let exact = exact_pass(product, terminal, objective, &order, max_rational_bits)?;
    if let Some(exact) = exact {
        return Ok(FhResult::Exact {
            values: exact.values,
            policy: exact.policy,
        });
    }

    let mut bits = precision_bits;
    loop {
        let intervals = interval_pass(product, terminal, objective, &order, bits)?;
        let mut ambiguous = None;
        for (_, _, initial) in &product.initial {
            let index = initial.index();
            let lower = intervals.lower.get(index).ok_or(FhError::InvalidState)?;
            let upper = intervals.upper.get(index).ok_or(FhError::InvalidState)?;
            let lower_value = lower.rational();
            let upper_value = upper.rational();
            if lower_value <= *threshold && *threshold <= upper_value {
                ambiguous = Some((*initial, lower.clone(), upper.clone()));
                break;
            }
        }
        if let Some((initial, lower, upper)) = ambiguous {
            if bits == max_precision_bits {
                return Ok(FhResult::PrecisionBudget {
                    initial,
                    lower,
                    upper,
                });
            }
            bits = bits.saturating_mul(2).min(max_precision_bits);
            continue;
        }
        return Ok(FhResult::Bounds {
            lower: intervals.lower,
            upper: intervals.upper,
            precision_bits: bits,
            policy: intervals.policy,
        });
    }
}

struct ExactValues {
    values: Vec<Rational>,
    policy: Vec<Option<usize>>,
}

struct IntervalValues {
    lower: Vec<Dyadic>,
    upper: Vec<Dyadic>,
    policy: Vec<Option<usize>>,
}

fn exact_pass(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    order: &[ProbStateId],
    max_rational_bits: u64,
) -> Result<Option<ExactValues>, FhError> {
    let zero = rational_zero();
    let mut values = vec![zero; product.states.len()];
    let mut policy = vec![None; product.states.len()];
    for state in order.iter().rev().copied() {
        let index = state.index();
        if let Some(value) = &terminal[index] {
            if !rational_fits(value, max_rational_bits) {
                return Ok(None);
            }
            values[index] = value.clone();
            continue;
        }
        let choices = action_groups(product, state)?;
        if choices.is_empty() {
            return Err(FhError::MissingAction);
        }
        let mut selected: Option<(usize, Rational)> = None;
        for (choice_index, edges) in choices.iter().enumerate() {
            let value = weighted_value(edges, &values)?;
            let replace = selected
                .as_ref()
                .is_none_or(|(_, current)| match objective {
                    FhObjective::Min => value < *current,
                    FhObjective::Max => value > *current,
                });
            if replace {
                selected = Some((choice_index, value));
            }
        }
        let (choice_index, value) = selected.ok_or(FhError::MissingAction)?;
        if !rational_fits(&value, max_rational_bits) {
            return Ok(None);
        }
        values[index] = value;
        policy[index] = Some(choice_index);
    }
    Ok(Some(ExactValues { values, policy }))
}

fn interval_pass(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    order: &[ProbStateId],
    bits: u32,
) -> Result<IntervalValues, FhError> {
    let zero = Dyadic {
        mantissa: BigInt::zero(),
        exponent: bits,
    };
    let mut lower = vec![zero.clone(); product.states.len()];
    let mut upper = vec![zero; product.states.len()];
    let mut policy = vec![None; product.states.len()];
    for state in order.iter().rev().copied() {
        let index = state.index();
        if let Some(value) = &terminal[index] {
            lower[index] = round(value, bits, false)?;
            upper[index] = round(value, bits, true)?;
            continue;
        }
        let choices = action_groups(product, state)?;
        if choices.is_empty() {
            return Err(FhError::MissingAction);
        }
        let mut action_values = Vec::with_capacity(choices.len());
        for edges in &choices {
            let (choice_lower, choice_upper) = weighted_interval(edges, &lower, &upper, bits)?;
            action_values.push((choice_lower, choice_upper));
        }
        let (first_lower, first_upper) = action_values.first().ok_or(FhError::MissingAction)?;
        let (mut value_lower, mut value_upper) = (first_lower.clone(), first_upper.clone());
        for (candidate_lower, candidate_upper) in action_values.iter().skip(1) {
            match objective {
                FhObjective::Min => {
                    if candidate_lower.rational() < value_lower.rational() {
                        value_lower = candidate_lower.clone();
                    }
                    if candidate_upper.rational() < value_upper.rational() {
                        value_upper = candidate_upper.clone();
                    }
                }
                FhObjective::Max => {
                    if candidate_lower.rational() > value_lower.rational() {
                        value_lower = candidate_lower.clone();
                    }
                    if candidate_upper.rational() > value_upper.rational() {
                        value_upper = candidate_upper.clone();
                    }
                }
            }
        }
        policy[index] =
            action_values.iter().enumerate().find_map(
                |(candidate, (candidate_lower, candidate_upper))| {
                    let proven_optimal = action_values.iter().enumerate().all(
                        |(other, (other_lower, other_upper))| {
                            if candidate == other {
                                return true;
                            }
                            match objective {
                                FhObjective::Min => {
                                    candidate_upper.rational() <= other_lower.rational()
                                }
                                FhObjective::Max => {
                                    candidate_lower.rational() >= other_upper.rational()
                                }
                            }
                        },
                    );
                    proven_optimal.then_some(candidate)
                },
            );
        lower[index] = value_lower;
        upper[index] = value_upper;
    }
    Ok(IntervalValues {
        lower,
        upper,
        policy,
    })
}

fn topological_order(product: &ProbProduct) -> Result<Vec<ProbStateId>, FhError> {
    let state_count = product.states.len();
    let mut indegree = vec![0_usize; state_count];
    for edge in &product.edges {
        if edge.from.index() >= state_count || edge.to.index() >= state_count {
            return Err(FhError::InvalidState);
        }
        indegree[edge.to.index()] = indegree[edge.to.index()]
            .checked_add(1)
            .ok_or(FhError::InvalidState)?;
    }
    let mut ready = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(ProbStateId(index));
        }
    }
    let mut order = Vec::with_capacity(state_count);
    while let Some(state) = ready.pop_front() {
        order.push(state);
        for edge in product.outgoing(state) {
            let degree = indegree
                .get_mut(edge.to.index())
                .ok_or(FhError::InvalidState)?;
            *degree = degree.checked_sub(1).ok_or(FhError::InvalidState)?;
            if *degree == 0 {
                ready.push_back(edge.to);
            }
        }
    }
    if order.len() != state_count {
        return Err(FhError::Cycle);
    }
    for (_, _, initial) in &product.initial {
        if initial.index() >= state_count {
            return Err(FhError::InvalidState);
        }
    }
    Ok(order)
}

fn action_groups(
    product: &ProbProduct,
    state: ProbStateId,
) -> Result<Vec<Vec<&ProbEdge>>, FhError> {
    let edges = product.outgoing(state);
    if edges.iter().any(|edge| edge.from != state) {
        return Err(FhError::InvalidState);
    }
    if product.kind == ProductKind::Dtmc {
        return if edges.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![edges.iter().collect()])
        };
    }
    let mut groups: Vec<(ProductAction, Vec<&ProbEdge>)> = Vec::new();
    for edge in edges {
        if edge.probability < rational_zero() {
            return Err(FhError::InvalidProbability);
        }
        match groups
            .iter_mut()
            .find(|(action, _)| action_equal(action, &edge.choice))
        {
            Some((_, group)) => group.push(edge),
            None => groups.push((edge.choice.clone(), vec![edge])),
        }
    }
    for (_, group) in &groups {
        validate_probability_mass(group)?;
    }
    Ok(groups.into_iter().map(|(_, edges)| edges).collect())
}

fn action_equal(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(left), ProductAction::PostState(right)) => left == right,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter)
        | (ProductAction::UnitDelay, ProductAction::UnitDelay)
        | (ProductAction::EnterIdleDelay, ProductAction::EnterIdleDelay)
        | (ProductAction::IdleDelay, ProductAction::IdleDelay)
        | (ProductAction::IdleObserve, ProductAction::IdleObserve) => true,
        (
            ProductAction::WorkloadResolve {
                identity: left,
                delay: left_delay,
            },
            ProductAction::WorkloadResolve {
                identity: right,
                delay: right_delay,
            },
        ) => left == right && left_delay == right_delay,
        (ProductAction::FreeDelays(left), ProductAction::FreeDelays(right)) => left == right,
        _ => false,
    }
}

fn weighted_value(edges: &[&ProbEdge], values: &[Rational]) -> Result<Rational, FhError> {
    validate_probability_mass(edges)?;
    edges.iter().try_fold(rational_zero(), |sum, edge| {
        let value = values.get(edge.to.index()).ok_or(FhError::InvalidState)?;
        Ok(sum.add(&edge.probability.mul(value)))
    })
}

fn weighted_interval(
    edges: &[&ProbEdge],
    lower: &[Dyadic],
    upper: &[Dyadic],
    bits: u32,
) -> Result<(Dyadic, Dyadic), FhError> {
    validate_probability_mass(edges)?;
    let scale = Rational::from_integer(Integer::one().shifted_left(u64::from(bits)));
    let (lower_sum, upper_sum) = edges.iter().try_fold(
        (rational_zero(), rational_zero()),
        |(lower_sum, upper_sum), edge| {
            let edge_lower = lower.get(edge.to.index()).ok_or(FhError::InvalidState)?;
            let edge_upper = upper.get(edge.to.index()).ok_or(FhError::InvalidState)?;
            Ok::<_, FhError>((
                lower_sum.add(&edge.probability.mul(&edge_lower.rational())),
                upper_sum.add(&edge.probability.mul(&edge_upper.rational())),
            ))
        },
    )?;
    let lower_scaled = lower_sum.mul(&scale);
    let upper_scaled = upper_sum.mul(&scale);
    Ok((
        round_scaled(&lower_scaled, bits, false)?,
        round_scaled(&upper_scaled, bits, true)?,
    ))
}

fn validate_probability_mass(edges: &[&ProbEdge]) -> Result<(), FhError> {
    let mut total = rational_zero();
    for edge in edges {
        if edge.probability < rational_zero() {
            return Err(FhError::InvalidProbability);
        }
        total = total.add(&edge.probability);
    }
    if total != rational_one() {
        return Err(FhError::InvalidProbability);
    }
    Ok(())
}

fn round(value: &Rational, bits: u32, upward: bool) -> Result<Dyadic, FhError> {
    let scale = BigInt::from(1_u8) << bits;
    let numerator = value.numerator().as_big() * &scale;
    let denominator = value.denominator().as_big();
    let mantissa = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    Ok(Dyadic {
        mantissa,
        exponent: bits,
    })
}

fn round_scaled(value: &Rational, bits: u32, upward: bool) -> Result<Dyadic, FhError> {
    let numerator = value.numerator().as_big();
    let denominator = value.denominator().as_big();
    let mantissa = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    Ok(Dyadic {
        mantissa,
        exponent: bits,
    })
}

pub(super) fn rational_fits(value: &Rational, max_bits: u64) -> bool {
    value.numerator().as_big().bits() <= max_bits && value.denominator().as_big().bits() <= max_bits
}

fn rational_zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn rational_one() -> Rational {
    Rational::from_integer(Integer::one())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("nonzero denominator")
    }

    fn product() -> ProbProduct {
        let key = |model| super::super::ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
            certificate_state: None,
        };
        ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Dtmc,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 3),
                    elapsed: Rational::from_integer(0_i64.into()),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(2, 3),
                    elapsed: Rational::from_integer(0_i64.into()),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: vec![Vec::new(); 3],
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        }
    }

    fn scheduler_product() -> ProbProduct {
        let key = |model| super::super::ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
            certificate_state: None,
        };
        let target_one = key(1);
        let target_zero = key(2);
        ProbProduct {
            states: vec![key(0), target_one.clone(), target_zero.clone()],
            kind: ProductKind::Mdp,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(target_one)),
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 1),
                    elapsed: Rational::from_integer(0_i64.into()),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(target_zero)),
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(1, 1),
                    elapsed: Rational::from_integer(0_i64.into()),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: vec![Vec::new(); 3],
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        }
    }

    #[test]
    fn zero_time_mec_collapses_to_the_objectives_best_exit() {
        let key = |model| super::super::ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
            certificate_state: None,
        };
        let zero = key(2);
        let one = key(1);
        let product = ProbProduct {
            states: vec![key(0), one.clone(), zero.clone()],
            kind: ProductKind::Mdp,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(key(0))),
                    action: None,
                    draw: None,
                    to: ProbStateId(0),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(one)),
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(zero)),
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: vec![
                (ProbStateId(1), MonitorVerdict::Accept),
                (ProbStateId(2), MonitorVerdict::Reject),
            ],
            labels: vec![Vec::new(); 3],
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let limits = ExactProbLimits::default();
        let minimum = decide_bounded_event(&product, &Bound::AtLeast(rational(1, 2)), &limits)
            .expect("zero-time MEC min exit is finite");
        let maximum = decide_bounded_event(&product, &Bound::AtMost(rational(1, 2)), &limits)
            .expect("zero-time MEC max exit is finite");
        assert!(matches!(
            minimum,
            FhResult::Exact { values, .. } if values[0] == rational(0, 1)
        ));
        assert!(matches!(
            maximum,
            FhResult::Exact { values, .. } if values[0] == rational(1, 1)
        ));
    }

    #[test]
    fn zero_time_mec_policy_takes_the_canonical_shortest_path_to_its_exit() {
        let key = |model| super::super::ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
            certificate_state: None,
        };
        let product = ProbProduct {
            states: vec![key(0), key(1), key(2), key(3)],
            kind: ProductKind::Mdp,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(key(1))),
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(1),
                    choice: ProductAction::PostState(Box::new(key(1))),
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(1),
                    choice: ProductAction::PostState(Box::new(key(2))),
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(2),
                    choice: ProductAction::PostState(Box::new(key(3))),
                    action: None,
                    draw: None,
                    to: ProbStateId(3),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: vec![Vec::new(); 4],
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let mut policy = vec![None; product.states.len()];
        policy[2] = Some(0);
        expand_zero_time_component_policy(&product, &[0, 1, 2], 2, &mut policy)
            .expect("every MEC state has a shortest route to the selected exit");
        assert_eq!(policy[0], Some(0));
        assert_eq!(policy[1], Some(1));
        assert_eq!(policy[2], Some(0));
    }

    /// Trace: TC-632
    #[trace("TC-632")]
    #[test]
    fn computes_the_exact_weighted_value() {
        let result = backward_induction(
            &product(),
            &[None, Some(rational(1, 1)), Some(rational(0, 1))],
            FhObjective::Min,
            &rational(1, 3),
            128,
            8,
            64,
        )
        .expect("valid finite product");

        let FhResult::Exact { values, .. } = result else {
            panic!("small rational result should remain exact");
        };
        assert_eq!(values[0], rational(1, 3));
    }

    /// Trace: TC-632
    #[trace("TC-632")]
    #[test]
    fn records_the_extremal_action_in_canonical_order() {
        let product = scheduler_product();
        let terminal = [None, Some(rational(1, 1)), Some(rational(0, 1))];
        let minimum = backward_induction(
            &product,
            &terminal,
            FhObjective::Min,
            &rational(1, 2),
            128,
            8,
            64,
        )
        .expect("valid finite product");
        let maximum = backward_induction(
            &product,
            &terminal,
            FhObjective::Max,
            &rational(1, 2),
            128,
            8,
            64,
        )
        .expect("valid finite product");
        assert!(matches!(minimum, FhResult::Exact { ref policy, .. } if policy[0] == Some(1)));
        assert!(matches!(maximum, FhResult::Exact { ref policy, .. } if policy[0] == Some(0)));
    }

    /// Trace: TC-632, FR-197-AC-4
    #[trace("TC-632", "FR-197-AC-4")]
    #[test]
    fn precision_budget_contains_an_exact_threshold() {
        let result = backward_induction(
            &product(),
            &[None, Some(rational(1, 1)), Some(rational(0, 1))],
            FhObjective::Min,
            &rational(1, 3),
            0,
            4,
            8,
        )
        .expect("valid finite product");

        let FhResult::PrecisionBudget { lower, upper, .. } = result else {
            panic!("exact threshold remains inside every outward interval");
        };
        assert!(lower.rational() <= rational(1, 3));
        assert!(rational(1, 3) <= upper.rational());
    }

    /// A collapsed outward interval still cannot decide equality after the
    /// exact pass exceeded its rational budget.
    #[trace("TC-632", "FR-197-AC-4")]
    #[test]
    fn collapsed_interval_at_threshold_remains_incomplete_after_rational_overflow() {
        let half = rational(1, 2);
        let result = backward_induction(
            &product(),
            &[None, Some(half.clone()), Some(half.clone())],
            FhObjective::Min,
            &half,
            1,
            1,
            1,
        )
        .expect("valid finite product");
        assert!(matches!(
            result,
            FhResult::PrecisionBudget { ref lower, ref upper, .. }
                if lower.rational() == half && upper.rational() == half
        ));
    }

    /// Trace: TC-638, FR-203-AC-2
    #[trace("TC-638", "FR-203-AC-2")]
    #[test]
    fn exact_limits_publish_the_fr203_defaults() {
        let limits = super::super::ExactProbLimits::default();
        assert_eq!(limits.max_iterations, 1_000_000);
        assert_eq!(limits.precision_bits, 64);
        assert_eq!(limits.max_precision_bits, 4_096);
        assert_eq!(limits.max_rational_bits, 65_536);
        assert_eq!(limits.max_policy_iterations, 10_000);
        assert_eq!(limits.max_witness_paths, 1_024);
    }
}
