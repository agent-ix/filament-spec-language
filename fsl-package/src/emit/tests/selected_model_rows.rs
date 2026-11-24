// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1398 acceptance evidence. No current FSL spec criterion covers
//! selected model tuple witness row production.

use super::*;
use filament_semantic_value::checking::CheckMode;
use fsl_semantics::model::intake::meaning;

#[path = "../../../../fsl-semantics/tests/support/tc235_k2.rs"]
#[allow(
    dead_code,
    reason = "this test uses only part of the shared K2 fixture"
)]
#[allow(
    clippy::duplicate_mod,
    reason = "a sibling test module also loads the K2 builder"
)]
mod fixture;

fn tuple_document(include_tuple: bool) -> Vec<u8> {
    if !include_tuple {
        return crate::emit::tests::wire_bytes(&fixture::k2())
            .expect("the package without the tuple serializes");
    }
    let tuple = fixture::declaration(
        "Pair",
        "tuple",
        json!({
            "positions": [
                {
                    "identity": fixture::id("Pair/p0"),
                    "typeRef": "ix://filament/native/Boolean",
                    "origin": fixture::origin("Pair/p0"),
                },
                {
                    "identity": fixture::id("Pair/p1"),
                    "typeRef": "ix://filament/native/Boolean",
                    "origin": fixture::origin("Pair/p1"),
                },
            ],
        }),
    );
    let tuple_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tuple"},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none",
            "shape": "tuple",
            "members": {"positions": "required"},
            "meaning": meaning::TUPLE_VALUE_TYPE,
        },
    });
    crate::emit::tests::wire_bytes(&fixture::domain_document(
        vec![tuple],
        vec![tuple_construct],
    ))
    .expect("the tuple model document serializes")
}

fn emit_with_tuple_source(tuple_is_referenced: bool) -> (CheckedPackage, Emission, Vec<u8>) {
    let document = tuple_document(tuple_is_referenced);
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the tuple package is admitted");
    let function = if tuple_is_referenced {
        "function keep using v(x: M::Pair): M::Pair pure { x }"
    } else {
        "function keep using v(): Boolean pure { true }"
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {:?};\n{function}\n",
        fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the package emits");
    (checked, emission, document)
}

fn emit_with_model_tuple_and_union() -> (CheckedPackage, Emission, Vec<u8>) {
    let source_origin = |line: u64| {
        json!({
            "source": {
                "sourceIdentity": fixture::id("model/phase"),
                "path": "model.fspec",
                "startLine": line,
                "startColumn": 1,
            }
        })
    };
    let tuple = fixture::declaration(
        "Slot",
        "tuple",
        json!({
            "positions": [
                {
                    "identity": fixture::id("Slot/p0"),
                    "typeRef": "ix://filament/native/Boolean",
                    "origin": fixture::origin("Slot/p0"),
                },
                {
                    "identity": fixture::id("Slot/p1"),
                    "typeRef": "ix://filament/native/Integer",
                    "origin": fixture::origin("Slot/p1"),
                },
            ],
        }),
    );
    let phase = fixture::declaration(
        "Phase",
        "tier",
        json!({
            "variants": [
                {
                    "identity": fixture::id("variant/Phase-Armed"),
                    "name": "Armed",
                    "origin": source_origin(1),
                    "payloadType": "ix://filament/native/Clock",
                },
                {
                    "identity": fixture::id("variant/Phase-Idle"),
                    "name": "Idle",
                    "origin": source_origin(2),
                },
            ],
        }),
    );
    let tuple_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tuple"},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none",
            "shape": "tuple",
            "members": {"positions": "required"},
            "meaning": meaning::TUPLE_VALUE_TYPE,
        },
    });
    let union_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tier"},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none",
            "shape": "enumeration",
            "members": {"variants": "required"},
            "meaning": meaning::VARIANT_TYPE,
        },
    });
    let document = crate::emit::tests::wire_bytes(&fixture::domain_document(
        vec![tuple, phase],
        vec![tuple_construct, union_construct],
    ))
    .expect("the selected model tuple and union document serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the selected model document is admitted");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {:?};\n\
         function keepSlot using v(x: M::Slot): M::Slot pure {{ x }}\n\
         function keepPhase using v(x: M::Phase): M::Phase pure {{ x }}\n",
        fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the tuple and union unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the tuple and union unit checks: {refusals:?}"));
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the package emits both selected types");
    (checked, emission, document)
}

