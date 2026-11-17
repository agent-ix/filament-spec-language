// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface stubs for the plan outcome document, the plan search
//! limits, plan refusals and the reference planning examples (FR-1010 to
//! FR-1014), and for the TLA+ and Lean export outcome, mappers and golden
//! corpus (FR-1015 to FR-1018). Each feeds source and a `native-run/1`
//! request through the command adapter (`fsl_command::command::run`) and
//! asserts on the output document, because the engines do not exist yet.

use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w21:{file}"),
        "document": "W21", "formal_revision": 1})
}

/// Writes the spine files, `unit` as `unit.native` and a `native-run/1`
/// request whose `request` carries `member`, runs it through the command
/// adapter, and returns its exit code and document.
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

fn names(value: &Value, member: &str) {
    assert!(
        value.to_string().contains(member),
        "names {member}: {value}"
    );
}

/// A plan synthesis over the spine model with `lengths`, `goal` and an
/// optional objective, plus the two temporal clauses a policy may name.
fn plan_unit(lengths: &str, objective: &str, goal: &str) -> String {
    format!(
        "{MODEL_HEADER}type Lengths = {lengths};\n\
         temporal SomeTime using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 3) }}\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         {objective} satisfies {{ {goal} }};\n"
    )
}

fn plan(unit: &str, synthesis: Value) -> (u8, Value) {
    let mut member = json!({"declaration": "Reach", "initial": "spine-snapshot.json"});
    for (key, value) in synthesis.as_object().into_iter().flatten() {
        member[key] = value.clone();
    }
    run(unit, true, ("synthesis", member))
}

// ---- QSL-1978: planner and optimizer ------------------------------------

/// FR-1010-AC-1 and FSpec FR-805-AC-1: an optimal plan writes the
/// `filament.plan-outcome/v1` document with all its members and exits 0.
#[test]
#[trace("TC-2860", "FR-1010-AC-1")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-1")]
#[trace("FSpec-TC-1950", "FSpec-FR-800-AC-1")]
#[ignore = "QSL-1978: the plan outcome document is not implemented yet"]
fn an_optimal_plan_writes_the_plan_outcome_document() {
    let (exit, value) = plan(&plan_unit("Int[0, 3]", "", "Reach.length >= 1"), json!({}));
    let outcome = &value["synthesis"];
    assert_eq!(outcome["format"], "filament.plan-outcome/v1", "{value}");
    assert_eq!(outcome["status"], "optimal", "{value}");
    for member in [
        "objective",
        "plan",
        "replay",
        "certificate",
        "certification",
        "explored",
        "limits",
    ] {
        assert!(outcome.get(member).is_some(), "{member}: {value}");
    }
    assert_eq!(outcome["replay"]["outcome"], "reproduced", "{value}");
    let steps = outcome["plan"]["steps"].as_array().expect("steps");
    assert_eq!(steps[0]["index"], 0, "{value}");
    assert_eq!(
        outcome["plan"]["states"].as_array().map(Vec::len),
        Some(steps.len() + 1),
        "{value}"
    );
    assert_eq!(exit, 0, "{value}");
}

/// FR-1010-AC-2 and FSpec FR-805-AC-2/AC-4, FR-806-AC-5: `infeasible`
/// writes a null plan and exits 10; an unknown member refuses, exit 20.
#[test]
#[trace("TC-2860", "FR-1010-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-4")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-5")]
#[ignore = "QSL-1978: plan outcome exits and the strict plan request reader are not implemented yet"]
fn an_infeasible_plan_exits_ten_and_a_malformed_request_refuses() {
    let unit = plan_unit("Int[0, 0]", "", "Reach.length >= 1");
    let (exit, value) = plan(&unit, json!({}));
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert!(value["synthesis"]["plan"].is_null(), "{value}");
    assert_eq!(exit, 10, "{value}");
    let (exit, value) = plan(&unit, json!({"strategy": "greedy"}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "unknown-member");
    names(&value, "/strategy");
}

/// FR-1011-AC-1, FSpec FR-801-AC-1/AC-6: the plan search stops at its own
/// named limits, distinct from the verification planner's.
#[test]
#[trace("TC-2861", "FR-1011-AC-1")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-1")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-6")]
#[ignore = "QSL-1978: plan_search limits are not implemented yet"]
fn the_plan_search_stops_at_its_named_limits() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 3");
    for (member, setting) in [
        ("max_work", "plan_search.max_work"),
        ("max_states", "plan_search.max_states"),
    ] {
        let (exit, value) = plan(&unit, json!({"limits": {member: 1}}));
        let outcome = &value["synthesis"];
        assert_eq!(outcome["status"], "incomplete", "{value}");
        assert_eq!(outcome["cause"]["setting"], setting, "{value}");
        assert_eq!(exit, 22, "{value}");
    }
}

