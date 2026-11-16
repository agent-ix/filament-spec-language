// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330-AC-18 (TC-2295): a record-sample counterexample names each
//! position by its index in the request trace. FR-641-AC-8 (TC-2299): a
//! clause run over a `model` trace writes the limits it ran under.

use crate::command_harness::run_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const EVENTS: &str = "filament.temporal.event-position.false-extension/v1";
const INFINITE: &str = "filament.temporal.infinite-trace/v1";

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile e = \"{EVENTS}\";\n\
         profile i = \"{INFINITE}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal E using e over (r: Reading) clock \"events\" on origin {{ always[0,2] holds(r.level < 2) }}\n\
         temporal Never using i over (r: Reading) clock \"steps\" on origin {{ always holds(r.level < 2) }}\n"
    )
}

fn clocks() -> Value {
    json!({"events": {"kind": "event_position", "sequence_authority": "events"}})
}

fn sample(level: i64) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}}})
}

fn samples(levels: &[i64]) -> Vec<Value> {
    levels.iter().copied().map(sample).collect()
}

fn run_request(temporal: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:record-sample", "document": "RecordSample", "formal_revision": 1}},
        "clocks": clocks(),
        "clause": {"selection": {"temporal": temporal}, "observations": {}}}})
}

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn result(output: &crate::command_harness::Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "expected a result document, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a result document")
}

/// The `counterexample` of a `violated` temporal disposition.
fn counterexample(document: &Value) -> &Value {
    let temporal = &document["disposition"]["temporal"];
    assert_eq!(temporal["kind"], "violated", "{document}");
    &temporal["counterexample"]
}

/// FR-330-AC-18: a lasso of record samples, and a finite sample trace,
/// each refute with one `{"sample": I}` step per position; two equal
/// samples at different positions get different steps.
#[test]
#[trace("TC-2295", "FR-330-AC-18", "FSpec-TC-1700", "FSpec-FR-364-AC-16")]
#[ignore = "QSL-2133: record-sample counterexample steps are not written by sample index yet"]
fn a_record_sample_counterexample_names_positions_by_index() {
    let directory = tempfile::tempdir().unwrap();
    let lasso = result(&run_file(&write(
        directory.path(),
        "lasso.json",
        &run_request(json!({"name": "Never", "trace": {"lasso": {
            "prefix": samples(&[0, 0]), "loop": samples(&[2])}}})),
    )));
    let steps = counterexample(&lasso);
    assert_eq!(
        steps["prefix"],
        json!([{"sample": "0"}, {"sample": "1"}]),
        "{lasso}"
    );
    assert_eq!(steps["loop"], json!([{"sample": "2"}]), "{lasso}");

    let finite = result(&run_file(&write(
        directory.path(),
        "finite.json",
        &run_request(json!({"name": "E", "trace": {"finite": samples(&[0, 2, 0])}})),
    )));
    let prefix = counterexample(&finite)["prefix"]
        .as_array()
        .expect("a prefix")
        .clone();
    assert!(!prefix.is_empty(), "{finite}");
    for (index, step) in prefix.iter().enumerate() {
        assert_eq!(step, &json!({"sample": index.to_string()}), "{finite}");
    }
}

/// FR-641-AC-8: a `model` trace's effective candidate bound, default
/// included, is the result's `limits` member; a sample trace writes none.
/// The model half needs a model subject whose trace runs to a result; the
/// coder builds it over the FR-330-AC-16 subject, as FR-641-AC-7's test notes.
#[test]
#[trace("TC-2299", "FR-641-AC-8", "FSpec-TC-1701", "FSpec-FR-352-AC-19")]
#[ignore = "QSL-2303: the run result does not write its limits member yet"]
fn a_model_trace_run_writes_its_limits() {
    let directory = tempfile::tempdir().unwrap();
    let model = |bound: Option<u64>| {
        let mut trace = json!({"initial": "s0", "universes": [], "prefix": [], "loop": []});
        if let Some(bound) = bound {
            trace["max_candidates"] = json!(bound);
        }
        run_request(json!({"name": "Never", "trace": {"model": trace}}))
    };
    let default = result(&run_file(&write(
        directory.path(),
        "default.json",
        &model(None),
    )));
    assert_eq!(
        default["limits"],
        json!({"expansion.max_candidates": "1000000"}),
        "{default}"
    );
    let five = result(&run_file(&write(
        directory.path(),
        "five.json",
        &model(Some(5)),
    )));
    assert_eq!(
        five["limits"],
        json!({"expansion.max_candidates": "5"}),
        "{five}"
    );
    let finite = result(&run_file(&write(
        directory.path(),
        "finite.json",
        &run_request(json!({"name": "E", "trace": {"finite": samples(&[0, 1])}})),
    )));
    assert!(finite.get("limits").is_none(), "{finite}");
}
