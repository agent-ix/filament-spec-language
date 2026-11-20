// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-740-AC-8 (TC-2062): the staged probabilistic examples compile through S4,
//! emit the expected checked nodes, and pass the strict package reader.

#![cfg(feature = "std")]

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{emit_checked, CheckedPackage};
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

fn compile_example(request_path: &str) -> EmittedExample {
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
    let checked = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        program.identity.clone(),
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
    let emission = emit_checked(&package).expect("S4 emits the package");
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
        panic!("the strict reader admits {}: {read:?}", program.identity);
    };
    EmittedExample {
        package_identity: program.identity,
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
            node["declaration"]["qualified_name"]
                .as_array()
                .is_some_and(|parts| parts.last().and_then(Value::as_str) == Some(name))
        })
        .unwrap_or_else(|| panic!("{tag}/{form} declaration {name}"))
}

fn assert_node_package(node: &Value, package: &str) {
    assert_eq!(node["node_id"]["package"], package, "{node}");
}

fn assert_random_parameter(node: &Value, parameter: &str, weighted_support: &[(&str, &str)]) {
    assert_eq!(
        node["declaration"]["qualified_name"][0],
        "Rover".to_string().as_str()
    );
    assert_eq!(node["body"].to_string().contains(parameter), true, "{node}");
    let body = node["body"].to_string();
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
    let body = claim["body"].to_string();
    assert!(
        body.contains(&scheduler.to_string()),
        "scheduler absent: {body}"
    );
    assert!(
        body.contains(&subject_package.to_owned()),
        "subject package absent: {body}"
    );
    assert!(
        body.contains("reward") && body.contains("delays"),
        "empty reward/delays bindings: {body}"
    );
    for distribution in distributions {
        assert!(
            body.contains(&distribution.to_string()),
            "distribution absent: {body}"
        );
    }
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
            false,
        ),
        (
            "examples/rover/requests/staged-vote-probability-missed.json",
            "staged-vote-probability-missed",
            "VoteAlmostNeverWrong",
            "Driving",
            "example/rover",
            true,
            true,
        ),
        (
            "examples/architecture/requests/staged-availability.json",
            "staged-availability",
            "CheckoutAvailable",
            "Traffic",
            "example/architecture",
            false,
            false,
        ),
        (
            "examples/architecture/requests/staged-availability-missed.json",
            "staged-availability-missed",
            "CheckoutAvailable",
            "Traffic",
            "example/architecture",
            false,
            true,
        ),
    ];
    for (request_path, package_suffix, claim_name, workload_name, model_package, rover, missed) in
        cases
    {
        let example = compile_example(request_path);
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
        assert!(!missed || claim["declaration"]["qualified_name"].as_array().is_some());
    }
}
