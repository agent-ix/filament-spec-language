// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2140, TC-2142 and TC-2143 (FR-760): the standard is a pinned Cargo
//! dependency, its vectors are read from that pin and never from a sibling
//! checkout, a checked package of another document type is refused, and the
//! facade crates carry the workspace version.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use serde_json::Value;

const STANDARD_PACKAGE: &str = "filament-specification-qualification";
const STANDARD_GIT: &str = "git+https://github.com/agent-ix/filament-specification";
const FIXTURE: &str = "proposals/checked-package-v2/fixtures/positive-control-operations.json";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn metadata(no_deps: bool) -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut command = Command::new(cargo);
    command.current_dir(workspace_root()).args([
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
    ]);
    if no_deps {
        command.arg("--no-deps");
    }
    let output = command.output().expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata writes JSON")
}

/// The directory of the pinned standard package, after checking that its
/// source is a tagged git source of the standard's repository.
fn pinned_standard_directory() -> PathBuf {
    let metadata = metadata(false);
    let package = metadata["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .find(|package| package["name"] == STANDARD_PACKAGE)
        .unwrap_or_else(|| panic!("{STANDARD_PACKAGE} is not a dependency of the workspace"));
    let source = package["source"].as_str().expect("a non-path source");
    assert!(
        source.starts_with(STANDARD_GIT) && source.contains("?tag=v"),
        "the standard is not pinned by a release tag: {source}"
    );
    Path::new(package["manifest_path"].as_str().expect("manifest path"))
        .parent()
        .expect("manifest directory")
        .to_path_buf()
}

fn pinned_fixture() -> Value {
    let bytes = fs::read(pinned_standard_directory().join(FIXTURE)).expect("pinned fixture");
    serde_json::from_slice(&bytes).expect("fixture is JSON")
}

fn scanned_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            scanned_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "toml")
        {
            out.push(path);
        }
    }
}

#[test]
#[trace("TC-2140", "FR-760-AC-1", "FSpec-TC-1641", "FSpec-FR-660-AC-2")]
#[ignore = "QSL-2227: the standard as a tag-pinned Cargo dependency is not implemented yet"]
fn the_standard_is_a_tag_pinned_dependency_whose_fixture_carries_the_wire_tag() {
    let fixture = pinned_fixture();
    assert_eq!(
        fixture["contract_version"],
        fsl_package::CHECKED_PACKAGE_V2,
        "the pinned fixture carries the document type FSL writes and reads"
    );
}

#[test]
#[trace("TC-2140", "FR-760-AC-2", "FSpec-TC-1641", "FSpec-FR-660-AC-2")]
#[ignore = "QSL-2227: removal of the FSPEC_DIR sibling-checkout lookup is not implemented yet"]
fn no_file_locates_the_standard_by_variable_or_sibling_path() {
    let root = workspace_root();
    let mut files = vec![root.join("Makefile")];
    for directory in ["xtask/src", "src", "tests"] {
        scanned_files(&root.join(directory), &mut files);
    }
    for entry in fs::read_dir(&root).expect("workspace root") {
        let path = entry.expect("entry").path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("fsl-"))
        {
            scanned_files(&path.join("src"), &mut files);
            scanned_files(&path.join("tests"), &mut files);
        }
    }
    let this_file = root.join(file!());
    let offenders: Vec<String> = files
        .iter()
        .filter(|path| **path != this_file)
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|text| {
                text.contains("FSPEC_DIR") || text.contains("../filament-specification")
            })
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "these files locate the standard outside Cargo: {offenders:?}"
    );
}

#[test]
#[trace("TC-2142", "FR-760-AC-4", "FSpec-TC-1643", "FSpec-FR-660-AC-4")]
#[ignore = "QSL-2227: reading the pinned standard's fixtures is not implemented yet"]
fn a_checked_package_of_another_document_type_is_refused() {
    let mut fixture = pinned_fixture();
    fixture["contract_version"] = Value::from("filament.checked-package/v3");
    let bytes = serde_json::to_vec(&fixture).expect("fixture re-encodes");
    let evidence = filament_contract_model::CheckedPackageEvidence::new();
    match filament_contract_model::read_checked_package(
        &bytes,
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::Refused(refusal) => {
            assert_eq!(
                refusal.code,
                filament_contract_model::CheckedPackageRefusalCode::UnknownContractVersion
            );
            assert_eq!(
                refusal.contract_version.as_deref(),
                Some("filament.checked-package/v3")
            );
        }
        other => panic!("another document type was not refused: {other:?}"),
    }
}

