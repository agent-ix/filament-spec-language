// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface decisions: extension definitions in a request (FR-660),
//! the run command's Markdown extraction in its default build (FR-662), the
//! parse report (FR-663), the current-selection `parameters` refusal
//! (FR-664) and the compile request's `handoff` member (FR-665). The
//! driver's verbs over these surfaces are tested in the driver repository.

use crate::support::config_version_spine::{spine, Case};
use filament_exact::Cancel;
use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{parse, ParseRequest};
use fsl_replay::OutcomeDocument;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";
const FORM: &str = "retention Audit { subject: \"audit\", days: 30 }";

fn retention_definition(directory: &Path, file: &str, keyword: &str) {
    let definition = json!({
        "grammar_schema": {"productions": [
            {"keyword": keyword, "clauses": ["subject", "days"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": keyword, "node_kind": keyword, "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": "Int[1, 3650]"}
            ]}
        ]}
    });
    std::fs::write(directory.join(file), definition.to_string()).unwrap();
}

/// Writes the extension unit and a compile request whose `extensions`
/// member is `extensions`, and returns the request path.
fn extension_request(directory: &Path, extensions: Value) -> std::path::PathBuf {
    std::fs::write(
        directory.join("unit.native"),
        format!("{HEADER}profile r = \"ext.retention\";\n{FORM}\n"),
    )
    .unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:extension-request", "document": "Ext", "formal_revision": 1}},
        "extensions": extensions
    }});
    let path = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn compile_refusal(path: &Path) -> (u8, Value) {
    let error = fsl_command::command::compile(path).expect_err("the request refuses");
    (error.category().exit_code(), error.value().unwrap())
}

/// FR-660-AC-2 (TC-1741): a header selecting an identity the request's
/// `extensions` do not supply refuses `unknown_profile`, not an
/// unknown-member request refusal.
#[test]
#[trace("TC-1741", "FR-660-AC-2")]
#[ignore = "QSL-2189: the request `extensions` member is not implemented yet"]
fn an_unsupplied_extension_selection_refuses_unknown_profile() {
    let directory = tempfile::tempdir().unwrap();
    retention_definition(directory.path(), "other.json", "other");
    let path = extension_request(
        directory.path(),
        json!([{"authority": "agent-ix", "identity": "ext.other", "file": "other.json"}]),
    );
    let (exit, value) = compile_refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "unknown_profile", "{value}");
    assert!(
        value.to_string().contains("unsupported-selection"),
        "{value}"
    );
}

/// FR-660-AC-3 (TC-1741): a supplied definition missing
/// `typed_node_schema` refuses `invalid_package`/`missing-member` naming it.
#[test]
#[trace("TC-1741", "FR-660-AC-3")]
#[ignore = "QSL-2189: the request `extensions` member is not implemented yet"]
fn a_supplied_definition_missing_a_member_refuses() {
    let directory = tempfile::tempdir().unwrap();
    let definition = json!({"grammar_schema": {"productions": [
        {"keyword": "retention", "clauses": ["subject", "days"]}
    ]}});
    std::fs::write(
        directory.path().join("retention.json"),
        definition.to_string(),
    )
    .unwrap();
    let path = extension_request(
        directory.path(),
        json!([{"authority": "agent-ix", "identity": "ext.retention", "file": "retention.json"}]),
    );
    let (exit, value) = compile_refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_package", "{value}");
    let text = value.to_string();
    assert!(text.contains("missing-member"), "{value}");
    assert!(text.contains("ext.retention"), "{value}");
    assert!(text.contains("typed_node_schema"), "{value}");
}

