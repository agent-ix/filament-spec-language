// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-763: every S2 refusal for a construct no form represents, or a
//! declaration no dispatch entry reads, carries
//! `unsupported_construct`/`not-yet-implemented` with the construct's kind,
//! text, owning declaration and profile, and a message that names the
//! construct. Observed through the `parse` operation's outcome document,
//! which runs S1 and S2.
use filament_exact::Cancel;
use fsl_cst::Limits;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{check, parse, select, DependencyInput, ParseRequest, SpineLimits};
use fsl_replay::OutcomeDocument;
use ix_trace_rs::trace;
use serde_json::Value;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     model M = \"acme/model\";\n";

const PROFILE: &str = "filament.value.complete/v1";

/// The unit's text and its `parse` outcome document as JSON.
fn parsed(declaration: &str) -> (String, Value, u8) {
    let text = format!("{HEADER}{declaration}\n");
    let identity = SourceIdentity::new("agent-ix", "test:forms-refusal");
    let result = parse(
        &ParseRequest {
            source: &identity,
            path: "forms-refusal.native",
            bytes: text.as_bytes(),
        },
        Limits::default(),
        &Cancel::new(),
    );
    let document = OutcomeDocument::from_parse(&result);
    let exit = document.category().exit_code();
    let value = serde_json::from_slice(&document.to_bytes().expect("the document encodes"))
        .expect("the document is JSON");
    (text, value, exit)
}

/// The one diagnostic of a unit S2 refuses, checked for the FR-763 code and
/// cause.
fn construct_diagnostic(declaration: &str) -> (String, Value) {
    let (text, value, exit) = parsed(declaration);
    assert_eq!(exit, 21, "{value}");
    assert_eq!(value["category"], "unsupported");
    let diagnostics = value["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{declaration}: {diagnostics:?}");
    let diagnostic = diagnostics[0].clone();
    assert_eq!(diagnostic["code"], "unsupported_construct", "{declaration}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{declaration}");
    (text, diagnostic)
}

fn function(body: &str) -> String {
    format!("function f using v(xs: Sequence<Integer>[0, 4]): Integer pure {{ {body} }}")
}

#[trace("TC-2155", "FR-763-AC-1")]
#[test]
fn an_undispatched_declaration_names_its_keyword() {
    for declaration in [
        "synthesis Pick using v grammar Digit domain Digit satisfies { Pick >= 3 };",
        "verify Release using v { check safe claim M::Safe method model_check domain M::Order bound 20; }",
    ] {
        let (_, value, _) = parsed(declaration);
        let diagnostics = value["diagnostics"].as_array().cloned().unwrap_or_default();
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic["code"] != "unsupported_construct"
                    && diagnostic["construct"] != "declaration"
            }),
            "{declaration}: {diagnostics:?}"
        );
    }
    let keywords: Vec<_> = fsl_forms::leading_keywords().collect();
    assert!(keywords.contains(&"synthesis"), "{keywords:?}");
    assert!(keywords.contains(&"verify"), "{keywords:?}");
}

#[trace("TC-2156", "FR-763-AC-2")]
#[test]
fn null_and_index_forms_parse_without_construct_refusals() {
    for body in ["null", "xs[0]"] {
        let (_, document, exit) = parsed(&function(body));
        assert_eq!(exit, 0, "{document}");
        assert_eq!(document["diagnostics"], serde_json::json!([]), "{body}");
    }
}

#[trace("TC-2157", "FR-763-AC-3")]
#[test]
fn typed_size_and_nested_index_forms_parse_without_construct_refusals() {
    for body in ["size<Integer>(xs)", "size(xs) + xs[1]"] {
        let (_, document, exit) = parsed(&function(body));
        assert_eq!(exit, 0, "{document}");
        assert_eq!(document["diagnostics"], serde_json::json!([]), "{body}");
    }
}

