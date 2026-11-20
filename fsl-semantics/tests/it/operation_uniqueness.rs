// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original model-input operation intake cases; loci are model member paths.

use fsl_semantics::model::domain_package::DomainPackageRecord;
use fsl_semantics::model::intake::UnitIntakeCause;
use fsl_semantics::model::refusal::ModelRefusalCause;
use ix_trace_rs::trace;
use serde_json::{Value, json};

use super::model_operations::{
    PACKAGE_IDENTITY, admit, config_version_document_with_operations, empty_frame, operation,
    operation_parameter,
};

const INTEGER: &str = "ix://filament/native/Integer";

fn input(operations: Vec<Value>) -> Vec<u8> {
    config_version_document_with_operations(operations)
}

fn query(name: &str, parameters: Value) -> Value {
    operation(name, parameters, Some(INTEGER), empty_frame())
}

fn key(name: &str) -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion/{name}")
}

fn refusal(document: &[u8], code: &str, cause: &str, node: &str, member: &str) {
    let refused = admit(document).expect_err("invalid operation model must not be selected");
    let UnitIntakeCause::Refused(found) = refused.cause else {
        panic!("a located intake refusal: {refused:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    let found = &found[0];
    assert_eq!(found.code.as_str(), code, "{found:?}");
    assert_eq!(found.cause.as_str(), cause, "{found:?}");
    match &found.cause {
        ModelRefusalCause::ConflictingBinding {
            key,
            member: Some(path),
        } => {
            assert_eq!(key.node, node);
            assert_eq!(path.to_string(), member);
        }
        ModelRefusalCause::IntakeMalformedDeclaration {
            node: actual,
            member: path,
            ..
        }
        | ModelRefusalCause::UnsupportedDeclarationForm {
            node: actual,
            member: path,
            ..
        } => {
            assert_eq!(actual, node);
            assert_eq!(path.to_string(), member);
        }
        other => panic!("the typed intake cause must identify {node} at {member}: {other:?}"),
    }
}

#[trace("TC-1307", "FR-512-AC-4")]
#[test]
fn duplicate_operation_identity_refuses_at_the_second_operation() {
    let op = query("advance", json!([]));
    admit(&input(vec![op.clone()])).expect("one operation admits");
    refusal(
        &input(vec![op.clone(), op]),
        "invalid_model_binding",
        "conflicting-binding",
        &key("advance"),
        "TypeDefinition.operations[1]",
    );
}

#[trace("TC-1307", "FR-512-AC-5")]
#[test]
fn equal_operation_names_with_distinct_identities_refuse_at_the_second_name() {
    let first = query("advance", json!([]));
    let mut second = query("other", json!([]));
    second["name"] = json!("advance");
    refusal(
        &input(vec![first, second]),
        "ambiguous_declaration",
        "ambiguous-name",
        &key("other"),
        "TypeDefinition.operations[1].name",
    );
}

#[trace("TC-1307", "FR-512-AC-7", "FR-512-AC-15")]
#[test]
fn duplicate_parameter_identity_preempts_an_unresolved_frame() {
    let parameter = operation_parameter("advance", "amount", INTEGER);
    let mut op = query("advance", json!([parameter.clone(), parameter]));
    op["frame"]["modifies"] = json!([key("missingField")]);
    refusal(
        &input(vec![op]),
        "invalid_model_binding",
        "conflicting-binding",
        &key("advance/amount"),
        "TypeDefinition.operations[0].params[1]",
    );
}

#[trace("TC-1307", "FR-512-AC-8")]
#[test]
fn duplicate_parameter_names_preserve_the_readers_exact_repeated_name() {
    let first = operation_parameter("advance", "amount", INTEGER);
    let mut second = operation_parameter("advance", "other", INTEGER);
    second["name"] = json!("amount");
    let rejected = admit(&input(vec![query("advance", json!([first, second]))]))
        .expect_err("reader rejects repeated parameter name");
    let UnitIntakeCause::Refused(found) = rejected.cause else {
        panic!("{rejected:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].detail.contains("DUPLICATE_PARAM"), "{found:?}");
    assert!(
        found[0]
            .detail
            .contains("/ir/types/1/operations/0/params/1/name"),
        "{found:?}"
    );
    let ModelRefusalCause::IntakeMalformedDeclaration { member, .. } = &found[0].cause else {
        panic!("{found:?}");
    };
    assert_eq!(
        member.to_string(),
        "TypeDefinition.operations[0].params[1].name"
    );
}

#[trace("TC-1307", "FR-512-AC-9", "FR-512-AC-15")]
#[test]
fn mismatched_parameter_name_preempts_frame_resolution() {
    let mut parameter = operation_parameter("advance", "amount", INTEGER);
    parameter["name"] = json!("distance");
    let mut op = query("advance", json!([parameter]));
    op["frame"]["modifies"] = json!([key("missingField")]);
    refusal(
        &input(vec![op]),
        "invalid_model_binding",
        "malformed-declaration",
        &key("advance/amount"),
        "TypeDefinition.operations[0].params[0].name",
    );
}

#[trace("TC-1307", "FR-512-AC-10", "FR-512-AC-11", "FR-512-AC-15")]
#[test]
fn result_parameter_is_reserved_with_and_without_a_return_type() {
    for returns in [Some(INTEGER), None] {
        let mut op = operation(
            "advance",
            json!([operation_parameter("advance", "result", INTEGER)]),
            returns,
            empty_frame(),
        );
        op["frame"]["modifies"] = json!([key("missingField")]);
        refusal(
            &input(vec![op]),
            "ambiguous_declaration",
            "ambiguous-name",
            &key("advance/result"),
            "TypeDefinition.operations[0].params[0].name",
        );
    }
}

#[trace("TC-1307", "FR-512-AC-12", "FR-512-AC-13", "FR-512-AC-14")]
#[test]
fn repeated_frame_entries_deduplicate_without_losing_distinct_entries() {
    let field = key("versionNumber");
    let object = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    for member in [
        "modifies",
        "modifies_receiver",
        "creates",
        "deletes",
        "deletes_parameters",
    ] {
        let parameters = json!([operation_parameter("advance", "target", &object)]);
        let mut once = operation("advance", parameters, None, empty_frame());
        let entry = match member {
            "modifies" | "modifies_receiver" => field.as_str(),
            "deletes_parameters" => "target",
            _ => object.as_str(),
        };
        once["frame"][member] = json!([entry]);
        let mut repeated = once.clone();
        repeated["frame"][member] = json!([entry, entry]);
        let once = admit(&input(vec![once])).expect("one frame grant admits");
        let repeated = admit(&input(vec![repeated])).expect("repeated grants admit");
        let effect = |models: &fsl_semantics::model::intake::SelectedModels| {
            models[0]
                .view
                .domain_package()
                .records
                .iter()
                .find_map(|record| match record {
                    DomainPackageRecord::OperationMember(op) if op.key.node == key("advance") => {
                        Some(op.effect.clone())
                    }
                    _ => None,
                })
                .expect("the declared operation survives intake")
        };
        let actual = effect(&repeated);
        assert_eq!(effect(&once), actual);
        let expected = filament_checked::key::DeclarationKey {
            package: PACKAGE_IDENTITY.to_owned(),
            node: entry.to_owned(),
        };
        match member {
            "modifies" => assert_eq!(actual.modifies, [expected]),
            "modifies_receiver" => assert_eq!(actual.receiver_modifies, [expected]),
            "creates" => assert_eq!(actual.creates, [expected]),
            "deletes" => assert_eq!(actual.deletes, [expected]),
            "deletes_parameters" => assert_eq!(actual.deletes_arguments, ["target"]),
            _ => unreachable!("the explicitly authored frame members"),
        }
    }
}

#[trace("TC-1307", "FR-512-AC-17")]
#[test]
fn operation_and_frame_order_do_not_change_the_selected_declarations() {
    let first = query(
        "advance",
        json!([operation_parameter("advance", "amount", INTEGER)]),
    );
    let second = query(
        "inspect",
        json!([operation_parameter("inspect", "amount", INTEGER)]),
    );
    let forward = admit(&input(vec![first.clone(), second.clone()]))
        .expect("same parameter name in distinct operations admits");
    let reverse = admit(&input(vec![second, first])).expect("reversed operations admit");
    let operations = |selected: &fsl_semantics::model::intake::SelectedModel| {
        selected
            .view
            .domain_package()
            .records
            .iter()
            .filter_map(|record| match record {
                DomainPackageRecord::OperationMember(op) => {
                    Some((op.key.clone(), op.parameters.clone(), op.effect.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(operations(&forward[0]), operations(&reverse[0]));
}

#[trace("TC-1307", "FR-512-AC-6", "FR-512-AC-17")]
#[test]
fn an_explicit_redefinition_keeps_its_own_operation_identity() {
    let parent = query("advance", json!([]));
    let bytes = super::model_operations::add_sub_type(input(vec![parent]), &[]);
    let mut document: Value = serde_json::from_slice(&bytes).expect("original model JSON");
    let mut child = query("advance", json!([]));
    child["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Sub/advance"));
    child["redefines"] = json!(key("advance"));
    let types = document["types"].as_array_mut().expect("types");
    types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE_IDENTITY}/Sub"))
        .expect("declared subtype")["operations"] = json!([child]);
    let selected = admit(&serde_json::to_vec(&document).expect("model bytes"))
        .expect("explicit operation redefinition admits");
    let keys: Vec<_> = selected[0]
        .view
        .domain_package()
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::OperationMember(op) => Some(op.key.node.as_str()),
            _ => None,
        })
        .collect();
    assert!(keys.contains(&key("advance").as_str()));
    assert!(keys.contains(&format!("ix://{PACKAGE_IDENTITY}/Sub/advance").as_str()));
}

#[trace("TC-1307", "FR-512-AC-5", "FR-512-AC-16")]
#[test]
fn repeated_operation_name_preempts_the_second_operations_reserved_parameter() {
    let first = query("advance", json!([]));
    let mut second = query(
        "other",
        json!([operation_parameter("other", "result", INTEGER)]),
    );
    second["name"] = json!("advance");
    refusal(
        &input(vec![first, second]),
        "ambiguous_declaration",
        "ambiguous-name",
        &key("other"),
        "TypeDefinition.operations[1].name",
    );
}

#[trace("TC-1307", "FR-512-AC-12", "FR-512-AC-14")]
#[test]
fn different_frame_scopes_conflict_but_creation_and_deletion_can_share_a_type() {
    let field = key("versionNumber");
    let object = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
    let mut op = query("advance", json!([]));
    op["frame"] = json!({"modifies": [], "creates": [object.clone()], "deletes": [object.clone()]});
    let selected = admit(&input(vec![op])).expect("creation and deletion are independent grants");
    let effect = selected[0]
        .view
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(op) => Some(&op.effect),
            _ => None,
        })
        .expect("operation");
    assert_eq!(effect.creates[0].node, object);
    assert_eq!(effect.deletes[0].node, object);
    let mut op = query("advance", json!([]));
    op["frame"] = json!({"modifies": [field.clone()], "modifies_receiver": [field], "creates": [], "deletes": []});
    let refusal = admit(&input(vec![op])).expect_err("a field cannot have both write scopes");
    let UnitIntakeCause::Refused(found) = refusal.cause else {
        panic!("{refusal:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code.as_str(), "invalid_model_binding");
    assert_eq!(found[0].cause.as_str(), "conflicting-binding");
}
