// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921's independent timed-product certificate admission.

use super::*;

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
    let route = if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        .is_some()
    {
        TimedProbabilityRoute::Digital
    } else {
        TimedProbabilityRoute::Region
    };
    if objective.route != route || objective.unit != timed.source().unit_node {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Err(ProbabilityDocumentError::Unsupported)
}
