// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fair cyclic MDP proof-role checks, kept separate from timed trace replay.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn check_bounds(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    document: &ProbabilityCertificateDocument,
    objective: ProbabilityObjective,
    bound: &Bound,
    side: SubsystemBoundKind,
    offer: Offer<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    cx: &mut mdp::Context<'_, '_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    if !matches!(
        objective,
        ProbabilityObjective::Minimum | ProbabilityObjective::Maximum
    ) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if offer.relation().misplaced(bound) {
        return Ok(misplaced_kind());
    }
    let values = match mdp::rows(
        graph,
        &ExactProbabilityCertificate {
            values: document.values.clone(),
        },
        cx,
    )? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Rejected(rejection) => {
            return Ok(ExactProbabilityCertificateCheck::Rejected(rejection));
        }
    };
    let action_sums = mdp::sums(graph, &values, None, cx)?;
    let existential = matches!(
        (objective, side),
        (ProbabilityObjective::Minimum, SubsystemBoundKind::Upper)
            | (ProbabilityObjective::Maximum, SubsystemBoundKind::Lower)
    );
    let hints = if existential {
        match offer.policy {
            Some(rows) => mdp::hinted_choices(system, graph, captured, rows, cx)?,
            None => BTreeMap::new(),
        }
    } else {
        if offer.policy.is_some() {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
        BTreeMap::new()
    };
    let mut selected = Vec::new();
    for (key, state) in &graph.states {
        cx.tick(mdp::CyclicMdpProbabilityPhase::ActionSums)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        if let Some(target) = state.terminal {
            if *value != rational(i64::from(target)) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            continue;
        }
        let sums = action_sums
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if sums.is_empty() {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let satisfies = |sum: &Rational| match side {
            SubsystemBoundKind::Lower => value <= sum,
            SubsystemBoundKind::Upper => sum <= value,
        };
        if existential {
            let schedules = mdp::schedules_at(system, captured, key, cx)?;
            if schedules.len() != sums.len() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            let candidates = match hints.get(key) {
                Some(None) => {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
                Some(Some(index)) => vec![*index],
                None => {
                    let mut order = (0..schedules.len()).collect::<Vec<_>>();
                    order.sort_by(|left, right| schedules[*left].cmp(&schedules[*right]));
                    order
                }
            };
            let Some(index) = candidates
                .into_iter()
                .find(|index| sums.get(*index).is_some_and(satisfies))
            else {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            };
            selected.push((
                key.clone(),
                crate::ProductAction::Identity(schedules[index].clone()),
            ));
        } else if sums.iter().any(|sum| !satisfies(sum)) {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            ));
        }
    }
    if existential {
        if !policy_is_fair(system, claim, graph, captured, &selected, cx)? {
            return Ok(rejected(
                ProbabilityCertificateRule::RankBroken,
                graph.initial.clone(),
            ));
        }
    }
    match (objective, side) {
        (ProbabilityObjective::Minimum, SubsystemBoundKind::Lower) => {
            // Universal lower bounds may be positive only outside fair
            // non-target recurrent components.
            if has_fair_non_target_end_component(system, claim, graph, captured, &values, cx)? {
                return Ok(rejected(
                    ProbabilityCertificateRule::RankBroken,
                    graph.initial.clone(),
                ));
            }
        }
        (ProbabilityObjective::Maximum, SubsystemBoundKind::Lower) => {
            if !policy_bottom_values_zero(system, graph, captured, &selected, &values, cx)? {
                return Ok(rejected(
                    ProbabilityCertificateRule::RankBroken,
                    graph.initial.clone(),
                ));
            }
        }
        _ => {}
    }
    let mut result = check_threshold(graph, &values, bound, true, limits, cancel)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut result {
        accepted.policy = selected;
    }
    Ok(result)
}

