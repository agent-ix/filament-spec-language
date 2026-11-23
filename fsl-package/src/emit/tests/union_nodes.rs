// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-320 (TC-827): the union type node, the construction node and the
//! `case` node, lowered from source, emitted as `filament.checked-package/v2`
//! and read back through FSL's whole I2 read.

use super::*;
use filament_semantic_value::checking::CheckMode;

const SHAPE: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n";

/// FR-318-AC-1's `area` and a function returning `Shape::Rect(2, 3)`.
const FUNCTIONS: &str = "function area using v(s: Shape): Integer pure \
     { case s { Empty: 0; Circle(r): r; Rect(w, h): w * h; } }\n\
     function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

/// `declarations` emitted.
fn emitted(declarations: &str) -> Emission {
    emit_from_text(declarations).1
}

/// The nodes of `wire` with `form`.
fn of_form<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect()
}

#[trace("FR-320-AC-1", "TC-827")]
#[test]
fn the_union_keeps_its_id_and_its_members_their_identities_through_emission_and_the_i2_read() {
    let (_, package) = package_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let wire = wire(&emission);

    // FSpec's spelling: one `composite_type`/`union`, one `union_value`, one
    // `expression`/`case`.
    let [union] = of_form(&wire, "union")[..] else {
        panic!("one union node")
    };
    assert_eq!(union["node_tag"], "composite_type");
    assert_eq!(union["declaration"], json!({"qualified_name": ["Shape"]}));
    assert_eq!(of_form(&wire, "union_value").len(), 1);
    let [case] = of_form(&wire, "case")[..] else {
        panic!("one case node")
    };
    assert_eq!(case["node_tag"], "expression");

    // The I2 read admits it, and the id it exports is the emitted id.
    let exports = verified_exports(&emission);
    let union_id = fsl_foundation::node_wire::node_key_from_json(&union["node_id"])
        .expect("a structural union key");
    assert_eq!(exports["Shape"], union_id.to_string());

    // Each member's identity, checked before emission, is the FR-441 key over
    // the id the I2 read returned.
    let graph = package.graph();
    let types = graph.scope().types();
    let declaration = types
        .composites()
        .find(|declaration| declaration.name() == "Shape")
        .expect("Shape is declared");
    assert_eq!(
        types
            .union_key(declaration.key())
            .map(|key| key.to_string()),
        Some(union_id.to_string())
    );
    let CompositeShape::Union(members) = declaration.shape() else {
        panic!("Shape is a union")
    };
    for member in members {
        let variant = member.member().expect("sealed").variant().clone();
        assert_eq!(
            variant,
            fsl_semantics::value::enumeration::mint_union_variant_id(
                union_id.clone(),
                member.identifier(),
            )
            .expect("a union variant"),
            "{}",
            member.identifier()
        );
    }

    // The same source recompiles to the same package bytes.
    let (_, recompiled) = emit_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    assert_eq!(recompiled.package.bytes(), emission.package.bytes());
}

#[trace("FR-320-AC-2", "TC-827")]
#[test]
fn an_ir_that_cannot_decode_the_union_omits_it_and_every_node_naming_it() {
    let (text, package) = package_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    let regions = |_: &Location| {
        Some(SourceRegion::new(package.graph().source().clone(), 0, text.len() as u64).unwrap())
    };
    let cannot_decode_union = |tag: CheckedNodeTag, form: &str| {
        !(tag == CheckedNodeTag::CompositeType && form == "union")
            && CheckedNodeKind::decode(tag, form).is_some()
    };
    let emission = emit_package_decoding(
        &package,
        regions,
        &cannot_decode_union,
        crate::EmitLimits::fixture(),
    )
    .expect("the remaining nodes emit");

    let graph = package.graph().semantic_graph();
    let id_of = |form: &str| -> CheckedNodeId {
        let nodes: Vec<_> = graph
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .collect();
        let [node] = nodes[..] else {
            panic!("one {form} node")
        };
        node_id(node.key())
    };
    let union = id_of("union");
    let cause_of = |id: &CheckedNodeId| {
        emission
            .omitted
            .iter()
            .find(|omission| omission.node == *id)
            .map(|omission| &omission.cause)
    };
    assert!(matches!(
        cause_of(&union),
        Some(OmissionCause::UnsupportedForm {
            node_tag: NodeTag::CompositeType,
            semantic_form: "union"
        })
    ));
    // Everything that names the union, directly or through the nodes that
    // name it, is omitted for naming an omitted node: the construction, the
    // `case`, both functions and the parameter typed at the union.
    for form in ["union_value", "case", "parameter"] {
        let omitted = graph
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .map(|node| node_id(node.key()))
            .filter(|id| cause_of(id).is_some())
            .count();
        assert!(omitted >= 1, "a {form} node names the union");
    }
    let functions: Vec<_> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "pure_function")
        .map(|node| node_id(node.key()))
        .collect();
    assert_eq!(functions.len(), 2);
    for function in &functions {
        assert!(
            matches!(cause_of(function), Some(OmissionCause::NamesOmittedNode(_))),
            "{function:?}: {:?}",
            emission.omitted
        );
    }
    // The scalar `Integer` node names no union and is written.
    let integer = id_of("integer");
    assert_eq!(cause_of(&integer), None);

    // The wire holds none of the omitted nodes and none names one.
    let wire = wire(&emission);
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wire).unwrap();
    let written: BTreeSet<NodeKey> = graph.keys().iter().cloned().collect();
    for omission in &emission.omitted {
        let id = serde_json::to_value(&omission.node).expect("an omitted id encodes");
        let key =
            fsl_foundation::node_wire::node_key_from_json(&id).expect("an omitted structural key");
        assert!(!written.contains(&key), "{omission:?}");
    }
    assert!(!written.is_empty());
    for node in nodes(&wire) {
        for dependency in node["dependencies"].as_array().expect("dependencies") {
            let target = graph
                .key_of_value(dependency)
                .expect("a structural dependency");
            assert!(written.contains(&target), "{node} names an omitted node");
        }
    }
}