#[test]
#[trace("TC-2143", "FR-760-AC-5")]
#[ignore = "QSL-2227: facade crates carrying the workspace version is not implemented yet"]
fn facade_crates_carry_the_workspace_version() {
    let metadata = metadata(true);
    let packages = metadata["packages"].as_array().expect("packages array");
    let version_of = |name: &str| {
        packages
            .iter()
            .find(|package| package["name"] == name)
            .and_then(|package| package["version"].as_str())
            .unwrap_or_else(|| panic!("{name} is a workspace package"))
            .to_owned()
    };
    let root = version_of("filament-spec-language");
    for facade in ["fsl-package", "fsl-replay", "fsl-analyze"] {
        assert_eq!(
            version_of(facade),
            root,
            "{facade} carries the workspace version"
        );
    }
}

/// The pinned standard package's directory, `version` and release tag.
fn pinned_standard() -> (PathBuf, String, String) {
    let directory = pinned_standard_directory();
    let metadata = metadata(false);
    let package = metadata["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .find(|package| package["name"] == STANDARD_PACKAGE)
        .expect("the standard is a dependency");
    let version = package["version"].as_str().expect("version").to_owned();
    let source = package["source"].as_str().expect("source");
    let tag = source
        .split("?tag=")
        .nth(1)
        .and_then(|rest| rest.split('#').next())
        .expect("a tag source")
        .to_owned();
    (directory, version, tag)
}

/// `(major, minor, patch)` of a `X.Y.Z` version.
fn semver(version: &str) -> (u64, u64, u64) {
    let parts: Vec<u64> = version
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().expect("numeric version part"))
        .collect();
    assert_eq!(parts.len(), 3, "{version} is X.Y.Z");
    (parts[0], parts[1], parts[2])
}

fn pinned_inventory(directory: &Path) -> Value {
    let path = directory.join("proposals/consumer-facades/consumer-facts.json");
    serde_json::from_slice(&fs::read(path).expect("the pinned inventory")).expect("inventory JSON")
}

/// The requirement file an inventory `requirement` URI names, in the pinned
/// standard or in this repository.
fn requirement_file(standard: &Path, uri: &str) -> Option<PathBuf> {
    let (root, id) = if let Some(id) = uri.strip_prefix("ix://agent-ix/filament-specification/") {
        (standard.join("spec"), id)
    } else {
        let id = uri.strip_prefix("ix://agent-ix/filament-spec-language/")?;
        (workspace_root().join("spec"), id)
    };
    let mut files = Vec::new();
    markdown_files(&root, &mut files);
    let prefix = format!("{id}-");
    files.into_iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&prefix))
    })
}

fn markdown_files(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            out.push(path);
        }
    }
}

#[test]
#[trace("FSpec-TC-1640", "FSpec-FR-660-AC-1")]
#[ignore = "QSL-2227: the tag-pinned standard and its published-file accessors are not implemented yet"]
fn the_pinned_standard_version_equals_its_tag_and_exposes_published_files() {
    let (directory, version, tag) = pinned_standard();
    assert_eq!(
        format!("v{version}"),
        tag,
        "the package version is the tag without its v"
    );
    let library = fs::read_to_string(directory.join("src/lib.rs")).expect("the standard library");
    assert!(
        library.contains("pub fn published_file("),
        "published_file is exported"
    );
    assert!(
        library.contains("pub fn published_files("),
        "published_files is exported"
    );
}

#[test]
#[trace("FSpec-TC-1644", "FSpec-FR-660-AC-5")]
#[ignore = "QSL-2227: tag-pinned standard releases are not implemented yet"]
fn the_resolved_standard_carries_a_semantic_version_promise() {
    // `pinned_standard_directory` (offline `cargo metadata`) already refuses a
    // source that is not a tagged git source of the standard; a `branch`
    // source fails there, naming the package.
    let (_, version, tag) = pinned_standard();
    let (major, minor, patch) = semver(&version);
    assert_eq!(
        tag,
        format!("v{major}.{minor}.{patch}"),
        "the resolved version {version} equals its tag {tag}"
    );
}

