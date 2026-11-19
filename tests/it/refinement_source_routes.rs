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
    compile_exit: u8,
    code: Option<String>,
    mentions: Option<String>,
    claim: Option<String>,
    result: Option<String>,
}

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/refinement")
}

fn expected(stem: &str) -> Expected {
    let bytes = std::fs::read(corpus().join(format!("{stem}.expected.json"))).unwrap();
    serde_json::from_slice(&bytes).expect("the authored expectation is strict JSON")
}

fn compile(stems: &[&str]) {
    let mut failures = Vec::new();
    for stem in stems {
        let expected = expected(stem);
        let path = corpus().join(format!("{stem}.request.json"));
        match command::compile_outcome(&path) {
            Ok(compiled) => {
                if expected.compile_exit != 0 || compiled.package_bytes().is_empty() {
                    failures.push(format!("{stem}: expected {expected:?}, compiled package"));
                }
            }
            Err(error) => {
                let value = error.artifact_value().expect("the refusal serializes");
                let diagnostic = value.pointer("/diagnostics/0").unwrap_or(&value);
                if error.category().exit_code() != expected.compile_exit
                    || expected.code.as_deref() != diagnostic["code"].as_str()
                    || !expected
                        .mentions
                        .as_deref()
                        .is_some_and(|name| error.to_string().contains(name))
                {
                    failures.push(format!(
                        "{stem}: expected {expected:?}, got {value}: {error}"
                    ));
                }
            }
        }
        println!("checked source case {stem}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[trace("FR-135", "FR-136", "FR-901")]
fn refinement_rows_compile_before_their_semantics_are_analyzed() {
    compile(&[
        "refinement-identity",
        "refinement-stutter",
        "refinement-constant-map",
    ]);
}

#[test]
#[trace("FR-135", "FR-136")]
fn missing_and_mistyped_refinement_rows_name_the_unresolved_member() {
    compile(&[
        "refinement-missing-step",
        "refinement-missing-object",
        "refinement-missing-population",
        "refinement-unknown-abstract-operation",
        "refinement-wrong-map-type",
    ]);
}

#[test]
#[trace("FR-172", "FR-488")]
fn universal_witness_set_and_single_existential_sources_compile() {
    compile(&[
        "hyper-universal-equal",
        "hyper-universal-false",
        "hyper-witness-equal",
        "hyper-witness-impossible",
        "hyper-single-reaches-three",
        "hyper-single-impossible",
    ]);
}

#[test]
#[trace("FR-172", "FR-612")]
fn hyper_trace_names_and_single_trace_boundaries_are_checked() {
    compile(&[
        "hyper-unknown-trace",
        "hyper-duplicate-trace",
        "hyper-plain-temporal-at",
    ]);
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
        .find(|item| item["claim_name"].as_str() == expected.claim.as_deref())
        .unwrap_or_else(|| panic!("{stem}: expected named claim {expected:?}: {value}"));
    assert_eq!(
        item["record"]["result"].as_str(),
        expected.result.as_deref(),
        "{stem}: {value}"
    );
    match expected.result.as_deref().expect("an analysis oracle") {
        "proved" => {
            assert_eq!(output.exit_code, 0, "{stem}: {value}");
            assert_eq!(item["certificate"]["accepted"], true, "{stem}: {value}");
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
