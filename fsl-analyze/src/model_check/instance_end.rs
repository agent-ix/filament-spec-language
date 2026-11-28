// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one classifier of how a stop ends a multi-instance model-check item
//! (FR-126 "TP-4 items"; arch rulings Q1 to Q3 on QSL-3601).

use fsl_replay::{IncompleteCause, ModelCheckLimit, ReachedLimit};

/// What a stop ends.
#[allow(dead_code)] // read by the liveness and digital folds
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum End {
    /// The whole run: no later instance is decided. `cancel` is set for a
    /// cancellation, which overrides an earlier instance's outcome.
    Run {
        /// Whether the stop is a cancellation.
        cancel: bool,
    },
    /// The one instance: the next instance is decided.
    Instance,
}

/// FR-126 "TP-4 items": the run-level ends are `max_states`,
/// `max_transitions`, the evaluation meter, `analysis.retained_bytes` and
/// cancellation (Q1, Q2); `max_clocks` is a property of the subject, the same
/// for every instance, so it ends the run too. Every other stop, including an
/// expansion stop that names no limit, ends its instance only (Q3).
#[allow(dead_code)] // read by the liveness and digital folds
pub(crate) fn instance_end(cause: &IncompleteCause, limit: Option<&ReachedLimit>) -> End {
    if matches!(cause, IncompleteCause::Cancelled { .. }) {
        return End::Run { cancel: true };
    }
    match limit.map(|reached| &reached.limit) {
        Some(
            ModelCheckLimit::MaxStates
            | ModelCheckLimit::MaxTransitions
            | ModelCheckLimit::EvaluationMeter { .. }
            | ModelCheckLimit::AnalysisRetainedBytes
            | ModelCheckLimit::MaxClocks,
        ) => End::Run { cancel: false },
        _ => End::Instance,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn reached(limit: ModelCheckLimit) -> ReachedLimit {
        ReachedLimit { limit, value: 1 }
    }

    fn end(limit: ModelCheckLimit) -> End {
        instance_end(&IncompleteCause::ResourceExhausted, Some(&reached(limit)))
    }

    /// FR-126-AC-15: `max_states` stops the run; the closed run-level list of
    /// FR-126 "TP-4 items" is the only list that does.
    #[trace("FR-126-AC-15", "FR-126-AC-30")]
    #[test]
    fn only_the_listed_limits_end_the_run() {
        for run in [
            ModelCheckLimit::MaxStates,
            ModelCheckLimit::MaxTransitions,
            ModelCheckLimit::AnalysisRetainedBytes,
            ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::WorkUnits,
            },
        ] {
            assert_eq!(end(run.clone()), End::Run { cancel: false }, "{run:?}");
        }
        for binding in [
            ModelCheckLimit::MaxCandidates,
            ModelCheckLimit::MaxAutomatonStates,
        ] {
            assert_eq!(end(binding.clone()), End::Instance, "{binding:?}");
        }
    }

    /// FR-126-AC-31: a cancellation ends the run and overrides.
    #[trace("FR-126-AC-31")]
    #[test]
    fn a_cancellation_ends_the_run_and_overrides() {
        let cancelled = IncompleteCause::Cancelled {
            source: filament_exact::CancelCause::Requested,
        };
        assert_eq!(instance_end(&cancelled, None), End::Run { cancel: true });
    }

    /// Arch Q3: an expansion stop with no named limit ends its instance.
    #[trace("FR-126-AC-30")]
    #[test]
    fn a_stop_with_no_named_limit_ends_the_instance_only() {
        assert_eq!(
            instance_end(&IncompleteCause::ResourceExhausted, None),
            End::Instance
        );
    }
}
