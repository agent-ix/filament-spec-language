// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public owner-admitted compilation and source-backed division execution.

use super::*;
use crate::{
    admit_integer_division, compile_package, compile_package_with_integer_division,
    AdmittedIntegerDivision, DefinitionRole, SourceIdentity,
};
use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    CheckedTerm,
};
use filament_eval::FamilyOutcome;
use filament_exact::{Integer, Outcome, ScalarLimits};
use ix_trace_rs::trace;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn unit(source: &str) -> CompileUnit<'_> {
    CompileUnit {
        source: SourceIdentity::new("test", "division-selection"),
        package_name: "division-selection".to_owned(),
        path: "division.native",
        bytes: source.as_bytes(),
    }
}
fn selection(role: DefinitionRole) -> AdmittedIntegerDivision {
    admit_integer_division(&[role.reference()], None).expect("owner admits its own closed law")
}
fn compile(source: &str, law: &AdmittedIntegerDivision) -> CompiledPackage {
    compile_package_with_integer_division(
        unit(source),
        [],
        &DependencyInput::default(),
        law,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("public selected source compilation")
}
fn meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: 10000,
        decimal_digits: 10000,
        scale_expansion: 10000,
        text_input_bytes: 10000,
        text_scalars: 10000,
        normalized_scalars: 10000,
        unit_edges: 10000,
        value_occurrences: 10000,
        work_units: 100000,
        result_units: 10000,
    })
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn division_owner_selection_reaches_public_compile_source_execution_and_strict_read() {
    let source = format!("{HEADER}function q using v(a: Int[-9,9], b: Int[-4,4]): Integer pure {{ if b != 0 then a div b else 0 }}\nfunction r using v(a: Int[-9,9], b: Int[-4,4]): Integer pure {{ if b != 0 then a rem b else 0 }}\n");
    for (role, cases) in [
        (
            DefinitionRole::IntegerDivisionEuclidean,
            [
                (-7_i64, 3_i64, -3_i64, 2_i64),
                (7, -3, -2, 1),
                (-7, -3, 3, 2),
            ],
        ),
        (
            DefinitionRole::IntegerDivisionFloor,
            [
                (-7_i64, 3_i64, -3_i64, 2_i64),
                (7, -3, -3, -2),
                (-7, -3, 2, -1),
            ],
        ),
        (
            DefinitionRole::IntegerDivisionTruncating,
            [(-7, 3, -2, -1), (7, -3, -2, 1), (-7, -3, 2, -1)],
        ),
    ] {
        let law = selection(role);
        let compiled = compile(&source, &law);
        let acquired = acquire_checked_closure_with_integer_division(
            unit(&source),
            [],
            &DependencyInput::default(),
            &law,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("public source-backed closure compilation");
        assert_eq!(acquired.package().bytes(), compiled.bytes());
        let mut evidence = CheckedPackageEvidence::new();
        evidence.support_feature("filament.value.complete/v1");
        let read = match CheckedPackageV2::read(
            compiled.bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            CheckedPackageV2ReadResult::Admitted(read) => read,
            other => panic!("actual emitted package admits independently: {other:?}"),
        };
        let mut found = 0;
        for node in &read.resolved_graph().nodes {
            let Some(CheckedTerm::Application { operation, .. }) = read.term(&node.node_id) else {
                continue;
            };
            if matches!(
                operation.identity(),
                Some("filament.op.integer.div" | "filament.op.integer.rem")
            ) {
                let laws = operation
                    .law_definitions()
                    .expect("reader admitted law views");
                assert_eq!(laws.len(), 1);
                assert_eq!(laws[0].authority.as_ref(), role.reference().authority);
                assert_eq!(laws[0].identity.as_ref(), role.reference().identity);
                found += 1;
            }
        }
        assert_eq!(
            found, 2,
            "both actual checked division applications retain the law"
        );
        for (a, b, q, r) in cases {
            for (name, expected) in [("q", q), ("r", r)] {
                let key = acquired
                    .handle
                    .package
                    .graph()
                    .function_identity(name)
                    .expect("actual checked function");
                let callable = acquired
                    .handle()
                    .lookup_callable(acquired.handle().package_name(), &key)
                    .expect("owner callable");
                let environment = ApplicationEnvironment::new(ObjectClosure::default());
                let mut meter = meter();
                let result = acquired
                    .handle()
                    .registry()
                    .begin_call(
                        &callable,
                        vec![
                            Value::Integer(Integer::from(a)),
                            Value::Integer(Integer::from(b)),
                        ],
                        &environment,
                        &mut meter,
                    )
                    .expect("public source-backed call")
                    .run()
                    .expect("source execution");
                assert!(
                    matches!(result.outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(ref actual))) if actual == &Integer::from(expected)),
                    "{role:?} {a} {name} {b}: {result:?}"
                );
            }
        }
    }
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn missing_division_selection_stays_typed_and_modulo_needs_no_selection() {
    let source = format!(
        "{HEADER}function q using v(a: Int[-9,9], b: Int[1,4]): Integer pure {{ a div b }}\n"
    );
    let refusal = compile_package(
        unit(&source),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect_err("a demanded selection is absent");
    assert_eq!(refusal.code(), fsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), Some("missing-selection"));
    for expression in ["a mod b", "a + b"] {
        let source = format!("{HEADER}function q using v(a: Int[-9,9], b: Int[1,4]): Integer pure {{ {expression} }}\n");
        compile_package(
            unit(&source),
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("no division-law demand");
    }
}

#[trace("TC-2210", "FR-495-AC-19", "FSpec-TC-1660", "FSpec-FR-147-AC-14")]
#[test]
fn exact_mod_intervals_remain_readable_in_bounded_and_integer_consumers() {
    let source = format!(
        "{HEADER}function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure {{ a mod b }}\n\
         function m2 using v(a: Int[0, 100], b: Int[1, 12]): Integer pure {{ a mod b }}\n\
         function accepts using v(r: Int[0, 11]): Boolean pure {{ r >= 0 }}\n\
         function via_call using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure {{ accepts(a mod b) }}\n\
         function accepts_wide using v(r: Integer): Boolean pure {{ r >= 0 }}\n"
    );
    let compiled = compile_without_law(&source).expect("mod compiles without a division law");
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let read = match CheckedPackageV2::read(
        compiled.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(read) => read,
        other => panic!("exact mod results remain independently readable: {other:?}"),
    };
    let modulo_nodes = read
        .resolved_graph()
        .nodes
        .iter()
        .filter_map(|node| read.term(&node.node_id))
        .filter(|term| {
            matches!(
                term,
                CheckedTerm::Application { operation, .. }
                    if operation.identity() == Some("filament.op.integer.mod")
            )
        })
        .count();
    assert!(
        modulo_nodes >= 1,
        "bounded and Integer consumers retain mod"
    );
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn facade_admission_retains_conflicting_unknown_and_incompatible_definition_refusals() {
    use filament_semantic_value::definition::PackageCause;
    let euclidean = DefinitionRole::IntegerDivisionEuclidean.reference();
    let floor = DefinitionRole::IntegerDivisionFloor.reference();
    let conflicting =
        admit_integer_division(&[euclidean.clone(), floor.clone()], None).unwrap_err();
    assert_eq!(conflicting.cause, PackageCause::ConflictingDefinition);
    let mut unknown = euclidean.clone();
    unknown.identity = "filament.integer_division.unknown/v1".to_owned();
    assert_eq!(
        admit_integer_division(&[unknown], None).unwrap_err().cause,
        PackageCause::IncompatibleDefinition
    );
    let mut foreign = euclidean.clone();
    foreign.authority = "foreign".to_owned();
    assert_eq!(
        admit_integer_division(&[foreign], None).unwrap_err().cause,
        PackageCause::IncompatibleDefinition
    );
    assert_eq!(
        admit_integer_division(&[euclidean], Some(&floor))
            .unwrap_err()
            .cause,
        PackageCause::IncompatibleDefinition
    );
}

#[trace("TC-1284", "FR-495-AC-8")]
#[test]
fn independent_reader_refuses_an_emitted_law_binding_not_selected_by_the_package() {
    use filament_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};
    let source = format!(
        "{HEADER}function q using v(a: Int[-9,9], b: Int[1,4]): Integer pure {{ a div b }}\n"
    );
    let compiled = compile(&source, &selection(DefinitionRole::IntegerDivisionFloor));
    let mut wire: serde_json::Value = serde_json::from_slice(compiled.bytes()).unwrap();
    let nodes = wire["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let (index, node) = nodes
        .iter_mut()
        .enumerate()
        .find(|(_, node)| node["body"]["operation"]["identity"] == "filament.op.integer.div")
        .expect("actual emitted application");
    node["body"]["operation"]["laws"][0]["definition"] =
        serde_json::to_value(DefinitionRole::IntegerDivisionTruncating.reference()).unwrap();
    let mut evidence = CheckedPackageEvidence::new();
    evidence.support_feature("filament.value.complete/v1");
    let result = CheckedPackageV2::read(
        &serde_json::to_vec(&wire).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Refused(refusal) = result else {
        panic!("an unselected law must refuse: {result:?}")
    };
    assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidPackage);
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::OperationLawUnselected)
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().unwrap().as_str(),
        format!("/semantic_graph/nodes/{index}/body/operation/laws/0/definition")
    );
}

