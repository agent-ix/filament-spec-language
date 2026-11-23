// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed cyclic until and always proof profiles: the checker-owned
//! classification graph, its maximal end components and the certificate graph
//! over the violating union. Nothing here reads a producer component set,
//! classification or target set.

use super::cyclic::{analyze, charge, enumerate_graph_capturing, visit, CapturedStates};
use super::*;
use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};

/// Which closed profile the checked document selects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Profile {
    /// Exactly one semantic enabled distribution per live state.
    Dtmc,
    /// Every enabled semantic action of every live state.
    Mdp,
}

/// The stage a caller control or observer sees.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Stage {
    /// Whole original reachable graph admission.
    Original,
    /// Classification graph: every decided state absorbs.
    Classification,
    /// End-component and bottom-component decomposition.
    Components,
    /// Certificate graph: the violating union is not expanded.
    Certificate,
}

/// The checker-derived certificate graph of one claim.
pub(super) struct Subject {
    /// Union states are targets; decided states outside the union are zero.
    pub(super) graph: CheckerGraph,
    /// Shortest-distance ranks, derived for the DTMC profile only.
    pub(super) ranks: Option<BTreeMap<ProbabilityProductKey, u64>>,
    /// The claim's event.
    pub(super) event: ProbabilityStateEvent,
    /// `maximum` under every scheduler, `workload` under a workload.
    pub(super) objective: ProbabilityObjective,
    /// The claim's own bound.
    pub(super) bound: Bound,
}

/// The state-predicate event a claim target names.
pub(super) fn event_of(target: &ReachTarget) -> Option<ProbabilityStateEvent> {
    match target {
        ReachTarget::Until(..) => Some(ProbabilityStateEvent::Until),
        ReachTarget::Always(_) => Some(ProbabilityStateEvent::Always),
        ReachTarget::Eventually(_) => None,
    }
}

/// Whether the claim is an unwindowed until or always claim.
pub(super) fn is_claim(claim: &CheckedProbabilisticClaim) -> bool {
    matches!(
        &claim.form,
        ProbForm::Reach {
            target: ReachTarget::Until(..) | ReachTarget::Always(_),
            ..
        }
    )
}

