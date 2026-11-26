// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-411 "Evidence on a partial product": where `max_states` stops the
//! whole graph, a `lower` document is built and checked over the states it
//! names and their successors.

use super::*;
use ix_trace_rs::implements;

/// The `cyclic_dtmc` flag of the base a partial product's `lower` document
/// takes for `claim`: the cyclic chain for an `eventually`, the acyclic
/// profile for an expected reward. No other form has one.
pub(super) fn cyclic_dtmc(claim: &CheckedProbabilisticClaim) -> Option<bool> {
    match claim.form {
        ProbForm::Reach {
            target: ReachTarget::Eventually(_),
            ..
        } => Some(true),
        ProbForm::ExpectedReward { .. } => Some(false),
        _ => None,
    }
}

/// The graph of the states `rows` name and their successors, where
/// `exhausted` stopped the whole graph. `exhausted` stands wherever that part
/// cannot show the document: more states named than `max_states` admits, a
/// base with no partial form, or a named or listed target key the part does
/// not reach.
#[allow(clippy::too_many_arguments)]
pub(super) fn explored_subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    cyclic_dtmc: bool,
    rows: &[ExactProbabilityValue],
    targets: &[ProbabilityProductKey],
    exhausted: ProbabilityDocumentError,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    control: &super::super::reward::RewardControl<'_>,
) -> Result<SubjectGraph, ProbabilityDocumentError> {
    if rows.len() > limits.states {
        return Err(exhausted);
    }
    let mut named = BTreeSet::new();
    for row in rows {
        poll(cancel)?;
        super::super::cyclic::charge(
            storage,
            limits.storage_bytes,
            row.state
                .as_bytes()
                .len()
                .saturating_add(std::mem::size_of::<ProbabilityProductKey>()),
        )?;
        named.insert(row.state.clone());
    }
    let subject = match subject_graph_with_identity(
        system,
        claims,
        item,
        limits,
        cancel,
        fsl_foundation::IdentityLimits::default(),
        cyclic_dtmc,
        false,
        Some(&named),
        meter,
        storage,
        control,
    ) {
        Err(
            ProbabilityDocumentError::Unsupported
            | ProbabilityDocumentError::Graph(ExactProbabilityGraphError::Cycle),
        ) => return Err(exhausted),
        other => other?,
    };
    let graph = &subject.0;
    if named
        .iter()
        .chain(targets)
        .any(|key| !graph.states.contains_key(key))
    {
        return Err(exhausted);
    }
    Ok(subject)
}

/// [`probability_proof_document`], ranking the states `certificate` names
/// alone, or, where `max_states` stops the whole graph, the document of a
/// `lower` proof over the states `certificate` names and their successors
/// (FR-1042 "Document").Over that part an `eventually` takes the cyclic chain base,
/// ranking each named state by its distance to a named target through named
/// states of positive value; an expected reward takes the acyclic profile,
/// ranked by height in that part. This does not grant acceptance.
#[implements("FR-1042-AC-25", "FR-1042-AC-26")]
pub fn partial_lower_probability_proof_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    // Both attempts share one arithmetic and retained-storage scope.
    let (whole, mut meter, mut storage) = super::mdp::probability_proof_document_spending(
        system,
        claims,
        item,
        certificate,
        limits,
        cancel,
    );
    let exhausted = match whole {
        Err(
            error @ ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                ..
            },
        ) => error,
        // The whole document fits where the explored model did not: its graph
        // stops at the targets, and so do its values.
        Ok(mut proof) => {
            proof.rank_named_rows();
            return Ok(proof);
        }
        Err(error) => return Err(error),
    };
    let Some(cyclic_dtmc) = cyclic_dtmc(bind_claim(claims, item, cancel)?) else {
        return Err(exhausted);
    };
    let (graph, optimization, _, heights, _, _, reward, _) = explored_subject(
        system,
        claims,
        item,
        limits,
        cancel,
        cyclic_dtmc,
        &certificate.values,
        &[],
        exhausted,
        &mut meter,
        &mut storage,
        &super::super::reward::RewardControl::new(cancel),
    )?;
    let ranks = if cyclic_dtmc {
        progress_ranks(&graph, &certificate.values, cancel, &mut meter)?
    } else {
        let heights = heights.ok_or(ProbabilityDocumentError::Malformed)?;
        certificate
            .values
            .iter()
            .map(|row| {
                heights
                    .get(&row.state)
                    .map(|height| (row.state.clone(), *height))
                    .ok_or(ProbabilityDocumentError::Malformed)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?
    };
    Ok(ProbabilityProofDocument::unfair(
        ProbabilityCertificateDocument {
            cyclic_dtmc,
            identity: item.clone(),
            targets: target_keys(&graph),
            event: None,
            failures: Vec::new(),
            reward: reward.map(|reward| reward.objective),
            optimization,
            timed: None,
            values: certificate.values.clone(),
            ranking: ranks.into_iter().collect(),
            policy: None,
        },
    ))
}

/// Each named state's distance to a named target through named states of
/// positive value, and 0 where none leads to one. A `lower` shows progress
/// through those alone, since an unnamed successor counts as 0.
fn progress_ranks(
    graph: &CheckerGraph,
    rows: &[ExactProbabilityValue],
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let mut reverse = BTreeMap::<&ProbabilityProductKey, Vec<&ProbabilityProductKey>>::new();
    let mut distances = BTreeMap::<&ProbabilityProductKey, u64>::new();
    let mut pending = VecDeque::new();
    for row in rows.iter().filter(|row| row.value > rational(0)) {
        let state = graph
            .states
            .get(&row.state)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.terminal == Some(true) {
            distances.insert(&row.state, 0);
            pending.push_back((&row.state, 0_u64));
        }
        for edge in state.choices.iter().flatten() {
            super::super::reward::visit(cancel, meter)?;
            reverse.entry(&edge.target).or_default().push(&row.state);
        }
    }
    while let Some((key, distance)) = pending.pop_front() {
        let next = distance.saturating_add(1);
        for previous in reverse.get(key).into_iter().flatten() {
            super::super::reward::visit(cancel, meter)?;
            if !distances.contains_key(previous) {
                distances.insert(previous, next);
                pending.push_back((previous, next));
            }
        }
    }
    Ok(rows
        .iter()
        .map(|row| {
            let distance = distances.get(&row.state).copied().unwrap_or(0);
            (row.state.clone(), distance)
        })
        .collect())
}
