// SPDX-License-Identifier: AGPL-3.0-or-later
//! A clock a collection write carries into another field (FR-230
//! "Collection-valued writes") is compared with that field's constants after
//! the write, so every per-field bound, cap and ceiling of the field it is
//! carried from covers them. Nothing compares `us`: `copy` writes `ts` from
//! it, and `work` compares `ts`. Each expected value is worked out by hand.

use std::num::NonZeroU64;

use filament_exact::{Integer, Rational};
use fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem, TimedSystem};
use fsl_replay::{
    CertificateMember, CertificateRule, CounterexampleKind, ModelTraceSettlement, ModelTraceValue,
    ZoneCertificateVerdict,
};
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::timed_product_certificate::{accepted, document, is_refuted, proved_value, settle};
use crate::timed_world::{ident, timers_document, wire_field, world_holding, World, DENSE};
use crate::zone_certificate::{claim_within, rejected, summary, verdict_within};
use crate::zone_replay::{payload, replay, walk, whole};
use crate::zone_search::{proved, INFINITE, TIMED};

/// A world whose `c` holds the `Timer` sequences `ts`, empty, and each of
/// `held`, with one element, and the clock `y`.
fn carrying_world(
    held: &[&str],
    operations: &[(&str, &[&str])],
    profiles: &str,
    body: &str,
) -> World {
    let mut document: Json = serde_json::from_slice(&timers_document(1, operations)).expect("JSON");
    let fields = document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields");
    let ts = fields
        .iter()
        .find(|field| field["name"] == "ts")
        .expect("ts")
        .clone();
    let timer = json!({"record": {
        "type": ident("Timer"), "fields": {"n": {"integer": "0"}}}});
    let mut holding = serde_json::Map::new();
    holding.insert("ts".to_owned(), json!({"sequence": []}));
    for name in held {
        let mut field = ts.clone();
        field["name"] = json!(name);
        field["identity"] = json!(ident(&format!("Call/{name}")));
        fields.push(field);
        holding.insert((*name).to_owned(), json!({"sequence": [timer.clone()]}));
    }
    fields.push(wire_field("y", "ix://filament/native/Clock"));
    world_holding(
        document.to_string().as_bytes(),
        profiles,
        &format!("{DENSE}{body}"),
        &Json::Object(holding),
    )
}

/// `early` (`y <= 0`) and `late` (`y >= 5`) each enter phase 1 and reset
/// `y`, which stays at most 1 in phases 1 and 2. `copy` writes `ts` from `us`
/// into phase 2, and `work`, which compares each element of `ts` with `guard`,
/// sets `late`. No element's clock is ever reset, so `work` is enabled only
/// after `late` entered phase 1.
fn gate(early: &str, late: &str, guard: &str, profiles: &str, claims: &str) -> World {
    carrying_world(
        &["us"],
        &[
            ("alpha", &["phase", "y"]),
            ("go", &["phase", "y"]),
            ("copy", &["phase", "ts"]),
            ("work", &["phase", "late"]),
        ],
        profiles,
        &format!(
            "pre Early using v on M::Call::{early} {{ self.phase = 0 and self.y <= 0 ms }}\n\
             post Earlied using v on M::Call::{early} {{ self.phase = 1 and self.y = 0 ms }}\n\
             pre Late using v on M::Call::{late} {{ self.phase = 0 and self.y >= 5 ms }}\n\
             post Lated using v on M::Call::{late} {{ self.phase = 1 and self.y = 0 ms }}\n\
             pre Copying using v on M::Call::copy {{ self.phase = 1 }}\n\
             post Copied using v on M::Call::copy {{ self.phase = 2 and self.ts = pre(self.us) }}\n\
             pre Bad using v on M::Call::work {{ self.phase = 2 and not self.late and {guard} }}\n\
             post Badded using v on M::Call::work {{ self.phase = 3 and self.late }}\n\
             time invariant Brief when {{ self.phase = 1 or self.phase = 2 }} {{ self.y <= 1 ms }}\n\
             {claims}"
        ),
    )
}

const AT_LEAST_FIVE: &str = "forall(e in self.ts: e.x >= 5 ms)";

/// Two elements' clocks, of `ts` and `us`, and `y`.
const GATE_CLOCKS: NonZeroU64 = NonZeroU64::MIN.saturating_add(2);

