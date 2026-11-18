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

/// FSpec FR-801-AC-7: every plan search limit has its published default, a
/// request overrides one member only, and an invalid value refuses at its
/// pointer before any search.
#[test]
#[trace("FSpec-TC-2003", "FSpec-FR-801-AC-7")]
#[ignore = "QSL-1978: plan_search limit defaults and their refusals are not implemented yet"]
fn plan_search_limits_have_published_defaults_and_refuse_invalid_values() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 1");
    let defaults = json!({
        "plan_search.max_states": 10_000_000_u64,
        "plan_search.max_work": 100_000_000_u64,
        "plan_search.max_certificate_entries": 16_777_216_u64,
        "plan_search.max_check_work": 100_000_000_u64,
        "plan_search.max_state_key_bytes": 1_048_576_u64,
    });
    let (_, value) = plan(&unit, json!({}));
    assert_eq!(value["synthesis"]["limits"], defaults, "{value}");

    let (_, value) = plan(&unit, json!({"limits": {"max_states": 500}}));
    let mut overridden = defaults.clone();
    overridden["plan_search.max_states"] = json!(500);
    assert_eq!(value["synthesis"]["limits"], overridden, "{value}");

    for (limit, cause) in [
        (json!(0), "invalid-value"),
        (json!(-1), "invalid-value"),
        (json!(1.5), "invalid-value"),
        (
            serde_json::from_str::<Value>("18446744073709551616").unwrap(),
            "invalid-value",
        ),
        (json!("10"), "wrong-value-kind"),
    ] {
        let (exit, value) = plan(&unit, json!({"limits": {"max_work": limit}}));
        assert_eq!(exit, 20, "{value}");
        names(&value, cause);
        names(&value, "/limits/max_work");
        assert!(value["synthesis"].get("status").is_none(), "{value}");
    }
    let (exit, value) = plan(&unit, json!({"limits": {"max_steps": 1}}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "unknown-member");
    names(&value, "/limits/max_steps");
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

// ---- QSL-2247: refinement transfer through a native run -------------------

/// The clauses to transfer, the contracts of the third counter model and the
/// identity refinement `ThirdRefinesSpec`, which maps the one concrete
/// operation to the one abstract operation and so is step-for-step.
const TRANSFER_DECLARATIONS: &str =
    "pre ThirdCanInc using v on Third::Counter::inc { self.value < 2 }\n\
     post ThirdInc using v on Third::Counter::inc \
     { pre(self.value) < 2 and self.value = pre(self.value) + 1 }\n\
     temporal PrevSafe using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value = 2) implies once[1,1] holds(c.value = 1)) }\n\
     temporal SoonTwo using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value = 1) implies eventually[0,2] holds(c.value = 2)) }\n\
     temporal AtMostTwo using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 2) }\n\
     temporal OverThird using inf over (c: Third::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 2) }\n\
     refinement ThirdRefinesSpec using inf abstract Spec concrete Third {\n\
       population Spec::counters from Third::counters;\n\
       object Spec::Counter from Third::Counter { value = self.value; }\n\
       step Third::Counter::inc -> Spec::Counter::inc(self);\n\
     }\n";

/// An initialization snapshot of the one counter `c` of `package`.
fn counter_snapshot(package: &str, fields: Value) -> Value {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": format!("{package}/start")},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/counters"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Counter"), "fields": fields}]}],
    })
}

/// A `refinement_transfer` member over `refinement` between `abstract` and
/// `concrete` for `clauses`, with the fixture's initial snapshots.
fn transfer_member(
    refinement: &str,
    (abstract_model, concrete): (&str, &str),
    clauses: &[&str],
) -> Value {
    let initial = |model: &str| match model {
        "Spec" => "spec-0",
        "Impl" => "cas-0",
        _ => "third-0",
    };
    json!({
        "refinement": refinement,
        "abstract": {"model": abstract_model, "initial": [initial(abstract_model)]},
        "concrete": {"model": concrete, "initial": [initial(concrete)]},
        "clauses": clauses,
        "observations": {"spec-0": "spec-0.json", "cas-0": "cas-0.json", "third-0": "third-0.json"},
    })
}