/// The `value-validity` records of `package`, with their claims.
fn value_validity_records(
    package: &CheckedPackage,
) -> Vec<(&OccurrenceKey, &fsl_semantics::check::RequirementRecord)> {
    package
        .graph()
        .requirements()
        .iter()
        .filter(|(_, record)| {
            record.requirements().kind() == fsl_semantics::check::Capability::ValueValidity
        })
        .collect()
}

/// FR-062 "Packaging is all-or-nothing" and FSpec FR-825, for `declarations`
/// emitted by an IR that cannot decode `refused`: the owner of each
/// `value-validity` record `selected` picks is omitted with cause
/// `NamesOmittedNode`; every record of an omitted owner is omitted naming
/// that owner, and the package holds neither its node, nor its
/// `requirement_items` row, nor its `generated` source-map entry; an
/// application is omitted once no record of it survives. Every record of a
/// written owner keeps its node and row, and the strict reader admits the
/// package.
fn records_go_with_their_applications(
    declarations: &str,
    refused: (CheckedNodeTag, &'static str),
    selected: &dyn Fn(&OccurrenceKey, &fsl_semantics::check::RequirementRecord) -> bool,
) -> (CheckedPackage, Emission) {
    let (text, package) = package_from_text(declarations);
    let regions = |_: &Location| {
        Some(SourceRegion::new(package.graph().source().clone(), 0, text.len() as u64).unwrap())
    };
    let decodes = |tag: CheckedNodeTag, form: &str| {
        !(tag == refused.0 && form == refused.1) && CheckedNodeKind::decode(tag, form).is_some()
    };
    let emission = emit_package_decoding(&package, regions, &decodes, crate::EmitLimits::fixture())
        .expect("the remaining nodes emit");
    let cause_of = |id: &CheckedNodeId| {
        emission
            .omitted
            .iter()
            .find(|omission| omission.node == *id)
            .map(|omission| &omission.cause)
    };

    let records = value_validity_records(&package);
    let owner_of = |record: &fsl_semantics::check::RequirementRecord| {
        node_id(
            package
                .graph()
                .function_identity(record.owner().expect("a value-validity owner"))
                .expect("the owner is a function"),
        )
    };
    let chosen: Vec<_> = records
        .iter()
        .filter(|(claim, record)| selected(claim, record))
        .collect();
    assert!(!chosen.is_empty(), "the fixture holds a selected record");
    for (claim, record) in &chosen {
        assert!(
            matches!(
                cause_of(&owner_of(record)),
                Some(OmissionCause::NamesOmittedNode(_))
            ),
            "{claim:?}: the owner is omitted: {:?}",
            emission.omitted
        );
    }
    // An application is omitted exactly when every record of it has an
    // omitted owner, or when the graph's own closure omits it.
    for (claim, _) in &records {
        let owners_gone = records
            .iter()
            .filter(|(other, _)| other.node() == claim.node())
            .all(|(_, record)| cause_of(&owner_of(record)).is_some());
        if owners_gone {
            assert!(
                cause_of(&node_id(claim.node())).is_some(),
                "{claim:?}: no record of the application survives: {:?}",
                emission.omitted
            );
        }
    }
    let omitted_owners: Vec<CheckedNodeId> = records
        .iter()
        .map(|(_, record)| owner_of(record))
        .filter(|owner| cause_of(owner).is_some())
        .collect();
    let written = records.len() - omitted_owners.len();

    // The omitted nodes the checked graph does not hold are the record
    // nodes, one per record with an omitted owner, each naming that owner.
    let graph_ids: BTreeSet<CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| node_id(node.key()))
        .collect();
    let record_omissions: Vec<&OmittedNode> = emission
        .omitted
        .iter()
        .filter(|omission| !graph_ids.contains(&omission.node))
        .collect();
    assert_eq!(
        record_omissions.len(),
        omitted_owners.len(),
        "{:?}",
        emission.omitted
    );
    for omission in &record_omissions {
        assert!(
            matches!(&omission.cause, OmissionCause::NamesOmittedNode(owner)
                if omitted_owners.contains(owner)),
            "{omission:?}"
        );
    }

    let wire = wire(&emission);
    let graph = WireGraph::of_package_value(&wire).unwrap();
    let omitted_keys: BTreeSet<NodeKey> = emission
        .omitted
        .iter()
        .map(|omission| {
            let id = serde_json::to_value(&omission.node).expect("an omitted id encodes");
            fsl_foundation::node_wire::node_key_from_json(&id).expect("an omitted structural key")
        })
        .collect();
    assert!(graph.keys().iter().all(|key| !omitted_keys.contains(key)));
    for entry in wire["source_map"].as_array().expect("a source map") {
        let key = graph
            .key_of_value(&entry["node_id"])
            .expect("a source-map entry names a written node");
        assert!(!omitted_keys.contains(&key), "{entry}");
    }
    assert_eq!(of_form(&wire, "value_validity").len(), written);
    let admitted = match filament_contract_model::CheckedPackageV2::read(
        emission.package().bytes(),
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        filament_contract_model::CheckedPackageV2ReadResult::Admitted(admitted) => admitted,
        failure => panic!("the strict reader admits the remaining package: {failure:?}"),
    };
    let rows = admitted
        .requirement_items()
        .iter()
        .filter(|row| row.kind == filament_contract_model::CapabilityKind::ValueValidity)
        .count();
    assert_eq!(rows, written);
    verified_exports(&emission);
    (package, emission)
}

