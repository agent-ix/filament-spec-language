// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640-AC-7 and AC-9: a clause over a model subject derives its clock
//! configuration under every profile, and library units read the request's
//! one `clocks` member.
use crate::command_harness::compile_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MODEL: &[u8] = include_bytes!("../../examples/config-version/model.semantic-ir.json");

fn source(file: &str, identity: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": identity,
        "document": "Clocks", "formal_revision": 1})
}

fn write(directory: &Path, file: &str, text: &[u8]) {
    std::fs::write(directory.join(file), text).unwrap();
}

fn compile(directory: &Path, request: &Value) -> crate::command_harness::Output {
    let path: PathBuf = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    compile_file(&path)
}

/// The value of the first `key` member anywhere in `value`.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members
            .get(key)
            .or_else(|| members.values().find_map(|member| find(member, key))),
        Value::Array(items) => items.iter().find_map(|item| find(item, key)),
        _ => None,
    }
}

#[trace("TC-2181", "FR-640-AC-7")]
#[test]
#[ignore = "QSL-1964: derived clock configurations for fixed-sample and timestamped-event model clauses not implemented yet"]
fn model_subject_clauses_derive_their_clock_configuration() {
    let directory = tempfile::tempdir().unwrap();
    write(directory.path(), "model.json", MODEL);
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile f = \"filament.temporal.fixed-sample.false-extension/v1\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal Sampled using f over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { eventually[0,3] holds(c.versionNumber >= 0) }\n\
         temporal Stamped using w over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { eventually[0,3] holds(c.versionNumber >= 0) }\n";
    write(directory.path(), "unit.native", unit.as_bytes());
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("model.json", "test:clocks-model")}],
        "program": {"source": source("unit.native", "test:clocks-unit")}}});
    let output = compile(directory.path(), &request);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    let rows = find(&package, "temporal_bindings")
        .and_then(Value::as_array)
        .expect("temporal binding rows");
    let clocks: Vec<&Value> = rows.iter().map(|row| &row["clock"]).collect();
    assert!(clocks.contains(&&json!({"kind": "fixed_sample",
        "epoch": {"kind": "integer", "decimal": "0"},
        "period": {"kind": "integer", "decimal": "1"}, "unit": "step"})));
    assert!(clocks.contains(&&json!({"kind": "timestamped_event", "timestamp_unit": "step"})));
}

#[trace("TC-2183", "FR-640-AC-9")]
#[test]
#[ignore = "QSL-1964: the request `clocks` member serving library units not implemented yet"]
fn library_units_share_the_request_clocks() {
    let directory = tempfile::tempdir().unwrap();
    let library = "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal Late using w over (r: Reading) clock \"plant-ms\" \
         on origin { eventually[0,5] holds(r.level > 0) }\n";
    let program = "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         import \"test/plant\" as P;\n\
         temporal Early using w over (r: P::Reading) clock \"plant-ms\" \
         on origin { eventually[0,2] holds(r.level > 0) }\n";
    write(directory.path(), "plant.native", library.as_bytes());
    write(directory.path(), "unit.native", program.as_bytes());
    let request = |clocks: Option<Value>| {
        let mut request = json!({"format": "native-compile/1", "request": {
            "models": [],
            "program": {"source": source("unit.native", "test:clocks-program")},
            "libraries": [{"identity": "test/plant",
                "source": source("plant.native", "test:clocks-plant")}]}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        request
    };
    let served = compile(
        directory.path(),
        &request(Some(
            json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}}),
        )),
    );
    assert_eq!(
        served.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&served.stderr)
    );
    let missing = compile(directory.path(), &request(None));
    assert_eq!(missing.status.code(), Some(20));
    let failure: Value = serde_json::from_slice(&missing.stderr).expect("a refusal");
    assert_eq!(
        find(&failure, "cause"),
        Some(&json!("missing-clock-selection"))
    );
    assert_eq!(find(&failure, "clock"), Some(&json!("plant-ms")));
}

#[trace("TC-2212", "FR-640-AC-10", "FSpec-TC-1661", "FSpec-FR-322-AC-452")]
#[test]
#[ignore = "QSL-2274: the program package does not record its libraries' clock selections yet"]
fn the_program_package_records_its_library_clock_selections() {
    let directory = tempfile::tempdir().unwrap();
    let library = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         function flag using v(): Boolean pure { true }\n\
         temporal Late using w over (r: Reading) clock \"lib-ms\" \
         on origin { eventually[0,5] holds(r.level > 0) }\n";
    let plain = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function flag using v(): Boolean pure { true }\n";
    let program = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         import \"test/plant\" as P;\n\
         function entry using v(): Boolean pure { P::flag() }\n";
    write(directory.path(), "plant.native", library.as_bytes());
    write(directory.path(), "plain.native", plain.as_bytes());
    write(directory.path(), "unit.native", program.as_bytes());
    let request = |file: &str, clocks: Option<Value>| {
        let mut request = json!({"format": "native-compile/1", "request": {
            "models": [],
            "program": {"source": source("unit.native", "test:clocks-program")},
            "libraries": [{"identity": "test/plant",
                "source": source(file, "test:clocks-plant")}]}});
        if let Some(clocks) = clocks {
            request["request"]["clocks"] = clocks;
        }
        request
    };
    let compiled = |unit: &str| {
        let output = compile(
            directory.path(),
            &request(
                "plant.native",
                Some(json!({"lib-ms": {"kind": "timestamped_event", "timestamp_unit": unit}})),
            ),
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let ms = compiled("ms");
    let package: Value = serde_json::from_slice(&ms).expect("package JSON");
    assert_eq!(
        package["lock"]["dependency_clock_selections"],
        json!([{"clock_name": "lib-ms",
            "clock": {"kind": "timestamped_event", "timestamp_unit": "ms"}}])
    );
    assert_ne!(ms, compiled("s"));

    let output = compile(directory.path(), &request("plain.native", None));
    assert_eq!(output.status.code(), Some(0));
    let package: Value = serde_json::from_slice(&output.stdout).expect("package JSON");
    assert!(package["lock"].get("dependency_clock_selections").is_none());
}
