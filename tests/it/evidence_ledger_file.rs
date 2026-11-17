// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-975 (TC-2750, TC-2751): the `filament.finite-window-evidence-ledger/1`
//! file a run writes and a correction reads, keyed by the whole obligation
//! instance (package, clause, occurrence, trigger), with the superseded
//! identities that tell `predecessor-not-latest` from `predecessor-unknown`.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";
const LEDGER: &str = "filament.finite-window-evidence-ledger/1";

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

/// The program of package `identity`, read from `file`.
fn program(file: &str, identity: &str) -> Value {
    json!({"source": {"file": file, "authority": "agent-ix",
        "identity": identity, "document": "EvidenceLedger", "formal_revision": 1}})
}

/// FR-501-AC-29's observations: stamp 0 (level 0), stamp 4 (level `four`)
/// and stamp 3 (level `three`, arriving late under the cutoff).
fn trace(four: i64, three: i64) -> Value {
    let observed = |stamp: u64, level: i64, arrival: u64| {
        json!({"position": {"sample": {"level": {"integer": level.to_string()}}},
            "stamp": stamp, "arrival": arrival})
    };
    json!({
        "observations": [observed(0, 0, 0), observed(4, four, 6), observed(3, three, 6)],
        "progress": {"watermark": 7, "completeness": {"complete": {"through": 7}},
            "scope": "closed", "execution": "closed"},
        "lower_boundary": "authoritative-origin",
        "binding": {"profile": STAMPED, "clock": "plant-ms",
            "parameters": clocks()["plant-ms"].clone()},
        "late_data_cutoff": {"allowed_lateness": 2},
    })
}

fn request(package: &str, timestamped: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [], "program": program(&format!("{package}.native"), package),
        "clocks": clocks(),
        "clause": {"selection": {"temporal": {"name": "Seen",
            "trace": {"timestamped": timestamped}}}, "observations": {}}}})
}

/// A request correcting `previous` with the ledger file `evidence.json`.
fn correcting(package: &str, timestamped: Value, previous: &Value) -> Value {
    let mut next = request(package, timestamped);
    let temporal = &mut next["request"]["clause"]["selection"]["temporal"];
    temporal["relation"] = json!({"superseding": previous.clone()});
    temporal["evidence_ledger"] = json!("evidence.json");
    next
}

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    for package in ["first", "second"] {
        std::fs::write(directory.join(format!("{package}.native")), unit()).unwrap();
    }
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn keep_ledger(directory: &Path, document: &Value) {
    std::fs::write(
        directory.join("evidence.json"),
        serde_json::to_vec(&document["evidence_ledger"]).unwrap(),
    )
    .unwrap();
}

fn document_of(output: &Output) -> Value {
    serde_json::from_slice(if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    })
    .expect("a document")
}

fn result(output: &Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "a result document, not {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a result document")
}

/// Assert `output` is a relation refusal at `evaluate` with `cause` and no
/// result document.
fn expect_relation_refusal(output: &Output, cause: &str) {
    assert!(output.stdout.is_empty(), "no result document");
    let text = document_of(output).to_string();
    for needle in [
        "invalid_runtime_input",
        "\"evaluate\"",
        "\"relation\"",
        cause,
    ] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
}

/// FR-975-AC-1: the written ledger holds one entry keyed by package, clause,
/// occurrence and `trigger` `null`, with lineage and superseded list; a
/// correction extends the list; reading and writing reproduces the bytes.
#[trace("TC-2750", "FR-975-AC-1")]
#[test]
#[ignore = "QSL-2339: the whole-key filament.finite-window-evidence-ledger/1 form not implemented yet"]
fn the_ledger_file_is_keyed_by_the_whole_obligation_instance() {
    let directory = tempfile::tempdir().unwrap();
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &request("first", trace(0, 2)),
    )));
    let ledger = &first["evidence_ledger"];
    assert_eq!(ledger["format"], LEDGER, "{ledger}");
    let entry = &ledger["results"][0];
    assert_eq!(
        ledger["results"].as_array().map(Vec::len),
        Some(1),
        "{ledger}"
    );
    for member in ["package", "clause", "occurrence"] {
        assert!(!entry["key"][member].is_null(), "{member}: {entry}");
    }
    assert!(entry["key"]["trigger"].is_null(), "{entry}");
    assert_eq!(entry["result_identity"], first["result_identity"]);
    assert_eq!(entry["lineage"], 0);
    assert_eq!(entry["superseded"], json!([]));
    keep_ledger(directory.path(), &first);
    let second = result(&run_file(&write(
        directory.path(),
        "second.json",
        &correcting("first", trace(2, 2), &first["result_identity"]),
    )));
    let entry = &second["evidence_ledger"]["results"][0];
    assert_eq!(entry["key"], first["evidence_ledger"]["results"][0]["key"]);
    assert_eq!(entry["lineage"], 1);
    assert_eq!(
        entry["superseded"],
        json!([first["result_identity"].clone()])
    );
}