/// An omitted owner: `area` names the union an IR cannot decode, so `area`,
/// its `w * h` application and that application's record are omitted, while
/// `keep`'s `x + 1` keeps its record and row.
#[trace("FR-062-AC-9", "FR-320-AC-2", "FR-940-AC-1")]
#[test]
fn omitting_a_function_omits_its_value_validity_records_rows_and_source_map_entries() {
    let declarations = format!(
        "{SHAPE}{FUNCTIONS}function keep using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n"
    );
    let (_, package) = package_from_text(&declarations);
    let owned = |owner: &str| {
        value_validity_records(&package)
            .iter()
            .filter(|(_, record)| record.owner() == Some(owner))
            .count()
    };
    assert!(
        owned("area") >= 1,
        "`w * h` is a scalar claim site of `area`"
    );
    assert_eq!(
        owned("keep"),
        1,
        "`x + 1` is the one scalar claim site of `keep`"
    );
    records_go_with_their_applications(
        &declarations,
        (CheckedNodeTag::CompositeType, "union"),
        &|_, record| record.owner() == Some("area"),
    );
}

/// An omitted guard: the `if` condition `pos(x)` is a call an IR cannot
/// decode. The application `x + 1` in its branch names only its operands,
/// and its record names the guard, so the application is omitted with it.
#[trace("FR-062-AC-9", "FR-940-AC-1")]
#[test]
fn an_omitted_guard_omits_the_application_it_guards() {
    records_go_with_their_applications(
        "predicate pos using v(x: Int[0, 9]): Boolean { x > 0 }\n\
         function g using v(x: Int[0, 9]): Integer pure { if pos(x) then x + 1 else 0 }\n\
         function keep using v(y: Int[0, 5]): Integer pure { y * 3 }\n",
        (CheckedNodeTag::Expression, "call"),
        &|_, record| record.owner() == Some("g") && !record.path_condition().is_empty(),
    );
}

/// An omitted narrow: the narrow that supplies `a mod b`'s result bound is a
/// conversion an IR cannot decode. It names the application, not the
/// reverse, so the application is omitted through its record.
#[trace("FR-062-AC-9", "FR-940-AC-1")]
#[test]
fn an_omitted_narrow_omits_the_application_it_bounds() {
    records_go_with_their_applications(
        "function n using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 20] pure { a mod b }\n\
         function keep using v(p: Boolean): Boolean pure { not p }\n",
        (CheckedNodeTag::Expression, "conversion"),
        &|_, record| record.conversion_context().is_some(),
    );
}

/// `a` names the union an IR cannot decode; `b` does not. Both bodies hold
/// the structurally equal `x + 1`, one application node with one record per
/// owner.
const SHARED: &str = "function a using v(x: Int[0, 9], s: Shape): Integer pure \
     { case s { Empty: x + 1; Circle(r): r; Rect(w, h): 0; } }\n";

/// The claim of `a`'s record of `x + 1`, whose application node `b`'s
/// record shares.
fn shared_application(package: &CheckedPackage) -> OccurrenceKey {
    let records = value_validity_records(package);
    let of = |owner: &str| -> Vec<&OccurrenceKey> {
        records
            .iter()
            .filter(|(_, record)| record.owner() == Some(owner))
            .map(|(claim, _)| *claim)
            .collect()
    };
    let (a, b) = (of("a"), of("b"));
    assert_eq!(a.len(), 1, "`x + 1` is `a`'s one scalar claim site");
    assert_eq!(b.len(), 1, "`x + 1` is `b`'s one scalar claim site");
    assert_eq!(a[0].node(), b[0].node(), "the two records share one node");
    assert_ne!(a[0], b[0], "each record is its own occurrence");
    a[0].clone()
}

