// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-650: the identity, unit and declaration spellings of the run value
//! form, its Quantity and Map forms, depth and `null`, population results and
//! a `call` selection's snapshot, through `command`'s `run`.

use crate::command_harness::{run_file, Output};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

fn program_source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:run-spellings:{file}"),
        "document": "RunSpellings", "formal_revision": 1})
}

/// Run `function(a = value)` of the model-free unit `unit`.
fn run_unit(unit: &str, function: &str, value: Value) -> Output {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("program.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": program_source("program.native")},
        "call": {"function": function, "arguments": [{"parameter": "a", "value": value}]},
    }});
    std::fs::write(
        directory.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.path().join("request.json"))
}

fn completed(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    document["outcome"]["value"].clone()
}

fn assert_call_refusal(output: &Output, code: &str, cause: &str, path: &str) {
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["stage"], "call", "{failure}");
    assert_eq!(failure["code"], code, "{failure}");
    assert_eq!(failure["cause"], cause, "{failure}");
    assert_eq!(failure["details"]["path"], path, "{failure}");
}

// --- the ConfigVersion spine -------------------------------------------

fn spine_source(file: &str, document: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:fr-650:{file}"),
        "document": document, "formal_revision": 1})
}

fn population() -> String {
    format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)
}

fn child() -> Value {
    json!({"population": population(), "key": "child"})
}

fn spine_dir(functions: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = std::fs::read_to_string(directory.path().join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.path().join("spine-unit.native"),
        format!("{unit}{functions}"),
    )
    .unwrap();
    directory
}

