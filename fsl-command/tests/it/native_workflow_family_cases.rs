// SPDX-License-Identifier: AGPL-3.0-or-later
//! Extracted source executes against admitted observations; family requirements
//! arise from checked applications rather than the run's capability request.

use crate::config_version_spine::{spine, Case};
use filament_exact::Cancel;
use fsl_command::command::{self, RunLimits};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{self as workflow, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::check::{BodyTerm, Capability, LeafTerm, MemberTerm};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn source(file: &str, identity: &str, document: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": identity,
        "document": document, "formal_revision": 1})
}

fn body() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = {:?};\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{\n\
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber\n\
         }}\n",
        spine::PACKAGE_IDENTITY
    )
}

fn request() -> Value {
    let program = json!({
            "source": source("spec.md", "ix://agent-ix/native-workflow/original", "Original"),
            "extraction": {"body": {"authority": "agent-ix", "identity": "body", "document": "Body"}},
            "clauses": [{"clause": "parent_order", "package": spine::PACKAGE_IDENTITY}]
    });
    json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "model", "Model")}],
        "program": program,
        "clause": {"selection": {"clause": {"name": "ParentOrder", "observation": "current",
            "self": {"population": format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
                "key": "child"}, "anchor": {"kind": "handler", "name": "validate"}}},
            "observations": {"current": "spine-snapshot.json"}}
    }})
}