/// FR-660-AC-4 (TC-1741): malformed `extensions` members refuse
/// `invalid-request` at stage `request` naming the member, and two entries
/// with one identity refuse `conflicting-definition`.
#[test]
#[trace("TC-1741", "FR-660-AC-4")]
#[ignore = "QSL-2189: the request `extensions` member is not implemented yet"]
fn malformed_extension_entries_refuse_before_compile() {
    for (extensions, member) in [
        (json!(null), "extensions"),
        (
            json!([{"authority": "agent-ix", "identity": "ext.retention",
                "file": "retention.json", "version": "1"}]),
            "version",
        ),
        (
            json!([{"authority": "agent-ix", "identity": "ext.retention"}]),
            "file",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = extension_request(directory.path(), extensions);
        let (exit, value) = compile_refusal(&path);
        assert_eq!(exit, 20, "{member}: {value}");
        assert_eq!(value["code"], "invalid-request", "{member}: {value}");
        assert_eq!(value["stage"], "request", "{member}: {value}");
        assert!(value.to_string().contains(member), "{member}: {value}");
    }
    let directory = tempfile::tempdir().unwrap();
    retention_definition(directory.path(), "retention.json", "retention");
    let entry =
        json!({"authority": "agent-ix", "identity": "ext.retention", "file": "retention.json"});
    let path = extension_request(directory.path(), json!([entry.clone(), entry]));
    let (exit, value) = compile_refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_package", "{value}");
    assert!(
        value.to_string().contains("conflicting-definition"),
        "{value}"
    );
}

/// FR-312's current-snapshot clause-run request over the spine files.
fn current_clause_request() -> Value {
    let source = |file: &str, document: &str| {
        json!({
            "file": file,
            "authority": "agent-ix", "identity": format!("test:fr-664:{file}"),
            "document": document, "formal_revision": 1
        })
    };
    json!({"format":"native-run/1", "request": {
        "models": [{"format":"semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
        "clause": {
            "selection": {"clause": {"name":"ParentOrder", "observation":"current",
                "self": {"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY),
                    "key":"child"}, "anchor":{"kind":"handler", "name":"validate"}}},
            "observations": {"current":"spine-snapshot.json"}
        }
    }})
}

fn run_refusal(directory: &Path, request: &Value) -> (u8, Value) {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    match fsl_command::command::run(&path) {
        Ok(result) => panic!("the request runs: {}", result.value),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

/// FR-664-AC-1 (TC-1747): a `Current` selection carrying `parameters`
/// refuses `invalid-request` at stage `request` naming `parameters`, with or
/// without the request's dependent files present.
#[test]
#[trace("TC-1747", "FR-664-AC-1")]
#[ignore = "QSL-1987: the current-selection `parameters` refusal is not pinned yet"]
fn a_current_selection_with_parameters_refuses_naming_parameters() {
    for parameters in [json!({}), json!({"target": {"reference": "child"}})] {
        for with_files in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            if with_files {
                spine::write(directory.path(), Case::Healthy).unwrap();
            }
            let mut request = current_clause_request();
            request["request"]["clause"]["selection"]["clause"]["parameters"] = parameters.clone();
            let (exit, value) = run_refusal(directory.path(), &request);
            assert_eq!(exit, 20, "{value}");
            assert_eq!(value["code"], "invalid-request", "{value}");
            assert_eq!(value["stage"], "request", "{value}");
            assert!(value.to_string().contains("parameters"), "{value}");
        }
    }
}

/// FR-662-AC-3 (TC-1744): the command owner's default build admits FR-031's
/// extraction mode, so a clause-run request with two `clauses` bindings
/// refuses `extraction-clause-count`, not `invalid-request`.
#[test]
#[trace("TC-1744", "FR-662-AC-3")]
#[ignore = "QSL-2019: the run command's extraction mode is not in the default build yet"]
fn the_run_command_reaches_markdown_extraction_by_default() {
    let directory = tempfile::tempdir().unwrap();
    let mut request = current_clause_request();
    request["request"]["program"] = json!({
        "source": {"file": "spec.md", "authority": "agent-ix", "identity": "doc",
            "document": "Doc", "formal_revision": 1},
        "extraction": {"body": {"authority": "agent-ix", "identity": "b", "document": "B"}},
        "clauses": [
            {"clause": "ParentOrder", "package": "doc"},
            {"clause": "NoCycle", "package": "doc"}
        ]
    });
    let (exit, value) = run_refusal(directory.path(), &request);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "extraction-clause-count", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    assert_eq!(value["details"], json!({"actual": 2}), "{value}");
}

/// FR-663-AC-1 (TC-1745): a completed parse's outcome document carries the
/// `parsed` report of the labels, path, byte count and S1 usage.
#[test]
#[trace("TC-1745", "FR-663-AC-1")]
#[ignore = "QSL-2191: the parse outcome's `parsed` result is not implemented yet"]
fn the_parse_outcome_reports_the_parsed_unit() {
    const FIXTURE: &[u8] = include_bytes!("../fixtures/spine-compile.native");
    let identity = SourceIdentity::new("agent-ix", "test:parent");
    let result = parse(
        &ParseRequest {
            source: &identity,
            path: "tests/fixtures/spine-compile.native",
            bytes: FIXTURE,
        },
        Limits::default(),
        &Cancel::new(),
    );
    let document: Value =
        serde_json::from_slice(&OutcomeDocument::from_parse(&result).to_bytes().unwrap()).unwrap();
    let report = &document["result"];
    assert_eq!(report["kind"], "parsed", "{document}");
    assert_eq!(
        report["source"],
        json!({"authority": "agent-ix", "identity": "test:parent"})
    );
    assert_eq!(report["path"], "tests/fixtures/spine-compile.native");
    assert_eq!(report["bytes"], FIXTURE.len().to_string());
    let settings: Vec<&str> = report["usage"]
        .as_array()
        .expect("usage is an array")
        .iter()
        .map(|row| row["setting"].as_str().unwrap())
        .collect();
    assert_eq!(
        settings,
        ["s1.input_bytes", "s1.tokens", "s1.nodes", "s1.work_units"]
    );
    assert_eq!(report["usage"][0]["used"], FIXTURE.len().to_string());
}

fn handoff_request(directory: &Path, handoff: Value) -> std::path::PathBuf {
    std::fs::copy(
        "tests/fixtures/spine-compile.native",
        directory.join("program.native"),
    )
    .unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "agent-ix",
            "identity": "test:spine", "document": "Spine", "formal_revision": 1}},
        "handoff": handoff
    }});
    let path = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// FR-665-AC-1 (TC-1748): a compile request's `handoff` member writes the
