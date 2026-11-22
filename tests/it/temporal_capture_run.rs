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
         function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure {{ xs }}\n\
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
         temporal EachLater using e over (r: Reading) clock \"events\" on each (q: Reading) \
         {{ capture start: Int[0, 9] = q.level; eventually[0,2] holds(r.level > start) }}\n\
         temporal EachLoop using i over (r: Reading) clock \"steps\" on each (q: Reading) \
         {{ capture start: Int[0, 9] = q.level; eventually[0,1] holds(r.level > start) }}\n"
    )
}

/// FR-860-AC-4's clause whose capture counts into `Zero`: the `count` range is
/// an `unproved-range` obligation that S3 refuses under Linked (FR-766), so the
/// clause is kept out of the shared unit.
const REFUSED: &str = "type Zero = Int[0, 0];\n\
     temporal Refused using e over (r: Reading) clock \"events\" on origin \
     { capture z: Zero = count<Zero>(m in ints(sequence[r.level]): m > 0); always[0,1] holds(z = 0) }\n";

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

/// FR-860-AC-4's clauses whose capture divides through the function `k`:
/// Linked checking refuses `k`'s unproved `mod` (FR-495-AC-3), so they are
/// kept out of the shared unit until the fixture is checked under
/// `CheckMode::Kernel`.
const DIVISION: &str = "function k using v(x: Integer): Integer pure { 3 mod x }\n\
     temporal Div using e over (r: Reading) clock \"events\" on origin \
     { capture m: Integer = k(2 - r.level); always[0,1] holds(m >= 0) }\n\
     temporal DivEach using e over (r: Reading) clock \"events\" on each (q: Reading) \
     { capture m: Integer = k(2 - r.level); always[0,1] holds(m >= 0) }\n";

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    write_with(directory, "", name, request)
}

/// [`write`] with `extra` declarations appended to the unit.
fn write_with(directory: &Path, extra: &str, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit() + extra).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn run(directory: &Path, name: &str, trace: Value) -> Output {
    run_with(directory, "", name, trace)
}

