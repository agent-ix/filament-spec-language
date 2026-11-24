// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940 (QSL-2785): a
//! `schedulability` item over a package reads its task set and claim from
//! the admitted `claim`/`schedulability` rows (FSpec FR-482) and runs the
//! item the source route runs over the same closed-form subject.

use filament_contract_model::{
    CheckedLiteralRead, CheckedLiteralValue, CheckedSchedulabilityKind, CheckedSchedulingPolicy,
    CheckedTaskArrival, CheckedTaskCriticality,
};
use filament_exact::{Identifier, Integer, Rational};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::{BarePackage, CheckedNodeId};
use fsl_replay::{
    Arrival, Criticality, QualifiedName, ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task,
    TaskSet, TaskSetRefusal, Wcet,
};
use fsl_route::request::ExtentClassification;
use fsl_semantics::check::Capability;

use super::super::super::{AnalyzeItem, ClaimItem, ItemOptions, schedule_item};
use super::super::{ItemSelection, ResolvedClaim, candidate_list, invalid_kind, unknown_option};
use super::{ClaimReads, PackageRunError, Resolution};

/// What a package's rows hold for one schedulability claim.
pub(super) struct ScheduleRoute {
    name: String,
    read: Result<(ScheduleSubject, String), TaskSetRefusal>,
}

impl ScheduleRoute {
    /// The EN-7 item the source route builds over the same subject.
    pub(super) fn item<'a>(self, index: RequestIndex, max_demand_points: u64) -> AnalyzeItem<'a> {
        match self.read {
            Ok((subject, unit)) => {
                schedule_item::subject_item(index, subject, self.name, unit, max_demand_points)
            }
            Err(refusal) => schedule_item::refused_item(index, refusal),
        }
    }
}

/// Why the package's rows give no closed-form subject.
enum Unread {
    /// No claim row at the declaration and ordinal.
    Absent,
    /// A row the strict reader admits but this read cannot spell.
    Malformed,
}

fn rational(read: &CheckedLiteralRead<'_>) -> Result<Rational, Unread> {
    match &read.value {
        CheckedLiteralValue::Rational {
            numerator,
            denominator,
        } => Rational::new(numerator.clone(), denominator.clone()).map_err(|_| Unread::Malformed),
        CheckedLiteralValue::Integer(value) => Ok(Rational::from_integer(value.clone())),
        _ => Err(Unread::Malformed),
    }
}

fn identifier(name: &str) -> Result<Identifier, Unread> {
    Identifier::new(name).map_err(|_| Unread::Malformed)
}

/// The closed-form subject of the claim at `declaration` and `ordinal`, with
/// the claim's occurrence and the task set's time unit.
type ScheduleSubjectRead = Result<(ScheduleSubject, String), TaskSetRefusal>;
type ScheduleSubjectResult = Result<(OccurrenceKey, ScheduleSubjectRead), Unread>;

