// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286/FR-644 published schema and golden wire checks.

use fsl_replay::{Operation, OutcomeDocument};
use filament_semantic_value::catalog::Category;
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn schema(text: &str) -> jsonschema::JSONSchema {
    let value: Value = serde_json::from_str(text).expect("schema JSON is valid");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&value)
        .expect("schema compiles")
}

/// FR-286-AC-25/26 (TC-3110): the check golden is its writer's bytes,
/// canonical, schema-valid, and closed against the named mutations.
#[trace("TC-3110", "FR-286-AC-25", "FR-286-AC-26")]
#[test]
fn outcome_golden_validates_and_rejects_mutations() {
    let validate = schema(fsl_replay::outcome::OUTCOME_SCHEMA_1);
    let success = OutcomeDocument::new(Operation::Check, None, Category::Success);
    let bytes = success.to_bytes().expect("canonical outcome bytes");
    assert_eq!(bytes, include_bytes!("../fixtures/outcome-golden/FR-286-AC-1.json"));
    let value: Value = serde_json::from_slice(&bytes).expect("outcome JSON");
    assert_eq!(fsl_foundation::canonical_json::encode(&value).unwrap(), bytes);
    assert!(validate.is_valid(&value));
    let mut wrong_operation_member = value.clone();
    wrong_operation_member["aggregate"] = json!({"status":"ok","gaps":[]});
    assert!(!validate.is_valid(&wrong_operation_member));

    let refused = OutcomeDocument::new(Operation::Check, None, Category::Refusal)
        .with_diagnostics(vec![fsl_replay::OutcomeDiagnostic::new(
            Some("type-mismatch"), "ill_typed", None, "bad value",
        )]);
    let refused_bytes = refused.to_bytes().expect("canonical refusal bytes");
    assert_eq!(
        refused_bytes,
        include_bytes!("../fixtures/outcome-golden/FR-286-AC-2.json")
    );
    let refused_value: Value = serde_json::from_slice(&refused_bytes).expect("outcome JSON");
    assert_eq!(fsl_foundation::canonical_json::encode(&refused_value).unwrap(), refused_bytes);
    assert!(validate.is_valid(&refused_value));
    let mut mutations = Vec::new();
    let mut missing_code = refused_value.clone();
    missing_code["diagnostics"][0].as_object_mut().unwrap().remove("code");
    mutations.push(missing_code);
    let mut extra_diagnostic_member = refused_value.clone();
    extra_diagnostic_member["diagnostics"][0]["fix"] = json!("do this");
    mutations.push(extra_diagnostic_member);
    let mut invalid_category = refused_value.clone();
    invalid_category["category"] = json!("violated");
    mutations.push(invalid_category);
    let mut empty_hint = refused_value;
    empty_hint["diagnostics"][0]["hint"] = json!("");
    mutations.push(empty_hint);
    for mutation in mutations {
        assert!(!validate.is_valid(&mutation));
    }
}

/// FR-644-AC-12 (TC-3115): published monitor schemas close the event and
/// summary top-level shapes and enforce settlement members.
#[trace("TC-3115", "FR-644-AC-12")]
#[test]
fn monitor_schemas_reject_invalid_settlement_and_extra_members() {
    let event_schema = schema(fsl_command::command::monitor::MONITOR_EVENT_SCHEMA_1);
    let event_line = include_bytes!("../fixtures/monitor-golden/FR-644-AC-2.jsonl")
        .strip_suffix(b"\n")
        .expect("golden line ends in newline");
    let event_golden: Value = serde_json::from_slice(event_line).expect("event golden JSON");
    assert_eq!(fsl_foundation::canonical_json::encode(&event_golden).unwrap(), event_line);
    assert!(event_schema.is_valid(&event_golden));
    let event = json!({
        "format":"filament.monitor-event/1", "record":2, "position":2,
        "clause":"E", "settled":true, "category":"success", "basis":"closed-scope",
        "disposition":{"stage":"S6a","category":"success","cause":"completed"},
        "settlement":{"step":"3","trigger":"decided","basis":"decisive-witness","horizon":"2","position":"2"}
    });
    assert!(event_schema.is_valid(&event));
    let mut missing = event.clone();
    missing.as_object_mut().unwrap().remove("settlement");
    assert!(!event_schema.is_valid(&missing));
    let mut extra = event.clone();
    extra["extra"] = json!(true);
    assert!(!event_schema.is_valid(&extra));
    let mut bad_trigger = event;
    bad_trigger["settlement"]["trigger"] = json!("decide");
    assert!(!event_schema.is_valid(&bad_trigger));

    let summary_schema = schema(fsl_command::command::monitor::MONITOR_SUMMARY_SCHEMA_1);
    let summary_line = include_bytes!("../fixtures/monitor-golden/summary-empty.jsonl")
        .strip_suffix(b"\n")
        .expect("golden line ends in newline");
    let summary_golden: Value = serde_json::from_slice(summary_line).expect("summary golden JSON");
    assert_eq!(
        fsl_foundation::canonical_json::encode(&summary_golden).unwrap(),
        summary_line
    );
    assert!(summary_schema.is_valid(&summary_golden));
    let summary = json!({"format":"filament.monitor-summary/1","clauses":[],"records":0,"refused":0,"ledger":null});
    assert!(summary_schema.is_valid(&summary));
    let mut bad_summary = summary;
    bad_summary["ledger_extra"] = json!([]);
    assert!(!summary_schema.is_valid(&bad_summary));
}
