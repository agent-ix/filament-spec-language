// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 "Diagnostic details": each outcome diagnostic carries `details`,
//! `null` or the closed object FR-100's command-error envelope carries for
//! the same refusal, so the `filament-outcome/1` document a run writes shows
//! the function, parameter position and JSON pointer of a run refusal.

use filament_exact::Cancel;
use fsl_command::command::{self, RunLimits};
use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{parse, ParseRequest};
use fsl_replay::{Operation, OutcomeDocument};
use ix_trace_rs::trace;
use serde_json::{json, Value};

/// The members the closed `details` vocabulary allows.
const VOCABULARY: &[&str] = &[
    "function",
    "position",
    "path",
    "parameter",
    "model_identity",
    "population",
    "expected",
    "member",
    "type",
    "required",
    "supplied",
    "stage",
    "invariant",
];

const UNIT: &str = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
type Big = Int[0, 1267650600228229401496703205376];
record P { x: Int[0, 9]; y: Int[0, 9]; }
function id_big using v(a: Big): Big pure { a }
function id_rec using v(a: P): P pure { a }
function id_set using v(a: Set<Int[0, 9]>[0, 3]): Set<Int[0, 9]>[0, 3] pure { a }
"#;

/// The refused run's `execute` outcome document and its command-error
/// envelope.
fn refused(unit: &str, function: &str, arguments: Value) -> (Value, Value) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("program.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "program.native", "authority": "agent-ix",
            "identity": "test:diagnostic-details", "document": "Details", "formal_revision": 1}},
        "call": {"function": function, "arguments": arguments},
    }});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let error = command::run_with_limits(&path, RunLimits::default(), &crate::negotiation::Fr290)
        .err()
        .unwrap_or_else(|| panic!("{function} refuses"));
    let document = error
        .outcome_document(Operation::Execute)
        .to_bytes()
        .expect("the document encodes");
    let envelope = error.value().expect("the envelope encodes");
    (serde_json::from_slice(&document).unwrap(), envelope)
}

fn only_diagnostic(document: &Value) -> Value {
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{document}");
    diagnostics[0].clone()
}

fn argument(value: Value) -> Value {
    json!([{"parameter": "a", "value": value}])
}

#[test]
#[trace("TC-2377", "FR-286-AC-17")]
fn argument_diagnostics_carry_position_and_path() {
    for (function, value, cause, details) in [
        (
            "id_rec",
            json!({"record": {"type": "P", "fields": {"x": {"integer": "1"}}}}),
            "missing-member",
            json!({"path": "/record/fields", "position": 0}),
        ),
        (
            "id_set",
            json!({"set": [{"integer": "1"}, {"integer": "1"}]}),
            "duplicate-member",
            json!({"path": "/set/1", "position": 0}),
        ),
    ] {
        let (document, envelope) = refused(UNIT, function, argument(value));
        let diagnostic = only_diagnostic(&document);
        assert_eq!(diagnostic["code"], "invalid_runtime_input", "{diagnostic}");
        assert_eq!(diagnostic["cause"], cause, "{diagnostic}");
        assert_eq!(diagnostic["details"], details, "{diagnostic}");
        assert_eq!(diagnostic["details"], envelope["details"], "{envelope}");
    }
}

#[test]
#[trace("TC-2378", "FR-286-AC-18")]
fn function_and_request_diagnostics_carry_details() {
    let (document, _) = refused(UNIT, "nope", json!([]));
    assert_eq!(
        only_diagnostic(&document)["details"],
        json!({"function": "nope"})
    );
    let (document, _) = refused(
        UNIT,
        "id_rec",
        json!([{"parameter": "zz", "value": {"integer": "1"}}]),
    );
    assert_eq!(
        only_diagnostic(&document)["details"],
        json!({"parameter": "zz"})
    );
    let (document, _) = refused(UNIT, "id_big", argument(json!("7")));
    assert_eq!(
        only_diagnostic(&document)["details"],
        json!({"path": "/call/arguments/0/value"})
    );
}

fn assert_closed(diagnostic: &Value) {
    let details = &diagnostic["details"];
    assert!(
        diagnostic.as_object().unwrap().contains_key("details"),
        "{diagnostic}"
    );
    if let Some(members) = details.as_object() {
        assert!(!members.is_empty(), "{diagnostic}");
        assert!(
            members
                .keys()
                .all(|member| VOCABULARY.contains(&member.as_str())),
            "{diagnostic}"
        );
    } else {
        assert!(details.is_null(), "{diagnostic}");
    }
}

#[test]
#[trace("TC-2379", "FR-286-AC-19")]
fn source_and_compile_diagnostics_carry_null_details() {
    let bad = "language \"ix:native\" edition \"1-draft\";\n\
               profile v = \"filament.value.complete/v1\";\n\
               function bad using v(): Int[0, 9] pure { true }\n";
    let (document, _) = refused(bad, "bad", json!([]));
    let diagnostic = only_diagnostic(&document);
    assert_eq!(diagnostic["code"], "ill_typed", "{diagnostic}");
    assert!(diagnostic["details"].is_null(), "{diagnostic}");
    assert_closed(&diagnostic);

    let text = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"filament.value.complete/v1\";\n\
                function f using v(xs: Sequence<Integer>[0, 4]): Integer pure { null }\n";
    let identity = SourceIdentity::new("agent-ix", "test:diagnostic-details");
    let result = parse(
        &ParseRequest {
            source: &identity,
            path: "details.native",
            bytes: text.as_bytes(),
        },
        Limits::default(),
        &Cancel::new(),
    );
    let document: Value = serde_json::from_slice(
        &OutcomeDocument::from_parse(&result)
            .to_bytes()
            .expect("the document encodes"),
    )
    .unwrap();
    for diagnostic in document["diagnostics"].as_array().expect("diagnostics") {
        assert!(diagnostic["details"].is_null(), "{diagnostic}");
        assert_closed(diagnostic);
    }
}
