// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002-AC-2, AC-5 and AC-6: the `1-draft` expression grammar refuses a
//! chained comparison at its second operator, a parenthesized expression
//! spans its parentheses, and a bounded malformed corpus never panics.
use fsl_cst::{CompleteCode, Limits, ParsedSource};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn document(expression: &str) -> String {
    format!(
        "{HEADER}function f using v(a: Integer, b: Integer, c: Integer, d: Integer): Boolean pure {{ {expression} }}\n"
    )
}

fn parse(bytes: &[u8]) -> Result<ParsedSource, Box<fsl_cst::CompleteDiagnostic>> {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:expression-syntax"),
        "expression.native",
        bytes,
        Limits::default(),
    )
}

#[trace("TC-012", "FR-002-AC-2")]
#[test]
fn a_chained_comparison_refuses_at_its_second_operator() {
    for (expression, second) in [
        ("a < b < c", "< c"),
        ("a = b != c", "!= c"),
        ("a < b + c = d", "= d"),
    ] {
        let text = document(expression);
        let parsed = parse(text.as_bytes()).expect("valid UTF-8 within the source ceiling");
        assert!(!parsed.is_admissible(), "{expression}");
        let diagnostic = parsed
            .diagnostics()
            .first()
            .unwrap_or_else(|| panic!("{expression} holds no diagnostic"));
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{expression}");
        let region = diagnostic.region.as_ref().expect("a located diagnostic");
        let at = text.find(expression).unwrap() + expression.find(second).unwrap();
        assert_eq!(usize::try_from(region.start()).unwrap(), at, "{expression}");
    }
}

#[trace("TC-012", "FR-002-AC-5")]
#[test]
fn a_parenthesized_expression_spans_its_parentheses() {
    let outer = "((a)) < (b + c)";
    let text = document(outer);
    let parsed = parse(text.as_bytes()).expect("valid UTF-8 within the source ceiling");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let base = text.find(outer).unwrap();
    let spans: Vec<(usize, usize)> = parsed
        .cst()
        .nodes()
        .iter()
        .map(|node| (node.span().start, node.span().end))
        .collect();
    for group in ["((a))", "(a)", "(b + c)"] {
        let start = base + outer.find(group).unwrap();
        assert!(
            spans.contains(&(start, start + group.len())),
            "no CST node spans exactly {group}"
        );
    }
}

#[trace("TC-019", "FR-002-AC-6")]
#[test]
fn a_bounded_malformed_corpus_returns_a_parse_or_a_diagnostic() {
    let check = |bytes: &[u8]| match parse(bytes) {
        Ok(parsed) if parsed.is_admissible() => {
            assert_eq!(parsed.source().text().as_bytes(), bytes);
        }
        Ok(parsed) => {
            for diagnostic in parsed.diagnostics() {
                let region = diagnostic.region.as_ref().expect("a located diagnostic");
                assert!(region.start() <= region.end());
                assert!(usize::try_from(region.end()).unwrap() <= bytes.len());
            }
        }
        Err(refusal) => {
            if let Some(region) = refusal.region.as_ref() {
                assert!(region.start() <= region.end());
                assert!(usize::try_from(region.end()).unwrap() <= bytes.len());
            }
        }
    };
    let valid = document("(a < b) and not (c = d) or a + b * c = d");
    for end in 0..valid.len() {
        check(&valid.as_bytes()[..end]);
    }
    let alphabet = b"\x00\xff\r\n\\\"{}()[]<>=:/019ae ";
    for &a in alphabet {
        for &b in alphabet {
            for &c in alphabet {
                check(&[a, b, c]);
                let mut text = document("true").into_bytes();
                let at = text.len() - 3;
                text.splice(at..at, [a, b, c]);
                check(&text);
            }
        }
    }
}

/// FR-764-AC-1: a bare decimal literal is a primary; a decimal literal
/// followed by a qualified name stays a quantity literal.
#[trace("TC-2161", "FR-764-AC-1")]
#[trace("FSpec-TC-1648", "FSpec-FR-662-AC-1")]
#[test]
#[ignore = "QSL-1965: the bare decimal primary not implemented yet"]
fn a_bare_decimal_in_a_primary_parses_as_a_decimal_literal() {
    for (expression, literal) in [("a < 0.9", "0.9"), ("a <= 5.0", "5.0"), ("0.5", "0.5")] {
        let text = document(expression);
        let parsed = parse(text.as_bytes()).expect("S1 reads the unit");
        assert!(
            parsed.is_admissible(),
            "{expression}: {:?}",
            parsed.diagnostics()
        );
        assert!(parsed.diagnostics().is_empty(), "{expression}");
        let start = text.rfind(literal).expect("in the unit");
        assert!(
            parsed.cst().nodes().iter().any(|node| {
                node.production() == fsl_cst::Production::Primary
                    && node.span().start == start
                    && node.span().end == start + literal.len()
            }),
            "{expression}: a primary spans exactly `{literal}`"
        );
    }
    for unchanged in ["decimal(5, 1) = decimal(5, 1)", "a = 5"] {
        let parsed = parse(document(unchanged).as_bytes()).expect("S1 reads the unit");
        assert!(
            parsed.is_admissible(),
            "{unchanged}: {:?}",
            parsed.diagnostics()
        );
    }
}

/// FR-764-AC-4: an exponent spelling is one numeric token, refused
/// `invalid_syntax`/`invalid-token` at the whole spelling.
#[trace("TC-2161", "FR-764-AC-4")]
#[trace("FSpec-TC-1648", "FSpec-FR-662-AC-4")]
#[test]
#[ignore = "QSL-1965: the unsupported numeric-form diagnostic not implemented yet"]
fn an_exponent_number_is_the_unsupported_numeric_form() {
    for spelling in ["1e5", "1.5E-3"] {
        let text = document(&format!("a < {spelling}"));
        let parsed = parse(text.as_bytes()).expect("S1 reads the unit");
        assert!(!parsed.is_admissible());
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{spelling}");
        assert_eq!(
            diagnostic.cause,
            fsl_cst::CompleteCause::InvalidToken,
            "{spelling}"
        );
        let start = text.rfind(spelling).expect("in the unit");
        let span = diagnostic.byte_span().expect("located");
        assert_eq!(
            (span.start, span.end),
            (start, start + spelling.len()),
            "{spelling}"
        );
        assert!(
            diagnostic.message.contains(spelling),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains("decimal(c, s)"),
            "{}",
            diagnostic.message
        );
    }
}