#[trace("TC-2158", "FR-763-AC-4")]
#[test]
fn a_qualified_reaches_edge_builds_without_a_forms_refusal() {
    let edge = "Config::ConfigVersion::parent";
    let (_, document, exit) = parsed(&format!(
        "invariant I using v on M::Order at current {{ reaches(self, self, {edge}) }}"
    ));
    assert_eq!(exit, 0, "{document}");
    assert_eq!(document["diagnostics"], serde_json::json!([]));
}

#[trace("TC-2159", "FR-763-AC-5")]
#[trace("TC-396")]
#[test]
fn every_construct_message_begins_with_the_construct() {
    let long = format!("{} @ a = x @ b", "x".repeat(188));
    assert_eq!(long.len(), 200);
    for condition in ["x @ a = x @ b", long.as_str()] {
        let declaration = format!(
            "temporal T using v over (x: M::Order) clock \"steps\" on origin {{ always holds({condition}) }}"
        );
        let (source, diagnostic) = construct_diagnostic(&declaration);
        let message = diagnostic["message"].as_str().expect("a message");
        assert!(
            message.starts_with("a hyper expression is not supported as a temporal condition yet"),
            "{message}"
        );
        assert_eq!(diagnostic["construct"], "hyper-condition");
        assert_eq!(diagnostic["declaration"], "T");
        assert_eq!(diagnostic["profile"], PROFILE);
        assert_eq!(
            locus_start(&diagnostic),
            Some(source.find(condition).expect("condition") as u64)
        );
        let text = diagnostic["text"].as_str().expect("text");
        if condition.len() <= 120 {
            assert_eq!(text, condition);
        } else {
            assert_eq!(text, format!("{}...", &condition[..120]));
        }
    }
}

#[trace("TC-2160", "FR-763-AC-6")]
#[trace("TC-2785", "FR-986-AC-1", "FR-986-AC-2")]
#[test]
fn a_null_expression_is_refused_at_check_with_its_locus() {
    for declaration in [
        "function f using v(): Option<Integer> pure { null }",
        "function g using v(a: Option<Integer>): Boolean pure { a = null }",
        "function h using v(): Integer pure { null }",
        "function k using v(): Boolean pure { let x = null in true }",
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{PROFILE}\";\n{}\n",
            declaration
        );
        let identity = SourceIdentity::new("agent-ix", "test:null-check");
        let limits = SpineLimits::default();
        let cancel = Cancel::new();
        let parsed = parse(
            &ParseRequest {
                source: &identity,
                path: "null.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("S2 admits null")
        .into_value();
        let models = select(
            &parsed,
            &fsl_semantics::model::intake::PackageInput::default(),
            limits.model,
            &cancel,
        )
        .expect("no model imports")
        .into_value();
        let result = check(
            &parsed,
            "null-check".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        );
        assert!(result.is_err(), "null is never a value");
        let document = OutcomeDocument::from_check(&result);
        assert_eq!(document.category().exit_code(), 20);
        let value: Value =
            serde_json::from_slice(&document.to_bytes().expect("encoded check outcome"))
                .expect("JSON");
        assert_eq!(value["category"], "refusal");
        assert_eq!(
            value["diagnostics"].as_array().expect("diagnostics").len(),
            1
        );
        let diagnostic = &value["diagnostics"][0];
        assert_eq!(diagnostic["code"], "ill_typed");
        assert_eq!(diagnostic["cause"], "type-mismatch");
        let start = text.rfind("null").expect("null expression") as u64;
        assert_eq!(locus_start(diagnostic), Some(start));
        assert_eq!(diagnostic["locus"]["end"].as_u64(), Some(start + 4));
        let message = diagnostic["message"].as_str().expect("message");
        for spelling in ["null", "none", "absent"] {
            assert!(message.contains(spelling), "{message}");
        }
    }
}

/// The start byte of a diagnostic's source locus, written as a JSON number
/// or as an ASCII decimal string.
fn locus_start(diagnostic: &Value) -> Option<u64> {
    let start = &diagnostic["locus"]["start"];
    start
        .as_u64()
        .or_else(|| start.as_str().and_then(|text| text.parse().ok()))
}
