// SPDX-License-Identifier: AGPL-3.0-or-later
//! The tail value of a plain deadline monitor's tail-entry state (FR-196
//! "Idle tail of a plain deadline monitor", FR-196-AC-20).
//!
//! The value is a function of the key alone: the monitor kind and phase, the
//! deadline clock's place (is the window `[0, D]` still reachable, `z <= D`),
//! `q`'s letter, and the side the claim's bound makes the scheduler take. The
//! checker computes the same table itself and shares no code with this
//! function (FR-921).

use ix_trace_rs::implements;

use super::{DeadlineKind, DeadlinePhase};

/// Whether the tail-entry state of `kind`, read under `phase`, accepts.
///
/// `window_reachable` is `z <= D` at `q`; `body_holds` is the monitor's body
/// (`B`) at `q`'s letter; `maximise` is true when the scheduler wants
/// acceptance (a maximum) and false when it wants rejection (a minimum).
#[implements("FR-196-AC-20")]
pub(super) fn plain_deadline_tail_accepts(
    kind: DeadlineKind,
    phase: DeadlinePhase,
    window_reachable: bool,
    body_holds: bool,
    maximise: bool,
) -> bool {
    match phase {
        DeadlinePhase::Accept => true,
        DeadlinePhase::Reject => false,
        DeadlinePhase::Pending => match kind {
            // The scheduler may place no stutter in the window, so a minimum
            // always rejects; a maximum accepts only when a stutter can land
            // and `q`'s letter satisfies the body.
            DeadlineKind::Eventually | DeadlineKind::Until => {
                maximise && body_holds && window_reachable
            }
            // The scheduler may place no stutter, so a maximum always
            // accepts; a minimum rejects only when a stutter can land and
            // `q`'s letter violates the body.
            DeadlineKind::Always => maximise || body_holds || !window_reachable,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-196's table, one row per case: kind, side, `z <= D`, `B` at `q`,
    /// the hand value. Three kinds, two sides, two window places, two letters.
    #[trace("FR-196-AC-20")]
    #[test]
    fn plain_deadline_tail_value_table_matches_the_hand_rows() {
        use DeadlineKind::{Always, Eventually, Until};
        // (kind, maximum, window reachable, B holds, value)
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
                plain_deadline_tail_accepts(kind, DeadlinePhase::Pending, window, body, maximise),
                value,
                "{kind:?} maximise={maximise} window={window} body={body}"
            );
        }
    }

    /// A decided monitor keeps its verdict whatever the tail could do.
    #[trace("FR-196-AC-20")]
    #[test]
    fn a_decided_monitor_keeps_its_verdict_in_the_tail() {
        for kind in [
            DeadlineKind::Eventually,
            DeadlineKind::Until,
            DeadlineKind::Always,
        ] {
            for maximise in [true, false] {
                for window in [true, false] {
                    for body in [true, false] {
                        assert!(plain_deadline_tail_accepts(
                            kind,
                            DeadlinePhase::Accept,
                            window,
                            body,
                            maximise
                        ));
                        assert!(!plain_deadline_tail_accepts(
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
    }
}
