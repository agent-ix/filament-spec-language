// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native cancellation survives the serialized-context error boundary.

use filament_exact::{Cancel, CancelCause};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_replay::replay_context::{NativeContextError, TemporalReadbackError};
use ix_trace_rs::trace;

/// FR-621-AC-20: retain the actual expansion owner's catalog, limit and
/// cancellation cause when exposing a native readback stop externally.
#[test]
#[trace("FR-621-AC-20")]
fn cancelled_expansion_keeps_original_native_catalog() {
    let world = crate::clause_item::world();
    let subject = world.subject(&["c"], crate::counter::UNLIMITED);
    let cancel = Cancel::new();
    let system = ModelSystem::new_with_cancel(subject.simulation, &cancel).unwrap();
    let state = system.initial().into_iter().next().unwrap();
    system.cancel_plan_with(&cancel);
    cancel.cancel(CancelCause::Requested);
    let stop = system
        .successors(&state)
        .expect_err("actual cancelled native expansion");
    let expected = stop.cause;
    assert_eq!(system.plan_cancel_cause(), Some(CancelCause::Requested));
    for timed in [false, true] {
        let readback = if timed {
            TemporalReadbackError::TimedStop(fsl_eval::simulation::model::TimedStop::Expansion(
                stop,
            ))
        } else {
            TemporalReadbackError::Expansion { stop }
        };
        let native = NativeContextError::TemporalReadback(Box::new(readback));
        assert_eq!(native.code().as_str(), expected.code());
        assert_eq!(native.cause(), Some(expected.cause()));
        assert_eq!(
            native.category(),
            fsl_foundation::diagnostic::category_of(&expected).unwrap()
        );
        let error = fsl_command::replay_context::ContextReplayError::Native(Box::new(native));
        let output = serde_json::to_value(error.outcome(None)).unwrap();
        assert_eq!(output["diagnostics"][0]["code"], expected.code());
        assert_eq!(output["diagnostics"][0]["cause"], expected.cause());
        assert!(output["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains(&format!("{stop:?}")));
    }
}
