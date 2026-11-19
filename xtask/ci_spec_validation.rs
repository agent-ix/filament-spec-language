// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::env;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const VALIDATOR_PACKAGE: &str = "@agent-ix/quire-cli";
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
    let node = find_executable("node")
        .ok_or_else(|| "node is required to run the @agent-ix/quire-cli Node launcher".to_owned())?;
    let launcher = find_npm_launcher().ok_or_else(|| {
        format!(
            "npm {VALIDATOR_PACKAGE} Node launcher is missing from PATH; expose its quire bin"
        )
    })?;
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

fn find_npm_launcher() -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let launchers = env::split_paths(&path)
        .map(|directory| directory.join("quire"))
        .filter(|candidate| is_executable(candidate))
        .filter_map(|candidate| candidate.canonicalize().ok())
        .filter(|candidate| {
            candidate.is_file()
                && candidate.extension().is_some_and(|extension| extension == "js")
                && candidate.ends_with(Path::new("node_modules/@agent-ix/quire-cli/bin/launcher.js"))
        })
        .collect::<BTreeSet<_>>();

    // BTreeSet ordering makes the chosen installation independent of PATH order.
    launchers.into_iter().next()
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
