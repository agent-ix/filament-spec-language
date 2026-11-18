// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface stubs for model-based testing (FR-752), hybrid
//! certificates (FR-751), the planner (FR-755), TLA+ and Lean export
//! (FR-756), the TL lowering (FR-757) and the layering CI gate (FR-345).
//! Each feeds source and a `native-run/1` request through the command
//! adapter (`fsl_command::command::run`) and asserts on the output document,
//! because the engines' own entry points do not exist yet.

use crate::support::config_version_spine::{Case, spine};
use ix_trace_rs::trace;
use serde_json::{Value, json};
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
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
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
    let unit = format!(
        "{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n"
    );
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

// ---- Fix round 1: coverage of the remaining criteria ------------------

const VALUE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n";

/// `SlowEnough10` over `Braking` without `t`: horizon 10 seconds.
fn braking_with_horizon() -> String {
    BRAKING.replace("always holds(h.v < 30)", "always [0, 10] holds(h.v < 30)")
}

fn refusal_names(value: &Value, member: &str) {
    assert!(
        value.to_string().contains(member),
        "names {member}: {value}"
    );
}

/// FR-750-AC-1: a provider's `delta-complete` `unsat` covering horizon 10
/// settles `proved`, `bounded-solver`, `uncertified`, naming the provider.
#[test]
#[trace("TC-2100", "FR-750-AC-1")]
#[ignore = "QSL-2234: analyze dispatch of a hybrid item to its provider not implemented yet"]
fn a_covering_delta_complete_proof_settles_proved_bounded_solver() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": "10", "jumps": 1, "exhaustive": ["time"]}});
    let (_, value) = run(
        &braking_with_horizon(),
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "backend": "hybrid-fixture", "entry": entry}),
        ),
    );
    let result = &value["hybrid_result"];
    assert_eq!(result["value"], "proved", "{value}");
    assert_eq!(result["basis"]["type"], "bounded-solver", "{value}");
    assert_eq!(result["certification"], "uncertified", "{value}");
    assert_eq!(result["backend"], "hybrid-fixture", "{value}");
}

/// FR-750-AC-2: a hybrid provider registration whose `dynamics` lacks
/// `hybrid` is refused naming the backend, and no provider is made.
#[test]
#[trace("TC-2100", "FR-750-AC-2")]
#[ignore = "QSL-2234: hybrid provider registration (`Engine::hybrid`) not implemented yet"]
fn a_hybrid_provider_without_hybrid_dynamics_is_refused() {
    let (exit, value) = run(
        &braking_with_horizon(),
        false,
        (
            "hybrid_providers",
            json!([{
        "backend": "discrete-only", "dynamics": ["discrete"],
        "advertises": [{"kind": "temporal-satisfaction", "mode": "bounded"}]}]),
        ),
    );
    assert_eq!(exit, 20, "{value}");
    refusal_names(&value, "discrete-only");
    refusal_names(&value, "invalid-dynamics");
}

fn point_trace(before_v: u32) -> Value {
    let state = |mode: &str, time: &str, v: u32| {
        json!({"mode": mode,
        "time": {"lower": time, "upper": time},
        "enclosure": {"v": {"lower": v.to_string(), "upper": v.to_string()},
            "a": {"lower": "2", "upper": "2"}}})
    };
    json!({"start": state("Moving", "0", 20), "jumps": [{"transition": 0,
        "before": {"time": {"lower": "10", "upper": "10"},
            "enclosure": {"v": {"lower": before_v.to_string(), "upper": before_v.to_string()},
                "a": {"lower": "2", "upper": "2"}}},
        "state": state("Stopped", "10", 0)}]})
}

/// FR-750-AC-5: a process provider's `refuted` point trace that exact
/// replay reproduces settles `refuted` with one `HybridCounterexample`; one
/// that does not reproduce settles `inconclusive`, `replay-parity`.
#[test]
#[trace("TC-2100", "FR-750-AC-5")]
#[ignore = "QSL-2234: process-provider composition for hybrid clause requests not implemented yet"]
fn a_process_provider_refutation_settles_by_exact_replay() {
    let unit = BRAKING.replace("always holds(h.v < 30)", "always [0, 12] holds(h.v > 0)");
    let entry = |before_v: u32| {
        json!({"request_index": 0, "result": "refuted",
        "counterexample": 0, "counterexamples": [{"type": "hybrid-counterexample",
            "kind": "formula", "trace": point_trace(before_v)}]})
    };
    let (_, value) = run(
        &unit,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "provider": "process", "entry": entry(0)}),
        ),
    );
    assert_eq!(value["hybrid_result"]["value"], "refuted", "{value}");
    assert_eq!(
        value["hybrid_result"]["counterexamples"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "{value}"
    );
    let (_, value) = run(
        &unit,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "provider": "process", "entry": entry(1)}),
        ),
    );
    assert_eq!(value["hybrid_result"]["value"], "inconclusive", "{value}");
    assert_eq!(
        value["hybrid_result"]["cause"]["type"], "replay-parity",
        "{value}"
    );
}

