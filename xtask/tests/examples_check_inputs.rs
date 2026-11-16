// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-765-AC-4: `make examples-check` refuses a missing driver or examples
//! checkout with a message naming it, and exits nonzero instead of skipping.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf()
}

/// Runs `make examples-check <assignments>` with the built xtask binary, so
/// the target's own recipe runs without rebuilding xtask.
fn make_examples_check(assignments: &[String]) -> Output {
    Command::new("make")
        .current_dir(workspace())
        .args(["--no-print-directory", "examples-check"])
        .arg(format!("EXAMPLES_XTASK={}", env!("CARGO_BIN_EXE_xtask")))
        .args(assignments)
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .output()
        .expect("run make examples-check")
}

fn scratch() -> tempfile::TempDir {
    let base = std::env::temp_dir()
        .canonicalize()
        .expect("canonical temp dir");
    tempfile::tempdir_in(base).expect("tempdir")
}

/// A folder holding the files that identify a filament-driver checkout.
fn stand_in_driver(root: &Path) -> PathBuf {
    let driver = root.join("filament-driver");
    fs::create_dir_all(driver.join("tests")).expect("create the stand-in driver");
    fs::write(driver.join("Cargo.toml"), "").expect("write Cargo.toml");
    fs::write(driver.join("tests/examples_gate.rs"), "").expect("write the gate");
    driver
}

fn assert_refused(output: &Output, needles: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "make examples-check passed:\n{stderr}"
    );
    for needle in needles {
        assert!(stderr.contains(needle), "`{needle}` is not in:\n{stderr}");
    }
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("--test examples_check"),
        "the target went on past the refusal"
    );
}

#[trace("FR-765-AC-4", "TC-2193")]
#[test]
fn a_missing_driver_checkout_is_an_error() {
    let root = scratch();
    let absent = root.path().join("filament-driver");
    let output = make_examples_check(&[format!("FSL_DRIVER_DIR={}", absent.display())]);
    assert_refused(
        &output,
        &[
            &format!("no filament-driver checkout at {}", absent.display()),
            "FSL_DRIVER_DIR",
            "Cargo.toml is missing",
        ],
    );
}

#[trace("FR-765-AC-4", "TC-2193")]
#[test]
fn a_missing_examples_checkout_is_an_error() {
    let root = scratch();
    let driver = stand_in_driver(root.path());
    let absent = root.path().join("filament-examples");
    let output = make_examples_check(&[
        format!("FSL_DRIVER_DIR={}", driver.display()),
        format!("FSL_EXAMPLES_DIR={}", absent.display()),
    ]);
    assert_refused(
        &output,
        &[
            &format!("no filament-examples checkout at {}", absent.display()),
            "FSL_EXAMPLES_DIR",
            "examples is missing",
        ],
    );
}

#[trace("FR-765-AC-4", "TC-2193")]
#[test]
fn an_examples_folder_without_the_runner_is_an_error() {
    let root = scratch();
    let driver = stand_in_driver(root.path());
    let examples = root.path().join("filament-examples");
    fs::create_dir_all(examples.join("examples")).expect("create examples/");
    let output = make_examples_check(&[
        format!("FSL_DRIVER_DIR={}", driver.display()),
        format!("FSL_EXAMPLES_DIR={}", examples.display()),
    ]);
    assert_refused(&output, &["tools/fsl-examples/Cargo.toml is missing"]);
}

#[trace("FR-765-AC-4", "TC-2193")]
#[test]
fn an_empty_driver_variable_is_an_error() {
    let output = make_examples_check(&["FSL_DRIVER_DIR=".to_owned()]);
    assert_refused(&output, &["FSL_DRIVER_DIR is not set", "filament-driver"]);
}
