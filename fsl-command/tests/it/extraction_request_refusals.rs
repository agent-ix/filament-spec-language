// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(feature = "quire-extraction")]

use fsl_command::command::{read_clause_run_request, RunCause};
use serde_json::{json, Value};
use std::fs;

fn write_request(directory: &std::path::Path, request: Value) -> std::path::PathBuf {
    let path = directory.join("request.json");
    let envelope = json!({"format": "native-run/1", "request": request});
    fs::write(
        &path,
        serde_json::to_vec(&envelope).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn request(binding_count: usize) -> Value {
    json!({
        "models": [],
        "program": {
            "source": {
                "file": "original.md",
                "authority": "agent-ix",
                "identity": "original",
                "document": "Original",
                "formal_revision": 1
            },
            "extraction": {
                "body": {"authority": "agent-ix", "identity": "body", "document": "Body"}
            },
            "clauses": (0..binding_count)
                .map(|index| json!({"clause": format!("clause-{index}"), "package": ""}))
                .collect::<Vec<_>>()
        },
        "clause": {
            "selection": {"function": {"name": "healthy", "arguments": [], "snapshot": "s"}},
            "observations": {}
        }
    })
}

fn outcome(error: &fsl_command::command::RunError) -> Value {
    error.value().expect("error envelope serializes")
}

fn refusal(path: &std::path::Path) -> Box<fsl_command::command::RunError> {
    match read_clause_run_request(path) {
        Err(error) => error,
        Ok(_) => panic!("request unexpectedly produced a successful clause run"),
    }
}

/// Trace: TC-109, FR-031-AC-6.
#[test]
fn extraction_and_package_conflict_refuses_before_reading_files() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let mut request = request(1);
    request["package"] = json!({"file": "absent-package.json"});
    let path = write_request(directory.path(), request);

    let error = refusal(&path);
    assert!(matches!(&error.cause, RunCause::ExtractionPackageConflict));
    let outcome = outcome(&error);
    assert_eq!(outcome["stage"], "request");
    assert_eq!(error.category().exit_code(), 20);
    assert_eq!(outcome["code"], "extraction-package-conflict");
}

/// Trace: TC-109, FR-031-AC-6.
#[test]
fn extraction_requires_one_clause_binding_and_reports_the_actual_count() {
    for actual in [0, 2] {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = write_request(directory.path(), request(actual));

        let error = refusal(&path);
        assert!(
            matches!(&error.cause, RunCause::ExtractionClauseCount { actual: count } if *count == actual)
        );
        let outcome = outcome(&error);
        assert_eq!(outcome["stage"], "request");
        assert_eq!(error.category().exit_code(), 20);
        assert_eq!(outcome["code"], "extraction-clause-count");
        assert_eq!(outcome["details"]["actual"], actual);
    }
}

/// Trace: TC-109, FR-031-AC-7.
#[test]
fn quire_context_refusal_preserves_its_diagnostics() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::write(directory.path().join("original.md"), "# Clause\n").expect("source writes");
    let path = write_request(directory.path(), request(1));

    let error = refusal(&path);
    let RunCause::InvalidQuireContext { diagnostics } = &error.cause else {
        panic!("expected invalid Quire context, got {:?}", error.cause);
    };
    let outcome = outcome(&error);
    assert_eq!(outcome["stage"], "quire-context");
    assert_eq!(error.category().exit_code(), 20);
    assert_eq!(outcome["code"], "invalid-quire-context");
    assert_eq!(
        outcome["details"]["diagnostics"],
        json!(diagnostics)
    );
}