fn emit_with_bounded_rational_tuple_and_union() -> (CheckedPackage, Emission, Vec<u8>) {
    let ratio = fixture::id("Ratio");
    let constraint = |keyword: &str, value: i64| {
        let identity = format!("{ratio}/constraints/{keyword}");
        json!({
            "identity": identity,
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": ratio,
            "diagnosticCode": format!("rational.{keyword}"),
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
        })
    };
    let ratio_type = fixture::declaration(
        "Ratio",
        "value_type",
        json!({
            "scalar": "rational",
            "constraints": [
                constraint("maxDenominator", 12),
                constraint("minNumerator", -3),
                constraint("minDenominator", 1),
                constraint("maxNumerator", 7),
            ],
        }),
    );
    let tuple = fixture::declaration(
        "P",
        "tuple",
        json!({"positions": [{
            "identity": fixture::id("P/p0"),
            "typeRef": fixture::id("Ratio"),
            "origin": fixture::origin("P/p0"),
        }]}),
    );
    let variant = fixture::declaration(
        "V",
        "tier",
        json!({"variants": [{
            "identity": fixture::id("variant/V-c"),
            "name": "c",
            "origin": {"source": {
                "sourceIdentity": fixture::id("model/phase"),
                "path": "model.fspec",
                "startLine": 1,
                "startColumn": 1,
            }},
            "payloadType": fixture::id("Ratio"),
        }]}),
    );
    let value_type_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "value_type"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "record", "members": {},
            "meaning": meaning::VALUE_TYPE},
    });
    let tuple_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tuple"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "tuple", "members": {"positions": "required"},
            "meaning": meaning::TUPLE_VALUE_TYPE},
    });
    let union_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tier"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "enumeration", "members": {"variants": "required"},
            "meaning": meaning::VARIANT_TYPE},
    });
    let document = crate::emit::tests::wire_bytes(&fixture::domain_document(
        vec![ratio_type, tuple, variant],
        vec![value_type_construct, tuple_construct, union_construct],
    ))
    .expect("the bounded Rational tuple and union document serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the bounded Rational model document is admitted");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {:?};\n\
         function keepRatio using v(x: M::Ratio): M::Ratio pure {{ x }}\n\
         function keepTuple using v(x: M::P): M::P pure {{ x }}\n\
         function keepVariant using v(x: M::V): M::V pure {{ x }}\n",
        fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the bounded Rational tuple and union unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| {
            panic!("the bounded Rational tuple and union check: {refusals:?}")
        });
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the bounded Rational tuple and union emit");
    (checked, emission, document)
}

/// FR-505-AC-10 (TC-1300): the tuple position and variant payload both carry
/// the literal bounded Rational type and reference the emitted rational-range
/// node from the selected package.
#[trace("FR-505-AC-10", "TC-1300")]
#[test]
fn bounded_rational_tuple_position_and_variant_payload_keep_the_range_node() {
    use filament_exact::{Integer, IntegerInterval, RationalDomain, ValueType};

    let (checked, emission, _document) = emit_with_bounded_rational_tuple_and_union();
    let semantic = checked.graph().semantic_graph();
    let tuple_node = semantic
        .nodes()
        .find(|node| {
            node.semantic_form() == "tuple_value_type"
                && matches!(node.owner(), Some(fsl_semantics::check::Owner::Model(owner))
                    if owner.node() == fixture::id("P"))
        })
        .expect("the selected P tuple is a model node");
    let tuple_type = checked
        .graph()
        .scope()
        .types()
        .composite(tuple_node.key())
        .expect("P has an admitted checked tuple type");
    let filament_semantic_value::declaration::CompositeShape::Tuple(positions) = tuple_type.shape()
    else {
        panic!("P is checked as a tuple type")
    };
    assert_eq!(positions.len(), 1);
    assert_eq!(
        positions[0],
        ValueType::Rational(
            RationalDomain::new(
                IntegerInterval::new(Integer::from(-3_i64), Integer::from(7_i64))
                    .expect("ordered numerator bounds"),
                IntegerInterval::new(Integer::from(1_i64), Integer::from(12_i64))
                    .expect("positive denominator bounds"),
            )
            .expect("valid Rational bounds"),
        ),
        "p0 has exactly Rational[-3, 7; 1, 12]"
    );

    let wire = wire(&emission);
    let range_nodes: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "rational_range")
        .collect();
    assert_eq!(
        range_nodes.len(),
        1,
        "one bounded Rational range node is emitted"
    );
    assert!(
        range_nodes[0]["node_id"]
            == json!({"ordinal": 0, "package": "pkg", "path": [
                "bounded_domain", "rational_range", "numerator_min", {"node": 0}, "-3",
                "numerator_max", "7", "denominator_min", "1", "denominator_max", "12", {"node": 1}
            ]}),
        "the emitted rational_range node has the exact AC-1 bounds identity: {}",
        range_nodes[0]["node_id"]
    );

    let union_rows = emission.selected_model_union_rows();
    assert_eq!(union_rows.len(), 1);
    let [case] = union_rows[0].cases.as_slice() else {
        panic!("V has the one case c")
    };
    assert_eq!(case.name.as_ref(), "c");
    let filament_contract_model::SelectedModelUnionPayload::Data {
        node: payload_node,
        original_type,
    } = &case.payload
    else {
        panic!("c carries Ratio as data")
    };
    assert_eq!(
        original_type,
        &ValueType::Rational(
            RationalDomain::new(
                IntegerInterval::new(Integer::from(-3_i64), Integer::from(7_i64))
                    .expect("ordered numerator bounds"),
                IntegerInterval::new(Integer::from(1_i64), Integer::from(12_i64))
                    .expect("positive denominator bounds"),
            )
            .expect("valid Rational bounds"),
        ),
        "c's payload has exactly Rational[-3, 7; 1, 12]"
    );
    assert_eq!(
        serde_json::to_value(payload_node).expect("the payload type node id serializes"),
        range_nodes[0]["node_id"],
        "c's payload names the emitted rational_range node"
    );
}

