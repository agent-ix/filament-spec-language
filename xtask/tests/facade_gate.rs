// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2574 to TC-2581 (FR-911, FR-912): every consumer fact has a contract
//! test that an outside crate compiles and runs, and `cargo xtask
//! facade-tests` fails on a fact with no live test, an ignored or disabled
//! test, and a test that names no inventory fact.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

const CONSUMER: &str = "facade-contract-consumer";
const CONSUMER_DIR: &str = "tests/fixtures/facade-contract-consumer";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits in the workspace")
        .to_path_buf()
}

fn cargo(args: &[&str]) -> Output {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    Command::new(cargo)
        .current_dir(workspace_root())
        .args(args)
        .output()
        .expect("cargo runs")
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

/// The fact ids the committed bindings name, in binding order.
fn bound_facts() -> Vec<String> {
    let text = fs::read_to_string(workspace_root().join("docs/consumer-facades.json"))
        .expect("the committed binding document");
    let bindings: Value = serde_json::from_str(&text).expect("bindings are JSON");
    bindings["consumers"]
        .as_array()
        .expect("consumers array")
        .iter()
        .flat_map(|consumer| consumer["facts"].as_array().expect("facts array").iter())
        .map(|fact| fact["fact"].as_str().expect("a fact id").to_owned())
        .collect()
}

/// `CF-PR-03` as the test-name prefix the gate reads, `cf_pr_03_`.
fn test_prefix(fact: &str) -> String {
    format!("{}_", fact.to_ascii_lowercase().replace('-', "_"))
}

/// A scratch contract-test directory with one live test per bound fact.
fn scratch_tests(extra: &str, skip: Option<&str>, attribute_on_pr_03: &str) -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("scratch directory");
    let mut source = String::new();
    for fact in bound_facts() {
        if skip == Some(fact.as_str()) {
            continue;
        }
        let attribute = if fact == "CF-PR-03" {
            attribute_on_pr_03
        } else {
            ""
        };
        source.push_str(&format!(
            "#[test]\n{attribute}\nfn {}reads_the_fact() {{}}\n",
            test_prefix(&fact)
        ));
    }
    source.push_str(extra);
    fs::write(scratch.path().join("contract.rs"), source).expect("write the scratch tests");
    scratch
}

fn gate_over(directory: &Path) -> Output {
    xtask(&[
        "facade-tests",
        "--tests",
        directory.to_str().expect("UTF-8 path"),
    ])
}

#[test]
#[trace("TC-2578", "FR-912-AC-1")]
fn every_bound_fact_id_has_a_test_prefix_the_gate_reads() {
    for fact in bound_facts() {
        let prefix = test_prefix(&fact);
        let parts: Vec<&str> = prefix.trim_end_matches('_').split('_').collect();
        assert_eq!(parts.len(), 3, "{fact} is CF-<code>-<nn>");
        assert_eq!(parts[0], "cf");
        assert!(
            ["ir", "pr", "cg", "rt", "dr", "ob"].contains(&parts[1]),
            "{fact}"
        );
        assert!(
            parts[2].len() == 2 && parts[2].bytes().all(|byte| byte.is_ascii_digit()),
            "{fact}"
        );
    }
}

#[test]
#[trace("TC-2574", "FR-911-AC-1")]
#[ignore = "QSL-2231: the `facade-contract-consumer` crate is not implemented yet"]
fn the_contract_consumer_sees_only_default_features() {
    let output = cargo(&["metadata", "--locked", "--offline", "--format-version", "1"]);
    assert!(output.status.success(), "{}", report(&output));
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata JSON");
    let package = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == CONSUMER)
        .unwrap_or_else(|| panic!("{CONSUMER} is not a workspace package"));
    assert!(package["manifest_path"]
        .as_str()
        .expect("manifest path")
        .contains(CONSUMER_DIR));
    for dependency in package["dependencies"].as_array().expect("dependencies") {
        let name = dependency["name"].as_str().expect("dependency name");
        if name.starts_with("fsl-") {
            assert_eq!(
                dependency["features"].as_array().map(Vec::len),
                Some(0),
                "{name} enables a feature"
            );
            assert_eq!(dependency["uses_default_features"], true, "{name}");
        }
    }
    let resolve = metadata["resolve"]["nodes"]
        .as_array()
        .expect("resolve nodes");
    let consumer_id = package["id"].as_str().expect("package id");
    let node = resolve
        .iter()
        .find(|node| node["id"] == consumer_id)
        .expect("the consumer is resolved");
    assert!(
        !node["features"]
            .as_array()
            .expect("features")
            .iter()
            .any(|feature| feature == "test-support"),
        "the consumer enables test-support"
    );
}

