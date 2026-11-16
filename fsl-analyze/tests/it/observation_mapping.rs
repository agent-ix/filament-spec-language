// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944-AC-1 (TC-2624): FR-501's counter and exact covers map onto the
//! observation owner's closed event-time intervals in half-ticks, and the
//! owner's `relation_to` then gives `ClockCover`'s answers on every pair.

use filament_observation::authority::partial::EventTimeInterval;
use filament_observation::authority::Limits;
use filament_observation::Identity;
use fsl_eval::temporal::{ClockCover, Containment};
use ix_trace_rs::trace;

/// FR-944 "Partial values and interval order": a reading's cover as a closed
/// interval in half-ticks, shifted by `shift` half-ticks.
fn cover(clock: ClockCover, reading: u64, shift: i128) -> EventTimeInterval {
    let k = i128::from(reading);
    let (earliest, latest) = match clock {
        ClockCover::Exact => (2 * k, 2 * k),
        ClockCover::Counter { uncertainty } => {
            let u = i128::from(uncertainty);
            (2 * (k - u), 2 * (k + 1 + u) - 1)
        }
    };
    EventTimeInterval::new(
        Identity::new("clock:counter"),
        Identity::new("profile:finite-window"),
        Identity::new("tick/2"),
        earliest + shift,
        latest + shift,
        Limits::owner_max(),
    )
    .expect("a valid interval")
}

#[test]
#[trace("TC-2624", "FR-944-AC-1")]
#[ignore = "QSL-2145: fsl-eval does not take its interval order and bound comparison from the observation owner yet"]
fn half_tick_intervals_give_the_counter_cover_answers() {
    let limits = Limits::owner_max();
    for clock in [ClockCover::Exact, ClockCover::Counter { uncertainty: 1 }] {
        for a in 0..=6u64 {
            for b in 0..=6u64 {
                let order = cover(clock, a, 0)
                    .relation_to(&cover(clock, b, 0), limits)
                    .expect("one domain");
                assert_eq!(
                    order.definitely_before(),
                    clock.definitely_before(a, b),
                    "{clock:?} definitely_before({a}, {b})"
                );
                for bound in 0..=6i64 {
                    let shifted = cover(clock, a, 2 * i128::from(bound));
                    let order = cover(clock, b, 0)
                        .relation_to(&shifted, limits)
                        .expect("one domain");
                    let owner = if !order.after() {
                        Containment::True
                    } else if !order.before() && !order.equal() {
                        Containment::False
                    } else {
                        Containment::Indeterminate
                    };
                    assert_eq!(
                        owner,
                        clock.within_bound(bound, a, b),
                        "{clock:?} within_bound({bound}, {a}, {b})"
                    );
                }
            }
        }
    }
}
