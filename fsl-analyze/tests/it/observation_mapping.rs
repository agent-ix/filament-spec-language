// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944-AC-1 (TC-2624): `ClockCover`'s `definitely_before` and
//! `within_bound`, which read the observation owner's `relation_to` on
//! FR-944's quarter-tick intervals, equal what the covers mean on every
//! reading and bound. The oracle here shares no code with the mapping: it
//! enumerates every pair of instants of the two covers on a lattice of
//! eighths of a tick, fine enough that an open end excludes its endpoint and
//! admits the instant one eighth inside it.

use fsl_eval::temporal::{ClockCover, Containment};
use ix_trace_rs::trace;

/// The instants a reading covers, in eighths of a tick.
fn instants(clock: ClockCover, reading: u64) -> Vec<i64> {
    let k = i64::try_from(reading).expect("a small reading");
    let (first, last) = match clock {
        ClockCover::Exact => (8 * k, 8 * k),
        ClockCover::Counter { uncertainty } => {
            let u = i64::try_from(uncertainty).expect("a small uncertainty");
            (8 * (k - u), 8 * (k + 1 + u) - 1)
        }
        ClockCover::Bounded { uncertainty } => {
            let u = i64::try_from(uncertainty).expect("a small uncertainty");
            (8 * (k - u), 8 * (k + u))
        }
    };
    (first..=last).collect()
}

/// Whether `later - earlier <= bound` holds for every pair, for none, or for
/// some.
fn oracle_within(clock: ClockCover, bound: i64, request: u64, ack: u64) -> Containment {
    let (mut holding, mut failing) = (false, false);
    for earlier in instants(clock, request) {
        for later in instants(clock, ack) {
            if later - earlier <= 8 * bound {
                holding = true;
            } else {
                failing = true;
            }
        }
    }
    match (holding, failing) {
        (true, false) => Containment::True,
        (false, true) => Containment::False,
        _ => Containment::Indeterminate,
    }
}

/// Whether every instant of `a` lies strictly below every instant of `b`.
fn oracle_before(clock: ClockCover, a: u64, b: u64) -> bool {
    let right = instants(clock, b);
    instants(clock, a)
        .iter()
        .all(|left| right.iter().all(|right| left < right))
}

#[test]
#[trace("TC-2624", "FR-944-AC-1")]
fn the_owner_order_of_covers_equals_the_pairwise_meaning_of_the_covers() {
    let covers = [
        ClockCover::Exact,
        ClockCover::Counter { uncertainty: 0 },
        ClockCover::Counter { uncertainty: 1 },
        ClockCover::Counter { uncertainty: 2 },
        ClockCover::Bounded { uncertainty: 0 },
        ClockCover::Bounded { uncertainty: 1 },
        ClockCover::Bounded { uncertainty: 2 },
    ];
    for clock in covers {
        for a in 0..=6u64 {
            for b in 0..=6u64 {
                assert_eq!(
                    clock.definitely_before(a, b),
                    oracle_before(clock, a, b),
                    "{clock:?} definitely_before({a}, {b})"
                );
                for bound in -1..=7i64 {
                    assert_eq!(
                        clock.within_bound(bound, a, b),
                        oracle_within(clock, bound, a, b),
                        "{clock:?} within_bound({bound}, {a}, {b})"
                    );
                }
            }
        }
    }
}