fn spine_request(directory: &Path, member: &str, body: Value) -> Output {
    let mut inner = json!({
        "models": [{"format": "semantic-ir/2.0.0", "source":
            spine_source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": spine_source("spine-unit.native", "SpineUnit")},
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

fn spine_call(directory: &Path, function: &str, arguments: Value) -> Output {
    spine_request(
        directory,
        "call",
        json!({"function": function, "snapshot": "spine-snapshot.json", "arguments": arguments}),
    )
}

const SAME: &str = "function same using v(r: Config::ConfigVersion): \
                    Config::ConfigVersion pure { r }\n";

#[test]
#[trace("TC-2370", "FR-650-AC-9")]
fn a_reference_result_spells_structural_identities() {
    let directory = spine_dir(SAME);
    let first = completed(&spine_call(
        directory.path(),
        "same",
        json!([{"parameter": "r", "value": {"reference": child()}}]),
    ));
    let triple = &first["reference"];
    let universe = triple["universe"].as_object().expect("a universe object");
    let mut members: Vec<_> = universe.keys().cloned().collect();
    members.sort();
    assert_eq!(members, ["ordinal", "package", "path"], "{first}");
    assert!(universe["ordinal"].is_u64(), "{first}");
    let kind = triple["type"].as_object().expect("a type object");
    assert_eq!(kind.len(), 2, "{first}");
    assert!(kind["owner_effective_type"].is_null(), "{first}");
    let original = kind["original"].as_object().expect("an original key");
    assert_eq!(original.len(), 2, "{first}");
    assert!(original["package"].is_string() && original["node"].is_string());
    assert_eq!(triple["object_identity"], "child");
    let again = completed(&spine_call(
        directory.path(),
        "same",
        json!([{"parameter": "r", "value": first.clone()}]),
    ));
    assert_eq!(again, first);

    let mut hex = first.clone();
    hex["reference"]["universe"] = json!("0".repeat(64));
    assert_call_refusal(
        &spine_call(
            directory.path(),
            "same",
            json!([{"parameter": "r", "value": hex}]),
        ),
        "invalid_runtime_input",
        "wrong-value-kind",
        "/reference/universe",
    );
    let mut extra = first;
    extra["reference"]["type"]["x"] = json!(1);
    assert_call_refusal(
        &spine_call(
            directory.path(),
            "same",
            json!([{"parameter": "r", "value": extra}]),
        ),
        "invalid_runtime_input",
        "unknown-member",
        "/reference/type/x",
    );
}

const QUANTITY_UNIT: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
function id_q using v(a: Quantity<ms>): Quantity<ms> pure { a }
"#;

/// The same unit with a bounded quantity: S1 does not parse
/// `Quantity<u>[min, max]` yet (QSL-2171).
const BOUNDED_QUANTITY_UNIT: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
function id_bq using v(a: Quantity<ms>[0 ms, 10 ms]): Quantity<ms>[0 ms, 10 ms] pure { a }
"#;

fn quantity(unit: &str, magnitude: Value) -> Value {
    json!({"kind": "quantity", "unit": unit, "magnitude": magnitude})
}

fn rational(numerator: &str, denominator: &str) -> Value {
    json!({"kind": "rational", "numerator": numerator, "denominator": denominator})
}

#[test]
#[trace("TC-2371", "FR-650-AC-10")]
fn a_quantity_round_trips_normalized() {
    let decimal = json!({"kind": "decimal", "coefficient": "150", "scale": "2"});
    assert_eq!(
        completed(&run_unit(
            QUANTITY_UNIT,
            "id_q",
            quantity("ms", decimal.clone())
        )),
        quantity("ms", rational("3", "2"))
    );
    assert_call_refusal(
        &run_unit(QUANTITY_UNIT, "id_q", quantity("s", decimal)),
        "invalid_runtime_input",
        "wrong-value-kind",
        "/unit",
    );
}

#[test]
#[ignore = "QSL-2171: S1 does not parse Quantity<u>[min, max] yet"]
#[trace("TC-2371", "FR-650-AC-10")]
fn a_bounded_quantity_refuses_out_of_range_bounds() {
    let ten = quantity("ms", rational("10", "1"));
    assert_eq!(
        completed(&run_unit(BOUNDED_QUANTITY_UNIT, "id_bq", ten.clone())),
        ten
    );
    assert_call_refusal(
        &run_unit(
            BOUNDED_QUANTITY_UNIT,
            "id_bq",
            quantity("ms", rational("11", "1")),
        ),
        "invalid_runtime_input",
        "invalid-value",
        "/magnitude",
    );
}

const MAP_UNIT: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
function id_map using v(a: Map<String, Int[0, 9]>[0, 2]): Map<String, Int[0, 9]>[0, 2] pure { a }
"#;

fn entry(key: &str, value: u8) -> Value {
    json!({"key": {"kind": "string", "text": key}, "value": {"integer": value.to_string()}})
}

#[test]
#[ignore = "QSL-2171: S1 does not parse Map<K, V>[a, b] and filament_exact has no map value yet"]
#[trace("TC-2372", "FR-650-AC-11")]
#[trace("FSpec-TC-1720", "FSpec-FR-181-AC-31")]
fn a_map_round_trips_in_key_order() {
    assert_eq!(
        completed(&run_unit(
            MAP_UNIT,
            "id_map",
            json!({"map": [entry("b", 2), entry("a", 1)]})
        )),
        json!({"map": [entry("a", 1), entry("b", 2)]})
    );
    let mut no_value = entry("a", 1);
    no_value.as_object_mut().unwrap().remove("value");
    let mut extra = entry("a", 1);
    extra["extra"] = json!(true);
    for (value, code, cause, path) in [
        (
            json!({"map": [entry("a", 1), entry("a", 2)]}),
            "invalid_runtime_input",
            "duplicate-member",
            "/map/1/key",
        ),
        (
            json!({"map": [entry("a", 1), entry("b", 2), entry("c", 3)]}),
            "cardinality_out_of_bound",
            "above-maximum",
            "",
        ),
        (
            json!({"map": [no_value]}),
            "invalid_runtime_input",
            "missing-member",
            "/map/0",
        ),
        (
            json!({"map": [extra]}),
            "invalid_runtime_input",
            "unknown-member",
            "/map/0/extra",
        ),
    ] {
        assert_call_refusal(&run_unit(MAP_UNIT, "id_map", value), code, cause, path);
    }
}

/// The spine model with a variant type `Tier` and a record value type
/// `Stamp` added.
fn extend_model(directory: &Path) {
    let path = directory.join("spine-model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let package = spine::PACKAGE_IDENTITY;
    let module = model["constructs"][0]["kind"]["module"].clone();
    let id = |name: &str| format!("ix://{package}/{name}");
    let generated = |name: &str| {
        json!({"generated": {"generatorIdentity": id(name), "generatorVersion": "1.0.0",
            "inputIdentities": [id(name)]}})
    };
    let construct = |name: &str, meaning: &str, shape: &str, members: Value| {
        json!({"kind": {"module": module, "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": shape, "members": members,
                "meaning": meaning}})
    };
    let constructs = model["constructs"].as_array_mut().unwrap();
    constructs.push(construct(
        "variant_type",
        "filament.meaning.model.variant-type/v1",
        "enumeration",
        json!({"variants": "required"}),
    ));
    constructs.push(construct(
        "record_value_type",
        "filament.meaning.model.record-value-type/v1",
        "record",
        json!({}),
    ));
    let case = |name: &str, line: u64| {
        json!({"identity": id(&format!("variant/Tier-{name}")), "name": name,
            "origin": {"source": {"sourceIdentity": id("spec"), "path": "spec.fspec",
                "startLine": line, "startColumn": 3}}})
    };
    let field = json!({"identity": id("Stamp/n"), "name": "n",
        "typeRef": "ix://filament/native/Integer", "presence": "required",
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": generated("Stamp/n")});
    let definition = |name: &str, kind: &str| {
        json!({"identity": id(name), "displayName": name,
            "kind": {"module": module, "name": kind}, "roles": [], "origin": generated(name),
            "constraints": [], "extensions": [], "unknownPolicy": "reject"})
    };
    let mut tier = definition("Tier", "variant_type");
    tier["variants"] = json!([case("Gold", 1), case("Silver", 2)]);
    let mut stamp = definition("Stamp", "record_value_type");
    stamp["supertypes"] = json!([]);
    stamp["fields"] = json!([field]);
    let types = model["types"].as_array_mut().unwrap();
    types.push(tier);
    types.push(stamp);
    std::fs::write(&path, serde_json::to_vec(&model).unwrap()).unwrap();
}

#[test]
#[trace("TC-2373", "FR-650-AC-12")]
fn model_declared_names_are_ix_identities() {
    let directory = spine_dir(
        "function id_tier using v(a: Config::Tier): Config::Tier pure { a }\n\
         function id_stamp using v(a: Config::Stamp): Config::Stamp pure { a }\n",
    );
    extend_model(directory.path());
    let tier_name = format!("ix://{}/Tier", spine::PACKAGE_IDENTITY);
    let gold = json!({"variant": {"type": tier_name, "case": "Gold"}});
    let stamp = json!({"record": {"type": format!("ix://{}/Stamp", spine::PACKAGE_IDENTITY),
        "fields": {"n": {"integer": "3"}}}});
    let call = |function: &str, value: Value| {
        spine_call(
            directory.path(),
            function,
            json!([{"parameter": "a", "value": value}]),
        )
    };
    assert_eq!(completed(&call("id_tier", gold.clone())), gold);
    assert_eq!(completed(&call("id_stamp", stamp.clone())), stamp);
    assert_call_refusal(
        &call(
            "id_tier",
            json!({"variant": {"type": "Tier", "case": "Gold"}}),
        ),
        "invalid_runtime_input",
        "wrong-value-kind",
        "/variant/type",
    );
    assert_call_refusal(
        &call(
            "id_tier",
            json!({"variant": {"type": tier_name, "case": "variant/Tier-Gold"}}),
        ),
        "invalid_runtime_input",
        "invalid-value",
        "/variant/case",
    );
}

/// The JSON nesting depth of `value`, counted without recursion.
fn depth(value: &Value) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(value, 1)];
    while let Some((node, level)) = stack.pop() {
        deepest = deepest.max(level);
        match node {
            Value::Array(items) => stack.extend(items.iter().map(|item| (item, level + 1))),
            Value::Object(members) => stack.extend(members.values().map(|item| (item, level + 1))),
            _ => {}
        }
    }
    deepest
}

#[test]
#[trace("TC-2374", "FR-650-AC-13")]
fn a_deep_value_round_trips_and_null_refuses() {
    const LEVELS: usize = 60;
    let mut type_text = String::from("Int[0, 1]");
    let mut value = json!({"integer": "1"});
    for _ in 0..LEVELS {
        type_text = format!("Sequence<{type_text}>[0, 1]");
        value = json!({"sequence": [value]});
    }
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         type Deep = {type_text};\n\
         function id_deep using v(a: Deep): Deep pure {{ a }}\n"
    );
    assert!(depth(&value) > 100);
    assert_eq!(completed(&run_unit(&unit, "id_deep", value.clone())), value);

    let record_unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         record P { x: Int[0, 9]; y: Int[0, 9]; }\n\
         function id_rec using v(a: P): P pure { a }\n";
    assert_call_refusal(
        &run_unit(
            record_unit,
            "id_rec",
            json!({"record": {"type": "P", "fields": {"x": {"integer": "1"}, "y": null}}}),
        ),
        "invalid_runtime_input",
        "wrong-value-kind",
        "/record/fields/y",
    );
}

#[test]
#[trace("TC-2375", "FR-650-AC-14")]
fn a_population_result_is_a_set_of_references() {
    let directory = spine_dir(
        "function members using v(p: Population<Config::ConfigVersion>[4]): \
         Population<Config::ConfigVersion>[4] pure { p }\n",
    );
    let value = completed(&spine_call(
        directory.path(),
        "members",
        json!([{"parameter": "p", "value": {"population": population()}}]),
    ));
    let members = value["set"].as_array().expect("a set");
    assert_eq!(members.len(), 2, "{value}");
    let identities: Vec<_> = members
        .iter()
        .map(|member| member["reference"]["object_identity"].clone())
        .collect();
    assert!(identities.contains(&json!("root")) && identities.contains(&json!("child")));
    let bytes: Vec<Vec<u8>> = members
        .iter()
        .map(|member| serde_json::to_vec(member).unwrap())
        .collect();
    assert!(bytes[0] < bytes[1], "{value}");
}

#[test]
#[trace("TC-2376", "FR-650-AC-15")]
fn a_foreign_universe_refuses_and_call_selection_snapshot() {
    let directory = spine_dir(SAME);
    let mut foreign = completed(&spine_call(
        directory.path(),
        "same",
        json!([{"parameter": "r", "value": {"reference": child()}}]),
    ));
    foreign["reference"]["universe"] =
        json!({"package": "other/package", "path": ["ConfigVersion"], "ordinal": 0});
    let output = spine_call(
        directory.path(),
        "sameIdentity",
        json!([{"parameter": "a", "value": foreign},
            {"parameter": "b", "value": {"reference": child()}}]),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "foreign_reference", "{failure}");
    assert_eq!(failure["cause"], "foreign-universe", "{failure}");
    assert_eq!(failure["stage"], "call", "{failure}");
    assert_eq!(failure["details"]["position"], 0, "{failure}");
}

#[test]
#[trace("TC-2376", "FR-650-AC-15")]
fn a_call_selection_snapshot_keys_the_items_universe() {
    let directory = spine_dir(SAME);
    let output = spine_request(
        directory.path(),
        "items",
        json!({"observations": {"current": "spine-snapshot.json"}, "selections": [
            {"call": {"function": "sameIdentity", "snapshot": "current", "arguments": [
                {"parameter": "a", "value": {"reference": child()}},
                {"parameter": "b", "value": {"reference": child()}}]}}]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        document["items"][0]["result"]["outcome"],
        json!({"kind": "completed", "value": {"boolean": true}}),
        "{document}"
    );
}

/// Check the spine unit extended with `functions` through `run` of
/// `function`, returning the refusal envelope.
fn spine_refusal(functions: &str, function: &str, arguments: Value) -> (Option<i32>, Value) {
    let directory = spine_dir(functions);
    let output = spine_call(directory.path(), function, arguments);
    let failure = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
    (output.status.code(), failure)
}

#[test]
#[trace("TC-1705", "FR-650-AC-6")]
fn a_collection_over_a_population_refuses_at_check_and_every_other_type_has_a_form() {
    for (functions, function, arguments) in [
        (
            "function gives using v(): Sequence<Population<Config::ConfigVersion>[1]>[0, 1] pure { sequence[] }\n",
            "gives",
            json!([]),
        ),
        (
            "function takes using v(a: Sequence<Population<Config::ConfigVersion>[1]>[0, 1]): Boolean pure { true }\n",
            "takes",
            json!([{"parameter": "a", "value": {"sequence": []}}]),
        ),
    ] {
        let (code, failure) = spine_refusal(functions, function, arguments);
        assert_eq!(code, Some(20), "{function}: {failure}");
        assert_eq!(failure["code"], "ill_typed", "{function}: {failure}");
        assert_eq!(failure["cause"], "operator-ineligible", "{function}: {failure}");
        assert_ne!(failure["stage"], "call", "{function}: {failure}");
    }
    let output = run_unit(QUANTITY_UNIT, "id_q", quantity("ms", rational("1", "1")));
    assert_ne!(output.status.code(), Some(21), "{output:?}");
    let output = run_unit(MAP_UNIT, "id_map", json!({"map": []}));
    assert_ne!(output.status.code(), Some(21), "{output:?}");
    let directory = spine_dir(SAME);
    let output = spine_call(
        directory.path(),
        "same",
        json!([{"parameter": "r", "value": {"reference": child()}}]),
    );
    assert_ne!(output.status.code(), Some(21), "{output:?}");
}

/// The package fixture whose `gives` result and `takes` parameter are a
/// collection over a population, as an IR producer can write them.
const POPULATION_COLLECTION: &str = "tests/fixtures/population-collection.checked-package.json";

#[test]
#[trace("TC-1705", "FR-650-AC-6")]
#[ignore = "QSL-2182: a selected package declaring a collection over a population is not refused unknown_required_feature at stage call yet"]
fn a_selected_package_with_a_population_collection_type_refuses_unsupported() {
    let fixture = std::fs::canonicalize(POPULATION_COLLECTION)
        .unwrap_or_else(|error| panic!("the fixture {POPULATION_COLLECTION} exists: {error}"));
    for (function, arguments, details) in [
        ("gives", json!([]), json!({"function": "gives"})),
        (
            "takes",
            json!([{"parameter": "a", "value": {"sequence": []}}]),
            json!({"position": 0}),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let request = json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0", "source":
                spine_source("spine-model.semantic-ir.json", "SpineModel")}],
            "package": {"file": fixture.to_string_lossy()},
            "call": {"function": function, "arguments": arguments},
        }});
        spine::write(directory.path(), Case::Healthy).unwrap();
        let path = directory.path().join("request.json");
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let output = run_file(&path);
        assert_eq!(output.status.code(), Some(21), "{function}: {output:?}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], "call", "{failure}");
        assert_eq!(failure["code"], "unknown_required_feature", "{failure}");
        assert_eq!(failure["cause"], "unsupported-feature", "{failure}");
        assert_eq!(failure["details"], details, "{failure}");
    }
}

/// FR-945-AC-1 and AC-2: a native-run/1 call over a `package` selection
/// resolves its function in the package; a missing function refuses at the
/// call, and `program` beside `package` refuses at the request.
#[test]
#[trace("TC-2625", "FR-945-AC-1")]
#[trace("TC-2625", "FR-945-AC-2")]
#[ignore = "QSL-2182: native-run/1 does not run a function of a selected checked package yet"]
fn a_run_resolves_its_function_in_a_selected_package() {
    let fixture = std::fs::canonicalize(POPULATION_COLLECTION)
        .unwrap_or_else(|error| panic!("the fixture {POPULATION_COLLECTION} exists: {error}"));
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let models = json!([{"format": "semantic-ir/2.0.0", "source":
        spine_source("spine-model.semantic-ir.json", "SpineModel")}]);
    let run = |request: Value| {
        let path = directory.path().join("request.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&json!({"format": "native-run/1", "request": request})).unwrap(),
        )
        .unwrap();
        let output = run_file(&path);
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap_or(Value::Null);
        (output.status.code(), failure)
    };
    let (code, failure) = run(json!({"models": models,
        "package": {"file": fixture.to_string_lossy()},
        "call": {"function": "missing", "arguments": []}}));
    assert_eq!(code, Some(20), "{failure}");
    assert_eq!(failure["stage"], "call", "{failure}");
    assert_eq!(failure["code"], "missing_declaration", "{failure}");
    let (code, failure) = run(json!({"models": models,
        "package": {"file": fixture.to_string_lossy()},
        "program": {"source": {"file": "spine-unit.native"}},
        "call": {"function": "gives", "arguments": []}}));
    assert_eq!(code, Some(20), "{failure}");
    assert_eq!(failure["stage"], "request", "{failure}");
    assert!(failure.to_string().contains("program"), "{failure}");
}

/// FR-650 "Unit names": a clause `function` selection reads its arguments
/// in the run value form against the compiled package, so a Quantity
/// written in a unit the program's source declares binds.
#[test]
#[trace("TC-2371", "FR-650-AC-10")]
fn a_function_selection_reads_a_quantity_in_a_source_declared_unit() {
    let directory = spine_dir(
        "dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         function short using v(a: Quantity<ms>): Boolean pure { true }\n",
    );
    let output = spine_request(
        directory.path(),
        "clause",
        json!({
            "selection": {"function": {"name": "short", "snapshot": "current", "arguments": [
                {"parameter": "a", "value": quantity("ms", rational("3", "2"))}]}},
            "observations": {"current": "spine-snapshot.json"},
        }),
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(!text.contains("unknown-unit"), "{text}");
}