/// [`run`] over the unit with `extra` declarations appended.
fn run_with(directory: &Path, extra: &str, name: &str, trace: Value) -> Output {
    let file = format!("{name}-{}.json", trace.to_string().len());
    run_file(&write_with(
        directory,
        extra,
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
fn each_activation_reads_its_own_capture() {
    let directory = tempfile::tempdir().unwrap();
    let each = result(&run(
        directory.path(),
        "Each",
        json!({"finite": moving(&[0, 1, 2])}),
    ));
    assert_eq!(temporal(&each)["kind"], "holds", "{each}");
}

/// FR-860-AC-3, FR-860 "Fixing the captures": an `on each` activation reads
/// its own capture at the later positions its window visits, not the capture
/// another activation fixed there. Over levels 0, 1, 0 the activation at 0
/// (`start` 0) is discharged by level 1 and the one at 1 (`start` 1) finds no
/// greater level, so the violation is at 1; a reading of each position's
/// atom with that position's own capture would report it at 0. The same
/// holds over an infinite-trace lasso.
#[test]
#[trace("TC-2440", "FR-860-AC-3")]
fn each_activation_reads_its_own_capture_at_later_positions() {
    let directory = tempfile::tempdir().unwrap();
    let later = result(&run(
        directory.path(),
        "EachLater",
        json!({"finite": moving(&[0, 1, 0])}),
    ));
    assert_eq!(temporal(&later)["kind"], "violated", "{later}");
    assert_eq!(temporal(&later)["position"], 1, "{later}");
    let rising = result(&run(
        directory.path(),
        "EachLater",
        json!({"finite": moving(&[0, 1, 2, 3])}),
    ));
    assert_eq!(temporal(&rising)["kind"], "violated", "{rising}");
    assert_eq!(temporal(&rising)["position"], 3, "{rising}");
    let looped = result(&run(
        directory.path(),
        "EachLoop",
        json!({"lasso": {"prefix": [], "loop": [sample(0, false), sample(1, false)]}}),
    ));
    assert_eq!(temporal(&looped)["kind"], "violated", "{looped}");
    assert_eq!(temporal(&looped)["position"], 1, "{looped}");
}

/// FR-860-AC-4 (negative): an undefined capture is an `undefined` violation
/// at its activation, never an internal failure.
#[test]
#[trace("TC-2440", "FR-860-AC-4")]
#[ignore = "QSL-2879: the fixture must be checked under CheckMode::Kernel and selected as a package (FR-860-AC-4); a capture fault through `k` is reported as runtime_invariant (exit 30), not at the capture"]
fn an_undefined_capture_is_reported_at_the_capture() {
    let directory = tempfile::tempdir().unwrap();
    let output = run_with(
        directory.path(),
        DIVISION,
        "Div",
        json!({"finite": moving(&[2, 0])}),
    );
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
    let defined = result(&run_with(
        directory.path(),
        DIVISION,
        "Div",
        json!({"finite": moving(&[0, 2])}),
    ));
    assert_eq!(temporal(&defined)["kind"], "holds", "{defined}");
    let each = result(&run_with(
        directory.path(),
        DIVISION,
        "DivEach",
        json!({"finite": moving(&[0, 2])}),
    ));
    assert_eq!(temporal(&each)["kind"], "undefined", "{each}");
    assert_eq!(temporal(&each)["position"], 1, "{each}");
}

/// FR-860-AC-4 (negative): a capture whose `count` range is unproved is
/// refused at check, never at run time and never as an internal failure.
#[test]
#[trace("TC-2440", "FR-860-AC-4")]
fn a_refused_capture_is_reported_at_the_capture() {
    let directory = tempfile::tempdir().unwrap();
    let refused = run_with(
        directory.path(),
        REFUSED,
        "Refused",
        json!({"finite": moving(&[1])}),
    );
    assert_eq!(refused.status.code(), Some(20));
    let document: Value = serde_json::from_slice(&refused.stdout).expect("a result document");
    let refusal = &document["disposition"]["refusal"];
    assert_eq!(document["disposition"]["stage"], "compile", "{document}");
    assert_eq!(refusal["code"], "undefined_expression", "{document}");
    assert_eq!(refusal["cause"], "unproved-range", "{document}");
}

/// FR-860-AC-5: the stepping monitor fixes the capture at record 0, reports
/// the batch run of each prefix, and settles `Fast` at record 3.
#[test]
#[trace("TC-2441", "FR-860-AC-5")]
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

/// FR-860 "Every trace kind and entry" (FR-860-AC-5's monitor over an `on
/// each` clause and an infinite-trace one): after each record the monitor's
/// disposition equals the batch run over the records so far, each
/// activation reading its own capture; `EachLater` ends violated at 1.
#[test]
#[trace("TC-2441", "FR-860-AC-5")]
fn the_monitor_reads_each_activations_own_capture() {
    let directory = tempfile::tempdir().unwrap();
    let origin = monitored(directory.path(), "Origin", &[5, 1, 1, 1]);
    assert_ne!(origin["category"], Value::Null, "{origin}");
    let each = monitored(directory.path(), "EachLater", &[0, 1, 0]);
    assert_eq!(each["temporal"]["kind"], "violated", "{each}");
    assert_eq!(each["temporal"]["position"], 1, "{each}");
}

/// The last disposition of the `native-monitor/1` run of `name` over records
/// of `levels`, each record's disposition equal in category to the batch run
/// over the records so far.
fn monitored(directory: &Path, name: &str, levels: &[i64]) -> Value {
    monitored_with(directory, "", name, levels, false)
}

/// [`monitored`] over the unit with `extra` declarations appended; `strict`
/// also requires each record's `temporal` and `truth` to equal the batch
/// run's (FR-820 Equality), not only its category.
fn monitored_with(
    directory: &Path,
    extra: &str,
    name: &str,
    levels: &[i64],
    strict: bool,
) -> Value {
    let mut records: Vec<String> = levels
        .iter()
        .map(|level| json!({"observation": sample(*level, false)}).to_string())
        .collect();
    records.push(json!({"end": true}).to_string());
    std::fs::write(directory.join("records.jsonl"), records.join("\n")).unwrap();
    let request = json!({"format": "native-monitor/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clauses": [{"name": name}], "trace": {"kind": "finite"},
        "records": "records.jsonl"}});
    let monitored = run_file(&write_with(
        directory,
        extra,
        &format!("monitor-{name}.json"),
        &request,
    ));
    assert_ne!(monitored.status.code(), Some(30));
    let document: Value = serde_json::from_slice(&monitored.stdout).expect("a monitor run");
    let events = document["events"].as_array().expect("events");
    let mut status = Value::Null;
    for record in 0..levels.len() {
        if let Some(event) = events.iter().find(|event| event["record"] == record) {
            status = event["disposition"].clone();
        }
        let batch = result(&run_with(
            directory,
            extra,
            name,
            json!({"finite": moving(&levels[..=record])}),
        ));
        assert_eq!(
            status["category"], batch["disposition"]["category"],
            "{name} at record {record}: {status} vs {batch}"
        );
        if strict {
            for member in ["temporal", "truth"] {
                assert_eq!(
                    status.get(member),
                    batch["disposition"].get(member),
                    "{name} {member} at record {record}: {status} vs {batch}"
                );
            }
        }
    }
    status
}

const GUARDED: &str = "temporal Guarded using i over (r: Reading) clock \"steps\" \
    on each (trigger: Reading) when (trigger.level > 0) { \
    capture beganFast: Boolean = trigger.level > 3; \
    eventually[0,1] holds(r.stopped or not beganFast) }\n";

/// Exercise the same closed lasso through source and the library package route.
fn guarded_routes(level: i64, stopped: bool) -> Vec<Value> {
    let directory = tempfile::tempdir().unwrap();
    let request = run_request(json!({"name": "Guarded", "trace": {
        "lasso": {"prefix": [], "loop": [sample(level, stopped)]}}}));
    let path = write_with(directory.path(), GUARDED, "source.json", &request);
    let source = result(&run_file(&path));
    let bytes = std::fs::read(directory.path().join("unit.native")).unwrap();
    std::fs::remove_file(directory.path().join("unit.native")).unwrap();
    let package = package_run(
        &fsl_replay::spine::ParseRequest {
            source: &fsl_foundation::SourceIdentity::new("agent-ix", "test:temporal-capture"),
            path: "unit.native",
            bytes: &bytes,
        },
        &Default::default(),
        &Default::default(),
        &fsl_replay::spine::TraceInput::Lasso {
            prefix: vec![],
            repeat: vec![fsl_replay::spine::sample_position(
                &sample(level, stopped)["sample"].to_string(),
            )],
        },
        None,
        "Guarded",
    );
    vec![source, package]
}

/// Compile once, then run the checked package with no source provision.
fn package_run(
    unit: &fsl_replay::spine::ParseRequest<'_>,
    packages: &fsl_semantics::model::intake::PackageInput,
    snapshots: &fsl_semantics::model::observation::DocumentProvision,
    trace: &fsl_replay::spine::TraceInput,
    over: Option<&fsl_replay::spine::OverObject>,
    name: &str,
) -> Value {
    use fsl_replay::spine;
    let limits = spine::SpineLimits::default();
    let cancel = filament_exact::Cancel::new();
    let parsed = spine::parse(unit, limits.source, &cancel)
        .unwrap()
        .into_value();
    let models = spine::select(&parsed, packages, limits.model, &cancel)
        .unwrap()
        .into_value();
    let checked = spine::check(
        &parsed,
        "guarded".to_owned(),
        &models,
        &Default::default(),
        limits,
        &cancel,
    )
    .unwrap()
    .into_value();
    let report = spine::run_package_temporal(&spine::PackageTemporalRequest {
        package: checked.package(),
        packages,
        model_limits: limits.model,
        snapshots,
        observation_limits: Default::default(),
        accounting: fsl_semantics::check::SCALAR_LIMITS_UNLIMITED,
        name,
        trace,
        over,
        ledger: None,
        sources: &[],
        authentication: None,
        cancel: &cancel,
    });
    serde_json::from_slice(&spine::write_clause_run_result(&report).unwrap()).unwrap()
}

/// A closed, complete stop-only lasso has no activation and no truth.
#[test]
#[trace("FSpec-FR-093-AC-5", "FR-622-AC-1", "FR-267-AC-28")]
fn a_closed_false_guard_capture_scope_is_inactive_on_both_routes() {
    for document in guarded_routes(0, true) {
        assert_eq!(document["axes"]["activation"], "inactive", "{document}");
        assert_eq!(document["axes"]["execution"], "completed", "{document}");
        assert_eq!(document["axes"]["truth"], Value::Null, "{document}");
        assert!(document["disposition"].get("truth").is_none(), "{document}");
        assert_eq!(temporal(&document)["kind"], "inactive", "{document}");
        let bytes = serde_json::to_vec(&document).unwrap();
        let fsl_replay::spine::NativeRunResult::Clause(record) =
            fsl_replay::spine::read_native_run_result(&bytes).expect("inactive result round-trips")
        else {
            panic!("expected a clause result");
        };
        assert_eq!(
            record.axes().activation,
            fsl_replay::ActivationAxis::Inactive
        );
        assert_eq!(record.truth(), None);
        let mut claimed = document;
        claimed["disposition"]["truth"] = json!(true);
        assert!(
            fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&claimed).unwrap())
                .is_err(),
            "an inactive result cannot carry truth"
        );
    }
}

