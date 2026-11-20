// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-740-AC-8 (TC-2062): the staged probabilistic examples compile through S4,
//! emit the expected checked nodes, and pass the strict package reader.

#![cfg(feature = "std")]

use filament_contract_model::{
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{emit_checked, CheckedPackage, EmitLimits};
use fsl_semantics::{
    check::PackageDeclarations,
    model::{
        accounting::ModelNormalizationLimits,
        intake::{admit_unit, package_input},
    },
};
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct CompileRequest {
    request: Request,
}

#[derive(Deserialize)]
struct Request {
    models: Vec<ModelRequest>,
    program: ProgramRequest,
}

#[derive(Deserialize)]
struct ModelRequest {
    source: SourceRequest,
}

#[derive(Deserialize)]
struct ProgramRequest {
    source: SourceRequest,
}

#[derive(Deserialize)]
struct SourceRequest {
    file: String,
    authority: String,
    identity: String,
}

struct EmittedExample {
    package_identity: String,
    wire: Value,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("fsl-package has the repository root as parent")
        .to_path_buf()
}

fn compile_example(request_path: &str, package_identity_override: Option<&str>) -> EmittedExample {
    let root = repository_root();
    let request_path = root.join(request_path);
    let request_bytes = fs::read(&request_path).expect("the vendored request reads");
    let request: CompileRequest = serde_json::from_slice(&request_bytes).expect("request JSON");
    let model = request.request.models.first().expect("one selected model");
    let model_bytes = fs::read(request_path.parent().unwrap().join(&model.source.file))
        .expect("the vendored model reads");
    let program = &request.request.program.source;
    let program_bytes = fs::read(request_path.parent().unwrap().join(&program.file))
        .expect("the vendored rule reads");

    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new(&program.authority, &program.identity),
        &program.file,
        &program_bytes,
        fsl_cst::Limits::default(),
    )
    .expect("S1 parses the rule");
    assert!(
        parsed.is_admissible(),
        "S1 diagnostics: {:?}",
        parsed.diagnostics()
    );
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = package_input([model_bytes.as_slice()]).expect("the selected model admits");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the request's model selection resolves");
    let package_identity = package_identity_override.unwrap_or(&program.identity);
    let checked = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        package_identity.to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("S3 assembles the checked package")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &fsl_semantics::check::test_support::ZeroRewardEvaluator,
    )
    .unwrap_or_else(|refusals| panic!("S3 checks the unit: {refusals:?}"));
    let package = CheckedPackage::link(checked);
    let emission =
        emit_checked(&package, EmitLimits::new(10_000_000)).expect("S4 emits the package");
    let wire: Value = serde_json::from_slice(emission.package().bytes()).expect("wire JSON");
    let mut evidence = emission.evidence().clone();
    let model_identity = serde_json::from_slice::<Value>(&model_bytes).expect("model JSON")
        ["package"]["identity"]
        .as_str()
        .expect("model package identity")
        .to_owned();
    evidence
        .insert_domain_package_document(model_identity, model_bytes)
        .expect("the selected model document supplies reader evidence");
    let read = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(_) = read else {
        panic!("the strict reader admits {package_identity}: {read:?}");
    };
    EmittedExample {
        package_identity: package_identity.to_owned(),
        wire,
    }
}

fn nodes<'a>(wire: &'a Value, tag: &str, form: &str) -> Vec<&'a Value> {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .filter(|node| node["node_tag"] == tag && node["semantic_form"] == form)
        .collect()
}

fn node_named<'a>(wire: &'a Value, tag: &str, form: &str, name: &str) -> &'a Value {
    nodes(wire, tag, form)
        .into_iter()
        .find(|node| {
            if tag == "claim" {
                node["node_id"]["path"][2].as_str() == Some(name)
            } else {
                node["body"].to_string().contains(&format!("\"{name}\""))
            }
        })
        .unwrap_or_else(|| panic!("{tag}/{form} declaration {name}"))
}

