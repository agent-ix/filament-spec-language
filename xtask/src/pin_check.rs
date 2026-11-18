// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-760-AC-3: every Agent-IX git dependency is pinned to a release tag in
//! the Cargo manifest that declares its source.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Outcome;
use crate::error::{Error, Result};

const USAGE: &str = "usage: cargo xtask pin-check [--manifest <Cargo.toml-or-directory>]";

/// Inspect all workspace Cargo manifests, or one manifest supplied by a test.
pub fn run(workspace_root: &Path, arguments: &[OsString]) -> Result<Outcome> {
    let manifests = match arguments {
        [] => manifests_under(workspace_root)?,
        [flag, path] if flag == "--manifest" => {
            let path = PathBuf::from(path);
            vec![if path.is_dir() {
                path.join("Cargo.toml")
            } else {
                path
            }]
        }
        _ => return Err(Error::Usage(USAGE)),
    };

    let mut findings = Vec::new();
    let mut manifest_count = 0usize;
    let mut dependency_count = 0usize;
    for path in manifests {
        let source = fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?;
        let manifest: toml::Value = source.parse().map_err(|error: toml::de::Error| {
            Error::io(
                &path,
                std::io::Error::new(std::io::ErrorKind::InvalidData, error),
            )
        })?;
        manifest_count += 1;
        inspect_manifest(&manifest, &path, &mut dependency_count, &mut findings);
    }

    if findings.is_empty() {
        Ok(Outcome::success(format!(
            "pin-check ok: {manifest_count} Cargo.toml manifests; {dependency_count} Agent-IX git dependencies are tag-pinned\n"
        )))
    } else {
        let mut summary = format!(
            "pin-check found {} unpinned Agent-IX git dependency(ies):\n",
            findings.len()
        );
        for finding in findings {
            summary.push_str("  ");
            summary.push_str(&finding);
            summary.push('\n');
        }
        Ok(Outcome {
            stdout: summary,
            code: 1,
        })
    }
}

fn manifests_under(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut manifests = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| Error::io(&directory, error))? {
            let entry = entry.map_err(|error| Error::io(&directory, error))?;
            let path = entry.path();
            if path.is_dir() {
                if matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some(".git" | "target")
                ) {
                    continue;
                }
                pending.push(path);
            } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
                manifests.push(path);
            }
        }
    }
    manifests.sort();
    Ok(manifests)
}

fn inspect_manifest(
    manifest: &toml::Value,
    path: &Path,
    dependency_count: &mut usize,
    findings: &mut Vec<String>,
) {
    let Some(root) = manifest.as_table() else {
        return;
    };
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = root.get(section).and_then(toml::Value::as_table) {
            inspect_dependencies(table, section, path, dependency_count, findings);
        }
    }
    if let Some(sources) = root.get("patch").and_then(toml::Value::as_table) {
        for (source, dependencies) in sources {
            let Some(dependencies) = dependencies.as_table() else {
                continue;
            };
            let name = format!("patch.{source}");
            inspect_dependencies(dependencies, &name, path, dependency_count, findings);
        }
    }
    if let Some(replacements) = root.get("replace").and_then(toml::Value::as_table) {
        inspect_dependencies(
            replacements,
            "replace",
            path,
            dependency_count,
            findings,
        );
    }
    if let Some(workspace) = root.get("workspace").and_then(toml::Value::as_table) {
        if let Some(table) = workspace
            .get("dependencies")
            .and_then(toml::Value::as_table)
        {
            inspect_dependencies(
                table,
                "workspace.dependencies",
                path,
                dependency_count,
                findings,
            );
        }
    }
    if let Some(targets) = root.get("target").and_then(toml::Value::as_table) {
        for (target, target_value) in targets {
            let Some(target_table) = target_value.as_table() else {
                continue;
            };
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(table) = target_table.get(section).and_then(toml::Value::as_table) {
                    let name = format!("target.{target}.{section}");
                    inspect_dependencies(table, &name, path, dependency_count, findings);
                }
            }
        }
    }
}

fn inspect_dependencies(
    dependencies: &toml::map::Map<String, toml::Value>,
    section: &str,
    path: &Path,
    dependency_count: &mut usize,
    findings: &mut Vec<String>,
) {
    for (name, dependency) in dependencies {
        // Cargo's string shorthand is only a version requirement; it has no
        // source selector to validate. Inline tables are represented as TOML
        // tables here, the same as expanded dependency tables.
        let Some(dependency) = dependency.as_table() else {
            continue;
        };
        let Some(git) = dependency.get("git").and_then(toml::Value::as_str) else {
            continue;
        };
        if !git.contains("agent-ix/") && !git.contains("agent-ix:") {
            continue;
        }
        *dependency_count += 1;
        let tag = dependency.get("tag").and_then(toml::Value::as_str);
        let branch = dependency.get("branch").and_then(toml::Value::as_str);
        let rev = dependency.get("rev").and_then(toml::Value::as_str);
        if tag.is_none() || branch.is_some() || rev.is_some() {
            let mut selectors = Vec::new();
            if tag.is_none() {
                selectors.push("missing tag".to_owned());
            }
            if branch.is_some() {
                selectors.push("uses branch".to_owned());
            }
            if rev.is_some() {
                selectors.push("uses rev".to_owned());
            }
            findings.push(format!(
                "{} [{section}] dependency {name:?} ({git}) {}",
                path.display(),
                selectors.join(", ")
            ));
        }
    }
}
