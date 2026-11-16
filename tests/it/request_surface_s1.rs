// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface decisions: extension definitions in a request (FR-660),
//! the run command's Markdown extraction in its default build (FR-662), the
//! parse report (FR-663), the current-selection `parameters` refusal
//! (FR-664) and the compile request's `handoff` member (FR-665). The
//! driver's verbs over these surfaces are tested in the driver repository.

use crate::command_harness::{run_file, Output};
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
            spine::write(directory.path(), Case::Healthy).unwrap();
            if !with_files {
                for file in ["spine-model.semantic-ir.json", "spine-unit.native"] {
                    std::fs::remove_file(directory.path().join(file)).unwrap();
                }
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
    let repeated = parse(
        &ParseRequest {
            source: &identity,
            path: "tests/fixtures/spine-compile.native",
            bytes: FIXTURE,
        },
        Limits::default(),
        &Cancel::new(),
    );
    let repeated: Value =
        serde_json::from_slice(&OutcomeDocument::from_parse(&repeated).to_bytes().unwrap())
            .unwrap();
    assert_eq!(repeated["result"], document["result"]);

    let token_bound = report["usage"][1]["used"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let bounded = parse(
        &ParseRequest {
            source: &identity,
            path: "tests/fixtures/spine-compile.native",
            bytes: FIXTURE,
        },
        Limits::default().with_tokens(token_bound),
        &Cancel::new(),
    );
    let bounded: Value =
        serde_json::from_slice(&OutcomeDocument::from_parse(&bounded).to_bytes().unwrap()).unwrap();
    assert_eq!(
        bounded["result"]["usage"][1]["used"],
        bounded["result"]["usage"][1]["bound"]
    );
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

/// FR-665's existing destination refusal leaves every existing byte intact.
#[test]
#[trace("TC-1748", "FR-665-AC-1")]
fn a_compile_handoff_refuses_an_existing_destination_without_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let handoff = directory.path().join("handoff");
    std::fs::create_dir(&handoff).unwrap();
    std::fs::write(handoff.join("sentinel"), b"keep me").unwrap();
    let request = handoff_request(
        directory.path(),
        json!({"directory": "handoff", "package_name": "spine",
            "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}),
    );

    let (exit, value) = compile_refusal(&request);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(std::fs::read(handoff.join("sentinel")).unwrap(), b"keep me");
    assert_eq!(std::fs::read_dir(&handoff).unwrap().count(), 1);
}

/// FR-665-AC-3 (TC-1748): a missing native-join budget, invalid corpus name or
/// unknown handoff member refuses at request admission and writes no directory.
#[test]
#[trace("TC-1748", "FR-665-AC-3")]
fn invalid_handoff_members_refuse_before_writing() {
    let cases = [
        (
            json!({"directory": "handoff", "package_name": "spine"}),
            "native_join",
        ),
        (
            json!({"directory": "handoff", "package_name": "spine", "corpus": "all",
                "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}),
            "corpus",
        ),
        (
            json!({"directory": "handoff", "package_name": "spine", "digest": "sha256",
                "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}),
            "digest",
        ),
    ];
    for (handoff, member) in cases {
        let directory = tempfile::tempdir().unwrap();
        let path = handoff_request(directory.path(), handoff);
        let (exit, value) = compile_refusal(&path);
        assert_eq!(exit, 20, "{member}: {value}");
        assert_eq!(value["code"], "invalid-request", "{member}: {value}");
        assert_eq!(value["stage"], "request", "{member}: {value}");
        assert!(value.to_string().contains(member), "{member}: {value}");
        assert!(!directory.path().join("handoff").exists());
    }
}

/// `version` with `arguments`, through [`driver`].
fn driver_version(arguments: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let argv: Vec<&str> = std::iter::once("version")
        .chain(arguments.iter().copied())
        .collect();
    driver(directory.path(), &argv)
}

/// FR-661 / FSpec FR-560-AC-1, AC-3 (TC-1742, FSpec-TC-1240): `version`
/// writes a `filament.build-inventory/v1` document holding the five lists,
/// and `--kind` selects one of them.
#[test]
#[trace("TC-1742", "FR-661-AC-1", "TC-1742", "FR-661-AC-3")]
#[trace(
    "FSpec-TC-1240",
    "FSpec-FR-560-AC-1",
    "FSpec-TC-1240",
    "FSpec-FR-560-AC-3"
)]
#[trace("FSpec-TC-1244", "FSpec-FR-301-AC-5")]
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

/// One CLI invocation, in process. The FSL workspace builds no binary and
/// depends on no driver (FR-287), so the invocation goes through the command
/// owner's in-process entry, `fsl_command::command::run`, as the driver
/// would hand it its argv: the argv is written to `argv.json` in `directory`
/// and run. Until the command owner takes these invocations, the call
/// refuses and the caller's assertion on the decided output fails.
fn driver(directory: &Path, arguments: &[&str]) -> Output {
    let path = directory.join("argv.json");
    std::fs::write(&path, json!({"argv": arguments}).to_string()).unwrap();
    run_file(&path)
}

/// The machine document on stdout, or `null` when stdout holds none, so the
/// caller's assertion on its members reports the difference.
fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or(Value::Null)
}

/// FR-660-AC-5 (TC-1741): an extension definition document is a dependent
/// file charged against `dependent_bytes`.
#[test]
#[trace("TC-1741", "FR-660-AC-5")]
#[ignore = "QSL-2189: the request `extensions` member is not implemented yet"]
fn an_extension_document_is_charged_against_dependent_bytes() {
    let directory = tempfile::tempdir().unwrap();
    retention_definition(directory.path(), "retention.json", "retention");
    let unit = format!(
        "{HEADER}profile r = \"ext.retention\";\n{FORM}\nfunction Keep using v (x: Integer): Integer pure {{ x }}\n"
    );
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:extension-run", "document": "Ext", "formal_revision": 1}},
        "call": {"function": "Keep", "arguments": [{"parameter": "x", "value": 1}]},
        "extensions": [{"authority": "agent-ix", "identity": "ext.retention",
            "file": "retention.json"}]
    }});
    let path = directory.path().join("run.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let total = unit.len()
        + std::fs::metadata(directory.path().join("retention.json"))
            .unwrap()
            .len() as usize;
    let limits = |dependent_bytes| fsl_command::command::RunLimits {
        dependent_bytes,
        ..fsl_command::command::RunLimits::default()
    };
    let error = fsl_command::command::run_with_limits(&path, limits(total - 1))
        .expect_err("one byte over the ceiling refuses");
    assert!(
        error
            .value()
            .unwrap()
            .to_string()
            .contains("dependent_bytes"),
        "{}",
        error.value().unwrap()
    );
    let result = fsl_command::command::run_with_limits(&path, limits(total))
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(result.exit_code, 0, "{}", result.value);
}

/// FR-661-AC-2, FSpec FR-560-AC-2 (TC-1742, FSpec-TC-1240): every profile
/// `version` lists compiles when a header selects it with its `requires`
/// profiles, and none is spelled `quire.*`.
#[test]
#[trace("TC-1742", "FR-661-AC-2", "FSpec-TC-1240", "FSpec-FR-560-AC-2")]
fn every_listed_profile_is_accepted_by_a_header() {
    let directory = tempfile::tempdir().unwrap();
    let inventory = stdout_json(&driver(
        directory.path(),
        &["version", "--format", "json", "--kind", "profiles"],
    ));
    assert!(inventory["profiles"].is_array(), "{inventory}");
    let profiles = inventory["profiles"].as_array().unwrap();
    assert!(!profiles.is_empty());
    for (index, profile) in profiles.iter().enumerate() {
        let identity = profile["identity"].as_str().unwrap();
        assert!(!identity.starts_with("quire."), "{identity}");
        let mut header = String::from("language \"ix:native\" edition \"1-draft\";\n");
        for (alias, required) in profile["requires"].as_array().unwrap().iter().enumerate() {
            header.push_str(&format!(
                "profile r{alias} = \"{}\";\n",
                required.as_str().unwrap()
            ));
        }
        header.push_str(&format!("profile p = \"{identity}\";\n"));
        let unit = format!("unit{index}.native");
        std::fs::write(directory.path().join(&unit), header).unwrap();
        let request = json!({"format": "native-compile/1", "request": {"models": [],
            "program": {"source": {"file": unit, "authority": "agent-ix",
                "identity": format!("test:profile-{index}"), "document": "P",
                "formal_revision": 1}}}});
        let path = directory.path().join(format!("compile{index}.json"));
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        if let Err(error) = fsl_command::command::compile(&path) {
            let value = error.value().unwrap();
            assert_ne!(value["code"], "unknown_profile", "{identity}: {value}");
        }
    }
}

/// FR-661-AC-3: a registry provider appears in `consumers` without changing
/// the compiler-owned lists.
#[test]
#[trace("TC-1742", "FR-661-AC-3")]
fn version_includes_provider_backed_registry_consumers() {
    use fsl_foundation::bound::DomainKind;
    use fsl_foundation::BackendId;
    use fsl_route::{Candidate, ManifestMembers, Mode, ProviderOrigin, Registry};

    let base = Registry::new();
    let before = fsl_replay::build_inventory(&base);
    let mut with_provider = Registry::new();
    let descriptor = fsl_route::descriptor_from_manifest(
        Candidate::new(BackendId::new("inventory-provider")),
        ProviderOrigin::Linked,
        [(Some("value-validity"), Some("unbounded"))],
        ManifestMembers {
            domains: Some(&[DomainKind::Integer.to_wire()]),
            ..ManifestMembers::default()
        },
    )
    .expect("the provider descriptor is valid");
    with_provider
        .register(descriptor)
        .expect("the provider registers");

    let after = fsl_replay::build_inventory(&with_provider);
    assert_eq!(before.editions, after.editions);
    assert_eq!(before.profiles, after.profiles);
    assert_eq!(before.families, after.families);
    assert_eq!(before.features, after.features);
    assert_eq!(after.consumers.len(), 1);
    assert_eq!(after.consumers[0].identity, "inventory-provider");
    assert_eq!(after.consumers[0].origin, "linked");
    assert_eq!(after.consumers[0].advertises.len(), 1);
    assert_eq!(after.consumers[0].advertises[0].kind, "value-validity");
    assert_eq!(after.consumers[0].advertises[0].mode, "unbounded");
    assert_eq!(
        before.to_bytes().expect("inventory writes"),
        fsl_replay::build_inventory(&base)
            .to_bytes()
            .expect("inventory writes")
    );
}

const SPINE_FIXTURE: &str = "tests/fixtures/spine-compile.native";

fn fence(heading: Option<&str>, body: &str) -> String {
    let heading = heading.map_or(String::new(), |id| format!("### {id}\n\n"));
    format!("{heading}```ix:native\n{body}```\n\n")
}

/// FR-662-AC-1, FSpec FR-563-AC-1 (TC-1743, FSpec-TC-1243): `check
/// --markdown` checks each native fence as its own named unit.
#[test]
#[trace("TC-1743", "FR-662-AC-1", "FSpec-TC-1243", "FSpec-FR-563-AC-1")]
#[trace("FSpec-TC-1244", "FSpec-FR-301-AC-5")]
#[ignore = "QSL-2019: no command checks the native fences of a Markdown document yet"]
fn check_markdown_names_each_fence_unit() {
    let directory = tempfile::tempdir().unwrap();
    let healthy = std::fs::read_to_string(SPINE_FIXTURE).unwrap();
    let broken = format!("{healthy}function Bad using v (x: Integer): Boolean pure {{ x }}\n");
    let document = format!(
        "# Spec\n\n{}{}```text\nnot checked\n```\n",
        fence(Some("Healthy"), &healthy),
        fence(Some("Broken"), &broken)
    );
    std::fs::write(directory.path().join("spec.md"), document).unwrap();
    let output = driver(
        directory.path(),
        &[
            "check",
            "--markdown",
            "agent-ix",
            "doc",
            "spec.md",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let outcome = stdout_json(&output);
    assert_eq!(outcome["operation"], "check", "{outcome}");
    let items = outcome["items"].as_array().expect("items");
    assert_eq!(items.len(), 2, "{outcome}");
    assert!(items[0].to_string().contains("doc#Healthy"), "{outcome}");
    let second = items[1].to_string();
    for fragment in ["ill_typed", "doc#Broken", "Broken", "spec.md"] {
        assert!(second.contains(fragment), "{fragment}: {outcome}");
    }
}

/// FR-662-AC-2 (TC-1743): two fences under one heading and a fence under no
/// heading refuse as their own items.
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
#[ignore = "QSL-2019: no command checks the native fences of a Markdown document yet"]
fn check_markdown_refuses_doubled_and_headless_fences() {
    let directory = tempfile::tempdir().unwrap();
    let healthy = std::fs::read_to_string(SPINE_FIXTURE).unwrap();
    let document = format!(
        "{}{}{}",
        fence(None, &healthy),
        fence(Some("Twice"), &healthy),
        fence(None, &healthy)
    );
    std::fs::write(directory.path().join("spec.md"), document).unwrap();
    let output = driver(
        directory.path(),
        &[
            "check",
            "--markdown",
            "agent-ix",
            "doc",
            "spec.md",
            "--format",
            "json",
        ],
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("missing_declaration"), "{output:?}");
    assert!(text.contains("extraction-clause-count"), "{output:?}");
    assert!(text.contains("\"actual\":2"), "{output:?}");
}

/// FSpec FR-563-AC-2 (FSpec-TC-1243): `check` without `--markdown` reads a
/// Markdown file as native source and refuses it.
#[test]
#[trace("FSpec-TC-1243", "FSpec-FR-563-AC-2")]
#[ignore = "QSL-2019: the `check --markdown` operand form is not implemented yet"]
fn check_without_markdown_reads_native_source() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("spec.md"), "# Spec\n\ntext\n").unwrap();
    let output = driver(
        directory.path(),
        &["check", "agent-ix", "doc", "spec.md", "--format", "json"],
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("invalid_syntax"));
    let markdown = driver(
        directory.path(),
        &[
            "check",
            "--markdown",
            "agent-ix",
            "doc",
            "spec.md",
            "--format",
            "json",
        ],
    );
    assert_eq!(markdown.status.code(), Some(0), "{markdown:?}");
}

/// FR-663-AC-2 (TC-1745): the `parsed` report is stable, and a token bound
/// equal to the count reached reports `used` equal to `bound`.
#[test]
#[trace("TC-1745", "FR-663-AC-2")]
#[ignore = "QSL-2191: the parse outcome's `parsed` result is not implemented yet"]
fn the_parsed_report_is_stable_and_reports_a_reached_bound() {
    let bytes = std::fs::read(SPINE_FIXTURE).unwrap();
    let identity = SourceIdentity::new("agent-ix", "test:parent");
    let report = |limits: Limits| -> Value {
        let result = parse(
            &ParseRequest {
                source: &identity,
                path: SPINE_FIXTURE,
                bytes: &bytes,
            },
            limits,
            &Cancel::new(),
        );
        let document: Value =
            serde_json::from_slice(&OutcomeDocument::from_parse(&result).to_bytes().unwrap())
                .unwrap();
        document["result"].clone()
    };
    let first = report(Limits::default());
    assert_eq!(first["kind"], "parsed", "{first}");
    assert_eq!(first, report(Limits::default()));
    let tokens: usize = first["usage"][1]["used"].as_str().unwrap().parse().unwrap();
    let bounded = report(Limits {
        tokens,
        ..Limits::default()
    });
    assert_eq!(bounded["usage"][1]["setting"], "s1.tokens");
    assert_eq!(bounded["usage"][1]["bound"], bounded["usage"][1]["used"]);
}

/// FR-663-AC-3 (TC-1746): a snapshot admission refusal carries a document
/// locus naming the object and member, and a positionless refusal a `null`
/// locus.
#[test]
#[trace("TC-1746", "FR-663-AC-3")]
#[ignore = "QSL-2191: refusal records do not carry a document locus yet"]
fn snapshot_refusals_carry_a_document_locus() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let snapshot_path = directory.path().join("spine-snapshot.json");
    let mut snapshot: Value =
        serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
    for object in snapshot["populations"][0]["objects"]
        .as_array_mut()
        .unwrap()
    {
        if object["key"] == "child" {
            object["fields"].as_object_mut().unwrap().remove("parent");
        }
    }
    std::fs::write(&snapshot_path, snapshot.to_string()).unwrap();
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&current_clause_request()).unwrap(),
    )
    .unwrap();
    let text = match fsl_command::command::run(&path) {
        Ok(result) => result.value.to_string(),
        Err(error) => error.value().unwrap().to_string(),
    };
    assert!(text.contains("missing-member"), "{text}");
    for fragment in ["\"locus\"", "\"member\":\"parent\"", "\"key\":\"child\""] {
        assert!(text.contains(fragment), "{fragment}: {text}");
    }
    let absent = fsl_command::command::run(&directory.path().join("absent.json"))
        .expect_err("an absent request refuses");
    let document: Value = serde_json::from_slice(
        &absent
            .outcome_document(fsl_replay::Operation::Execute)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    let diagnostic = &document["diagnostics"][0];
    assert!(
        diagnostic.get("locus").is_some_and(Value::is_null),
        "{document}"
    );
}