/// FR-751-AC-2: a `flowpipe` step violating the Picard condition is
/// rejected at that step's pointer and the proof stays `uncertified`.
#[test]
#[trace("TC-2101", "FR-751-AC-2")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-2")]
#[ignore = "QSL-2234: core check of flowpipe certificates not implemented yet"]
fn a_flowpipe_step_violating_picard_is_rejected() {
    let interval = |lower: &str, upper: &str| json!({"lower": lower, "upper": upper});
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "flowpipe-outward-rounded",
            "horizon": "10", "jumps": 1, "exhaustive": ["time"]},
        "certificate": {"type": "flowpipe", "steps": [{"mode": "Moving",
            "time": interval("0", "10"), "step": "10",
            "box": {"v": interval("0", "20"), "a": interval("1", "2")},
            "apriori": {"v": interval("19", "20"), "a": interval("1", "2")}}]}});
    let (_, value) = run(
        &braking_with_horizon(),
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "entry": entry}),
        ),
    );
    let result = &value["hybrid_result"];
    assert_eq!(result["certification"], "uncertified", "{value}");
    assert_eq!(result["certificate"]["outcome"], "rejected", "{value}");
    assert_eq!(
        result["certificate"]["at"], "/certificate/steps/0",
        "{value}"
    );
}

/// FR-751-AC-3: a certificate check given a work limit of one box
/// evaluation is rejected `work-limit`, never accepted.
#[test]
#[trace("TC-2101", "FR-751-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
#[ignore = "QSL-2234: work-limited certificate check not implemented yet"]
fn a_work_limited_certificate_check_is_rejected() {
    let (_, value) = run(
        BRAKING,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "entry": certified_entry(20), "limits": {"work": 1}}),
        ),
    );
    assert_eq!(
        value["hybrid_result"]["certificate"]["outcome"], "rejected",
        "{value}"
    );
    assert_eq!(
        value["hybrid_result"]["certificate"]["rule"], "work-limit",
        "{value}"
    );
}

/// FSpec FR-652-AC-4: a `flowpipe` certificate on an `inductive-solver`
/// entry refuses `invalid_result`/`invalid-value` at `/certificate`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
#[ignore = "QSL-2234: certificate form check not implemented yet"]
fn a_mismatched_certificate_form_refuses_at_certificate() {
    let mut entry = certified_entry(20);
    entry["certificate"] = json!({"type": "flowpipe", "steps": []});
    let (_, value) = run(
        BRAKING,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "SlowEnough", "entry": entry}),
        ),
    );
    refusal_names(&value, "invalid_result");
    refusal_names(&value, "/certificate");
}

/// FSpec FR-652-AC-3 and FR-193-AC-22: a point trace through a mode whose
/// flow is not polynomial refutes with an accepted `validated-witness`, and
/// without one stays `inconclusive`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-193-AC-22")]
#[ignore = "QSL-2234: validated-witness refutation not implemented yet"]
fn a_validated_witness_refutes_through_a_non_polynomial_flow() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Grow using p {\n\
           mode Up invariant { v >= 1 and v <= 100 } flow { v' = v * v / (1 + v * v); }\n\
         }\n\
         temporal Below using p over (h: Grow) clock \"model-time\" \
         on origin { always [0, 40] holds(h.v < 30) }\n";
    let trace = json!({"start": {"mode": "Up", "time": {"lower": "0", "upper": "0"},
        "enclosure": {"v": {"lower": "1", "upper": "1"}}}, "jumps": []});
    let witness = json!({"type": "validated-witness", "dwells": [[{"mode": "Up",
        "time": {"lower": "0", "upper": "40"}, "step": "40",
        "box": {"v": {"lower": "1", "upper": "1"}},
        "apriori": {"v": {"lower": "1", "upper": "41"}}}]]});
    let entry = |certificate: Option<Value>| {
        let mut entry = json!({"request_index": 0, "result": "inconclusive",
            "cause": {"type": "solver-inconclusive", "trace": trace.clone()}});
        if let Some(certificate) = certificate {
            entry["certificate"] = certificate;
        }
        entry
    };
    let (_, with) = run(
        unit,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "Below", "entry": entry(Some(witness))}),
        ),
    );
    assert_eq!(with["hybrid_result"]["value"], "refuted", "{with}");
    let (_, without) = run(
        unit,
        false,
        (
            "hybrid_result",
            json!({
        "claim": "Below", "entry": entry(None)}),
        ),
    );
    assert_eq!(
        without["hybrid_result"]["value"], "inconclusive",
        "{without}"
    );
}

