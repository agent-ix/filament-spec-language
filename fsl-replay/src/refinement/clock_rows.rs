// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": the abstract clock behaviour of one concrete edge
//! over a timed abstract subject whose hidden candidates each took their own
//! step. Replay and the zone-certificate checker read it here; the product
//! applies the same rule with its own code in fsl-analyze. The edge's rows
//! must agree on one guarded step.

use fsl_eval::simulation::model::{GuardedStep, ModelSystem, TimedSystem};
use fsl_semantics::model::state::ModelState;

use crate::refinement::failure::CandidateStep;

/// What an edge's candidate rows ask of the abstract clocks.
#[derive(Debug)]
pub enum RowClockStep {
    /// Every row is a candidate-local stutter: no guard, carry or reset.
    Stutter,
    /// Every row takes a step with this guard, carries and resets.
    Agreed(Box<GuardedStep>),
    /// The rows differ in guard, carries or resets, mix stutter with a step,
    /// or name a step no candidate has: the single abstract clock valuation
    /// cannot follow them, so nothing is concluded from the guard.
    Disagree,
}

/// Why the rows could not be read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowClockError {
    /// A candidate's state has no canonical key.
    Key,
    /// The timed subject could not expand a candidate.
    Expansion,
}

/// Read `rows` against the candidates they start from: each row's step is the
/// guarded step of the candidate whose key is the row's `pre_state`, with
/// exactly the row's transition.
///
/// # Errors
///
/// [`RowClockError`] when a candidate has no key or cannot be expanded.
pub fn row_clock_step(
    abstract_system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    candidates: &[ModelState],
    rows: &[CandidateStep],
) -> Result<RowClockStep, RowClockError> {
    let mut chosen: Option<GuardedStep> = None;
    let mut stutters = false;
    for row in rows {
        let Some(transition) = &row.transition else {
            stutters = true;
            continue;
        };
        let mut found = None;
        for candidate in candidates {
            let key = abstract_system
                .state_key_of(candidate)
                .map_err(|_| RowClockError::Key)?
                .0;
            if key != row.pre_state {
                continue;
            }
            let state = timed
                .system()
                .pre_state(candidate.clone(), timed.system().declared_anchor());
            found = timed
                .guarded_steps(&state)
                .map_err(|_| RowClockError::Expansion)?
                .into_iter()
                .find(|step| step.transition() == transition);
            break;
        }
        let Some(step) = found else {
            return Ok(RowClockStep::Disagree);
        };
        match &chosen {
            None => chosen = Some(step),
            Some(first)
                if first.parts() == step.parts()
                    && first.carries() == step.carries()
                    && first.resets() == step.resets() => {}
            Some(_) => return Ok(RowClockStep::Disagree),
        }
    }
    Ok(match chosen {
        Some(_) if stutters => RowClockStep::Disagree,
        Some(step) => RowClockStep::Agreed(Box::new(step)),
        None => RowClockStep::Stutter,
    })
}

/// How the abstract time invariants of a position's candidates read at one
/// clock valuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvariantReading {
    /// Every candidate's invariant holds.
    Holds,
    /// Every candidate's invariant fails: no candidate run goes on.
    Fails,
    /// Some candidate's invariant holds and some fails.
    Mixed,
}

impl InvariantReading {
    /// The reading of `failing` of `total` candidates failing.
    pub fn of(failing: usize, total: usize) -> Self {
        if failing == 0 {
            Self::Holds
        } else if failing == total {
            Self::Fails
        } else {
            Self::Mixed
        }
    }
}