fn compile_without_law(source: &str) -> Result<CompiledPackage, crate::ReplayRefusal> {
    compile_package(
        unit(source),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
}

/// FR-670-AC-5: a header law and a caller-supplied law agree or refuse
/// `conflicting-definition`; a caller law alone still decides.
#[trace("TC-1782", "FR-670-AC-5", "FSpec-TC-1280", "FSpec-FR-570-AC-3")]
#[test]
#[ignore = "QSL-1989: a header law is not resolved, so it cannot agree or conflict with a caller-supplied law yet"]
fn a_header_law_agrees_with_or_refuses_a_caller_supplied_law() {
    let body = "function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n";
    let headed =
        format!("{HEADER}profile d = \"filament.value.integer-division.floor/v1\";\n{body}");
    compile(&headed, &selection(DefinitionRole::IntegerDivisionFloor));
    let refusal = compile_package_with_integer_division(
        unit(&headed),
        [],
        &DependencyInput::default(),
        &selection(DefinitionRole::IntegerDivisionTruncating),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect_err("the header and the caller name different laws");
    assert_eq!(refusal.code(), fsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("conflicting-definition"));
    compile(
        &format!("{HEADER}{body}"),
        &selection(DefinitionRole::IntegerDivisionTruncating),
    );
}

/// FR-670-AC-6 and FSpec FR-570-AC-5 (identity half): a law header in a unit
/// with no `div` or `rem` leaves the emitted package unchanged.
#[trace("TC-1782", "FR-670-AC-6", "FSpec-TC-1280", "FSpec-FR-570-AC-5")]
#[test]
#[ignore = "QSL-1989: a header naming the integer-division law is refused wrong-selection-role today"]
fn a_law_header_without_div_or_rem_leaves_the_package_identity_unchanged() {
    let body = "function m using v(a: Int[-7, 7], b: Int[1, 3]): Int[-6, 9] pure { a mod b + b }\n";
    let plain = compile_without_law(&format!("{HEADER}{body}")).expect("no law is demanded");
    let headed = compile_without_law(&format!(
        "{HEADER}profile d = \"filament.value.integer-division.floor/v1\";\n{body}"
    ))
    .expect("an inert law header is admitted");
    assert_eq!(headed.bytes(), plain.bytes());
}