const RELEASE_PLAN: &str = "invariant Never using v on Config::ConfigVersion at current { false }\n\
     invariant Always using v on Config::ConfigVersion at current { true }\n\
     verify Release using v {\n\
       check safe claim Never method model_based_test domain Config::ConfigVersion bound 20;\n\
       check tested claim Always method model_based_test domain Config::ConfigVersion bound 20;\n\
       check timely claim Always method model_based_test domain Config::ConfigVersion depends [safe];\n\
     }\n";

fn compile_bytes(unit: &str) -> Result<Vec<u8>, Value> {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-compile/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")}}});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    fsl_command::command::compile(&path).map_err(|error| error.value().unwrap())
}

/// FR-753-AC-3 and FSpec FR-192-AC-11: the checked package is byte-equal
/// with and without the plan, and the plan document holds three nodes and
/// one `requires` edge.
#[test]
#[trace("TC-2103", "FR-753-AC-3")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-11")]
#[ignore = "QSL-2236: `verify` plan emission outside the checked package not implemented yet"]
fn a_plan_leaves_the_package_identity_unchanged_and_emits_its_document() {
    let plan_free = RELEASE_PLAN.split("verify Release").next().unwrap();
    let without = compile_bytes(&format!("{MODEL_HEADER}{plan_free}")).expect("compiles");
    let with = compile_bytes(&format!("{MODEL_HEADER}{RELEASE_PLAN}"))
        .unwrap_or_else(|refusal| panic!("compiles with the plan: {refusal}"));
    assert_eq!(with, without);
    let (_, value) = run(
        &format!("{MODEL_HEADER}{RELEASE_PLAN}"),
        true,
        (
            "verification_plan",
            json!({"plan": "Release", "emit": true}),
        ),
    );
    let document = &value["verification_plan"]["document"];
    assert_eq!(
        document["format"], "filament.verification-plan/v1",
        "{value}"
    );
    assert_eq!(
        document["nodes"].as_array().map(Vec::len),
        Some(3),
        "{value}"
    );
    assert_eq!(
        document["edges"],
        json!([{"from": "timely", "to": "safe", "type": "requires"}]),
        "{value}"
    );
}

/// FR-753-AC-4: executing `Release` with `safe` refuted still runs `tested`
/// and blocks `timely` naming `safe`, one disposition per node.
#[test]
#[trace("TC-2103", "FR-753-AC-4")]
#[ignore = "QSL-2236: verification plan execution not implemented yet"]
fn executing_a_plan_blocks_only_descendants_of_a_failed_step() {
    let (_, value) = run(
        &format!("{MODEL_HEADER}{RELEASE_PLAN}"),
        true,
        (
            "verification_plan",
            json!({"plan": "Release", "execute": true,
            "initial": "spine-snapshot.json", "seed": 7}),
        ),
    );
    let nodes = &value["verification_plan"]["dispositions"];
    assert_eq!(nodes["safe"]["status"], "refuted", "{value}");
    assert_eq!(nodes["tested"]["evidence"], "tested", "{value}");
    assert_eq!(nodes["timely"]["status"], "blocked", "{value}");
    assert_eq!(nodes["timely"]["blocked_by"], "safe", "{value}");
    assert_eq!(nodes.as_object().map(|n| n.len()), Some(3), "{value}");
}

fn pick(block: &str, objective: &str) -> String {
    format!(
        "{VALUE_HEADER}type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit {objective} satisfies {{ {block} }};\n"
    )
}

/// FR-754-AC-3: `Pick` returns `candidate` 7 `validated`; `= 50` returns
/// `no-candidate` after ten values; `minimize Pick` with `Pick >= 3` gives 3.
#[test]
#[trace("TC-2104", "FR-754-AC-3")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-6")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
#[ignore = "QSL-2235: bounded synthesis run not implemented yet"]
fn pick_finds_validates_exhausts_and_optimizes() {
    let synth = |unit: String| run(&unit, false, ("synthesis", json!({"declaration": "Pick"}))).1;
    let found = synth(pick("Pick * Pick = 49", ""));
    assert_eq!(found["synthesis"]["status"], "candidate", "{found}");
    assert_eq!(found["synthesis"]["candidate"], 7, "{found}");
    assert_eq!(found["synthesis"]["check"], "validated", "{found}");
    let none = synth(pick("Pick * Pick = 50", ""));
    assert_eq!(none["synthesis"]["status"], "no-candidate", "{none}");
    assert_eq!(none["synthesis"]["visited"], 10, "{none}");
    let optimal = synth(pick("Pick >= 3", "minimize Pick"));
    assert_eq!(optimal["synthesis"]["candidate"], 3, "{optimal}");
    assert_eq!(optimal["synthesis"]["optimal"], true, "{optimal}");
}

