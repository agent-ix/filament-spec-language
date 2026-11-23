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

fn emitted_records(declarations: &str) -> (CheckedPackage, Emission, Vec<ValueValidityRecord>) {
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
    let records = admitted
        .requirement_items()
        .iter()
        .filter(|row| row.kind == CapabilityKind::ValueValidity)
        .map(|row| row.record.clone().expect("each scalar item has its record"))
        .collect::<Vec<_>>();
    assert!(
        !records.is_empty(),
        "the arithmetic produces scalar obligations"
    );
    (package, emission, records)
}

/// A bare Boolean parameter keeps its own identity and occurrence as the guard.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn bare_parameter_guard_preserves_source_identity_and_occurrence() {
    let (package, emission, records) = emitted_records(GUARDED);
    let wire: serde_json::Value = serde_json::from_slice(emission.package().bytes()).unwrap();
    let identities = WireGraph::of_package_value(&wire).expect("the emitted graph has identities");
    let mut guarded = 0;
    for (claim, source) in package.graph().requirements().iter() {
        if source.path_condition().is_empty() {
            continue;
        }
        let application = identities
            .wire_value(claim.node())
            .expect("the claim is emitted");
        let record = records
            .iter()
            .find(|record| {
                serde_json::to_value(&record.application).unwrap() == application
                    && record.occurrence == claim.origin().ordinal()
            })
            .expect("the guarded source claim has an admitted record");
        assert_eq!(source.path_condition().len(), 1);
        assert_eq!(record.path_condition.len(), 1);
        let source_guard = &source.path_condition()[0];
        let guard = &record.path_condition[0];
        assert_eq!(
            serde_json::to_value(&guard.guard).unwrap(),
            identities
                .wire_value(source_guard.occurrence().node())
                .unwrap(),
            "the record names the original parameter, without an expression shim"
        );
        assert_eq!(
            guard.occurrence,
            source_guard.occurrence().origin().ordinal()
        );
        assert_eq!(guard.holds, source_guard.holds());
        let node = emission
            .wire_nodes
            .iter()
            .find(|node| node.node_id == guard.guard)
            .expect("the guard is a document-held node");
        assert_eq!(node.node_tag.as_ref(), "value");
        assert_eq!(node.semantic_form.as_ref(), "parameter");
        guarded += 1;
    }
    assert!(guarded >= 2, "both branches retain their source guard");
}

/// Both arms of an if retain the positive or negative condition of that arm.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn both_guard_holds_outcomes_survive_native_admission() {
    let (package, _, records) = emitted_records(GUARDED);
    let expected = package
        .graph()
        .requirements()
        .values()
        .flat_map(|source| source.path_condition().iter().map(|guard| guard.holds()))
        .collect::<BTreeSet<_>>();
    let actual = records
        .iter()
        .flat_map(|record| record.path_condition.iter().map(|guard| guard.holds))
        .collect::<BTreeSet<_>>();
    assert_eq!(expected, BTreeSet::from([false, true]));
    assert_eq!(actual, expected);
}

/// An unconditional arithmetic record carries an empty path-condition tuple.
#[trace("FR-940-AC-1", "FR-940-AC-13")]
#[test]
fn unguarded_record_keeps_an_empty_path_condition() {
    let (package, _, records) =
        emitted_records("function Plain using v(x: Int[0, 9]): Integer pure { x + 1 }");
    assert!(package
        .graph()
        .requirements()
        .values()
        .all(|source| source.path_condition().is_empty()));
    assert!(records
        .iter()
        .all(|record| record.path_condition.is_empty()));
}

/// FSpec FR-825: the timed-model writer re-encodes an admitted base package
/// and carries its requirement rows with the record nodes they name, so the
/// strict reader admits the result with the same records.
#[trace("FR-940-AC-1", "FR-576-AC-1")]
#[test]
fn the_timed_model_writer_carries_value_validity_rows() {
    let (_, emission, records) = emitted_records(GUARDED);
    let admitted = match filament_contract_model::read_checked_package(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("the scalar base must read: {other:?}"),
    };
    let timed = crate::emit::emit_timed_models(&admitted, &[], crate::EmitLimits::fixture())
        .expect("a package without timed nodes re-emits");
    let reread = match filament_contract_model::read_checked_package(
        timed.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        timed.evidence(),
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(reread) => reread,
        other => panic!("the re-emitted value-validity rows must read: {other:?}"),
    };
    assert_eq!(reread.requirement_items(), admitted.requirement_items());
    let carried = reread
        .requirement_items()
        .iter()
        .filter_map(|row| row.record.clone())
        .collect::<Vec<_>>();
    assert_eq!(carried, records);
}