/// FR-975-AC-2: a clause of the same name in another package is another
/// obligation, so a correction across packages is `predecessor-unknown`.
#[trace("TC-2750", "FR-975-AC-2")]
#[test]
#[ignore = "QSL-2339: ledger lookup by package, clause, occurrence and trigger not implemented yet (looked up by clause name only)"]
fn a_clause_of_the_same_name_in_another_package_is_another_obligation() {
    let directory = tempfile::tempdir().unwrap();
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &request("first", trace(0, 2)),
    )));
    keep_ledger(directory.path(), &first);
    let mut other = request("second", trace(0, 2));
    other["request"]["clause"]["selection"]["temporal"]["evidence_ledger"] = json!("evidence.json");
    let other = result(&run_file(&write(directory.path(), "other.json", &other)));
    let results = other["evidence_ledger"]["results"]
        .as_array()
        .cloned()
        .unwrap();
    assert_eq!(results.len(), 2, "{other}");
    assert_ne!(results[0]["key"]["package"], results[1]["key"]["package"]);
    keep_ledger(directory.path(), &other);
    let across = run_file(&write(
        directory.path(),
        "across.json",
        &correcting("second", trace(2, 2), &first["result_identity"]),
    ));
    expect_relation_refusal(&across, "predecessor-unknown");
}

/// FR-975-AC-3: a superseded predecessor is `predecessor-not-latest`, an
/// identity the entry does not hold is `predecessor-unknown`, and the latest
/// is accepted at lineage 2.
#[trace("TC-2750", "FR-975-AC-3", "FR-526-AC-8")]
#[test]
#[ignore = "QSL-2339: whole-key ledger form with superseded identities not implemented yet"]
fn a_superseded_predecessor_is_not_latest_and_an_unheld_one_is_unknown() {
    let directory = tempfile::tempdir().unwrap();
    let r1 = result(&run_file(&write(
        directory.path(),
        "r1.json",
        &request("first", trace(0, 2)),
    )));
    keep_ledger(directory.path(), &r1);
    let r2 = result(&run_file(&write(
        directory.path(),
        "r2.json",
        &correcting("first", trace(2, 2), &r1["result_identity"]),
    )));
    keep_ledger(directory.path(), &r2);
    assert_eq!(
        r2["evidence_ledger"]["results"][0]["superseded"],
        json!([r1["result_identity"].clone()])
    );
    let stale = run_file(&write(
        directory.path(),
        "stale.json",
        &correcting("first", trace(1, 2), &r1["result_identity"]),
    ));
    expect_relation_refusal(&stale, "predecessor-not-latest");
    let stranger = run_file(&write(
        directory.path(),
        "stranger.json",
        &correcting("first", trace(1, 2), &json!("never-issued")),
    ));
    expect_relation_refusal(&stranger, "predecessor-unknown");
    let latest = result(&run_file(&write(
        directory.path(),
        "latest.json",
        &correcting("first", trace(1, 2), &r2["result_identity"]),
    )));
    assert_eq!(latest["lineage"], 2, "{latest}");
}

/// FR-975-AC-4: a ledger file that is not a closed, ordered, consistent
/// document refuses `invalid-request` at stage `request` naming
/// `evidence_ledger`; a missing path refuses naming the path.
#[trace("TC-2751", "FR-975-AC-4")]
#[test]
#[ignore = "QSL-2339: the whole-key ledger form and its refusals not implemented yet"]
fn a_malformed_ledger_file_refuses_naming_evidence_ledger() {
    let directory = tempfile::tempdir().unwrap();
    let r1 = result(&run_file(&write(
        directory.path(),
        "r1.json",
        &request("first", trace(0, 2)),
    )));
    let ledger = r1["evidence_ledger"].clone();
    let entry = ledger["results"][0].clone();
    let mut extra = entry.clone();
    extra["extra"] = json!(1);
    let mut no_key = entry.clone();
    no_key.as_object_mut().unwrap().remove("key");
    let mut inconsistent = entry.clone();
    inconsistent["lineage"] = json!(1);
    let mut later = entry.clone();
    later["key"]["package"] = json!("zzz");
    let malformed = [
        json!({"format": LEDGER, "results": [extra]}),
        json!({"format": "filament.finite-window-evidence-ledger/2", "results": [entry.clone()]}),
        json!({"format": LEDGER, "results": [no_key]}),
        json!({"format": LEDGER, "results": [entry.clone(), entry.clone()]}),
        json!({"format": LEDGER, "results": [later, entry.clone()]}),
        json!({"format": LEDGER, "results": [inconsistent]}),
    ];
    for document in malformed {
        std::fs::write(
            directory.path().join("evidence.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        let output = run_file(&write(
            directory.path(),
            "next.json",
            &correcting("first", trace(2, 2), &r1["result_identity"]),
        ));
        assert!(output.stdout.is_empty(), "no result document: {document}");
        let text = document_of(&output).to_string();
        for needle in ["invalid-request", "\"request\"", "evidence_ledger"] {
            assert!(text.contains(needle), "{needle}: {text}");
        }
    }
    std::fs::remove_file(directory.path().join("evidence.json")).unwrap();
    let missing = run_file(&write(
        directory.path(),
        "missing.json",
        &correcting("first", trace(2, 2), &r1["result_identity"]),
    ));
    assert!(missing.stdout.is_empty(), "no result document");
    assert!(
        document_of(&missing).to_string().contains("evidence.json"),
        "the refusal names the path"
    );
}
