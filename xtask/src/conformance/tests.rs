// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-boundary checks with synthetic Cargo output, not vector evidence.

use super::{run_with, CHECKS};
use crate::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_double() -> PathBuf {
    static EXECUTABLE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    EXECUTABLE
        .get_or_init(|| {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.join("target"));
            let target = if target.is_absolute() {
                target
            } else {
                workspace.join(target)
            };
            let directory = target.join("ci-conformance-process-double");
            fs::create_dir_all(&directory).unwrap();
            let executable = directory.join("cargo");
            let output = Command::new("rustc")
                .args([
                    "--edition=2021",
                    "--crate-name",
                    "conformance_cargo_double",
                    "-D",
                    "warnings",
                ])
                .arg(workspace.join("xtask/tests/fixtures/ci_conformance_cargo_double.rs"))
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            executable
        })
        .clone()
}

fn run_case(workspace: &Path, mode: &str, fail_at: usize) -> Result<String, Error> {
    let log = workspace.join("calls");
    let index = fail_at.to_string();
    run_with(
        &cargo_double(),
        workspace,
        &[
            ("CONFORMANCE_DOUBLE_LOG", log.as_os_str()),
            ("CONFORMANCE_DOUBLE_MODE", OsStr::new(mode)),
            ("CONFORMANCE_DOUBLE_FAIL_AT", OsStr::new(&index)),
        ],
    )
}

#[cfg(unix)]
#[ix_trace_rs::trace("FR-350-AC-21")]
#[test]
fn both_source_fixture_runners_are_required_conformance_selections() {
    for (selection, summary) in [
        (
            "fspec_finite_window::conformance_fspec_finite_window_source_windows",
            "conformance: 77 finite-window source windows matched",
        ),
        (
            "fspec_state_core::conformance_fspec_state_core_source_cases",
            "conformance: 94 state-core source cases matched",
        ),
    ] {
        let check = CHECKS
            .iter()
            .find(|check| check.selection == selection)
            .unwrap_or_else(|| panic!("{selection} is required"));
        assert_eq!(check.package, "fsl-conformance");
        assert!(matches!(check.target, super::Target::Test));
        let output = std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(0),
            stdout: format!("test result: ok. 1 passed; 0 failed; 0 ignored;\n{summary}\n")
                .into_bytes(),
            stderr: Vec::new(),
        };
        super::check_output(check, &output).expect("the summary line is accepted");
        let missing = std::process::Output {
            stdout: b"test result: ok. 1 passed; 0 failed; 0 ignored;\n".to_vec(),
            ..output
        };
        assert!(matches!(
            super::check_output(check, &missing),
            Err(Error::ConformanceCheck { .. })
        ));
    }
}

#[cfg(unix)]
#[ix_trace_rs::trace("FR-350-AC-21")]
#[test]
fn the_finite_window_source_runner_is_a_required_conformance_selection() {
    let check = CHECKS
        .iter()
        .find(|check| {
            check.selection == "fspec_finite_window::conformance_fspec_finite_window_source_windows"
        })
        .expect("the source runner is required");
    assert_eq!(check.package, "fsl-conformance");
    assert!(matches!(check.target, super::Target::Test));
    let output = std::process::Output {
        status: std::os::unix::process::ExitStatusExt::from_raw(0),
        stdout: b"test result: ok. 1 passed; 0 failed; 0 ignored;\nconformance: 46 finite-window source windows matched\n".to_vec(),
        stderr: Vec::new(),
    };
    super::check_output(check, &output).expect("source window evidence is required");
    let missing = std::process::Output {
        stdout: b"test result: ok. 1 passed; 0 failed; 0 ignored;\n".to_vec(),
        ..output
    };
    assert!(matches!(
        super::check_output(check, &missing),
        Err(Error::ConformanceCheck { .. })
    ));
}

#[test]
fn each_pinned_vector_selection_requires_success_and_its_summary() {
    let root = crate::test_support::tempdir();
    let log = root.path().join("calls");
    run_with(
        &cargo_double(),
        root.path(),
        &[("CONFORMANCE_DOUBLE_LOG", log.as_os_str())],
    )
    .unwrap();
    let calls = fs::read_to_string(log).unwrap();
    assert_eq!(calls.lines().count(), CHECKS.len());
    for (line, check) in calls.lines().zip(CHECKS) {
        assert_eq!(line, check.selection);
    }
}

#[test]
fn a_failed_cargo_selection_stops_the_gate() {
    let root = crate::test_support::tempdir();
    assert!(matches!(
        run_case(root.path(), "nonzero", 2),
        Err(Error::ConformanceCheck { .. })
    ));
    assert_eq!(
        fs::read_to_string(root.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        3
    );
}

#[test]
fn each_selection_refuses_missing_evidence_or_failure() {
    for (index, _) in CHECKS.iter().enumerate() {
        for mode in ["zero-tests", "ignored", "skip", "no-summary", "nonzero"] {
            let root = crate::test_support::tempdir();
            assert!(matches!(
                run_case(root.path(), mode, index),
                Err(Error::ConformanceCheck { .. })
            ));
            assert_eq!(
                fs::read_to_string(root.path().join("calls"))
                    .unwrap()
                    .lines()
                    .count(),
                index + 1,
                "{mode} at {}",
                CHECKS[index].selection
            );
        }
    }
}

/// QSL-2270: what a selection's Cargo child writes reaches the gate's own
/// output as it is written and is still captured whole for the checks.
#[test]
fn child_output_is_echoed_and_captured() {
    let child =
        b"   Compiling fsl-package v0.3.0\nBlocking waiting for file lock on build directory\n";
    let mut echoed = Vec::new();
    let captured = super::echo_and_capture(&child[..], &mut echoed).unwrap();
    assert_eq!(echoed, child);
    assert_eq!(captured, child);
}

/// A sink that refuses writes, standing in for a closed gate stdout.
struct ClosedSink;

impl std::io::Write for ClosedSink {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A gate output that cannot be written stops the echo, never the capture
/// the checks judge.
#[test]
fn a_closed_echo_sink_still_captures_child_output() {
    let child = b"test result: ok. 1 passed; 0 failed; 0 ignored;\n";
    let captured = super::echo_and_capture(&child[..], ClosedSink).unwrap();
    assert_eq!(captured, child);
}

#[test]
fn zero_source_map_entries_refuse() {
    let root = crate::test_support::tempdir();
    assert!(matches!(
        run_case(root.path(), "zero-vectors", 0),
        Err(Error::ConformanceCheck { .. })
    ));
}
