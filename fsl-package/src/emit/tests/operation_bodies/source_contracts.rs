// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original operation bodies recovered through the strict checked-package reader.

use super::*;
use filament_contract_model::{CheckedBindingNameKind, CheckedLiteralValue, CheckedTerm};

fn admitted(
    document: &[u8],
    emission: &Emission,
) -> std::sync::Arc<filament_contract_model::CheckedPackageV2> {
    let mut evidence = read_evidence(emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("the selected original input is evidence");
    match read_with(emission, &evidence) {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("strict reader must admit the emitted operation body: {other:?}"),
    }
}

#[trace("TC-1011", "FR-576-AC-1", "FR-576-AC-2", "FR-576-AC-9")]
#[test]
fn strict_body_lookup_recovers_integer_and_boolean_values_without_an_invocation() {
    for (name, type_ref, expression, expected) in [
        (
            "score",
            "ix://filament/native/Integer",
            "43",
            CheckedLiteralValue::Integer(filament_exact::Integer::from(43)),
        ),
        (
            "ready",
            "ix://filament/native/Boolean",
            "false",
            CheckedLiteralValue::Boolean(false),
        ),
    ] {
        let mut op = query("ConfigVersion", name, None);
        op["returns"]["typeRef"] = json!(type_ref);
        let document = package(vec![op], &[]);
        let source = format!(
            "body authored using v on Config::ConfigVersion::{name}() {{ {expression} }}\n"
        );
        let emission = emitted(&document, &source);
        let checked = admitted(&document, &emission);
        let row = checked
            .operation_body_rows()
            .expect("body rows retained")
            .iter()
            .find(|row| row.operation.as_ref() == name)
            .expect("operation row");
        let OperationBody::Function { node: function } = &row.body else {
            panic!("body must not disappear");
        };
        let graph = checked.resolved_graph();
        let function = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == function)
            .expect("function lookup");
        assert_eq!(function.node_tag.as_ref(), "function");
        let CheckedTerm::Aggregate { members } = &function.body else {
            panic!("function aggregate");
        };
        let expression = members
            .iter()
            .find_map(|member| match member {
                CheckedTerm::Binding { name, value }
                    if matches!(name.kind(), CheckedBindingNameKind::Body) =>
                {
                    Some(value.as_ref())
                }
                _ => None,
            })
            .expect("body member");
        let CheckedTerm::Reference { target } = expression else {
            panic!("body expression reference");
        };
        let expression = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == target)
            .expect("expression lookup");
        assert_eq!(
            expression
                .body
                .literal_read()
                .expect("authored literal")
                .value,
            expected
        );
        let wire = wire(&emission);
        assert!(
            wire["source_map"]
                .as_array()
                .expect("source map")
                .iter()
                .any(|entry| entry["node_id"] == json!(function.node_id)),
            "the recovered function must retain its authored provenance"
        );
    }
}

#[trace("TC-1011", "FR-576-AC-3", "FR-576-AC-4", "FR-576-AC-10")]
#[test]
fn a_bodyless_void_operation_has_explicit_absence_beside_a_real_body() {
    let score = query("ConfigVersion", "score", None);
    let mut touch = query("ConfigVersion", "touch", None);
    touch.as_object_mut().expect("operation").remove("returns");
    let document = package(vec![score, touch], &[]);
    let emission = emitted(
        &document,
        "body authored using v on Config::ConfigVersion::score() { 43 }\n",
    );
    let checked = admitted(&document, &emission);
    let rows = checked.operation_body_rows().expect("operation body rows");
    let touch = rows
        .iter()
        .find(|row| row.operation.as_ref() == "touch")
        .expect("void operation remains declared");
    assert_eq!(touch.body, OperationBody::NoBody);
    let score = rows
        .iter()
        .find(|row| row.operation.as_ref() == "score")
        .expect("value operation");
    assert!(matches!(score.body, OperationBody::Function { .. }));
    let graph = checked.resolved_graph();
    assert!(
        !graph.nodes.iter().any(|node| match &node.owner {
            Some(filament_contract_model::CheckedNodeOwner::Model { node, .. }) =>
                node.as_ref() == owner("ConfigVersion/touch"),
            _ => false,
        }),
        "no synthetic void body node may appear"
    );
}

#[trace("TC-1011", "FR-576-AC-2", "FR-576-AC-10")]
#[test]
fn uncalled_wrong_type_wrong_arity_and_void_bodies_refuse_before_emission() {
    let mut void = query("ConfigVersion", "touch", None);
    void.as_object_mut().expect("operation").remove("returns");
    let document = package(vec![query("ConfigVersion", "score", None), void], &[]);
    for source in [
        "body wrongValue using v on Config::ConfigVersion::score() { true }\n",
        "body wrongArity using v on Config::ConfigVersion::score(extra: Integer) { extra }\n",
        "body voidValue using v on Config::ConfigVersion::touch() { 43 }\n",
    ] {
        let refusals = compile(&document, source, &["Config"])
            .expect_err("an unreachable invalid body must not emit a package");
        assert!(!refusals.is_empty());
        assert!(
            refusals
                .iter()
                .all(|refusal| refusal.cause.code().as_str() == "ill_typed"
                    && refusal.cause.cause() == Some("type-mismatch")),
            "{refusals:?}"
        );
    }
}
