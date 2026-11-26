// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202 subsystem counterexamples for a failing unwindowed `eventually`,
//! `until`, `always` or expected-reward claim. EN-5 selects the witness
//! scheduler (none under a workload), solves the chain it induces, writes a
//! `lower` or `upper`
//! certificate over that chain and settles `refuted` only after the core's
//! independent replay reproduces it.

use super::candidate::{CandidateBudget, rational_bytes, sort};
use super::*;
use crate::exact_probabilistic::{
    ExactProbLimits, ProbStateId, ProductAction, avoiding_policy, bottom_components, induced_chain,
    policy_action,
};
use fsl_replay::{ProbabilitySubsystemCertificate, SubsystemBoundKind};

enum Role {
    /// A failing `>=` unwindowed state event: a lower bound on its violation.
    Avoidance(Boundary),
    /// `expected ... <= θ`: a lower bound on the expected reward.
    RewardLower,
    /// `expected ... >= θ`: a proper upper bound on the expected reward.
    RewardUpper,
}

/// Values, optional ranks and scheduler of one induced-chain certificate.
struct Evidence {
    values: Vec<(usize, Rational)>,
    ranks: Option<Vec<(usize, u64)>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refute(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    start: ProbStateId,
    values: &[ExtRational],
    binding: Option<&ObjectReference>,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    budget: &mut CandidateBudget<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let label = |state: usize, at: usize| {
        product
            .labels
            .get(state)
            .and_then(|labels| labels.get(at))
            .copied()
            .unwrap_or(false)
    };
    let stutters = |state: ProbStateId| {
        product
            .outgoing(state)
            .iter()
            .all(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
    };
    let count = product.states.len();
    let (role, threshold) = match &claim.form {
        ProbForm::Reach { bound, target } => {
            let (accepting, threshold) = match bound {
                Bound::AtLeast(threshold) => (false, threshold),
                Bound::AtMost(threshold) => (true, threshold),
            };
            let boundary = match target {
                ReachTarget::Eventually(_) => {
                    let success = (0..count).map(|state| label(state, 0)).collect::<Vec<_>>();
                    Boundary {
                        stop: success.clone(),
                        success,
                        undecided_accepts: false,
                        accepting,
                    }
                }
                ReachTarget::Until(..) => {
                    let success = (0..count).map(|state| label(state, 1)).collect::<Vec<_>>();
                    let stop = product
                        .ids()
                        .map(|state| {
                            let index = state.index();
                            success[index] || !label(index, 0) || stutters(state)
                        })
                        .collect();
                    Boundary {
                        success,
                        stop,
                        undecided_accepts: false,
                        accepting,
                    }
                }
                ReachTarget::Always(_) => Boundary {
                    success: product
                        .ids()
                        .map(|state| label(state.index(), 0) && stutters(state))
                        .collect(),
                    stop: product
                        .ids()
                        .map(|state| !label(state.index(), 0) || stutters(state))
                        .collect(),
                    undecided_accepts: true,
                    accepting,
                },
            };
            (Role::Avoidance(boundary), threshold)
        }
        ProbForm::Probability {
            bound,
            event:
                Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
        } if super::is_unbounded_eventually_formula(formula) => {
            let (accepting, threshold) = match bound {
                Bound::AtLeast(threshold) => (false, threshold),
                Bound::AtMost(threshold) => (true, threshold),
            };
            let success = (0..count).map(|state| label(state, 0)).collect::<Vec<_>>();
            let boundary = Boundary {
                stop: success.clone(),
                success,
                undecided_accepts: false,
                accepting,
            };
            (Role::Avoidance(boundary), threshold)
        }
        ProbForm::ExpectedReward {
            bound: Bound::AtMost(threshold),
            ..
        } => (Role::RewardLower, threshold),
        ProbForm::ExpectedReward {
            bound: Bound::AtLeast(threshold),
            ..
        } => (Role::RewardUpper, threshold),
        _ => return not_yet(),
    };
    let target = (0..count).map(|state| label(state, 0)).collect::<Vec<_>>();
    let stop = match &role {
        Role::Avoidance(boundary) => boundary.stop.clone(),
        Role::RewardLower | Role::RewardUpper => target.clone(),
    };
    let every = matches!(claim.scheduler, ProbScheduler::Every);
    // Where the optimum is already extreme the solver's choice may be any
    // value-preserving action; keep the witness inside that region so the
    // undecided bottom component it reaches realizes the extreme. Under a
    // fairness set the caller's policy is already the checked fair one
    // (FR-200), and another value-preserving action may be unfair.
    let region = |extreme: i64| {
        let extreme = Rational::from_integer(filament_exact::Integer::from(extreme));
        values
            .iter()
            .map(|value| !matches!(value, ExtRational::Finite(value) if *value == extreme))
            .collect::<Vec<_>>()
    };
    let stopped = |stop: Stop| match stop {
        Stop::Solver(error) => unbounded_failure(error, limits, subject),
        Stop::Document(error) => document_failure(error, subject),
    };
    let selected = match &role {
        Role::Avoidance(boundary)
            if every
                && claim.fairness.is_empty()
                && boundary.accepting == boundary.undecided_accepts =>
        {
            avoiding_policy(
                product,
                &region(i64::from(boundary.accepting)),
                policy,
                false,
                &mut || step(budget).map_err(Stop::from),
            )
        }
        Role::RewardLower if every => avoiding_policy(product, &target, policy, true, &mut || {
            step(budget).map_err(Stop::from)
        }),
        _ => Ok(policy.to_vec()),
    };
    let selected = match selected {
        Ok(selected) => selected,
        Err(stop) => return stopped(stop),
    };
    let chain = induced_chain(product, &selected);
    let reached = match reached(&chain, start, &stop, budget) {
        Ok(reached) => reached,
        Err(error) => return document_failure(error, subject),
    };
    let evidence = match &role {
        Role::Avoidance(boundary) => avoidance(
            &chain, start, boundary, &reached, threshold, limits, cancel, budget,
        ),
        Role::RewardLower | Role::RewardUpper => {
            let ProbForm::ExpectedReward { reward, .. } = &claim.form else {
                return broken();
            };
            reward_bound(
                &chain,
                start,
                &target,
                &reached,
                reward.as_str(),
                threshold,
                matches!(role, Role::RewardUpper),
                limits,
                cancel,
                budget,
            )
        }
    };
    let evidence = match evidence {
        Ok(evidence) => evidence,
        // The claim was refuted (this route runs only for a bound the solved
        // values violate) yet the selected chain does not refute it: the
        // solver and the chain disagree, the engine's own fault.
        Err(stop) => return stopped(stop),
    };
    let evidence = match require_witness(evidence) {
        Ok(evidence) => evidence,
        Err(report) => return *report,
    };
    fn state_key(
        product: &ProbProduct,
        binding: Option<&ObjectReference>,
        budget: &mut CandidateBudget<'_>,
        state: usize,
    ) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
        let model = &product
            .states
            .get(state)
            .ok_or(ProbabilityDocumentError::Malformed)?
            .model;
        budget.key(binding, model)
    }
    let key =
        |budget: &mut CandidateBudget<'_>, state: usize| state_key(product, binding, budget, state);
    let mut values = Vec::new();
    for (state, value) in &evidence.values {
        if let Err(error) = budget.reserve(rational_bytes(value)) {
            return document_failure(error, subject);
        }
        match key(budget, *state) {
            Ok(state) => values.push(ExactProbabilityValue {
                state,
                value: value.clone(),
            }),
            Err(error) => return document_failure(error, subject),
        }
    }
    if let Err(error) = sort(&mut values, |row| &row.state, &mut budget.meter, cancel) {
        return document_failure(error, subject);
    }
    let ranking = match &evidence.ranks {
        None => None,
        Some(ranks) => {
            let mut rows = Vec::new();
            for (state, rank) in ranks {
                match key(budget, *state) {
                    Ok(state) => rows.push((state, *rank)),
                    Err(error) => return document_failure(error, subject),
                }
            }
            if let Err(error) = sort(&mut rows, |row| &row.0, &mut budget.meter, cancel) {
                return document_failure(error, subject);
            }
            Some(rows)
        }
    };
    let mut scheduler = Vec::new();
    let mut spelled = Vec::new();
    if every {
        for state in product.ids() {
            if !reached[state.index()] || stop[state.index()] {
                continue;
            }
            let Some(action) = policy_action(product, state, &selected) else {
                continue;
            };
            let ProductAction::Identity(identity) = &action else {
                // Terminal stutter has no scheduler entry; other actions
                // belong to products this role does not refute.
                if matches!(
                    action,
                    ProductAction::TerminalStutter | ProductAction::PostState(_)
                ) {
                    continue;
                }
                return not_yet();
            };
            let state_key = match key(budget, state.index()) {
                Ok(state_key) => state_key,
                Err(error) => return document_failure(error, subject),
            };
            let schedule = match fsl_replay::Schedule::of(system, identity) {
                Ok(schedule) => schedule,
                Err(error) => return document_failure(schedule_failure(error), subject),
            };
            spelled.push((
                fsl_eval::simulation::StateKey::from_canonical_bytes(state_key.as_bytes().to_vec()),
                identity.clone(),
            ));
            scheduler.push((
                state_key,
                fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
                    fsl_replay::ProductAction::Identity(schedule),
                ),
            ));
        }
        scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    }
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, limits, subject),
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let subsystem = match ProbabilitySubsystemCertificate::value_bound(
        system,
        system.input().probabilistic.claims(),
        &item,
        if matches!(role, Role::RewardUpper) {
            SubsystemBoundKind::Upper
        } else {
            SubsystemBoundKind::Lower
        },
        &values,
        ranking.as_deref(),
        document_limits,
        cancel,
    ) {
        Ok(subsystem) => subsystem,
        Err(error) => return document_failure(error, subject),
    };
    let initial_key = match key(budget, start.index()) {
        Ok(initial_key) => initial_key,
        Err(error) => return document_failure(error, subject),
    };
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        instance: None,
        initial: first.clone(),
        initial_key,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    finish_replay(
        replay_witness(system, &item, &witness, limits, cancel),
        claim,
        system,
        &witness,
        &spelled,
        limits,
        subject,
    )
}

