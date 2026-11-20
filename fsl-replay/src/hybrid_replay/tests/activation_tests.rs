// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": `on each` claims under `model-time` whose
//! obligations read their activation position's state, decided with the
//! activation instant as a second real variable. Every expectation is hand
//! derived from the point lasso's solution.

use std::cmp::Ordering;

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
       on each (q: Shuttle) when (q.v > 0 and q.v <= 1 / 2) { not once holds(h.v > 3 / 4) }\n\
     temporal Origin using p over (h: Shuttle) clock \"model-time\" \
       on origin { capture v0: Rational[0, 2; 1, 2] = h.v; eventually [0, 1] holds(h.v >= v0 + 2) }\n\
     temporal OriginHeld using p over (h: Shuttle) clock \"model-time\" \
       on origin { capture v0: Rational[0, 2; 1, 2] = h.v; eventually [0, 1] holds(h.v >= v0 + 1) }\n";

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

/// `Ramp`: `v` rises from 0 at rate 1, never jumps, and the finite trace
/// ends at time 10. The obligation activated at `tau` in `(0, 1)` captures
/// `v0 = tau`; its atom `2 <= v <= 2 + 4 v0` holds on `[2, 2 + 4 tau]`, an
/// interval of length `4 tau`, so `always [0, 1]` holds on `[2, 1 + 4 tau]`,
/// nonempty exactly when `tau >= 1/4`, and `eventually [0, 5]` over it holds
/// at `tau` exactly then. The truth changes where two root curves of the
/// atom, `2 + 4 tau` and `2`, are the window width 1 apart: `tau = 1/4`.
const RAMP: &str = "hybrid Ramp using p {\n\
       initial { mode Run when { v = 0 and w = 0 }; }\n\
       mode Run invariant { v >= 0 } flow { v' = 1; w' = 0; }\n\
     }\n\
     temporal Nested using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 5] always [0, 1] holds(h.v >= 2 and h.v <= 2 + 4 * v0) }\n\
     temporal NestedLow using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1 / 8) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 5] always [0, 1] holds(h.v >= 2 and h.v <= 2 + 4 * v0) }\n\
     temporal Deep using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [2, 9] always [1, 6] eventually [0, 1] holds(h.v >= 4 and h.v <= 4 + 12 * v0) }\n\
     temporal Cube using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(v0 * v0 * v0 != 4) }\n\
     temporal CubeHeld using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(v0 * v0 * v0 != 4 or h.v >= 0) }\n\
     temporal Touch using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [1, 1] holds(h.v * h.v * h.v != 4 + v0 - v0) }\n\
     temporal TouchHeld using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [1, 1] holds(h.v * h.v * h.v != 4 + v0 - v0 or h.v >= 0) }\n\
     temporal Meet using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       always [0, 2] holds(v0 * v0 * v0 != 4 or h.v != v0 + 1) }\n\
     temporal MeetHeld using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       always [0, 2] holds(v0 * v0 * v0 != 4 or h.v != v0 + 3) }\n\
     temporal Shared using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 - 7) * (v0 * v0 * v0 - 4) != 0 or (v0 - 7) * (h.v - 3) < 0) }\n\
     temporal Reducible using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 2 and q.v < 4) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 - 7) * (v0 - 3) != 0 or (v0 - 7) * (h.v - 9) < 0) }\n\
     temporal Pole using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 10) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 10] holds(1 / ((v0 - 5) * (v0 - 5) + (h.v - 3) * (h.v - 3)) != 0) }\n\
     temporal PoleFour using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 10) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 10] holds(1 / ((v0 - 4) * (v0 - 4) + (h.v - 3) * (h.v - 3)) != 0) }\n\
     temporal Sqrt using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 10] holds(1 / (h.v * h.v - 2) != 0 and v0 > 0) }\n\
     temporal Back using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - v0 + 1) != 0) }\n\
     temporal Irrational using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v > 2 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - v0) != 0) }\n\
     temporal LeadHolds using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v = 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 - 2) * h.v * h.v - 1 < 0) }\n\
     temporal LeadFails using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v = 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 - 2) * h.v * h.v + 1 < 0) }\n\
     temporal LeadClosed using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v >= 2 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 - 2) * h.v * h.v + 1 > 0) }\n\
     temporal CubeLeadHolds using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v * q.v = 4) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 * v0 - 4) * h.v * h.v * h.v - 1 < 0) }\n\
     temporal CubeLeadFails using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v * q.v = 4) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 * v0 - 4) * h.v * h.v * h.v + 1 < 0) }\n\
     temporal Lead3 using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 * v0 - 4) * h.v != 1) }\n\
     temporal Lead3b using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 * v0 - 4) * h.v * h.v != 1) }\n\
     temporal Lead3c using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 5] holds((v0 * v0 * v0 - 4) * h.v * h.v * h.v != 1) }\n\
     temporal Vanish using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 5] holds(not ((v0 * v0 * v0 - 4) * (h.v - 3) = 0)) }\n\
     temporal BigAtoms using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(h.v * 369988485035126972924700782451696644186473100389722973815184405301748249 > 1) and eventually [0, 1] holds(h.v * 1109965455105380918774102347355089932559419301169168921445553215905244747 > 1) and eventually [0, 1] holds(h.v * 3329896365316142756322307042065269797678257903507506764336659647715734241 > 1) and eventually [0, 1] holds(h.v * 9989689095948428268966921126195809393034773710522520293009978943147202723 > 1) and eventually [0, 1] holds(h.v * 29969067287845284806900763378587428179104321131567560879029936829441608169 > 1) }\n\
     temporal BigMoves using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - v0 - 1) * 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000) != 0) }\n\
     temporal CurveSq using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v * q.v > 2 and q.v < 3) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - v0 * v0 - v0) != 0) }\n\
     temporal CurveRoot using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v * h.v - v0 - 3) != 0) }\n\
     temporal CurveDip using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 3) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - (3 * v0 - 4) * (3 * v0 - 4) - 1) != 0) }\n\
     temporal CurveDown using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 3) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v + v0 - 5) != 0) }\n\
     temporal CurveMerge using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v * h.v - v0) != 0) }\n\
     temporal CurveTwice using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - 3) * (h.v - 2 * v0)) != 0) }\n\
     temporal CurveBig using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - 3 * v0) * 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000) != 0) }\n\
     temporal CurveCirc using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - v0 - 2) * (h.v - v0 - 2) - 2) != 0) }\n\
     temporal CrossPair using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 1 / 2 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - 2 * v0) * (h.v - 3 + v0)) != 0) }\n\
     temporal TwoBranch using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - 1) * (h.v - 1) - v0) != 0) }\n\
     temporal TwoBranchQ using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1 / 4) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / ((h.v - 1) * (h.v - 1) - v0) != 0) }\n\
     temporal DipA using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - (v0 - 7 / 10) * (v0 - 7 / 10) - 13 / 10) != 0) }\n\
     temporal DipC using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 3) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - (v0 - 2) * (v0 - 2) * (v0 + 1) - 3) != 0) }\n\
     temporal CurveHi5 using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v * h.v * h.v * h.v * h.v - 3 * h.v * h.v * h.v + v0 * v0 * v0 * h.v - 4 * v0) != 0) }\n\
     temporal CurveBig5 using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v * h.v * h.v - 1000000000000000000000000000000 / 7 * v0 * v0 * h.v + v0 - 3) != 0) }\n\
     temporal TauDeg using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 2] holds(h.v * h.v * h.v != v0 * v0 * v0 * v0 * v0 + v0 + 1) }\n\
     temporal CurveHigh using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v * h.v * h.v * h.v * h.v - 3 * h.v * h.v * h.v + v0 * h.v - 4) != 0) }\n\
     temporal Worst using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 2) { capture v0: Rational[0, 10; 1, 2] = q.v; \
       always [0, 1] holds(h.v * h.v * h.v * h.v * h.v - 3 * h.v * h.v * h.v + v0 * h.v <= 40) }\n\
     temporal Moves using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - v0) != 0) }\n\
     temporal MovesOne using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - v0 - 1) != 0) }\n\
     temporal Fixed using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v > 0 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 1] holds(1 / (h.v - 2) != 0 and v0 > 0) }\n\
     temporal NestedHeld using p over (h: Ramp) clock \"model-time\" \
       on each (q: Ramp) when (q.v >= 1 / 4 and q.v < 1) { capture v0: Rational[0, 2; 1, 2] = q.v; \
       eventually [0, 5] always [0, 1] holds(h.v >= 2 and h.v <= 2 + 4 * v0) }\n";

