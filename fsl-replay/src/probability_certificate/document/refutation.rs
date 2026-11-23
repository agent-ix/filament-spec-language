// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202's counterexample role for untimed unwindowed `eventually` and
//! expected-reward claims: a `lower` or `upper` subsystem certificate in the
//! value-bounds shape, checked over the chain the workload or the witness
//! scheduler induces, then compared strictly against the claim's bound.
//!
//! The base objective is the claim's own exact cyclic profile objective,
//! derived here from the checked subject on both the writing and the checking
//! side. An event-avoidance `lower` certificate (FSpec FR-412) bounds the
//! probability of reaching the event-free bottom components of the induced
//! chain, which this checker derives itself; no producer component list or
//! qualitative set is read.

use super::*;

/// The side of the violating quantity a subsystem certificate bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubsystemBoundKind {
    /// `x <= F(x)`: a lower bound on the violating quantity.
    Lower,
    /// `F(y) <= y` with a properness ranking: an upper bound on the value.
    Upper,
}

impl SubsystemBoundKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBoundObjective {
    profile: String,
    base: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBoundDocument {
    identity: WireIdentity,
    objective: WireBoundObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Option<Vec<WireRank>>,
    policy: Option<Box<serde_json::value::RawValue>>,
}

struct Decoded {
    identity: ObligationIdentity,
    base: Value,
    kind: SubsystemBoundKind,
    values: BTreeMap<ProbabilityProductKey, Rational>,
    ranking: Option<BTreeMap<ProbabilityProductKey, u64>>,
}

struct Subject {
    graph: CheckerGraph,
    reward: Option<super::super::reward::RewardGraph>,
    bound: Bound,
    workload: bool,
    identity: Value,
    base: Value,
}

/// Whether value-bounds bytes wrap a base this role owns rather than FR-1043's
/// finite-horizon measure base.
pub(in crate::probability_certificate) fn owns(
    bytes: &[u8],
) -> Result<bool, ProbabilityDocumentError> {
    #[derive(Deserialize)]
    struct Base<'a> {
        #[serde(borrow)]
        profile: &'a str,
    }
    #[derive(Deserialize)]
    struct Objective<'a> {
        #[serde(borrow)]
        base: Base<'a>,
    }
    #[derive(Deserialize)]
    struct Dispatch<'a> {
        #[serde(borrow)]
        objective: Objective<'a>,
    }
    let dispatch: Dispatch<'_> =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    Ok(dispatch.objective.base.profile != super::measure::PROFILE)
}

/// Admit canonical counterexample-role bytes without checking them.
pub(in crate::probability_certificate) fn admit(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    decode(bytes, limits, cancel, &mut meter).map(|_| ())
}

