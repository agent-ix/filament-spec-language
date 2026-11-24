// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921's independent timed-product certificate admission.

use super::*;
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Integer, Rational};
use fsl_eval::simulation::model::{ClockValuation, ModelSimState, TimedState, TimedSystem};
use num_traits::Zero;
use serde_json::{Value as JsonValue, json};

mod elapsed;

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    bound_side: Option<SubsystemBoundKind>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let document =
        read_document_profile(bytes, limits, cancel, &mut meter, false, None, &mut storage)?;
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    let Some(objective) = document.timed.as_ref() else {
        return Err(ProbabilityDocumentError::MalformedMember("objective"));
    };
    let timed = fsl_eval::simulation::model::TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    let route = if region_route(&timed, claim) {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let scale = certificate_scale(system, &timed, claim, route)?;
    if objective.route != route
        || objective.unit != timed.source().unit_node
        || objective.scale != scale
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let deadline = objective.form == TimedProbabilityForm::Deadline;
    if bound_side.is_some() && !deadline {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if objective.form == TimedProbabilityForm::ExpectedElapsed {
        return check_expected_elapsed(
            system,
            claim,
            bytes,
            document,
            &timed,
            route,
            scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
        );
    }
    if objective.form == TimedProbabilityForm::ElapsedQuantile {
        return elapsed::check(
            system,
            claim,
            bytes,
            document,
            &timed,
            route,
            scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
        );
    }
    if objective.form != TimedProbabilityForm::Deadline {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let Some((bound, watch)) = bounded_event(claim, objective.scale) else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let optimization = match (&claim.scheduler, &bound) {
        (fsl_semantics::check::ProbScheduler::Every, Bound::AtLeast(_)) => {
            ProbabilityObjective::Minimum
        }
        (fsl_semantics::check::ProbScheduler::Every, Bound::AtMost(_)) => {
            ProbabilityObjective::Maximum
        }
        (fsl_semantics::check::ProbScheduler::Workload(_), _) => ProbabilityObjective::Workload,
    };
    let workload_claim = matches!(
        claim.scheduler,
        fsl_semantics::check::ProbScheduler::Workload(_)
    );
    let optimization_admitted = if workload_claim {
        workload_objective(document.optimization)
    } else {
        document.optimization == optimization
    };
    if !optimization_admitted {
        return Err(ProbabilityDocumentError::OptimizationMismatch);
    }
    if !document.ranking.is_empty() {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if document.cyclic_dtmc
        || document.event.is_some()
        || !document.failures.is_empty()
        || document.reward.is_some()
        || !claim.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let graph = match route {
        TimedProbabilityRoute::Digital => enumerate_digital_deadline(
            system,
            &timed,
            claim,
            binding.as_ref(),
            &watch,
            watch.horizon(),
            objective.scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
            &mut BTreeMap::new(),
        )?,
        TimedProbabilityRoute::Region => enumerate_region_deadline(
            system,
            &timed,
            claim,
            binding.as_ref(),
            &watch,
            watch.horizon(),
            objective.scale,
            scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
            &mut BTreeMap::new(),
            &mut BTreeSet::new(),
        )?,
    };
    // A workload resolves a tie by its weights and leaves the scheduler only
    // the delay choices FR-204 names; the checker's own graph says whether
    // any remain, and the document's objective must say the same.
    let optimization = if workload_claim {
        let residual = graph.states.values().any(|state| state.choices.len() > 1);
        if residual && route == TimedProbabilityRoute::Region {
            // The region product mirrors only the states that leave the
            // scheduler no choice.
            return Err(ProbabilityDocumentError::Unsupported);
        }
        workload_objective_for(&bound, residual)
    } else {
        optimization
    };
    if document.optimization != optimization {
        return Err(ProbabilityDocumentError::OptimizationMismatch);
    }
    let replay_targets = target_keys(&graph);
    if replay_targets != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if graph.states.len() != document.values.len()
        || document
            .values
            .iter()
            .any(|row| !graph.states.contains_key(&row.state))
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let mut checked = super::super::check_reachability_values(
        &graph,
        &certificate,
        scheduler_objective(optimization),
        &bound,
        super::super::GraphCycles::CollapseNonprogressive,
        match bound_side {
            None => super::super::ValueRelation::Exact,
            Some(SubsystemBoundKind::Lower) => super::super::ValueRelation::Lower,
            Some(SubsystemBoundKind::Upper) => super::super::ValueRelation::Upper,
        },
        cancel,
    )
    .map_err(ProbabilityDocumentError::Graph)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut checked {
        accepted.document = Some(bytes.into());
    }
    Ok(checked)
}

#[allow(clippy::too_many_arguments)]
fn check_expected_elapsed(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    bytes: &[u8],
    document: ProbabilityCertificateDocument,
    timed: &TimedSystem<'_, '_>,
    route: TimedProbabilityRoute,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let Some((bound, predicate)) = mean_time_query(claim) else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
        || !claim.fairness.is_empty()
        || document.cyclic_dtmc
        || document.event.is_some()
        || !document.failures.is_empty()
        || document.reward.is_some()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let optimization = match bound {
        Bound::AtLeast(_) => ProbabilityObjective::Minimum,
        Bound::AtMost(_) => ProbabilityObjective::Maximum,
    };
    if document.optimization != optimization {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let (graph, costs) = enumerate_region_mean_time(
        system,
        timed,
        claim,
        binding.as_ref(),
        predicate,
        scale,
        route,
        limits,
        cancel,
        meter,
        storage,
    )?;
    if target_keys(&graph) != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let checked = check_mean_time_values(
        &graph,
        &costs,
        &document,
        optimization,
        &bound,
        limits,
        cancel,
        meter,
    )?;
    match checked {
        ExactProbabilityCertificateCheck::Accepted(mut accepted) => {
            accepted.document = Some(bytes.into());
            Ok(ExactProbabilityCertificateCheck::Accepted(accepted))
        }
        rejected => Ok(rejected),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_mean_time_values(
    graph: &CheckerGraph,
    costs: &BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>,
    document: &ProbabilityCertificateDocument,
    objective: ProbabilityObjective,
    bound: &Bound,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut values = BTreeMap::new();
    for row in &document.values {
        poll(cancel)?;
        check_scalar(&row.value, limits)?;
        if row.value < zero() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        values.insert(row.state.clone(), row.value.clone());
    }
    if values.len() != graph.states.len()
        || graph.states.keys().any(|key| !values.contains_key(key))
    {
        return Ok(rejected(
            ProbabilityCertificateRule::Malformed,
            graph.initial.clone(),
        ));
    }
    let mut ranks = BTreeMap::new();
    for (key, rank) in &document.ranking {
        poll(cancel)?;
        if !graph.states.contains_key(key) || ranks.insert(key.clone(), *rank).is_some() {
            return Ok(rejected(
                ProbabilityCertificateRule::RankMissing,
                key.clone(),
            ));
        }
    }
    if ranks.len() != graph.states.len() {
        let at = graph
            .states
            .keys()
            .find(|key| !ranks.contains_key(*key))
            .cloned()
            .unwrap_or_else(|| graph.initial.clone());
        return Ok(rejected(ProbabilityCertificateRule::RankMissing, at));
    }
    let components =
        super::super::nonprogressive_end_components_by_kind(graph, cancel, Some(costs))?;
    let mut component_of = BTreeMap::new();
    for (component, (_, members)) in components.iter().enumerate() {
        for key in members {
            component_of.insert(key.clone(), component);
        }
    }
    for (kind, members) in &components {
        poll(cancel)?;
        let member_set = members.iter().cloned().collect::<BTreeSet<_>>();
        let mut exits = Vec::new();
        let mut positive_internal_cost = false;
        for key in members {
            let state = graph
                .states
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let action_costs = costs.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            if action_costs.len() != state.choices.len() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            for (choice_index, choice) in state.choices.iter().enumerate() {
                let edge_costs = action_costs
                    .get(choice_index)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                if edge_costs.len() != choice.len() {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                let mut mass = zero();
                let mut internal = zero();
                let mut reward = zero();
                for (edge, cost) in choice.iter().zip(edge_costs) {
                    poll(cancel)?;
                    check_scalar(cost, limits)?;
                    mass = arithmetic(
                        filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
                        meter,
                        cancel,
                    )?;
                    let term = if member_set.contains(&edge.target) {
                        if !cost.is_zero() {
                            positive_internal_cost = true;
                        }
                        internal = arithmetic(
                            filament_exact::RationalArithmetic::Add(&internal, &edge.probability),
                            meter,
                            cancel,
                        )?;
                        zero()
                    } else {
                        values
                            .get(&edge.target)
                            .ok_or(ProbabilityDocumentError::Malformed)?
                            .clone()
                    };
                    let step_value = arithmetic(
                        filament_exact::RationalArithmetic::Add(cost, &term),
                        meter,
                        cancel,
                    )?;
                    let weighted = arithmetic(
                        filament_exact::RationalArithmetic::Multiply(
                            &edge.probability,
                            &step_value,
                        ),
                        meter,
                        cancel,
                    )?;
                    reward = arithmetic(
                        filament_exact::RationalArithmetic::Add(&reward, &weighted),
                        meter,
                        cancel,
                    )?;
                }
                if mass != one() {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                let denominator = one().sub(&internal);
                if !denominator.is_zero() {
                    if let Some(value) = reward.div(&denominator) {
                        exits.push((key.clone(), choice_index, value));
                    }
                }
            }
        }
        if exits.is_empty() && positive_internal_cost {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let expected = match objective {
            ProbabilityObjective::Minimum => exits.iter().map(|exit| exit.2.clone()).min(),
            ProbabilityObjective::Maximum => exits.iter().map(|exit| exit.2.clone()).max(),
            ProbabilityObjective::Workload
            | ProbabilityObjective::WorkloadMinimum
            | ProbabilityObjective::WorkloadMaximum => {
                return Err(ProbabilityDocumentError::Unsupported);
            }
        }
        .unwrap_or_else(zero);
        for key in members {
            if values.get(key) != Some(&expected) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
        }
        if !mean_time_component_policy_valid(
            graph,
            &member_set,
            &exits,
            &expected,
            costs,
            &values,
            &ranks,
            cancel,
            meter,
            *kind,
        )? {
            return Ok(rejected(
                ProbabilityCertificateRule::RankBroken,
                members[0].clone(),
            ));
        }
    }
    for (key, state) in &graph.states {
        poll(cancel)?;
        if component_of.contains_key(key) {
            continue;
        }
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let action_costs = costs.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        if state.terminal == Some(true) {
            if value != &zero() || *rank != 0 {
                return Ok(rejected(
                    ProbabilityCertificateRule::RankBroken,
                    key.clone(),
                ));
            }
            continue;
        }
        if *rank == 0 || state.choices.is_empty() || action_costs.len() != state.choices.len() {
            return Ok(rejected(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            ));
        }
        let mut best: Option<Rational> = None;
        let mut progress = match objective {
            ProbabilityObjective::Maximum => true,
            ProbabilityObjective::Minimum => false,
            ProbabilityObjective::Workload
            | ProbabilityObjective::WorkloadMinimum
            | ProbabilityObjective::WorkloadMaximum => {
                return Err(ProbabilityDocumentError::Unsupported);
            }
        };
        for (choice_index, choice) in state.choices.iter().enumerate() {
            let edge_costs = action_costs
                .get(choice_index)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if edge_costs.len() != choice.len() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            let mut sum = zero();
            let mut mass = zero();
            let mut descends = false;
            for (edge, cost) in choice.iter().zip(edge_costs) {
                poll(cancel)?;
                check_scalar(cost, limits)?;
                let successor = values
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let next_cost = arithmetic(
                    filament_exact::RationalArithmetic::Add(cost, successor),
                    meter,
                    cancel,
                )?;
                let weighted = arithmetic(
                    filament_exact::RationalArithmetic::Multiply(&edge.probability, &next_cost),
                    meter,
                    cancel,
                )?;
                sum = arithmetic(
                    filament_exact::RationalArithmetic::Add(&sum, &weighted),
                    meter,
                    cancel,
                )?;
                mass = arithmetic(
                    filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
                    meter,
                    cancel,
                )?;
                if edge.probability > zero()
                    && ranks
                        .get(&edge.target)
                        .is_some_and(|target_rank| target_rank < rank)
                {
                    descends = true;
                }
            }
            if mass != one() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            best = Some(match best {
                None => sum.clone(),
                Some(previous) => match objective {
                    ProbabilityObjective::Minimum => previous.min(sum.clone()),
                    ProbabilityObjective::Maximum => previous.max(sum.clone()),
                    ProbabilityObjective::Workload
                    | ProbabilityObjective::WorkloadMinimum
                    | ProbabilityObjective::WorkloadMaximum => {
                        return Err(ProbabilityDocumentError::Unsupported);
                    }
                },
            });
            match objective {
                ProbabilityObjective::Maximum => progress &= descends,
                ProbabilityObjective::Minimum if sum == *value => progress |= descends,
                ProbabilityObjective::Minimum => {}
                ProbabilityObjective::Workload
                | ProbabilityObjective::WorkloadMinimum
                | ProbabilityObjective::WorkloadMaximum => {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
            }
        }
        if best.as_ref() != Some(value) {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            ));
        }
        if !progress {
            return Ok(rejected(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            ));
        }
    }
    let initial = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    check_scalar(
        match bound {
            Bound::AtLeast(value) | Bound::AtMost(value) => value,
        },
        limits,
    )?;
    let meets = match bound {
        Bound::AtLeast(threshold) => initial >= threshold,
        Bound::AtMost(threshold) => initial <= threshold,
    };
    if !meets {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: None,
            bound: None,
            value: initial.clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
            certificate: ExactProbabilityCertificate {
                values: document.values.clone(),
            },
            document: None,
        },
    ))
}

/// Check a checker-derived shortest-path policy from a component member to
/// an objective-best exit. This independently validates the macro state's
/// exit closure instead of trusting member values alone.
#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn mean_time_component_policy_valid(
    graph: &CheckerGraph,
    members: &BTreeSet<ProbabilityProductKey>,
    exits: &[(ProbabilityProductKey, usize, Rational)],
    expected: &Rational,
    costs: &BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    kind: super::super::ComponentKind,
) -> Result<bool, ProbabilityDocumentError> {
    let keys = graph.states.keys().cloned().collect::<Vec<_>>();
    let indices = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let candidate = members
        .iter()
        .map(|key| {
            indices
                .get(key)
                .copied()
                .ok_or(ProbabilityDocumentError::Malformed)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let zero = zero();
    for (owner, owner_choice, exit_value) in exits {
        poll(cancel)?;
        if exit_value != expected {
            continue;
        }
        let mut distance = members
            .iter()
            .map(|key| (key.clone(), None))
            .collect::<BTreeMap<_, Option<usize>>>();
        *distance
            .get_mut(owner)
            .ok_or(ProbabilityDocumentError::Malformed)? = Some(0);
        loop {
            let mut changed = false;
            for key in members {
                poll(cancel)?;
                if distance.get(key).copied().flatten().is_some() {
                    continue;
                }
                let state = graph
                    .states
                    .get(key)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let action_costs = costs.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                let mut nearest: Option<usize> = None;
                for (choice_index, choice) in state.choices.iter().enumerate() {
                    let edge_costs = action_costs
                        .get(choice_index)
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    if !super::super::internal_nonprogressive_choice(
                        key,
                        choice,
                        Some(edge_costs),
                        &candidate,
                        &indices,
                        &zero,
                        kind,
                    )? {
                        continue;
                    }
                    for edge in choice {
                        if edge.probability <= zero {
                            continue;
                        }
                        if let Some(next) = distance.get(&edge.target).copied().flatten() {
                            let candidate_distance = next + 1;
                            nearest = Some(nearest.map_or(candidate_distance, |current| {
                                current.min(candidate_distance)
                            }));
                        }
                    }
                }
                if let Some(nearest) = nearest {
                    *distance
                        .get_mut(key)
                        .ok_or(ProbabilityDocumentError::Malformed)? = Some(nearest);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        if members
            .iter()
            .any(|key| distance.get(key).copied().flatten().is_none())
        {
            continue;
        }
        let mut valid = true;
        for key in members {
            poll(cancel)?;
            let state = graph
                .states
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let action_costs = costs.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let selected = if key == owner {
                *owner_choice
            } else {
                let current = distance.get(key).copied().flatten().unwrap_or(0);
                let mut selected = None;
                for (choice_index, choice) in state.choices.iter().enumerate() {
                    let edge_costs = action_costs
                        .get(choice_index)
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    if super::super::internal_nonprogressive_choice(
                        key,
                        choice,
                        Some(edge_costs),
                        &candidate,
                        &indices,
                        &zero,
                        kind,
                    )? && choice.iter().any(|edge| {
                        edge.probability > zero
                            && distance
                                .get(&edge.target)
                                .copied()
                                .flatten()
                                .is_some_and(|next| next < current)
                    }) {
                        selected = Some(choice_index);
                        break;
                    }
                }
                let Some(choice_index) = selected else {
                    valid = false;
                    break;
                };
                choice_index
            };
            let Some(choice) = state.choices.get(selected) else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            let Some(edge_costs) = action_costs.get(selected) else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            let mut sum = zero.clone();
            let mut descends = false;
            for (edge, cost) in choice.iter().zip(edge_costs) {
                poll(cancel)?;
                let successor = values
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let next_value = arithmetic(
                    filament_exact::RationalArithmetic::Add(cost, successor),
                    meter,
                    cancel,
                )?;
                let weighted = arithmetic(
                    filament_exact::RationalArithmetic::Multiply(&edge.probability, &next_value),
                    meter,
                    cancel,
                )?;
                sum = arithmetic(
                    filament_exact::RationalArithmetic::Add(&sum, &weighted),
                    meter,
                    cancel,
                )?;
                descends |= edge.probability > zero
                    && ranks
                        .get(&edge.target)
                        .zip(ranks.get(key))
                        .is_some_and(|(next, current)| next < current);
            }
            if sum != *expected || !descends {
                valid = false;
                break;
            }
        }
        if valid {
            return Ok(true);
        }
    }
    Ok(false)
}

fn mean_time_query(
    claim: &CheckedProbabilisticClaim,
) -> Option<(Bound, &fsl_semantics::check::CheckedPredicate)> {
    let fsl_semantics::check::ProbForm::MeanTime { bound, until } = &claim.form else {
        return None;
    };
    Some((bound.clone(), until))
}

fn zero() -> Rational {
    Rational::from_integer(Integer::from(0_i64))
}

/// Exact weights of each region-product state's policy choices.
type RegionPolicyWeights = BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>;

#[derive(Clone)]
struct MeanRegionState {
    timed: TimedState,
    target: bool,
}

#[allow(clippy::too_many_arguments)]
fn enumerate_region_mean_time(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    scale: u64,
    route: TimedProbabilityRoute,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<(CheckerGraph, RegionPolicyWeights), ProbabilityDocumentError> {
    let (base_scale, subdivisions) = match route {
        TimedProbabilityRoute::Region => region_scale_parts(timed, claim, scale)?,
        TimedProbabilityRoute::Digital => (scale, 1),
    };
    let mut caps = Vec::with_capacity(timed.places().len());
    for place in timed.places() {
        let mut largest = 0_u64;
        for constraint in clock_constraints(timed)
            .into_iter()
            .filter(|constraint| constraint.clock() == place.clock())
        {
            largest = largest.max(
                time_units(constraint.bound(), scale)
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            );
        }
        caps.push(
            largest
                .checked_add(1)
                .ok_or(ProbabilityDocumentError::Unsupported)?,
        );
    }
    let mut initial = timed.initial();
    if initial.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let initial = initial.pop().ok_or(ProbabilityDocumentError::Unsupported)?;
    let first = MeanRegionState {
        target: mean_target(system, predicate, binding, initial.discrete(), cancel)?,
        timed: initial,
    };
    let first_key = mean_region_key(
        system,
        timed,
        binding,
        &first,
        &caps,
        base_scale,
        subdivisions,
        scale,
        route,
    )?;
    reserve_storage(
        storage,
        first_key
            .as_bytes()
            .len()
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, MeanRegionState)>()),
        limits,
    )?;
    let mut pending = VecDeque::from([first_key.clone()]);
    let mut reached = BTreeMap::from([(first_key.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut costs = BTreeMap::new();
    let mut edges = 0_u64;
    while let Some(key) = pending.pop_front() {
        charge_work(meter, cancel, 1)?;
        poll(cancel)?;
        let current = reached
            .get(&key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if current.target {
            costs.insert(key.clone(), Vec::new());
            insert_checker_state(
                key,
                CheckerState {
                    terminal: Some(true),
                    choices: Vec::new(),
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        if timed
            .is_quiescent(current.timed.discrete())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut choices = Vec::new();
        let mut choice_costs = Vec::new();
        if let Some((delayed_key, delayed, delay_cost)) = next_mean_region_delay(
            system,
            timed,
            predicate,
            binding,
            &current,
            &caps,
            base_scale,
            subdivisions,
            scale,
            route,
            cancel,
        )? {
            admit_mean_region_state(
                delayed_key.clone(),
                delayed,
                &mut reached,
                &mut pending,
                limits,
                storage,
            )?;
            choices.push(vec![ProbabilityEdge {
                target: delayed_key,
                probability: one(),
            }]);
            choice_costs.push(vec![delay_cost]);
            edges = edges.saturating_add(1);
            charge_work(meter, cancel, 1)?;
        }
        let clocked_steps = timed
            .steps(&current.timed)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let actions = system
            .actions_with_cancel(current.timed.discrete(), cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        admit_workload(system, claim, &actions)?;
        for action in actions {
            charge_work(meter, cancel, 1)?;
            let mut choice: Vec<ProbabilityEdge> = Vec::new();
            let mut enabled = true;
            for draw in action.draws {
                if draw.successors.len() != 1 {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let (transition, discrete_post) = &draw.successors[0];
                let Some((_, timed_post)) = clocked_steps.iter().find(|(step, post)| {
                    step.transition() == transition
                        && system
                            .state_key(post.discrete())
                            .ok()
                            .zip(system.state_key(discrete_post).ok())
                            .is_some_and(|(left, right)| left.0 == right.0)
                }) else {
                    enabled = false;
                    break;
                };
                let post = TimedState::new(
                    timed_post.discrete().clone(),
                    ClockValuation::new(quantized_clocks(timed_post, &caps, scale)?),
                );
                let target = MeanRegionState {
                    target: mean_target(system, predicate, binding, post.discrete(), cancel)?,
                    timed: post,
                };
                let target_key = mean_region_key(
                    system,
                    timed,
                    binding,
                    &target,
                    &caps,
                    base_scale,
                    subdivisions,
                    scale,
                    route,
                )?;
                admit_mean_region_state(
                    target_key.clone(),
                    target,
                    &mut reached,
                    &mut pending,
                    limits,
                    storage,
                )?;
                if let Some(edge) = choice.iter_mut().find(|edge| edge.target == target_key) {
                    edge.probability = edge.probability.add(&draw.probability);
                } else {
                    choice.push(ProbabilityEdge {
                        target: target_key,
                        probability: draw.probability.clone(),
                    });
                }
                edges = edges.saturating_add(1);
                charge_work(meter, cancel, 1)?;
            }
            if enabled && !choice.is_empty() {
                choice_costs.push(vec![zero(); choice.len()]);
                choices.push(choice);
            }
        }
        if edges > u64::try_from(limits.edges).unwrap_or(u64::MAX) {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
                actual: edges,
            });
        }
        insert_checker_state(
            key.clone(),
            CheckerState {
                terminal: choices.is_empty().then_some(false),
                choices,
            },
            &mut states,
            storage,
            limits,
        )?;
        costs.insert(key, choice_costs);
    }
    Ok((
        CheckerGraph {
            initial: first_key,
            states,
        },
        costs,
    ))
}

fn mean_target(
    system: &ModelSystem<'_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    state: &ModelSimState,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let readings = system
        .state_predicates_with_cancel(state, &[(predicate, binding)], cancel)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    match readings.as_slice() {
        [fsl_eval::simulation::model::PredicateReading::True] => Ok(true),
        [fsl_eval::simulation::model::PredicateReading::False] => Ok(false),
        [
            fsl_eval::simulation::model::PredicateReading::Undefined { .. }
            | fsl_eval::simulation::model::PredicateReading::Refused(_),
        ] => Err(ProbabilityDocumentError::Unsupported),
        _ => Err(ProbabilityDocumentError::Unsupported),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn mean_region_key(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    binding: Option<&filament_exact::ObjectReference>,
    state: &MeanRegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    scale: u64,
    route: TimedProbabilityRoute,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let (discrete_bytes, _) = system
        .state_key(state.timed.discrete())
        .map_err(ProbabilityDocumentError::KeyEncoding)?;
    let discrete: JsonValue =
        serde_json::from_slice(&discrete_bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let binding: Option<JsonValue> = binding
        .map(|reference| {
            let text = fsl_eval::simulation::model::reference_canonical_text(reference)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            serde_json::from_str(&text).map_err(|_| ProbabilityDocumentError::Malformed)
        })
        .transpose()?;
    let unit = json!([
        "declared",
        fsl_foundation::node_wire::node_key_to_json(&timed.source().unit_node)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
    ]);
    let mut clocks = Vec::with_capacity(timed.places().len());
    for ((place, value), cap) in timed
        .places()
        .iter()
        .zip(state.timed.clocks().values())
        .zip(caps)
    {
        let field = timed
            .model()
            .clock(place.clock())
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let owner = field.owner();
        let owner = owner.original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver = fsl_eval::simulation::model::reference_canonical_text(place.object())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver: JsonValue =
            serde_json::from_str(&receiver).map_err(|_| ProbabilityDocumentError::Malformed)?;
        clocks.push(crate::RegionClockKey {
            identity: json!([declaring, field.name(), receiver]),
            value: time_units(value, scale).ok_or(ProbabilityDocumentError::Unsupported)?,
            cap: *cap,
        });
    }
    if route == TimedProbabilityRoute::Digital {
        return crate::digital_probability_product_key(crate::DigitalProductKeyInput {
            binding,
            discrete,
            unit,
            scale: base_scale,
            clocks: clocks
                .into_iter()
                .map(|clock| DigitalClockKey {
                    identity: clock.identity,
                    value: clock.value,
                    cap: clock.cap,
                })
                .collect(),
            deadline: None,
            elapsed: None,
            tail: crate::DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .map_err(ProbabilityDocumentError::Graph);
    }
    let mut coordinates = clocks
        .iter()
        .map(|clock| {
            quire_canonical::to_vec(&clock.identity, quire_canonical::Limits::new(u64::MAX))
                .map(|identity| (identity, clock.value.to_string()))
                .map_err(|_| ProbabilityDocumentError::Unsupported)
        })
        .collect::<Result<Vec<_>, _>>()?;
    coordinates.sort_by(|left, right| left.0.cmp(&right.0));
    let corner = Some(
        coordinates
            .into_iter()
            .map(|(_, coordinate)| coordinate)
            .collect(),
    );
    crate::region_probability_product_key(crate::RegionProductKeyInput {
        binding,
        discrete,
        unit,
        scale: base_scale,
        subdivisions,
        clocks,
        deadline: None,
        tail: crate::DigitalTail::Running,
        monitor: None,
        accumulator: None,
        pending: None,
        elapsed: None,
        kind: crate::RegionKeyKind::BoundaryRegion,
        corner,
    })
    .map_err(ProbabilityDocumentError::Graph)
}

#[allow(clippy::too_many_arguments)]
fn next_mean_region_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    current: &MeanRegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    scale: u64,
    route: TimedProbabilityRoute,
    cancel: &Cancel,
) -> Result<Option<(ProbabilityProductKey, MeanRegionState, Rational)>, ProbabilityDocumentError> {
    let current_key = mean_region_key(
        system,
        timed,
        binding,
        current,
        caps,
        base_scale,
        subdivisions,
        scale,
        route,
    )?;
    let delta = rational_units(1, scale)?;
    let max_steps = if route == TimedProbabilityRoute::Digital {
        1
    } else {
        caps.iter()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_add(subdivisions.saturating_mul(2))
            .saturating_add(1)
    };
    let mut state = current.timed.clone();
    for elapsed in 1..=max_steps {
        poll(cancel)?;
        let Some(delayed) = timed
            .delay(&state, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        else {
            return Ok(None);
        };
        let reached = MeanRegionState {
            target: mean_target(system, predicate, binding, delayed.discrete(), cancel)?,
            timed: TimedState::new(
                delayed.discrete().clone(),
                ClockValuation::new(quantized_clocks(&delayed, caps, scale)?),
            ),
        };
        let key = mean_region_key(
            system,
            timed,
            binding,
            &reached,
            caps,
            base_scale,
            subdivisions,
            scale,
            route,
        )?;
        if route == TimedProbabilityRoute::Digital && key == current_key {
            return Ok(None);
        }
        if key != current_key || elapsed == max_steps {
            let cost = rational_units(elapsed, scale)?;
            return Ok(Some((key, reached, cost)));
        }
        state = delayed;
    }
    Err(ProbabilityDocumentError::Unsupported)
}

fn admit_mean_region_state(
    key: ProbabilityProductKey,
    state: MeanRegionState,
    reached: &mut BTreeMap<ProbabilityProductKey, MeanRegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
    limits: ProbabilityDocumentLimits,
    storage: &mut u64,
) -> Result<(), ProbabilityDocumentError> {
    if reached.contains_key(&key) {
        return Ok(());
    }
    let required = u64::try_from(reached.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if required > u64::try_from(limits.states).unwrap_or(u64::MAX) {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::States,
            bound: u64::try_from(limits.states).unwrap_or(u64::MAX),
            actual: required,
        });
    }
    reserve_storage(
        storage,
        key.as_bytes()
            .len()
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, MeanRegionState)>()),
        limits,
    )?;
    reached.insert(key.clone(), state);
    pending.push_back(key);
    Ok(())
}

fn deadline_event(
    claim: &CheckedProbabilisticClaim,
) -> Option<(Bound, &fsl_semantics::check::CheckedPredicate)> {
    let fsl_semantics::check::ProbForm::Probability { bound, event } = &claim.form else {
        return None;
    };
    let fsl_semantics::check::Event::Formula {
        formula,
        timed_horizon: Some(_),
        ..
    } = event
    else {
        return None;
    };
    let root = formula.formula().root();
    let fsl_semantics::check::FormulaNode::Unary {
        operator: fsl_semantics::check::UnaryTemporal::Eventually,
        interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
        operand,
    } = formula.formula().nodes().get(root.index())?
    else {
        return None;
    };
    if !interval.lower().is_zero() || !interval.lower_closed() || !interval.upper_closed() {
        return None;
    }
    let fsl_semantics::check::FormulaNode::Holds { .. } =
        formula.formula().nodes().get(operand.index())?
    else {
        return None;
    };
    Some((bound.clone(), formula.condition(operand.index())?))
}

/// The independently checked FR-204 bounded monitor shape. Operator node
/// positions are retained so nested occurrences remain distinct in the key.
fn bounded_event(claim: &CheckedProbabilisticClaim, scale: u64) -> Option<(Bound, Watch<'_>)> {
    use fsl_semantics::check::{
        BinaryTemporal, CheckedInterval, FormulaNode, ProbForm, UnaryTemporal,
    };
    let ProbForm::Probability {
        bound,
        event:
            fsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
    } = &claim.form
    else {
        return None;
    };
    let checked = formula.formula();
    let nodes = checked.nodes();
    let root_id = checked.root();
    let root_index = root_id.index();
    let (root_kind, root_interval, operands) = match nodes.get(root_index)? {
        FormulaNode::Unary {
            operator: UnaryTemporal::Eventually,
            interval: Some(CheckedInterval::Timed(interval)),
            operand,
        } => {
            if let Some(FormulaNode::Unary {
                operator: UnaryTemporal::Always,
                interval: Some(CheckedInterval::Timed(inner)),
                operand: atom,
            }) = nodes.get(operand.index())
            {
                let inner_upper = inner.upper()?;
                if inner.lower() != inner_upper
                    || !interval.lower_closed()
                    || !interval.upper_closed()
                    || !inner.lower_closed()
                    || !inner.upper_closed()
                {
                    return None;
                }
                if !matches!(nodes.get(atom.index())?, FormulaNode::Holds { .. }) {
                    return None;
                }
                let outer_upper = interval.upper()?;
                let root = fsl_eval::probabilistic::TimedOperatorState {
                    occurrence: u64::try_from(root_index).ok()?,
                    kind: fsl_eval::probabilistic::DeadlineKind::Eventually,
                    lower: time_units(interval.lower(), scale)?,
                    upper: time_units(outer_upper, scale)?,
                    elapsed: 0,
                    phase: fsl_eval::probabilistic::DeadlinePhase::Pending,
                };
                let nested = fsl_eval::probabilistic::TimedOperatorState {
                    occurrence: u64::try_from(operand.index()).ok()?,
                    kind: fsl_eval::probabilistic::DeadlineKind::Always,
                    lower: time_units(inner.lower(), scale)?,
                    upper: time_units(inner_upper, scale)?,
                    elapsed: 0,
                    phase: fsl_eval::probabilistic::DeadlinePhase::Pending,
                };
                let horizon = root.upper.checked_add(nested.upper)?;
                let predicate = formula.condition(atom.index())?;
                return Some((
                    bound.clone(),
                    Watch::Nested {
                        root,
                        nested,
                        predicate,
                        horizon,
                    },
                ));
            }
            (
                fsl_eval::probabilistic::DeadlineKind::Eventually,
                interval,
                vec![*operand],
            )
        }
        FormulaNode::Unary {
            operator: UnaryTemporal::Always,
            interval: Some(CheckedInterval::Timed(interval)),
            operand,
        } => (
            fsl_eval::probabilistic::DeadlineKind::Always,
            interval,
            vec![*operand],
        ),
        FormulaNode::Binary {
            operator: BinaryTemporal::Until,
            interval: Some(CheckedInterval::Timed(interval)),
            left,
            right,
        } => (
            fsl_eval::probabilistic::DeadlineKind::Until,
            interval,
            vec![*left, *right],
        ),
        _ => return None,
    };
    if !root_interval.lower_closed() || !root_interval.upper_closed() {
        return None;
    }
    let upper = root_interval.upper()?;
    let predicates = operands
        .into_iter()
        .map(|operand| {
            matches!(nodes.get(operand.index())?, FormulaNode::Holds { .. })
                .then(|| formula.condition(operand.index()))?
        })
        .collect::<Option<Vec<_>>>()?;
    let horizon = time_units(upper, scale)?;
    let root = fsl_eval::probabilistic::TimedOperatorState {
        occurrence: u64::try_from(root_index).ok()?,
        kind: root_kind,
        lower: time_units(root_interval.lower(), scale)?,
        upper: horizon,
        elapsed: 0,
        phase: fsl_eval::probabilistic::DeadlinePhase::Pending,
    };
    Some((
        bound.clone(),
        Watch::Bounded {
            root,
            predicates,
            horizon,
        },
    ))
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Pending,
    Accepted,
    Rejected,
    /// An elapsed measure before `A` held.
    Inactive,
    /// An elapsed measure after `A`, before `B`.
    Active,
    /// `B` ended an activated elapsed measure.
    Complete,
    /// The elapsed clock passed the window first.
    Censored,
    /// One checked timed operator occurrence.
    Bounded(fsl_eval::probabilistic::TimedOperatorState),
    /// A nested formula retains both checked operator occurrences.
    NestedBounded(
        fsl_eval::probabilistic::TimedOperatorState,
        fsl_eval::probabilistic::TimedOperatorState,
    ),
}

impl Phase {
    fn decided(self) -> bool {
        match self {
            Self::Pending | Self::Inactive | Self::Active => false,
            Self::Bounded(operator) | Self::NestedBounded(operator, _) => {
                operator.phase != fsl_eval::probabilistic::DeadlinePhase::Pending
            }
            _ => true,
        }
    }
}

fn phase_value(phase: Phase) -> Option<bool> {
    use fsl_eval::probabilistic::DeadlinePhase as MonitorPhase;
    match phase {
        Phase::Accepted => Some(true),
        Phase::Rejected => Some(false),
        Phase::Bounded(operator) => match operator.phase {
            MonitorPhase::Accept => Some(true),
            MonitorPhase::Reject => Some(false),
            MonitorPhase::Pending => None,
        },
        Phase::NestedBounded(root, _) => match root.phase {
            MonitorPhase::Accept => Some(true),
            MonitorPhase::Reject => Some(false),
            MonitorPhase::Pending => None,
        },
        _ => None,
    }
}

fn monitor_json(phase: Phase) -> JsonValue {
    use fsl_eval::probabilistic::{DeadlineKind, DeadlinePhase, TimedOperatorState};
    let kind_name = |kind| match kind {
        DeadlineKind::Eventually => "eventually",
        DeadlineKind::Always => "always",
        DeadlineKind::Until => "until",
    };
    let phase_name = |phase| match phase {
        DeadlinePhase::Pending => "pending",
        DeadlinePhase::Accept => "accept",
        DeadlinePhase::Reject => "reject",
    };
    let encode_operator = |operator: TimedOperatorState| {
        json!([
            operator.occurrence.to_string(),
            kind_name(operator.kind),
            operator.lower.to_string(),
            operator.upper.to_string(),
            operator.elapsed.to_string(),
            phase_name(operator.phase)
        ])
    };
    match phase {
        Phase::Bounded(operator) => {
            json!([
                "deadline",
                kind_name(operator.kind),
                phase_name(operator.phase)
            ])
        }
        Phase::NestedBounded(root, nested) => {
            json!([
                "timed-formula",
                encode_operator(root),
                Some(encode_operator(nested))
            ])
        }
        Phase::Pending => json!(["deadline", "eventually", "pending"]),
        Phase::Accepted => json!(["deadline", "eventually", "accept"]),
        Phase::Rejected => json!(["deadline", "eventually", "reject"]),
        _ => JsonValue::Null,
    }
}

/// FR-877's elapsed measure and its quantile transform, as the checker
/// reads them from the checked claim.
struct ElapsedWatch<'a> {
    from: &'a fsl_semantics::check::CheckedPredicate,
    until: &'a fsl_semantics::check::CheckedPredicate,
    q: Rational,
    threshold: u64,
    strict: bool,
}

/// What the enumerator observes along a run: FR-204's deadline event, or
/// FR-877's elapsed measure.
enum Watch<'a> {
    Deadline(&'a fsl_semantics::check::CheckedPredicate),
    Bounded {
        root: fsl_eval::probabilistic::TimedOperatorState,
        predicates: Vec<&'a fsl_semantics::check::CheckedPredicate>,
        horizon: u64,
    },
    Nested {
        root: fsl_eval::probabilistic::TimedOperatorState,
        nested: fsl_eval::probabilistic::TimedOperatorState,
        predicate: &'a fsl_semantics::check::CheckedPredicate,
        horizon: u64,
    },
    Elapsed(ElapsedWatch<'a>),
}

impl Watch<'_> {
    fn start(&self) -> Phase {
        match self {
            Self::Deadline(_) => Phase::Pending,
            Self::Bounded { root, .. } => Phase::Bounded(*root),
            Self::Nested { root, nested, .. } => {
                let mut root = *root;
                // The analyzer creates the nested monitor at the initial
                // state without consuming its predicate letter.
                root.elapsed = u64::MAX;
                Phase::NestedBounded(root, *nested)
            }
            Self::Elapsed(_) => Phase::Inactive,
        }
    }

    fn horizon(&self) -> u64 {
        match self {
            Self::Bounded { horizon, .. } | Self::Nested { horizon, .. } => *horizon,
            _ => 0,
        }
    }

    /// The phase and the clock after reading `state`, `previous` being the
    /// phase before the move and `elapsed` the clock after it.
    #[allow(clippy::too_many_arguments)]
    fn observe(
        &self,
        system: &ModelSystem<'_>,
        binding: Option<&filament_exact::ObjectReference>,
        state: &ModelSimState,
        previous: Phase,
        elapsed: u64,
        deadline: u64,
        cancel: &Cancel,
    ) -> Result<(Phase, u64), ProbabilityDocumentError> {
        let watch = match self {
            Self::Deadline(predicate) => {
                let phase = phase_at(system, predicate, binding, state, elapsed, deadline, cancel)?;
                return Ok((phase, elapsed));
            }
            Self::Bounded {
                root, predicates, ..
            } => {
                let mut operator = match previous {
                    Phase::Bounded(operator) => operator,
                    _ => *root,
                };
                if operator.phase == fsl_eval::probabilistic::DeadlinePhase::Pending {
                    let truths = read_truths(system, predicates, binding, state, cancel)?;
                    operator.phase = deadline_read(
                        operator.kind,
                        operator.phase,
                        &truths,
                        elapsed,
                        operator.lower,
                    );
                    if elapsed > operator.upper {
                        operator.phase = deadline_timeout(operator.kind);
                    }
                }
                operator.elapsed = elapsed.min(operator.upper);
                return Ok((Phase::Bounded(operator), elapsed));
            }
            Self::Nested {
                root,
                nested,
                predicate,
                ..
            } => {
                let (mut root, mut nested) = match previous {
                    Phase::NestedBounded(root, nested) => (root, nested),
                    _ => (*root, *nested),
                };
                if root.elapsed == u64::MAX {
                    // Still evaluate the initial predicate independently so
                    // undefined/refused readings cannot enter a certificate,
                    // but mirror the analyzer's pending initial monitor.
                    let _ = read_truths(system, &[*predicate], binding, state, cancel)?;
                    root.elapsed = elapsed;
                    return Ok((Phase::NestedBounded(root, nested), elapsed));
                }
                if root.phase == fsl_eval::probabilistic::DeadlinePhase::Pending {
                    let truths = read_truths(system, &[*predicate], binding, state, cancel)?;
                    let truth = truths.first().copied().unwrap_or(false);
                    let lower = root.lower.saturating_add(nested.lower);
                    let upper = root.upper.saturating_add(nested.upper);
                    if (lower..=upper).contains(&elapsed) {
                        nested.elapsed = elapsed.saturating_sub(root.lower);
                        nested.phase = if truth {
                            root.phase = fsl_eval::probabilistic::DeadlinePhase::Accept;
                            fsl_eval::probabilistic::DeadlinePhase::Accept
                        } else {
                            fsl_eval::probabilistic::DeadlinePhase::Reject
                        };
                    } else {
                        nested.elapsed = elapsed.saturating_sub(root.lower).min(nested.upper);
                        nested.phase = fsl_eval::probabilistic::DeadlinePhase::Pending;
                    }
                    if elapsed > upper {
                        root.phase = fsl_eval::probabilistic::DeadlinePhase::Reject;
                    }
                }
                root.elapsed = elapsed;
                return Ok((Phase::NestedBounded(root, nested), elapsed));
            }
            Self::Elapsed(watch) => watch,
        };
        let holds = |predicate| -> Result<bool, ProbabilityDocumentError> {
            let readings = system
                .state_predicates_with_cancel(state, &[(predicate, binding)], cancel)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            match readings.as_slice() {
                [fsl_eval::simulation::model::PredicateReading::True] => Ok(true),
                [fsl_eval::simulation::model::PredicateReading::False] => Ok(false),
                _ => Err(ProbabilityDocumentError::Unsupported),
            }
        };
        Ok(match previous {
            Phase::Inactive => {
                if !holds(watch.from)? {
                    (Phase::Inactive, 0)
                } else if holds(watch.until)? {
                    (Phase::Complete, 0)
                } else {
                    (Phase::Active, 0)
                }
            }
            Phase::Active => {
                if elapsed > deadline {
                    (Phase::Censored, deadline.saturating_add(1))
                } else if holds(watch.until)? {
                    (Phase::Complete, elapsed)
                } else {
                    (Phase::Active, elapsed)
                }
            }
            other => (other, elapsed),
        })
    }

    /// FR-877's terminal reward `[M <= c] - q` (`[M < c] - q` when strict) of
    /// a decided elapsed measure; a censored measure exceeds every threshold.
    fn reward(&self, phase: Phase, elapsed: u64) -> Option<(Rational, bool)> {
        let Self::Elapsed(watch) = self else {
            return None;
        };
        let within = match phase {
            Phase::Complete if watch.strict => elapsed < watch.threshold,
            Phase::Complete => elapsed <= watch.threshold,
            Phase::Censored => false,
            _ => return None,
        };
        let hit = if within { one() } else { zero() };
        Some((hit.sub(&watch.q), within))
    }
}

fn read_truths(
    system: &ModelSystem<'_>,
    predicates: &[&fsl_semantics::check::CheckedPredicate],
    binding: Option<&filament_exact::ObjectReference>,
    state: &ModelSimState,
    cancel: &Cancel,
) -> Result<Vec<bool>, ProbabilityDocumentError> {
    let readings = predicates
        .iter()
        .map(|predicate| (*predicate, binding))
        .collect::<Vec<_>>();
    system
        .state_predicates_with_cancel(state, &readings, cancel)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .into_iter()
        .map(|reading| match reading {
            fsl_eval::simulation::model::PredicateReading::True => Ok(true),
            fsl_eval::simulation::model::PredicateReading::False => Ok(false),
            fsl_eval::simulation::model::PredicateReading::Undefined { .. }
            | fsl_eval::simulation::model::PredicateReading::Refused(_) => {
                Err(ProbabilityDocumentError::Unsupported)
            }
        })
        .collect()
}

fn deadline_read(
    kind: fsl_eval::probabilistic::DeadlineKind,
    phase: fsl_eval::probabilistic::DeadlinePhase,
    truths: &[bool],
    elapsed: u64,
    lower: u64,
) -> fsl_eval::probabilistic::DeadlinePhase {
    use fsl_eval::probabilistic::{DeadlineKind as Kind, DeadlinePhase as Phase};
    if phase != Phase::Pending {
        return phase;
    }
    match (kind, truths) {
        (Kind::Eventually, [target]) if elapsed >= lower && *target => Phase::Accept,
        (Kind::Always, [target]) if elapsed >= lower && !*target => Phase::Reject,
        (Kind::Until, [from, until]) => {
            if elapsed >= lower && *until {
                Phase::Accept
            } else if !*from {
                Phase::Reject
            } else {
                Phase::Pending
            }
        }
        _ => Phase::Pending,
    }
}

fn deadline_timeout(
    kind: fsl_eval::probabilistic::DeadlineKind,
) -> fsl_eval::probabilistic::DeadlinePhase {
    match kind {
        fsl_eval::probabilistic::DeadlineKind::Always => {
            fsl_eval::probabilistic::DeadlinePhase::Accept
        }
        fsl_eval::probabilistic::DeadlineKind::Eventually
        | fsl_eval::probabilistic::DeadlineKind::Until => {
            fsl_eval::probabilistic::DeadlinePhase::Reject
        }
    }
}

#[derive(Clone)]
struct RegionState {
    timed: TimedState,
    elapsed: u64,
    phase: Phase,
    tail: DigitalTail,
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn enumerate_digital_deadline(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    watch: &Watch<'_>,
    deadline: u64,
    key_scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    terminals: &mut BTreeMap<ProbabilityProductKey, (Rational, bool)>,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let mut caps = Vec::with_capacity(timed.places().len());
    for place in timed.places() {
        let mut largest = 0_u64;
        for constraint in clock_constraints(timed)
            .into_iter()
            .filter(|constraint| constraint.clock() == place.clock())
        {
            largest = largest.max(
                time_units(constraint.bound(), key_scale)
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            );
        }
        caps.push(
            largest
                .checked_add(1)
                .ok_or(ProbabilityDocumentError::Unsupported)?,
        );
    }
    let mut initial = timed.initial();
    if initial.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let state = initial.pop().ok_or(ProbabilityDocumentError::Unsupported)?;
    let (phase, elapsed) = watch.observe(
        system,
        binding,
        state.discrete(),
        watch.start(),
        0,
        deadline,
        cancel,
    )?;
    let first = RegionState {
        timed: state,
        elapsed,
        phase,
        tail: DigitalTail::Running,
    };
    let first_key = digital_key(system, timed, binding, &first, &caps, key_scale, deadline)?;
    reserve_storage(
        storage,
        first_key
            .as_bytes()
            .len()
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, RegionState)>()),
        limits,
    )?;
    let mut pending = VecDeque::from([first_key.clone()]);
    let mut reached = BTreeMap::from([(first_key.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut edges = 0_u64;
    while let Some(key) = pending.pop_front() {
        charge_work(meter, cancel, 1)?;
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let current = reached
            .get(&key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if let Some(value) = phase_value(current.phase) {
            insert_checker_state(
                key,
                CheckerState {
                    terminal: Some(value),
                    choices: Vec::new(),
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        match current.phase {
            Phase::Accepted => {
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(true),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Rejected => {
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Complete | Phase::Censored => {
                if let Some(reward) = watch.reward(current.phase, current.elapsed) {
                    terminals.insert(key.clone(), reward);
                }
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Pending
            | Phase::Inactive
            | Phase::Active
            | Phase::Bounded(_)
            | Phase::NestedBounded(_, _) => {}
        }
        if current.tail != DigitalTail::Running {
            let mut choices = Vec::new();
            if let Some((target, delayed)) = digital_idle_delay(
                system, timed, watch, binding, &current, &caps, key_scale, deadline, cancel,
            )? {
                admit_region_state(
                    target.clone(),
                    delayed,
                    &mut reached,
                    &mut pending,
                    limits,
                    storage,
                )?;
                choices.push(vec![ProbabilityEdge {
                    target,
                    probability: one(),
                }]);
                edges = edges.saturating_add(1);
            }
            if current.tail == DigitalTail::IdleDelayed {
                let mut observed = current.clone();
                observed.tail = DigitalTail::IdleReady;
                let target = digital_key(
                    system, timed, binding, &observed, &caps, key_scale, deadline,
                )?;
                admit_region_state(
                    target.clone(),
                    observed,
                    &mut reached,
                    &mut pending,
                    limits,
                    storage,
                )?;
                choices.push(vec![ProbabilityEdge {
                    target,
                    probability: one(),
                }]);
                edges = edges.saturating_add(1);
            }
            insert_checker_state(
                key,
                CheckerState {
                    terminal: None,
                    choices,
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        let actions = system
            .actions_with_cancel(current.timed.discrete(), cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        admit_workload(system, claim, &actions)?;
        let enabled = digital_enabled_actions(system, timed, &current.timed, claim, cancel)?;
        if matches!(watch, Watch::Deadline(_))
            && matches!(
                claim.scheduler,
                fsl_semantics::check::ProbScheduler::Workload(_)
            )
        {
            let run = WorkloadRun {
                system,
                timed,
                claim,
                watch,
                binding,
                caps: &caps,
                scale: key_scale,
                deadline,
                limits,
                cancel,
            };
            let choices = digital_workload_choices(
                &run,
                &current,
                &enabled,
                meter,
                storage,
                &mut reached,
                &mut pending,
            )?;
            edges = edges.saturating_add(
                choices
                    .iter()
                    .map(|choice| u64::try_from(choice.len()).unwrap_or(u64::MAX))
                    .fold(0_u64, u64::saturating_add),
            );
            if edges > u64::try_from(limits.edges).unwrap_or(u64::MAX) {
                return Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::Edges,
                    bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
                    actual: edges,
                });
            }
            if choices.is_empty() && current.phase == Phase::Active {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            insert_checker_state(
                key,
                CheckerState {
                    terminal: choices.is_empty().then_some(false),
                    choices,
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        let delta = rational_units(1, key_scale)?;

        let can_delay = timed
            .delay(&current.timed, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .is_some();
        let mut given_choices = Vec::with_capacity(actions.len());
        for action in &actions {
            if let Some(support) = discrete_delay(system, &action.identity.operation) {
                let (choice, admitted_edges) = digital_discrete_delay_choice(
                    system,
                    timed,
                    claim,
                    watch,
                    binding,
                    &current,
                    action,
                    support,
                    &caps,
                    key_scale,
                    deadline,
                    limits,
                    cancel,
                    meter,
                    storage,
                    &mut reached,
                    &mut pending,
                )?;
                edges = edges.saturating_add(admitted_edges);
                given_choices.push(Some(choice));
            } else {
                given_choices.push(None);
            }
        }
        let given_actions = given_choices
            .iter()
            .filter(|choice| {
                choice
                    .as_ref()
                    .is_some_and(|rows| rows.iter().any(|edges| !edges.is_empty()))
            })
            .count();
        let has_free_action = enabled
            .iter()
            .any(|action| discrete_delay(system, &action.identity.operation).is_none());
        if given_actions == 0 && !has_free_action && !can_delay {
            if current.phase == Phase::Active {
                // A time-lock under a running measure has no specified reward.
                return Err(ProbabilityDocumentError::Unsupported);
            }
            insert_checker_state(
                key,
                CheckerState {
                    terminal: Some(false),
                    choices: Vec::new(),
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        if given_actions > 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut choices = Vec::new();
        if given_actions == 0 {
            for choice in digital_time_choices(
                system,
                timed,
                watch,
                binding,
                &current,
                &caps,
                key_scale,
                deadline,
                limits,
                cancel,
                meter,
                storage,
                &mut reached,
                &mut pending,
            )? {
                edges = edges.saturating_add(1);
                choices.push(choice);
            }
        }
        let clocked_steps = timed
            .steps(&current.timed)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        for (action, given_choice) in actions.into_iter().zip(given_choices) {
            charge_work(meter, cancel, 1)?;
            if let Some(rows) = given_choice {
                for row in rows {
                    if !row.is_empty() {
                        choices.push(row);
                    }
                }
                continue;
            }
            let mut choice: Vec<ProbabilityEdge> = Vec::new();
            let mut enabled = true;
            for draw in action.draws {
                if draw.successors.len() != 1 {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let (transition, discrete_post) = &draw.successors[0];
                let Some((_, timed_post)) = clocked_steps.iter().find(|(step, post)| {
                    step.transition() == transition
                        && system
                            .state_key(post.discrete())
                            .ok()
                            .zip(system.state_key(discrete_post).ok())
                            .is_some_and(|(left, right)| left.0 == right.0)
                }) else {
                    enabled = false;
                    break;
                };
                let clocks = quantized_clocks(timed_post, &caps, key_scale)?;
                let post =
                    TimedState::new(timed_post.discrete().clone(), ClockValuation::new(clocks));
                let (phase, elapsed) = watch.observe(
                    system,
                    binding,
                    post.discrete(),
                    current.phase,
                    current.elapsed,
                    deadline,
                    cancel,
                )?;
                let reached_state = RegionState {
                    timed: post,
                    elapsed,
                    phase,
                    tail: DigitalTail::Running,
                };
                let target = digital_key(
                    system,
                    timed,
                    binding,
                    &reached_state,
                    &caps,
                    key_scale,
                    deadline,
                )?;
                admit_region_state(
                    target.clone(),
                    reached_state,
                    &mut reached,
                    &mut pending,
                    limits,
                    storage,
                )?;
                if let Some(edge) = choice.iter_mut().find(|edge| edge.target == target) {
                    edge.probability = edge.probability.add(&draw.probability);
                } else {
                    choice.push(ProbabilityEdge {
                        target,
                        probability: draw.probability.clone(),
                    });
                }
                edges = edges.saturating_add(1);
                charge_work(meter, cancel, 1)?;
            }
            if enabled && !choice.is_empty() {
                choices.push(choice);
            }
        }
        if edges > u64::try_from(limits.edges).unwrap_or(u64::MAX) {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
                actual: edges,
            });
        }
        if choices.is_empty() && current.phase == Phase::Active {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        insert_checker_state(
            key,
            CheckerState {
                terminal: choices.is_empty().then_some(false),
                choices,
            },
            &mut states,
            storage,
            limits,
        )?;
    }
    Ok(CheckerGraph {
        initial: first_key,
        states,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn enumerate_region_deadline(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    watch: &Watch<'_>,
    deadline: u64,
    key_scale: u64,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    terminals: &mut BTreeMap<ProbabilityProductKey, (Rational, bool)>,
    activated: &mut BTreeSet<ProbabilityProductKey>,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let (base_scale, subdivisions) = region_scale_parts(timed, claim, scale)?;
    let mut caps = Vec::with_capacity(timed.places().len());
    for place in timed.places() {
        let mut largest = 0_u64;
        for constraint in clock_constraints(timed)
            .into_iter()
            .filter(|constraint| constraint.clock() == place.clock())
        {
            largest = largest.max(
                time_units(constraint.bound(), key_scale)
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            );
        }
        caps.push(
            largest
                .checked_add(1)
                .ok_or(ProbabilityDocumentError::Unsupported)?,
        );
    }
    let mut initial = timed.initial();
    if initial.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let state = initial.pop().ok_or(ProbabilityDocumentError::Unsupported)?;
    let (phase, elapsed) = watch.observe(
        system,
        binding,
        state.discrete(),
        watch.start(),
        0,
        deadline,
        cancel,
    )?;
    let first = RegionState {
        timed: state,
        elapsed,
        phase,
        tail: DigitalTail::Running,
    };
    let first_key = region_key(
        system,
        timed,
        binding,
        &first,
        &caps,
        base_scale,
        subdivisions,
        key_scale,
        deadline,
    )?;
    reserve_storage(
        storage,
        first_key
            .as_bytes()
            .len()
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, RegionState)>()),
        limits,
    )?;
    let mut pending = VecDeque::from([first_key.clone()]);
    let mut reached = BTreeMap::from([(first_key.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut edges = 0_u64;
    while let Some(key) = pending.pop_front() {
        charge_work(meter, cancel, 1)?;
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let current = reached
            .get(&key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if let Some(value) = phase_value(current.phase) {
            insert_checker_state(
                key,
                CheckerState {
                    terminal: Some(value),
                    choices: Vec::new(),
                },
                &mut states,
                storage,
                limits,
            )?;
            continue;
        }
        match current.phase {
            Phase::Accepted => {
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(true),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Rejected => {
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Complete | Phase::Censored => {
                activated.insert(key.clone());
                if let Some(reward) = watch.reward(current.phase, current.elapsed) {
                    terminals.insert(key.clone(), reward);
                }
                insert_checker_state(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                    &mut states,
                    storage,
                    limits,
                )?;
                continue;
            }
            Phase::Pending
            | Phase::Inactive
            | Phase::Active
            | Phase::Bounded(_)
            | Phase::NestedBounded(_, _) => {
                if current.phase == Phase::Active {
                    activated.insert(key.clone());
                }
            }
        }
        let mut choices = Vec::new();
        if let Some((delayed_key, delayed)) = next_region_delay(
            system,
            timed,
            watch,
            binding,
            &current,
            &caps,
            base_scale,
            subdivisions,
            key_scale,
            deadline,
            cancel,
        )? {
            admit_region_state(
                delayed_key.clone(),
                delayed,
                &mut reached,
                &mut pending,
                limits,
                storage,
            )?;
            choices.push(vec![ProbabilityEdge {
                target: delayed_key,
                probability: one(),
            }]);
            edges = edges.saturating_add(1);
            charge_work(meter, cancel, 1)?;
        }
        let clocked_steps = timed
            .steps(&current.timed)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let actions = system
            .actions_with_cancel(current.timed.discrete(), cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        admit_workload(system, claim, &actions)?;
        for action in actions {
            charge_work(meter, cancel, 1)?;
            let mut choice: Vec<ProbabilityEdge> = Vec::new();
            let mut enabled = true;
            for draw in action.draws {
                if draw.successors.len() != 1 {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let (transition, discrete_post) = &draw.successors[0];
                let Some((_, timed_post)) = clocked_steps.iter().find(|(step, post)| {
                    step.transition() == transition
                        && system
                            .state_key(post.discrete())
                            .ok()
                            .zip(system.state_key(discrete_post).ok())
                            .is_some_and(|(left, right)| left.0 == right.0)
                }) else {
                    enabled = false;
                    break;
                };
                let clocks = quantized_clocks(timed_post, &caps, key_scale)?;
                let post =
                    TimedState::new(timed_post.discrete().clone(), ClockValuation::new(clocks));
                let (phase, elapsed) = watch.observe(
                    system,
                    binding,
                    post.discrete(),
                    current.phase,
                    current.elapsed,
                    deadline,
                    cancel,
                )?;
                let reached_state = RegionState {
                    timed: post,
                    elapsed,
                    phase,
                    tail: DigitalTail::Running,
                };
                let target = region_key(
                    system,
                    timed,
                    binding,
                    &reached_state,
                    &caps,
                    base_scale,
                    subdivisions,
                    key_scale,
                    deadline,
                )?;
                admit_region_state(
                    target.clone(),
                    reached_state,
                    &mut reached,
                    &mut pending,
                    limits,
                    storage,
                )?;
                if let Some(edge) = choice.iter_mut().find(|edge| edge.target == target) {
                    edge.probability = edge.probability.add(&draw.probability);
                } else {
                    choice.push(ProbabilityEdge {
                        target,
                        probability: draw.probability.clone(),
                    });
                }
                edges = edges.saturating_add(1);
                charge_work(meter, cancel, 1)?;
            }
            if enabled && !choice.is_empty() {
                choices.push(choice);
            }
        }
        if edges > u64::try_from(limits.edges).unwrap_or(u64::MAX) {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
                actual: edges,
            });
        }
        if choices.is_empty() && current.phase == Phase::Active {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        insert_checker_state(
            key,
            CheckerState {
                terminal: choices.is_empty().then_some(false),
                choices,
            },
            &mut states,
            storage,
            limits,
        )?;
    }
    Ok(CheckerGraph {
        initial: first_key,
        states,
    })
}

fn region_scale_parts(
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    full_scale: u64,
) -> Result<(u64, u64), ProbabilityDocumentError> {
    let deadline = matches!(
        &claim.form,
        fsl_semantics::check::ProbForm::Probability {
            event: fsl_semantics::check::Event::Formula {
                timed_horizon: Some(_),
                ..
            },
            ..
        }
    ) || elapsed::query(claim).is_some();
    let subdivisions = u64::try_from(timed.places().len())
        .ok()
        .and_then(|clocks| clocks.checked_add(if deadline { 1 } else { 0 }))
        .and_then(|clocks| clocks.checked_add(1))
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let base = full_scale
        .checked_div(subdivisions)
        .filter(|base| *base > 0)
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if base.checked_mul(subdivisions) != Some(full_scale) {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Ok((base, subdivisions))
}

fn time_bound(
    claim: &CheckedProbabilisticClaim,
    scale: u64,
) -> Result<u64, ProbabilityDocumentError> {
    let fsl_semantics::check::ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
        ..
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let root = formula.formula().root();
    let Some(fsl_semantics::check::FormulaNode::Unary {
        interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
        ..
    }) = formula.formula().nodes().get(root.index())
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let upper = interval
        .upper()
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    time_units(upper, scale).ok_or(ProbabilityDocumentError::Unsupported)
}

fn time_units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

fn rational_units(value: u64, scale: u64) -> Result<Rational, ProbabilityDocumentError> {
    Rational::new(Integer::from(value), Integer::from(scale))
        .map_err(|_| ProbabilityDocumentError::Unsupported)
}

fn quantized_clocks(
    state: &TimedState,
    caps: &[u64],
    scale: u64,
) -> Result<Vec<Rational>, ProbabilityDocumentError> {
    state
        .clocks()
        .values()
        .iter()
        .zip(caps)
        .map(|(value, cap)| {
            let value = time_units(value, scale)
                .ok_or(ProbabilityDocumentError::Unsupported)?
                .min(*cap);
            rational_units(value, scale)
        })
        .collect()
}

fn phase_at(
    system: &ModelSystem<'_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    state: &ModelSimState,
    elapsed: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Phase, ProbabilityDocumentError> {
    if elapsed > deadline {
        return Ok(Phase::Rejected);
    }
    let readings = system
        .state_predicates_with_cancel(state, &[(predicate, binding)], cancel)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    let [reading] = readings.as_slice() else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    match reading {
        fsl_eval::simulation::model::PredicateReading::True => Ok(Phase::Accepted),
        fsl_eval::simulation::model::PredicateReading::False => Ok(Phase::Pending),
        fsl_eval::simulation::model::PredicateReading::Undefined { .. }
        | fsl_eval::simulation::model::PredicateReading::Refused(_) => {
            Err(ProbabilityDocumentError::Unsupported)
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn region_key(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    binding: Option<&filament_exact::ObjectReference>,
    state: &RegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    full_scale: u64,
    deadline: u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let (discrete_bytes, _) = system
        .state_key(state.timed.discrete())
        .map_err(ProbabilityDocumentError::KeyEncoding)?;
    let discrete: JsonValue =
        serde_json::from_slice(&discrete_bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let binding: Option<JsonValue> = binding
        .map(|reference| {
            let text = fsl_eval::simulation::model::reference_canonical_text(reference)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            serde_json::from_str(&text).map_err(|_| ProbabilityDocumentError::Malformed)
        })
        .transpose()?;
    let unit = json!([
        "declared",
        fsl_foundation::node_wire::node_key_to_json(&timed.source().unit_node)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
    ]);
    let mut clocks = Vec::with_capacity(timed.places().len());
    for ((place, value), cap) in timed
        .places()
        .iter()
        .zip(state.timed.clocks().values())
        .zip(caps)
    {
        let field = timed
            .model()
            .clock(place.clock())
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let owner = field.owner();
        let owner = owner.original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver = fsl_eval::simulation::model::reference_canonical_text(place.object())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver: JsonValue =
            serde_json::from_str(&receiver).map_err(|_| ProbabilityDocumentError::Malformed)?;
        let identity = json!([declaring, field.name(), receiver]);
        let value = time_units(value, full_scale).ok_or(ProbabilityDocumentError::Unsupported)?;
        clocks.push((identity, value, *cap));
    }
    // FR-920: an elapsed measure's `["measure",null,phase]`; its clock is
    // `null` before activation.
    let measure = match state.phase {
        Phase::Inactive => Some(("inactive", None)),
        Phase::Active => Some(("active", Some(state.elapsed))),
        Phase::Complete => Some(("complete", Some(state.elapsed))),
        Phase::Censored => Some(("censored", Some(state.elapsed))),
        _ => None,
    };
    let monitor = monitor_json(state.phase);
    let mut keyed_clocks = clocks
        .into_iter()
        .map(|(identity, value, cap)| {
            if identity.as_array().is_none_or(|parts| parts.len() != 3) || value > cap {
                return Err(ProbabilityDocumentError::Malformed);
            }
            let bytes =
                super::super::canonical_json(&identity).map_err(ProbabilityDocumentError::Graph)?;
            Ok((bytes, identity, value, cap))
        })
        .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
    keyed_clocks.sort_by(|left, right| left.0.cmp(&right.0));
    if keyed_clocks.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let mut fractions = keyed_clocks
        .iter()
        .filter_map(|(_, _, value, cap)| {
            (*value < *cap)
                .then_some(*value % subdivisions)
                .filter(|fraction| *fraction > 0)
        })
        .collect::<BTreeSet<_>>();
    if state.elapsed < deadline.saturating_add(subdivisions) && state.phase != Phase::Inactive {
        let fraction = state.elapsed % subdivisions;
        if fraction > 0 {
            fractions.insert(fraction);
        }
    }
    let fraction_rank = |value: u64, cap: u64| -> String {
        if value >= cap {
            return "0".to_owned();
        }
        let fraction = value % subdivisions;
        if fraction == 0 {
            return "0".to_owned();
        }
        fractions
            .iter()
            .position(|candidate| *candidate == fraction)
            .and_then(|position| u64::try_from(position + 1).ok())
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "0".to_owned())
    };
    let encoded_clocks = keyed_clocks
        .into_iter()
        .map(|(_, identity, value, cap)| {
            json!([
                identity,
                if value >= cap {
                    value.div_ceil(subdivisions)
                } else {
                    value / subdivisions
                }
                .to_string(),
                cap.div_ceil(subdivisions).to_string(),
                fraction_rank(value, cap)
            ])
        })
        .collect::<Vec<_>>();
    let deadline_key = match measure {
        Some((_, value)) => json!([
            "elapsed",
            (deadline / subdivisions).to_string(),
            value.map(|value| (value / subdivisions).to_string()),
            value.map_or_else(
                || "0".to_owned(),
                |value| fraction_rank(value, deadline.saturating_add(subdivisions))
            )
        ]),
        None => json!([
            "deadline",
            (deadline / subdivisions).to_string(),
            (state.elapsed / subdivisions).to_string(),
            fraction_rank(state.elapsed, deadline.saturating_add(subdivisions))
        ]),
    };
    let model = json!([
        "filament.simulation.state-key/v1",
        [
            "region",
            discrete,
            unit,
            base_scale.to_string(),
            encoded_clocks,
            deadline_key,
            "running"
        ]
    ]);
    let product = json!([
        "filament.probability-product-key/v1",
        binding,
        null,
        null,
        model,
        match measure {
            Some((name, _)) => json!(["measure", null, name]),
            None => monitor,
        },
        null,
        null
    ]);
    super::super::canonical_json(&product)
        .map(ProbabilityProductKey::from_canonical_bytes)
        .map_err(ProbabilityDocumentError::Graph)
}

fn digital_key(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    binding: Option<&filament_exact::ObjectReference>,
    state: &RegionState,
    caps: &[u64],
    scale: u64,
    deadline: u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let (discrete_bytes, _) = system
        .state_key(state.timed.discrete())
        .map_err(ProbabilityDocumentError::KeyEncoding)?;
    let discrete: JsonValue =
        serde_json::from_slice(&discrete_bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let binding: Option<JsonValue> = binding
        .map(|reference| {
            let text = fsl_eval::simulation::model::reference_canonical_text(reference)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            serde_json::from_str(&text).map_err(|_| ProbabilityDocumentError::Malformed)
        })
        .transpose()?;
    let unit = json!([
        "declared",
        fsl_foundation::node_wire::node_key_to_json(&timed.source().unit_node)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
    ]);
    let mut clocks = Vec::with_capacity(timed.places().len());
    for ((place, value), cap) in timed
        .places()
        .iter()
        .zip(state.timed.clocks().values())
        .zip(caps)
    {
        let field = timed
            .model()
            .clock(place.clock())
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let owner = field.owner();
        let owner = owner.original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver = fsl_eval::simulation::model::reference_canonical_text(place.object())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver: JsonValue =
            serde_json::from_str(&receiver).map_err(|_| ProbabilityDocumentError::Malformed)?;
        let identity = json!([declaring, field.name(), receiver]);
        let value = time_units(value, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
        clocks.push(DigitalClockKey {
            identity,
            value,
            cap: *cap,
        });
    }
    let measure = match state.phase {
        Phase::Inactive => Some(("inactive", None)),
        Phase::Active => Some(("active", Some(state.elapsed))),
        Phase::Complete => Some(("complete", Some(state.elapsed))),
        Phase::Censored => Some(("censored", Some(state.elapsed))),
        _ => None,
    };
    let monitor = monitor_json(state.phase);
    digital_probability_product_key(DigitalProductKeyInput {
        binding,
        discrete,
        unit,
        scale,
        clocks,
        deadline: measure.is_none().then_some(DigitalDeadlineKey {
            limit: deadline,
            elapsed: state.elapsed,
        }),
        tail: state.tail,
        monitor: Some(match measure {
            Some((name, _)) => json!(["measure", null, name]),
            None => monitor,
        }),
        accumulator: None,
        pending: None,
        elapsed: measure.map(|(_, value)| (deadline, value)),
    })
    .map_err(ProbabilityDocumentError::Graph)
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn next_digital_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    scale: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Option<(ProbabilityProductKey, RegionState)>, ProbabilityDocumentError> {
    let current_key = digital_key(system, timed, binding, current, caps, scale, deadline)?;
    let delta = rational_units(1, scale)?;
    let mut state = current.clone();
    while state.elapsed <= deadline {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let Some(delayed) = timed
            .delay(&state.timed, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        else {
            return Ok(None);
        };
        let elapsed = state
            .elapsed
            .saturating_add(1)
            .min(deadline.saturating_add(1));
        let clocks = quantized_clocks(&delayed, caps, scale)?;
        let delayed = TimedState::new(delayed.discrete().clone(), ClockValuation::new(clocks));
        let (phase, elapsed) = watch.observe(
            system,
            binding,
            delayed.discrete(),
            state.phase,
            elapsed,
            deadline,
            cancel,
        )?;
        let reached = RegionState {
            timed: delayed,
            elapsed,
            phase,
            tail: current.tail,
        };
        let key = digital_key(system, timed, binding, &reached, caps, scale, deadline)?;
        if key != current_key {
            return Ok(Some((key, reached)));
        }
        if reached.phase == Phase::Inactive {
            // An unactivated measure has no clock to move: no later delay differs.
            return Ok(None);
        }
        state = reached;
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn digital_idle_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    scale: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Option<(ProbabilityProductKey, RegionState)>, ProbabilityDocumentError> {
    let delta = rational_units(1, scale)?;
    let Some(delayed) = timed
        .delay(&current.timed, &delta)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
    else {
        return Ok(None);
    };
    let elapsed = current
        .elapsed
        .saturating_add(1)
        .min(deadline.saturating_add(1));
    let delayed = TimedState::new(
        delayed.discrete().clone(),
        ClockValuation::new(quantized_clocks(&delayed, caps, scale)?),
    );
    let (phase, elapsed) = watch.observe(
        system,
        binding,
        delayed.discrete(),
        current.phase,
        elapsed,
        deadline,
        cancel,
    )?;
    let state = RegionState {
        timed: delayed,
        elapsed,
        phase,
        tail: DigitalTail::IdleDelayed,
    };
    let key = digital_key(system, timed, binding, &state, caps, scale, deadline)?;
    Ok(Some((key, state)))
}

/// Whether `objective` is one of the three workload spellings.
pub(in crate::probability_certificate) fn workload_objective(
    objective: ProbabilityObjective,
) -> bool {
    matches!(
        objective,
        ProbabilityObjective::Workload
            | ProbabilityObjective::WorkloadMinimum
            | ProbabilityObjective::WorkloadMaximum
    )
}

/// The objective a workload claim's document states: `workload` when the
/// workload leaves the scheduler no choice, otherwise the extremum of the
/// bound's direction over the choices it does leave (FR-204, FR-920).
pub(in crate::probability_certificate) fn workload_objective_for(
    bound: &Bound,
    residual: bool,
) -> ProbabilityObjective {
    match (residual, bound) {
        (false, _) => ProbabilityObjective::Workload,
        (true, Bound::AtLeast(_)) => ProbabilityObjective::WorkloadMinimum,
        (true, Bound::AtMost(_)) => ProbabilityObjective::WorkloadMaximum,
    }
}

/// The scheduler objective the shared graph checks evaluate: a workload's
/// residual choices are optimised exactly as the scheduler's are.
fn scheduler_objective(objective: ProbabilityObjective) -> ProbabilityObjective {
    match objective {
        ProbabilityObjective::WorkloadMinimum => ProbabilityObjective::Minimum,
        ProbabilityObjective::WorkloadMaximum => ProbabilityObjective::Maximum,
        other => other,
    }
}

/// The workload weight of `operation`. A weight of zero is refused here: the
/// workload would never take the operation, so the product cannot be built
/// from it (`admit_workload` refuses an enabled operation with no positive
/// weight before a state is expanded, and this refuses it where a weight is
/// read).
fn workload_weight(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    operation: &str,
) -> Result<Rational, ProbabilityDocumentError> {
    let fsl_semantics::check::ProbScheduler::Workload(name) = &claim.scheduler else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let workload = system
        .source_package()
        .and_then(|package| package.graph().probabilistic_model().workload(name))
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let weight = workload
        .weights()
        .iter()
        .find(|(weighted, _)| {
            system.operation_text(weighted.declaring(), weighted.name()) == Some(operation)
        })
        .map(|(_, weight)| weight.clone())
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if weight.is_zero() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(weight)
}

/// The share of each of `tied`, in order: its operation's weight over the
/// total of the distinct operations present, split uniformly over the
/// identities of that operation (ADR-026 SD-3).
fn operation_shares(
    tied: &[(fsl_eval::simulation::model::ScheduledIdentity, Rational)],
) -> Result<Vec<Rational>, ProbabilityDocumentError> {
    let mut operations: Vec<(&str, &Rational, u64)> = Vec::new();
    for (identity, weight) in tied {
        match operations
            .iter_mut()
            .find(|(name, _, _)| *name == identity.operation)
        {
            Some((_, _, count)) => *count += 1,
            None => operations.push((identity.operation.as_str(), weight, 1)),
        }
    }
    let total = operations
        .iter()
        .fold(zero(), |sum, (_, weight, _)| sum.add(weight));
    tied.iter()
        .map(|(identity, _)| {
            let (_, weight, count) = operations
                .iter()
                .find(|(name, _, _)| *name == identity.operation)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            weight
                .div(&total)
                .and_then(|share| share.div(&Rational::from_integer(Integer::from(*count))))
                .ok_or(ProbabilityDocumentError::Unsupported)
        })
        .collect()
}

/// Every ordering of the tied identities with its probability times
/// `probability`: drawn without replacement by the operations' weights, and
/// uniformly within an operation (ADR-026 SD-3).
fn weighted_orderings(
    tied: &[(fsl_eval::simulation::model::ScheduledIdentity, Rational)],
    probability: &Rational,
    prefix: &mut Vec<fsl_eval::simulation::model::ScheduledIdentity>,
    out: &mut Vec<(
        Vec<fsl_eval::simulation::model::ScheduledIdentity>,
        Rational,
    )>,
    limit: usize,
) -> Result<(), ProbabilityDocumentError> {
    if tied.is_empty() {
        if out.len() >= limit {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: u64::try_from(limit).unwrap_or(u64::MAX),
                actual: u64::try_from(out.len().saturating_add(1)).unwrap_or(u64::MAX),
            });
        }
        out.push((prefix.clone(), probability.clone()));
        return Ok(());
    }
    let shares = operation_shares(tied)?;
    for (index, ((identity, _), share)) in tied.iter().zip(&shares).enumerate() {
        let rest: Vec<_> = tied
            .iter()
            .enumerate()
            .filter(|(position, _)| *position != index)
            .map(|(_, entry)| entry.clone())
            .collect();
        prefix.push(identity.clone());
        weighted_orderings(&rest, &probability.mul(share), prefix, out, limit)?;
        prefix.pop();
    }
    Ok(())
}

/// Under a workload every operation enabled at a reached state must be
/// positively weighted. The workload resolves a state by its weights, so an
/// operation without one cannot be dropped from the product: the checker
/// would see a time-lock or a single choice where the engine sees a race.
fn admit_workload(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    actions: &[fsl_eval::simulation::model::Action],
) -> Result<(), ProbabilityDocumentError> {
    let fsl_semantics::check::ProbScheduler::Workload(name) = &claim.scheduler else {
        return Ok(());
    };
    let workload = system
        .source_package()
        .and_then(|package| package.graph().probabilistic_model().workload(name))
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    for action in actions {
        let weighted = workload.weights().iter().any(|(weighted, weight)| {
            !weight.is_zero()
                && system.operation_text(weighted.declaring(), weighted.name())
                    == Some(action.identity.operation.as_str())
        });
        if !weighted {
            return Err(ProbabilityDocumentError::Unsupported);
        }
    }
    Ok(())
}

fn discrete_delay(system: &ModelSystem<'_>, operation: &str) -> Option<Vec<(Rational, Rational)>> {
    system
        .input()
        .probabilistic
        .delays()
        .iter()
        .find(|delay| {
            let named = delay.operation();
            system.operation_text(named.declaring(), named.name()) == Some(operation)
        })
        .and_then(|delay| match delay.distribution() {
            fsl_semantics::check::DelayDistribution::Discrete(values) => Some(values.clone()),
            _ => None,
        })
}

#[allow(clippy::too_many_arguments)]
fn digital_discrete_delay_choice(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    action: &fsl_eval::simulation::model::Action,
    support: Vec<(Rational, Rational)>,
    caps: &[u64],
    scale: u64,
    deadline: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
) -> Result<(Vec<Vec<ProbabilityEdge>>, u64), ProbabilityDocumentError> {
    let mut draws = Vec::new();
    for (delay, weight) in support {
        poll(cancel)?;
        let ticks = time_units(&delay, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
        let Some(state) = digital_state_after_delay(
            system, timed, watch, binding, current, caps, scale, deadline, ticks, cancel,
        )?
        else {
            continue;
        };
        draws.push((ticks, weight, state));
    }
    // The delay is drawn conditioned on its window, as under a workload.
    let outcomes = condition_on_window(draws, |_, state| {
        Ok(
            digital_enabled_actions(system, timed, &state.timed, claim, cancel)?
                .iter()
                .any(|candidate| candidate.identity == action.identity),
        )
    })?;
    if outcomes.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let mut choices: Vec<(
        fsl_eval::simulation::model::ScheduledIdentity,
        Vec<ProbabilityEdge>,
    )> = Vec::new();
    let mut edges = 0_u64;
    for (_, delay_probability, delayed) in outcomes {
        if delayed.phase.decided() {
            let target = digital_key(system, timed, binding, &delayed, caps, scale, deadline)?;
            admit_region_state(target.clone(), delayed, reached, pending, limits, storage)?;
            append_checker_edge(
                &mut choices,
                action.identity.clone(),
                target,
                delay_probability,
            );
            edges = edges.saturating_add(1);
            charge_work(meter, cancel, 1)?;
            continue;
        }
        let tied = digital_enabled_actions(system, timed, &delayed.timed, claim, cancel)?
            .into_iter()
            .filter(|candidate| {
                candidate.identity == action.identity
                    || discrete_delay(system, &candidate.identity.operation).is_none()
            })
            .collect::<Vec<_>>();
        let identities: Vec<_> = tied
            .iter()
            .map(|candidate| candidate.identity.clone())
            .collect();
        for (first, chosen) in identities.iter().enumerate() {
            let mut order = Vec::with_capacity(identities.len());
            order.push(chosen.clone());
            order.extend(
                identities
                    .iter()
                    .enumerate()
                    .filter(|(position, _)| *position != first)
                    .map(|(_, identity)| identity.clone()),
            );
            let branches = digital_fire_chain(
                system, timed, claim, watch, binding, &delayed, &order, caps, scale, deadline,
                cancel,
            )?;
            for (branch_probability, final_state) in branches {
                let target =
                    digital_key(system, timed, binding, &final_state, caps, scale, deadline)?;
                admit_region_state(
                    target.clone(),
                    final_state,
                    reached,
                    pending,
                    limits,
                    storage,
                )?;
                append_checker_edge(
                    &mut choices,
                    chosen.clone(),
                    target,
                    delay_probability.mul(&branch_probability),
                );
                edges = edges.saturating_add(1);
                charge_work(meter, cancel, 1)?;
            }
        }
    }
    Ok((choices.into_iter().map(|(_, edges)| edges).collect(), edges))
}

/// Fire `order` one identity after another from `start`: each one that is
/// still enabled takes every draw of its action, the others are skipped. The
/// result is each branch's probability and state.
#[allow(clippy::too_many_arguments)]
fn digital_fire_chain(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    start: &RegionState,
    order: &[fsl_eval::simulation::model::ScheduledIdentity],
    caps: &[u64],
    scale: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Vec<(Rational, RegionState)>, ProbabilityDocumentError> {
    let mut branches = vec![(one(), start.clone())];
    for identity in order.iter().cloned() {
        let mut next = Vec::new();
        for (branch_probability, state) in branches {
            let available = digital_enabled_actions(system, timed, &state.timed, claim, cancel)?;
            let Some(enabled) = available
                .iter()
                .find(|candidate| candidate.identity == identity)
            else {
                next.push((branch_probability, state));
                continue;
            };
            let clocked_steps = timed
                .steps(&state.timed)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            for draw in &enabled.draws {
                let [(transition, discrete_post)] = draw.successors.as_slice() else {
                    return Err(ProbabilityDocumentError::Unsupported);
                };
                let Some((_, timed_post)) = clocked_steps.iter().find(|(step, post)| {
                    step.transition() == transition
                        && system
                            .state_key(post.discrete())
                            .ok()
                            .zip(system.state_key(discrete_post).ok())
                            .is_some_and(|(left, right)| left.0 == right.0)
                }) else {
                    continue;
                };
                let post = TimedState::new(
                    timed_post.discrete().clone(),
                    ClockValuation::new(quantized_clocks(timed_post, caps, scale)?),
                );
                let (phase, elapsed) = watch.observe(
                    system,
                    binding,
                    post.discrete(),
                    state.phase,
                    state.elapsed,
                    deadline,
                    cancel,
                )?;
                next.push((
                    branch_probability.mul(&draw.probability),
                    RegionState {
                        timed: post,
                        elapsed,
                        phase,
                        tail: DigitalTail::Running,
                    },
                ));
            }
        }
        branches = next;
    }
    Ok(branches)
}

/// The unit-delay states from `current`, index `k` being the state after `k`
/// units, and whether the last one repeats forever (ADR-026 TS-2).
struct DigitalTrajectory {
    states: Vec<RegionState>,
    forever: bool,
}

/// The delays after which one identity is enabled (FR-204's window).
struct DigitalWindow {
    identity: fsl_eval::simulation::model::ScheduledIdentity,
    delays: BTreeSet<u64>,
    forever: bool,
    given: Option<Vec<(u64, Rational)>>,
}

impl DigitalWindow {
    fn contains(&self, delay: u64, reached: u64) -> bool {
        self.delays.contains(&delay.min(reached)) && (delay <= reached || self.forever)
    }
}

/// Everything a workload state's expansion reads.
struct WorkloadRun<'a, 'b> {
    system: &'a ModelSystem<'b>,
    timed: &'a TimedSystem<'a, 'b>,
    claim: &'a CheckedProbabilisticClaim,
    watch: &'a Watch<'a>,
    binding: Option<&'a filament_exact::ObjectReference>,
    caps: &'a [u64],
    scale: u64,
    deadline: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
}

impl WorkloadRun<'_, '_> {
    fn trajectory(
        &self,
        current: &RegionState,
    ) -> Result<DigitalTrajectory, ProbabilityDocumentError> {
        let mut states = vec![current.clone()];
        let mut forever = false;
        loop {
            poll(self.cancel)?;
            if states.len() > self.limits.states {
                return Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::States,
                    bound: u64::try_from(self.limits.states).unwrap_or(u64::MAX),
                    actual: u64::try_from(states.len()).unwrap_or(u64::MAX),
                });
            }
            let Some(last) = states.last() else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            let next = if last.phase.decided() {
                // A decided monitor no longer reads the delay, but the
                // product's windows run on until the clocks stop moving.
                let delta = rational_units(1, self.scale)?;
                let Some(delayed) = self
                    .timed
                    .delay(&last.timed, &delta)
                    .map_err(|_| ProbabilityDocumentError::Unsupported)?
                else {
                    break;
                };
                Some(RegionState {
                    timed: TimedState::new(
                        delayed.discrete().clone(),
                        ClockValuation::new(quantized_clocks(&delayed, self.caps, self.scale)?),
                    ),
                    elapsed: last
                        .elapsed
                        .saturating_add(1)
                        .min(self.deadline.saturating_add(1)),
                    phase: last.phase,
                    tail: last.tail,
                })
            } else {
                digital_state_after_delay(
                    self.system,
                    self.timed,
                    self.watch,
                    self.binding,
                    last,
                    self.caps,
                    self.scale,
                    self.deadline,
                    1,
                    self.cancel,
                )?
            };
            let Some(next) = next else {
                break;
            };
            let same = digital_key(
                self.system,
                self.timed,
                self.binding,
                &next,
                self.caps,
                self.scale,
                self.deadline,
            )? == digital_key(
                self.system,
                self.timed,
                self.binding,
                last,
                self.caps,
                self.scale,
                self.deadline,
            )?;
            if same {
                forever = true;
                break;
            }
            states.push(next);
        }
        Ok(DigitalTrajectory { states, forever })
    }

    fn windows(
        &self,
        trajectory: &DigitalTrajectory,
    ) -> Result<Vec<DigitalWindow>, ProbabilityDocumentError> {
        let mut windows: Vec<DigitalWindow> = Vec::new();
        for (delay, state) in trajectory.states.iter().enumerate() {
            let delay = u64::try_from(delay).map_err(|_| ProbabilityDocumentError::Unsupported)?;
            for action in digital_enabled_actions(
                self.system,
                self.timed,
                &state.timed,
                self.claim,
                self.cancel,
            )? {
                if let Some(window) = windows
                    .iter_mut()
                    .find(|window| window.identity == action.identity)
                {
                    window.delays.insert(delay);
                    continue;
                }
                let given = match discrete_delay(self.system, &action.identity.operation) {
                    Some(support) => Some(
                        support
                            .into_iter()
                            .map(|(delay, weight)| {
                                time_units(&delay, self.scale)
                                    .map(|units| (units, weight))
                                    .ok_or(ProbabilityDocumentError::Unsupported)
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                    None => None,
                };
                windows.push(DigitalWindow {
                    identity: action.identity.clone(),
                    delays: BTreeSet::from([delay]),
                    forever: false,
                    given,
                });
            }
        }
        if trajectory.forever {
            let last = u64::try_from(trajectory.states.len().saturating_sub(1))
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            for window in &mut windows {
                window.forever = window.delays.contains(&last);
            }
        }
        Ok(windows)
    }

    /// The states one race outcome reaches: the state after `delay` units,
    /// then the tied identities in each weighted order while enabled.
    fn race_states(
        &self,
        trajectory: &DigitalTrajectory,
        delay: u64,
        tied: &[fsl_eval::simulation::model::ScheduledIdentity],
        probability: &Rational,
    ) -> Result<Vec<(Rational, RegionState)>, ProbabilityDocumentError> {
        let last = trajectory.states.len().saturating_sub(1);
        let index = usize::try_from(delay).unwrap_or(usize::MAX).min(last);
        let start = trajectory
            .states
            .get(index)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let weighted = tied
            .iter()
            .map(|identity| {
                Ok((
                    identity.clone(),
                    workload_weight(self.system, self.claim, &identity.operation)?,
                ))
            })
            .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
        let mut orders = Vec::new();
        weighted_orderings(
            &weighted,
            probability,
            &mut Vec::new(),
            &mut orders,
            self.limits.edges,
        )?;
        let mut reached = Vec::new();
        for (order, share) in orders {
            for (branch, state) in digital_fire_chain(
                self.system,
                self.timed,
                self.claim,
                self.watch,
                self.binding,
                start,
                &order,
                self.caps,
                self.scale,
                self.deadline,
                self.cancel,
            )? {
                reached.push((share.mul(&branch), state));
            }
        }
        Ok(reached)
    }

    /// `states` as one choice's edges, each target admitted to the graph.
    fn choice_of(
        &self,
        states: Vec<(Rational, RegionState)>,
        into: &mut Vec<ProbabilityEdge>,
        reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
        pending: &mut VecDeque<ProbabilityProductKey>,
        storage: &mut u64,
    ) -> Result<(), ProbabilityDocumentError> {
        for (probability, state) in states {
            let target = digital_key(
                self.system,
                self.timed,
                self.binding,
                &state,
                self.caps,
                self.scale,
                self.deadline,
            )?;
            admit_region_state(
                target.clone(),
                state,
                reached,
                pending,
                self.limits,
                storage,
            )?;
            if let Some(edge) = into.iter_mut().find(|edge| edge.target == target) {
                edge.probability = edge.probability.add(&probability);
            } else {
                into.push(ProbabilityEdge {
                    target,
                    probability,
                });
            }
        }
        Ok(())
    }
}

/// ADR-026 SD-3: a given delay is drawn conditioned on its window. `draws`
/// are the support's delays in digital units with their weights and a value
/// each carries; `inside` says whether the identity is enabled after the
/// delay. The result keeps the delays inside the window, each weight over the
/// total weight inside, and is empty when no mass is inside. The
/// every-scheduler and the workload products both read a given delay here, so
/// one model has one reading.
fn condition_on_window<T>(
    draws: Vec<(u64, Rational, T)>,
    mut inside: impl FnMut(u64, &T) -> Result<bool, ProbabilityDocumentError>,
) -> Result<Vec<(u64, Rational, T)>, ProbabilityDocumentError> {
    let mut kept = Vec::with_capacity(draws.len());
    for (delay, weight, carried) in draws {
        if inside(delay, &carried)? {
            kept.push((delay, weight, carried));
        }
    }
    let total = kept
        .iter()
        .fold(zero(), |sum, (_, weight, _)| sum.add(weight));
    if total.is_zero() {
        return Ok(Vec::new());
    }
    kept.into_iter()
        .map(|(delay, weight, carried)| {
            weight
                .div(&total)
                .map(|share| (delay, share, carried))
                .ok_or(ProbabilityDocumentError::Unsupported)
        })
        .collect()
}

/// Whether two distributions over product states are the same.
fn same_choice(left: &[ProbabilityEdge], right: &[ProbabilityEdge]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.target == b.target && a.probability == b.probability)
}

/// FR-204's choices of a running state under a workload, each a distribution
/// the workload's weights fix: the race of the identities whose delays
/// `discrete` distributions give (with the scheduler's free delays after the
/// draw when free identities race too, `FreeDelays`), or, when no given
/// delay races, the time-passing choices and one resolution of the free
/// identity enabled now (`WorkloadResolve`).
///
/// Several racing given delays race together: the least draw wins and the
/// workload's weights order the identities tied at it. Two states
/// refuse, where the `WorkloadResolve` reading and ADR-026 SD-1 and SD-3 (the
/// scheduler picks each free identity's delay inside its window) give
/// different verdicts, until the standard rules between them: several free
/// identities enabled now, and a free identity that the unit delay disables.
#[allow(clippy::too_many_arguments)]
fn digital_workload_choices(
    run: &WorkloadRun<'_, '_>,
    current: &RegionState,
    enabled: &[fsl_eval::simulation::model::Action],
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
) -> Result<Vec<Vec<ProbabilityEdge>>, ProbabilityDocumentError> {
    let trajectory = run.trajectory(current)?;
    let windows = run.windows(&trajectory)?;
    let reach = u64::try_from(trajectory.states.len().saturating_sub(1))
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    // Each given delay is drawn conditioned on its window; an identity with no
    // conditioned mass in its window does not race (ADR-026 SD-2, SD-3).
    let mut racers: Vec<(usize, Vec<(u64, Rational)>)> = Vec::new();
    for (index, window) in windows.iter().enumerate() {
        let Some(support) = &window.given else {
            continue;
        };
        let draws = support
            .iter()
            .map(|(delay, weight)| (*delay, weight.clone(), ()))
            .collect();
        let conditioned =
            condition_on_window(draws, |delay, ()| Ok(window.contains(delay, reach)))?;
        if conditioned.is_empty() {
            continue;
        }
        racers.push((
            index,
            conditioned
                .into_iter()
                .map(|(delay, share, ())| (delay, share))
                .collect(),
        ));
    }
    let free: Vec<&DigitalWindow> = windows
        .iter()
        .filter(|window| window.given.is_none())
        .collect();
    let mut choices: Vec<Vec<ProbabilityEdge>> = Vec::new();
    if racers.is_empty() {
        // The free identities enabled now.
        let free_now: Vec<&fsl_eval::simulation::model::Action> = enabled
            .iter()
            .filter(|action| discrete_delay(run.system, &action.identity.operation).is_none())
            .collect();
        // FR-204 reads this state as `UnitDelay` and one weighted
        // `WorkloadResolve`; ADR-026 SD-1 and SD-3 (and FR-204's own
        // `FreeDelays` rule) have the scheduler pick each free identity's
        // delay inside its window. The two readings give different verdicts
        // where several free identities are enabled now, and where the unit
        // delay disables one that is enabled now, so neither is decided here.
        if free_now.len() >= 2 {
            return Err(ProbabilityDocumentError::WorkloadFreeDelays(
                WorkloadFreeDelayRefusal::ManyFreeEnabled,
            ));
        }
        if reach >= 1 {
            for action in &free_now {
                let closes = windows
                    .iter()
                    .find(|window| window.identity == action.identity)
                    .is_some_and(|window| !window.contains(1, reach));
                if closes {
                    return Err(ProbabilityDocumentError::WorkloadFreeDelays(
                        WorkloadFreeDelayRefusal::ClosesBeforeFiring,
                    ));
                }
            }
        }
        choices.extend(digital_time_choices(
            run.system,
            run.timed,
            run.watch,
            run.binding,
            current,
            run.caps,
            run.scale,
            run.deadline,
            run.limits,
            run.cancel,
            meter,
            storage,
            reached,
            pending,
        )?);
        // The workload resolves the free identity enabled now.
        let weighted = free_now
            .iter()
            .map(|action| {
                Ok((
                    action.identity.clone(),
                    workload_weight(run.system, run.claim, &action.identity.operation)?,
                ))
            })
            .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
        let shares = operation_shares(&weighted)?;
        let mut resolve = Vec::new();
        for ((identity, _), share) in weighted.iter().zip(&shares) {
            let states = digital_fire_chain(
                run.system,
                run.timed,
                run.claim,
                run.watch,
                run.binding,
                current,
                std::slice::from_ref(identity),
                run.caps,
                run.scale,
                run.deadline,
                run.cancel,
            )?
            .into_iter()
            .map(|(probability, state)| (share.mul(&probability), state))
            .collect();
            run.choice_of(states, &mut resolve, reached, pending, storage)?;
            charge_work(meter, run.cancel, 1)?;
        }
        if !resolve.is_empty() {
            choices.push(resolve);
        }
        return Ok(choices);
    }
    let racing: Vec<&fsl_eval::simulation::model::ScheduledIdentity> = racers
        .iter()
        .map(|(index, _)| &windows[*index].identity)
        .collect();
    // The given vectors: the product of every racer's conditioned support,
    // each with the product of the conditioned probabilities (FR-204). Each
    // vector is at least one edge, so a product above the edge limit is
    // refused before any vector is built.
    let edge_limit = u64::try_from(run.limits.edges).unwrap_or(u64::MAX);
    let rows = racers.iter().fold(1_u64, |rows, (_, support)| {
        rows.saturating_mul(u64::try_from(support.len()).unwrap_or(u64::MAX))
    });
    if rows > edge_limit {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::Edges,
            bound: edge_limit,
            actual: rows,
        });
    }
    let mut joint: Vec<(Vec<u64>, Rational)> = vec![(Vec::new(), one())];
    for (_, support) in &racers {
        let mut next = Vec::new();
        for (prefix, weight) in &joint {
            for (delay, share) in support {
                poll(run.cancel)?;
                let mut draws = prefix.clone();
                draws.push(*delay);
                next.push((draws, weight.mul(share)));
            }
        }
        joint = next;
    }
    let tied_given =
        |least: u64, draws: &[u64]| -> Vec<fsl_eval::simulation::model::ScheduledIdentity> {
            racing
                .iter()
                .zip(draws)
                .filter(|(_, draw)| **draw == least)
                .map(|(identity, _)| (*identity).clone())
                .collect()
        };
    if free.is_empty() {
        // Given delays alone: the least draw wins, ties by the workload's
        // weights; one race, no scheduler choice.
        let mut race = Vec::new();
        for (draws, share) in joint {
            let least = draws
                .iter()
                .copied()
                .min()
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let states = run.race_states(&trajectory, least, &tied_given(least, &draws), &share)?;
            run.choice_of(states, &mut race, reached, pending, storage)?;
            charge_work(meter, run.cancel, 1)?;
        }
        choices.push(race);
        return Ok(choices);
    }
    // FR-204 `FreeDelays`: the scheduler picks every free identity's delay in
    // its window after the given delay is drawn, so a row is one drawn vector
    // of the given delays with its options (one per assignment of the free
    // delays). The action that picks option `a(g)` at each row `g` is one
    // scheduler choice, and its value factors by row
    // (`Σ_g P(g) opt_a Q(g, a)`); the options of a row that are the same
    // distribution are one option.
    let mut per_draw: Vec<Vec<Vec<ProbabilityEdge>>> = Vec::new();
    for (draws, share) in &joint {
        let draw = draws
            .iter()
            .copied()
            .min()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let mut assignment = vec![0_usize; free.len()];
        let mut options: Vec<Vec<ProbabilityEdge>> = Vec::new();
        'assignments: loop {
            let chosen: Vec<u64> = free
                .iter()
                .zip(&assignment)
                .map(|(window, position)| {
                    window
                        .delays
                        .iter()
                        .copied()
                        .nth(*position)
                        .ok_or(ProbabilityDocumentError::Malformed)
                })
                .collect::<Result<_, _>>()?;
            let least = chosen.iter().copied().fold(draw, u64::min);
            let mut tied = tied_given(least, draws);
            for (window, delay) in free.iter().zip(&chosen) {
                if *delay == least {
                    tied.push(window.identity.clone());
                }
            }
            let states = run.race_states(&trajectory, least, &tied, share)?;
            let mut option = Vec::new();
            run.choice_of(states, &mut option, reached, pending, storage)?;
            if !options.iter().any(|seen| same_choice(seen, &option)) {
                options.push(option);
            }
            charge_work(meter, run.cancel, 1)?;
            let mut position = free.len();
            loop {
                if position == 0 {
                    break 'assignments;
                }
                position -= 1;
                assignment[position] += 1;
                if assignment[position] < free[position].delays.len() {
                    break;
                }
                assignment[position] = 0;
            }
        }
        per_draw.push(options);
    }
    // The checker's graph states list their choices, so the scheduler's
    // choices are listed here: every function from a drawn delay to one of its
    // options. The edge limit is checked against the edges of the listing
    // before any choice is built, never against a count of choices.
    let mut functions = 1_u64;
    let mut edges_per_function = 0_u64;
    for options in &per_draw {
        let count = u64::try_from(options.len()).unwrap_or(u64::MAX);
        functions = functions.saturating_mul(count);
        let widest = options.iter().map(Vec::len).max().unwrap_or(0);
        edges_per_function =
            edges_per_function.saturating_add(u64::try_from(widest).unwrap_or(u64::MAX));
    }
    let edges = functions.saturating_mul(edges_per_function.max(1));
    if functions > 1 && edges > edge_limit {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::Edges,
            bound: edge_limit,
            actual: edges,
        });
    }
    let mut picked = vec![0_usize; per_draw.len()];
    let mut listed = 0_u64;
    loop {
        poll(run.cancel)?;
        let mut choice: Vec<ProbabilityEdge> = Vec::new();
        for (options, position) in per_draw.iter().zip(&picked) {
            for edge in &options[*position] {
                if let Some(existing) = choice.iter_mut().find(|e| e.target == edge.target) {
                    existing.probability = existing.probability.add(&edge.probability);
                } else {
                    choice.push(edge.clone());
                }
            }
        }
        listed = listed.saturating_add(u64::try_from(choice.len()).unwrap_or(u64::MAX));
        if listed > edge_limit {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: edge_limit,
                actual: listed,
            });
        }
        choices.push(choice);
        let mut position = picked.len();
        loop {
            if position == 0 {
                return Ok(choices);
            }
            position -= 1;
            picked[position] += 1;
            if picked[position] < per_draw[position].len() {
                break;
            }
            picked[position] = 0;
        }
    }
}

/// The time-passing choices of a running state that no identity races from:
/// the next distinct unit delay and, where the state may idle, the idle
/// delay.
#[allow(clippy::too_many_arguments)]
fn digital_time_choices(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    key_scale: u64,
    deadline: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
) -> Result<Vec<Vec<ProbabilityEdge>>, ProbabilityDocumentError> {
    let mut choices = Vec::new();
    if let Some((delayed_key, delayed)) = next_digital_delay(
        system, timed, watch, binding, current, caps, key_scale, deadline, cancel,
    )? {
        admit_region_state(
            delayed_key.clone(),
            delayed,
            reached,
            pending,
            limits,
            storage,
        )?;
        choices.push(vec![ProbabilityEdge {
            target: delayed_key,
            probability: one(),
        }]);
        charge_work(meter, cancel, 1)?;
    }
    let invariants = timed
        .invariants(current.timed.discrete())
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    let urgent = timed
        .is_urgent(current.timed.discrete())
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    if invariants.is_empty() && !urgent {
        if let Some((target, idle)) = digital_idle_delay(
            system, timed, watch, binding, current, caps, key_scale, deadline, cancel,
        )? {
            admit_region_state(target.clone(), idle, reached, pending, limits, storage)?;
            choices.push(vec![ProbabilityEdge {
                target,
                probability: one(),
            }]);
            charge_work(meter, cancel, 1)?;
        }
    }
    Ok(choices)
}

fn append_checker_edge(
    choices: &mut Vec<(
        fsl_eval::simulation::model::ScheduledIdentity,
        Vec<ProbabilityEdge>,
    )>,
    identity: fsl_eval::simulation::model::ScheduledIdentity,
    target: ProbabilityProductKey,
    probability: Rational,
) {
    let Some((_, edges)) = choices
        .iter_mut()
        .find(|(existing, _)| *existing == identity)
    else {
        choices.push((
            identity,
            vec![ProbabilityEdge {
                target,
                probability,
            }],
        ));
        return;
    };
    if let Some(edge) = edges.iter_mut().find(|edge| edge.target == target) {
        edge.probability = edge.probability.add(&probability);
    } else {
        edges.push(ProbabilityEdge {
            target,
            probability,
        });
    }
}

fn digital_enabled_actions(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    state: &TimedState,
    claim: &CheckedProbabilisticClaim,
    cancel: &Cancel,
) -> Result<Vec<fsl_eval::simulation::model::Action>, ProbabilityDocumentError> {
    let actions = system
        .actions_with_cancel(state.discrete(), cancel)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    admit_workload(system, claim, &actions)?;
    let clocked_steps = timed
        .steps(state)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    Ok(actions
        .into_iter()
        .filter(|action| {
            action.draws.iter().any(|draw| {
                draw.successors.iter().any(|(transition, discrete_post)| {
                    clocked_steps.iter().any(|(step, timed_post)| {
                        step.transition() == transition
                            && system
                                .state_key(timed_post.discrete())
                                .ok()
                                .zip(system.state_key(discrete_post).ok())
                                .is_some_and(|(left, right)| left.0 == right.0)
                    })
                })
            })
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn digital_state_after_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    scale: u64,
    deadline: u64,
    ticks: u64,
    cancel: &Cancel,
) -> Result<Option<RegionState>, ProbabilityDocumentError> {
    let delta = rational_units(1, scale)?;
    let mut state = current.clone();
    for _ in 0..ticks {
        poll(cancel)?;
        let Some(delayed) = timed
            .delay(&state.timed, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        else {
            return Ok(None);
        };
        let elapsed = state
            .elapsed
            .saturating_add(1)
            .min(deadline.saturating_add(1));
        let delayed = TimedState::new(
            delayed.discrete().clone(),
            ClockValuation::new(quantized_clocks(&delayed, caps, scale)?),
        );
        let (phase, elapsed) = watch.observe(
            system,
            binding,
            delayed.discrete(),
            state.phase,
            elapsed,
            deadline,
            cancel,
        )?;
        state = RegionState {
            timed: delayed,
            elapsed,
            phase,
            tail: current.tail,
        };
        if state.phase.decided() && !matches!(watch, Watch::Bounded { .. } | Watch::Nested { .. }) {
            return Ok(Some(state));
        }
    }
    Ok(Some(state))
}

#[allow(
    clippy::too_many_arguments,
    reason = "timed certificate checks bind the subject and claim with independent scale, accounting, storage, and cancellation limits"
)]
fn next_region_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    watch: &Watch<'_>,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    scale: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Option<(ProbabilityProductKey, RegionState)>, ProbabilityDocumentError> {
    let current_key = region_key(
        system,
        timed,
        binding,
        current,
        caps,
        base_scale,
        subdivisions,
        scale,
        deadline,
    )?;
    let delta = rational_units(1, scale)?;
    let limit = caps
        .iter()
        .max()
        .copied()
        .unwrap_or(0)
        .saturating_add(1)
        .saturating_mul(subdivisions)
        .saturating_add(2);
    let mut steps = 0_u64;
    let mut state = current.clone();
    while state.elapsed <= deadline {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let Some(delayed) = timed
            .delay(&state.timed, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        else {
            return Ok(None);
        };
        let elapsed = state
            .elapsed
            .saturating_add(1)
            .min(deadline.saturating_add(1));
        let clocks = quantized_clocks(&delayed, caps, scale)?;
        let delayed = TimedState::new(delayed.discrete().clone(), ClockValuation::new(clocks));
        let (phase, elapsed) = watch.observe(
            system,
            binding,
            delayed.discrete(),
            state.phase,
            elapsed,
            deadline,
            cancel,
        )?;
        let reached = RegionState {
            timed: delayed,
            elapsed,
            phase,
            tail: current.tail,
        };
        let key = region_key(
            system,
            timed,
            binding,
            &reached,
            caps,
            base_scale,
            subdivisions,
            scale,
            deadline,
        )?;
        if key != current_key {
            return Ok(Some((key, reached)));
        }
        // An inactive measure does not advance, so once every clock is past its
        // cap no further region is reachable by waiting.
        steps += 1;
        if reached.phase == Phase::Inactive && steps > limit {
            return Ok(None);
        }
        state = reached;
    }
    Ok(None)
}

fn admit_region_state(
    key: ProbabilityProductKey,
    state: RegionState,
    reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
    limits: ProbabilityDocumentLimits,
    storage: &mut u64,
) -> Result<(), ProbabilityDocumentError> {
    if reached.contains_key(&key) {
        return Ok(());
    }
    let required = u64::try_from(reached.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if required > u64::try_from(limits.states).unwrap_or(u64::MAX) {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::States,
            bound: u64::try_from(limits.states).unwrap_or(u64::MAX),
            actual: required,
        });
    }
    reserve_storage(
        storage,
        key.as_bytes()
            .len()
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, RegionState)>()),
        limits,
    )?;
    reached.insert(key.clone(), state);
    pending.push_back(key);
    Ok(())
}

fn reserve_storage(
    storage: &mut u64,
    bytes: usize,
    limits: ProbabilityDocumentLimits,
) -> Result<(), ProbabilityDocumentError> {
    let actual = storage.saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    if actual > limits.storage_bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::StorageBytes,
            limits.storage_bytes,
            actual,
        ));
    }
    *storage = actual;
    Ok(())
}

fn insert_checker_state(
    key: ProbabilityProductKey,
    state: CheckerState,
    states: &mut BTreeMap<ProbabilityProductKey, CheckerState>,
    storage: &mut u64,
    limits: ProbabilityDocumentLimits,
) -> Result<(), ProbabilityDocumentError> {
    let edge_count = state.choices.iter().map(Vec::len).sum::<usize>();
    let bytes = key
        .as_bytes()
        .len()
        .saturating_add(std::mem::size_of::<(ProbabilityProductKey, CheckerState)>())
        .saturating_add(
            state
                .choices
                .len()
                .saturating_mul(std::mem::size_of::<Vec<ProbabilityEdge>>()),
        )
        .saturating_add(edge_count.saturating_mul(std::mem::size_of::<ProbabilityEdge>()));
    reserve_storage(storage, bytes, limits)?;
    states.insert(key, state);
    Ok(())
}

fn charge_work(
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
    units: u64,
) -> Result<(), ProbabilityDocumentError> {
    poll(cancel)?;
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                .work(Integer::from(units)),
        )
        .map_err(|error| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(error)),
        })
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1_i64))
}

/// The checker derives the route from the checked clock constraints and
/// temporal intervals; the objective's route string is never authoritative.
fn region_route(
    timed: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
) -> bool {
    clock_constraints(timed).into_iter().any(|constraint| {
        matches!(
            constraint.op(),
            fsl_semantics::check::ClockOp::Less | fsl_semantics::check::ClockOp::Greater
        )
    }) || elapsed::query(claim).is_some_and(|query| matches!(query.bound, Bound::AtLeast(_)))
        || matches!(
            &claim.form,
            fsl_semantics::check::ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula { formula, .. },
                ..
            } if formula.formula().nodes().iter().any(|node| match node {
                fsl_semantics::check::FormulaNode::Unary {
                    interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
                    ..
                }
                | fsl_semantics::check::FormulaNode::Binary {
                    interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
                    ..
                } => !interval.upper_closed() || !interval.lower_closed(),
                _ => false,
            })
        )
}

fn clock_constraints<'a>(
    timed: &'a fsl_eval::simulation::model::TimedSystem<'_, '_>,
) -> Vec<&'a fsl_semantics::check::ClockConstraint> {
    fn part_constraints<'a>(
        part: &'a fsl_semantics::check::ConvexPart,
        found: &mut Vec<&'a fsl_semantics::check::ClockConstraint>,
    ) {
        found.extend(part.clocks());
        for family in part.families() {
            for inner in family.parts() {
                part_constraints(inner, found);
            }
        }
    }

    let mut found = Vec::new();
    for guard in timed.model().guards() {
        for part in guard.parts() {
            part_constraints(part, &mut found);
        }
    }
    for invariant in timed.model().invariants() {
        found.extend(invariant.bounds());
    }
    found
}

/// Recompute the canonical lattice and region refinement from the checked
/// model. A document cannot select a coarser grid that merges strict-boundary
/// states.
fn certificate_scale(
    system: &ModelSystem<'_>,
    timed: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    route: TimedProbabilityRoute,
) -> Result<u64, ProbabilityDocumentError> {
    let mut denominators = Vec::new();
    for constraint in clock_constraints(timed) {
        denominators.push(constraint.bound().denominator().as_big().clone());
    }
    for reset in timed.model().resets() {
        for reset in reset.resets() {
            denominators.push(reset.value().denominator().as_big().clone());
        }
    }
    if let fsl_semantics::check::ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                timed_horizon: Some(horizon),
                ..
            },
        ..
    } = &claim.form
    {
        denominators.push(horizon.denominator().as_big().clone());
    }
    if let Some(query) = elapsed::query(claim) {
        denominators.push(query.within.denominator().as_big().clone());
        denominators.push(
            elapsed::threshold(&query.bound)
                .denominator()
                .as_big()
                .clone(),
        );
    }
    if let fsl_semantics::check::ProbForm::MeanTime { bound, .. } = &claim.form {
        let value = match bound {
            Bound::AtLeast(value) | Bound::AtMost(value) => value,
        };
        denominators.push(value.denominator().as_big().clone());
    }
    for delay in system.input().probabilistic.delays() {
        if let fsl_semantics::check::DelayDistribution::Discrete(values) = delay.distribution() {
            denominators.extend(
                values
                    .iter()
                    .map(|(value, _)| value.denominator().as_big().clone()),
            );
        }
    }
    let base = denominators
        .iter()
        .fold(BigInt::one(), |scale, denominator| scale.lcm(denominator))
        .to_u64()
        .filter(|scale| *scale > 0)
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let subdivisions = if route == TimedProbabilityRoute::Region {
        let has_deadline = elapsed::query(claim).is_some()
            || matches!(
                &claim.form,
                fsl_semantics::check::ProbForm::Probability {
                    event: fsl_semantics::check::Event::Formula {
                        timed_horizon: Some(_),
                        ..
                    },
                    ..
                }
            );
        u64::try_from(timed.places().len())
            .ok()
            .and_then(|clocks| clocks.checked_add(if has_deadline { 1 } else { 0 }))
            .and_then(|clocks| clocks.checked_add(1))
            .ok_or(ProbabilityDocumentError::Unsupported)?
    } else {
        1
    };
    base.checked_mul(subdivisions)
        .ok_or(ProbabilityDocumentError::Unsupported)
}

/// FR-923: replay a path-set refutation of a timed deadline claim, or of an
/// elapsed-quantile claim (FR-877) through [`elapsed::replay`].
///
/// The checker enumerates the digital or region product itself, exactly as it
/// does to check a certificate, and reads the witness as paths of product-state
/// keys from the initial state to a decided state. The witness's own action
/// spellings and post-states are not trusted: a step is valid when the key it
/// reaches is an outcome of one choice of the state it leaves, and one state
/// never takes two different choices (a memoryless scheduler). The mass of the
/// paths is then a lower bound of the probability the claim's bound forbids,
/// under a scheduler that no state of the witness makes time-convergent.
///
/// # Errors
/// [`ProbabilityWitnessError`] when the claim is not a timed deadline claim,
/// the product cannot be enumerated within the limits, or the witness names a
/// state, step or choice the checker's product does not have.
pub(in crate::probability_certificate) fn replay_timed_witness(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &super::super::ProbabilisticCounterexample,
    limits: &crate::ExactProbLimits,
    cancel: &Cancel,
) -> Result<super::super::ProbabilityWitnessReplay, super::super::ProbabilityWitnessError> {
    use super::super::{ProbabilityWitnessError as Error, ProbabilityWitnessReplay as Replay};
    let unsupported = || Error::Graph(ExactProbabilityGraphError::UnsupportedClaimForm);
    let from_document = |error: ProbabilityDocumentError| match error {
        ProbabilityDocumentError::Limit { limit, bound, .. } => Error::Limit {
            limit: limit.model_limit(),
            value: bound,
        },
        ProbabilityDocumentError::Cancelled(cause) => {
            Error::Graph(ExactProbabilityGraphError::Cancelled(cause))
        }
        ProbabilityDocumentError::Graph(error) => Error::Graph(error),
        _ => unsupported(),
    };
    if let Some(query) = elapsed::query(claim) {
        return elapsed::replay(system, claim, &query, witness, limits, cancel);
    }
    let Some((bound, predicate)) = deadline_event(claim) else {
        return Err(unsupported());
    };
    if !claim.fairness.is_empty() {
        return Err(unsupported());
    }
    let timed = TimedSystem::new(system, &claim.subject.alias).map_err(|_| unsupported())?;
    let route = if region_route(&timed, claim) {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let scale = certificate_scale(system, &timed, claim, route).map_err(from_document)?;
    let binding = claim_binding(system, claim)
        .map_err(|_| unsupported())?
        .map(|binding| binding.reference);
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let mut meter =
        filament_exact::Meter::new(document_limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let graph = match route {
        TimedProbabilityRoute::Digital => enumerate_digital_deadline(
            system,
            &timed,
            claim,
            binding.as_ref(),
            &Watch::Deadline(predicate),
            time_bound(claim, scale).map_err(from_document)?,
            scale,
            document_limits,
            cancel,
            &mut meter,
            &mut storage,
            &mut BTreeMap::new(),
        ),
        TimedProbabilityRoute::Region => enumerate_region_deadline(
            system,
            &timed,
            claim,
            binding.as_ref(),
            &Watch::Deadline(predicate),
            time_bound(claim, scale).map_err(from_document)?,
            scale,
            scale,
            document_limits,
            cancel,
            &mut meter,
            &mut storage,
            &mut BTreeMap::new(),
            &mut BTreeSet::new(),
        ),
    }
    .map_err(from_document)?;
    // A workload resolves a tie by its weights, so the digital graph holds
    // only the choices FR-204 leaves the scheduler and the witness selects
    // among those; the region graph mirrors only states with no choice.
    if route == TimedProbabilityRoute::Region
        && matches!(
            claim.scheduler,
            fsl_semantics::check::ProbScheduler::Workload(_)
        )
        && graph.states.values().any(|state| state.choices.len() > 1)
    {
        return Err(unsupported());
    }
    if witness.initial_key != graph.initial {
        return Err(Error::Refused);
    }
    let failing = matches!(bound, Bound::AtLeast(_));
    let threshold = match &bound {
        Bound::AtMost(threshold) => threshold.clone(),
        Bound::AtLeast(threshold) => one().sub(threshold),
    };
    let sequences: Vec<Vec<ProbabilityProductKey>> = witness
        .paths
        .iter()
        .map(|path| {
            path.steps
                .iter()
                .map(|step| step.post_key.clone())
                .collect()
        })
        .collect();
    let Some((kept, mass)) = price_witness_paths(
        &graph,
        failing,
        &threshold,
        &sequences,
        limits.max_rational_bits,
        cancel,
    )?
    else {
        return Ok(Replay::Parity);
    };
    let mut validated = witness.clone();
    validated.paths = kept
        .into_iter()
        .map(|index| witness.paths[index].clone())
        .collect();
    Ok(Replay::Reproduced(Box::new(
        super::super::ReproducedProbabilityWitness::reproduced(validated, mass),
    )))
}

/// The paths of a witness that the checker graph validates, and their mass.
///
/// `sequences` holds each witness path as the keys it visits after the initial
/// state. The witness selects the scheduler; the mass is the checker graph's
/// own probability of the failing outcome under that memoryless choice (states
/// no path visits take their first choice), so a step the graph does not have
/// contributes nothing and a cycle under the choice is refused. The reported
/// paths must reproduce themselves: those that are not outcomes of the chosen
/// choice are dropped and the choices and mass re-derived from the rest until
/// the set is stable. `None` (Parity) when a path is not a distinct walk to
/// the forbidden decided state, or the final mass does not exceed `threshold`.
fn price_witness_paths(
    graph: &CheckerGraph,
    failing: bool,
    threshold: &Rational,
    sequences: &[Vec<ProbabilityProductKey>],
    max_rational_bits: u64,
    cancel: &Cancel,
) -> Result<Option<(Vec<usize>, Rational)>, super::super::ProbabilityWitnessError> {
    price_paths(
        graph,
        &Payoff::Deadline { failing, threshold },
        sequences,
        max_rational_bits,
        cancel,
    )
}

/// What a witness path set is priced against: FR-204's forbidden deadline
/// outcome, or FR-877's elapsed-quantile transform.
enum Payoff<'a> {
    Deadline {
        failing: bool,
        threshold: &'a Rational,
    },
    /// The transform's terminals: `violating` ends the paths the refutation
    /// counts (`M > c` for `<=`, `M < c` for `>=`) and `activated` holds every
    /// decided measure. `strict` is the `>=` claim, whose refutation is the
    /// non-strict `max >= 0` and needs a positive violating mass.
    Elapsed {
        violating: &'a BTreeSet<ProbabilityProductKey>,
        activated: &'a BTreeSet<ProbabilityProductKey>,
        q: &'a Rational,
        strict: bool,
    },
}

impl Payoff<'_> {
    fn ends(&self, key: &ProbabilityProductKey, terminal: Option<bool>) -> bool {
        match self {
            Self::Deadline { failing, .. } => terminal == Some(!*failing),
            Self::Elapsed { violating, .. } => violating.contains(key),
        }
    }

    fn weight(&self, key: &ProbabilityProductKey, verdict: bool) -> Rational {
        let counted = match self {
            Self::Deadline { failing, .. } => verdict == !*failing,
            Self::Elapsed { violating, .. } => violating.contains(key),
        };
        if counted { one() } else { zero() }
    }

    fn activation(&self, key: &ProbabilityProductKey) -> Rational {
        match self {
            Self::Elapsed { activated, .. } if activated.contains(key) => one(),
            _ => zero(),
        }
    }

    /// Whether `mass`, with `activation` the mass of decided measures, refutes.
    fn refutes(&self, mass: &Rational, activation: &Rational) -> bool {
        match self {
            Self::Deadline { threshold, .. } => mass > *threshold,
            Self::Elapsed { q, strict, .. } => {
                if *strict {
                    *mass > zero() && *mass >= q.mul(activation)
                } else {
                    *mass > one().sub(q).mul(activation)
                }
            }
        }
    }
}

fn price_paths(
    graph: &CheckerGraph,
    payoff: &Payoff<'_>,
    sequences: &[Vec<ProbabilityProductKey>],
    max_rational_bits: u64,
    cancel: &Cancel,
) -> Result<Option<(Vec<usize>, Rational)>, super::super::ProbabilityWitnessError> {
    use super::super::ProbabilityWitnessError as Error;
    let cancelled = |error: ProbabilityDocumentError| match error {
        ProbabilityDocumentError::Cancelled(cause) => {
            Error::Graph(ExactProbabilityGraphError::Cancelled(cause))
        }
        _ => Error::Graph(ExactProbabilityGraphError::LimitReached),
    };
    let limit_bits = |value: &Rational| {
        if value.max_part_bits() > max_rational_bits {
            Err(Error::Limit {
                limit: crate::ModelCheckLimit::ExactProbMaxRationalBits,
                value: max_rational_bits,
            })
        } else {
            Ok(())
        }
    };
    let covers = |choice: &Vec<ProbabilityEdge>, target: &ProbabilityProductKey| {
        choice
            .iter()
            .any(|edge| &edge.target == target && edge.probability > zero())
    };
    // The witness selects the scheduler; the mass is the checker graph's own
    // probability of the failing outcome under that memoryless choice (states
    // the witness never visits take their first choice). A step the checker
    // does not itself have therefore contributes nothing, and a cycle under the
    // choice is refused rather than approximated.
    #[allow(clippy::too_many_arguments)]
    fn mass_of<'a>(
        graph: &'a CheckerGraph,
        chosen: &BTreeMap<ProbabilityProductKey, usize>,
        weight: &dyn Fn(&ProbabilityProductKey, bool) -> Rational,
        key: &'a ProbabilityProductKey,
        memo: &mut BTreeMap<&'a ProbabilityProductKey, Rational>,
        on_path: &mut BTreeSet<&'a ProbabilityProductKey>,
        cancel: &Cancel,
        check_bits: &dyn Fn(&Rational) -> Result<(), super::super::ProbabilityWitnessError>,
    ) -> Result<Rational, super::super::ProbabilityWitnessError> {
        use super::super::ProbabilityWitnessError as Error;
        if let Some(value) = memo.get(key) {
            return Ok(value.clone());
        }
        poll(cancel).map_err(|_| Error::Graph(ExactProbabilityGraphError::LimitReached))?;
        let state = graph.states.get(key).ok_or(Error::Refused)?;
        let value = match state.terminal {
            Some(verdict) => weight(key, verdict),
            None => {
                if !on_path.insert(key) {
                    return Err(Error::Refused);
                }
                let index = chosen.get(key).copied().unwrap_or(0);
                let mut sum = zero();
                if let Some(choice) = state.choices.get(index) {
                    for edge in choice.iter().filter(|edge| edge.probability > zero()) {
                        let child = mass_of(
                            graph,
                            chosen,
                            weight,
                            &edge.target,
                            memo,
                            on_path,
                            cancel,
                            check_bits,
                        )?;
                        sum = sum.add(&edge.probability.mul(&child));
                        check_bits(&sum)?;
                    }
                }
                on_path.remove(key);
                sum
            }
        };
        memo.insert(key, value.clone());
        Ok(value)
    }
    // The choices and the mass a set of paths selects. `None` when a path is
    // not a distinct walk to the forbidden decided state (a Parity case).
    let solve = |paths: &[usize]| -> Result<
        Option<(BTreeMap<ProbabilityProductKey, usize>, Rational, Rational)>,
        Error,
    > {
        let mut used: BTreeMap<ProbabilityProductKey, BTreeSet<ProbabilityProductKey>> =
            BTreeMap::new();
        let mut seen = BTreeSet::new();
        for index in paths {
            let path = &sequences[*index];
            poll(cancel).map_err(cancelled)?;
            let mut at = graph.initial.clone();
            let mut sequence = Vec::with_capacity(path.len());
            for step in path {
                let state = graph.states.get(&at).ok_or(Error::Refused)?;
                if state.terminal.is_some() {
                    return Err(Error::Refused);
                }
                used.entry(at.clone()).or_default().insert(step.clone());
                sequence.push(step.clone());
                at = step.clone();
            }
            let end = graph.states.get(&at).ok_or(Error::Refused)?;
            if sequence.is_empty() || !payoff.ends(&at, end.terminal) {
                return Ok(None);
            }
            if !seen.insert(sequence) {
                return Ok(None);
            }
        }
        // One choice per visited state: the one the paths cover most (the first
        // on a tie).
        let mut chosen: BTreeMap<ProbabilityProductKey, usize> = BTreeMap::new();
        for (key, targets) in &used {
            let state = graph.states.get(key).ok_or(Error::Refused)?;
            let mut best: Option<(usize, usize)> = None;
            for (position, choice) in state.choices.iter().enumerate() {
                let covered = targets
                    .iter()
                    .filter(|target| covers(choice, target))
                    .count();
                if best.is_none_or(|(_, most)| covered > most) {
                    best = Some((position, covered));
                }
            }
            let (index, _) = best.ok_or(Error::Refused)?;
            chosen.insert(key.clone(), index);
        }
        let priced = |weight: &dyn Fn(&ProbabilityProductKey, bool) -> Rational| {
            mass_of(
                graph,
                &chosen,
                weight,
                &graph.initial,
                &mut BTreeMap::new(),
                &mut BTreeSet::new(),
                cancel,
                &limit_bits,
            )
        };
        let mass = priced(&|key, verdict| payoff.weight(key, verdict))?;
        let activation = priced(&|key, _| payoff.activation(key))?;
        Ok(Some((chosen, mass, activation)))
    };
    // The reported evidence must reproduce itself: keep only the paths every
    // step of which is an outcome of the chosen choice, then re-derive the
    // choices and the mass from those paths alone, until the set is stable. The
    // reported mass is the last one derived and still exceeds the threshold.
    let mut kept: Vec<usize> = (0..sequences.len()).collect();
    let mass = loop {
        let Some((chosen, mass, activation)) = solve(&kept)? else {
            return Ok(None);
        };
        if !payoff.refutes(&mass, &activation) {
            return Ok(None);
        }
        let before = kept.len();
        kept.retain(|index| {
            let mut at = &graph.initial;
            sequences[*index].iter().all(|step| {
                let covered = graph
                    .states
                    .get(at)
                    .zip(chosen.get(at))
                    .and_then(|(state, index)| state.choices.get(*index))
                    .is_some_and(|choice| covers(choice, step));
                at = step;
                covered
            })
        });
        if kept.is_empty() {
            return Ok(None);
        }
        if kept.len() == before {
            break mass;
        }
    };
    Ok(Some((kept, mass)))
}

/// Whether `claim` is a timed form this module replays a witness of: FR-204's
/// deadline form, or FR-877's elapsed quantile over a timed subject.
pub(in crate::probability_certificate) fn is_timed_witness_claim(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
) -> bool {
    deadline_event(claim).is_some()
        || (elapsed::query(claim).is_some()
            && TimedSystem::new(system, &claim.subject.alias).is_ok())
}

/// The clock valuation a digital timed product key names, in
/// [`TimedSystem::places`] order: each place's canonical identity is looked
/// up in the key, so the key's own (identity-sorted) clock order never
/// decides which clock a value belongs to. `None` for a region key, a key
/// that names a clock twice or not at all, or a value that is not a whole
/// number of ticks.
pub fn digital_key_clocks(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    key: &ProbabilityProductKey,
) -> Option<fsl_eval::simulation::model::ClockValuation> {
    let root: JsonValue = serde_json::from_slice(key.as_bytes()).ok()?;
    let keyed = root.get(4)?.get(1)?;
    if keyed.get(0)?.as_str()? != "digital" {
        return None;
    }
    let number = |value: &JsonValue| -> Option<u64> {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    };
    let scale = number(keyed.get(3)?)?;
    let listed = keyed.get(4)?.as_array()?;
    let timed = TimedSystem::new(system, &claim.subject.alias).ok()?;
    if listed.len() != timed.places().len() {
        return None;
    }
    let mut values = Vec::with_capacity(listed.len());
    for place in timed.places() {
        let field = timed.model().clock(place.clock())?;
        let owner = field.owner();
        let owner = owner.original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .ok()?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring).ok()?;
        let receiver =
            fsl_eval::simulation::model::reference_canonical_text(place.object()).ok()?;
        let receiver: JsonValue = serde_json::from_str(&receiver).ok()?;
        let identity = json!([declaring, field.name(), receiver]);
        let mut matching = listed
            .iter()
            .filter(|clock| clock.get(0) == Some(&identity));
        let clock = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        let whole = number(clock.get(1)?)?;
        values
            .push(Rational::from_integer(whole.into()).div(&Rational::from_integer(scale.into()))?);
    }
    Some(fsl_eval::simulation::model::ClockValuation::new(values))
}

#[cfg(test)]
mod witness_pricing_tests {
    use super::*;

    fn key(name: &str) -> ProbabilityProductKey {
        ProbabilityProductKey::from_canonical_bytes(name.as_bytes().to_vec())
    }

    fn part(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
    }

    fn edge(target: &str, numerator: i64, denominator: i64) -> ProbabilityEdge {
        ProbabilityEdge {
            target: key(target),
            probability: part(numerator, denominator),
        }
    }

    fn graph(states: Vec<(&str, Option<bool>, Vec<Vec<ProbabilityEdge>>)>) -> CheckerGraph {
        CheckerGraph {
            initial: key("I"),
            states: states
                .into_iter()
                .map(|(name, terminal, choices)| (key(name), CheckerState { terminal, choices }))
                .collect(),
        }
    }

    fn path(steps: &[&str]) -> Vec<ProbabilityProductKey> {
        steps.iter().map(|step| key(step)).collect()
    }

    fn elapsed_price(
        graph: &CheckerGraph,
        violating: &[&str],
        activated: &[&str],
        q: Rational,
        strict: bool,
        sequences: &[Vec<ProbabilityProductKey>],
    ) -> Option<(Vec<usize>, Rational)> {
        let violating = violating.iter().map(|name| key(name)).collect();
        let activated = activated.iter().map(|name| key(name)).collect();
        let payoff = Payoff::Elapsed {
            violating: &violating,
            activated: &activated,
            q: &q,
            strict,
        };
        price_paths(graph, &payoff, sequences, 4096, &Cancel::new()).expect("the graph prices")
    }

    /// FR-877: a `>=` refutation is the non-strict `max >= 0`, so a counted
    /// mass exactly `q` of the decided mass refutes, while a `<=` refutation is
    /// the strict `min < 0`, so the same equality does not.
    #[test]
    fn an_elapsed_refutation_is_non_strict_for_at_least_and_strict_for_at_most() {
        let graph = graph(vec![
            ("I", None, vec![vec![edge("a", 1, 2), edge("b", 1, 2)]]),
            ("a", Some(false), vec![]),
            ("b", Some(false), vec![]),
        ]);
        let sequences = [path(&["a"])];
        let both = ["a", "b"];
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(1, 2), true, &sequences),
            Some((vec![0], part(1, 2)))
        );
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(3, 5), true, &sequences),
            None
        );
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(1, 2), false, &sequences),
            None
        );
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(3, 5), false, &sequences),
            Some((vec![0], part(1, 2)))
        );
    }

    /// FR-877: only decided measures weigh against `q`; mass that ends in a
    /// state no measure decided is neither counted nor part of the activated
    /// mass, and a path to a decided measure the refutation does not count is
    /// no evidence.
    #[test]
    fn an_elapsed_refutation_prices_against_the_decided_mass_only() {
        let graph = graph(vec![
            (
                "I",
                None,
                vec![vec![edge("a", 1, 4), edge("b", 1, 4), edge("c", 1, 2)]],
            ),
            ("a", Some(false), vec![]),
            ("b", Some(false), vec![]),
            ("c", Some(false), vec![]),
        ]);
        let both = ["a", "b"];
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(1, 2), true, &[path(&["a"])]),
            Some((vec![0], part(1, 4)))
        );
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(1, 2), true, &[path(&["b"])]),
            None
        );
        assert_eq!(
            elapsed_price(&graph, &["a"], &both, part(1, 2), true, &[path(&["c"])]),
            None
        );
    }

    fn price(
        graph: &CheckerGraph,
        threshold: Rational,
        sequences: &[Vec<ProbabilityProductKey>],
    ) -> Option<(Vec<usize>, Rational)> {
        price_witness_paths(graph, false, &threshold, sequences, 4096, &Cancel::new())
            .expect("the graph prices")
    }

    /// The tie's two orderings: `A` fires the operation that retries at once, `B`
    /// fires the one that loses half the time and retries later.
    fn tie() -> CheckerGraph {
        graph(vec![
            (
                "I",
                None,
                vec![
                    vec![edge("ACC", 1, 1)],
                    vec![edge("L", 1, 2), edge("REJ", 1, 2)],
                ],
            ),
            ("L", None, vec![vec![edge("ACC2", 1, 1)]]),
            ("ACC", Some(true), vec![]),
            ("ACC2", Some(true), vec![]),
            ("REJ", Some(false), vec![]),
        ])
    }

    /// A witness whose path takes the NON-retrying ordering and still ends in
    /// an accepted state selects that ordering, whose mass is below the bound,
    /// so it does not refute; the honest path selects the other ordering.
    #[test]
    fn a_path_of_the_non_failing_ordering_that_ends_accepted_does_not_refute() {
        let graph = tie();
        assert_eq!(price(&graph, part(3, 5), &[path(&["L", "ACC2"])]), None);
        assert_eq!(
            price(&graph, part(3, 5), &[path(&["ACC"])]),
            Some((vec![0], part(1, 1)))
        );
    }

    /// A path whose first step is no outcome of the chosen choice is dropped
    /// from the evidence, and the choices and the mass are re-derived from the
    /// kept path alone: here dropping it changes the choice at `T`, the mass
    /// falls to the bound and the witness no longer refutes.
    #[test]
    fn evidence_is_priced_from_the_validated_paths_alone() {
        let graph = graph(vec![
            (
                "I",
                None,
                vec![
                    vec![edge("T", 1, 1)],
                    vec![edge("T", 1, 1)],
                    vec![edge("X", 1, 1)],
                ],
            ),
            ("X", None, vec![vec![edge("T", 1, 1)]]),
            (
                "T",
                None,
                vec![
                    vec![edge("a", 1, 4), edge("REJ", 3, 4)],
                    vec![edge("a", 1, 2), edge("b", 1, 2)],
                ],
            ),
            ("a", Some(true), vec![]),
            ("b", Some(true), vec![]),
            ("REJ", Some(false), vec![]),
        ]);
        let kept = path(&["T", "a"]);
        let dropped = path(&["X", "T", "b"]);
        // The two together choose the second choice at `T` (mass 1), but the
        // dropped path leaves, and the kept one alone ties back to the first.
        assert_eq!(price(&graph, part(1, 2), &[kept.clone(), dropped]), None);
        // Alone, the kept path prices the first choice at `T`.
        assert_eq!(
            price(&graph, part(1, 5), &[kept]),
            Some((vec![0], part(1, 4)))
        );
    }
}