fn ramp_trace() -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0), n(0)]),
        jumps: Vec::new(),
        delta: None,
        loop_to: None,
        end: Some(before(n(10), &[n(10), n(0)])),
    }
}

/// FR-450-AC-12 (nested windows): the obligations are false exactly for
/// `tau` in `(0, 1/4)`, where `always [0, 1]` fits no region, so the claim
/// is reproduced although the one rational sample of `(0, 1)` between the
/// projection's other roots, 1/2, holds. With the guard `q.v < 1/8` every
/// activation instant is false; with `q.v >= 1/4` every one holds (at
/// `tau = 1/4` the atom's region is `[2, 3]`, exactly the window's width).
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn nested_windows_over_tau_dependent_atoms_cut_where_two_roots_are_a_width_apart() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Nested"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "NestedLow"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "NestedHeld"), &trace),
        not_reproduced()
    );
}

/// An `on origin` capture is read at the first position, not as a function
/// of the dwell's time: over `Shuttle`'s moving dwell `v0 = 0`, so the claim
/// needing `v >= 2` is reproduced and the one needing `v >= 1`, reached at
/// the end of the dwell, is not.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn an_origin_capture_over_a_moving_dwell_is_read_at_the_first_position() {
    let graph = shuttle();
    let trace = shuttle_lasso();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Origin"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "OriginHeld"), &trace),
        not_reproduced()
    );
}

