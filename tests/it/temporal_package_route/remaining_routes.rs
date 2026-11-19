// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-level clock, streaming and model-analysis journeys.

use super::source_journeys::{rover_root, run};
use filament_exact::Cancel;
use fsl_command::command::{self, MonitorProviders, RunLimits};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn corpus(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus/temporal")
        .join(name)
}

fn source(file: &Path) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": "test:temporal-routes",
        "document": "TemporalRoutes", "formal_revision": 1})
}

fn write(directory: &Path, request: &Value) -> PathBuf {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

/// Turn an authoritative compile request into a derived request in a
/// temporary directory while keeping every supplied file in its own repo.
fn authoritative_request(path: &Path) -> Value {
    let mut request: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let base = path.parent().unwrap();
    let program = &mut request["request"]["program"]["source"]["file"];
    *program = json!(base.join(program.as_str().unwrap()));
    for model in request["request"]["models"].as_array_mut().unwrap() {
        let file = &mut model["source"]["file"];
        *file = json!(base.join(file.as_str().unwrap()));
    }
    request
}

fn clock_request(file: &str, clause: &str, trace: Value) -> Value {
    let path = corpus(&format!("{file}.request.json"));
    let mut request = authoritative_request(&path);
    request["format"] = json!("native-run/1");
    request["request"]["clause"] = json!({
        "selection": {"temporal": {"name": clause, "trace": trace}}, "observations": {}});
    request
}

fn pulse(ok: bool) -> Value {
    json!({"sample": {"ok": {"boolean": ok}}})
}

fn progress(watermark: u64, closure: &str) -> Value {
    json!({"watermark": watermark, "completeness": {"complete": {"through": watermark}},
        "scope": closure, "execution": closure})
}

fn binding(profile: &str, parameters: Value) -> Value {
    json!({"profile": profile, "clock": "sensor", "parameters": parameters})
}

fn assert_result(path: &Path, exit: u8, result: &str) -> (Value, Value) {
    let (actual, document, native) = run(path);
    assert_eq!(actual, exit, "{document}");
    let items = document["items"].as_array().expect("one clause result");
    assert_eq!(items.len(), 1, "{document}");
    assert_eq!(items[0]["result"], result, "{document}");
    (document, native)
}

#[test]
#[trace("FR-194-AC-4", "FR-194-AC-7", "FR-330-AC-8")]
fn model_subject_window_aggregates_refuse_at_the_aggregate() {
    let directory = tempfile::tempdir().unwrap();
    let root = rover_root();
    let model = json!({"format":"semantic-ir/2.0.0", "source": {
        "file": root.join("model/rover.semantic-ir.json"), "authority":"example",
        "identity":"example:rover/model", "document":"model", "formal_revision":1}});
    let request = |file: &str| {
        json!({"format":"native-compile/1", "request": {
        "models":[model.clone()], "program":{"source":source(&corpus(file))},
        "clocks":{"events":{"kind":"event_position","sequence_authority":"events"}}}})
    };
    let control = command::compile_outcome(&write(
        directory.path(),
        &request("remaining/model-control.native"),
    ))
    .expect("the model, import, subject and clock are accepted without an aggregate");
    assert!(!control.package_bytes().is_empty());
    for (name, term) in [
        ("count", "count"),
        ("sum", "sum"),
        ("min", "min"),
        ("max", "max"),
        ("fraction", "fraction"),
        ("quantile", "quantile"),
    ] {
        let file = format!("remaining/model-window-{name}.native");
        let text = std::fs::read_to_string(corpus(&file)).unwrap();
        let start = text.find(&format!("({term}")).expect("the aggregate term") + 1;
        let error = match command::compile_outcome(&write(directory.path(), &request(&file))) {
            Ok(_) => {
                panic!("{name}: model-subject aggregates are outside the supported projection")
            }
            Err(error) => error,
        };
        assert_eq!(error.category().exit_code(), 21, "{name}: {error}");
        let value = error.artifact_value().unwrap();
        assert_eq!(value["code"], "unsupported_construct", "{name}: {value}");
        assert_eq!(value["cause"], "expression-form", "{name}: {value}");
        let lifecycle: Value = serde_json::from_slice(
            &error
                .outcome_document(fsl_replay::Operation::Package)
                .to_bytes()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            lifecycle["diagnostics"][0]["locus"]["start"], start,
            "{name}: {lifecycle}"
        );
    }
}

#[test]
#[trace("FR-194", "FR-330")]
fn record_window_count_has_true_false_and_insufficient_history_cases() {
    let directory = tempfile::tempdir().unwrap();
    for (values, exit, expected) in [
        (vec![true, true, true], 0, "tested"),
        (vec![false, true, true], 10, "refuted"),
    ] {
        let mut request = clock_request(
            "event-window",
            "Count",
            json!({"finite": values.into_iter().map(pulse).collect::<Vec<_>>()}),
        );
        request["request"]["program"]["source"] =
            source(&corpus("remaining/record-window-count.native"));
        assert_result(&write(directory.path(), &request), exit, expected);
    }
    let mut request = clock_request(
        "event-window",
        "NeedsHistory",
        json!({"finite":[pulse(true)]}),
    );
    request["request"]["program"]["source"] =
        source(&corpus("remaining/record-window-count.native"));
    let (exit, document, native) = run(&write(directory.path(), &request));
    assert_eq!(exit, 10, "{document}");
    let temporal = &native["disposition"]["temporal"];
    assert_eq!(temporal["kind"], "insufficient-data", "{native}");
    assert_eq!(temporal["count"], 1, "{native}");
    assert_eq!(temporal["min_count"], 2, "{native}");
}

#[test]
#[trace("FR-642-AC-5", "FR-645-AC-4")]
fn timed_model_declines_untimed_snapshots_with_trace_kind_evidence() {
    let root = rover_root();
    let directory = tempfile::tempdir().unwrap();
    for (trace, supplied) in [
        (json!({"finite":["start"]}), "finite"),
        (json!({"lasso":{"prefix":[],"loop":["start"]}}), "lasso"),
    ] {
        let mut request =
            authoritative_request(&root.join("requests/timed-safe-stop-compiles.json"));
        request["format"] = json!("native-run/1");
        request["request"]["clause"] = json!({"selection":{"temporal": {
            "name":"SafeStopWithin200msModelTime", "trace":trace,
            "over":{"reference":{"population":"ix://example/rover/rovers","key":"r1"}}}},
            "observations":{"start":root.join("data/t0-driving.json")}});
        let (exit, document, native) = run(&write(directory.path(), &request));
        assert_eq!(exit, 20, "{supplied}: {document}");
        assert_eq!(native["disposition"]["stage"], "admit", "{native}");
        let refusal = &native["disposition"]["refusal"];
        assert_eq!(refusal["code"], "invalid_runtime_input", "{native}");
        assert_eq!(refusal["cause"], "invalid-value", "{native}");
        assert_eq!(
            refusal["fields"]["reason"], "trace-kind-mismatch",
            "{native}"
        );
        assert_eq!(
            refusal["fields"]["profile"], "filament.temporal.timed/v1",
            "{native}"
        );
        assert_eq!(refusal["fields"]["expected"], json!(["timed"]), "{native}");
        assert_eq!(refusal["fields"]["supplied"], supplied, "{native}");
    }
}

#[test]
#[trace("FR-504", "FR-641")]
fn sampled_replay_settles_silent_positions_and_reports_incomplete_input() {
    let directory = tempfile::tempdir().unwrap();
    let parameters = json!({"kind":"fixed_sample", "epoch":{"kind":"integer","decimal":"10"},
        "period":{"kind":"integer","decimal":"2"}, "unit":"ms"});
    for (positions, complete, exit, expected) in [
        (
            vec![pulse(true), pulse(true), pulse(true)],
            true,
            0,
            "tested",
        ),
        (
            vec![pulse(true), pulse(false), pulse(true)],
            true,
            10,
            "refuted",
        ),
        (vec![pulse(true)], true, 10, "refuted"),
        (vec![pulse(true)], false, 11, "indeterminate"),
    ] {
        let mut supplied_progress = progress(2, "closed");
        if !complete {
            supplied_progress["completeness"] = json!("incomplete");
        }
        let trace = json!({"progressed":{"positions":positions,"progress":supplied_progress,
            "lower_boundary":"authoritative-origin",
            "binding":binding("filament.temporal.fixed-sample.false-extension/v1", parameters.clone())}});
        let request = clock_request("fixed-window", "SampledPulse", trace);
        let (_, native) = assert_result(&write(directory.path(), &request), exit, expected);
        assert_eq!(
            native["ledger"]["format"], "filament.progress-ledger/1",
            "{native}"
        );
    }
}

#[test]
#[trace("FR-501", "FR-641")]
fn timestamped_replay_distinguishes_inclusive_deadlines_and_open_windows() {
    let directory = tempfile::tempdir().unwrap();
    for (stamp, ready, watermark, closure, exit, expected) in [
        (7, true, 7, "closed", 0, "tested"),
        (8, true, 8, "closed", 10, "refuted"),
        (6, false, 6, "open", 0, "pending"),
        (7, false, 7, "closed", 10, "refuted"),
    ] {
        let trace = json!({"timestamped":{
            "observations":[{"position":pulse(false),"stamp":0},{"position":pulse(ready),"stamp":stamp}],
            "progress":progress(watermark,closure),"lower_boundary":"authoritative-origin",
            "binding":binding("filament.temporal.timestamped-event.finite-window/v1",
                json!({"kind":"timestamped_event","timestamp_unit":"ms"}))}});
        let request = clock_request("timestamped-window", "StampedPulse", trace);
        let (_, native) = assert_result(&write(directory.path(), &request), exit, expected);
        assert_eq!(
            native["ledger"]["format"], "filament.progress-ledger/1",
            "{native}"
        );
    }
}

#[test]
#[trace("FR-644")]
fn public_monitor_streams_settle_the_three_clock_families() {
    let root = rover_root();
    for (request_name, records_name, exit, category) in [
        (
            "monitor-event-position",
            "monitor-event-position",
            0,
            "success",
        ),
        (
            "monitor-event-position",
            "monitor-event-position-late",
            10,
            "violation",
        ),
        ("monitor-fixed-sample", "monitor-fixed-sample", 0, "success"),
        (
            "monitor-fixed-sample",
            "monitor-fixed-sample-late",
            10,
            "violation",
        ),
        ("monitor-timestamped", "monitor-timestamped", 0, "success"),
        (
            "monitor-timestamped",
            "monitor-timestamped-late",
            10,
            "violation",
        ),
    ] {
        let bytes = std::fs::read(root.join(format!("records/{records_name}.jsonl"))).unwrap();
        let records = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| Ok(line.to_vec()));
        let mut events = Vec::new();
        let summary = command::monitor(
            &root.join(format!("requests/{request_name}.json")),
            records,
            &mut |event| events.push(event.as_value().clone()),
            MonitorProviders {
                negotiate: &crate::negotiation::Fr290,
                providers: Vec::new(),
                options: &fsl_analyze::provider::ProviderOptions::default(),
            },
            RunLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|error| panic!("{records_name}: {error}"));
        assert_eq!(
            summary.exit_code,
            exit,
            "{records_name}: {}",
            summary.as_value()
        );
        assert_eq!(
            summary.as_value()["clauses"][0]["category"],
            category,
            "{}",
            summary.as_value()
        );
        assert!(
            events.iter().any(|event| event["category"] == category),
            "{records_name}: {events:?}"
        );
    }
}

