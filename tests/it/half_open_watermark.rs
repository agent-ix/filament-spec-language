// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-641-AC-6 (TC-2241): a request-file progress assertion whose watermark
//! is `{"half_open": 0}` covers no tick and has no inclusive reading, so the
//! run reader refuses it at stage `request`; a positive exclusive end `E`
//! reads as the inclusive watermark `E - 1`.

use crate::command_harness::run_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal T using w over (r: Reading) clock \"plant-ms\" on origin {{ eventually[0,5] holds(r.level > 0) }}\n"
    )
}

fn clocks() -> Value {
    json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}})
}

/// A clause-run request of `T` over three samples stamped 0, 2 and 5 with
/// the progress watermark `watermark`.
fn request(watermark: Value) -> Value {
    let observations: Vec<Value> = [(0, 0), (2, 0), (5, 1)]
        .into_iter()
        .map(|(stamp, level): (u64, i64)| {
            json!({"position": {"sample": {"level": {"integer": level.to_string()}}},
                "stamp": stamp})
        })
        .collect();
    json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:half-open", "document": "HalfOpen", "formal_revision": 1}},
        "clocks": clocks(),
        "clause": {"selection": {"temporal": {"name": "T", "trace": {"timestamped": {
            "observations": observations,
            "progress": {"watermark": watermark,
                "completeness": {"complete": {"through": 5}},
                "scope": "closed", "execution": "closed"},
            "lower_boundary": "authoritative-origin",
            "binding": {"profile": STAMPED, "clock": "plant-ms",
                "parameters": clocks()["plant-ms"].clone()}}}}},
            "observations": {}}}})
}

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

#[test]
#[trace("TC-2241", "FR-641-AC-6")]
#[ignore = "QSL-2133: a half-open watermark of 0 refused at the request stage is not implemented yet"]
fn a_half_open_watermark_of_zero_refuses_at_the_request_stage() {
    let directory = tempfile::tempdir().unwrap();
    let refused = run_file(&write(
        directory.path(),
        "zero.json",
        &request(json!({"half_open": 0})),
    ));
    assert_eq!(refused.status.code(), Some(20));
    assert!(refused.stdout.is_empty(), "no result document");
    let envelope: Value = serde_json::from_slice(&refused.stderr).expect("a refusal envelope");
    assert_eq!(envelope["stage"], "request", "{envelope}");
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert!(
        envelope
            .to_string()
            .contains("trace.progress.watermark.half_open"),
        "{envelope}"
    );
    let half_open = run_file(&write(
        directory.path(),
        "six.json",
        &request(json!({"half_open": 6})),
    ));
    let inclusive = run_file(&write(directory.path(), "five.json", &request(json!(5))));
    assert_eq!(half_open.status.code(), inclusive.status.code());
    let disposition = |output: &crate::command_harness::Output| {
        let document: Value = serde_json::from_slice(&output.stdout).expect("a result document");
        document["disposition"].clone()
    };
    assert_eq!(disposition(&half_open), disposition(&inclusive));
}