/// The work units one exact replay spends.
fn units(graph: &Unit, name: &str, trace: &HybridTrace) -> u64 {
    let mut meter = HybridReplayMeter::new(u64::MAX);
    super::super::exact_replay(graph, claim(graph, name), trace, &mut meter).expect("unmetered");
    meter.consumed()
}

/// The activation scan is charged to the caller's meter before each step:
/// the exact units each claim spends are pinned (each charge site of the
/// scan contributes to at least one of them, so a deleted charge changes a
/// count), and a bound one below a claim's units is exhausted, never a
/// panic or an answer.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn the_activation_scan_is_charged_to_the_meter() {
    let shuttle = shuttle();
    let lasso = shuttle_lasso();
    let ramp = check(RAMP);
    let ramp_trace = ramp_trace();
    let phase = check(PHASE);
    let phase_trace = phase_lasso();
    let cases = [
        (&shuttle, "Cap", &lasso, 1753),
        (&shuttle, "Range", &lasso, 1370),
        (&shuttle, "Past", &lasso, 46),
        (&shuttle, "Origin", &lasso, 36),
        (&ramp, "Nested", &ramp_trace, 17181),
        (&phase, "Twice", &phase_trace, 71),
        (&ramp, "Cube", &ramp_trace, 1707),
        (&ramp, "Touch", &ramp_trace, 4043),
        (&ramp, "Moves", &ramp_trace, 717),
        (&ramp, "Meet", &ramp_trace, 5241),
        (&ramp, "Shared", &ramp_trace, 4622),
        (&ramp, "Pole", &ramp_trace, 1884),
        (&ramp, "LeadHolds", &ramp_trace, 12458),
        (&ramp, "CubeLeadHolds", &ramp_trace, 91496),
        (&ramp, "Lead3", &ramp_trace, 104904),
        (&ramp, "Sqrt", &ramp_trace, 1480),
        (&ramp, "BigAtoms", &ramp_trace, 7415),
        (&ramp, "BigMoves", &ramp_trace, 1486),
        (&ramp, "CurveSq", &ramp_trace, 16425),
        (&ramp, "CurveRoot", &ramp_trace, 3332),
        (&ramp, "CurveDip", &ramp_trace, 92983),
        (&ramp, "CurveDown", &ramp_trace, 6591),
        (&ramp, "CurveMerge", &ramp_trace, 11540),
        (&ramp, "CurveTwice", &ramp_trace, 6250),
        (&ramp, "CurveBig", &ramp_trace, 7399),
        (&ramp, "CurveCirc", &ramp_trace, 25820),
        (&ramp, "CrossPair", &ramp_trace, 62283),
        (&ramp, "TwoBranch", &ramp_trace, 21149),
        (&ramp, "TwoBranchQ", &ramp_trace, 4615),
        (&ramp, "DipA", &ramp_trace, 33949),
        (&ramp, "DipC", &ramp_trace, 96879),
    ];
    for (graph, name, trace, expected) in cases {
        let spent = units(graph, name, trace);
        assert_eq!(spent, expected, "{name}");
        assert_eq!(
            super::super::exact_replay(
                graph,
                claim(graph, name),
                trace,
                &mut HybridReplayMeter::new(spent - 1)
            ),
            Err(HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits
            }),
            "{name}"
        );
    }
}

