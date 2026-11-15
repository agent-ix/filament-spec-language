// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface stubs for model-based testing (FR-752), hybrid
//! certificates (FR-751), the planner (FR-755), TLA+ and Lean export
//! (FR-756), the TL lowering (FR-757) and the layering CI gate (FR-345).
//! Each feeds source and a `native-run/1` request through the command
//! adapter (`fsl_command::command::run`) and asserts on the output document,
//! because the engines' own entry points do not exist yet.

use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n";

/// FR-447's `Braking` without `t`, with `SlowEnough` over it.
const BRAKING: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always holds(h.v < 30) }\n";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:s1-16:{file}"),
        "document": "S116", "formal_revision": 1})
}

/// Writes the spine files (model and snapshot), `unit` as `unit.native`
/// and a `native-run/1` request whose `request` carries `member`, runs it
/// through the command adapter, and returns its exit code and document.
fn run(unit: &str, with_model: bool, member: (&str, Value)) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let models = if with_model {
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}])
    } else {
        json!([])
    };
    let mut request = json!({"models": models, "program": {"source": source("unit.native")}});
    request[member.0] = member.1;
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

fn write_request(directory: &Path, request: &Value) -> (u8, Value) {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    match fsl_command::command::run(&path) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

// ---- FR-752: model-based testing --------------------------------------

/// FR-752-AC-1: a state clause false at every state is refuted by a
/// 1-minimal counterexample of zero invocations, replayed natively.
#[test]
#[trace("TC-2102", "FR-752-AC-1")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-6")]
#[ignore = "QSL-2237: model-based test generation and shrinking not implemented yet"]
fn a_failing_clause_retains_a_one_minimal_replayed_counterexample() {
    let unit = format!(
        "{MODEL_HEADER}invariant Never using v on Config::ConfigVersion at current {{ false }}\n"
    );
    let (_, value) = run(
        &unit,
        true,
        (
            "model_based_test",
            json!({
        "clause": "Never", "seed": 7, "max_cases": 100, "max_trace_length": 8,
        "initial": "spine-snapshot.json"}),
        ),
    );
    let result = &value["model_based_test"];
    assert_eq!(result["status"], "refuted", "{value}");
    let counterexample = &result["counterexample"];
    assert_eq!(counterexample["minimality"], "1-minimal", "{value}");
    assert_eq!(counterexample["replay"], "reproduced", "{value}");
    assert_eq!(counterexample["invocations"], json!([]), "{value}");
}

/// FR-752-AC-2: every accepted shrink step is strictly smaller and fails
/// the same clause.
#[test]
#[trace("TC-2102", "FR-752-AC-2")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-5")]
#[ignore = "QSL-2237: model-based test shrinking not implemented yet"]
fn every_accepted_shrink_step_is_smaller_and_fails_the_same_clause() {
    let unit = format!(
        "{MODEL_HEADER}invariant Never using v on Config::ConfigVersion at current {{ false }}\n"
    );
    for seed in [1, 2, 3, 42] {
        let (_, value) = run(
            &unit,
            true,
            (
                "model_based_test",
                json!({
            "clause": "Never", "seed": seed, "max_cases": 50, "max_trace_length": 6,
            "initial": "spine-snapshot.json"}),
            ),
        );
        let steps = value["model_based_test"]["shrink_steps"]
            .as_array()
            .unwrap_or_else(|| panic!("shrink steps: {value}"));
        let mut previous = u64::MAX;
        for step in steps {
            let size = step["size"].as_u64().expect("a step size");
            assert!(size < previous, "{value}");
            previous = size;
            assert_eq!(step["failing_clause"], "Never", "{value}");
        }
    }
}

/// FR-752-AC-3: a clause that every case passes yields `tested` evidence,
/// never `proved`, within `max_cases`.
#[test]
#[trace("TC-2102", "FR-752-AC-3")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-7")]
#[ignore = "QSL-2237: model-based testing `tested` evidence not implemented yet"]
fn a_passing_run_records_tested_and_never_proved() {
    let unit = format!(
        "{MODEL_HEADER}invariant Always using v on Config::ConfigVersion at current {{ true }}\n"
    );
    let (_, value) = run(
        &unit,
        true,
        (
            "model_based_test",
            json!({
        "clause": "Always", "seed": 7, "max_cases": 50, "max_trace_length": 4,
        "initial": "spine-snapshot.json"}),
        ),
    );
    let result = &value["model_based_test"];
    assert_eq!(result["evidence"], "tested", "{value}");
    assert_ne!(result["status"], "proved", "{value}");
    assert!(
        result["cases"]["passed"].as_u64().is_some_and(|n| n <= 50),
        "{value}"
    );
}

// ---- FR-751: hybrid certificates --------------------------------------

fn certified_entry(moving_v_upper: u32) -> Value {
    let interval =
        |lower: u32, upper: u32| json!({"lower": lower.to_string(), "upper": upper.to_string()});
    json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "inductive-solver", "method": "inductive-enclosure"},
        "certificate": {"type": "inductive-enclosure", "boxes": {
            "Moving": {"v": interval(0, moving_v_upper), "a": interval(1, 2)},
            "Stopped": {"v": interval(0, 0), "a": interval(1, 2)}}}})
}

