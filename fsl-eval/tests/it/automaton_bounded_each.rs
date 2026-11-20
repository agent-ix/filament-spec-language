// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's canonical bounded translation of `eventually[0,u] holds(A)` and
//! `always[0,u] holds(A)`, against the reference evaluator of FR-327: over
//! every trace of up to four positions, every horizon from zero to three and
//! every reading of the atom and of an `on each` `when` condition, the
//! translation driven letter by letter rejects exactly where the evaluator
//! reports a violation.
//!
//! The grid covers an `on each` activation with and without a condition
//! (overlapping live activations, a discharge and a reopening at one
//! position, windows part elapsed, horizons shorter and longer than the
//! trace, the terminal false extension) and the infinite-trace profile,
//! whose terminal state stutters the last real letter for ever.

use filament_exact::{Meter, Origin, Role};
use fsl_eval::automaton::{BoundedState, BoundedTranslation, StateContext};
use fsl_eval::temporal::{
    evaluate, AtomReading, CaptureReading, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, TemporalTrace,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use ix_trace_rs::trace;

use crate::temporal_trace::{fixture, Fixture, EVENT, INFINITE};

/// The truth of the `holds` atom and of the `when` condition at each
/// position of a trace.
struct Script {
    holds: Vec<bool>,
    guard: Vec<bool>,
}

fn reading(truth: bool) -> AtomReading {
    if truth {
        AtomReading::True
    } else {
        AtomReading::False
    }
}

impl TemporalLetters for Script {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(reading(
            self.holds[usize::try_from(position.index()).expect("small")],
        ))
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(reading(
            self.guard[usize::try_from(position.index()).expect("small")],
        ))
    }

    fn captures(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        Ok(CaptureReading::Fixed)
    }

    fn counts_step(&self, _position: TemporalPosition) -> bool {
        true
    }

    fn counts_loop_edge(&self) -> bool {
        true
    }
}

fn translation(fixture: &Fixture) -> BoundedTranslation {
    let occurrence = OccurrenceKey::new(
        crate::support::ids::node_key([0xab]),
        Origin::new(Role::new("temporal"), 0),
    );
    let context = StateContext::clause(&fixture.clause, occurrence, Vec::new());
    BoundedTranslation::clause(&fixture.clause, &context).expect("a bounded translation")
}

/// Whether the evaluator reports a violation of the clause on `trace`.
fn violated(fixture: &Fixture, trace: TemporalTrace, script: &mut Script) -> bool {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    match evaluate(&fixture.clause, trace, script, &mut meter).expect("a readable trace") {
        TemporalOutcome::Holds => false,
        TemporalOutcome::Violated { .. } => true,
        other => panic!("a decided outcome, not {other:?}"),
    }
}

/// Whether the translation rejects the trace its letters script, closing
/// the last region by the false extension, or by stuttering the last real
/// letter when the translation reads the infinite trace.
fn rejects(translation: &BoundedTranslation, script: &Script) -> bool {
    let each = translation.each();
    let mut state = if each {
        translation.start_each(script.holds[0], script.guard[0])
    } else {
        translation.start(script.holds[0])
    };
    for position in 1..script.holds.len() {
        if state.is_rejecting() {
            return true;
        }
        state = if each {
            translation.step_each(state, true, script.holds[position], script.guard[position])
        } else {
            translation.step(state, true, script.holds[position])
        };
    }
    if translation.stutters() {
        let last = script.holds.len() - 1;
        // Past the horizon the stutter repeats a state, whatever it reads.
        for _ in 0..=translation.horizon() + 1 {
            if state.is_rejecting() {
                return true;
            }
            state = if each {
                translation.step_each(state, true, script.holds[last], script.guard[last])
            } else {
                translation.step(state, true, script.holds[last])
            };
        }
        return state.is_rejecting();
    }
    state.is_rejecting() || translation.terminal(state).is_rejecting()
}

fn truths(len: usize) -> Vec<Vec<bool>> {
    (0..1_u32 << len)
        .map(|code| (0..len).map(|bit| code >> bit & 1 == 1).collect())
        .collect()
}

