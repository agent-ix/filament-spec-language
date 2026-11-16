// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2582 (FR-913): the package compiled from formatted source has the
//! original's semantic graph, apart from source regions, and the strict
//! reader gives both packages the same verdict.

use std::collections::BTreeMap;

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;
use serde_json::Value;

use crate::support::front_end::emitted;

/// The emitted package's bytes for `text`.
fn package_bytes(text: &str) -> Vec<u8> {
    emitted(
        SourceIdentity::new("test", "format-readback"),
        "unit.native",
        text.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the unit compiles")
    .package()
    .bytes()
    .to_vec()
}

/// `value` with every member that records a source region removed.
fn without_regions(value: &Value) -> Value {
    match value {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(name, _)| !matches!(name.as_str(), "region" | "regions" | "span"))
                .map(|(name, member)| (name.clone(), without_regions(member)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_regions).collect()),
        other => other.clone(),
    }
}

/// Whether Contract IR's strict reader admits `bytes`, or the code it
/// refuses them with.
fn verdict(bytes: &[u8]) -> Result<(), String> {
    let evidence = filament_contract_model::CheckedPackageEvidence::new();
    match filament_contract_model::read_checked_package(
        bytes,
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => Ok(()),
        filament_contract_model::CheckedPackageDispatchResult::Refused(refusal) => {
            Err(format!("{:?}", refusal.code))
        }
        other => Err(format!("{other:?}")),
    }
}

#[test]
#[trace("TC-2582", "FR-913-AC-1", "FR-003-AC-9")]
#[ignore = "QSL-2231: the format-then-emit read-back agreement test is not implemented yet"]
fn format_then_emit_reads_back_the_same_package() {
    let original = include_str!("../fixtures/spine-compile.native");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "format-readback"),
        "unit.native",
        original.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the fixture parses");
    let formatted = fsl_cst::format::format(&parsed).expect("the fixture formats");
    let before = package_bytes(original);
    let after = package_bytes(&formatted);
    let graph = |bytes: &[u8]| {
        let wire: Value = serde_json::from_slice(bytes).expect("the package is JSON");
        without_regions(&wire["semantic_graph"])
    };
    assert_eq!(graph(&after), graph(&before));
    assert_eq!(verdict(&after), verdict(&before));
    assert_eq!(verdict(&before), Ok(()));
}
