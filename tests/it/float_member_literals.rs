// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322-AC-288: a float model member compared with a float constant
//! reaches the checked package as two operands of one type node, so Contract
//! IR's strict read admits the package. A constant compared with anything else
//! keeps its own float type node.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::{Value, json};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"test/float-export\";\n";

const NAN: &str = "float64(bits: 0x7ff8000000000001)";
const ONE: &str = "float64(bits: 0x3ff0000000000000)";

/// The `test/float-export` model with a `Float32` field `s` beside the
/// `Float64` field `a`.
fn model() -> Vec<u8> {
    let mut model = crate::export_rulings::float_model();
    let fields = model["types"][0]["fields"].as_array_mut().unwrap();
    let mut single = fields[0].clone();
    let identity = "ix://test/float-export/Gauge/s";
    single["identity"] = json!(identity);
    single["name"] = json!("s");
    single["typeRef"] = json!("ix://filament/native/Float32");
    single["origin"]["generated"]["generatorIdentity"] = json!(identity);
    single["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(single);
    serde_json::to_vec(&model).unwrap()
}

fn invariant(expression: &str) -> String {
    format!("invariant Probe using v on Config::Gauge at current {{ {expression} }}\n")
}

fn post(expression: &str) -> String {
    format!("post Probe using v on Config::Gauge::flip {{ {expression} }}\n")
}

/// The package of `items`, once Contract IR's strict read admits it.
fn admitted(items: &str) -> Value {
    let document = model();
    let documents = [document.as_slice()];
    let bytes = crate::emit_checked_v2::producer_bytes(
        Path::new("float-member.native"),
        &format!("{HEADER}{items}"),
        &documents,
    );
    crate::emit_checked_v2::read_back(&bytes, &documents);
    serde_json::from_slice(&bytes).expect("the package is JSON")
}

fn nodes(wire: &Value) -> &[Value] {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes are emitted")
}

/// The semantic form a type node's identity names, its second path segment.
fn form(identity: &Value) -> &str {
    identity["path"][1]
        .as_str()
        .unwrap_or_else(|| panic!("a type node identity names its form: {identity}"))
}

/// The forms of each equality decision's two operand source types, sorted.
fn operand_forms(wire: &Value) -> Vec<[&str; 2]> {
    let mut forms: Vec<[&str; 2]> = wire["checked_term_data"]["equality_decisions"]
        .as_array()
        .expect("equality decisions are emitted")
        .iter()
        .map(|decision| ["left", "right"].map(|side| form(&decision[side]["source_type"])))
        .collect();
    forms.sort_unstable();
    forms
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_member_equal_to_a_constant_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("self.a = {NAN}")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_constant_equal_to_a_member_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("{NAN} = self.a")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_member_unequal_to_a_constant_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("self.a != {NAN}")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_constant_unequal_to_a_member_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("{NAN} != self.a")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_single_precision_member_compares_at_its_scalar_type() {
    let wire = admitted(&invariant("self.s = float32(bits: 0x3f800000)"));
    assert_eq!(operand_forms(&wire), [["float32", "float32"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_negated_member_equal_to_a_constant_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("-self.a = {NAN}")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_constant_equal_to_a_negated_member_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("{NAN} = -self.a")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_prior_member_equal_to_a_constant_compares_at_the_scalar_type() {
    let wire = admitted(&post(&format!("pre(self.a) = {NAN}")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_constant_equal_to_a_prior_member_compares_at_the_scalar_type() {
    let wire = admitted(&post(&format!("{NAN} = pre(self.a)")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn a_negated_prior_member_equal_to_a_constant_compares_at_the_scalar_type() {
    let wire = admitted(&post(&format!("-pre(self.a) = {NAN}")));
    assert_eq!(operand_forms(&wire), [["float64", "float64"]]);
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn each_member_comparison_of_one_clause_compares_at_the_scalar_type() {
    let wire = admitted(&invariant(&format!("self.a = {NAN} and self.a != {ONE}")));
    assert_eq!(
        operand_forms(&wire),
        [["float64", "float64"], ["float64", "float64"]]
    );
}

#[trace("FSpec-TC-1926", "FSpec-FR-322-AC-288")]
#[test]
fn one_constant_compared_with_a_member_and_a_parameter_takes_each_operand_type() {
    let wire = admitted(&format!(
        "{}function same using v(p: Float64[exact]): Boolean pure {{ p = {NAN} }}\n",
        invariant(&format!("self.a = {NAN}")),
    ));
    assert_eq!(
        operand_forms(&wire),
        [["float64", "float64"], ["float_rounding", "float_rounding"]]
    );
}

/// The intrinsic records no equality decision, so its constant argument stays
/// at the constant's own float type node.
#[trace("TC-1091", "FR-397-AC-10")]
#[test]
fn a_constant_argument_of_numeric_equal_keeps_its_rounding_type() {
    let wire = admitted(&invariant(&format!(
        "filament::value::ieee::numericEqual(self.a, {NAN})"
    )));
    let literals: Vec<&str> = nodes(&wire)
        .iter()
        .filter(|node| node["body"]["value_kind"] == "float64_bits")
        .map(|node| form(&node["semantic_type"]))
        .collect();
    assert_eq!(literals, ["float_rounding"]);
}
