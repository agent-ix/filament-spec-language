// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-038 RL-7: each row of the route admission table, given to EN-5 and to
//! the certificate checker, with the route each must give written here by
//! hand (FR-921-AC-5), and the TC-374 subjects whose integer-time reading
//! gives a false verdict.

use filament_exact::Cancel;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, AnalyzeRecord, Engines};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::RegionCause;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    timed_certificate_route, IncompleteCause, ProbabilityDocumentError, ProbabilityDocumentLimits,
    TerminalValue, TimedProbabilityRoute,
};
use ix_trace_rs::trace;

use super::models::PROFILES;
use super::models::{self, every, under};
use crate::timed_world::{self, World, DENSE};

fn model(clocks: &[&str], operations: &[(&str, &[&str])], body: &str) -> World {
    timed_world::world(
        &timed_world::document(clocks, operations),
        PROFILES,
        &format!("{DENSE}{body}"),
    )
}

/// TC-374's `Overtake`: `a` and `c` have given delays, `b` and `d` are free.
/// `DDone` is `late`. On dense time its maximum is 1, at integer times 1/2.
fn overtake(claims: &str) -> World {
    model(
        &["x"],
        &[
            ("a", &["phase"]),
            ("b", &["phase"]),
            ("c", &["phase"]),
            ("d", &["phase", "late"]),
        ],
        &format!(
            "pre A using v on M::Call::a {{ self.phase = 0 }}\n\
             post DidA using v on M::Call::a {{ self.phase = 1 }}\n\
             delay M::Call::a ~ discrete {{ 1 ms: 1 }};\n\
             pre B using v on M::Call::b {{ self.phase = 0 }}\n\
             post DidB using v on M::Call::b {{ self.phase = 2 }}\n\
             pre C using v on M::Call::c {{ self.phase = 2 }}\n\
             post DidC using v on M::Call::c {{ self.phase = 3 }}\n\
             delay M::Call::c ~ discrete {{ 1 ms: 1 }};\n\
             pre D using v on M::Call::d {{ self.phase = 2 and self.x >= 1 ms }}\n\
             post DidD using v on M::Call::d {{ self.phase = 3 and self.late = true }}\n\
             workload W on M {{ weight Call::a = 1; weight Call::b = 1; weight Call::c = 1; weight Call::d = 1; }}\n\
             {claims}"
        ),
    )
}

/// TC-374's `Pinch`: `first` runs at `t` in [0, 1] ms and only `t` 0 or 1
/// reaches `Done` (`late`); every other `t` time-locks, so the maximum
/// expected elapsed time is infinite on dense time and 1 ms at integer times.
fn pinch(claims: &str) -> World {
    model(
        &["x", "z"],
        &[
            ("first", &["phase", "z"]),
            ("low", &["phase", "late"]),
            ("high", &["phase", "late"]),
        ],
        &format!(
            "pre First using v on M::Call::first {{ self.phase = 0 }}\n\
             post FirstDone using v on M::Call::first {{ self.phase = 1 and self.z = 0 ms }}\n\
             pre Low using v on M::Call::low {{ self.phase = 1 and self.x <= 0 ms }}\n\
             post LowDone using v on M::Call::low {{ self.phase = 2 and self.late = true }}\n\
             pre High using v on M::Call::high {{ self.phase = 1 and self.x >= 1 ms }}\n\
             post HighDone using v on M::Call::high {{ self.phase = 2 and self.late = true }}\n\
             time invariant Start when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
             time invariant Mid when {{ self.phase = 1 }} {{ self.z <= 0 ms }}\n\
             {claims}"
        ),
    )
}

/// TC-374's `FairLoop`: `spin` (the vector's `loop`) resets `x` while
/// `x <= 1 ms` and `leave` (its `t`) sets `late` once `x >= 1 ms`. Under
/// `fair strong leave` the dense minimum of reaching `late` is 0.
fn fair_loop(claims: &str) -> World {
    model(
        &["x"],
        &[("spin", &["x"]), ("leave", &["late"])],
        &format!(
            "pre Spin using v on M::Call::spin {{ not self.late and self.x <= 1 ms }}\n\
             post Spun using v on M::Call::spin {{ self.x = 0 ms }}\n\
             pre Leave using v on M::Call::leave {{ not self.late and self.x >= 1 ms }}\n\
             post Left using v on M::Call::leave {{ self.late = true }}\n\
             time invariant Stay when {{ not self.late }} {{ self.x <= 1 ms }}\n\
             {claims}"
        ),
    )
}