#[test]
#[trace("TC-2575", "FR-911-AC-2", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]
#[ignore = "QSL-2231: the per-fact contract call tests are not implemented yet"]
fn every_bound_fact_has_a_passing_contract_call_test() {
    let listed = cargo(&["test", "-p", CONSUMER, "--", "--list"]);
    assert!(listed.status.success(), "{}", report(&listed));
    let names = String::from_utf8_lossy(&listed.stdout).into_owned();
    for fact in bound_facts() {
        let prefix = test_prefix(&fact);
        assert!(
            names.lines().any(|line| line
                .rsplit("::")
                .next()
                .is_some_and(|name| name.starts_with(&prefix))),
            "no contract test reads {fact}"
        );
    }
    let run = cargo(&["test", "-p", CONSUMER, "--", "--include-ignored"]);
    assert!(run.status.success(), "{}", report(&run));
    assert!(report(&run).contains(" 0 ignored"), "{}", report(&run));
}

#[test]
#[trace("TC-2576", "FR-911-AC-3")]
#[ignore = "QSL-2231: the protocol contract tests' typed-refusal cases are not implemented yet"]
fn a_wrong_input_to_a_protocol_accessor_gives_its_typed_refusal() {
    let source = fs::read_to_string(
        workspace_root()
            .join(CONSUMER_DIR)
            .join("tests/it/protocol.rs"),
    )
    .expect("the protocol contract tests");
    assert!(source.contains("fsl_package::protocol_view::channel_endpoints"));
    assert!(source.contains("ViewRefusal"));
    let run = cargo(&["test", "-p", CONSUMER, "cf_pr_03"]);
    assert!(run.status.success(), "{}", report(&run));
}

#[test]
#[trace("TC-2577", "FR-911-AC-4")]
#[ignore = "QSL-2231: the consumer crate's private-item compile-fail doctest is not implemented yet"]
fn a_private_item_is_not_reachable_from_the_consumer_crate() {
    let library = fs::read_to_string(workspace_root().join(CONSUMER_DIR).join("src/lib.rs"))
        .expect("the consumer library");
    assert!(library.contains("compile_fail,E0603"));
    assert!(library.contains("fsl_package::protocol_view::relation_over"));
    let doc = cargo(&["test", "-p", CONSUMER, "--doc"]);
    assert!(doc.status.success(), "{}", report(&doc));
}

#[test]
#[trace("TC-2578", "FR-912-AC-1")]
#[ignore = "QSL-2232: `xtask facade-tests` is not implemented yet"]
fn gate_passes_when_every_fact_has_a_live_test() {
    let scratch = scratch_tests("", None, "");
    let output = gate_over(scratch.path());
    assert_eq!(output.status.code(), Some(0), "{}", report(&output));
    let text = report(&output);
    for fact in bound_facts() {
        assert!(text.contains(&fact), "{fact} is not reported");
        assert!(
            text.contains(&test_prefix(&fact)),
            "{fact}'s test is not reported"
        );
    }
}

#[test]
#[trace("TC-2579", "FR-912-AC-2")]
#[ignore = "QSL-2232: `xtask facade-tests` is not implemented yet"]
fn gate_fails_on_fact_without_test() {
    let scratch = scratch_tests("", Some("CF-PR-03"), "");
    let output = gate_over(scratch.path());
    assert_eq!(output.status.code(), Some(1), "{}", report(&output));
    assert!(report(&output).contains("CF-PR-03"));
}

#[test]
#[trace("TC-2580", "FR-912-AC-3")]
#[ignore = "QSL-2232: `xtask facade-tests` is not implemented yet"]
fn gate_fails_on_ignored_contract_test() {
    for attribute in ["#[ignore]", "#[cfg(any())]"] {
        let scratch = scratch_tests("", None, attribute);
        let output = gate_over(scratch.path());
        assert_eq!(
            output.status.code(),
            Some(1),
            "{attribute}: {}",
            report(&output)
        );
        let text = report(&output);
        assert!(
            text.contains("cf_pr_03_reads_the_fact"),
            "{attribute}: {text}"
        );
        assert!(text.contains("contract.rs"), "{attribute}: {text}");
        assert!(text.contains("CF-PR-03"), "{attribute}: {text}");
    }
}

#[test]
#[trace("TC-2581", "FR-912-AC-4")]
#[ignore = "QSL-2232: `xtask facade-tests` and its `make ci` step are not implemented yet"]
fn gate_fails_on_test_for_unlisted_fact() {
    let scratch = scratch_tests("#[test]\nfn cf_pr_99_unknown() {}\n", None, "");
    let output = gate_over(scratch.path());
    assert_eq!(output.status.code(), Some(1), "{}", report(&output));
    let text = report(&output);
    assert!(text.contains("cf_pr_99_unknown"));
    assert!(text.contains("CF-PR-99"));
    let makefile = fs::read_to_string(workspace_root().join("Makefile")).expect("Makefile");
    assert!(makefile.contains("xtask facade-tests"));
}
