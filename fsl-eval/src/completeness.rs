// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-861's tracked exception (QSL-2327): the one completeness revision rule
//! `fsl-eval` holds itself, until `filament-observation` is pinned at a
//! release that carries its `completeness::revise` (FR-013).

use crate::temporal::{CoveredList, ObservedPosition, ProgressCompleteness};

/// Why a `Complete { through }` assertion contradicts the retained one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompleteThroughRefusal {
    /// The retained assertion was `Complete` and the new one retreats: an
    /// `Incomplete`, or a `Complete` through an earlier tick.
    Retreat,
    /// A position stamped at or before the retained boundary that the
    /// retained covered set does not hold.
    UncoveredPosition(Box<ObservedPosition>),
}

/// Compare a new completeness assertion with the retained one (FR-502,
/// FR-944 "Owner requirement: completeness revision"): after `Complete {
/// through }`, a later `Complete` through the same or a later tick is
/// accepted; an `Incomplete`, a `Complete` through an earlier tick and a
/// position at or before the boundary that `covered` does not hold are
/// refused. A retained `Incomplete` refuses nothing.
///
/// This is QSL-2327's local copy of the owner's `completeness::revise`
/// boundary and coverage comparison; it is deleted when the owner's rule is
/// called instead.
pub fn revise_complete_through(
    retained: ProgressCompleteness,
    asserted: ProgressCompleteness,
    covered: &CoveredList,
    positions: &[ObservedPosition],
) -> Result<(), CompleteThroughRefusal> {
    let ProgressCompleteness::Complete { through } = retained else {
        return Ok(());
    };
    let retreats = match asserted {
        ProgressCompleteness::Complete { through: asserted } => asserted < through,
        ProgressCompleteness::Incomplete => true,
    };
    if retreats {
        return Err(CompleteThroughRefusal::Retreat);
    }
    match positions
        .iter()
        .find(|position| position.stamp <= through && !covered.contains(position))
    {
        Some(position) => Err(CompleteThroughRefusal::UncoveredPosition(Box::new(
            position.clone(),
        ))),
        None => Ok(()),
    }
}
