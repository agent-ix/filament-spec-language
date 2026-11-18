// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-860 (TC-2440, TC-2441): a temporal clause whose `holds` conditions read
//! a capture runs over a supplied trace to a verdict, each activation keeping
//! the capture value it took at its own position, and a capture fault is
//! reported at the capture, never as an internal fault (exit 30).

use crate::command_harness::{run_file, Output};
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
         type Zero = Int[0, 0];\n\
         function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure {{ xs }}\n\
         function k using v(x: Integer): Integer pure {{ 3 mod x }}\n\
         record Reading {{ level: Int[0, 9]; stopped: Boolean; }}\n\
         temporal Fast using e over (r: Reading) clock \"events\" on origin \
         {{ capture startFast: Boolean = r.level > 3; eventually[0,3] holds(r.stopped or not startFast) }}\n\
         temporal FastNoCapture using e over (r: Reading) clock \"events\" on origin \
         {{ eventually[0,3] holds(r.stopped or not (r.level > 3)) }}\n\
         temporal Origin using i over (r: Reading) clock \"steps\" on origin \
         {{ capture startFast: Boolean = r.level > 3; eventually[0,3] holds(r.stopped or not startFast) }}\n\
         temporal OriginNoCapture using i over (r: Reading) clock \"steps\" on origin \
         {{ eventually[0,3] holds(r.stopped or not (r.level > 3)) }}\n\
         temporal Each using e over (r: Reading) clock \"events\" on each (q: Reading) \
         {{ capture start: Int[0, 9] = q.level; always[0,0] holds(r.level = start) }}\n\
         temporal Div using e over (r: Reading) clock \"events\" on origin \
         {{ capture m: Integer = k(2 - r.level); always[0,1] holds(m >= 0) }}\n\
         temporal DivEach using e over (r: Reading) clock \"events\" on each (q: Reading) \
         {{ capture m: Integer = k(2 - r.level); always[0,1] holds(m >= 0) }}\n\
         temporal Refused using e over (r: Reading) clock \"events\" on origin \
         {{ capture z: Zero = count<Zero>(m in ints(sequence[r.level]): m > 0); always[0,1] holds(z = 0) }}\n"
    )
}

fn clocks() -> Value {
    json!({"events": {"kind": "event_position", "sequence_authority": "events"}})
}

fn program() -> Value {
    json!({"source": {"file": "unit.native", "authority": "agent-ix",
        "identity": "test:temporal-capture", "document": "TemporalCapture", "formal_revision": 1}})
}

fn sample(level: i64, stopped: bool) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}, "stopped": {"boolean": stopped}}})
}

fn moving(levels: &[i64]) -> Vec<Value> {
    levels.iter().map(|level| sample(*level, false)).collect()
}

fn run_request(temporal: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clause": {"selection": {"temporal": temporal}, "observations": {}}}})
}

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn run(directory: &Path, name: &str, trace: Value) -> Output {
    let file = format!("{name}-{}.json", trace.to_string().len());
    run_file(&write(
        directory,
        &file,
        &run_request(json!({"name": name, "trace": trace})),
    ))
}

