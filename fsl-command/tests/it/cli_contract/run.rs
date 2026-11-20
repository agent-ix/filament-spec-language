// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2471: `fsl run <request>` at the command surface the driver calls
//! (`run_with_limits_and_overrides`): a completed call,
//! an indeterminate finite-window clause, refusals, an exhausted budget, the
//! `--clocks`, `--limit`, `--request-bytes` and `--dependent-bytes` inputs,
//! and odd request and source input.

use fsl_command::command::{RequestOverrides, RunLimits};
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use super::{run, Case, Outcome, INVALID_UTF8};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function Remaining using v(total: Integer, held: Integer): Integer pure { total - held }\n\
    function IsPacked using v(n: Int[0, 10]): Boolean pure { n >= 6 }\n";

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

fn request(file: &str, function: &str, arguments: &[(&str, Value)]) -> Value {
    let arguments: Vec<Value> = arguments
        .iter()
        .map(|(parameter, value)| json!({"parameter": parameter, "value": value}))
        .collect();
    json!({"format": "native-run/1", "request": {"models": [], "program": {"source": {
        "file": file, "authority": "run-contracts", "identity": "contracts:packing",
        "document": "Packing", "formal_revision": 1}},
        "call": {"function": function, "arguments": arguments}}})
}

fn packing(function: &str, arguments: &[(&str, Value)]) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    let path = case.json("run.json", &request("packing.fsl", function, arguments));
    (case, path)
}

fn plain(path: &std::path::Path) -> Outcome {
    run(path, RunLimits::default(), &RequestOverrides::default())
}

fn remaining() -> (Case, std::path::PathBuf) {
    packing(
        "Remaining",
        &[
            ("total", json!({"integer": "17"})),
            ("held", json!({"integer": "5"})),
        ],
    )
}

/// FR-100, FR-286-AC-5: a completed call writes one `execute` outcome with
/// `result` `completed` and its value, exit 0.
#[test]
#[trace("FR-100", "FR-286-AC-5", "FR-285-AC-1")]
fn a_completed_call_writes_its_value() {
    let (_case, path) = remaining();
    let outcome = plain(&path);
    let document = outcome.expect("execute", "success", 0);
    assert_eq!(
        document["result"],
        json!({"kind": "completed", "value": {"integer": "12"}}),
        "{document}"
    );
    let (_case, path) = packing("IsPacked", &[("n", json!({"integer": "7"}))]);
    let document = plain(&path);
    assert_eq!(
        document.expect("execute", "success", 0)["result"]["value"],
        json!({"boolean": true})
    );
}

/// A finite-window timestamped clause over stamps 0, 2 and 5 whose
/// `level > 0` atom holds as `levels` says; `missing` leaves the valuation of
/// the atom absent at position 1 (FR-501-AC-4).
fn finite_window(case: &Case, levels: [i64; 3], missing: bool) -> std::path::PathBuf {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal Seen using t over (r: Reading) clock \"plant-ms\" on origin {{\n\
           eventually[0,5] holds(r.level > 0)\n\
         }}\n"
    );
    case.write("program.native", source.as_bytes());
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let observations: Vec<Value> = [0_u64, 2, 5]
        .into_iter()
        .zip(levels)
        .map(|(stamp, level)| {
            json!({"position": {"sample": {"level": {"integer": level.to_string()}}},
                "stamp": stamp})
        })
        .collect();
    let missing: Vec<Value> = if missing {
        vec![json!({"position": 1, "atom": 1})]
    } else {
        vec![]
    };
    let request = json!({"format": "native-run/1", "request": {"models": [],
        "program": {"source": {"file": "program.native", "authority": "test",
            "identity": "test:finite-window", "document": "FiniteWindow", "formal_revision": 1}},
        "clocks": clocks,
        "clause": {"selection": {"temporal": {"name": "Seen", "trace": {"timestamped": {
            "observations": observations,
            "progress": {"watermark": 5, "completeness": {"complete": {"through": 5}},
                "scope": "closed", "execution": "closed"},
            "lower_boundary": "authoritative-origin",
            "binding": {"profile": STAMPED, "clock": "plant-ms",
                "parameters": clocks["plant-ms"]},
            "missing": missing}}}},
            "observations": {}}}});
    case.json("run.json", &request)
}

/// FR-285-AC-7, AC-12, FR-501-AC-4: a finite-window clause whose valuation is
/// absent is `indeterminate`, exit 11; complete and satisfied it succeeds,
/// exit 0; complete and never satisfied it is a violation, exit 10.
#[test]
#[trace("FR-285-AC-7", "FR-285-AC-12", "FR-501-AC-4")]
fn finite_window_clause_reaches_indeterminate_and_violation() {
    let case = Case::new();
    let open = plain(&finite_window(&case, [0, 0, 0], true));
    assert_eq!(open.exit, 11, "{}", open.document);
    assert!(
        open.document.to_string().contains("indeterminate"),
        "{}",
        open.document
    );
    let held = plain(&finite_window(&case, [0, 0, 1], false));
    assert_eq!(held.exit, 0, "{}", held.document);
    let failed = plain(&finite_window(&case, [0, 0, 0], false));
    assert_eq!(failed.exit, 10, "{}", failed.document);
}

/// FR-100-AC-5: an unknown function refuses `missing_declaration` at stage
/// `call`, exit 20, naming the function in `details`.
#[test]
#[trace("FR-100-AC-5")]
fn an_unknown_function_refuses_missing_declaration() {
    let (_case, path) = packing("nope", &[]);
    let diagnostic = plain(&path)
        .refused("execute", 20, "missing_declaration")
        .clone();
    assert_eq!(
        diagnostic["details"],
        json!({"function": "nope"}),
        "{diagnostic}"
    );
}