/// Reject a positive lower value on a reachable fair non-target end component.
/// Components and closed actions are rebuilt from checker graph edges.
fn has_fair_non_target_end_component(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    values: &mdp::Values,
    cx: &mut mdp::Context<'_, '_>,
) -> Result<bool, ProbabilityDocumentError> {
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};

    let keys = graph
        .states
        .iter()
        .filter_map(|(key, state)| state.terminal.is_none().then_some(key.clone()))
        .collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut identities = Vec::with_capacity(keys.len());
    for key in &keys {
        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
        let model_state = captured
            .borrow()
            .get(key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let actions = system
            .actions_with_cancel(&model_state, cx.cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let graph_state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if actions.len() != graph_state.choices.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        identities.push(
            actions
                .into_iter()
                .map(|action| action.identity)
                .collect::<Vec<_>>(),
        );
    }

    let mut pending = vec![(0..keys.len()).collect::<Vec<_>>()];
    while let Some(mut candidate) = pending.pop() {
        loop {
            let mut internal = BTreeMap::<usize, Vec<usize>>::new();
            for &state_index in &candidate {
                let key = &keys[state_index];
                let state = graph
                    .states
                    .get(key)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let choices = state
                    .choices
                    .iter()
                    .enumerate()
                    .filter_map(|(choice_index, edges)| {
                        edges
                            .iter()
                            .all(|edge| {
                                edge.probability <= rational(0)
                                    || positions
                                        .get(&edge.target)
                                        .is_some_and(|target| candidate.contains(target))
                            })
                            .then_some(choice_index)
                    })
                    .collect::<Vec<_>>();
                if !choices.is_empty() {
                    internal.insert(state_index, choices);
                }
            }
            let pruned = candidate
                .iter()
                .copied()
                .filter(|state| internal.contains_key(state))
                .collect::<Vec<_>>();
            if pruned.is_empty() {
                break;
            }
            if pruned.len() != candidate.len() {
                candidate = pruned;
                continue;
            }
            let local = candidate
                .iter()
                .enumerate()
                .map(|(index, state)| (*state, index))
                .collect::<BTreeMap<_, _>>();
            let mut edges = vec![Vec::new(); candidate.len()];
            let mut reverse = vec![Vec::new(); candidate.len()];
            for (local_index, &state_index) in candidate.iter().enumerate() {
                let key = &keys[state_index];
                let state = graph
                    .states
                    .get(key)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let choices = internal
                    .get(&state_index)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                for &choice_index in choices {
                    for edge in &state.choices[choice_index] {
                        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
                        if edge.probability <= rational(0) {
                            continue;
                        }
                        let target = *positions
                            .get(&edge.target)
                            .and_then(|global| local.get(global))
                            .ok_or(ProbabilityDocumentError::Malformed)?;
                        edges[local_index].push(target);
                        reverse[target].push(local_index);
                    }
                }
            }
            let (_, components, _) =
                super::super::bottom_components(&edges, &reverse, &mut |_| {
                    cx.tick(mdp::CyclicMdpProbabilityPhase::Components)
                })?;
            if components.len() != 1 || components[0].len() != candidate.len() {
                for component in components {
                    pending.push(
                        component
                            .into_iter()
                            .map(|index| candidate[index])
                            .collect(),
                    );
                }
                break;
            }

            let mut fair = true;
            for constraint in &claim.fairness {
                cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
                let Some(operation) = system.operation_text(
                    constraint.operation().declaring_type(),
                    constraint.operation().name(),
                ) else {
                    continue;
                };
                let mut obligations = Vec::new();
                if constraint.granularity() == FairnessGranularity::Whole {
                    obligations.push(None);
                } else {
                    for &state_index in &candidate {
                        for identity in &identities[state_index] {
                            if identity.operation == operation
                                && !obligations
                                    .iter()
                                    .any(|known: &Option<_>| known == &Some(identity))
                            {
                                obligations.push(Some(identity));
                            }
                        }
                    }
                }
                for obligation in obligations {
                    let matches = |identity: &fsl_eval::simulation::model::ScheduledIdentity| {
                        identity.operation == operation
                            && obligation.is_none_or(|wanted| identity == wanted)
                    };
                    let mut enabled_all = true;
                    let mut enabled_any = false;
                    let mut internal_any = false;
                    for &state_index in &candidate {
                        let enabled = identities[state_index].iter().any(matches);
                        enabled_all &= enabled;
                        enabled_any |= enabled;
                        internal_any |= internal[&state_index]
                            .iter()
                            .any(|choice| matches(&identities[state_index][*choice]));
                    }
                    let required = match constraint.kind() {
                        FairnessKind::Weak => enabled_all,
                        FairnessKind::Strong => enabled_any,
                    };
                    if required && !internal_any {
                        fair = false;
                        break;
                    }
                }
                if !fair {
                    break;
                }
            }
            if fair
                && candidate.iter().any(|state| {
                    values
                        .get(&keys[*state])
                        .is_some_and(|value| *value > rational(0))
                })
            {
                return Ok(true);
            }
            break;
        }
    }
    Ok(false)
}

fn policy_is_fair(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    selected: &[(ProbabilityProductKey, crate::ProductAction)],
    cx: &mut mdp::Context<'_, '_>,
) -> Result<bool, ProbabilityDocumentError> {
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
    let selected = selected.iter().cloned().collect::<BTreeMap<_, _>>();
    let keys = graph.states.keys().cloned().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    let mut enabled = Vec::with_capacity(keys.len());
    let mut taken = Vec::with_capacity(keys.len());
    for (index, key) in keys.iter().enumerate() {
        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
        let state = &graph.states[key];
        let Some(action) = selected.get(key) else {
            edges[index].push(index);
            reverse[index].push(index);
            enabled.push(Vec::new());
            taken.push(None);
            continue;
        };
        let ProductAction::Identity(schedule) = action else {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        };
        let actions = system
            .actions_with_cancel(
                &captured
                    .borrow()
                    .get(key)
                    .cloned()
                    .ok_or(ProbabilityDocumentError::Malformed)?,
                cx.cancel,
            )
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        if actions.len() != state.choices.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let schedules = mdp::schedules_at(system, captured, key, cx)?;
        let Some(choice) = schedules.iter().position(|candidate| candidate == schedule) else {
            return Ok(false);
        };
        enabled.push(
            actions
                .iter()
                .map(|action| action.identity.clone())
                .collect::<Vec<_>>(),
        );
        taken.push(Some(actions[choice].identity.clone()));
        for edge in &state.choices[choice] {
            if edge.probability <= rational(0) {
                continue;
            }
            let target = *positions
                .get(&edge.target)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            edges[index].push(target);
            reverse[target].push(index);
        }
    }
    let (_, components, bottom) = super::super::bottom_components(&edges, &reverse, &mut |_| {
        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)
    })?;
    for (component_index, component) in components.iter().enumerate() {
        if !bottom[component_index] {
            continue;
        }
        for constraint in &claim.fairness {
            cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
            let Some(operation) = system.operation_text(
                constraint.operation().declaring_type(),
                constraint.operation().name(),
            ) else {
                continue;
            };
            let mut obligations: Vec<Option<&fsl_eval::simulation::model::ScheduledIdentity>> =
                Vec::new();
            if constraint.granularity() == FairnessGranularity::Whole {
                obligations.push(None);
            } else {
                for &state in component {
                    for identity in &enabled[state] {
                        if identity.operation == operation
                            && !obligations.iter().any(|known| *known == Some(identity))
                        {
                            obligations.push(Some(identity));
                        }
                    }
                }
            }
            for obligation in obligations {
                let matches = |identity: &fsl_eval::simulation::model::ScheduledIdentity| {
                    identity.operation == operation
                        && obligation.is_none_or(|wanted| identity == wanted)
                };
                let mut enabled_all = true;
                let mut enabled_any = false;
                let mut taken_any = false;
                for &state in component {
                    let is_enabled = enabled[state].iter().any(matches);
                    enabled_all &= is_enabled;
                    enabled_any |= is_enabled;
                    taken_any |= taken[state].as_ref().is_some_and(matches);
                }
                let required = match constraint.kind() {
                    FairnessKind::Weak => enabled_all,
                    FairnessKind::Strong => enabled_any,
                };
                if required && !taken_any {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn policy_bottom_values_zero(
    system: &ModelSystem<'_>,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    selected: &[(ProbabilityProductKey, crate::ProductAction)],
    values: &mdp::Values,
    cx: &mut mdp::Context<'_, '_>,
) -> Result<bool, ProbabilityDocumentError> {
    let selected = selected.iter().cloned().collect::<BTreeMap<_, _>>();
    let keys = graph.states.keys().cloned().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    for (index, key) in keys.iter().enumerate() {
        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)?;
        let state = &graph.states[key];
        let Some(action) = selected.get(key) else {
            edges[index].push(index);
            reverse[index].push(index);
            continue;
        };
        let ProductAction::Identity(schedule) = action else {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        };
        let choice = mdp::schedules_at(system, captured, key, cx)?
            .iter()
            .position(|candidate| candidate == schedule)
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        for edge in &state.choices[choice] {
            if edge.probability <= rational(0) {
                continue;
            }
            let target = *positions
                .get(&edge.target)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            edges[index].push(target);
            reverse[target].push(index);
        }
    }
    let (_, components, bottom) = super::super::bottom_components(&edges, &reverse, &mut |_| {
        cx.tick(mdp::CyclicMdpProbabilityPhase::Components)
    })?;
    for (component_index, component) in components.iter().enumerate() {
        if bottom[component_index]
            && component.iter().any(|state| {
                graph.states[&keys[*state]].terminal != Some(true)
                    && values
                        .get(&keys[*state])
                        .is_some_and(|value| *value > rational(0))
            })
        {
            return Ok(false);
        }
    }
    Ok(true)
}
