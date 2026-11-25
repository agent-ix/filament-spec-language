// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public emitter coverage for empty, case-arm, and related protocol location
//! rows (FR-383-AC-37/38/60/65/66).
#![cfg(feature = "std")]

use std::collections::BTreeMap;

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{emit_checked, CheckedPackage, EmitLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input, PackageInput};
use ix_trace_rs::trace;
use serde_json::{json, Value};

#[path = "../../fsl-semantics/tests/support/tc1070.rs"]
#[allow(dead_code)]
mod tc1070;

fn checked(source: &str, package: Option<&[u8]>) -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:protocol-rows"),
        "program.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert_eq!(parsed.diagnostics(), []);
    let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
    let packages: PackageInput = match package {
        Some(bytes) => package_input([bytes]).expect("the model package admits"),
        None => package_input(std::iter::empty::<&[u8]>()).expect("empty models admit"),
    };
    let admitted = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model selections admit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/protocol-rows".to_owned(),
        unit,
        admitted,
        Vec::new(),
    )
    .expect("the package assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusal| panic!("the package checks: {refusal:?}"));
    CheckedPackage::link(graph)
}

fn wire(package: &CheckedPackage) -> Value {
    let emission = emit_checked(package, EmitLimits::new(64 * 1024 * 1024))
        .expect("the checked package emits");
    assert!(emission.omitted().is_empty(), "{:?}", emission.omitted());
    let mut wire: Value =
        serde_json::from_slice(emission.package().bytes()).expect("the emitted JSON parses");
    fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
        .expect("every operand slot names its emitted node");
    wire
}

#[trace("TC-1063", "FR-383-AC-37")]
#[test]
fn unit_without_protocol_slots_emits_empty_protocol_locations() {
    let package = checked(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function equal using v(a: Integer, b: Integer): Boolean pure { a = b }\n",
        None,
    );
    let emitted = wire(&package);
    assert_eq!(
        emitted["checked_term_data"]["protocol_locations"],
        json!([])
    );
}

#[trace("TC-1063", "FR-383-AC-38")]
#[test]
fn package_without_checked_terms_omits_checked_term_data() {
    let package = checked(
        "language \"ix:native\" edition \"1-draft\";\nrecord Empty { x: Integer; }\n",
        None,
    );
    let emitted = wire(&package);
    assert!(emitted.get("checked_term_data").is_none(), "{emitted}");
}

fn choice_domain() -> Vec<u8> {
    let mut document = tc1070::document();
    let mut other = document["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ty| ty["displayName"] == "Node")
        .expect("Node")
        .clone();
    other["identity"] = json!("ix://test/orders/Other");
    other["displayName"] = json!("Other");
    let mut other_total = other["fields"][0].clone();
    other_total["identity"] = json!("ix://test/orders/Other/total");
    other_total["origin"]["generated"]["inputIdentities"] = json!(["ix://test/orders/Other/total"]);
    other["identityFields"] = json!(["ix://test/orders/Other/total"]);
    other["fields"] = json!([other_total]);
    other["operations"] = json!([]);
    other["relationships"] = json!([]);
    document["types"].as_array_mut().unwrap().push(other);
    let mut choice = variant_fields::variant_type("Choice", &["Left", "Right"]);
    choice["variants"][0]["payloadType"] = json!("ix://filament/native/Integer");
    choice["variants"][1]["payloadType"] = json!("ix://filament/native/Boolean");
    document["types"].as_array_mut().unwrap().push(choice);
    document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["displayName"] == "Node")
        .expect("Node")["operations"][0]["returns"]["typeRef"] = json!("ix://test/orders/Choice");
    document["constructs"].as_array_mut().unwrap().push(json!({
        "kind":{"module":"test/business","name":"tier"},
        "moduleVersion":"1.0.0",
        "construct":{"identity":"none","shape":"enumeration",
            "members":{"variants":"required"},
            "meaning":fsl_semantics::model::intake::meaning::VARIANT_TYPE}
    }));
    document.to_string().into_bytes()
}

