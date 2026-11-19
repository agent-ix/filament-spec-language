// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1036 (TC-3016) and FR-1037 (TC-3017): a `synthesis` declaration is
//! emitted as one `claim`/`synthesis_request` node (FSpec FR-322
//! "Synthesis-request nodes"), and the run request's one `synthesis` member
//! selects a declaration by its declared name and runs a value synthesis,
//! writing the `filament.synthesis-outcome/v1` document (FSpec FR-823).

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::support::config_version_spine::{Case, spine};
use crate::support::front_end::emitted;

const VALUE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n";

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n";

fn pick(block: &str, objective: &str) -> String {
    format!(
        "{VALUE_HEADER}type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit {objective} satisfies {{ {block} }};\n"
    )
}

/// The emitted package of `unit` as JSON.
fn package(unit: &str, packages: &fsl_semantics::model::intake::PackageInput) -> Value {
    let emission = emitted(
        SourceIdentity::new("test", "synthesis-request"),
        "unit.native",
        unit.as_bytes(),
        packages,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("the unit compiles and emits: {refusal:?}"));
    serde_json::from_slice(emission.package().bytes()).expect("a JSON package")
}

/// The package's `claim`/`synthesis_request` nodes.
fn synthesis_nodes(wire: &Value) -> Vec<&Value> {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("a node list")
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "synthesis_request")
        .collect()
}

/// The body binding names of `node`, in order.
fn binding_names(node: &Value) -> Vec<&str> {
    node["body"]["members"]
        .as_array()
        .expect("an aggregate body")
        .iter()
        .filter_map(|member| member["name"].as_str())
        .collect()
}

fn binding<'a>(node: &'a Value, name: &str) -> &'a Value {
    node["body"]["members"]
        .as_array()
        .expect("an aggregate body")
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("binding {name}: {node}"))
}

/// FR-1036-AC-1, FSpec FR-322-AC-467: `Pick` is one declared
/// `synthesis_request` node with the six bindings and one `claim_names` row.
#[test]
#[trace("TC-3016", "FR-1036-AC-1")]
#[trace("FSpec-TC-2037", "FSpec-FR-322-AC-467")]
fn a_value_synthesis_is_one_synthesis_request_node() {
    let wire = package(&pick("Pick * Pick = 49", ""), &Default::default());
    let nodes = synthesis_nodes(&wire);
    assert_eq!(nodes.len(), 1, "{wire}");
    let node = nodes[0];
    assert_eq!(node["node_id"]["path"], json!(["Pick"]), "{node}");
    assert_eq!(
        binding_names(node),
        [
            "grammar",
            "domain",
            "candidate",
            "objective",
            "satisfies",
            "symmetry"
        ],
        "{node}"
    );
    assert_eq!(
        binding(node, "grammar")["value"],
        binding(node, "domain")["value"],
        "{node}"
    );
    assert_eq!(
        binding(node, "objective")["value"]["members"],
        json!([]),
        "{node}"
    );
    assert_eq!(
        binding(node, "symmetry")["value"]["value"],
        "identity",
        "{node}"
    );
    let rows = wire["claim_names"].as_array().expect("claim_names rows");
    assert_eq!(
        rows.iter().filter(|row| row["name"] == "Pick").count(),
        1,
        "{wire}"
    );
    let objective = package(&pick("Pick >= 3", "minimize Pick"), &Default::default());
    let node = synthesis_nodes(&objective)[0];
    let members = &binding(node, "objective")["value"]["members"];
    assert_eq!(members[0]["value"]["value"], "minimize", "{node}");
    assert_eq!(members[2]["value"]["value"], "Pick", "{node}");
}