/// An unbounded claim, which the infinite-trace profile reads.
fn unbounded(name: &str, scheduler: &str, form: &str) -> String {
    format!("probabilistic {name} using inf on M over (c: M::Call) {scheduler} on origin {{ {form} }}\n")
}

fn settle(world: &World, claim: &str) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let request = ProductRequest {
        subject: ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim: world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .unwrap_or_else(|| panic!("the claim {claim} checks")),
        limits: ExactProbLimits::default(),
    };
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

fn checker(world: &World, claim: &str) -> Result<TimedProbabilityRoute, ProbabilityDocumentError> {
    timed_certificate_route(
        &world.system(),
        world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .unwrap_or_else(|| panic!("the claim {claim} checks")),
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 16_000_000,
            states: 4096,
            edges: 65_536,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
        &Cancel::new(),
    )
}

/// EN-5 stops the claim `incomplete` naming `cause`, and the checker admits
/// no certificate route for it.
fn assert_region_not_built(world: &World, claim: &str, cause: RegionCause) {
    let record = settle(world, claim);
    assert!(
        matches!(
            record.terminal().map(|terminal| terminal.value()),
            Some(TerminalValue::Incomplete(
                IncompleteCause::UnresolvedDynamics
            ))
        ),
        "{claim}: {record:#?}"
    );
    assert_eq!(record.region_cause, Some(cause), "{claim}");
    assert!(record.exact_probabilistic.is_none(), "{claim}");
    let found = checker(world, claim);
    assert!(
        matches!(found, Err(ProbabilityDocumentError::ObjectiveMismatch)),
        "{claim}: {found:?}"
    );
}

/// EN-5 reads the claim over digital clocks and proves it, and the checker
/// gives the digital route.
/// EN-5 does not leave digital clocks for the claim, and the checker gives the
/// digital route.
fn assert_digital(world: &World, claim: &str) {
    let record = settle(world, claim);
    assert_eq!(record.region_cause, None, "{claim}: {record:#?}");
    let found = checker(world, claim);
    assert!(
        matches!(found, Ok(TimedProbabilityRoute::Digital)),
        "{claim}: {found:?}"
    );
}

fn assert_digital_proof(world: &World, claim: &str) {
    let record = settle(world, claim);
    assert!(
        matches!(
            record.terminal().map(|terminal| terminal.value()),
            Some(TerminalValue::Proved { .. })
        ),
        "{claim}: {record:#?}"
    );
    assert_eq!(record.region_cause, None, "{claim}");
    let found = checker(world, claim);
    assert!(
        matches!(found, Ok(TimedProbabilityRoute::Digital)),
        "{claim}: {found:?}"
    );
}

#[trace("FSpec-TC-374", "FSpec-FR-421-AC-10")]
#[test]
fn overtake_mixed_delays_are_not_read_at_integer_times() {
    let world = overtake(&unbounded(
        "Overtake",
        "under W",
        "probability <= 0.6 [ eventually holds(c.late) ]",
    ));
    assert_region_not_built(&world, "Overtake", RegionCause::MixedDelays);
}

#[trace("FSpec-TC-374", "FSpec-FR-421-AC-12")]
#[test]
fn pinch_expected_elapsed_under_a_time_lock_is_not_read_at_integer_times() {
    let world = pinch(&every(
        "Pinch",
        "expected elapsed until holds(c.phase = 2) <= 2 ms",
    ));
    assert_region_not_built(&world, "Pinch", RegionCause::TimeLock);
}

#[trace("FSpec-TC-374", "FSpec-FR-421-AC-13")]
#[test]
fn fair_loop_is_not_read_at_integer_times() {
    let world = fair_loop(
        "probabilistic FairLoop using inf on M over (c: M::Call) under every scheduler \
         fair strong Call::leave on origin { probability >= 0.9 [ eventually holds(c.late) ] }\n",
    );
    assert_region_not_built(&world, "FairLoop", RegionCause::Fairness);
}

