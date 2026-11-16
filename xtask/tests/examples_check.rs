// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-765-AC-3: a verdict this checkout's `fsl` no longer gives fails
//! `make examples-check`.
//!
//! Runs only from `make examples-check` (`test = false` in Cargo.toml), after
//! the gate itself passed over the real suite: it needs the sibling driver and
//! examples checkouts named by `FSL_DRIVER_DIR` and `FSL_EXAMPLES_DIR`. The
//! stand-in suite is a copy of the examples checkout with one running case's
//! recorded outcome changed, so the recorded verdict differs from the one
//! this checkout's `fsl` gives: the same disagreement a language change that
//! alters a verdict produces.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf()
}

fn examples() -> PathBuf {
    let dir = std::env::var_os("FSL_EXAMPLES_DIR")
        .filter(|v| !v.is_empty())
        .expect("FSL_EXAMPLES_DIR names the examples checkout; run `make examples-check`");
    PathBuf::from(dir)
}

/// Copies `from` into `to`, leaving out version control and build output.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|e| panic!("create {}: {e}", to.display()));
    for entry in fs::read_dir(from).unwrap_or_else(|e| panic!("list {}: {e}", from.display())) {
        let entry = entry.unwrap_or_else(|e| panic!("list {}: {e}", from.display()));
        let name = entry.file_name();
        if name == ".git" || name == "target" {
            continue;
        }
        let target = to.join(&name);
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target)
                .unwrap_or_else(|e| panic!("copy {}: {e}", entry.path().display()));
        }
    }
}

/// The id of the first `[[case]]` of `manifest` whose status is `running`.
fn first_running_case(manifest: &str) -> Option<String> {
    manifest.split("[[case]]").skip(1).find_map(|case| {
        let value = |key: &str| {
            case.lines()
                .filter_map(|l| l.trim().strip_prefix(key)?.trim().strip_prefix('='))
                .map(|v| v.trim().trim_matches('"').to_owned())
                .next()
        };
        (value("status")? == "running").then(|| value("id"))?
    })
}

/// The first system, by folder name, with a running case, and that case.
fn first_running_system(suite: &Path) -> (String, String) {
    let mut systems: Vec<PathBuf> = fs::read_dir(suite.join("examples"))
        .expect("list examples/")
        .map(|entry| entry.expect("list examples/").path())
        .collect();
    systems.sort();
    systems
        .iter()
        .find_map(|dir| {
            let manifest = fs::read_to_string(dir.join("example.toml")).ok()?;
            let id = first_running_case(&manifest)?;
            Some((dir.file_name()?.to_string_lossy().into_owned(), id))
        })
        .expect("an example system has a running case")
}

#[trace("FR-765-AC-3", "TC-2192")]
#[test]
fn a_verdict_this_checkout_does_not_give_fails_make_examples_check() {
    // Outside this workspace: under it, cargo would take the copied runner
    // crate for a member of this workspace.
    let temp = std::env::temp_dir()
        .canonicalize()
        .expect("canonical temp dir");
    let base = tempfile::tempdir_in(temp).expect("tempdir");
    let suite = base.path().join("filament-examples");
    copy_tree(&examples(), &suite);
    let (system, id) = first_running_system(&suite);
    let path = suite
        .join("examples")
        .join(&system)
        .join("expected")
        .join(format!("{id}.json"));
    let recorded = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut outcome: Value =
        serde_json::from_str(&recorded).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let category = outcome["category"]
        .as_str()
        .unwrap_or_else(|| panic!("{} has no category", path.display()))
        .to_owned();
    outcome["category"] = Value::String(format!("{category}-edited"));
    fs::write(&path, outcome.to_string()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

    // EXAMPLES_CHECK_TESTS is emptied so a passing gate could not run this
    // test again; the built xtask binary avoids a rebuild.
    let output = Command::new("make")
        .current_dir(workspace())
        .args(["--no-print-directory", "examples-check"])
        .arg(format!("EXAMPLES_XTASK={}", env!("CARGO_BIN_EXE_xtask")))
        .arg(format!("FSL_EXAMPLES_DIR={}", suite.display()))
        .arg("EXAMPLES_CHECK_TESTS=")
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .output()
        .expect("run make examples-check");
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !output.status.success(),
        "make examples-check passed:\n{all}"
    );
    assert!(
        all.contains(&format!(
            "{system}/{id}: outcome differs from expected/{id}.json in category"
        )),
        "the failure does not name {system}/{id}:\n{all}"
    );
    assert!(
        all.contains("examples-check: the driver examples gate failed"),
        "the target did not report the failed gate:\n{all}"
    );
}
