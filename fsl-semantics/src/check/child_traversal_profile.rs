// SPDX-License-Identifier: AGPL-3.0-or-later
//! Feature-gated dynamic accounting for checked-node child iteration.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChildTraversalProfile {
    /// Number of child iterators constructed during the profiled evaluation.
    pub iterator_count: u64,
    /// Number of child items yielded during the profiled evaluation.
    pub child_count: u64,
    /// Allocations made while constructing or advancing those iterators.
    pub allocation_count: u64,
}

thread_local! {
    static ACTIVE: Cell<Option<ChildTraversalProfile>> = const { Cell::new(None) };
}

/// Start dynamic child-traversal accounting on the current thread.
pub fn begin_child_traversal_profile() {
    ACTIVE.with(|active| active.set(Some(ChildTraversalProfile::default())));
}

/// Finish dynamic child-traversal accounting on the current thread.
pub fn end_child_traversal_profile() -> ChildTraversalProfile {
    ACTIVE.with(|active| active.take().unwrap_or_default())
}

pub(crate) fn measure_child<T>(iterator_construction: bool, operation: impl FnOnce() -> T) -> T {
    let active = ACTIVE.with(|active| {
        if let Some(mut profile) = active.get() {
            if iterator_construction {
                profile.iterator_count += 1;
            }
            active.set(Some(profile));
            true
        } else {
            false
        }
    });
    if !active {
        return operation();
    }

    let (value, allocations) = allocation_counter::measure(operation);
    ACTIVE.with(|active| {
        if let Some(mut profile) = active.get() {
            profile.allocation_count += allocations.count_total;
            active.set(Some(profile));
        }
    });
    value
}

pub(crate) fn note_child_yielded() {
    ACTIVE.with(|active| {
        if let Some(mut profile) = active.get() {
            profile.child_count += 1;
            active.set(Some(profile));
        }
    });
}
