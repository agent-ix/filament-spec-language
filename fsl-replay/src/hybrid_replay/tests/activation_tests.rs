// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": `on each` claims under `model-time` whose
//! obligations read their activation position's state, decided with the
//! activation instant as a second real variable. Every expectation is hand
//! derived from the point lasso's solution.

use ix_trace_rs::trace;

use super::*;

const SHUTTLE: &str = "hybrid Shuttle using p {\n\
       initial { mode Moving when { v = 0 }; }\n\
       mode Moving invariant { v >= 0 and v <= 1 } flow { v' = 1; }\n\
         transition to Moving when { v = 1 } reset { v = 0 };\n\
         transition to Stopped when { v >= 0 } reset { v >= 0 };\n\
       mode Stopped invariant { v >= 1 and v <= 2 } flow { v' = 0; }\n\
     }\n";

const SHUTTLE_CLAIMS: &str = "temporal Cap using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(h.v >= v0 + 1 / 2) }\n\
     temporal CapHeld using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(h.v >= v0) }\n\
     temporal Range using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 1; 1, 2] = q.v + 1 / 2; \
       eventually [0, 1] holds(h.v + 1 / 2 >= v0) }\n\
     temporal Hist using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0) { historically holds(h.v > 0) }\n\
     temporal Once using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0) { once holds(h.v = 0) }\n\
     temporal Past using p over (h: Shuttle) clock \"model-time\" \
       on each (q: Shuttle) when (q.v > 0 and q.v <= 1 / 2) { not once holds(h.v > 3 / 4) }\n";

/// `Shuttle`'s point lasso: from `(Moving, v = 0)` at time 0 by transition 0
/// (`before` `(time 1, v = 1)`) to `(Moving, v = 0)` at time 1, `loop_to` 0.
fn shuttle_lasso() -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(1), &[n(1)])),
            state: state(0, n(1), &[n(0)]),
        }],
        delta: None,
        loop_to: Some(0),
        end: None,
    }
}

fn shuttle() -> Unit {
    check(&format!("{SHUTTLE}{SHUTTLE_CLAIMS}"))
}

fn not_reproduced() -> Result<ExactReplay, HybridReplayRefusal> {
    Ok(ExactReplay::NotReproduced {
        at: JsonPointer::root(),
        check: ExactCheck::Claim,
    })
}

/// FR-750-AC-12 (a): the obligation activated at `tau` captures `v0 = tau`
/// and needs `v >= tau + 1/2` somewhere in `[tau, tau + 1]`, where `v` peaks
/// at 1: it holds for `tau <= 1/2` and fails for `tau` in `(1/2, 1)`, so the
/// claim reproduces with kind `Formula`; with body `v >= v0`, true at every
/// activation instant, it is not reproduced at check 5.
#[trace("TC-3037", "FR-750-AC-12")]
#[test]
fn a_capture_dependent_obligation_fails_exactly_where_the_activation_instant_makes_it_so() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Cap"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "CapHeld"), &trace),
        not_reproduced()
    );
}

/// FR-450-AC-12 (a): the capture `tau + 1/2` is outside `[0, 1]` for the
/// activation instants in `(1/2, 1)` of each period and is `1`, in range,
/// at `tau = 1/2`: the set is open with no earliest instant, so the
/// reproduced `outside-capture-range` is reported at its infimum 1/2.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn an_open_set_of_out_of_range_captures_reports_its_infimum() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    let Ok(ExactReplay::Reproduced(found)) = exact_replay(&graph, claim(&graph, "Range"), &trace)
    else {
        panic!("the capture is out of range");
    };
    let HybridCounterexampleKind::UndefinedEvaluation { at, cause } = &found.kind else {
        panic!("an undefined evaluation: {:?}", found.kind);
    };
    assert_eq!(*cause, UndefinedCause::OutsideCaptureRange { capture: 0 });
    assert_eq!(at.state, 0);
    assert_eq!(at.time, ExactReal::Rational(q(1, 2)));
}

/// FR-450-AC-12 (b): `historically` reads from position 0, where `v = 0`,
/// so every obligation is false; `once holds(h.v = 0)` is true at every
/// activation for the same reason.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn a_past_operator_reads_the_behaviour_from_position_zero() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Hist"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Once"), &trace),
        not_reproduced()
    );
}

/// FR-450-AC-12 (c): `not once holds(h.v > 3/4)` at `tau` in `(0, 1/2]` is
/// true in the first period and false from the second on, whose history
/// holds the first period's `v` in `(3/4, 1)`: the past reach is one
/// period, so the first two periods decide the claim.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn a_past_reach_of_one_period_decides_two_periods() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Past"), &trace),
        Ok(reproduced_formula(&trace))
    );
}

const PHASE: &str = "hybrid Phase using p {\n\
       initial { mode Warm when { x = 0 and w = 1 }; }\n\
       mode Warm invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; }\n\
         transition to Cold when { x = 1 } reset { x = 0 and w = 0 };\n\
       mode Cold invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; }\n\
         transition to Cold when { x = 1 } reset { x = 0 and w = 0 };\n\
     }\n\
     temporal Twice using p over (h: Phase) clock \"model-time\" \
       on each (q: Phase) { once [0, 1] once [0, 1] holds(h.w = 1) }\n";

/// `Phase`'s point lasso: `Warm` from `x = 0, w = 1` at time 0 to `Cold`
/// at time 1, and `Cold`'s self loop to time 2, `loop_to` 1.
fn phase_lasso() -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0), n(1)]),
        jumps: vec![
            TraceJump {
                transition: 0,
                before: Some(before(n(1), &[n(1), n(1)])),
                state: state(1, n(1), &[n(0), n(0)]),
            },
            TraceJump {
                transition: 0,
                before: Some(before(n(2), &[n(1), n(0)])),
                state: state(1, n(2), &[n(0), n(0)]),
            },
        ],
        delta: None,
        loop_to: Some(1),
        end: None,
    }
}

/// FR-450-AC-12 (d): `once [0, 1] once [0, 1] holds(h.w = 1)` has past
/// reach 2 over a period of 1, so the prefix and three periods are decided:
/// the obligations hold at every activation instant at most 3 and fail
/// after 3, which a replay of the prefix and two periods would miss.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn a_past_reach_of_two_periods_decides_three() {
    let graph = check(PHASE);
    let trace = phase_lasso();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Twice"), &trace),
        Ok(reproduced_formula(&trace))
    );
}

/// The activation scan is charged to the caller's meter: a bound below
/// what it needs is exhausted, never a panic or an answer.
#[trace("TC-1221", "FR-450-AC-7")]
#[test]
fn the_activation_scan_is_charged_to_the_meter() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    let claim = claim(&graph, "Cap");
    let mut spent = HybridReplayMeter::new(u64::MAX);
    super::super::exact_replay(&graph, claim, &trace, &mut spent).expect("unmetered");
    assert!(spent.consumed() > 100, "{}", spent.consumed());
    for bound in [spent.consumed() - 1, spent.consumed() / 2, 30] {
        assert_eq!(
            super::super::exact_replay(&graph, claim, &trace, &mut HybridReplayMeter::new(bound)),
            Err(HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits
            }),
            "{bound}"
        );
    }
}
