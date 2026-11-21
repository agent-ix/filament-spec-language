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
    aggregate_unit(&format!(
        "quantile {quantile} of r.latency over past[0 ms, 300 s] <= 5 ms"
    ))
}

/// A timestamped-event unit whose clause `P` holds the window-aggregate atom
/// `atom` at every tick of `[0, 300000]` ms.
fn aggregate_unit(atom: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile w = \"{STAMPED}\";\n\
         dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         record Resp {{ latency: Quantity<ms>; }}\n\
         temporal P using w over (r: Resp) clock \"plant-ms\" on origin \
         {{ always[0, 300000] ({atom}) }}\n"
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
    // The exit ladder (FR-285), not the refusal-record map `category_of`.
    let category = model_subject_aggregate_refusal().code().category();
    assert_eq!(
        category,
        filament_semantic_value::catalog::Category::Unsupported
    );
    assert_eq!(category.exit_code(), 21);
}

/// FR-194-AC-13, its second sentence: the model-subject aggregate refusal
/// `unsupported_construct`/`expression-form` (FR-194-AC-7) has category
/// `unsupported`, so `fsl run` exits 21, never 20 (FR-330-AC-8).
#[test]
#[trace("TC-2387", "FR-194-AC-13")]
fn the_model_subject_aggregate_refusal_is_unsupported() {
    // The exit ladder (FR-285), not the refusal-record map `category_of`.
    let category = model_subject_aggregate_refusal().code().category();
    assert_eq!(
        category,
        filament_semantic_value::catalog::Category::Unsupported
    );
    assert_eq!(category.exit_code(), 21);
}

/// The model-subject aggregate refusal (FR-194-AC-7) as the checker raises
/// it: `unsupported_construct`/`expression-form`, read from the refusal.
fn model_subject_aggregate_refusal() -> fsl_semantics::check::TemporalCause {
    let cause = fsl_semantics::check::TemporalCause::AggregateModelSubject {
        span: fsl_foundation::Span { start: 0, end: 1 },
    };
    assert_eq!(
        (cause.code().to_string(), cause.tag()),
        ("unsupported_construct".to_owned(), "expression-form")
    );
    cause
}

/// FR-194-AC-1 over source: each window aggregate kind, parsed from
/// source under the timestamped-event profile, runs through `command::run`
/// over AC-1's 20 latencies to the verdict its window values give. At the
/// last tick the window holds 20 responses: 18 of 1 ms, then 6 ms and 7 ms.
#[test]
#[trace("TC-629", "FR-194-AC-1")]
fn every_window_aggregate_kind_reaches_a_verdict_from_source() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    for (atom, exit, verdict) in [
        ("count over past[0 ms, 300 s] <= 20", 0, "holds"),
        ("count over past[0 ms, 300 s] <= 19", 10, "violated"),
        (
            "sum of r.latency over past[0 ms, 300 s] <= 31 ms",
            0,
            "holds",
        ),
        (
            "sum of r.latency over past[0 ms, 300 s] <= 30 ms",
            10,
            "violated",
        ),
        (
            "min of r.latency over past[0 ms, 300 s] >= 1 ms",
            0,
            "holds",
        ),
        (
            "min of r.latency over past[0 ms, 300 s] >= 2 ms",
            10,
            "violated",
        ),
        (
            "max of r.latency over past[0 ms, 300 s] <= 7 ms",
            0,
            "holds",
        ),
        (
            "max of r.latency over past[0 ms, 300 s] <= 6 ms",
            10,
            "violated",
        ),
        (
            "fraction holds(r.latency <= 1 ms) over past[0 ms, 300 s] >= 9/10",
            0,
            "holds",
        ),
        (
            "fraction holds(r.latency <= 1 ms) over past[0 ms, 300 s] >= 19/20",
            10,
            "violated",
        ),
        (
            "fraction holds(r.latency <= 1 ms) over past[0 ms, 300 s] >= rational(9, 10)",
            0,
            "holds",
        ),
        (
            "quantile 0.9 of r.latency over past[0 ms, 300 s] <= 5 ms",
            0,
            "holds",
        ),
        (
            "quantile 0.95 of r.latency over past[0 ms, 300 s] <= 5 ms",
            10,
            "violated",
        ),
        (
            "quantile 0.9 of r.latency over past[0 ms, 300 s] <= 0.005 s",
            0,
            "holds",
        ),
        (
            "quantile 0.95 of r.latency over past[0 ms, 300 s] <= 0.005 s",
            10,
            "violated",
        ),
    ] {
        let (code, document) = run(
            directory.path(),
            &aggregate_unit(atom),
            clocks.clone(),
            "P",
            responses(),
        );
        assert_eq!(code, exit, "{atom}: {document}");
        assert_eq!(
            document["disposition"]["temporal"]["kind"], verdict,
            "{atom}: {document}"
        );
    }
}

