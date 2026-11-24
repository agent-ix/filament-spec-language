// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-6 to AC-9 (TC-2998): a model unit reaches every command that
//! takes a model, through a request's `models` member, `check`'s model
//! selections and a compile request that writes the compiled document.

use fsl_command::command::{self, CheckInputs};
use fsl_forms::cst::{self, CompleteCause, CompleteCode};
use fsl_foundation::SourceIdentity;
use fsl_replay::Operation;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{compile_model_unit, package_input, package_input_from_sources};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// FR-710-AC-1's model unit.
const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total;
}
population orders: Order;
";

/// FR-710-AC-4's model unit with an unresolved relationship target.
const GHOST: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order { relationship next: Ghost [0, 1]; }
";

#[test]
#[trace("FR-081-AC-1", "FR-081-AC-3", "FR-081-AC-6", "FR-710-AC-7")]
fn original_effective_identity_reaches_native_requests() {
    let directory = tempfile::tempdir().unwrap();
    let model = include_str!("../../../examples/model-identity/model.fsl");
    let program = include_str!("../../../examples/model-identity/program.fsl");
    for (file, text) in [
        ("model.fsl", model),
        ("program.fsl", program),
        (
            "current.json",
            include_str!("../../../examples/model-identity/current.json"),
        ),
        (
            "run.json",
            include_str!("../../../examples/model-identity/run.json"),
        ),
    ] {
        std::fs::write(directory.path().join(file), text).unwrap();
    }
    let native_entry = json!([{"format": "native-model-unit/1", "source": source("model.fsl")}]);
    let path = compile_request(directory.path(), &[], "program.fsl", native_entry, &[]);
    let native =
        command::compile_outcome(&path).expect("source inheritance and redefinition compile");
    let model_source = fsl_cst::diagnostic::read_source(
        SourceIdentity::new("agent-ix", "test:model.fsl"),
        "model.fsl",
        model.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .unwrap();
    let packages = package_input_from_sources([("example/identity", &model_source)]).unwrap();
    std::fs::write(
        directory.path().join("model.json"),
        &packages["example/identity"],
    )
    .unwrap();
    let document_entry = json!([{"format": "semantic-ir/2.0.0", "source": source("model.json")}]);
    let path = compile_request(
        directory.path(),
        &[],
        "program.fsl",
        document_entry.clone(),
        &[],
    );
    let document = command::compile_outcome(&path).expect("same-origin document compiles");
    assert_eq!(native.package_bytes(), document.package_bytes());

    let run_path = directory.path().join("run.json");
    let native_run = command::run(&run_path, &crate::negotiation::Fr290)
        .expect("the authored correspondence example executes");
    assert_eq!(native_run.exit_code, 0, "{:?}", native_run.value);
    let expected = json!({"kind": "boolean", "value": true});
    assert_eq!(
        native_run.value.as_value()["disposition"]["outcome"]["value"],
        expected
    );
    let mut request: Value = serde_json::from_slice(&std::fs::read(&run_path).unwrap()).unwrap();
    request["request"]["models"] = document_entry;
    std::fs::write(&run_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let document_run = command::run(&run_path, &crate::negotiation::Fr290).unwrap();
    assert_eq!(document_run.exit_code, 0);
    assert_eq!(
        document_run.value.as_value()["disposition"]["outcome"]["value"],
        expected
    );

    // Different data must change the actual source-request verdict.
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../examples/model-identity/current.json"
    ))
    .unwrap();
    snapshot["populations"][0]["objects"][0]["fields"]["field"]["integer"] = json!("8");
    std::fs::write(
        directory.path().join("current.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
    request["request"]["models"] =
        json!([{"format": "native-model-unit/1", "source": source("model.fsl")}]);
    std::fs::write(&run_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let violated = command::run(&run_path, &crate::negotiation::Fr290).unwrap();
    assert_eq!(violated.exit_code, 10);
    assert_eq!(
        violated.value.as_value()["disposition"]["outcome"]["value"],
        json!({"kind": "boolean", "value": false})
    );

    let conflict = "language \"ix:native\" edition \"1-draft\";
        domain \"example/identity\";
        object Base { inherited: Integer; field: Integer; }
        object Mid extends Base { field: Integer redefines Base::field; }
        object Other extends Base { field: Integer redefines Base::field; }
        object Leaf extends Mid, Other {}
        population leaves: Leaf;";
    std::fs::write(directory.path().join("model.fsl"), conflict).unwrap();
    let refused = command::run(&run_path, &crate::negotiation::Fr290)
        .expect("a clause refusal is a completed report");
    assert_eq!(refused.exit_code, 20, "{:?}", refused.value);
    assert!(refused.value.as_value()["disposition"]["outcome"]["value"].is_null());
    let path = compile_request(
        directory.path(),
        &[],
        "program.fsl",
        json!([{"format": "native-model-unit/1", "source": source("model.fsl")}]),
        &[],
    );
    let error = command::compile_outcome(&path)
        .err()
        .expect("sibling redefiners expose no executable view");
    let outcome: Value = serde_json::from_slice(
        &error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(error.category().exit_code(), 20, "{outcome}");
    assert_eq!(outcome["last_stage"], "I1", "{outcome}");
    assert_eq!(
        outcome["diagnostics"][0]["code"], "invalid_model_binding",
        "{outcome}"
    );
    assert_eq!(
        outcome["diagnostics"][0]["cause"], "derivation-conflict",
        "{outcome}"
    );
}

const SYSTEMS_MODEL: &str = r#"language "ix:native" edition "1-draft";
domain "test/plant";
object Sys { }
object Pump { }
object Tank { }
interface Flow { rate: Integer; }
part SysPump: Pump [1, 1] in Sys;
part SysTank: Tank [1, 1] in Sys;
port PumpOut: Flow [1, 1] out on SysPump;
port TankIn: Flow [1, 1] in on SysTank;
connection Pipe SysPump.PumpOut -> SysTank.TankIn;
"#;

const SYSTEMS_PROTOCOL: &str = r#"language "ix:native" edition "1-draft";
profile p = "filament.protocol.finite-global/v1";
model N = "test/plant";
protocol Pipeline using p over (input: N::Sys) on origin {
  role Outlet on N::PumpOut;
  role Inlet on N::TankIn;
  relationship Link = N::Pipe;
  channel Feed from Outlet to Inlet via Link carries N::Flow
    ordering unordered delivery exactly-once-premise capacity 2 overflow reject;
  run sequence Main { }
  finish Closed as (closed: Boolean) { true };
}
"#;

fn systems_request(directory: &Path, model: &str, protocol: &str) -> PathBuf {
    compile_request(
        directory,
        &[("plant.fsl", model), ("pipeline.fsl", protocol)],
        "pipeline.fsl",
        json!([{"format": "native-model-unit/1", "source": source("plant.fsl")}]),
        &[],
    )
}

#[test]
#[trace("TC-3013", "FR-710-AC-11", "FR-897-AC-1")]
fn systems_source_compiles_to_the_same_checked_package_as_its_document() {
    for (model, direction) in [
        (SYSTEMS_MODEL.to_owned(), "source-to-target"),
        (
            SYSTEMS_MODEL
                .replace(" -> ", " <-> ")
                .replace(" out on", " inout on")
                .replace(" in on", " inout on"),
            "bidirectional",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = systems_request(directory.path(), &model, SYSTEMS_PROTOCOL);
        let from_source =
            command::compile_outcome(&path).expect("source model and protocol compile");
        let model_source = fsl_cst::diagnostic::read_source(
            SourceIdentity::new("agent-ix", "test:plant.fsl"),
            "plant.fsl",
            model.as_bytes(),
            fsl_cst::Limits::default().source_bytes,
        )
        .unwrap();
        let packages = package_input_from_sources([("test/plant", &model_source)]).unwrap();
        let document = &packages["test/plant"];
        let wire: Value = serde_json::from_slice(document).unwrap();
        let node = |name: &str| {
            wire["types"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["identity"] == format!("ix://test/plant/{name}"))
                .unwrap()
        };
        let one = json!({"lower":1,"upper":1,"ordered":false,"unique":true});
        for (part, kind) in [("SysPump", "Pump"), ("SysTank", "Tank")] {
            assert_eq!(node(part)["owner"], "ix://test/plant/Sys");
            assert_eq!(
                node(part)["declaredType"],
                format!("ix://test/plant/{kind}")
            );
            assert_eq!(node(part)["multiplicity"], one);
        }
        for (port, part, directed) in [("PumpOut", "SysPump", "out"), ("TankIn", "SysTank", "in")] {
            assert_eq!(node(port)["owner"], format!("ix://test/plant/{part}"));
            assert_eq!(node(port)["interfaceType"], "ix://test/plant/Flow");
            assert_eq!(
                node(port)["direction"],
                if direction == "bidirectional" {
                    "inout"
                } else {
                    directed
                }
            );
            assert_eq!(node(port)["multiplicity"], one);
            assert_eq!(
                node(port)["origin"]["source"]["sourceIdentity"],
                "ix://agent-ix/test:plant.fsl"
            );
        }
        assert_eq!(node("Pipe")["flowDirection"], direction);
        for (end, port) in [("sourceEnd", "PumpOut"), ("targetEnd", "TankIn")] {
            assert_eq!(node("Pipe")[end]["type"], format!("ix://test/plant/{port}"));
            assert_eq!(node("Pipe")[end]["multiplicity"], one);
        }
        // Read the source-produced document through real intake, not just its JSON shape.
        let (_, admitted) = fsl_semantics::model::intake::admit(
            "test/plant",
            &packages,
            fsl_foundation::IntakeLimits::default(),
        )
        .unwrap();
        let records = fsl_semantics::model::intake::read_records("test/plant", &admitted).unwrap();
        assert_eq!(
            records
                .iter()
                .filter(|r| matches!(
                    r,
                    fsl_semantics::model::domain_package::DomainPackageRecord::Component(_)
                ))
                .count(),
            2
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| matches!(
                    r,
                    fsl_semantics::model::domain_package::DomainPackageRecord::Endpoint(_)
                ))
                .count(),
            2
        );
        std::fs::write(directory.path().join("plant.json"), document).unwrap();
        let path = compile_request(
            directory.path(),
            &[],
            "pipeline.fsl",
            json!([{"format":"semantic-ir/2.0.0", "source":source("plant.json")}]),
            &[],
        );
        let from_document =
            command::compile_outcome(&path).expect("Semantic IR model and protocol compile");
        assert_eq!(from_source.package_bytes(), from_document.package_bytes());
        let checked: Value = serde_json::from_slice(from_source.package_bytes()).unwrap();
        let nodes = checked["semantic_graph"]["nodes"].as_array().unwrap();
        let channel = nodes
            .iter()
            .find(|node| node["semantic_form"] == "channel")
            .expect("channel emitted");
        let connection = channel["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|argument| argument["name"] == "connection")
            .expect("via emitted");
        let target = &connection["value"]["target"];
        assert!(!target.is_null(), "{connection}");
        assert_eq!(target["package"], "test/plant");
        assert_eq!(
            target["path"],
            json!(["relation", "relationship", "ix://test/plant/Pipe"])
        );
        assert!(nodes.iter().any(|node| &node["node_id"] == target));
        assert!(channel["dependencies"].as_array().unwrap().contains(target));
        let clause = nodes
            .iter()
            .find(|node| node["semantic_form"] == "protocol_clause")
            .expect("protocol emitted");
        let relationships = clause["body"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|argument| argument["name"] == "relationships")
            .unwrap();
        assert!(relationships["value"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .any(|term| &term["target"] == target));
        let mut ports = Vec::new();
        for role in nodes.iter().filter(|node| node["semantic_form"] == "role") {
            let member = role["body"]["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .find(|argument| argument["name"] == "model_member")
                .expect("Port role retains member");
            let fields = member["value"]["members"].as_array().unwrap();
            let field = |name: &str| {
                &fields.iter().find(|field| field["name"] == name).unwrap()["value"]["value"]
            };
            assert_eq!(field("package"), "test/plant");
            ports.push(field("node").as_str().unwrap());
        }
        ports.sort_unstable();
        assert_eq!(ports, ["ix://test/plant/PumpOut", "ix://test/plant/TankIn"]);
    }
}

#[test]
#[trace("TC-3013", "FR-710-AC-12")]
fn systems_source_rejects_each_invalid_endpoint_before_writing_a_document() {
    for (original, changed, offending, code, cause, retained) in [
        (
            "SysPump.PumpOut",
            "Absent.PumpOut",
            "Absent",
            CompleteCode::MissingDeclaration,
            CompleteCause::MissingDeclaration,
            vec!["ix://test/plant/Absent"],
        ),
        (
            "SysTank.TankIn",
            "Absent.TankIn",
            "Absent",
            CompleteCode::MissingDeclaration,
            CompleteCause::MissingDeclaration,
            vec!["ix://test/plant/Absent"],
        ),
        (
            "SysPump.PumpOut",
            "SysPump.Absent",
            "Absent",
            CompleteCode::MissingDeclaration,
            CompleteCause::MissingDeclaration,
            vec!["ix://test/plant/Absent"],
        ),
        (
            "SysTank.TankIn",
            "SysTank.Absent",
            "Absent",
            CompleteCode::MissingDeclaration,
            CompleteCause::MissingDeclaration,
            vec!["ix://test/plant/Absent"],
        ),
        (
            "SysPump.PumpOut",
            "Sys.PumpOut",
            "Sys",
            CompleteCode::InvalidModelBinding,
            CompleteCause::WrongExport,
            vec!["part", "object", "ix://test/plant/Sys"],
        ),
        (
            "SysTank.TankIn",
            "Sys.TankIn",
            "Sys",
            CompleteCode::InvalidModelBinding,
            CompleteCause::WrongExport,
            vec!["part", "object", "ix://test/plant/Sys"],
        ),
        (
            "SysPump.PumpOut",
            "SysPump.SysTank",
            "SysTank",
            CompleteCode::InvalidModelBinding,
            CompleteCause::WrongExport,
            vec!["port", "part", "ix://test/plant/SysTank"],
        ),
        (
            "SysTank.TankIn",
            "SysTank.SysPump",
            "SysPump",
            CompleteCode::InvalidModelBinding,
            CompleteCause::WrongExport,
            vec!["port", "part", "ix://test/plant/SysPump"],
        ),
        (
            "SysPump.PumpOut",
            "SysTank.PumpOut",
            "SysTank",
            CompleteCode::InvalidModelBinding,
            CompleteCause::MalformedDeclaration,
            vec![
                "ix://test/plant/PumpOut",
                "ix://test/plant/SysPump",
                "ix://test/plant/SysTank",
            ],
        ),
        (
            "SysTank.TankIn",
            "SysPump.TankIn",
            "SysPump",
            CompleteCode::InvalidModelBinding,
            CompleteCause::MalformedDeclaration,
            vec![
                "ix://test/plant/TankIn",
                "ix://test/plant/SysPump",
                "ix://test/plant/SysTank",
            ],
        ),
    ] {
        let model = SYSTEMS_MODEL.replace(original, changed);
        let parsed = cst::parse(
            SourceIdentity::new("agent-ix", "test:plant.fsl"),
            "plant.fsl",
            model.as_bytes(),
            cst::Limits::default(),
        )
        .unwrap();
        let errors = fsl_semantics::model::intake::compile_model_unit_with_causes(
            &parsed,
            ModelNormalizationLimits::default(),
        )
        .expect_err("invalid endpoint produces no document");
        let [error] = errors.as_slice() else {
            panic!("{errors:?}")
        };
        assert_eq!(error.code, code, "{changed}: {error:?}");
        assert_eq!(error.cause(), cause, "{changed}: {error:?}");
        let start = model.find("connection Pipe ").unwrap()
            + "connection Pipe ".len()
            + model[model.find("connection Pipe ").unwrap() + "connection Pipe ".len()..]
                .find(changed)
                .unwrap()
            + changed.find(offending).unwrap();
        assert_eq!(
            error.byte_span(),
            Some(fsl_foundation::Span {
                start,
                end: start + offending.len()
            })
        );
        assert_eq!(
            error.source_identity(),
            &SourceIdentity::new("agent-ix", "test:plant.fsl")
        );
        assert_eq!(error.source_path(), "plant.fsl");
        for detail in retained {
            assert!(error.message.contains(detail), "{detail}: {error:?}");
        }
    }
    for (declaration, kind) in [
        ("population Aux: Pump;", "population"),
        ("dimension Aux;", "dimension"),
        (
            "dimension Length; unit Aux: Length = rational(1, 1);",
            "unit",
        ),
    ] {
        for (original, changed, required) in [
            ("SysPump.PumpOut", "Aux.PumpOut", "part"),
            ("SysTank.TankIn", "Aux.TankIn", "part"),
            ("SysPump.PumpOut", "SysPump.Aux", "port"),
            ("SysTank.TankIn", "SysTank.Aux", "port"),
        ] {
            let model = SYSTEMS_MODEL
                .replace(
                    "connection Pipe",
                    &format!("{declaration}\nconnection Pipe"),
                )
                .replace(original, changed);
            let parsed = cst::parse(
                SourceIdentity::new("agent-ix", "test:plant.fsl"),
                "plant.fsl",
                model.as_bytes(),
                cst::Limits::default(),
            )
            .unwrap();
            let errors = fsl_semantics::model::intake::compile_model_unit_with_causes(
                &parsed,
                ModelNormalizationLimits::default(),
            )
            .expect_err("a declared non-system name has the wrong kind");
            let [error] = errors.as_slice() else {
                panic!("{errors:?}")
            };
            assert_eq!(error.code, CompleteCode::InvalidModelBinding, "{error:?}");
            assert_eq!(error.cause(), CompleteCause::WrongExport, "{error:?}");
            assert!(
                error
                    .message
                    .contains(&format!("requires {required}, found {kind}")),
                "{error:?}"
            );
            let start = model.rfind("Aux").unwrap();
            assert_eq!(
                error.byte_span(),
                Some(fsl_foundation::Span {
                    start,
                    end: start + 3
                })
            );
        }
    }
}

#[test]
#[trace("TC-3013", "FR-897-AC-3", "FR-897-AC-5")]
fn systems_source_refuses_reversed_channel_and_non_port_roles() {
    let mut variants = vec![(
        SYSTEMS_PROTOCOL.replace("from Outlet to Inlet", "from Inlet to Outlet"),
        "Link".to_owned(),
    )];
    for role in ["PumpOut", "TankIn"] {
        for member in ["SysPump", "Pipe"] {
            variants.push((
                SYSTEMS_PROTOCOL.replace(&format!("on N::{role};"), &format!("on N::{member};")),
                format!("N::{member}"),
            ));
        }
    }
    let directory = tempfile::tempdir().unwrap();
    for (protocol, located) in variants {
        let path = systems_request(directory.path(), SYSTEMS_MODEL, &protocol);
        let error = command::compile_outcome(&path)
            .err()
            .expect("invalid channel binding refuses");
        let artifact = error.artifact_value().unwrap();
        assert_eq!(artifact["code"], "invalid_model_binding", "{artifact}");
        assert_eq!(artifact["cause"], "malformed-declaration", "{artifact}");
        let start = if located == "Link" {
            protocol.find("via Link").unwrap() + 4
        } else {
            protocol.find(&located).unwrap()
        };
        assert_eq!(
            artifact["details"]["span"]["start"]["byte"], start,
            "{artifact}"
        );
        assert_eq!(
            artifact["details"]["span"]["end"]["byte"],
            start + located.len(),
            "{artifact}"
        );
        assert_eq!(
            artifact["details"]["source"]["identity"],
            "test:pipeline.fsl"
        );
    }
}

#[test]
#[trace("TC-3013", "FR-086-AC-2")]
fn systems_source_preserves_independent_connection_admission_causes() {
    let directory = tempfile::tempdir().unwrap();
    for (model, code, cause) in [
        (
            SYSTEMS_MODEL.replace("out on SysPump", "in on SysPump"),
            "invalid_model_binding",
            "port-direction",
        ),
        (
            SYSTEMS_MODEL
                .replace(
                    "interface Flow { rate: Integer; }",
                    "interface Flow { rate: Integer; }\ninterface Other { marker: Boolean; }",
                )
                .replace("TankIn: Flow", "TankIn: Other"),
            "ill_typed",
            "type-mismatch",
        ),
        (
            SYSTEMS_MODEL.replace("PumpOut: Flow [1, 1]", "PumpOut: Flow [2, 2]"),
            "ill_typed",
            "multiplicity-narrowing",
        ),
    ] {
        let path = systems_request(directory.path(), &model, SYSTEMS_PROTOCOL);
        let error = command::compile_outcome(&path)
            .err()
            .expect("connection admission refuses");
        let artifact = error.artifact_value().unwrap();
        assert_eq!(artifact["code"], code, "{artifact}");
        assert_eq!(artifact["cause"], cause, "{artifact}");
        assert!(
            artifact.to_string().contains("ix://test/plant/Pipe"),
            "{artifact}"
        );
        let start = SYSTEMS_PROTOCOL.find("N::Pipe").unwrap();
        assert_eq!(
            artifact["details"]["span"]["start"]["byte"], start,
            "{artifact}"
        );
        assert_eq!(
            artifact["details"]["span"]["end"]["byte"],
            start + "N::Pipe".len(),
            "{artifact}"
        );
    }
}

/// A program selecting `identity` as `M`, with one invariant over `M::Order`.
fn program(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         model M = \"{identity}\";\n\
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n\
         function one using P(): Int[1, 1] pure {{ 1 }}\n"
    )
}

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:{file}"),
        "document": "ModelUnit", "formal_revision": 1})
}

/// Write `files` into `directory` and a `native-compile/1` request over
/// `program` and `models`, returning the request path.
fn compile_request(
    directory: &Path,
    files: &[(&str, &str)],
    program_file: &str,
    models: Value,
    extra: &[(&str, Value)],
) -> PathBuf {
    for (name, text) in files {
        std::fs::write(directory.join(name), text).unwrap();
    }
    let mut request = json!({"program": {"source": source(program_file)}});
    if !models.is_null() {
        request["models"] = models;
    }
    for (member, value) in extra {
        request[*member] = value.clone();
    }
    let path = directory.join("compile.json");
    let envelope = json!({"format": "native-compile/1", "request": request});
    std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    path
}

fn run_request(
    directory: &Path,
    files: &[(&str, &str)],
    program_file: &str,
    models: Value,
) -> PathBuf {
    for (name, text) in files {
        std::fs::write(directory.join(name), text).unwrap();
    }
    let request = json!({"format": "native-run/1", "request": {
        "models": models,
        "program": {"source": source(program_file)},
        "call": {"function": "one", "arguments": [], "accounting": {"work_units": 1000}}
    }});
    let path = directory.join("run.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn refusal(error: &command::RunError) -> Value {
    error.value().expect("refusal envelope")
}

fn check_document(outcome: &command::CheckOutcome) -> Value {
    serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap()
}

fn write_route_request(directory: &Path, name: &str, format: &str, request: Value) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": format, "request": request})).unwrap(),
    )
    .unwrap();
    path
}

fn format_cases(directory: &Path) -> [(String, String, &'static str); 2] {
    let compiled = package_input([SHOP.as_bytes()]).expect("the model unit compiles");
    let json_path = directory.join("model.semantic-ir.json");
    std::fs::write(&json_path, &compiled["example/shop"]).unwrap();
    let native_path = directory.join("model.native");
    std::fs::write(&native_path, SHOP).unwrap();
    [
        (
            "native-model-unit/1".to_owned(),
            "model.semantic-ir.json".to_owned(),
            "native-model-unit/1 requires a model unit",
        ),
        (
            "semantic-ir/2.0.0".to_owned(),
            "model.native".to_owned(),
            "selected as a semantic-ir/2.0.0 document",
        ),
    ]
}

fn assert_selected_model_format_refusal(
    error: &command::RunError,
    operation: Operation,
    file: &str,
    reason: &str,
) {
    let outcome: Value = serde_json::from_slice(
        &error
            .outcome_document(operation)
            .to_bytes()
            .expect("model format refusal outcome serializes"),
    )
    .expect("model format refusal outcome is JSON");
    assert_eq!(outcome["last_stage"], "I1", "{outcome}");
    let diagnostic = &outcome["diagnostics"][0];
    assert_eq!(diagnostic["code"], "invalid_model_binding", "{outcome}");
    assert_eq!(diagnostic["cause"], "malformed-declaration", "{outcome}");
    assert_eq!(diagnostic["locus"]["kind"], "region", "{outcome}");
    assert_eq!(
        diagnostic["locus"]["source"]["identity"],
        format!("test:{file}"),
        "{outcome}"
    );
    assert!(
        diagnostic["message"].as_str().unwrap().contains(reason),
        "{outcome}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-6")]
fn a_package_input_compiles_a_model_unit_under_its_domain_identity() {
    let packages = package_input([SHOP.as_bytes()]).expect("a model unit is a package input");
    assert!(
        packages.contains_key("example/shop"),
        "{:?}",
        packages.keys()
    );
    let document: Value =
        serde_json::from_slice(&packages["example/shop"]).expect("the compiled document");
    assert_eq!(document["contractVersion"], "2.0.0", "{document}");
    let conflict = package_input([SHOP.as_bytes(), packages["example/shop"].as_slice()])
        .expect_err("a model unit and its document collide");
    assert!(
        format!("{conflict:?}").contains("ConflictingPackage"),
        "{conflict:?}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-7")]
fn a_native_model_unit_models_entry_compiles_like_its_document() {
    let directory = tempfile::tempdir().unwrap();
    let unit_entry = json!([{"format": "native-model-unit/1", "source": source("shop.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[
            ("shop.fsl", SHOP),
            ("program.fsl", &program("example/shop")),
        ],
        "program.fsl",
        unit_entry.clone(),
        &[],
    );
    let from_unit = command::compile_outcome(&path).expect("the model unit route compiles");
    let model_source = fsl_cst::diagnostic::read_source(
        SourceIdentity::new("agent-ix", "test:shop.fsl"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the model source metadata is admitted");
    let document = package_input_from_sources([("example/shop", &model_source)])
        .expect("the source compiles to its Semantic IR document")["example/shop"]
        .clone();
    std::fs::write(directory.path().join("shop.semantic-ir.json"), &document).unwrap();
    let document_entry =
        json!([{"format": "semantic-ir/2.0.0", "source": source("shop.semantic-ir.json")}]);
    let path = compile_request(
        directory.path(),
        &[],
        "program.fsl",
        document_entry.clone(),
        &[],
    );
    let from_document = command::compile_outcome(&path).expect("the document route compiles");
    assert_eq!(from_unit.package_bytes(), from_document.package_bytes());

    let unit_run = run_request(
        directory.path(),
        &[
            ("shop.fsl", SHOP),
            ("program.fsl", &program("example/shop")),
        ],
        "program.fsl",
        unit_entry,
    );
    let from_unit = command::run(&unit_run, &crate::negotiation::Fr290)
        .expect("the native model-unit run completes");
    let document_run = run_request(directory.path(), &[], "program.fsl", document_entry.clone());
    let from_document = command::run(&document_run, &crate::negotiation::Fr290)
        .expect("the semantic document run completes");
    assert_eq!(
        from_unit.value.as_value()["outcome"],
        from_document.value.as_value()["outcome"]
    );

    let ghost_entry = json!([{"format": "native-model-unit/1", "source": source("ghost.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[("ghost.fsl", GHOST)],
        "program.fsl",
        ghost_entry,
        &[],
    );
    let ghost_error = command::compile_outcome(&path)
        .err()
        .expect("Ghost refuses");
    let ghost = refusal(&ghost_error);
    let ghost_outcome: Value = serde_json::from_slice(
        &ghost_error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ghost["code"], "missing_declaration", "{ghost}");
    assert_eq!(ghost_outcome["diagnostics"][0]["cause"], "missing-name");
    assert_eq!(ghost_outcome["last_stage"], "I1");
    assert!(
        ghost_outcome["diagnostics"][0]["locus"]["source"]["identity"]
            .as_str()
            .is_some_and(|identity| identity.ends_with("ghost.fsl")),
        "{ghost_outcome}"
    );

    let program_entry = json!([{"format": "native-model-unit/1", "source": source("program.fsl")}]);
    let path = compile_request(directory.path(), &[], "program.fsl", program_entry, &[]);
    let not_a_unit_error = command::compile_outcome(&path).err().expect("no domain");
    let not_a_unit = refusal(&not_a_unit_error);
    assert_eq!(not_a_unit["code"], "invalid_model_binding", "{not_a_unit}");
    assert!(
        not_a_unit.to_string().contains("native-model-unit/1"),
        "{not_a_unit}"
    );
    let not_a_unit_outcome: Value = serde_json::from_slice(
        &not_a_unit_error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        not_a_unit_outcome["last_stage"], "I1",
        "{not_a_unit_outcome}"
    );
    assert_eq!(
        not_a_unit_outcome["diagnostics"][0]["cause"], "malformed-declaration",
        "{not_a_unit_outcome}"
    );

    let wrong_format = json!([{"format": "semantic-ir/2.0.0", "source": source("shop.fsl")}]);
    let path = compile_request(
        directory.path(),
        &[("shop.fsl", SHOP)],
        "program.fsl",
        wrong_format,
        &[],
    );
    let malformed_error = command::compile_outcome(&path)
        .err()
        .expect("a model unit is not a semantic-ir document");
    let malformed = refusal(&malformed_error);
    assert_eq!(malformed["code"], "invalid_model_binding", "{malformed}");
    let malformed_outcome: Value = serde_json::from_slice(
        &malformed_error
            .outcome_document(Operation::Package)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        malformed_outcome["diagnostics"][0]["cause"],
        "malformed-declaration"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-8")]
fn check_takes_a_model_unit_as_a_selection_and_as_its_source() {
    let directory = tempfile::tempdir().unwrap();
    let shop = directory.path().join("shop.fsl");
    std::fs::write(&shop, SHOP).unwrap();
    let rule = directory.path().join("rule.fsl");
    std::fs::write(&rule, program("example/shop")).unwrap();
    let inputs = CheckInputs {
        models: vec![("example/shop".to_owned(), shop.clone())],
        ..CheckInputs::default()
    };
    let checked = check_document(&command::check_outcome(&rule, &inputs));
    assert_eq!(checked["category"], "success", "{checked}");
    assert_eq!(checked["last_stage"], "S4", "{checked}");
    let document = package_input([SHOP.as_bytes()]).expect("the compiled document");
    let document_path = directory.path().join("shop.semantic-ir.json");
    std::fs::write(&document_path, &document["example/shop"]).unwrap();
    let document_inputs = CheckInputs {
        models: vec![("example/shop".to_owned(), document_path)],
        ..CheckInputs::default()
    };
    let checked_document = check_document(&command::check_outcome(&rule, &document_inputs));
    assert_eq!(checked["category"], checked_document["category"]);
    assert_eq!(checked["diagnostics"], checked_document["diagnostics"]);
    assert_eq!(checked["last_stage"], checked_document["last_stage"]);

    let other_rule = directory.path().join("other-rule.fsl");
    std::fs::write(&other_rule, program("example/other")).unwrap();
    let other_inputs = CheckInputs {
        models: vec![("example/other".to_owned(), shop.clone())],
        ..CheckInputs::default()
    };
    let wrong_selection = check_document(&command::check_outcome(&other_rule, &other_inputs));
    assert_eq!(wrong_selection["category"], "refusal", "{wrong_selection}");
    assert_eq!(
        wrong_selection["diagnostics"][0]["code"], "invalid_model_binding",
        "{wrong_selection}"
    );
    assert_eq!(
        wrong_selection["diagnostics"][0]["cause"], "wrong-model-selection",
        "{wrong_selection}"
    );

    let alone = check_document(&command::check_outcome(&shop, &CheckInputs::default()));
    assert_eq!(alone["category"], "success", "{alone}");
    assert_eq!(alone["last_stage"], "I1", "{alone}");

    let ghost = directory.path().join("ghost.fsl");
    std::fs::write(&ghost, GHOST).unwrap();
    let refused = check_document(&command::check_outcome(&ghost, &CheckInputs::default()));
    assert_eq!(
        refused["diagnostics"][0]["code"], "missing_declaration",
        "{refused}"
    );
    assert_eq!(
        refused["diagnostics"][0]["cause"], "missing-name",
        "{refused}"
    );
    assert_eq!(refused["last_stage"], "I1", "{refused}");
    assert!(
        refused["diagnostics"][0]["locus"]["source"]["identity"]
            .as_str()
            .is_some_and(|identity| identity.ends_with("ghost.fsl")),
        "{refused}"
    );
}

#[test]
fn clause_request_honors_model_format_without_content_sniffing() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("program.fsl"),
        program("example/shop"),
    )
    .unwrap();
    for (format, file, reason) in format_cases(directory.path()) {
        let request = json!({
            "models": [{"format": format, "source": source(&file)}],
            "program": {"source": source("program.fsl")},
            "clause": {
                "selection": {"clause": {
                    "name": "Q", "observation": "current",
                    "self": {"population": "ix://example/shop/orders", "key": "order-1"},
                    "anchor": {"kind": "handler", "name": "validate"}
                }},
                "observations": {}
            }
        });
        let path = write_route_request(directory.path(), "clause.json", "native-run/1", request);
        let error = command::read_clause_run_request(&path)
            .err()
            .expect("the selected format must be enforced");
        assert_selected_model_format_refusal(&error, Operation::Execute, &file, reason);
    }
}

#[test]
fn analyze_package_request_honors_model_format_without_content_sniffing() {
    let directory = tempfile::tempdir().unwrap();
    let handoff = directory.path().join("handoff");
    std::fs::create_dir(&handoff).unwrap();
    std::fs::write(
        handoff.join("checked-package.json"),
        r#"{"contract_version":"filament.checked-package/v2"}"#,
    )
    .unwrap();
    for (format, file, reason) in format_cases(directory.path()) {
        let request = json!({
            "package": {"directory": "handoff"},
            "models": [{"format": format, "source": source(&file)}]
        });
        let path = write_route_request(
            directory.path(),
            "analyze.json",
            "native-analyze/1",
            request,
        );
        let error = command::prepare_analyze(&path, command::RunLimits::default())
            .err()
            .expect("the selected format must be enforced");
        assert_selected_model_format_refusal(&error, Operation::Analyze, &file, reason);
    }
}

#[test]
fn package_run_request_honors_model_format_without_content_sniffing() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("package.json"),
        r#"{"contract_version":"filament.checked-package/v2"}"#,
    )
    .unwrap();
    for (format, file, reason) in format_cases(directory.path()) {
        let request = json!({
            "models": [{"format": format, "source": source(&file)}],
            "package": {"file": "package.json"},
            "call": {"function": "one", "arguments": [], "accounting": {"work_units": 1000}}
        });
        let path = write_route_request(
            directory.path(),
            "package-run.json",
            "native-run/1",
            request,
        );
        let error = command::prepare_run(&path, command::RunLimits::default())
            .err()
            .expect("the selected format must be enforced");
        assert_selected_model_format_refusal(&error, Operation::Execute, &file, reason);
    }
}

#[test]
fn analyze_duplicate_domain_packages_keep_the_typed_conflict_refusal() {
    let directory = tempfile::tempdir().unwrap();
    let handoff = directory.path().join("handoff");
    std::fs::create_dir(&handoff).unwrap();
    std::fs::write(
        handoff.join("checked-package.json"),
        r#"{"contract_version":"filament.checked-package/v2"}"#,
    )
    .unwrap();
    let package = package_input([SHOP.as_bytes()]).expect("the model compiles");
    let document = &package["example/shop"];
    std::fs::write(directory.path().join("first.json"), document).unwrap();
    std::fs::write(directory.path().join("second.json"), document).unwrap();
    let request = json!({
        "package": {"directory": "handoff"},
        "models": [
            {"format": "semantic-ir/2.0.0", "source": source("first.json")},
            {"format": "semantic-ir/2.0.0", "source": source("second.json")}
        ]
    });
    let path = write_route_request(
        directory.path(),
        "analyze.json",
        "native-analyze/1",
        request,
    );
    let error = command::prepare_analyze(&path, command::RunLimits::default())
        .err()
        .expect("duplicate selected package identities refuse");
    let outcome: Value = serde_json::from_slice(
        &error
            .outcome_document(Operation::Analyze)
            .to_bytes()
            .expect("typed package conflict outcome serializes"),
    )
    .expect("typed package conflict outcome is JSON");
    assert_eq!(outcome["last_stage"], "I1", "{outcome}");
    assert_eq!(
        outcome["diagnostics"][0]["code"], "invalid_package",
        "{outcome}"
    );
    assert_eq!(
        outcome["diagnostics"][0]["cause"], "conflicting-definition",
        "{outcome}"
    );
}

#[test]
#[trace("TC-2998", "FR-710-AC-9")]
fn a_compile_request_over_a_model_unit_writes_its_document() {
    let directory = tempfile::tempdir().unwrap();
    let output = json!({"package": "shop.semantic-ir.json"});
    let path = compile_request(
        directory.path(),
        &[("shop.fsl", SHOP)],
        "shop.fsl",
        Value::Null,
        &[("output", output)],
    );
    let outcome = command::compile_outcome(&path).expect("the model unit compiles");
    let parsed = cst::parse(
        SourceIdentity::new("agent-ix", "test:shop.fsl"),
        "shop.fsl",
        SHOP.as_bytes(),
        cst::Limits::default(),
    )
    .expect("the model unit parses");
    let expected = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the model unit compiles")
        .bytes()
        .to_vec();
    assert_eq!(outcome.package_bytes(), expected.as_slice());
    let outcome_document: Value =
        serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap();
    assert_eq!(outcome_document["last_stage"], "I1");
    assert_eq!(outcome_document["artifacts"], json!([]));
    assert_eq!(
        serde_json::from_slice::<Value>(outcome.package_bytes()).unwrap()["contractVersion"],
        "2.0.0"
    );
    assert_eq!(
        std::fs::read(directory.path().join("shop.semantic-ir.json")).unwrap(),
        expected
    );

    for (member, value) in [
        ("models", json!([])),
        ("libraries", json!([])),
        (
            "libraries",
            json!([{"identity": "test/geometry", "source": source("shop.fsl")}]),
        ),
        ("clocks", json!({})),
        (
            "handoff",
            json!({"directory": "handoff", "package_name": "shop"}),
        ),
        ("refinement_internals", json!({})),
        ("integer_division", json!("floor")),
    ] {
        let output_path = format!("refused-{member}.json");
        let path = compile_request(
            directory.path(),
            &[],
            "shop.fsl",
            Value::Null,
            &[(member, value), ("output", json!({"package": output_path}))],
        );
        let refused = refusal(&command::compile_outcome(&path).err().expect("refuses"));
        assert_eq!(refused["code"], "invalid-request", "{member}: {refused}");
        assert!(refused.to_string().contains(member), "{refused}");
        assert!(!directory.path().join(output_path).exists());
    }
}

const GRAPH: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/graph\";
object Node { rank: Int[0, 9]; relationship next: Node [0, 1]; }
population nodes: Node;
";

/// `Forward` asks whether `self` reaches the node of rank 2, `Backward`
/// whether it reaches the node of rank 0.
const GRAPH_PROGRAM: &str = "language \"ix:native\" edition \"1-draft\";
profile P = \"filament.model.complete/v1\";
model M = \"example/graph\";
invariant Forward using P on M::Node at current { exists(n in M::nodes : n.rank = 2 and reaches(self, n, next)) }
invariant Backward using P on M::Node at current { exists(n in M::nodes : n.rank = 0 and reaches(self, n, next)) }
";

/// The `nodes` snapshot of the compiled relationship: `a -> b -> c`, with
/// ranks 0, 1 and 2.
fn graph_snapshot() -> Value {
    let node = |key: &str, rank: &str| json!({"key": key, "type": "ix://example/graph/Node", "fields": {"rank": {"integer": rank}}});
    let end = |key: &str| json!({"population": "ix://example/graph/nodes", "key": key});
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "agent-ix", "identity": "test:graph:current"},
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": "example/graph"},
        "populations": [{
            "population": "ix://example/graph/nodes",
            "complete": true,
            "objects": [node("a", "0"), node("b", "1"), node("c", "2")],
        }],
        "relationships": [{
            "relationship": "ix://example/graph/relationship/Node-next-Node",
            "complete": true,
            "instances": [
                {"source": end("a"), "target": end("b")},
                {"source": end("b"), "target": end("c")},
            ],
        }],
    })
}

/// Runs the clause `name` over node `key`: whether it holds.
fn graph_clause_holds(directory: &Path, name: &str, key: &str) -> bool {
    std::fs::write(directory.join("graph.fsl"), GRAPH).unwrap();
    std::fs::write(directory.join("program.fsl"), GRAPH_PROGRAM).unwrap();
    std::fs::create_dir_all(directory.join("data")).unwrap();
    std::fs::write(
        directory.join("data/current.json"),
        serde_json::to_vec(&graph_snapshot()).unwrap(),
    )
    .unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "native-model-unit/1", "source": source("graph.fsl")}],
        "clause": {
            "selection": {"clause": {"name": name, "observation": "current",
                "self": {"population": "ix://example/graph/nodes", "key": key},
                "anchor": {"kind": "handler", "name": "validate"}}},
            "observations": {"current": "data/current.json"}},
        "program": {"source": source("program.fsl")}}});
    let path = directory.join("graph-run.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let (exit, value) = match command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    };
    let holds =
        value["disposition"]["outcome"]["value"] == json!({"kind": "boolean", "value": true});
    assert_eq!(
        exit,
        if holds { 0 } else { 10 },
        "{name} over {key}: {value}"
    );
    assert!(
        holds
            || value["disposition"]["outcome"]["value"]
                == json!({"kind": "boolean", "value": false}),
        "{name} over {key}: {value}"
    );
    holds
}

#[test]
#[trace("TC-1942", "FR-710-AC-2")]
fn reaches_checks_and_evaluates_over_a_model_unit_relationship() {
    let directory = tempfile::tempdir().unwrap();
    let dir = directory.path();
    assert!(graph_clause_holds(dir, "Forward", "a"), "a reaches c");
    assert!(graph_clause_holds(dir, "Forward", "b"), "b reaches c");
    assert!(
        !graph_clause_holds(dir, "Forward", "c"),
        "c reaches nothing"
    );
    assert!(
        !graph_clause_holds(dir, "Backward", "c"),
        "c does not reach a"
    );
    assert!(
        !graph_clause_holds(dir, "Backward", "b"),
        "b does not reach a"
    );
}