#[test]
#[trace("FR-644-AC-4")]
fn public_monitor_refuses_a_missing_clause_before_consuming_the_stream() {
    let root = rover_root();
    let directory = tempfile::tempdir().unwrap();
    let mut request = authoritative_request(&root.join("requests/monitor-timestamped.json"));
    request["request"]["clauses"] = json!([{"name":"AbsentTemporalClause"}]);
    let consumed = std::cell::Cell::new(0);
    let records = std::iter::once_with(|| {
        consumed.set(consumed.get() + 1);
        Ok(b"{\"end\":true}".to_vec())
    });
    let error = command::monitor(
        &write(directory.path(), &request),
        records,
        &mut |_| {},
        MonitorProviders {
            negotiate: &crate::negotiation::Fr290,
            providers: Vec::new(),
            options: &fsl_analyze::provider::ProviderOptions::default(),
        },
        RunLimits::default(),
        &Cancel::new(),
    )
    .expect_err("the missing clause is refused before input is read");
    assert_eq!(error.category().exit_code(), 20, "{error}");
    assert!(
        error.to_string().contains("AbsentTemporalClause"),
        "{error}"
    );
    assert_eq!(consumed.get(), 0);
}

#[test]
#[trace("FR-621", "FR-330")]
fn public_analyze_proves_fair_each_service_and_refutes_unfair_service() {
    let examples = rover_root().parent().unwrap().to_path_buf();
    let root = examples.join("lock");
    let directory = tempfile::tempdir().unwrap();
    let mut request = authoritative_request(&root.join("requests/fairness/one-served.json"));
    request["format"] = json!("native-analyze/1");
    request["request"]["documents"] = json!([{"file":root.join("data/lock-free.json")}]);
    request["request"]["items"] = json!([
        {"claim":"ServedWithoutFairness"}, {"claim":"ServedIfAcquireIsStronglyFair"},
        {"claim":"ServedIfEachAcquireIsStronglyFair"}]);
    let output = command::analyze(
        &write(directory.path(), &request),
        &crate::negotiation::Fr290,
        &Cancel::new(),
    )
    .expect("the source analysis request settles its claims");
    assert_eq!(output.exit_code, 10, "{}", output.value);
    let items = output.value.as_value()["items"]
        .as_array()
        .expect("analysis item records");
    assert_eq!(items.len(), 3, "{}", output.value);
    for (index, result) in ["refuted", "refuted", "proved"].into_iter().enumerate() {
        assert_eq!(items[index]["record"]["result"], result, "{}", output.value);
        if result == "refuted" {
            assert!(
                items[index]["counterexample"]["loop"]
                    .as_array()
                    .is_some_and(|cycle| !cycle.is_empty()),
                "{}",
                items[index]
            );
        } else {
            assert_eq!(
                items[index]["certificate"]["accepted"], true,
                "{}",
                items[index]
            );
        }
    }
    request["request"]["items"] = json!([{"claim":"AbsentTemporalClaim"}]);
    let error = command::analyze(
        &write(directory.path(), &request),
        &crate::negotiation::Fr290,
        &Cancel::new(),
    )
    .expect_err("an unknown claim is a request refusal");
    assert_eq!(error.category().exit_code(), 20, "{error}");
    assert!(error.to_string().contains("AbsentTemporalClaim"), "{error}");
}
