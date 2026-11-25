// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-401's public unsupported-entry vocabulary and FR-286 rendering.

use filament_eval::{
    CallFailure, ExecutionBackendChoice, UnsupportedCause, UnsupportedEntry,
    UntranslatableConstruct,
};
use filament_exact::{DecimalType, Integer, NodeKey, PathSegment, RoundingMode, ValueType};
use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
use fsl_replay::{
    write_clause_run_result, ClauseDisposition, ClauseRunReport, ClauseRunUsage, RunRefusal,
    SettlementBasis,
};

fn location() -> ExpressionLocation {
    ExpressionLocation::at(
        ExpressionOwner::Body {
            function: "f".to_owned(),
            index: 0,
        },
        &[2, 1],
    )
}

fn refusal_document(entry: UnsupportedEntry) -> serde_json::Value {
    let report = ClauseRunReport {
        disposition: ClauseDisposition::ArgumentRefusal(Box::new(RunRefusal::Unsupported(entry))),
        usage: ClauseRunUsage::default(),
        basis: SettlementBasis::Unavailable,
        witness: None,
        state_clause_counterexample: None,
        frame_field_values: None,
        consequent_source_events: Default::default(),
        ledger: None,
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
    };
    let bytes = write_clause_run_result(&report).expect("writes FR-286 result");
    serde_json::from_slice(&bytes).expect("result is JSON")
}

#[test]
fn unsupported_entry_types_and_fr286_payloads_are_stable() {
    let missing = UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause: UnsupportedCause::NotSupplied,
    };
    let failure = CallFailure::Unsupported(missing.clone());
    assert_eq!(
        failure.category(),
        filament_semantic_value::catalog::Category::Unsupported
    );
    assert_eq!(missing.catalog_code().code(), "unimplemented_capability");
    assert_eq!(missing.catalog_code().cause(), "backend-not-supplied");
    assert_eq!(missing.payload().backend, "jit");
    assert_eq!(missing.payload().construct, None);
    assert_eq!(missing.payload().construct_name, None);
    assert_eq!(missing.payload().location, None);
    let missing_doc = refusal_document(missing);
    assert_eq!(
        missing_doc["disposition"]["refusal"]["code"],
        "unimplemented_capability"
    );
    assert_eq!(
        missing_doc["disposition"]["refusal"]["cause"],
        "backend-not-supplied"
    );
    assert_eq!(
        missing_doc["disposition"]["refusal"]["fields"]["backend"],
        "jit"
    );
    assert!(missing_doc["disposition"]["refusal"]["fields"]
        .get("location")
        .is_none());

    let source_location = location();
    let cases = [
        (
            UntranslatableConstruct::ExpressionForm("let".to_owned()),
            "expression-form",
            "let",
        ),
        (
            UntranslatableConstruct::ValueType(Box::new(ValueType::Decimal(
                DecimalType::new(
                    Integer::from(0_i64),
                    Integer::from(1_i64),
                    0,
                    1,
                    RoundingMode::Exact,
                )
                .expect("valid Decimal type"),
            ))),
            "value-type",
            "Decimal",
        ),
        (
            UntranslatableConstruct::Operation(
                NodeKey::new(
                    "acme/test",
                    vec![PathSegment::Text("operation".to_owned())],
                    0,
                )
                .expect("valid operation key"),
            ),
            "operation",
            "acme/test:[\"operation\",]:0",
        ),
    ];
    for (construct, expected_kind, expected_name) in cases {
        let entry = UnsupportedEntry {
            backend: ExecutionBackendChoice::Jit,
            cause: UnsupportedCause::Untranslatable {
                construct,
                location: source_location.clone(),
            },
        };
        assert_eq!(entry.catalog_code().code(), "unimplemented_capability");
        assert_eq!(entry.catalog_code().cause(), "untranslatable-entry");
        let payload = entry.payload();
        assert_eq!(payload.backend, "jit");
        assert_eq!(payload.construct, Some(expected_kind));
        assert_eq!(payload.construct_name.as_deref(), Some(expected_name));
        assert_eq!(payload.location, Some(&source_location));

        let document = refusal_document(entry);
        let refusal = &document["disposition"]["refusal"];
        assert_eq!(refusal["code"], "unimplemented_capability");
        assert_eq!(refusal["cause"], "untranslatable-entry");
        assert_eq!(refusal["fields"]["backend"], "jit");
        assert_eq!(refusal["fields"]["construct"], expected_kind);
        assert_eq!(refusal["fields"]["construct_name"], expected_name);
        let serialized_location: serde_json::Value = serde_json::from_str(
            refusal["fields"]["location"]
                .as_str()
                .expect("location is serialized as JSON text"),
        )
        .expect("serialized location is JSON");
        assert_eq!(
            serialized_location,
            serde_json::json!({
                "origin": {"kind": "body", "function": "f", "index": 0},
                "path": [2, 1],
            })
        );
    }
}