/// FR-664-AC-2 (TC-1747): FR-652's context and value-invariants selections
/// refuse a `parameters` member before any file is read.
#[test]
#[trace("TC-1747", "FR-664-AC-2")]
#[ignore = "QSL-1987: the context and value-invariants selections are not implemented yet"]
fn context_and_value_invariant_selections_refuse_parameters() {
    for selection in [
        json!({"context": {"name": "Consistent", "invocation": "i1", "side": "pre",
            "parameters": {}}}),
        json!({"value_invariants": {"snapshot": "s1", "parameters": {}}}),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut request = current_clause_request();
        request["request"]["clause"]["selection"] = selection;
        let (exit, value) = run_refusal(directory.path(), &request);
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(value.to_string().contains("parameters"), "{value}");
        assert!(!value.to_string().contains("unknown variant"), "{value}");
    }
}

/// FR-665-AC-2, FSpec FR-301-AC-5 (TC-1748, FSpec-TC-1244): `inspect
/// --handoff` verifies a written handoff and refuses an altered one.
#[test]
#[trace("TC-1748", "FR-665-AC-2", "FSpec-TC-1244", "FSpec-FR-301-AC-5")]
fn inspect_handoff_verifies_and_refuses_an_altered_copy() {
    let directory = tempfile::tempdir().unwrap();
    let path = handoff_request(
        directory.path(),
        json!({"directory": "handoff", "package_name": "spine",
            "native_join": {"work_units": 1_000_000, "storage": 1_000_000}}),
    );
    fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let verified = fsl_command::command::inspect_handoff(&directory.path().join("handoff"))
        .expect("a written handoff verifies");
    let document: Value = serde_json::from_slice(&verified.to_bytes().unwrap()).unwrap();
    assert_eq!(document["operation"], "inspect");
    assert_eq!(document["category"], "success");

    let selection_path = directory.path().join("handoff/selection.json");
    let mut selection: Value =
        serde_json::from_slice(&std::fs::read(&selection_path).unwrap()).unwrap();
    let node_counts = selection["package"]["node_counts"]
        .as_object_mut()
        .expect("the selection has node counts");
    let (form, count) = node_counts.iter_mut().next().expect("a counted node form");
    let form = form.clone();
    *count = (count.as_u64().unwrap() + 1).into();
    std::fs::write(&selection_path, serde_json::to_vec(&selection).unwrap()).unwrap();
    let mismatch = fsl_command::command::inspect_handoff(&directory.path().join("handoff"))
        .expect_err("an altered node count is refused");
    assert!(
        matches!(
            mismatch,
            fsl_command::checked_handoff::Error::SelectionMismatch(ref refusal)
                if refusal.axis == fsl_command::checked_handoff::SelectionAxis::PackageNodes
        ),
        "{form}: {mismatch:?}"
    );

    let package = directory.path().join("handoff/checked-package.json");
    // Restore the valid sidecar so the byte mutation reaches the strict reader.
    let mut selection: Value =
        serde_json::from_slice(&std::fs::read(&selection_path).unwrap()).unwrap();
    let count = selection["package"]["node_counts"][&form]
        .as_u64()
        .expect("the changed node count is present");
    selection["package"]["node_counts"][&form] = (count - 1).into();
    std::fs::write(&selection_path, serde_json::to_vec(&selection).unwrap()).unwrap();
    let mut bytes = std::fs::read(&package).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 1;
    std::fs::write(&package, bytes).unwrap();
    let refusal = fsl_command::command::inspect_handoff(&directory.path().join("handoff"))
        .expect_err("an altered package is refused");
    assert!(
        matches!(
            refusal,
            fsl_command::checked_handoff::Error::ReaderRefused(_)
        ),
        "{refusal:?}"
    );
}