/// FR-100-AC-4, FR-286-AC-18: an out-of-range argument and a wrong value
/// kind each refuse `invalid_runtime_input` at stage `call`, exit 20.
#[test]
#[trace("FR-100-AC-4", "FR-286-AC-18")]
fn call_defects_refuse_invalid_runtime_input() {
    for (function, arguments) in [
        ("IsPacked", vec![("n", json!({"integer": "11"}))]),
        ("IsPacked", vec![("n", json!({"boolean": true}))]),
    ] {
        let (_case, path) = packing(function, &arguments);
        let outcome = plain(&path);
        outcome.refused("execute", 20, "invalid_runtime_input");
    }
}

/// FR-100-AC-6: a call budget of zero work units ends `incomplete`, exit 22.
#[test]
#[trace("FR-100-AC-6", "FR-285-AC-6")]
fn an_exhausted_call_budget_exits_22() {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    let mut exhausted = request(
        "packing.fsl",
        "Remaining",
        &[
            ("total", json!({"integer": "17"})),
            ("held", json!({"integer": "5"})),
        ],
    );
    exhausted["request"]["call"]["accounting"] = json!({"work_units": 0});
    let outcome = plain(&case.json("run.json", &exhausted));
    outcome.expect("execute", "incomplete", 22);
}

/// `--request-bytes` and `--dependent-bytes` (FR-312): one byte under the
/// request or source size stops at intake with exit 22 naming the ceiling;
/// the exact size runs.
#[test]
#[trace("FR-312", "FR-869-AC-1")]
fn request_and_dependent_byte_ceilings_are_reached_and_raised() {
    let (_case, path) = remaining();
    let request_size = std::fs::metadata(&path).unwrap().len() as usize;
    let over = run(
        &path,
        RunLimits {
            request_bytes: request_size - 1,
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    );
    over.expect("execute", "incomplete", 22);
    assert!(
        over.document.to_string().contains("request_bytes"),
        "{}",
        over.document
    );

    let over = run(
        &path,
        RunLimits {
            dependent_bytes: SOURCE.len() - 1,
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    );
    over.expect("execute", "incomplete", 22);
    assert!(
        over.document.to_string().contains("dependent_bytes"),
        "{}",
        over.document
    );

    let fits = run(
        &path,
        RunLimits {
            request_bytes: request_size,
            dependent_bytes: SOURCE.len(),
        },
        &RequestOverrides::default(),
    );
    fits.expect("execute", "success", 0);
}

/// FR-001-AC-4 and FR-285-AC-5 (`--limit s1.input_bytes=N`): a run whose
/// source is over the S1 byte ceiling refuses `stage_limit_exceeded`
/// naming the setting, exit 22.
#[test]
#[trace("FR-001-AC-4", "FR-285-AC-5")]
fn source_byte_stage_limit_is_reached() {
    let (_case, path) = remaining();
    let mut limits = fsl_replay::spine::SpineLimits::default();
    limits.source.source_bytes = 1;
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome = run(&path, RunLimits::default(), &overrides);
    let diagnostic = outcome.refused("execute", 22, "stage_limit_exceeded");
    assert!(
        diagnostic.to_string().contains("s1.input_bytes"),
        "{diagnostic}"
    );
}

/// FR-640-AC-14 (`--clocks`): a request that carries `clocks` and is also
/// handed the override refuses `invalid-request` at stage `request`, exit 20.
#[test]
#[trace("FR-640-AC-14")]
fn clocks_given_twice_refuse() {
    let case = Case::new();
    let path = finite_window(&case, [0, 0, 1], false);
    let supplied = RawValue::from_string(
        json!({"plant-ms": {"kind": "timestamped_event",
            "timestamp_unit": "ms"}})
        .to_string(),
    )
    .unwrap();
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..RequestOverrides::default()
    };
    let outcome = run(&path, RunLimits::default(), &overrides);
    outcome.refused("execute", 20, "invalid-request");
    assert!(
        outcome.document["last_stage"].is_null(),
        "{}",
        outcome.document
    );
}

/// FR-001-AC-3, AC-8: invalid UTF-8 and NUL sources refuse, exit 20.
#[test]
#[trace("FR-001-AC-3", "FR-001-AC-8")]
fn invalid_utf8_and_nul_sources_refuse() {
    let mut nul = SOURCE.as_bytes().to_vec();
    nul.extend_from_slice(b"\0\n");
    for source in [INVALID_UTF8.to_vec(), nul] {
        let case = Case::new();
        case.write("packing.fsl", &source);
        let path = case.json("run.json", &request("packing.fsl", "Remaining", &[]));
        let outcome = plain(&path);
        outcome.expect("execute", "refusal", 20);
        assert!(
            !outcome.document["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{}",
            outcome.document
        );
    }
}

/// FR-001 file errors and malformed requests: a missing, a directory and an
/// empty request operand, a missing source, malformed JSON and an unknown
/// member each refuse, exit 20, with one diagnostic.
#[test]
#[trace("FR-001", "FR-286")]
fn odd_operands_and_malformed_requests_refuse() {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    std::fs::create_dir_all(case.path("folder")).unwrap();
    let mut unknown = request("packing.fsl", "Remaining", &[]);
    unknown["request"]["extra"] = json!(1);
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        case.write("empty.json", b""),
        case.json("source.json", &request("absent.fsl", "Remaining", &[])),
        case.write("malformed.json", b"[1,"),
        case.json("unknown.json", &unknown),
    ] {
        let outcome = plain(&path);
        outcome.expect("execute", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}: {}",
            path.display(),
            outcome.document
        );
    }
}
