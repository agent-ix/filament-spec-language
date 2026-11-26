// SPDX-License-Identifier: AGPL-3.0-or-later
//! The outcome layer of timed items (ADR-026 TD-1, TD-6 and TV-1): the
//! order a would-be proof is classified in, and what a time-lock does to a
//! refuting reading. Each route finds divergence and fairness on its own
//! abstraction and hands the answer here; no product, zone or integer-clock
//! code lives here.
//!
//! The instancing causes that come first (FR-125 "Instancing": requires
//! bound, no initial state, refused identity, no instance) belong to the
//! instance set of a claim's `over` parameter. [`TimedVacuity`] holds what is
//! left for a run that reads its subject: the two reasons a search that found
//! no violation is still not a proof.

use ix_trace_rs::implements;

/// Why a timed item that found no violation is not proved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimedVacuity {
    /// The subject has no initial state.
    NoInitialState,
    /// No initial state has a fair time-divergent behaviour, so a proof
    /// would be vacuous.
    NoAdmittedBehaviour,
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
