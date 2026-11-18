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
fn a_compile_output_member_writes_the_package_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    // One compile: a second run over the same request refuses (AC-2).
    let outcome = fsl_command::command::compile_outcome(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(
        std::fs::read(directory.path().join("spine.package.json")).unwrap(),
        outcome.package_bytes()
    );
    let document: Value = serde_json::from_slice(&outcome.document().to_bytes().unwrap()).unwrap();
    assert_eq!(document["artifacts"], json!([]));
}

/// FR-810-AC-2: an existing path, a malformed member and a missing parent
/// directory each refuse, and no file is written or changed.
#[test]
#[trace("TC-2297", "FR-810-AC-2")]
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
    assert_eq!(value["code"], "io-error", "{value}");
    assert!(
        value.to_string().contains("missing/spine.package.json"),
        "{value}"
    );
    assert!(!directory.path().join("missing").exists());

    // A program that does not check writes no file and refuses as it does
    // without `output`.
    let directory = tempfile::tempdir().unwrap();
    let path = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    std::fs::write(directory.path().join("program.native"), b"not a program").unwrap();
    let (exit, value) = refusal(&path);
    assert_ne!(value["code"], "io-error", "{value}");
    assert_ne!(value["stage"], "request", "{value}");
    assert_ne!(exit, 0, "{value}");
    assert!(!directory.path().join("spine.package.json").exists());
    let mut bare: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    bare["request"].as_object_mut().unwrap().remove("output");
    std::fs::write(&path, serde_json::to_vec(&bare).unwrap()).unwrap();
    let (bare_exit, bare_value) = refusal(&path);
    assert_eq!(
        (exit, &value["code"], &value["stage"], &value["message"]),
        (
            bare_exit,
            &bare_value["code"],
            &bare_value["stage"],
            &bare_value["message"]
        )
    );
}

/// FR-810-AC-3: `RequestOverrides::output` writes the same bytes as the
/// request member, and supplying both refuses and writes neither file.
#[test]
#[trace("TC-2297", "FR-810-AC-3")]
fn the_override_entry_writes_the_same_package_file() {
    let directory = tempfile::tempdir().unwrap();
    let member = request(
        directory.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    let package = fsl_command::command::compile(&member)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));

    let bare = tempfile::tempdir().unwrap();
    let path = request(
        bare.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "unused"}),
    );
    let mut document: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    document["request"]
        .as_object_mut()
        .unwrap()
        .remove("output");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    let cli = bare.path().join("cli.package.json");
    let overrides = fsl_command::command::RequestOverrides {
        output: Some(cli.as_path()),
        ..Default::default()
    };
    let outcome = fsl_command::command::compile_outcome_with_overrides(&path, &overrides)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    assert_eq!(std::fs::read(&cli).unwrap(), package);
    assert_eq!(outcome.package_bytes(), package);

    let twice = tempfile::tempdir().unwrap();
    let path = request(
        twice.path(),
        "tests/fixtures/spine-compile.native",
        json!({"package": "spine.package.json"}),
    );
    let cli = twice.path().join("cli.package.json");
    let overrides = fsl_command::command::RequestOverrides {
        output: Some(cli.as_path()),
        ..Default::default()
    };
    let error = fsl_command::command::compile_outcome_with_overrides(&path, &overrides)
        .err()
        .expect("both destinations refuse");
    let value = error.value().unwrap();
    assert_eq!(error.category().exit_code(), 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    let text = value.to_string();
    assert!(
        text.contains("output") && text.contains("--output"),
        "{value}"
    );
    assert!(!cli.exists() && !twice.path().join("spine.package.json").exists());
}

/// FR-810-AC-5: `output` and `handoff` are independent writes, and an
/// `output` path inside the handoff directory refuses naming both.
#[test]
#[trace("TC-2297", "FR-810-AC-5")]
#[ignore = "QSL-2309: needs the compile request's `handoff` member (QSL-2193, #2730), then the refusal of an output path inside the handoff directory"]
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

/// The `check_mode` values of every node of the package at `path`.
fn check_modes(path: &Path) -> Vec<String> {
    let package: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut modes = Vec::new();
    let mut stack = vec![&package];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(mode)) = map.get("check_mode") {
                    modes.push(mode.clone());
                }
                stack.extend(map.values());
            }
            Value::Array(items) => stack.extend(items.iter()),
            _ => {}
        }
    }
    modes.sort();
    modes
}

/// FR-952-AC-5 (TC-2975): a compile request's `check_mode` member selects
/// Kernel checking for the unit's function bodies; absent it is Linked, and
/// a value other than `"linked"` or `"kernel"` refuses at stage `request`.
#[test]
#[trace("TC-2975", "FR-952-AC-5")]
#[ignore = "QSL-2420: the compile request check_mode member not implemented yet"]
fn a_compile_request_selects_kernel_checking() {
    let unit = |with_r: bool| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure {{ x mod y }}\n{}",
            if with_r {
                "function r using v(x: Int[0, 9]): Integer pure { 10 mod x }\n"
            } else {
                ""
            }
        )
    };
    let compile = |with_r: bool, mode: Option<Value>| {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("program.native"), unit(with_r)).unwrap();
        let mut members = json!({
            "models": [],
            "program": {"source": {"file": "program.native", "authority": "agent-ix",
                "identity": "test:modes", "document": "Modes", "formal_revision": 1}},
            "output": {"package": "modes.package.json"}
        });
        if let Some(mode) = mode {
            members["check_mode"] = mode;
        }
        let path = directory.path().join("compile.json");
        let request = json!({"format": "native-compile/1", "request": members});
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let result = fsl_command::command::compile_outcome(&path)
            .map(|_| check_modes(&directory.path().join("modes.package.json")))
            .map_err(|error| (error.category().exit_code(), error.value().unwrap()));
        (directory, result)
    };
    let (_kept, kernel) = compile(true, Some(json!("kernel")));
    assert_eq!(kernel.expect("Kernel admits r"), ["kernel", "kernel"]);
    let (_kept, linked) = compile(false, None);
    assert_eq!(linked.expect("Linked by default"), ["linked"]);
    let (_kept, refused) = compile(true, Some(json!("linked")));
    assert!(refused.is_err(), "Linked refuses r's 10 mod x");
    for bad in [json!("Kernel"), json!(1), Value::Null] {
        let (_kept, refused) = compile(false, Some(bad.clone()));
        let (exit, value) = refused.expect_err("a malformed check_mode refuses");
        assert_eq!(exit, 20, "{bad}");
        assert!(value.to_string().contains("check_mode"), "{bad}: {value}");
    }
}