/// FR-062 "Packaging is all-or-nothing", FSpec FR-825: omitting `a` omits
/// `a`, its record, row and source-map entry, but not the application it
/// shares with the written `b`, whose record is the application's only
/// written record.
#[trace("FR-062-AC-9", "FR-940-AC-1")]
#[test]
fn an_omitted_owner_leaves_a_shared_application_with_the_written_owners_record() {
    let declarations =
        format!("{SHAPE}{SHARED}function b using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n");
    let (package, emission) = records_go_with_their_applications(
        &declarations,
        (CheckedNodeTag::CompositeType, "union"),
        &|_, record| record.owner() == Some("a"),
    );
    let claim = shared_application(&package);
    let application = node_id(claim.node());
    let function = |name: &str| node_id(package.graph().function_identity(name).unwrap());
    let omitted: Vec<&CheckedNodeId> = emission
        .omitted
        .iter()
        .map(|omission| &omission.node)
        .collect();
    assert!(omitted.contains(&&function("a")), "{omitted:?}");
    assert!(!omitted.contains(&&function("b")), "{omitted:?}");
    assert!(!omitted.contains(&&application), "{omitted:?}");
    assert!(
        emission.omitted.iter().any(|omission| omission.cause
            == OmissionCause::NamesOmittedNode(function("a"))
            && omission.node != application),
        "`a`'s record is omitted naming `a`: {:?}",
        emission.omitted
    );

    let wire = wire(&emission);
    let graph = WireGraph::of_package_value(&wire).unwrap();
    let admitted = match filament_contract_model::CheckedPackageV2::read(
        emission.package().bytes(),
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        filament_contract_model::CheckedPackageV2ReadResult::Admitted(admitted) => admitted,
        failure => panic!("the strict reader admits the package: {failure:?}"),
    };
    let key_of = |id: &CheckedNodeId| {
        graph
            .key_of_value(&serde_json::to_value(id).unwrap())
            .expect("a written node")
    };
    let of_application: Vec<_> = admitted
        .requirement_items()
        .iter()
        .filter_map(|row| row.record.as_ref())
        .filter(|record| key_of(&record.application) == *claim.node())
        .collect();
    assert_eq!(of_application.len(), 1, "only `b`'s record of `x + 1`");
    assert_eq!(
        key_of(&of_application[0].owner),
        package.graph().function_identity("b").unwrap()
    );
}

/// With both owners omitted no record of the shared `x + 1` survives, so the
/// application is omitted too.
#[trace("FR-062-AC-9", "FR-940-AC-1")]
#[test]
fn a_shared_application_is_omitted_once_every_owner_is() {
    let declarations = format!(
        "{SHAPE}{SHARED}function b using v(x: Int[0, 9], s: Shape): Integer pure \
         {{ case s {{ Empty: 0; Circle(r): x + 1; Rect(w, h): w; }} }}\n\
         function keep using v(p: Boolean): Boolean pure {{ not p }}\n"
    );
    let (package, emission) = records_go_with_their_applications(
        &declarations,
        (CheckedNodeTag::CompositeType, "union"),
        &|_, record| {
            record
                .owner()
                .is_some_and(|owner| owner == "a" || owner == "b")
        },
    );
    let claim = shared_application(&package);
    assert!(
        matches!(
            emission
                .omitted
                .iter()
                .find(|omission| omission.node == node_id(claim.node()))
                .map(|omission| &omission.cause),
            Some(OmissionCause::NamesOmittedNode(_))
        ),
        "{:?}",
        emission.omitted
    );
}

#[trace("FR-320-AC-4", "FR-319-AC-4", "TC-827")]
#[test]
fn a_construction_is_emitted_with_its_application_preimage_key() {
    let emission = emitted(&format!(
        "{SHAPE}function empty using v(): Shape pure {{ Shape::Empty }}\n\
         function rect using v(a: Integer, b: Integer): Shape pure {{ Shape::Rect(a, b) }}\n"
    ));
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let values = of_form(&wire, "union_value");
    assert_eq!(
        values.len(),
        2,
        "the nullary and the reference-payload vector"
    );
    let [union] = of_form(&wire, "union")[..] else {
        panic!("one union node")
    };
    for value in values {
        let member = value["body"]["members"][0]["name"]
            .as_str()
            .expect("a member name");
        let payload = &value["body"]["members"][0]["value"];
        assert_eq!(value["body"]["term"], "aggregate", "{member}");
        assert_eq!(value["body"]["members"].as_array().map(Vec::len), Some(1));
        assert_eq!(payload["term"], "aggregate", "{member}");
        let payload_terms = payload["members"].as_array().expect("payload terms");
        assert_eq!(payload_terms.len(), if member == "Empty" { 0 } else { 2 });
        assert!(
            payload_terms.iter().all(|term| term["term"] == "reference"),
            "{member}: payloads are reference terms"
        );
        assert_eq!(value["semantic_type"], union["node_id"], "{member}");
        // No `declaration`, no `owner`, no recursion group.
        assert!(value.get("declaration").is_none(), "{member}");
        assert!(value.get("owner").is_none(), "{member}");
        assert!(value.get("recursion_group").is_none(), "{member}");
    }
    // FSL's whole I2 read admits those keys.
    verified_exports(&emission);
}

