// SPDX-License-Identifier: AGPL-3.0-or-later
//! The public source-object identity-row API preserves FR-531 member facts.

#![cfg(feature = "std")]

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    NativeJoinBudget,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    emit_checked, CheckedPackage, EmitLimits, ReadIdentityRows, SourceObjectMemberFacts,
    SourceObjectOperationFacts,
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

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"filament.model.complete/v1\";\n\
    record Point { x: Integer; }\n";

fn checked_source_object(
    source: &[u8],
) -> (CheckedPackage, filament_contract_model::CheckedPackageV2) {
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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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

/// The identity file the public constructor serializes for `source`.
fn serialized_identities(source: &str, objects: usize) -> Value {
    let (source_package, checked_package) = checked_source_object(source.as_bytes());
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
    let operations: Vec<Vec<SourceObjectOperationFacts>> = emission
        .source_object_operations()
        .iter()
        .map(Clone::clone)
        .collect();
    assert_eq!(member_facts.len(), objects);
    assert!(member_facts.iter().all(|facts| facts.len() == 1));
    assert_eq!(operations.len(), objects);

    let constructed = ReadIdentityRows::of_rows_with_source_rows(
        &checked_package,
        rows.declarations().to_vec(),
        rows.fields().to_vec(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
        rows.source_objects().to_vec(),
        rows.source_operations().to_vec(),
        member_facts,
        operations,
    );
    let bytes = constructed
        .to_bytes(&checked_package)
        .expect("the public constructor serializes its object member facts");
    let identities: Value = serde_json::from_slice(&bytes).expect("identity JSON");
    identities
}

#[trace("FR-531-AC-8", "FR-569-AC-8", "TC-1408")]
#[test]
fn public_source_object_rows_constructor_serializes_nonempty_member_facts() {
    let identities =
        serialized_identities(&format!("{HEADER}object Order {{ point: Point; }}\n"), 1);
    let member = &identities["source_objects"][0]["members"][0];
    assert_eq!(member["admitted_name"], "point");
    assert_eq!(member["presence"], "required");
    assert!(member["original_member_identity"].is_string());
    assert!(member["value_type"].is_object());
    assert!(member["type_node"].is_object());
    assert_eq!(identities["source_objects"][0]["abstract"], json!(false));
    assert_eq!(identities["source_objects"][0]["operations"], json!([]));
}

/// FR-569-AC-16 through the public constructor: own operations only, each
/// `{identity, query, clock_parameters, parameters}`.
#[trace("FR-531-AC-8", "FR-569-AC-16", "TC-1408")]
#[test]
fn public_source_object_rows_constructor_serializes_own_operations() {
    let identities = serialized_identities(
        &format!(
        "{HEADER}object Order {{ point: Point; operation size(): Integer; operation bump(): Boolean modifies self.point; }}\nobject Sub extends Order {{ operation size(): Integer redefines Order::size; }}\n"
        ),
        2,
    );
    let objects = identities["source_objects"].as_array().expect("rows");
    let row = |name: &str| {
        objects
            .iter()
            .find(|row| row["source_identity"] == name)
            .unwrap_or_else(|| panic!("row for {name}"))
    };
    assert_eq!(
        row("Order")["operations"],
        json!([
            {"identity": "Order::size", "query": true, "clock_parameters": [], "parameters": []},
            {"identity": "Order::bump", "query": false, "clock_parameters": [], "parameters": []},
        ])
    );
    assert_eq!(
        row("Sub")["operations"],
        json!([{"identity": "Sub::size", "query": true, "clock_parameters": [], "parameters": []}])
    );
}

/// The `admitted_name`s of each emitted source object's member facts, in row
/// order, for `source`.
fn member_names(source: &str) -> Vec<Vec<String>> {
    let (package, _) = checked_source_object(source.as_bytes());
    let emission = emit_checked(&package, EmitLimits::new(10_000_000)).expect("the package emits");
    emission
        .source_object_member_facts()
        .iter()
        .map(|facts| facts.iter().map(|fact| fact.admitted_name.to_string()).collect())
        .collect()
}

fn names(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|name| (*name).to_owned()).collect())
        .collect()
}

/// FR-569-AC-6, AC-8: a subtype that adds a field emits and is admitted; its
/// view lists the inherited member first, then its own in source order.
#[trace("FR-569-AC-6", "FR-569-AC-8", "TC-1408")]
#[test]
fn subtype_adding_a_field_emits_inherited_then_own_members() {
    assert_eq!(
        member_names(&format!(
            "{HEADER}object Order {{ point: Point; }}\nobject Rush extends Order {{ level: Integer; }}\n"
        )),
        names(&[&["point"], &["point", "level"]])
    );
}

/// FR-569-AC-6, AC-8: each level of a chain adds a field; the view is the
/// chain's fields, outermost supertype first, own fields in source order.
#[trace("FR-569-AC-6", "FR-569-AC-8", "TC-1408")]
#[test]
fn multi_level_subtypes_list_members_outermost_first() {
    assert_eq!(
        member_names(&format!(
            "{HEADER}object A {{ z: Integer; a: Integer; }}\nobject B extends A {{ y: Integer; }}\nobject C extends B {{ b: Integer; x: Integer; }}\n"
        )),
        names(&[&["z", "a"], &["z", "a", "y"], &["z", "a", "y", "b", "x"]])
    );
}

/// FR-569-AC-6, AC-8: two supertypes' fields come in `extends` order, then
/// the subtype's own.
#[trace("FR-569-AC-6", "FR-569-AC-8", "TC-1408")]
#[test]
fn two_supertypes_list_members_in_extends_order() {
    assert_eq!(
        member_names(&format!(
            "{HEADER}object Pa {{ q: Integer; }}\nobject Rb {{ a: Integer; }}\nobject Sc extends Pa, Rb {{ m: Integer; }}\n"
        )),
        names(&[&["q"], &["a"], &["q", "a", "m"]])
    );
}

/// FR-569-AC-16: a subtype adding a field and an operation lists the
/// operation as its own entry only.
#[trace("FR-569-AC-8", "FR-569-AC-16", "TC-1408")]
#[test]
fn subtype_adding_a_field_and_an_operation_emits() {
    let (package, _) = checked_source_object(
        format!(
            "{HEADER}object Order {{ point: Point; operation size(): Integer; }}\nobject Rush extends Order {{ level: Integer; operation weight(): Integer; }}\n"
        )
        .as_bytes(),
    );
    let emission = emit_checked(&package, EmitLimits::new(10_000_000)).expect("the package emits");
    let operations: Vec<Vec<String>> = emission
        .source_object_operations()
        .iter()
        .map(|entries| entries.iter().map(|entry| format!("{entry:?}")).collect())
        .collect();
    assert_eq!(operations.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
}
