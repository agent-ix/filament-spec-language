// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-038-AC-644 and FR-038-AC-646: an operation member FSL builds from its
//! typed members is written by the IR's `from_form` and read back by the
//! strict reader as the same operation.

use filament_contract_model::{CheckedApplicationOperation, ExecutionPoint, ExecutionPointName};
use filament_exact::Identifier;
use fsl_foundation::bound::{TemporalInterval, TemporalUpper};
use fsl_forms::StateClauseKind;
use fsl_semantics::value::{AcceptedPoint, Member};

use super::*;

/// The operation the emitter writes for `member` alone, read back strictly.
fn written(member: &Member) -> (Value, CheckedApplicationOperation) {
    let (typed, point) = member_value(&StructuralIds::default(), member).expect("typed member");
    let operation =
        CheckedApplicationOperation::from_form(&filament_contract_model::CheckedApplicationOperationForm {
            identity: "filament.op.test.member".into(),
            laws: Vec::new(),
            mode: None,
            member: Some(typed),
            execution_point: point,
            leaves: Vec::new(),
        })
        .expect("the form writes");
    let wire = serde_json::to_value(&operation).expect("operation wire");
    let read: CheckedApplicationOperation =
        serde_json::from_value(wire.clone()).expect("strict read of the write");
    assert_eq!(read, operation, "the strict read is the written operation");
    (wire["member"].clone(), operation)
}

/// Every id-free member kind the emitter builds writes its closed wire member
/// and reads back as the same operation.
#[trace("FR-038-AC-644", "FR-038-AC-646")]
#[test]
fn emitted_operation_members_write_then_strictly_read_back() {
    let cases = [
        (
            Member::ProfileOperator {
                operator: Identifier::new("add").expect("identifier"),
            },
            json!({"kind":"profile_operator","operator":"add"}),
        ),
        (
            Member::StateClause {
                clause: StateClauseKind::Precondition,
                execution_point: Some(AcceptedPoint::new(ExecutionPoint::Pre {
                    operation: ExecutionPointName::new("go").expect("name"),
                })),
            },
            json!({"kind":"state_clause","clause":"precondition",
                "execution_point":{"kind":"pre","operation":"go"}}),
        ),
        (
            Member::TemporalInterval {
                interval: Some(
                    TemporalInterval::new(1, TemporalUpper::Finite(3)).expect("interval"),
                ),
            },
            json!({"kind":"temporal_interval","interval":{"lower":"1","upper":"3"}}),
        ),
        (
            Member::TemporalInterval { interval: None },
            json!({"kind":"temporal_interval","interval":null}),
        ),
    ];
    for (member, wire) in cases {
        let (member_wire, _) = written(&member);
        assert_eq!(member_wire, wire, "wire bytes of {member:?}");
    }
}