/// A guarded `on each` clause under a bounded profile over a closed finite
/// trace.
const GUARDED_CLOSED: &str = "temporal GuardedClosed using e over (r: Reading) clock \"events\" \
    on each (trigger: Reading) when (trigger.level > 0) { \
    capture beganFast: Boolean = trigger.level > 3; \
    eventually[0,1] holds(r.stopped or not beganFast) }\n";

/// A closed finite trace whose every guard is false has no activation and no
/// truth; one true guard makes it active again.
#[test]
#[trace("FSpec-FR-093-AC-5", "FR-267-AC-28")]
fn a_closed_finite_trace_with_every_guard_false_is_inactive() {
    let directory = tempfile::tempdir().unwrap();
    let inactive = result(&run_with(
        directory.path(),
        GUARDED_CLOSED,
        "GuardedClosed",
        json!({"finite": moving(&[0, 0, 0])}),
    ));
    assert_eq!(inactive["axes"]["activation"], "inactive", "{inactive}");
    assert_eq!(inactive["axes"]["execution"], "completed", "{inactive}");
    assert_eq!(inactive["axes"]["truth"], Value::Null, "{inactive}");
    assert_eq!(temporal(&inactive)["kind"], "inactive", "{inactive}");
    let active = result(&run_with(
        directory.path(),
        GUARDED_CLOSED,
        "GuardedClosed",
        json!({"finite": moving(&[0, 1, 0])}),
    ));
    assert_eq!(active["axes"]["activation"], "active", "{active}");
    assert_ne!(temporal(&active)["kind"], "inactive", "{active}");
}

