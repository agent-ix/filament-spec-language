// SPDX-License-Identifier: AGPL-3.0-or-later
//! The local FSpec vector gate. Path resolution and child environments live here,
//! so Make never interprets checkout path bytes as source.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Instant;

use ix_trace_rs::string_edge;

use crate::error::{Error, Result};

struct Check {
    package: &'static str,
    target: Target,
    selection: &'static str,
    summaries: &'static [&'static [&'static str]],
    summary_mode: SummaryMode,
}

#[derive(Clone, Copy)]
enum SummaryMode {
    Exact,
    Prefix,
}

#[derive(Clone, Copy)]
enum Target {
    Lib,
    Test,
}

#[derive(Clone, Copy)]
enum OutputMarker {
    SuccessfulTest,
    Skipped,
    Other,
}

#[string_edge]
fn output_marker(line: &str) -> OutputMarker {
    if line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;") {
        OutputMarker::SuccessfulTest
    } else if line.starts_with("skipped:") {
        OutputMarker::Skipped
    } else {
        OutputMarker::Other
    }
}

impl Target {
    fn argument(self) -> &'static str {
        match self {
            Self::Lib => "--lib",
            Self::Test => "--test",
        }
    }
}

const CHECKS: &[Check] = &[
    Check { package: "fsl-conformance", target: Target::Test, selection: "fspec_finite_window::conformance_fspec_finite_window_source_windows", summaries: &[&["conformance: ", " finite-window source windows matched"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-conformance", target: Target::Test, selection: "fspec_state_core::conformance_fspec_state_core_source_cases", summaries: &[&["conformance: ", " state-core source cases matched"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_c14_source_map_lookup_over_fspec_positive_fixtures", summaries: &[&["conformance: ", " source-map entries over ", " positive fixtures"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_i2_read_over_fspec_checked_package_v2_fixtures", summaries: &[&["conformance: ", " positive fixtures through FSL's full I2 read"], &["conformance: ", " adverse mutations refused"]], summary_mode: SummaryMode::Prefix },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_fr340_frame_mutations_match_fspec_vectors", summaries: &[&["conformance: ", " frame-body mutation vectors matched"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_node_index_vectors_refuse_malformed_wire", summaries: &[&["conformance: ", " node-index vector outcomes matched"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_dependency_selection_vectors", summaries: &[&["conformance: ", " dependency-selection entry mutations and ", " order vectors"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "emit::tests::golden::conformance_emitted_application_nodes_match_fspec_positive_fixtures", summaries: &[&["conformance: ", " emitted application nodes match FSpec's positive fixtures"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-semantics", target: Target::Test, selection: "complete_value_lock::conformance_admit_selection_matches_fspec_vectors", summaries: &[&["conformance: ", " accepted and ", " refused selection vectors"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-semantics", target: Target::Test, selection: "variant_type::conformance_clock_tuple_union_intake_vectors_are_admitted", summaries: &[&["conformance: ", " Clock tuple/union intake vectors admitted"]], summary_mode: SummaryMode::Exact },
    Check { package: "fsl-package", target: Target::Lib, selection: "checked_v2::tests::protocol_corpus::conformance_protocol_vector_positive_packages_are_admitted", summaries: &[&["conformance: ", " protocol vector positive packages admitted"]], summary_mode: SummaryMode::Exact },
];

fn matches_summary(line: &str, pieces: &[&str], mode: SummaryMode) -> bool {
    let Some(mut remainder) = line.strip_prefix(pieces[0]) else {
        return false;
    };
    if pieces.len() == 1 {
        return true;
    }
    for (index, piece) in pieces.iter().enumerate().skip(1) {
        let digits = remainder.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || remainder.starts_with('0') {
            return false;
        }
        remainder = &remainder[digits..];
        let Some(rest) = remainder.strip_prefix(piece) else {
            return false;
        };
        remainder = rest;
        if index == pieces.len() - 1 && !piece.ends_with('(') && matches!(mode, SummaryMode::Exact)
        {
            return remainder.is_empty();
        }
    }
    true
}

/// Checks one selection's finished output. The output was already echoed
/// while the child ran (see [`run_echoed`]), so this only judges it.
fn check_output(check: &Check, output: &std::process::Output) -> Result<()> {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: format!("cargo exited with {}", output.status),
        });
    }
    if !combined
        .lines()
        .any(|line| matches!(output_marker(line), OutputMarker::SuccessfulTest))
    {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required test did not execute successfully".to_owned(),
        });
    }
    if combined
        .lines()
        .any(|line| matches!(output_marker(line), OutputMarker::Skipped))
    {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required vector check skipped".to_owned(),
        });
    }
    if !check.summaries.iter().all(|pattern| {
        combined
            .lines()
            .any(|line| matches_summary(line, pattern, check.summary_mode))
    }) {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required vector summary did not appear".to_owned(),
        });
    }
    Ok(())
}

