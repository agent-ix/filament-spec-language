// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-765: `cargo xtask examples-check` runs the filament-driver examples
//! gate (driver FR-021) with `fsl` built from this language checkout.
//!
//! The sibling driver checkout (`FSL_DRIVER_DIR`) and filament-examples
//! checkout (`FSL_EXAMPLES_DIR`) are both required; a missing one is an
//! error, never a skip. The driver's `tests/examples_gate.rs` runs with
//! `FSL_LANGUAGE_DIR` naming this workspace, which makes it patch every
//! language package of its `fsl` build to this checkout's crates and leave the
//! driver's `Cargo.lock` as it was.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Runs the driver's examples gate over the named checkouts with `fsl` built
/// from `workspace`.
pub fn run(workspace: &Path) -> Result<String> {
    let (driver, examples) = checkouts()?;
    let language = workspace
        .canonicalize()
        .map_err(|source| Error::io(workspace, source))?;
    // The driver's own toolchain file and target directory apply: the
    // gate is the driver's test, built in the driver workspace.
    let status = Command::new("cargo")
        .current_dir(&driver)
        .args(["test", "--locked", "--test", "examples_gate"])
        .env(EXAMPLES_VAR, &examples)
        .env(LANGUAGE_VAR, &language)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("CARGO")
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .map_err(|source| Error::ExamplesCheckSpawn { source })?;
    if !status.success() {
        return Err(Error::ExamplesCheckFailed { status });
    }
    Ok(format!(
        "examples-check: {} passed with fsl built from {}\n",
        examples.display(),
        language.display()
    ))
}