/// Why producing a subsystem counterexample stopped.
enum Stop {
    /// An exact solve on the induced chain.
    Solver(UnboundedError),
    /// Cancellation, the caller's meter or candidate storage.
    Document(ProbabilityDocumentError),
}

impl From<UnboundedError> for Stop {
    fn from(error: UnboundedError) -> Self {
        Self::Solver(error)
    }
}

impl From<ProbabilityDocumentError> for Stop {
    fn from(error: ProbabilityDocumentError) -> Self {
        Self::Document(error)
    }
}

/// Poll cancellation and charge one graph step to the candidate meter.
fn step(budget: &mut CandidateBudget<'_>) -> Result<(), ProbabilityDocumentError> {
    if let Some(cause) = budget.cancel.tripped() {
        return Err(ProbabilityDocumentError::Cancelled(cause));
    }
    budget
        .meter
        .charge(filament_exact::Charge::new(
            filament_exact::ChargePoint::CollectionMemberTest,
        ))
        .map_err(|incomplete| match budget.cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(incomplete)),
        })
}

/// The states the chain reaches from `start`, stopping at targets.
fn reached(
    chain: &ProbProduct,
    start: ProbStateId,
    target: &[bool],
    budget: &mut CandidateBudget<'_>,
) -> Result<Vec<bool>, ProbabilityDocumentError> {
    let mut reached = vec![false; chain.states.len()];
    let mut pending = vec![start];
    reached[start.index()] = true;
    while let Some(state) = pending.pop() {
        step(budget)?;
        if target[state.index()] {
            continue;
        }
        for edge in chain.outgoing(state) {
            step(budget)?;
            if !edge.probability.is_zero() && !reached[edge.to.index()] {
                reached[edge.to.index()] = true;
                pending.push(edge.to);
            }
        }
    }
    Ok(reached)
}

