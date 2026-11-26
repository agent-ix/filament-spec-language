// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface stubs for model-based testing (FR-752), the planner
//! (FR-755), TLA+ and Lean export
//! (FR-756), the TL lowering (FR-757) and the layering CI gate (FR-345).
//! Each feeds source and a `native-run/1` request through the command
//! adapter (`fsl_command::command::run`) and asserts on the output document.
//! Separately staged extensions retain named ignores.

use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

// The domain document supplies the operation frame; the authored spine
// postcondition constrains that frame's possible post-states.
// The postcondition needs both object navigation and Integer conversion;
// its model layer includes graph and value forms. The plan uses temporal v.
const PLAN_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.temporal.infinite-trace/v1\";\n\
     profile g = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n\
     post VersionAdvances using g on Config::ConfigVersion::attemptUpdate {\n\
       (result implies convert<Integer>(self.versionNumber) = pre(self.versionNumber) + 1)\n\
       and (not result implies self.versionNumber = pre(self.versionNumber)) }\n";

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
    run_with(unit, with_model, member, None)
}

/// [`run`] over the model document `model` in place of the spine model when
/// one is given.
fn run_with(
    unit: &str,
    with_model: bool,
    member: (&str, Value),
    model: Option<&[u8]>,
) -> (u8, Value) {
    run_with_model(unit, with_model, vec![member], model)
}

/// [`run`] with several request members at once.
fn run_with_members(unit: &str, with_model: bool, members: Vec<(&str, Value)>) -> (u8, Value) {
    run_with_model(unit, with_model, members, None)
}

fn run_with_model(
    unit: &str,
    with_model: bool,
    members: Vec<(&str, Value)>,
    model: Option<&[u8]>,
) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    if let Some(model) = model {
        std::fs::write(directory.path().join("spine-model.semantic-ir.json"), model).unwrap();
    }
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let models = if with_model {
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}])
    } else {
        json!([])
    };
    let mut request = json!({"models": models, "program": {"source": source("unit.native")}});
    for (name, member) in members {
        request[name] = member;
    }
    if request["model_based_test"]["initial"].is_string() {
        let snapshot_path = directory.path().join("spine-snapshot.json");
        let bytes = std::fs::read(&snapshot_path).expect("the spine snapshot exists");
        let mut snapshot: Value = serde_json::from_slice(&bytes).expect("a JSON snapshot");
        // Model runs and plan searches start at initialization, rather than
        // the handler observation used by the spine's invariant walkthrough.
        snapshot["anchor"] = json!({"kind": "initialization", "name": "start"});
        std::fs::write(
            snapshot_path,
            serde_json::to_vec(&snapshot).expect("the snapshot serializes"),
        )
        .expect("the initial snapshot is written");
    }
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

/// Run with the explicitly named, authored four-value planning model.
fn run_plan(unit: &str, mut synthesis: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    crate::support::plan_model::write(directory.path());
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    synthesis["initial"] = json!("planning-initial.json");
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0", "source": source("planning-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")}, "synthesis": synthesis
        }}),
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

/// A state clause false at every state is refuted by a 1-minimal
/// counterexample of zero invocations, replayed natively.
#[test]
fn a_failing_clause_retains_a_one_minimal_replayed_counterexample() {
    let unit = format!(
        "{MODEL_HEADER}invariant Never using v on Config::ConfigVersion at current {{ false }}\n"
    );
    let (code, value) = run(
        &unit,
        true,
        (
            "model_based_test",
            json!({
        "clause": "Never", "seed": 7, "max_cases": 100, "max_trace_length": 8,
        "initial": "spine-snapshot.json"}),
        ),
    );
    assert_eq!(code, 10, "{value}");
    let result = &value["model_based_test"];
    assert_eq!(result["status"], "refuted", "{value}");
    let counterexample = &result["counterexample"];
    assert_eq!(counterexample["minimality"], "1-minimal", "{value}");
    assert_eq!(counterexample["replay"], "reproduced", "{value}");
    assert_eq!(counterexample["invocations"], json!([]), "{value}");
}

/// A false initial state remains a replayable 1-minimal counterexample for a
/// fixed set of seeds.
#[test]
fn false_initial_state_remains_one_minimal_across_seeds() {
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
        let result = &value["model_based_test"];
        assert_eq!(result["clause"], "Never", "{value}");
        assert_eq!(result["status"], "refuted", "{value}");
        assert_eq!(
            result["counterexample"]["minimality"], "1-minimal",
            "{value}"
        );
        assert_eq!(result["counterexample"]["replay"], "reproduced", "{value}");
        assert_eq!(
            result["counterexample"]["disposition"], "invariant-violated",
            "{value}"
        );
        assert_eq!(
            result["counterexample"]["invocations"],
            json!([]),
            "{value}"
        );
    }
}

/// A direct request for a clause that every case passes yields `tested`
/// evidence, never `proved`, within `max_cases`.
#[test]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-7")]
fn a_passing_run_records_tested_and_never_proved() {
    let unit = format!(
        "{MODEL_HEADER}invariant Always using v on Config::ConfigVersion at current {{ true }}\n"
    );
    let mut domain_package: Value =
        serde_json::from_str(spine::DOMAIN_PACKAGE).expect("the shared model is JSON");
    let version_number = domain_package["types"]
        .as_array_mut()
        .expect("the model has types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/VersionNumber")
        .expect("the model declares VersionNumber");
    let max = version_number["constraints"]
        .as_array_mut()
        .expect("VersionNumber has constraints")
        .iter_mut()
        .find(|constraint| constraint["keyword"] == "max")
        .expect("VersionNumber has a maximum");
    max["operands"]["value"] = json!("2");
    let domain_package = serde_json::to_string(&domain_package).expect("the model serializes");
    let (code, value) = run_with(
        &unit,
        true,
        (
            "model_based_test",
            json!({
        "clause": "Always", "seed": 7, "max_cases": 50, "max_trace_length": 4,
        "initial": "spine-snapshot.json"}),
        ),
        Some(domain_package.as_bytes()),
    );
    assert_eq!(code, 0, "{value}");
    let result = &value["model_based_test"];
    assert_eq!(result["evidence"], "tested", "{value}");
    assert_eq!(result["status"], "tested", "{value}");
    assert!(result["verdict"].is_null(), "{value}");
    assert!(
        result["cases"]["generated"]
            .as_u64()
            .is_some_and(|n| (1..=50).contains(&n)),
        "{value}"
    );
    assert!(
        result["cases"]["passed"].as_u64().is_some_and(|n| n > 0),
        "{value}"
    );
}

// ---- FR-755: planner ---------------------------------------------------

/// FR-1010-AC-3 and FSpec FR-805-AC-6: minimizing length to reach
/// `length >= 1` gives an `optimal` one-step plan whose objective value is
/// `{"type": "integer", "value": "1"}`; plan lengths `Int[0, 0]` make the goal `infeasible`
/// with a closure certificate and every no-plan member present and `null`.
#[test]
#[trace("FR-1010")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-6")]
fn the_planner_returns_optimal_and_infeasible_with_certificates() {
    let unit = |lengths: &str| {
        format!(
            "{PLAN_HEADER}type Lengths = {lengths};\n\
             synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
             satisfies {{ Reach.length >= 1 }};\n"
        )
    };
    let (_, value) = run_plan(
        &unit("Int[0, 3]"),
        json!({
        "declaration": "Reach", "initial": "spine-snapshot.json"}),
    );
    let result = &value["synthesis"];
    assert_eq!(result["status"], "optimal", "{value}");
    assert_eq!(
        result["objective"]["value"],
        json!({"type": "integer", "value": "1"}),
        "{value}"
    );
    assert_eq!(result["certificate"]["type"], "potential", "{value}");
    assert_eq!(result["certification"], "certified", "{value}");
    let (_, value) = run_plan(
        &unit("Int[0, 0]"),
        json!({
        "declaration": "Reach", "initial": "spine-snapshot.json"}),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert_eq!(
        value["synthesis"]["certificate"]["type"], "closure",
        "{value}"
    );
    let infeasible = &value["synthesis"];
    for member in ["plan", "replay", "bound"] {
        assert!(
            infeasible.get(member).is_some_and(Value::is_null),
            "{member}: {value}"
        );
    }
    assert!(infeasible["objective"]["value"].is_null(), "{value}");
    assert_eq!(infeasible["objective"]["expression"], "length", "{value}");
}

// ---- FR-756: TLA+ and Lean export --------------------------------------

/// FR-756-AC-1: each profile echoes its id and family and classifies the
/// counter's state clause `value <= 5` `preserved`; two exports are byte-equal.
#[test]
#[trace("TC-2106", "FR-756-AC-1")]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-1")]
fn tla_and_lean_exports_echo_their_profile_and_preserve_a_finite_invariant() {
    for (profile, family) in [
        ("filament.output.tla2/v1", "tla"),
        ("filament.output.lean4/v1", "lean"),
    ] {
        let export = || {
            crate::plan_export_surface::counter_export(
                json!({"profile": profile, "obligations": ["Bounded"]}),
            )
        };
        let (_, first) = export();
        let (_, second) = export();
        assert_eq!(first, second, "deterministic");
        assert_eq!(first["export"]["mapping_profile_id"], profile, "{first}");
        assert_eq!(first["export"]["target_family"], family, "{first}");
        assert_eq!(
            first["export"]["records"][0]["obligation"], "Bounded",
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
    assert_eq!(
        lowered["atoms"]["p0"], "holds(c.versionNumber <= 2)",
        "{value}"
    );
}

// ---- FR-345: the layering gate in CI -----------------------------------

/// FR-345-AC-15: `CI_CHECKS` lists `refinement-layering`, whose recipe runs
/// `layering-check`, the check that passes only on the CLI's exit 21 run.
#[test]
#[trace("TC-2108", "FR-345-AC-15")]
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
        makefile.contains("refinement generate-layering")
            && makefile.contains("refinement layering-check")
            && makefile.contains("trap 'rm -rf \"$$corpus\"' EXIT"),
        "the recipe generates the corpus, checks it, and removes the temporary directory"
    );
    // The check accepts no exit other than 21: 0 and 10 are rejected.
    for code in [0, 10] {
        assert!(xtask::refinement::layering_check::expected_run(Some(code), b"{}").is_err());
    }
}

// ---- Fix round 1: coverage of the remaining criteria ------------------

const VALUE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n";

fn refusal_names(value: &Value, member: &str) {
    assert!(
        value.to_string().contains(member),
        "names {member}: {value}"
    );
}

const RELEASE_PLAN: &str = "invariant Never using v on Config::ConfigVersion at current { false }\n\
     invariant Ready using v on Config::ConfigVersion at current { true }\n\
     verify Release using v {\n\
       check safe claim Never method model_based_test domain Config::ConfigVersion bound 20;\n\
       check tested claim Ready method model_based_test domain Config::ConfigVersion bound 20;\n\
       check timely claim Ready method model_based_test domain Config::ConfigVersion depends [safe];\n\
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
            json!({"plan": "Release", "emit": true, "execute": false}),
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
#[trace("TC-3018", "FR-1038-AC-1")]
fn executing_a_plan_blocks_only_descendants_of_a_failed_step() {
    // FSpec FR-824: `initial` is the state each consuming step starts from,
    // so it is a snapshot anchored at an initialization (FR-120).
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{MODEL_HEADER}{RELEASE_PLAN}"),
    )
    .unwrap();
    let initial = snapshot_of(directory.path(), &["c1"], true);
    std::fs::write(
        directory.path().join("initial.json"),
        serde_json::to_vec(&initial).unwrap(),
    )
    .unwrap();
    let (exit, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "verification_plan": {"plan": "Release", "execute": true,
                "initial": "initial.json", "seed": 7, "max_trace_length": 8}}}),
    );
    let nodes = &value["verification_plan"]["dispositions"];
    assert_eq!(nodes["safe"]["status"], "refuted", "{value}");
    assert_eq!(nodes["tested"]["status"], "tested", "{value}");
    assert_eq!(nodes["tested"]["evidence"], "tested", "{value}");
    assert_eq!(nodes["timely"]["status"], "blocked", "{value}");
    assert_eq!(nodes["timely"]["blocked_by"], "safe", "{value}");
    assert_eq!(nodes.as_object().map(|n| n.len()), Some(3), "{value}");
    let outcome = &value["verification_plan"];
    assert_eq!(
        outcome["format"], "filament.verification-plan-outcome/v1",
        "{value}"
    );
    assert_eq!(outcome["failed_steps"], json!(["safe"]), "{value}");
    assert_eq!(
        outcome["inputs"],
        json!({"initial": "initial.json", "seed": 7, "max_trace_length": 8,
            "max_value_bits": null}),
        "{value}"
    );
    assert_eq!(exit, 22, "{value}");
}

/// FSpec FR-824-AC-1's request, with no `initial`: no step settles `failed` for
/// want of an initial state. FSpec expects `tested` to run from every state
/// its domain admits; the FR-752 entry cannot enumerate them yet.
#[test]
fn a_step_consuming_initial_with_none_is_unsupported_never_failed() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{MODEL_HEADER}{RELEASE_PLAN}"),
    )
    .unwrap();
    let (exit, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "verification_plan": {"plan": "Release", "seed": 7, "max_trace_length": 8}}}),
    );
    let outcome = &value["verification_plan"];
    // The FR-752 entry cannot enumerate a domain's initial states, so the step
    // is `unsupported` with its cause stated, and no step is `failed`.
    let tested = &outcome["dispositions"]["tested"];
    assert_eq!(tested["status"], "unsupported", "{value}");
    assert_eq!(
        tested["result"]["refusal"],
        json!({"code": "unsupported_projection",
            "cause": "unsupported-requested-capability", "member": "initial"}),
        "{value}"
    );
    assert_eq!(outcome["failed_steps"], json!([]), "{value}");
    assert_ne!(exit, 30, "{value}");
    assert_eq!(
        outcome["inputs"],
        json!({"initial": null, "seed": 7, "max_trace_length": 8, "max_value_bits": null}),
        "{value}"
    );
}

/// The `details.path` of a refused run's document.
fn refusal_path(value: &Value) -> &str {
    value["details"]["path"]
        .as_str()
        .unwrap_or_else(|| panic!("a path: {value}"))
}

/// FR-1038-AC-2, FSpec FR-824-AC-3: the strict reader refuses a malformed
/// `verification_plan` member at its exact pointer before any stage runs, so
/// no outcome is written and the run exits `20`.
#[test]
#[trace("TC-3018", "FR-1038-AC-2")]
#[trace("TC-3032", "FR-1038-AC-3")]
#[trace("FSpec-TC-2039", "FSpec-FR-824-AC-3")]
#[trace("FSpec-TC-2046", "FSpec-FR-824-AC-6")]
fn the_verification_plan_reader_refuses_a_malformed_member_at_its_pointer() {
    let unit = format!("{MODEL_HEADER}{RELEASE_PLAN}");
    for (member, cause, pointer) in [
        (
            json!({"plan": "Release", "seed": "7"}),
            "wrong-value-kind",
            "/request/verification_plan/seed",
        ),
        (
            json!({"plan": "Release", "seed": -1}),
            "invalid-value",
            "/request/verification_plan/seed",
        ),
        (
            json!({"plan": "Release", "seed": 7.5}),
            "invalid-value",
            "/request/verification_plan/seed",
        ),
        (
            serde_json::from_str(r#"{"plan": "Release", "seed": 18446744073709551616}"#).unwrap(),
            "invalid-value",
            "/request/verification_plan/seed",
        ),
        (
            json!({"plan": "Release", "emit": "yes"}),
            "wrong-value-kind",
            "/request/verification_plan/emit",
        ),
        (
            json!({"plan": "Release", "emit": false, "execute": false}),
            "invalid-value",
            "/request/verification_plan/execute",
        ),
        (
            json!({"plan": "Release", "engine": "x"}),
            "unknown-member",
            "/request/verification_plan/engine",
        ),
        (
            json!({"plan": "Release", "document": {"file": "p.json"}}),
            "unknown-member",
            "/request/verification_plan/document",
        ),
        (json!([]), "wrong-value-kind", "/request/verification_plan"),
        (
            json!(null),
            "wrong-value-kind",
            "/request/verification_plan",
        ),
    ] {
        let (exit, value) = run(&unit, true, ("verification_plan", member));
        assert_eq!(exit, 20, "{value}");
        assert_eq!(refusal_path(&value), pointer, "{value}");
        refusal_names(&value, cause);
        assert!(
            value.get("verification_plan").is_none(),
            "no outcome: {value}"
        );
    }
}

/// FR-1038-AC-3, FSpec FR-824-AC-6: a `null` member inside the object is
/// absent, so the request passes the reader and reaches the stages after it
/// (which refuse the unimplemented member as before, not at its kind).
#[test]
#[trace("TC-3032", "FR-1038-AC-3")]
#[trace("FSpec-TC-2046", "FSpec-FR-824-AC-6")]
fn a_null_member_inside_the_verification_plan_object_is_absent() {
    let unit = format!("{MODEL_HEADER}{RELEASE_PLAN}");
    let (_, value) = run(
        &unit,
        true,
        (
            "verification_plan",
            json!({"plan": "Release", "seed": null, "emit": null, "initial": null,
                "max_value_bits": null, "max_trace_length": 8}),
        ),
    );
    let path = value["details"]["path"].as_str().unwrap_or_default();
    assert!(
        !path.starts_with("/request/verification_plan/"),
        "the reader refused a null member: {value}"
    );
    assert_eq!(
        value["verification_plan"]["inputs"],
        json!({"initial": null, "seed": 0, "max_trace_length": 8, "max_value_bits": null}),
        "{value}"
    );
}

/// FR-1038-AC-2: the plan is selected by declared name, an omitted name runs
/// the only plan, and a bad selection refuses at
/// `/request/verification_plan/plan` with no outcome. The supplied
/// initialization snapshot provides the finite universe its state subject
/// requires (FR-120): one object over the narrowed domain, because an
/// expansion holds one successor per joint valuation of every object's
/// modified field.
#[test]
#[trace("TC-3018", "FR-1038-AC-2")]
fn the_verification_plan_member_selects_one_plan_or_refuses() {
    let one = format!("{MODEL_HEADER}{RELEASE_PLAN}");
    let directory = bump_directory(&one, &["c1"], true);
    let (_, single_plan_value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "verification_plan": {"initial": "s.json", "max_trace_length": 8}}}),
    );
    assert_eq!(
        single_plan_value["verification_plan"]["plan"], "Release",
        "{single_plan_value}"
    );
    assert_eq!(
        single_plan_value["verification_plan"]["format"], "filament.verification-plan-outcome/v1",
        "{single_plan_value}"
    );
    assert_eq!(
        single_plan_value["verification_plan"]["dispositions"]["safe"]["status"], "refuted",
        "{single_plan_value}"
    );
    assert_eq!(
        single_plan_value["verification_plan"]["inputs"]["max_trace_length"], 8,
        "{single_plan_value}"
    );
    let (_, value) = run(
        &one,
        true,
        ("verification_plan", json!({"emit": true, "execute": false})),
    );
    assert_eq!(value["verification_plan"]["plan"], "Release", "{value}");
    let two = format!(
        "{one}verify Other using v {{\n\
           check sole claim Ready method model_based_test domain Config::ConfigVersion bound 5;\n\
         }}\n"
    );
    for (unit, member, code, cause) in [
        (
            two.as_str(),
            json!({}),
            "invalid_runtime_input",
            "missing-member",
        ),
        (
            one.as_str(),
            json!({"plan": "Nope"}),
            "missing_declaration",
            "missing-name",
        ),
        (
            one.as_str(),
            json!({"plan": "Never"}),
            "missing_declaration",
            "missing-name",
        ),
    ] {
        let (exit, value) = run(unit, true, ("verification_plan", member));
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], code, "{value}");
        refusal_names(&value, cause);
        assert_eq!(
            refusal_path(&value),
            "/request/verification_plan/plan",
            "{value}"
        );
        assert!(
            value.get("verification_plan").is_none(),
            "no outcome: {value}"
        );
    }
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
#[trace("TC-3017", "FR-1037-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-6")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
fn pick_finds_validates_exhausts_and_optimizes() {
    let synth = |unit: String| run(&unit, false, ("synthesis", json!({"declaration": "Pick"}))).1;
    let found = synth(pick("Pick * Pick = 49", ""));
    assert_eq!(found["synthesis"]["status"], "candidate", "{found}");
    assert_eq!(
        found["synthesis"]["candidate"],
        json!({"type": "integer", "value": "7"}),
        "{found}"
    );
    assert_eq!(found["synthesis"]["check"], "validated", "{found}");
    let none = synth(pick("Pick * Pick = 50", ""));
    assert_eq!(none["synthesis"]["status"], "no-candidate", "{none}");
    assert_eq!(none["synthesis"]["explored"]["visited"], 10, "{none}");
    let optimal = synth(pick("Pick >= 3", "minimize Pick"));
    assert_eq!(
        optimal["synthesis"]["candidate"],
        json!({"type": "integer", "value": "3"}),
        "{optimal}"
    );
    assert_eq!(optimal["synthesis"]["optimal"], true, "{optimal}");
    assert_eq!(optimal["synthesis"]["explored"]["visited"], 4, "{optimal}");
    let maximal = synth(pick("Pick >= 3", "maximize Pick"));
    assert_eq!(
        maximal["synthesis"]["candidate"],
        json!({"type": "integer", "value": "9"}),
        "{maximal}"
    );
    assert_eq!(maximal["synthesis"]["optimal"], true, "{maximal}");
}

