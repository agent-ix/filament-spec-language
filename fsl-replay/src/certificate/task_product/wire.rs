// SPDX-License-Identifier: AGPL-3.0-or-later
//! The JSON codec of FSpec FR-418's `task_product` member and of
//! `TaskTransitionTemplate` (the `TaskProductCertificate` and
//! `TaskTransitionTemplate` definitions of the FR-331 schema, FR-418-AC-11).
//!
//! The reader is strict: it admits exactly the schema's shape and refuses
//! everything else with a [`TaskProductShapeRefusal`], which maps onto the
//! certificate's `malformed` rule at the `task_product` member. It checks
//! wire shape only; recomputing the product is the checker's work.

use filament_exact::{Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use quire_canonical::Limits;
use serde_json::{json, Map, Value};

use super::{
    TaskArrivalKind, TaskClockCeiling, TaskClosureEvidence, TaskClosureMethod,
    TaskDiagonalConstraint, TaskDiagonalOperator, TaskProductCertificate, TaskQueueBound,
    TaskSubtraction, TaskTimeProgress, TaskTransitionTemplate,
};
use crate::certificate::closure::CertificateRejection;
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::TaskTransitionAction;

/// The largest index or count the wire carries (RFC 8785 exact integers).
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// The byte bound of one transition-identity text.
const IDENTITY_LIMITS: Limits = Limits::new(1 << 20);

/// Why a `task_product` value is not a `TaskProductCertificate`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskProductShapeReason {
    /// A required member is absent.
    Missing,
    /// A member the definition does not name.
    UnknownMember,
    /// The value is of another JSON kind.
    WrongKind,
    /// An integer outside its range, or a string that is not the canonical
    /// spelling of its type, or a rational not in lowest terms.
    NotCanonical,
    /// An enumerated spelling the definition does not list.
    UnknownVariant,
    /// Members that are individually valid but contradict each other, such
    /// as a closure label that does not belong to the method, a cap the
    /// method forbids, or an empty `queue_bounds`.
    Inconsistent,
}

/// A `task_product` value that is not a `TaskProductCertificate`, with the
/// JSON pointer (relative to the member) of the first offence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("task_product{pointer}: {reason:?}")]
pub struct TaskProductShapeRefusal {
    /// The JSON pointer of the offence, empty for the member itself.
    pub pointer: String,
    /// What is wrong there.
    pub reason: TaskProductShapeReason,
}

impl TaskProductShapeRefusal {
    /// The certificate rejection FSpec FR-418 gives it: `malformed` at the
    /// certificate member `{member: task_product}`.
    pub fn rejection(&self) -> CertificateRejection {
        CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(
                filament_contract_model::CertificateMember::TaskProduct,
            ),
        }
    }
}

/// A value that has no wire spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the task-product value has no wire spelling")]
pub struct TaskProductWireError;

fn refuse<T>(pointer: &str, reason: TaskProductShapeReason) -> Result<T, TaskProductShapeRefusal> {
    Err(TaskProductShapeRefusal {
        pointer: pointer.to_owned(),
        reason,
    })
}

fn child(pointer: &str, name: impl std::fmt::Display) -> String {
    format!("{pointer}/{name}")
}

/// An object whose members are exactly `required` plus any of `optional`.
fn object<'v>(
    value: &'v Value,
    pointer: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<&'v Map<String, Value>, TaskProductShapeRefusal> {
    let Some(map) = value.as_object() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    for name in map.keys() {
        if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
            return refuse(&child(pointer, name), TaskProductShapeReason::UnknownMember);
        }
    }
    for name in required {
        if !map.contains_key(*name) {
            return refuse(&child(pointer, name), TaskProductShapeReason::Missing);
        }
    }
    Ok(map)
}

fn member<'v>(map: &'v Map<String, Value>, name: &str) -> &'v Value {
    // Callers read only members `object` has proved present.
    map.get(name).unwrap_or(&Value::Null)
}

fn array<'v>(value: &'v Value, pointer: &str) -> Result<&'v [Value], TaskProductShapeRefusal> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or(TaskProductShapeRefusal {
            pointer: pointer.to_owned(),
            reason: TaskProductShapeReason::WrongKind,
        })
}