/// FSpec FR-194-AC-10: a search stopped by its work limit is `incomplete`
/// naming the limit, never `no-candidate`.
#[test]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-10")]
#[ignore = "QSL-2235: work-limited bounded synthesis not implemented yet"]
fn a_work_limited_synthesis_is_incomplete() {
    let (_, value) = run(
        &pick("Pick * Pick = 50", ""),
        false,
        (
            "synthesis",
            json!({"declaration": "Pick", "limits": {"work": 1}}),
        ),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
    refusal_names(&value["synthesis"], "work");
}

fn reach(objective: &str, policy: Value) -> Value {
    let unit = format!(
        "{MODEL_HEADER}type Lengths = Int[0, 3];\n\
         temporal NotOne using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ always holds(c.versionNumber != 1) }}\n\
         temporal SomeTime using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 3) }}\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         {objective} satisfies {{ Reach.length >= 1 }};\n"
    );
    run(
        &unit,
        true,
        (
            "synthesis",
            json!({"declaration": "Reach",
        "initial": "spine-snapshot.json", "policy": policy}),
        ),
    )
    .1
}

/// FR-755-AC-3 and FSpec FR-650-AC-4: under the policy `NotOne` the
/// replayed plan never visits version 1; a policy naming no clause refuses
/// `missing-name`; an `eventually` policy is evaluated, not refused.
#[test]
#[trace("TC-2105", "FR-755-AC-3")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-4")]
#[ignore = "QSL-1978: planner policy clauses not implemented yet"]
fn the_planner_keeps_plans_within_policy() {
    let value = reach("", json!(["NotOne"]));
    let result = &value["synthesis"];
    assert!(
        matches!(result["status"].as_str(), Some("optimal" | "infeasible")),
        "{value}"
    );
    for state in result["plan"]["states"].as_array().into_iter().flatten() {
        assert_ne!(state["versionNumber"], 1, "{value}");
    }
    refusal_names(&reach("", json!(["Nowhere"])), "missing-name");
    // FSpec FR-803: a non-safety policy is evaluated on each completed plan.
    let eventually = reach("", json!(["SomeTime"]));
    assert!(
        matches!(
            eventually["synthesis"]["status"].as_str(),
            Some("optimal" | "infeasible")
        ),
        "{eventually}"
    );
}

/// FSpec FR-650-AC-2: a per-step cost objective is minimized and certified.
#[test]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-2")]
#[ignore = "QSL-1978: per-step cost objectives not implemented yet"]
fn the_planner_minimizes_a_per_step_cost_sum() {
    let value = reach("minimize Reach.steps.sum(s => 2)", json!([]));
    assert_eq!(value["synthesis"]["status"], "optimal", "{value}");
    assert_eq!(value["synthesis"]["objective"], 2, "{value}");
    assert_eq!(
        value["synthesis"]["certificate"]["type"], "potential",
        "{value}"
    );
}

/// FSpec FR-650-AC-3 and AC-5: plan lengths admitting no goal give
/// `infeasible`; a run stopped by its work limit is `incomplete` naming it.
#[test]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-3")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-5")]
#[ignore = "QSL-1978: planner infeasibility and work limits not implemented yet"]
fn the_planner_reports_infeasible_and_incomplete() {
    let unit = format!(
        "{MODEL_HEADER}type Lengths = Int[0, 0];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach.length >= 1 }};\n"
    );
    let (_, value) = run(
        &unit,
        true,
        (
            "synthesis",
            json!({"declaration": "Reach",
        "initial": "spine-snapshot.json"}),
        ),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    let wide = unit.replace("Int[0, 0]", "Int[0, 3]");
    let (_, value) = run(
        &wide,
        true,
        (
            "synthesis",
            json!({"declaration": "Reach",
        "initial": "spine-snapshot.json", "limits": {"max_work": 1}}),
        ),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
}

/// FSpec FR-650-AC-6: a supplied plan with a disabled invocation is never
/// reported; replaying it is a method failure naming that step.
#[test]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-6")]
#[ignore = "QSL-1978: native replay of planner plans not implemented yet"]
fn a_plan_with_a_disabled_invocation_is_a_method_failure() {
    let unit = format!(
        "{MODEL_HEADER}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach.length >= 1 }};\n"
    );
    let (_, value) = run(
        &unit,
        true,
        (
            "synthesis",
            json!({"declaration": "Reach",
        "initial": "spine-snapshot.json",
        "replay_plan": [{"operation": "noSuchOperation"}]}),
        ),
    );
    refusal_names(&value, "method-failure");
    refusal_names(&value, "noSuchOperation");
}

/// FSpec FR-651-AC-2, AC-4 and FR-297-AC-5: the Lean export preserves the
/// invariant; a non-obligation member is refused; a cross-family selection
/// refuses with no target output.
#[test]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-2")]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-4")]
#[trace("FSpec-TC-1604", "FSpec-FR-297-AC-5")]
#[ignore = "QSL-1979: Lean profile, obligation refusal and family check not implemented yet"]
fn lean_export_refusals_and_family_check() {
    let unit = format!(
        "{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n"
    );
    let (_, lean) = run(
        &unit,
        true,
        (
            "export",
            json!({
        "profile": "filament.output.lean4/v1", "obligations": ["Low"]}),
        ),
    );
    assert_eq!(
        lean["export"]["records"][0]["disposition"], "preserved",
        "{lean}"
    );
    let (_, missing) = run(
        &unit,
        true,
        (
            "export",
            json!({
        "profile": "filament.output.tla2/v1", "obligations": ["Low", "Nothing"]}),
        ),
    );
    assert_eq!(
        missing["export"]["records"][1]["disposition"], "refused",
        "{missing}"
    );
    let (exit, cross) = run(
        &unit,
        true,
        (
            "export",
            json!({
        "profile": "filament.output.lean4/v1", "target_family": "tla", "obligations": ["Low"]}),
        ),
    );
    assert_eq!(exit, 20, "{cross}");
    assert!(cross.get("export").is_none(), "{cross}");
}

/// FSpec FR-651-AC-5: TLC's reachable state count and verdict over the
/// TLA+ export equal FSL's exploration of the same model.
#[test]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-5")]
#[ignore = "QSL-1979: TLA+ export conformance with an observer not implemented yet"]
fn tla_export_agrees_with_fsl_exploration() {
    let unit = format!(
        "{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n"
    );
    let (_, value) = run(
        &unit,
        true,
        (
            "export",
            json!({
        "profile": "filament.output.tla2/v1", "obligations": ["Low"], "observer": "tlc",
        "initial": "spine-snapshot.json"}),
        ),
    );
    let export = &value["export"];
    assert!(export["observer"]["reachable_states"].is_u64(), "{value}");
    assert_eq!(
        export["observer"]["reachable_states"], export["native"]["reachable_states"],
        "{value}"
    );
    assert_eq!(
        export["observer"]["verdict"], export["native"]["verdict"],
        "{value}"
    );
}

/// FR-757-AC-2 and AC-3: native and `tl-mltl` verdicts agree at every
/// activation position; a timed-profile clause lowers to no formula.
#[test]
#[trace("TC-2107", "FR-757-AC-2")]
#[trace("TC-2107", "FR-757-AC-3")]
#[ignore = "QSL-2147: cross_evaluate.initial (behaviour enumeration) is not implemented yet"]
fn tl_cross_evaluation_agrees_and_timed_clauses_do_not_lower() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal Low using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { always [0, 3] holds(c.versionNumber <= 2) }\n";
    let (_, value) = run(
        unit,
        true,
        (
            "tl_lowering",
            json!({"clause": "Low",
        "cross_evaluate": {"initial": "spine-snapshot.json", "max_steps": 4}}),
        ),
    );
    assert_eq!(
        value["tl_lowering"]["cross_evaluation"]["faults"],
        json!([]),
        "{value}"
    );
    assert_eq!(
        value["tl_lowering"]["cross_evaluation"]["agree"], true,
        "{value}"
    );
    let timed = timed_low();
    let (_, value) = run(&timed, true, ("tl_lowering", json!({"clause": "Low"})));
    assert!(value["tl_lowering"].get("formula").is_none(), "{value}");
    assert_eq!(
        value["tl_lowering"]["disposition"], "unsupported",
        "{value}"
    );
}

/// FSpec FR-190-AC-8 to AC-10: an unbounded parameter needs
/// `max_value_bits`; `max_cases` 0 and an unknown clause refuse naming the
/// member; two equal runs give byte-equal records.
#[test]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-8")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-9")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-10")]
#[ignore = "QSL-2237: model-based test request rules not implemented yet"]
fn model_based_test_request_rules() {
    let unit =
        format!("{VALUE_HEADER}predicate Square using v (x: Integer): Boolean {{ x * x >= 0 }}\n");
    let mbt = |member: Value| run(&unit, false, ("model_based_test", member)).1;
    let base = json!({"clause": "Square", "seed": 7, "max_cases": 20});
    refusal_names(&mbt(base.clone()), "max_value_bits");
    let mut bounded = base.clone();
    bounded["max_value_bits"] = json!(8);
    let first = mbt(bounded.clone());
    assert_eq!(
        first["model_based_test"]["provenance"]["max_value_bits"], 8,
        "{first}"
    );
    assert_eq!(first, mbt(bounded.clone()), "deterministic");
    let mut zero = bounded.clone();
    zero["max_cases"] = json!(0);
    refusal_names(&mbt(zero), "max_cases");
    let mut unknown = bounded;
    unknown["clause"] = json!("Nothing");
    refusal_names(&mbt(unknown), "Nothing");
}

// ---- FR-931: the `tl_lowering` request member ---------------------------

/// FR-757-AC-1's `Low` under `profile`, with the body `body`.
fn low(profile: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"example/config-version\";\n\
         temporal Low using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {body} }}\n"
    )
}

/// The timed form of `Low` (FR-757-AC-3, FR-931-AC-2): the ConfigVersion model
/// timed in `ms`, under the timed profile with no `clock` member, so bound
/// `model-time`; it checks and FR-527 classifies it unsupported. The header
/// selects the timed facet (`filament.temporal.complete/v1`), which a `time`
/// member needs (FR-230).
fn timed_low() -> String {
    "language \"ix:native\" edition \"1-draft\";\n\
     profile tc = \"filament.temporal.complete/v1\";\n\
     profile t = \"filament.temporal.timed/v1\";\n\
     model Config = \"example/config-version\";\n\
     dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n\
     temporal Low using t over (c: Config::ConfigVersion) \
     on origin { always [0 ms, 3 ms] holds(c.versionNumber <= 2) }\n"
        .to_owned()
}

const EVENT_POSITION: &str = "filament.temporal.event-position.false-extension/v1";
const LOW_BODY: &str = "always [0, 3] holds(c.versionNumber <= 2)";

/// FR-931-AC-1 and AC-2: the default and the named correspondences, and the
/// unsupported record when the clause does not have the named reading.
#[test]
#[trace("TC-2601", "FR-931-AC-1")]
#[trace("TC-2601", "FR-931-AC-2")]
fn a_tl_lowering_request_selects_its_correspondence() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let lower = |member: Value| run(&unit, true, ("tl_lowering", member));
    let (code, default) = lower(json!({"clause": "Low"}));
    assert_eq!(code, 0, "{default}");
    let record = &default["tl_lowering"];
    assert_eq!(record["disposition"], "supported", "{default}");
    assert_eq!(record["correspondence"], "mltl.closed-trace/v1");
    assert_eq!(record["formula"], "G[0,3] p0");
    assert!(record["atoms"]["p0"].is_string(), "{default}");
    let (code, named) = lower(json!({"clause": "Low", "correspondence": "mltl.closed-trace/v1"}));
    assert_eq!(code, 0);
    assert_eq!(
        named["tl_lowering"], default["tl_lowering"],
        "byte-equal records"
    );

    let (code, open) = lower(json!({"clause": "Low", "correspondence": "mltl.online-prefix/v1"}));
    assert_eq!(code, 0, "{open}");
    assert_eq!(open["tl_lowering"]["disposition"], "supported");
    assert_eq!(
        open["tl_lowering"]["correspondence"],
        "mltl.online-prefix/v1"
    );

    let (code, past) =
        lower(json!({"clause": "Low", "correspondence": "mltl.origin-complete-history/v1"}));
    assert_eq!(code, 21, "{past}");
    let record = &past["tl_lowering"];
    assert_eq!(record["disposition"], "unsupported");
    assert_eq!(record["classified"], "mltl.closed-trace/v1");
    assert_eq!(record["dimensions"], json!(["direction"]));
    assert!(record.get("formula").is_none(), "{past}");

    let history = low(
        EVENT_POSITION,
        "historically [0, 2] holds(c.versionNumber <= 2)",
    );
    let (code, value) = run(&history, true, ("tl_lowering", json!({"clause": "Low"})));
    assert_eq!(code, 0, "{value}");
    assert_eq!(
        value["tl_lowering"]["correspondence"],
        "mltl.origin-complete-history/v1"
    );
}

