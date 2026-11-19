// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S3 control-step classification for protocol-only constructs.

use ix_trace_rs::trace;

use super::{checks, recovery_flow, refusals};
use crate::check::protocol_clause::{CheckedControl, ControlStep, ProtocolNodeId};

fn control(kind: fsl_forms::ProtocolNodeKind) -> CheckedControl {
    CheckedControl {
        node: ProtocolNodeId(usize::MAX),
        kind,
        name: kind.label().to_owned(),
        path: Vec::new(),
        children: Vec::new(),
    }
}

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
            .all(|control| !matches!(control.step(), ControlStep::Unstepped(_))),
        "neither specified control remains unstepped"
    );
    assert_eq!(
        control(fsl_forms::ProtocolNodeKind::Finish).step(),
        ControlStep::Finish
    );
    assert_eq!(
        control(fsl_forms::ProtocolNodeKind::CompensateTemplate).step(),
        ControlStep::CompensateTemplate
    );
}

/// A recovery predicate remains restricted to its declared stage binders.
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