fn text(value: &Value, pointer: &str) -> Result<String, TaskProductShapeRefusal> {
    match value.as_str() {
        Some("") => refuse(pointer, TaskProductShapeReason::NotCanonical),
        Some(text) => Ok(text.to_owned()),
        None => refuse(pointer, TaskProductShapeReason::WrongKind),
    }
}

fn count(value: &Value, pointer: &str) -> Result<u64, TaskProductShapeRefusal> {
    match value.as_u64() {
        Some(count) if count <= MAX_COUNT => Ok(count),
        Some(_) => refuse(pointer, TaskProductShapeReason::NotCanonical),
        None => refuse(
            pointer,
            if value.is_number() {
                TaskProductShapeReason::NotCanonical
            } else {
                TaskProductShapeReason::WrongKind
            },
        ),
    }
}

fn positive_count(value: &Value, pointer: &str) -> Result<u64, TaskProductShapeRefusal> {
    match count(value, pointer)? {
        0 => refuse(pointer, TaskProductShapeReason::NotCanonical),
        count => Ok(count),
    }
}

fn integer(value: &Value, pointer: &str) -> Result<Integer, TaskProductShapeRefusal> {
    let Some(spelling) = value.as_str() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    spelling
        .parse()
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))
}

fn positive_u64(value: &Value, pointer: &str) -> Result<u64, TaskProductShapeRefusal> {
    match integer(value, pointer)?.to_u64() {
        Some(0) | None => refuse(pointer, TaskProductShapeReason::NotCanonical),
        Some(value) => Ok(value),
    }
}

fn rational(value: &Value, pointer: &str) -> Result<Rational, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["numerator", "denominator"], &[])?;
    let numerator = integer(member(map, "numerator"), &child(pointer, "numerator"))?;
    let denominator = integer(member(map, "denominator"), &child(pointer, "denominator"))?;
    let invalid = || TaskProductShapeRefusal {
        pointer: pointer.to_owned(),
        reason: TaskProductShapeReason::NotCanonical,
    };
    if denominator.is_zero() || denominator.is_negative() {
        return Err(invalid());
    }
    let reduced = Rational::new(numerator.clone(), denominator.clone()).map_err(|_| invalid())?;
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return Err(invalid());
    }
    Ok(reduced)
}

fn rational_wire(value: &Rational) -> Value {
    json!({
        "denominator": value.denominator().to_string(),
        "numerator": value.numerator().to_string(),
    })
}

fn enumerated<T: Copy>(
    value: &Value,
    pointer: &str,
    table: &[(&str, T)],
) -> Result<T, TaskProductShapeRefusal> {
    let Some(spelling) = value.as_str() else {
        return refuse(pointer, TaskProductShapeReason::WrongKind);
    };
    table
        .iter()
        .find(|(name, _)| *name == spelling)
        .map(|(_, variant)| *variant)
        .ok_or(TaskProductShapeRefusal {
            pointer: pointer.to_owned(),
            reason: TaskProductShapeReason::UnknownVariant,
        })
}

const ACTIONS: [(&str, TaskTransitionAction); 8] = [
    ("activate", TaskTransitionAction::Activate),
    ("release", TaskTransitionAction::Release),
    ("dispatch", TaskTransitionAction::Dispatch),
    ("preempt", TaskTransitionAction::Preempt),
    ("resume", TaskTransitionAction::Resume),
    ("complete", TaskTransitionAction::Complete),
    ("budget-change", TaskTransitionAction::BudgetChange),
    ("deadline-miss", TaskTransitionAction::DeadlineMiss),
];

fn action_name(action: TaskTransitionAction) -> &'static str {
    ACTIONS
        .iter()
        .find(|(_, candidate)| *candidate == action)
        .map_or("", |(name, _)| name)
}