fn owner_model_source(over: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.protocol.finite-global/v1\";\n\
         model M = \"test/orders\";\n\
         protocol Flow using P over (input: {over}) on origin {{\n\
           role Service on M::Node;\n\
           role Alternate on M::Other;\n\
           run sequence Main {{ }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

fn owner_model_clause(wire: &Value) -> &Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes")
        .iter()
        .find(|node| node["semantic_form"] == "protocol_clause")
        .expect("the emitted protocol clause")
}

fn owner_model_node<'a>(wire: &'a Value, name: &str) -> &'a Value {
    let identity = format!("ix://test/orders/{name}");
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes")
        .iter()
        .find(|node| node["owner"]["node"] == identity)
        .unwrap_or_else(|| {
            panic!(
                "the emitted model node for {identity}: {}",
                wire["semantic_graph"]
            )
        })
}

fn refusal_path(refusal: &filament_contract_model::CheckedPackageRefusal) -> Option<&str> {
    refusal
        .loci
        .first()?
        .path
        .as_ref()
        .map(|path| path.as_str())
}

/// Asserts the clause's path is `Flow` followed by one operand segment
/// `{"operand": k}` whose operand-row entry is the graph index of `target`.
fn assert_owner_operand_names(wire: &Value, target: &str) {
    let nodes = wire["semantic_graph"]["nodes"].as_array().expect("nodes");
    let clause = nodes
        .iter()
        .position(|node| node["semantic_form"] == "protocol_clause")
        .expect("protocol clause position");
    let path = &nodes[clause]["node_id"]["path"];
    let slot = path[1]["operand"].as_u64().expect("an operand slot");
    assert_eq!(path, &json!(["Flow", {"operand": slot}]));
    let entry = &wire["semantic_graph"]["operand_rows"][clause][slot as usize];
    let index = entry.as_u64().expect("a graph index row entry") as usize;
    assert_eq!(&nodes[index], owner_model_node(wire, target));
}

fn protocol_owner_wire(over: &str, domain: &[u8]) -> Value {
    let package = checked(&owner_model_source(over), Some(domain));
    let emission = emit_checked(&package, EmitLimits::new(64 * 1024 * 1024))
        .expect("the owner-model package emits");
    serde_json::from_slice(emission.package().bytes()).expect("the emitted JSON parses")
}

fn protocol_owner_read(
    wire: &Value,
    domain: &[u8],
) -> filament_contract_model::CheckedPackageDispatchResult {
    let mut evidence = filament_contract_model::CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("required features")
    {
        evidence.support_feature(feature.as_str().expect("feature identity"));
    }
    evidence
        .insert_domain_package_document("test/orders", domain.to_vec())
        .expect("fixture model evidence is unique");
    let bytes = serde_json::to_vec(wire).expect("the mutated package serializes");
    filament_contract_model::read_checked_package(
        &bytes,
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    )
}

fn protocol_owner_refusal(
    wire: &Value,
    domain: &[u8],
) -> filament_contract_model::CheckedPackageRefusal {
    match protocol_owner_read(wire, domain) {
        filament_contract_model::CheckedPackageDispatchResult::Refused(refusal) => refusal,
        other => panic!("the mutated package is refused: {other:?}"),
    }
}