/// checked-package handoff directory beside the compile.
#[test]
#[trace("TC-1748", "FR-665-AC-1")]
#[ignore = "QSL-2193: the compile request's `handoff` member is not implemented yet"]
fn a_compile_handoff_member_writes_the_directory() {
    let directory = tempfile::tempdir().unwrap();
    let path = handoff_request(
        directory.path(),
        json!({"directory": "handoff", "package_name": "spine",
            "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}),
    );
    let package = fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let handoff = directory.path().join("handoff");
    assert_eq!(
        std::fs::read(handoff.join("checked-package.json")).unwrap(),
        package
    );
    let selection: Value =
        serde_json::from_slice(&std::fs::read(handoff.join("selection.json")).unwrap()).unwrap();
    assert_eq!(selection["format"], "filament.checked-handoff.selection/1");
    for file in ["identities.json", "equality.json", "inventory.json"] {
        assert!(handoff.join(file).is_file(), "{file}");
    }
    assert!(!handoff.join("mutations").exists());
}

/// FR-665-AC-3 (TC-1748): a `handoff` member without `native_join` refuses
/// `invalid-request` naming it and writes no directory.
#[test]
#[trace("TC-1748", "FR-665-AC-3")]
#[ignore = "QSL-2193: the compile request's `handoff` member is not implemented yet"]
fn a_handoff_member_without_a_native_join_budget_refuses() {
    let directory = tempfile::tempdir().unwrap();
    let path = handoff_request(
        directory.path(),
        json!({"directory": "handoff", "package_name": "spine"}),
    );
    let (exit, value) = compile_refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    assert!(value.to_string().contains("native_join"), "{value}");
    assert!(!directory.path().join("handoff").exists());
}

/// The driver binary: `FSL_BIN` when set, else `fsl` on `PATH`. The FSL
/// workspace builds no user binary (FR-287), so `version` runs through the
/// driver's CLI, as `retained_criteria_matrix` runs `quire` from `PATH`.
fn driver_version(arguments: &[&str]) -> std::process::Output {
    let binary = std::env::var("FSL_BIN").unwrap_or_else(|_| "fsl".to_owned());
    std::process::Command::new(binary)
        .arg("version")
        .args(arguments)
        .output()
        .expect("the driver binary runs")
}

/// FR-661 / FSpec FR-560-AC-1, AC-3 (TC-1742, FSpec-TC-1240): `version`
/// writes a `filament.build-inventory/v1` document holding the five lists,
/// and `--kind` selects one of them.
#[test]
#[trace("TC-1742", "FR-661-AC-1", "FSpec-TC-1240", "FSpec-FR-560-AC-1")]
#[trace("FSpec-TC-1240", "FSpec-FR-560-AC-3")]
#[ignore = "QSL-2194: `version` does not emit the build inventory yet"]
fn version_emits_the_build_inventory_and_kind_selects_one_list() {
    let lists = ["editions", "profiles", "families", "features", "consumers"];
    let output = driver_version(&["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_eq!(
        document["format"], "filament.build-inventory/v1",
        "{document}"
    );
    for list in lists {
        assert!(document[list].is_array(), "{list}: {document}");
    }
    assert!(document["editions"]
        .as_array()
        .unwrap()
        .contains(&json!("1-draft")));
    let features: Vec<String> = fsl_semantics::library::bundle::CapabilityId::complete_inventory()
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    assert_eq!(document["features"], json!(features));
    assert_eq!(driver_version(&["--format", "json"]).stdout, output.stdout);

    let selected = driver_version(&["--format", "json", "--kind", "profiles"]);
    assert_eq!(selected.status.code(), Some(0), "{selected:?}");
    let selected: Value = serde_json::from_slice(&selected.stdout).unwrap();
    let mut members: Vec<&str> = selected
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    members.sort_unstable();
    assert_eq!(members, ["application", "format", "profiles"]);

    let refused = driver_version(&["--kind", "widgets"]);
    assert_eq!(refused.status.code(), Some(20), "{refused:?}");
    assert!(refused.stdout.is_empty());
    let message = String::from_utf8_lossy(&refused.stderr);
    for name in std::iter::once("widgets").chain(lists) {
        assert!(message.contains(name), "{name}: {message}");
    }
}
