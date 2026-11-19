// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EDF processor-demand test with Quick Processor-demand Analysis
//! (ADR-026 RT-5; Baruah, Rosier and Howell; Zhang and Burns).

use filament_exact::{Cancel, Rational};
use fsl_replay::{
    ClosedFormEvidence, ClosedFormOutcome, DemandWitness, MissEvidence, ScheduleSubject,
};

use super::arith::{ceil, floor, int, quotient};
use super::{poll, Analysed, ScheduleLimits, Stop};

fn utilization(subject: &ScheduleSubject) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        sum.add(&quotient(task.wcet.hi(), task.arrival.interval()))
    })
}

/// `dbf(t) = sum of max(0, floor((t - D) / T) + 1) * C`.
fn demand_bound(subject: &ScheduleSubject, t: &Rational) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        let jobs = floor(&quotient(&t.sub(&task.deadline), task.arrival.interval())).add(&int(1));
        if jobs > int(0) {
            sum.add(&jobs.mul(task.wcet.hi()))
        } else {
            sum
        }
    })
}

/// The synchronous busy period: the least `w` with
/// `w = sum of ceil(w / T) * C`, which exists when utilization is at most 1.
fn busy_period(subject: &ScheduleSubject, cancel: &Cancel) -> Analysed<Rational> {
    let tasks = subject.task_set().tasks();
    let mut window = tasks
        .iter()
        .fold(int(0), |sum, task| sum.add(task.wcet.hi()));
    loop {
        poll(cancel)?;
        let next = tasks.iter().fold(int(0), |sum, task| {
            let count = ceil(&quotient(&window, task.arrival.interval()));
            sum.add(&count.mul(task.wcet.hi()))
        });
        if next == window {
            return Ok(window);
        }
        window = next;
    }
}

/// The greatest absolute deadline strictly below `t`.
fn deadline_below(subject: &ScheduleSubject, t: &Rational) -> Option<Rational> {
    subject
        .task_set()
        .tasks()
        .iter()
        .filter(|task| *t > task.deadline)
        .map(|task| {
            let before =
                ceil(&quotient(&t.sub(&task.deadline), task.arrival.interval())).sub(&int(1));
            task.deadline.add(&before.mul(task.arrival.interval()))
        })
        .max()
}

pub(super) fn analyse(
    subject: &ScheduleSubject,
    limits: &ScheduleLimits,
    cancel: &Cancel,
) -> Analysed<ClosedFormOutcome> {
    let load = utilization(subject);
    if load > int(1) {
        return Ok(ClosedFormOutcome::Miss(MissEvidence::Utilization(load)));
    }
    let length = busy_period(subject, cancel)?;
    let shortest = subject
        .task_set()
        .tasks()
        .iter()
        .map(|task| task.deadline.clone())
        .min();
    let mut sequence: Vec<Rational> = Vec::new();
    let mut next = deadline_below(subject, &length);
    while let (Some(t), Some(shortest)) = (next.take(), shortest.as_ref()) {
        poll(cancel)?;
        let checked = u64::try_from(sequence.len()).map_err(|_| Stop::DemandPoints)?;
        if checked >= limits.max_demand_points {
            return Err(Stop::DemandPoints);
        }
        let demand = demand_bound(subject, &t);
        if demand > t {
            return Ok(ClosedFormOutcome::Miss(MissEvidence::Edf(DemandWitness {
                t,
                demand,
            })));
        }
        sequence.push(t.clone());
        if demand > *shortest {
            next = if demand < t {
                Some(demand)
            } else {
                deadline_below(subject, &t)
            };
        }
    }
    Ok(ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: length,
        qpa_sequence: sequence,
    }))
}

#[cfg(test)]
mod tests {
    use filament_exact::{Cancel, Integer, Rational};
    use fsl_replay::{
        Arrival, ClosedFormOutcome, Criticality, Identifier, MissEvidence, QualifiedName,
        ScheduleClaim, ScheduleLimits, ScheduleSubject, SchedulingPolicy, Task, TaskSet, Wcet,
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

    /// Trace: TC-702, FR-247-AC-2.
    #[trace("TC-702", "FR-247-AC-2")]
    #[test]
    fn edf_oracle_refutes_utilization_above_one() {
        let set = TaskSet::new(
            QualifiedName::new(vec![Identifier::new("Ctl").expect("a set name")])
                .expect("one name segment"),
            vec![task("a", 1, 4, 1), task("b", 2, 6, 2), task("c", 6, 12, 3)],
        )
        .expect("an admitted task set");
        let subject = ScheduleSubject::new(set, ScheduleClaim::Schedulable(SchedulingPolicy::Edf))
            .expect("an admitted EDF claim");
        let outcome = match analyse(&subject, &ScheduleLimits::default(), &Cancel::new()) {
            Ok(outcome) => outcome,
            Err(_) => panic!("the EDF utilization oracle completes"),
        };

        assert_eq!(
            outcome,
            ClosedFormOutcome::Miss(MissEvidence::Utilization(
                Rational::new(Integer::from(13), Integer::from(12)).expect("a nonzero divisor")
            ))
        );
    }
}
