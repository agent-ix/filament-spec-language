// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-7, the closed-form schedulability engine (ADR-026 RT-3 to RT-7,
//! FR-247).
//!
//! [`analyze`] decides a [`ScheduleSubject`] in exact rationals: fixed-priority
//! response-time analysis with the busy-period extension, the EDF
//! processor-demand test with Quick Processor-demand Analysis, and AMC-rtb.
//! Each fixed-priority iteration stops at its fixpoint or as soon as its
//! response time passes its bound, so it needs no iteration cap. The result
//! is a pure function of the subject and the limits; the checker in
//! `fsl-replay` recomputes its evidence with its own code (ADR-026 CF-6).

mod amc;
mod arith;
mod busy_period;
mod edf;
mod fixed_priority;
pub mod manifest;

use filament_exact::{Cancel, CancelCause};
use fsl_replay::{
    ClosedFormOutcome, IncompleteCause, ModelCheckLimit, ScheduleClaim, ScheduleSubject,
    SchedulingPolicy,
};

/// FR-247's published default for `max_demand_points`: 2^20.
pub(crate) const DEFAULT_MAX_DEMAND_POINTS: u64 = 1_048_576;

/// EN-7's caller-set limits (FR-247).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleLimits {
    /// How many demand points QPA may check and how many busy-period and
    /// fixed-priority window iterations the recurrences may take: each costs one
    /// unit.
    pub max_demand_points: u64,
}

impl Default for ScheduleLimits {
    fn default() -> Self {
        Self {
            max_demand_points: DEFAULT_MAX_DEMAND_POINTS,
        }
    }
}

/// Why an analysis stopped before it decided.
enum Stop {
    Cancelled(CancelCause),
    DemandPoints,
}

type Analysed<T> = Result<T, Stop>;

/// Cancellation and the one count every recurrence step charges: a QPA demand
/// point, a busy-period window iteration or a fixed-priority window iteration
/// each cost one unit of `max_demand_points` (FR-247-AC-5).
struct Pace<'a> {
    cancel: &'a Cancel,
    limit: u64,
    used: u64,
}

impl<'a> Pace<'a> {
    fn new(cancel: &'a Cancel, limit: u64) -> Self {
        Self {
            cancel,
            limit,
            used: 0,
        }
    }

    /// Poll cancellation, then charge one unit before the step it pays for.
    fn step(&mut self) -> Analysed<()> {
        if self.cancel.poll() {
            if let Some(cause) = self.cancel.cause() {
                return Err(Stop::Cancelled(cause));
            }
        }
        if self.used >= self.limit {
            return Err(Stop::DemandPoints);
        }
        self.used += 1;
        Ok(())
    }
}

/// Decide `subject` under `limits`, polling `cancel` as it iterates.
pub fn analyze(
    subject: &ScheduleSubject,
    limits: &ScheduleLimits,
    cancel: &Cancel,
) -> ClosedFormOutcome {
    let mut pace = Pace::new(cancel, limits.max_demand_points);
    let decided = match subject.claim() {
        ScheduleClaim::Schedulable(SchedulingPolicy::Edf) => edf::analyse(subject, &mut pace),
        ScheduleClaim::Schedulable(SchedulingPolicy::Amc) => amc::analyse(subject, &mut pace),
        ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority)
        | ScheduleClaim::Response { .. } => fixed_priority::analyse(subject, &mut pace),
    };
    decided.unwrap_or_else(|stop| {
        ClosedFormOutcome::Stopped(match stop {
            Stop::Cancelled(source) => IncompleteCause::Cancelled { source },
            Stop::DemandPoints => IncompleteCause::LimitReached {
                limit: ModelCheckLimit::ScheduleMaxDemandPoints,
                value: limits.max_demand_points,
                required: None,
            },
        })
    })
}
