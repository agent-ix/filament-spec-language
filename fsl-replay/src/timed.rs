// SPDX-License-Identifier: AGPL-3.0-or-later
//! The core's own timed machinery (ADR-026 CF-3): the fresh exploration
//! FR-237's replay runs, apart from EN-6's, over the certificate checker's
//! zones.

pub(crate) mod explore;

use fsl_eval::simulation::model::TimedSystem;
use fsl_semantics::check::{ClockConstraint, ConvexPart};
use ix_trace_rs::implements;

/// Every atomic clock constraint the model compares a clock with: each guard
/// part's, those of each quantifier family below it (FR-230 "Family parts"),
/// and each time invariant's bounds. The core's own walk, apart from every
/// engine's (ADR-026 CF-6).
#[implements("FR-245-AC-2")]
pub(crate) fn clock_constraints<'a>(timed: &'a TimedSystem<'_, '_>) -> Vec<&'a ClockConstraint> {
    let model = timed.model();
    let mut parts: Vec<&ConvexPart> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .collect();
    let mut found = Vec::new();
    let mut next = 0;
    while let Some(part) = parts.get(next).copied() {
        next += 1;
        found.extend(part.clocks());
        parts.extend(part.families().iter().flat_map(|family| family.parts()));
    }
    found.extend(
        model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds()),
    );
    found
}
