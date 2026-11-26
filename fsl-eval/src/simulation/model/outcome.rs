// SPDX-License-Identifier: AGPL-3.0-or-later
//! The outcome layer of timed items (ADR-026 TD-1, TD-6 and TV-1): the
//! order a would-be proof is classified in, and what a time-lock does to a
//! refuting reading. Each route finds divergence and fairness on its own
//! abstraction and hands the answer here; no product, zone or integer-clock
//! code lives here.
//!
//! The instance set of a claim's `over` parameter is
//! [`ModelSystem::claim_instances`](super::ModelSystem::claim_instances);
//! [`claim_start`] places its causes among a timed item's, in FR-125
//! "Instancing" order.

use fsl_semantics::check::{CheckedTemporalClause, TimedPropertyForm};
use ix_trace_rs::implements;

use crate::automaton::invariant_atom;

use super::{ClaimInstances, NoClaimInstances, NotSimulated};

/// Whether a result of `clause` states the non-Zeno premise (FSpec FR-416
/// "Non-Zeno premise"): a TT-1 or TT-2 claim states none, whatever product
/// decides it.
#[implements("FSpec-FR-416-AC-10")]
pub fn states_non_zeno(clause: &CheckedTemporalClause) -> bool {
    invariant_atom(clause).is_none()
        && !matches!(
            clause.timed_form(),
            Some(TimedPropertyForm::TimedInvariant | TimedPropertyForm::BoundedWindow { .. })
        )
}

/// Why a timed item that found no violation is not proved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimedVacuity {
    /// The subject has no initial state.
    NoInitialState,
    /// The claim's `over` type admits no key.
    NoInstance,
    /// No initial state has a fair time-divergent behaviour, so a proof
    /// would be vacuous.
    NoAdmittedBehaviour,
}

/// Start a timed claim over its instance set: `initial_states` is the timed
/// subject's, `instances` what
/// [`ModelSystem::claim_instances`](super::ModelSystem::claim_instances)
/// returned for the claim's `over` type. In order: the extent refusal, then
/// no initial state, then no instance; otherwise the instances.
///
/// # Errors
///
/// [`NotSimulated`] where the instance set has no bounded extent.
#[implements("FR-125-AC-7", "FR-125-AC-13")]
pub fn claim_start(
    initial_states: usize,
    instances: Result<ClaimInstances, NoClaimInstances>,
) -> Result<Result<ClaimInstances, TimedVacuity>, NotSimulated> {
    match instances {
        Err(NoClaimInstances::NotSimulated(refusal)) => Err(refusal),
        Err(NoClaimInstances::NoInitialState) => Ok(Err(TimedVacuity::NoInitialState)),
        Err(NoClaimInstances::NoInstance) | Ok(_) if initial_states == 0 => {
            Ok(Err(TimedVacuity::NoInitialState))
        }
        Err(NoClaimInstances::NoInstance) => Ok(Err(TimedVacuity::NoInstance)),
        Ok(instances) => Ok(Ok(instances)),
    }
}

/// Classify the start of a would-be proof, in order: a subject with no
/// initial state is [`TimedVacuity::NoInitialState`], whatever else is true
/// of it; one whose `admitted` search finds no behaviour is
/// [`TimedVacuity::NoAdmittedBehaviour`]; otherwise the evidence the search
/// returned. `admitted` is not run with no initial state.
///
/// A violation is not classified here: one on an admitted behaviour refutes
/// ([`admitted_refutation`]), and a derived time-lock-freedom item reports a
/// time-lock before it asks this.
///
/// # Errors
///
/// What `admitted` stops with.
#[implements("FR-125-AC-13", "FR-232-AC-3", "FR-240-AC-4")]
pub fn admitted_start<W, E>(
    initial_states: usize,
    admitted: impl FnOnce() -> Result<Option<W>, E>,
) -> Result<Result<W, TimedVacuity>, E> {
    if initial_states == 0 {
        return Ok(Err(TimedVacuity::NoInitialState));
    }
    Ok(admitted()?.ok_or(TimedVacuity::NoAdmittedBehaviour))
}

/// The time-lock rule for a false, undefined or short reading: it counts
/// only where `diverges`, the route's own finding that a time-divergent
/// behaviour passes through the state, holds. On a time-locked state the
/// reading is excluded (`None`) and the search goes on through every other
/// branch.
///
/// # Errors
///
/// What `refutation`, run only for a reading that counts, stops with.
#[implements("FR-239-AC-5")]
pub fn admitted_refutation<R, E>(
    diverges: bool,
    refutation: impl FnOnce() -> Result<R, E>,
) -> Result<Option<R>, E> {
    diverges.then(refutation).transpose()
}