/// Shortest positive-probability distance to a `goal` state over `through`,
/// by one breadth-first search over predecessor edges.
fn distances(
    chain: &ProbProduct,
    goal: &[bool],
    through: &[bool],
    budget: &mut CandidateBudget<'_>,
) -> Result<Vec<Option<u64>>, ProbabilityDocumentError> {
    let mut predecessors = vec![Vec::new(); chain.states.len()];
    for edge in &chain.edges {
        step(budget)?;
        if !edge.probability.is_zero() {
            predecessors[edge.to.index()].push(edge.from.index());
        }
    }
    let mut distance = goal
        .iter()
        .map(|goal| goal.then_some(0))
        .collect::<Vec<Option<u64>>>();
    let mut pending = (0..goal.len())
        .filter(|state| goal[*state])
        .collect::<std::collections::VecDeque<_>>();
    while let Some(state) = pending.pop_front() {
        step(budget)?;
        let next = distance[state].map_or(0, |rank| rank + 1);
        for &previous in &predecessors[state] {
            step(budget)?;
            if distance[previous].is_none() && through[previous] {
                distance[previous] = Some(next);
                pending.push_back(previous);
            }
        }
    }
    Ok(distance)
}

/// The claim's absorbing states on the chain and the side it is refuted on.
struct Boundary {
    /// Accepting decided states: the event, `until`'s target, or an `always`
    /// state that keeps `P` by terminal stutter.
    success: Vec<bool>,
    /// States where the replayed product stops: accepting and rejecting.
    stop: Vec<bool>,
    /// Whether a bottom component with no decided state accepts (`always`).
    undecided_accepts: bool,
    /// Refute a `<=` bound through the accepting union; otherwise a `>=`
    /// bound through the rejecting union.
    accepting: bool,
}