/// FR-1011-AC-2, FSpec FR-801-AC-2: an exhausted evaluation meter names
/// its `accounting` setting.
#[test]
#[trace("TC-2861", "FR-1011-AC-2")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-2")]
#[ignore = "QSL-1978: the plan search does not charge the evaluation meter yet"]
fn an_exhausted_evaluation_meter_names_its_accounting_setting() {
    let (_, value) = plan(
        &plan_unit("Int[0, 3]", "", "Reach.length >= 3"),
        json!({"accounting": {"work_units": 1}}),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
    assert_eq!(
        value["synthesis"]["cause"]["setting"], "accounting.work_units",
        "{value}"
    );
}

/// FR-1012-AC-1, FSpec FR-803-AC-2: a non-safety policy is evaluated on
/// each completed plan, never refused.
#[test]
#[trace("TC-2862", "FR-1012-AC-1")]
#[trace("FSpec-TC-1953", "FSpec-FR-803-AC-2")]
#[ignore = "QSL-1978: non-safety plan policies are not implemented yet"]
fn a_non_safety_policy_is_evaluated_on_each_completed_plan() {
    let (exit, value) = plan(
        &plan_unit("Int[0, 3]", "", "Reach.length >= 1"),
        json!({"policy": ["SomeTime"]}),
    );
    assert_ne!(exit, 20, "{value}");
    let outcome = &value["synthesis"];
    if outcome["status"] == "optimal" {
        let visits = outcome["plan"]["states"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|state| state.to_string().contains("\"versionNumber\":3"));
        assert!(visits, "{value}");
    } else {
        assert_eq!(outcome["status"], "infeasible", "{value}");
    }
}

/// FR-1012-AC-2, FSpec FR-804-AC-4: equal requests give byte-identical
/// plan outcome documents.
#[test]
#[trace("TC-2862", "FR-1012-AC-2")]
#[trace("FSpec-TC-1954", "FSpec-FR-804-AC-4")]
#[ignore = "QSL-1978: deterministic plan outcomes are not implemented yet"]
fn equal_plan_requests_give_byte_identical_outcomes() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 2");
    let (_, first) = plan(&unit, json!({}));
    let (_, second) = plan(&unit, json!({}));
    assert_eq!(first["synthesis"]["status"], "optimal", "{first}");
    assert_eq!(first.to_string(), second.to_string());
}

