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

fn refusal(
    document: &[u8],
    code: &str,
    cause: &str,
    node: &str,
    member: &str,
) -> fsl_semantics::model::normalize::ModelRefusal {
    let refused = admit(document).expect_err("invalid operation model must not be selected");
    let UnitIntakeCause::Refused(found) = refused.cause else {
        panic!("a located intake refusal: {refused:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    assert_refusal(&found[0], code, cause, node, member);
    found[0].clone()
}

fn assert_refusal(
    found: &fsl_semantics::model::normalize::ModelRefusal,
    code: &str,
    cause: &str,
    node: &str,
    member: &str,
) {
    assert_eq!(found.code.as_str(), code, "{found:?}");
    assert_eq!(found.cause.as_str(), cause, "{found:?}");
    match &found.cause {
        ModelRefusalCause::ConflictingBinding {
            key,
            member: Some(path),
        }
        | ModelRefusalCause::AmbiguousName { key, member: path } => {
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

fn assert_collision_participants(found: &fsl_semantics::model::normalize::ModelRefusal) {
    assert!(
        found.detail.contains(&key("advance")) && found.detail.contains(&key("other")),
        "both distinct colliding operation identities must be retained: {found:?}"
    );
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
    let found = refusal(
        &input(vec![first, second]),
        "ambiguous_declaration",
        "ambiguous-name",
        &key("other"),
        "TypeDefinition.operations[1].name",
    );
    assert_collision_participants(&found);
}

#[trace("TC-1307", "FR-512-AC-7", "FR-512-AC-15")]
#[test]
fn duplicate_parameter_identity_preempts_an_unresolved_frame() {
    let parameter = operation_parameter("advance", "amount", INTEGER);
    let mut op = query("advance", json!([parameter.clone(), parameter]));
    op["frame"]["modifies"] = json!([key("missingField")]);
    let found = refusal(
        &input(vec![op]),
        "invalid_model_binding",
        "conflicting-binding",
        &key("advance/amount"),
        "TypeDefinition.operations[0].params[1]",
    );
    assert!(
        found.detail.contains("DUPLICATE_PARAM"),
        "the original reader diagnostic is retained: {found:?}"
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

#[trace("TC-1307", "FR-512-AC-12")]
#[test]
fn repeated_frame_entries_deduplicate_without_losing_distinct_entries() {
    assert_deduplicated_frame("modifies");
    let first = key("versionNumber");
    let second = key("parent");
    let mut op = query("advance", json!([]));
    op["frame"]["modifies"] = json!([first, second, first, second]);
    let models = admit(&input(vec![op])).expect("repeated and distinct grants admit");
    let effect = models[0]
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
        .expect("the declared operation survives intake");
    let nodes: Vec<_> = effect
        .modifies
        .iter()
        .map(|key| key.node.as_str())
        .collect();
    assert_eq!(nodes, [first.as_str(), second.as_str()]);
}

fn assert_deduplicated_frame(member: &str) {
    let field = key("versionNumber");
    let object = format!("ix://{PACKAGE_IDENTITY}/ConfigVersion");
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

#[trace("TC-1307", "FR-512-AC-12")]
#[test]
fn receiver_frame_duplicates_deduplicate() {
    assert_deduplicated_frame("modifies_receiver");
}

#[trace("TC-1307", "FR-512-AC-13")]
#[test]
fn creation_frame_duplicates_deduplicate() {
    assert_deduplicated_frame("creates");
}

#[trace("TC-1307", "FR-512-AC-13")]
#[test]
fn deletion_frame_duplicates_deduplicate() {
    assert_deduplicated_frame("deletes");
}

#[trace("TC-1307", "FR-512-AC-13")]
#[test]
fn parameter_deletion_frame_duplicates_deduplicate() {
    assert_deduplicated_frame("deletes_parameters");
}

#[trace("TC-1307", "FR-512-AC-17")]
#[test]
fn operation_and_frame_order_do_not_change_the_selected_declarations() {
    let mut first = query(
        "advance",
        json!([operation_parameter("advance", "amount", INTEGER)]),
    );
    first["frame"]["modifies"] = json!([key("parent"), key("versionNumber")]);
    let second = query(
        "inspect",
        json!([operation_parameter("inspect", "amount", INTEGER)]),
    );
    let forward_bytes = input(vec![first.clone(), second.clone()]);
    first["frame"]["modifies"] = json!([key("versionNumber"), key("parent")]);
    let reverse_bytes = input(vec![second, first]);
    let forward = super::model_operations::admit_and_assemble(&forward_bytes)
        .expect("forward source unit assembles");
    let reverse = super::model_operations::admit_and_assemble(&reverse_bytes)
        .expect("reordered source unit assembles");
    let operations = |package: &fsl_semantics::check::PackageDeclarations| {
        let owner = package.selected_views["Config"].type_identities()
            [&filament_checked::key::DeclarationKey {
                package: PACKAGE_IDENTITY.to_owned(),
                node: format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"),
            }]
            .clone();
        package
            .operations
            .declared_by(owner)
            .iter()
            .map(|operation| {
                let mut effect = operation.effect().clone();
                // These grants denote sets; this comparison does not reorder the admitted input.
                effect.modifies.sort();
                effect.receiver_modifies.sort();
                effect.creates.sort();
                effect.deletes.sort();
                effect.deletes_arguments.sort();
                (
                    operation.name().to_owned(),
                    (
                        operation.parameters().to_vec(),
                        operation.result().cloned(),
                        effect,
                    ),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let expected = operations(&forward);
    assert_eq!(
        expected.keys().map(String::as_str).collect::<Vec<_>>(),
        ["advance", "inspect"]
    );
    assert_eq!(expected["advance"].0[0].0, "amount");
    assert_eq!(
        expected["advance"]
            .2
            .modifies
            .iter()
            .map(|key| key.node.as_str())
            .collect::<Vec<_>>(),
        [key("parent"), key("versionNumber")]
    );
    assert_eq!(expected, operations(&reverse));
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
    let found = refusal(
        &input(vec![first, second]),
        "ambiguous_declaration",
        "ambiguous-name",
        &key("other"),
        "TypeDefinition.operations[1].name",
    );
    assert_collision_participants(&found);
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
    assert_refusal(
        &found[0],
        "invalid_model_binding",
        "conflicting-binding",
        &key("advance"),
        "Operation.frame.modifies_receiver[0]",
    );
    assert!(
        found[0].detail.contains(".frame.modifies_receiver[0]")
            && found[0].detail.contains(".frame.modifies[0]"),
        "both differently scoped entries must be named: {found:?}"
    );
}

#[trace("TC-1307", "FR-512-AC-1")]
#[test]
fn a_population_cannot_acquire_an_operations_member() {
    let mut document: Value = serde_json::from_slice(&input(vec![])).expect("original model");
    document["populations"][0]["operations"] = json!([query("advance", json!([]))]);
    let refused = admit(&serde_json::to_vec(&document).expect("model JSON"))
        .expect_err("population operations are not admitted");
    let UnitIntakeCause::Refused(found) = refused.cause else {
        panic!("{refused:?}");
    };
    assert!(
        found
            .iter()
            .any(|refusal| refusal.detail.contains("/ir/populations/0")
                && refusal.detail.contains("operations")),
        "the semantic-IR reader must retain its rejected member: {found:?}"
    );
}

fn extra_owner(kind: &str, meaning: &str, operations: Vec<Value>) -> Vec<u8> {
    use super::model_operations::{wire_construct, wire_field, wire_type};
    let mut document: Value = serde_json::from_slice(&input(vec![])).expect("original model");
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(wire_construct(
            kind,
            meaning,
            if kind == "event_type" {
                json!({"identityFields": "optional", "occurrenceField": "required"})
            } else {
                json!({"identityFields": "optional"})
            },
        ));
    let owner = format!("ix://{PACKAGE_IDENTITY}/Parcel");
    let mut node = wire_type(
        &owner,
        kind,
        json!({"fields": [], "operations": operations, "supertypes": [], "identityFields": []}),
    );
    node.as_object_mut()
        .expect("type node")
        .remove("identityFields");
    if kind == "event_type" {
        let occurred = format!("{owner}/occurred");
        node["fields"] = json!([wire_field(
            &occurred,
            "occurred",
            "ix://filament/native/Timestamp",
            "required",
            1
        )]);
        node["occurrenceField"] = json!(occurred);
        node.as_object_mut()
            .expect("event node")
            .remove("identityFields");
    }
    document["types"].as_array_mut().expect("types").push(node);
    serde_json::to_vec(&document).expect("model JSON")
}

#[trace("TC-1307", "FR-512-AC-2", "FR-512-AC-16")]
#[test]
fn event_owner_refuses_before_operation_name_collisions() {
    use fsl_semantics::model::intake::meaning;
    assert_owner_precedence(
        "event_type",
        meaning::EVENT_TYPE,
        "invalid_model_binding",
        "malformed-declaration",
    );
}

#[trace("TC-1307", "FR-512-AC-3", "FR-512-AC-16")]
#[test]
fn record_owner_refuses_before_operation_name_collisions() {
    use fsl_semantics::model::intake::meaning;
    assert_owner_precedence(
        "record_value_type",
        meaning::RECORD_VALUE_TYPE,
        "unsupported_construct",
        "declaration-form",
    );
}

fn assert_owner_precedence(kind: &str, meaning: &str, code: &str, cause: &str) {
    admit(&extra_owner(kind, meaning, vec![])).expect("the original non-operation owner admits");
    let mut first = query("advance", json!([]));
    first["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/advance"));
    let mut second = query("other", json!([]));
    second["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/other"));
    second["name"] = json!("advance");
    let refused = admit(&extra_owner(kind, meaning, vec![first, second]))
        .expect_err("each operation must fail its owner rule before name validation");
    let UnitIntakeCause::Refused(found) = refused.cause else {
        panic!("{refused:?}");
    };
    assert_eq!(
        found.len(),
        2,
        "one owner refusal per operation, no operation-name refusal: {found:?}"
    );
    for (index, name) in ["advance", "other"].iter().enumerate() {
        assert_refusal(
            &found[index],
            code,
            cause,
            &format!("ix://{PACKAGE_IDENTITY}/Parcel/{name}"),
            &format!("TypeDefinition.operations[{index}]"),
        );
    }
}

#[trace("TC-1307", "FR-512-AC-5")]
#[test]
fn unrelated_objects_can_declare_the_same_operation_name() {
    use super::model_operations::wire_type;
    let mut document: Value =
        serde_json::from_slice(&input(vec![query("advance", json!([]))])).expect("original model");
    let mut other = query("advance", json!([]));
    let other_owner = format!("ix://{PACKAGE_IDENTITY}/Gauge");
    other["identity"] = json!(format!("{other_owner}/advance"));
    document["types"]
        .as_array_mut()
        .expect("types")
        .push(wire_type(
            &other_owner,
            "object_type",
            json!({"fields": [], "operations": [other], "supertypes": []}),
        ));
    let selected = admit(&serde_json::to_vec(&document).expect("model JSON"))
        .expect("names belong to different owners");
    let operations: Vec<_> = selected[0]
        .view
        .domain_package()
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::OperationMember(op) => Some(op.key.node.as_str()),
            _ => None,
        })
        .collect();
    assert!(operations.contains(&key("advance").as_str()));
    assert!(operations.contains(&format!("{other_owner}/advance").as_str()));
}

#[trace("TC-1307", "FR-512-AC-11", "FR-512-AC-15")]
#[test]
fn result_parameter_without_a_return_type_refuses_before_frame_resolution() {
    let mut op = operation(
        "advance",
        json!([operation_parameter("advance", "result", INTEGER)]),
        None,
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

#[trace("TC-1307", "FR-512-AC-12")]
#[test]
fn two_distinct_modifies_grants_keep_their_declared_order() {
    let mut op = query("advance", json!([]));
    op["frame"]["modifies"] = json!([key("versionNumber"), key("parent")]);
    let selected = admit(&input(vec![op])).expect("two different field grants admit");
    let effect = selected[0]
        .view
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(op) => Some(&op.effect),
            _ => None,
        })
        .expect("operation survives intake");
    assert_eq!(
        effect
            .modifies
            .iter()
            .map(|key| key.node.as_str())
            .collect::<Vec<_>>(),
        [key("versionNumber"), key("parent")]
    );
}

#[trace("TC-1307", "FR-512-AC-4", "FR-512-AC-17")]
#[test]
fn unrelated_record_order_does_not_retarget_a_duplicate_operation_identity() {
    let op = query("advance", json!([]));
    let mut document: Value =
        serde_json::from_slice(&input(vec![op.clone(), op])).expect("original input");
    document["types"].as_array_mut().expect("types").insert(
        0,
        super::model_operations::wire_type(
            &format!("ix://{PACKAGE_IDENTITY}/Gauge"),
            "object_type",
            json!({"fields": [], "operations": [], "supertypes": []}),
        ),
    );
    let forward = serde_json::to_vec(&document).expect("model bytes");
    document["types"].as_array_mut().expect("types").reverse();
    let reverse = serde_json::to_vec(&document).expect("reordered model bytes");
    for bytes in [&forward, &reverse] {
        refusal(
            bytes,
            "invalid_model_binding",
            "conflicting-binding",
            &key("advance"),
            "TypeDefinition.operations[1]",
        );
    }
}

/// Move only unrelated type records; operation, parameter and frame order stays authored.
fn unrelated_record_permutations(document: &[u8]) -> [Vec<u8>; 2] {
    let mut document: Value = serde_json::from_slice(document).expect("original model input");
    document["types"].as_array_mut().expect("types").insert(
        0,
        super::model_operations::wire_type(
            &format!("ix://{PACKAGE_IDENTITY}/Unrelated"),
            "object_type",
            json!({"fields": [], "operations": [], "supertypes": []}),
        ),
    );
    let first = serde_json::to_vec(&document).expect("model bytes");
    document["types"].as_array_mut().expect("types").reverse();
    [
        first,
        serde_json::to_vec(&document).expect("reordered model bytes"),
    ]
}

#[trace("TC-1307", "FR-512-AC-7", "FR-512-AC-15", "FR-512-AC-17")]
#[test]
fn unrelated_record_order_keeps_duplicate_parameter_identity_and_precedence_loci() {
    for bad_frame in [false, true] {
        let parameter = operation_parameter("advance", "amount", INTEGER);
        let mut operation = query("advance", json!([parameter.clone(), parameter]));
        if bad_frame {
            operation["frame"]["modifies"] = json!([key("missingField")]);
        }
        for document in unrelated_record_permutations(&input(vec![operation])) {
            let found = refusal(
                &document,
                "invalid_model_binding",
                "conflicting-binding",
                &key("advance/amount"),
                "TypeDefinition.operations[0].params[1]",
            );
            assert!(found.detail.contains("DUPLICATE_PARAM"), "{found:?}");
        }
    }
}

#[trace("TC-1307", "FR-512-AC-8", "FR-512-AC-17")]
#[test]
fn unrelated_record_order_keeps_the_readers_repeated_parameter_name_pointer() {
    let first = operation_parameter("advance", "amount", INTEGER);
    let mut second = operation_parameter("advance", "other", INTEGER);
    second["name"] = json!("amount");
    for document in
        unrelated_record_permutations(&input(vec![query("advance", json!([first, second]))]))
    {
        let source: Value = serde_json::from_slice(&document).expect("input JSON");
        let owner_index = source["types"]
            .as_array()
            .expect("types")
            .iter()
            .position(|node| node["identity"] == format!("ix://{PACKAGE_IDENTITY}/ConfigVersion"))
            .expect("operation owner in this input");
        let rejected = admit(&document).expect_err("reader rejects the repeated parameter name");
        let UnitIntakeCause::Refused(found) = rejected.cause else {
            panic!("{rejected:?}");
        };
        assert_eq!(found.len(), 1, "no additional intake refusal: {found:?}");
        assert!(found[0].detail.contains("DUPLICATE_PARAM"), "{found:?}");
        assert!(
            found[0].detail.contains(&format!(
                "/ir/types/{owner_index}/operations/0/params/1/name"
            )),
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
}

#[trace(
    "TC-1307",
    "FR-512-AC-2",
    "FR-512-AC-3",
    "FR-512-AC-16",
    "FR-512-AC-17"
)]
#[test]
fn unrelated_record_order_keeps_owner_refusals_ahead_of_operation_name_collisions() {
    use fsl_semantics::model::intake::meaning;
    for (kind, meaning, code, cause) in [
        (
            "event_type",
            meaning::EVENT_TYPE,
            "invalid_model_binding",
            "malformed-declaration",
        ),
        (
            "record_value_type",
            meaning::RECORD_VALUE_TYPE,
            "unsupported_construct",
            "declaration-form",
        ),
    ] {
        let mut first = query("advance", json!([]));
        first["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/advance"));
        let mut second = query("other", json!([]));
        second["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/other"));
        second["name"] = json!("advance");
        for document in
            unrelated_record_permutations(&extra_owner(kind, meaning, vec![first, second]))
        {
            let rejected = admit(&document).expect_err("owner refusals precede namespace checks");
            let UnitIntakeCause::Refused(found) = rejected.cause else {
                panic!("{rejected:?}");
            };
            assert_eq!(found.len(), 2, "one owner refusal per operation: {found:?}");
            for (index, name) in ["advance", "other"].iter().enumerate() {
                assert_refusal(
                    &found[index],
                    code,
                    cause,
                    &format!("ix://{PACKAGE_IDENTITY}/Parcel/{name}"),
                    &format!("TypeDefinition.operations[{index}]"),
                );
            }
        }
    }
}

#[trace("TC-1307", "FR-512-AC-5", "FR-512-AC-16", "FR-512-AC-17")]
#[test]
fn unrelated_record_order_keeps_both_equal_name_collision_participants() {
    for reserved_parameter in [false, true] {
        let first = query("advance", json!([]));
        let parameters = if reserved_parameter {
            json!([operation_parameter("other", "result", INTEGER)])
        } else {
            json!([])
        };
        let mut second = query("other", parameters);
        second["name"] = json!("advance");
        for document in unrelated_record_permutations(&input(vec![first, second])) {
            let found = refusal(
                &document,
                "ambiguous_declaration",
                "ambiguous-name",
                &key("other"),
                "TypeDefinition.operations[1].name",
            );
            assert_collision_participants(&found);
        }
    }
}

#[trace("TC-1307", "FR-512-AC-9", "FR-512-AC-15", "FR-512-AC-17")]
#[test]
fn unrelated_record_order_keeps_mismatched_parameter_name_and_precedence_loci() {
    for bad_frame in [false, true] {
        let mut parameter = operation_parameter("advance", "amount", INTEGER);
        parameter["name"] = json!("distance");
        let mut operation = query("advance", json!([parameter]));
        if bad_frame {
            operation["frame"]["modifies"] = json!([key("missingField")]);
        }
        for document in unrelated_record_permutations(&input(vec![operation])) {
            refusal(
                &document,
                "invalid_model_binding",
                "malformed-declaration",
                &key("advance/amount"),
                "TypeDefinition.operations[0].params[0].name",
            );
        }
    }
}

#[trace(
    "TC-1307",
    "FR-512-AC-10",
    "FR-512-AC-11",
    "FR-512-AC-15",
    "FR-512-AC-17"
)]
#[test]
fn unrelated_record_order_keeps_reserved_result_name_and_precedence_loci() {
    for returns in [None, Some(INTEGER)] {
        for bad_frame in [false, true] {
            let mut operation = operation(
                "advance",
                json!([operation_parameter("advance", "result", INTEGER)]),
                returns,
                empty_frame(),
            );
            if bad_frame {
                operation["frame"]["modifies"] = json!([key("missingField")]);
            }
            for document in unrelated_record_permutations(&input(vec![operation])) {
                refusal(
                    &document,
                    "ambiguous_declaration",
                    "ambiguous-name",
                    &key("advance/result"),
                    "TypeDefinition.operations[0].params[0].name",
                );
            }
        }
    }
}

/// A document whose `Parcel` owner of `kind` declares `operations`; the
/// owner is otherwise well formed.
fn operation_owner(kind: &str, operations: Vec<Value>) -> Vec<u8> {
    use super::model_operations::{wire_construct, wire_field, wire_type};
    use fsl_semantics::model::intake::meaning;
    let owner = format!("ix://{PACKAGE_IDENTITY}/Parcel");
    let id = format!("{owner}/id");
    let id_field = wire_field(&id, "id", "ix://filament/native/Boolean", "required", 1);
    let step = format!("{owner}/receive");
    let states: Vec<Value> = ["Draft", "Placed"]
        .iter()
        .map(|name| {
            let identity = format!("{owner}/{name}");
            json!({"identity": identity, "name": name, "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }}})
        })
        .collect();
    let (meaning, members, extra) = match kind {
        "state_machine" => (
            meaning::STATE_MACHINE,
            json!({"states": "required", "transitions": "required"}),
            json!({
                "fields": [],
                "states": states,
                "transitions": [{
                    "identity": format!("{owner}/advance_transition"),
                    "from": format!("{owner}/Draft"),
                    "to": format!("{owner}/Placed"),
                    "trigger": format!("{owner}/advance"),
                    "emits": [],
                    "origin": {"generated": {
                        "generatorIdentity": owner,
                        "generatorVersion": "1.0.0",
                        "inputIdentities": [owner],
                    }},
                }],
            }),
        ),
        "process" => (
            meaning::PROCESS,
            json!({"identityFields": "required", "steps": "required"}),
            json!({"identityFields": [id], "fields": [id_field], "steps": [{
                "identity": step, "name": "receive", "stepKind": "command",
                "consumes": [], "emits": [],
                "origin": {"generated": {
                    "generatorIdentity": step,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [step],
                }},
            }]}),
        ),
        "persistence_interface" => (
            meaning::PERSISTENCE_INTERFACE,
            json!({"persists": "required"}),
            json!({"fields": [], "persists": [format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")]}),
        ),
        other => panic!("no operation owner of kind {other}"),
    };
    let mut document: Value = serde_json::from_slice(&input(vec![])).expect("original model");
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(wire_construct(kind, meaning, members));
    let mut node = wire_type(&owner, kind, extra);
    node["operations"] = json!(operations);
    document["types"].as_array_mut().expect("types").push(node);
    serde_json::to_vec(&document).expect("model JSON")
}

fn parcel_operation(name: &str, identity_tail: &str, parameters: Value) -> Value {
    let mut op = query(name, parameters);
    op["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/{identity_tail}"));
    op
}

#[trace("TC-1307", "FR-512-AC-5")]
#[test]
fn equal_operation_names_refuse_for_every_admitted_owner_kind() {
    for kind in ["state_machine", "process", "persistence_interface"] {
        let first = parcel_operation("advance", "advance", json!([]));
        let second = parcel_operation("advance", "other", json!([]));
        admit(&operation_owner(kind, vec![first.clone()]))
            .unwrap_or_else(|refused| panic!("{kind} with one operation admits: {refused:?}"));
        let refused = admit(&operation_owner(kind, vec![first, second]))
            .expect_err("equal names of distinct identities must refuse");
        let UnitIntakeCause::Refused(found) = refused.cause else {
            panic!("{kind}: {refused:?}");
        };
        assert_eq!(found.len(), 1, "{kind}: {found:?}");
        assert_refusal(
            &found[0],
            "ambiguous_declaration",
            "ambiguous-name",
            &format!("ix://{PACKAGE_IDENTITY}/Parcel/other"),
            "TypeDefinition.operations[1].name",
        );
    }
}

#[trace("TC-1307", "FR-512-AC-2", "FR-512-AC-3", "FR-512-AC-16")]
#[test]
fn record_and_event_owner_refusals_precede_parameter_name_refusals() {
    use fsl_semantics::model::intake::meaning;
    let parameter = |name: &str| {
        let mut parameter = operation_parameter("advance", name, INTEGER);
        parameter["identity"] = json!(format!("ix://{PACKAGE_IDENTITY}/Parcel/advance/{name}"));
        parameter
    };
    let mut mismatched = parameter("amount");
    mismatched["name"] = json!("distance");
    let reserved = parameter("result");
    for (kind, owner_meaning, code, cause) in [
        (
            "event_type",
            meaning::EVENT_TYPE,
            "invalid_model_binding",
            "malformed-declaration",
        ),
        (
            "record_value_type",
            meaning::RECORD_VALUE_TYPE,
            "unsupported_construct",
            "declaration-form",
        ),
    ] {
        for parameter in [mismatched.clone(), reserved.clone()] {
            let operation = parcel_operation("advance", "advance", json!([parameter]));
            let refused = admit(&extra_owner(kind, owner_meaning, vec![operation]))
                .expect_err("the owner rule refuses the operation");
            let UnitIntakeCause::Refused(found) = refused.cause else {
                panic!("{kind}: {refused:?}");
            };
            assert_eq!(found.len(), 1, "{kind}: owner refusal only: {found:?}");
            assert_refusal(
                &found[0],
                code,
                cause,
                &format!("ix://{PACKAGE_IDENTITY}/Parcel/advance"),
                "TypeDefinition.operations[0]",
            );
        }
    }
}
