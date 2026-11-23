// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-662 Outputs: a `StageFailure` covers the Markdown document as a whole,
//! and `OutcomeDocument::from_native_fence_failure` projects each arm of it as
//! one `check` document that carries no items.

#![cfg(feature = "quire-extraction")]

use filament_exact::{Cancel, CancelCause};
use filament_semantic_value::catalog::{Category, InternalFault};
use fsl_foundation::diagnostic::{LimitExceeded, StageFailure};
use fsl_foundation::{Setting, Source, SourceIdentity};
use fsl_replay::spine::{self, CheckLimits, FenceCheckCause};
use fsl_replay::OutcomeDocument;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DOCUMENT: &str =
    "## Invariants\n### Only\n```ix:native\nlanguage \"ix:native\" edition \"1-draft\";\n```\n";

fn document_json(document: &OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().unwrap()).unwrap()
}

fn check(limits: CheckLimits, cancel: &Cancel) -> Result<(), StageFailure<FenceCheckCause>> {
    let original = Source::read(
        SourceIdentity::new("agent-ix", "doc"),
        "spec.md",
        DOCUMENT.as_bytes(),
        DOCUMENT.len(),
    )
    .unwrap();
    spine::check_native_fences(
        &spine::NativeFenceRequest {
            original: &original,
            packages: &Default::default(),
            dependencies: &Default::default(),
            extensions: None,
            integer_division: None,
        },
        limits,
        cancel,
    )
    .map(|_| ())
}

fn only_diagnostic(document: &Value) -> &Value {
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{document}");
    &diagnostics[0]
}

/// An extraction refusal (the original exceeds the extraction byte limit)
/// and the same cause carried with an empty payload give the same document:
/// `check`, category incomplete, no stage, no items, one `resource_exhausted`.
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
fn an_extraction_refusal_projects_a_check_document_without_items() {
    let mut limits = CheckLimits::default();
    limits.extraction.source_bytes = 1;
    let failure = check(limits, &Cancel::new()).unwrap_err();
    let document = OutcomeDocument::from_native_fence_failure(&failure);
    assert_eq!(document.category(), Category::Incomplete);
    let value = document_json(&document);
    assert_eq!(value["operation"], "check", "{value}");
    assert_eq!(value["category"], "incomplete", "{value}");
    assert_eq!(value["last_stage"], Value::Null, "{value}");
    assert_eq!(value["items"], json!([]), "{value}");
    let diagnostic = only_diagnostic(&value);
    assert_eq!(diagnostic["code"], "resource_exhausted", "{value}");
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("original document exceeds the extraction source-byte limit"),
        "{value}"
    );

    let StageFailure::Refused(cause) = failure else {
        panic!("the extraction refusal is a plain refusal");
    };
    let with_payload = StageFailure::RefusedWithPayload(Box::new(cause), ());
    assert_eq!(
        document_json(&OutcomeDocument::from_native_fence_failure(&with_payload)),
        value
    );
}

/// A reached limit, with or without a retained payload, is incomplete at the
/// limit's stage with the `stage_limit_exceeded` code.
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
fn a_reached_limit_projects_an_incomplete_document_at_its_stage() {
    let limit = LimitExceeded::new(Setting::S1InputBytes, 4, 9);
    let plain = OutcomeDocument::from_native_fence_failure(&StageFailure::Limit(limit.clone()));
    assert_eq!(plain.category(), Category::Incomplete);
    let value = document_json(&plain);
    assert_eq!(value["operation"], "check", "{value}");
    assert_eq!(value["last_stage"], "S1", "{value}");
    assert_eq!(value["items"], json!([]), "{value}");
    assert_eq!(only_diagnostic(&value)["code"], "stage_limit_exceeded");
    let with_payload =
        OutcomeDocument::from_native_fence_failure(&StageFailure::LimitWithPayload(limit, ()));
    assert_eq!(document_json(&with_payload), value);
}

/// A cancelled check returns no report and projects an incomplete document
/// carrying the `cancelled` code and the cause.
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
fn a_cancelled_check_projects_an_incomplete_cancelled_document() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let failure = check(CheckLimits::default(), &cancel).unwrap_err();
    let document = OutcomeDocument::from_native_fence_failure(&failure);
    assert_eq!(document.category(), Category::Incomplete);
    let value = document_json(&document);
    assert_eq!(value["last_stage"], Value::Null, "{value}");
    assert_eq!(value["items"], json!([]), "{value}");
    let diagnostic = only_diagnostic(&value);
    assert_eq!(diagnostic["code"], "cancelled", "{value}");
    assert_eq!(diagnostic["cause"], "requested", "{value}");
}

/// This constructed fault exercises the fault arm; it does not claim that a
/// document triggers an internal invariant failure.
#[test]
#[trace("TC-1743", "FR-662-AC-2")]
fn a_constructed_fault_projects_an_internal_failure_at_no_stage() {
    let fault = InternalFault::new("S4", "constructed-test-invariant");
    let document = OutcomeDocument::from_native_fence_failure(&StageFailure::Fault(fault));
    assert_eq!(document.category(), Category::InternalFailure);
    let value = document_json(&document);
    assert_eq!(value["operation"], "check", "{value}");
    assert_eq!(value["last_stage"], Value::Null, "{value}");
    assert_eq!(value["items"], json!([]), "{value}");
    assert_eq!(
        only_diagnostic(&value)["message"],
        "internal invariant constructed-test-invariant broken in S4"
    );
}