/// FR-820 Equality: the monitor's carried report over a closed trace whose
/// every guard is false equals the batch run's for every prefix, and a later
/// true guard makes it active again.
#[test]
#[trace("FSpec-FR-093-AC-5", "FR-267-AC-28")]
fn the_monitor_reports_a_closed_false_guard_scope_inactive_like_the_batch_run() {
    let directory = tempfile::tempdir().unwrap();
    let inactive = monitored_with(
        directory.path(),
        GUARDED_CLOSED,
        "GuardedClosed",
        &[0, 0, 0],
        true,
    );
    assert_eq!(inactive["temporal"]["kind"], "inactive", "{inactive}");
    let active = monitored_with(
        directory.path(),
        GUARDED_CLOSED,
        "GuardedClosed",
        &[0, 1, 0],
        true,
    );
    assert_eq!(active["temporal"]["kind"], "holds", "{active}");
}

/// The actual guard, rather than the captured predicate or body truth,
/// decides whether an obligation exists. Active controls still hold/refute.
#[test]
#[trace("FSpec-FR-093-AC-6", "FR-860-AC-3")]
fn false_guard_deliveries_create_no_capture_obligation_on_both_routes() {
    for document in guarded_routes(0, false) {
        assert_eq!(document["axes"]["activation"], "inactive", "{document}");
        assert_eq!(document["axes"]["truth"], Value::Null, "{document}");
    }
    for (level, stopped, kind, truth) in [
        (1, false, "holds", "true"),
        (5, true, "holds", "true"),
        (5, false, "violated", "false"),
    ] {
        for document in guarded_routes(level, stopped) {
            assert_eq!(document["axes"]["activation"], "active", "{document}");
            assert_eq!(document["axes"]["truth"], truth, "{document}");
            assert_eq!(temporal(&document)["kind"], kind, "{document}");
        }
    }
}