fn temporal(flag: &str) -> String {
    format!(
        "temporal Never using inf over (c: M::Call) clock \"model-steps\" on origin \
         {{ always holds(not c.{flag}) }}\n"
    )
}

/// FR-230 "Collection-valued writes" under ADR-026 EZ-4: `always not late`
/// is false, `late` after 5, `copy` and `work` reaching it, in whichever
/// order the two operations that enter phase 1 are named.
#[trace("TC-692")]
#[test]
fn a_clock_carried_into_a_compared_field_keeps_its_bound() {
    for (early, late) in [("alpha", "go"), ("go", "alpha")] {
        let world = gate(early, late, AT_LEAST_FIVE, INFINITE, &temporal("late"));
        assert_eq!(
            summary(&claim_within(&world, GATE_CLOCKS)),
            "Violated",
            "{late} after 5"
        );
        let steps = walk(
            &world,
            &[(late, whole(5)), ("copy", whole(0)), ("work", whole(0))],
        );
        let run = payload(&world, steps, CounterexampleKind::Formula, None);
        assert_eq!(
            replay(&world, &run).expect("replays"),
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: Some(3),
            }
        );
    }
}

/// FR-245 "LU bounds": `us`'s clock takes the constants `ts`'s is compared
/// with, 5 from below and 7 from above, and the checker rejects either bound
/// left out. The places are `ts`'s, `us`'s and `y`; `y` is compared with 5
/// from below and with 0 and 1 from above.
#[trace("TC-700", "FR-245-AC-2")]
#[trace("TC-1197", "FR-245-AC-7")]
#[test]
fn bounds_below_a_carried_clock_constant_are_rejected() {
    let world = gate(
        "alpha",
        "go",
        "forall(e in self.ts: e.x >= 5 ms) and forall(e in self.ts: e.x <= 7 ms)",
        INFINITE,
        &temporal("retried"),
    );
    let honest = proved(claim_within(&world, GATE_CLOCKS));
    let bound = |value: i64| Some(Integer::from(value));
    assert_eq!(honest.scale, Integer::from(1_i64));
    assert_eq!(honest.lu_bounds.lower, [bound(5), bound(5), bound(5)]);
    assert_eq!(honest.lu_bounds.upper, [bound(7), bound(7), bound(1)]);
    assert_eq!(
        verdict_within(&world, GATE_CLOCKS, &honest),
        ZoneCertificateVerdict::Accepted
    );
    let bounds = rejected(
        CertificateRule::Malformed,
        fsl_replay::CertificateLocus::CertificateMember(CertificateMember::Bounds),
    );
    let mut lower = honest.clone();
    lower.lu_bounds.lower[1] = None;
    assert_eq!(verdict_within(&world, GATE_CLOCKS, &lower), bounds);
    let mut upper = honest;
    upper.lu_bounds.upper[1] = None;
    assert_eq!(verdict_within(&world, GATE_CLOCKS, &upper), bounds);
}

/// A clock carried twice, `vs` into `us` by `shift` and `us` into `ts` by
/// `copy`: the constant `ts`'s is compared with reaches `vs`'s. `shift`'s
/// postcondition is declared first, so one pass over the writes in order
/// does not reach it.
#[trace("TC-700", "FR-245-AC-2")]
#[test]
fn a_clock_carried_twice_keeps_the_last_fields_bound() {
    let world = carrying_world(
        &["us", "vs"],
        &[("shift", &["us"]), ("copy", &["ts"]), ("work", &["late"])],
        INFINITE,
        &format!(
            "post Shifted using v on M::Call::shift {{ self.us = pre(self.vs) }}\n\
             post Copied using v on M::Call::copy {{ self.ts = pre(self.us) }}\n\
             pre Work using v on M::Call::work {{ {AT_LEAST_FIVE} }}\n\
             post Worked using v on M::Call::work {{ self.late }}\n\
             pre Shift using v on M::Call::shift {{ true }}\n\
             pre Copy using v on M::Call::copy {{ true }}\n\
             {}",
            temporal("retried")
        ),
    );
    let max_clocks = NonZeroU64::MIN.saturating_add(3);
    let honest = proved(claim_within(&world, max_clocks));
    let bound = |value: i64| Some(Integer::from(value));
    assert_eq!(honest.lu_bounds.lower, [bound(5), bound(5), bound(5), None]);
    assert_eq!(honest.lu_bounds.upper, [None, None, None, None]);
    assert_eq!(
        verdict_within(&world, max_clocks, &honest),
        ZoneCertificateVerdict::Accepted
    );
}