/// FR-1037: a plan synthesis selected through the `synthesis` member runs
/// the plan search and writes its `filament.plan-outcome/v1` document; the
/// value enumerator never reports a verdict for it.
#[test]
#[trace("TC-3017", "FR-1037-AC-1")]
fn a_plan_synthesis_runs_the_plan_search_not_the_enumerator() {
    let unit = format!(
        "{PLAN_HEADER}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach.length >= 1 }};\n"
    );
    let (exit, value) = run_plan(
        &unit,
        json!({"declaration": "Reach", "initial": "spine-snapshot.json"}),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["format"], "filament.plan-outcome/v1", "{value}");
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(exit, 0, "{value}");
    assert!(outcome.get("candidates").is_none(), "{value}");
}

/// FSpec FR-194-AC-10: a search stopped by its work limit is `incomplete`
/// naming the limit, never `no-candidate`.
#[test]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-10")]
fn a_work_limited_synthesis_is_incomplete() {
    let (_, value) = run(
        &pick("Pick * Pick = 50", ""),
        false,
        (
            "synthesis",
            json!({"declaration": "Pick", "limits": {"max_work": 1}}),
        ),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
    refusal_names(&value["synthesis"]["cause"], "synthesis.max_work");
}

fn reach(objective: &str, policy: Value) -> Value {
    let unit = format!(
        "{PLAN_HEADER}type Lengths = Int[0, 3];\n\
         temporal NotOne using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ always holds(c.versionNumber != 1) }}\n\
         temporal SomeTime using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 3) }}\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         {objective} satisfies {{ Reach.length >= 1 }};\n"
    );
    run_plan(
        &unit,
        json!({"declaration": "Reach",
        "policy": policy}),
    )
    .1
}

/// FR-755-AC-3 and FSpec FR-650-AC-4: under the policy `NotOne` the
/// replayed plan never visits version 1; a policy naming no clause refuses
/// `missing-name`; an `eventually` policy is evaluated, not refused.
#[test]
#[trace("TC-2105", "FR-755-AC-3")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-4")]
#[ignore = "QSL-2874: positive-length canonical policy monitors remain required after wiring and lambda"]
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
fn the_planner_minimizes_a_per_step_cost_sum() {
    let value = reach("minimize Reach.steps.sum(s => 2)", json!([]));
    assert_eq!(value["synthesis"]["status"], "optimal", "{value}");
    assert_eq!(
        value["synthesis"]["objective"]["value"],
        json!({"type": "integer", "value": "2"}),
        "{value}"
    );
    assert_eq!(
        value["synthesis"]["certificate"]["type"], "potential",
        "{value}"
    );
}

/// FSpec FR-650-AC-3: plan lengths admitting no goal give `infeasible`; a
/// run stopped by its work limit is `incomplete` naming it.
#[test]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-3")]
fn the_planner_reports_infeasible_and_incomplete() {
    let unit = format!(
        "{PLAN_HEADER}type Lengths = Int[0, 0];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach.length >= 1 }};\n"
    );
    let (_, value) = run_plan(
        &unit,
        json!({"declaration": "Reach",
        "initial": "spine-snapshot.json"}),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    let wide = unit.replace("Int[0, 0]", "Int[0, 3]");
    let (_, value) = run_plan(
        &wide,
        json!({"declaration": "Reach",
        "initial": "spine-snapshot.json", "limits": {"max_work": 1}}),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
}

/// FR-755-AC-4 and FSpec FR-650-AC-7: the planner's test seams are library
/// parameters, never request members, so a `synthesis` member naming one
/// refuses `unknown-member` at its `/synthesis` pointer before any search.
/// The replay failure itself runs through the plan substitution seam in
/// `plan_export_surface.rs`.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-7")]
fn a_test_seam_named_on_the_wire_is_an_unknown_member() {
    let unit = format!(
        "{PLAN_HEADER}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach.length >= 1 }};\n"
    );
    let (exit, value) = run(
        &unit,
        true,
        (
            "synthesis",
            json!({"declaration": "Reach",
        "initial": "spine-snapshot.json",
        "replay_plan": [{"operation": "double"}]}),
        ),
    );
    assert_eq!(exit, 20, "{value}");
    refusal_names(&value, "unknown-member");
    refusal_names(&value, "/synthesis/replay_plan");
    assert!(value["synthesis"].get("status").is_none(), "{value}");
}

/// FSpec FR-651-AC-2, AC-4 and FR-297-AC-5: the Lean export preserves the
/// invariant; a non-obligation member is refused; a cross-family selection
/// refuses with no target output.
#[test]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-2")]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-4")]
#[trace("FSpec-TC-1604", "FSpec-FR-297-AC-5")]
#[ignore = "QSL-2873: Lean profile, obligation refusal and family check not implemented yet"]
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
/// counter's TLA+ export equal FSL's exploration of the same model, six
/// states with `Bounded` holding; the export reports no native count.
#[test]
#[trace("FSpec-TC-1604", "FSpec-FR-651-AC-5")]
#[ignore = "QSL-2873: TLA+ export conformance with an observer not implemented yet"]
fn tla_export_agrees_with_fsl_exploration() {
    let (_, value) = crate::plan_export_surface::counter_export(json!({
        "profile": "filament.output.tla2/v1", "obligations": ["Bounded"], "observer": "tlc"}));
    let export = &value["export"];
    assert!(export.get("native").is_none(), "{value}");
    assert_eq!(export["observer"]["reachable_states"], 6, "{value}");
    assert_eq!(export["observer"]["verdict"], "holds", "{value}");
}

/// FR-757-AC-2 and AC-3: native and `tl-mltl` verdicts agree at every
/// activation position; a timed-profile clause lowers to no formula.
#[test]
#[trace("TC-2107", "FR-757-AC-2")]
#[trace("TC-2107", "FR-757-AC-3")]
fn tl_cross_evaluation_agrees_and_timed_clauses_do_not_lower() {
    // The corpus is every behaviour of up to 4 steps of FR-931-AC-6's
    // two-object `bump` snapshot: `Low` holds on some and is violated on
    // those that reach `versionNumber` 3. The spine snapshot is anchored at
    // the handler `validate`, which FR-120 refuses as an initial state.
    let (code, value) = cross_from(&bump_low(), &["a", "b"], true, 4);
    assert_eq!(code, 0, "{value}");
    let cross = &value["tl_lowering"]["cross_evaluation"];
    assert!(
        cross["traces"].as_u64().expect("a trace count") > 1,
        "{value}"
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
#[trace("TC-2102", "FR-752-AC-5")]
fn model_based_test_request_rules() {
    let unit =
        format!("{VALUE_HEADER}predicate Square using v (x: Integer): Boolean {{ x * x >= 0 }}\n");
    let mbt = |member: Value| run(&unit, false, ("model_based_test", member)).1;
    let base = json!({"clause": "Square", "seed": 7, "max_cases": 20});

    for bits in [0, 65] {
        let (code, refusal) = run(
            &unit,
            false,
            (
                "model_based_test",
                json!({"clause": "Square", "seed": 7, "max_cases": 20, "max_value_bits": bits}),
            ),
        );
        assert_eq!(code, 20, "{refusal}");
        assert_eq!(refusal["code"], "invalid_runtime_input", "{refusal}");
        assert_eq!(refusal["cause"], "invalid-value", "{refusal}");
        refusal_names(&refusal, "max_value_bits");
    }
    let missing_bits = mbt(base.clone());
    assert_eq!(
        missing_bits["code"], "invalid_runtime_input",
        "{missing_bits}"
    );
    assert_eq!(missing_bits["cause"], "missing-member", "{missing_bits}");
    refusal_names(&missing_bits, "max_value_bits");
    let mut bounded = base.clone();
    bounded["max_value_bits"] = json!(8);
    let first = mbt(bounded.clone());
    assert_eq!(
        first["model_based_test"]["provenance"]["max_value_bits"], 8,
        "{first}"
    );
    let partitions = first["model_based_test"]["partitions"]
        .as_array()
        .expect("the value partitions are reported");
    assert_eq!(
        partitions
            .iter()
            .take(5)
            .map(|partition| partition["kind"].as_str().expect("partition kind"))
            .collect::<Vec<_>>(),
        [
            "size-least",
            "size-unit",
            "size-unit",
            "size-extreme-low",
            "size-extreme-high",
        ],
        "{first}"
    );
    assert_eq!(
        partitions
            .iter()
            .take(5)
            .map(|partition| partition["value"]["value"].as_str().expect("integer value"))
            .collect::<Vec<_>>(),
        ["0", "-1", "1", "-255", "255"],
        "{first}"
    );
    assert_eq!(
        first["model_based_test"]["provenance"]["size_bounded"][0]["values"], "511",
        "{first}"
    );
    assert_eq!(first["model_based_test"]["status"], "tested", "{first}");
    assert_eq!(
        first["model_based_test"]["cases"]["generated"], 20,
        "{first}"
    );
    assert!(
        partitions.iter().all(|partition| {
            partition["value"]["value"]
                .as_str()
                .and_then(|value| value.parse::<i64>().ok())
                .is_some_and(|value| (-255..=255).contains(&value))
        }),
        "{first}"
    );
    assert!(
        partitions.iter().all(|partition| {
            !partition["kind"]
                .as_str()
                .expect("partition kind")
                .ends_with("-refusal")
        }),
        "{first}"
    );
    assert_eq!(first, mbt(bounded.clone()), "deterministic");
    let mut with_length = bounded.clone();
    with_length["max_trace_length"] = json!(3);
    let with_length = mbt(with_length);
    for member in ["status", "cases", "partitions", "counterexample"] {
        assert_eq!(
            with_length["model_based_test"][member], first["model_based_test"][member],
            "a value clause accepts max_trace_length and does not use it: {member}"
        );
    }
    for (member, value, code, cause) in [
        (
            "max_cases",
            json!(0),
            "invalid_runtime_input",
            "invalid-value",
        ),
        (
            "max_cases",
            json!(-1),
            "invalid_runtime_input",
            "invalid-value",
        ),
        (
            "max_value_bits",
            json!(256),
            "invalid_runtime_input",
            "invalid-value",
        ),
        (
            "max_value_bits",
            json!(-1),
            "invalid_runtime_input",
            "invalid-value",
        ),
        (
            "max_trace_length",
            json!(0),
            "invalid_runtime_input",
            "invalid-value",
        ),
        (
            "clause",
            json!("Nothing"),
            "invalid_runtime_input",
            "unknown-member",
        ),
    ] {
        let mut request = bounded.clone();
        request[member] = value;
        let refusal = mbt(request);
        assert_eq!(refusal["code"], code, "{member}: {refusal}");
        assert_eq!(refusal["cause"], cause, "{member}: {refusal}");
        refusal_names(&refusal, member);
    }

    let bounded_unit =
        format!("{VALUE_HEADER}predicate Small using v (x: Int[0, 5]): Boolean {{ x <= 3 }}\n");
    let (bounded_code, bounded_value) = run(
        &bounded_unit,
        false,
        (
            "model_based_test",
            json!({
                "clause": "Small",
                "seed": 7,
                "max_cases": 20,
                "max_value_bits": 8,
            }),
        ),
    );
    assert_eq!(bounded_code, 20, "{bounded_value}");
    assert_eq!(
        bounded_value["code"], "invalid_runtime_input",
        "{bounded_value}"
    );
    assert_eq!(bounded_value["cause"], "unknown-member", "{bounded_value}");

    let (code, conflict) = run_with_members(
        &unit,
        false,
        vec![
            ("tl_lowering", json!({"clause": "Nothing"})),
            ("model_based_test", base),
        ],
    );
    assert_eq!(code, 20, "{conflict}");
    assert_eq!(conflict["code"], "invalid-request", "{conflict}");
    assert_eq!(conflict["stage"], "request", "{conflict}");
    assert!(
        conflict["message"].as_str().is_some_and(|message| {
            message.contains("model_based_test")
                && message.contains("tl_lowering")
                && message.contains("mutually exclusive")
        }),
        "{conflict}"
    );
}

/// A state clause's initial snapshot holding a population marked incomplete
/// refuses `incomplete_population` naming it, as the universe snapshot of a
/// TL cross-evaluation does, rather than running over a missing universe.
#[test]
fn model_based_test_refuses_an_incomplete_initial_population() {
    let unit = format!(
        "{MODEL_HEADER}invariant Never using v on Config::ConfigVersion at current {{ false }}\n"
    );
    let directory = bump_directory(&unit, &["a"], false);
    let (code, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "model_based_test": {"clause": "Never", "seed": 1, "max_cases": 5,
                "max_trace_length": 2, "initial": "s.json"}}}),
    );
    assert_eq!(code, 22, "{value}");
    assert_eq!(value["code"], "incomplete_population", "{value}");
    refusal_names(&value, "config_history");
}

/// FR-752-AC-5: a direct value-clause request evaluates
/// generated Integer assignments with the checked package's native oracle and
/// retains a reproduced, minimized failing assignment.
#[test]
#[trace("TC-2102", "FR-752-AC-5")]
fn model_based_test_value_clause_runs_and_shrinks_natively() {
    let unit = format!(
        "{VALUE_HEADER}predicate BelowTwoHundred using v (x: Integer): Boolean {{ x < 200 }}\n"
    );
    let (code, value) = run(
        &unit,
        false,
        (
            "model_based_test",
            json!({
                "clause": "BelowTwoHundred",
                "seed": 7,
                "max_cases": 20,
                "max_value_bits": 8,
            }),
        ),
    );
    assert_eq!(code, 10, "{value}");
    let record = &value["model_based_test"];
    assert_eq!(record["status"], "refuted", "{value}");
    assert_eq!(record["cases"]["generated"], 5, "{value}");
    assert_eq!(
        record["counterexample"]["minimality"], "1-minimal",
        "{value}"
    );
    assert_eq!(record["counterexample"]["replay"], "reproduced", "{value}");
    assert_eq!(
        record["counterexample"]["assignment"][0],
        json!({"type": "integer", "value": "200"}),
        "{value}"
    );
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
    assert_eq!(
        record["atoms"]["p0"], "holds(c.versionNumber <= 2)",
        "{default}"
    );
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

/// `Low` with the source union `Shape` declared by the unit, the leaf `leaf`
/// as its body under `always [0, 3]`.
fn shape_low(leaf: &str) -> String {
    low(EVENT_POSITION, &format!("always [0, 3] {leaf}"))
        .replacen(
            "model Config",
            "profile v = \"filament.value.complete/v1\";\nmodel Config",
            1,
        )
        .replacen(
            "temporal Low",
            "union Shape { Circle(Integer), Rect(Integer, Integer), Empty }\ntemporal Low",
            1,
        )
}

/// FR-931-AC-13 (source route): a `case` prints its arms in member
/// declaration order with each binder under its declared name, nested
/// binders keep their scopes, `_` is an ordinary binder, and renaming a
/// binder changes the label. The package route is
/// `case_binders_match_across_the_source_and_package_routes`.
#[test]
#[trace("TC-3039", "FR-931-AC-13")]
fn case_binders_print_under_their_declared_names() {
    let atom = |leaf: &str| {
        let (code, value) = run(
            &shape_low(leaf),
            true,
            ("tl_lowering", json!({"clause": "Low"})),
        );
        assert_eq!(code, 0, "{value}");
        value["tl_lowering"]["atoms"]["p0"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let first = atom(
        "holds(case Shape::Rect(c.versionNumber,2) { Empty: true; Rect(w, h): w+h<=4; Circle(r): r<=2; })",
    );
    // The union's declaration key `ix://<SourceOwner identity>/Shape` (FR-931 "Names").
    let key = first
        .strip_prefix("holds(case ")
        .and_then(|rest| rest.split_once("::Rect("))
        .map(|(key, _)| key.to_owned())
        .unwrap_or_else(|| panic!("{first}"));
    assert!(
        key.starts_with("ix://") && key.ends_with("/Shape"),
        "{first}"
    );
    assert_eq!(
        first,
        format!(
            "holds(case {key}::Rect(c.versionNumber, 2) {{ Circle(r): r <= 2; \
             Rect(w, h): w + h <= 4; Empty: true; }})"
        )
    );
    let nested = atom(
        "holds(let k = c.versionNumber in case Shape::Rect(k, 1) { Circle(r): r <= k; \
         Rect(_, h): case Shape::Circle(h) { Circle(x): x + k <= 4; Rect(a, b): false; Empty: true; }; \
         Empty: true; })",
    );
    assert_eq!(
        nested,
        format!(
            "holds(let k = c.versionNumber in case {key}::Rect(k, 1) {{ Circle(r): r <= k; \
             Rect(_, h): case {key}::Circle(h) {{ Circle(x): x + k <= 4; Rect(a, b): false; \
             Empty: true; }}; Empty: true; }})"
        )
    );
    let renamed = atom(
        "holds(case Shape::Rect(c.versionNumber,2) { Empty: true; Rect(width, h): width+h<=4; Circle(r): r<=2; })",
    );
    assert!(
        renamed.contains("Rect(width, h): width + h <= 4"),
        "{renamed}"
    );
    assert_ne!(renamed, first);
}

/// The atom of `leaf` lowered on the source route.
fn source_only(leaf: &str) -> String {
    let (code, value) = run(
        &shape_low(leaf),
        true,
        ("tl_lowering", json!({"clause": "Low"})),
    );
    assert_eq!(code, 0, "{value}");
    value["tl_lowering"]["atoms"]["p0"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// A leaf over `shape_low` lowered on the
/// source route and, after compiling `shape_low(leaf)` to a checked package
/// and removing the source, on the package route (FR-1039) over the
/// handoff directory, whose identity rows carry the unit's source union. Returns each
/// leaf's source and package `atoms` entry of `p0`.
fn both_routes(leaf: &str) -> (String, String) {
    both_routes_over(leaf, json!({"directory": "handoff"}))
}

/// [`both_routes`] with the package leg selecting `package`.
fn both_routes_over(leaf: &str, package: Value) -> (String, String) {
    unit_both_routes(&shape_low(leaf), package)
}

/// [`both_routes_over`] over the whole `unit`.
fn unit_both_routes(unit: &str, package: Value) -> (String, String) {
    unit_both_routes_with(unit, package, |_| {})
}

/// FR-931-AC-19 (TC-2601): a printed atom with a let binder shadowing the
/// temporal parameter reparses with the same two binding sites.
#[test]
#[trace("TC-2601", "FR-931-AC-19")]
fn a_tl_lowering_shadowed_parameter_atom_reparses_to_same_bindings() {
    let leaf = "holds(let c = c.versionNumber in c = 2)";
    let unit = low(EVENT_POSITION, &format!("always [0, 3] {leaf}"));
    let (from_source, from_package) = unit_both_routes(&unit, json!({"directory": "handoff"}));

    assert_eq!(
        from_source, "holds(let c = c.versionNumber in c = 2)",
        "the printer preserves the shadowed binder's declared name"
    );
    assert_eq!(from_package, from_source, "both routes print the same atom");

    let reparsed_unit = low(EVENT_POSITION, &format!("always [0, 3] {from_source}"));
    let (reparsed_source, reparsed_package) =
        unit_both_routes(&reparsed_unit, json!({"directory": "handoff"}));

    assert_eq!(reparsed_source, from_source, "source route round trip");
    assert_eq!(reparsed_package, from_source, "package route round trip");
}

/// [`unit_both_routes`] with `edit` run over the compiled directory before the
/// package route reads it.
fn unit_both_routes_with(unit: &str, package: Value, edit: impl FnOnce(&Path)) -> (String, String) {
    unit_both_routes_over_model(unit, package, None, edit)
}

/// [`unit_both_routes_with`] over the model document `model` in place of the
/// spine model when one is given.
fn unit_both_routes_over_model(
    unit: &str,
    package: Value,
    model: Option<&[u8]>,
    edit: impl FnOnce(&Path),
) -> (String, String) {
    let unit = unit.to_owned();
    let lower = json!({"clause": "Low"});
    let (code, value) = run_with(&unit, true, ("tl_lowering", lower.clone()), model);
    assert_eq!(code, 0, "{value}\n{unit}");
    let from_source = value["tl_lowering"]["atoms"]["p0"]
        .as_str()
        .unwrap()
        .to_owned();

    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    if let Some(model) = model {
        std::fs::write(directory.path().join("spine-model.semantic-ir.json"), model).unwrap();
    }
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    let models =
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}]);
    let compile = directory.path().join("compile.json");
    std::fs::write(
        &compile,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": {
            "models": models, "program": {"source": source("unit.native")},
            "output": {"package": "package.json"},
            "handoff": {"directory": "handoff", "package_name": "test:s1-16:unit.native",
                "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}}}))
        .unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&compile).expect("the unit compiles");
    std::fs::remove_file(directory.path().join("unit.native")).unwrap();
    edit(directory.path());
    let (code, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {"models": models,
            "package": package, "tl_lowering": lower}}),
    );
    assert_eq!(code, 0, "{value}");
    let from_package = value["tl_lowering"]["atoms"]["p0"]
        .as_str()
        .unwrap()
        .to_owned();
    (from_source, from_package)
}

/// FR-931-AC-18, FR-1039-AC-6 (TC-2601, TC-3019): a unit's checked package
/// selected as its handoff `package` `{"directory": D}` with the source
/// removed returns the record the source route returns, byte-equal, for each
/// correspondence, and a name that is no temporal clause refuses
/// `missing_declaration` as on the source route. Over a `{file}` selection
/// that name refuses the same way and a `cross_evaluate` refuses
/// `invalid-request` with `requires: directory`.
#[test]
#[trace("TC-2601", "FR-931-AC-18")]
#[trace("TC-3019", "FR-1039-AC-6")]
fn a_tl_lowering_over_a_package_returns_the_source_record() {
    let unit = low(EVENT_POSITION, LOW_BODY);
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    let models =
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}]);
    let compile = directory.path().join("compile.json");
    std::fs::write(
        &compile,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": {
            "models": models, "program": {"source": source("unit.native")},
            "output": {"package": "package.json"},
            "handoff": {"directory": "handoff", "package_name": "test:s1-16:unit.native",
                "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}}}))
        .unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&compile).expect("the unit compiles");
    std::fs::remove_file(directory.path().join("unit.native")).unwrap();
    for lowering in [
        json!({"clause": "Low"}),
        json!({"clause": "Low", "correspondence": "mltl.online-prefix/v1"}),
        json!({"clause": "Low", "correspondence": "mltl.origin-complete-history/v1"}),
        json!({"clause": "Nothing"}),
    ] {
        let (source_code, from_source) = run(&unit, true, ("tl_lowering", lowering.clone()));
        for package in [
            json!({"directory": "handoff"}),
            json!({"file": "package.json", "equality": "handoff/equality.json"}),
        ] {
            let (code, from_package) = write_request(
                directory.path(),
                &json!({"format": "native-run/1", "request": {"models": models,
                    "package": package, "tl_lowering": lowering}}),
            );
            assert_eq!(code, source_code, "{package} {lowering}: {from_package}");
            assert_eq!(from_package, from_source, "{package} {lowering}");
        }
    }
    let (code, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {"models": models,
            "package": {"file": "package.json"}, "tl_lowering": {"clause": "Nothing"}}}),
    );
    assert_eq!(code, 20, "{value}");
    assert_eq!(value["code"], "missing_declaration", "{value}");
    assert_eq!(value["cause"], "missing-name", "{value}");
    let (code, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {"models": models,
            "package": {"file": "package.json"},
            "tl_lowering": {"clause": "Low", "cross_evaluate": {"trace": {"finite": []}}}}}),
    );
    assert_eq!(code, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    assert_eq!(
        value["details"],
        json!({"member": "tl_lowering.cross_evaluate", "requires": "directory"}),
        "{value}"
    );
    for (request, member) in [
        (
            json!({"models": models, "package": {"directory": "handoff"},
                "tl_lowering": {"clause": "Low", "formula": 1}}),
            "tl_lowering.formula",
        ),
        (
            json!({"models": models, "package": {"directory": "handoff"},
                "call": {"function": "f", "arguments": []},
                "tl_lowering": {"clause": "Low"}}),
            "tl_lowering",
        ),
    ] {
        let (code, value) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": request}),
        );
        assert_eq!(code, 20, "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(
            value["message"].as_str().unwrap().contains(member),
            "{value}"
        );
    }
}