fn decode(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Decoded, ProbabilityDocumentError> {
    poll(cancel)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            size,
        ));
    }
    let parsed = quire_canonical::read_bounded(bytes, limits.bytes, limits.storage_bytes).map_err(
        |error| match error {
            quire_canonical::ReadError::Limit(limit) => exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limit.bound,
                limit.required,
            ),
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    if quire_canonical::to_vec(&parsed, quire_canonical::Limits::new(limits.bytes))
        .map_err(|_| ProbabilityDocumentError::Malformed)?
        != bytes
    {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    drop(parsed);
    let wire: WireBoundDocument =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if wire.objective.profile != super::measure::bounds::PROFILE {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let kind = match wire.kind.as_str() {
        "lower" => SubsystemBoundKind::Lower,
        "upper" => SubsystemBoundKind::Upper,
        _ => return Err(ProbabilityDocumentError::MalformedMember("kind")),
    };
    if wire.policy.is_some() {
        // The counterexample's own scheduler is the only policy this role reads.
        return Err(ProbabilityDocumentError::MalformedMember("policy"));
    }
    let rows = wire
        .values
        .len()
        .max(wire.ranking.as_ref().map_or(0, Vec::len));
    if rows > limits.states {
        return Err(exceeded(
            ProbabilityDocumentLimit::States,
            u64::try_from(limits.states).unwrap_or(u64::MAX),
            u64::try_from(rows).unwrap_or(u64::MAX),
        ));
    }
    let node = fsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let identity = ObligationIdentity::new(
        node,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let mut values = BTreeMap::new();
    let mut previous: Option<ProbabilityProductKey> = None;
    for row in wire.values {
        poll(cancel)?;
        let state = decode_key(&row.state, limits)?;
        if previous.as_ref().is_some_and(|previous| previous >= &state) {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
        previous = Some(state.clone());
        values.insert(
            state,
            reward_ratio(row.value, limits.scalar_bits, true, meter, cancel)?,
        );
    }
    let ranking = match wire.ranking {
        None => None,
        Some(rows) => {
            let mut ranking = BTreeMap::new();
            let mut previous: Option<ProbabilityProductKey> = None;
            for row in rows {
                poll(cancel)?;
                if row.rank > MAX_COUNT {
                    return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                }
                let state = decode_key(&row.state, limits)?;
                if previous.as_ref().is_some_and(|previous| previous >= &state) {
                    return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                }
                previous = Some(state.clone());
                ranking.insert(state, row.rank);
            }
            Some(ranking)
        }
    };
    Ok(Decoded {
        identity,
        base: wire.objective.base,
        kind,
        values,
        ranking,
    })
}

fn subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Subject, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    if !claim.fairness.is_empty()
        || fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    let workload = claim_workload(system, claim)?;
    let identity = fsl_foundation::IdentityLimits::default();
    let control = super::super::reward::RewardControl::new(cancel);
    let (bound, graph, reward) = match &claim.form {
        ProbForm::ExpectedReward {
            bound,
            reward: name,
            until,
        } => {
            let (graph, _, reward) = super::super::reward::enumerate(
                system,
                claim,
                name,
                until,
                reference,
                workload,
                limits,
                cancel,
                identity,
                if workload.is_some() {
                    super::super::reward::RewardProfile::CyclicDtmc
                } else {
                    super::super::reward::RewardProfile::CyclicMdp
                },
                meter,
                storage,
                &control,
            )?;
            (bound, graph, Some(reward))
        }
        // Event avoidance refutes only the `>=` side: `Pr(not eventually E) > 1 - θ`.
        ProbForm::Reach {
            bound: bound @ Bound::AtLeast(_),
            target: ReachTarget::Eventually(predicate),
        } => (
            bound,
            super::super::cyclic::enumerate_graph(
                system,
                Some(predicate),
                reference,
                workload,
                limits,
                cancel,
                identity,
                meter,
                storage,
                true,
                true,
            )?,
            None,
        ),
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    let base = ProbabilityCertificateDocument {
        cyclic_dtmc: workload.is_some(),
        identity: item.clone(),
        targets: target_keys(&graph),
        event: None,
        failures: Vec::new(),
        reward: reward.as_ref().map(|reward| reward.objective.clone()),
        optimization: claim_objective(&claim.scheduler, bound),
        timed: None,
        values: Vec::new(),
        ranking: Vec::new(),
    };
    let written = writer::encode_fair_profile(&base, limits, cancel, workload.is_none(), &[])?;
    let mut written: Value =
        serde_json::from_slice(&written).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let identity = written
        .get_mut("identity")
        .map(Value::take)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let base = written
        .get_mut("objective")
        .map(Value::take)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    Ok(Subject {
        graph,
        reward,
        bound: bound.clone(),
        workload: workload.is_some(),
        identity,
        base,
    })
}

/// Write a counterexample-role certificate whose base objective is derived
/// from the checked subject. This grants no acceptance.
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn write(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    kind: SubsystemBoundKind,
    values: &[ExactProbabilityValue],
    ranking: Option<&[(ProbabilityProductKey, u64)]>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let subject = subject(
        system,
        claims,
        item,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    let mut rows = Vec::new();
    for row in values {
        poll(cancel)?;
        rows.push(json!({
            "state": URL_SAFE_NO_PAD.encode(row.state.as_bytes()),
            "value": {
                "numerator": row.value.numerator().to_string(),
                "denominator": row.value.denominator().to_string(),
            },
        }));
    }
    let mut document = serde_json::Map::new();
    document.insert("identity".into(), subject.identity);
    document.insert(
        "objective".into(),
        json!({"profile": super::measure::bounds::PROFILE, "base": subject.base}),
    );
    document.insert("kind".into(), Value::from(kind.as_str()));
    document.insert("values".into(), Value::Array(rows));
    if let Some(ranking) = ranking {
        let mut ranks = Vec::new();
        for (state, rank) in ranking {
            poll(cancel)?;
            ranks.push(json!({"state": URL_SAFE_NO_PAD.encode(state.as_bytes()), "rank": rank}));
        }
        document.insert("ranking".into(), Value::Array(ranks));
    }
    let bytes = canonical(&Value::Object(document), limits.bytes)?;
    decode(&bytes, limits, cancel, &mut meter)?;
    Ok(bytes)
}

/// The outcome of checking a certificate in its counterexample role.
#[derive(Clone, Debug)]
pub(in crate::probability_certificate) enum SubsystemRefutationCheck {
    /// Every condition and the strict refutation comparison hold.
    Accepted,
    /// A certificate condition or the strict comparison failed.
    Rejected(ProbabilityCertificateRejection),
    /// The scheduler is missing, extra or names no choice of the rebuilt chain.
    SchedulerRefused,
}

/// Rebuild the workload- or scheduler-induced chain of an untimed unwindowed
/// `eventually` or expected-reward claim and check a counterexample-role
/// certificate over it. `scheduler` maps each scheduled complete key to the
/// index of its chosen action in the subject's enabled-action order; it is
/// empty under a workload.
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &BTreeMap<ProbabilityProductKey, usize>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<SubsystemRefutationCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let decoded = decode(bytes, limits, cancel, &mut meter)?;
    if &decoded.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let subject = subject(
        system,
        claims,
        item,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    if decoded.base != subject.base {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if subject.workload && !scheduler.is_empty() {
        return Ok(SubsystemRefutationCheck::SchedulerRefused);
    }
    let Some(induced) = induce(&subject, scheduler, &mut meter, cancel)? else {
        return Ok(SubsystemRefutationCheck::SchedulerRefused);
    };
    let reject = |rule, at: &ProbabilityProductKey| {
        Ok(SubsystemRefutationCheck::Rejected(
            ProbabilityCertificateRejection {
                rule,
                at: ProbabilityCertificateLocus::ProductState(at.clone()),
            },
        ))
    };
    let malformed = |member| {
        Ok(SubsystemRefutationCheck::Rejected(
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member(member),
            },
        ))
    };
    let expected = match (&subject.reward, &subject.bound) {
        (Some(_), Bound::AtMost(_)) | (None, _) => SubsystemBoundKind::Lower,
        (Some(_), Bound::AtLeast(_)) => SubsystemBoundKind::Upper,
    };
    if decoded.kind != expected {
        return malformed("kind");
    }
    let ranked = !(subject.reward.is_some() && expected == SubsystemBoundKind::Lower);
    if decoded.ranking.is_some() != ranked {
        return malformed("ranking");
    }
    if let Some(ranking) = &decoded.ranking {
        if ranking.keys().any(|key| !decoded.values.contains_key(key)) {
            return malformed("ranking");
        }
    }
    let graph = &subject.graph;
    if !decoded.values.contains_key(&graph.initial)
        || decoded.values.keys().any(|key| !induced.contains_key(key))
        || expected == SubsystemBoundKind::Upper && decoded.values.len() != induced.len()
        || subject.reward.is_none() && decoded.values.values().any(|value| *value > rational(1))
    {
        return malformed("values");
    }
    let mut full = BTreeMap::new();
    for key in induced.keys() {
        super::super::reward::visit(cancel, &mut meter)?;
        full.insert(
            key.clone(),
            decoded
                .values
                .get(key)
                .cloned()
                .unwrap_or_else(|| rational(0)),
        );
    }
    let ranks = decoded.ranking.as_ref();
    let control = super::super::reward::RewardControl::new(cancel);
    let avoided = if subject.reward.is_none() {
        Some(event_free_bottom(graph, &induced, &mut meter, cancel)?)
    } else {
        None
    };
    for (key, value) in &decoded.values {
        super::super::reward::visit(cancel, &mut meter)?;
        let state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let choice = induced.get(key).copied().flatten();
        if state.terminal == Some(true) {
            // The event or the reward target: its fixed value is 0 for the
            // avoided-event probability and for the remaining reward.
            if expected == SubsystemBoundKind::Lower && *value != rational(0) {
                return reject(ProbabilityCertificateRule::InequalityFails, key);
            }
            continue;
        }
        if avoided
            .as_ref()
            .is_some_and(|avoided| avoided.contains(key))
        {
            // An event-free bottom state is an auxiliary target of value 1.
            continue;
        }
        let Some(index) = choice else {
            // Terminal stutter: the reward accrues nothing and the state
            // never reaches the target, so only an upper bound is improper.
            if expected == SubsystemBoundKind::Upper {
                return reject(ProbabilityCertificateRule::RankBroken, key);
            }
            continue;
        };
        let edges = state
            .choices
            .get(index)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let sum = action_sum(
            key,
            index,
            edges,
            &full,
            subject.reward.as_ref(),
            limits,
            cancel,
            &mut meter,
            &control,
        )?;
        let holds = match expected {
            SubsystemBoundKind::Lower => *value <= sum,
            SubsystemBoundKind::Upper => sum <= *value,
        };
        if !holds {
            return reject(ProbabilityCertificateRule::InequalityFails, key);
        }
        if !ranked || expected == SubsystemBoundKind::Lower && *value == rational(0) {
            continue;
        }
        let Some(rank) = ranks.and_then(|ranks| ranks.get(key)) else {
            return reject(ProbabilityCertificateRule::RankMissing, key);
        };
        let mut progress = false;
        for edge in edges {
            super::super::reward::visit(cancel, &mut meter)?;
            let positive = expected == SubsystemBoundKind::Upper
                || full
                    .get(&edge.target)
                    .is_some_and(|next| *next > rational(0));
            if edge.probability > rational(0)
                && positive
                && ranks
                    .and_then(|ranks| ranks.get(&edge.target))
                    .is_some_and(|next| next < rank)
            {
                progress = true;
                break;
            }
        }
        if !progress {
            return reject(ProbabilityCertificateRule::RankBroken, key);
        }
    }
    let initial = full
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let threshold = match &subject.bound {
        Bound::AtLeast(threshold) | Bound::AtMost(threshold) => threshold,
    };
    check_scalar(threshold, limits)?;
    let refutes = match (&subject.reward, &subject.bound) {
        (None, Bound::AtLeast(threshold)) => *initial > rational(1).sub(threshold),
        (Some(_), Bound::AtMost(threshold)) => initial > threshold,
        (Some(_), Bound::AtLeast(threshold)) => initial < threshold,
        (None, Bound::AtMost(_)) => return Err(ProbabilityDocumentError::Unsupported),
    };
    if !refutes {
        return reject(ProbabilityCertificateRule::ThresholdMissed, &graph.initial);
    }
    poll(cancel)?;
    Ok(SubsystemRefutationCheck::Accepted)
}

/// The states the chain reaches from the initial state, each with its chosen
/// action, stopping at targets and terminal states. `None` when the scheduler
/// misses a reached choice state or names a state the chain does not reach.
fn induce(
    subject: &Subject,
    scheduler: &BTreeMap<ProbabilityProductKey, usize>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<BTreeMap<ProbabilityProductKey, Option<usize>>>, ProbabilityDocumentError> {
    let graph = &subject.graph;
    let mut induced = BTreeMap::new();
    let mut pending = VecDeque::from([graph.initial.clone()]);
    induced.insert(graph.initial.clone(), None);
    while let Some(key) = pending.pop_front() {
        super::super::reward::visit(cancel, meter)?;
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.choices.is_empty() {
            continue;
        }
        let index = if subject.workload {
            if state.choices.len() != 1 {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            0
        } else {
            match scheduler.get(&key) {
                Some(index) if *index < state.choices.len() => *index,
                _ => return Ok(None),
            }
        };
        induced.insert(key.clone(), Some(index));
        for edge in &state.choices[index] {
            super::super::reward::visit(cancel, meter)?;
            if edge.probability > rational(0) && !induced.contains_key(&edge.target) {
                induced.insert(edge.target.clone(), None);
                pending.push_back(edge.target.clone());
            }
        }
    }
    if scheduler
        .keys()
        .any(|key| !matches!(induced.get(key), Some(Some(_))))
    {
        return Ok(None);
    }
    Ok(Some(induced))
}

/// The union of the induced chain's bottom strongly connected components that
/// contain no event state. A stopped event state and a terminal state stutter.
fn event_free_bottom(
    graph: &CheckerGraph,
    induced: &BTreeMap<ProbabilityProductKey, Option<usize>>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<BTreeSet<ProbabilityProductKey>, ProbabilityDocumentError> {
    let keys = induced.keys().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    for (index, key) in keys.iter().enumerate() {
        super::super::reward::visit(cancel, meter)?;
        let state = graph
            .states
            .get(*key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        match induced.get(*key).copied().flatten() {
            None => {
                edges[index].push(index);
                reverse[index].push(index);
            }
            Some(choice) => {
                for edge in state
                    .choices
                    .get(choice)
                    .ok_or(ProbabilityDocumentError::Malformed)?
                {
                    super::super::reward::visit(cancel, meter)?;
                    if edge.probability > rational(0) {
                        let next = *positions
                            .get(&edge.target)
                            .ok_or(ProbabilityDocumentError::Malformed)?;
                        edges[index].push(next);
                        reverse[next].push(index);
                    }
                }
            }
        }
    }
    // Iterative Kosaraju over the checker's own induced edges.
    let mut seen = vec![false; keys.len()];
    let mut order = Vec::new();
    for start in 0..keys.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((state, offset)) = stack.last_mut() {
            super::super::reward::visit(cancel, meter)?;
            if let Some(&next) = edges[*state].get(*offset) {
                *offset += 1;
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                let (state, _) = stack.pop().ok_or(ProbabilityDocumentError::Malformed)?;
                order.push(state);
            }
        }
    }
    let mut component_of = vec![usize::MAX; keys.len()];
    let mut components = Vec::new();
    for start in order.into_iter().rev() {
        if component_of[start] != usize::MAX {
            continue;
        }
        let index = components.len();
        let mut component = Vec::new();
        let mut stack = vec![start];
        component_of[start] = index;
        while let Some(state) = stack.pop() {
            super::super::reward::visit(cancel, meter)?;
            component.push(state);
            for &next in &reverse[state] {
                if component_of[next] == usize::MAX {
                    component_of[next] = index;
                    stack.push(next);
                }
            }
        }
        components.push(component);
    }
    let mut bottom = vec![true; components.len()];
    for (state, successors) in edges.iter().enumerate() {
        for &next in successors {
            if component_of[state] != component_of[next] {
                bottom[component_of[state]] = false;
            }
        }
    }
    let mut avoided = BTreeSet::new();
    for (index, component) in components.iter().enumerate() {
        super::super::reward::visit(cancel, meter)?;
        let event = component.iter().any(|state| {
            graph
                .states
                .get(keys[*state])
                .is_some_and(|state| state.terminal == Some(true))
        });
        if bottom[index] && !event {
            avoided.extend(component.iter().map(|state| keys[*state].clone()));
        }
    }
    Ok(avoided)
}

/// Admit any subsystem certificate a probabilistic counterexample may carry:
/// long-run, this role's value bounds, or FR-1043's mean evidence.
pub(in crate::probability_certificate) fn admit_subsystem(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    if profile_dispatch(bytes, limits, cancel)? == ProfileDispatch::LongRun {
        return super::long_run::admit(bytes, limits, cancel);
    }
    if super::measure::bounds::is_bound(bytes)? && owns(bytes)? {
        return admit(bytes, limits, cancel);
    }
    super::measure::bounds::admit_subsystem(bytes, limits, cancel)
}
