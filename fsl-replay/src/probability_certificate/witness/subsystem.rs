// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202's checked-subject subsystem entry for untimed long-run fractions,
//! expected rewards and unwindowed `eventually`, `until` and `always` claims.

use super::*;
use crate::probability_certificate::document::{long_run, refutation};

/// Bind the evidence to the checked claim, rebuild the workload- or
/// scheduler-induced chain and check its certificate in the counterexample
/// role. A token is minted only after the strict refutation comparison and a
/// final cancellation poll.
pub(super) fn replay(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let long_run_claim = matches!(claim.form, ProbForm::LongRunFraction { .. });
    if !(long_run_claim
        || matches!(claim.form, ProbForm::ExpectedReward { .. })
        || matches!(
            claim.form,
            ProbForm::Reach {
                target: ReachTarget::Eventually(_)
                    | ReachTarget::Until(..)
                    | ReachTarget::Always(_),
                ..
            }
        ))
        || !witness.paths.is_empty()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    if !claim.fairness.is_empty()
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    let subsystem = witness
        .subsystem
        .as_ref()
        .ok_or(ProbabilityWitnessError::Refused)?;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != witness.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    if key_of(system, reference, initial, limits, cancel)? != witness.initial_key
        || key_of(system, reference, &witness.initial, limits, cancel)? != witness.initial_key
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let workload = matches!(claim.scheduler, ProbScheduler::Workload(_));
    let rows = deterministic_scheduler_rows(&witness.scheduler)?;
    // A workload counterexample has no scheduler. An every-scheduler one
    // carries FR-200's deterministic memoryless scheduler, whose coverage of
    // the induced chain the role checker decides.
    if workload && !rows.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let claims = system.input().probabilistic.claims();
    let member = |error| match error {
        ProbabilityDocumentError::MalformedMember(member)
        | ProbabilityDocumentError::MissingMember(member) => {
            ProbabilityWitnessError::CertificateRejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member(member),
            })
        }
        other => bounded::document_error(other),
    };
    if long_run_claim {
        match long_run::check_refutation(
            system,
            claims,
            &witness.item,
            subsystem.as_bytes(),
            document_limits,
            cancel,
        )
        .map_err(member)?
        {
            ExactProbabilityCertificateCheck::Accepted(_) => {}
            ExactProbabilityCertificateCheck::Rejected(rejection) => {
                return Err(ProbabilityWitnessError::CertificateRejected(rejection))
            }
            ExactProbabilityCertificateCheck::AcceptedVacuous(_) => {
                return Err(ProbabilityWitnessError::Refused)
            }
        }
    } else {
        match refutation::check(
            system,
            claims,
            &witness.item,
            subsystem.as_bytes(),
            &rows,
            document_limits,
            cancel,
        )
        .map_err(member)?
        {
            refutation::SubsystemRefutationCheck::Accepted => {}
            refutation::SubsystemRefutationCheck::Rejected(rejection) => {
                return Err(ProbabilityWitnessError::CertificateRejected(rejection))
            }
            refutation::SubsystemRefutationCheck::SchedulerRefused => {
                return Err(ProbabilityWitnessError::Refused)
            }
        }
    }
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence: witness.clone(),
            mass: rational(1),
            undefined: None,
        },
    )))
}