/// The optimization these profiles require, derived from the checked claim.
pub(super) fn objective_of(scheduler: &ProbScheduler) -> ProbabilityObjective {
    match scheduler {
        ProbScheduler::Workload(_) => ProbabilityObjective::Workload,
        ProbScheduler::Every => ProbabilityObjective::Maximum,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn certificate_graph(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    profile: Profile,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: fsl_foundation::IdentityLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    stage: &mut dyn FnMut(Stage) -> Result<(), ProbabilityDocumentError>,
) -> Result<Subject, ProbabilityDocumentError> {
    let ProbForm::Reach { bound, target } = &claim.form else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let event = event_of(target).ok_or(ProbabilityDocumentError::Unsupported)?;
    let (Bound::AtLeast(threshold) | Bound::AtMost(threshold)) = bound;
    if !claim.fairness.is_empty()
        || fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
        || (profile == Profile::Mdp && !matches!(claim.scheduler, ProbScheduler::Every))
        || *threshold <= rational(0)
        || *threshold >= rational(1)
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    stage(Stage::Original)?;
    let captured = CapturedStates::default();
    let mut original = enumerate_graph_capturing::<fsl_semantics::check::CheckedPredicate>(
        system,
        None,
        binding,
        workload,
        limits,
        cancel,
        identity,
        meter,
        storage,
        false,
        true,
        Some(&captured),
    )?;
    for state in original.states.values() {
        if profile == Profile::Dtmc && state.choices.len() > 1 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        for choice in &state.choices {
            let mut mass = rational(0);
            for edge in choice {
                visit(cancel, meter)?;
                if edge.probability <= rational(0) {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                super::document::check_scalar(&edge.probability, limits)?;
                mass = super::document::arithmetic(
                    filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
                    meter,
                    cancel,
                )?;
                super::document::check_scalar(&mass, limits)?;
            }
            if mass != rational(1) {
                return Err(ProbabilityDocumentError::Malformed);
            }
        }
    }

    stage(Stage::Classification)?;
    let mut classified: BTreeMap<ProbabilityProductKey, CheckerState> = BTreeMap::new();
    let mut pending = VecDeque::from([original.initial.clone()]);
    while let Some(key) = pending.pop_front() {
        visit(cancel, meter)?;
        if classified.contains_key(&key) {
            continue;
        }
        let mut state = original
            .states
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let model = captured
            .borrow_mut()
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let (verdict, _) = state_predicate::outcome(
            system,
            target,
            binding,
            &model,
            state.choices.is_empty(),
            cancel,
            #[cfg(feature = "test-support")]
            &mut |_| {},
        )?;
        state.terminal = verdict;
        if verdict.is_some() {
            state.choices.clear();
        } else {
            for edge in state.choices.iter().flatten() {
                visit(cancel, meter)?;
                pending.push_back(edge.target.clone());
            }
        }
        classified.insert(key, state);
    }
    drop(captured);

    stage(Stage::Components)?;
    let keys: Vec<&ProbabilityProductKey> = classified.keys().collect();
    let components = end_components(&classified, limits, storage, cancel, meter)?;
    // The verdict a decided state or a component must have to violate the
    // claim: a `>= θ` claim is violated by a rejection, a `<= θ` claim by an
    // acceptance. A component holding no decided state accepts for always
    // and rejects for until, as FR-202's event-avoidance check classifies it.
    let violating = matches!(bound, Bound::AtMost(_));
    let undecided_accepts = matches!(target, ReachTarget::Always(_));
    let mut union: BTreeSet<ProbabilityProductKey> = BTreeSet::new();
    for (key, state) in &classified {
        visit(cancel, meter)?;
        if state.terminal == Some(violating) {
            charge(
                storage,
                limits.storage_bytes,
                key.as_bytes().len().saturating_mul(2),
            )?;
            union.insert(key.clone());
        }
    }
    if undecided_accepts == violating {
        for component in &components {
            for index in component {
                visit(cancel, meter)?;
                let key = keys
                    .get(*index)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                charge(
                    storage,
                    limits.storage_bytes,
                    key.as_bytes().len().saturating_mul(2),
                )?;
                union.insert((*key).clone());
            }
        }
    }
    drop(components);
    drop(keys);

    stage(Stage::Certificate)?;
    let initial = original.initial.clone();
    let mut states = BTreeMap::new();
    let mut pending = VecDeque::from([initial.clone()]);
    while let Some(key) = pending.pop_front() {
        visit(cancel, meter)?;
        if states.contains_key(&key) {
            continue;
        }
        let mut state = classified
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if union.contains(&key) {
            state.terminal = Some(true);
            state.choices.clear();
        } else if state.terminal.is_some() {
            state.terminal = Some(false);
        } else {
            for edge in state.choices.iter().flatten() {
                visit(cancel, meter)?;
                pending.push_back(edge.target.clone());
            }
        }
        charge(
            storage,
            limits.storage_bytes,
            key.as_bytes()
                .len()
                .saturating_mul(3)
                .saturating_add(std::mem::size_of::<CheckerState>()),
        )?;
        states.insert(key, state);
    }
    let mut graph = CheckerGraph { initial, states };
    let ranks = match profile {
        Profile::Dtmc => Some(analyze(&mut graph, limits, cancel, meter)?),
        Profile::Mdp => None,
    };
    Ok(Subject {
        graph,
        ranks,
        event,
        objective: objective_of(&claim.scheduler),
        bound: bound.clone(),
    })
}

/// The maximal end components of the classification graph, as sorted state
/// indexes. A decided state has no action, so it is its own absorbing
/// component and appears in none of them. Under one distribution per state
/// these are the bottom strongly connected components.
fn end_components(
    states: &BTreeMap<ProbabilityProductKey, CheckerState>,
    limits: ProbabilityDocumentLimits,
    storage: &mut u64,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Vec<Vec<usize>>, ProbabilityDocumentError> {
    let index: BTreeMap<&ProbabilityProductKey, usize> = states
        .keys()
        .enumerate()
        .map(|(position, key)| (key, position))
        .collect();
    let count = states.len();
    let edges = states
        .values()
        .flat_map(|state| state.choices.iter())
        .map(Vec::len)
        .sum::<usize>();
    let pass = count.saturating_mul(
        std::mem::size_of::<Vec<usize>>().saturating_add(8 * std::mem::size_of::<usize>()),
    );
    charge(
        storage,
        limits.storage_bytes,
        edges
            .saturating_mul(std::mem::size_of::<usize>())
            .saturating_add(pass),
    )?;
    let mut actions: Vec<Vec<Vec<usize>>> = Vec::with_capacity(count);
    for state in states.values() {
        visit(cancel, meter)?;
        let mut per_state = Vec::with_capacity(state.choices.len());
        for choice in &state.choices {
            let mut successors = Vec::with_capacity(choice.len());
            for edge in choice {
                visit(cancel, meter)?;
                if edge.probability > rational(0) {
                    successors.push(
                        *index
                            .get(&edge.target)
                            .ok_or(ProbabilityDocumentError::Malformed)?,
                    );
                }
            }
            per_state.push(successors);
        }
        actions.push(per_state);
    }
    let mut result = Vec::new();
    let mut work = vec![(0..count).collect::<Vec<usize>>()];
    while let Some(mut set) = work.pop() {
        charge(storage, limits.storage_bytes, pass.saturating_add(edges.saturating_mul(std::mem::size_of::<usize>())))?;
        let mut member = vec![false; count];
        for state in &set {
            member[*state] = true;
        }
        loop {
            let mut removed = false;
            let mut kept = Vec::with_capacity(set.len());
            for state in &set {
                visit(cancel, meter)?;
                let stays = actions[*state]
                    .iter()
                    .any(|action| !action.is_empty() && action.iter().all(|next| member[*next]));
                if stays {
                    kept.push(*state);
                } else {
                    member[*state] = false;
                    removed = true;
                }
            }
            set = kept;
            if !removed {
                break;
            }
        }
        if set.is_empty() {
            continue;
        }
        let mut adjacent = vec![Vec::new(); count];
        for state in &set {
            for action in &actions[*state] {
                visit(cancel, meter)?;
                if !action.is_empty() && action.iter().all(|next| member[*next]) {
                    adjacent[*state].extend(action.iter().copied());
                }
            }
        }
        let parts = strongly_connected(&set, &adjacent, count, cancel, meter)?;
        if parts.len() == 1 {
            result.push(set);
        } else {
            work.extend(parts);
        }
    }
    result.sort();
    Ok(result)
}

/// Iterative Tarjan decomposition of the subgraph on `set`.
fn strongly_connected(
    set: &[usize],
    adjacent: &[Vec<usize>],
    count: usize,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Vec<Vec<usize>>, ProbabilityDocumentError> {
    let mut number = vec![usize::MAX; count];
    let mut low = vec![0_usize; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut next = 0_usize;
    let mut parts = Vec::new();
    for root in set {
        if number[*root] != usize::MAX {
            continue;
        }
        number[*root] = next;
        low[*root] = next;
        next += 1;
        stack.push(*root);
        on_stack[*root] = true;
        let mut calls: Vec<(usize, usize)> = vec![(*root, 0)];
        while let Some(&(node, edge)) = calls.last() {
            visit(cancel, meter)?;
            if let Some(&successor) = adjacent[node].get(edge) {
                if let Some(top) = calls.last_mut() {
                    top.1 += 1;
                }
                if number[successor] == usize::MAX {
                    number[successor] = next;
                    low[successor] = next;
                    next += 1;
                    stack.push(successor);
                    on_stack[successor] = true;
                    calls.push((successor, 0));
                } else if on_stack[successor] {
                    low[node] = low[node].min(number[successor]);
                }
            } else {
                calls.pop();
                if let Some(&(parent, _)) = calls.last() {
                    low[parent] = low[parent].min(low[node]);
                }
                if low[node] == number[node] {
                    let mut part = Vec::new();
                    loop {
                        let member = stack.pop().ok_or(ProbabilityDocumentError::Malformed)?;
                        on_stack[member] = false;
                        part.push(member);
                        if member == node {
                            break;
                        }
                    }
                    part.sort_unstable();
                    parts.push(part);
                }
            }
        }
    }
    Ok(parts)
}

/// The inclusive bound on the violated quantity a claim bound requires:
/// `v <= 1 - θ` for `>= θ` and `v <= θ` for `<= θ`.
pub(super) fn violated_bound(
    bound: &Bound,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Bound, ProbabilityDocumentError> {
    Ok(Bound::AtMost(match bound {
        Bound::AtLeast(threshold) => complement(threshold, limits, cancel, meter)?,
        Bound::AtMost(threshold) => threshold.clone(),
    }))
}

fn complement(
    value: &Rational,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Rational, ProbabilityDocumentError> {
    let one = rational(1);
    let result = super::document::arithmetic(
        filament_exact::RationalArithmetic::Subtract(&one, value),
        meter,
        cancel,
    )?;
    super::document::check_scalar(&result, limits)?;
    Ok(result)
}

/// The claim's own quantity for a violated quantity `v`.
fn claim_value(
    bound: &Bound,
    violated: &Rational,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Rational, ProbabilityDocumentError> {
    match bound {
        Bound::AtLeast(_) => complement(violated, limits, cancel, meter),
        Bound::AtMost(_) => Ok(violated.clone()),
    }
}

/// Convert EN-5's solution of the claim into the violated quantity the
/// profile certifies, over exactly the rows of `graph`.
pub(super) fn violated_values(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    bound: &Bound,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    let mut values = Vec::new();
    for row in &certificate.values {
        visit(cancel, meter)?;
        if !graph.states.contains_key(&row.state) {
            continue;
        }
        values.push(ExactProbabilityValue {
            state: row.state.clone(),
            value: claim_value(bound, &row.value, limits, cancel, meter)?,
        });
    }
    Ok(ExactProbabilityCertificate { values })
}

/// Report an accepted violated-quantity check as the claim's own value and
/// bound side.
pub(super) fn claim_acceptance(
    check: ExactProbabilityCertificateCheck,
    bound: &Bound,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let ExactProbabilityCertificateCheck::Accepted(mut accepted) = check else {
        return Ok(check);
    };
    accepted.value = claim_value(bound, &accepted.value, limits, cancel, meter)?;
    for row in &mut accepted.certificate.values {
        visit(cancel, meter)?;
        row.value = claim_value(bound, &row.value, limits, cancel, meter)?;
    }
    accepted.side = match bound {
        Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
        Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
    };
    Ok(ExactProbabilityCertificateCheck::Accepted(accepted))
}
