// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retain every bounded S1 diagnostic through the refusal, JSON outcome and
//! text projection.

use filament_exact::Cancel;
use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use fsl_replay::OutcomeDocument;
use fsl_replay::spine::{
    DependencyInput, ParseRequest, SpineLimits, check, parse, refusal_or_fault, select,
};
use ix_trace_rs::trace;
use serde_json::Value;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n";

fn parsed_document(text: &str) -> (OutcomeDocument, String) {
    let identity = SourceIdentity::new("agent-ix", "test:diagnostic-golden");
    let result = parse(
        &ParseRequest {
            source: &identity,
            path: "diagnostics.native",
            bytes: text.as_bytes(),
        },
        Limits::default(),
        &Cancel::new(),
    );
    let document = OutcomeDocument::from_parse(&result);
    let rendered = result
        .err()
        .map(|failure| {
            refusal_or_fault(failure)
                .expect("the malformed source is a refusal")
                .to_string()
        })
        .expect("the malformed unit refuses");
    (document, rendered)
}

fn checked_document(text: &str) -> (OutcomeDocument, String) {
    let identity = SourceIdentity::new("agent-ix", "test:diagnostic-golden-check");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let fault = |failure| refusal_or_fault(failure).unwrap_or_else(|fault| panic!("{fault:?}"));
    let parsed = parse(
        &ParseRequest {
            source: &identity,
            path: "diagnostics.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(fault)
    .expect("the well-formed unit parses")
    .into_value();
    let models = select(
        &parsed,
        &fsl_semantics::model::intake::PackageInput::default(),
        limits.model,
        &cancel,
    )
    .map_err(fault)
    .expect("the unit has no model requirements")
    .into_value();
    let result = check(
        &parsed,
        "diagnostics".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    );
    let document = OutcomeDocument::from_check(&result);
    let rendered = result
        .err()
        .map(|failure| {
            refusal_or_fault(failure)
                .expect("the checker failure is a refusal")
                .to_string()
        })
        .expect("the checker refuses three unproved ranges");
    (document, rendered)
}

fn json(document: &OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().expect("outcome encodes")).expect("outcome is JSON")
}

#[test]
#[trace("TC-2187", "FR-762-AC-9")]
fn two_syntax_errors_survive_as_two_diagnostics() {
    let text = format!("{HEADER}record First {{ bad: ; }}\nrecord Second {{ bad: ; }}\n");
    let (document, rendered) = parsed_document(&text);
    let value = json(&document);
    let diagnostics = value["diagnostics"].as_array().expect("diagnostics list");
    assert_eq!(value["operation"], "parse", "{value}");
    assert_eq!(value["category"], "refusal", "{value}");
    assert_eq!(diagnostics.len(), 2, "{value}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["code"] == "invalid_syntax" && !diagnostic["locus"].is_null()
        }),
        "{value}"
    );
    let first_start = diagnostics[0]["locus"]["start"]
        .as_u64()
        .expect("first start");
    let second_start = diagnostics[1]["locus"]["start"]
        .as_u64()
        .expect("second start");
    assert!(first_start < second_start);
    assert_eq!(rendered.matches("invalid_syntax").count(), 2, "{rendered}");
    assert!(!rendered.contains("and 1 more"), "{rendered}");
}

#[test]
#[trace("TC-2153", "FR-762-AC-6")]
fn the_diagnostic_bound_is_reported_in_the_retained_list() {
    let errors: String = (0..150)
        .map(|index| format!("record R{index} {{ bad: ; }}\n"))
        .collect();
    let text = format!("{HEADER}{errors}");
    let (document, rendered) = parsed_document(&text);
    let diagnostics = json(&document)["diagnostics"]
        .as_array()
        .expect("diagnostics list")
        .clone();
    assert_eq!(diagnostics.len(), 101, "bounded list includes cut marker");
    let cut = diagnostics.last().expect("cut marker");
    assert_eq!(cut["cause"], "diagnostic-list-cut");
    assert!(
        cut["message"].as_str().unwrap().contains("50 additional"),
        "{cut}"
    );
    assert!(
        rendered.contains("50 additional") && rendered.contains("100"),
        "{rendered}"
    );
    assert!(!rendered.contains("and 99 more"), "{rendered}");
}

#[test]
#[trace("TC-770", "FR-286-AC-2")]
fn check_refusal_json_and_text_include_every_diagnostic() {
    let text = format!(
        "{HEADER}function fa using v(x: Int[0, 9]): Int[0, 9] pure {{ x + 1 }}\n\
         function fb using v(x: Int[0, 9]): Int[0, 9] pure {{ x + 1 }}\n\
         function fc using v(x: Int[0, 9]): Int[0, 9] pure {{ x + 1 }}\n"
    );
    let (document, rendered) = checked_document(&text);
    let value = json(&document);
    let diagnostics = value["diagnostics"].as_array().expect("diagnostics list");
    assert_eq!(value["operation"], "check", "{value}");
    assert_eq!(diagnostics.len(), 3, "{value}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["code"] == "undefined_expression"
                && diagnostic["cause"] == "unproved-range"
                && !diagnostic["locus"].is_null()
        }),
        "{value}"
    );
    assert_eq!(rendered.matches("unproved-range").count(), 3, "{rendered}");
    assert!(!rendered.contains(", and 2 more"), "{rendered}");
}