/// A source transition identity `{type, operation, receiver, arguments}`.
fn source_transition(
    value: &Value,
    pointer: &str,
) -> Result<ModelTransition, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &["type", "operation", "receiver", "arguments"],
        &[],
    )?;
    if member(map, "type").as_str() != Some("transition") {
        return refuse(
            &child(pointer, "type"),
            TaskProductShapeReason::UnknownVariant,
        );
    }
    let operation = text(member(map, "operation"), &child(pointer, "operation"))?;
    let typed = |value: &Value, at: String| -> Result<String, TaskProductShapeRefusal> {
        if value.get("type").and_then(Value::as_str).is_none() {
            return refuse(&at, TaskProductShapeReason::WrongKind);
        }
        serde_json::to_string(value).or_else(|_| refuse(&at, TaskProductShapeReason::NotCanonical))
    };
    let receiver = match member(map, "receiver") {
        Value::Null => None,
        value => Some(typed(value, child(pointer, "receiver"))?),
    };
    let arguments_at = child(pointer, "arguments");
    let arguments = array(member(map, "arguments"), &arguments_at)?
        .iter()
        .enumerate()
        .map(|(index, value)| typed(value, child(&arguments_at, index)))
        .collect::<Result<Vec<_>, _>>()?;
    let operation = serde_json::to_string(&Value::String(operation))
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))?;
    ModelTransition::new(operation, receiver, arguments, IDENTITY_LIMITS)
        .or_else(|_| refuse(pointer, TaskProductShapeReason::NotCanonical))
}

fn source_transition_wire(transition: &ModelTransition) -> Result<Value, TaskProductWireError> {
    crate::task_source_transition_wire(transition).map_err(|_| TaskProductWireError)
}

impl TaskTransitionTemplate {
    /// Reads a `TaskTransitionTemplate` (FR-418-AC-11): the closed shape
    /// with `type` `task-transition-template`, and no occurrence,
    /// generation, demand, jitter or deadline.
    ///
    /// # Errors
    ///
    /// [`TaskProductShapeRefusal`] naming the first offence.
    pub fn from_wire(value: &Value) -> Result<Self, TaskProductShapeRefusal> {
        Self::read(value, "")
    }

    fn read(value: &Value, pointer: &str) -> Result<Self, TaskProductShapeRefusal> {
        let map = object(
            value,
            pointer,
            &[
                "type",
                "action",
                "taskset",
                "task",
                "job_rank",
                "slot",
                "release",
                "source_transition",
            ],
            &["priority"],
        )?;
        if member(map, "type").as_str() != Some("task-transition-template") {
            return refuse(
                &child(pointer, "type"),
                TaskProductShapeReason::UnknownVariant,
            );
        }
        let source = match member(map, "source_transition") {
            Value::Null => None,
            value => Some(source_transition(
                value,
                &child(pointer, "source_transition"),
            )?),
        };
        let priority = map
            .get("priority")
            .map(|value| count(value, &child(pointer, "priority")))
            .transpose()?;
        Ok(Self {
            action: enumerated(member(map, "action"), &child(pointer, "action"), &ACTIONS)?,
            task_set: text(member(map, "taskset"), &child(pointer, "taskset"))?,
            task: text(member(map, "task"), &child(pointer, "task"))?,
            job_rank: count(member(map, "job_rank"), &child(pointer, "job_rank"))?,
            slot: count(member(map, "slot"), &child(pointer, "slot"))?,
            release: source_transition(member(map, "release"), &child(pointer, "release"))?,
            source_transition: source,
            priority,
        })
    }

    /// The template's wire value, members in ascending order.
    ///
    /// # Errors
    ///
    /// [`TaskProductWireError`] when a source transition has no spelling.
    pub fn to_wire(&self) -> Result<Value, TaskProductWireError> {
        let mut map = Map::new();
        map.insert("action".to_owned(), json!(action_name(self.action)));
        map.insert("job_rank".to_owned(), json!(self.job_rank));
        if let Some(priority) = self.priority {
            map.insert("priority".to_owned(), json!(priority));
        }
        map.insert("release".to_owned(), source_transition_wire(&self.release)?);
        map.insert("slot".to_owned(), json!(self.slot));
        map.insert(
            "source_transition".to_owned(),
            match &self.source_transition {
                Some(transition) => source_transition_wire(transition)?,
                None => Value::Null,
            },
        );
        map.insert("task".to_owned(), json!(self.task));
        map.insert("taskset".to_owned(), json!(self.task_set));
        map.insert("type".to_owned(), json!("task-transition-template"));
        Ok(Value::Object(map))
    }
}

