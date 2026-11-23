// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-869-AC-1 (TC-2461): a compile's intake ceilings are caller options, so
//! a refusal at the request-file ceiling names the option that raises it.

use fsl_command::command::{
    compile_outcome_with_limits_and_overrides, RequestOverrides, RunLimits,
};
use ix_trace_rs::trace;
use serde_json::json;

/// FR-869-AC-1: a request file one byte over 1 MiB settles `incomplete` with
/// `resource_exhausted` (exit 22), naming `request_bytes`, its maximum and
/// `--request-bytes`.
#[test]
#[trace("TC-2461", "FR-869-AC-1")]
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
    let error = compile_outcome_with_limits_and_overrides(
        &path,
        RunLimits::default(),
        &RequestOverrides::default(),
    )
    .err()
    .expect("the request is over its ceiling");
    assert_eq!(error.category().exit_code(), 22);
    let failure = error.artifact_value().unwrap();
    assert_eq!(failure["code"], "resource_exhausted", "{failure}");
    assert_eq!(failure["status"], "incomplete", "{failure}");
    assert_eq!(failure["details"]["limit"], "request_bytes", "{failure}");
    assert_eq!(failure["details"]["maximum"], 1024 * 1024, "{failure}");
    assert_eq!(failure["details"]["option"], "--request-bytes", "{failure}");
}