/// FR-1036-AC-2: a plan synthesis's node names the object type as `grammar`
/// and the length range as `domain`.
#[test]
#[trace("TC-3016", "FR-1036-AC-2")]
fn a_plan_synthesis_node_names_the_object_type_and_the_lengths() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let document = std::fs::read(directory.path().join("spine-model.semantic-ir.json")).unwrap();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the spine model admits");
    let wire = package(
        &format!(
            "{MODEL_HEADER}type Lengths = Int[0, 3];\n\
             synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
             satisfies {{ true }};\n"
        ),
        &packages,
    );
    let node = synthesis_nodes(&wire)[0];
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let target = |name: &str| {
        let reference = &binding(node, name)["value"]["target"];
        nodes
            .iter()
            .find(|candidate| &candidate["node_id"] == reference)
            .unwrap_or_else(|| panic!("{name} resolves: {node}"))
    };
    assert_eq!(target("grammar")["node_tag"], "model", "{node}");
    assert_eq!(target("grammar")["semantic_form"], "object_type", "{node}");
    assert_eq!(target("domain")["semantic_form"], "integer_range", "{node}");
}

// ---- FR-1037: the run request's `synthesis` member ----------------------

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w60:{file}"),
        "document": "W60", "formal_revision": 1})
}

/// Runs `unit` through the command adapter with `synthesis` as the
/// request's `synthesis` member, returning its exit code and document.
fn run(unit: &str, synthesis: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [], "program": {"source": source("unit.native")}, "synthesis": synthesis}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

const OUTCOME_MEMBERS: [&str; 13] = [
    "format",
    "package",
    "declaration",
    "status",
    "check",
    "candidate",
    "objective",
    "optimal",
    "explored",
    "lineage",
    "counterexample",
    "limits",
    "cause",
];

/// FR-1037-AC-1, FSpec FR-823-AC-1/AC-2: the outcome document for a
/// validated, an exhausted, an optimal and a limited run.
#[test]
#[trace("TC-3017", "FR-1037-AC-1")]
#[trace("TC-2104", "FR-754-AC-3")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-1")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
fn the_synthesis_member_writes_the_synthesis_outcome() {
    let (exit, found) = run(
        &pick("Pick * Pick = 49", ""),
        json!({"declaration": "Pick"}),
    );
    let outcome = &found["synthesis"];
    assert_eq!(exit, 0, "{found}");
    assert_eq!(
        outcome["format"], "filament.synthesis-outcome/v1",
        "{found}"
    );
    for member in OUTCOME_MEMBERS {
        assert!(outcome.get(member).is_some(), "{member}: {found}");
    }
    assert_eq!(outcome["status"], "candidate", "{found}");
    assert_eq!(outcome["check"], "validated", "{found}");
    assert_eq!(
        outcome["candidate"],
        json!({"type": "integer", "value": "7"}),
        "{found}"
    );
    assert_eq!(outcome["explored"]["visited"], 8, "{found}");
    assert_eq!(outcome["explored"]["work"], 9, "{found}");
    assert_eq!(outcome["lineage"]["index"], 7, "{found}");
    let (exit, none) = run(
        &pick("Pick * Pick = 50", ""),
        json!({"declaration": "Pick"}),
    );
    assert_eq!(exit, 10, "{none}");
    assert_eq!(none["synthesis"]["status"], "no-candidate", "{none}");
    assert_eq!(none["synthesis"]["explored"]["visited"], 10, "{none}");
    assert_eq!(none["synthesis"]["candidate"], Value::Null, "{none}");
    let (_, optimal) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick"}),
    );
    assert_eq!(
        optimal["synthesis"]["candidate"],
        json!({"type": "integer", "value": "3"}),
        "{optimal}"
    );
    assert_eq!(optimal["synthesis"]["optimal"], true, "{optimal}");
    assert_eq!(optimal["synthesis"]["explored"]["work"], 6, "{optimal}");
    assert_eq!(
        optimal["synthesis"]["objective"]["direction"], "minimize",
        "{optimal}"
    );
    assert_eq!(optimal["synthesis"]["explored"]["visited"], 4, "{optimal}");
    assert_eq!(
        optimal["synthesis"]["objective"]["expression"], "Pick",
        "{optimal}"
    );
    assert_eq!(
        optimal["synthesis"]["objective"]["value"],
        json!({"type": "integer", "value": "3"}),
        "{optimal}"
    );
    let (exit, limited_objective) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick", "limits": {"max_work": 4}}),
    );
    assert_eq!(exit, 22, "{limited_objective}");
    assert_eq!(limited_objective["synthesis"]["status"], "incomplete");
    assert_eq!(limited_objective["synthesis"]["check"], "unchecked");
    assert_eq!(limited_objective["synthesis"]["optimal"], false);
    assert_eq!(limited_objective["synthesis"]["candidate"]["value"], "3");
    assert_eq!(limited_objective["synthesis"]["explored"]["work"], 4);
    let (exit, validated_without_objective) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick", "limits": {"max_work": 5}}),
    );
    assert_eq!(exit, 22, "{validated_without_objective}");
    assert_eq!(
        validated_without_objective["synthesis"]["status"],
        "incomplete"
    );
    assert_eq!(
        validated_without_objective["synthesis"]["check"],
        "validated"
    );
    assert_eq!(
        validated_without_objective["synthesis"]["explored"]["work"],
        5
    );
    let (exit, limited) = run(
        &pick("Pick * Pick = 50", ""),
        json!({"declaration": "Pick", "limits": {"max_candidates": 3}}),
    );
    assert_eq!(exit, 22, "{limited}");
    assert_eq!(limited["synthesis"]["status"], "incomplete", "{limited}");
    names(&limited["synthesis"]["cause"], "synthesis.max_candidates");
    let (exit, deadline) = run(
        &pick("Pick = 7", ""),
        json!({"declaration": "Pick", "accounting": {}, "deadline_ms": 0}),
    );
    assert_eq!(exit, 22, "{deadline}");
    assert_eq!(deadline["synthesis"]["status"], "incomplete", "{deadline}");
    assert_eq!(
        deadline["synthesis"]["explored"]["visited"], 0,
        "{deadline}"
    );
    assert_eq!(deadline["synthesis"]["explored"]["work"], 0, "{deadline}");
    assert_eq!(
        deadline["synthesis"]["cause"]["kind"], "deadline",
        "{deadline}"
    );
}

