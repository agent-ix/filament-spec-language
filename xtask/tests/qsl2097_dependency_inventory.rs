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

fn dependencies(value: &Value) -> Vec<(String, String)> {
    let Some(table) = value.as_table() else {
        return Vec::new();
    };
    table
        .iter()
        .map(|(alias, declaration)| {
            let name = declaration
                .get("package")
                .and_then(Value::as_str)
                .unwrap_or(alias)
                .to_owned();
            (name, alias.clone())
        })
        .collect()
}

fn manifest_dependencies(manifest: &Value) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        result.extend(dependencies(&manifest[section]));
    }
    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        for target in targets.values() {
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                result.extend(dependencies(&target[section]));
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
    for member in members.iter().filter_map(Value::as_str) {
        let manifest_path = root.join(member).join("Cargo.toml");
        let manifest: Value = toml::from_str(
            &fs::read_to_string(&manifest_path)
                .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display())),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", manifest_path.display()));
        for (package, alias) in manifest_dependencies(&manifest) {
            assert!(
                locked.contains(package.as_str()),
                "{} dependency `{alias}` resolves to `{package}`, absent from Cargo.lock",
                manifest_path.display()
            );
        }
    }
}

/// Registry packages retain both their registry source and checksum in the
/// lockfile; git packages retain a pinned source revision.
#[test]
#[trace("NFR-004-M-2")]
fn locked_third_party_packages_retain_their_provenance() {
    let lock: Value =
        toml::from_str(&fs::read_to_string(root().join("Cargo.lock")).expect("read lockfile"))
            .expect("parse lockfile");
    for package in lock["package"].as_array().expect("lock packages") {
        let Some(source) = package.get("source").and_then(Value::as_str) else {
            continue;
        };
        if source.starts_with("registry+") {
            assert!(package.get("checksum").and_then(Value::as_str).is_some());
        } else if source.starts_with("git+") {
            assert!(
                source.contains("#"),
                "git source is not revision-pinned: {source}"
            );
        }
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

/// QSL-2097's added and modified Rust sources are the included-file inventory;
/// each one must carry the selected grant in its source header.
#[test]
#[trace("NFR-004-M-1")]
fn every_included_rust_source_records_its_selected_grant() {
    let root = root();
    for path in [
        "tests/it/main.rs",
        "tests/it/temporal_package_route.rs",
        "tests/it/temporal_result_evidence.rs",
        "tests/it/qsl2097_measurements.rs",
        "xtask/tests/qsl2097_meter_memory.rs",
        "xtask/tests/qsl2097_dependency_inventory.rs",
    ] {
        let content =
            fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(
            content
                .lines()
                .take(2)
                .any(|line| line.starts_with("// SPDX-License-Identifier:")),
            "Rust source {path} does not record its selected grant"
        );
    }
}
