// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3444: the region route's delay move goes to the region's time
//! successor (FR-922), whatever representative the region was first reached by.

use fsl_analyze::analyze::AnalyzeRecord;
use fsl_replay::{ProofBasis, TerminalValue};

use super::{PROFILES, settle};
use crate::timed_world::{self, World};

fn outcome(record: &AnalyzeRecord) -> String {
    match record.terminal().map(|terminal| terminal.value()) {
        Some(TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        }) => format!(
            "proved {}/{}",
            value.value().numerator().as_big(),
            value.value().denominator().as_big()
        ),
        Some(TerminalValue::Refuted { .. }) => "refuted".to_owned(),
        Some(value) => format!("{value:?}"),
        None => format!("{record:?}"),
    }
}

/// `Max`, `Min` and `Half` over `eventually[0 ms, 4 ms] holds(c.late)`, every
/// scheduler. With dense extrema 0 and 1 all three are false.
fn observed(world: &World) -> [(&'static str, String); 3] {
    ["Max", "Min", "Half"].map(|claim| (claim, outcome(&settle(world, claim))))
}

fn all_refuted() -> [(&'static str, String); 3] {
    ["Max", "Min", "Half"].map(|claim| (claim, "refuted".to_owned()))
}

/// `first` resets `x` strictly inside (0, 1) ms while `y` runs on, and `third`
/// needs `x < 1 ms` and `y > 1 ms`.
fn two_clocks(invariant: &str) -> World {
    let claims = [("Max", "<= 0.999"), ("Min", ">= 0.001"), ("Half", "<= 0.5")]
        .map(|(name, bound)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
                 on origin {{ probability {bound} [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n"
            )
        })
        .concat();
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 and self.x > 0 ms and self.x < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.phase = 1 and self.x < 1 ms and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.phase = 2 and self.late = true }}\n\
         {invariant}\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(
            &["x", "y"],
            &[("first", &["phase", "x"]), ("third", &["phase", "late"])],
        ),
        PROFILES,
        &body,
    )
}

/// `first` at 0.5 ms and `third` at 1.2 ms meet both guards, so the maximum is
/// 1; a scheduler that never takes `third` gives the minimum 0. The region
/// holding such a `third` lies between the instant `x` leaves zero and the
/// instant `y` passes 1 ms.
#[test]
fn a_delay_reaches_the_region_between_two_clocks_integer_crossings() {
    let world = two_clocks("time invariant Bounded when { self.phase < 2 } { self.y <= 3 ms }\n");
    assert_eq!(observed(&world), all_refuted());
}

/// `first` runs before 1 ms and resets `x`, then `third` needs `y > 1 ms`
/// under `phase_one`, which keeps `x` short of 1 ms. Time cannot diverge
/// without `third`, so the claim that `late` is reached holds with value 1.
fn forced_third(third: &str, phase_one: &str) -> World {
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 and self.x > 0 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.phase = 1 and {third} and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.phase = 2 and self.late = true }}\n\
         time invariant Zeroth when {{ self.phase = 0 }} {{ self.y < 1 ms }}\n\
         time invariant Oneth when {{ self.phase = 1 }} {{ {phase_one} }}\n\
         probabilistic Reached using ev on M over (c: M::Call) under every scheduler on origin \
         {{ probability >= 0.5 [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(
            &["x", "y"],
            &[("first", &["phase", "x"]), ("third", &["phase", "late"])],
        ),
        PROFILES,
        &body,
    )
}

/// A strict invariant: the walk that skips a region lands on `x = 1 ms`, which
/// the invariant excludes, so the delay itself is lost and the state reads as
/// time-locked.
#[test]
fn a_strict_invariant_does_not_turn_the_skip_into_a_time_lock() {
    let world = forced_third("self.x < 1 ms", "self.x < 1 ms");
    assert_eq!(outcome(&settle(&world, "Reached")), "proved 1/1");
}

/// A closed invariant admits the point the walk lands on, so `third` at
/// `x <= 1 ms` is still reached.
#[test]
fn a_closed_invariant_keeps_its_last_instant() {
    let world = forced_third("self.x <= 1 ms", "self.x <= 1 ms");
    assert_eq!(outcome(&settle(&world, "Reached")), "proved 1/1");
}

/// The same two-clock subject with no time invariant, so that every state can
/// also wait forever (QSL-3443).
#[test]
fn the_two_clock_subject_with_idle_states_reaches_late() {
    assert_eq!(observed(&two_clocks("")), all_refuted());
}