/// FR-501 "Verdicts", FR-194: an aggregate is an atom, so its valuation absent
/// from the admitted observation of an eligible position is
/// `MissingValuation` (both possibilities) as an absent `holds` valuation is:
/// the request admits and the verdict is `indeterminate` (exit 11) with reason
/// `missing-valuation` at the absent position 19. The clause's formula is the
/// aggregate atom (node 1) under `always` (node 0), so node 0 and any node
/// past the formula name no atom: a typed `invalid-value` refusal (exit 20).
#[test]
#[trace("TC-2387", "FR-501-AC-4", "FR-194-AC-1")]
fn an_absent_aggregate_valuation_is_a_missing_valuation() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let unit = aggregate_unit("count over past[0 ms, 300 s] <= 20");
    let (exit, document) = run(directory.path(), &unit, clocks.clone(), "P", responses());
    assert_eq!(exit, 0, "{document}");
    let with_missing = |atom: u64| {
        let mut trace = responses();
        trace["timestamped"]["missing"] = json!([{"position": 19, "atom": atom}]);
        trace
    };
    let (exit, document) = run(
        directory.path(),
        &unit,
        clocks.clone(),
        "P",
        with_missing(1),
    );
    assert_eq!(exit, 11, "{document}");
    assert_eq!(
        document["disposition"]["temporal"]["kind"], "indeterminate",
        "{document}"
    );
    assert_eq!(
        document["disposition"]["temporal"]["reason"]["kind"], "missing-valuation",
        "{document}"
    );
    assert_eq!(
        document["disposition"]["temporal"]["reason"]["position"], 19,
        "{document}"
    );
    for atom in [0, 2] {
        let (exit, document) = run(
            directory.path(),
            &unit,
            clocks.clone(),
            "P",
            with_missing(atom),
        );
        assert_eq!(exit, 20, "atom {atom}: {document}");
        assert_eq!(
            document["disposition"]["refusal"]["cause"], "invalid-value",
            "atom {atom}: {document}"
        );
    }
}

/// `responses()` with its progress replaced: `watermark`, completeness
/// through `through`, scope and execution open.
fn responses_with_progress(watermark: u64, through: u64) -> Value {
    let mut trace = responses();
    trace["timestamped"]["progress"] = json!({"watermark": watermark,
        "completeness": {"complete": {"through": through}},
        "scope": "open", "execution": "open"});
    trace
}