fn write(directory: &Path, request: &Value) -> PathBuf {
    let path = directory.join("run.json");
    std::fs::write(
        &path,
        serde_json::to_vec(request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn run(path: &Path) -> command::RunResult {
    command::run(path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("{}", error.value().expect("error document")))
}

fn markdown(body: &str, newline: &str) -> String {
    format!("# Original\n\n## Invariants\n\n### parent_order\n```ix:native\n{body}```\n")
        .replace('\n', newline)
}

#[test]
#[trace("FR-031-AC-4")]
fn extracted_parent_order_executes_both_truth_values_and_retains_quire_provenance() {
    for newline in ["\n", "\r\n"] {
        for (case, category, exit, truth) in [
            (Case::Healthy, "success", 0, true),
            (Case::Violating, "violation", 10, false),
        ] {
            let directory = tempfile::tempdir().expect("temporary directory");
            spine::write(directory.path(), case).expect("observations write");
            std::fs::write(directory.path().join("spec.md"), markdown(&body(), newline)).unwrap();
            let extracted_path = write(directory.path(), &request());
            let admitted = command::read_clause_run_request(&extracted_path)
                .expect("Quire admits the selected source");
            let workflow::ClauseRunSource::Extracted(admitted) = admitted.source else {
                panic!("Markdown requests retain the extracted source");
            };
            let upstream = admitted.extraction();
            let upstream = json!({"availability": upstream.availability,
                "clause_text": upstream.clause_text, "diagnostics": upstream.diagnostics,
                "clauses": upstream.clauses});
            let expected_segments: Vec<_> = admitted
                .map()
                .segments()
                .iter()
                .map(|segment| {
                    json!({"body_start": segment.body.start, "body_end": segment.body.end,
                    "original_start": segment.original.start, "original_end": segment.original.end})
                })
                .collect();
            let extracted = run(&extracted_path);
            let document = extracted.value.as_value();
            assert_eq!(extracted.exit_code, exit, "{document}");
            assert_eq!(document["disposition"]["category"], category, "{document}");
            assert_eq!(document["disposition"]["truth"], truth, "{document}");
            let extraction = &document["extraction"];
            assert_eq!(
                extraction["original"]["identity"],
                "ix://agent-ix/native-workflow/original"
            );
            assert_eq!(extraction["body"]["authority"], "agent-ix");
            assert_eq!(extraction["body"]["identity"], "body");
            assert_eq!(extraction["binding"]["clause"], "parent_order");
            assert_eq!(extraction["outcome"], upstream);
            assert!(!expected_segments.is_empty());
            assert_eq!(extraction["map"]["segments"], json!(expected_segments));
        }
    }
}

#[test]
#[trace("TC-109")]
fn extracted_bodies_refuse_missing_models_and_absent_declarations() {
    for (body, omit_models, selected, stage, code) in [
        (body(), true, "ParentOrder", "compile", "missing_import"),
        (body(), false, "Absent", "select", "missing_declaration"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine::write(directory.path(), Case::Healthy).unwrap();
        std::fs::write(directory.path().join("spec.md"), markdown(&body, "\n")).unwrap();
        let mut request = request();
        if omit_models {
            request["request"]["models"] = json!([]);
        }
        request["request"]["clause"]["selection"]["clause"]["name"] = json!(selected);
        let result = run(&write(directory.path(), &request));
        let document = result.value.as_value();
        assert_eq!(result.exit_code, 20, "{document}");
        assert_eq!(document["disposition"]["stage"], stage, "{document}");
        assert_eq!(
            document["disposition"]["refusal"]["code"], code,
            "{document}"
        );
        assert!(document["disposition"]["truth"].is_null(), "{document}");
    }
}

#[test]
#[trace("FR-031-AC-2")]
fn an_absent_extracted_selection_and_a_formal_fence_never_become_native_truth() {
    let directory = tempfile::tempdir().unwrap();
    let original = markdown(&body(), "\n");
    std::fs::write(directory.path().join("spec.md"), &original).unwrap();
    let mut missing = request();
    missing["request"]["program"]["clauses"][0]["clause"] = json!("absent");
    let error = command::run(
        &write(directory.path(), &missing),
        &crate::negotiation::Fr290,
    )
    .expect_err("selection fails before absent model and observation files are read");
    assert!(matches!(
        &error.cause,
        command::RunCause::ExtractionFailure {
            code: fsl_foundation::Code::InvalidModelBinding,
            stage: command::ExtractionFailureStage::Selection,
            ..
        }
    ));
    let failure = error.value().unwrap();
    assert_eq!(error.category().exit_code(), 20);
    assert_eq!(
        failure["details"]["original"]["identity"],
        "ix://agent-ix/native-workflow/original"
    );
    assert_eq!(failure["details"]["binding"]["clause"], "absent");
    assert!(failure["details"]["outcome"]["clauses"].is_array());

    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(
        directory.path().join("spec.md"),
        original.replace("```ix:native", "```ix:formal"),
    )
    .unwrap();
    let result = run(&write(directory.path(), &request()));
    let document = result.value.as_value();
    assert_eq!(result.exit_code, 20, "{document}");
    assert_eq!(document["disposition"]["stage"], "compile");
    assert_eq!(
        document["disposition"]["refusal"]["code"],
        "unknown_language"
    );
    assert!(document["disposition"]["truth"].is_null());
}

#[test]
#[ignore = "QSL-2725: extracted-body clause-count admission is unimplemented; FR-031-AC-9 remains uncounted"]
fn an_extracted_body_with_two_clauses_refuses_its_actual_count() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let body = format!(
        "{}\ninvariant Other using v on Config::ConfigVersion at current {{ true }}\n",
        body()
    );
    std::fs::write(directory.path().join("spec.md"), markdown(&body, "\n")).unwrap();
    let path = write(directory.path(), &request());
    let admitted = command::read_clause_run_request(&path).expect("the selected fence extracts");
    let workflow::ClauseRunSource::Extracted(extracted) = &admitted.source else {
        panic!("the request reaches the extracted-body boundary");
    };
    assert!(extracted
        .map()
        .body()
        .text()
        .contains("invariant ParentOrder"));
    assert!(extracted.map().body().text().contains("invariant Other"));
    let result = run(&path);
    let document = result.value.as_value();
    assert_eq!(result.exit_code, 20, "{document}");
    assert_eq!(document["disposition"]["stage"], "compile");
    assert_eq!(
        document["disposition"]["refusal"]["code"],
        "extraction-clause-count"
    );
    assert_eq!(
        document["disposition"]["refusal"]["details"],
        json!({"actual": 2})
    );
    assert!(document["disposition"]["truth"].is_null());
}

#[test]
#[trace("FR-026-AC-7")]
fn native_run_intake_accepts_exact_byte_limits_and_returns_incomplete_above_them() {
    let directory = tempfile::tempdir().unwrap();
    let text = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function answer using v(): Integer pure { 7 }\n";
    std::fs::write(directory.path().join("unit.native"), text).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [], "program": {"source": source("unit.native", "p", "P")},
        "call": {"function": "answer", "arguments": []}
    }});
    let path = write(directory.path(), &request);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(b"   \n");
    std::fs::write(&path, &bytes).unwrap();
    let exact = RunLimits {
        request_bytes: bytes.len(),
        dependent_bytes: text.len(),
        ..RunLimits::default()
    };
    let result = command::run_with_limits(&path, exact, &crate::negotiation::Fr290).unwrap();
    assert_eq!(result.exit_code, 0, "{}", result.value);
    for (setting, option, limits, bound) in [
        (
            "request_bytes",
            "--request-bytes",
            RunLimits {
                request_bytes: bytes.len() - 1,
                ..exact
            },
            bytes.len() - 1,
        ),
        (
            "dependent_bytes",
            "--dependent-bytes",
            RunLimits {
                dependent_bytes: text.len() - 1,
                ..exact
            },
            text.len() - 1,
        ),
    ] {
        let error = command::run_with_limits(&path, limits, &crate::negotiation::Fr290)
            .expect_err("one byte above the configured limit stops intake");
        assert_eq!(error.category().exit_code(), 22);
        let outcome: Value = serde_json::from_slice(
            &error
                .outcome_document(fsl_replay::Operation::Execute)
                .to_bytes()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(outcome["category"], "incomplete", "{outcome}");
        let diagnostics = outcome["diagnostics"].as_array().expect("diagnostics");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["code"], "resource_exhausted", "{outcome}");
        assert!(diagnostics[0]["cause"].is_null());
        assert!(diagnostics[0]["locus"].is_null());
        let message = diagnostics[0]["message"].as_str().expect("limit message");
        assert!(
            message.contains(setting)
                && message.contains(option)
                && message.contains(&bound.to_string()),
            "{outcome}"
        );
    }
    assert_eq!(run(&path).exit_code, 0);
}