const METHODS: [(&str, TaskClosureMethod); 2] = [
    (
        "closed-finite-product/v1",
        TaskClosureMethod::ClosedFiniteProduct,
    ),
    (
        "closed-no-release-product/v1",
        TaskClosureMethod::ClosedNoReleaseProduct,
    ),
];

const CLOSURES: [(&str, TaskClosureEvidence); 2] = [
    (
        "recompute-all-reachable-successors/v1",
        TaskClosureEvidence::RecomputeAllReachableSuccessors,
    ),
    (
        "recompute-all-source-successors-no-task-release/v1",
        TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease,
    ),
];

const ARRIVALS: [(&str, TaskArrivalKind); 2] = [
    ("periodic", TaskArrivalKind::Periodic),
    ("sporadic", TaskArrivalKind::Sporadic),
];

const OPERATORS: [(&str, TaskDiagonalOperator); 4] = [
    ("<", TaskDiagonalOperator::Less),
    ("<=", TaskDiagonalOperator::LessOrEqual),
    (">", TaskDiagonalOperator::Greater),
    (">=", TaskDiagonalOperator::GreaterOrEqual),
];

impl TaskProductCertificate {
    /// Reads a `task_product` member (FR-418-AC-11, AC-14): the
    /// `bounded-subtraction-diagonal/v1` abstraction, the structured
    /// `time_progress`, and the four arrays, each row closed. It refuses an
    /// unknown method, a closure label that is not the method's, a zero cap
    /// under `closed-finite-product/v1`, a nonzero cap, clock ceilings,
    /// diagonals or subtractions under `closed-no-release-product/v1`, an
    /// empty `queue_bounds`, and a template or subtraction that carries an
    /// exact task identity.
    ///
    /// # Errors
    ///
    /// [`TaskProductShapeRefusal`] naming the first offence.
    pub fn from_wire(value: &Value) -> Result<Self, TaskProductShapeRefusal> {
        let map = object(
            value,
            "",
            &[
                "abstraction",
                "queue_bounds",
                "clock_ceilings",
                "diagonal_constraints",
                "subtractions",
                "time_progress",
            ],
            &[],
        )?;
        if member(map, "abstraction").as_str() != Some(Self::ABSTRACTION) {
            return refuse("/abstraction", TaskProductShapeReason::UnknownVariant);
        }
        let progress = object(
            member(map, "time_progress"),
            "/time_progress",
            &["method", "live_job_cap", "closure"],
            &[],
        )?;
        let method = enumerated(
            member(progress, "method"),
            "/time_progress/method",
            &METHODS,
        )?;
        let live_job_cap = count(
            member(progress, "live_job_cap"),
            "/time_progress/live_job_cap",
        )?;
        let closure = enumerated(
            member(progress, "closure"),
            "/time_progress/closure",
            &CLOSURES,
        )?;
        let (cap_ok, closure_ok) = match method {
            TaskClosureMethod::ClosedFiniteProduct => (
                live_job_cap > 0,
                closure == TaskClosureEvidence::RecomputeAllReachableSuccessors,
            ),
            TaskClosureMethod::ClosedNoReleaseProduct => (
                live_job_cap == 0,
                closure == TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease,
            ),
        };
        if !cap_ok {
            return refuse(
                "/time_progress/live_job_cap",
                TaskProductShapeReason::Inconsistent,
            );
        }
        if !closure_ok {
            return refuse(
                "/time_progress/closure",
                TaskProductShapeReason::Inconsistent,
            );
        }

        let rows = array(member(map, "queue_bounds"), "/queue_bounds")?;
        if rows.is_empty() {
            return refuse("/queue_bounds", TaskProductShapeReason::Inconsistent);
        }
        let queue_bounds = rows
            .iter()
            .enumerate()
            .map(|(index, row)| read_queue_bound(row, &format!("/queue_bounds/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        let clock_ceilings = array(member(map, "clock_ceilings"), "/clock_ceilings")?
            .iter()
            .enumerate()
            .map(|(index, row)| read_ceiling(row, &format!("/clock_ceilings/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        let diagonal_constraints =
            array(member(map, "diagonal_constraints"), "/diagonal_constraints")?
                .iter()
                .enumerate()
                .map(|(index, row)| read_diagonal(row, &format!("/diagonal_constraints/{index}")))
                .collect::<Result<Vec<_>, _>>()?;
        let subtractions = array(member(map, "subtractions"), "/subtractions")?
            .iter()
            .enumerate()
            .map(|(index, row)| read_subtraction(row, &format!("/subtractions/{index}")))
            .collect::<Result<Vec<_>, _>>()?;
        if method == TaskClosureMethod::ClosedNoReleaseProduct {
            for (name, empty) in [
                ("clock_ceilings", clock_ceilings.is_empty()),
                ("diagonal_constraints", diagonal_constraints.is_empty()),
                ("subtractions", subtractions.is_empty()),
            ] {
                if !empty {
                    return refuse(&format!("/{name}"), TaskProductShapeReason::Inconsistent);
                }
            }
        }
        Ok(Self {
            abstraction: Self::ABSTRACTION,
            clock_ceilings,
            diagonal_constraints,
            queue_bounds,
            subtractions,
            time_progress: TaskTimeProgress {
                method,
                live_job_cap,
                closure,
            },
        })
    }

    /// The member's wire value, members in ascending order. It writes what
    /// the value holds; whether the value is a correct product is the
    /// checker's question, not the writer's.
    ///
    /// # Errors
    ///
    /// [`TaskProductWireError`] when a source transition has no spelling.
    pub fn to_wire(&self) -> Result<Value, TaskProductWireError> {
        let queue_bounds: Vec<Value> = self
            .queue_bounds
            .iter()
            .map(|row| {
                json!({
                    "arrival": row.arrival.as_str(),
                    "interarrival": rational_wire(&row.interarrival),
                    "max_jitter": rational_wire(&row.max_jitter),
                    "max_jobs": row.max_jobs,
                    "relative_deadline": rational_wire(&row.relative_deadline),
                    "task": row.task,
                    "taskset": row.task_set,
                    "work_lower": rational_wire(&row.work_lower),
                    "work_upper": rational_wire(&row.work_upper),
                })
            })
            .collect();
        let clock_ceilings: Vec<Value> = self
            .clock_ceilings
            .iter()
            .map(|row| json!({"clock": row.clock, "maximum": row.maximum.to_string()}))
            .collect();
        let diagonal_constraints: Vec<Value> = self
            .diagonal_constraints
            .iter()
            .map(|row| {
                json!({
                    "constant": row.constant.to_string(),
                    "left": row.left,
                    "operator": row.operator.as_str(),
                    "right": row.right,
                })
            })
            .collect();
        let subtractions = self
            .subtractions
            .iter()
            .map(|row| {
                Ok(json!({
                    "clock": row.clock,
                    "constant": row.constant.to_string(),
                    "transition": row.transition.to_wire()?,
                }))
            })
            .collect::<Result<Vec<Value>, TaskProductWireError>>()?;
        Ok(json!({
            "abstraction": self.abstraction,
            "clock_ceilings": clock_ceilings,
            "diagonal_constraints": diagonal_constraints,
            "queue_bounds": queue_bounds,
            "subtractions": subtractions,
            "time_progress": {
                "closure": self.time_progress.closure.as_str(),
                "live_job_cap": self.time_progress.live_job_cap,
                "method": self.time_progress.method.as_str(),
            },
        }))
    }
}

fn read_queue_bound(
    value: &Value,
    pointer: &str,
) -> Result<TaskQueueBound, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &[
            "taskset",
            "task",
            "arrival",
            "interarrival",
            "relative_deadline",
            "max_jitter",
            "work_lower",
            "work_upper",
            "max_jobs",
        ],
        &[],
    )?;
    let at = |name: &str| child(pointer, name);
    Ok(TaskQueueBound {
        task_set: text(member(map, "taskset"), &at("taskset"))?,
        task: text(member(map, "task"), &at("task"))?,
        arrival: enumerated(member(map, "arrival"), &at("arrival"), &ARRIVALS)?,
        interarrival: rational(member(map, "interarrival"), &at("interarrival"))?,
        relative_deadline: rational(member(map, "relative_deadline"), &at("relative_deadline"))?,
        max_jitter: rational(member(map, "max_jitter"), &at("max_jitter"))?,
        work_lower: rational(member(map, "work_lower"), &at("work_lower"))?,
        work_upper: rational(member(map, "work_upper"), &at("work_upper"))?,
        max_jobs: positive_count(member(map, "max_jobs"), &at("max_jobs"))?,
    })
}

fn read_ceiling(value: &Value, pointer: &str) -> Result<TaskClockCeiling, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["clock", "maximum"], &[])?;
    Ok(TaskClockCeiling {
        clock: text(member(map, "clock"), &child(pointer, "clock"))?,
        maximum: positive_u64(member(map, "maximum"), &child(pointer, "maximum"))?,
    })
}

fn read_diagonal(
    value: &Value,
    pointer: &str,
) -> Result<TaskDiagonalConstraint, TaskProductShapeRefusal> {
    let map = object(
        value,
        pointer,
        &["left", "operator", "right", "constant"],
        &[],
    )?;
    Ok(TaskDiagonalConstraint {
        left: text(member(map, "left"), &child(pointer, "left"))?,
        operator: enumerated(
            member(map, "operator"),
            &child(pointer, "operator"),
            &OPERATORS,
        )?,
        right: text(member(map, "right"), &child(pointer, "right"))?,
        constant: integer(member(map, "constant"), &child(pointer, "constant"))?,
    })
}

fn read_subtraction(
    value: &Value,
    pointer: &str,
) -> Result<TaskSubtraction, TaskProductShapeRefusal> {
    let map = object(value, pointer, &["transition", "clock", "constant"], &[])?;
    Ok(TaskSubtraction {
        transition: TaskTransitionTemplate::read(
            member(map, "transition"),
            &child(pointer, "transition"),
        )?,
        clock: text(member(map, "clock"), &child(pointer, "clock"))?,
        constant: positive_u64(member(map, "constant"), &child(pointer, "constant"))?,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const ZC19: &str = r#"{"abstraction":"bounded-subtraction-diagonal/v1","clock_ceilings":[],"diagonal_constraints":[],"queue_bounds":[{"arrival":"periodic","interarrival":{"denominator":"1","numerator":"2"},"max_jitter":{"denominator":"1","numerator":"0"},"max_jobs":1,"relative_deadline":{"denominator":"1","numerator":"2"},"task":"A","taskset":"S","work_lower":{"denominator":"1","numerator":"1"},"work_upper":{"denominator":"1","numerator":"1"}}],"subtractions":[{"clock":"A.service","constant":"1","transition":{"action":"complete","job_rank":0,"release":{"arguments":[],"operation":"Task::release","receiver":null,"type":"transition"},"slot":0,"source_transition":null,"task":"A","taskset":"S","type":"task-transition-template"}}],"time_progress":{"closure":"recompute-all-reachable-successors/v1","live_job_cap":1,"method":"closed-finite-product/v1"}}"#;

    const ZC26: &str = r#"{"abstraction":"bounded-subtraction-diagonal/v1","clock_ceilings":[],"diagonal_constraints":[],"queue_bounds":[{"arrival":"periodic","interarrival":{"denominator":"1","numerator":"2"},"max_jitter":{"denominator":"1","numerator":"0"},"max_jobs":1,"relative_deadline":{"denominator":"1","numerator":"2"},"task":"A","taskset":"S","work_lower":{"denominator":"1","numerator":"1"},"work_upper":{"denominator":"1","numerator":"2"}}],"subtractions":[],"time_progress":{"closure":"recompute-all-source-successors-no-task-release/v1","live_job_cap":0,"method":"closed-no-release-product/v1"}}"#;

    fn parse(text: &str) -> Value {
        serde_json::from_str(text).expect("fixture is JSON")
    }

    fn mutated(text: &str, edit: impl FnOnce(&mut Value)) -> Value {
        let mut value = parse(text);
        edit(&mut value);
        value
    }

    fn refusal(value: &Value) -> TaskProductShapeRefusal {
        TaskProductCertificate::from_wire(value).expect_err("the shape is refused")
    }

    /// ZC-19 and ZC-26 read and write back to the published bytes.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn the_published_members_round_trip_byte_for_byte() {
        for fixture in [ZC19, ZC26] {
            let read = TaskProductCertificate::from_wire(&parse(fixture)).expect("reads");
            let written = read.to_wire().expect("writes");
            assert_eq!(serde_json::to_string(&written).expect("text"), fixture);
        }
    }

    /// The template is the closed finite shape.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn a_template_round_trips_and_carries_an_optional_priority() {
        let mut value = parse(ZC19)["subtractions"][0]["transition"].clone();
        value["priority"] = json!(3);
        let template = TaskTransitionTemplate::from_wire(&value).expect("reads");
        assert_eq!(template.priority, Some(3));
        assert_eq!(template.to_wire().expect("writes"), value);
    }

    /// ZC-20 and ZC-22: an exact task identity is not a template.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn an_exact_task_identity_is_refused_in_place_of_a_template() {
        let value = mutated(ZC19, |value| {
            let transition = &mut value["subtractions"][0]["transition"];
            transition["type"] = json!("task-transition");
        });
        let refused = refusal(&value);
        assert_eq!(refused.pointer, "/subtractions/0/transition/type");
        assert_eq!(refused.reason, TaskProductShapeReason::UnknownVariant);
        let rejection = refused.rejection();
        assert_eq!(rejection.rule, CertificateRule::Malformed);
    }

    /// ZC-21: an occurrence, generation or demand added to a template.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn an_occurrence_or_generation_added_to_a_template_is_refused() {
        for member in ["job", "generation", "demand", "jitter", "deadline"] {
            let value = mutated(ZC19, |value| {
                value["subtractions"][0]["transition"][member] = json!(0);
            });
            let refused = refusal(&value);
            assert_eq!(
                refused.reason,
                TaskProductShapeReason::UnknownMember,
                "{member}"
            );
        }
    }

