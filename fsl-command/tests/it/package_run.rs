// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100, FR-945, FR-1039 (TC-3019): a `native-run/1` request that selects a
//! checked package in place of a `program` runs its `call` from the package,
//! under the package's recorded integer-division law (FR-767), with the
//! source removed, and refuses a package that cannot run at stage `request`.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::{json, Value};

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
    compile_named(directory, unit, "package-run-unit");
}

fn compile_named(directory: &Path, unit: &str, package_name: &str) {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let request = json!({"models": [], "program": {"source": source("unit.native")},
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": package_name,
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
            json!({"type":"integer", "value":"-4"}),
            "{value}"
        );
        assert_eq!(value["package"], package, "{value}");
        assert!(value.get("source").is_none(), "{value}");
    }
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "call": {"function": "twice", "arguments": [{"parameter": "a", "value": {"type":"integer","value":"-7"}}]}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["outcome"]["value"],
        json!({"type":"integer", "value":"-14"}),
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
                {"parameter": "a", "value": {"type":"integer","value":"-7"}}
            ]}}
        ]}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["items"][0]["result"]["outcome"]["value"],
        json!({"type":"integer", "value":"-4"}),
        "{value}"
    );
    assert_eq!(
        value["items"][1]["result"]["outcome"]["value"],
        json!({"type":"integer", "value":"-14"}),
        "{value}"
    );
}

/// FR-1039: a package request with two run members refuses before package reads.
#[test]
#[trace("TC-3019", "FR-1039-AC-2")]
fn package_synthesis_conflicts_with_a_call_before_package_read() {
    let directory = tempfile::tempdir().unwrap();
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "absent.json"},
            "synthesis": {"declaration": "Pick"},
            "call": {"function": "half", "arguments": []}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    names(&value, "synthesis");
    names(&value, "mutually exclusive");
    assert!(!value.to_string().contains("absent.json"), "{value}");
}

/// FR-1015: export over a checked package uses the request's exact source
/// identity/version pair and does not need the source file to remain present.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
#[trace("TC-3019", "FR-1039-AC-4")]
fn package_export_uses_its_explicit_source_package_without_source_reads() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    let source_package = json!({"identity": "recorded:package-run-unit", "version": "7"});
    let (exit, value) = run(
        directory.path(),
        json!({"models": [],
            "package": {"file": "package.json", "source_package": source_package},
            "export": {"profile": "filament.output.tla2/v1",
                "obligations": ["not-a-claim"], "minimum_fidelity": "any"}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["package"]["source_package"], source_package,
        "{value}"
    );
    assert!(value.get("source").is_none(), "{value}");
}

/// FR-1015: package export requires explicit source metadata before the
/// package path is read; checked-package/v2 does not retain formal_revision.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
#[trace("TC-3019", "FR-1039-AC-4")]
fn package_export_without_source_package_refuses_before_package_read() {
    let directory = tempfile::tempdir().unwrap();
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "absent.json"},
            "export": {"profile": "filament.output.tla2/v1",
                "obligations": ["not-a-claim"]}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    names(&value, "missing-member");
    names(&value, "/request/package/source_package");
    assert!(!value.to_string().contains("absent.json"), "{value}");
}