/// Copies every chunk `source` yields to `sink` as it arrives and returns
/// all of it. A failed write to `sink` stops the echo, not the capture.
fn echo_and_capture(mut source: impl Read, mut sink: impl Write) -> io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut echo = true;
    loop {
        let read = match source.read(&mut buffer) {
            Ok(0) => return Ok(captured),
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let chunk = &buffer[..read];
        if echo {
            echo = sink.write_all(chunk).and_then(|()| sink.flush()).is_ok();
        }
        captured.extend_from_slice(chunk);
    }
}

/// Runs `command` with its stdout and stderr echoed live and captured.
///
/// Cargo writes `Compiling …` and `Blocking waiting for file lock …` to
/// stderr while it builds the test binary. Capturing with
/// [`Command::output`] alone held those lines back until the child exited, so
/// a long compile or a lock wait read as a hung test (QSL-2270).
fn run_echoed(command: &mut Command) -> io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other("child pipes were not created"));
    };
    let stderr = std::thread::spawn(move || echo_and_capture(stderr, io::stderr()));
    let stdout = echo_and_capture(stdout, io::stdout());
    let stderr = stderr
        .join()
        .unwrap_or_else(|_| Err(io::Error::other("stderr echo thread panicked")));
    let status = child.wait()?;
    Ok(Output {
        status,
        stdout: stdout?,
        stderr: stderr?,
    })
}

fn run_with(cargo: &Path, workspace: &Path, child_env: &[(&str, &OsStr)]) -> Result<String> {
    let total = CHECKS.len();
    for (position, check) in CHECKS.iter().enumerate() {
        let ordinal = position + 1;
        eprintln!(
            "conformance: [{ordinal}/{total}] {} {}: {} (cargo builds the test binary first; its progress follows)",
            check.package,
            check.target.argument(),
            check.selection,
        );
        let started = Instant::now();
        let mut command = Command::new(cargo);
        command.current_dir(workspace).args([
            "test",
            "--locked",
            "-p",
            check.package,
            check.target.argument(),
        ]);
        if matches!(check.target, Target::Test) {
            command.arg("it");
        }
        command
            .args([
                "--",
                "--exact",
                check.selection,
                "--format",
                "terse",
                "--nocapture",
            ])
            .envs(child_env.iter().copied());
        let output = run_echoed(&mut command).map_err(|source| Error::ConformanceSpawn {
            selection: check.selection,
            source,
        })?;
        eprintln!(
            "conformance: [{ordinal}/{total}] {} finished in {:.1}s (build and run)",
            check.selection,
            started.elapsed().as_secs_f64(),
        );
        check_output(check, &output)?;
    }
    Ok(String::new())
}

/// Run every required exact FSpec vector selection against the pinned package.
pub fn run(workspace: &Path) -> Result<String> {
    run_with(Path::new("cargo"), workspace, &[])
}

fn run_ci_with(
    make: &Path,
    workspace: &Path,
    args: &[&str],
    child_env: &[(&str, &OsStr)],
) -> Result<String> {
    let status = Command::new(make)
        .current_dir(workspace)
        .arg("--no-print-directory")
        .arg("ci-internal")
        .args(args)
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .envs(child_env.iter().copied())
        .status()
        .map_err(|source| Error::ConformanceCiSpawn { source })?;
    if !status.success() {
        return Err(Error::ConformanceCiFailed { status });
    }
    Ok(String::new())
}

/// Launch the local CI graph; each vector reader uses the pinned package API.
pub fn run_ci(workspace: &Path) -> Result<String> {
    run_ci_with(Path::new("make"), workspace, &[], &[])
}

#[cfg(test)]
mod tests;