#[trace("TC-1063", "FR-383-AC-27")]
#[test]
fn protocol_owner_edges_follow_object_types_reached_through_subjects() {
    let domain = choice_domain();
    for over in ["Boolean", "Integer"] {
        let emitted = protocol_owner_wire(over, &domain);
        assert!(
            owner_model_clause(&emitted).get("owner_model").is_none(),
            "non-object subject {over} has no owner edge"
        );
        assert_eq!(
            owner_model_clause(&emitted)["node_id"]["path"],
            json!(["Flow"])
        );
    }
    for over in ["Option<M::Node>", "Sequence<M::Node>", "M::Node"] {
        let emitted = protocol_owner_wire(over, &domain);
        let owner = &owner_model_node(&emitted, "Node")["node_id"];
        let clause = owner_model_clause(&emitted);
        assert_eq!(
            &clause["owner_model"], owner,
            "the owner edge follows {over} to Node"
        );
        assert!(clause["dependencies"].as_array().unwrap().contains(owner));
        assert_owner_operand_names(&emitted, "Node");
    }

    let mut emitted = protocol_owner_wire("M::Node", &domain);
    let clause_position = emitted["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "protocol_clause")
        .expect("protocol clause position");
    let clause_path = format!("/semantic_graph/nodes/{clause_position}");
    let role_position = emitted["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "role")
        .expect("role position");
    let owner = emitted["semantic_graph"]["nodes"][clause_position]["owner_model"].clone();
    let other_model = owner_model_node(&emitted, "Other")["node_id"].clone();
    let boolean = emitted["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["semantic_form"] == "boolean")
        .expect("Boolean type node")["node_id"]
        .clone();

    let mut missing = emitted.clone();
    missing["semantic_graph"]["nodes"][clause_position]
        .as_object_mut()
        .unwrap()
        .remove("owner_model");
    let refusal = protocol_owner_refusal(&missing, &domain);
    assert_eq!(format!("{:?}", refusal.code), "MalformedWire");
    assert_eq!(refusal_path(&refusal), Some(clause_path.as_str()));

    let mut misplaced = emitted.clone();
    misplaced["semantic_graph"]["nodes"][role_position]["owner_model"] = owner.clone();
    let refusal = protocol_owner_refusal(&misplaced, &domain);
    assert_eq!(format!("{:?}", refusal.code), "MalformedWire");
    assert_eq!(
        refusal_path(&refusal),
        Some(format!("/semantic_graph/nodes/{role_position}/owner_model").as_str())
    );

    let mut external = emitted.clone();
    external["semantic_graph"]["nodes"][clause_position]["owner_model"] =
        json!({"package":"other", "node":owner});
    let refusal = protocol_owner_refusal(&external, &domain);
    assert_eq!(format!("{:?}", refusal.code), "MalformedWire");
    assert_eq!(
        refusal_path(&refusal),
        Some(format!("{clause_path}/owner_model").as_str())
    );

    let mut unlisted = emitted.clone();
    unlisted["semantic_graph"]["nodes"][clause_position]["dependencies"]
        .as_array_mut()
        .unwrap()
        .retain(|dependency| dependency != &owner);
    let refusal = protocol_owner_refusal(&unlisted, &domain);
    assert_eq!(
        format!("{:?}", refusal.code),
        "InvalidSemanticGraph",
        "{refusal:?}; owner={owner}; deps={}",
        emitted["semantic_graph"]["nodes"][clause_position]["dependencies"]
    );
    assert_eq!(
        refusal_path(&refusal),
        Some(format!("{clause_path}/dependencies").as_str())
    );

    let mut wrong_kind = emitted.clone();
    wrong_kind["semantic_graph"]["nodes"][clause_position]["owner_model"] = boolean;
    let wrong_owner = wrong_kind["semantic_graph"]["nodes"][clause_position]["owner_model"].clone();
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wrong_kind)
        .expect("the mutated wire keeps its node identities");
    let mut dependencies = wrong_kind["semantic_graph"]["nodes"][clause_position]["dependencies"]
        .as_array_mut()
        .expect("clause dependencies")
        .iter()
        .map(|dependency| {
            if dependency == &owner {
                wrong_owner.clone()
            } else {
                dependency.clone()
            }
        })
        .collect::<Vec<_>>();
    dependencies.sort_by_key(|dependency| graph.key_of_value(dependency).unwrap());
    wrong_kind["semantic_graph"]["nodes"][clause_position]["dependencies"] =
        Value::Array(dependencies);
    let refusal = protocol_owner_refusal(&wrong_kind, &domain);
    assert_eq!(format!("{:?}", refusal.code), "InvalidModelBinding");
    assert_eq!(format!("{:?}", refusal.cause), "Some(MalformedDeclaration)");
    assert_eq!(
        refusal_path(&refusal),
        Some(format!("{clause_path}/owner_model").as_str())
    );

    emitted["semantic_graph"]["nodes"][clause_position]["owner_model"] = other_model.clone();
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&emitted)
        .expect("the retargeted wire keeps its node identities");
    let mut dependencies = emitted["semantic_graph"]["nodes"][clause_position]["dependencies"]
        .as_array()
        .expect("clause dependencies")
        .clone();
    dependencies.retain(|dependency| dependency != &owner);
    dependencies.push(other_model);
    dependencies.sort_by_key(|dependency| graph.key_of_value(dependency).unwrap());
    dependencies.dedup();
    emitted["semantic_graph"]["nodes"][clause_position]["dependencies"] =
        Value::Array(dependencies);
    let refusal = protocol_owner_refusal(&emitted, &domain);
    assert_eq!(
        format!("{:?}", refusal.code),
        "InvalidPackage",
        "{refusal:?}"
    );
    assert_eq!(format!("{:?}", refusal.cause), "Some(StaleNodeKey)");
    assert_eq!(
        refusal_path(&refusal),
        Some(format!("{clause_path}/owner_model").as_str())
    );
    let locus = refusal
        .locus
        .expect("the stale owner refusal has a node locus");
    assert_eq!(
        graph
            .key_of_value(&serde_json::to_value(&locus).unwrap())
            .expect("the locus names a graph node"),
        graph
            .key_of_value(&emitted["semantic_graph"]["nodes"][clause_position]["node_id"])
            .unwrap(),
    );
    assert_owner_operand_names(&emitted, "Node");

    // The edge, dependency and final path operand all name the new target.
    let retargeted = protocol_owner_wire("M::Other", &domain);
    let other = &owner_model_node(&retargeted, "Other")["node_id"];
    assert_eq!(&owner_model_clause(&retargeted)["owner_model"], other);
    assert_owner_operand_names(&retargeted, "Other");
    let read = protocol_owner_read(&retargeted, &domain);
    assert!(
        matches!(
            read,
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_)
        ),
        "{read:?}"
    );
}