/// FR-1015: package export reports a malformed source identity/version pair
/// as a typed request refusal before the checked-package path is read.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
#[trace("TC-3019", "FR-1039-AC-4")]
fn package_export_with_malformed_source_package_refuses_before_package_read() {
    let directory = tempfile::tempdir().unwrap();
    for (source_package, cause, path) in [
        (
            json!(null),
            "wrong-value-kind",
            "/request/package/source_package",
        ),
        (
            json!("not-an-object"),
            "wrong-value-kind",
            "/request/package/source_package",
        ),
        (
            json!({"identity": 42, "version": "7"}),
            "wrong-value-kind",
            "/request/package/source_package/identity",
        ),
        (
            json!({"identity": "recorded:package-run-unit", "version": false}),
            "wrong-value-kind",
            "/request/package/source_package/version",
        ),
        (
            json!({"identity": "recorded:package-run-unit"}),
            "missing-member",
            "/request/package/source_package/version",
        ),
        (
            json!({"identity": "", "version": "7"}),
            "invalid-value",
            "/request/package/source_package/identity",
        ),
        (
            json!({"identity": "recorded:package-run-unit", "version": ""}),
            "invalid-value",
            "/request/package/source_package/version",
        ),
        (
            json!({"identity": "recorded:package-run-unit", "version": "7", "digest": "sha256:abc"}),
            "unknown-member",
            "/request/package/source_package/digest",
        ),
    ] {
        let (exit, value) = run(
            directory.path(),
            json!({"models": [],
                "package": {"file": "absent.json", "source_package": source_package},
                "export": {"profile": "filament.output.tla2/v1",
                    "obligations": ["not-a-claim"]}}),
        );
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        names(&value, cause);
        names(&value, path);
        assert!(!value.to_string().contains("absent.json"), "{value}");
    }
}