/// A driver can execute the value returned by the public preparation API.
/// Provenance: QSL-2348.
#[test]
#[trace("TC-2601", "FR-931-AC-1")]
#[trace("FR-312-AC-3", "TC-2455", "FR-865-AC-5")]
fn a_prepared_tl_lowering_uses_the_public_executor_and_keeps_limits() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = low(EVENT_POSITION, LOW_BODY);
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();

    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("spine-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")},
        "tl_lowering": {"clause": "Low"}
    }});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();

    let expected = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let prepared =
        fsl_command::command::prepare_run(&path, fsl_command::command::RunLimits::default())
            .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let fsl_command::command::PreparedRun::TlLowering(prepared) = prepared else {
        panic!("the tl_lowering request must prepare a TL lowering")
    };
    let actual = fsl_command::command::execute_tl_lowering(*prepared)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));

    assert_eq!(actual.exit_code, expected.exit_code);
    assert_eq!(actual.value.to_string(), expected.value.to_string());

    let mut failing_request = request.clone();
    failing_request["request"]["tl_lowering"]["clause"] = json!("Missing");
    let failing_path = directory.path().join("failing-lowering.json");
    std::fs::write(&failing_path, serde_json::to_vec(&failing_request).unwrap()).unwrap();
    let prepared = fsl_command::command::prepare_run(
        &failing_path,
        fsl_command::command::RunLimits::default(),
    )
    .unwrap_or_else(|error| panic!("the failing clause still prepares: {error}"));
    let fsl_command::command::PreparedRun::TlLowering(prepared) = prepared else {
        panic!("the request must prepare a TL lowering")
    };
    let error = fsl_command::command::execute_tl_lowering(*prepared)
        .expect_err("the absent clause fails during lowering");
    let paths = error
        .sources()
        .as_slice()
        .iter()
        .map(fsl_foundation::Source::path)
        .collect::<Vec<_>>();
    assert!(paths.contains(&"unit.native"), "{paths:?}");
    assert!(paths.contains(&"spine-model.semantic-ir.json"), "{paths:?}");

    let limited = fsl_command::command::prepare_run(
        &path,
        fsl_command::command::RunLimits {
            dependent_bytes: 1,
            ..fsl_command::command::RunLimits::default()
        },
    )
    .err()
    .expect("the configured dependent-file ceiling must refuse before execution");
    assert_eq!(limited.category().exit_code(), 22);
    let refusal = limited.value().expect("a refusal outcome document");
    assert!(refusal.to_string().contains("dependent_bytes"), "{refusal}");
}

