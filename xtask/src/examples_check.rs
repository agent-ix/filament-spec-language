// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-765: `cargo xtask examples-check` runs the filament-driver examples
//! gate (driver FR-021) with `fsl` built from this language checkout.
//!
//! The sibling driver checkout (`FSL_DRIVER_DIR`) and filament-examples
//! checkout (`FSL_EXAMPLES_DIR`) are both required; a missing one is an
//! error, never a skip. The driver's `tests/examples_gate.rs` runs with
//! `FSL_LANGUAGE_DIR` naming this workspace, which makes it patch every
//! language package of its `fsl` build to this checkout's crates, in a copy of
//! the driver workspace, and print the line [`BUILT_FROM`] naming this
//! checkout. The command fails when that line is missing or names another
//! path: an older driver ignores `FSL_LANGUAGE_DIR` and builds `fsl` from git.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// The variable naming the sibling filament-driver checkout.
pub const DRIVER_VAR: &str = "FSL_DRIVER_DIR";
/// The variable naming the filament-examples checkout.
pub const EXAMPLES_VAR: &str = "FSL_EXAMPLES_DIR";
/// The variable the driver gate reads the language checkout from.
pub const LANGUAGE_VAR: &str = "FSL_LANGUAGE_DIR";

/// One checkout the gate needs and the files that identify it.
struct Required {
    var: &'static str,
    repository: &'static str,
    files: &'static [&'static str],
}

const DRIVER: Required = Required {
    var: DRIVER_VAR,
    repository: "filament-driver",
    files: &["Cargo.toml", "tests/examples_gate.rs"],
};

const EXAMPLES: Required = Required {
    var: EXAMPLES_VAR,
    repository: "filament-examples",
    files: &["examples", "tools/fsl-examples/Cargo.toml"],
};

/// The checkout `value` names for `required`, or why it is not one.
fn resolve(required: &Required, value: Option<OsString>) -> Result<PathBuf> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Err(Error::ExamplesCheckUnset {
            var: required.var,
            repository: required.repository,
        });
    };
    let dir = PathBuf::from(value);
    if let Some(missing) = required
        .files
        .iter()
        .copied()
        .find(|f| !dir.join(f).exists())
    {
        return Err(Error::ExamplesCheckMissing {
            var: required.var,
            repository: required.repository,
            path: dir,
            missing,
        });
    }
    dir.canonicalize().map_err(|source| Error::io(dir, source))
}

/// The driver and examples checkouts named by the environment.
pub fn checkouts() -> Result<(PathBuf, PathBuf)> {
    let driver = resolve(&DRIVER, std::env::var_os(DRIVER_VAR))?;
    let examples = resolve(&EXAMPLES, std::env::var_os(EXAMPLES_VAR))?;
    Ok((driver, examples))
}

/// The line prefix the driver gate prints once it has checked that its `fsl`
/// build took every language package from the named checkout (driver
/// FR-021-AC-7).
pub const BUILT_FROM: &str = "fsl built from language checkout ";

/// Checks the driver gate's output for the line naming `language` as the
/// checkout its `fsl` was built from. A driver that predates the
/// `FSL_LANGUAGE_DIR` build prints none and tests `fsl` from git; a line
/// naming another path tested another checkout.
pub fn check_built_from(stdout: &str, language: &Path) -> Result<()> {
    let reported: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.split_once(BUILT_FROM).map(|(_, path)| path.trim_end()))
        .collect();
    let expected = language.to_string_lossy();
    let other = reported.iter().find(|path| **path != expected);
    if reported.is_empty() || other.is_some() {
        return Err(Error::ExamplesCheckUnverified {
            language: language.to_path_buf(),
            reported: other.map(|path| (*path).to_owned()),
        });
    }
    Ok(())
}

/// The commit a checkout is at, for the gate to name: the `HEAD` sha, with
/// ` (dirty)` appended when tracked files differ from it, ` (dirty state unknown)`
/// when `git status` fails, or `unknown (not a git checkout)` when git gives no `HEAD`.
/// Untracked files do not count as dirty.
pub fn checkout_revision(dir: &Path) -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let Some(sha) = git(&["rev-parse", "HEAD"]).filter(|sha| !sha.is_empty()) else {
        return "unknown (not a git checkout)".to_owned();
    };
    match git(&["status", "--porcelain", "--untracked-files=no"]) {
        Some(status) if status.is_empty() => sha,
        Some(_) => format!("{sha} (dirty)"),
        None => format!("{sha} (dirty state unknown)"),
    }
}