/// Compiles the unit in `directory` to its FR-517 handoff `handoff`, with the
/// spine model selected, and removes the unit's source. Returns the request's
/// `models` member.
fn compile_handoff(directory: &Path, unit: &str) -> Value {
    compile_handoff_with_clocks(directory, unit, None)
}

fn compile_handoff_with_clocks(directory: &Path, unit: &str, clocks: Option<Value>) -> Value {
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    let models =
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}]);
    let compile = directory.join("compile.json");
    let mut request = json!({"format": "native-compile/1", "request": {
        "models": models, "program": {"source": source("unit.native")},
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": "test:s1-16:unit.native",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}}});
    if let Some(clocks) = clocks {
        request["request"]["clocks"] = clocks;
    }
    std::fs::write(&compile, serde_json::to_vec(&request).unwrap()).unwrap();
    fsl_command::command::compile(&compile).expect("the unit compiles");
    std::fs::remove_file(directory.join("unit.native")).unwrap();
    models
}

/// FR-931-AC-18, FR-931-AC-9: the lasso `cross_evaluate` request over the
/// unit's FR-517 handoff `{"directory": D}` with the source removed returns
/// the source route's record, `cross_evaluation` included, byte-equal and with
/// the same exit code. The bounded-past clauses are supported; the future
/// clauses retain the source route's `infinite-trace` unsupported dimension
/// (FR-931-AC-9). Over `{file}` and `{file, equality}` it refuses
/// `invalid-request` with `requires: directory`.
#[test]
#[trace("TC-2601", "FR-931-AC-18")]
fn a_lasso_cross_evaluation_over_a_handoff_directory_returns_the_source_record() {
    let unit = lasso_unit();
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let models = compile_handoff(directory.path(), &unit);
    for (trace, _, _) in lasso_cases() {
        for clause in ["Prev", "PrevZero", "Safe", "Hits"] {
            let lower = json!({"clause": clause, "cross_evaluate": {"trace": trace}});
            let (source_code, from_source) = run(&unit, true, ("tl_lowering", lower.clone()));
            let expected = if matches!(clause, "Safe" | "Hits") {
                21
            } else {
                0
            };
            assert_eq!(source_code, expected, "{clause} {trace}: {from_source}");
            let cross = &from_source["tl_lowering"]["cross_evaluation"];
            if expected == 0 {
                assert_eq!(cross["traces"], 1, "{from_source}");
                assert_eq!(cross["agree"], true, "{from_source}");
            } else {
                assert_eq!(
                    from_source["tl_lowering"]["dimensions"],
                    json!(["infinite-trace"])
                );
            }

            let (code, from_package) = write_request(
                directory.path(),
                &json!({"format": "native-run/1", "request": {"models": models,
                    "package": {"directory": "handoff"}, "tl_lowering": lower}}),
            );
            assert_eq!(code, source_code, "{clause} {trace}: {from_package}");
            assert_eq!(from_package, from_source, "{clause} {trace}");
        }
    }
    let lower = json!({"clause": "Prev", "cross_evaluate": {"trace": lasso_cases()[0].0}});
    for package in [
        json!({"file": "package.json"}),
        json!({"file": "package.json", "equality": "handoff/equality.json"}),
    ] {
        let (code, value) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {"models": models,
                "package": package, "tl_lowering": lower}}),
        );
        assert_eq!(code, 20, "{package}: {value}");
        assert_eq!(value["code"], "invalid-request", "{package}: {value}");
        assert_eq!(
            value["details"],
            json!({"member": "tl_lowering.cross_evaluate", "requires": "directory"}),
            "{package}: {value}"
        );
    }
}

/// FR-1039 Outputs (TC-3019): a `clause` run of a temporal clause over record
/// samples, over the unit's handoff `{"directory": D}` with the source
/// removed, returns the source route's result document, byte-equal, and the
/// verdict the lasso's slots give (`Settles` and `Recurs` over lassos whose
/// prefix and loop slots differ in value) and for finite traces; over `{file}`
/// and `{file, equality}` it refuses `invalid-request` with
/// `requires: directory`.
#[test]
#[trace("TC-3019", "FR-1039")]
fn a_clause_run_over_a_handoff_directory_returns_the_source_result() {
    let unit = lasso_unit();
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let models = compile_handoff(directory.path(), &unit);
    let finite = json!({"finite": [level_sample(1), level_sample(1), level_sample(2)]});
    let mut cases: Vec<(&str, Value, Option<u8>)> = Vec::new();
    for (trace, settles, recurs) in lasso_cases() {
        cases.push(("Settles", trace.clone(), Some(settles)));
        cases.push(("Recurs", trace, Some(recurs)));
    }
    cases.push(("Settles", finite.clone(), None));
    cases.push(("Recurs", finite, None));
    for (name, trace, expected) in &cases {
        let clause = json!({"selection": {"temporal": {"name": name, "trace": trace}},
            "observations": {}});
        let (source_code, from_source) = run(&unit, true, ("clause", clause.clone()));
        assert_ne!(source_code, 20, "{name} {trace}: {from_source}");
        if let Some(expected) = expected {
            assert_eq!(source_code, *expected, "{name} {trace}: {from_source}");
        }
        let (code, from_package) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {"models": models,
                "package": {"directory": "handoff"}, "clause": clause}}),
        );
        assert_eq!(code, source_code, "{name} {trace}: {from_package}");
        assert_eq!(from_package, from_source, "{name} {trace}");
    }
    for package in [
        json!({"file": "package.json"}),
        json!({"file": "package.json", "equality": "handoff/equality.json"}),
    ] {
        let (code, value) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {"models": models,
                "package": package,
                "clause": {"selection": {"temporal": {"name": "Settles",
                    "trace": cases[0].1}}, "observations": {}}}}),
        );
        assert_eq!(code, 20, "{package}: {value}");
        assert_eq!(value["code"], "invalid-request", "{package}: {value}");
        assert_eq!(
            value["details"],
            json!({"member": "clause", "requires": "directory"}),
            "{package}: {value}"
        );
    }
}

/// FR-1039 (TC-3019): an evaluated refusal on a lasso retains its complete
/// source disposition, including its caller-rendered region, on both routes.
#[test]
#[trace("TC-3019", "FR-1039", "FR-1039-AC-11")]
fn a_refused_condition_in_a_clause_run_has_the_same_code_and_cause_on_both_routes() {
    let unit = package_refusal_unit("filament.temporal.infinite-trace/v1");
    let clause = json!({"selection":{"temporal":{"name":"C","trace":
        {"lasso":{"prefix":[],"loop":[level_sample(1)]}}}},"observations":{}});
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let models = compile_handoff(directory.path(), &unit);
    std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
    let mut caller = source("unit.native");
    caller["file"] = json!("caller.native");
    let expected = write_request(
        directory.path(),
        &json!({"format":"native-run/1","request":{
        "models":models,"program":{"source":caller},"clause":clause}}),
    );
    assert_eq!(expected.0, 22, "{}", expected.1);
    let refusal = &expected.1["disposition"]["outcome"];
    assert_eq!(refusal["kind"], "refused", "{}", expected.1);
    assert_eq!(refusal["code"], "incomplete_population");
    assert_eq!(refusal["cause"], "missing-required-artifact");
    assert_eq!(
        refusal["fields"],
        json!({"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)})
    );
    assert_eq!(refusal["locus"]["file"], "caller.native");
    assert!(refusal["locus"]["span"].is_object());
    let actual = write_request(
        directory.path(),
        &json!({"format":"native-run/1","request":{
        "models":models,"package":{"directory":"handoff"},"sources":[caller],"clause":clause}}),
    );
    assert_eq!(
        actual, expected,
        "all refusal fields and the actual source span are retained"
    );
    fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&actual.1).unwrap()).unwrap();
}

