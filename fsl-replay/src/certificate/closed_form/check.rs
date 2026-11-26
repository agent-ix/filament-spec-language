// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-248's layer-6 entry: rebuild the schedulability subject from checked
//! package evidence, then independently recompute the reported outcome.

use filament_exact::{Cancel, Identifier};
use ix_trace_rs::implements;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{self as checked, ScheduleClaim as CheckedScheduleClaim};

use crate::execute::{self, ReplayRefusal};
use crate::{
    ClosedFormCheck, ClosedFormOutcome, ReplayRequest, ScheduleClaim, ScheduleSubject, Task,
    TaskSet, TaskSetRefusal,
};

/// The claim occurrence selected by FR-454: `[node, "claim", ordinal]`.
pub type ClaimTriple = OccurrenceKey;

/// The closed-form result together with the identity of its originating
/// obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosedFormResult {
    /// The obligation whose schedulability claim was analyzed.
    pub originating_obligation: fsl_package::ObligationIdentity,
    /// The analysis outcome and its evidence.
    pub outcome: ClosedFormOutcome,
}

/// Why the closed-form check could not obtain a checked schedulability
/// subject from the request.
#[derive(Debug)]
pub enum CertificateRefusal {
    /// Recompilation or package checking refused the request.
    Recompile(ReplayRefusal),
    /// The requested claim occurrence does not resolve to a checked
    /// schedulability claim with a task set.
    ClaimRootUnresolved,
    /// The checked task set or claim cannot form an admitted replay subject.
    TaskSet(TaskSetRefusal),
    /// A checked identifier cannot be represented by the replay API.
    InvalidIdentifier(String),
    /// The selected item, reported outcome, and claim occurrence do not
    /// identify the same schedulability obligation.
    ObligationIdentityMismatch {
        /// The identity selected by the caller.
        item: fsl_package::ObligationIdentity,
        /// The identity carried by the closed-form result.
        result: fsl_package::ObligationIdentity,
        /// The identity derived from the selected claim occurrence.
        claim: fsl_package::ObligationIdentity,
    },
}

/// Recompile the request, read the task set and claim at `claim`, and
/// independently check the evidence in exact rationals (FR-248).
#[implements("FR-248-AC-1")]
pub fn check_closed_form(
    request: ReplayRequest,
    item: &fsl_package::ObligationIdentity,
    claim: &ClaimTriple,
    result: &ClosedFormResult,
    cancel: &Cancel,
) -> Result<ClosedFormCheck, CertificateRefusal> {
    let claim_identity = fsl_package::ObligationIdentity::from_claim_occurrence(claim);
    if item != &result.originating_obligation || result.originating_obligation != claim_identity {
        return Err(CertificateRefusal::ObligationIdentityMismatch {
            item: item.clone(),
            result: result.originating_obligation.clone(),
            claim: claim_identity,
        });
    }
    let caller = execute::request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        request.limits(),
    )
    .map_err(CertificateRefusal::Recompile)?;
    let recompiled =
        execute::recompile(&request, &caller).map_err(CertificateRefusal::Recompile)?;
    let checked = recompiled.checked.package();
    let model = checked.graph().task_sets();
    let checked_claim = model
        .claims()
        .iter()
        .find(|candidate| candidate.occurrence().as_ref() == Some(claim))
        .ok_or(CertificateRefusal::ClaimRootUnresolved)?;
    let source_set = model
        .task_set_of(checked_claim)
        .ok_or(CertificateRefusal::ClaimRootUnresolved)?;
    let name = Identifier::new(&source_set.name)
        .map_err(|_| CertificateRefusal::InvalidIdentifier(source_set.name.clone()))?;
    let task_set = TaskSet::new(
        crate::QualifiedName::new(vec![name])
            .map_err(|_| CertificateRefusal::ClaimRootUnresolved)?,
        source_set
            .tasks
            .iter()
            .map(convert_task)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(CertificateRefusal::TaskSet)?;
    let claim = match &checked_claim.claim {
        CheckedScheduleClaim::Schedulable(policy) => ScheduleClaim::Schedulable(match policy {
            checked::SchedulingPolicy::FixedPriority => crate::SchedulingPolicy::FixedPriority,
            checked::SchedulingPolicy::Edf => crate::SchedulingPolicy::Edf,
            checked::SchedulingPolicy::Amc => crate::SchedulingPolicy::Amc,
        }),
        CheckedScheduleClaim::Response { task, bound } => ScheduleClaim::Response {
            task: Identifier::new(task)
                .map_err(|_| CertificateRefusal::InvalidIdentifier(task.clone()))?,
            bound: bound.clone(),
        },
    };
    let subject = ScheduleSubject::new(task_set, claim).map_err(CertificateRefusal::TaskSet)?;
    Ok(crate::recompute(&subject, &result.outcome, cancel))
}

fn convert_task(task: &checked::Task) -> Result<Task, CertificateRefusal> {
    let name = Identifier::new(&task.name)
        .map_err(|_| CertificateRefusal::InvalidIdentifier(task.name.clone()))?;
    Ok(Task {
        name,
        arrival: match &task.arrival {
            checked::Arrival::Periodic { period } => crate::Arrival::Periodic {
                period: period.clone(),
            },
            checked::Arrival::Sporadic { min_interarrival } => crate::Arrival::Sporadic {
                min_interarrival: min_interarrival.clone(),
            },
        },
        deadline: task.deadline.clone(),
        wcet: match &task.wcet {
            checked::Wcet::Single(budget) => crate::Wcet::Single(budget.clone()),
            checked::Wcet::Dual { lo, hi } => crate::Wcet::Dual {
                lo: lo.clone(),
                hi: hi.clone(),
            },
        },
        priority: task.priority,
        criticality: match task.criticality {
            checked::Criticality::Lo => crate::Criticality::Lo,
            checked::Criticality::Hi => crate::Criticality::Hi,
        },
        jitter: task.jitter.clone(),
        blocking: task.blocking.clone(),
    })
}