fn assert_node_package(node: &Value, package: &str) {
    assert_eq!(node["node_id"]["package"], package, "{node}");
}

fn binding<'a>(term: &'a Value, name: &str) -> &'a Value {
    match term {
        Value::Object(object) => {
            if object.get("name").and_then(Value::as_str) == Some(name) {
                return &object["value"];
            }
            object
                .values()
                .map(|value| binding(value, name))
                .find(|found| !found.is_null())
                .unwrap_or(&Value::Null)
        }
        Value::Array(items) => items
            .iter()
            .map(|value| binding(value, name))
            .find(|found| !found.is_null())
            .unwrap_or(&Value::Null),
        _ => &Value::Null,
    }
}

fn reference_targets(term: &Value) -> Vec<Value> {
    match term {
        Value::Object(object) => {
            let mut found = object
                .get("target")
                .cloned()
                .into_iter()
                .collect::<Vec<_>>();
            for (key, value) in object {
                if key != "target" {
                    found.extend(reference_targets(value));
                }
            }
            found
        }
        Value::Array(items) => items.iter().flat_map(reference_targets).collect(),
        _ => Vec::new(),
    }
}

fn path_contains_node(claim: &Value, node_id: &Value) -> bool {
    claim["node_id"]["path"]
        .as_array()
        .is_some_and(|path| path.iter().any(|segment| segment == node_id))
}

fn node_with_id<'a>(wire: &'a Value, node_id: &Value) -> &'a Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .find(|node| &node["node_id"] == node_id)
        .unwrap_or_else(|| panic!("node id is present: {node_id}"))
}

fn normalized_claim_path(
    claim: &Value,
    claim_name: &str,
    threshold: &str,
    package_identity: &str,
) -> Value {
    fn normalize(value: &mut Value, claim_name: &str, threshold: &str, package_identity: &str) {
        match value {
            Value::String(text) if text == claim_name => *text = "<claim-name>".to_owned(),
            Value::String(text) if text == threshold => *text = "<threshold>".to_owned(),
            Value::Array(items) => {
                for item in items {
                    normalize(item, claim_name, threshold, package_identity);
                }
            }
            Value::Object(items) => {
                for item in items.values_mut() {
                    normalize(item, claim_name, threshold, package_identity);
                }
                if items.get("package").and_then(Value::as_str) == Some(package_identity) {
                    items.insert(
                        "package".to_owned(),
                        Value::String("<unit-package>".to_owned()),
                    );
                }
            }
            _ => {}
        }
    }
    let mut path = claim["node_id"]["path"].clone();
    normalize(&mut path, claim_name, threshold, package_identity);
    path
}

fn assert_probability_claim(wire: &Value, claim: &Value, direction: &str, threshold: &str) {
    let body = &claim["body"];
    let form = binding(body, "form");
    let form_text = form.to_string();
    assert!(form_text.contains("probability"), "{form_text}");
    assert!(form_text.contains(direction), "{form_text}");
    assert!(
        form_text.contains(threshold),
        "expected {threshold}: {form_text}"
    );
    let event = binding(form, "event");
    assert!(event.to_string().contains("formula"), "{event}");
    let formula_nodes = reference_targets(binding(event, "formula"));
    let [formula_node] = formula_nodes.as_slice() else {
        panic!("one formula node reference: {event}");
    };
    let formula = node_with_id(wire, formula_node);
    assert_eq!(formula["node_tag"], "temporal");
    assert_eq!(formula["semantic_form"], "formula");
    assert_eq!(claim["node_id"]["ordinal"], 0);
    assert!(binding(body, "activation").to_string().contains("origin"));
    assert!(binding(body, "confidence")["members"]
        .as_array()
        .is_some_and(Vec::is_empty));
    let over_nodes = reference_targets(binding(body, "over"));
    let [over_node] = over_nodes.as_slice() else {
        panic!("one over parameter node: {}", binding(body, "over"));
    };
    assert_eq!(node_with_id(wire, over_node)["semantic_form"], "parameter");
}