/// Writes ADR-020 §8's counter models, a unit with `CasRefinesCounter` and
/// [`TRANSFER_DECLARATIONS`], the initial snapshots and a `native-run/1`
/// request carrying `members` (with the `Spec` model's entry dropped when
/// `supply_spec` is false), runs it and returns its exit code and document.
fn transfer_run(members: &[(&str, Value)], supply_spec: bool) -> (u8, Value) {
    use crate::support::refinement_cas::{self, Variant};
    let directory = tempfile::tempdir().unwrap();
    let declarations = format!(
        "{TRANSFER_DECLARATIONS}{}",
        refinement_cas::cas_refines_counter()
    );
    let unit = refinement_cas::unit_text(
        &refinement_cas::packages(Variant::CAS),
        &declarations,
        Variant::CAS,
    );
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let identities = [
        refinement_cas::SPEC,
        refinement_cas::CAS,
        refinement_cas::THIRD,
    ];
    let mut models = Vec::new();
    for (index, document) in refinement_cas::documents(Variant::CAS).iter().enumerate() {
        let file = format!("model-{index}.json");
        std::fs::write(directory.path().join(&file), document).unwrap();
        if index > 0 || supply_spec {
            models.push(
                json!({"format": "semantic-ir/2.0.0", "source": {"file": file,
                "authority": "agent-ix", "identity": identities[index],
                "document": format!("Model{index}"), "formal_revision": 1}}),
            );
        }
    }
    let zero = json!({"integer": "0"});
    let snapshots = [
        (
            "spec-0.json",
            counter_snapshot(refinement_cas::SPEC, json!({"value": zero})),
        ),
        (
            "third-0.json",
            counter_snapshot(refinement_cas::THIRD, json!({"value": zero})),
        ),
        (
            "cas-0.json",
            counter_snapshot(
                refinement_cas::CAS,
                json!({"value": zero, "tmpA": zero, "busyA": {"boolean": false},
                    "tmpB": zero, "busyB": {"boolean": false}}),
            ),
        ),
    ];
    for (file, snapshot) in snapshots {
        std::fs::write(directory.path().join(file), snapshot.to_string()).unwrap();
    }
    let mut request = json!({"models": models, "program": {"source": source("unit.native")}});
    for (name, member) in members {
        request[*name] = member.clone();
    }
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

/// The transfer result of `clause` in a `refinement_transfer` outcome.
fn transfer<'a>(value: &'a Value, clause: &str) -> &'a Value {
    value["refinement_transfer"]["transfers"]
        .as_array()
        .and_then(|transfers| transfers.iter().find(|entry| entry["clause"] == clause))
        .map(|entry| &entry["result"])
        .unwrap_or_else(|| panic!("a transfer of {clause}: {value}"))
}

/// FR-1030-AC-1/AC-2, FR-1020-AC-1/AC-2, FSpec FR-380-AC-9, FSpec
/// FR-820-AC-1/AC-2: `once[1,1]` and `eventually[0,2]` clauses transfer
/// `proved` through the step-for-step `ThirdRefinesSpec`, and settle
/// `transfer-not-preserving` through `CasRefinesCounter`, whose step map
/// sends concrete operations to `stutter`; a fragment clause still transfers.
#[test]
#[trace("TC-2920", "FR-1030-AC-1")]
#[trace("TC-2920", "FR-1030-AC-2")]
#[trace("TC-2870", "FR-1020-AC-1")]
#[trace("TC-2870", "FR-1020-AC-2")]
#[trace("FSpec-TC-1042", "FSpec-FR-380-AC-9")]
#[trace("FSpec-TC-2000", "FSpec-FR-820-AC-1")]
#[trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
#[ignore = "QSL-2247: the refinement_transfer request member and step-for-step transfer are not implemented yet"]
fn previous_transfers_only_through_a_step_for_step_refinement() {
    let (exit, value) = transfer_run(
        &[(
            "refinement_transfer",
            transfer_member(
                "ThirdRefinesSpec",
                ("Spec", "Third"),
                &["PrevSafe", "SoonTwo"],
            ),
        )],
        true,
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["refinement_transfer"]["refinement"]["step_for_step"]["holds"], true,
        "{value}"
    );
    for clause in ["PrevSafe", "SoonTwo"] {
        let result = transfer(&value, clause);
        assert_eq!(result["value"], "proved", "{value}");
        assert_eq!(result["basis"], "decisive-witness", "{value}");
        assert_eq!(
            result["transfer"]["premises"].as_array().map(Vec::len),
            Some(2),
            "{value}"
        );
    }

    let (exit, value) = transfer_run(
        &[(
            "refinement_transfer",
            transfer_member(
                "CasRefinesCounter",
                ("Spec", "Impl"),
                &["PrevSafe", "SoonTwo", "AtMostTwo"],
            ),
        )],
        true,
    );
    assert_eq!(exit, 22, "{value}");
    let step_for_step = &value["refinement_transfer"]["refinement"]["step_for_step"];
    assert_eq!(step_for_step["holds"], false, "{value}");
    assert_eq!(
        step_for_step["row"]["operation"], "Impl::Counter::beginA",
        "{value}"
    );
    assert_eq!(step_for_step["row"]["arm"], "stutter", "{value}");
    for (clause, operator) in [("PrevSafe", "once[1,1]"), ("SoonTwo", "eventually[0,2]")] {
        let result = transfer(&value, clause);
        assert_eq!(result["value"], "inconclusive", "{value}");
        assert_eq!(
            result["cause"]["type"], "transfer-not-preserving",
            "{value}"
        );
        assert_eq!(result["cause"]["reason"], "fragment", "{value}");
        assert_eq!(result["cause"]["operator"], operator, "{value}");
    }
    assert_eq!(transfer(&value, "AtMostTwo")["value"], "proved", "{value}");
}