/// The lines the gate prints first, naming the revision of each checkout it
/// uses, so a result says which driver and examples it ran against.
pub fn revision_lines(driver: &Path, examples: &Path, language: &Path) -> String {
    format!(
        "examples-check: driver {} at {}\nexamples-check: examples {} at {}\nexamples-check: language {} at {}\n",
        driver.display(),
        checkout_revision(driver),
        examples.display(),
        checkout_revision(examples),
        language.display(),
        checkout_revision(language),
    )
}

/// Runs the driver's examples gate over the named checkouts with `fsl` built
/// from `workspace`.
pub fn run(workspace: &Path) -> Result<String> {
    let (driver, examples) = checkouts()?;
    let language = workspace
        .canonicalize()
        .map_err(|source| Error::io(workspace, source))?;
    io::stdout()
        .lock()
        .write_all(revision_lines(&driver, &examples, &language).as_bytes())
        .map_err(|source| Error::io("<stdout>", source))?;
    // The driver's own toolchain file and target directory apply: the
    // gate is the driver's test, built in the driver workspace. Its stdout
    // is read for the built-from line, then passed on.
    let output = Command::new("cargo")
        .current_dir(&driver)
        .args([
            "test",
            "--locked",
            "--test",
            "examples_gate",
            "--",
            "--nocapture",
        ])
        .env(EXAMPLES_VAR, &examples)
        .env(LANGUAGE_VAR, &language)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO")
        .env_remove("CARGO_TARGET_DIR")
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|source| Error::ExamplesCheckSpawn { source })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    io::stdout()
        .lock()
        .write_all(stdout.as_bytes())
        .map_err(|source| Error::io("<stdout>", source))?;
    if !output.status.success() {
        return Err(Error::ExamplesCheckFailed {
            status: output.status,
        });
    }
    check_built_from(&stdout, &language)?;
    Ok(format!(
        "examples-check: {} passed with fsl built from {}\n",
        examples.display(),
        language.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}");
    }

    fn head(dir: &Path) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("run git");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn committed() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        git(dir.path(), &["init", "-q"]);
        std::fs::write(dir.path().join("f"), "a").expect("write");
        git(dir.path(), &["add", "f"]);
        git(dir.path(), &["commit", "-q", "-m", "m"]);
        dir
    }

    #[test]
    fn a_clean_checkout_is_named_by_its_head_commit() {
        let dir = committed();
        let revision = checkout_revision(dir.path());
        assert_eq!(revision, head(dir.path()));
        assert_eq!(revision.len(), 40);
    }

    #[test]
    fn a_modified_checkout_is_marked_dirty() {
        let dir = committed();
        std::fs::write(dir.path().join("f"), "b").expect("write");
        assert_eq!(
            checkout_revision(dir.path()),
            format!("{} (dirty)", head(dir.path()))
        );
    }

    #[test]
    fn untracked_files_do_not_make_a_checkout_dirty() {
        let dir = committed();
        std::fs::write(dir.path().join("untracked"), "x").expect("write");
        assert_eq!(checkout_revision(dir.path()), head(dir.path()));
    }

    #[test]
    fn a_checkout_whose_status_fails_is_marked_dirty_state_unknown() {
        let dir = committed();
        // A corrupt index makes `git status` fail while `rev-parse HEAD` works.
        std::fs::write(dir.path().join(".git/index"), "garbage").expect("corrupt index");
        assert_eq!(
            checkout_revision(dir.path()),
            format!("{} (dirty state unknown)", head(dir.path()))
        );
    }

    #[test]
    fn a_folder_that_is_not_a_checkout_is_named_unknown() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            checkout_revision(dir.path()),
            "unknown (not a git checkout)"
        );
    }

    #[test]
    fn the_revision_lines_name_the_driver_examples_and_language_checkouts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let shown = dir.path().display();
        let unknown = "unknown (not a git checkout)";
        assert_eq!(
            revision_lines(dir.path(), dir.path(), dir.path()),
            format!(
                "examples-check: driver {shown} at {unknown}\nexamples-check: examples {shown} at {unknown}\nexamples-check: language {shown} at {unknown}\n"
            )
        );
    }
}