/// FR-1039-AC-6 (FR-931-AC-6's snapshot): `cross_evaluate` `{"initial": S,
/// "max_steps": N}` over the handoff `{"directory": D}` with the source removed
/// returns the source route's record with its `cross_evaluation`, byte-equal,
/// for each step bound, and an `over` population with no object gives zero
/// traces on both.
#[test]
#[trace("TC-3019", "FR-1039-AC-6")]
fn an_initial_cross_evaluation_over_a_handoff_directory_returns_the_source_record() {
    let unit = bump_low();
    for (keys, max_steps) in [(&["a", "b"][..], 1), (&["a", "b"][..], 2), (&[][..], 2)] {
        let directory = bump_directory(&unit, keys, true);
        let lower = json!({"clause": "Low",
            "cross_evaluate": {"initial": "s.json", "max_steps": max_steps}});
        let (source_code, from_source) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {
                "models": [{"format": "semantic-ir/2.0.0",
                    "source": source("spine-model.semantic-ir.json")}],
                "program": {"source": source("unit.native")}, "tl_lowering": lower}}),
        );
        assert_eq!(source_code, 0, "{from_source}");
        let traces = from_source["tl_lowering"]["cross_evaluation"]["traces"]
            .as_u64()
            .expect("a trace count");
        assert_eq!(traces == 0, keys.is_empty(), "{from_source}");

        let models = compile_handoff(directory.path(), &unit);
        let (code, from_package) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {"models": models,
                "package": {"directory": "handoff"}, "tl_lowering": lower}}),
        );
        assert_eq!(code, source_code, "{keys:?} {max_steps}: {from_package}");
        assert_eq!(from_package, from_source, "{keys:?} {max_steps}");
        for package in [
            json!({"file": "package.json"}),
            json!({"file": "package.json", "equality": "handoff/equality.json"}),
        ] {
            let (code, value) = write_request(
                directory.path(),
                &json!({"format": "native-run/1", "request": {"models": models,
                    "package": package, "tl_lowering": lower}}),
            );
            assert_eq!(code, 20, "{package}: {value}");
            assert_eq!(value["code"], "invalid-request", "{package}: {value}");
            assert_eq!(
                value["details"],
                json!({"member": "tl_lowering.cross_evaluate", "requires": "directory"}),
                "{package}: {value}"
            );
        }
    }
}

/// FR-931 "Cross-evaluation" (package route): a request the source route
/// answers with its clause report, a sample its record type refuses, is
/// answered with the same report, byte-equal and with the same exit code,
/// over the handoff `{"directory": D}`.
#[test]
#[trace("TC-3019", "FR-1039")]
fn a_refused_sample_in_a_cross_evaluation_returns_the_source_clause_report() {
    let unit = lasso_unit();
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let models = compile_handoff(directory.path(), &unit);
    let lower = json!({"clause": "Settles",
        "cross_evaluate": {"trace": {"finite": [level_sample(1), level_sample(12)]}}});
    let (source_code, from_source) = run(&unit, true, ("tl_lowering", lower.clone()));
    assert!(from_source.get("tl_lowering").is_none(), "{from_source}");
    let (code, from_package) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {"models": models,
            "package": {"directory": "handoff"}, "tl_lowering": lower}}),
    );
    assert_eq!(code, source_code, "{from_package}\n{from_source}");
    assert_eq!(from_package, from_source);
}

/// FR-931 "Cross-evaluation" (package route): an `initial` snapshot with an
/// incomplete population refuses before the clause is resolved, as on the
/// source route, so a clause the package does not declare gives the same
/// refusal on both.
#[test]
#[trace("TC-3019", "FR-1039")]
fn an_incomplete_snapshot_refuses_before_a_missing_clause_on_both_routes() {
    let unit = bump_low();
    let directory = bump_directory(&unit, &["a"], false);
    let lower = json!({"clause": "NoSuchClause",
        "cross_evaluate": {"initial": "s.json", "max_steps": 1}});
    let (source_code, from_source) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")}, "tl_lowering": lower}}),
    );
    assert!(
        from_source.to_string().contains("incomplete"),
        "the source route refuses the population: {from_source}"
    );
    let models = compile_handoff(directory.path(), &unit);
    let (code, from_package) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {"models": models,
            "package": {"directory": "handoff"}, "tl_lowering": lower}}),
    );
    assert_eq!(code, source_code, "{from_package}\n{from_source}");
    assert_eq!(from_package, from_source);
}

/// FR-400-AC-13 and FR-931-AC-13 (package route): the reader-backed read
/// gives every `case` and `let` binding site the declared names the
/// in-process read gives, observed through the one printer that reads them,
/// so each leaf, and each leaf with its binders renamed, gives byte-equal
/// `atoms` on both routes.
#[test]
#[trace("TC-3039", "FR-400-AC-13")]
#[trace("TC-3039", "FR-931-AC-13")]
fn case_binders_match_across_the_source_and_package_routes() {
    for leaf in [
        "holds(case Shape::Rect(c.versionNumber,2) { Empty: true; Rect(w, h): w+h<=4; Circle(r): r<=2; })",
        "holds(case Shape::Rect(c.versionNumber,2) { Empty: true; Rect(width, h): width+h<=4; Circle(r): r<=2; })",
        "holds(let k = c.versionNumber in case Shape::Rect(k, 1) { Circle(r): r <= k; \
         Rect(_, h): case Shape::Circle(h) { Circle(x): x + k <= 4; Rect(a, b): false; Empty: true; }; \
         Empty: true; })",
        "holds(let kk = c.versionNumber in case Shape::Rect(kk, 1) { Circle(rr): rr <= kk; \
         Rect(_, hh): case Shape::Circle(hh) { Circle(xx): xx + kk <= 4; Rect(aa, bb): false; Empty: true; }; \
         Empty: true; })",
    ] {
        let (from_source, from_package) = both_routes(leaf);
        assert_eq!(from_source, from_package, "{leaf}");
    }
}

/// FR-931-AC-18 and FR-1039 (`{file}` and `{file, equality}` forms): the atom
/// text of a clause that reads a model object and a source union is built
/// from the package alone, so each selection returns the source route's atom.
#[test]
#[trace("TC-3019", "FR-1039-AC-6")]
fn the_file_form_prints_the_source_routes_atoms() {
    let leaf = "holds(case Shape::Rect(c.versionNumber,2) { Empty: true; Rect(w, h): w+h<=4; Circle(r): r<=2; })";
    for package in [
        json!({"file": "package.json"}),
        json!({"file": "package.json", "equality": "handoff/equality.json"}),
    ] {
        let (from_source, from_package) = both_routes_over(leaf, package.clone());
        assert_eq!(from_source, from_package, "{package}");
    }
}

/// The unit of `a_reading_clause_with_a_capture_lowers_alike_on_both_routes`:
/// an `on each` clause with one capture, an IEEE ordering over a non-local
/// operand written in a `holds` condition and in a pure function it calls.
const READING_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile t = \"filament.temporal.infinite-trace/v1\";\n\
     record Reading { level: Int[0, 9]; stopped: Boolean; }\n\
     function below using v(x: Reading): Boolean pure \
     { -float64(bits: 0x3ff0000000000000) < float64(bits: 0x0000000000000000) }\n\
     temporal Direct using t over (r: Reading) clock \"steps\" \
     on each (q: Reading) when (r.level >= 0 and q.level >= 0) \
     { capture start: Int[0, 9] = q.level; \
     once [1, 1] holds(-float64(bits: 0x3ff0000000000000) < float64(bits: 0x0000000000000000) \
     and r.level >= start) }\n\
     temporal Called using t over (r: Reading) clock \"steps\" \
     on each (q: Reading) { capture start: Int[0, 9] = q.level; \
     once [1, 1] holds(below(r) and r.level >= start) }\n";

/// FR-931-AC-14, FR-397-AC-29: the checker numbers an IEEE operand binder
/// after every binder its term is typed over, so a `holds` condition of an
/// `on each` clause with a capture, and a function body it calls (numbered
/// from its own parameters), lower to the same record on both routes.
#[test]
#[trace("TC-3039", "FR-931-AC-14")]
fn a_reading_clause_with_a_capture_lowers_alike_on_both_routes() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("unit.native"), READING_UNIT).unwrap();
    let compile = directory.path().join("compile.json");
    std::fs::write(
        &compile,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": {
            "models": [], "program": {"source": source("unit.native")},
            "output": {"package": "package.json"},
            "handoff": {"directory": "handoff", "package_name": "test:s1-16:unit.native",
                "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}}}))
        .unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&compile).expect("the unit compiles");
    std::fs::remove_file(directory.path().join("unit.native")).unwrap();
    for clause in ["Direct", "Called"] {
        let lower = json!({"clause": clause, "correspondence": "mltl.infinite-trace/v1"});
        let (source_code, from_source) = run(READING_UNIT, false, ("tl_lowering", lower.clone()));
        let (code, from_package) = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {"models": [],
                "package": {"directory": "handoff"}, "tl_lowering": lower}}),
        );
        assert_eq!(source_code, 0, "{clause}: {from_source}");
        assert_eq!(code, 0, "{clause}: {from_package}");
        assert_eq!(from_package, from_source, "{clause}");
    }
}

/// FR-931-AC-14: the checker's own `#ieee_operand` binders of an IEEE
/// ordering never print. With one bound operand the composition prints
/// lesser-left; with two it keeps its source orientation. An author's
/// `ieee_operand_0` around a comparison prints as written. Both routes agree.
#[test]
#[trace("TC-3039", "FR-931-AC-14")]
fn the_checker_ieee_operand_binder_never_prints() {
    const A: &str = "float64(bits: 0x3ff0000000000000)";
    const Z: &str = "float64(bits: 0x0000000000000000)";
    let check = |leaf: String, expected: &str| {
        let (from_source, from_package) = both_routes(&leaf);
        assert_eq!(from_source, expected, "{leaf}");
        assert_eq!(from_package, expected, "{leaf}");
        assert!(
            !from_source.contains("let") && !from_source.contains('#'),
            "{from_source}"
        );
    };
    let one = format!("holds(-{A} < {Z})");
    check(one.clone(), &one);
    check(format!("holds({Z} > -{A})"), &one);
    let greater = format!("holds(-{A} > -{Z})");
    check(greater.clone(), &greater);
    let lesser = format!("holds(-{Z} < -{A})");
    check(lesser.clone(), &lesser);
    assert_ne!(greater, lesser);

    let written = format!("holds(let ieee_operand_0 = -{A} in ieee_operand_0 < {Z})");
    let (from_source, from_package) = both_routes(&written);
    assert_eq!(from_source, written);
    assert_eq!(from_package, written);
}

/// FR-931-AC-14 (source route): a float literal that only a temporal clause
/// writes selects the IEEE profile, so the unit compiles with no float
/// declared and the lowering prints the ordering as written.
#[test]
#[trace("TC-3039", "FR-931-AC-14")]
fn a_float_literal_only_a_temporal_clause_writes_selects_the_ieee_profile() {
    let leaf = "holds(-float64(bits: 0x3ff0000000000000) < float64(bits: 0x0000000000000000))";
    assert_eq!(source_only(leaf), leaf);
}

/// FR-397-AC-29: the checker names each IEEE ordering operand binder
/// `#ieee_operand`, outer (left operand) first, and binds no literal or
/// local read; no parameter node name begins with `ieee_operand`.
#[test]
#[trace("TC-3039", "FR-397-AC-29")]
fn the_checker_names_its_ieee_operand_binders_with_a_hash() {
    for (body, levels) in [
        ("-p < -q", vec![2, 3]),
        ("-p < q", vec![2]),
        ("p < q", vec![]),
    ] {
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             function lt using v(p: Float64[nearest-even], q: Float64[nearest-even]): Boolean pure {{ {body} }}\n"
        );
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("unit.native"), unit).unwrap();
        let compile = directory.path().join("compile.json");
        std::fs::write(
            &compile,
            serde_json::to_vec(&json!({"format": "native-compile/1", "request": {
                "models": [], "program": {"source": source("unit.native")},
                "output": {"package": "package.json"}}}))
            .unwrap(),
        )
        .unwrap();
        fsl_command::command::compile(&compile).expect("the unit compiles");
        let package: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("package.json")).unwrap())
                .unwrap();
        // Every parameter node's `name` and `level` members.
        let mut parameters = Vec::new();
        let nodes = package["semantic_graph"]["nodes"].as_array().unwrap();
        for node in nodes
            .iter()
            .filter(|node| node["semantic_form"] == "parameter")
        {
            let member = |name: &str| {
                node["body"]["members"]
                    .as_array()
                    .and_then(|members| members.iter().find(|member| member["name"] == name))
                    .map(|member| member["value"]["value"].clone())
            };
            parameters.push((member("name"), member("level")));
        }
        // The binders bind the operand expressions, so a one-operand binder
        // fails the count and the levels.
        let mut hashed = parameters
            .iter()
            .filter(|(name, _)| name.as_ref().is_some_and(|name| name == "#ieee_operand"))
            .map(|(_, level)| {
                level
                    .as_ref()
                    .and_then(Value::as_str)
                    .and_then(|level| level.parse::<u64>().ok())
            })
            .collect::<Vec<_>>();
        hashed.sort_unstable();
        assert_eq!(
            hashed,
            levels.iter().copied().map(Some).collect::<Vec<_>>(),
            "{body}: {parameters:?}"
        );
        assert!(
            !parameters.iter().any(|(name, _)| name
                .as_ref()
                .and_then(Value::as_str)
                .is_some_and(|name| name.starts_with("ieee_operand"))),
            "{body}: {parameters:?}"
        );
    }
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

/// FR-931-AC-4: cross-evaluation agrees on the behaviours of AC-6's snapshot.
#[test]
#[trace("TC-2603", "FR-931-AC-4")]
fn tl_cross_evaluation_agrees_on_the_snapshot_behaviours() {
    let (code, value) = cross_from(&bump_low(), &["a", "b"], true, 2);
    assert_eq!(code, 0, "{value}");
    let cross = &value["tl_lowering"]["cross_evaluation"];
    let traces = cross["traces"].as_u64().expect("a trace count");
    assert!(traces >= 2, "{value}");
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
#[trace("TC-2962", "FR-931-AC-6")]
fn tl_cross_evaluation_reads_each_behaviour_as_a_closed_trace() {
    // FR-931-AC-6's snapshot `S` (objects `a` and `b`, the model stepping
    // `bump`), not the spine snapshot: that one is anchored at the handler
    // `validate`, which FR-120 refuses as an initial state.
    let unit = bump_low();
    let traces = |max_steps: u64| {
        let (code, value) = cross_from(&unit, &["a", "b"], true, max_steps);
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
#[trace("TC-2962", "FR-931-AC-7")]
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

/// FR-931-AC-6's unit: `Low` beside a postcondition that makes the model's
/// one operation step `versionNumber + 1` (`bump`) up to its bound 3.
fn bump_low() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"{EVENT_POSITION}\";\n\
         model Config = \"example/config-version\";\n\
         post Bump using v on Config::ConfigVersion::attemptUpdate {{ \
         pre(self.versionNumber) < 3 and self.versionNumber = pre(self.versionNumber) + 1 }}\n\
         temporal Low using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {LOW_BODY} }}\n"
    )
}

/// The spine snapshot with the population `complete` as given and holding
/// one object of `versionNumber` 0 and no parent per key of `keys`.
fn snapshot_of(directory: &Path, keys: &[&str], complete: bool) -> Value {
    let bytes = std::fs::read(directory.join("spine-snapshot.json")).unwrap();
    let mut snapshot: Value = serde_json::from_slice(&bytes).unwrap();
    // FR-120: an initial state is a snapshot anchored at an initialization.
    snapshot["anchor"] = json!({"kind": "initialization", "name": "start"});
    let population = &mut snapshot["populations"][0];
    let template = population["objects"][0].clone();
    population["complete"] = json!(complete);
    population["objects"] = keys
        .iter()
        .map(|key| {
            let mut object = template.clone();
            object["key"] = json!(key);
            object["fields"]["versionNumber"] = json!({"integer": "0"});
            object["fields"]["parent"] = json!({"absent": {}});
            object
        })
        .collect();
    snapshot
}

/// Cross-evaluates `unit`'s `Low` from the snapshot `snapshot_of(keys,
/// complete)` written as `s.json`, with `max_steps`. The ConfigVersion model
/// is narrowed to `versionNumber: Int[0, 3]`: FR-120 enumerates every
/// object's modified field, so two objects over `Int[0, 1000]` give 1001^2
/// candidates per application, past the default `max_candidates`.
fn cross_from(unit: &str, keys: &[&str], complete: bool, max_steps: u64) -> (u8, Value) {
    let directory = bump_directory(unit, keys, complete);
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "tl_lowering": {"clause": "Low",
                "cross_evaluate": {"initial": "s.json", "max_steps": max_steps}}}}),
    )
}

/// The spine files with the model narrowed to `versionNumber: Int[0, 3]`,
/// `unit` as `unit.native` and `snapshot_of(keys, complete)` as `s.json`.
fn bump_directory(unit: &str, keys: &[&str], complete: bool) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let model = directory.path().join("spine-model.semantic-ir.json");
    let narrowed = std::fs::read_to_string(&model)
        .unwrap()
        .replace("\"value\": \"1000\"", "\"value\": \"3\"");
    assert!(!narrowed.contains("\"1000\""), "the bound is narrowed");
    std::fs::write(&model, narrowed).unwrap();
    let snapshot = snapshot_of(directory.path(), keys, complete);
    std::fs::write(
        directory.path().join("s.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    directory
}

/// FR-931 "Cross-evaluation": a work-budget stop during enumeration reports
/// the counts reached and exits 22, never a correspondence loss (exit 30),
/// wherever the budget runs out: at an expansion, inside the subject's own
/// clause evaluation, or inside the native evaluator of a trace. The
/// request has no budget member, so the run is driven through the library
/// entry (`run_temporal_behaviours`, `tl_record`) under each `work_units`
/// bound from 1 to 200; an unbounded run completes.
#[test]
#[trace("TC-2962", "FR-931-AC-6")]
fn a_work_budget_stop_during_enumeration_exits_incomplete() {
    // `on each`: the last activation is evaluated after the last row is read,
    // so a budget can run out inside the evaluator there.
    let unit = bump_low().replace("on origin", "on each (d: Config::ConfigVersion)");
    let directory = bump_directory(&unit, &["a", "b"], true);
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("spine-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")},
        "clause": {"selection": {"temporal": {"name": "Low", "trace": {"finite": []}}},
            "observations": {"initial": "s.json"}}}});
    let path = directory.path().join("budget.json");
    std::fs::write(&path, request.to_string()).unwrap();
    let run = |work_units: u64| {
        let mut clause_run =
            fsl_command::command::read_clause_run_request(&path).expect("a clause run");
        clause_run.accounting.work_units = work_units;
        let initial = clause_run
            .snapshots
            .keys()
            .next()
            .expect("the snapshot")
            .clone();
        let report =
            fsl_replay::spine::run_temporal_behaviours(clause_run, &initial, 2).expect("a report");
        let fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) =
            &report.disposition
        else {
            panic!("a valuation: {:?}", report.disposition);
        };
        fsl_command::command::tl_record(
            valuation,
            &unit,
            None,
            &fsl_command::command::AtomMapping::default(),
        )
        .expect("the clause lowers")
    };
    let full = run(u64::MAX);
    assert_eq!(full.exit_code, 0, "{}", full.record);
    let complete = full.record["cross_evaluation"]["traces"].clone();
    // Every bound from 1 up: the low ones stop at the subject's own clause
    // evaluation, then at the trace reads, then inside the evaluator at an
    // `on each` activation, each an incomplete run.
    for work_units in 1..=200 {
        let record = run(work_units);
        let cross = &record.record["cross_evaluation"];
        assert_ne!(record.exit_code, 30, "{work_units}: {}", record.record);
        assert_eq!(
            cross["faults"],
            json!([]),
            "{work_units}: {}",
            record.record
        );
        assert_eq!(record.exit_code, 22, "{work_units}: {}", record.record);
        assert!(
            cross["traces"].as_u64() < complete.as_u64(),
            "{}",
            record.record
        );
    }
}