/// The first undefined evaluation a replay of `name` over `trace` reports.
fn undefined(graph: &Unit, name: &str, trace: &HybridTrace) -> (UndefinedCause, ExactReal) {
    let Ok(ExactReplay::Reproduced(found)) = exact_replay(graph, claim(graph, name), trace) else {
        panic!("{name} reproduces");
    };
    let HybridCounterexampleKind::UndefinedEvaluation { at, cause } = &found.kind else {
        panic!("{name}: an undefined evaluation: {:?}", found.kind);
    };
    (*cause, at.time.clone())
}

/// FR-450-AC-12 (irrational critical instant): over `Ramp` the capture
/// `v0 = tau` makes `v0^3 != 4` false at exactly the activation instant
/// `tau = 4^(1/3)`, an irrational critical value of the projection (the root
/// of `tau^3 - 4`). The obligation is false there only, so the claim is
/// reproduced; with `or h.v >= 0` it is true there too and the claim holds.
#[trace("TC-1221")]
#[test]
fn an_obligation_false_at_an_irrational_activation_instant_is_reproduced() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Cube"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "CubeHeld"), &trace),
        not_reproduced()
    );
}

/// FR-450-AC-12 (irrational critical instant, atom depending on `tau`):
/// at `tau = 4^(1/3)` the atom `h.v != v0 + 1` is false at the irrational
/// instant `tau + 1 = 1 + 4^(1/3)`, inside the window `[tau, tau + 2]`, and
/// `v0^3 != 4` is false there too, so `always [0, 2]` of their disjunction
/// is false at that one activation instant and the claim is reproduced.
/// With `v0 + 3` the instant `tau + 3` lies outside the window and every
/// obligation holds.
#[trace("TC-1221")]
#[test]
fn an_atom_depending_on_the_activation_instant_is_read_at_an_irrational_one() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Meet"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "MeetHeld"), &trace),
        not_reproduced()
    );
}

/// FR-450-AC-12 (irrational instant read at an irrational instant):
/// `eventually [1, 1]` reads the atom `h.v^3 != 4` at `tau + 1`, which is the
/// irrational `4^(1/3)` exactly at `tau = 4^(1/3) - 1`, the root of
/// `(tau + 1)^3 - 4`, where the obligation alone is false.
#[trace("TC-1221")]
#[test]
fn an_atom_read_at_an_irrational_instant_of_an_irrational_activation_is_decided() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Touch"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "TouchHeld"), &trace),
        not_reproduced()
    );
}