/// FR-062-AC-9 (TC-160 step 8): an unlocked enum and each node that names it
/// are omitted; the independent function is written and reads back Verified.
#[trace("TC-160", "FR-062-AC-9")]
#[test]
fn a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written() {
    let foreign = super::status_enum(
        json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model", "package_name": "example-model"}),
        "Foreign",
    );
    let foreign_key = foreign.declaration.key();
    let text = b"function area(value: Foreign): Boolean pure { value = Foreign::Ready }\n\
                 function keep(): Boolean pure { true }\n";
    let area_text = b"function area(value: Foreign): Boolean pure { value = Foreign::Ready }";
    let area_type_text = b"Foreign";
    let area_type_start = text
        .windows(area_type_text.len())
        .position(|window| window == area_type_text)
        .expect("area parameter type is in source");
    let area_body_text = b"value = Foreign::Ready";
    let area_body_start = text
        .windows(area_body_text.len())
        .position(|window| window == area_body_text)
        .expect("area body is in source");
    let keep_start = text
        .windows(b"function keep".len())
        .position(|window| window == b"function keep")
        .expect("keep declaration is in source");
    let keep_text = b"function keep(): Boolean pure { true }";
    let keep_body_text = b"true";
    let keep_body_start = text[keep_start..]
        .windows(keep_body_text.len())
        .position(|window| window == keep_body_text)
        .map(|offset| keep_start + offset)
        .expect("keep body is in source");
    let mut area_body_spans = ExpressionSpans::new(fsl_foundation::Span {
        start: area_body_start,
        end: area_body_start + area_body_text.len(),
    })
    .expect("area body span");
    let area_body_root = area_body_spans.root();
    let parameter_reference = b"value";
    let parameter_reference_start = text[area_body_start..]
        .windows(parameter_reference.len())
        .position(|window| window == parameter_reference)
        .map(|offset| area_body_start + offset)
        .expect("area body parameter reference is in source");
    area_body_spans
        .push_child(
            area_body_root,
            fsl_foundation::Span {
                start: parameter_reference_start,
                end: parameter_reference_start + parameter_reference.len(),
            },
        )
        .expect("parameter reference span is inside area body");
    let ready_reference = b"Foreign::Ready";
    let ready_reference_start = text[area_body_start..]
        .windows(ready_reference.len())
        .position(|window| window == ready_reference)
        .map(|offset| area_body_start + offset)
        .expect("area body Ready reference is in source");
    area_body_spans
        .push_child(
            area_body_root,
            fsl_foundation::Span {
                start: ready_reference_start,
                end: ready_reference_start + ready_reference.len(),
            },
        )
        .expect("Ready reference span is inside area body");
    let area = FunctionDeclaration::new(
        "area",
        vec![(
            "value".to_owned(),
            TypeForm::name(
                "Foreign",
                fsl_foundation::Span {
                    start: area_type_start,
                    end: area_type_start + area_type_text.len(),
                },
            ),
        )],
        boolean(),
        None,
        Expression::binary(BinaryOperator::Equal, name("value"), name("Foreign::Ready")),
    )
    .with_spans(DeclarationSpans {
        declaration: fsl_foundation::Span {
            start: 0,
            end: area_text.len(),
        },
        body: area_body_spans,
        measure: None,
    })
    .expect("area spans match its body");
    let keep = FunctionDeclaration::new(
        "keep",
        Vec::new(),
        boolean(),
        None,
        Expression::boolean(true),
    )
    .with_spans(DeclarationSpans {
        declaration: fsl_foundation::Span {
            start: keep_start,
            end: keep_start + keep_text.len(),
        },
        body: ExpressionSpans::new(fsl_foundation::Span {
            start: keep_body_start,
            end: keep_body_start + keep_body_text.len(),
        })
        .expect("keep body span"),
        measure: None,
    })
    .expect("keep spans match its body");
    let source =
        fsl_semantics::check::admitted_source(fsl_foundation::SourceIdentity::new("a", "u"), text);
    let mut declarations =
        PackageDeclarations::new(source, "pkg", fsl_foundation::IdentityLimits::default());
    declarations.enums = vec![foreign];
    declarations.functions = vec![area, keep];
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .expect("the two functions check");
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package)
        .expect("emit_checked omits the unlocked enum instead of trying to place it");
    let exports = verified_exports(&emission);
    assert_eq!(
        exports.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["keep"]),
        "the I2 read exports only the independent function"
    );

    let graph = package.graph().semantic_graph();
    let id_for_key = |key: &NodeKey| node_id(key.clone());
    let foreign_id = id_for_key(&foreign_key);
    let area_id = id_for_key(
        &package
            .graph()
            .function_identity("area")
            .expect("area exists"),
    );
    let ready_key = graph
        .nodes()
        .find(|node| {
            node.semantic_form() == "enum_value"
                && fsl_foundation::node_wire::node_key_to_json(&node.key())
                    .expect("an enum value key encodes")["path"]
                    .as_array()
                    .and_then(|path| path.last())
                    == Some(&json!("Ready"))
        })
        .expect("the graph holds the Ready enum value")
        .key();
    let ready_id = id_for_key(&ready_key);
    let parameter_id = id_for_key(
        &graph
            .nodes()
            .find(|node| node.semantic_form() == "parameter")
            .expect("area has a value parameter")
            .key(),
    );
    let body_id = id_for_key(
        &graph
            .nodes()
            .find(|node| node.semantic_form() == "binary")
            .expect("area body is an equality")
            .key(),
    );
    let keep_id = id_for_key(
        &package
            .graph()
            .function_identity("keep")
            .expect("keep exists"),
    );
    let cause_of = |id: &CheckedNodeId| {
        emission
            .omitted
            .iter()
            .find(|omission| omission.node == *id)
            .map(|omission| &omission.cause)
    };
    assert!(matches!(
        cause_of(&foreign_id),
        Some(OmissionCause::UnlockedOwner)
    ));
    assert!(matches!(
        cause_of(&area_id),
        Some(OmissionCause::NamesOmittedNode(_))
    ));
    assert_eq!(
        cause_of(&ready_id),
        Some(&OmissionCause::NamesOmittedNode(foreign_id.clone())),
        "Ready is omitted because its enum declaration is unlocked"
    );
    assert_eq!(
        cause_of(&parameter_id),
        Some(&OmissionCause::NamesOmittedNode(foreign_id.clone())),
        "the value parameter is omitted because its type is unlocked"
    );
    assert_eq!(
        cause_of(&body_id),
        Some(&OmissionCause::NamesOmittedNode(ready_id.clone())),
        "the equality body is omitted because it names Ready"
    );
    assert_eq!(
        cause_of(&area_id),
        Some(&OmissionCause::NamesOmittedNode(parameter_id)),
        "the area declaration is omitted because its parameter is omitted"
    );
    assert_eq!(cause_of(&keep_id), None, "keep is written");

    // The checked package itself is retained by the second emission path.
    // This is the same package as the first half, but no source occurrence is
    // placeable, so there are no bytes to return.
    let refusal = emit_package(&package, |_| None).expect_err("the package has no placed regions");
    assert!(
        matches!(refusal, EmitRefusal::UnlocatedOccurrence { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), Code::UnsupportedProjection);
}

