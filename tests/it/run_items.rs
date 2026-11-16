// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-651 and FR-652: several selections, population expansions, contexts
//! and value invariants in one `run` request, and FR-650's Reference
//! argument over a call snapshot, through `command`'s `run`.
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{run_file, Output};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-651:{file}"),
        "document": document, "formal_revision": 1
    })
}

fn population() -> String {
    format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)
}

/// Write the healthy case's spine files into `directory`, plus the
/// violating case's snapshot as `violating-snapshot.json`.
fn write_cases(directory: &Path) {
    spine::write(directory, Case::Healthy).unwrap();
    add_value_invariant(directory);
    let snapshot_path = directory.join("spine-snapshot.json");
    let mut snapshot: Value =
        serde_json::from_slice(&std::fs::read(&snapshot_path).unwrap()).unwrap();
    snapshot["populations"].as_array_mut().unwrap().push(json!({
        "population": format!("ix://{}/unrelated_history", spine::PACKAGE_IDENTITY),
        "complete": false,
        "objects": [],
    }));
    std::fs::write(&snapshot_path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    let violating = directory.join("violating");
    spine::write(&violating, Case::Violating).unwrap();
    std::fs::copy(
        violating.join("spine-snapshot.json"),
        directory.join("violating-snapshot.json"),
    )
    .unwrap();
}

fn add_value_invariant(directory: &Path) {
    let path = directory.join("spine-model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let object_type = model["types"]
        .as_array_mut()
        .and_then(|types| {
            types.iter_mut().find(|record| {
                record["identity"] == format!("ix://{}/ConfigVersion", spine::PACKAGE_IDENTITY)
            })
        })
        .expect("ConfigVersion model type");
    object_type["clauses"] = json!([{
        "identity": format!("ix://{}/ConfigVersion/PositiveVersion", spine::PACKAGE_IDENTITY),
        "language": "quire",
        "clauseId": "PositiveVersion",
        "text": "self.versionNumber > 1",
        "origin": {"generated": {
            "generatorIdentity": format!("ix://{}/ConfigVersion/PositiveVersion", spine::PACKAGE_IDENTITY),
            "generatorVersion": "1.0.0",
            "inputIdentities": [format!("ix://{}/ConfigVersion/PositiveVersion", spine::PACKAGE_IDENTITY)],
        }},
    }]);
    let unrelated_type_identity = format!("ix://{}/ArchiveEntry", spine::PACKAGE_IDENTITY);
    let mut unrelated_type = object_type.clone();
    unrelated_type["identity"] = json!(unrelated_type_identity);
    unrelated_type["displayName"] = json!("ArchiveEntry");
    unrelated_type["fields"] = json!([]);
    unrelated_type["operations"] = json!([]);
    unrelated_type["clauses"] = json!([]);
    unrelated_type["origin"]["generated"]["generatorIdentity"] = json!(unrelated_type_identity);
    unrelated_type["origin"]["generated"]["inputIdentities"] = json!([unrelated_type_identity]);
    model["types"].as_array_mut().unwrap().push(unrelated_type);
    let mut unrelated = model["populations"][0].clone();
    unrelated["identity"] = json!(format!(
        "ix://{}/unrelated_history",
        spine::PACKAGE_IDENTITY
    ));
    unrelated["displayName"] = unrelated["identity"].clone();
    unrelated["members"] = json!([unrelated_type_identity]);
    unrelated["origin"]["generated"]["generatorIdentity"] = unrelated["identity"].clone();
    unrelated["origin"]["generated"]["inputIdentities"] = json!([unrelated["identity"]]);
    model["populations"].as_array_mut().unwrap().push(unrelated);
    std::fs::write(&path, serde_json::to_vec(&model).unwrap()).unwrap();
}

/// Add TC-1045's small state machine alongside the shared ConfigVersion
/// package and write an invocation whose post context can violate its frame.
fn add_context_fixture(directory: &Path) {
    let model_path = directory.join("spine-model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(&model_path).unwrap()).unwrap();
    let machine = format!("ix://{}/OrderLifecycle", spine::PACKAGE_IDENTITY);
    let origin = |node: &str| {
        json!({"generated": {
            "generatorIdentity": node,
            "generatorVersion": "1.0.0",
            "inputIdentities": [node],
        }})
    };
    let field = |name: &str| {
        let identity = format!("{machine}/{name}");
        json!({"identity": identity, "name": name,
            "typeRef": "ix://filament/native/Boolean", "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin(&identity)})
    };
    let operation = |name: &str, modifies: &[&str]| {
        let identity = format!("{machine}/{name}");
        json!({"identity": identity, "name": name, "params": [], "pre": [], "post": [],
            "frame": {"modifies": modifies.iter().map(|field| format!("{machine}/{field}")).collect::<Vec<_>>(),
                "creates": [], "deletes": []}, "origin": origin(&identity)})
    };
    let state = |name: &str| {
        let identity = format!("{machine}/{name}");
        json!({"identity": identity, "name": name, "origin": origin(&identity)})
    };
    let transition = |name: &str, from: &str, to: &str, trigger: &str| {
        let identity = format!("{machine}/{name}");
        json!({"identity": identity, "from": format!("{machine}/{from}"),
            "to": format!("{machine}/{to}"), "trigger": format!("{machine}/{trigger}"),
            "emits": [], "origin": origin(&identity)})
    };
    let clause_identity = format!("{machine}/Consistent");
    let machine_type = json!({
        "identity": machine,
        "displayName": machine,
        "kind": {"module": spine::PACKAGE_IDENTITY, "name": "state_machine"},
        "roles": [], "origin": origin(&machine), "constraints": [], "extensions": [],
        "unknownPolicy": "reject", "fields": [field("placed"), field("cancelled")],
        "operations": [operation("place", &["placed"]), operation("cancel", &["cancelled"])],
        "clauses": [{"identity": clause_identity, "language": "quire", "clauseId": "Consistent",
            "text": "not (self.placed and self.cancelled)", "origin": origin(&clause_identity)}],
        "states": [state("Draft"), state("Placed"), state("Cancelled")],
        "transitions": [transition("transition0", "Draft", "Placed", "place"),
            transition("transition1", "Draft", "Cancelled", "cancel"),
            transition("transition2", "Placed", "Cancelled", "cancel")],
    });
    model["types"].as_array_mut().unwrap().push(machine_type);
    let construct = model["constructs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"]["name"] == "object_type")
        .unwrap()
        .clone();
    let mut lifecycle_construct = construct;
    lifecycle_construct["kind"]["name"] = json!("state_machine");
    lifecycle_construct["construct"]["meaning"] = json!("filament.meaning.model.state-machine/v1");
    lifecycle_construct["construct"]["members"] =
        json!({"states": "required", "transitions": "required"});
    model["constructs"]
        .as_array_mut()
        .unwrap()
        .push(lifecycle_construct);
    std::fs::write(&model_path, serde_json::to_vec(&model).unwrap()).unwrap();

    let unit_path = directory.join("spine-unit.native");
    let mut unit = std::fs::read_to_string(&unit_path).unwrap();
    unit.push_str(&format!("invariant Consistent using v on Config::OrderLifecycle at current {{ not (self.placed and self.cancelled) }}\n"));
    std::fs::write(unit_path, unit).unwrap();

    let invocation_identity = "test:fr-652:invocation";
    let machine_fields = |placed, cancelled| json!({"placed": {"boolean": placed}, "cancelled": {"boolean": cancelled}});
    let pre_path = directory.join("spine-pre-snapshot.json");
    let post_path = directory.join("spine-post-snapshot.json");
    for (path, observation, placed, cancelled) in [
        (&pre_path, "pre", false, false),
        (&post_path, "post", true, false),
    ] {
        let mut snapshot: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        snapshot["invocation"] = json!({"authority": "agent-ix", "identity": invocation_identity});
        snapshot["observation"] = json!(observation);
        snapshot["contexts"] =
            json!([{"machine": machine, "fields": machine_fields(placed, cancelled)}]);
        std::fs::write(path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    }
    let invocation_path = directory.join("spine-invocation.json");
    let mut invocation: Value =
        serde_json::from_slice(&std::fs::read(&invocation_path).unwrap()).unwrap();
    invocation["identity"] = json!({"authority": "agent-ix", "identity": invocation_identity});
    invocation["context"] = json!(machine);
    invocation["operation"] = json!("place");
    invocation["self"] = Value::Null;
    invocation["parameters"] = json!({});
    invocation["result"] = Value::Null;
    std::fs::write(invocation_path, serde_json::to_vec(&invocation).unwrap()).unwrap();
}

/// The `ParentOrder` clause selection over observation `observation` with
/// `self` `self_value`.
fn parent_order(observation: &str, self_value: Value) -> Value {
    json!({"clause": {"name": "ParentOrder", "observation": observation,
        "self": self_value, "anchor": {"kind": "handler", "name": "validate"}}})
}

fn child() -> Value {
    json!({"population": population(), "key": "child"})
}

/// A native-run/1 request over the spine files carrying `body` as its
/// selection member (`call`, `clause` or `items`).
fn request(directory: &Path, member: &str, body: Value) -> Output {
    let mut inner = json!({
        "models": [{"format":"semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
    });
    inner[member] = body;
    let request = json!({"format": "native-run/1", "request": inner});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

fn observations() -> Value {
    json!({"current": "spine-snapshot.json", "violating": "violating-snapshot.json"})
}

fn items_document(output: &Output) -> Value {
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "no items document: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(document["format"], "native-run-items/1", "{document}");
    document
}

/// FR-651 permits a population expansion at a temporal `over` position.
#[test]
#[trace("TC-1709", "FR-651-AC-2")]
fn temporal_over_population_selector_expands_one_item_per_object() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let unit_path = directory.path().join("spine-unit.native");
    let unit = std::fs::read_to_string(&unit_path).unwrap();
    let unit = unit.replace(
        "profile v = \"filament.model.complete/v1\";",
        "profile v = \"filament.model.complete/v1\";\nprofile f = \"filament.temporal.infinite-trace/v1\";",
    );
    std::fs::write(
        unit_path,
        format!("{unit}\ntemporal Reaches using f over (r: Config::ConfigVersion) clock \"model-steps\" on origin {{ eventually holds(r.versionNumber > 1) }}\n"),
    )
    .unwrap();
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(),
            "accounting": {"work_units": 1000},
            "selections": [{"temporal": {"name": "Reaches",
                "trace": {"finite": ["current"]},
                "over": {"reference": {"each": {"population": population()}}}}}]}),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let document = items_document(&output);
    assert_eq!(document["selections"], json!([{"index": 0, "expanded": 2}]));
    assert_eq!(document["items"][0]["object"]["key"], "root");
    assert_eq!(document["items"][1]["object"]["key"], "child");
}

/// FR-651-AC-2: a function selection expands its Reference argument over the
/// selected snapshot, preserving the snapshot lookup for every item.
#[test]
#[trace("TC-1709", "FR-651-AC-2")]
fn function_argument_population_selector_expands_one_item_per_object() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(
        directory.path(),
        "function itemReferenceIsSelf using v(r: Config::ConfigVersion): Boolean pure { r = r }\n",
    );
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"function": {"name": "itemReferenceIsSelf", "snapshot": "current", "arguments": [
                {"parameter": "r", "value": {"each": {"population": population()}}}
            ]}}
        ]}),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let document = items_document(&output);
    assert_eq!(document["selections"], json!([{"index": 0, "expanded": 2}]));
    assert_eq!(document["items"][0]["object"]["key"], "root");
    assert_eq!(document["items"][1]["object"]["key"], "child");
    for item in document["items"].as_array().unwrap() {
        assert_eq!(item["result"]["format"], "native-run-result/2", "{item}");
        assert_eq!(
            item["result"]["disposition"]["category"], "success",
            "{item}"
        );
    }
}