/// The rows ADR-038 admits to digital clocks: a given-delay subject, and a
/// free-delay subject with no reachable time-lock.
#[trace("TC-2490", "FR-921-AC-5")]
#[test]
fn admitted_rows_are_digital_for_the_engine_and_the_checker() {
    let given = models::send_reply(
        3,
        None,
        &unbounded(
            "Given",
            "under Even",
            "probability >= 0.9 [ eventually holds(c.phase = 1) ]",
        ),
    );
    assert_digital(&given, "Given");
    let free = models::retx(
        &(unbounded(
            "Reach",
            "under every scheduler",
            "probability >= 0.9 [ eventually holds(c.late) ]",
        ) + &every("Mean", "expected elapsed until holds(c.late) <= 3 ms")
            + &every(
                "Deadline",
                "probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ]",
            )),
    );
    assert_digital(&free, "Reach");
    assert_digital_proof(&free, "Mean");
    assert_digital_proof(&free, "Deadline");
}

/// A reachable time-lock admits only the maximum of a reach or
/// deadline-on-origin event: on `Split` the maximum of reaching phase 2 is
/// 1/2 on dense and integer times alike, the minimum 0 against 1/2.
#[trace("TC-2490", "FR-921-AC-5")]
#[trace("FSpec-TC-374", "FSpec-FR-421-AC-8")]
#[test]
fn a_time_lock_admits_the_maximum_only() {
    let form = "eventually[0 ms, 3 ms] holds(c.phase = 2)";
    let world = models::split(
        &(every("Most", &format!("probability <= 0.6 [ {form} ]"))
            + &every("Least", &format!("probability >= 0.25 [ {form} ]"))),
    );
    assert_digital_proof(&world, "Most");
    assert_region_not_built(&world, "Least", RegionCause::TimeLock);
}

#[trace("TC-2490", "FR-921-AC-5")]
#[test]
fn a_free_subject_read_from_a_later_position_needs_regions() {
    let world = models::skew(&every(
        "Later",
        "quantile 0.4 of (elapsed from holds(c.inflight) until holds(c.late) within 4 ms) <= 1 ms",
    ));
    assert_region_not_built(&world, "Later", RegionCause::LaterPosition);
}

#[trace("TC-2490", "FR-921-AC-5")]
#[test]
fn a_nested_window_over_free_delays_needs_regions() {
    let world = models::skew(&every(
        "Nested",
        "probability >= 0.4 [ eventually[0 ms, 4 ms] \
         (holds(c.inflight and not c.late) and eventually[0 ms, 1 ms] holds(c.late)) ]",
    ));
    assert_region_not_built(&world, "Nested", RegionCause::OtherFormula);
}

#[trace("TC-2490", "FR-921-AC-5")]
#[test]
fn an_unweighted_time_fraction_needs_regions() {
    let world = models::send_reply(
        0,
        None,
        &under(
            "Even",
            "Burst",
            "mean of (fraction holds(c.phase = 1) over [0, 1 ms]) <= 0.7",
        ),
    );
    assert_region_not_built(&world, "Burst", RegionCause::UnweightedTimeFraction);
}

/// TC-374's `Skew` reading: a delay-weighted fraction over free delays.
#[trace("TC-2490", "FR-921-AC-5")]
#[trace("FSpec-TC-374", "FSpec-FR-421-AC-11")]
#[test]
fn a_delay_weighted_fraction_over_free_delays_needs_regions() {
    let world = models::skew(&every(
        "Weighted",
        "probability <= 0.6 [ fraction holds(c.inflight and not c.late) over [0, 2 ms] weighted by delay >= rational(1, 4) ]",
    ));
    assert_region_not_built(&world, "Weighted", RegionCause::DelayWeightedFraction);
}

/// A strict subject takes the region route on both sides.
#[trace("TC-2490", "FR-921-AC-5")]
#[test]
fn a_strict_subject_takes_the_region_route() {
    let world = models::two_clocks(
        true,
        &every(
            "Strict",
            "probability <= 0.5 [ eventually[0 ms, 4 ms] holds(c.late) ]",
        ),
    );
    let record = settle(&world, "Strict");
    assert_eq!(record.region_cause, None);
    let found = checker(&world, "Strict");
    assert!(
        matches!(found, Ok(TimedProbabilityRoute::Region)),
        "{found:?}"
    );
}