const READING: &str = "union Reading { Raw(Integer), Wide(Int[0, 2000]), Tiny(Int[0, 9]) }\n";

/// The node `reference` names.
fn node_named<'w>(wire: &'w Value, reference: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == *reference)
        .expect("the reference names a node")
}

/// The type node the union declares for the payload of `member`.
fn declared_payload<'w>(wire: &'w Value, member: &str) -> &'w Value {
    let [union] = of_form(wire, "union")[..] else {
        panic!("one union node")
    };
    let declared = union["body"]["members"]
        .as_array()
        .expect("union members")
        .iter()
        .find(|binding| binding["name"] == member)
        .expect("the member is declared");
    &declared["value"]["members"][0]["target"]
}

/// IR's strict-reader verdict on the package envelope `wire`: the refusal's
/// code, cause and the node it is located at, `None` when it admits.
fn strict_verdict(wire: &Value) -> Option<(String, Option<String>, String)> {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    match filament_contract_model::read_checked_package(
        &jcs(wire),
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => None,
        filament_contract_model::CheckedPackageDispatchResult::Refused(refusal) => Some((
            format!("{:?}", refusal.code),
            refusal.cause.map(|cause| format!("{cause:?}")),
            refusal.loci[0]
                .path
                .as_ref()
                .map_or(String::new(), |path| path.as_str().to_owned()),
        )),
        other => panic!("not a V2 verdict: {other:?}"),
    }
}

/// `key` with every occurrence of `from` replaced by `to` among its operands.
fn substituted(key: &NodeKey, from: &NodeKey, to: &NodeKey) -> NodeKey {
    if key == from {
        return to.clone();
    }
    let path = key
        .path()
        .into_iter()
        .map(|segment| match segment {
            PathSegment::Node(operand) => PathSegment::Node(substituted(&operand, from, to)),
            text => text,
        })
        .collect();
    NodeKey::new(key.package(), path, key.ordinal()).expect("a well-formed key")
}

/// Whether `value` holds a node id that names one of `keys`.
fn names_one_of(value: &Value, graph: &WireGraph, keys: &BTreeSet<NodeKey>) -> bool {
    match value {
        Value::Object(members)
            if members.contains_key("package") && members.contains_key("path") =>
        {
            graph.key_of_value(value).is_ok_and(|key| {
                keys.contains(&key)
                    || key.path().iter().any(|segment| {
                        matches!(segment, PathSegment::Node(operand) if keys.contains(operand))
                    })
            })
        }
        Value::Object(members) => members
            .values()
            .any(|member| names_one_of(member, graph, keys)),
        Value::Array(items) => items.iter().any(|item| names_one_of(item, graph, keys)),
        _ => false,
    }
}

/// The wire form of `key` in a graph whose nodes, in order, are `order`.
fn wire_id_in(key: &NodeKey, order: &BTreeMap<NodeKey, usize>) -> Value {
    let path: Vec<Value> = key
        .path()
        .into_iter()
        .map(|segment| match segment {
            PathSegment::Text(text) => json!(text),
            PathSegment::Node(operand) => json!({"node": order[&operand]}),
        })
        .collect();
    json!({"ordinal": key.ordinal(), "package": key.package(), "path": path})
}

