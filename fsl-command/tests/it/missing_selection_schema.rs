// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 "Missing selections" and "Diagnostic details": the published outcome
//! schema admits exactly the nine `selection` kinds and the `missing` and
//! `accepted` details members.

use fsl_replay::outcome::hint::MissingSelection;
use fsl_replay::outcome::OUTCOME_SCHEMA_1;
use fsl_replay::{Operation, OutcomeDiagnostic, OutcomeDocument};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn schema() -> jsonschema::JSONSchema {
    let value: Value = serde_json::from_str(OUTCOME_SCHEMA_1).expect("schema JSON is valid");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&value)
        .expect("schema compiles")
}

fn refusal(diagnostic: OutcomeDiagnostic) -> Value {
    let document = OutcomeDocument::new(
        Operation::Check,
        None,
        filament_semantic_value::catalog::Category::Refusal,
    )
    .with_diagnostics(vec![diagnostic]);
    serde_json::from_slice(&document.to_bytes().expect("the document serializes"))
        .expect("the document is JSON")
}

fn missing_import(selection: Option<MissingSelection>) -> OutcomeDiagnostic {
    OutcomeDiagnostic::new(Some("missing-selection"), "missing_import", None, "missing")
        .with_selection(selection)
}

/// FR-286-AC-40: the schema accepts each of the nine kinds, a diagnostic
/// without one, and nothing else.
#[test]
#[trace("TC-3203", "FR-286-AC-40")]
fn schema_accepts_exactly_the_nine_selection_kinds() {
    let schema = schema();
    for kind in MissingSelection::ALL {
        let document = refusal(missing_import(Some(kind)));
        assert!(schema.is_valid(&document), "{kind:?}: {document}");
        assert_eq!(document["diagnostics"][0]["selection"], kind.as_str());
    }
    let defensive = refusal(missing_import(None));
    assert!(schema.is_valid(&defensive), "{defensive}");
    assert_eq!(defensive["diagnostics"][0].get("selection"), None);
    for bad in ["models", "Model", "", "library "] {
        let mut document = refusal(missing_import(Some(MissingSelection::Model)));
        document["diagnostics"][0]["selection"] = json!(bad);
        assert!(!schema.is_valid(&document), "{bad:?} must be refused");
    }
}

/// FR-286-AC-38: the schema admits the `missing` and `accepted` members of a
/// diagnostic's `details` as FR-1047 writes them, and stays closed.
#[test]
#[trace("TC-3201", "FR-286-AC-38")]
fn schema_admits_missing_and_accepted_details() {
    let schema = schema();
    let missing = json!({"missing": [
        "/request/models",
        ["/request/program", "/request/package"],
        ["/request/call", "/request/clause", "/request/items"]]});
    let accepted = json!({"accepted": ["native-run/1", "native-monitor/1", "native-replay/1"],
        "path": "/format"});
    for details in [missing, accepted] {
        let diagnostic = OutcomeDiagnostic::new(None, "invalid-request", None, "bad request")
            .with_details(details.as_object().cloned());
        let document = refusal(diagnostic);
        assert!(schema.is_valid(&document), "{document}");
        assert_eq!(document["diagnostics"][0]["details"], details);
    }
    let unknown = OutcomeDiagnostic::new(None, "invalid-request", None, "bad request")
        .with_details(json!({"unrecognized": true}).as_object().cloned());
    assert!(!schema.is_valid(&refusal(unknown)));
}

/// FR-286-AC-39: the `packaged` result is exactly its three members.
#[test]
#[trace("TC-3203", "FR-286-AC-39")]
fn schema_describes_the_packaged_result() {
    let schema = schema();
    let document = |result: Value| {
        let mut document = serde_json::from_slice::<Value>(
            &OutcomeDocument::new(
                Operation::Package,
                Some(fsl_replay::OutcomeStage::S4),
                filament_semantic_value::catalog::Category::Success,
            )
            .with_packaged(7, Some("reqs/p.json".to_owned()), None)
            .to_bytes()
            .unwrap(),
        )
        .unwrap();
        document["result"] = result;
        document
    };
    let written = document(json!({"kind": "packaged", "package": {
        "bytes": 7, "output": "reqs/p.json", "handoff": null}}));
    assert!(schema.is_valid(&written), "{written}");
    for bad in [
        json!({"kind": "packaged", "package": {"bytes": 7, "output": "p"}}),
        json!({"kind": "packaged", "package": {"bytes": "7", "output": null, "handoff": null}}),
        json!({"kind": "packaged", "package": {"bytes": 7, "output": null, "handoff": null, "extra": 1}}),
        json!({"kind": "packaged"}),
    ] {
        let invalid = document(bad.clone());
        assert!(!schema.is_valid(&invalid), "{bad}");
    }
}
