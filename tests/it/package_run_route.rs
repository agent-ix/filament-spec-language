// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1039 (TC-3019) and FR-100-AC-22: a `native-run/1` request that
//! selects a checked package in place of a `program` runs from the package's
//! own `lock` and name rows, never reads or compiles source, and refuses a
//! package that cannot run at stage `request` with the code and locus stated.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::{json, Value};

const HALF: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     function half using v(): Integer pure { -7 div 2 }\n";

const PICK: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     type Digit = Int[0, 9];\n\
     synthesis Pick using v grammar Digit domain Digit satisfies { Pick * Pick = 49 };\n";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w60-package:{file}"),
        "document": "W60Package", "formal_revision": 1})
}

/// Compiles `unit` in `directory` with the extra compile `members`, writing
/// `package.json` and the `handoff` directory, then removes the source.
fn compile(directory: &Path, unit: &str, members: Value) {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let mut request = json!({"models": [], "program": {"source": source("unit.native")},
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": "w60-unit",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}});
    for (key, value) in members.as_object().into_iter().flatten() {
        request[key] = value.clone();
    }
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path).expect("the unit compiles");
    std::fs::remove_file(directory.join("unit.native")).unwrap();
}

/// Runs `request` as a `native-run/1` request in `directory`.
fn run(directory: &Path, request: Value) -> (u8, Value) {
    let path = directory.join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-run/1", "request": request})).unwrap(),
    )
    .unwrap();
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

/// FR-1039-AC-1 and FR-100-AC-22: the package's recorded `floor` law
/// evaluates `-7 div 2` to `-4` with the source removed; `integer_division`
/// beside `package` and a request with neither selection refuse.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
#[trace("TC-3019", "FR-100-AC-22")]
fn a_package_only_run_uses_the_recorded_law_without_source() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), HALF, json!({"integer_division": "floor"}));
    let call = json!({"function": "half", "arguments": []});
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"}, "call": call}),
    );
    assert_eq!(exit, 0, "{value}");
    names(&value, "-4");
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "absent.json"},
            "integer_division": "floor", "call": call}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "integer_division");
    assert!(!value.to_string().contains("absent.json"), "{value}");
    let (exit, value) = run(directory.path(), json!({"models": [], "call": call}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "program");
}

/// FR-1039-AC-2: a value synthesis runs from the handoff directory and from
/// the package file; a `clause` member over a file selection needs the
/// directory.
#[test]
#[trace("TC-3019", "FR-1039-AC-2")]
#[ignore = "FR-754: the synthesis lowering is not installed, so the package this run selects cannot be compiled (SynthesisNotChecked)"]
fn a_package_run_selects_its_form_by_the_member() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), PICK, json!({}));
    for package in [
        json!({"directory": "handoff"}),
        json!({"file": "package.json"}),
    ] {
        let (exit, value) = run(
            directory.path(),
            json!({"models": [], "package": package, "synthesis": {"declaration": "Pick"}}),
        );
        assert_eq!(exit, 0, "{value}");
        assert_eq!(
            value["synthesis"]["candidate"],
            json!({"type": "integer", "value": "7"}),
            "{value}"
        );
    }
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "clause": {"selection": {"clause": "Pick"}, "observations": {}}}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "\"requires\":\"directory\"");
}

/// FR-1039-AC-3: a missing file, a non-v2 document, a package the reader
/// refuses and a `models` entry the package does not record each refuse at
/// stage `request` with the stated code.
#[test]
#[trace("TC-3019", "FR-1039-AC-3")]
#[ignore = "FR-1039 names code unknown_contract_version, which the closed diagnostic catalogue (fsl-foundation CATALOG_CATEGORIES) does not hold; spec decision needed"]
fn a_package_that_cannot_run_refuses_at_the_request() {
    let directory = tempfile::tempdir().unwrap();
    crate::support::config_version_spine::spine::write(
        directory.path(),
        crate::support::config_version_spine::Case::Healthy,
    )
    .unwrap();
    compile(directory.path(), HALF, json!({"integer_division": "floor"}));
    let mut wire: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("package.json")).unwrap())
            .unwrap();
    wire["contract_version"] = json!("filament.checked-package/v1");
    std::fs::write(directory.path().join("v1.json"), wire.to_string()).unwrap();
    wire["contract_version"] = json!("filament.checked-package/v2");
    wire["unexpected"] = json!(true);
    std::fs::write(directory.path().join("extra.json"), wire.to_string()).unwrap();
    let call = json!({"function": "half", "arguments": []});
    let spine_entry = json!([{"format": "semantic-ir/2.0.0",
        "source": source("spine-model.semantic-ir.json")}]);
    for (file, models, code, locus) in [
        ("absent.json", json!([]), "invalid-request", "package"),
        (
            "v1.json",
            json!([]),
            "unsupported-wire",
            "/request/package/contract_version",
        ),
        (
            "extra.json",
            json!([]),
            "invalid_package",
            "/request/package",
        ),
        (
            "package.json",
            spine_entry,
            "invalid-request",
            "/request/models/0",
        ),
    ] {
        let (exit, value) = run(
            directory.path(),
            json!({"models": models, "package": {"file": file}, "call": call}),
        );
        assert_eq!(exit, 20, "{file}: {value}");
        names(&value, code);
        names(&value, locus);
        names(&value, "request");
    }
}
