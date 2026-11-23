// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202's checked-subject subsystem entry for untimed long-run fractions,
//! expected rewards and unwindowed `eventually` claims.

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
                target: ReachTarget::Eventually(_),
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
        let scheduler = if workload {
            BTreeMap::new()
        } else {
            choice_indices(system, reference, &rows, limits, cancel)?
        };
        match refutation::check(
            system,
            claims,
            &witness.item,
            subsystem.as_bytes(),
            &scheduler,
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

/// Each scheduled complete key's chosen action, as its index in the subject's
/// own enabled-action order. Entries are strictly key-ordered identity
/// choices at states the subject actually reaches.
fn choice_indices(
    system: &ModelSystem<'_>,
    reference: Option<&ObjectReference>,
    rows: &[(ProbabilityProductKey, ProductAction)],
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<BTreeMap<ProbabilityProductKey, usize>, ProbabilityWitnessError> {
    if rows.len() > limits.model.limits.max_states {
        return Err(ExactProbabilityGraphError::LimitReached.into());
    }
    let mut wanted = BTreeMap::new();
    let mut previous: Option<&ProbabilityProductKey> = None;
    for (key, action) in rows {
        poll_cancel(cancel)?;
        let ProductAction::Identity(schedule) = action else {
            return Err(ProbabilityWitnessError::Refused);
        };
        if previous.is_some_and(|previous| previous >= key) {
            return Err(ProbabilityWitnessError::Refused);
        }
        previous = Some(key);
        wanted.insert(key.clone(), schedule);
    }
    let mut states = BTreeMap::new();
    let storage = std::cell::RefCell::new(0u64);
    let stopped = std::cell::RefCell::new(None);
    let mut issue = None;
    enumerate_model_graph_with_keys_and_reservation(
        system,
        None::<&fsl_semantics::check::CheckedPredicate>,
        reference,
        None,
        limits.model.limits.max_states,
        limits.model.limits.max_transitions,
        cancel,
        false,
        &|state| {
            key_of(system, reference, state, limits, cancel).map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |bytes| {
            bounded::retain(bytes, &mut storage.borrow_mut(), limits).map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        #[cfg(feature = "test-support")]
        &mut |_| {},
        &mut |key, state| {
            if wanted.contains_key(key) {
                bounded::retain(
                    key.as_bytes().len().saturating_mul(2),
                    &mut storage.borrow_mut(),
                    limits,
                )
                .map_err(|error| {
                    *stopped.borrow_mut() = Some(error);
                    ExactProbabilityGraphError::ExpansionRefused
                })?;
                states.insert(key.clone(), state.clone());
            }
            Ok(())
        },
        &mut |failure| issue = Some(failure),
    )
    .map_err(|error| {
        if let Some(error) = stopped.borrow_mut().take() {
            return error;
        }
        issue
            .take()
            .map_or(ProbabilityWitnessError::Graph(error), |issue| {
                ProbabilityWitnessError::Build(Box::new(issue))
            })
    })?;
    let mut indices = BTreeMap::new();
    for (key, schedule) in wanted {
        poll_cancel(cancel)?;
        let state = states.get(&key).ok_or(ProbabilityWitnessError::Refused)?;
        let actions = system
            .actions_with_cancel(state, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        let index = actions
            .iter()
            .position(|action| {
                crate::Schedule::of(system, &action.identity).as_ref() == Ok(schedule)
            })
            .ok_or(ProbabilityWitnessError::Refused)?;
        indices.insert(key, index);
    }
    Ok(indices)
}