/// FR-751-AC-1: an accepted `inductive-enclosure` settles the proof
/// `certified`; one whose `Moving` box excludes the initial `v = 20` is
/// rejected at rule (a) and the proof stays `uncertified`.
#[test]
#[trace("TC-2101", "FR-751-AC-1")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-1")]
#[ignore = "QSL-2234: core check of hybrid provider certificates not implemented yet"]
fn an_inductive_enclosure_certificate_is_checked_in_the_core() {
    let (_, accepted) = run(
        BRAKING,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "entry": certified_entry(20)}),
        ),
    );
    assert_eq!(accepted["hybrid_result"]["value"], "proved", "{accepted}");
    assert_eq!(
        accepted["hybrid_result"]["certification"], "certified",
        "{accepted}"
    );
    let (_, rejected) = run(
        BRAKING,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "entry": certified_entry(19)}),
        ),
    );
    assert_eq!(
        rejected["hybrid_result"]["certification"], "uncertified",
        "{rejected}"
    );
    assert_eq!(
        rejected["hybrid_result"]["certificate"]["outcome"], "rejected",
        "{rejected}"
    );
    assert_eq!(
        rejected["hybrid_result"]["certificate"]["rule"], "a",
        "{rejected}"
    );
}

// ---- FR-755: planner ---------------------------------------------------

