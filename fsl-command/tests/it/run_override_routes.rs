// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 Behavior 15 on the monitor batch run route: a caller override is
//! applied or the request is refused `invalid-request` naming it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use fsl_command::command::{self, RequestOverrides, RunError, RunResult};
use fsl_foundation::{AccountingSetting, Setting};
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use crate::negotiation::Fr290;

/// A finite monitor program holding `1 + 1 + 1`, which is five expression
/// nodes.
const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile t = \"filament.temporal.infinite-trace/v1\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    record Load { bins: Int[0, 15]; }\n\
    function f using v(): Integer pure { 1 + 1 + 1 }\n\
    temporal StaysLow using t over (s: Load) clock \"steps\" on origin {\n\
        always holds(s.bins <= 6)\n\
    }\n";

/// A directory holding the program, a record stream that only ends, and a
/// batch `native-monitor/1` request over them with `members` added.
fn monitor_request(members: Value) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("load.fsl"), SOURCE).unwrap();
    std::fs::write(directory.path().join("records.jsonl"), "{\"end\":true}\n").unwrap();
    let mut request = json!({"format": "native-monitor/1", "request": {
        "models": [], "program": {"source": {"file": "load.fsl", "authority": "monitor-contracts",
            "identity": "load", "document": "LoadMonitor", "formal_revision": 1}},
        "clauses": [{"name": "StaysLow"}], "trace": {"kind": "finite"},
        "records": "records.jsonl"}});
    for (name, member) in members.as_object().unwrap() {
        request["request"][name] = member.clone();
    }
    let path = directory.path().join("monitor.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    (directory, path)
}

fn run(
    path: &std::path::Path,
    overrides: &RequestOverrides<'_>,
) -> Result<RunResult, Box<RunError>> {
    command::run_with_overrides(path, overrides, &Fr290)
}

fn refusal(path: &std::path::Path, overrides: &RequestOverrides<'_>) -> Value {
    run(path, overrides)
        .expect_err("the run refuses")
        .value()
        .expect("a refusal envelope")
}

/// FR-255-AC-32: a monitor request whose source holds five expression nodes,
/// run with the caller's `s3.nodes` at 4, returns `stage_limit_exceeded` with
/// bound 4, count 5 and setting `s3.nodes`.
#[test]
#[trace("FR-255-AC-32")]
fn a_monitor_run_applies_the_callers_stage_limit() {
    let (_directory, path) = monitor_request(json!({}));
    let control = run(&path, &RequestOverrides::default()).expect("the control run completes");
    assert_eq!(control.exit_code, 0);

    let limits = std::collections::BTreeMap::from([(Setting::S3Nodes, 4)]);
    let envelope = refusal(
        &path,
        &RequestOverrides {
            limits: Some(&limits),
            ..Default::default()
        },
    );
    assert_eq!(envelope["code"], "stage_limit_exceeded", "{envelope}");
    let text = envelope.to_string();
    assert!(
        text.contains("S3 node limit 4 reached (5)") && text.contains("--limit s3.nodes=<n>"),
        "{envelope}"
    );
}

/// FR-255 Behavior 15: the caller's clocks reach the monitor's compile, a
/// request that also carries `clocks` is refused, and a clock the program does
/// not declare is refused by name as the request member would be.
#[test]
#[trace("FR-640-AC-14")]
fn a_monitor_run_applies_the_callers_clocks() {
    let spare = json!({"spare": {"kind": "event_position", "sequence_authority": "spare"}});
    let supplied = RawValue::from_string(spare.to_string()).unwrap();
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..Default::default()
    };
    let (_directory, bare) = monitor_request(json!({}));
    let envelope = refusal(&bare, &overrides);
    assert_eq!(envelope["cause"], "unused-clock-selection", "{envelope}");

    let (_directory, doubled) = monitor_request(json!({"clocks": {}}));
    let envelope = refusal(&doubled, &overrides);
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert!(
        envelope["message"].as_str().unwrap().contains("--clocks"),
        "{envelope}"
    );
}

/// FR-255 Behavior 15: a monitor run refuses an output override and a stage
/// setting the operation does not expose, naming each.
#[test]
fn a_monitor_run_refuses_an_override_it_cannot_apply() {
    let (directory, path) = monitor_request(json!({}));
    let output = directory.path().join("out.json");
    let limits = std::collections::BTreeMap::from([(Setting::ProgressCoveredPositions, 3)]);
    for (overrides, member) in [
        (
            RequestOverrides {
                output: Some(&output),
                ..Default::default()
            },
            "output",
        ),
        (
            RequestOverrides {
                limits: Some(&limits),
                ..Default::default()
            },
            "progress.covered_positions",
        ),
    ] {
        let envelope = refusal(&path, &overrides);
        assert_eq!(envelope["code"], "invalid-request", "{envelope}");
        assert!(
            envelope["message"].as_str().unwrap().contains(member),
            "{envelope}"
        );
    }
    assert!(!output.exists());
}

/// FR-255-AC-39: a monitor run given `accounting.work_units=0` over a stream
/// that evaluates one record settles `incomplete` naming the counter, its
/// bound and its setting.
#[test]
#[trace("FR-255-AC-39")]
fn a_monitor_run_applies_the_callers_accounting_counter() {
    let (directory, path) = monitor_request(json!({}));
    std::fs::write(
        directory.path().join("records.jsonl"),
        "{\"observation\":{\"sample\":{\"bins\":{\"integer\":\"2\"}}}}\n{\"end\":true}\n",
    )
    .unwrap();
    let accounting = BTreeMap::from([(AccountingSetting::WorkUnits, 0)]);
    let result = run(
        &path,
        &RequestOverrides {
            accounting: Some(&accounting),
            ..Default::default()
        },
    )
    .expect("an exhausted budget settles, it does not refuse");
    let document = result.value.to_string();
    assert_eq!(result.exit_code, 22, "{document}");
    for expected in [
        r#""limit":"work_units""#,
        "accounting.work_units",
        r#""setting""#,
    ] {
        assert!(document.contains(expected), "{expected}: {document}");
    }
}