/// A selected tuple row names the full structural id of the emitted model
/// tuple; a package without that tuple has no row or node.
#[test]
fn selected_tuple_rows_match_emitted_model_nodes_and_skip_absent_nodes() {
    let (_, emission, _) = emit_with_tuple_source(true);
    let emitted_wire = wire(&emission);
    let tuple_nodes: Vec<&Value> = nodes(&emitted_wire)
        .iter()
        .filter(|node| node["node_tag"] == "model" && node["semantic_form"] == "tuple_value_type")
        .collect();
    let [tuple_node] = tuple_nodes.as_slice() else {
        panic!("one selected model tuple node is emitted")
    };
    let [row] = emission.selected_tuple_declaration_rows() else {
        panic!("one tuple witness accompanies the emitted tuple")
    };
    assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
    assert_eq!(row.original_identity.as_ref(), fixture::id("Pair").as_str());
    assert_eq!(
        row.node_key,
        fsl_semantics::model::key::model_node_key(
            fixture::PACKAGE,
            "model",
            "tuple_value_type",
            &fixture::id("Pair"),
        )
        .expect("the selected model tuple key is valid")
    );
    assert_eq!(
        serde_json::to_value(&row.structural_node).expect("the row id serializes"),
        tuple_node["node_id"]
    );

    let (_, absent, _) = emit_with_tuple_source(false);
    assert!(
        absent.selected_tuple_declaration_rows().is_empty(),
        "a row is omitted when its selected tuple node is not emitted"
    );
    assert!(!nodes(&wire(&absent)).iter().any(|node| {
        node["node_tag"] == "model" && node["semantic_form"] == "tuple_value_type"
    }));
}

