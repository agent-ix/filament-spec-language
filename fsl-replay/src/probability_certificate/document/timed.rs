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
use serde_json::{json, Value as JsonValue};

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
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
    if objective.form != TimedProbabilityForm::Deadline {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let Some((bound, predicate)) = deadline_event(claim) else {
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
    if document.optimization != optimization || !document.ranking.is_empty() {
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
    if !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if system
        .input()
        .probabilistic
        .delays()
        .iter()
        .any(|delay| delay.operation().model() == claim.subject.alias.as_str())
    {
        return Err(ProbabilityDocumentError::Unsupported);
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
            predicate,
            objective.scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
        )?,
        TimedProbabilityRoute::Region => enumerate_region_deadline(
            system,
            &timed,
            claim,
            binding.as_ref(),
            predicate,
            objective.scale,
            scale,
            limits,
            cancel,
            &mut meter,
            &mut storage,
        )?,
    };
    if target_keys(&graph) != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let mut checked =
        super::super::check_exact_reachability(&graph, &certificate, optimization, &bound, cancel)
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
    if route != TimedProbabilityRoute::Region
        || !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
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
    for (key, state) in &graph.states {
        poll(cancel)?;
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
            ProbabilityObjective::Workload => return Err(ProbabilityDocumentError::Unsupported),
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
                    ProbabilityObjective::Workload => unreachable!(),
                },
            });
            match objective {
                ProbabilityObjective::Maximum => progress &= descends,
                ProbabilityObjective::Minimum if sum == *value => progress |= descends,
                ProbabilityObjective::Minimum => {}
                ProbabilityObjective::Workload => unreachable!(),
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
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<
    (
        CheckerGraph,
        BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>,
    ),
    ProbabilityDocumentError,
> {
    let (base_scale, subdivisions) = region_scale_parts(timed, claim, scale)?;
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
        [fsl_eval::simulation::model::PredicateReading::Undefined { .. }
        | fsl_eval::simulation::model::PredicateReading::Refused(_)] => {
            Err(ProbabilityDocumentError::Unsupported)
        }
        _ => Err(ProbabilityDocumentError::Unsupported),
    }
}

fn mean_region_key(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    binding: Option<&filament_exact::ObjectReference>,
    state: &MeanRegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    scale: u64,
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
    )?;
    let delta = rational_units(1, scale)?;
    let max_steps = caps
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .saturating_add(subdivisions.saturating_mul(2))
        .saturating_add(1);
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
        )?;
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone)]
struct RegionState {
    timed: TimedState,
    elapsed: u64,
    phase: Phase,
}

fn enumerate_digital_deadline(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    key_scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let deadline = time_bound(claim, key_scale)?;
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
    let phase = phase_at(
        system,
        predicate,
        binding,
        state.discrete(),
        0,
        deadline,
        cancel,
    )?;
    let first = RegionState {
        timed: state,
        elapsed: 0,
        phase,
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
            Phase::Pending => {}
        }
        if timed
            .is_quiescent(current.timed.discrete())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut choices = Vec::new();
        if let Some((delayed_key, delayed)) = next_digital_delay(
            system, timed, predicate, binding, &current, &caps, key_scale, deadline, cancel,
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
                let phase = phase_at(
                    system,
                    predicate,
                    binding,
                    post.discrete(),
                    current.elapsed,
                    deadline,
                    cancel,
                )?;
                let reached_state = RegionState {
                    timed: post,
                    elapsed: current.elapsed,
                    phase,
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

fn enumerate_region_deadline(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    key_scale: u64,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let (base_scale, subdivisions) = region_scale_parts(timed, claim, scale)?;
    let deadline = time_bound(claim, key_scale)?;
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
    let phase = phase_at(
        system,
        predicate,
        binding,
        state.discrete(),
        0,
        deadline,
        cancel,
    )?;
    let first = RegionState {
        timed: state,
        elapsed: 0,
        phase,
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
            Phase::Pending => {}
        }
        if timed
            .is_quiescent(current.timed.discrete())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        {
            // Idle observation adds a distinct scheduler choice. This slice
            // refuses it until the tail has a checker-owned product state.
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut choices = Vec::new();
        if let Some((delayed_key, delayed)) = next_region_delay(
            system,
            timed,
            predicate,
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
                let phase = phase_at(
                    system,
                    predicate,
                    binding,
                    post.discrete(),
                    current.elapsed,
                    deadline,
                    cancel,
                )?;
                let reached_state = RegionState {
                    timed: post,
                    elapsed: current.elapsed,
                    phase,
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
    );
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
    let monitor = match state.phase {
        Phase::Pending => "pending",
        Phase::Accepted => "accept",
        Phase::Rejected => "reject",
    };
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
    if state.elapsed < deadline.saturating_add(subdivisions) {
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
    let deadline_key = json!([
        "deadline",
        (deadline / subdivisions).to_string(),
        (state.elapsed / subdivisions).to_string(),
        fraction_rank(state.elapsed, deadline.saturating_add(subdivisions))
    ]);
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
        model,
        ["deadline", "eventually", monitor],
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
    let phase = match state.phase {
        Phase::Pending => "pending",
        Phase::Accepted => "accept",
        Phase::Rejected => "reject",
    };
    digital_probability_product_key(DigitalProductKeyInput {
        binding,
        discrete,
        unit,
        scale,
        clocks,
        deadline: Some(DigitalDeadlineKey {
            limit: deadline,
            elapsed: state.elapsed,
        }),
        tail: DigitalTail::Running,
        monitor: Some(json!(["deadline", "eventually", phase])),
        accumulator: None,
        pending: None,
    })
    .map_err(ProbabilityDocumentError::Graph)
}

fn next_digital_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
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
        let phase = phase_at(
            system,
            predicate,
            binding,
            delayed.discrete(),
            elapsed,
            deadline,
            cancel,
        )?;
        let reached = RegionState {
            timed: delayed,
            elapsed,
            phase,
        };
        let key = digital_key(system, timed, binding, &reached, caps, scale, deadline)?;
        if key != current_key {
            return Ok(Some((key, reached)));
        }
        state = reached;
    }
    Ok(None)
}

fn next_region_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
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
        let phase = phase_at(
            system,
            predicate,
            binding,
            delayed.discrete(),
            elapsed,
            deadline,
            cancel,
        )?;
        let reached = RegionState {
            timed: delayed,
            elapsed,
            phase,
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
    }) || matches!(
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
        let has_deadline = matches!(
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
