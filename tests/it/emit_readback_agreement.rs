// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2582 (FR-913): the package compiled from formatted source has the
//! original's semantic graph, apart from source regions, and the strict
//! reader gives both packages the same verdict.

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;
use serde_json::Value;

use crate::support::front_end::emitted;

/// The emitted package's bytes for `text`.
fn package_bytes(text: &str, packages: &fsl_semantics::model::intake::PackageInput) -> Vec<u8> {
    emitted(
        SourceIdentity::new("test", "format-readback"),
        "unit.native",
        text.as_bytes(),
        packages,
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
fn verdict(bytes: &[u8], document: &[u8]) -> Result<(), String> {
    let mut evidence = filament_contract_model::CheckedPackageEvidence::new();
    let wire: Value = serde_json::from_slice(bytes).expect("the package is JSON");
    for feature in wire["lock"]["required_features"]
        .as_array()
        .into_iter()
        .flatten()
    {
        evidence.support_feature(feature.as_str().expect("feature name").to_owned());
    }
    evidence
        .insert_domain_package_document("acme/orders", document.to_vec().into_boxed_slice())
        .expect("strict reader evidence key");
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
fn format_then_emit_reads_back_the_same_package() {
    let model = crate::support::spine_model::unit();
    let document = crate::support::spine_model::DOCUMENT;
    let packages =
        fsl_semantics::model::intake::package_input([document]).expect("the domain fixture admits");
    for (name, original) in [
        (
            "spine-compile.native",
            include_str!("../fixtures/spine-compile.native"),
        ),
        ("spine-model.native", model.as_str()),
        (
            "spine-run.native",
            include_str!("../fixtures/spine-run.native"),
        ),
        (
            "value-format.native",
            include_str!("../fixtures/value-format.native"),
        ),
    ] {
        let source = SourceIdentity::new("test", name);
        let parsed = fsl_cst::parse(
            source,
            name,
            original.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .unwrap_or_else(|error| panic!("{name}: the fixture parses: {error}"));
        let formatted = fsl_cst::format::format(&parsed)
            .unwrap_or_else(|error| panic!("{name}: the fixture formats: {error}"));
        let before = package_bytes(original, &packages);
        let after = package_bytes(&formatted, &packages);
        let graph = |bytes: &[u8]| {
            let wire: Value = serde_json::from_slice(bytes).expect("the package is JSON");
            without_regions(&wire["semantic_graph"])
        };
        assert_eq!(graph(&after), graph(&before), "{name}");
        assert_eq!(
            verdict(&after, document),
            verdict(&before, document),
            "{name}"
        );
        assert_eq!(verdict(&before, document), Ok(()), "{name}");
    }
}