/// FR-450-AC-12 (b): the atom `1 / (h.v - v0)` has no value at `t = v0 =
/// tau`, an instant that moves with the activation instant along the line
/// `x = tau` over the open cell `(0, 1)` of activation instants: the
/// earliest instant with no value has infimum 0, not attained. With
/// `h.v - v0 - 1` the line is `x = tau + 1` and the infimum is 1. With
/// `h.v - 2` the instant 2 is the same for every `tau`.
#[trace("TC-1221")]
#[test]
fn an_atom_with_no_value_at_a_moving_instant_reports_its_infimum() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        undefined(&graph, "Moves", &trace),
        (UndefinedCause::DivisionByZero, ExactReal::Rational(q(0, 1)))
    );
    assert_eq!(
        undefined(&graph, "MovesOne", &trace),
        (UndefinedCause::DivisionByZero, ExactReal::Rational(q(1, 1)))
    );
    assert_eq!(
        undefined(&graph, "Fixed", &trace),
        (UndefinedCause::DivisionByZero, ExactReal::Rational(q(2, 1)))
    );
}

/// FR-450 "Exact replay" has no acceptance criterion for irrational
/// activation instants or for the infimum of a moving instant with no
/// value, so these tests carry the test case alone.
///
/// An algebraic instant's polynomial is square-free, not irreducible: the
/// root `4^(1/3)` of `(t - 7)(t^3 - 4)` shares the factor `t - 7` with the
/// atom `(tau - 7)(x - 3)`, whose resultant with it vanishes. The cells are
/// cut at `x = 3` all the same: at `tau = 4^(1/3)` the atom reads
/// `(tau - 7)(tau - 3) > 0` at `x = tau`, so the body is false there and
/// only there, and the claim is reproduced. The root `3` of
/// `(t - 7)(t - 3)` is a rational cut read as a root of a reducible
/// polynomial; the body is false at `tau = 3` alone.
#[trace("TC-1221")]
#[test]
fn a_shared_factor_with_the_instants_polynomial_does_not_hide_a_cut() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Shared"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Reducible"), &trace),
        Ok(reproduced_formula(&trace))
    );
}

/// The degenerate eliminations. The guards hold at one activation instant
/// only, so the reading there decides the claim. At `tau = sqrt 2` the
/// leading coefficient in `x` of `(tau^2 - 2) x^2 - 1` vanishes and the
/// atom drops to the constant -1: `< 0` holds at every instant (no
/// counterexample) and `(tau^2 - 2) x^2 + 1 < 0` is false there, a
/// counterexample. A closed guard `tau^2 >= 2` adds instants where the sign
/// is that of `tau^2 - 2` (nonnegative): `+ 1 > 0` always holds. The same
/// at `tau = 4^(1/3)` with a cubic leading coefficient in `tau` and a cubic
/// in `x`. `(tau^3 - 4)(x - 3)` vanishes for every `x` at that instant: the
/// atom is false for every `x` there (`eventually` finds no `x`) and true
/// elsewhere, so the claim is reproduced.
#[trace("TC-1221")]
#[test]
fn a_vanishing_leading_coefficient_and_a_polynomial_vanishing_everywhere_are_decided() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    for (name, reproduced) in [
        ("LeadHolds", false),
        ("LeadFails", true),
        ("LeadClosed", false),
        ("CubeLeadHolds", false),
        ("CubeLeadFails", true),
        ("Vanish", true),
    ] {
        assert_eq!(
            exact_replay(&graph, claim(&graph, name), &trace),
            if reproduced {
                Ok(reproduced_formula(&trace))
            } else {
                not_reproduced()
            },
            "{name}"
        );
    }
}

/// The work a replay of `name` charges before `bound` is exhausted, from a
/// thread the test gives up on after a minute: a meter that did not bound
/// the work would hang here, not pass.
fn charged_within(name: &'static str, bound: u64) -> (Result<(), HybridReplayRefusal>, u64) {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let graph = check(RAMP);
        let trace = ramp_trace();
        let mut meter = HybridReplayMeter::new(bound);
        let found = super::super::exact_replay(&graph, claim(&graph, name), &trace, &mut meter);
        let _ = sender.send((found.map(|_| ()), meter.consumed()));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(60))
        .unwrap_or_else(|_| panic!("{name} ran past a minute under a meter of {bound}"))
}

