// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-810-AC-1, AC-2 (TC-2297): a compile request's `output` member writes
//! the FSpec FR-322 checked package to a new file, and every refusal writes
//! none. FR-810-AC-3's `RequestOverrides::output` and the driver's
//! `--output` are the coder's to add and test.

use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn request(directory: &Path, program: &str, output: Value) -> PathBuf {
    std::fs::copy(program, directory.join("program.native")).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "agent-ix",
            "identity": "test:spine", "document": "Spine", "formal_revision": 1}},
        "output": output
    }});
    let path = directory.join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn refusal(path: &Path) -> (u8, Value) {
    let error = fsl_command::command::compile(path).expect_err("the request refuses");
    (error.category().exit_code(), error.value().unwrap())
}

/// FR-810-AC-1: the written file holds exactly the package bytes.
#[test]
#[trace("TC-2297", "FR-810-AC-1")]
#[ignore = "QSL-2299: the compile request's `output` member is not implemented yet"]
fn a_compile_output_member_writes_the_package_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    let package = fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(
        std::fs::read(directory.path().join("spine.package.json")).unwrap(),
        package
    );
    let outcome = fsl_command::command::compile_outcome(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let document: Value = serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap();
    assert_eq!(document["artifacts"], json!([]));
}

/// FR-810-AC-2: an existing path, a malformed member and a missing parent
/// directory each refuse, and no file is written or changed.
#[test]
#[trace("TC-2297", "FR-810-AC-2")]
#[ignore = "QSL-2299: the compile request's `output` member is not implemented yet"]
fn a_compile_output_member_refuses_and_writes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let existing = directory.path().join("spine.package.json");
    std::fs::write(&existing, b"keep").unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    let (exit, value) = refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert!(value.to_string().contains("spine.package.json"), "{value}");
    assert_eq!(std::fs::read(&existing).unwrap(), b"keep");

    for (output, member) in [
        (json!({"package": ""}), "package"),
        (json!({"path": "x"}), "path"),
        (json!({"package": 3}), "package"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = request(
            directory.path(),
            "tests/fixtures/spine-compile.native",
            output,
        );
        let (exit, value) = refusal(&path);
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(value.to_string().contains(member), "{value}");
        assert!(!directory.path().join("x").exists());
    }

    let directory = tempfile::tempdir().unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "missing/spine.package.json"}),
    );
    let (exit, value) = refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert!(!directory.path().join("missing").exists());
}

/// FR-810-AC-3: the override entry the driver's `--output` reaches writes
/// the same bytes. `RequestOverrides` has no `output` member yet, so this
/// stub drives the entry over a request carrying `output`; the coder adds
/// `RequestOverrides::output` and asserts the override and the doubly
/// supplied refusal here.
#[test]
#[trace("TC-2297", "FR-810-AC-3")]
#[ignore = "QSL-2299: RequestOverrides::output and the compile `output` member are not implemented yet"]
fn the_override_entry_writes_the_same_package_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "cli.package.json"}),
    );
    let outcome = fsl_command::command::compile_outcome_with_overrides(
        &path,
        &fsl_command::command::RequestOverrides::default(),
    )
    .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(
        std::fs::read(directory.path().join("cli.package.json")).unwrap(),
        outcome.package_bytes()
    );
}

/// FR-810-AC-5: `output` and `handoff` are independent writes, and an
/// `output` path inside the handoff directory refuses naming both.
#[test]
#[trace("TC-2297", "FR-810-AC-5")]
#[ignore = "QSL-2299: the compile request's `output` member is not implemented yet"]
fn output_and_handoff_are_independent_writes() {
    let both = |directory: &Path, output: Value| {
        let path = request(directory, "tests/fixtures/spine-compile.native", output);
        let mut document: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        document["request"]["handoff"] = json!({"directory": "handoff", "package_name": "spine",
            "native_join": {"work_units": 1_000_000, "storage": 1_000_000}});
        std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        path
    };
    let directory = tempfile::tempdir().unwrap();
    let path = both(directory.path(), json!({"package": "spine.package.json"}));
    fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(
        std::fs::read(directory.path().join("spine.package.json")).unwrap(),
        std::fs::read(directory.path().join("handoff/checked-package.json")).unwrap()
    );

    let directory = tempfile::tempdir().unwrap();
    let path = both(directory.path(), json!({"package": "handoff/pkg.json"}));
    let (exit, value) = refusal(&path);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    let text = value.to_string();
    assert!(
        text.contains("output") && text.contains("handoff"),
        "{value}"
    );
    assert!(!directory.path().join("handoff").exists());
}
