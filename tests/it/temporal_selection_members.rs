// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-985 (TC-2781, TC-2782): the temporal selection's closed member set
//! `{name, trace, over, ledger, relation, evidence_ledger, admit_late}`,
//! the shapes of the three correction members and their refusals.
use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal Seen using w over (r: Reading) clock \"plant-ms\" on origin \
         {{ eventually[0,5] holds(r.level = 2) }}\n"
    )
}

fn clocks() -> Value {
    json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}})
}

fn sample(level: i64) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}}})
}

fn timestamped() -> Value {
    json!({"timestamped": {
        "observations": [{"position": sample(0), "stamp": 0}, {"position": sample(2), "stamp": 4}],
        "progress": {"watermark": 7, "completeness": {"complete": {"through": 7}},
            "scope": "closed", "execution": "closed"},
        "lower_boundary": "authoritative-origin",
        "binding": {"profile": STAMPED, "clock": "plant-ms",
            "parameters": clocks()["plant-ms"].clone()},
    }})
}

/// Run a request whose temporal selection is `temporal`.
fn run(directory: &Path, temporal: Value) -> Output {
    std::fs::write(directory.join("seen.native"), unit()).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "seen.native", "authority": "agent-ix",
            "identity": "seen", "document": "Seen", "formal_revision": 1}},
        "clocks": clocks(),
        "clause": {"selection": {"temporal": temporal}, "observations": {}}}});
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    run_file(&path)
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn expect_request_refusal(output: &Output, member: &str) {
    assert_eq!(output.status.code(), Some(20), "{member}: {}", text(output));
    assert!(output.stdout.is_empty(), "{member}: no result document");
    let refusal = text(output);
    for needle in ["invalid-request", "\"request\"", member] {
        assert!(refusal.contains(needle), "{member} {needle}: {refusal}");
    }
}

/// FR-985-AC-2: malformed correction members and an unknown member refuse at
/// stage `request` naming the member.
#[trace("TC-2781", "FR-985-AC-2")]
#[test]
fn malformed_correction_members_refuse_at_the_request() {
    let directory = tempfile::tempdir().unwrap();
    for (member, value) in [
        ("trigger", json!("t")),
        ("relation", json!("superseded")),
        ("relation", json!({"superseding": 1})),
        ("relation", json!({"superseding": "x", "invalidating": "y"})),
        ("evidence_ledger", Value::Null),
        ("admit_late", json!("o1")),
    ] {
        let mut temporal = json!({"name": "Seen", "trace": timestamped()});
        temporal[member] = value;
        expect_request_refusal(&run(directory.path(), temporal), member);
    }
}

/// FR-985-AC-1 (absent relation): a timestamped selection with no relation
/// reads as `original` and is accepted.
#[trace("TC-2781", "FR-985-AC-1")]
#[test]
fn an_absent_relation_reads_as_original() {
    let directory = tempfile::tempdir().unwrap();
    let absent = run(
        directory.path(),
        json!({"name": "Seen", "trace": timestamped()}),
    );
    let original = run(
        directory.path(),
        json!({"name": "Seen", "trace": timestamped(), "relation": "original"}),
    );
    assert_eq!(absent.status.code(), Some(0), "{}", text(&absent));
    assert_eq!(absent.stdout, original.stdout);
}

/// FR-985-AC-3: the correction members are refused off a timestamped trace.
#[trace("TC-2782", "FR-985-AC-3")]
#[test]
fn correction_members_are_refused_off_a_timestamped_trace() {
    let directory = tempfile::tempdir().unwrap();
    for (member, value) in [
        ("relation", json!("original")),
        ("evidence_ledger", json!("evidence.json")),
        ("admit_late", json!([])),
    ] {
        let mut temporal = json!({"name": "Seen", "trace": {"finite": [sample(0), sample(2)]}});
        temporal[member] = value;
        expect_request_refusal(&run(directory.path(), temporal), member);
    }
}

/// FR-985-AC-4: a correction without a ledger refuses at `evaluate`.
#[trace("TC-2782", "FR-985-AC-4")]
#[test]
fn a_correction_without_a_ledger_refuses_at_evaluate() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(
        directory.path(),
        json!({"name": "Seen", "trace": timestamped(), "relation": {"superseding": "r"}}),
    );
    assert!(output.stdout.is_empty(), "no result document");
    let refusal = text(&output);
    for needle in [
        "invalid_runtime_input",
        "\"evaluate\"",
        "\"relation\"",
        "no-ledger",
    ] {
        assert!(refusal.contains(needle), "{needle}: {refusal}");
    }
}
