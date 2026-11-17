// SPDX-License-Identifier: AGPL-3.0-or-later
//! The public source-object identity-row API preserves FR-531 member facts.

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    NativeJoinBudget,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    emit_checked, CheckedPackage, EmitLimits, ReadIdentityRows, SourceObjectMemberFacts,
};
use fsl_semantics::{
    check::PackageDeclarations,
    model::{
        accounting::ModelNormalizationLimits,
        intake::{admit_unit, package_input, PackageInput},
    },
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn checked_source_object() -> (CheckedPackage, filament_contract_model::CheckedPackageV2) {
    let source = b"language \"ix:native\" edition \"1-draft\";\n\
        profile P = \"filament.model.complete/v1\";\n\
        record Point { x: Integer; }\n\
        object Order { point: Point; }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:public-source-object-rows"),
        "program.native",
        source,
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert_eq!(parsed.diagnostics(), []);
    let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
    let inputs: PackageInput = package_input(std::iter::empty::<&[u8]>()).unwrap();
    let selected = admit_unit(
        &unit.selections().models,
        &inputs,
        ModelNormalizationLimits::default(),
    )
    .expect("empty selected models admit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "public-source-objects".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the package assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package, EmitLimits::new(10_000_000)).expect("the package emits");
    let wire: Value = serde_json::from_slice(emission.package().bytes()).expect("wire JSON");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(checked) = read else {
        panic!("the emitted package is admitted: {read:?}");
    };
    (package, *checked)
}

#[trace("FR-531-AC-8", "FR-569-AC-8", "TC-1408")]
#[test]
fn public_source_object_rows_constructor_serializes_nonempty_member_facts() {
    let (source_package, checked_package) = checked_source_object();
    let emission =
        emit_checked(&source_package, EmitLimits::new(10_000_000)).expect("the package emits");
    let rows = emission
        .identity_rows(
            &checked_package,
            &mut NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("identity rows join");
    let member_facts: Vec<Vec<SourceObjectMemberFacts>> = emission
        .source_object_member_facts()
        .iter()
        .map(Clone::clone)
        .collect();
    assert_eq!(member_facts.len(), 1);
    assert_eq!(member_facts[0].len(), 1);

    let constructed = ReadIdentityRows::of_rows_with_source_rows(
        &checked_package,
        rows.declarations().to_vec(),
        rows.fields().to_vec(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
        rows.source_objects().to_vec(),
        rows.source_operations().to_vec(),
        member_facts,
    );
    let bytes = constructed
        .to_bytes(&checked_package)
        .expect("the public constructor serializes its object member facts");
    let identities: Value = serde_json::from_slice(&bytes).expect("identity JSON");
    let member = &identities["source_objects"][0]["members"][0];
    assert_eq!(member["admitted_name"], "point");
    assert_eq!(member["presence"], "required");
    assert!(member["original_member_identity"].is_string());
    assert!(member["value_type"].is_object());
    assert!(member["type_node"].is_object());
    assert_eq!(identities["source_objects"][0]["abstract"], json!(false));
}
