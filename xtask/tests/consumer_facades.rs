// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2141 and TC-2144 to TC-2147 (FR-760, FR-761): the pin check over the
//! workspace manifests, and the consumer API contract check that binds every
//! fact of the standard's consumer-fact inventory to a public accessor.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{json, Value};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn xtask(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("xtask runs")
}

fn report(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn committed_bindings() -> Value {
    let text = fs::read_to_string(workspace_root().join("docs/consumer-facades.json"))
        .expect("the committed binding document");
    serde_json::from_str(&text).expect("bindings are JSON")
}

fn protocol_facts(bindings: &mut Value) -> &mut Vec<Value> {
    bindings["consumers"]
        .as_array_mut()
        .expect("consumers array")
        .iter_mut()
        .find(|consumer| consumer["consumer"] == "protocol")
        .expect("the protocol consumer is bound")["facts"]
        .as_array_mut()
        .expect("facts array")
}

/// Runs the contract check over `bindings` written to a scratch file.
fn check_with(bindings: &Value) -> Output {
    let scratch = tempfile::tempdir().expect("scratch directory");
    let path = scratch.path().join("consumer-facades.json");
    fs::write(&path, serde_json::to_vec_pretty(bindings).expect("encode")).expect("write");
    xtask(&[
        "consumer-facades",
        "--bindings",
        path.to_str().expect("UTF-8 path"),
    ])
}

#[test]
#[trace("TC-2141", "FR-760-AC-3", "FSpec-TC-1642", "FSpec-FR-660-AC-3")]
#[ignore = "QSL-2227: `xtask pin-check` and tag pins on every agent-ix dependency are not implemented yet"]
fn pin_check_passes_the_workspace_and_refuses_a_branch_dependency() {
    let workspace = xtask(&["pin-check"]);
    assert_eq!(workspace.status.code(), Some(0), "{}", report(&workspace));

    let scratch = tempfile::tempdir().expect("scratch directory");
    let manifest = scratch.path().join("Cargo.toml");
    fs::write(
        &manifest,
        "[package]\nname = \"scratch\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [dependencies]\nfilament-contract-model = { git = \"https://github.com/agent-ix/filament-contract-ir\", branch = \"main\" }\n",
    )
    .expect("write scratch manifest");
    fs::create_dir(scratch.path().join("src")).expect("src");
    fs::write(scratch.path().join("src/lib.rs"), "").expect("lib.rs");
    let refused = xtask(&[
        "pin-check",
        "--manifest",
        manifest.to_str().expect("UTF-8 path"),
    ]);
    let text = report(&refused);
    assert_eq!(refused.status.code(), Some(1), "{text}");
    assert!(text.contains("filament-contract-model"), "{text}");
    assert!(
        text.contains(manifest.to_str().expect("UTF-8 path")),
        "{text}"
    );
}

#[test]
#[trace("TC-2144", "FR-761-AC-1", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]
#[ignore = "QSL-2228: `xtask consumer-facades` and the bound accessors are not implemented yet"]
fn the_contract_check_passes_over_the_committed_bindings() {
    let output = xtask(&["consumer-facades"]);
    let text = report(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    let bindings = committed_bindings();
    for consumer in bindings["consumers"].as_array().expect("consumers") {
        for fact in consumer["facts"].as_array().expect("facts") {
            let id = fact["fact"].as_str().expect("fact id");
            assert!(text.contains(id), "the report lists {id}: {text}");
        }
    }
    for path in [
        "fsl_package::protocol_view::role_internality",
        "fsl_package::SchedulabilityRoot::obligation_identity",
        "fsl_replay::check_closed_form",
        "fsl_analyze::monitor_root::admit_progress_view",
        "fsl_package::checked_views::subject_kinds",
    ] {
        assert!(text.contains(path), "the report resolves {path}: {text}");
    }
}

#[test]
#[trace("TC-2145", "FR-761-AC-2", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]
#[ignore = "QSL-2228: `xtask consumer-facades` is not implemented yet"]
fn an_accessor_no_module_exposes_fails_the_check() {
    let mut bindings = committed_bindings();
    let facts = protocol_facts(&mut bindings);
    let endpoints = facts
        .iter_mut()
        .find(|fact| fact["fact"] == "CF-PR-03")
        .expect("CF-PR-03 is bound");
    endpoints["accessors"] = json!(["no_such_accessor"]);
    let output = check_with(&bindings);
    let text = report(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("CF-PR-03"), "{text}");
    assert!(
        text.contains("fsl_package::protocol_view::no_such_accessor"),
        "{text}"
    );
}

#[test]
#[trace("TC-2146", "FR-761-AC-3", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]
#[ignore = "QSL-2228: `xtask consumer-facades` is not implemented yet"]
fn an_unbound_inventory_fact_fails_the_check() {
    let mut bindings = committed_bindings();
    protocol_facts(&mut bindings).retain(|fact| fact["fact"] != "CF-PR-04");
    let output = check_with(&bindings);
    let text = report(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("CF-PR-04"), "{text}");
    assert!(text.contains("unbound"), "{text}");
}

#[test]
#[trace("TC-2147", "FR-761-AC-4", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]
#[ignore = "QSL-2228: `xtask consumer-facades` is not implemented yet"]
fn a_binding_with_no_inventory_fact_fails_the_check() {
    let mut bindings = committed_bindings();
    protocol_facts(&mut bindings).push(json!({"fact": "CF-PR-99", "accessors": ["protocol_view"]}));
    let output = check_with(&bindings);
    let text = report(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("CF-PR-99"), "{text}");
}