fn assert_random_parameter(node: &Value, parameter: &str, weighted_support: &[(&str, &str)]) {
    let body = node["body"].to_string();
    assert!(
        body.contains(&format!("\"parameter\"")) && body.contains(parameter),
        "{body}"
    );
    for (value, weight) in weighted_support {
        assert!(
            body.contains(value) && body.contains(weight),
            "{parameter} {value}:{weight}: {body}"
        );
    }
}

fn assert_claim_bindings(
    claim: &Value,
    scheduler: &Value,
    distributions: &[Value],
    subject_package: &str,
) {
    let body = &claim["body"];
    let scheduler_targets = reference_targets(binding(body, "scheduler"));
    assert_eq!(scheduler_targets, [scheduler.clone()]);
    assert!(
        path_contains_node(claim, scheduler),
        "scheduler is a claim path operand"
    );

    let mut actual_distributions = reference_targets(binding(body, "distributions"));
    actual_distributions.sort_by_key(Value::to_string);
    let mut expected_distributions = distributions.to_vec();
    expected_distributions.sort_by_key(Value::to_string);
    assert_eq!(actual_distributions, expected_distributions);
    for distribution in distributions {
        assert!(
            path_contains_node(claim, distribution),
            "distribution is a claim path operand"
        );
    }
    assert!(reference_targets(binding(body, "reward")).is_empty());
    assert!(reference_targets(binding(body, "delays")).is_empty());
    let subject = reference_targets(binding(body, "subject"));
    assert_eq!(subject.len(), 1);
    assert_eq!(subject[0]["package"], subject_package);
    assert!(
        path_contains_node(claim, &subject[0]),
        "subject is a claim path operand"
    );
    assert_eq!(
        claim["dependencies"],
        serde_json::json!(reference_targets(body))
    );
    assert_eq!(claim["node_id"]["ordinal"], 0);
}

