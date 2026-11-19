// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820 "Equality": the streaming evaluator, advanced one position at a
//! time, gives after every position the outcome the batch evaluator gives
//! over the trace of the positions read so far, for each scan family and its
//! dual, under the closed (event-position) and prefix (infinite-trace)
//! readings, `on origin` and `on each`.

use filament_exact::{LimitKind, Meter};
use fsl_eval::temporal::{
    evaluate, ClauseStream, TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace,
};
use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use ix_trace_rs::trace;

use crate::temporal_trace::{counters, fixture, Fixture, Forced, Letters, EVENT, INFINITE};

/// Every `c.level` sequence over 0, 1, 2 of `len` positions.
fn sequences(len: u32) -> Vec<Vec<i64>> {
    let total = 3_usize.pow(len);
    (0..total)
        .map(|mut code| {
            (0..len)
                .map(|_| {
                    let level = i64::try_from(code % 3).expect("a small level");
                    code /= 3;
                    level
                })
                .collect()
        })
        .collect()
}

/// The batch outcome of `fixture` over `levels`.
fn batch(
    fixture: &Fixture,
    levels: &[i64],
    forced: &[(u64, &'static str, Forced)],
) -> Result<TemporalOutcome, TemporalRefusal> {
    let mut letters = Letters::new(fixture, counters(levels));
    letters.forced = forced.to_vec();
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let len = u64::try_from(levels.len()).expect("a short trace");
    evaluate(
        &fixture.clause,
        TemporalTrace::Finite { len },
        &mut letters,
        &mut meter,
    )
}

/// Advance a stream of `fixture` over `levels`, asserting after each
/// position that its outcome is the batch outcome over that prefix, and
/// return the most residuals it held after any position.
fn agree(fixture: &Fixture, levels: &[i64], forced: &[(u64, &'static str, Forced)]) -> u64 {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let mut stream = ClauseStream::open(&fixture.clause, &mut meter)
        .expect("the profile is a position profile")
        .expect("an unlimited meter");
    let mut letters = Letters::new(fixture, counters(levels));
    letters.forced = forced.to_vec();
    let mut held = 0;
    for (index, _) in levels.iter().enumerate() {
        let at = TemporalPosition::new(u64::try_from(index).expect("a short trace"));
        stream
            .push(&fixture.clause, &mut letters, at, &mut meter)
            .expect("an unlimited meter");
        held = held.max(stream.held());
        let prefix = &levels[..=index];
        assert_eq!(
            stream.outcome(&mut meter),
            batch(fixture, prefix, forced),
            "{} over {prefix:?}",
            fixture.text.lines().last().unwrap_or_default()
        );
    }
    held
}

/// Every sequence of `len` positions over `body` under `profile` and
/// `activation`.
fn corpus(profile: &str, activation: &str, body: &str, len: u32) {
    let fixture = fixture(profile, "c: Counter", activation, body);
    for levels in sequences(len) {
        agree(&fixture, &levels, &[]);
    }
}

const ORIGIN: &str = "on origin";
const EACH: &str = "on each (q: Counter)";

/// FR-820-AC-1, "Equality": `eventually` and its dual `always`, bounded,
/// shifted and open, nested and under `implies`.
#[test]
#[trace("FR-820-AC-1", "TC-2320")]
fn eventually_and_always_stream_equal_the_batch_run() {
    let bounded = [
        "eventually[0,2] holds(c.level = 2)",
        "always[1,3] holds(c.level <= 1)",
        "always[0,4] (holds(c.level = 1) implies eventually[1,2] holds(c.level = 2))",
        "eventually[0,3] always[0,1] holds(c.level = 1)",
        "not always[0,2] holds(c.level >= 1)",
    ];
    for body in bounded {
        for activation in [ORIGIN, EACH] {
            corpus(EVENT, activation, body, 6);
            corpus(INFINITE, activation, body, 6);
        }
    }
    let open = [
        "always holds(c.level <= 1)",
        "eventually holds(c.level = 2)",
        "always (holds(c.level = 1) implies eventually holds(c.level = 2))",
        "always eventually[0,1] holds(c.level = 0)",
        "eventually[2,*] holds(c.level = 1)",
        "always[1,*] holds(c.level > 0)",
    ];
    for body in open {
        for activation in [ORIGIN, EACH] {
            corpus(INFINITE, activation, body, 6);
        }
    }
}

/// FR-820 "Summarized positions": an obligation carried over a long trace
/// holds a residual per distinct obligation, not per position.
#[test]
#[trace("FR-820-AC-2", "TC-2320")]
fn a_long_trace_holds_a_bounded_number_of_residuals() {
    let levels: Vec<i64> = (0..300).map(|index| index % 3).collect();
    for (profile, activation, body) in [
        (INFINITE, ORIGIN, "always holds(c.level <= 2)"),
        (
            INFINITE,
            EACH,
            "always (holds(c.level = 1) implies eventually holds(c.level = 2))",
        ),
        (
            INFINITE,
            ORIGIN,
            "always (holds(c.level = 2) implies once[1,2] holds(c.level = 0))",
        ),
        (EVENT, EACH, "eventually[0,2] holds(c.level = 2)"),
    ] {
        let fixture = fixture(profile, "c: Counter", activation, body);
        let held = agree(&fixture, &levels, &[]);
        assert!(held <= 8, "{body}: {held} residuals");
    }
}

/// FR-820 "Equality": a position whose atom is undefined, refused or short of
/// data is read as the batch run reads it, the positions after it unknown.
#[test]
#[trace("FR-820-AC-1", "TC-2320")]
fn stopped_readings_stream_equal_the_batch_run() {
    let body = "always[0,5] (holds(c.level = 1) implies eventually[0,2] holds(c.level = 2))";
    for forced in [Forced::Undefined, Forced::Refused, Forced::ShortOfData] {
        for at in 0..4 {
            for profile in [EVENT, INFINITE] {
                for activation in [ORIGIN, EACH] {
                    let fixture = fixture(profile, "c: Counter", activation, body);
                    for levels in sequences(5) {
                        agree(&fixture, &levels, &[(at, "holds(c.level = 2)", forced)]);
                    }
                }
            }
        }
    }
}

/// Each position's work is charged on the caller's meter.
#[test]
#[trace("FR-820-AC-2", "TC-2320")]
fn each_position_charges_its_own_work() {
    let fixture = fixture(INFINITE, "c: Counter", ORIGIN, "always holds(c.level <= 2)");
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let mut stream = ClauseStream::open(&fixture.clause, &mut meter)
        .expect("a position profile")
        .expect("an unlimited meter");
    let levels: Vec<i64> = vec![0; 200];
    let mut letters = Letters::new(&fixture, counters(&levels));
    let mut charges = Vec::new();
    for index in 0..200 {
        let mut step = Meter::new(SCALAR_LIMITS_UNLIMITED);
        stream
            .push(
                &fixture.clause,
                &mut letters,
                TemporalPosition::new(index),
                &mut step,
            )
            .expect("an unlimited meter");
        charges.push(step.consumed(LimitKind::WorkUnits));
    }
    // The first position opens the activation; every later one does the
    // same work.
    assert!(
        charges[1..].iter().all(|charge| *charge == charges[1]),
        "{charges:?}"
    );
}

/// FR-820-AC-1, "Equality": `once` and its dual `historically`, closed
/// windows reading before the origin, open windows reading from 0, and a
/// future scan inside a past one.
#[test]
#[trace("FR-820-AC-1", "TC-2320")]
fn once_and_historically_stream_equal_the_batch_run() {
    let bodies = [
        "always[0,5] (holds(c.level = 2) implies once[1,2] holds(c.level = 0))",
        "historically[0,2] holds(c.level <= 1)",
        "always[0,5] historically[1,3] holds(c.level >= 1)",
        "once[0,3] eventually[0,1] holds(c.level = 2)",
        "eventually[0,3] once[2,4] holds(c.level = 1)",
        "always[0,5] not once[0,1] historically[0,1] holds(c.level = 0)",
    ];
    for body in bodies {
        for activation in [ORIGIN, EACH] {
            corpus(EVENT, activation, body, 6);
            corpus(INFINITE, activation, body, 6);
        }
    }
    for body in [
        "always (holds(c.level = 2) implies once holds(c.level = 0))",
        "always historically[1,*] holds(c.level >= 1)",
        "always[0,5] once holds(c.level = 1)",
        "always[0,5] historically[1,*] holds(c.level <= 1)",
        "always once eventually holds(c.level = 1)",
    ] {
        for activation in [ORIGIN, EACH] {
            corpus(INFINITE, activation, body, 6);
        }
    }
}

/// FR-820-AC-1, "Equality": `until` and its dual `release`, bounded, shifted
/// and open.
#[test]
#[trace("FR-820-AC-1", "TC-2320")]
fn until_and_release_stream_equal_the_batch_run() {
    let bodies = [
        "holds(c.level <= 1) until[0,3] holds(c.level = 2)",
        "holds(c.level = 1) release[0,3] holds(c.level >= 1)",
        "always[0,4] (holds(c.level = 0) implies (holds(c.level <= 1) until[1,2] holds(c.level = 2)))",
        "(eventually[0,1] holds(c.level = 1)) until[0,3] holds(c.level = 2)",
        "holds(c.level >= 1) release[2,4] (holds(c.level = 0) until[0,1] holds(c.level = 1))",
    ];
    for body in bodies {
        for activation in [ORIGIN, EACH] {
            corpus(EVENT, activation, body, 6);
            corpus(INFINITE, activation, body, 6);
        }
    }
    for body in [
        "holds(c.level <= 1) until holds(c.level = 2)",
        "holds(c.level = 1) release holds(c.level >= 1)",
        "always (holds(c.level = 0) implies (holds(c.level >= 0) until[1,*] holds(c.level = 2)))",
        "(eventually holds(c.level = 1)) until holds(c.level = 2)",
    ] {
        for activation in [ORIGIN, EACH] {
            corpus(INFINITE, activation, body, 6);
        }
    }
}

/// FR-820-AC-1, "Equality": `since` and its dual `triggered`, closed windows
/// reading before the origin and open windows reading from 0.
#[test]
#[trace("FR-820-AC-1", "TC-2320")]
fn since_and_triggered_stream_equal_the_batch_run() {
    let bodies = [
        "always[0,5] (holds(c.level = 2) implies (holds(c.level >= 1) since[0,3] holds(c.level = 0)))",
        "always[0,5] (holds(c.level = 2) implies (holds(c.level >= 1) triggered[1,3] holds(c.level = 0)))",
        "holds(c.level <= 1) since[0,2] holds(c.level = 1)",
        "eventually[0,2] (holds(c.level = 1) since[0,2] eventually[0,1] holds(c.level = 2))",
    ];
    for body in bodies {
        for activation in [ORIGIN, EACH] {
            corpus(EVENT, activation, body, 6);
            corpus(INFINITE, activation, body, 6);
        }
    }
    for body in [
        "always (holds(c.level = 2) implies (holds(c.level >= 1) since holds(c.level = 0)))",
        "always (holds(c.level = 1) triggered[0,*] holds(c.level >= 1))",
        "always[0,5] (holds(c.level = 1) since[1,*] holds(c.level = 0))",
        "always[0,5] (holds(c.level = 1) triggered holds(c.level >= 1))",
    ] {
        for activation in [ORIGIN, EACH] {
            corpus(INFINITE, activation, body, 6);
        }
    }
}
