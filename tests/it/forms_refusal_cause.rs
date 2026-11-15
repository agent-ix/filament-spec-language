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
use fsl_replay::spine::{parse, ParseRequest};
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
    let (text, value, _) = parsed(declaration);
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
#[ignore = "QSL-2196: the forms-stage construct payload not implemented yet"]
fn an_undispatched_declaration_names_its_keyword() {
    for (declaration, keyword, name) in [
        (
            "synthesis Pick using v grammar Digit domain Digit satisfies { Pick >= 3 };",
            "synthesis",
            "Pick",
        ),
        (
            "verify Release using v { check safe claim M::Safe method model_check domain M::Order bound 20; }",
            "verify",
            "Release",
        ),
    ] {
        let (_, diagnostic) = construct_diagnostic(declaration);
        assert_eq!(diagnostic["construct"], "declaration", "{keyword}");
        assert_eq!(diagnostic["text"], keyword);
        assert_eq!(diagnostic["declaration"], name);
        assert_eq!(diagnostic["profile"], PROFILE);
    }
}

#[trace("TC-2156", "FR-763-AC-2")]
#[test]
#[ignore = "QSL-2196: the forms-stage construct payload not implemented yet"]
fn a_null_primary_names_null() {
    let (text, diagnostic) = construct_diagnostic(&function("null"));
    assert_eq!(diagnostic["construct"], "null-value");
    assert_eq!(diagnostic["text"], "null");
    assert_eq!(diagnostic["declaration"], "f");
    assert_eq!(diagnostic["profile"], PROFILE);
    let at = text.rfind("null").expect("in the unit");
    assert_eq!(locus_start(&diagnostic), Some(at as u64));
}

#[trace("TC-2157", "FR-763-AC-3")]
#[test]
#[ignore = "QSL-2196: the forms-stage construct payload not implemented yet"]
fn size_of_type_and_index_selection_name_their_construct() {
    for (body, construct) in [
        ("size<Integer>(xs)", "size-of-type"),
        ("xs[0]", "index-selection"),
    ] {
        let (_, diagnostic) = construct_diagnostic(&function(body));
        assert_eq!(diagnostic["construct"], construct, "{body}");
        assert_eq!(diagnostic["text"], body);
    }
}

#[trace("TC-2158", "FR-763-AC-4")]
#[test]
#[ignore = "QSL-2196: the forms-stage construct payload not implemented yet"]
fn a_qualified_reaches_edge_names_the_edge() {
    let edge = "Config::ConfigVersion::parent";
    let (text, diagnostic) = construct_diagnostic(&format!(
        "invariant I using v on M::Order at current {{ reaches(self, self, {edge}) }}"
    ));
    assert_eq!(diagnostic["construct"], "qualified-reaches-edge");
    assert_eq!(diagnostic["text"], edge);
    let at = text.find(edge).expect("in the unit");
    assert_eq!(locus_start(&diagnostic), Some(at as u64));
}

#[trace("TC-2159", "FR-763-AC-5")]
#[test]
#[ignore = "QSL-2196: the forms-stage construct message not implemented yet"]
fn every_construct_message_begins_with_the_construct() {
    for (declaration, prefix) in [
        (
            function("null"),
            "`null` is not supported as a value here yet".to_owned(),
        ),
        (
            function("xs[0]"),
            "the index selection `xs[0]` is not supported here yet".to_owned(),
        ),
        (
            function("size<Integer>(xs)"),
            "`size<Integer>` is not supported in expressions yet".to_owned(),
        ),
        (
            "invariant I using v on M::Order at current { reaches(self, self, M::Order::next) }"
                .to_owned(),
            "`reaches` over the qualified edge `M::Order::next` is not supported yet".to_owned(),
        ),
        (
            "synthesis Pick using v grammar Digit domain Digit satisfies { Pick >= 3 };".to_owned(),
            "`synthesis` declarations are not supported yet".to_owned(),
        ),
    ] {
        let (_, diagnostic) = construct_diagnostic(&declaration);
        let message = diagnostic["message"].as_str().expect("a message");
        assert!(message.starts_with(&prefix), "{message}");
        assert!(!diagnostic["text"].as_str().expect("text").is_empty());
    }
    let long = format!("size<Integer>({})", "xs".repeat(92));
    let (_, diagnostic) = construct_diagnostic(&function(&long));
    let text = diagnostic["text"].as_str().expect("text");
    assert!(text.len() <= 123 && text.ends_with("..."), "{text}");
}

#[trace("TC-2160", "FR-763-AC-6")]
#[test]
#[ignore = "QSL-2196: the forms-stage construct payload not implemented yet"]
fn the_parse_document_renders_the_construct_cause() {
    let (_, value, exit) = parsed(&function("null"));
    assert_eq!(value["category"], "unsupported");
    assert_eq!(exit, 21);
    let diagnostic = &value["diagnostics"][0];
    assert!(!diagnostic["cause"].is_null());
    assert_eq!(diagnostic["cause"], "not-yet-implemented");
    assert_eq!(diagnostic["construct"], "null-value");
    assert_eq!(diagnostic["text"], "null");
    assert_eq!(diagnostic["declaration"], "f");
    assert_eq!(diagnostic["profile"], PROFILE);
    assert!(diagnostic["message"]
        .as_str()
        .expect("a message")
        .starts_with("`null` is not supported as a value here yet"));
}

/// The start byte of a diagnostic's source locus, written as a JSON number
/// or as an ASCII decimal string.
fn locus_start(diagnostic: &Value) -> Option<u64> {
    let start = &diagnostic["locus"]["start"];
    start
        .as_u64()
        .or_else(|| start.as_str().and_then(|text| text.parse().ok()))
}
