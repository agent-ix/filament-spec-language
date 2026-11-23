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
    // route; `>= c` by its maximum on the region route (its `M < c` is
    // strict), certified by an `upper` document.
    let (at_least, expected_route, optimization) = match &query.bound {
        Bound::AtMost(_) => (
            false,
            TimedProbabilityRoute::Digital,
            ProbabilityObjective::Minimum,
        ),
        Bound::AtLeast(_) => (
            true,
            TimedProbabilityRoute::Region,
            ProbabilityObjective::Maximum,
        ),
    };
    if route != expected_route
        || !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
        || !claim.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if document.optimization != optimization
        || !document.ranking.is_empty()
        || document.cyclic_dtmc
        || document.event.is_some()
        || !document.failures.is_empty()
        || document.reward.is_some()
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let (graph, terminals, activated) = enumerate(
        system, claim, &query, timed, scale, limits, cancel, meter, storage,
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
        optimization,
        &if at_least {
            Bound::AtMost(zero())
        } else {
            Bound::AtLeast(zero())
        },
        if at_least {
            super::super::super::RewardValues::StrictUpper
        } else {
            super::super::super::RewardValues::Exact
        },
        if at_least {
            ProbabilityBoundSide::AtLeast
        } else {
            ProbabilityBoundSide::AtMost
        },
        &activated,
        cancel,
    )
    .map_err(ProbabilityDocumentError::Graph)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut checked {
        accepted.document = Some(bytes.into());
    }
    Ok(checked)
}

/// The checker's own graph of the elapsed-quantile product, the decided
/// measures' rewards (`(reward, within)` per terminal key) and the keys of
/// every activated state. `<= c` is enumerated over the digital route and
/// `>= c` over the region route, whose `M < c` is strict.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn enumerate(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    query: &Query<'_>,
    timed: &TimedSystem<'_, '_>,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<
    (
        CheckerGraph,
        BTreeMap<ProbabilityProductKey, (Rational, bool)>,
        BTreeSet<ProbabilityProductKey>,
    ),
    ProbabilityDocumentError,
> {
    let at_least = matches!(query.bound, Bound::AtLeast(_));
    let window = time_units(&query.within, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    let threshold_units =
        time_units(threshold(&query.bound), scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let watch = Watch::Elapsed(ElapsedWatch {
        from: query.from,
        until: query.until,
        q: query.q.clone(),
        threshold: threshold_units,
        strict: at_least,
    });
    let mut terminals = BTreeMap::new();
    let mut activated = BTreeSet::new();
    let graph = if at_least {
        enumerate_region_deadline(
            system,
            timed,
            claim,
            binding.as_ref(),
            &watch,
            window,
            scale,
            scale,
            limits,
            cancel,
            meter,
            storage,
            &mut terminals,
            &mut activated,
        )?
    } else {
        enumerate_digital_deadline(
            system,
            timed,
            claim,
            binding.as_ref(),
            &watch,
            window,
            scale,
            limits,
            cancel,
            meter,
            storage,
            &mut terminals,
        )?
    };
    Ok((graph, terminals, activated))
}

/// FR-877: replay a path-set refutation of an elapsed-quantile claim.
///
/// The checker enumerates the elapsed product itself, exactly as it does to
/// check a certificate, and reads the witness as paths of product-state keys
/// from the initial state to a decided measure the refutation counts: `M > c`
/// (or censored) for `<= c`, `M < c` for `>= c`. The witness's own action
/// spellings are not trusted; the paths select one choice per state, and the
/// counted mass and the mass of every decided measure are recomputed under it.
/// `<= c` is refuted when the counted mass exceeds `(1 - q)` of the decided
/// mass (the transform's minimum is negative); `>= c` when it is positive and
/// reaches `q` of the decided mass (the transform's maximum is not negative).
pub(super) fn replay(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    query: &Query<'_>,
    witness: &super::super::super::ProbabilisticCounterexample,
    limits: &crate::ExactProbLimits,
    cancel: &Cancel,
) -> Result<
    super::super::super::ProbabilityWitnessReplay,
    super::super::super::ProbabilityWitnessError,
> {
    use super::super::super::{
        ProbabilityWitnessError as Error, ProbabilityWitnessReplay as Replay,
    };
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
    let at_least = matches!(query.bound, Bound::AtLeast(_));
    if !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
        || !claim.fairness.is_empty()
    {
        return Err(unsupported());
    }
    let timed = TimedSystem::new(system, &claim.subject.alias).map_err(|_| unsupported())?;
    let region = region_route(&timed, claim);
    if region != at_least {
        return Err(unsupported());
    }
    let route = if region {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let scale = certificate_scale(system, &timed, claim, route).map_err(from_document)?;
    let reference = claim_binding(system, claim)
        .map_err(|_| unsupported())?
        .map(|binding| binding.reference);
    if reference != witness.binding {
        return Err(Error::Refused);
    }
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let mut meter =
        filament_exact::Meter::new(document_limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let (graph, terminals, _) = enumerate(
        system,
        claim,
        query,
        &timed,
        scale,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    )
    .map_err(from_document)?;
    if witness.initial_key != graph.initial {
        return Err(Error::Refused);
    }
    let violating = terminals
        .iter()
        .filter(|(_, (_, within))| *within == at_least)
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    let activated = terminals.keys().cloned().collect::<BTreeSet<_>>();
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
    let payoff = Payoff::Elapsed {
        violating: &violating,
        activated: &activated,
        q: &query.q,
        strict: at_least,
    };
    let Some((kept, mass)) = price_paths(
        &graph,
        &payoff,
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
        super::super::super::ReproducedProbabilityWitness::reproduced(validated, mass),
    )))
}