/// FR-931-AC-6: from a snapshot of `a` and `b` whose model steps `bump`, the
/// run evaluates both instances and each behaviour of 1 and of 2 steps as its
/// own closed trace: `traces` is 2 (the instances) times the 1- and 2-step
/// paths, and every trace compares at least its activation position.
#[test]
#[trace("TC-2962", "FR-931-AC-6")]
fn tl_cross_evaluation_enumerates_instances_and_behaviours_of_a_snapshot() {
    let unit = bump_low();
    let cross = |max_steps: u64| {
        let (code, value) = cross_from(&unit, &["a", "b"], true, max_steps);
        assert_eq!(code, 0, "{value}");
        let cross = value["tl_lowering"]["cross_evaluation"].clone();
        assert_eq!(cross["agree"], true, "{value}");
        assert_eq!(cross["faults"], json!([]), "{value}");
        cross
    };
    // FR-120's successors of `(a, b)`: for each receiver below 3, the
    // receiver bumped and the other object's `versionNumber` (in the frame
    // `modifies [versionNumber]`, unconstrained by `Bump`) each of 0 to 3.
    let successors = |(a, b): (u64, u64)| {
        let mut next = Vec::new();
        for other in 0..=3 {
            if a < 3 {
                next.push((a + 1, other));
            }
            if b < 3 {
                next.push((other, b + 1));
            }
        }
        next
    };
    let one_step = successors((0, 0));
    let two_step: usize = one_step.iter().map(|state| successors(*state).len()).sum();
    let paths = |steps: u64| match steps {
        1 => one_step.len(),
        _ => one_step.len() + two_step,
    };
    for max_steps in [1, 2] {
        let cross = cross(max_steps);
        let traces = cross["traces"].as_u64().expect("a trace count");
        // Two instances, `a` then `b`, each reading every path; a 1-step
        // behaviour is a trace of its own beside its 2-step extensions.
        assert_eq!(
            traces,
            2 * u64::try_from(paths(max_steps)).unwrap(),
            "{cross}"
        );
        assert!(cross["positions"].as_u64().expect("a position count") >= traces);
    }
}

/// FR-931-AC-7: an incomplete population refuses `incomplete_population`
/// naming it, exit 20; an empty population gives zero traces and zero
/// positions, exit 0.
#[test]
#[trace("TC-2962", "FR-931-AC-7")]
fn tl_cross_evaluation_refuses_an_incomplete_universe_and_counts_an_empty_one() {
    let unit = bump_low();
    let (code, value) = cross_from(&unit, &["a"], false, 2);
    // `incomplete_population` is the incomplete row, exit 22 (ADR-029 CB-4).
    assert_eq!(code, 22, "{value}");
    assert_eq!(value["stage"], "request", "{value}");
    refusal_names(&value, "incomplete_population");
    refusal_names(&value, "config_history");
    let (code, value) = cross_from(&unit, &[], true, 2);
    assert_eq!(code, 0, "{value}");
    let cross = &value["tl_lowering"]["cross_evaluation"];
    assert_eq!(cross["traces"], 0, "{value}");
    assert_eq!(cross["positions"], 0, "{value}");
    assert_eq!(cross["agree"], true, "{value}");
}

/// FR-931-AC-9's infinite-trace clause `C` over FR-932's `Counter` record,
/// with `constraint` before its formula.
fn counter_unit(constraint: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile m = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         record Counter {{ level: Int[0, 9]; }}\n\
         temporal C using t over (c: Counter) clock \"steps\" on each (q: Counter) \
         {{ {constraint}once[1,1] not holds(c.level = 0) }}\n"
    )
}

/// The record-sample unit of the lasso parity tests: `Settles` holds where
/// level 0 occurs only in the prefix, `Recurs` where it recurs in the loop, so
/// the verdict of each depends on which slot every position of a lasso reads.
fn lasso_unit() -> String {
    "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile m = \"filament.model.complete/v1\";\n\
     profile t = \"filament.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\";\n\
     record Counter { level: Int[0, 9]; }\n\
     temporal Settles using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { eventually always not holds(c.level = 0) }\n\
     temporal Recurs using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { always eventually holds(c.level = 0) }\n\
     temporal Prev using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { once[1,1] not holds(c.level = 0) }\n\
     temporal PrevZero using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { once[1,1] holds(c.level = 0) }\n\
     temporal Safe using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { always not holds(c.level = 0) }\n\
     temporal Hits using t over (c: Counter) clock \"steps\" on each (q: Counter) \
     { eventually holds(c.level = 0) }\n"
        .to_owned()
}

/// A record sample of the lasso unit.
fn level_sample(level: u32) -> Value {
    json!({"sample": {"level": {"integer": level.to_string()}}})
}

/// The lassos of the parity tests, each with the exit code `Settles` and
/// `Recurs` settle with. In the first, level 0 is only in the prefix; in the
/// second, level 0 is only in the loop, after a prefix and a loop slot that
/// differ in value, so a lasso that wraps to the wrong slot (the loop offset
/// dropped, or the prefix folded into the cycle) reads another verdict.
fn lasso_cases() -> [(Value, u8, u8); 2] {
    [
        (
            json!({"lasso": {"prefix": [level_sample(0)],
                "loop": [level_sample(1), level_sample(2)]}}),
            0,
            10,
        ),
        (
            json!({"lasso": {"prefix": [level_sample(2)],
                "loop": [level_sample(1), level_sample(0)]}}),
            10,
            0,
        ),
    ]
}

/// FR-931-AC-10, fairness half (TC-3023 step 4): the infinite-trace clause
/// `C` with a weak fairness constraint over an observed lasso settles
/// `MissingFairnessPremise` as FR-329 settles it, exit 21, and its record
/// carries no `cross_evaluation`, because the native side has no verdict to
/// compare; the same clause without the constraint cross-evaluates.
#[test]
#[trace("TC-3023", "FR-931-AC-10")]
fn a_fairness_constraint_over_a_lasso_settles_missing_fairness_premise() {
    let unit = counter_unit;
    let lasso = json!({"lasso": {"prefix": [],
        "loop": [{"sample": {"level": {"integer": "0"}}}]}});
    let lower = |unit: String| {
        run(
            &unit,
            true,
            (
                "tl_lowering",
                json!({"clause": "C", "cross_evaluate": {"trace": lasso}}),
            ),
        )
    };
    let (code, value) = lower(unit(""));
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["tl_lowering"]["cross_evaluation"]["agree"], true);
    assert_eq!(
        value["tl_lowering"]["atoms"]["p0"], "holds(c.level = 0)",
        "{value}"
    );

    let (code, value) = lower(unit(
        "fair weak each Config::ConfigVersion::attemptUpdate; ",
    ));
    assert_eq!(code, 21, "{value}");
    assert!(
        value.to_string().contains("MissingFairnessPremise")
            || value.to_string().contains("missing-fairness-premise"),
        "{value}"
    );
    assert!(value.get("tl_lowering").is_none(), "{value}");
}

/// FR-931-AC-13 over leaves with no model field read (the package route of
/// `tl_lowering` is not implemented yet): each
/// `case` and `let` binder prints under its declared name, arms print in
/// member declaration order, `_` is an ordinary binder, and renaming a binder
/// renames the label.
#[test]
#[trace("TC-3039", "FR-931-AC-13")]
fn case_binders_print_under_their_declared_names_without_a_model_read() {
    let leaf = |binders: [&str; 8]| {
        let [k, r, h, x, a, b, w, m] = binders;
        format!(
            "holds((let {k} = 3 in case Shape::Rect({k}, 1) {{ Circle({r}): {r} <= {k}; \
             Rect(_, {h}): case Shape::Circle({h}) {{ Circle({x}): {x} + {k} <= 4; \
             Rect({a}, {b}): false; Empty: true; }}; Empty: true; }}) and (case Shape::Rect(1, 2) \
             {{ Empty: true; Rect({w}, {m}): {w} + {m} <= 4; Circle(z): z <= 2; }}))"
        )
    };
    let from_source = source_only(&leaf(["k", "r", "h", "x", "a", "b", "w", "m"]));
    let key = from_source
        .strip_prefix("holds((let k = 3 in case ")
        .and_then(|rest| rest.split_once("::Rect("))
        .map(|(key, _)| key.to_owned())
        .unwrap_or_else(|| panic!("{from_source}"));
    assert!(
        key.starts_with("ix://") && key.ends_with("/Shape"),
        "{from_source}"
    );
    assert_eq!(
        from_source,
        format!(
            "holds((let k = 3 in case {key}::Rect(k, 1) {{ Circle(r): r <= k; \
             Rect(_, h): case {key}::Circle(h) {{ Circle(x): x + k <= 4; Rect(a, b): false; \
             Empty: true; }}; Empty: true; }}) and (case {key}::Rect(1, 2) \
             {{ Circle(z): z <= 2; Rect(w, m): w + m <= 4; Empty: true; }}))"
        )
    );
    let renamed = leaf(["kk", "rr", "hh", "xx", "aa", "bb", "ww", "mm"]);
    let renamed_source = source_only(&renamed);
    assert_ne!(renamed_source, from_source);
    assert!(
        renamed_source.contains("Rect(ww, mm): ww + mm <= 4"),
        "{renamed_source}"
    );
}

/// The `Low` unit with `declarations` before its clause and `leaf` under
/// `always`.
fn declaring_low(declarations: &str, leaf: &str) -> String {
    shape_low(leaf).replacen("union Shape", &format!("{declarations}\nunion Shape"), 1)
}

/// FR-931-AC-16, FSpec FR-322 "Named type arguments" (QSL-2630): the source
/// route prints the declaration key a `count` type argument names, and a
/// chain of aliases at the outer alias. The package half and the
/// alias-removed copy are `a_named_count_type_argument_prints_over_the_package`.
#[test]
#[trace("TC-2601", "FR-931-AC-16")]
fn a_named_count_type_argument_prints_its_alias_key_on_the_source_route() {
    let identity = "test:s1-16:unit.native";
    for (declarations, printed) in [
        ("type Small = Int[0, 2];", format!("ix://{identity}/Small")),
        (
            "type Small = Tiny;\ntype Tiny = Int[0, 2];",
            format!("ix://{identity}/Small"),
        ),
    ] {
        let leaf =
            "holds(let q: Sequence<Integer>[0, 2] = sequence[c.versionNumber, c.versionNumber] \
                    in count<Small>(x in q: x > 1) >= 1)";
        let unit = declaring_low(declarations, leaf);
        let (code, value) = run(&unit, true, ("tl_lowering", json!({"clause": "Low"})));
        assert_eq!(code, 0, "{value}");
        let from_source = value["tl_lowering"]["atoms"]["p0"]
            .as_str()
            .unwrap()
            .to_owned();
        let expected = format!("count<{printed}>(x in q: x > 1)");
        assert!(
            from_source.contains(&expected),
            "{declarations}: {from_source}"
        );
    }
}

/// QSL-2726 item 15: a `seq` source inside a temporal clause emits a package
/// the strict read admits, and both routes print its atom alike.
#[test]
#[trace("TC-2601")]
fn a_seq_source_inside_a_clause_emits_a_package_the_strict_read_admits() {
    let leaf = "holds(size(seq(i in 0 .. 3: i)) >= 1)";
    let (from_source, from_package) = both_routes(leaf);
    assert_eq!(from_source, from_package);
}

/// The units the model's alias `D` needs.
const MS_UNITS: &str =
    "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\n";

/// The spine model with `ConfigVersion` also holding `xs: Sequence<Digit>[0, 4]`
/// (FR-931-AC-16's fixture, `Digit` being `Int[0, 9]`) and `ds: Sequence<D>[0, 2]`
/// with `D` an alias of the unit `ms`, and the scalars `Quad` (`Int[0, 4]`) and
/// `Total` (`Int[0, 36]`).
fn model_with_xs() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/config-version/model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let types = model["types"].as_array_mut().unwrap();
    let scalar = |name: &str, maximum: &str| -> Value {
        serde_json::from_str(
            &types[0]
                .to_string()
                .replace("VersionNumber", name)
                .replace("\"1000\"", &format!("\"{maximum}\"")),
        )
        .unwrap()
    };
    let field = |name: &str, type_ref: &str, upper: u64| -> Value {
        let mut field: Value = serde_json::from_str(
            &types[1]["fields"][0]
                .to_string()
                .replace("versionNumber", name)
                .replace("ix://example/config-version/VersionNumber", type_ref),
        )
        .unwrap();
        field["multiplicity"] =
            json!({"lower": 0, "upper": upper, "ordered": true, "unique": false});
        field
    };
    let added = [
        scalar("Digit", "9"),
        scalar("Quad", "4"),
        scalar("Total", "36"),
    ];
    let identity = "ix://example/config-version/D";
    let alias = json!({"identity": identity, "displayName": identity, "kind": "alias",
        "roles": [], "constraints": [], "extensions": [], "unknownPolicy": "reject",
        "target": "ix://filament/unit/ms",
        "origin": {"generated": {"generatorIdentity": identity,
            "generatorVersion": "1.0.0", "inputIdentities": [identity]}}});
    let xs = field("xs", "ix://example/config-version/Digit", 4);
    let ds = field("ds", identity, 2);
    let fields = types[1]["fields"].as_array_mut().unwrap();
    fields.push(xs);
    fields.push(ds);
    types.extend(added);
    types.push(alias);
    serde_json::to_vec(&model).unwrap()
}