    /// ZC-27 and AC-11: zero cap, unknown method and a producer-asserted
    /// closure label are refused.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn a_zero_cap_an_unknown_method_or_a_foreign_closure_label_is_refused() {
        let zero = mutated(ZC19, |value| {
            value["time_progress"]["live_job_cap"] = json!(0)
        });
        assert_eq!(refusal(&zero).pointer, "/time_progress/live_job_cap");
        let method = mutated(ZC19, |value| {
            value["time_progress"]["method"] = json!("closed-finite-product/v2");
        });
        assert_eq!(
            refusal(&method).reason,
            TaskProductShapeReason::UnknownVariant
        );
        let label = mutated(ZC19, |value| {
            value["time_progress"]["closure"] = json!("asserted-closed");
        });
        assert_eq!(
            refusal(&label).reason,
            TaskProductShapeReason::UnknownVariant
        );
        let swapped = mutated(ZC19, |value| {
            value["time_progress"]["closure"] =
                json!("recompute-all-source-successors-no-task-release/v1");
        });
        assert_eq!(refusal(&swapped).pointer, "/time_progress/closure");
    }

    /// ZC-29 and the wire spellings of counts and integers.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn empty_queue_bounds_and_non_canonical_numbers_are_refused() {
        let empty = mutated(ZC26, |value| value["queue_bounds"] = json!([]));
        assert_eq!(refusal(&empty).pointer, "/queue_bounds");
        let float = mutated(ZC19, |value| {
            value["queue_bounds"][0]["max_jobs"] = json!(1.0);
        });
        assert_eq!(refusal(&float).reason, TaskProductShapeReason::NotCanonical);
        let zero_jobs = mutated(ZC19, |value| {
            value["queue_bounds"][0]["max_jobs"] = json!(0)
        });
        assert_eq!(refusal(&zero_jobs).pointer, "/queue_bounds/0/max_jobs");
        let leading = mutated(ZC19, |value| {
            value["subtractions"][0]["constant"] = json!("01");
        });
        assert_eq!(
            refusal(&leading).reason,
            TaskProductShapeReason::NotCanonical
        );
        let number = mutated(ZC19, |value| {
            value["subtractions"][0]["constant"] = json!(1)
        });
        assert_eq!(refusal(&number).reason, TaskProductShapeReason::WrongKind);
        let unreduced = mutated(ZC19, |value| {
            value["queue_bounds"][0]["interarrival"] =
                json!({"numerator": "4", "denominator": "2"});
        });
        assert_eq!(
            refusal(&unreduced).reason,
            TaskProductShapeReason::NotCanonical
        );
        let missing = mutated(ZC19, |value| {
            value
                .as_object_mut()
                .expect("object")
                .remove("subtractions");
        });
        assert_eq!(refusal(&missing).reason, TaskProductShapeReason::Missing);
        let policy = mutated(ZC19, |value| value["policy"] = json!("edf"));
        assert_eq!(
            refusal(&policy).reason,
            TaskProductShapeReason::UnknownMember
        );
    }

    /// ZC-27 (AC-14 shape): a positive cap, scheduler clocks, diagonals or
    /// subtractions under the no-release method.
    #[trace("TC-371", "FR-418-AC-14")]
    #[test]
    fn the_no_release_method_refuses_a_cap_and_scheduler_metadata() {
        let cap = mutated(ZC26, |value| {
            value["time_progress"]["live_job_cap"] = json!(1)
        });
        assert_eq!(refusal(&cap).pointer, "/time_progress/live_job_cap");
        let ceiling = mutated(ZC26, |value| {
            value["clock_ceilings"] = json!([{"clock": "A.service", "maximum": "2"}]);
        });
        assert_eq!(refusal(&ceiling).pointer, "/clock_ceilings");
        let diagonal = mutated(ZC26, |value| {
            value["diagonal_constraints"] =
                json!([{"left": "a", "operator": "<=", "right": "b", "constant": "-1"}]);
        });
        assert_eq!(refusal(&diagonal).pointer, "/diagonal_constraints");
        let subtraction = mutated(ZC26, |value| {
            value["subtractions"] = parse(ZC19)["subtractions"].clone();
        });
        assert_eq!(refusal(&subtraction).pointer, "/subtractions");
        let label = mutated(ZC26, |value| {
            value["time_progress"]["closure"] = json!("recompute-all-reachable-successors/v1");
        });
        assert_eq!(refusal(&label).pointer, "/time_progress/closure");
        assert!(TaskProductCertificate::from_wire(&parse(ZC26)).is_ok());
    }

    /// Ceilings, diagonals and every operator round-trip.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn ceilings_and_diagonal_constraints_round_trip() {
        let value = mutated(ZC19, |value| {
            value["clock_ceilings"] = json!([{"clock": "A.service", "maximum": "4"}]);
            value["diagonal_constraints"] = json!([
                {"constant": "-1", "left": "a", "operator": "<", "right": "b"},
                {"constant": "0", "left": "a", "operator": "<=", "right": "b"},
                {"constant": "2", "left": "a", "operator": ">", "right": "b"},
                {"constant": "3", "left": "a", "operator": ">=", "right": "b"},
            ]);
        });
        let read = TaskProductCertificate::from_wire(&value).expect("reads");
        assert_eq!(read.diagonal_constraints.len(), 4);
        assert_eq!(read.to_wire().expect("writes"), value);
        let bad = mutated(ZC19, |value| {
            value["diagonal_constraints"] =
                json!([{"constant": "0", "left": "a", "operator": "=", "right": "b"}]);
        });
        assert_eq!(refusal(&bad).reason, TaskProductShapeReason::UnknownVariant);
    }

    /// Every shape refusal is `malformed` at `{member: task_product}`.
    #[trace("TC-371", "FR-418-AC-11")]
    #[test]
    fn a_shape_refusal_maps_to_malformed_at_the_task_product_member() {
        let refused = refusal(&json!(null));
        let rejection = refused.rejection();
        assert_eq!(rejection.rule, CertificateRule::Malformed);
        assert_eq!(
            rejection.at,
            CertificateLocus::CertificateMember(
                filament_contract_model::CertificateMember::TaskProduct
            )
        );
    }
}