/// The canonical stop-speed-0 example runs unchanged from source and
/// through the library package entry over the same admitted snapshot.
#[test]
#[trace("FSpec-FR-093-AC-5", "FSpec-FR-093-AC-6")]
#[ignore = "QSL-2904: requires the authoritative filament-examples corpus via FSL_EXAMPLES_DIR"]
fn the_canonical_false_guard_capture_example_is_inactive() {
    let root = PathBuf::from(
        std::env::var_os("FSL_EXAMPLES_DIR")
            .expect("set FSL_EXAMPLES_DIR to the authoritative filament-examples checkout"),
    );
    let request = root.join("examples/docs-captures/requests/false-guard-inactive.json");
    let source_run = fsl_command::command::run(&request, &crate::negotiation::Fr290).unwrap();
    assert_eq!(source_run.exit_code, 0);
    let outcome: Value =
        serde_json::from_slice(&source_run.outcome.as_ref().unwrap().to_bytes().unwrap()).unwrap();
    let item = &outcome["items"][0];
    assert_eq!(item["clause"], "StopForEachFastStart", "{outcome}");
    assert_eq!(item["axes"]["activation"], "inactive", "{outcome}");
    assert_eq!(item["axes"]["execution"], "completed", "{outcome}");
    assert_eq!(item["axes"]["truth"], Value::Null, "{outcome}");

    let input: Value = serde_json::from_slice(&std::fs::read(&request).unwrap()).unwrap();
    let base = request.parent().unwrap();
    let read = |file: &Value| std::fs::read(base.join(file.as_str().unwrap())).unwrap();
    let source = &input["request"]["program"]["source"];
    let bytes = read(&source["file"]);
    let model = read(&input["request"]["models"][0]["source"]["file"]);
    let packages = fsl_semantics::model::intake::package_input([model.as_slice()]).unwrap();
    let stop = read(&input["request"]["clause"]["observations"]["stop"]);
    let document: Value = serde_json::from_slice(&stop).unwrap();
    let label = fsl_semantics::model::observation::DocumentRef {
        authority: document["identity"]["authority"]
            .as_str()
            .unwrap()
            .to_owned(),
        identity: document["identity"]["identity"]
            .as_str()
            .unwrap()
            .to_owned(),
    };
    let snapshots = [(label.clone(), stop)].into_iter().collect();
    let over = &input["request"]["clause"]["selection"]["temporal"]["over"]["reference"];
    let package = package_run(
        &fsl_replay::spine::ParseRequest {
            source: &fsl_foundation::SourceIdentity::new(
                source["authority"].as_str().unwrap(),
                source["identity"].as_str().unwrap(),
            ),
            path: source["file"].as_str().unwrap(),
            bytes: &bytes,
        },
        &packages,
        &snapshots,
        &fsl_replay::spine::TraceInput::Lasso {
            prefix: vec![],
            repeat: vec![label],
        },
        Some(&fsl_replay::spine::OverObject {
            population: over["population"].as_str().unwrap().to_owned(),
            key: over["key"].as_str().unwrap().to_owned(),
        }),
        "StopForEachFastStart",
    );
    assert_eq!(package["axes"]["activation"], "inactive", "{package}");
    assert_eq!(package["axes"]["execution"], "completed", "{package}");
    assert_eq!(package["axes"]["truth"], Value::Null, "{package}");
    assert!(package["disposition"].get("truth").is_none(), "{package}");
}

/// FR-860-AC-4, FR-337 "Captures": a possibly-zero `mod` written in a
/// capture's initializer is an `unproved-nonzero` obligation that S3 refuses
/// under Linked (FR-495-AC-3), because a declared clause is always checked
/// Linked; the run refuses at compile, exit 20, never at run time.
#[test]
#[trace("TC-2440", "FR-860-AC-4")]
fn a_possibly_zero_mod_in_a_capture_is_refused_at_check() {
    let directory = tempfile::tempdir().unwrap();
    let inline = "temporal DivInline using e over (r: Reading) clock \"events\" on origin \
         { capture m: Integer = 3 mod (2 - r.level); always[0,1] holds(m >= 0) }\n";
    let refused = run_with(
        directory.path(),
        inline,
        "DivInline",
        json!({"finite": moving(&[0, 2])}),
    );
    assert_eq!(refused.status.code(), Some(20));
    let document: Value = serde_json::from_slice(&refused.stdout).expect("a result document");
    let refusal = &document["disposition"]["refusal"];
    assert_eq!(document["disposition"]["stage"], "compile", "{document}");
    assert_eq!(refusal["code"], "undefined_expression", "{document}");
    assert_eq!(refusal["cause"], "unproved-nonzero", "{document}");
}