/// Removes every `type_argument` member's `alias` from the package files of
/// a compiled directory.
fn remove_type_argument_aliases(directory: &Path) {
    fn strip(value: &mut Value) {
        match value {
            Value::Object(map) => {
                if map.get("kind") == Some(&json!("type_argument")) {
                    map.remove("alias");
                }
                map.values_mut().for_each(strip);
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    for file in ["package.json", "handoff/checked-package.json"] {
        let path = directory.join(file);
        let mut package: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        strip(&mut package);
        std::fs::write(&path, serde_json::to_vec(&package).unwrap()).unwrap();
    }
}

/// FR-931-AC-16's package half (QSL-2630): over a model whose `c` also holds
/// `xs`, the unit's checked package selected as `package` with the source
/// removed prints each leaf as the source route does, `count<Small>` from
/// the `alias` of its `type_argument` member (also through a chain of
/// aliases), and a copy of the package with the alias removed prints
/// `count<Int[0, 4]>`.
#[test]
#[trace("TC-2601", "FR-931-AC-16")]
fn a_named_count_type_argument_prints_over_the_package() {
    let model = model_with_xs();
    let identity = "test:s1-16:unit.native";
    let plain = "holds(count<Config::Quad>(x in c.xs: x>1) >= 1)";
    let leaves = [
        (
            "type Small = Int[0, 4];",
            "holds(count<Small>(x in c.xs: x>1) >= 1)",
        ),
        (
            "type Small = Tiny;\ntype Tiny = Int[0, 4];",
            "holds(count<Small>(x in c.xs: x>1) >= 1)",
        ),
        ("type Small = Int[0, 4];", plain),
        (
            "type Small = Int[0, 4];",
            "holds(sum<Config::Total>(x in c.xs: x) <= 9)",
        ),
    ];
    for (declarations, leaf) in leaves {
        let unit = declaring_low(&format!("{MS_UNITS}{declarations}"), leaf);
        let (from_source, from_package) = unit_both_routes_over_model(
            &unit,
            json!({"directory": "handoff"}),
            Some(&model),
            |_| {},
        );
        assert_eq!(from_source, from_package, "{declarations}: {leaf}");
        let printed = if leaf.contains("count<Small>") {
            format!("count<ix://{identity}/Small>(x in c.xs: x > 1)")
        } else if leaf.contains("count<Config::Quad>") {
            "count<Int[0, 4]>(x in c.xs: x > 1)".to_owned()
        } else if leaf.contains("sum<Config::Total>") {
            "sum<Int[0, 36]>(x in c.xs: x)".to_owned()
        } else {
            continue;
        };
        assert!(from_package.contains(&printed), "{from_package}");
    }
    let unit = declaring_low(
        &format!("{MS_UNITS}type Small = Int[0, 4];"),
        "holds(count<Small>(x in c.xs: x>1) >= 1)",
    );
    let (_, without_alias) = unit_both_routes_over_model(
        &unit,
        json!({"directory": "handoff"}),
        Some(&model),
        remove_type_argument_aliases,
    );
    assert_eq!(
        without_alias,
        "holds(count<Int[0, 4]>(x in c.xs: x > 1) >= 1)"
    );
}

/// FSpec FR-322 "Named type arguments" end to end (QSL-2726): over a real
/// domain package, `sum<Config::D>` names the model's alias `D` of `ms`, so
/// its `type_argument` member carries the alias keyed by the model's
/// identity, and `count<Config::Quad>` names a model scalar, which carries
/// none.
#[test]
#[trace("TC-2601", "FR-931-AC-16")]
fn a_model_alias_type_argument_carries_the_model_key_and_a_model_scalar_none() {
    let model = model_with_xs();
    let unit = declaring_low(
        MS_UNITS,
        "holds(sum<Config::D>(x in c.ds: x) >= 0 ms and count<Config::Quad>(x in c.xs: x>1) >= 1)",
    );
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(
        directory.path().join("spine-model.semantic-ir.json"),
        &model,
    )
    .unwrap();
    std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
    let compile = directory.path().join("compile.json");
    std::fs::write(
        &compile,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0",
                "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "output": {"package": "package.json"}}}))
        .unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&compile).expect("the unit compiles");
    let package: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("package.json")).unwrap())
            .unwrap();
    let mut arguments: Vec<(String, Option<Value>)> = package["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["body"]["operation"]["member"]["kind"] == "type_argument")
        .map(|node| {
            (
                node["body"]["operation"]["identity"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                node["body"]["operation"]["member"].get("alias").cloned(),
            )
        })
        .collect();
    arguments.sort_by(|left, right| left.0.cmp(&right.0));
    let model_key = "ix://example/config-version/D";
    let [(count, count_alias), (sum, sum_alias)] = arguments.as_slice() else {
        panic!("one count and one sum: {arguments:?}");
    };
    assert!(
        count.ends_with("count") && sum.contains(".sum"),
        "{arguments:?}"
    );
    assert_eq!(count_alias, &None, "a model scalar gets no alias");
    assert_eq!(
        sum_alias,
        &Some(json!({"package": "example/config-version", "node": model_key})),
        "a model alias gets the model key"
    );
}

/// Run supplied clauses through both selectors after removing the source file.
#[derive(Clone, Copy)]
enum SelectionObjects {
    Present,
    MissingSecond,
    UniverseOnly,
}

fn package_clause_selection_outcomes(
    unit: &str,
    traces: &[(Value, u8)],
    refused_violation: bool,
    objects: SelectionObjects,
) {
    let execute = |directory: &Path, request: &Value| {
        let path = directory.join("request.json");
        std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
        match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
            Ok(result) => (
                result.exit_code,
                result.value.as_value().clone(),
                result
                    .outcome
                    .map(|outcome| outcome.to_bytes().expect("the complete outcome encodes")),
            ),
            Err(error) => (error.category().exit_code(), error.value().unwrap(), None),
        }
    };
    let directory = bump_directory(unit, &["a"], true);
    let mut second = snapshot_of(directory.path(), &["a"], true);
    second["identity"]["identity"] = json!("selection-second");
    second["populations"][0]["objects"][0]["fields"]["versionNumber"] = json!({"integer": "1"});
    match objects {
        SelectionObjects::Present => {}
        SelectionObjects::MissingSecond => second["populations"][0]["objects"] = json!([]),
        SelectionObjects::UniverseOnly => {
            second["populations"][0]["objects"][0]["key"] = json!("b")
        }
    }

    std::fs::write(
        directory.path().join("second.json"),
        serde_json::to_vec(&second).unwrap(),
    )
    .unwrap();
    let first = snapshot_of(directory.path(), &["a"], true);
    let population = first["populations"][0]["population"].clone();
    let models =
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}]);
    let clauses: Vec<_> = traces
        .iter()
        .map(|(trace, expected)| {
            let clause = json!({"selection": {"temporal": {"name": "Low", "trace": trace,
            "over": {"reference": {"population": population, "key": "a"}}}},
            "observations": {"s0": "s.json", "s1": "second.json"}});
            let result = execute(
                directory.path(),
                &json!({"format": "native-run/1", "request": {
            "models": models, "program": {"source": source("unit.native")}, "clause": clause}}),
            );
            assert_eq!(result.0, *expected, "source {trace}: {}", result.1);
            (clause, result)
        })
        .collect();
    let models = compile_handoff(directory.path(), unit);
    for (clause, expected) in clauses {
        let actual = execute(
            directory.path(),
            &json!({"format": "native-run/1", "request": {
            "models": models, "package": {"directory": "handoff"}, "clause": clause}}),
        );
        if refused_violation {
            assert_eq!(actual.0, 20, "{clause}: {}", actual.1);
            assert_eq!(actual.1["code"], "invalid-request", "{}", actual.1);
            refusal_names(&actual.1, "clause.selection");
        } else {
            assert_eq!(actual, expected, "{clause}");
            fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&actual.1).unwrap())
                .expect("the result reads through the native result reader");
            if clause["selection"]["temporal"]["trace"]
                .get("model")
                .is_some()
            {
                for limit in [
                    json!("0"),
                    json!("01"),
                    json!("9007199254740992"),
                    json!(1),
                    Value::Null,
                ] {
                    let mut malformed = actual.1.clone();
                    malformed["limits"]["expansion.max_candidates"] = limit;
                    assert!(fsl_replay::spine::read_native_run_result(
                        &serde_json::to_vec(&malformed).unwrap()
                    )
                    .is_err());
                }
                let mut malformed = actual.1.clone();
                malformed["limits"]["unknown"] = json!("1");
                assert!(fsl_replay::spine::read_native_run_result(
                    &serde_json::to_vec(&malformed).unwrap()
                )
                .is_err());
            }
            if expected.0 == 10
                && clause["selection"]["temporal"]["trace"]
                    .get("model")
                    .is_some()
            {
                let temporal = &actual.1["disposition"]["temporal"];
                if temporal["kind"] == "violated" {
                    assert_eq!(temporal["counterexample"]["trace"], "model", "{}", actual.1);
                } else {
                    assert_eq!(temporal["kind"], "undefined", "{}", actual.1);
                    assert!(temporal.get("counterexample").is_none());
                }
                let mut malformed = actual.1.clone();
                malformed["disposition"]["temporal"]["counterexample"] = Value::Null;
                assert!(fsl_replay::spine::read_native_run_result(
                    &serde_json::to_vec(&malformed).unwrap()
                )
                .is_err());
            }
        }
    }
}

/// FR-1039 Outputs: finite observations and distinct lasso slots have the
/// source outcome, including undefined conditions, with no source file available.
#[test]
#[trace("TC-3019", "FR-1039")]
fn package_clause_selections_snapshot_results_match_source() {
    for (objects, expected) in [
        (SelectionObjects::Present, 0),
        (SelectionObjects::MissingSecond, 10),
    ] {
        let unit = bump_low().replace(LOW_BODY, "always[0,1] holds(c.versionNumber <= 2)");
        package_clause_selection_outcomes(
            &unit,
            &[(json!({"finite": ["s0", "s1"]}), expected)],
            false,
            objects,
        );
        let unit = unit.replace(EVENT_POSITION, "filament.temporal.infinite-trace/v1");
        package_clause_selection_outcomes(
            &unit,
            &[(
                json!({"lasso": {"prefix": ["s0"], "loop": ["s1"]}}),
                expected,
            )],
            false,
            objects,
        );
    }
}

/// FR-1039 Outputs: model behaviours are re-executed over the package world;
/// a terminal state's true and undefined conditions retain the source outcomes.
#[test]
#[trace("TC-3019", "FR-1039")]
fn package_clause_selections_model_results_match_source() {
    for (body, objects, expected) in [
        ("holds(c.versionNumber <= 2)", SelectionObjects::Present, 0),
        (
            "holds(deref(Config::ConfigVersion#\"b\").versionNumber >= 0)",
            SelectionObjects::UniverseOnly,
            10,
        ),
    ] {
        let unit = bump_low()
            .replace(EVENT_POSITION, "filament.temporal.infinite-trace/v1")
            .replace(
                "pre(self.versionNumber) < 3 and self.versionNumber = pre(self.versionNumber) + 1",
                "false",
            )
            .replace(LOW_BODY, body);
        let trace = json!({"model": {"initial": "s0", "universes": ["s0", "s1"], "prefix": [],
            "loop": [{"transition": {"type": "terminal-stutter"}, "post_state": {"key": "W10"}}]}});
        package_clause_selection_outcomes(&unit, &[(trace, expected)], false, objects);
    }
}

/// FR-1039 Outputs: timed snapshots use exact intervals, finite completeness
/// and the lasso period, authenticating the same selected clock occurrence.
#[test]
#[trace("TC-3019", "FR-1039")]
fn package_clause_selections_timed_results_match_source() {
    for (objects, expected) in [
        (SelectionObjects::Present, 0),
        (SelectionObjects::MissingSecond, 10),
    ] {
        let unit = timed_low();
        let prefix = json!([{"position": "s0", "stamp": "0"}, {"position": "s1", "stamp": "2"}]);
        package_clause_selection_outcomes(
            &unit,
            &[
                (
                    json!({"timed": {"prefix": prefix, "through": "3"}}),
                    expected,
                ),
                (
                    json!({"timed": {"prefix": [{"position": "s0", "stamp": "0"}], "loop": [{"position": "s1", "stamp": "2"}], "period": "3"}}),
                    expected,
                ),
            ],
            false,
            objects,
        );
    }
}

/// FR-1039 Outputs: violated snapshot clauses follow the same traced
/// decisive decision walk and return the source route's complete record.
#[test]
#[trace("TC-3019", "FR-1039")]
fn package_clause_snapshot_violations_match_source_decisive_locations() {
    let unit = bump_low().replace(LOW_BODY, "holds(c.versionNumber > 2)");
    package_clause_selection_outcomes(
        &unit,
        &[(json!({"finite": ["s0", "s1"]}), 10)],
        false,
        SelectionObjects::Present,
    );
}

/// A model violation reads the decisive atom over its reconstructed state.
#[test]
#[trace("TC-840", "FR-330-AC-16")]
fn package_clause_model_violations_match_source_decisive_locations() {
    let unit = bump_low()
        .replace(LOW_BODY, "holds(c.versionNumber > 2)")
        .replace(EVENT_POSITION, "filament.temporal.infinite-trace/v1")
        .replace(
            "pre(self.versionNumber) < 3 and self.versionNumber = pre(self.versionNumber) + 1",
            "false",
        );
    package_clause_selection_outcomes(
        &unit,
        &[(
            json!({"model": {"initial": "s0", "universes": ["s0", "s1"], "prefix": [],
            "loop": [{"transition": {"type": "terminal-stutter"}, "post_state": {"key": "W10"}}]}}),
            10,
        )],
        false,
        SelectionObjects::Present,
    );
}

/// FR-125 "Instancing": over `c: Config::Draft` at a key whose admitted
/// record is a `Published`, a sibling of `Draft`, a violated model trace keeps
/// its counterexample, bound to the admitted type in the universe of
/// `ConfigVersion`, on the source and package routes.
#[test]
#[trace("TC-520", "FR-125-AC-9")]
fn a_model_counterexample_keeps_a_non_conforming_admitted_binding() {
    let identity = |name: &str| format!("ix://example/config-version/{name}");
    let unit = bump_low()
        .replace(EVENT_POSITION, "filament.temporal.infinite-trace/v1")
        .replace(
            "pre(self.versionNumber) < 3 and self.versionNumber = pre(self.versionNumber) + 1",
            "false",
        )
        .replace(
            "over (c: Config::ConfigVersion)",
            "over (c: Config::Draft)",
        )
        .replace(LOW_BODY, "holds(false)");
    let directory = bump_directory(&unit, &["a"], true);

    let path = directory.path().join("spine-model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let types = model["types"].as_array_mut().unwrap();
    let base = types
        .iter()
        .find(|node| node["identity"] == identity("ConfigVersion").as_str())
        .expect("the model declares ConfigVersion")
        .clone();
    for name in ["Draft", "Published"] {
        let mut subtype = base.clone();
        subtype["identity"] = json!(identity(name));
        subtype["displayName"] = json!(identity(name));
        subtype["supertypes"] = json!([identity("ConfigVersion")]);
        for member in ["fields", "operations", "relationships", "constraints"] {
            subtype[member] = json!([]);
        }
        types.push(subtype);
    }
    std::fs::write(&path, serde_json::to_vec(&model).unwrap()).unwrap();
    let mut snapshot = snapshot_of(directory.path(), &["a"], true);
    snapshot["populations"][0]["objects"][0]["type"] = json!(identity("Published"));
    let population = snapshot["populations"][0]["population"].clone();
    std::fs::write(
        directory.path().join("s.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();

    let snapshot_identity = snapshot["identity"]["identity"].as_str().unwrap().to_owned();
    let clause = json!({
        "selection": {"temporal": {"name": "Low",
            "trace": {"model": {"initial": snapshot_identity.as_str(),
                "universes": [snapshot_identity.as_str()], "prefix": [],
                "loop": [{"transition": {"type": "terminal-stutter"},
                    "post_state": {"key": "W10"}}]}},
            "over": {"reference": {"population": population, "key": "a"}}}},
        "observations": {snapshot_identity.as_str(): "s.json"}});
    let expected = json!({
        "type": "reference",
        "identity": "a",
        "object_type": {"owner_effective_type": null, "original": {
            "package": "example/config-version", "node": identity("Published")}},
        "universe": {"package": "example/config-version", "ordinal": 0,
            "path": ["model", "object_type", identity("ConfigVersion")]}});
    let execute = |request: Value| {
        let path = directory.path().join("request.json");
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
        let document = result.value.as_value().clone();
        assert_eq!(result.exit_code, 10, "{document}");
        assert_eq!(
            document["disposition"]["temporal"]["counterexample"]["over_binding"], expected,
            "{document}"
        );
    };
    let models =
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}]);
    execute(json!({"format": "native-run/1", "request": {
        "models": models, "program": {"source": source("unit.native")}, "clause": clause}}));
    let models = compile_handoff(directory.path(), &unit);
    execute(json!({"format": "native-run/1", "request": {
        "models": models, "package": {"directory": "handoff"}, "clause": clause}}));
}

/// The decisive walk follows the taken `if` arm and authored connective to
/// the quantifier which supplied the actual false element.
#[test]
#[trace("TC-840", "FR-330-AC-12")]
fn package_remainder_decisive_walk_reaches_the_quantifier_through_branches() {
    let unit = format!("{}\nfunction ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure {{ xs }}\n",
        bump_low().replace(LOW_BODY,
        "holds(if c.versionNumber >= 0 then (true and forall(n in ints(sequence[3, c.versionNumber, 5]): n > 0)) else true)"));
    package_clause_selection_outcomes(
        &unit,
        &[(json!({"finite": ["s0", "s1"]}), 10)],
        false,
        SelectionObjects::Present,
    );
}

/// FR-931 keeps canonical text and first-formula-use numbering even when
/// structural atoms repeat and their authored whitespace differs.
#[test]
#[trace("TC-2601", "FR-931-AC-11", "FR-931-AC-18")]
fn package_remainder_tl_atoms_keep_canonical_text_and_first_occurrence_ids() {
    let unit = bump_low().replace(LOW_BODY,
        "holds( ( c.versionNumber<=2 ) ) and (holds(c.versionNumber<1) or holds(c.versionNumber <= 2))");
    let directory = bump_directory(&unit, &["a"], true);
    let member = json!({"clause": "Low"});
    let expected = run(&unit, true, ("tl_lowering", member.clone()));
    assert_eq!(expected.0, 0, "{}", expected.1);
    let models = compile_handoff(directory.path(), &unit);
    let actual = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
        "models": models, "package": {"directory": "handoff"}, "tl_lowering": member }}),
    );
    assert_eq!(actual, expected);
    assert_eq!(
        actual.1["tl_lowering"]["atoms"],
        json!({
        "p0": "holds(c.versionNumber <= 2)", "p1": "holds(c.versionNumber < 1)" })
    );
    let formula = actual.1["tl_lowering"]["formula"].as_str().unwrap();
    assert_eq!(formula.matches("p0").count(), 2, "{formula}");
    assert_eq!(formula.matches("p1").count(), 1, "{formula}");
}

