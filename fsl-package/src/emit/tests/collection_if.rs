// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-64, FR-093-AC-65 (TC-1422, TC-1423): an `if` is an ordinary
//! operand of the collection forms and a `let` can bind an option `value`
//! under its presence guard; each lowers, emits and reads back `Verified`.

use super::owners::node_where;
use super::variant_types::fixture;
use super::*;
use filament_semantic_value::checking::CheckMode;

/// The package of `unit_body`, emitted over the FR-360 model `M` and read back.
fn emitted(unit_body: &str) -> Value {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {package:?};\n\
         {unit_body}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    );
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    wire
}

/// The nodes applying `operation`.
fn applying<'w>(wire: &'w Value, operation: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == operation)
        .collect()
}

/// The node `reference` (a `reference` term) names.
fn target<'w>(wire: &'w Value, reference: &Value) -> &'w Value {
    assert_eq!(reference["term"], "reference", "{reference}");
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == reference["target"])
        .unwrap_or_else(|| panic!("no node {reference}"))
}

/// The single `forall`/`exists` node and the node of its binding body and of
/// its collection operand.
fn quantifier<'w>(wire: &'w Value, operation: &str) -> (&'w Value, &'w Value) {
    let found = applying(wire, operation);
    assert_eq!(found.len(), 1, "one {operation} node");
    let quantify = found[0];
    let arguments = &quantify["body"]["arguments"];
    assert_eq!(arguments[1]["term"], "binding");
    (
        target(wire, &arguments[0]),
        target(wire, &arguments[1]["value"]),
    )
}

/// An `if` node: `filament.op.control.if` over a condition and two branches.
fn assert_if(node: &Value) {
    assert_eq!(
        node["body"]["operation"]["identity"],
        "filament.op.control.if"
    );
    assert_eq!(node["body"]["arguments"].as_array().unwrap().len(), 3);
}

/// FR-093-AC-64: the element body of a `forall` or `exists` is an `if`, both
/// over a parameter sequence and over a model collection field, and the
/// collection operand of a `forall` is itself an `if`.
#[trace("FR-093-AC-64", "TC-1422")]
#[test]
fn an_if_is_the_element_body_or_the_collection_operand_of_a_quantifier() {
    let wire = emitted(
        "function elements using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure \
         { forall(x in s: (if x < 5 then x else 5) <= 5) }\n",
    );
    let (_, body) = quantifier(&wire, "filament.op.collection.forall");
    assert_eq!(
        body["body"]["operation"]["identity"],
        "filament.op.integer.le"
    );
    assert_if(target(&wire, &body["body"]["arguments"][0]));

    let wire = emitted(
        "function elements using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure \
         { exists(x in s: if x < 5 then true else false) }\n",
    );
    let (_, body) = quantifier(&wire, "filament.op.collection.exists");
    assert_if(body);

    let wire = emitted(
        "invariant Gold using v on M::Account at current \
         { forall(h in self.history: (if h = M::Tier::Gold then true else false)) }\n",
    );
    let (_, body) = quantifier(&wire, "filament.op.collection.forall");
    assert_if(body);
    assert_eq!(node_where(&wire, "model", "variant_type").len(), 1);

    let wire = emitted(
        "function operand using v(c: Boolean, s: Sequence<Int[0, 9]>[0, 5], \
         t: Sequence<Int[0, 9]>[0, 5]): Boolean pure \
         { forall(x in (if c then s else t): x < 5) }\n",
    );
    let (operand, _) = quantifier(&wire, "filament.op.collection.forall");
    assert_if(operand);
}

/// FR-093-AC-65: `if present(o) then (let q = value(o) in q) else 0` over an
/// `Option` parameter lowers the guard to `option.present`, the guarded read to
/// `option.value` bound by a `control.let`, and the whole to a `control.if`.
#[trace("FR-093-AC-65", "TC-1423")]
#[test]
fn a_let_binds_an_option_value_under_its_presence_guard() {
    let wire = emitted(
        "function guarded using v(o: Option<Int[0, 9]>): Integer pure \
         { if present(o) then (let q = value(o) in q) else 0 }\n",
    );
    let found = applying(&wire, "filament.op.control.if");
    assert_eq!(found.len(), 1, "one if node");
    let conditional = found[0];
    let arguments = &conditional["body"]["arguments"];
    assert_eq!(
        target(&wire, &arguments[0])["body"]["operation"]["identity"],
        "filament.op.option.present"
    );
    let binding = target(&wire, &arguments[1]);
    assert_eq!(
        binding["body"]["operation"]["identity"],
        "filament.op.control.let"
    );
    let bound = &binding["body"]["arguments"][0];
    let value = if bound["term"] == "binding" {
        &bound["value"]
    } else {
        bound
    };
    assert_eq!(
        target(&wire, value)["body"]["operation"]["identity"],
        "filament.op.option.value"
    );
}