fn read(
    package: &BarePackage<'_>,
    declaration: &CheckedNodeId,
    ordinal: u64,
) -> ScheduleSubjectResult {
    let roots =
        fsl_package::schedulability_roots(package.package()).map_err(|_| Unread::Malformed)?;
    let root = roots
        .iter()
        .find(|root| {
            root.claim().declaration() == declaration
                && root.claim().occurrence().ordinal == ordinal
        })
        .ok_or(Unread::Absent)?;
    let claim = root.claim();
    let mut unit = String::new();
    let mut tasks = Vec::new();
    for task in claim.tasks() {
        let deadline = task.deadline();
        if unit.is_empty() {
            unit = unit_name(package, deadline.type_id);
        }
        let period = rational(&task.period())?;
        let wcet = match task.wcet_hi() {
            Some(hi) => Wcet::Dual {
                lo: rational(&task.wcet())?,
                hi: rational(&hi)?,
            },
            None => Wcet::Single(rational(&task.wcet())?),
        };
        let priority = match task.priority() {
            Some(CheckedLiteralRead {
                value: CheckedLiteralValue::Integer(value),
                ..
            }) => Some(Integer::to_u64(&value).ok_or(Unread::Malformed)?),
            Some(_) => return Err(Unread::Malformed),
            None => None,
        };
        tasks.push(Task {
            name: identifier(task.name())?,
            arrival: match task.arrival() {
                CheckedTaskArrival::Periodic => Arrival::Periodic { period },
                CheckedTaskArrival::Sporadic => Arrival::Sporadic {
                    min_interarrival: period,
                },
            },
            deadline: rational(&deadline)?,
            wcet,
            priority,
            criticality: match task.criticality() {
                CheckedTaskCriticality::Lo => Criticality::Lo,
                CheckedTaskCriticality::Hi => Criticality::Hi,
            },
            jitter: rational(&task.jitter())?,
            blocking: rational(&task.blocking())?,
        });
    }
    let schedule = match claim.kind() {
        CheckedSchedulabilityKind::Schedulable => {
            ScheduleClaim::Schedulable(match claim.policy() {
                CheckedSchedulingPolicy::FixedPriority => SchedulingPolicy::FixedPriority,
                CheckedSchedulingPolicy::Edf => SchedulingPolicy::Edf,
                CheckedSchedulingPolicy::Amc => SchedulingPolicy::Amc,
            })
        }
        CheckedSchedulabilityKind::Response => ScheduleClaim::Response {
            task: identifier(claim.response_task().ok_or(Unread::Malformed)?)?,
            bound: rational(&claim.response_bound().ok_or(Unread::Malformed)?)?,
        },
    };
    // The task set's name is not a decision fact: no engine, record or
    // counterexample reads it.
    let name = QualifiedName::new(vec![identifier("TaskSet")?]).map_err(|_| Unread::Malformed)?;
    let subject = TaskSet::new(name, tasks)
        .and_then(|set| ScheduleSubject::new(set, schedule))
        .map(|subject| (subject, unit));
    Ok((root.occurrence().clone(), subject))
}

/// The unit's name as the source route spells it: the declaration of the
/// scalar type the task set's time values are typed with.
fn unit_name(package: &BarePackage<'_>, unit: &CheckedNodeId) -> String {
    package
        .package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.node_id == *unit)
        .and_then(|node| node.declaration.as_ref())
        .map(|declaration| declaration.qualified_name.join("::"))
        .unwrap_or_default()
}

impl<'r, 'a: 'r> ClaimReads<'r, 'a> {
    /// A `schedulability` item over the claim at `declaration` and
    /// `ordinal`, as the source route's `resolve_schedule_claim` resolves it.
    pub(super) fn schedule(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<ScheduleRoute>, PackageRunError> {
        let (claim, read) = match read(self.package, declaration, ordinal) {
            Ok(found) => found,
            Err(Unread::Absent | Unread::Malformed) => {
                return Err(super::pending_declaration(index, declaration));
            }
        };
        if selection
            .kind
            .as_deref()
            .is_some_and(|label| label != Capability::Schedulability.to_wire())
        {
            return Ok(Resolution::Settled(invalid_kind(
                index,
                Some(name),
                "inapplicable-kind",
            )));
        }
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self
            .registry
            .candidates(Capability::Schedulability, named.as_ref());
        let candidates = if named.is_some() {
            candidates
        } else {
            crate::tie_rule::narrow_task_product_default(candidates)
        };
        let backends = candidate_list(&candidates);
        let extent = if matches!(backends.as_slice(), [backend]
            if backend.id().as_str() == crate::schedulability::manifest::BACKEND_ID)
        {
            ExtentClassification::Unbounded {
                finite_bound_available: false,
            }
        } else {
            ExtentClassification::Bounded
        };
        let limits = selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.limits);
        let item = ClaimItem {
            request_index: index,
            claim,
            kind: Capability::Schedulability,
            extent,
            backend: named,
            candidates: backends,
            options: ItemOptions {
                required: selection.required.unwrap_or(true),
                max_cases: None,
                max_task_product_work: limits.and_then(|limits| limits.max_task_product_work),
                max_check_work: limits.and_then(|limits| limits.max_check_work),
                trace: None,
                traces: None,
                recording: None,
                population: selection.population.clone(),
                assessment: selection.assessment,
                backend_options,
                protocol_analysis: None,
            },
            form: None,
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: self.subject.clone(),
                unrouted: selection.unrouted(),
            }),
            ScheduleRoute {
                name: name.to_owned(),
                read,
            },
        ))
    }
}
