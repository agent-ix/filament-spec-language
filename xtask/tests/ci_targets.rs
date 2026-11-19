// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-765: `make ci` runs the examples gate locally, and no hosted workflow
//! runs on a push or a pull request.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf()
}

/// The names the Makefile's `CI_CHECKS :=` line lists.
fn ci_checks(makefile: &str) -> Vec<&str> {
    let mut lines = makefile
        .lines()
        .filter_map(|line| line.strip_prefix("CI_CHECKS :="));
    let line = lines.next().expect("the Makefile has a CI_CHECKS line");
    assert!(
        lines.next().is_none(),
        "the Makefile has two CI_CHECKS lines"
    );
    line.split_whitespace().collect()
}

#[trace("FR-765-AC-1", "TC-2190")]
#[test]
fn ci_includes_examples_check() {
    let makefile = fs::read_to_string(workspace().join("Makefile")).expect("read the Makefile");
    assert!(
        ci_checks(&makefile).contains(&"examples-check"),
        "CI_CHECKS does not name examples-check: {:?}",
        ci_checks(&makefile)
    );

    // The dry run of the aggregate graph reaches the target's recipe.
    let output = Command::new("make")
        .current_dir(workspace())
        .args(["--no-print-directory", "-n", "ci-internal"])
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .output()
        .expect("run make -n ci-internal");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "make -n ci-internal failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.lines().any(|line| line.ends_with(" examples-check")),
        "make -n ci-internal does not run `xtask examples-check`:\n{stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.ends_with("--package xtask --test examples_check")),
        "make -n ci-internal does not run the examples_check tests:\n{stdout}"
    );
}

/// The trigger names of one workflow's top-level `on:` key: a scalar
/// (`on: push`), a flow list (`on: [push, pull_request]`), or a block of
/// mapping keys or list items one indentation level below `on:`.
fn triggers(workflow: &str) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = workflow.lines().collect();
    let Some(at) = lines.iter().position(|l| {
        let key = l.split(':').next().unwrap_or_default();
        matches!(key, "on" | "\"on\"" | "'on'")
    }) else {
        return Err("no top-level `on:` key".to_owned());
    };
    let unquote = |s: &str| s.trim().trim_matches(['"', '\'']).to_owned();
    let inline = lines[at].split_once(':').map_or("", |(_, v)| v);
    let inline = inline.split('#').next().unwrap_or_default().trim();
    if !inline.is_empty() {
        // Fail closed on any inline form this reader does not split: a flow
        // mapping, or a flow list that continues onto later lines.
        if inline.starts_with('{') {
            return Err(format!("`on: {inline}` is a flow mapping; write a block"));
        }
        let names = match inline.strip_prefix('[') {
            Some(list) => list
                .strip_suffix(']')
                .ok_or_else(|| format!("`on: {inline}` has an unclosed `[`"))?,
            None => inline,
        };
        let names: Vec<String> = names.split(',').map(unquote).collect();
        if names.iter().any(String::is_empty) {
            return Err(format!("`on: {inline}` gives an empty trigger name"));
        }
        return Ok(names);
    }
    let block = lines[at + 1..]
        .iter()
        .take_while(|l| l.trim().is_empty() || l.starts_with([' ', '\t']) || l.starts_with('#'))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let indent = |l: &str| l.len() - l.trim_start().len();
    let rows: Vec<&&str> = block.collect();
    let Some(level) = rows.iter().map(|l| indent(l)).min() else {
        return Err("an empty `on:` block".to_owned());
    };
    Ok(rows
        .iter()
        .filter(|l| indent(l) == level)
        .map(|l| {
            let row = l.trim().trim_start_matches("- ");
            unquote(row.split(':').next().unwrap_or_default())
        })
        .collect())
}

#[trace("FR-765-AC-2", "TC-2191")]
#[test]
fn no_workflow_gains_a_push_or_pull_request_trigger() {
    let dir = workspace().join(".github/workflows");
    let mut workflows: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("list .github/workflows")
        .map(|entry| entry.expect("list .github/workflows").path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    workflows.sort();
    assert!(
        !workflows.is_empty(),
        "no workflow files under {}",
        dir.display()
    );
    for path in &workflows {
        let text = fs::read_to_string(path).expect("read a workflow");
        let names = triggers(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!names.is_empty(), "{}: no trigger read", path.display());
        let hosted: Vec<&String> = names
            .iter()
            .filter(|n| matches!(n.as_str(), "push" | "pull_request"))
            .collect();
        assert!(
            hosted.is_empty(),
            "{} declares {hosted:?}; hosted CI stays manual (FR-765)",
            path.display()
        );
    }
}

#[trace("FR-765-AC-2", "TC-2191")]
#[test]
fn the_trigger_reader_finds_every_spelling_of_push() {
    let cases = [
        ("on: push\n", vec!["push"]),
        ("on: [push, workflow_dispatch]\n", vec!["push", "workflow_dispatch"]),
        ("\"on\":\n  pull_request:\n    branches: [main]\n", vec!["pull_request"]),
        ("on:\n  - push\n  - pull_request_target\n", vec!["push", "pull_request_target"]),
        (
            "name: x\non:\n  # manual only\n  workflow_dispatch:\n  push:\n    branches: [main]\njobs:\n",
            vec!["workflow_dispatch", "push"],
        ),
    ];
    for (workflow, expected) in cases {
        assert_eq!(
            triggers(workflow),
            Ok(expected.iter().map(|s| (*s).to_owned()).collect()),
            "{workflow}"
        );
    }
    // Spellings the reader cannot split are refused, so the workflow scan
    // fails instead of reading a push trigger as some other name.
    let refused = [
        ("name: x\njobs:\n", "no top-level `on:` key"),
        ("on: {push: {branches: [main]}}\n", "flow mapping"),
        ("on: [\n  push,\n  workflow_dispatch]\n", "unclosed `[`"),
        ("on: [push,\n  pull_request]\n", "unclosed `[`"),
        ("on: [push, , workflow_dispatch]\n", "empty trigger name"),
    ];
    for (workflow, reason) in refused {
        let error = triggers(workflow).expect_err(workflow);
        assert!(error.contains(reason), "`{error}` does not say `{reason}`");
    }
}