/// FR-931-AC-2's timed case: `Low` under the timed profile is unsupported
/// with no classified target and no formula.
#[test]
#[trace("TC-2601", "FR-931-AC-2")]
fn a_timed_clause_lowers_to_no_formula() {
    let timed = timed_low();
    let (code, value) = run(&timed, true, ("tl_lowering", json!({"clause": "Low"})));
    assert_eq!(code, 21, "{value}");
    assert_eq!(
        value["tl_lowering"]["disposition"], "unsupported",
        "{value}"
    );
    assert!(value["tl_lowering"].get("classified").is_none(), "{value}");
    assert!(value["tl_lowering"].get("formula").is_none(), "{value}");
}

/// FR-931-AC-3: every malformed `tl_lowering` member is refused, exit 20,
/// naming the member.
#[test]
#[trace("TC-2602", "FR-931-AC-3")]
fn a_malformed_tl_lowering_member_is_refused() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let lower = |member: Value| run(&unit, true, ("tl_lowering", member));
    let (code, value) = lower(json!({"clause": "Low", "correspondence": "mltl.closed-trace/v2"}));
    assert_eq!(code, 20, "{value}");
    let text = value.to_string();
    assert!(text.contains("unsupported-selection"), "{text}");
    refusal_names(&value, "tl_lowering.correspondence");
    refusal_names(&value, "mltl.closed-trace/v2");
    for (member, names) in [
        (
            json!({"clause": "Low", "correspondence": 1}),
            "tl_lowering.correspondence",
        ),
        (
            json!({"clause": "Low", "correspondence": null}),
            "tl_lowering.correspondence",
        ),
        (json!({}), "tl_lowering.clause"),
        (json!({"clause": "Low", "formula": "G p0"}), "formula"),
        (
            json!({"clause": "Low", "cross_evaluate": {"trace": {"finite": []},
                "initial": "spine-snapshot.json", "max_steps": 1}}),
            "tl_lowering.cross_evaluate",
        ),
        (
            json!({"clause": "Low", "cross_evaluate": {}}),
            "tl_lowering.cross_evaluate",
        ),
        (
            json!({"clause": "Low", "cross_evaluate": {"initial": "spine-snapshot.json",
                "max_steps": 0}}),
            "max_steps",
        ),
        (
            json!({"clause": "Low", "cross_evaluate": {"trace": {"lasso": {"prefix": [],
                "loop": ["spine-snapshot.json"]}}}}),
            "tl_lowering.cross_evaluate.trace",
        ),
    ] {
        let (code, value) = lower(member.clone());
        assert_eq!(code, 20, "{member}: {value}");
        assert!(
            value.to_string().contains("invalid-request"),
            "{member}: {value}"
        );
        refusal_names(&value, names);
    }
    let (code, value) = lower(json!({"clause": "Nothing"}));
    assert_eq!(code, 20, "{value}");
    assert!(value.to_string().contains("missing-name"), "{value}");
    refusal_names(&value, "tl_lowering.clause");
}

