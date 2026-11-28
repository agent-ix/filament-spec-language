// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker's own value of a plain deadline monitor's tail-entry state
//! (FR-196 "Idle tail of a plain deadline monitor", FR-921).
//!
//! Written from the spec table alone: it shares no code with the producer's
//! function in `fsl-analyze`, and it is called on the key's readings, never on
//! a value the document states.

use fsl_eval::probabilistic::{DeadlineKind, DeadlinePhase};
use ix_trace_rs::implements;

/// Whether `e(q)` accepts, for a monitor of `kind` in `phase`.
///
/// `window_reachable` is `z <= D` at `q`; `body_holds` is the body (`B`) at
/// `q`'s letter; `maximise` is true when the scheduler wants acceptance (the
/// claim's bound reads the maximum) and false when it wants rejection.
#[implements("FR-196-AC-20")]
pub(super) fn tail_accepts(
    kind: DeadlineKind,
    phase: DeadlinePhase,
    window_reachable: bool,
    body_holds: bool,
    maximise: bool,
) -> bool {
    match (phase, kind, maximise) {
        (DeadlinePhase::Accept, _, _) => true,
        (DeadlinePhase::Reject, _, _) => false,
        // eventually / until, maximum: a stutter must land in the window with
        // the body true.
        (DeadlinePhase::Pending, DeadlineKind::Eventually | DeadlineKind::Until, true) => {
            body_holds && window_reachable
        }
        // eventually / until, minimum: the scheduler places no stutter.
        (DeadlinePhase::Pending, DeadlineKind::Eventually | DeadlineKind::Until, false) => false,
        // always, maximum: the scheduler places no stutter, nothing violates.
        (DeadlinePhase::Pending, DeadlineKind::Always, true) => true,
        // always, minimum: a violating stutter lands iff the body fails at q
        // and the window is reachable.
        (DeadlinePhase::Pending, DeadlineKind::Always, false) => !(window_reachable && !body_holds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-196's table, hand-copied: kind, maximum, `z <= D`, `B` at `q`, value.
    #[trace("FR-196-AC-20")]
    #[test]
    fn the_checker_tail_value_matches_the_24_hand_rows() {
        use DeadlineKind::{Always, Eventually, Until};
        let rows: [(DeadlineKind, bool, bool, bool, bool); 24] = [
            (Eventually, true, true, true, true),
            (Eventually, true, true, false, false),
            (Eventually, true, false, true, false),
            (Eventually, true, false, false, false),
            (Eventually, false, true, true, false),
            (Eventually, false, true, false, false),
            (Eventually, false, false, true, false),
            (Eventually, false, false, false, false),
            (Until, true, true, true, true),
            (Until, true, true, false, false),
            (Until, true, false, true, false),
            (Until, true, false, false, false),
            (Until, false, true, true, false),
            (Until, false, true, false, false),
            (Until, false, false, true, false),
            (Until, false, false, false, false),
            (Always, true, true, true, true),
            (Always, true, true, false, true),
            (Always, true, false, true, true),
            (Always, true, false, false, true),
            (Always, false, true, true, true),
            (Always, false, true, false, false),
            (Always, false, false, true, true),
            (Always, false, false, false, true),
        ];
        for (kind, maximise, window, body, value) in rows {
            assert_eq!(
                tail_accepts(kind, DeadlinePhase::Pending, window, body, maximise),
                value,
                "{kind:?} maximise={maximise} window={window} body={body}"
            );
        }
    }

    #[trace("FR-196-AC-20")]
    #[test]
    fn a_decided_monitor_keeps_its_verdict_in_the_checker_tail() {
        for kind in [
            DeadlineKind::Eventually,
            DeadlineKind::Until,
            DeadlineKind::Always,
        ] {
            for bits in 0..8_u8 {
                let (maximise, window, body) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
                assert!(tail_accepts(
                    kind,
                    DeadlinePhase::Accept,
                    window,
                    body,
                    maximise
                ));
                assert!(!tail_accepts(
                    kind,
                    DeadlinePhase::Reject,
                    window,
                    body,
                    maximise
                ));
            }
        }
    }
}
