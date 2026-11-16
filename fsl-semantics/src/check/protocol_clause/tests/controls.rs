// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S3 control-step classification for protocol-only constructs.

use ix_trace_rs::trace;

use super::{checks, recovery_flow, refusals};
use crate::check::protocol_clause::ControlStep;

/// FR-426-AC-9, FR-427-AC-6: finish is stepped by the evaluator and the
/// compensation template is driven by its compensation threads.
#[trace("FSpec-FR-426-AC-9", "FSpec-FR-427-AC-6")]
#[test]
fn no_specified_control_is_unstepped() {
    let graph = checks(&recovery_flow("Main::Committed", ""));
    let protocol = graph.protocol("RecoveryFlow").expect("checked protocol");
    assert!(
        protocol
            .controls
            .controls
            .iter()
            .any(|control| control.step() == ControlStep::Finish),
        "the protocol finish has a stepped control"
    );
    assert!(
        protocol
            .controls
            .controls
            .iter()
            .any(|control| control.step() == ControlStep::CompensateTemplate),
        "the compensation declaration has an evaluator-owned control"
    );
    assert!(
        protocol
            .controls
            .controls
            .iter()
            .all(|control| !matches!(control.step(), ControlStep::Unstepped(_))),
        "neither specified control remains unstepped"
    );
}

/// A recovery predicate remains restricted to its declared stage binders.
#[trace("FSpec-FR-427-AC-6")]
#[test]
fn a_recovery_predicate_over_an_unbound_name_refuses() {
    let source = recovery_flow("Main::Committed", "").replace(
        "recover (recovery: Boolean) { true }",
        "recover (recovery: Boolean) { outside }",
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "the recovery expression refuses once");
    assert_eq!(found[0].cause.code().as_str(), "missing_declaration");
    assert_eq!(found[0].cause.cause(), Some("missing-name"));
}
