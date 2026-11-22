// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2265: the README's first rule compiles against its named model.

use ix_trace_rs::trace;
use serde_json::json;

const README: &str = include_str!("../../README.md");
const CONFIG_VERSION_MODEL: &str =
    include_str!("../../examples/config-version/model.semantic-ir.json");

/// The FSL code fence under the README's "A first rule" heading.
fn first_rule_source() -> String {
    let section = README
        .split_once("## A first rule\n")
        .map(|(_, section)| section)
        .expect("README has the A first rule section")
        .split_once("\n### Rules in Markdown specs\n")
        .map(|(section, _)| section)
        .expect("README first-rule section has a following section");
    let (_, after_open) = section
        .split_once("```fsl stage=parse\n")
        .expect("README first-rule section has a code fence");
    let (source, _) = after_open
        .split_once("\n```")
        .expect("README first-rule code fence is closed");
    source.to_owned()
}

/// Provenance: QSL-2265.
#[trace("FR-027-AC-9")]
#[test]
fn first_rule_readme_example_compiles_against_config_version() {
    let directory = tempfile::tempdir().expect("temporary compile directory");
    std::fs::write(
        directory.path().join("first-rule.native"),
        first_rule_source(),
    )
    .expect("write extracted README source");
    std::fs::write(
        directory.path().join("config-version.json"),
        CONFIG_VERSION_MODEL,
    )
    .expect("write ConfigVersion model");

    let request = json!({
        "format": "native-compile/1",
        "request": {
            "models": [{
                "format": "semantic-ir/2.0.0",
                "source": {
                    "file": "config-version.json",
                    "authority": "agent-ix",
                    "identity": "test:readme-config-version",
                    "document": "ConfigVersion",
                    "formal_revision": 1
                }
            }],
            "program": {"source": {
                "file": "first-rule.native",
                "authority": "agent-ix",
                "identity": "test:readme-first-rule",
                "document": "README-A-first-rule",
                "formal_revision": 1
            }}
        }
    });
    std::fs::write(
        directory.path().join("compile.json"),
        serde_json::to_vec(&request).expect("serialize compile request"),
    )
    .expect("write compile request");

    let output = crate::command_harness::compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "README first rule must compile against example/config-version: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
