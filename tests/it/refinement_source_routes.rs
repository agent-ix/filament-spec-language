// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original refinement and hyperproperty programs through the public source commands.

use filament_exact::Cancel;
use fsl_command::command;
use ix_trace_rs::trace;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    claim: String,
    result: String,
}

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/refinement")
}

fn expected(stem: &str) -> Expected {
    let bytes = std::fs::read(corpus().join(format!("{stem}.verdict.json"))).unwrap();
    serde_json::from_slice(&bytes).expect("the authored expectation is strict JSON")
}

#[test]
#[trace("FR-135", "FR-136")]
fn incomplete_refinement_rows_identify_the_missing_member() {
    let mut messages = Vec::new();
    for (stem, member) in [
        ("refinement-missing-step", "inc"),
        ("refinement-missing-object", "Counter"),
        ("refinement-unknown-abstract-operation", "absent"),
    ] {
        let error = command::compile_outcome(&corpus().join(format!("{stem}.request.json")))
            .err()
            .expect("incomplete refinement coverage is refused");
        let message = error.to_string();
        assert!(message.contains(member), "{stem}: {message}");
        messages.push(message);
    }
    for pair in messages.windows(2) {
        assert_ne!(
            pair[0], pair[1],
            "different missing rows keep distinct diagnostics"
        );
    }
}

fn analyze(stem: &str) {
    let expected = expected(stem);
    let output = command::analyze(
        &corpus().join(format!("{stem}.analyze.json")),
        &crate::negotiation::Fr290,
        &Cancel::new(),
    )
    .unwrap_or_else(|error| panic!("{stem}: source analysis must settle: {error}"));
    let value = output.value.as_value();
    let items = value["items"].as_array().expect("analysis item records");
    let item = items
        .iter()
        .find(|item| item["claim_name"].as_str() == Some(expected.claim.as_str()))
        .unwrap_or_else(|| panic!("{stem}: expected named claim {expected:?}: {value}"));
    assert_eq!(
        item["record"]["result"].as_str(),
        Some(expected.result.as_str()),
        "{stem}: {value}"
    );
    match expected.result.as_str() {
        "proved" => {
            assert_eq!(output.exit_code, 0, "{stem}: {value}");
            assert_eq!(
                item["record"]["certification"], "certified",
                "{stem}: {value}"
            );
        }
        "refuted" => {
            assert_eq!(output.exit_code, 10, "{stem}: {value}");
            assert!(
                item.get("counterexample")
                    .is_some_and(|witness| !witness.is_null()),
                "{stem}: {value}"
            );
        }
        other => panic!("unsupported authored result {other}"),
    }
}

#[test]
#[trace("FR-144", "FR-149")]
fn identity_refinement_is_certified_from_source() {
    analyze("refinement-identity");
}

#[test]
#[trace("FR-142", "FR-145")]
fn declaring_a_changing_counter_step_as_stutter_is_refuted() {
    analyze("refinement-stutter");
}

#[test]
#[trace("FR-142", "FR-145")]
fn a_type_correct_constant_mapping_is_refuted_by_analysis() {
    analyze("refinement-constant-map");
}

#[test]
#[trace("FR-163", "FR-182")]
fn two_copy_determinism_is_certified_from_source() {
    analyze("hyper-universal-equal");
}

#[test]
#[trace("FR-182", "FR-183")]
fn a_false_two_copy_rule_publishes_a_replayed_counterexample() {
    analyze("hyper-universal-false");
}

#[test]
#[trace("FR-177", "FR-163")]
fn every_counter_trace_has_an_equal_existential_partner() {
    analyze("hyper-witness-equal");
}

#[test]
#[trace("FR-177", "FR-183")]
fn equal_initial_values_exhaust_a_different_partner_witness_set() {
    analyze("hyper-witness-impossible");
}

#[test]
#[trace("FR-181")]
fn a_single_counter_trace_can_reach_three() {
    analyze("hyper-single-reaches-three");
}

#[test]
#[trace("FR-181")]
fn no_counter_trace_can_stay_zero() {
    analyze("hyper-single-impossible");
}

#[test]
#[trace("FR-135")]
fn a_missing_population_row_names_its_population() {
    let error =
        command::compile_outcome(&corpus().join("refinement-missing-population.request.json"))
            .err()
            .expect("a missing population row refuses");
    assert!(error.to_string().contains("counters"), "{error}");
}

#[test]
#[trace("FR-612-AC-1")]
fn indexed_reads_outside_hyper_keep_the_defined_boundary_cause() {
    let error =
        command::compile_outcome(&corpus().join("pending/hyper-plain-temporal-at.request.json"))
            .err()
            .expect("the hyper-only expression refuses");
    let value = error.artifact_value().expect("the refusal encodes");
    assert_eq!(error.category().exit_code(), 21, "{value}");
    assert_eq!(value["code"], "unsupported_construct", "{value}");
    assert_eq!(value["cause"], "expression-form", "{value}");
    assert!(
        value["message"]
            .as_str()
            .is_some_and(|message| message.contains("hyper")),
        "{value}"
    );
    let source = std::fs::read_to_string(corpus().join("pending/hyper-plain-temporal-at.native"))
        .expect("the case source");
    let at = source.find("c.value @ a").expect("the indexed read");
    assert_eq!(value["details"]["span"]["start"]["byte"], at, "{value}");
    assert_eq!(
        value["details"]["span"]["end"]["byte"],
        at + "c.value @ a".len(),
        "{value}"
    );
}