fn checked(body: &str) -> workflow::CheckedUnit {
    let source = SourceIdentity::new("agent-ix", "family-contract");
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{body}\n");
    let limits = SpineLimits::default();
    let parsed = workflow::parse(
        &ParseRequest {
            source: &source,
            path: "family.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &Cancel::new(),
    )
    .expect("family source parses")
    .into_value();
    let models = workflow::select(&parsed, &BTreeMap::new(), limits.model, &Cancel::new())
        .expect("no models required")
        .into_value();
    workflow::check(
        &parsed,
        "family".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &Cancel::new(),
    )
    .expect("family body checks")
    .into_value()
}

#[test]
#[trace("FR-062-AC-4", "FR-057-AC-10")]
fn checked_boolean_connectives_request_nothing_and_scalar_addition_requests_its_own_site() {
    let boolean =
        checked("function both using v(b: Boolean, c: Boolean): Boolean pure { b and c }");
    assert!(boolean.package().graph().requirements().is_empty());
    let numeric_body =
        "function addPair using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }";
    let numeric = checked(numeric_body);
    let graph = numeric.package().graph();
    let first = graph.requirements();
    let repeated = checked(numeric_body);
    assert_eq!(
        first,
        repeated.package().graph().requirements(),
        "independent checks compute identical requirements"
    );
    assert_eq!(first.len(), 1);
    let (site, record) = first.iter().next().expect("one addition claim");
    let application = graph
        .semantic_graph()
        .node(site.node().clone())
        .expect("checked claim site");
    let BodyTerm::Application(application) = application.body() else {
        panic!("the claim belongs to the scalar addition");
    };
    assert_eq!(application.operation.identity(), "filament.op.integer.add");
    assert_eq!(
        site.origin(),
        &filament_exact::Origin::new(filament_exact::Role::new("expression"), 0)
    );
    assert_eq!(record.requirements().kind(), Capability::ValueValidity);
    assert_eq!(record.owner(), Some("addPair"));
    assert_eq!(
        record.result_bound().value_type(),
        &filament_exact::ValueType::Int(
            filament_exact::IntegerInterval::new(0_i64.into(), 18_i64.into())
                .expect("closed sum interval"),
        )
    );
    assert_eq!(record.extent_roots().len(), 2);
    let context = record
        .conversion_context()
        .expect("the declared result bound narrows the addition");
    let narrow = graph
        .semantic_graph()
        .node(context.node().clone())
        .expect("checked result narrow");
    let BodyTerm::Application(narrow) = narrow.body() else {
        panic!("the result conversion is an application");
    };
    assert_eq!(narrow.operation.identity(), "filament.op.numeric.narrow");
    assert!(
        matches!(narrow.arguments.as_slice(), [MemberTerm::Leaf(LeafTerm::Reference {target})] if target.admitted() == Some(site.node()))
    );
    assert!(record.path_condition().is_empty());
    let location = graph
        .occurrence(site.node().clone(), site.origin())
        .expect("claim has its own expression occurrence");
    assert_eq!(
        location.origin,
        filament_semantic_value::location::ExpressionOwner::Body {
            function: "addPair".to_owned(),
            index: 0,
        }
    );
    assert!(location.path().is_empty());
    assert!(graph
        .occurrences_at(location)
        .contains(&(site.node().clone(), site.origin().clone())));
}
