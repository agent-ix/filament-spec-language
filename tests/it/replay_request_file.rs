// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862 (TC-2443), FSpec FR-715 (FSpec TC-1750): `command::run` reads a
//! `native-replay/1` request file in FSpec FR-336's replay request encoding.
//! A replay reads its clock configurations from the proving package, so a
//! `clocks` member is refused at stage `request`, naming it.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn refusal(output: &Output) -> Value {
    assert!(output.stdout.is_empty(), "expected no result document");
    serde_json::from_slice(&output.stderr).expect("a refusal envelope")
}

/// A replay request object in FSpec FR-336's mapping, with `extra` merged
/// into it. Its members are well formed in shape; the refusals below are
/// decided before the package is admitted.
fn replay_request(extra: Value) -> Value {
    let mut request = json!({
        "profile_selections": [],
        "package": {},
        "dependencies": [],
        "replay_target": {"members": {"function": "entry"}},
        "source": {"kind": "input", "assignments": []},
        "obligation_identity": {"node_id": {}, "model": null, "ordinal": "0"},
        "backend": "fsl",
        "state_environment": [],
        "accounting_limits": {},
        "stage_limits": {},
        "supplied_limits": null,
        "object_environment": null,
        "declared_domains": [],
        "byte_provision": []
    });
    if let (Some(members), Some(added)) = (request.as_object_mut(), extra.as_object()) {
        for (name, value) in added {
            members.insert(name.clone(), value.clone());
        }
    }
    json!({"format": "native-replay/1", "request": request})
}

/// The request-stage refusal naming `member`, never the unknown-format
/// refusal a reader without the replay format gives.
fn assert_names(output: &Output, member: &str) {
    assert_eq!(output.status.code(), Some(20));
    let envelope = refusal(output);
    assert_eq!(envelope["stage"], "request", "{envelope}");
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert!(
        envelope.to_string().contains(&format!("\"{member}\""))
            || envelope.to_string().contains(&format!("/{member}")),
        "the refusal names {member}: {envelope}"
    );
    assert!(
        !envelope.to_string().contains("native-replay/1 is not"),
        "a replay request is not an unknown format: {envelope}"
    );
}

/// FR-862-AC-1, FSpec FR-715-AC-1 (negative): a `clocks` member, an unknown
/// member and an empty request each refuse at stage `request`.
#[test]
#[trace("TC-2443", "FR-862-AC-1", "FSpec-TC-1750", "FSpec-FR-715-AC-1")]
#[ignore = "QSL-2292: command::run does not read a native-replay/1 request yet"]
fn a_replay_request_file_refuses_a_clocks_member() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = replay_request(json!({"clocks": {"plant": {"kind": "timed", "unit": "ms"}}}));
    assert_names(
        &run_file(&write(directory.path(), "clocks.json", &clocks)),
        "clocks",
    );
    let extra = replay_request(json!({"extra": true}));
    assert_names(
        &run_file(&write(directory.path(), "extra.json", &extra)),
        "extra",
    );
    let empty = json!({"format": "native-replay/1", "request": {}});
    assert_names(
        &run_file(&write(directory.path(), "empty.json", &empty)),
        "profile_selections",
    );
}