fn assert_request_refusal(output: &Output, member: &str) {
    assert_eq!(output.status.code(), Some(20), "{member}: {output:?}");
    assert!(output.stdout.is_empty(), "{member}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid-request", "{member}: {failure}");
    assert_eq!(failure["stage"], "request", "{member}: {failure}");
    assert!(failure.to_string().contains(member), "{member}: {failure}");
}

#[test]
#[trace("TC-1708", "FR-651-AC-1")]
fn two_clause_selections_report_two_items() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("violating", child()),
        ]}),
    );
    assert_eq!(output.status.code(), Some(10), "{output:?}");
    let document = items_document(&output);
    assert_eq!(
        document["selections"],
        json!([{"index": 0, "expanded": 1}, {"index": 1, "expanded": 1}])
    );
    assert_eq!(
        document["items"][0]["result"]["disposition"]["category"],
        "success"
    );
    assert_eq!(
        document["items"][1]["result"]["disposition"]["category"],
        "violation"
    );
    assert_eq!(document["category"], "violation");
}

#[test]
#[trace("TC-1709", "FR-651-AC-2")]
fn population_selector_expands_one_item_per_object() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", json!({"each": {"population": population()}})),
        ]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["selections"], json!([{"index": 0, "expanded": 2}]));
    assert_eq!(document["items"][0]["object"]["key"], "root");
    assert_eq!(document["items"][1]["object"]["key"], "child");
}