fn provider_config(directory: &Path, document: &Value) {
    std::fs::write(directory.join("providers.json"), document.to_string()).unwrap();
}

/// FSpec FR-561-AC-1 (FSpec-TC-1241): an empty `providers` list runs with
/// the linked providers only.
#[test]
#[trace("FSpec-TC-1241", "FSpec-FR-561-AC-1")]
#[ignore = "QSL-2192: `--provider-config` is not read yet"]
fn an_empty_provider_configuration_is_read() {
    let directory = tempfile::tempdir().unwrap();
    provider_config(
        directory.path(),
        &json!({"format": "filament.provider-config/v1", "providers": []}),
    );
    handoff_request(directory.path(), json!(null));
    let output = driver(
        directory.path(),
        &[
            "prove",
            "--provider-config",
            "providers.json",
            "--format",
            "json",
            "compile.json",
        ],
    );
    let text = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains("missing field"), "{output:?}");
    assert!(!text.contains("providers"), "{output:?}");
    assert_ne!(output.status.code(), Some(20), "{output:?}");
}

/// FSpec FR-561-AC-2 (FSpec-TC-1241): a provider configuration without
/// `providers`, or with a malformed entry or format, refuses with exit 20.
#[test]
#[trace("FSpec-TC-1241", "FSpec-FR-561-AC-2")]
#[trace("FSpec-TC-1244", "FSpec-FR-301-AC-5")]
#[ignore = "QSL-2192: `--provider-config` is not read yet"]
fn malformed_provider_configurations_refuse() {
    for (document, named) in [
        (
            json!({"format": "filament.provider-config/v1"}),
            "providers",
        ),
        (
            json!({"format": "filament.provider-config/v1",
                "providers": [{"path": "bin/p", "env": {}}]}),
            "env",
        ),
        (
            json!({"format": "filament.provider-config/v1", "providers": [{}]}),
            "path",
        ),
        (
            json!({"format": "filament.provider-config/v2", "providers": []}),
            "unsupported-wire",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        provider_config(directory.path(), &document);
        let output = driver(
            directory.path(),
            &[
                "prove",
                "--provider-config",
                "providers.json",
                "--format",
                "json",
                "compile.json",
            ],
        );
        assert_eq!(output.status.code(), Some(20), "{named}: {output:?}");
        let text = String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout);
        assert!(text.contains(named), "{named}: {output:?}");
        assert!(text.contains("providers.json"), "{named}: {output:?}");
    }
}

