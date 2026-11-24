// SPDX-License-Identifier: AGPL-3.0-or-later
//! Manifest and lockfile controls for QSL-2097 dependency measurements.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use toml::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level below the workspace")
        .to_path_buf()
}

fn rust_sources(path: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap_or_else(|error| panic!("{}: {error}", path.display())) {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn dependencies(value: &Value, workspace_dependencies: &Value) -> Vec<(String, String)> {
    let Some(table) = value.as_table() else {
        return Vec::new();
    };
    table
        .iter()
        .map(|(alias, declaration)| {
            let direct = declaration
                .as_table()
                .and_then(|declaration| declaration.get("package"))
                .and_then(Value::as_str)
                .or_else(|| {
                    declaration
                        .as_table()
                        .and_then(|declaration| declaration.get("workspace"))
                        .and_then(Value::as_bool)
                        .filter(|inherited| *inherited)
                        .and_then(|_| workspace_dependencies.get(alias))
                        .and_then(|declaration| declaration.as_table())
                        .and_then(|declaration| declaration.get("package"))
                        .and_then(Value::as_str)
                });
            let name = direct.unwrap_or(alias).to_owned();
            (name, alias.clone())
        })
        .collect()
}

fn manifest_dependencies(
    manifest: &Value,
    workspace_dependencies: &Value,
) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(declaration) = manifest.get(section) {
            result.extend(dependencies(declaration, workspace_dependencies));
        }
    }
    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        for target in targets.values() {
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(declaration) = target.get(section) {
                    result.extend(dependencies(declaration, workspace_dependencies));
                }
            }
        }
    }
    result
}

/// Every workspace member's direct dependency names resolve to a package
/// recorded in the checked-in lockfile, including target and build deps.
#[test]
#[trace("NFR-002-M-1")]
fn direct_workspace_dependencies_are_present_in_the_lockfile() {
    let root = root();
    let workspace: Value = toml::from_str(
        &fs::read_to_string(root.join("Cargo.toml")).expect("read workspace manifest"),
    )
    .expect("parse workspace manifest");
    let lock: Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.lock")).expect("read lockfile"))
            .expect("parse lockfile");
    let locked: BTreeSet<&str> = lock["package"]
        .as_array()
        .expect("lock package array")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    let members = workspace["workspace"]["members"]
        .as_array()
        .expect("workspace members");
    let workspace_dependencies = &workspace["workspace"]["dependencies"];
    for member in members.iter().filter_map(Value::as_str) {
        let manifest_path = root.join(member).join("Cargo.toml");
        let manifest: Value = toml::from_str(
            &fs::read_to_string(&manifest_path)
                .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display())),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display()));
        for (package, alias) in manifest_dependencies(&manifest, workspace_dependencies) {
            assert!(
                locked.contains(package.as_str()),
                "{} dependency `{alias}` resolves to `{package}`, absent from Cargo.lock",
                manifest_path.display()
            );
        }
    }
}

fn manifest_version(manifest: &Value, name: &str) -> &str {
    let declaration = manifest["dependencies"]
        .get(name)
        .unwrap_or_else(|| panic!("xtask declares {name}"));
    let required = declaration
        .as_table()
        .and_then(|declaration| declaration.get("version"))
        .and_then(Value::as_str)
        .or_else(|| declaration.as_str())
        .unwrap_or_else(|| panic!("{name} version"));
    required
}

fn assert_registry_dependency_grant(
    lock: &Value,
    registry: &Path,
    name: &str,
    version: &str,
    expected_license: &str,
) {
    let package = lock["package"]
        .as_array()
        .expect("lock packages")
        .iter()
        .find(|package| {
            package["name"].as_str() == Some(name) && package["version"].as_str() == Some(version)
        })
        .unwrap_or_else(|| panic!("{name} {version} is locked"));
    assert!(package["source"]
        .as_str()
        .is_some_and(|source| source.starts_with("registry+")));
    assert!(package["checksum"]
        .as_str()
        .is_some_and(|checksum| !checksum.is_empty()));

    let crate_manifest = fs::read_dir(&registry)
        .unwrap_or_else(|error| panic!("{}: {error}", registry.display()))
        .filter_map(Result::ok)
        .map(|cache| cache.path().join(format!("{name}-{version}/Cargo.toml")))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("locked {name} source is available for grant inspection"));
    let source: Value = toml::from_str(
        &fs::read_to_string(&crate_manifest).expect("read locked dependency manifest"),
    )
    .expect("parse locked dependency manifest");
    assert_eq!(
        source["package"]["license"].as_str(),
        Some(expected_license),
        "{name} {version} retains its reviewed grant"
    );
    let crate_dir = crate_manifest.parent().expect("crate directory");
    for notice in ["LICENSE-MIT", "LICENSE-APACHE"] {
        let notice = crate_dir.join(notice);
        let contents = fs::read_to_string(&notice)
            .unwrap_or_else(|error| panic!("{}: {error}", notice.display()));
        assert!(!contents.trim().is_empty(), "{} is empty", notice.display());
    }
}

/// Check grants and notices for two independent third-party packages used by
/// xtask. A changed grant on either package fails against its literal reviewed
/// identity, while the lock and source metadata retain the full audit evidence.
#[test]
#[trace("NFR-004-M-2")]
fn locked_registry_dependencies_retain_their_grants_and_notices() {
    let root = root();
    let manifest: Value = toml::from_str(
        &fs::read_to_string(root.join("xtask/Cargo.toml")).expect("read xtask manifest"),
    )
    .expect("parse xtask manifest");
    let lock: Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.lock")).expect("read lockfile"))
            .expect("parse lockfile");
    let registry = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME or CARGO_HOME")).join(".cargo")
        })
        .join("registry/src");
    for (name, license) in [
        ("toml", "MIT OR Apache-2.0"),
        ("serde", "MIT OR Apache-2.0"),
    ] {
        assert_registry_dependency_grant(
            &lock,
            &registry,
            name,
            manifest_version(&manifest, name),
            license,
        );
    }
}

/// The root native compiler has no mandatory opt-in feature. A separate
/// `cargo check --no-default-features` control qualifies the minimal build.
#[test]
#[trace("NFR-002-M-3")]
fn native_compiler_has_no_required_optional_feature() {
    let manifest: Value =
        toml::from_str(&fs::read_to_string(root().join("Cargo.toml")).expect("read root manifest"))
            .expect("parse root manifest");
    let defaults = manifest["features"]["default"]
        .as_array()
        .expect("the default feature set is explicit");
    assert!(
        defaults.is_empty(),
        "default features unexpectedly required: {defaults:?}"
    );
}

/// NFR-004's selected grant is AGPL-3.0-or-later. Inspect every Rust source
/// included in the three allowed test roots, not a hand-maintained subset.
#[test]
#[trace("NFR-004-M-1")]
fn every_included_rust_source_records_its_selected_grant() {
    let root = root();
    let mut sources = Vec::new();
    for path in ["tests/it", "xtask/tests", "fsl-foundation/tests"] {
        rust_sources(&root.join(path), &mut sources);
    }
    assert!(!sources.is_empty(), "included source inventory is empty");
    for source in sources {
        let content = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
        assert_eq!(
            content.lines().next(),
            Some("// SPDX-License-Identifier: AGPL-3.0-or-later"),
            "Rust source {} does not record the exact spec-selected grant",
            source.strip_prefix(&root).unwrap_or(&source).display()
        );
    }
}