#[path = "../../fsl-semantics/tests/support/variant_fields.rs"]
#[allow(dead_code, unused_imports)]
mod variant_fields;

#[trace("TC-1063", "FR-383-AC-60")]
#[test]
fn each_choice_arm_emits_one_literal_application_row() {
    let source = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Composed using P over (view: M::Node) on origin {
  role Service on M::Node;
  run sequence Main {
    attempt Cases by Service on M::Node::step contracts []
        as (selected: M::Choice) {
      case selected { Left(x): x >= 1; Right(b): not b; }
    };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;
    let domain = choice_domain();
    let package = checked(source, Some(&domain));
    let emitted = wire(&package);
    let rows = emitted["checked_term_data"]["protocol_locations"]
        .as_array()
        .expect("protocol locations are present");
    let case_rows: Vec<_> = rows
        .iter()
        .filter(|row| row["location"]["origin"]["name"] == "protocol Composed run Main Cases body")
        .collect();
    assert_eq!(case_rows.len(), 2, "one row per arm: {case_rows:?}");
    let mut paths = case_rows
        .iter()
        .map(|row| {
            row["location"]["path"]
                .as_array()
                .expect("the row has a child path")
                .iter()
                .map(|step| step.as_u64().unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(
        paths,
        [vec![1], vec![2]],
        "the Left and Right arm positions"
    );
    for row in case_rows {
        let node = emitted["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["node_id"] == row["application"])
            .unwrap_or_else(|| {
                panic!(
                    "row application {} names a node in {}",
                    row["application"], emitted["semantic_graph"]["nodes"]
                )
            });
        let expected = if row["location"]["path"] == json!([1]) {
            "filament.op.integer.ge"
        } else {
            "filament.op.boolean.not"
        };
        assert_eq!(node["body"]["operation"]["identity"], expected, "{node}");
    }
}

#[trace("TC-1063", "FR-383-AC-66")]
#[test]
fn repeating_the_second_related_clause_preserves_public_rows() {
    let call_both = "related by Owns (identityRef(view), identityRef(view))";
    let call_from = "related by Owns (identityRef(view), view)";
    let source = |clauses: &str| {
        let padding = if clauses.matches("related by Owns").count() == 2 {
            " ".repeat(call_both.len() + 1)
        } else {
            String::new()
        };
        tc1070::SOURCE
            .replace(
                "profile S = \"filament.state.graph/v1\";",
                "profile S = \"filament.state.graph/v1\";\nprofile v = \"filament.value.complete/v1\";",
            )
            .replace(
                "model M = \"test/orders\";",
                "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }",
            )
            .replace(
                "related by Owns (view, view) { true }",
                &format!("{clauses}{padding} {{ true }}"),
            )
    };
    let domain = tc1070::document().to_string().into_bytes();
    let compile = |source: &str| {
        let package = checked(source, Some(&domain));
        let emission =
            emit_checked(&package, EmitLimits::new(64 * 1024 * 1024)).expect("the fixture emits");
        assert!(emission.omitted().is_empty(), "{:?}", emission.omitted());
        let mut wire: Value = serde_json::from_slice(emission.package().bytes()).unwrap();
        fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
            .expect("every operand slot names its emitted node");
        (package, wire, emission.source_map().clone())
    };
    let (base_package, base, base_map) = compile(&source(&format!("{call_from} {call_both}")));
    let (repeat_package, repeated, repeat_map) =
        compile(&source(&format!("{call_from} {call_both} {call_both}")));

    let node_index = |wire: &Value| {
        wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("nodes are present")
            .iter()
            .map(|node| (node["node_id"].to_string(), node.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(node_index(&base), node_index(&repeated));
    assert_eq!(
        base_package.graph().protocols().len(),
        repeat_package.graph().protocols().len()
    );
    assert_eq!(base_map, repeat_map);
    assert_eq!(
        base["checked_term_data"]["protocol_locations"],
        repeated["checked_term_data"]["protocol_locations"]
    );
}

fn provisional_related_origin(name: &str) -> bool {
    let Some((_, suffix)) = name.rsplit_once(" related ") else {
        return false;
    };
    let positions = suffix.split_whitespace().collect::<Vec<_>>();
    positions.len() == 2
        && positions
            .iter()
            .all(|position| position.parse::<usize>().is_ok())
}

#[trace("TC-1063", "FR-383-AC-65")]
#[test]
fn related_ends_keep_only_final_record_positions_in_checked_and_wire_locations() {
    let call_both = "related by Owns (identityRef(view), identityRef(view))";
    let call_from = "related by Owns (identityRef(view), view)";
    let base = tc1070::SOURCE
        .replace(
            "profile S = \"filament.state.graph/v1\";",
            "profile S = \"filament.state.graph/v1\";\nprofile v = \"filament.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\nfunction identityRef using v(x: M::Node): M::Node pure { x }",
        );
    let domain = tc1070::document().to_string().into_bytes();

    for (first, second) in [(call_from, call_both), (call_both, call_from)] {
        let source = base.replace(
            "related by Owns (view, view) { true }",
            &format!("{first} {second} {{ true }}"),
        );
        let package = checked(&source, Some(&domain));
        let provisional =
            package
                .graph()
                .occurrences()
                .filter_map(|(_, _, location)| match &location.origin {
                    filament_semantic_value::location::ExpressionOwner::TypeDeclaration {
                        name,
                    } if provisional_related_origin(name) => Some(name.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
        assert!(provisional.is_empty(), "checked locations: {provisional:?}");

        let emitted = wire(&package);
        let rows = emitted["checked_term_data"]["protocol_locations"]
            .as_array()
            .expect("protocol location rows are emitted");
        let provisional_rows = rows
            .iter()
            .filter_map(|row| {
                let name = row["location"]["origin"]["name"].as_str()?;
                provisional_related_origin(name).then_some(name)
            })
            .collect::<Vec<_>>();
        assert!(
            provisional_rows.is_empty(),
            "wire rows: {provisional_rows:?}"
        );
    }
}
