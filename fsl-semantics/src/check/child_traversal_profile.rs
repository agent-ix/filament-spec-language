// SPDX-License-Identifier: AGPL-3.0-or-later
//! Feature-gated dynamic accounting for checked-node child iteration.

use std::cell::Cell;
use ix_trace_rs::implements;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[implements("NFR-141-AC-7")]
/// Dynamic counters for one evaluation's checked-node child traversal.
pub struct ChildTraversalProfile {
    /// Number of child iterators constructed during the profiled evaluation.
    pub iterator_count: u64,
    /// Number of child items yielded during the profiled evaluation.
    pub child_count: u64,
    /// Allocations made while constructing or advancing those iterators.
    pub allocation_count: u64,
}

thread_local! {
    static ACTIVE: Cell<Option<(ChildTraversalProfile, AllocatorMeasure)>> = const { Cell::new(None) };
}

/// Callback that measures allocations made by one child-iteration operation.
pub type AllocatorMeasure = fn(&mut dyn FnMut()) -> u64;

/// Start dynamic child-traversal accounting on the current thread.
#[implements("NFR-141-AC-7")]
pub fn begin_child_traversal_profile(measure: AllocatorMeasure) {
    ACTIVE.with(|active| active.set(Some((ChildTraversalProfile::default(), measure))));
}

/// Finish dynamic child-traversal accounting on the current thread.
#[implements("NFR-141-AC-7")]
pub fn end_child_traversal_profile() -> ChildTraversalProfile {
    ACTIVE.with(|active| {
        active
            .take()
            .map_or_else(ChildTraversalProfile::default, |(profile, _)| profile)
    })
}

#[implements("NFR-141-AC-7")]
pub(crate) fn measure_child<T>(iterator_construction: bool, operation: impl FnOnce() -> T) -> T {
    let measure = ACTIVE.with(|active| {
        if let Some((mut profile, measure)) = active.get() {
            if iterator_construction {
                profile.iterator_count += 1;
            }
            active.set(Some((profile, measure)));
            Some(measure)
        } else {
            None
        }
    });
    let Some(measure) = measure else {
        return operation();
    };

    let mut value = None;
    let mut pending = Some(operation);
    let mut invoke = || {
        if let Some(operation) = pending.take() {
            value = Some(operation());
        }
    };
    let allocation_count = measure(&mut invoke);
    ACTIVE.with(|active| {
        if let Some((mut profile, measure)) = active.get() {
            profile.allocation_count += allocation_count;
            active.set(Some((profile, measure)));
        }
    });
    match value {
        Some(value) => value,
        None => unreachable!("the measured child operation always returns"),
    }
}

#[implements("NFR-141-AC-7")]
pub(crate) fn note_child_yielded() {
    note_children_yielded(1);
}

#[implements("NFR-141-AC-7")]
pub(crate) fn note_children_yielded(count: usize) {
    ACTIVE.with(|active| {
        if let Some((mut profile, measure)) = active.get() {
            profile.child_count += u64::try_from(count).unwrap_or(u64::MAX);
            active.set(Some((profile, measure)));
        }
    });
}