/// The result document of a run that wrote one; a capture reading is never
/// an internal fault (exit 30).
fn result(output: &Output) -> Value {
    assert_ne!(
        output.status.code(),
        Some(30),
        "a capture run is not an internal failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "expected a result document, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a result document")
}

fn temporal(document: &Value) -> &Value {
    &document["disposition"]["temporal"]
}

/// FR-860-AC-1: `startFast` keeps the value it took at position 0, so `Fast`
/// is violated where the capture-free formula holds.
#[test]
#[trace("TC-2440", "FR-860-AC-1")]
#[ignore = "QSL-2001: a holds condition that reads a capture is not evaluated over a trace yet"]
fn a_capture_keeps_the_value_it_took_at_the_activation() {
    let directory = tempfile::tempdir().unwrap();
    let fast = result(&run(
        directory.path(),
        "Fast",
        json!({"finite": moving(&[5, 1, 1, 1])}),
    ));
    assert_eq!(temporal(&fast)["kind"], "violated", "{fast}");
    assert_eq!(temporal(&fast)["position"], 0, "{fast}");
    let free = result(&run(
        directory.path(),
        "FastNoCapture",
        json!({"finite": moving(&[5, 1, 1, 1])}),
    ));
    assert_eq!(temporal(&free)["kind"], "holds", "{free}");
    let slow = result(&run(
        directory.path(),
        "Fast",
        json!({"finite": moving(&[1, 5, 5, 5])}),
    ));
    assert_eq!(temporal(&slow)["kind"], "holds", "{slow}");
}

/// FR-860-AC-2: the infinite-trace rule over a lasso reads the capture of
/// its one origin activation at every loop repetition.
#[test]
#[trace("TC-2440", "FR-860-AC-2")]
#[ignore = "QSL-2001: a holds condition that reads a capture is not evaluated over a trace yet"]
fn an_infinite_trace_capture_is_fixed_over_a_lasso() {
    let directory = tempfile::tempdir().unwrap();
    let falling = json!({"lasso": {"prefix": [], "loop": [sample(5, false), sample(1, false)]}});
    let captured = result(&run(directory.path(), "Origin", falling.clone()));
    assert_eq!(temporal(&captured)["kind"], "violated", "{captured}");
    assert_eq!(temporal(&captured)["position"], 0, "{captured}");
    let free = result(&run(directory.path(), "OriginNoCapture", falling));
    assert_eq!(temporal(&free)["kind"], "holds", "{free}");
    let stopping = json!({"lasso": {"prefix": [], "loop": [sample(5, false), sample(5, true)]}});
    let stopped = result(&run(directory.path(), "Origin", stopping));
    assert_eq!(temporal(&stopped)["kind"], "holds", "{stopped}");
}

/// FR-860-AC-3: each `on each` activation reads its own capture.
#[test]
#[trace("TC-2440", "FR-860-AC-3")]
#[ignore = "QSL-2001: a holds condition that reads a capture is not evaluated over a trace yet"]
fn each_activation_reads_its_own_capture() {
    let directory = tempfile::tempdir().unwrap();
    let each = result(&run(
        directory.path(),
        "Each",
        json!({"finite": moving(&[0, 1, 2])}),
    ));
    assert_eq!(temporal(&each)["kind"], "holds", "{each}");
}

/// FR-860-AC-4 (negative): an undefined capture is an `undefined` violation
/// at its activation, a refused capture a stage `evaluate` refusal, neither
/// an internal failure.
#[test]
#[trace("TC-2440", "FR-860-AC-4")]
#[ignore = "QSL-2001: the fixture must be checked under CheckMode::Kernel and selected as a package (FR-860-AC-4); a capture fault through `k` is reported as runtime_invariant (exit 30), not at the capture"]
fn a_capture_fault_is_reported_at_the_capture() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), "Div", json!({"finite": moving(&[2, 0])}));
    assert_eq!(output.status.code(), Some(10));
    let undefined = result(&output);
    assert_eq!(temporal(&undefined)["kind"], "undefined", "{undefined}");
    assert_eq!(temporal(&undefined)["position"], 0, "{undefined}");
    assert_eq!(
        undefined["disposition"]["category"], "violation",
        "{undefined}"
    );
    assert!(
        undefined.to_string().contains("division-by-zero"),
        "{undefined}"
    );
    let defined = result(&run(
        directory.path(),
        "Div",
        json!({"finite": moving(&[0, 2])}),
    ));
    assert_eq!(temporal(&defined)["kind"], "holds", "{defined}");
    let each = result(&run(
        directory.path(),
        "DivEach",
        json!({"finite": moving(&[0, 2])}),
    ));
    assert_eq!(temporal(&each)["kind"], "undefined", "{each}");
    assert_eq!(temporal(&each)["position"], 1, "{each}");

    let refused = run(directory.path(), "Refused", json!({"finite": moving(&[1])}));
    assert_eq!(refused.status.code(), Some(20));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(text.contains("integer_out_of_domain"), "{text}");
    assert!(!text.contains("runtime_invariant"), "{text}");
}

/// FR-860-AC-5: the stepping monitor fixes the capture at record 0, reports
/// the batch run of each prefix, and settles `Fast` at record 3.
#[test]
#[trace("TC-2441", "FR-860-AC-5")]
#[ignore = "QSL-2001: a holds condition that reads a capture is not evaluated over a trace yet"]
fn the_monitor_keeps_the_capture_of_its_activation() {
    let directory = tempfile::tempdir().unwrap();
    let levels = [5, 1, 1, 1];
    let mut records: Vec<String> = levels
        .iter()
        .map(|level| json!({"observation": sample(*level, false)}).to_string())
        .collect();
    records.push(json!({"end": true}).to_string());
    std::fs::write(directory.path().join("records.jsonl"), records.join("\n")).unwrap();
    let request = json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clauses": [{"name": "Fast"}], "trace": {"kind": "finite"},
        "records": "records.jsonl"}});
    let monitored = run_file(&write(directory.path(), "monitor.json", &request));
    assert_ne!(monitored.status.code(), Some(30));
    let document: Value = serde_json::from_slice(&monitored.stdout).expect("a monitor run");
    let events = document["events"].as_array().expect("events");
    let mut status = Value::Null;
    for record in 0..levels.len() {
        if let Some(event) = events.iter().find(|event| event["record"] == record) {
            status = event["disposition"].clone();
            assert_eq!(event["settled"], record == 3, "record {record}: {event}");
            if record == 3 {
                assert_eq!(event["settlement"]["step"], "4", "{event}");
                assert_eq!(event["settlement"]["trigger"], "decided", "{event}");
            }
        }
        let batch = result(&run(
            directory.path(),
            "Fast",
            json!({"finite": moving(&levels[..=record])}),
        ));
        assert_eq!(
            status["category"], batch["disposition"]["category"],
            "record {record}"
        );
    }
    assert_eq!(status["temporal"]["kind"], "violated", "{status}");
}
