// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202 subsystem counterexamples for a failing unwindowed `eventually` or
//! expected-reward claim. EN-5 selects the witness scheduler (none under a
//! workload), solves the chain it induces, writes a `lower` or `upper`
//! certificate over that chain and settles `refuted` only after the core's
//! independent replay reproduces it.

use super::candidate::{rational_bytes, sort, CandidateBudget};
use super::*;
use crate::exact_probabilistic::{
    avoiding_policy, bottom_components, induced_chain, policy_action, ExactProbLimits, ProbStateId,
    ProductAction,
};
use fsl_replay::{ProbabilitySubsystemCertificate, SubsystemBoundKind};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Role {
    /// `probability >= θ [eventually E]`: a lower bound on avoiding E.
    EventAvoidance,
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
    target: &[bool],
    binding: Option<&ObjectReference>,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    budget: &mut CandidateBudget<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let (role, threshold) = match &claim.form {
        ProbForm::Reach {
            bound: Bound::AtLeast(threshold),
            target: ReachTarget::Eventually(_),
        } => (Role::EventAvoidance, threshold),
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
    let every = matches!(claim.scheduler, ProbScheduler::Every);
    let selected = if every && role != Role::RewardUpper {
        match avoiding_policy(product, target, policy, role == Role::RewardLower) {
            Ok(selected) => selected,
            Err(error) => return unbounded_failure(error, limits, subject),
        }
    } else {
        policy.to_vec()
    };
    let chain = induced_chain(product, &selected);
    let reached = reached(&chain, start, target);
    let evidence = match role {
        Role::EventAvoidance => avoidance(&chain, start, target, &reached, threshold, limits),
        Role::RewardLower | Role::RewardUpper => {
            let ProbForm::ExpectedReward { reward, .. } = &claim.form else {
                return broken();
            };
            reward_bound(
                &chain,
                start,
                target,
                &reached,
                reward.as_str(),
                threshold,
                role == Role::RewardUpper,
                limits,
            )
        }
    };
    let evidence = match evidence {
        Ok(Some(evidence)) => evidence,
        // The selected chain does not refute the bound: no counterexample.
        Ok(None) => return not_yet(),
        Err(error) => return unbounded_failure(error, limits, subject),
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
            if !reached[state.index()] || target[state.index()] {
                continue;
            }
            let Some(action) = policy_action(product, state, &selected) else {
                continue;
            };
            let ProductAction::Identity(identity) = &action else {
                // Terminal stutter has no scheduler entry; other actions
                // belong to products this role does not refute.
                if matches!(action, ProductAction::TerminalStutter) {
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
                Err(_) => return not_yet(),
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
        if role == Role::RewardUpper {
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

/// The states the chain reaches from `start`, stopping at targets.
fn reached(chain: &ProbProduct, start: ProbStateId, target: &[bool]) -> Vec<bool> {
    let mut reached = vec![false; chain.states.len()];
    let mut pending = vec![start];
    reached[start.index()] = true;
    while let Some(state) = pending.pop() {
        if target[state.index()] {
            continue;
        }
        for edge in chain.outgoing(state) {
            if !edge.probability.is_zero() && !reached[edge.to.index()] {
                reached[edge.to.index()] = true;
                pending.push(edge.to);
            }
        }
    }
    reached
}

/// Shortest positive-probability distance to a `goal` state over `through`.
fn distances(chain: &ProbProduct, goal: &[bool], through: &[bool]) -> Vec<Option<u64>> {
    let mut distance = goal
        .iter()
        .map(|goal| goal.then_some(0))
        .collect::<Vec<Option<u64>>>();
    let mut round = 0;
    loop {
        round += 1;
        let previous = distance.clone();
        for state in chain.ids() {
            let index = state.index();
            if previous[index].is_some() || !through[index] {
                continue;
            }
            if chain
                .outgoing(state)
                .iter()
                .any(|edge| !edge.probability.is_zero() && previous[edge.to.index()].is_some())
            {
                distance[index] = Some(round);
            }
        }
        if distance == previous {
            return distance;
        }
    }
}

/// FSpec FR-412's event-avoidance values: the probability of reaching the
/// chain's event-free bottom components, with ranks by distance to them.
fn avoidance(
    chain: &ProbProduct,
    start: ProbStateId,
    target: &[bool],
    reached: &[bool],
    threshold: &Rational,
    limits: &ExactProbLimits,
) -> Result<Option<Evidence>, UnboundedError> {
    let adjacency = chain
        .ids()
        .map(|state| {
            if target[state.index()] {
                return vec![state.index()];
            }
            let mut successors = chain
                .outgoing(state)
                .iter()
                .filter(|edge| !edge.probability.is_zero())
                .map(|edge| edge.to.index())
                .collect::<Vec<_>>();
            if successors.is_empty() {
                successors.push(state.index());
            }
            successors
        })
        .collect::<Vec<_>>();
    let mut avoided = vec![false; chain.states.len()];
    for component in bottom_components(&adjacency) {
        if component.iter().all(|state| !target[*state]) {
            for state in component {
                avoided[state] = true;
            }
        }
    }
    let outside = target.iter().map(|target| !target).collect::<Vec<_>>();
    let solved = decide_until(chain, &outside, &avoided, UnboundedObjective::Min, limits)?;
    let value = |state: usize| match solved.values.get(state) {
        Some(ExtRational::Finite(value)) => Ok(value.clone()),
        _ => Err(UnboundedError::InvalidState),
    };
    let one = Rational::from_integer(filament_exact::Integer::from(1_i64));
    if value(start.index())? <= one.sub(threshold) {
        return Ok(None);
    }
    let positive = (0..chain.states.len())
        .map(|state| value(state).map(|value| !value.is_zero() && !target[state]))
        .collect::<Result<Vec<_>, _>>()?;
    let distance = distances(chain, &avoided, &positive);
    let mut values = Vec::new();
    let mut ranks = Vec::new();
    for (state, reached) in reached.iter().enumerate() {
        if *reached {
            values.push((state, value(state)?));
            ranks.push((state, distance[state].unwrap_or(0)));
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
) -> Result<Option<Evidence>, UnboundedError> {
    let expected = decide_expected_reward(chain, target, reward, UnboundedObjective::Min, limits)?;
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
        let distance = distances(chain, target, &vec![true; target.len()]);
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
            let reaches = decide_reachability(chain, target, UnboundedObjective::Min, limits)?;
            let never = |state: usize| match reaches.values.get(state) {
                Some(ExtRational::Finite(value)) => Ok(one.sub(value)),
                _ => Err(UnboundedError::InvalidState),
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
        None => return Err(UnboundedError::InvalidState),
    };
    for (state, reached) in reached.iter().enumerate() {
        if !*reached {
            continue;
        }
        let value = match (expected.values.get(state), &scale) {
            (Some(ExtRational::Finite(value)), _) => value.clone(),
            (Some(ExtRational::PlusInfinity), Some((scale, never))) => scale.mul(&never[state]),
            _ => return Err(UnboundedError::InvalidState),
        };
        values.push((state, value));
    }
    Ok(Some(Evidence {
        values,
        ranks: None,
    }))
}
