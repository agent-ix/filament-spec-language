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
