// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed cyclic until and always proof profiles: the checker-owned
//! classification graph, its maximal end components and the certificate graph
//! over the violating union. Nothing here reads a producer component set,
//! classification or target set.

use super::cyclic::{
    analyze, captured_cost, charge, enumerate_graph_capturing, visit, CapturedStates,
};
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
        || *threshold <= rational(0)
        || *threshold >= rational(1)
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    // The profile follows from the subject: a workload is a DTMC subject.
    if profile == Profile::Mdp && !matches!(claim.scheduler, ProbScheduler::Every) {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
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
    // The subject selects the profile (FR-201): the DTMC profile when every
    // state has one distribution, the MDP profile otherwise. A document under
    // the other profile is an objective mismatch.
    let choosy = original
        .states
        .values()
        .any(|state| state.choices.len() > 1);
    for state in original.states.values() {
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
    // After classification, so an undefined predicate is reported first.
    if (profile == Profile::Dtmc) == choosy {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }

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

/// Bytes of the retained successor lists (`edges` indices plus one list per
/// state and action) and of the largest refinement step's vectors (membership,
/// kept, adjacency, Tarjan numbering, stacks and parts), counted once.
fn end_component_peak(count: usize, edges: usize) -> usize {
    let word = std::mem::size_of::<usize>();
    let pass = count.saturating_mul(std::mem::size_of::<Vec<usize>>().saturating_add(8 * word));
    edges
        .saturating_mul(word)
        .saturating_add(pass)
        .saturating_mul(2)
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
    // The peak, charged once: the cumulative counter would otherwise count
    // the per-item vectors again for every refinement, though each is freed
    // before the next is built.
    charge(
        storage,
        limits.storage_bytes,
        end_component_peak(count, edges),
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
    // Every vector below is sized to the work item's set, so a split into many
    // components costs time proportional to the sets, which `visit` meters.
    while let Some(mut set) = work.pop() {
        let mut member = vec![true; set.len()];
        let local = |set: &[usize], state: usize| set.binary_search(&state).ok();
        loop {
            let mut removed = false;
            let mut kept = Vec::with_capacity(set.len());
            for (at, state) in set.iter().enumerate() {
                visit(cancel, meter)?;
                let stays = member[at]
                    && actions[*state].iter().any(|action| {
                        !action.is_empty()
                            && action
                                .iter()
                                .all(|next| local(&set, *next).is_some_and(|l| member[l]))
                    });
                if stays {
                    kept.push(*state);
                } else {
                    member[at] = false;
                    removed = true;
                }
            }
            if !removed {
                break;
            }
            set = kept;
            member = vec![true; set.len()];
        }
        if set.is_empty() {
            continue;
        }
        let mut adjacent = vec![Vec::new(); set.len()];
        for (at, state) in set.iter().enumerate() {
            for action in &actions[*state] {
                visit(cancel, meter)?;
                if action.is_empty() {
                    continue;
                }
                let targets: Option<Vec<usize>> =
                    action.iter().map(|next| local(&set, *next)).collect();
                if let Some(targets) = targets {
                    adjacent[at].extend(targets);
                }
            }
        }
        let roots: Vec<usize> = (0..set.len()).collect();
        let parts = strongly_connected(&roots, &adjacent, set.len(), cancel, meter)?;
        if parts.len() == 1 {
            result.push(set);
        } else {
            work.extend(parts.into_iter().map(|part| {
                let mut part: Vec<usize> = part.into_iter().map(|at| set[at]).collect();
                part.sort_unstable();
                part
            }));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(index: usize) -> ProbabilityProductKey {
        ProbabilityProductKey::from_canonical_bytes(index.to_be_bytes().to_vec())
    }

    /// A line of states, each looping on itself and moving to the next: every
    /// state is its own component, so refinement splits the whole set into
    /// `count` work items.
    fn line(count: usize) -> BTreeMap<ProbabilityProductKey, CheckerState> {
        let edge = |target: usize| ProbabilityEdge {
            target: key(target),
            probability: rational(1),
        };
        (0..count)
            .map(|index| {
                let choice = if index + 1 < count {
                    vec![edge(index), edge(index + 1)]
                } else {
                    vec![edge(index)]
                };
                (
                    key(index),
                    CheckerState {
                        terminal: None,
                        choices: vec![choice],
                    },
                )
            })
            .collect()
    }

    fn limits_with(storage_bytes: u64) -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes,
            states: 1024,
            edges: 4096,
            scalar_bits: 128,
            accounting: crate::AccountingLimits::default().0,
        }
    }

    fn run(
        states: &BTreeMap<ProbabilityProductKey, CheckerState>,
        storage_bytes: u64,
    ) -> Result<Vec<Vec<usize>>, ProbabilityDocumentError> {
        run_with_work(states, storage_bytes, None)
    }

    fn run_with_work(
        states: &BTreeMap<ProbabilityProductKey, CheckerState>,
        storage_bytes: u64,
        work_units: Option<u64>,
    ) -> Result<Vec<Vec<usize>>, ProbabilityDocumentError> {
        let mut bound = limits_with(storage_bytes);
        if let Some(work_units) = work_units {
            bound.accounting.work_units = work_units;
        }
        let cancel = Cancel::new();
        let mut meter = filament_exact::Meter::new(bound.accounting).with_cancel(cancel.clone());
        let mut storage = 0;
        end_components(states, bound, &mut storage, &cancel, &mut meter)
    }

    /// FR-201-AC-63: storage is charged at its peak once, so a subject with
    /// many component splits is admitted at exactly the computed amount and
    /// refused with the storage cause one byte below it.
    #[test]
    fn end_component_storage_is_charged_at_its_computed_peak_once() {
        let count = 64;
        let states = line(count);
        let edges = 2 * count - 1;
        let exact = end_component_peak(count, edges) as u64;
        // Refinement splits the whole set into `count` singletons; only the last
        // one (whose action stays inside it) is an end component.
        assert_eq!(run(&states, exact).unwrap(), vec![vec![count - 1]]);
        assert!(matches!(
            run(&states, exact - 1),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::StorageBytes,
                bound,
                ..
            }) if bound == exact - 1
        ));
    }

    /// FR-201-AC-63: the work budget bounds end-component decomposition. A
    /// line of self-looping states splits into one work item per state; the
    /// per-item cost follows the item's set, so the total stays linear in the
    /// state count instead of quadratic.
    #[test]
    fn end_component_work_is_linear_in_the_states_and_bounded_by_the_work_ceiling() {
        let count = 512;
        let states = line(count);
        let storage = end_component_peak(count, 2 * count - 1) as u64;
        let (mut low, mut high) = (0_u64, 1 << 24);
        while low < high {
            let middle = low + (high - low) / 2;
            if run_with_work(&states, storage, Some(middle)).is_ok() {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let least = low;
        assert_eq!(
            run_with_work(&states, storage, Some(least)).unwrap(),
            vec![vec![count - 1]]
        );
        assert!(matches!(
            run_with_work(&states, storage, Some(least - 1)),
            Err(ProbabilityDocumentError::Accounting(_))
        ));
        // Count-sized vectors per item would cost count * count / 2 visits.
        assert!(
            least < 16 * count as u64,
            "least work ceiling {least} is not linear in {count} states"
        );
    }

    #[test]
    fn a_captured_model_state_is_charged_its_key_twice_and_its_clone() {
        assert_eq!(
            captured_cost(10),
            20 + std::mem::size_of::<fsl_eval::simulation::model::ModelSimState>()
        );
    }
}
