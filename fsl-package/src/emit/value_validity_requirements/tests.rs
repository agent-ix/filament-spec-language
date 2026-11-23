// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scalar requirement facts survive emission and strict native admission.

use std::collections::BTreeSet;

use filament_contract_model::{
    CapabilityKind, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    ValueValidityRecord,
};
use fsl_semantics::check::wire_id::WireGraph;
use ix_trace_rs::trace;

use crate::emit::{emit_checked, Emission};
use crate::CheckedPackage;

const GUARDED: &str = "function Guarded using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x * 2 }";

fn admitted_package(
    declarations: &str,
) -> (CheckedPackage, Emission, CheckedPackageV2) {
    let (_, package) = crate::emit::tests::package_from_text(declarations);
    let emission = emit_checked(&package, crate::EmitLimits::fixture()).expect("scalar unit emits");
    let admitted = match CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        CheckedPackageV2ReadResult::Admitted(admitted) => admitted,
        failure => panic!("the strict reader admits the scalar records: {failure:?}"),
    };
    (package, emission, *admitted)
}

fn emitted_records(
    declarations: &str,
) -> (CheckedPackage, Emission, CheckedPackageV2, Vec<ValueValidityRecord>) {
    let (package, emission, admitted) = admitted_package(declarations);
    let records = admitted
        .requirement_items()
        .iter()
        .filter(|row| row.kind == CapabilityKind::ValueValidity)
        .map(|row| {
            super::value_record(row)
                .expect("each scalar item has its record")
                .clone()
        })
        .collect::<Vec<_>>();
    assert!(
        !records.is_empty(),
        "the arithmetic produces scalar obligations"
    );
    (package, emission, admitted, records)
}

/// The guards of `record`, outermost first, as `(guard node, occurrence, holds)`.
fn chain(
    admitted: &CheckedPackageV2,
    record: &ValueValidityRecord,
) -> Vec<(serde_json::Value, bool)> {
    admitted
        .path_condition(record)
        .iter()
        .map(|guard| (serde_json::to_value(guard.guard).unwrap(), guard.holds))
        .collect()
}

/// The checker's own chain of the record at `claim`, in the wire's guard form.
fn source_chain(
    identities: &WireGraph,
    source: &fsl_semantics::check::RequirementRecord,
) -> Vec<(serde_json::Value, bool)> {
    source
        .path_condition()
        .iter()
        .map(|guard| {
            (
                identities.wire_value(guard.occurrence().node()).unwrap(),
                guard.holds(),
            )
        })
        .collect()
}

/// A bare Boolean parameter keeps its own identity and occurrence as the
/// guard, now held once in the shared frame table: `if p then x + 1 else x * 2`
/// holds exactly the frames `(None, p, true)` and `(None, p, false)`, in that
/// order, and the `+` and `*` records cite them as their terminal frames.
#[trace("FR-940-AC-1", "FR-940-AC-13", "FR-062-AC-17")]
#[test]
fn bare_parameter_guard_is_one_shared_frame_per_outcome() {
    let (package, emission, admitted, records) = emitted_records(GUARDED);
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let identities = WireGraph::of_package_value(&wire).expect("the emitted graph has identities");
    let frames = admitted.path_condition_frames();
    assert_eq!(frames.len(), 2, "one frame per outcome of the one guard");
    assert_eq!(frames[0].parent, None);
    assert_eq!(frames[1].parent, None);
    assert!(frames[0].holds && !frames[1].holds, "then frame first, else frame second");
    assert_eq!(frames[0].guard, frames[1].guard, "both read the one parameter");
    assert_eq!(frames[0].occurrence, frames[1].occurrence);
    assert_eq!(
        frames[0].occurrence, 0,
        "the first `expression` occurrence of the guard node, as the reader derives it"
    );
    let nodes = crate::emit::tests::typed_nodes(&emission);
    let guard = nodes
        .iter()
        .find(|node| node.node_id == frames[0].guard)
        .expect("the guard is a document-held node");
    assert_eq!(guard.node_tag.as_ref(), "value");
    assert_eq!(
        guard.semantic_form.as_ref(),
        "parameter",
        "the frame names the original parameter, without an expression shim"
    );
    let mut terminals = Vec::new();
    let mut guarded = 0;
    for (claim, source) in package.graph().requirements().iter() {
        let application = identities
            .wire_value(claim.node())
            .expect("the claim is emitted");
        let record = records
            .iter()
            .find(|record| {
                serde_json::to_value(&record.application).unwrap() == application
                    && record.occurrence == claim.origin().ordinal()
            })
            .expect("the source claim has an admitted record");
        assert_eq!(
            chain(&admitted, record),
            source_chain(&identities, source),
            "the shared view yields the checker's own chain"
        );
        if source.path_condition().is_empty() {
            assert_eq!(record.path_condition_frame, None);
            continue;
        }
        assert_eq!(source.path_condition().len(), 1);
        terminals.push(record.path_condition_frame.expect("a guarded record cites a frame").0);
        guarded += 1;
    }
    terminals.sort_unstable();
    assert_eq!(terminals, [0, 1], "each branch's record cites its own frame");
    assert_eq!(guarded, 2, "both branches retain their source guard");
}

