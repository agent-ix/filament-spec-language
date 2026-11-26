// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3443: the region product has the idle tail of a quiescent state, so a
//! stutter position can fall in any later region (FR-921, FR-922).

use fsl_analyze::analyze::AnalyzeRecord;
use fsl_replay::{ProofBasis, TerminalValue};
use serde_json::json;

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

/// `step` is enabled by `guard`, sets `late`, and leaves the subject
/// quiescent. `c.retried` holds throughout.
fn one_step(guard: &str, claims: &[(&str, &str, &str)]) -> World {
    let claims = claims
        .iter()
        .map(|(name, bound, formula)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
                 on origin {{ probability {bound} [ {formula} ] }}\n"
            )
        })
        .collect::<String>();
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ {guard} and not self.late }}\n\
         post Stepped using v on M::Call::step {{ self.late = true }}\n\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world_holding(
        &timed_world::document(&["x"], &[("step", &["late", "x"])]),
        PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    )
}

/// Nothing sets `retried` false, so a claim that waits for it to be seen in
/// [1, 2] ms has every behaviour of a quiescent state still undecided, and the
/// certificate over those states is accepted with value 0.
#[test]
fn a_quiescent_undecided_region_state_is_certified() {
    let world = one_step(
        "self.x > 1 ms",
        &[(
            "Never",
            "<= 0.01",
            "eventually[1 ms, 2 ms] holds(c.retried)",
        )],
    );
    assert_eq!(outcome(&settle(&world, "Never")), "proved 0/1");
}

/// `step` before 1 ms leaves the subject quiescent with `late` true, and an
/// idle position in [2, 3] ms then reads it: the minimum of
/// `always[2 ms, 3 ms] holds(not c.late)` is 0. Never taking `step`, or
/// observing nothing, gives the maximum 1.
#[test]
fn an_idle_position_in_a_later_region_decides_the_window() {
    let formula = "always[2 ms, 3 ms] holds(not c.late)";
    let world = one_step(
        "self.x < 1 ms",
        &[("Min", ">= 0.001", formula), ("Max", "<= 0.999", formula)],
    );
    assert_eq!(outcome(&settle(&world, "Min")), "refuted");
    assert_eq!(outcome(&settle(&world, "Max")), "refuted");
}
