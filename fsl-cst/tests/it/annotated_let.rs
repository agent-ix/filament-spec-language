// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-980 (TC-2770, TC-2772): `let x: T = e in b` parses with the
//! annotation on the binder, malformed annotations refuse at the offending
//! token, and the formatter keeps the annotation's tokens.
use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production, TokenClass};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:annotated-let".into(),
        },
        "annotated-let.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn texts<'t>(text: &'t str, parsed: &ParsedSource, production: Production) -> Vec<&'t str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| {
            let Span { start, end } = node.span();
            &text[start..end]
        })
        .collect()
}

fn significant(parsed: &ParsedSource) -> Vec<Vec<u8>> {
    parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .map(|token| token.spelling().to_vec())
        .collect()
}

/// FR-980-AC-1: the annotation is a `TypeReference` inside the `let`.
#[trace("TC-2770", "FR-980-AC-1")]
#[test]
fn an_annotated_let_holds_its_type_reference() {
    let text =
        unit("function f using v(y: Int[0,2]): Integer pure { let x: Int[0,3] = y + 1 in x }");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let lets: Vec<_> = texts(&text, &parsed, Production::Expression)
        .into_iter()
        .filter(|expression| expression.starts_with("let"))
        .collect();
    assert_eq!(lets, ["let x: Int[0,3] = y + 1 in x"]);
    assert!(texts(&text, &parsed, Production::TypeReference).contains(&"Int[0,3]"));
}

/// FR-980-AC-2: each malformed annotation refuses at the token where the
/// grammar requires something else.
#[trace("TC-2770", "FR-980-AC-2")]
#[test]
fn a_malformed_let_annotation_refuses_at_the_offending_token() {
    for (body, at) in [
        ("let x: = 1 in x", "= 1"),
        ("let x: Int[0,3] 1 in x", "1 in"),
        ("let x Int[0,3] = 1 in x", "Int"),
    ] {
        let text = unit(&format!("function f using v(): Integer pure {{ {body} }}"));
        let parsed = parse(&text);
        let first = parsed.diagnostics().first().expect("refused");
        assert_eq!(
            (first.code, first.cause),
            (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
            "{body}"
        );
        let body_at = text.find(body).expect("the body");
        assert_eq!(
            first.byte_span().expect("located").start,
            body_at + body.find(at).expect("the token"),
            "{body}"
        );
    }
}

/// FR-980-AC-5: formatting keeps every annotation token in order and is
/// idempotent.
#[trace("TC-2772", "FR-980-AC-5")]
#[test]
#[ignore = "QSL-1974 integration: the canonical formatter is not on this branch yet"]
fn an_annotated_let_formats_and_keeps_its_tokens() {
    let text = unit("function f using v(y: Int[0,2]): Integer pure { let x:Int[0,3]=y+1 in x }");
    let parsed = parse(&text);
    let formatted = fsl_cst::format::format(&parsed).expect("formats");
    let reparsed = parse(&formatted);
    assert_eq!(significant(&parsed), significant(&reparsed));
    assert_eq!(
        fsl_cst::format::format(&reparsed).expect("formats again"),
        formatted
    );
    assert!(
        formatted.contains("let x: Int[0, 3] = y + 1 in x"),
        "{formatted}"
    );
}