#[test]
#[trace("TC-3019", "FR-1039-AC-8", "FR-100-AC-23")]
fn package_remainder_caller_source_carrier_refuses_before_file_io() {
    let directory = tempfile::tempdir().unwrap();
    let valid = source("unavailable.native");
    let mut cases = vec![
        (Value::Null, "/request/sources".to_owned()),
        (json!({}), "/request/sources".to_owned()),
        (json!([[]]), "/request/sources/0".to_owned()),
    ];
    for field in [
        "file",
        "authority",
        "identity",
        "document",
        "formal_revision",
    ] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        cases.push((json!([missing]), format!("/request/sources/0/{field}")));
        let mut wrong = valid.clone();
        wrong[field] = if field == "formal_revision" {
            json!(-1)
        } else {
            json!(7)
        };
        cases.push((json!([wrong]), format!("/request/sources/0/{field}")));
    }
    let mut extra = valid.clone();
    extra["text"] = json!("unused");
    cases.push((json!([extra]), "/request/sources/0/text".to_owned()));
    let mut repeated = valid.clone();
    repeated["file"] = json!("another.native");
    repeated["document"] = json!("Other");
    cases.push((
        json!([valid.clone(), repeated]),
        "/request/sources/1".to_owned(),
    ));
    for (sources, pointer) in cases {
        let (code, value) = write_request(
            directory.path(),
            &json!({"format":"native-run/1", "request":{
            "models":[], "package":{"file":"unavailable-package.json"},
            "sources":sources, "tl_lowering":{"clause":"Low"}}}),
        );
        assert_eq!(code, 20, "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert!(
            value["message"].as_str().unwrap().contains(&pointer),
            "{pointer}: {value}"
        );
    }
    for sources in [json!([]), json!([valid])] {
        let (code, value) = write_request(
            directory.path(),
            &json!({"format":"native-run/1", "request":{
            "models":[], "program":{"source":source("unavailable-program.native")},
            "sources":sources, "call":{"function":"f","arguments":[]}}}),
        );
        assert_eq!(code, 20, "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(
            value["message"]
                .as_str()
                .unwrap()
                .contains("/request/sources"),
            "{value}"
        );
    }
    for (field, supplied) in [("document", json!("")), ("formal_revision", json!(0))] {
        let mut selected = source("unavailable.native");
        selected[field] = supplied;
        let (code, value) = write_request(
            directory.path(),
            &json!({"format":"native-run/1", "request":{
            "models":[], "package":{"file":"unavailable-package.json"},
            "sources":[selected], "tl_lowering":{"clause":"Low"}}}),
        );
        assert_eq!(code, 20, "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert_eq!(value["code"], "invalid-identifier", "{value}");
        assert_eq!(
            value["details"]["code"],
            if field == "formal_revision" {
                "invalid_source_revision"
            } else {
                "invalid_identifier"
            },
            "{value}"
        );
        assert!(
            value
                .to_string()
                .contains(&format!("/request/sources/0/{field}")),
            "{value}"
        );
    }
    let selected = source("unavailable.native");
    let mut raw_cases = vec![(
        "\"sources\":[],\"sources\":[]".to_owned(),
        "/request/sources".to_owned(),
    )];
    let rendered = serde_json::to_string(&selected).unwrap();
    for field in [
        "file",
        "authority",
        "identity",
        "document",
        "formal_revision",
    ] {
        raw_cases.push((
            format!(
                "\"sources\":[{},\"{field}\":{}}}]",
                &rendered[..rendered.len() - 1],
                selected[field]
            ),
            format!("/request/sources/0/{field}"),
        ));
    }
    for value in ["18446744073709551616", "1.5", "null"] {
        let mut selected = selected.clone();
        selected.as_object_mut().unwrap().remove("formal_revision");
        let rendered = serde_json::to_string(&selected).unwrap();
        raw_cases.push((
            format!(
                "\"sources\":[{},\"formal_revision\":{value}}}]",
                &rendered[..rendered.len() - 1]
            ),
            "/request/sources/0/formal_revision".to_owned(),
        ));
    }
    for (members, pointer) in raw_cases {
        let raw = format!("{{\"format\":\"native-run/1\",\"request\":{{\"models\":[],\"package\":{{\"file\":\"unavailable-package.json\"}},{members},\"tl_lowering\":{{\"clause\":\"Low\"}}}}}}");
        let path = directory.path().join("raw.json");
        std::fs::write(&path, raw).unwrap();
        let refusal = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .expect_err("malformed carrier refuses");
        let value = refusal.value().unwrap();
        assert_eq!(refusal.category().exit_code(), 20, "{value}");
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert!(
            value["message"].as_str().unwrap().contains(&pointer),
            "{pointer}: {value}"
        );
    }
    let mut invalid_revision = source("unavailable.native");
    invalid_revision["formal_revision"] = json!(0);
    for members in [
        "\"sources\":[],\"sources\":null".to_owned(),
        format!("\"sources\":[{invalid_revision}],\"sources\":42"),
    ] {
        let raw = format!("{{\"format\":\"native-run/1\",\"request\":{{\"models\":[],\"package\":{{\"file\":\"unavailable-package.json\"}},{members},\"call\":{{\"function\":\"unused\",\"arguments\":[]}}}}}}");
        let path = directory.path().join("priority.json");
        std::fs::write(&path, raw).unwrap();
        let refused = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .err()
            .unwrap();
        let value = refused.value().unwrap();
        assert_eq!(refused.category().exit_code(), 20);
        assert_eq!(value["code"], "invalid-request", "{value}");
        assert_eq!(value["stage"], "request", "{value}");
        assert_eq!(
            value["message"],
            "invalid clause request member /request/sources: repeated member"
        );
    }
}

#[test]
#[trace("TC-3019", "FR-1039-AC-9", "FR-1039-AC-12")]
fn package_remainder_caller_sources_admit_and_leave_tl_atoms_canonical() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = low(EVENT_POSITION, "always[0,3] holds( c.versionNumber <= 2 )");
    let models = compile_handoff(directory.path(), &unit);
    std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
    let request = json!({"format":"native-run/1", "request":{
        "models":models,"package":{"directory":"handoff"},"tl_lowering":{"clause":"Low"}}});
    let expected = write_request(directory.path(), &request);
    assert_eq!(expected.0, 0, "{:?}", expected);
    let mut selected = source("unit.native");
    selected["file"] = json!("caller.native");
    selected["document"] = json!("Other");
    selected["formal_revision"] = json!(7);
    for sources in [json!([]), json!([selected.clone()])] {
        let mut supplied = request.clone();
        supplied["request"]["sources"] = sources;
        assert_eq!(write_request(directory.path(), &supplied), expected);
    }
    selected["file"] = json!("unavailable.native");
    let mut unavailable = request.clone();
    unavailable["request"]["sources"] = json!([selected]);
    let (code, value) = write_request(directory.path(), &unavailable);
    assert_eq!(code, 20, "{value}");
    assert_eq!(value["stage"], "file", "{value}");
    assert_eq!(value["code"], "io-error", "{value}");
    for (bytes, blank_label) in [
        (vec![0xff], None),
        (b"\xef\xbb\xbftext".to_vec(), None),
        (b"text\0".to_vec(), None),
        (unit.as_bytes().to_vec(), Some("authority")),
        (unit.as_bytes().to_vec(), Some("identity")),
    ] {
        std::fs::write(directory.path().join("offered.native"), bytes).unwrap();
        let mut selected = source("offered.native");
        if let Some(label) = blank_label {
            selected[label] = json!("");
        }
        let from_source = write_request(
            directory.path(),
            &json!({"format":"native-run/1", "request":{
            "models":[],"program":{"source":selected},"call":{"function":"f","arguments":[]}}}),
        );
        let mut supplied = request.clone();
        supplied["request"]["sources"] = json!([selected]);
        let from_package = write_request(directory.path(), &supplied);
        assert_eq!(from_package, from_source);
        assert_eq!(from_package.0, 20, "{:?}", from_package);
        assert_eq!(from_package.1["stage"], "source", "{:?}", from_package);
    }
}

#[test]
#[trace("TC-3019", "FR-026-AC-8", "NFR-009-M-4")]
fn package_remainder_caller_sources_share_intake_limits_at_the_boundary() {
    use fsl_command::command::{prepare_run, RunLimits};
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = low(EVENT_POSITION, "always[0,3] holds(c.versionNumber <= 2)");
    let models = compile_handoff(directory.path(), &unit);
    std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
    let mut sources = Vec::new();
    for index in 0..61 {
        let mut selected = source("caller.native");
        selected["identity"] = json!(format!("test:render:{index}"));
        sources.push(selected);
    }
    let mut request = json!({"format":"native-run/1", "request":{
        "models":models,"package":{"file":"package.json","equality":"handoff/equality.json"},
        "sources":sources,"tl_lowering":{"clause":"Low"}}});
    let path = directory.path().join("bounded.json");
    let bytes = serde_json::to_vec(&request).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let dependent_bytes = [
        "package.json",
        "handoff/equality.json",
        "spine-model.semantic-ir.json",
    ]
    .into_iter()
    .map(|file| {
        std::fs::metadata(directory.path().join(file))
            .unwrap()
            .len() as usize
    })
    .sum::<usize>()
        + 61 * unit.len();
    let limits = RunLimits {
        request_bytes: bytes.len(),
        dependent_bytes,
    };
    prepare_run(&path, limits).expect("64 selected files exactly at both byte bounds admit");
    for (limits, setting) in [
        (
            RunLimits {
                request_bytes: bytes.len() - 1,
                ..limits
            },
            "request_bytes",
        ),
        (
            RunLimits {
                dependent_bytes: dependent_bytes - 1,
                ..limits
            },
            "dependent_bytes",
        ),
    ] {
        let refusal = prepare_run(&path, limits)
            .err()
            .expect("one byte over stops intake");
        assert_eq!(refusal.category().exit_code(), 22, "{refusal}");
        assert!(refusal.to_string().contains(setting), "{refusal}");
    }
    let mut selected = source("caller.native");
    selected["identity"] = json!("test:render:62");
    request["request"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(selected);
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let refusal = prepare_run(&path, RunLimits::default())
        .err()
        .expect("65 files stops intake");
    assert_eq!(refusal.category().exit_code(), 22, "{refusal}");
    let native = refusal.value().unwrap();
    assert_eq!(native["details"]["maximum"], 64, "{native}");
    assert_eq!(native["details"]["remaining"], 61, "{native}");
    assert_eq!(native["details"]["requested"], 62, "{native}");
    request["request"]["sources"] = json!([]);
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    prepare_run(&path, RunLimits::default()).expect("the next invocation has fresh budgets");
}

/// FR-1039 Outputs: timed record samples share the snapshot timed evaluator,
/// with a selected role clock and exact endpoint inclusion.
#[test]
#[trace("TC-3019", "FR-1039")]
fn package_clause_selections_timed_samples_match_source() {
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"filament.value.complete/v1\";\n",
        "profile t = \"filament.temporal.timed/v1\";\n",
        "dimension Time; unit ms : Time = rational(1, 1);\n",
        "record Reading { level: Int[0, 9]; }\n",
        "temporal Low using t over (r: Reading) clock \"plant\" on origin { ",
        "eventually(0 ms, 2 ms] holds(r.level = 1) }\n",
    );
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let clocks = json!({"plant": {"kind": "timed", "unit": "ms"}});
    let traces = [
        json!({"timed": {"prefix": [{"position": level_sample(0), "stamp": "0"},
            {"position": level_sample(1), "stamp": "2"}], "through": "2"}}),
        json!({"timed": {"prefix": [{"position": level_sample(0), "stamp": "0"}],
            "loop": [{"position": level_sample(1), "stamp": "2"}], "period": "3"}}),
    ];
    let clauses: Vec<_> = traces.into_iter().map(|trace| {
        let clause = json!({"selection": {"temporal": {"name": "Low", "trace": trace}}, "observations": {}});
        let result = write_request(directory.path(), &json!({"format": "native-run/1", "request": {
            "models": [], "program": {"source": source("unit.native")}, "clocks": clocks, "clause": clause}}));
        assert_eq!(result.0, 0, "{}", result.1);
        (clause, result)
    }).collect();
    compile_handoff_with_clocks(directory.path(), unit, Some(clocks));
    for (clause, expected) in clauses {
        let actual = write_request(
            directory.path(),
            &json!({"format": "native-run/1", "request": {
            "models": [], "package": {"directory": "handoff"}, "clause": clause}}),
        );
        assert_eq!(actual, expected);
    }
}

/// Actual shared atoms preserve support positions and caller whitespace; a
/// differing spelling selects the checked canonical label on both routes.
#[test]
#[trace("TC-1295", "FR-501-AC-31", "FR-1039-AC-10", "FR-283-AC-18")]
fn package_remainder_window_shared_atoms_match_source_and_require_caller_bytes() {
    let fixture = include_str!("../corpus/temporal/timestamped-window.native");
    let clocks = json!({"sensor": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let profile = "filament.temporal.timestamped-event.finite-window/v1";
    for progressed in [false, true] {
        for (body, label, candidates) in [
            ("always[0,5] holds( p.ok )", "holds( p.ok )", 1),
            (
                "always[0,5] (holds( p.ok ) and holds( p.ok ))",
                "holds( p.ok )",
                2,
            ),
            (
                "always[0,5] (holds( p.ok ) and holds(p.ok))",
                "holds(p.ok)",
                2,
            ),
        ] {
            let profile = if progressed { EVENT_POSITION } else { profile };
            let unit = fixture
                .replace(
                    "filament.temporal.timestamped-event.finite-window/v1",
                    profile,
                )
                .replace("eventually[0,7] holds(p.ok)", body);
            let duplicate = unit[unit.find("temporal StampedPulse").unwrap()..]
                .replace("temporal StampedPulse", "temporal OtherPulse");
            let unit = format!("{unit}\n{duplicate}");
            let clocks = if progressed {
                json!({"sensor": {"kind":"event_position", "sequence_authority":"pulse-sequence"}})
            } else {
                clocks.clone()
            };
            let directory = tempfile::tempdir().unwrap();
            spine::write(directory.path(), Case::Healthy).unwrap();
            std::fs::write(directory.path().join("unit.native"), &unit).unwrap();
            let observations: Vec<Value> = [(0, true), (2, false), (5, true)].into_iter().map(|(stamp, ok)| {
            json!({"position": {"sample": {"ok": {"boolean": ok}}}, "stamp": stamp})
        }).collect();
            let progress = json!({"watermark":5,"completeness":{"complete":{"through":5}},"scope":"closed","execution":"closed"});
            let binding =
                json!({"profile":profile,"clock":"sensor","parameters":clocks["sensor"].clone()});
            let trace = if progressed {
                json!({"progressed":{"positions":observations.iter().map(|observation| observation["position"].clone()).collect::<Vec<_>>(),
                "progress":progress,"lower_boundary":"authoritative-origin","binding":binding}})
            } else {
                json!({"timestamped":{"observations":observations,"progress":progress,"lower_boundary":"authoritative-origin","binding":binding}})
            };
            let clause = json!({"selection":{"temporal":{"name":"StampedPulse","trace":trace}},"observations":{}});
            let expected = write_request(
                directory.path(),
                &json!({"format":"native-run/1", "request": {
                    "models": [], "program": {"source": source("unit.native")}, "clocks": clocks, "clause": clause
                }}),
            );
            assert_eq!(expected.0, 10, "source {body}: {}", expected.1);
            assert_eq!(
                expected.1["support"],
                Value::Array(vec![json!({"position":1,"atom":label}); candidates]),
                "{}",
                expected.1
            );
            assert_eq!(expected.1["basis"], "closed-scope");
            compile_handoff_with_clocks(directory.path(), &unit, Some(clocks.clone()));
            let package_bytes = std::fs::read(directory.path().join("package.json")).unwrap();
            let package = fsl_package::admit_checked_package(
                &package_bytes,
                &Default::default(),
                fsl_replay::spine::SpineLimits::default().imports,
            )
            .expect("the genuine package strictly admits");
            let row = package
                .temporal_clause_names()
                .unwrap()
                .iter()
                .find(|row| row.name.as_ref() == "StampedPulse")
                .unwrap();
            let view =
                fsl_package::checked_views::temporal_subject(&package, &row.declaration).unwrap();
            let atom = view
                .formula_graph()
                .iter()
                .find(|entry| {
                    entry.kind() == Some(fsl_package::checked_views::FormulaOperation::Holds)
                })
                .unwrap()
                .node;
            let expression_regions: usize = package
                .source_map()
                .iter()
                .filter(|entry| {
                    entry.node_id == *atom
                        && entry.role == filament_contract_model::CheckedOccurrenceRole::Expression
                })
                .map(|entry| entry.regions.len())
                .sum();
            assert_eq!(
                expression_regions,
                candidates * 2,
                "{body}: both claims share the actual node and retain their own source rows"
            );
            std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
            let mut caller = source("unit.native");
            caller["file"] = json!("caller.native");
            caller["document"] = json!("DifferentDocument");
            caller["formal_revision"] = json!(9);
            let request = json!({"format":"native-run/1", "request": {"models":[],
            "package":{"directory":"handoff"}, "sources":[caller], "clause":clause}});
            package_window_adapter_controls(&package, directory.path(), &unit, label, candidates);
            let actual = write_request(directory.path(), &request);
            assert_eq!(actual, expected, "package {body}");
            fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&actual.1).unwrap())
                .expect("strict native read");
            for supplied in [
                Value::Null,
                json!([]),
                json!([{ "file":"caller.native", "authority":"agent-ix", "identity":"wrong", "document":"Other", "formal_revision":1 }]),
            ] {
                let mut missing = request.clone();
                if supplied.is_null() {
                    missing["request"]
                        .as_object_mut()
                        .unwrap()
                        .remove("sources");
                } else {
                    missing["request"]["sources"] = supplied;
                }
                let refusal = write_request(directory.path(), &missing);
                assert_eq!(refusal.0, 20, "{}", refusal.1);
                assert_eq!(refusal.1["stage"], "call", "{}", refusal.1);
                assert_eq!(refusal.1["code"], "invalid_runtime_input", "{}", refusal.1);
                assert_eq!(refusal.1["cause"], "missing-member", "{}", refusal.1);
                assert_eq!(refusal.1["details"]["field"], "sources", "{}", refusal.1);
            }
        }
    }
}

