// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100, FR-945, FR-1039 (TC-3019): a `native-run/1` request that selects a
//! checked package in place of a `program` runs its `call` from the package,
//! under the package's recorded integer-division law (FR-767), with the
//! source removed, and refuses a package that cannot run at stage `request`.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::{Value, json};

const FLOOR_HALF: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile d = \"filament.value.integer-division.floor/v1\";\n\
     function half using v(): Integer pure { -7 div 2 }\n\
     function twice using v(a: Int[-7, 7]): Int[-14, 14] pure { a + a }\n";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:package-run:{file}"),
        "document": "PackageRun", "formal_revision": 1})
}

/// Compiles `unit` in `directory` to `package.json` and the `handoff`
/// directory, then removes the source.
fn compile(directory: &Path, unit: &str) {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let request = json!({"models": [], "program": {"source": source("unit.native")},
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": "package-run-unit",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}});
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path).expect("the unit compiles");
    std::fs::remove_file(directory.join("unit.native")).unwrap();
}

/// Runs `request` as a `native-run/1` request in `directory`: the exit code
/// and the result or refusal document.
fn run(directory: &Path, request: Value) -> (u8, Value) {
    let path = directory.join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-run/1", "request": request})).unwrap(),
    )
    .unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

/// FR-1039-AC-1, FR-100-AC-22, FR-945-AC-1: the package's recorded `floor`
/// law evaluates `-7 div 2` to `-4` from the file and from the handoff
/// directory with the source removed, the result echoing the selection;
/// `integer_division` beside `package`, `program` beside `package`, and a
/// request with neither selection refuse at the request.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
#[trace("TC-3019", "FR-100-AC-22")]
#[trace("TC-2625", "FR-945-AC-1")]
fn a_package_only_call_runs_under_the_recorded_law_without_source() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    let call = json!({"function": "half", "arguments": []});
    for package in [
        json!({"file": "package.json"}),
        json!({"directory": "handoff"}),
    ] {
        let (exit, value) = run(
            directory.path(),
            json!({"models": [], "package": package, "call": call}),
        );
        assert_eq!(exit, 0, "{value}");
        assert_eq!(
            value["outcome"]["value"],
            json!({"integer": "-4"}),
            "{value}"
        );
        assert_eq!(value["package"], package, "{value}");
        assert!(value.get("source").is_none(), "{value}");
    }
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "call": {"function": "twice", "arguments": [{"parameter": "a", "value": -7}]}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["outcome"]["value"],
        json!({"integer": "-14"}),
        "{value}"
    );

    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "absent.json"},
            "integer_division": "floor", "call": call}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "integer_division");
    assert!(!value.to_string().contains("absent.json"), "{value}");
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "program": {"source": source("unit.native")}, "call": call}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    names(&value, "program");
    let (exit, value) = run(directory.path(), json!({"models": [], "call": call}));
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    names(&value, "program");
}

/// FR-1039: native-run's multi-selection wrapper executes each package call
/// from the checked artifact after the source used to produce it is removed.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
fn package_call_items_run_from_the_checked_artifact_without_source() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"directory": "handoff"},
        "items": {"selections": [
            {"call": {"function": "half", "arguments": []}},
            {"call": {"function": "twice", "arguments": [
                {"parameter": "a", "value": -7}
            ]}}
        ]}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["items"][0]["result"]["outcome"]["value"],
        json!({"integer": "-4"}),
        "{value}"
    );
    assert_eq!(
        value["items"][1]["result"]["outcome"]["value"],
        json!({"integer": "-14"}),
        "{value}"
    );
}

/// FR-1039-AC-2: a package synthesis still refuses until QSL-2235 lowers the
/// checked synthesis's S1-S3 records; the refusal precedes package reads.
#[test]
#[trace("TC-3019", "FR-1039-AC-2")]
fn package_synthesis_refusal_names_qsl_2235_before_reading_the_package() {
    let directory = tempfile::tempdir().unwrap();
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "absent.json"},
            "synthesis": {"selection": "planned"},
            "call": {"function": "half", "arguments": []}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    names(&value, "synthesis");
    names(&value, "QSL-2235");
    assert!(!value.to_string().contains("absent.json"), "{value}");
}

/// FR-100 "Request-file preparation for the driver": preparing a package
/// request performs no call, and the prepared package call runs to the same
/// value as `run`.
#[test]
#[trace("TC-3019", "FR-100-AC-22")]
fn a_prepared_package_call_runs_through_its_executor() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-run/1", "request": {"models": [],
            "package": {"file": "package.json"},
            "call": {"function": "half", "arguments": []}}}))
        .unwrap(),
    )
    .unwrap();
    let prepared =
        fsl_command::command::prepare_run(&path, fsl_command::command::RunLimits::default())
            .unwrap_or_else(|error| panic!("the package request prepares: {error}"));
    let fsl_command::command::PreparedRun::Package(prepared) = prepared else {
        panic!("a package request prepares a package call");
    };
    let result = fsl_command::command::execute_package_run(*prepared).expect("the call runs");
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        result.value.as_value()["outcome"]["value"],
        json!({"integer": "-4"}),
        "{}",
        result.value
    );
}

/// FR-945-AC-2: a function the package does not declare refuses
/// `missing_declaration` at stage `call`.
#[test]
#[trace("TC-2625", "FR-945-AC-2")]
fn a_package_call_names_a_declared_function() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "call": {"function": "missing", "arguments": []}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "call", "{value}");
    assert_eq!(value["code"], "missing_declaration", "{value}");
}

/// FR-1039-AC-3: a missing file, a non-v2 document, a package the strict
/// reader refuses and a `models` entry the package records no selection of
/// each refuse at stage `request`, exit 20, with no fallback to source.
#[test]
#[trace("TC-3019", "FR-1039-AC-3")]
fn a_package_that_cannot_run_refuses_at_the_request() {
    let directory = tempfile::tempdir().unwrap();
    crate::config_version_spine::spine::write(
        directory.path(),
        crate::config_version_spine::Case::Healthy,
    )
    .unwrap();
    compile(directory.path(), FLOOR_HALF);
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
        ("absent.json", json!([]), "invalid-request", "absent.json"),
        (
            "v1.json",
            json!([]),
            "unknown-contract-version",
            "/request/package",
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
        assert_eq!(value["stage"], "request", "{file}: {value}");
        names(&value, code);
        names(&value, locus);
    }
}
