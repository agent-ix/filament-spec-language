// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002-AC-2, AC-5 and AC-6: the `1-draft` expression grammar refuses a
//! chained comparison at its second operator, a parenthesized expression
//! spans its parentheses, and a bounded malformed corpus never panics.
use fsl_cst::{CompleteCode, Limits, ParsedSource};
use fsl_foundation::{Phase, SourceIdentity};
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

#[trace("FSpec-TC-1648", "FSpec-FR-662-AC-1")]
#[test]
fn a_population_query_type_argument_names_the_declared_type_requirement() {
    for (expression, anchor) in [
        ("sum<Int[0, 80]>(x in a: x)", "Int"),
        ("count<Integer>(x in a: x)", "Integer"),
        ("fold<Integer>(0, x in a: x)", "Integer"),
        ("reduce<Int[0, 9]>(0, x in a: x)", "Int"),
    ] {
        let text = document(expression);
        let parsed = parse(text.as_bytes()).expect("valid UTF-8 within the source ceiling");
        assert!(!parsed.is_admissible(), "{expression}");
        let diagnostic = parsed.diagnostics().first().expect("a syntax refusal");
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{expression}");
        assert_eq!(diagnostic.phase, Phase::Parse, "{expression}");
        let start = text.find(expression).expect("query in the document")
            + expression.find(anchor).expect("anchor in type argument");
        assert_eq!(
            diagnostic.byte_span().expect("located").start,
            start,
            "{expression}"
        );
        assert!(
            diagnostic.message.contains("declared type name here"),
            "{}",
            diagnostic.message
        );
        assert!(diagnostic.message.contains("type Total = Integer;"));
    }

    for expression in ["sum<M::Total>(x in a: x)", "fold<M::Integer>(0, x in a: x)"] {
        let text = document(expression);
        let parsed = parse(text.as_bytes()).expect("valid UTF-8 within the source ceiling");
        assert!(
            parsed.is_admissible(),
            "{expression}: {:?}",
            parsed.diagnostics()
        );
        let formatted = fsl_cst::format::format(&parsed).expect("accepted query formats");
        let reparsed = parse(formatted.as_bytes()).expect("formatted source is valid UTF-8");
        assert!(
            reparsed.is_admissible(),
            "{expression}: {:?}",
            reparsed.diagnostics()
        );
        let significant = |source: &ParsedSource| {
            source
                .cst()
                .tokens()
                .iter()
                .filter(|token| {
                    !matches!(
                        token.kind(),
                        fsl_cst::TokenKind::Whitespace | fsl_cst::TokenKind::Comment
                    )
                })
                .map(|token| token.spelling().to_vec())
                .collect::<Vec<_>>()
        };
        assert_eq!(significant(&parsed), significant(&reparsed), "{expression}");
        assert_eq!(
            fsl_cst::format::format(&reparsed).expect("idempotent formatting"),
            formatted,
            "{expression}"
        );
    }

    for (expression, offending) in [("sum<>(x in a: x)", ">"), ("sum<123>(x in a: x)", "123")] {
        let text = document(expression);
        let parsed = parse(text.as_bytes()).expect("valid UTF-8 within the source ceiling");
        assert!(!parsed.is_admissible(), "{expression}");
        let diagnostic = parsed.diagnostics().first().expect("a syntax refusal");
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{expression}");
        assert_eq!(
            diagnostic.byte_span().expect("located").start,
            text.find(offending).expect("offending token"),
            "{expression}"
        );
    }
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
        assert!(
            parsed.cst().recoveries().is_empty(),
            "{expression}: an admitted primary has no refusal or expected-token recovery"
        );
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
    for unchanged in ["decimal(5, 1) = decimal(5, 1)", "5", "a.b"] {
        let text = document(unchanged);
        let parsed = parse(text.as_bytes()).expect("S1 reads the unit");
        assert!(
            parsed.is_admissible(),
            "{unchanged}: {:?}",
            parsed.diagnostics()
        );
    }
    let quantity = "0.5 M::ms";
    let text = document(quantity);
    let parsed = parse(text.as_bytes()).expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let start = text.find(quantity).expect("quantity in the unit");
    assert!(
        parsed.cst().nodes().iter().any(|node| {
            node.production() == fsl_cst::Production::QuantityLiteral
                && node.span().start == start
                && node.span().end == start + quantity.len()
        }),
        "one QuantityLiteral spans exactly `{quantity}`"
    );
}

/// FR-764-AC-4: an exponent spelling is one numeric token, refused
/// `invalid_syntax`/`invalid-token` at the whole spelling.
#[trace("TC-2161", "FR-764-AC-4")]
#[trace("FSpec-TC-1648", "FSpec-FR-662-AC-4")]
#[test]
fn an_exponent_number_is_the_unsupported_numeric_form() {
    for spelling in ["1e5", "1.5E-3"] {
        let text = document(spelling);
        let parsed = parse(text.as_bytes()).expect("S1 reads the unit");
        assert!(!parsed.is_admissible());
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.phase, Phase::Lex, "{spelling}");
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
            diagnostic
                .message
                .contains("an exponent is not part of a number"),
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