/// The meter bounds the time an algebraic activation instant takes: a
/// leading coefficient cubic in `tau` and a quadratic or cubic in `x` took
/// seconds to minutes before the coefficients were reduced modulo `tau`'s
/// polynomial and each step charged by the size of its operands. Under a
/// small bound they are exhausted at once, and the units a claim spends grow
/// with the degrees it reads.
#[trace("TC-1221")]
#[test]
fn the_meter_bounds_the_work_at_an_algebraic_activation_instant() {
    for name in ["Lead3", "Lead3b", "Lead3c"] {
        let (found, consumed) = charged_within(name, 20_000);
        assert_eq!(
            found,
            Err(HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits
            }),
            "{name}"
        );
        assert!(consumed <= 20_000, "{name}");
    }
    // The infimum of a curved instant of degree 5, and of one with a
    // coefficient of 100 bits, under a small and a middling meter.
    for name in ["CurveHi5", "CurveBig5"] {
        for bound in [2_000, 20_000, 100_000, 300_000] {
            let (found, consumed) = charged_within(name, bound);
            assert_eq!(
                found,
                Err(HybridReplayRefusal::ResourceExhausted {
                    counter: LimitKind::WorkUnits
                }),
                "{name} {bound}"
            );
            assert!(consumed <= bound, "{name} {bound}");
        }
    }
    let graph = check(RAMP);
    let trace = ramp_trace();
    let (square, cube, linear) = (
        units(&graph, "LeadHolds", &trace),
        units(&graph, "CubeLeadHolds", &trace),
        units(&graph, "Lead3", &trace),
    );
    assert!(cube > square, "{square} {cube}");
    assert!(linear > 4 * square, "{square} {linear}");
}

/// A rational limit of a curved moving instant is reported as the rational
/// (FR-450: the point interval), not as a root with an isolating interval:
/// the two branches of `(x - 2 tau)(x - 3 + tau)` meet at 1 (`tau = 1`), the
/// branches of `(x - 1)^2 = tau` leave 0 and 1/2 as their limits, and the
/// minimum of `x = (tau - 2)^2 (tau + 1) + 3` is 3.
#[trace("TC-1221")]
#[test]
fn a_rational_limit_of_a_curved_instant_is_reported_as_a_point() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    for (name, expected) in [
        ("CrossPair", q(1, 1)),
        ("TwoBranch", q(0, 1)),
        ("TwoBranchQ", q(1, 2)),
        ("DipC", q(3, 1)),
        ("DipA", q(13, 10)),
    ] {
        let (cause, time) = undefined(&graph, name, &trace);
        assert_eq!(cause, UndefinedCause::DivisionByZero, "{name}");
        assert_eq!(time, ExactReal::Rational(expected), "{name}");
        assert!(matches!(time, ExactReal::Rational(_)), "{name}: {time:?}");
    }
}

/// The cut at the stationary polynomial is needed for soundness: over
/// `(0, 2)` the curve `x = (tau - 7/10)^2 + 13/10` has its minimum 13/10 at
/// `tau = 7/10`, strictly inside the cell and on no other cut, so without the
/// cut the limits at the cell's ends alone give a later instant (with the cut
/// disabled the claim reports about 1.394, and no gap).
#[trace("TC-1221")]
#[test]
fn the_minimum_inside_a_cell_is_the_infimum_of_a_curved_instant() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    let (_, time) = undefined(&graph, "DipA", &trace);
    assert_eq!(time, ExactReal::Rational(q(13, 10)));
    assert!(time < ExactReal::Rational(q(139, 100)));
}