/// FSpec FR-561-AC-3 (FSpec-TC-1241): a verb that routes no items refuses
/// `--provider-config` as an unknown option.
#[test]
#[trace("FSpec-TC-1241", "FSpec-FR-561-AC-3")]
#[ignore = "QSL-2192: `--provider-config` is not specified per verb yet"]
fn parse_refuses_a_provider_configuration_option() {
    let directory = tempfile::tempdir().unwrap();
    provider_config(
        directory.path(),
        &json!({"format": "filament.provider-config/v1", "providers": []}),
    );
    std::fs::copy(SPINE_FIXTURE, directory.path().join("unit.native")).unwrap();
    let output = driver(
        directory.path(),
        &[
            "parse",
            "--provider-config",
            "providers.json",
            "agent-ix",
            "unit",
            "unit.native",
        ],
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("provider-config"));
}

fn extension_set(directory: &Path, entries: Value) {
    std::fs::write(
        directory.join("set.json"),
        json!({"format": "filament.extension-set/v1", "extensions": entries}).to_string(),
    )
    .unwrap();
}

/// FSpec FR-562-AC-1 (FSpec-TC-1242): `check --extensions` supplies the
/// definition a unit selects.
#[test]
#[trace("FSpec-TC-1242", "FSpec-FR-562-AC-1")]
#[ignore = "QSL-2189: `--extensions` is not implemented yet"]
fn check_with_an_extension_set_admits_the_selected_form() {
    let directory = tempfile::tempdir().unwrap();
    retention_definition(directory.path(), "retention.json", "retention");
    extension_set(
        directory.path(),
        json!([{"authority": "agent-ix", "identity": "ext.retention", "file": "retention.json"}]),
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{HEADER}profile r = \"ext.retention\";\n{FORM}\n"),
    )
    .unwrap();
    let with = driver(
        directory.path(),
        &[
            "check",
            "--extensions",
            "set.json",
            "agent-ix",
            "unit",
            "unit.native",
        ],
    );
    assert_eq!(with.status.code(), Some(0), "{with:?}");
    let without = driver(
        directory.path(),
        &["check", "agent-ix", "unit", "unit.native"],
    );
    assert_eq!(without.status.code(), Some(20), "{without:?}");
    assert!(String::from_utf8_lossy(&without.stdout).contains("unknown_profile"));
}