/// FR-352/FR-354: FSL's model-intake rows join the admitted runtime type view;
/// IR refuses the same tuple when its producer witness is omitted.
#[test]
fn selected_tuple_row_projects_the_admitted_type_and_missing_row_refuses() {
    use crate::checked_v2::{read_checked_package_v2, V2ReadLimits};
    use filament_contract_model::ExactTypeProjectionErrorCode;

    let (checked, emission, document) = emit_with_tuple_source(true);
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(&document), document.clone())
        .expect("the selected model document is evidence for admission");
    let read = read_checked_package_v2(
        emission.package().bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    )
    .expect("IR admits the producer rows' package");
    let admitted = read.into_value().admitted;
    let identities = checked
        .graph()
        .selected_exact_identities(
            &admitted,
            &mut filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("the selected declaration and field rows join admission");

    let projected = admitted
        .project_selected_type_declarations(
            &identities,
            emission.selected_tuple_declaration_rows(),
            emission.selected_source_unit_rows(),
            &mut filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("the complete selected model type view joins");
    let [tuple] = projected.tuples() else {
        panic!("the selected tuple is projected exactly once");
    };
    assert_eq!(tuple.original_identity(), fixture::id("Pair"));
    assert_eq!(
        tuple.node_key(),
        &emission.selected_tuple_declaration_rows()[0].node_key
    );
    let refusal = admitted
        .project_selected_type_declarations(
            &identities,
            &[],
            emission.selected_source_unit_rows(),
            &mut filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect_err("a missing selected tuple row refuses the complete projection");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}

/// FR-531-AC-51 (TC-1331): identities/7 carries and joins both selected model
/// tuple and union witnesses, and the runtime projection preserves the tuple
/// position types and union case identities.
#[trace("FR-531-AC-51", "TC-1331")]
#[test]
fn model_tuple_and_union_rows_round_trip_and_project_their_exact_identities() {
    use crate::{IdentityFileRefusal, ReadIdentityRows};
    use filament_contract_model::NativeJoinBudget;

    let (_, emission, document) = emit_with_model_tuple_and_union();
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(&document), document.clone())
        .expect("the selected model document is evidence for admission");
    let read_package = crate::checked_v2::read_checked_package_v2(
        emission.package().bytes(),
        library(),
        crate::checked_v2::V2ReadLimits::fixture(),
        &evidence,
    )
    .expect("the emitted package admits");
    let admitted = read_package.into_value().admitted;
    let budget = || NativeJoinBudget::new(10_000_000, 10_000_000);
    let rows = emission
        .identity_rows(&admitted, &mut budget())
        .expect("the writer includes complete selected-model witnesses");
    let bytes = rows.to_bytes(&admitted).expect("identities/7 writes");
    let value: Value = serde_json::from_slice(&bytes).expect("the identity file is JSON");
    assert_eq!(value["format"], crate::IDENTITIES_FORMAT);
    assert_eq!(value["model_tuples"].as_array().unwrap().len(), 1);
    assert_eq!(value["model_unions"].as_array().unwrap().len(), 1);
    assert_eq!(
        rows.to_bytes(&admitted).expect("the same rows write again"),
        bytes,
        "two writes of equal input are byte-identical"
    );

    let read = ReadIdentityRows::read(&bytes, &admitted, &mut budget())
        .expect("tuple and union rows join against the admitted package");
    assert_eq!(
        read.model_tuples(),
        emission.selected_tuple_declaration_rows()
    );
    assert_eq!(read.model_unions(), emission.selected_model_union_rows());
    assert_eq!(
        read.to_bytes(&admitted).expect("the read rows write"),
        bytes
    );
    let identities = read
        .selected(&admitted, &mut budget())
        .expect("the complete identity rows project");
    let projected = admitted
        .project_selected_type_declarations(
            &identities,
            read.model_tuples(),
            emission.selected_source_unit_rows(),
            &mut budget(),
        )
        .expect("the selected tuple and union types project");
    let [slot] = projected.tuples() else {
        panic!("Slot is projected exactly once")
    };
    assert_eq!(slot.original_identity(), "ix://test/orders/Slot");
    assert_eq!(slot.positions().len(), 2);
    assert_eq!(slot.positions()[0], filament_exact::ValueType::Boolean);
    assert_eq!(slot.positions()[1], filament_exact::ValueType::Integer);
    assert_eq!(read.model_unions()[0].cases[0].name.as_ref(), "Armed");
    assert_eq!(read.model_unions()[0].cases[1].name.as_ref(), "Idle");
    assert_eq!(projected.unions(), read.model_unions());

    let mut missing_tuples = value.clone();
    missing_tuples
        .as_object_mut()
        .unwrap()
        .remove("model_tuples");
    assert!(matches!(
        ReadIdentityRows::read(
            &serde_json::to_vec(&missing_tuples).unwrap(),
            &admitted,
            &mut budget()
        ),
        Err(IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::ModelTuples
        })
    ));
    let mut missing_unions = value.clone();
    missing_unions
        .as_object_mut()
        .unwrap()
        .remove("model_unions");
    assert!(matches!(
        ReadIdentityRows::read(
            &serde_json::to_vec(&missing_unions).unwrap(),
            &admitted,
            &mut budget()
        ),
        Err(IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::ModelUnions
        })
    ));

    let mut malformed_tuple = value.clone();
    malformed_tuple["model_tuples"][0]["unknown"] = json!(true);
    assert!(matches!(
        ReadIdentityRows::read(
            &serde_json::to_vec(&malformed_tuple).unwrap(),
            &admitted,
            &mut budget()
        ),
        Err(IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::ModelTuples
        })
    ));
    let mut foreign_tuple = value;
    foreign_tuple["model_tuples"][0]["selected_document"] = json!("foreign/document");
    assert!(matches!(
        ReadIdentityRows::read(
            &serde_json::to_vec(&foreign_tuple).unwrap(),
            &admitted,
            &mut budget()
        ),
        Err(IdentityFileRefusal::Native(_))
    ));
}