/// `value` with every node id it holds re-derived through `renamed` and
/// `order`.
fn reidentified(
    value: &mut Value,
    graph: &WireGraph,
    renamed: &dyn Fn(&NodeKey) -> NodeKey,
    order: &BTreeMap<NodeKey, usize>,
) {
    match value {
        Value::Object(members)
            if members.contains_key("package") && members.contains_key("path") =>
        {
            let key = graph
                .key_of_value(&Value::Object(members.clone()))
                .expect("a node id");
            *value = wire_id_in(&renamed(&key), order);
        }
        Value::Object(members) => members
            .values_mut()
            .for_each(|member| reidentified(member, graph, renamed, order)),
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| reidentified(item, graph, renamed, order)),
        _ => {}
    }
}

/// `wire` with the nodes at `dropped` removed, every id naming one of them
/// replaced by the id of `replacement` (when given), and every node's
/// identity and `dependencies` re-derived: a well-formed package the emitter
/// did not write. A `claim`/`value_validity` record node naming a removed
/// node is removed with it (FR-062 "Packaging is all-or-nothing"), and with
/// it the `requirement_items` row that names it (FSpec FR-825).
fn edited(wire: &Value, dropped: &[usize], replacement: Option<(usize, usize)>) -> Value {
    let graph = WireGraph::of_package_value(wire).expect("a structural graph");
    let keys = graph.keys().to_vec();
    // A replacement re-points names of a removed node; a record whose
    // application is removed is still removed, because a record re-pointed
    // at the replacement would name an application it does not describe.
    let removed: BTreeSet<NodeKey> = dropped.iter().map(|at| keys[*at].clone()).collect();
    let mut dropped = dropped.to_vec();
    for (at, node) in nodes(wire).iter().enumerate() {
        let names_removed = node["dependencies"]
            .as_array()
            .expect("dependencies")
            .iter()
            .any(|dependency| {
                removed.contains(&graph.key_of_value(dependency).expect("a dependency"))
            });
        if node["semantic_form"] == "value_validity" && names_removed && !dropped.contains(&at) {
            dropped.push(at);
        }
    }
    let dropped = dropped.as_slice();
    let gone: BTreeSet<NodeKey> = dropped.iter().map(|at| keys[*at].clone()).collect();
    let renamed = |key: &NodeKey| match replacement {
        Some((from, to)) => substituted(key, &keys[from], &keys[to]),
        None => key.clone(),
    };
    let mut kept: Vec<(NodeKey, Value)> = nodes(wire)
        .iter()
        .enumerate()
        .filter(|(at, _)| !dropped.contains(at))
        .map(|(at, node)| (renamed(&keys[at]), node.clone()))
        .collect();
    kept.sort_by(|left, right| left.0.cmp(&right.0));
    let order: BTreeMap<NodeKey, usize> = kept
        .iter()
        .enumerate()
        .map(|(at, (key, _))| (key.clone(), at))
        .collect();
    let mut rewritten = wire.clone();
    // Every member other than the graph drops the rows that name a removed
    // node and re-derives the ids of the rest.
    if let Value::Object(members) = &mut rewritten {
        for (name, member) in members.iter_mut() {
            if name == "semantic_graph" {
                continue;
            }
            if let Value::Array(items) = member {
                items.retain(|item| !names_one_of(item, &graph, &gone));
            } else if let Value::Object(tables) = member {
                for table in tables.values_mut() {
                    if let Value::Array(items) = table {
                        items.retain(|item| !names_one_of(item, &graph, &gone));
                    }
                }
            }
            reidentified(member, &graph, &renamed, &order);
        }
        // FSpec FR-825: `requirement_items` is absent exactly when the
        // package has no requirement record.
        if members
            .get("requirement_items")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            members.remove("requirement_items");
        }
    }
    let mut graph_nodes = Vec::new();
    for (_, mut node) in kept {
        let positions: Vec<usize> = node["dependencies"]
            .as_array()
            .expect("dependencies")
            .iter()
            .map(|dependency| {
                order[&renamed(&graph.key_of_value(dependency).expect("a dependency"))]
            })
            .collect();
        reidentified(&mut node, &graph, &renamed, &order);
        let moved: Vec<Value> = node["dependencies"].as_array().unwrap().clone();
        let mut keyed: Vec<(usize, Value)> = positions.into_iter().zip(moved).collect();
        keyed.sort_by_key(|(at, _)| *at);
        keyed.dedup_by_key(|(at, _)| *at);
        node["dependencies"] = Value::Array(
            keyed
                .into_iter()
                .map(|(_, dependency)| dependency)
                .collect(),
        );
        graph_nodes.push(node);
    }
    rewritten["semantic_graph"]["nodes"] = Value::Array(graph_nodes);
    rewritten
}

/// The `(member, payload node)` of every `union_value` node of `wire`.
fn constructions(wire: &Value) -> Vec<(String, &Value)> {
    of_form(wire, "union_value")
        .into_iter()
        .map(|value| {
            let binding = &value["body"]["members"][0];
            let member = binding["name"].as_str().expect("a member name").to_owned();
            let payload = &binding["value"]["members"];
            assert_eq!(payload.as_array().map(Vec::len), Some(1), "{member}");
            (member, node_named(wire, &payload[0]["target"]))
        })
        .collect()
}