/// FR-194-AC-16 through `command::run`: a window not asserted complete is
/// never a final violation. AC-13's p95 unit over AC-1's 20 latencies, with
/// watermark 19 and completeness through 9, is `pending-window` with deadline
/// 300000; with completeness through 19 it is `violated`, exit 10.
#[test]
#[trace("TC-629", "FR-194-AC-16")]
fn a_window_not_asserted_complete_is_pending_until_it_is_covered() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let unit = quantile_unit("0.95");
    let (exit, pending) = run(
        directory.path(),
        &unit,
        clocks.clone(),
        "P",
        responses_with_progress(19, 9),
    );
    assert_eq!(exit, 0, "{pending}");
    let temporal = &pending["disposition"]["temporal"];
    assert_eq!(temporal["kind"], "pending-window", "{pending}");
    assert_eq!(temporal["deadline"], 300_000, "{pending}");
    let (exit, covered) = run(
        directory.path(),
        &unit,
        clocks,
        "P",
        responses_with_progress(19, 19),
    );
    assert_eq!(exit, 10, "{covered}");
    assert_eq!(
        covered["disposition"]["temporal"]["kind"], "violated",
        "{covered}"
    );
}

/// FR-194-AC-17 through `command::run`: AC-1's trace with the response
/// stamped 18 ms holding no valuation of the aggregate (the request names it by position and atom node), complete through 300000, is
/// `indeterminate` (exit 11) with reason `missing-valuation`, never
/// `invalid-value` and never a quantile over the other 19 values.
#[test]
#[trace("TC-629", "FR-194-AC-17")]
fn a_window_member_without_a_valuation_is_a_missing_valuation() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let mut trace = responses();
    // The aggregate atom is node 1 (as in `an_absent_aggregate_valuation_is_a_missing_valuation`);
    // the observation at position 18 holds no valuation for it, so every
    // window that reads position 18 is a missing valuation.
    trace["timestamped"]["missing"] = json!([{"position": 18, "atom": 1}]);
    // FR-641: the entry marks an admitted observation's valuation absent; the
    // observation itself still admits, so its sample keeps a `latency`, here
    // 100 ms: read, the `quantile 0.95` atom at 19 would be false and the run
    // `violated`, so only the member rule makes it `indeterminate`; the 0.9 case
    // is `indeterminate` at 18 from the atom-level absence alone.
    trace["timestamped"]["observations"][18]["position"]["sample"]["latency"]["magnitude"]
        ["numerator"] = json!("100");
    for quantile in ["0.95", "0.9"] {
        let (exit, document) = run(
            directory.path(),
            &quantile_unit(quantile),
            clocks.clone(),
            "P",
            trace.clone(),
        );
        assert_eq!(exit, 11, "{quantile}: {document}");
        assert_eq!(
            document["disposition"]["temporal"]["kind"], "indeterminate",
            "{quantile}: {document}"
        );
        assert_eq!(
            document["disposition"]["temporal"]["reason"]["kind"], "missing-valuation",
            "{quantile}: {document}"
        );
        assert_eq!(
            document["disposition"]["temporal"]["reason"]["position"], 18,
            "{quantile}: {document}"
        );
    }
}

/// FR-194-AC-16, `min_count 30`: with completeness through 9 and watermark 19,
/// and again complete through 300000 with watermark 300000, the run reports
/// `insufficient-data` at position 0 with count 1 and `min_count` 30: FR-501
/// "Verdicts" returns an undefined atom in evaluation order before any
/// pending activation, and the atoms at positions 0 to 9 are covered and short
/// of 30 values.
#[test]
#[trace("TC-629", "FR-194-AC-16")]
fn a_covered_window_short_of_min_count_is_undefined_through_run() {
    let directory = tempfile::tempdir().unwrap();
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let unit =
        aggregate_unit("quantile 0.95 of r.latency over past[0 ms, 300 s] min_count 30 <= 5 ms");
    for trace in [responses_with_progress(19, 9), responses()] {
        let (_, document) = run(directory.path(), &unit, clocks.clone(), "P", trace);
        let temporal = &document["disposition"]["temporal"];
        assert_eq!(temporal["kind"], "insufficient-data", "{temporal}");
        assert_eq!(temporal["position"], 0, "{temporal}");
        assert_eq!(temporal["count"], 1, "{temporal}");
        assert_eq!(temporal["min_count"], 30, "{temporal}");
    }
}
