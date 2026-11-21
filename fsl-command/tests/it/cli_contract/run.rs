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

/// The command admits canonical values through the declared kernel type and
/// writes canonical results, preserving exact values and normalizing sets
/// and rational fractions.
#[test]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-18")]
fn canonical_run_values_preserve_exact_values_and_normalize_admitted_inputs() {
    let case = Case::new();
    case.write(
        "identity.fsl",
        br#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
function Number using v(x: Integer): Integer pure { x }
function Flag using v(x: Boolean): Boolean pure { x }
function Fraction using v(x: Rational[-10, 10; 1, 20]): Rational[-10, 10; 1, 20] pure { x }
function Words using v(x: Text[0, 40; unicode-scalars]): Text[0, 40; unicode-scalars] pure { x }
function List using v(x: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { x }
function Members using v(x: Set<Integer>[0, 3]): Set<Integer>[0, 3] pure { x }
function Constant using v(): Integer pure { 5 }
"#,
    );
    let integer = |value: &str| json!({"type":"integer", "value":value});
    let inputs = [
        (
            "Number",
            integer("12345678901234567890123"),
            integer("12345678901234567890123"),
        ),
        ("Number", integer("-5"), integer("-5")),
        ("Number", integer("0"), integer("0")),
        (
            "Flag",
            json!({"type":"boolean","value":true}),
            json!({"type":"boolean","value":true}),
        ),
        (
            "Fraction",
            json!({"type":"rational","numerator":"4","denominator":"6"}),
            json!({"type":"rational","numerator":"2","denominator":"3"}),
        ),
        (
            "Words",
            json!({"type":"text","value":"hello"}),
            json!({"type":"text","value":"hello"}),
        ),
        (
            "List",
            json!({"type":"sequence","elements":[]}),
            json!({"type":"sequence","elements":[]}),
        ),
        (
            "List",
            json!({"type":"sequence","elements":[integer("3"),integer("1"),integer("2")]}),
            json!({"type":"sequence","elements":[integer("3"),integer("1"),integer("2")]}),
        ),
        (
            "Members",
            json!({"type":"set","elements":[integer("3"),integer("1")]}),
            json!({"type":"set","elements":[integer("1"),integer("3")]}),
        ),
    ];
    for (function, input, expected) in inputs {
        let path = case.json(
            "run.json",
            &request("identity.fsl", function, &[("x", input)]),
        );
        let outcome = plain(&path);
        let document = outcome.expect("execute", "success", 0);
        assert_eq!(
            document["result"]["value"], expected,
            "{function}: {document}"
        );
    }
    let path = case.json("constant.json", &request("identity.fsl", "Constant", &[]));
    assert_eq!(
        plain(&path).expect("execute", "success", 0)["result"]["value"],
        integer("5")
    );
}

/// Legacy shapes and malformed canonical inputs refuse at the command boundary.
#[test]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-19")]
fn canonical_run_values_refuse_legacy_and_malformed_inputs() {
    for value in [
        json!({"integer":"7"}),
        json!(7),
        json!({"type":"integer","value":"07"}),
        json!({"type":"integer","value":"7","extra":true}),
    ] {
        let (_case, path) = packing("IsPacked", &[("n", value)]);
        let outcome = plain(&path);
        assert_eq!(outcome.exit, 20, "{}", outcome.document);
    }
    let (_case, path) = packing("IsPacked", &[("n", json!({"type":"integer","value":"11"}))]);
    assert_eq!(plain(&path).exit, 20);
}

/// Run the owner-maintained corpus in place; the source checkout is never
/// copied into this repository and its refusal snapshots are not the oracle.
#[test]
#[ignore = "external acceptance lane: FSL_RUN_INPUTS_FIXTURES names the authoritative examples/run-inputs directory"]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-18")]
fn canonical_run_acceptance_values_from_authoritative_examples() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("FSL_RUN_INPUTS_FIXTURES")
            .expect("the acceptance lane supplies its read-only fixture directory"),
    );
    let integer = |value: &str| json!({"type":"integer", "value":value});
    for (case, expected) in [
        ("call-two-arguments", integer("5")),
        ("call-arguments-reordered", integer("5")),
        ("call-big-integer", integer("99999999999999999999999")),
        ("call-negative-integer", integer("-5")),
        ("call-zero-integer", integer("0")),
        ("call-boolean", json!({"type":"boolean","value":true})),
        (
            "call-sequence-at-bound",
            json!({"type":"sequence","elements":[integer("1"),integer("2"),integer("3")]}),
        ),
        (
            "call-sequence-empty",
            json!({"type":"sequence","elements":[]}),
        ),
        ("call-digit-at-max", integer("9")),
        ("call-digit-at-min", integer("0")),
        ("call-tagged-integer", integer("5")),
        (
            "call-tagged-rational",
            json!({"type":"rational","numerator":"2","denominator":"3"}),
        ),
        ("call-tagged-text", json!({"type":"text","value":"hi"})),
        (
            "call-tagged-set-order",
            json!({"type":"set","elements":[integer("1"),integer("3")]}),
        ),
        ("option-engine-interpreter", integer("5")),
        ("option-request-bytes-large", integer("5")),
    ] {
        let outcome = plain(&directory.join("requests").join(format!("{case}.json")));
        let document = outcome.expect("execute", "success", 0);
        assert_eq!(document["result"]["value"], expected, "{case}: {document}");
    }
}

