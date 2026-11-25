// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared read-only independence checks for protocol integration targets.

use std::collections::BTreeSet;

use fsl_eval::simulation::independent;
use fsl_eval::simulation::protocol::{MemoryModel, ProtocolSystem, ProtocolTransition, StepKind};

use crate::fill::Reached;
/// The states both orders of the steps `first` then `second` reach from
/// `reached[at]`: every successor of `first`, then every successor of
/// `second` from there.
fn after_both<M: MemoryModel>(
    reached: &[Reached<M>],
    at: usize,
    first: &ProtocolTransition,
    second: &ProtocolTransition,
) -> BTreeSet<usize> {
    reached[at]
        .steps
        .iter()
        .filter(|(step, _)| step == first)
        .flat_map(|(_, next)| {
            reached[*next]
                .steps
                .iter()
                .filter(|(step, _)| step == second)
                .map(|(_, last)| *last)
        })
        .collect()
}

/// ADR-027 FT-4 (ADR-021 POR-4): at every reachable state of `system`,
/// every pair of enabled steps reported independent commutes: each stays
/// enabled after the other, and both orders reach the same states. The
/// number of independent pairs checked.
#[allow(dead_code, reason = "only the refinement binary calls this wrapper")]
pub(crate) fn assert_independent_steps_commute<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    reached: &[Reached<M>],
) -> usize {
    commuting_pairs(system, reached).len()
}

/// [`assert_independent_steps_commute`]'s check, with the kinds of each
/// independent pair it checked.
pub(crate) fn commuting_pairs<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    reached: &[Reached<M>],
) -> Vec<(StepKind, StepKind)> {
    let mut checked = Vec::new();
    for (at, state) in reached.iter().enumerate() {
        let mut identities: Vec<&ProtocolTransition> = Vec::new();
        for (step, _) in &state.steps {
            if !identities.contains(&step) {
                identities.push(step);
            }
        }
        // A footprint is read once per step, not once per pair it is in, and the
        // state's access classification once for all of them.
        let footprints = system
            .footprints(&state.state, &identities)
            .expect("footprints");
        for (index, first) in identities.iter().enumerate() {
            for (offset, second) in identities[index + 1..].iter().enumerate() {
                let left = &footprints[index];
                let right = &footprints[index + 1 + offset];
                if !independent(left, right) {
                    continue;
                }
                checked.push((first.kind(), second.kind()));
                let forward = after_both(reached, at, first, second);
                let backward = after_both(reached, at, second, first);
                assert!(
                    !forward.is_empty() && forward == backward,
                    "from state {at}, {first:?} then {second:?} reach {forward:?}, \
                     the other order {backward:?}"
                );
            }
        }
    }
    checked
}
