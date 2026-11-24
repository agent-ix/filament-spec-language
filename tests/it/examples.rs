// SPDX-License-Identifier: AGPL-3.0-or-later
//! Locate the authoritative sibling `filament-examples` checkout for tests.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The examples checkout selected by `FSL_EXAMPLES_DIR` or Makefile's sibling default.
pub(crate) fn examples_root() -> PathBuf {
    let configured = std::env::var_os("FSL_EXAMPLES_DIR").map(PathBuf::from);
    let sibling = sibling_examples_root();
    let root = match configured.as_ref() {
        Some(root) => root.clone(),
        None => match &sibling {
            Ok(root) => root.clone(),
            Err(reason) => panic!(
                "FSL_EXAMPLES_DIR is unset and the sibling filament-examples default could not be found: {reason}"
            ),
        },
    };

    if !root.join("examples").is_dir() {
        let configured = configured
            .as_ref()
            .map_or_else(|| "<unset>".to_owned(), |path| path.display().to_string());
        let sibling = match sibling {
            Ok(path) => path.display().to_string(),
            Err(reason) => reason,
        };
        panic!(
            "FSL_EXAMPLES_DIR={configured} is not an authoritative examples checkout; sibling default: {sibling}"
        );
    }

    root
}

fn sibling_examples_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|error| format!("could not run git rev-parse --git-common-dir: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --git-common-dir failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let common_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let Some(siblings) = common_dir.parent().and_then(Path::parent) else {
        return Err(format!(
            "git common directory {} has no sibling root",
            common_dir.display()
        ));
    };
    Ok(siblings.join("filament-examples"))
}
