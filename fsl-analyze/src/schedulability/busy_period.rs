// SPDX-License-Identifier: AGPL-3.0-or-later
//! The fixed-priority recurrence over the jobs of a task's level-`i` busy
//! period (ADR-026 RT-4), shared by fixed priority and both AMC modes.

use filament_exact::{Cancel, Rational};
use fsl_replay::{JobIterates, Task};

use super::arith::{ceil, int, quotient};
use super::{poll, Analysed};

/// One higher-priority task's term: `ceil((w + J) / T) * C`.
pub(super) struct Term<'a> {
    pub(super) period: &'a Rational,
    pub(super) jitter: &'a Rational,
    pub(super) budget: &'a Rational,
}

/// What a task's recurrence is built from. The window of job `q` is
/// `(q + 1) * own + blocking + constant + sum of its terms`.
pub(super) struct Level<'a> {
    pub(super) own: &'a Rational,
    pub(super) blocking: &'a Rational,
    pub(super) constant: Rational,
    pub(super) terms: Vec<Term<'a>>,
}

impl Level<'_> {
    fn start(&self, job: u64) -> Rational {
        int(job)
            .add(&int(1))
            .mul(self.own)
            .add(self.blocking)
            .add(&self.constant)
    }

    fn step(&self, job: u64, window: &Rational) -> Rational {
        self.terms.iter().fold(self.start(job), |sum, term| {
            let count = ceil(&quotient(&window.add(term.jitter), term.period));
            sum.add(&count.mul(term.budget))
        })
    }
}

/// How a task's jobs ended.
pub(super) enum Jobs {
    /// Every job of the busy period met `bound`.
    Met(Vec<JobIterates>),
    /// Job `job` passed `bound`, with the iterates up to the first that did.
    Missed { job: u64, iterates: Vec<Rational> },
}

/// Iterate each job of `task`'s busy period until the busy period closes or
/// a job's response time `w + J - q * T` passes `bound`.
pub(super) fn jobs(
    task: &Task,
    level: &Level<'_>,
    bound: &Rational,
    cancel: &Cancel,
) -> Analysed<Jobs> {
    let period = task.arrival.interval();
    let mut met = Vec::new();
    for job in 0_u64.. {
        let shift = task.jitter.sub(&int(job).mul(period));
        let mut window = level.start(job);
        let mut iterates = vec![window.clone()];
        loop {
            poll(cancel)?;
            if window.add(&shift) > *bound {
                return Ok(Jobs::Missed { job, iterates });
            }
            let next = level.step(job, &window);
            iterates.push(next.clone());
            if next == window {
                break;
            }
            window = next;
        }
        // The next job is released at (q + 1) * T - J; the busy period
        // stays open while this job completes after that.
        let open = window > int(job).add(&int(1)).mul(period).sub(&task.jitter);
        met.push(JobIterates(iterates));
        if !open {
            break;
        }
    }
    Ok(Jobs::Met(met))
}

#[cfg(test)]
mod tests {
    use filament_exact::{Cancel, Integer, Rational};
    use fsl_replay::{Arrival, Criticality, Identifier, Task, Wcet};
    use ix_trace_rs::trace;

    use super::{jobs, Jobs, Level, Term};
    use crate::schedulability::arith::int;

    fn integer(value: i64) -> Rational {
        Rational::from_integer(Integer::from(value))
    }

    fn task(name: &str, execution: i64, period: i64, deadline: i64, priority: u64) -> Task {
        Task {
            name: Identifier::new(name).expect("a task name"),
            arrival: Arrival::Periodic {
                period: integer(period),
            },
            deadline: integer(deadline),
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
    fn busy_period_oracle_keeps_the_next_job_until_the_window_closes() {
        let higher = task("a", 2, 4, 4, 1);
        let target = task("b", 5, 10, 12, 2);
        let level = Level {
            own: target.wcet.hi(),
            blocking: &target.blocking,
            constant: int(0),
            terms: vec![Term {
                period: higher.arrival.interval(),
                jitter: &higher.jitter,
                budget: higher.wcet.hi(),
            }],
        };
        let result = match jobs(&target, &level, &integer(12), &Cancel::new()) {
            Ok(result) => result,
            Err(_) => panic!("the busy-period oracle completes"),
        };
        let Jobs::Met(jobs) = result else {
            panic!("both jobs meet the declared response bound");
        };

        assert_eq!(jobs.len(), 2);
        assert_eq!(
            jobs[0].0,
            vec![integer(5), integer(9), integer(11), integer(11)]
        );
        assert_eq!(
            jobs[1].0,
            vec![
                integer(10),
                integer(16),
                integer(18),
                integer(20),
                integer(20)
            ]
        );
    }
}
