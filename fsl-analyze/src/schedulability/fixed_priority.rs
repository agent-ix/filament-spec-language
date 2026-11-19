// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed-priority response-time analysis (ADR-026 RT-4) and `response`
//! claims.

use filament_exact::Cancel;
use fsl_replay::{
    ClosedFormEvidence, ClosedFormOutcome, MissEvidence, ScheduleSubject, Task, TaskFixpoint,
};

use super::arith::int;
use super::busy_period::{jobs, Jobs, Level, Term};
use super::Analysed;

/// The level-`i` recurrence of `task` with every budget at its highest.
fn level<'a>(subject: &'a ScheduleSubject, task: &'a Task) -> Level<'a> {
    Level {
        own: task.wcet.hi(),
        blocking: &task.blocking,
        constant: int(0),
        terms: subject
            .higher_priority(task)
            .map(|other| Term {
                period: other.arrival.interval(),
                jitter: &other.jitter,
                budget: other.wcet.hi(),
            })
            .collect(),
    }
}

/// Analyse each claimed task, highest priority first: the first to pass its
/// bound is the miss.
pub(super) fn analyse(subject: &ScheduleSubject, cancel: &Cancel) -> Analysed<ClosedFormOutcome> {
    let mut fixpoints = Vec::new();
    for task in subject.analysed() {
        match jobs(task, &level(subject, task), subject.bound(task), cancel)? {
            Jobs::Met(jobs) => fixpoints.push(TaskFixpoint {
                task: task.name.clone(),
                jobs,
            }),
            Jobs::Missed { job, iterates } => {
                return Ok(ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
                    task: task.name.clone(),
                    job,
                    iterates,
                }))
            }
        }
    }
    Ok(ClosedFormOutcome::Schedulable(
        ClosedFormEvidence::FixedPoints(fixpoints),
    ))
}

#[cfg(test)]
mod tests {
    use filament_exact::{Cancel, Integer, Rational};
    use fsl_replay::{
        Arrival, ClosedFormEvidence, ClosedFormOutcome, Criticality, Identifier, QualifiedName,
        ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task, TaskSet, Wcet,
    };
    use ix_trace_rs::trace;

    use super::analyse;

    fn integer(value: i64) -> Rational {
        Rational::from_integer(Integer::from(value))
    }

    fn task(name: &str, execution: i64, period: i64, priority: u64) -> Task {
        Task {
            name: Identifier::new(name).expect("a task name"),
            arrival: Arrival::Periodic {
                period: integer(period),
            },
            deadline: integer(period),
            wcet: Wcet::Single(integer(execution)),
            priority: Some(priority),
            criticality: Criticality::Lo,
            jitter: integer(0),
            blocking: integer(0),
        }
    }

    /// Trace: TC-702, FR-247-AC-1.
    #[trace("TC-702", "FR-247-AC-1")]
    #[test]
    fn buttazzo_response_time_oracle_matches_ctl_fixpoints() {
        let set = TaskSet::new(
            QualifiedName::new(vec![Identifier::new("Ctl").expect("a set name")])
                .expect("one name segment"),
            vec![
                task("a", 5, 20, 1),
                task("b", 10, 50, 2),
                task("c", 20, 100, 3),
            ],
        )
        .expect("an admitted task set");
        let subject = ScheduleSubject::new(
            set,
            ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority),
        )
        .expect("an admitted fixed-priority claim");
        let outcome = match analyse(&subject, &Cancel::new()) {
            Ok(outcome) => outcome,
            Err(_) => panic!("the fixed-priority oracle completes"),
        };
        let ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(fixpoints)) = outcome
        else {
            panic!("the Ctl oracle is schedulable");
        };
        let responses: Vec<_> = fixpoints
            .iter()
            .map(|fixpoint| {
                fixpoint.jobs[0]
                    .last()
                    .expect("a job has a response-time fixpoint")
                    .clone()
            })
            .collect();

        assert_eq!(responses, vec![integer(5), integer(15), integer(40)]);
    }
}
