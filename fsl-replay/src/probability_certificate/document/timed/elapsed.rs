// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-877's independent check of an elapsed-quantile document: the checker
//! rebuilds the elapsed clock, activation, completion, censoring and the
//! quantile transform's terminal reward itself (FR-921 "Re-derived by the
//! checker", item 6) and reads nothing of them from the document.

use super::*;

/// The checked claim's elapsed-quantile query.
pub(super) struct Query<'a> {
    pub(super) q: Rational,
    pub(super) bound: Bound,
    pub(super) from: &'a fsl_semantics::check::CheckedPredicate,
    pub(super) until: &'a fsl_semantics::check::CheckedPredicate,
    pub(super) within: Rational,
}

pub(super) fn query(claim: &CheckedProbabilisticClaim) -> Option<Query<'_>> {
    let fsl_semantics::check::ProbForm::Quantile {
        q,
        measure:
            fsl_semantics::check::BoundedMeasure::Elapsed {
                from,
                until,
                within,
                ..
            },
        threshold,
    } = &claim.form
    else {
        return None;
    };
    Some(Query {
        q: q.clone(),
        bound: threshold.clone(),
        from,
        until,
        within: within.clone(),
    })
}

/// The threshold `c` of the claim's comparison.
pub(super) fn threshold(bound: &Bound) -> &Rational {
    match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check(
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
    let Some(query) = query(claim) else {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    };
    // FR-877: `<= c` is decided by the transform's minimum on the digital
    // route; `>= c` takes the region route (its `M < c` is strict).
    let Bound::AtMost(threshold) = &query.bound else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if route != TimedProbabilityRoute::Digital
        || !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
        || !claim.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if document.optimization != ProbabilityObjective::Minimum
        || !document.ranking.is_empty()
        || document.cyclic_dtmc
        || document.event.is_some()
        || !document.failures.is_empty()
        || document.reward.is_some()
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let window = time_units(&query.within, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    let threshold_units =
        time_units(threshold, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let watch = Watch::Elapsed(ElapsedWatch {
        from: query.from,
        until: query.until,
        q: query.q.clone(),
        threshold: threshold_units,
        strict: false,
    });
    let mut terminals = BTreeMap::new();
    let graph = enumerate_digital_deadline(
        system,
        timed,
        binding.as_ref(),
        &watch,
        window,
        scale,
        limits,
        cancel,
        meter,
        storage,
        &mut terminals,
    )?;
    let mut targets = terminals
        .iter()
        .filter(|(_, (_, within))| *within)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    targets.sort();
    if targets != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let rewards = terminals
        .into_iter()
        .map(|(key, (reward, _))| (key, reward))
        .collect();
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let mut checked = super::super::super::check_exact_terminal_rewards(
        &graph,
        &rewards,
        &certificate,
        ProbabilityObjective::Minimum,
        &Bound::AtLeast(zero()),
        cancel,
    )
    .map_err(ProbabilityDocumentError::Graph)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut checked {
        accepted.document = Some(bytes.into());
    }
    Ok(checked)
}
