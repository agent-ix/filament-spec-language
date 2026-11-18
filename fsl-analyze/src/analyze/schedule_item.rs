// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-7 item over a checked schedulability or response claim (FR-246,
//! FR-633): the claim's task set, with its WCETs, blocking terms and jitter,
//! is part of the checked claim, so the checked package is everything the
//! item needs. EN-7 decides it in closed form (FR-247) and FR-248's map
//! settles it after the independent recomputation agrees.

use filament_exact::Identifier;
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    Arrival, ClosedFormAnalysis, ClosedFormOutcome, MissEvidence, Criticality, DeclineCode, ProofRefusalCause, QualifiedName,
    ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task, TaskSet, TaskSetRefusal, TerminalValue,
    Wcet,
};
use fsl_route::Mode;
use fsl_semantics::check::{self as checked, Capability, DeclarationFamily};

use super::{AnalyzeItem, EngineReport, Refusal};
use crate::schedulability::{self, manifest, ScheduleLimits};

/// What EN-7 reported for one item: FR-248's settled value and the outcome
/// whose evidence it settled.
#[derive(Debug)]
pub(crate) struct ClosedFormReport {
    /// The value FR-248's map settled the outcome as.
    pub(crate) value: TerminalValue,
    /// EN-7's outcome, with its evidence.
    pub(crate) outcome: ClosedFormOutcome,
    /// FSpec FR-419's `schedulability-miss` counterexample of a miss.
    pub(crate) miss: Option<serde_json::Value>,
}

/// Why a checked claim has no closed-form subject.
enum Lowering {
    /// The package holds no checked schedulability claim at `claim`, or no
    /// task set it is over.
    Absent,
    /// The checked task set or claim names a value the subject refuses.
    Refused(TaskSetRefusal),
}

/// The EN-7 item of the checked claim at `claim`, or `None` when the package
/// holds no schedulability claim there.
pub(super) fn schedule_item<'a>(
    package: &CheckedPackage,
    request_index: RequestIndex,
    claim: &OccurrenceKey,
) -> Option<AnalyzeItem<'a>> {
    let (subject, name, unit) = match subject_of(package, claim) {
        Ok(found) => found,
        Err(Lowering::Absent) => return None,
        Err(Lowering::Refused(refusal)) => {
            let diagnostic = CatalogCode::new(refusal.code().as_str(), refusal.cause());
            return Some(item(request_index, move |_| {
                EngineReport::Refused(Refusal::Declined(
                    ProofRefusalCause::InvalidInput,
                    DeclineCode::Fsl(refusal.code()),
                    diagnostic,
                ))
            }));
        }
    };
    Some(item(request_index, move |cancel| {
        let outcome = schedulability::analyze(&subject, &ScheduleLimits::default(), cancel);
        let value = fsl_replay::settle(&subject, &outcome, cancel);
        let miss = miss_wire(&name, &unit, &subject, &outcome);
        EngineReport::ClosedForm(Box::new(ClosedFormReport {
            value,
            outcome,
            miss,
        }))
    }))
}

fn item<'a>(
    request_index: RequestIndex,
    work: impl FnOnce(&filament_exact::Cancel) -> EngineReport + 'a,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        request_index,
        Capability::Schedulability,
        Some(DeclarationFamily::RealTime),
        Mode::Unbounded,
        manifest::BACKEND_ID,
        work,
    )
}

/// The closed-form subject of the checked claim at `claim`: its task set and
/// its claim, in the task set's one unit.
fn subject_of(
    package: &CheckedPackage,
    claim: &OccurrenceKey,
) -> Result<(ScheduleSubject, String, String), Lowering> {
    let model = package.graph().task_sets();
    let checked = model
        .claims()
        .iter()
        .find(|checked| checked.occurrence().as_ref() == Some(claim))
        .ok_or(Lowering::Absent)?;
    let set = model.task_set_of(checked).ok_or(Lowering::Absent)?;
    let name = QualifiedName::new(vec![identifier(&set.name)?]).map_err(|_| Lowering::Absent)?;
    let tasks = set.tasks.iter().map(task).collect::<Result<Vec<_>, _>>()?;
    let task_set = TaskSet::new(name, tasks).map_err(Lowering::Refused)?;
    let claim = match &checked.claim {
        checked::ScheduleClaim::Schedulable(policy) => ScheduleClaim::Schedulable(match policy {
            checked::SchedulingPolicy::FixedPriority => SchedulingPolicy::FixedPriority,
            checked::SchedulingPolicy::Edf => SchedulingPolicy::Edf,
            checked::SchedulingPolicy::Amc => SchedulingPolicy::Amc,
        }),
        checked::ScheduleClaim::Response { task, bound } => ScheduleClaim::Response {
            task: identifier(task)?,
            bound: bound.clone(),
        },
    };
    let subject = ScheduleSubject::new(task_set, claim).map_err(Lowering::Refused)?;
    let unit = set.unit.as_ref().map(|unit| unit.name.clone()).unwrap_or_default();
    Ok((subject, checked.name.clone(), unit))
}

