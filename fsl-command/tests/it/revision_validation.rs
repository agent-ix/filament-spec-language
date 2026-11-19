// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request identity refusals name the wire member that failed validation.

use fsl_command::command::{self, PreparedRun, RunLimits};
use ix_trace_rs::trace;
use serde_json::json;
use std::path::Path;

const PROGRAM: &[u8] = include_bytes!("../../../tests/fixtures/spine-run.native");

fn write_request(directory: &Path, revision: u64) -> std::path::PathBuf {
    if revision > 0 {
        std::fs::write(directory.join("program.native"), PROGRAM).unwrap();
    }
    let request = json!({
        "format": "native-run/1",
        "request": {
            "models": [],
            "program": {"source": {
                "file": "program.native",
                "authority": "test",
                "identity": "test:program",
                "document": "RevisionCheck",
                "formal_revision": revision
            }},
            "call": {"function": "seven", "arguments": []}
        }
    });
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

#[test]
#[trace("TC-450", "FR-100-AC-3")]
fn zero_formal_revision_names_the_request_member_and_revision_one_is_admitted() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_request(directory.path(), 0);
    let error = command::prepare_run(&path, RunLimits::default())
        .err()
        .expect("revision zero is refused");
    let message = error.to_string();
    assert_eq!(message, "program.source.formal_revision must be positive");
    assert!(!message.contains("Diagnostic {"));
    assert!(!message.contains("Span {"));

    let document = error.value().unwrap();
    assert_eq!(document["code"], "invalid-identifier");
    assert_eq!(document["message"], message);
    assert_eq!(
        document["details"]["path"],
        "program.source.formal_revision"
    );
    assert_eq!(
        document["details"]["message"],
        "program.source.formal_revision must be positive"
    );
    let rendered = document.to_string();
    assert!(!rendered.contains("Diagnostic {"));
    assert!(!rendered.contains("Span {"));

    let positive = write_request(directory.path(), 1);
    assert!(matches!(
        command::prepare_run(&positive, RunLimits::default()).unwrap(),
        PreparedRun::Function(_)
    ));
}
