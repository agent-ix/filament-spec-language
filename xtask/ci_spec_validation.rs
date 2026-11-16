// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const VALIDATOR_PACKAGE: &str = "@agent-ix/quire-cli";
const VALIDATOR_LAUNCHER: &str = "bin/launcher.js";
const DOCUMENTS: &str = "spec/**/*.md";

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("check-spec-validation: {message}");
            ExitCode::from(127)
        }
    }
}

fn run() -> Result<u8, String> {
    let npm = find_executable("npm")
        .ok_or_else(|| "npm is required to locate the @agent-ix/quire-cli Node launcher".to_owned())?;
    let node = find_executable("node")
        .ok_or_else(|| "node is required to run the @agent-ix/quire-cli Node launcher".to_owned())?;
    let launcher = npm_launcher(&npm)?;
    let documents = env::var_os("SPEC_VALIDATION_DOCUMENTS")
        .unwrap_or_else(|| DOCUMENTS.into());

    let status = Command::new(node)
        .arg(launcher)
        .args(["validate", "--scope", "."])
        .arg(documents)
        .env("PATH", "")
        .status()
        .map_err(|error| format!("could not run the @agent-ix/quire-cli Node launcher: {error}"))?;

    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}

fn npm_launcher(npm: &Path) -> Result<PathBuf, String> {
    let output = Command::new(npm)
        .args(["root", "-g"])
        .output()
        .map_err(|error| format!("could not ask npm for its global package root: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "could not locate the npm {VALIDATOR_PACKAGE} Node launcher (npm root -g failed): {}",
            detail.trim()
        ));
    }

    let root = String::from_utf8_lossy(&output.stdout);
    let root = root.trim();
    if root.is_empty() {
        return Err(format!(
            "could not locate the npm {VALIDATOR_PACKAGE} Node launcher (npm root -g returned an empty path)"
        ));
    }

    let launcher = Path::new(root)
        .join(VALIDATOR_PACKAGE)
        .join(VALIDATOR_LAUNCHER);
    let launcher = launcher.canonicalize().map_err(|_| {
        format!(
            "npm {VALIDATOR_PACKAGE} Node launcher is missing: {}",
            launcher.display()
        )
    })?;
    if !launcher.is_file() || launcher.extension() != Some(OsStr::new("js")) {
        return Err(format!(
            "npm {VALIDATOR_PACKAGE} Node launcher is missing or is not a JavaScript file: {}",
            launcher.display()
        ));
    }

    Ok(launcher)
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