/// FR-1012-AC-3, FSpec FR-802-AC-1 and FR-805-AC-3: a decimal per-step
/// cost is exact, certified by `potential`, and never a JSON float.
#[test]
#[trace("TC-2862", "FR-1012-AC-3")]
#[trace("FSpec-TC-1952", "FSpec-FR-802-AC-1")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-3")]
#[ignore = "QSL-1978: exact decimal plan objectives are not implemented yet"]
fn a_decimal_step_cost_is_exact_and_certified() {
    let (_, value) = plan(
        &plan_unit(
            "Int[0, 3]",
            "minimize Reach.steps.sum(s => 0.10)",
            "Reach.length >= 3",
        ),
        json!({}),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert!(!outcome["objective"]["value"].is_f64(), "{value}");
    names(&outcome["objective"]["value"], "0.30");
    assert_eq!(outcome["certificate"]["type"], "potential", "{value}");
}

/// FR-1013-AC-1, FSpec FR-806-AC-2: a negative plan length and a read of
/// `elapsed` over an untimed model refuse at S3.
#[test]
#[trace("TC-2863", "FR-1013-AC-1")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-2")]
#[ignore = "QSL-1978: plan declaration refusals at S3 are not implemented yet"]
fn plan_declarations_refuse_at_check() {
    let (exit, value) = plan(&plan_unit("Int[-1, 3]", "", "Reach.length >= 1"), json!({}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "type-mismatch");
    let (exit, value) = plan(
        &plan_unit("Int[0, 3]", "minimize Reach.elapsed", "Reach.length >= 1"),
        json!({}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "missing-name");
    names(&value, "elapsed");
}

/// FR-1014-AC-1, FSpec FR-810-AC-1 to AC-6: each reference planning
/// example plans with its stated result and exact objective.
#[test]
#[trace("TC-2864", "FR-1014-AC-1")]
#[trace("FSpec-TC-1960", "FSpec-FR-810-AC-1")]
#[trace("FSpec-TC-1961", "FSpec-FR-810-AC-2")]
#[trace("FSpec-TC-1962", "FSpec-FR-810-AC-3")]
#[trace("FSpec-TC-1963", "FSpec-FR-810-AC-4")]
#[trace("FSpec-TC-1964", "FSpec-FR-810-AC-5")]
#[trace("FSpec-TC-1965", "FSpec-FR-810-AC-6")]
#[ignore = "QSL-1978: the six reference planning examples are not published yet"]
fn the_six_reference_planning_examples_plan_as_specified() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/planning");
    for (case, objective) in [
        ("o1-rollout", "15"),
        ("o2-battery-dispatch", "0.30"),
        ("o3-production-schedule", "60"),
        ("o4-staffing", "20"),
        ("o5-robot-routing", "10"),
        ("o6-autoscaling", "0.80"),
    ] {
        let request = root.join(case).join("request.json");
        assert!(request.is_file(), "missing example {}", request.display());
        let (exit, value) = match fsl_command::command::run(&request, &crate::negotiation::Fr290) {
            Ok(result) => (
                result.exit_code,
                serde_json::from_str::<Value>(&result.value.to_string()).unwrap(),
            ),
            Err(error) => (error.category().exit_code(), error.value().unwrap()),
        };
        assert_eq!(exit, 0, "{case}: {value}");
        assert_eq!(value["synthesis"]["status"], "optimal", "{case}: {value}");
        names(&value["synthesis"]["objective"]["value"], objective);
    }
}

// ---- QSL-1979: TLA+ and Lean export -------------------------------------

fn low_unit() -> String {
    format!("{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n")
}

fn export(unit: &str, with_model: bool, member: Value) -> (u8, Value) {
    run(unit, with_model, ("export", member))
}

/// FR-1015-AC-1, FSpec FR-811-AC-2/AC-3 and FR-817-AC-3: below the
/// default floor the export refuses with no text, exit 21; under `any` it
/// assembles with the unrepresented record claiming no region.
#[test]
#[trace("TC-2865", "FR-1015-AC-1")]
#[trace("FSpec-TC-1970", "FSpec-FR-811-AC-2")]
#[trace("FSpec-TC-1970", "FSpec-FR-811-AC-3")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-3")]
#[ignore = "QSL-1979: the export fidelity floor is not implemented yet"]
fn an_export_below_its_fidelity_floor_refuses_with_no_text() {
    let request = |floor: Option<&str>| {
        let mut member =
            json!({"profile": "filament.output.tla2/v1", "obligations": ["SlowEnough"]});
        if let Some(floor) = floor {
            member["minimum_fidelity"] = json!(floor);
        }
        export(BRAKING, false, member)
    };
    let (exit, value) = request(None);
    assert_eq!(
        value["export"]["format"], "filament.export-outcome/v1",
        "{value}"
    );
    assert_eq!(value["export"]["status"], "refused", "{value}");
    names(&value["export"], "below-fidelity-floor");
    assert!(value["export"].get("target_text").is_none(), "{value}");
    assert_eq!(exit, 21, "{value}");
    let (exit, value) = request(Some("any"));
    assert_eq!(value["export"]["status"], "assembled", "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "unrepresented", "{value}");
    assert_eq!(record["regions"], json!([]), "{value}");
    assert_eq!(exit, 0, "{value}");
}

/// FR-1015-AC-2, FSpec FR-817-AC-4: a denied output-byte charge stops the
/// export `incomplete` naming `export.output_bytes`, exit 22.
#[test]
#[trace("TC-2865", "FR-1015-AC-2")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-4")]
#[ignore = "QSL-1979: export limits are not implemented yet"]
fn an_export_stops_at_its_output_byte_limit() {
    let (exit, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.tla2/v1", "obligations": ["Low"],
            "limits": {"output_bytes": 1}}),
    );
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    names(&value["export"], "export.output_bytes");
    assert!(value["export"].get("target_text").is_none(), "{value}");
    assert_eq!(exit, 22, "{value}");
}

/// FR-1016-AC-1, FSpec FR-812-AC-1, FR-814-AC-3 and FR-817-AC-1: the TLA+
/// module is canonical ASCII with one `fsl:` comment per definition.
#[test]
#[trace("TC-2866", "FR-1016-AC-1")]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-1")]
#[trace("FSpec-TC-1973", "FSpec-FR-814-AC-3")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-1")]
#[ignore = "QSL-1979: the TLA+ mapper is not implemented yet"]
fn the_tla_module_is_canonical_text() {
    let (exit, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.tla2/v1", "obligations": ["Low"]}),
    );
    let text = value["export"]["target_text"]
        .as_str()
        .expect("target text");
    assert!(text.starts_with("---- MODULE M_"), "{text}");
    assert!(text.is_ascii(), "{text}");
    assert!(
        text.ends_with("====\n") && !text.ends_with("\n\n"),
        "{text}"
    );
    for part in [
        "VARIABLES",
        "Init ==",
        "Next ==",
        "Spec ==",
        "WF_vars(Next)",
        "\\* fsl: ",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    assert!(!text.lines().any(|line| line.ends_with(' ')), "{text}");
    assert!(value["export"]["name_map"].is_array(), "{value}");
    assert_eq!(exit, 0, "{value}");
}

/// FR-1017-AC-1, FSpec FR-813-AC-5 and FR-651-AC-2: the Lean file states
/// the obligation as a proposition with no theorem, proof or `sorry`.
#[test]
#[trace("TC-2867", "FR-1017-AC-1")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-5")]
#[ignore = "QSL-1979: the Lean mapper is not implemented yet"]
fn the_lean_file_states_obligations_without_proofs() {
    let (_, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.lean4/v1", "obligations": ["Low"]}),
    );
    let text = value["export"]["target_text"]
        .as_str()
        .expect("target text");
    assert!(text.is_ascii(), "{text}");
    for part in [
        "structure State",
        "def Init",
        "def Step",
        "def Ob_",
        "-- fsl: ",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    assert!(
        !text.contains("theorem") && !text.contains("sorry"),
        "{text}"
    );
}

/// FR-1018-AC-1, FSpec FR-818-AC-1: the counter export equals its reviewed
/// golden files byte for byte.
#[test]
#[trace("TC-2868", "FR-1018-AC-1")]
#[trace("FSpec-TC-1977", "FSpec-FR-818-AC-1")]
#[ignore = "QSL-1979: the export golden corpus is not published yet"]
fn the_counter_export_equals_its_golden_files() {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export");
    for (profile, file) in [
        ("filament.output.tla2/v1", "counter.tla"),
        ("filament.output.lean4/v1", "counter.lean"),
    ] {
        let expected = std::fs::read_to_string(golden.join(file))
            .unwrap_or_else(|error| panic!("golden {file}: {error}"));
        let (_, value) = export(
            &low_unit(),
            true,
            json!({"profile": profile, "obligations": ["Low"]}),
        );
        assert_eq!(value["export"]["target_text"], json!(expected), "{file}");
    }
}

/// FR-1018-AC-2, FSpec FR-818-AC-2: with TLC present its state count and
/// verdict equal FSL's; without it the oracle names the missing tool.
#[test]
#[trace("TC-2868", "FR-1018-AC-2")]
#[trace("FSpec-TC-1977", "FSpec-FR-818-AC-2")]
#[ignore = "QSL-1979: the TLC export oracle is not implemented yet"]
fn the_tlc_oracle_agrees_or_names_the_missing_tool() {
    let (_, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.tla2/v1", "obligations": ["Low"],
            "observer": "tlc", "initial": "spine-snapshot.json"}),
    );
    let observer = &value["export"]["observer"];
    assert!(observer.is_object(), "{value}");
    if observer["outcome"] == "tool-not-found" {
        names(observer, "tlc not found");
    } else {
        assert_eq!(
            observer["reachable_states"], value["export"]["native"]["reachable_states"],
            "{value}"
        );
        assert_eq!(
            observer["verdict"], value["export"]["native"]["verdict"],
            "{value}"
        );
    }
}

// ---- QSL-2234: reference hybrid provider --------------------------------

/// `Braking` with the violated claim `TooSlow` beside `SlowEnough`.
fn braking_pair() -> String {
    BRAKING.replace("always holds(h.v < 30)", "always [0, 10] holds(h.v < 30)")
        + "temporal TooSlow using p over (h: Braking) clock \"model-time\" \
           on origin { always [0, 10] holds(h.v > 5) }\n"
}

fn reference_analyze(unit: &str, claim: &str) -> (u8, Value) {
    run(
        unit,
        false,
        (
            "hybrid_analyze",
            json!({"claim": claim, "engines": "fsl_with_reference_hybrid"}),
        ),
    )
}

/// FR-1019-AC-1 and AC-2: the reference provider proves `SlowEnough` with a
/// certified outward-rounded flowpipe and refutes `TooSlow` by exact replay.
#[test]
#[trace("TC-2869", "FR-1019-AC-1")]
#[trace("TC-2869", "FR-1019-AC-2")]
#[ignore = "QSL-2234: the first-party reference hybrid provider is not implemented yet"]
fn the_reference_hybrid_provider_proves_and_refutes() {
    let unit = braking_pair();
    let (_, value) = reference_analyze(&unit, "SlowEnough");
    let result = &value["hybrid_result"];
    assert_eq!(result["value"], "proved", "{value}");
    assert_eq!(
        result["basis"]["method"], "flowpipe-outward-rounded",
        "{value}"
    );
    assert_eq!(result["certification"], "certified", "{value}");
    assert_eq!(result["backend"], "fsl-hybrid-reference", "{value}");
    let (_, value) = reference_analyze(&unit, "TooSlow");
    assert_eq!(value["hybrid_result"]["value"], "refuted", "{value}");
    assert_eq!(
        value["hybrid_result"]["counterexamples"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "{value}"
    );
}

/// FR-1019-AC-3: a non-polynomial flow is outside the reference provider's
/// class and settles `unsupported` with a located `expression-form` cause.
#[test]
#[trace("TC-2869", "FR-1019-AC-3")]
#[ignore = "QSL-2234: the reference hybrid provider's class check is not implemented yet"]
fn a_non_polynomial_flow_is_a_located_unsupported() {
    let unit = braking_pair().replace("flow { v' = -a; a' = 0; }", "flow { v' = sin(v); a' = 0; }");
    let (_, value) = reference_analyze(&unit, "SlowEnough");
    assert_eq!(value["hybrid_result"]["value"], "unsupported", "{value}");
    names(&value["hybrid_result"], "expression-form");
    names(&value["hybrid_result"], "Moving");
    names(&value["hybrid_result"], "locus");
}

// ---- QSL-1978: hybrid plans ----------------------------------------------

const TANK: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Tank using p {\n\
       mode Fill invariant { h <= 10 } flow { h' = 1; }\n\
         transition to Drain when { h >= 8 } reset { h = h };\n\
       mode Drain invariant { h >= 0 } flow { h' = -2; }\n\
     }\n\
     type Lengths = Int[0, 1];\n";

/// FR-1021-AC-1, FSpec FR-819-AC-1 to AC-4: a hybrid model plans
/// `feasible` through its provider, `infeasible` with a certified closure,
/// `unsupported` with no provider, and needs a `dwell_grid`.
#[test]
#[trace("TC-2871", "FR-1021-AC-1")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-1")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-2")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-3")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-4")]
#[ignore = "QSL-1978: hybrid plan synthesis is not implemented yet"]
fn a_hybrid_model_plans_through_its_provider() {
    let unit = |goal: &str| {
        format!(
            "{TANK}synthesis Reach using p grammar Tank domain Lengths satisfies {{ {goal} }};\n"
        )
    };
    let member = |grid: Option<u32>, engines: &str| {
        let mut synthesis = json!({"declaration": "Reach", "engines": engines});
        if let Some(grid) = grid {
            synthesis["dwell_grid"] = json!(grid);
        }
        synthesis
    };
    let (_, value) = run(
        &unit("Reach.final.mode = Tank::Mode::Drain"),
        false,
        ("synthesis", member(Some(1), "fsl_with_reference_hybrid")),
    );
    assert_eq!(value["synthesis"]["status"], "feasible", "{value}");
    assert!(value["synthesis"]["bound"].is_null(), "{value}");
    let (_, value) = run(
        &unit("Reach.final.h >= 12"),
        false,
        ("synthesis", member(Some(1), "fsl_with_reference_hybrid")),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert_eq!(value["synthesis"]["certification"], "certified", "{value}");
    let (_, value) = run(
        &unit("Reach.final.mode = Tank::Mode::Drain"),
        false,
        ("synthesis", member(Some(1), "fsl")),
    );
    names(&value, "hybrid-dynamics");
    let (exit, value) = run(
        &unit("Reach.final.mode = Tank::Mode::Drain"),
        false,
        ("synthesis", member(None, "fsl_with_reference_hybrid")),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "dwell_grid");
}

// ---- QSL-2247: previous through a step-for-step refinement ----------------

const COUNTER_REFINEMENT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"filament.temporal.infinite-trace/v1\";\n\
     model Spec = \"example/spec-counter\";\n\
     model Impl = \"example/impl-counter\";\n\
     temporal PrevSafe using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value = 2) implies once[1,1] holds(c.value = 1)) }\n\
     refinement R using t abstract Spec::Counter concrete Impl::Counter {\n\
       step Impl::Counter::inc -> Spec::Counter::inc(self);\n\
     }\n";

/// FR-1020-AC-1/AC-2, FSpec FR-380-AC-9: a proved `once[1,1]` clause
/// transfers through a step-for-step refinement, and not through one that
/// maps a concrete step to `stutter`.
#[test]
#[trace("TC-2870", "FR-1020-AC-1")]
#[trace("TC-2870", "FR-1020-AC-2")]
#[trace("FSpec-TC-1042", "FSpec-FR-380-AC-9")]
#[ignore = "QSL-2247: step-for-step transfer of previous through a model refinement is not implemented yet"]
fn previous_transfers_only_through_a_step_for_step_refinement() {
    let transfer = |unit: &str| {
        run(
            unit,
            false,
            (
                "refinement_transfer",
                json!({"claim": "PrevSafe", "refinement": "R"}),
            ),
        )
        .1
    };
    let value = transfer(COUNTER_REFINEMENT);
    assert_eq!(value["refinement_transfer"]["value"], "proved", "{value}");
    assert_eq!(
        value["refinement_transfer"]["basis"], "decisive-witness",
        "{value}"
    );
    let stuttering = COUNTER_REFINEMENT.replace(
        "Spec::Counter::inc(self);\n",
        "Spec::Counter::inc(self);\nstep Impl::Counter::retry -> stutter;\n",
    );
    let value = transfer(&stuttering);
    assert_eq!(
        value["refinement_transfer"]["value"], "inconclusive",
        "{value}"
    );
    names(&value, "transfer-not-preserving");
    names(&value, "once[1,1]");
}
