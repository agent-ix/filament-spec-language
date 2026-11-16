// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clock reach through `command::run`: an open prefix and an incomplete input
//! of a progressed trace (FR-641-AC-9), and a window aggregate over a
//! timestamped trace (FR-194-AC-13).

use crate::command_harness::run_file;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const SAMPLED: &str = "filament.temporal.fixed-sample.false-extension/v1";
const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

fn program() -> Value {
    json!({"source": {"file": "unit.native", "authority": "agent-ix",
        "identity": "test:clock-reach", "document": "ClockReach", "formal_revision": 1}})
}

/// Run clause `name` of `unit` over `trace` with `clocks`, returning the exit
/// code and the result document.
fn run(directory: &Path, unit: &str, clocks: Value, name: &str, trace: Value) -> (i32, Value) {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [], "program": program(), "clocks": clocks,
        "clause": {"selection": {"temporal": {"name": name, "trace": trace}},
            "observations": {}}}});
    let path = directory.join(format!("{name}.json"));
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let output = run_file(&path);
    let document = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "a result document, not {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap_or(-1), document)
}

const SAMPLED_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile f = \"filament.temporal.fixed-sample.false-extension/v1\";\n\
     record Reading { level: Int[0, 9]; }\n\
     temporal S using f over (r: Reading) clock \"sample\" on origin { eventually[0,3] holds(r.level = 2) }\n";

fn sample_clock() -> Value {
    json!({"kind": "fixed_sample", "epoch": {"kind": "integer", "decimal": "0"},
        "period": {"kind": "integer", "decimal": "1"}, "unit": "s"})
}

fn progressed(watermark: u64, completeness: Value, closure: &str) -> Value {
    json!({"progressed": {
        "positions": [{"sample": {"level": {"integer": "0"}}}],
        "progress": {"watermark": watermark, "completeness": completeness,
            "scope": closure, "execution": closure},
        "lower_boundary": "authoritative-origin",
        "binding": {"profile": SAMPLED, "clock": "sample", "parameters": sample_clock()}}})
}

#[test]
#[trace("TC-2384", "FR-641-AC-9")]
#[ignore = "QSL-2149: an incomplete progressed input is not reported as its own indeterminate result yet"]
fn an_open_prefix_and_an_incomplete_input_differ() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"sample": sample_clock()});
    let (exit, open) = run(
        directory.path(),
        SAMPLED_UNIT,
        clocks.clone(),
        "S",
        progressed(0, json!({"complete": {"through": 0}}), "open"),
    );
    assert_eq!(exit, 0, "{open}");
    assert_eq!(open["disposition"]["temporal"]["kind"], "pending-window");
    assert_eq!(open["disposition"]["temporal"]["deadline"], 3);
    let (exit, incomplete) = run(
        directory.path(),
        SAMPLED_UNIT,
        clocks,
        "S",
        progressed(5, json!("incomplete"), "closed"),
    );
    assert_eq!(exit, 11, "{incomplete}");
    let temporal = &incomplete["disposition"]["temporal"];
    assert_eq!(temporal["kind"], "indeterminate", "{incomplete}");
    assert!(
        temporal.to_string().contains("incomplete-input"),
        "{incomplete}"
    );
}

fn quantile_unit(quantile: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"{STAMPED}\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         record Resp {{ latency: Quantity<ms>; }}\n\
         temporal P using w over (r: Resp) clock \"plant-ms\" on origin \
         {{ always[0 ms, 300 s] (quantile {quantile} of r.latency over past[0, 300 s] <= 5 ms) }}\n"
    )
}

fn responses() -> Value {
    let latencies = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 6, 7];
    let observations: Vec<Value> = latencies
        .iter()
        .enumerate()
        .map(|(stamp, latency)| {
            json!({"position": {"sample": {"latency": {"kind": "quantity", "unit": "ms",
                "magnitude": {"kind": "rational", "numerator": latency.to_string(),
                    "denominator": "1"}}}}, "stamp": stamp})
        })
        .collect();
    json!({"timestamped": {
        "observations": observations,
        "progress": {"watermark": 300_000, "completeness": {"complete": {"through": 300_000}},
            "scope": "closed", "execution": "closed"},
        "lower_boundary": "authoritative-origin",
        "binding": {"profile": STAMPED, "clock": "plant-ms",
            "parameters": {"kind": "timestamped_event", "timestamp_unit": "ms"}}}})
}

#[test]
#[trace("TC-2387", "FR-194-AC-13")]
#[ignore = "QSL-2138: window aggregates over a trace subject do not reach a verdict through run yet"]
fn a_quantile_aggregate_reaches_a_verdict_through_run() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let (exit, p95) = run(
        directory.path(),
        &quantile_unit("0.95"),
        clocks.clone(),
        "P",
        responses(),
    );
    assert_eq!(exit, 10, "{p95}");
    assert_eq!(p95["disposition"]["temporal"]["kind"], "violated");
    let (exit, p90) = run(
        directory.path(),
        &quantile_unit("0.9"),
        clocks,
        "P",
        responses(),
    );
    assert_eq!(exit, 0, "{p90}");
    assert_eq!(p90["disposition"]["temporal"]["kind"], "holds");
    let catalog = filament_semantic_value::catalog::CatalogCode::new(
        "unsupported_construct",
        "expression-form",
    );
    assert_eq!(
        fsl_foundation::diagnostic::category_of(&catalog),
        Some(filament_semantic_value::catalog::Category::Unsupported)
    );
}