/// Negative: one refused item leaves the others unchanged.
#[test]
#[trace("TC-1710", "FR-651-AC-3")]
fn one_refused_item_leaves_the_others_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("current", json!({"population": population(), "key": "ghost"})),
            parent_order("violating", child()),
        ]}),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let document = items_document(&output);
    assert_eq!(
        document["items"][0]["result"]["disposition"]["category"],
        "success"
    );
    assert_eq!(
        document["items"][1]["result"]["disposition"]["category"],
        "refusal"
    );
    assert_eq!(
        document["items"][2]["result"]["disposition"]["category"],
        "violation"
    );
    assert_eq!(document["category"], "refusal");
}

/// Negative: malformed items requests refuse at the request stage.
#[test]
#[trace("TC-1712", "FR-651-AC-5")]
fn malformed_items_requests_refuse_at_the_request_stage() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let each = json!({"each": {"population": population()}});
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": []}),
    );
    assert_request_refusal(&output, "selections");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"clause": parent_order("current", child())["clause"].clone(),
             "frame": {"operation": "x", "invocation": "current"}}
        ]}),
    );
    assert_request_refusal(&output, "selections");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"clause": {"name": "ParentOrder", "observation": "current",
                "unexpected": true}},
        ]}),
    );
    assert_request_refusal(&output, "selections");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection": parent_order("current", each)}),
    );
    assert_request_refusal(&output, "each");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("missing", child()),
        ]}),
    );
    assert_request_refusal(&output, "missing");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "current", "extra": true}},
        ]}),
    );
    assert_request_refusal(&output, "value_invariants");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "missing"}},
        ]}),
    );
    assert_request_refusal(&output, "missing");
}