/// FR-1037-AC-2, FSpec FR-823-AC-3/AC-4: selection by declared name, an
/// omitted name over one and two declarations, and kind-closed members.
#[test]
#[trace("TC-3017", "FR-1037-AC-2")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-3")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-4")]
fn the_synthesis_member_selects_one_declaration_or_refuses() {
    let only = pick("Pick * Pick = 49", "");
    let (exit, value) = run(&only, json!({}));
    assert_eq!(exit, 0, "{value}");
    assert_eq!(value["synthesis"]["declaration"], "Pick", "{value}");
    let two = format!(
        "{only}synthesis Other using v grammar Digit domain Digit satisfies {{ Other = 1 }};\n"
    );
    let with_predicate =
        format!("{only}predicate Positive using v (x: Digit): Boolean {{ x >= 0 }}\n");
    for (unit, synthesis, cause, pointer) in [
        (
            two.clone(),
            json!({}),
            "missing-member",
            "/synthesis/declaration",
        ),
        (
            only.clone(),
            json!({"declaration": "Nope"}),
            "missing-name",
            "/synthesis/declaration",
        ),
        (
            with_predicate,
            json!({"declaration": "Positive"}),
            "missing-name",
            "/synthesis/declaration",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "initial": "snapshot.json"}),
            "unknown-member",
            "/synthesis/initial",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "limits": {"work": 1}}),
            "unknown-member",
            "/synthesis/limits/work",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "engines": []}),
            "unknown-member",
            "/synthesis/engines",
        ),
    ] {
        let (exit, value) = run(&unit, synthesis);
        assert_eq!(exit, 20, "{value}");
        names(&value, cause);
        names(&value, pointer);
        assert!(value["synthesis"].get("status").is_none(), "{value}");
    }
}
