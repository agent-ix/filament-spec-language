// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-984 (TC-2780): an indexed read is refused by where it is written; a
//! non-hyper temporal clause refuses it at S3 (exit 21), a function body at
//! S1 (exit 20), and a hyper clause admits it.
use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_replay::OutcomeDocument;
use ix_trace_rs::trace;
use serde_json::Value;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile c = \"filament.temporal.infinite-trace/v1\";\n";

fn json(document: &OutcomeDocument) -> (Value, u8) {
    let value = serde_json::from_slice(&document.to_bytes().expect("the document encodes"))
        .expect("the document is JSON");
    (value, document.category().exit_code())
}

fn parse_document(text: &str) -> (Value, u8) {
    let source = SourceIdentity::new("agent-ix", "test:indexed-read");
    let result = spine::parse(
        &ParseRequest {
            source: &source,
            path: "indexed-read.native",
            bytes: text.as_bytes(),
        },
        SpineLimits::default().source,
        &Cancel::new(),
    );
    json(&OutcomeDocument::from_parse(&result))
}

fn check_document(text: &str) -> (Value, u8) {
    let source = SourceIdentity::new("agent-ix", "test:indexed-read");
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "indexed-read.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &Cancel::new(),
    )
    .expect("the source parses")
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &Cancel::new())
        .expect("no model is selected")
        .into_value();
    let checked = spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &Cancel::new(),
    );
    json(&OutcomeDocument::from_check(&checked))
}

/// FR-984-AC-1: a non-hyper temporal clause refuses `@` at S3, exit 21.
#[trace("TC-2780", "FR-984-AC-1", "FSpec-TC-1003", "FSpec-FR-501-AC-2")]
#[test]
#[ignore = "QSL-2162: the S3 refusal of `@` in a non-hyper temporal clause is not locked through the outcome document yet"]
fn an_indexed_read_in_a_non_hyper_temporal_clause_refuses_at_s3() {
    let text = format!(
        "{HEADER}record R {{ l: Integer; }}\n\
         temporal T using c over (v: R) on origin {{ always holds(v.l @ a) }}\n"
    );
    let (value, exit) = check_document(&text);
    assert_eq!(
        (value["category"].as_str(), exit),
        (Some("unsupported"), 21),
        "{value}"
    );
    let diagnostic = &value["diagnostics"][0];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{value}");
    assert_eq!(diagnostic["cause"], "expression-form", "{value}");
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap_or_default()
            .contains("hyper"),
        "{value}"
    );
}

/// FR-984-AC-2: in a function body `@` is a syntax error at S1, exit 20.
#[trace("TC-2780", "FR-984-AC-2")]
#[test]
#[ignore = "QSL-2162: the S1 refusal category and exit of `@` in a function body are not locked yet"]
fn an_indexed_read_in_a_function_body_is_a_syntax_error() {
    let text = format!("{HEADER}function f using v(x: Integer): Integer pure {{ x @ a }}\n");
    let (value, exit) = parse_document(&text);
    assert_eq!(
        (value["category"].as_str(), exit),
        (Some("refusal"), 20),
        "{value}"
    );
    let diagnostic = &value["diagnostics"][0];
    assert_eq!(diagnostic["code"], "invalid_syntax", "{value}");
    assert_eq!(diagnostic["cause"], "unexpected-token", "{value}");
}

/// FR-984-AC-3: a hyper clause over behaviours admits two indexed reads.
#[trace("TC-2780", "FR-984-AC-3")]
#[test]
#[ignore = "QSL-2162: the positive hyper case is locked with the two refusals; not run as a gate test yet"]
fn a_hyper_clause_admits_indexed_reads() {
    let text = format!(
        "{HEADER}model V = \"acme/vault\";\n\
         hyper NI using c over behaviours (v: V::Vault) forall trace a of V forall trace b of V \
         {{ always holds(v.l @ a = v.l @ b) }}\n"
    );
    let (value, exit) = parse_document(&text);
    assert_eq!(exit, 0, "{value}");
}