/// FR-931-AC-4: cross-evaluation agrees on the spine's behaviours.
#[test]
#[trace("TC-2603", "FR-931-AC-4")]
#[ignore = "QSL-2147: cross_evaluate.initial (behaviour enumeration from a snapshot) is not implemented yet"]
fn tl_cross_evaluation_agrees_on_the_spine_behaviours() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let (code, value) = run(
        &unit,
        true,
        (
            "tl_lowering",
            json!({"clause": "Low",
                "cross_evaluate": {"initial": "spine-snapshot.json", "max_steps": 4}}),
        ),
    );
    assert_eq!(code, 0, "{value}");
    let cross = &value["tl_lowering"]["cross_evaluation"];
    let traces = cross["traces"].as_u64().expect("a trace count");
    assert!(traces >= 1, "{value}");
    assert!(cross["positions"].as_u64().expect("a position count") >= traces);
    assert_eq!(cross["agree"], true);
    assert_eq!(cross["faults"], json!([]));
}

/// FR-931-AC-5: only `fsl-command` names the TL crates.
#[test]
#[trace("TC-2603", "FR-931-AC-5")]
fn only_fsl_command_names_the_tl_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = |crate_dir: &str| {
        std::fs::read_to_string(root.join(crate_dir).join("Cargo.toml")).expect("a manifest")
    };
    assert!(
        manifest("fsl-command").contains("tl-mltl"),
        "fsl-command holds the TL edge"
    );
    for other in [
        ".",
        "fsl-foundation",
        "fsl-cst",
        "fsl-source",
        "fsl-forms",
        "fsl-semantics",
        "fsl-package",
        "fsl-route",
        "fsl-eval",
        "fsl-replay",
        "fsl-analyze",
        "fsl-inspect",
    ] {
        let text = manifest(other);
        assert!(
            !text.contains("tl-syntax") && !text.contains("tl-mltl"),
            "{other} names no TL crate"
        );
    }
}

