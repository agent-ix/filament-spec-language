// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921's independent timed-product certificate admission.

use super::*;
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive};

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = read_document_profile(bytes, limits, cancel, &mut meter, false, None, &mut 0)?;
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
        || objective.form != TimedProbabilityForm::Deadline
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Err(ProbabilityDocumentError::Unsupported)
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
