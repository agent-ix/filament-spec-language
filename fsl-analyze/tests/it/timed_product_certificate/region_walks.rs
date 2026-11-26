// SPDX-License-Identifier: AGPL-3.0-or-later
//! Red tests for the walks beside the QSL-3444 delay move: expected elapsed
//! and a nested formula over the region route, and `always` at a time-lock.

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
        Some(TerminalValue::Proved { .. }) => "proved".to_owned(),
        Some(TerminalValue::Refuted { .. }) => "refuted".to_owned(),
        Some(value) => format!("{value:?}"),
        None => format!("no terminal: {:?}", record.settlement),
    }
}

/// `first` runs before `y` reaches 1 ms and resets `x`; `third` needs
/// `y > 1 ms` while `x < 1 ms`. Time diverges only through `third`, so `late`
/// is reached on every behaviour, at an instant in (1, 2) ms.
fn forced(claims: &str) -> World {
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 and self.x > 0 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.phase = 1 and self.x < 1 ms and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.phase = 2 and self.late = true }}\n\
         time invariant Zeroth when {{ self.phase = 0 }} {{ self.y < 1 ms }}\n\
         time invariant Oneth when {{ self.phase = 1 }} {{ self.x < 1 ms }}\n\
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

/// The time to `late` has infimum 1 ms and supremum 2 ms over every scheduler.
#[test]
fn expected_elapsed_over_two_clock_regions() {
    let claims = [
        ("High", "<= 5 ms"),
        ("Low", ">= 0.1 ms"),
        ("Under", "<= 1.5 ms"),
    ]
    .map(|(name, bound)| {
        format!(
            "probabilistic {name} using inf on M over (c: M::Call) under every scheduler \
                 on origin {{ expected elapsed until holds(c.late) {bound} }}\n"
        )
    })
    .concat();
    let world = forced(&claims);
    let observed = ["High", "Low", "Under"].map(|name| outcome(&settle(&world, name)));
    assert!(observed[0].starts_with("proved"), "{}", observed[0]);
    assert!(observed[1].starts_with("proved"), "{}", observed[1]);
    assert_eq!(observed[2], "refuted");
}

/// Every behaviour reaches `late` inside the window and it stays true, so the
/// minimum of the nested formula is 1.
#[test]
fn nested_formula_over_two_clock_regions() {
    let claim = "probabilistic Min using ev on M over (c: M::Call) under every scheduler \
                 on origin { probability >= 0.001 [ eventually[0 ms, 4 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let world = forced(claim);
    let observed = outcome(&settle(&world, "Min"));
    assert!(observed.starts_with("proved"), "{observed}");
}

/// No operation ever runs, time stops at 1 ms and `late` stays false, so
/// `always ... holds(not late)` has value 1 (FR-204 "Form shapes": the
/// complement of `eventually ... holds(late)`).
fn time_lock(invariant: &str) -> [String; 2] {
    let claims = [("Early", "0 ms, 2 ms"), ("Later", "2 ms, 3 ms")]
        .map(|(name, window)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
                 on origin {{ probability >= 0.5 [ always[{window}] holds(not c.late) ] }}\n"
            )
        })
        .concat();
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ self.late and not self.late }}\n\
         post Stepped using v on M::Call::step {{ self.late = true }}\n\
         time invariant Stop when {{ true }} {{ {invariant} }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(
        &timed_world::document(&["x"], &[("step", &["late"])]),
        PROFILES,
        &body,
    );
    ["Early", "Later"].map(|name| outcome(&settle(&world, name)))
}

#[test]
fn always_at_a_time_lock_on_digital_clocks() {
    assert_eq!(time_lock("self.x <= 1 ms"), ["proved 1/1", "proved 1/1"]);
}

#[test]
fn always_at_a_time_lock_on_clock_regions() {
    assert_eq!(time_lock("self.x < 1 ms"), ["proved 1/1", "proved 1/1"]);
}