/// FR-931-AC-6: `cross_evaluate.initial` evaluates every `over` instance of
/// the snapshot's universe and reads each behaviour of 1 to `max_steps`
/// steps as its own closed trace, so the trace count grows with `max_steps`.
#[test]
#[trace("TC-2961", "FR-931-AC-6")]
#[ignore = "QSL-2147: cross_evaluate.initial instance enumeration and closed-trace reading are not implemented yet"]
fn tl_cross_evaluation_reads_each_behaviour_as_a_closed_trace() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let traces = |max_steps: u64| {
        let (code, value) = run(
            &unit,
            true,
            (
                "tl_lowering",
                json!({"clause": "Low",
                    "cross_evaluate": {"initial": "spine-snapshot.json", "max_steps": max_steps}}),
            ),
        );
        assert_eq!(code, 0, "{value}");
        let cross = &value["tl_lowering"]["cross_evaluation"];
        assert_eq!(cross["agree"], true, "{value}");
        cross["traces"].as_u64().expect("a trace count")
    };
    let one = traces(1);
    let two = traces(2);
    assert!(one >= 1);
    assert!(
        two > one,
        "the 1-step behaviours are traces of their own beside the 2-step ones"
    );
}

/// FR-931-AC-7 (negative): `over` beside `initial` refuses naming it.
#[test]
#[trace("TC-2961", "FR-931-AC-7")]
#[ignore = "QSL-2147: cross_evaluate.initial refusals are not implemented yet"]
fn tl_cross_evaluation_refuses_over_beside_initial() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let (code, value) = run(
        &unit,
        true,
        (
            "tl_lowering",
            json!({"clause": "Low", "cross_evaluate": {"initial": "spine-snapshot.json",
                "max_steps": 2, "over": {"reference": {"population": "P", "key": "c1"}}}}),
        ),
    );
    assert_eq!(code, 20, "{value}");
    refusal_names(&value, "tl_lowering.cross_evaluate.over");
}