/// FR-651-AC-5: program syntax refusal belongs to the request and emits no
/// partial items result.
#[test]
#[trace("TC-1712", "FR-651-AC-5")]
fn items_program_syntax_error_refuses_without_an_items_document() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    std::fs::write(
        directory.path().join("spine-unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nfunction broken {",
    )
    .unwrap();
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
        ]}),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("native-run-items/1"),
        "{output:?}"
    );
}

/// FR-651-AC-5: a declaration-resolution failure is settled as a request
/// refusal and never becomes an item result.
#[test]
#[trace("TC-1712", "FR-651-AC-5")]
fn items_compile_refusal_has_no_items_document() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let source = directory.path().join("spine-unit.native");
    let mut text = std::fs::read_to_string(&source).unwrap();
    text.push_str("function invalid using v(): Boolean pure { 1 + true }\n");
    std::fs::write(source, text).unwrap();
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child())
        ]}),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refusal["stage"], "request", "{refusal}");
    assert_eq!(refusal["code"], "invalid-request", "{refusal}");
    assert!(refusal.to_string().contains("compilation"), "{refusal}");
}

#[test]
#[trace("TC-1713", "FR-651-AC-6")]
fn unheld_population_selector_gives_one_refused_item() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", json!({"each": {"population": "ix://example/config-version/nowhere"}})),
            parent_order("current", child()),
        ]}),
    );
    let document = items_document(&output);
    assert_eq!(
        document["items"][0]["result"]["disposition"]["category"], "refusal",
        "{document}"
    );
    assert!(document["items"][0]["result"]
        .to_string()
        .contains("nowhere"));
    assert!(document["items"][0].get("object").is_none());
    assert_eq!(
        document["items"][1]["result"]["disposition"]["category"],
        "success"
    );
}

/// Negative: FR-652 selections with malformed members refuse.
#[test]
#[trace("TC-1715", "FR-652-AC-2")]
fn malformed_context_and_value_invariant_selections_refuse() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"context": {"name": "ParentOrder", "invocation": "current", "side": "mid"}}}),
    );
    assert_request_refusal(&output, "side");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"value_invariants": {"snapshot": "current"}}}),
    );
    assert_request_refusal(&output, "value_invariants");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"context": {"name": "ParentOrder", "invocation": "i", "side": "pre", "self": {}}}}),
    );
    assert_request_refusal(&output, "self");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"context": {"name": "ParentOrder", "invocation": "current", "side": "pre"}}}),
    );
    assert_request_refusal(&output, "invocation");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "current", "invocation": "current"}},
        ]}),
    );
    assert_request_refusal(&output, "invocation");
}