/// FR-317-AC-7 (TC-3134): a union-member construction widens a bounded
/// integer payload argument by the exact conversion a function-call argument
/// gets, an `Integer` argument and a literal take none, and each payload term
/// has the member's declared payload type node.
#[trace("TC-3134", "FR-317-AC-7")]
#[test]
fn a_union_payload_argument_takes_the_exact_conversion_a_call_argument_takes() {
    let emission = emitted(&format!(
        "{READING}function g using v(a: Integer): Boolean pure {{ true }}\n\
         function call using v(x: Int[0, 1000]): Boolean pure {{ g(x) }}\n\
         function raw using v(x: Int[0, 1000]): Reading pure {{ Reading::Raw(x) }}\n\
         function wide using v(x: Int[0, 1000]): Reading pure {{ Reading::Wide(x) }}\n\
         function plain using v(n: Integer): Reading pure {{ Reading::Raw(n) }}\n\
         function literal using v(): Reading pure {{ Reading::Raw(7) }}\n"
    ));
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    verified_exports(&emission);

    let built = constructions(&wire);
    let forms_of = |member: &str| {
        let mut forms: Vec<&str> = built
            .iter()
            .filter(|(name, _)| name == member)
            .map(|(_, payload)| payload["semantic_form"].as_str().expect("a form"))
            .collect();
        forms.sort_unstable();
        forms
    };
    assert_eq!(forms_of("Raw"), ["conversion", "literal", "parameter"]);
    assert_eq!(forms_of("Wide"), ["conversion"]);

    // Every payload term has the member's declared payload type node.
    for (member, payload) in &built {
        assert_eq!(
            payload["semantic_type"],
            *declared_payload(&wire, member),
            "{member}: {payload}"
        );
    }

    // The conversion of `x` to `Integer` is the node `g(x)` gives its
    // argument.
    let converted = |member: &str| {
        built
            .iter()
            .find(|(name, payload)| name == member && payload["semantic_form"] == "conversion")
            .map(|(_, payload)| *payload)
            .expect("a converted payload")
    };
    let raw = converted("Raw");
    assert_eq!(
        raw["body"]["operation"]["identity"],
        "filament.op.numeric.convert"
    );
    let call = of_form(&wire, "call")
        .into_iter()
        .find(|node| node["body"]["arguments"].as_array().map(Vec::len) == Some(2))
        .expect("the call g(x)");
    assert_eq!(call["body"]["arguments"][1]["target"], raw["node_id"]);
    assert_eq!(
        converted("Wide")["body"]["operation"]["identity"],
        "filament.op.numeric.convert"
    );
}

/// FR-317-AC-7 (TC-3134): the checked package edited by hand so that the
/// `Raw(x)` payload term is `x` itself, typed `Int[0, 1000]` at the `Integer`
/// member, with the conversion node removed and every identity and
/// `dependencies` list re-derived, is admitted with the construction left
/// out and refused `ill_typed`/`type-mismatch` at the `union_value` node with
/// it.
#[trace("TC-3134", "FR-317-AC-7")]
#[test]
fn a_payload_argument_left_unconverted_is_refused_by_the_strict_reader() {
    let emission = emitted(&format!(
        "{READING}function raw using v(x: Int[0, 1000]): Reading pure {{ Reading::Raw(x) }}\n\
         function keep using v(): Boolean pure {{ true }}\n"
    ));
    let wire = wire(&emission);
    assert_eq!(
        strict_verdict(&wire),
        None,
        "the emitted package is admitted"
    );
    assert_eq!(
        strict_verdict(&edited(&wire, &[], None)),
        None,
        "re-deriving every identity changes nothing"
    );

    let position = |form: &str| {
        nodes(&wire)
            .iter()
            .position(|node| node["semantic_form"] == form)
            .unwrap_or_else(|| panic!("a {form} node"))
    };
    let [conversion, value, parameter] = ["conversion", "union_value", "parameter"].map(position);
    let function = nodes(&wire)
        .iter()
        .position(|node| node["declaration"] == json!({"qualified_name": ["raw"]}))
        .expect("the raw function");

    // With the construction left out: the function, its construction, the
    // conversion and the parameter are removed.
    let without = edited(&wire, &[function, value, conversion, parameter], None);
    assert_eq!(
        strict_verdict(&without),
        None,
        "strict wire, graph and identity admission pass without the construction"
    );

    // With it, the payload term being `x`.
    let unconverted = edited(&wire, &[conversion], Some((conversion, parameter)));
    let verdict = strict_verdict(&unconverted).expect("the reader refuses the unconverted payload");
    let at = unconverted["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "union_value")
        .expect("the construction remains");
    assert_eq!(verdict.0, "IllTyped", "{verdict:?}");
    assert_eq!(verdict.1.as_deref(), Some("TypeMismatch"), "{verdict:?}");
    assert!(
        verdict
            .2
            .starts_with(&format!("/semantic_graph/nodes/{at}")),
        "{verdict:?}"
    );
}