/// The byte-only adapter retains actual loci and obeys caller source and
/// finite progress/work boundaries, independently of command file intake.
fn package_window_adapter_controls(
    package: &filament_contract_model::CheckedPackageV2,
    directory: &Path,
    unit: &str,
    label: &str,
    support_count: usize,
) {
    use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef, SourceRegion};
    use fsl_foundation::{Source, SourceIdentity};
    use fsl_replay::spine::{self as replay, ClauseDisposition, TemporalVerdict};
    let limits = replay::SpineLimits::default();
    let mut budget = limits.imports.native_join_budget();
    let identities = fsl_package::ReadIdentityRows::read(
        &std::fs::read(directory.join("handoff/identities.json")).unwrap(),
        package,
        &mut budget,
    )
    .expect("the real handoff identity rows join");
    let equality = fsl_package::ReadEqualityRows::read(
        &std::fs::read(directory.join("handoff/equality.json")).unwrap(),
        package,
    )
    .expect("the real equality rows read");
    let bare = fsl_package::BarePackage::new(package)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the one bounded join admits the caller's actual witnesses");
    let packages = fsl_semantics::model::intake::PackageInput::default();
    let model = fsl_eval::simulation::model::PackageModelInput::of(&bare, &packages, limits.model)
        .expect("the admitted record package supplies its own type world");
    let row = package
        .temporal_clause_names()
        .unwrap()
        .iter()
        .find(|row| row.name.as_ref() == "StampedPulse")
        .unwrap();
    let view = fsl_package::checked_views::temporal_subject(package, &row.declaration).unwrap();
    let key = view.declaration_key().unwrap();
    let claim = OccurrenceKey::new(
        key.clone(),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence),
    );
    let clause = model
        .temporal_run_clause(&claim, limits.checking.nodes())
        .unwrap();
    let source_identity = SourceIdentity::new("agent-ix", "test:s1-16:unit.native");
    let source = |identity: SourceIdentity, bytes: &[u8]| {
        Source::read_typed(identity, "caller.native", bytes, unit.len()).unwrap_or_else(|refusal| {
            panic!("the valid caller source reads: {}", refusal.error.message)
        })
    };
    let sources = [source(source_identity.clone(), unit.as_bytes())];
    let region = |region: &filament_contract_model::CheckedSourceRegion| {
        SourceRegion::new(
            RawSourceRef::new(
                region.source.authority.as_ref(),
                region.source.identity.as_ref(),
            )
            .unwrap(),
            region.start,
            region.end,
        )
        .unwrap()
    };
    let declaration = region(&row.declaration_region);
    let atom = view
        .formula_graph()
        .iter()
        .find(|entry| entry.kind() == Some(fsl_package::checked_views::FormulaOperation::Holds))
        .unwrap()
        .node;
    let candidates = package
        .source_map()
        .iter()
        .filter(|entry| {
            entry.node_id == *atom
                && entry.role == filament_contract_model::CheckedOccurrenceRole::Expression
        })
        .flat_map(|entry| entry.regions.iter())
        .map(region)
        .filter(|region| {
            region.source() == declaration.source()
                && region.start() >= declaration.start()
                && region.end() <= declaration.end()
        })
        .collect::<Vec<_>>();
    let expected_region = match candidates.as_slice() {
        [only] => only.clone(),
        [_, _, ..] => declaration.clone(),
        _ => panic!("the genuine atom has caller regions"),
    };
    let first_atom = clause
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, fsl_semantics::check::FormulaNode::Holds { .. }))
        .unwrap();
    let projected = bare.project_holds(&key, first_atom).unwrap();
    let projection = projected.projection();
    let location = projection.location(projection.root()).unwrap();
    assert!(
        projection.source_at_location(&bare, location).is_err(),
        "global sharing does not identify the selected claim's occurrence"
    );
    let scoped = projection.source_at_location_within(&bare, location, &declaration);
    if support_count == 1 {
        let scoped = scoped.unwrap().unwrap();
        assert_eq!(scoped.source(), declaration.source());
        assert!(scoped.start() >= declaration.start() && scoped.end() <= declaration.end());
    } else {
        assert!(matches!(
            scoped,
            Err(fsl_package::occurrence_source::OccurrenceSourceRefusal::AmbiguousRegion)
        ));
    }
    let positions = [true, false, true]
        .map(|ok| replay::sample_position(&json!({"ok":{"boolean":ok}}).to_string()));
    let progress = replay::ProgressAssertion {
        watermark: 5,
        completeness: replay::ProgressCompleteness::Complete { through: 5 },
        scope: replay::ProgressClosure::Closed,
        execution: replay::ProgressClosure::Closed,
    };
    let parameters = if clause.profile().identity() == EVENT_POSITION {
        fsl_package::ClockConfiguration::EventPosition {
            sequence_authority: "pulse-sequence".into(),
        }
    } else {
        fsl_package::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into(),
        }
    };
    let binding = replay::TraceBinding {
        profile: clause.profile().identity().to_owned(),
        clock_name: "sensor".to_owned(),
        parameters,
    };
    let trace = if clause.profile().identity() == EVENT_POSITION {
        replay::TraceInput::Progressed {
            trace: positions.to_vec(),
            progress,
            lower_boundary: replay::ProgressLowerBoundary::AuthoritativeOrigin,
            binding,
        }
    } else {
        replay::TraceInput::Timestamped {
            observations: positions
                .iter()
                .cloned()
                .zip([0, 2, 5])
                .map(|(document, stamp)| replay::TimestampedObservation {
                    document,
                    stamp,
                    order_key: None,
                })
                .collect(),
            progress,
            window: Box::new(replay::TimestampedWindow {
                binding: Some(binding),
                authoritative_origin: true,
                cover: None,
                missing_valuations: Vec::new(),
                late: Default::default(),
            }),
        }
    };
    let accounting = fsl_replay::AccountingLimits::default().0;
    let run = |sources: &[Source], ledger, capacity, accounting| {
        replay::run_package_sampled_authenticated(
            &replay::PackageSampledRequest {
                package: &bare,
                sources,
                ledger,
                progress_capacity: capacity,
                evaluator: (&bare).into(),
                name: "StampedPulse",
                clause: Some((&clause, &key)),
                types: model.input().types,
                trace: &trace,
                accounting,
            },
            package,
        )
    };
    let full = run(
        &sources,
        None,
        limits.progress.covered_positions,
        accounting,
    )
    .expect("the real byte-only trace evaluates");
    assert!(matches!(
        full.disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at: 0 })
    ));
    let window = full.finite_window.as_ref().unwrap();
    assert_eq!(
        window.support,
        vec![
            replay::FiniteWindowSupport {
                position: 1,
                atom: label.to_owned()
            };
            support_count
        ]
    );
    assert!(!window.atom_regions.is_empty());
    for actual in window.atom_regions.values() {
        assert_eq!(actual, &expected_region);
    }
    let checked = crate::support::front_end::checked(
        source_identity.clone(),
        "unit.native",
        unit.as_bytes(),
        &packages,
        &replay::DependencyInput::default(),
    )
    .unwrap();
    let node = checked
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == "StampedPulse")
        .unwrap();
    let first = node
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, fsl_semantics::check::FormulaNode::Holds { .. }))
        .unwrap();
    let rendered =
        replay::render_clause_atom(checked.package(), &sources, "StampedPulse", first).unwrap();
    assert_eq!(
        (rendered.name.as_str(), &rendered.region),
        (label, &expected_region)
    );
    let ledger = full
        .ledger
        .as_ref()
        .expect("the real progress ledger is retained");
    let repeated = run(
        &sources,
        Some(ledger),
        limits.progress.covered_positions,
        accounting,
    )
    .expect("the carried ledger accepts the same facts");
    assert_eq!(repeated.ledger, full.ledger);
    assert_eq!(
        repeated.finite_window.as_ref().unwrap().support,
        window.support
    );
    let bounded = run(&sources, None, 2, accounting).unwrap();
    assert!(matches!(
        bounded.disposition,
        ClauseDisposition::Temporal(TemporalVerdict::ProgressLimitReached(_))
    ));
    let mut budget = accounting;
    budget.work_units = 0;
    let stopped = run(&sources, None, limits.progress.covered_positions, budget).unwrap();
    assert_eq!(stopped.category().exit_code(), 22);
    assert!(matches!(
        stopped.disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Incomplete(_))
    ));
    let mut invalid = vec![
        Vec::new(),
        vec![source(
            SourceIdentity::new("other", "reference"),
            unit.as_bytes(),
        )],
        vec![source(source_identity.clone(), b"short")],
    ];
    let mut selected_regions = candidates.clone();
    if candidates.len() > 1 {
        selected_regions.push(declaration);
    }
    for selected in selected_regions {
        let end = usize::try_from(selected.end()).unwrap();
        assert!(end > 0 && end < unit.len());
        let mut bytes = unit.as_bytes().to_vec();
        bytes.splice(end - 1..end + 1, "é".bytes());
        assert!(
            std::str::from_utf8(&bytes).is_ok(),
            "the caller supplies valid UTF-8 with a split region endpoint"
        );
        invalid.push(vec![source(source_identity.clone(), &bytes)]);
    }
    for missing in invalid {
        let refusal = run(
            &missing,
            None,
            limits.progress.covered_positions,
            accounting,
        )
        .unwrap_err();
        let replay::PackageSelectionNotRead::Run(refusal) = refusal else {
            panic!("the original caller-source refusal")
        };
        assert!(matches!(
            *refusal,
            replay::RunRefusal::SourceNotSupplied { .. }
        ));
        assert!(
            matches!(replay::render_clause_atom(checked.package(), &missing, "StampedPulse", first), Err(ref refusal) if matches!(**refusal, replay::RunRefusal::SourceNotSupplied { .. }))
        );
    }
}

fn package_refusal_unit(profile: &str) -> String {
    "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.model.complete/v1\";\n\
        profile t = \"filament.temporal.event-position.false-extension/v1\";\n\
        model Config = \"example/config-version\";\n\
        record Counter { level: Int[0, 9]; }\n\
        type Zero = Int[0, 0];\n\
        function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { xs }\n\
        type Total = Int[0, 4000];\n\
        function same using v(r: Config::ConfigVersion): Config::ConfigVersion pure { r }\n\
        function totalVersions using v(p: Population<Config::ConfigVersion>[4]): Total pure { sum<Total>(r in allInstances<Config::ConfigVersion>(p): deref(r).versionNumber) }\n\
        temporal C using t over (c: Counter) clock \"sensor\" on origin \
        { always[0,5] holds(size(allInstances<Config::ConfigVersion>(Config::config_history)) >= 0) }\n\
        temporal Other using t over (c: Counter) clock \"sensor\" on origin \
        { always[0,5] holds(size(allInstances<Config::ConfigVersion>(Config::config_history)) >= 0) }\n".replace(EVENT_POSITION, profile)
}

/// A real call refusal retains its source record and caller-rendered span.
#[test]
#[trace("TC-3019", "FR-1039-AC-11", "TC-451", "FR-100-AC-9")]
fn package_remainder_call_refusal_keeps_caller_source_locus() {
    assert_package_refusal_sources("call");
}

/// A refused native condition renders its actual selected-claim region.
#[test]
#[trace("TC-3019", "FR-1039-AC-11")]
fn package_remainder_condition_refusal_keeps_caller_source_locus() {
    assert_package_refusal_sources("clause");
}

fn assert_package_refusal_sources(member: &str) {
    use fsl_foundation::{Source, SourceIdentity, Span};
    let unit = package_refusal_unit(EVENT_POSITION);
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Incomplete).unwrap();
    std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
    let clocks =
        json!({"sensor":{"kind":"event_position","sequence_authority":"counter-sequence"}});
    let models = compile_handoff_with_clocks(directory.path(), &unit, Some(clocks.clone()));
    let offered = Source::read_typed(
        SourceIdentity::new("agent-ix", "test:s1-16:unit.native"),
        "caller.native",
        unit.as_bytes(),
        unit.len(),
    )
    .unwrap_or_else(|refusal| panic!("valid caller source: {}", refusal.error.message));
    let (selection, term) = match member {
        "call" => (
            json!({"function":"totalVersions","snapshot":"spine-snapshot.json",
            "arguments":[{"parameter":"p","value":{"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)}}]}),
            "allInstances<Config::ConfigVersion>(p)",
        ),
        "clause" => (
            json!({"selection":{"temporal":{"name":"C","trace":{"finite":[level_sample(1)]}}},
            "observations":{}}),
            "allInstances<Config::ConfigVersion>(Config::config_history)",
        ),
        _ => panic!("unknown fixture member {member}"),
    };
    let owner = if member == "call" {
        0
    } else {
        unit.find("temporal C").unwrap()
    };
    let start = owner + unit[owner..].find(term).unwrap();
    let span = serde_json::to_value(
        offered
            .locate(Span {
                start,
                end: start + term.len(),
            })
            .unwrap(),
    )
    .unwrap();
    let mut caller = source("unit.native");
    caller["file"] = json!("caller.native");
    caller["document"] = json!("CallerDocument");
    caller["formal_revision"] = json!(19);
    let mut source_request = json!({"format":"native-run/1","request":{
        "models":models,"program":{"source":caller},"clocks":clocks
    }});
    source_request["request"][member] = selection.clone();
    let expected = write_request(directory.path(), &source_request);
    assert_eq!(expected.0, 22, "source {member}: {}", expected.1);
    let outcome = |value: &Value| {
        if member == "call" {
            value["outcome"].clone()
        } else {
            value["disposition"]["outcome"].clone()
        }
    };
    let refusal = outcome(&expected.1);
    assert_eq!(refusal["kind"], "refused", "{}", expected.1);
    assert_eq!(refusal["code"], "incomplete_population", "{refusal}");
    assert_eq!(
        refusal["cause"],
        if member == "call" {
            "incomplete-scope"
        } else {
            "missing-required-artifact"
        },
        "{refusal}"
    );
    assert_eq!(
        refusal["fields"],
        json!({"population":format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)})
    );
    assert_eq!(
        refusal["locus"],
        json!({"file":"caller.native","span":span})
    );
    if member == "clause" {
        fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&expected.1).unwrap())
            .unwrap();
    }
    let mut request = json!({"format":"native-run/1","request":{
        "models":models,"package":{"directory":"handoff"},"sources":[caller]
    }});
    request["request"][member] = selection;
    for (document, revision) in [("CallerDocument", 19), ("AnotherDocument", 27)] {
        request["request"]["sources"][0]["document"] = json!(document);
        request["request"]["sources"][0]["formal_revision"] = json!(revision);
        let actual = write_request(directory.path(), &request);
        assert_eq!(actual.0, expected.0, "package {member}: {}", actual.1);
        assert_eq!(
            outcome(&actual.1),
            refusal,
            "package {member}: {}",
            actual.1
        );
        if member == "clause" {
            assert_eq!(actual, expected, "all original condition report fields");
            fsl_replay::spine::read_native_run_result(&serde_json::to_vec(&actual.1).unwrap())
                .unwrap();
        }
    }
    let assert_missing = |request: &Value| {
        let result = write_request(directory.path(), request);
        assert_eq!(result.0, 20, "{member}: {}", result.1);
        assert_eq!(result.1["code"], "invalid_runtime_input", "{}", result.1);
        assert_eq!(result.1["cause"], "missing-member", "{}", result.1);
        assert_eq!(result.1["stage"], "call", "{}", result.1);
        assert_eq!(result.1["details"], json!({"field":"sources"}));
        assert!(result.1["message"]
            .as_str()
            .unwrap()
            .contains("test:s1-16:unit.native"));
        assert!(result.1.get("outcome").is_none());
        assert!(result.1.get("disposition").is_none());
    };
    let mut missing = request.clone();
    missing["request"]
        .as_object_mut()
        .unwrap()
        .remove("sources");
    assert_missing(&missing);
    missing = request.clone();
    missing["request"]["sources"] = json!([]);
    assert_missing(&missing);
    missing = request.clone();
    missing["request"]["sources"][0]["identity"] = json!("wrong-reference");
    assert_missing(&missing);
    std::fs::write(directory.path().join("caller.native"), "short").unwrap();
    assert_missing(&request);
    for endpoint in [start, start + term.len()] {
        let mut split = unit.as_bytes().to_vec();
        split[endpoint - 1..endpoint + 1].copy_from_slice("é".as_bytes());
        assert!(
            std::str::from_utf8(&split).is_ok(),
            "the entire caller document remains valid UTF-8"
        );
        std::fs::write(directory.path().join("caller.native"), split).unwrap();
        assert_missing(&request);
    }
    std::fs::write(directory.path().join("caller.native"), &unit).unwrap();
}

// ---- FR-752/FR-753: the expansion candidate bound ----------------------

const CANDIDATE_PLAN: &str =
    "invariant Ready using v on Config::ConfigVersion at current { true }\n\
     verify Release using v {\n\
       check tested claim Ready method model_based_test domain Config::ConfigVersion bound 20;\n\
     }\n";

/// FR-753-AC-7's fixture: `ConfigVersion` with fields `x` and `y`, each
/// `Int[0, 1000]`, both modified by `attemptUpdate`; a complete population of
/// the objects `root` and `child` at 0 as `initial.json`; `Ready` and the
/// `Release` plan as `unit.native`. One expansion of the initial state has
/// 2 x 1001 x 1001 = 2004002 candidates.
fn two_field_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let path = directory.path().join("spine-model.semantic-ir.json");
    let mut model: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let object = model["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/ConfigVersion")
        .expect("the ConfigVersion type");
    let identity = |name: &str| format!("ix://example/config-version/ConfigVersion/{name}");
    let template = object["fields"][0].clone();
    let field = |name: &str| {
        let mut field = template.clone();
        field["name"] = json!(name);
        field["identity"] = json!(identity(name));
        field["origin"]["generated"]["generatorIdentity"] = json!(identity(name));
        field["origin"]["generated"]["inputIdentities"] = json!([identity(name)]);
        field
    };
    let parent = object["fields"][1].clone();
    object["fields"] = json!([field("x"), field("y"), parent]);
    object["operations"][0]["frame"]["modifies"] = json!([identity("x"), identity("y")]);
    std::fs::write(&path, serde_json::to_vec(&model).unwrap()).unwrap();
    let mut snapshot = snapshot_of(directory.path(), &["root", "child"], true);
    for object in snapshot["populations"][0]["objects"]
        .as_array_mut()
        .unwrap()
    {
        let fields = object["fields"].as_object_mut().unwrap();
        fields.remove("versionNumber");
        fields.insert("x".to_owned(), json!({"integer": "0"}));
        fields.insert("y".to_owned(), json!({"integer": "0"}));
    }
    std::fs::write(
        directory.path().join("initial.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{MODEL_HEADER}{CANDIDATE_PLAN}"),
    )
    .unwrap();
    directory
}

fn run_two_field(member: &str, value: Value) -> (u8, Value) {
    let directory = two_field_directory();
    let mut request = json!({
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("spine-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")}});
    request[member] = value;
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

/// 200000 is the published bound; the expansion's 2004002 candidates pass it.
fn candidate_limit_reached() -> Value {
    json!({"kind": "limit-reached", "limit": "max_candidates", "value": 200_000,
        "setting": "expansion.max_candidates"})
}

/// FR-753-AC-7 and FR-752-AC-13: a plan step with no caller bound expands
/// under 200000 candidates and settles `incomplete` at the first expansion
/// past it, exit 22.
#[test]
#[trace("FR-753-AC-7")]
#[trace("FR-752-AC-13")]
fn a_plan_step_past_the_candidate_bound_settles_incomplete() {
    let (exit, value) = run_two_field(
        "verification_plan",
        json!({"plan": "Release", "execute": true, "initial": "initial.json",
            "seed": 7, "max_trace_length": 8}),
    );
    let step = &value["verification_plan"]["dispositions"]["tested"];
    assert_eq!(step["status"], "incomplete", "{value}");
    assert_eq!(
        step["result"]["cause"],
        candidate_limit_reached(),
        "{value}"
    );
    assert_eq!(exit, 22, "{value}");
}

/// FR-752-AC-16: the same fixture as a direct `model_based_test` member.
#[test]
#[trace("FR-752-AC-16")]
fn a_direct_member_past_the_candidate_bound_settles_incomplete() {
    let (exit, value) = run_two_field(
        "model_based_test",
        json!({"clause": "Ready", "seed": 7, "max_cases": 20, "max_trace_length": 8,
            "initial": "initial.json"}),
    );
    let record = &value["model_based_test"];
    assert_eq!(record["status"], "incomplete", "{value}");
    assert_eq!(record["cause"], candidate_limit_reached(), "{value}");
    assert_eq!(exit, 22, "{value}");
}