/// FSpec FR-412's lower values for the violating quantity: the probability
/// of reaching the union of the chain's bottom components classified on the
/// violating side, with ranks by distance to it.
#[allow(clippy::too_many_arguments)]
fn avoidance(
    chain: &ProbProduct,
    start: ProbStateId,
    boundary: &Boundary,
    reached: &[bool],
    threshold: &Rational,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    budget: &mut CandidateBudget<'_>,
) -> Result<Option<Evidence>, Stop> {
    let mut adjacency = Vec::with_capacity(chain.states.len());
    for state in chain.ids() {
        step(budget)?;
        if boundary.stop[state.index()] {
            adjacency.push(vec![state.index()]);
            continue;
        }
        let mut successors = Vec::new();
        for edge in chain.outgoing(state) {
            step(budget)?;
            if !edge.probability.is_zero() {
                successors.push(edge.to.index());
            }
        }
        if successors.is_empty() {
            successors.push(state.index());
        }
        adjacency.push(successors);
    }
    let mut violated = vec![false; chain.states.len()];
    for component in bottom_components(&adjacency) {
        let accepts = if component.iter().any(|state| boundary.success[*state]) {
            true
        } else if component.iter().any(|state| boundary.stop[*state]) {
            false
        } else {
            boundary.undecided_accepts
        };
        if accepts == boundary.accepting {
            for state in component {
                violated[state] = true;
            }
        }
    }
    // Decided states outside the union absorb at value 0.
    let open = (0..chain.states.len())
        .map(|state| violated[state] || !boundary.stop[state])
        .collect::<Vec<_>>();
    let solved = decide_until(
        chain,
        &open,
        &violated,
        UnboundedObjective::Min,
        limits,
        cancel,
    )?;
    let value = |state: usize| match solved.values.get(state) {
        Some(ExtRational::Finite(value)) => Ok(value.clone()),
        _ => Err(UnboundedError::InvalidState),
    };
    let one = Rational::from_integer(filament_exact::Integer::from(1_i64));
    let past = if boundary.accepting {
        threshold.clone()
    } else {
        one.sub(threshold)
    };
    if value(start.index())? <= past {
        return Ok(None);
    }
    let positive = (0..chain.states.len())
        .map(|state| value(state).map(|value| !value.is_zero() && open[state]))
        .collect::<Result<Vec<_>, _>>()?;
    let distance = distances(chain, &violated, &positive, budget)?;
    let mut values = Vec::new();
    let mut ranks = Vec::new();
    // A `lower` certificate names its positive states; an unnamed one counts 0.
    for (state, reached) in reached.iter().enumerate() {
        let value = value(state)?;
        if *reached && !value.is_zero() {
            // A positive state with no path to the union cannot be ranked:
            // decline rather than emit evidence replay must reject.
            let Some(rank) = distance[state] else {
                return Ok(None);
            };
            values.push((state, value));
            ranks.push((state, rank));
        }
    }
    Ok(Some(Evidence {
        values,
        ranks: Some(ranks),
    }))
}

