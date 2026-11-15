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