/// A unit of the meter costs about the same wall time on every path
/// (QSL-2724): a big-rational product costs the square of its operands'
/// words, and the algebraic paths (reduction modulo `tau`'s polynomial, gcds,
/// Sturm sequences, resultants, the curve's root isolation) used to charge
/// it linearly and ran 50 to 300 times slower per unit than the rational
/// ones. The slowest and fastest per-unit times of a spread of claims stay
/// within a factor of 50, and a degree-5 curve or guard that took minutes
/// is exhausted under a small meter within seconds.
#[trace("TC-1221")]
#[test]
fn a_unit_of_the_meter_costs_the_same_time_on_every_path() {
    let mut rates = Vec::new();
    for (name, bound) in [
        ("Nested", 300_000),
        ("Lead3", 300_000),
        ("Lead3b", 300_000),
        ("CurveBig", 300_000),
        ("CurveSq", 300_000),
        ("CurveHigh", 300_000),
        ("Worst", 300_000),
        ("TauDeg", 300_000),
        ("CurveHi5", 20_000),
        ("CurveHi5", 100_000),
        ("CurveHi5", 300_000),
        ("CurveBig5", 20_000),
        ("CurveBig5", 100_000),
        ("CurveBig5", 300_000),
    ] {
        let started = std::time::Instant::now();
        let (_, consumed) = charged_within(name, bound);
        let elapsed = started.elapsed();
        assert!(consumed <= bound, "{name}");
        if elapsed.as_millis() >= 20 {
            rates.push((
                name,
                bound,
                elapsed.as_nanos() / u128::from(consumed.max(1)),
            ));
        }
    }
    assert!(rates.len() >= 8, "{rates:?}");
    let slowest = rates.iter().map(|(_, _, r)| *r).max().unwrap_or(0);
    let fastest = rates.iter().map(|(_, _, r)| *r).min().unwrap_or(1).max(1);
    assert!(slowest <= 50 * fastest, "ns per unit: {rates:?}");
}

/// An expression with no value only at the first-pass sample of an open
/// cell is placed once the cell is cut: `1 / ((tau - 5)^2 + (x - 3)^2)` has
/// no value at `(5, 3)` alone, and `(0, 10)` is sampled at 5. The record is
/// the point `(5, 3)`, reported at 3; the same shape at `tau = 4` is
/// reported at 3 as well.
#[trace("TC-1221")]
#[test]
fn an_expression_with_no_value_at_the_first_sample_is_placed_after_the_cell_is_cut() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    for name in ["Pole", "PoleFour"] {
        assert_eq!(
            undefined(&graph, name, &trace),
            (UndefinedCause::DivisionByZero, ExactReal::Rational(q(3, 1))),
            "{name}"
        );
    }
}

/// An irrational instant with no value that is the same for every `tau` is
/// kept as it is: `1 / (x^2 - 2)` has no value at `sqrt 2`, attained.
#[trace("TC-1221")]
#[test]
fn an_irrational_instant_with_no_value_for_every_activation_instant_is_reported() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    let (cause, time) = undefined(&graph, "Sqrt", &trace);
    assert_eq!(cause, UndefinedCause::DivisionByZero);
    assert!(matches!(time, ExactReal::Root(_)));
    assert_eq!(
        time.sign_of(&Poly::new(vec![q(-2, 1), q(0, 1), q(1, 1)])),
        Ordering::Equal
    );
}

/// The infimum of `x = tau + c` with a negative `c`: `1 / (x - tau + 1)` has
/// no value at `x = tau - 1`, `tau` in `(1, 2)`, infimum 0. With an
/// irrational start of the cell, `tau^2 > 2`, the atom `1 / (x - tau)` has
/// infimum `sqrt 2`.
#[trace("TC-1221")]
#[test]
fn the_infimum_of_a_moving_instant_shifts_a_negative_offset_and_an_irrational_start() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    assert_eq!(
        undefined(&graph, "Back", &trace),
        (UndefinedCause::DivisionByZero, ExactReal::Rational(q(0, 1)))
    );
    let (cause, time) = undefined(&graph, "Irrational", &trace);
    assert_eq!(cause, UndefinedCause::DivisionByZero);
    assert_eq!(
        time.sign_of(&Poly::new(vec![q(-2, 1), q(0, 1), q(1, 1)])),
        Ordering::Equal
    );
}

