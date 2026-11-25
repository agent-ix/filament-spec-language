// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056-AC-33: a normalize-stage declaration refusal retains the artifact
//! and source position of the refused node's FCD origin, and a
//! `malformed-declaration` the semantic-IR reader raises at intake retains
//! them too. Typed vectors over a K2 package whose every node carries a source
//! origin. Assertions compare the typed code, cause, node, member path,
//! artifact and position, never detail text.

use super::*;

use ix_trace_rs::trace;

const ARTIFACT_COLUMN: u64 = 5;

fn spec_artifact() -> String {
    id("spec")
}

/// Gives every node of `value` a source origin at its own line (numbered in
/// document order) and records that line by node identity.
fn stamp_sources(value: &mut Value, lines: &mut BTreeMap<String, u64>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(identity)) = map.get("identity").cloned() {
                if map.contains_key("origin") {
                    let line = lines.len() as u64 + 1;
                    lines.insert(identity, line);
                    let source = json!({
                        "sourceIdentity": spec_artifact(),
                        "path": "spec.fspec",
                        "startLine": line,
                        "startColumn": ARTIFACT_COLUMN,
                    });
                    // The IR reader requires a source clause to carry its span.
                    if map.contains_key("clauseId") {
                        map.insert("sourceSpan".to_owned(), source.clone());
                    }
                    map.insert("origin".to_owned(), json!({ "source": source }));
                }
            }
            for child in map.values_mut() {
                stamp_sources(child, lines);
            }
        }
        Value::Array(items) => {
            for child in items {
                stamp_sources(child, lines);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// The K2 package mutated by `mutate`, every node sourced; the line of each
/// node by identity.
fn sourced(mutate: impl FnOnce(&mut Value)) -> (Value, BTreeMap<String, u64>) {
    let mut lines = BTreeMap::new();
    let document = mutate_k2(|document| {
        mutate(document);
        stamp_sources(document, &mut lines);
    });
    (document, lines)
}

fn refused(document: &Value) -> Vec<ModelRefusal> {
    let package = read_k2(document).expect("the semantic-IR reader admits the document");
    refused_package(&package)
}

fn refused_package(package: &DomainPackage) -> Vec<ModelRefusal> {
    match normalize_with_meter(package, ModelNormalizationLimits::default()).0 {
        NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
        other => panic!("the vector must refuse, got {other:?}"),
    }
}

/// `refusal` is a `malformed-declaration` at `node`/`member` that retains the
/// node's source artifact and position.
fn assert_retains_origin(
    refusal: &ModelRefusal,
    lines: &BTreeMap<String, u64>,
    node: &str,
    member: &str,
) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    let identity = if node.starts_with("ix://") {
        node.to_owned()
    } else {
        id(node)
    };
    let line = *lines
        .get(&identity)
        .unwrap_or_else(|| panic!("{identity} is a sourced node"));
    let ModelRefusalCause::IntakeMalformedDeclaration {
        node: actual,
        artifact,
        span,
        member: actual_member,
    } = &refusal.cause
    else {
        panic!("expected a malformed declaration, got {refusal:?}");
    };
    assert_eq!(actual, &identity);
    assert_eq!(actual_member, &super::member(member));
    assert_eq!(
        artifact.as_deref(),
        Some(spec_artifact().as_str()),
        "{refusal:?}"
    );
    let span = span.expect("the node's source position is retained");
    assert_eq!(
        (span.start.line as u64, span.start.column as u64),
        (line, ARTIFACT_COLUMN)
    );
}

fn only_refusal(
    mutate: impl FnOnce(&mut Value),
    node: &str,
    member: &str,
) -> (ModelRefusal, BTreeMap<String, u64>) {
    let (document, lines) = sourced(mutate);
    let mut found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    let refusal = found.remove(0);
    assert_retains_origin(&refusal, &lines, node, member);
    (refusal, lines)
}

/// A record value type that declares identity fields.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_record_value_type_with_identity_fields_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
        },
        "Money",
        "TypeDefinition.identityFields",
    );
}

/// An event type that declares identity fields.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_event_type_with_identity_fields_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderPlaced")["identityFields"] =
                json!([id("OrderPlaced/order_id")]);
        },
        "OrderPlaced",
        "TypeDefinition.identityFields",
    );
}