/// The public command entry uses the existing plan coordinator and retains
/// distinct passed, refuted and blocked outcomes.
#[test]
#[ignore = "external acceptance lane: FSL_RUN_INPUTS_FIXTURES names the authoritative examples/run-inputs directory"]
#[trace("FSpec-FR-824-AC-1", "FSpec-FR-824-AC-6", "FR-1038-AC-1")]
fn canonical_run_acceptance_plans_retain_dispositions_and_severity() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("FSL_RUN_INPUTS_FIXTURES")
            .expect("the acceptance lane supplies its read-only fixture directory"),
    );
    for (case, status, exit) in [
        ("plan-passes", "passed", 0),
        ("plan-fails", "failed", 10),
        ("plan-blocks", "failed", 22),
        ("plan-null-members-as-absent", "passed", 0),
    ] {
        let outcome = plain(&directory.join("requests").join(format!("{case}.json")));
        let document = &outcome.document;
        assert_eq!(outcome.exit, exit, "{case}: {document}");
        assert_eq!(document["status"], status, "{case}: {document}");
        let dispositions = document["dispositions"]
            .as_object()
            .expect("one named disposition per plan step");
        assert_eq!(dispositions.len(), 2, "{case}: {document}");
        if case == "plan-fails" || case == "plan-blocks" {
            assert_eq!(document["failed_steps"], json!(["low"]), "{document}");
            assert_eq!(dispositions["low"]["status"], "refuted", "{document}");
        }
        if case == "plan-blocks" {
            assert!(
                dispositions
                    .values()
                    .any(|step| step["status"] == "blocked" && step["result"].is_null()),
                "{document}"
            );
        }
    }
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
    let document = open.expect("monitor", "indeterminate", 11);
    let item = &document["items"][0];
    assert_eq!(item["result"], "indeterminate", "{document}");
    assert_eq!(item["cause"]["kind"], "missing-valuation", "{document}");
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

/// FR-100-AC-4: an out-of-range argument (`invalid-value`) and a wrong value
/// kind (`wrong-value-kind`) each refuse `invalid_runtime_input`, exit 20.
#[test]
#[trace("FR-100-AC-4")]
fn call_defects_refuse_invalid_runtime_input() {
    for (function, arguments, cause) in [
        (
            "IsPacked",
            vec![("n", json!({"integer": "11"}))],
            "invalid-value",
        ),
        (
            "IsPacked",
            vec![("n", json!({"boolean": true}))],
            "wrong-value-kind",
        ),
    ] {
        let (_case, path) = packing(function, &arguments);
        let outcome = plain(&path);
        let diagnostic = outcome.refused("execute", 20, "invalid_runtime_input");
        assert_eq!(diagnostic["cause"], cause, "{diagnostic}");
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

/// `--request-bytes` and `--dependent-bytes` (FR-026-AC-4): one byte under the
/// request or source size stops at intake with exit 22 naming the ceiling;
/// the exact size runs.
#[test]
#[trace("FR-026-AC-4")]
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
            ..RunLimits::default()
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