#[trace("TC-2062", "FR-740-AC-8")]
#[test]
fn staged_probabilistic_examples_emit_and_strict_reader_admits_all_units() {
    let cases = [
        (
            "examples/rover/requests/staged-vote-probability.json",
            "staged-vote-probability",
            "VoteRarelyWrong",
            "Driving",
            "example/rover",
            true,
            "at_most",
            "1/10000",
        ),
        (
            "examples/rover/requests/staged-vote-probability-missed.json",
            "staged-vote-probability-missed",
            "VoteAlmostNeverWrong",
            "Driving",
            "example/rover",
            true,
            "at_most",
            "1/1000000",
        ),
        (
            "examples/architecture/requests/staged-availability.json",
            "staged-availability",
            "CheckoutAvailable",
            "Traffic",
            "example/architecture",
            false,
            "at_least",
            "99/100",
        ),
        (
            "examples/architecture/requests/staged-availability-missed.json",
            "staged-availability-missed",
            "CheckoutAvailable",
            "Traffic",
            "example/architecture",
            false,
            "at_least",
            "999/1000",
        ),
    ];
    let mut emitted = Vec::new();
    for (
        request_path,
        package_suffix,
        claim_name,
        workload_name,
        model_package,
        rover,
        direction,
        threshold,
    ) in cases
    {
        let example = compile_example(request_path, None);
        assert!(example.package_identity.ends_with(package_suffix));
        let random_nodes = nodes(&example.wire, "probabilistic", "random_parameter");
        let workload = node_named(&example.wire, "probabilistic", "workload", workload_name);
        let claim = node_named(&example.wire, "claim", "probabilistic_claim", claim_name);
        if rover {
            assert_eq!(random_nodes.len(), 3);
            for (name, parameter) in [
                ("faultA", "faultA"),
                ("faultB", "faultB"),
                ("faultC", "faultC"),
            ] {
                let node = node_named(&example.wire, "probabilistic", "random_parameter", name);
                assert_random_parameter(node, parameter, &[("true", "1"), ("false", "999")]);
                assert_node_package(node, &example.package_identity);
            }
            for operation in ["sample", "loseLidar", "brake", "halt", "enterFailsafe"] {
                assert!(
                    workload["body"].to_string().contains(operation),
                    "{workload}"
                );
            }
        } else {
            assert_eq!(random_nodes.len(), 1);
            let random = node_named(&example.wire, "probabilistic", "random_parameter", "ok");
            let body = random["body"].to_string();
            assert!(body.contains("true") && body.contains("999"), "{body}");
            assert!(body.contains("false") && body.contains("1"), "{body}");
            assert!(body.contains("Service") && body.contains("call"), "{body}");
            assert_node_package(random, &example.package_identity);
        }
        assert_node_package(workload, &example.package_identity);
        assert_node_package(claim, &example.package_identity);
        let distribution_ids: Vec<Value> = random_nodes
            .iter()
            .map(|node| node["node_id"].clone())
            .collect();
        let mut sorted_ids = distribution_ids.clone();
        sorted_ids.sort_by_key(Value::to_string);
        assert_eq!(
            distribution_ids, sorted_ids,
            "claim distributions are sorted by node id"
        );
        assert_claim_bindings(
            claim,
            &workload["node_id"],
            &distribution_ids,
            model_package,
        );
        assert_eq!(claim["occurrences"].as_array().map(Vec::len), Some(1));
        assert_probability_claim(&example.wire, claim, direction, threshold);
        emitted.push(example);
    }

    for (passing_at, missed_at, pass_name, missed_name, pass_bound, missed_bound) in [
        (
            0,
            1,
            "VoteRarelyWrong",
            "VoteAlmostNeverWrong",
            "1/10000",
            "1/1000000",
        ),
        (
            2,
            3,
            "CheckoutAvailable",
            "CheckoutAvailable",
            "99/100",
            "999/1000",
        ),
    ] {
        let passing = &emitted[passing_at];
        let missed = &emitted[missed_at];
        let passing_claim = node_named(&passing.wire, "claim", "probabilistic_claim", pass_name);
        let missed_claim = node_named(&missed.wire, "claim", "probabilistic_claim", missed_name);
        assert_ne!(passing_claim["node_id"], missed_claim["node_id"]);
        assert_ne!(
            passing_claim["node_id"]["path"],
            missed_claim["node_id"]["path"]
        );
        assert_eq!(
            normalized_claim_path(
                passing_claim,
                pass_name,
                pass_bound,
                &passing.package_identity,
            ),
            normalized_claim_path(
                missed_claim,
                missed_name,
                missed_bound,
                &missed.package_identity,
            ),
            "missed and passing claim paths differ only by claim name and bound"
        );
        let passing_subject = reference_targets(binding(&passing_claim["body"], "subject"));
        let missed_subject = reference_targets(binding(&missed_claim["body"], "subject"));
        assert_eq!(
            passing_subject, missed_subject,
            "both claims bind the same model_root"
        );
    }

    let architecture_missed_same_package = compile_example(
        "examples/architecture/requests/staged-availability-missed.json",
        Some("example:architecture/staged-availability"),
    );
    let architecture_passing = &emitted[2];
    let same_package_claim = node_named(
        &architecture_missed_same_package.wire,
        "claim",
        "probabilistic_claim",
        "CheckoutAvailable",
    );
    let passing_claim = node_named(
        &architecture_passing.wire,
        "claim",
        "probabilistic_claim",
        "CheckoutAvailable",
    );
    assert_ne!(passing_claim["node_id"], same_package_claim["node_id"]);
    let unrelated_ids = |wire: &Value| {
        let mut ids = wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("semantic graph nodes")
            .iter()
            .filter(|node| node["node_tag"] != "claim")
            .map(|node| node["node_id"].clone())
            .collect::<Vec<_>>();
        ids.sort_by_key(Value::to_string);
        ids
    };
    assert_eq!(
        unrelated_ids(&architecture_passing.wire),
        unrelated_ids(&architecture_missed_same_package.wire),
        "the threshold changes only the claim identity under a shared package identity"
    );
}