#[test]
#[trace("FSpec-TC-1645", "FSpec-FR-661-AC-1")]
#[ignore = "QSL-2228: the consumer-fact inventory read from the pinned standard is not implemented yet"]
fn the_pinned_inventory_has_its_closed_shape() {
    let (directory, _, _) = pinned_standard();
    let inventory = pinned_inventory(&directory);
    let object = inventory.as_object().expect("inventory object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["consumers", "inventory"]);
    assert_eq!(inventory["inventory"], "filament.consumer-facts/v1");
    let expected = [
        ("ir", "IR"),
        ("protocol", "PR"),
        ("code-generation", "CG"),
        ("runtime", "RT"),
        ("driver", "DR"),
        ("observation", "OB"),
    ];
    let consumers = inventory["consumers"].as_array().expect("consumers");
    assert_eq!(consumers.len(), expected.len());
    for (consumer, (name, code)) in consumers.iter().zip(expected) {
        assert_eq!(consumer["consumer"], name);
        let facts = consumer["facts"].as_array().expect("facts");
        assert!(!facts.is_empty(), "{name} lists facts");
        for (ordinal, fact) in facts.iter().enumerate() {
            assert_eq!(fact["id"], format!("CF-{code}-{:02}", ordinal + 1));
            let uri = fact["requirement"].as_str().expect("requirement URI");
            assert!(
                requirement_file(&directory, uri).is_some(),
                "{uri} names an existing requirement"
            );
        }
    }
}

#[test]
#[trace("FSpec-TC-1645", "FSpec-FR-661-AC-2")]
#[ignore = "QSL-2228: the consumer-fact inventory read from the pinned standard is not implemented yet"]
fn the_pinned_inventory_lists_the_required_facts() {
    let (directory, _, _) = pinned_standard();
    let inventory = pinned_inventory(&directory);
    let text: Vec<(String, String)> = inventory["consumers"]
        .as_array()
        .expect("consumers")
        .iter()
        .flat_map(|consumer| {
            let name = consumer["consumer"].as_str().expect("name").to_owned();
            consumer["facts"]
                .as_array()
                .expect("facts")
                .iter()
                .map(move |fact| {
                    (
                        name.clone(),
                        fact["fact"].as_str().expect("fact").to_lowercase(),
                    )
                })
        })
        .collect();
    let required = [
        ("ir", "document type"),
        ("ir", "import view"),
        ("ir", "schedulability claim roots"),
        ("ir", "obligation identity of a schedulability claim"),
        ("protocol", "protocol view"),
        ("protocol", "causal relation"),
        ("protocol", "endpoints"),
        ("protocol", "internality"),
        ("protocol", "await"),
        ("protocol", "visibility of a choice"),
        ("protocol", "attempts maximum"),
        ("code-generation", "call site of a function"),
        ("code-generation", "imported function"),
        ("code-generation", "obligation identity"),
        ("code-generation", "closed-form"),
        ("runtime", "monitor plan root"),
        ("runtime", "progress view"),
        ("driver", "reader of native-run-result/2"),
        ("driver", "writer of a clause run result"),
        ("driver", "typed run result"),
        ("observation", "subject kinds"),
    ];
    for (consumer, phrase) in required {
        assert!(
            text.iter()
                .any(|(name, fact)| name == consumer && fact.contains(phrase)),
            "{consumer} lists a fact about {phrase}"
        );
    }
}

#[test]
#[trace("FSpec-TC-1647", "FSpec-FR-661-AC-4")]
#[ignore = "QSL-2228: the consumer-fact inventory read from the pinned standard is not implemented yet"]
fn every_inventory_fact_requirement_names_its_fact_id() {
    let (directory, _, _) = pinned_standard();
    let inventory = pinned_inventory(&directory);
    for consumer in inventory["consumers"].as_array().expect("consumers") {
        for fact in consumer["facts"].as_array().expect("facts") {
            let id = fact["id"].as_str().expect("id");
            let uri = fact["requirement"].as_str().expect("requirement");
            let file = requirement_file(&directory, uri)
                .unwrap_or_else(|| panic!("{uri} names an existing requirement"));
            let text = fs::read_to_string(&file).expect("requirement text");
            assert!(text.contains(id), "{} names {id}", file.display());
        }
    }
}