/// Both arms of an if retain the positive or negative condition of that arm.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn both_guard_holds_outcomes_survive_native_admission() {
    let (package, _, admitted, records) = emitted_records(GUARDED);
    let expected = package
        .graph()
        .requirements()
        .values()
        .flat_map(|source| source.path_condition().iter().map(|guard| guard.holds()))
        .collect::<BTreeSet<_>>();
    let actual = records
        .iter()
        .flat_map(|record| chain(&admitted, record).into_iter().map(|guard| guard.1))
        .collect::<BTreeSet<_>>();
    assert_eq!(expected, BTreeSet::from([false, true]));
    assert_eq!(actual, expected);
}

/// An unconditional arithmetic record cites no frame and the package holds
/// no table.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn unguarded_record_keeps_an_empty_path_condition() {
    let (package, _, admitted, records) =
        emitted_records("function Plain using v(x: Int[0, 9]): Integer pure { x + 1 }");
    assert!(package
        .graph()
        .requirements()
        .values()
        .all(|source| source.path_condition().is_empty()));
    assert!(records
        .iter()
        .all(|record| record.path_condition_frame.is_none()
            && admitted.path_condition(record).is_empty()));
    assert!(admitted.path_condition_frames().is_empty());
}

/// FSpec FR-825 package order over hand-built rows: claim key first, then
/// role spelling, ordinal and capability spelling on ties. The `NodeKey`
/// order used for the claim equals the order of the structural ids the reader
/// compares (`CheckedNodeId`'s order), including nested operand keys and
/// ordinals.
#[trace("FR-940-AC-1")]
#[test]
fn package_order_sorts_by_claim_key_then_role_ordinal_and_kind() {
    use filament_contract_model::{
        CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, RequirementExtent,
        RequirementItemRow,
    };
    use filament_exact::{NodeKey, PathSegment};

    let text = |value: &str| PathSegment::Text(value.to_owned());
    let leaf = |name: &str, ordinal| NodeKey::new("p", vec![text(name)], ordinal).unwrap();
    let nested = |inner: NodeKey| NodeKey::new("p", vec![text("b"), PathSegment::Node(inner)], 0).unwrap();
    let keys = [
        nested(leaf("z", 0)),
        leaf("b", 1),
        nested(leaf("a", 0)),
        leaf("b", 0),
        NodeKey::new("o", vec![text("z")], 0).unwrap(),
        NodeKey::new("p", vec![text("b"), text("c")], 0).unwrap(),
    ];
    let ids = fsl_semantics::check::wire_id::StructuralIds::default();
    let mut by_key = keys.to_vec();
    by_key.sort();
    let mut by_id = keys.to_vec();
    by_id.sort_by_cached_key(|key| crate::emit::node_id(&ids, key));
    assert_eq!(by_key, by_id, "NodeKey order is the reader's structural-id order");

    let claim = CheckedNodeId {
        package: "p".into(),
        path: Vec::new(),
        ordinal: 0,
    };
    let row = |role, ordinal, kind| RequirementItemRow {
        claim: claim.clone(),
        occurrence: CheckedOccurrence { role, ordinal },
        kind,
        extent: RequirementExtent::Bounded,
        record: None,
    };
    let tie = leaf("b", 0);
    let first = leaf("a", 0);
    let rows = vec![
        (tie.clone(), row(CheckedOccurrenceRole::Expression, 1, CapabilityKind::ValueValidity)),
        (tie.clone(), row(CheckedOccurrenceRole::Expression, 0, CapabilityKind::ValueValidity)),
        (tie.clone(), row(CheckedOccurrenceRole::Claim, 3, CapabilityKind::TemporalSatisfaction)),
        (tie.clone(), row(CheckedOccurrenceRole::Expression, 0, CapabilityKind::OperationContract)),
        (first, row(CheckedOccurrenceRole::Generated, 9, CapabilityKind::ValueValidity)),
    ];
    let ordered = super::package_order(rows)
        .into_iter()
        .map(|row| (row.occurrence.role, row.occurrence.ordinal, row.kind))
        .collect::<Vec<_>>();
    assert_eq!(
        ordered,
        [
            (CheckedOccurrenceRole::Generated, 9, CapabilityKind::ValueValidity),
            (CheckedOccurrenceRole::Claim, 3, CapabilityKind::TemporalSatisfaction),
            (CheckedOccurrenceRole::Expression, 0, CapabilityKind::OperationContract),
            (CheckedOccurrenceRole::Expression, 0, CapabilityKind::ValueValidity),
            (CheckedOccurrenceRole::Expression, 1, CapabilityKind::ValueValidity),
        ]
    );
}