/// FR-419's task automaton keeps each source clock exactly up to the largest
/// constant its value can be compared with: 5 for `us`'s element, which
/// `copy` carries into `ts`.
#[test]
fn a_task_automaton_ceiling_follows_a_carried_clock() {
    let world = carrying_world(
        &["us"],
        &[("fire_a", &["phase"]), ("copy", &["ts"]), ("work", &["phase"])],
        TIMED,
        &format!(
            "post Copied using v on M::Call::copy {{ self.ts = pre(self.us) }}\n\
             pre Work using v on M::Call::work {{ {AT_LEAST_FIVE} }}\n\
             post Worked using v on M::Call::work {{ self.phase = 1 }}\n\
             taskset One {{ task a periodic 100 ms deadline 100 ms bcet 1 ms wcet 1 ms priority 1 lo; }}\n\
             schedulable OneFp on One under fixed_priority;\n\
             post Fire_a using v on M::Call::fire_a {{ self.phase = pre(self.phase) }}\n\
             release One.a on M::Call::fire_a;\n"
        ),
    );
    let system = world.system();
    let timed = TimedSystem::with_max_clocks(&system, "M", NonZeroU64::MIN.saturating_add(1))
        .expect("a timed subject");
    let product = TaskProductSystem::new(timed, "OneFp").expect("a checked task product");
    let automaton = TaskAutomaton::new(&product);
    let ceilings: Vec<Rational> = automaton.clocks()[..3]
        .iter()
        .map(|clock| clock.ceiling.clone())
        .collect();
    assert_eq!(ceilings, [whole(5), whole(5), whole(0)]);
}

/// `go` is forced at 5, and `copy` and `work` within a time unit each, so
/// `late` is reached within 10 ms with probability 1 under every scheduler:
/// `at least 0.5` holds with the value 1 and `at most 0.5` does not, over the
/// digital product (closed guard) and the region product (strict guard).
#[test]
fn a_probability_cap_follows_a_carried_clock() {
    let claim = |name: &str, bound: &str| {
        format!(
            "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
             on origin {{ probability {bound} [ eventually[0 ms, 10 ms] holds(c.late) ] }}\n"
        )
    };
    let claims = format!(
        "time invariant Wait when {{ self.phase = 0 }} {{ self.y <= 5 ms }}\n{}{}",
        claim("AtMost", "<= 0.5"),
        claim("AtLeast", ">= 0.5")
    );
    for guard in [AT_LEAST_FIVE, "forall(e in self.ts: e.x > 4 ms)"] {
        let world = carrying_world(
            &["us"],
            &[
                ("go", &["phase", "y"]),
                ("copy", &["phase", "ts"]),
                ("work", &["phase", "late"]),
            ],
            PROBABILISTIC,
            &format!(
                "pre Late using v on M::Call::go {{ self.phase = 0 and self.y >= 5 ms }}\n\
                 post Lated using v on M::Call::go {{ self.phase = 1 and self.y = 0 ms }}\n\
                 pre Copying using v on M::Call::copy {{ self.phase = 1 }}\n\
                 post Copied using v on M::Call::copy \
                 {{ self.phase = 2 and self.ts = pre(self.us) }}\n\
                 pre Bad using v on M::Call::work {{ self.phase = 2 and {guard} }}\n\
                 post Badded using v on M::Call::work {{ self.phase = 3 and self.late }}\n\
                 time invariant Brief when {{ self.phase = 1 or self.phase = 2 }} \
                 {{ self.y <= 1 ms }}\n{claims}"
            ),
        );
        let at_least = settle(&world, "AtLeast");
        assert_eq!(proved_value(&at_least), whole(1), "{guard}");
        assert!(accepted(&world, "AtLeast", &document(&at_least)), "{guard}");
        let at_most = settle(&world, "AtMost");
        assert!(is_refuted(&at_most), "{guard}: {at_most:#?}");
    }
}

const PROBABILISTIC: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
     profile inf = \"filament.temporal.infinite-trace/v1\";\n";