/// FSpec FR-562-AC-2, FR-301-AC-5 (FSpec-TC-1242, FSpec-TC-1244): malformed
/// extension sets refuse with exit 20.
#[test]
#[trace(
    "FSpec-TC-1242",
    "FSpec-FR-562-AC-2",
    "FSpec-TC-1244",
    "FSpec-FR-301-AC-5"
)]
#[ignore = "QSL-2189: `--extensions` is not implemented yet"]
fn malformed_extension_sets_refuse() {
    let entry =
        json!({"authority": "agent-ix", "identity": "ext.retention", "file": "retention.json"});
    for (entries, code) in [
        (
            json!([{"authority": "agent-ix", "identity": "ext.retention",
                "file": "retention.json", "version": "1"}]),
            "invalid-request",
        ),
        (
            json!([{"authority": "agent-ix", "identity": "ext.retention"}]),
            "invalid-request",
        ),
        (json!([entry.clone(), entry]), "conflicting-definition"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        retention_definition(directory.path(), "retention.json", "retention");
        extension_set(directory.path(), entries);
        std::fs::copy(SPINE_FIXTURE, directory.path().join("unit.native")).unwrap();
        let output = driver(
            directory.path(),
            &[
                "check",
                "--extensions",
                "set.json",
                "agent-ix",
                "unit",
                "unit.native",
            ],
        );
        assert_eq!(output.status.code(), Some(20), "{code}: {output:?}");
        let text = String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout);
        assert!(text.contains(code), "{code}: {output:?}");
    }
}