/// FR-450-AC-12 (budget): three nested windows over a tau-dependent atom.
/// The atom `4 <= v <= 4 + 12 tau` holds on `[4, 4 + 12 tau]` of a ramp that
/// ends at time 40, so `eventually [0, 1]` holds at `[3, 4 + 12 tau]`,
/// `always [1, 6]` at `[2, 12 tau - 2]`, and `eventually [2, 9]` from `tau`
/// needs `12 tau - 2 >= tau + 2`, that is `tau >= 4/11`. The obligation is
/// false for `tau` in `(0, 4/11)`, so the claim is reproduced. Shifted copies
/// are made of the polynomials read, not of the window lines (which already
/// hold their differences), and that keeps the decision within 400,000 work
/// units; with every line copied it exhausts that meter.
#[trace("TC-1221", "FR-450-AC-12")]
#[test]
fn three_nested_windows_are_decided_within_a_moderate_meter() {
    let graph = check(RAMP);
    let trace = HybridTrace {
        end: Some(before(n(40), &[n(40), n(0)])),
        ..ramp_trace()
    };
    let mut meter = HybridReplayMeter::new(1_000_000);
    assert_eq!(
        super::super::exact_replay(&graph, claim(&graph, "Deep"), &trace, &mut meter),
        Ok(reproduced_formula(&trace))
    );
}

/// The infimum of an instant with no value that moves along a curve other
/// than `x = tau + c` (QSL-2706): the lesser of the limits of the root curve
/// at the two ends of the cell of activation instants, not attained.
/// Hand-derived: `x = tau^2 + tau` over `tau^2 > 2`, `tau < 3` starts at the
/// irrational `tau = sqrt 2`, where it is `2 + sqrt 2`, the root above 3 of
/// `x^2 - 4x + 2`; `x^2 = tau + 3` over `(0, 1)` starts at `sqrt 3`;
/// `x = 5 - tau` over `(1, 3)` ends at 2; `x^2 = tau` over `(0, 1)` merges its
/// two branches at 0; `(x - 3)(x - 2 tau)` over `(1, 2)` has the vertical
/// factor `x - 3` (the curve `x = 2 tau` starts at 2, below 3 until
/// `tau = 3/2`); `(x - 3 tau) 10^300` over `(1, 2)` starts at 3.
#[trace("TC-1221")]
#[test]
fn the_infimum_of_a_curved_moving_instant_is_the_limit_of_its_root_curve() {
    let graph = check(RAMP);
    let trace = ramp_trace();
    for (name, expected) in [
        ("CurveDown", 2),
        ("CurveMerge", 0),
        ("CurveTwice", 2),
        ("CurveBig", 3),
        ("CurveDip", 1),
    ] {
        assert_eq!(
            undefined(&graph, name, &trace),
            (
                UndefinedCause::DivisionByZero,
                ExactReal::Rational(q(expected, 1))
            ),
            "{name}"
        );
    }
    let (_, time) = undefined(&graph, "CurveRoot", &trace);
    assert_eq!(
        time.sign_of(&Poly::new(vec![q(-3, 1), q(0, 1), q(1, 1)])),
        Ordering::Equal
    );
    assert!(time > ExactReal::Rational(q(0, 1)));
    let (_, time) = undefined(&graph, "CurveCirc", &trace);
    assert_eq!(
        time.sign_of(&Poly::new(vec![q(2, 1), q(-4, 1), q(1, 1)])),
        Ordering::Equal
    );
    assert!(time > ExactReal::Rational(q(1, 2)) && time < ExactReal::Rational(q(3, 5)));
    let (_, time) = undefined(&graph, "CurveSq", &trace);
    assert_eq!(
        time.sign_of(&Poly::new(vec![q(2, 1), q(-4, 1), q(1, 1)])),
        Ordering::Equal
    );
    assert!(time > ExactReal::Rational(q(3, 1)));
}