/// Every trace up to four positions, every horizon up to three, each
/// operator, against the evaluator.
fn grid(profile: &str, activation: &str, guarded: bool) {
    for upper in 0..=3_u64 {
        for operator in ["eventually", "always"] {
            let body = format!("{operator}[0,{upper}] holds(c.level = 1)");
            let fixture = fixture(profile, "c: Counter", activation, &body);
            let translation = translation(&fixture);
            for len in 1..=4_usize {
                for holds in truths(len) {
                    let guards = if guarded {
                        truths(len)
                    } else {
                        vec![vec![true; len]]
                    };
                    for guard in guards {
                        let mut script = Script {
                            holds: holds.clone(),
                            guard: guard.clone(),
                        };
                        let trace = if profile == INFINITE {
                            TemporalTrace::Lasso {
                                prefix: u64::try_from(len - 1).expect("small"),
                                loop_len: 1,
                            }
                        } else {
                            TemporalTrace::Finite {
                                len: u64::try_from(len).expect("small"),
                            }
                        };
                        assert_eq!(
                            rejects(&translation, &script),
                            violated(&fixture, trace, &mut script),
                            "{body} {activation} holds {holds:?} guard {guard:?}",
                        );
                    }
                }
            }
        }
    }
}

/// FR-338-AC-6 against FR-327: an `on origin` bounded clause under the
/// false extension.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_origin_translation_agrees_with_the_evaluator_over_every_short_trace() {
    grid(EVENT, "on origin", false);
}

/// FR-338-AC-10 against FR-327: an unguarded `on each` clause, whose live
/// activations overlap, discharge and reopen.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn the_each_translation_agrees_with_the_evaluator_over_every_short_trace() {
    grid(EVENT, "on each (q: Counter)", false);
}

/// FR-338-AC-6 against FR-327: a guarded `on each` clause, activating at
/// the positions its condition holds.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_guarded_each_translation_agrees_with_the_evaluator_over_every_short_trace() {
    grid(EVENT, "on each (q: Counter) when (c.level = 1)", true);
}

/// FR-338-AC-6 against FR-329: an infinite-trace bounded clause stutters
/// its last real letter, so an open `always` window holds exactly when that
/// letter satisfies the atom, and an open `eventually` window meets exactly
/// when it does.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_infinite_translation_agrees_with_the_evaluator_over_every_short_lasso() {
    grid(INFINITE, "on origin", false);
}

/// FR-338-AC-10: the merged window an `on each` monitor keeps. A satisfying
/// letter discharges every open `eventually` occurrence at once; an
/// activation at a position that the letter does not satisfy keeps the
/// older, earlier deadline; an `always` activation extends to the furthest
/// extent, and a failing letter inside any open window rejects.
#[trace("TC-525", "FR-338-AC-28")]
#[test]
fn an_each_monitor_merges_its_activations_into_one_window() {
    let eventually = translation(&fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "eventually[0,2] holds(c.level = 1)",
    ));
    let open = |remaining| BoundedState::Open { remaining };
    // Opened at position 0 with the whole window left.
    let first = eventually.start_each(false, true);
    assert_eq!(first, open(2));
    // A second activation, unsatisfied, keeps the older deadline.
    let second = eventually.step_each(first, true, false, true);
    assert_eq!(second, open(1));
    // A satisfying letter discharges both, and the activation at the same
    // position is discharged by it too.
    assert_eq!(
        eventually.step_each(second, true, true, true),
        BoundedState::Idle
    );
    // A step out of an exhausted window rejects before its letter is read.
    assert_eq!(
        eventually.step_each(open(0), true, true, true),
        BoundedState::Reject
    );
    // A step that does not count leaves the deadline where it was.
    assert_eq!(eventually.step_each(open(1), false, false, false), open(1));

    let always = translation(&fixture(
        EVENT,
        "c: Counter",
        "on each (q: Counter)",
        "always[0,2] holds(c.level = 1)",
    ));
    let first = always.start_each(true, true);
    assert_eq!(first, open(2));
    // The extent runs down with distance, a new activation restores it, and
    // the furthest is kept.
    assert_eq!(always.step_each(first, true, true, false), open(1));
    assert_eq!(always.step_each(open(0), true, true, true), open(2));
    // An exhausted extent discharges at its closed boundary, and the next
    // letter is read against no obligation.
    assert_eq!(
        always.step_each(open(0), true, false, false),
        BoundedState::Idle
    );
    // A failing letter inside an open window, or at an activation, rejects.
    assert_eq!(
        always.step_each(open(1), true, false, false),
        BoundedState::Reject
    );
    assert_eq!(
        always.step_each(BoundedState::Idle, true, false, true),
        BoundedState::Reject
    );
    assert_eq!(
        always.step_each(BoundedState::Idle, true, false, false),
        BoundedState::Idle
    );
    // The terminal false extension rejects an extent that still has
    // positions to read and discharges an exhausted one.
    assert_eq!(always.terminal(open(1)), BoundedState::Reject);
    assert_eq!(always.terminal(open(0)), BoundedState::Idle);
    assert_eq!(eventually.terminal(open(0)), BoundedState::Reject);
    assert_eq!(eventually.terminal(BoundedState::Idle), BoundedState::Idle);
}