/// FR-312's `parameters` member is accepted for a pre-snapshot selection.
#[test]
#[trace("TC-1908", "FR-312-AC-5")]
fn parameters_member_accepted_for_pre_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Unchanged).unwrap();
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": {"pre": "spine-pre-snapshot.json"}, "selection":
            {"clause": {"name": "ParentOrder", "observation": "pre",
                "self": child(), "parameters": {}}}}),
    );
    let decoded =
        fsl_command::command::read_clause_run_request(&directory.path().join("request.json"))
            .expect("parameters are admitted for a pre snapshot");
    let fsl_replay::spine::ClauseRunSelection::Clause(clause) = decoded.selection else {
        panic!("the request selects one clause");
    };
    assert!(matches!(
        clause.input,
        fsl_semantics::model::observation::ClauseSelectionInput::PreCall { .. }
    ));
    assert!(output.stdout.len() > 0, "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["disposition"]["stage"], "admit");
    assert_eq!(document["disposition"]["refusal"]["code"], "wrong_snapshot");
}

/// FR-652-AC-3: a value_invariants selection is reachable from an items
/// request and lists one item per evaluated invariant and value.
#[test]
#[trace("TC-1716", "FR-652-AC-3")]
fn value_invariants_selection_is_an_items_selection() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "current"}}
        ]}),
    );
    let document = items_document(&output);
    assert!(
        !document["items"].as_array().unwrap().is_empty(),
        "{document}"
    );
    assert_eq!(output.status.code(), Some(10), "{output:?}");
    assert_eq!(document["category"], "violation", "{document}");
    assert_eq!(document["items"].as_array().unwrap().len(), 2, "{document}");
    assert_eq!(
        document["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["path"]["root"]["subject"]["object_identity"].clone())
            .collect::<Vec<_>>(),
        vec![json!("child"), json!("root")],
        "FR-366 path order: {document}"
    );
    for item in document["items"].as_array().unwrap() {
        assert!(item.get("invariant").is_some(), "{item}");
        assert!(item.get("path").is_some(), "{item}");
        assert_eq!(item["result"]["format"], "native-run-result/2", "{item}");
        let key = item["path"]["root"]["subject"]["object_identity"]
            .as_str()
            .unwrap_or_else(|| panic!("missing path object: {item}"));
        assert_eq!(
            item["result"]["disposition"]["category"],
            if key == "root" {
                "violation"
            } else {
                "success"
            },
            "{item}"
        );
        assert_eq!(item["result"]["disposition"]["stage"], "evaluate");
        assert!(item["result"].get("usage").is_some());
        assert!(item["result"].get("axes").is_some());
        fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&item["result"]).unwrap())
            .unwrap_or_else(|error| panic!("invalid native-run-result/2 item {item}: {error}"));
    }
}

/// Negative: FR-650-AC-4's Reference argument with no call snapshot.
#[test]
#[trace("TC-1703", "FR-650-AC-4")]
#[ignore = "QSL-1988: Reference arguments over a call snapshot not implemented yet"]
fn reference_argument_without_a_snapshot_refuses() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let call = |snapshot: Option<&str>| {
        let mut call = json!({"function": "same", "arguments": [
            {"parameter": "r", "value": {"reference": child()}}]});
        if let Some(snapshot) = snapshot {
            call["snapshot"] = json!(snapshot);
        }
        call
    };
    let output = request(directory.path(), "call", call(Some("spine-snapshot.json")));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(document["outcome"]["value"]["reference"]["object_identity"].is_string());
    let output = request(directory.path(), "call", call(None));
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid_runtime_input", "{failure}");
    assert_eq!(failure["cause"], "missing-member", "{failure}");
    assert_eq!(failure["stage"], "call", "{failure}");
    assert_eq!(
        failure["details"],
        json!({"position": 0, "path": "/call/snapshot"}),
        "{failure}"
    );
}

/// Append `functions` to the written spine unit.
fn extend_unit(directory: &Path, functions: &str) {
    let unit = std::fs::read_to_string(directory.join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.join("spine-unit.native"),
        format!("{unit}{functions}"),
    )
    .unwrap();
}

