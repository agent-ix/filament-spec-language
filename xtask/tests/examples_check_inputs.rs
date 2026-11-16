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

/// A cargo package standing in for a driver whose `examples_gate` test
/// passes and prints `line` (formatted with the `FSL_LANGUAGE_DIR` value as
/// `{dir}`) when that variable is set; an empty `line` prints nothing, as a
/// driver that predates the `FSL_LANGUAGE_DIR` build does.
fn stand_in_gate(root: &Path, line: &str) -> PathBuf {
    let driver = root.join("filament-driver");
    fs::create_dir_all(driver.join("tests")).expect("create the stand-in driver");
    fs::create_dir_all(driver.join("src")).expect("create src/");
    fs::write(
        driver.join("Cargo.toml"),
        "[package]\nname = \"stand-in-driver\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [workspace]\n\n[[test]]\nname = \"examples_gate\"\npath = \"tests/examples_gate.rs\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(driver.join("src/lib.rs"), "//! Stand-in driver.\n").expect("write lib.rs");
    let print = if line.is_empty() {
        String::new()
    } else {
        format!("println!(\"{{}}\", {line:?}.replace(\"{{dir}}\", &dir));")
    };
    fs::write(
        driver.join("tests/examples_gate.rs"),
        format!(
            "#[test]\nfn gate() {{\n    if let Ok(dir) = std::env::var(\"FSL_LANGUAGE_DIR\") {{\n        \
             {print}\n    }}\n}}\n"
        ),
    )
    .expect("write the gate");
    let status = Command::new("cargo")
        .current_dir(&driver)
        .arg("generate-lockfile")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO")
        .status()
        .expect("run cargo generate-lockfile");
    assert!(status.success(), "cargo generate-lockfile failed");
    driver
}

/// A folder holding the files that identify a filament-examples checkout.
fn stand_in_examples(root: &Path) -> PathBuf {
    let examples = root.join("filament-examples");
    fs::create_dir_all(examples.join("examples")).expect("create examples/");
    fs::create_dir_all(examples.join("tools/fsl-examples")).expect("create the runner folder");
    fs::write(examples.join("tools/fsl-examples/Cargo.toml"), "").expect("write Cargo.toml");
    examples
}

/// Runs `make examples-check` over a stand-in gate printing `line`.
fn run_stand_in_gate(line: &str) -> Output {
    let root = scratch();
    let driver = stand_in_gate(root.path(), line);
    let examples = stand_in_examples(root.path());
    make_examples_check(&[
        format!("FSL_DRIVER_DIR={}", driver.display()),
        format!("FSL_EXAMPLES_DIR={}", examples.display()),
        "EXAMPLES_CHECK_TESTS=".to_owned(),
    ])
}

fn this_checkout() -> String {
    workspace()
        .canonicalize()
        .expect("canonical workspace")
        .display()
        .to_string()
}

#[trace("FR-765-AC-5", "TC-2194")]
#[test]
fn a_driver_gate_that_does_not_report_this_checkout_fails() {
    let output = run_stand_in_gate("");
    assert_refused(
        &output,
        &[&format!(
            "did not report building fsl from {}; the sibling driver may predate",
            this_checkout()
        )],
    );
}

#[trace("FR-765-AC-5", "TC-2194")]
#[test]
fn a_driver_gate_that_reports_another_checkout_fails() {
    let output = run_stand_in_gate("fsl built from language checkout /elsewhere");
    assert_refused(&output, &["(it reported /elsewhere)"]);
}

#[trace("FR-765-AC-5", "TC-2194")]
#[test]
fn a_driver_gate_that_reports_this_checkout_passes() {
    let output = run_stand_in_gate("fsl built from language checkout {dir}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "make examples-check failed:\n{}\n{stdout}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(&format!("passed with fsl built from {}", this_checkout())),
        "{stdout}"
    );
}
