// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public emitter coverage for empty, case-arm, and repeated-related protocol
//! location rows (FR-383-AC-37/38/60/66).
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