/// An event type whose occurrence field resolves to a non-Timestamp scalar:
/// refused by the semantic-IR reader at intake, and by normalization for a
/// package whose occurrence field is retargeted after the read.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_event_occurrence_that_is_not_a_timestamp_keeps_its_origin() {
    let (document, lines) = sourced(|document| {
        type_named_mut(document, "Instant")["target"] = json!("ix://filament/native/Integer");
    });
    let found = intake_refusals(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "OrderPlaced",
        "TypeDefinition.occurrenceField",
    );

    let (document, lines) = sourced(|_| {});
    let mut package = read_k2(&document).expect("the sourced K2 package reads");
    let event = package
        .records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::EventType(event) if event.key == package_key("OrderPlaced") => {
                Some(event)
            }
            _ => None,
        })
        .expect("OrderPlaced is an event type record");
    event.occurrence_field = Some(package_key("OrderPlaced/order_id"));
    let found = refused_package(&package);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "OrderPlaced",
        "TypeDefinition.occurrenceField",
    );
}

/// An operation an event type declares is refused at the operation's own node
/// and origin, not the event type's.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_operation_declared_by_an_event_type_keeps_its_own_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderPlaced")["operations"] =
                json!([operation("OrderPlaced", "replay")]);
        },
        "OrderPlaced/replay",
        "TypeDefinition.operations[0]",
    );
}

/// A state machine transition that emits a type of another meaning.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_state_machine_transition_emitting_a_non_event_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderLifecycle")["transitions"][0]["emits"] =
                json!([id("Order")]);
        },
        "OrderLifecycle",
        "TypeDefinition.transitions[0].emits[0]",
    );
}

/// A state machine that declares no states, and one that declares no
/// transitions.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_state_machine_missing_required_members_keeps_its_origin() {
    let (document, lines) = sourced(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"] = json!([]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "OrderLifecycle",
        "TypeDefinition.transitions",
    );

    // A machine with no operations and no transitions: both refusals, at the
    // machine.
    let (document, lines) = sourced(|document| {
        let machine = type_named_mut(document, "OrderLifecycle");
        machine["operations"] = json!([]);
        machine["transitions"] = json!([]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 2, "{found:?}");
    for (refusal, path) in found
        .iter()
        .zip(["TypeDefinition.operations", "TypeDefinition.transitions"])
    {
        assert_retains_origin(refusal, &lines, "OrderLifecycle", path);
    }

    // With its transitions kept, each trigger names nothing: the semantic-IR
    // reader refuses them and intake ends.
    let (document, _) = sourced(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"] = json!([]);
    });
    let found = intake_refusals(&document);
    assert_eq!(missing_names(&found), dangling_triggers(), "{found:?}");

    // Another node's reader refusal beside the machine's is reported exactly
    // once, in reader order.
    let (document, _) = sourced(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"] = json!([]);
        type_named_mut(document, "Order")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "ghost", &id("Nope")));
    });
    let found = intake_refusals(&document);
    let mut expected = vec![(
        id("Order/ghost"),
        id("Nope"),
        "/types/0/fields/2/typeRef".to_owned(),
    )];
    expected.extend(dangling_triggers());
    assert_eq!(missing_names(&found), expected, "{found:?}");
}

/// Node, absent declaration and reader pointer of each refusal, in report
/// order; every one must be a `missing_declaration`/`missing-name`.
fn missing_names(found: &[ModelRefusal]) -> Vec<(String, String, String)> {
    found
        .iter()
        .map(|refusal| {
            assert_eq!(refusal.code, Code::MissingDeclaration, "{refusal:?}");
            assert_eq!(refusal.cause.as_str(), "missing-name", "{refusal:?}");
            let ModelRefusalCause::IntakeMissingDeclaration {
                node,
                reference,
                path,
                ..
            } = &refusal.cause
            else {
                panic!("expected an intake missing-name: {refusal:?}");
            };
            (node.clone(), reference.clone(), path.as_str().to_owned())
        })
        .collect()
}

/// The three K2 transition triggers of an `OrderLifecycle` with no operations.
fn dangling_triggers() -> Vec<(String, String, String)> {
    [(0, "place"), (1, "cancel"), (2, "cancel")]
        .map(|(position, operation)| {
            (
                id("OrderLifecycle"),
                id(&format!("OrderLifecycle/{operation}")),
                format!("/types/4/transitions/{position}/trigger"),
            )
        })
        .to_vec()
}