/// FR-650: the package owner admits a call snapshot and runs without looking
/// for the source that produced the checked artifact.
#[test]
#[trace("TC-3019", "FR-1039-AC-1")]
fn package_call_snapshot_is_admitted_by_the_v2_package_owner() {
    let directory = tempfile::tempdir().unwrap();
    compile(directory.path(), FLOOR_HALF);
    std::fs::write(
        directory.path().join("snapshot.json"),
        json!({"format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "package-call-snapshot"},
            "observation": "current",
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": []})
        .to_string(),
    )
    .unwrap();
    let (exit, value) = run(
        directory.path(),
        json!({"models": [], "package": {"file": "package.json"},
            "call": {"function": "half", "arguments": [], "snapshot": "snapshot.json"}}),
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["outcome"]["value"],
        json!({"type":"integer", "value":"-4"}),
        "{value}"
    );
    assert!(value.get("source").is_none(), "{value}");
}

/// FR-931-AC-11/12: source TL atoms use one precedence-aware printer over
/// checked expressions, retain the `holds(...)` wrapper, and use alias-free
/// declaration identities for calls and enum literals.
#[test]
#[trace("FR-931-AC-11", "FR-931-AC-12")]
fn source_tl_labels_use_checked_precedence_and_alias_free_declaration_keys() {
    let directory = tempfile::tempdir().unwrap();
    let mut unit = crate::config_version_spine::spine::unit_text();
    unit = unit.replace(
        "profile v = \"filament.model.complete/v1\";",
        "profile v = \"filament.model.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         import \"test/lib\" as library;",
    );
    unit.push_str(
        "enum Mode { On, Off }\n\
         record Pair { left: Integer; right: Integer; }\n\
         temporal Canonical using inf over (x: Config::ConfigVersion) clock \"model-steps\" on origin { once[0,2] (holds(if library::positive(2) then Mode::On = Mode::On else Mode::Off = Mode::Off) and holds(Pair{left: 1, right: 2} = Pair{left: 1, right: 2}) and holds(x.versionNumber <= 2)) }\n",
    );
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    std::fs::write(
        directory.path().join("library.native"),
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function positive using v(n: Int[0, 1000]): Boolean pure { n > 0 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("model.semantic-ir.json"),
        crate::config_version_spine::spine::DOMAIN_PACKAGE,
    )
    .unwrap();

    let request = json!({"models": [{"format": "semantic-ir/2.0.0", "source": {
        "file": "model.semantic-ir.json", "authority": "agent-ix",
        "identity": "example/config-version", "document": "ConfigVersion",
        "formal_revision": 1}}],
        "libraries": [{"identity":"test/lib", "source":source("library.native")}],
        "program": {"source": source("unit.native")},
        "tl_lowering": {"clause": "Canonical"}});
    let (exit, value) = run(directory.path(), request.clone());
    assert_eq!(exit, 0, "{value}");
    let atom = value["tl_lowering"]["atoms"]["p0"]
        .as_str()
        .expect("the atom has canonical checked text");
    assert_eq!(
        atom,
        "holds(if ix://test/lib/positive(2) then ix://test:package-run:unit.native/Mode::On = ix://test:package-run:unit.native/Mode::On else ix://test:package-run:unit.native/Mode::Off = ix://test:package-run:unit.native/Mode::Off)",
        "{value}"
    );
    assert_eq!(
        value["tl_lowering"]["atoms"]["p1"],
        "holds(ix://test:package-run:unit.native/Pair { left: 1, right: 2 } = ix://test:package-run:unit.native/Pair { left: 1, right: 2 })",
        "{value}"
    );
    assert_eq!(
        value["tl_lowering"]["atoms"]["p2"], "holds(x.versionNumber <= 2)",
        "{value}"
    );

    let renamed_unit = unit
        .replace("model Config =", "model C =")
        .replace("Config::", "C::")
        .replace("as library", "as math")
        .replace("library::", "math::");
    std::fs::write(directory.path().join("unit.native"), renamed_unit).unwrap();
    let (renamed_exit, renamed) = run(directory.path(), request);
    assert_eq!(renamed_exit, 0, "{renamed}");
    assert_eq!(
        renamed["tl_lowering"]["atoms"],
        value["tl_lowering"]["atoms"]
    );
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
        json!({"type":"integer", "value":"-4"}),
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
        assert_eq!(value["stage"], "request", "{file}: {value}");
        names(&value, code);
        names(&value, locus);
    }
}

/// The public source and source-free handoff calls preserve source enum
/// arguments, canonical enum results and ordered comparison.
#[test]
#[trace("FR-531-AC-50", "FR-1039-AC-1")]
fn source_enum_package_calls_match_source_canonical_values_without_source() {
    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        enum Status { READY, DONE }\n\
        ordered enum Level { LOW, MID, HIGH }\n\
        function ready using v(): Status pure { Status::READY }\n\
        function echo using v(s: Status): Status pure { s }\n\
        function later using v(): Boolean pure { Level::LOW < Level::HIGH }\n";
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("unit.native"), UNIT).unwrap();
    let request = |call: Value| {
        json!({"models": [],
        "program": {"source": source("unit.native")}, "call": call})
    };
    let ready = json!({"function":"ready", "arguments":[]});
    let (exit, original) = run(directory.path(), request(ready.clone()));
    assert_eq!(exit, 0, "{original}");
    let enum_value = original["outcome"]["value"].clone();
    assert_eq!(enum_value["type"], "enum", "{original}");
    assert_eq!(enum_value["member"], "READY", "{original}");
    let echo = json!({"function":"echo", "arguments":[
        {"parameter":"s", "value": enum_value}]});
    let later = json!({"function":"later", "arguments":[]});
    let mut expected = Vec::new();
    for call in [ready, echo, later] {
        let (exit, value) = run(directory.path(), request(call.clone()));
        assert_eq!(exit, 0, "{value}");
        expected.push((call, value["outcome"]["value"].clone()));
    }
    assert_eq!(expected[1].1, expected[0].1);
    assert_eq!(expected[2].1, json!({"type":"boolean", "value":true}));
    let selected_source = source("unit.native");
    // The source command compiles under its actual identity label. Select
    // that same package for the handoff instead of this file's other fixture name.
    compile_named(
        directory.path(),
        UNIT,
        selected_source["identity"]
            .as_str()
            .expect("source identity label"),
    );
    assert!(!directory.path().join("unit.native").exists());
    for (call, canonical) in expected {
        let (exit, value) = run(
            directory.path(),
            json!({"models":[],
            "package":{"directory":"handoff"}, "call":call}),
        );
        assert_eq!(exit, 0, "{value}");
        assert_eq!(value["outcome"]["value"], canonical, "{value}");
        assert!(value.get("source").is_none(), "{value}");
    }
}