fn identifier(name: &str) -> Result<Identifier, Lowering> {
    Identifier::new(name).map_err(|_| Lowering::Absent)
}

fn task(task: &checked::Task) -> Result<Task, Lowering> {
    Ok(Task {
        name: identifier(&task.name)?,
        arrival: match &task.arrival {
            checked::Arrival::Periodic { period } => Arrival::Periodic {
                period: period.clone(),
            },
            checked::Arrival::Sporadic { min_interarrival } => Arrival::Sporadic {
                min_interarrival: min_interarrival.clone(),
            },
        },
        deadline: task.deadline.clone(),
        wcet: match &task.wcet {
            checked::Wcet::Single(budget) => Wcet::Single(budget.clone()),
            checked::Wcet::Dual { lo, hi } => Wcet::Dual {
                lo: lo.clone(),
                hi: hi.clone(),
            },
        },
        priority: task.priority,
        criticality: match task.criticality {
            checked::Criticality::Lo => Criticality::Lo,
            checked::Criticality::Hi => Criticality::Hi,
        },
        jitter: task.jitter.clone(),
        blocking: task.blocking.clone(),
    })
}

/// FSpec FR-419's `schedulability-miss` counterexample of the miss `outcome`
/// reports over `subject` (FSL FR-621 "A refuted schedulability item"): every
/// time value an FR-181 rational in the task set's checked unit `unit`.
/// `None` for an outcome that is no miss, or a miss naming a task the set
/// does not hold.
pub(super) fn miss_wire(
    claim: &str,
    unit: &str,
    subject: &ScheduleSubject,
    outcome: &ClosedFormOutcome,
) -> Option<serde_json::Value> {
    let ClosedFormOutcome::Miss(evidence) = outcome else {
        return None;
    };
    let rational = |value: &filament_exact::Rational| {
        serde_json::json!({
            "type": "rational",
            "numerator": value.numerator().as_big().to_string(),
            "denominator": value.denominator().as_big().to_string(),
        })
    };
    let miss = match evidence {
        MissEvidence::FixedPriority {
            task,
            job,
            iterates,
        } => {
            let found = subject.task_set().task(task)?;
            let last = iterates.last()?;
            let offset = filament_exact::Rational::from_integer(filament_exact::Integer::from(i128::from(*job)))
                .mul(found.arrival.interval());
            let response = last.add(&found.jitter).sub(&offset);
            let bound = match subject.claim() {
                ScheduleClaim::Response { bound, .. } => bound.clone(),
                ScheduleClaim::Schedulable(_) => found.deadline.clone(),
            };
            let mut miss = serde_json::json!({
                "kind": "response",
                "task": task.as_str(),
                "job": job.to_string(),
                "iterates": iterates.iter().map(&rational).collect::<Vec<_>>(),
                "response": rational(&response),
                "bound": rational(&bound),
            });
            if subject.analysis() == ClosedFormAnalysis::AmcRtb {
                miss["mode"] = serde_json::json!("lo");
            }
            miss
        }
        MissEvidence::Edf(witness) => serde_json::json!({
            "kind": "demand",
            "t": rational(&witness.t),
            "demand": rational(&witness.demand),
        }),
        MissEvidence::Utilization(utilization) => serde_json::json!({
            "kind": "utilization",
            "utilization": rational(utilization),
        }),
    };
    Some(serde_json::json!({
        "type": "schedulability-miss",
        "claim": claim,
        "analysis": subject.analysis(),
        "unit": unit,
        "miss": miss,
    }))
}