/// A process subtype of a state machine that supplies no identity fields:
/// the identity refusal and then the supertype refusal, each at the process.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_process_without_identity_or_process_supertype_keeps_its_origin() {
    let (document, lines) = sourced(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [],
                "supertypes": [id("OrderLifecycle")],
                "operations": [],
                "steps": [{
                    "identity": id("ExpressFulfilment/expedite"),
                    "name": "expedite",
                    "stepKind": "command",
                    "consumes": [],
                    "emits": [],
                    "origin": origin("ExpressFulfilment/expedite"),
                }],
            }),
        ));
    });
    let found = refused(&document);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "ExpressFulfilment",
        "TypeDefinition.identityFields",
    );
    assert_retains_origin(
        &found[1],
        &lines,
        "ExpressFulfilment",
        "TypeDefinition.supertypes[0]",
    );
}

fn relationship(owner: &str, name: &str, target: &str) -> Value {
    let node = format!("relationship/{owner}-{name}-{target}");
    json!({
        "identity": id(&node),
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": origin(&node),
        "sourceEnd": {
            "role": name,
            "type": id(owner),
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        },
        "targetEnd": {
            "type": id(target),
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        },
    })
}

/// A relationship whose source end is not an object type.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_relationship_source_of_another_meaning_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Money")["relationships"] =
                json!([relationship("Money", "owner", "Order")]);
        },
        "relationship/Money-owner-Order",
        "TypeDefinition.relationships[0]",
    );
}

/// A relationship whose target end is not an object type.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_relationship_target_of_another_meaning_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Fulfilment")["relationships"] =
                json!([relationship("Fulfilment", "store", "OrderStore")]);
        },
        "relationship/Fulfilment-store-OrderStore",
        "Relationship.target",
    );
}

/// A node whose origin is `generated` has no source: artifact and span are
/// `None`, the one case the field documents.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_generated_origin_leaves_artifact_and_span_none() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    let ModelRefusalCause::IntakeMalformedDeclaration { artifact, span, .. } = &found[0].cause
    else {
        panic!("expected a malformed declaration: {found:?}");
    };
    assert_eq!((artifact, span), (&None, &None));
}

/// TC-235 M10 run 10: a population member naming a declaration of the wrong
/// meaning refuses `malformed-declaration` at the population node, path
/// `PopulationDefinition.members[i]`, keeping the population's origin. A
/// member naming no declaration stays `missing_declaration`, which the
/// semantic-IR reader raises at intake.
#[trace("FSpec-TC-235", "TC-1005", "FR-056-AC-33")]
#[test]
fn a_population_member_of_another_meaning_refuses_malformed_with_origin() {
    for wrong in [
        "Money",
        "OrderPlaced",
        "Instant",
        "OrderStore",
        "OrderManagement",
    ] {
        let (document, lines) = sourced(|document| {
            document["populations"] = json!([{
                "identity": id("PO"),
                "displayName": id("PO"),
                "kind": {"module": BUSINESS, "name": "population"},
                "members": [id("Order"), id(wrong)],
                "extent": "closed",
                "origin": origin("PO"),
            }]);
        });
        let found = refused(&document);
        assert_eq!(found.len(), 1, "{wrong}: {found:?}");
        assert_retains_origin(&found[0], &lines, "PO", "PopulationDefinition.members[1]");
    }

    let (document, _) = sourced(|document| {
        document["populations"] = json!([{
            "identity": id("PO"),
            "displayName": id("PO"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Missing")],
            "extent": "closed",
            "origin": origin("PO"),
        }]);
    });
    let found = intake_refusals(&document);
    assert_eq!(
        missing_names(&found),
        [(
            id("PO"),
            id("Missing"),
            "/populations/0/members/0".to_owned()
        )],
        "{found:?}"
    );
}

/// TC-235 M10 run 10 as the row is written: K2 plus population `PA` (closed)
/// with member types `[OrderManagement]` refuses after 22 charges,
/// `malformed-declaration` naming `PA` at `PopulationDefinition.members[0]`.
#[trace("FSpec-TC-235", "TC-1005", "FR-056-AC-33")]
#[test]
fn tc235_m10_run_10_population_of_a_namespace_refuses_at_members_zero() {
    let (document, lines) = sourced(|document| {
        document["populations"] = json!([{
            "identity": id("PA"),
            "displayName": id("PA"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("OrderManagement")],
            "extent": "closed",
            "origin": origin("PA"),
        }]);
    });
    let package = read_k2(&document).expect("K2 plus PA reads");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 22);
    let NormalizeOutcome::Refused(found) = outcome else {
        panic!("run 10 must refuse, got {outcome:?}");
    };
    assert_eq!(found.len(), 1, "{found:?}");
    assert_retains_origin(&found[0], &lines, "PA", "PopulationDefinition.members[0]");
}
