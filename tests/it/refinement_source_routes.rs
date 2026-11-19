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
        ("refinement-missing-population", "counters"),
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
fn no_counter_trace_can_reach_four() {
    analyze("hyper-single-impossible");
}