/// FR-755-AC-1/AC-2: minimizing length to reach `length >= 1` gives an
/// `optimal` one-step plan with an accepted `potential` certificate; plan
/// lengths `Int[0, 0]` make the goal `infeasible` with a closure certificate.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-1")]
#[ignore = "QSL-1978: planner and optimizer not implemented yet"]
fn the_planner_returns_optimal_and_infeasible_with_certificates() {
    let unit = |lengths: &str| {
        format!(
            "{MODEL_HEADER}type Lengths = {lengths};\n\
             synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
             minimize Reach.length satisfies {{ Reach.length >= 1 }};\n"
        )
    };
    let (_, value) = run(
        &unit("Int[0, 3]"),
        true,
        (
            "synthesis",
            json!({
        "declaration": "Reach", "initial": "spine-snapshot.json"}),
        ),
    );
    let result = &value["synthesis"];
    assert_eq!(result["status"], "optimal", "{value}");
    assert_eq!(result["objective"], 1, "{value}");
    assert_eq!(result["certificate"]["type"], "potential", "{value}");
    assert_eq!(result["certification"], "certified", "{value}");
    let (_, value) = run(
        &unit("Int[0, 0]"),
        true,
        (
            "synthesis",
            json!({
        "declaration": "Reach", "initial": "spine-snapshot.json"}),
        ),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert_eq!(
        value["synthesis"]["certificate"]["type"], "closure",
        "{value}"
    );
}

// ---- FR-756: TLA+ and Lean export --------------------------------------

/// FR-756-AC-1: each profile echoes its id and family and classifies the
/// finite invariant `preserved`; two exports are byte-equal.
#[test]
#[trace("TC-2106", "FR-756-AC-1")]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-1")]
#[ignore = "QSL-1979: TLA+ and Lean output profiles not implemented yet"]
fn tla_and_lean_exports_echo_their_profile_and_preserve_a_finite_invariant() {
    let unit = format!("{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n");
    for (profile, family) in [
        ("filament.output.tla2/v1", "tla"),
        ("filament.output.lean4/v1", "lean"),
    ] {
        let export = || {
            run(
                &unit,
                true,
                (
                    "export",
                    json!({
            "profile": profile, "obligations": ["Low"]}),
                ),
            )
        };
        let (_, first) = export();
        let (_, second) = export();
        assert_eq!(first, second, "deterministic");
        assert_eq!(first["export"]["mapping_profile_id"], profile, "{first}");
        assert_eq!(first["export"]["target_family"], family, "{first}");
        assert_eq!(
            first["export"]["records"][0]["obligation"], "Low",
            "{first}"
        );
        assert_eq!(
            first["export"]["records"][0]["disposition"], "preserved",
            "{first}"
        );
    }
}

/// FR-756-AC-2: hybrid flows export `unrepresented`, never `preserved`.
#[test]
#[trace("TC-2106", "FR-756-AC-2")]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-3")]
#[ignore = "QSL-1979: TLA+ and Lean fidelity classes not implemented yet"]
fn hybrid_flows_export_unrepresented() {
    let (_, value) = run(
        BRAKING,
        false,
        (
            "export",
            json!({
        "profile": "filament.output.tla2/v1", "obligations": ["SlowEnough"]}),
        ),
    );
    assert_eq!(
        value["export"]["records"][0]["disposition"], "unrepresented",
        "{value}"
    );
}

// ---- FR-757: TL lowering -----------------------------------------------

/// FR-757-AC-1: a TL-supported clause lowers to `G[0,3] p0` under
/// `mltl.closed-trace/v1`, `p0` naming its `holds` leaf.
#[test]
#[trace("TC-2107", "FR-757-AC-1")]
#[ignore = "QSL-2147: lowering to TL formulas not implemented yet"]
fn a_supported_clause_lowers_to_a_tl_formula() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal Low using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { always [0, 3] holds(c.versionNumber <= 2) }\n";
    let (_, value) = run(unit, true, ("tl_lowering", json!({"clause": "Low"})));
    let lowered = &value["tl_lowering"];
    assert_eq!(lowered["correspondence"], "mltl.closed-trace/v1", "{value}");
    assert_eq!(lowered["formula"], "G[0,3] p0", "{value}");
    assert!(lowered["atoms"]["p0"].is_string(), "{value}");
}

// ---- FR-345: the layering gate in CI -----------------------------------

/// FR-345-AC-15: `CI_CHECKS` lists `refinement-layering`, whose recipe runs
/// `layering-check`, the check that passes only on the CLI's exit 21 run.
#[test]
#[trace("TC-2108", "FR-345-AC-15")]
#[ignore = "QSL-2246: `refinement-layering` is not in CI_CHECKS yet"]
fn ci_checks_include_the_layering_gate() {
    let makefile = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Makefile"))
        .expect("the Makefile");
    let checks = makefile
        .lines()
        .find_map(|line| line.strip_prefix("CI_CHECKS :="))
        .expect("a CI_CHECKS line");
    assert!(
        checks
            .split_whitespace()
            .any(|target| target == "refinement-layering"),
        "{checks}"
    );
    assert!(
        makefile.contains("refinement layering-check"),
        "the recipe runs layering-check"
    );
    // The check accepts no exit other than 21: 0 and 10 are rejected.
    for code in [0, 10] {
        assert!(xtask::refinement::layering_check::expected_run(Some(code), b"{}").is_err());
    }
}