#[path = "refinement_remaining_source_routes.rs"]
mod remaining;

/// The same model-to-model refinement must retain its entire certified record
/// when its emitted package is the only remaining program representation.
/// This compares public records under identical default engine limits; it does
/// not claim to measure internal charges, which these records do not expose.
#[test]
#[trace("FR-940", "FR-630", "FR-144", "FR-149")]
fn identity_refinement_package_matches_source_without_source() {
    refinement_package_matches_source_without_source("refinement");
}

/// A refinement declaration cannot stand in for a different temporal property.
/// Both routes retain their exact refusal after all source files are removed.
#[test]
#[trace("FR-940", "FR-630", "FR-626")]
fn unrelated_temporal_kinds_do_not_run_package_refinement() {
    for item_kind in ["deadlock-freedom", "time-lock-freedom", "race-freedom"] {
        refinement_package_matches_source_without_source(item_kind);
    }
}

fn refinement_package_matches_source_without_source(item_kind: &str) {
    use fsl_foundation::source::SourceLabel;
    use serde_json::{json, Value};

    let directory = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_path = directory.path().join("unit.native");
    std::fs::copy(corpus().join("refinement-identity.native"), &source_path).unwrap();
    let mut request: Value = serde_json::from_slice(
        &std::fs::read(corpus().join("refinement-identity.analyze.json")).unwrap(),
    )
    .unwrap();
    request["request"]["program"]["source"]["file"] = json!("unit.native");
    request["request"]["models"][0]["source"]["file"] =
        json!(root.join("fsl-conformance/cases/counter-step-package.json"));
    request["request"]["documents"][0]["file"] =
        json!(root.join("fsl-conformance/cases/counter-step-v0.json"));
    request["request"]["items"][0]["temporal"]["item_kind"] = json!(item_kind);
    let source_request = directory.path().join("source.json");
    std::fs::write(&source_request, serde_json::to_vec(&request).unwrap()).unwrap();
    let prepared = command::prepare_analyze(&source_request, command::RunLimits::default())
        .expect("the actual source request prepares")
        .into_input();
    let package_name = prepared
        .source
        .source()
        .identity()
        .label(SourceLabel::Identity)
        .to_owned();
    let source = command::analyze(&source_request, &crate::negotiation::Fr290, &Cancel::new())
        .expect("source refinement analysis settles before package comparison");
    let source_value = source.value.as_value().clone();
    if item_kind == "refinement" {
        assert_eq!(source.exit_code, 0, "source fixture: {source_value}");
        assert_eq!(source_value["items"][0]["record"]["result"], "proved");
        assert_eq!(
            source_value["items"][0]["record"]["certification"],
            "certified"
        );
        assert_eq!(
            source_value["items"][0]["record"]["backend"],
            fsl_analyze::model_check::manifest::BACKEND_ID,
            "the fixture selects EN-1's actual refinement checker: {source_value}"
        );
    } else {
        let (exit, cause) = match item_kind {
            "deadlock-freedom" | "time-lock-freedom" => (20, "unknown-claim"),
            "race-freedom" => (21, "unsupported-requested-capability"),
            other => panic!("uncovered temporal item kind {other}"),
        };
        assert_eq!(source.exit_code, exit, "{item_kind}: {source_value}");
        let item = &source_value["items"][0];
        assert_eq!(
            item["record"]["diagnostic"]["cause"], cause,
            "{source_value}"
        );
        assert!(item["certificate"].is_null(), "{source_value}");
        assert!(item["counterexample"].is_null(), "{source_value}");
        assert!(item["replay"].is_null(), "{source_value}");
    }

    let compile = json!({"format": "native-compile/1", "request": {
        "program": request["request"]["program"],
        "models": request["request"]["models"],
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": package_name,
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}
    }});
    let compile_path = directory.path().join("compile.json");
    std::fs::write(&compile_path, serde_json::to_vec(&compile).unwrap()).unwrap();
    command::compile(&compile_path).expect("the actual refinement emits its handoff");
    std::fs::remove_file(&source_path).unwrap();
    let handoff_sources = directory.path().join("handoff/sources");
    if handoff_sources.exists() {
        std::fs::remove_dir_all(&handoff_sources).unwrap();
    }
    assert!(!source_path.exists());
    assert!(!handoff_sources.exists());

    request["request"]
        .as_object_mut()
        .unwrap()
        .remove("program");
    request["request"]["package"] = json!({"directory": "handoff"});
    let package_request = directory.path().join("package-request.json");
    std::fs::write(&package_request, serde_json::to_vec(&request).unwrap()).unwrap();
    let (package_exit, package_value) =
        match command::analyze(&package_request, &crate::negotiation::Fr290, &Cancel::new()) {
            Ok(output) => (output.exit_code, output.value.as_value().clone()),
            Err(error) => (error.category().exit_code(), error.value().unwrap()),
        };
    assert_eq!(
        package_exit, source.exit_code,
        "source: {source_value}; package: {package_value}"
    );
    assert_eq!(package_value, source_value);
}
