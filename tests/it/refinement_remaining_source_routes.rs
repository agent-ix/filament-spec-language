// SPDX-License-Identifier: AGPL-3.0-or-later
//! Remaining projection, execution-relation and protocol-refinement source cases.

use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/refinement/remaining")
}

fn emitted(stem: &str) -> Value {
    let compiled = command::compile_outcome(&corpus().join(format!("{stem}.request.json")))
        .unwrap_or_else(|error| panic!("{stem} must compile: {error}"));
    serde_json::from_slice(compiled.package_bytes()).expect("the emitted package is JSON")
}

fn nodes(package: &Value) -> &[Value] {
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("the semantic graph has nodes")
}

fn semantic_node<'a>(package: &'a Value, form: &str) -> &'a Value {
    let found: Vec<&Value> = nodes(package)
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect();
    let [node] = found.as_slice() else {
        panic!("one {form} node, got {}", found.len());
    };
    node
}

fn binding<'a>(term: &'a Value, name: &str) -> &'a Value {
    let member = term["members"]
        .as_array()
        .expect("an aggregate term")
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("the aggregate has binding {name}: {term}"));
    &member["value"]
}

fn referent<'a>(package: &'a Value, reference: &Value) -> &'a Value {
    assert_eq!(reference["term"], "reference", "{reference}");
    nodes(package)
        .iter()
        .find(|node| node["node_id"] == reference["target"])
        .unwrap_or_else(|| panic!("the reference resolves: {reference}"))
}

#[test]
#[trace("FR-171", "FR-172", "FR-136")]
fn remaining_source_forms_compile_and_refuse_as_specified() {
    assert_eq!(
        crate::source_corpus::run_directory(&corpus()).expect("the source cases match"),
        "flips_with: none"
    );
}

#[test]
#[trace("FR-171", "FR-488")]
#[ignore = "QSL-2609: emitted internal references must resolve to their semantic nodes"]
fn projection_emits_its_bound_skip_operation_and_match() {
    let package = emitted("projection-operation-anchor");
    let hyper = semantic_node(&package, "hyperproperty");
    let body = &hyper["body"];
    assert_eq!(binding(body, "domain")["value"], "behaviours");
    let quantifiers = binding(body, "quantifiers")["members"]
        .as_array()
        .expect("the quantifier prefix");
    let variables: Vec<&str> = quantifiers
        .iter()
        .map(|quantifier| {
            binding(quantifier, "variable")["value"]
                .as_str()
                .expect("a quantified name")
        })
        .collect();
    assert_eq!(variables, ["left", "right"]);
    let skipped = binding(body, "align_skip")["members"]
        .as_array()
        .expect("the skip list");
    assert_eq!(skipped.len(), 1);
    let operation = referent(&package, &skipped[0]);
    assert_eq!(operation["semantic_form"], "operation_anchor");
    assert_eq!(binding(&operation["body"], "operation")["value"], "inc");
    let matched = binding(body, "match")["members"]
        .as_array()
        .expect("the match conjuncts");
    assert_eq!(matched.len(), 1);
    assert_eq!(referent(&package, &matched[0])["node_tag"], "expression");
}

#[test]
#[trace("FR-136", "FR-901")]
fn protocol_finish_is_a_concrete_step_mapped_to_stutter() {
    let package = emitted("protocol-selected-by-step");
    let refinement = semantic_node(&package, "refinement");
    let body = &refinement["body"];
    assert_eq!(body["name"], "SessionToCounter");
    let classes: Vec<&str> = body["class_catalog"]["concrete"]
        .as_array()
        .expect("concrete step classes")
        .iter()
        .map(|class| class["class"].as_str().expect("a class spelling"))
        .collect();
    assert_eq!(classes, ["Counter::inc", "Session::Complete"]);
    let steps = body["steps"].as_array().expect("refinement step rows");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["target"]["kind"], "operation");
    assert_eq!(steps[1]["source_class"], "Session::Complete");
    assert_eq!(steps[1]["target"]["kind"], "stutter");
    let finish = nodes(&package)
        .iter()
        .find(|node| node["node_id"] == steps[1]["source_node"])
        .expect("the concrete finish node remains in the emitted protocol");
    assert_eq!(finish["semantic_form"], "obligation");
    assert_eq!(finish["body"]["operation"]["member"]["operator"], "finish");
}

#[test]
#[trace("FR-171")]
#[ignore = "QSL-2609: emitted internal references must resolve to their semantic nodes"]
fn execution_relations_keep_distinct_receivers_of_the_same_operation() {
    let package = emitted("relation-pre-and-post");
    let relation = semantic_node(&package, "step_relation");
    let executions = binding(&relation["body"], "executions")["members"]
        .as_array()
        .expect("the two model executions");
    assert_eq!(executions.len(), 2);
    assert_eq!(executions[0]["name"], "runA");
    assert_eq!(executions[1]["name"], "runB");
    let left = &executions[0]["value"];
    let right = &executions[1]["value"];
    assert_eq!(binding(left, "operation"), binding(right, "operation"));
    assert_ne!(binding(left, "parameters"), binding(right, "parameters"));
    let operation = referent(&package, binding(left, "operation"));
    assert_eq!(binding(&operation["body"], "operation")["value"], "inc");
    let body = referent(&package, binding(&relation["body"], "body"));
    assert_eq!(
        body["body"]["operation"]["identity"],
        "filament.op.boolean.implies"
    );
}