const SAME: &str = "function same using v(r: Config::ConfigVersion): \
                    Config::ConfigVersion pure { r }\n";

/// Call `same` with `value` over the healthy snapshot and return the
/// completed value.
fn same_of(directory: &Path, value: Value) -> Value {
    let output = request(
        directory,
        "call",
        json!({"function": "same", "snapshot": "spine-snapshot.json",
            "arguments": [{"parameter": "r", "value": value}]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    document["outcome"]["value"].clone()
}

/// FR-650-AC-5: a Reference result is its identity triple and passes back
/// unchanged as an argument.
#[test]
#[trace("TC-1704", "FR-650-AC-5")]
#[ignore = "QSL-2182: structural identity spelling of a Reference result and call.snapshot reference arguments not implemented yet"]
fn reference_result_passes_back_as_an_argument() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let first = same_of(directory.path(), json!({"reference": child()}));
    let triple = &first["reference"];
    assert!(triple["universe"]["package"].is_string(), "{first}");
    assert!(triple["universe"]["path"].is_array(), "{first}");
    assert!(triple["universe"]["ordinal"].is_u64(), "{first}");
    assert!(triple["type"]["original"]["node"].is_string(), "{first}");
    assert!(triple["object_identity"].is_string(), "{first}");
    assert_eq!(same_of(directory.path(), first.clone()), first);
}

/// FR-650-AC-7: an FR-312 `function` selection admits the run value form.
#[test]
#[trace("TC-1706", "FR-650-AC-7")]
#[ignore = "QSL-1988: run value form arguments in a clause-run function selection not implemented yet"]
fn function_selection_admits_an_identity_reference() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let identity = same_of(directory.path(), json!({"reference": child()}));
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection": {"function": {
            "name": "sameIdentity", "snapshot": "current", "arguments": [
                {"parameter": "a", "value": identity},
                {"parameter": "b", "value": {"reference": child()}}]}}}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["disposition"]["truth"], true, "{document}");
}

/// FR-651-AC-4: the most severe item decides the exit, and an item's budget
/// is its own.
#[test]
#[trace("TC-1711", "FR-651-AC-4")]
fn items_take_the_most_severe_exit_and_their_own_budgets() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(
        directory.path(),
        "function seven using v(): Int[0, 9] pure { 7 }\n",
    );
    let seven = json!({"call": {"function": "seven", "arguments": []}});
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "accounting": {"work_units": 0},
            "selections": [parent_order("violating", child()), seven.clone()]}),
    );
    assert_eq!(output.status.code(), Some(22), "{output:?}");
    let document = items_document(&output);
    assert_eq!(
        document["items"][1]["result"]["outcome"]["kind"],
        "incomplete"
    );
    assert_eq!(document["category"], "incomplete");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "accounting": {"work_units": 1000},
            "selections": [seven.clone(), seven]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = items_document(&output);
    for item in document["items"].as_array().unwrap() {
        assert_eq!(item["result"]["outcome"]["kind"], "completed", "{item}");
    }
}

/// FR-652-AC-1: a `context` selection is read and reaches FR-106 admission,
/// which refuses an object invariant selected by `Context` (FR-106-AC-46).
#[test]
#[trace("TC-1714", "FR-652-AC-1")]
fn context_selection_matches_in_process_for_pre_and_post() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::ForbiddenParent).unwrap();
    for side in ["pre", "post"] {
        let output = request(
            directory.path(),
            "clause",
            json!({"observations": {"i": "spine-invocation.json",
                "pre": "spine-pre-snapshot.json", "post": "spine-post-snapshot.json"}, "selection":
                {"context": {"name": "ParentOrder", "invocation": "i", "side": side}}}),
        );
        let in_process_request =
            fsl_command::command::read_clause_run_request(&directory.path().join("request.json"))
                .expect("context request decodes for in-process parity");
        let selection = in_process_request.selection.clone();
        let report = fsl_replay::spine::run_clause(in_process_request)
            .expect("context request is non-empty");
        let in_process = fsl_replay::spine::write_clause_run_result_for(&report, Some(&selection))
            .expect("in-process result serializes");
        let command_document: Value = serde_json::from_slice(&output.stdout).unwrap();
        let direct_document: Value = serde_json::from_slice(&in_process).unwrap();
        assert_eq!(
            command_document, direct_document,
            "{side}: command/direct parity"
        );
        assert_eq!(output.status.code(), Some(20), "{side}: {output:?}");
        let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{side}: no clause-run document: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert_eq!(document["format"], "native-run-result/2", "{document}");
        assert_eq!(document["disposition"]["stage"], "admit", "{document}");
        assert!(
            document.to_string().contains("wrong-observation"),
            "{document}"
        );
    }
}

