// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-869-AC-1 (TC-2461): a compile's intake ceilings are caller options, so
//! a refusal at the request-file ceiling names the option that raises it.
//! `compile_outcome_with_limits_and_overrides` and `RequestOverrides::limits`
//! (FR-869-AC-1, AC-2) are the code ticket's to add and test.

use crate::command_harness::compile_file;
use ix_trace_rs::trace;
use serde_json::json;

/// FR-869-AC-1: a request file one byte over 1 MiB.
#[test]
#[trace("TC-2461", "FR-869-AC-1")]
#[ignore = "QSL-2190: compile's request_bytes refusal naming --request-bytes is not implemented yet"]
fn a_compile_request_over_its_ceiling_names_the_option() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(
        "tests/fixtures/spine-compile.native",
        directory.path().join("program.native"),
    )
    .unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "agent-ix",
            "identity": "test:fr-869", "document": "Spine", "formal_revision": 1}},
    }});
    let mut bytes = serde_json::to_vec(&request).unwrap();
    bytes.resize(1024 * 1024 + 1, b' ');
    let path = directory.path().join("compile.json");
    std::fs::write(&path, bytes).unwrap();
    let output = compile_file(&path);
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let failure = String::from_utf8_lossy(&output.stderr);
    assert!(failure.contains("request_bytes"), "{failure}");
    assert!(failure.contains("--request-bytes"), "{failure}");
}