/// FR-1031-AC-1 to AC-3: conflicting selection members, a duplicate clause,
/// a missing model, a subject that is not the refinement's and clauses
/// outside the transferable family each refuse at their stage and pointer.
#[test]
#[trace("TC-2921", "FR-1031-AC-1")]
#[trace("TC-2921", "FR-1031-AC-2")]
#[trace("TC-2921", "FR-1031-AC-3")]
#[ignore = "QSL-2247: refinement_transfer request refusals are not implemented yet"]
fn a_refinement_transfer_request_is_refused_by_stage() {
    let member = |abstract_model: &str, concrete: &str, clauses: &[&str]| {
        transfer_member("ThirdRefinesSpec", (abstract_model, concrete), clauses)
    };
    let refused = |members: &[(&str, Value)], supply_spec: bool, stage: &str, code: &str| {
        let (exit, value) = transfer_run(members, supply_spec);
        assert_eq!(exit, 20, "{value}");
        names(&value, stage);
        names(&value, code);
        value
    };

    let call = json!({"function": "f", "arguments": []});
    let value = refused(
        &[
            (
                "refinement_transfer",
                member("Spec", "Third", &["PrevSafe"]),
            ),
            ("call", call),
        ],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "refinement_transfer");
    names(&value, "call");
    let value = refused(
        &[
            (
                "refinement_transfer",
                member("Spec", "Third", &["PrevSafe"]),
            ),
            ("items", json!([])),
        ],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "items");
    let value = refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["PrevSafe", "PrevSafe"]),
        )],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "/refinement_transfer/clauses/1");

    let value = refused(
        &[(
            "refinement_transfer",
            member("Ghost", "Third", &["PrevSafe"]),
        )],
        true,
        "select",
        "missing-name",
    );
    names(&value, "/refinement_transfer/abstract/model");
    refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["PrevSafe"]),
        )],
        false,
        "compile",
        "missing-selection",
    );
    let value = refused(
        &[("refinement_transfer", member("Spec", "Impl", &["PrevSafe"]))],
        true,
        "select",
        "invalid-value",
    );
    names(&value, "/refinement_transfer/concrete/model");
    names(&value, "Third");

    // `CanInc` is a state precondition, not an infinite-trace clause.
    let value = refused(
        &[("refinement_transfer", member("Spec", "Third", &["CanInc"]))],
        true,
        "select",
        "type-mismatch",
    );
    names(&value, "/refinement_transfer/clauses/0");
    let value = refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["OverThird"]),
        )],
        true,
        "select",
        "invalid-value",
    );
    names(&value, "/refinement_transfer/clauses/0");
}

// ---- QSL-1979: target results as trusted evidence -------------------------

/// FR-1022-AC-1, FSpec FR-815-AC-1: a Lean proof of an exported
/// proposition enters as `trusted` evidence, never `certified`, or settles
/// `tool-unavailable` when Lean is absent.
#[test]
#[trace("TC-2872", "FR-1022-AC-1")]
#[trace("FSpec-TC-1974", "FSpec-FR-815-AC-1")]
#[ignore = "QSL-1979: intake of TLA+ and Lean results as trusted evidence is not implemented yet"]
fn a_lean_proof_enters_as_trusted_evidence() {
    let (_, exported) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.lean4/v1", "obligations": ["Low"]}),
    );
    let package = exported["export"]["package"].clone();
    assert!(package.is_object(), "{exported}");
    let (exit, value) = run(
        &low_unit(),
        true,
        (
            "export_evidence",
            json!({"package": package, "obligation": "Low", "tool": "lean",
                "artifact": "low_proof.lean"}),
        ),
    );
    let result = &value["export_evidence"];
    if result["value"] == "unsupported" {
        names(result, "tool-unavailable");
        assert_eq!(exit, 21, "{value}");
    } else {
        assert_eq!(result["value"], "proved", "{value}");
        assert_eq!(result["certification"], "trusted", "{value}");
    }
}

/// FR-1022-AC-3, FSpec FR-815-AC-3: evidence for an unrepresented record is
/// refused at its obligation member.
#[test]
#[trace("TC-2872", "FR-1022-AC-3")]
#[trace("FSpec-TC-1974", "FSpec-FR-815-AC-3")]
#[ignore = "QSL-1979: evidence intake refusals are not implemented yet"]
fn evidence_for_an_unrepresented_record_is_refused() {
    let (exit, value) = run(
        BRAKING,
        false,
        (
            "export_evidence",
            json!({"package": {}, "obligation": "SlowEnough", "tool": "tlc",
                "artifact": "slow.cfg"}),
        ),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "/export_evidence/obligation");
}