/// FR-652-AC-1: state-machine context succeeds on both invocation sides and
/// refuses a post-side change outside the operation frame.
#[test]
#[trace("TC-1045", "FR-652-AC-1", "FR-106-AC-46")]
fn state_machine_context_runs_pre_post_and_checks_post_frame() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Unchanged).unwrap();
    add_context_fixture(directory.path());
    for side in ["pre", "post"] {
        let output = request(
            directory.path(),
            "clause",
            json!({"observations": {"i": "spine-invocation.json",
                "pre": "spine-pre-snapshot.json", "post": "spine-post-snapshot.json"}, "selection":
                {"context": {"name": "Consistent", "invocation": "i", "side": side}}}),
        );
        assert_eq!(output.status.code(), Some(0), "{side}: {output:?}");
        let document: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(document["format"], "native-run-result/2", "{document}");
        assert_eq!(
            document["disposition"]["category"], "success",
            "{side}: {document}"
        );
        assert_eq!(document["disposition"]["truth"], true, "{side}: {document}");
        let in_process_request =
            fsl_command::command::read_clause_run_request(&directory.path().join("request.json"))
                .expect("the state-machine context request decodes");
        let selection = in_process_request.selection.clone();
        let report = fsl_replay::spine::run_clause(in_process_request)
            .expect("the context run is non-empty");
        let in_process = fsl_replay::spine::write_clause_run_result_for(&report, Some(&selection))
            .expect("the in-process context result serializes");
        let direct: Value = serde_json::from_slice(&in_process).unwrap();
        assert_eq!(document, direct, "{side}: command/direct parity");
    }

    let post_path = directory.path().join("spine-post-snapshot.json");
    let mut post: Value = serde_json::from_slice(&std::fs::read(&post_path).unwrap()).unwrap();
    post["contexts"][0]["fields"]["cancelled"] = json!({"boolean": true});
    std::fs::write(&post_path, serde_json::to_vec(&post).unwrap()).unwrap();
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": {"i": "spine-invocation.json",
            "pre": "spine-pre-snapshot.json", "post": "spine-post-snapshot.json"}, "selection":
            {"context": {"name": "Consistent", "invocation": "i", "side": "post"}}}),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["disposition"]["stage"], "admit", "{document}");
    assert_eq!(
        document["disposition"]["refusal"]["code"], "frame_violation",
        "{document}"
    );
    assert_eq!(
        document["disposition"]["refusal"]["cause"], "unauthorized-change",
        "{document}"
    );
}

/// FR-106-AC-48: `ValueInvariants` admits one current snapshot, reachable
/// through FR-652's `value_invariants` selection.
#[test]
#[trace("TC-1051", "FR-106-AC-48")]
fn value_invariants_admits_a_current_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    spine::write(directory.path(), Case::Unchanged).unwrap();
    add_value_invariant(directory.path());
    add_context_fixture(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": {"current": "spine-snapshot.json",
            "i": "spine-invocation.json", "pre": "spine-pre-snapshot.json",
            "post": "spine-post-snapshot.json"}, "selections": [
            {"value_invariants": {"snapshot": "current", "invocation": "i"}}]}),
    );
    let document = items_document(&output);
    assert_ne!(document["category"], "refusal", "{document}");
}

/// FSpec FR-550-AC-3: items settle independently and the overall category
/// is the most severe.
#[test]
#[trace("FSpec-TC-1202", "FSpec-FR-550-AC-3")]
fn fspec_items_settle_independently() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("current", json!({"population": population(), "key": "ghost"})),
            parent_order("current", child()),
        ]}),
    );
    let document = items_document(&output);
    assert_eq!(
        document["items"][0]["result"],
        document["items"][2]["result"]
    );
    assert_eq!(
        document["items"][1]["result"]["disposition"]["category"],
        "refusal"
    );
    assert_eq!(document["category"], "refusal");
}