/// The expected reward on the induced chain. An upper bound needs it finite
/// at every reached state, with ranks by distance to the target. A lower
/// bound names infinite states at `K · Pr(never target)`, where `K` puts the
/// initial state above the bound: such values satisfy `x <= F(x)` because
/// the never-target probability is harmonic and every reward is nonnegative.
#[allow(clippy::too_many_arguments)]
fn reward_bound(
    chain: &ProbProduct,
    start: ProbStateId,
    target: &[bool],
    reached: &[bool],
    reward: &str,
    threshold: &Rational,
    upper: bool,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    budget: &mut CandidateBudget<'_>,
) -> Result<Option<Evidence>, Stop> {
    let expected = decide_expected_reward(
        chain,
        target,
        reward,
        UnboundedObjective::Min,
        limits,
        cancel,
    )?;
    let one = Rational::from_integer(filament_exact::Integer::from(1_i64));
    let initial = expected.values.get(start.index());
    let mut values = Vec::new();
    if upper {
        let Some(ExtRational::Finite(initial)) = initial else {
            return Ok(None);
        };
        if initial >= threshold {
            return Ok(None);
        }
        for (state, reached) in reached.iter().enumerate() {
            if !*reached {
                continue;
            }
            let Some(ExtRational::Finite(value)) = expected.values.get(state) else {
                return Ok(None);
            };
            values.push((state, value.clone()));
        }
        let distance = distances(chain, target, &vec![true; target.len()], budget)?;
        let mut ranks = Vec::new();
        for (state, reached) in reached.iter().enumerate() {
            if *reached {
                let Some(rank) = distance[state] else {
                    return Ok(None);
                };
                ranks.push((state, rank));
            }
        }
        return Ok(Some(Evidence {
            values,
            ranks: Some(ranks),
        }));
    }
    let scale = match initial {
        Some(ExtRational::Finite(initial)) if initial > threshold => None,
        Some(ExtRational::Finite(_)) => return Ok(None),
        Some(ExtRational::PlusInfinity) => {
            let reaches =
                decide_reachability(chain, target, UnboundedObjective::Min, limits, cancel)?;
            let never = |state: usize| match reaches.values.get(state) {
                Some(ExtRational::Finite(value)) => Ok(one.sub(value)),
                _ => Err(Stop::Solver(UnboundedError::InvalidState)),
            };
            let avoided = never(start.index())?;
            let Some(scale) = threshold.div(&avoided).map(|ratio| ratio.add(&one)) else {
                return Ok(None);
            };
            Some((
                scale,
                (0..chain.states.len())
                    .map(never)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        None => return Err(UnboundedError::InvalidState.into()),
    };
    for (state, reached) in reached.iter().enumerate() {
        if !*reached {
            continue;
        }
        let value = match (expected.values.get(state), &scale) {
            (Some(ExtRational::Finite(value)), _) => value.clone(),
            (Some(ExtRational::PlusInfinity), Some((scale, never))) => scale.mul(&never[state]),
            _ => return Err(UnboundedError::InvalidState.into()),
        };
        values.push((state, value));
    }
    Ok(Some(Evidence {
        values,
        ranks: None,
    }))
}
