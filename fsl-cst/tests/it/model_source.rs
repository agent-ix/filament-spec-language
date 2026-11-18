// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model source syntax prerequisites for FR-300/301.

use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production, TokenKind};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn source(body: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\npre P using m on M::T::op {{ {body} }}\n")
}

fn parse(text: &str, limits: Limits) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "model-syntax"),
        "model-syntax.native",
        text.as_bytes(),
        limits,
    )
    .expect("source metadata admitted")
}

#[trace("TC-790")]
#[test]
fn model_forms_preserve_source_bytes_and_named_productions() {
    for body in [
        "deref(r).field",
        "allInstances<M::T>(p)",
        "lookup<M::T>(p, r) absent refused",
        "lookup<M::T>(p, r) absent empty",
        "lookup<M::T>(p, r) absent undefined",
        "reaches(r, s, parent)",
        "self.query()",
        "deref(r).query(1, lookup<M::T>(p, r) absent refused).field",
        "r.query().next(2, 3)",
        "r.exhausted()",
    ] {
        let text = source(body);
        let parsed = parse(&text, Limits::default());
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
        assert_eq!(parsed.cst().render(), text.as_bytes());
        assert_eq!(parsed.source().identity().identity, "model-syntax");
        assert_eq!(parsed.effective_limits(), Limits::default());
        let start = text.find(body).unwrap();
        assert!(parsed.cst().nodes().iter().any(|node| node.production()
            == Production::Expression
            && node.span()
                == Span {
                    start,
                    end: start + body.len()
                }));
    }
}

#[trace("TC-790")]
#[test]
fn population_type_references_are_admitted_as_named_type_productions() {
    for spelling in [
        "Population<M::Order>[3]",
        "Population<M::Order>[0]",
        "Population<M::Order>[123456789012345678901234567890]",
        "Population<M::Order>",
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function f using v(p: {spelling}): Boolean pure {{ true }}\n"
        );
        let parsed = parse(&text, Limits::default());
        assert!(
            parsed.is_admissible(),
            "{spelling}: {:?}",
            parsed.diagnostics()
        );
        assert_eq!(parsed.cst().render(), text.as_bytes());
        let start = text.find(spelling).unwrap();
        let population = parsed
            .cst()
            .nodes()
            .iter()
            .find(|node| {
                node.production() == Production::TypeReference && node.span().start == start
            })
            .expect("population type reference");
        assert_eq!(population.span().end, start + spelling.len());
    }
}

#[trace("TC-790")]
#[test]
fn population_type_references_reject_malformed_maxima_at_the_offending_token() {
    for (spelling, offending, cause) in [
        (
            "Population<M::Order>[]",
            "]",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[-1]",
            "-",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[03]",
            "03",
            CompleteCause::InvalidToken,
        ),
        (
            "Population<M::Order>[1.5]",
            "1.5",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[1e3]",
            "1e3",
            CompleteCause::InvalidToken,
        ),
        (
            "Population<M::Order>[*]",
            "*",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order>[0,3]",
            ",",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<M::Order[3]",
            "[",
            CompleteCause::UnexpectedToken,
        ),
        (
            "Population<Boolean>[3]",
            "Boolean",
            CompleteCause::UnexpectedToken,
        ),
        (
            "population<M::Order>[3]",
            "<",
            CompleteCause::UnexpectedToken,
        ),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function f using v(p: {spelling}): Boolean pure {{ true }}\n"
        );
        let parsed = parse(&text, Limits::default());
        assert!(!parsed.is_admissible(), "{spelling}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{spelling}");
        assert_eq!(diagnostic.cause, cause, "{spelling}");
        let spelling_start = text.find(spelling).unwrap();
        let start = spelling_start + text[spelling_start..].find(offending).unwrap();
        let region = diagnostic.region.as_ref().expect("located refusal");
        assert_eq!(
            usize::try_from(region.start()).unwrap(),
            start,
            "{spelling}: text={text:?}, diagnostic={diagnostic:?}"
        );
        assert_eq!(
            usize::try_from(region.end()).unwrap(),
            start + offending.len(),
            "{spelling}: {region:?}"
        );
    }
}

#[trace("TC-790")]
#[test]
fn population_type_reference_reports_missing_maximum_close_at_eof() {
    let spelling = "Population<M::Order>[3";
    let text = format!(
        "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function f using v(p: {spelling}"
    );
    let parsed = parse(&text, Limits::default());
    assert!(!parsed.is_admissible(), "{spelling}");
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax);
    assert_eq!(diagnostic.cause, CompleteCause::UnexpectedEnd);
    let region = diagnostic.region.as_ref().expect("located EOF refusal");
    // The parser's located EOF is the zero-width position at the end of the
    // truncated source.
    let eof = text.len();
    assert_eq!(usize::try_from(region.start()).unwrap(), eof);
    assert_eq!(usize::try_from(region.end()).unwrap(), eof);
}

#[trace("TC-790")]
#[test]
fn population_keyword_is_reserved_only_in_type_and_binder_positions() {
    let uppercase_text = "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; predicate Population using v(): Boolean { true }\n";
    let uppercase = parse(uppercase_text, Limits::default());
    assert!(!uppercase.is_admissible());
    let region = uppercase.diagnostics()[0]
        .region
        .as_ref()
        .expect("located reserved binder refusal");
    let start = uppercase_text.find("Population").unwrap();
    assert_eq!(usize::try_from(region.start()).unwrap(), start);
    assert_eq!(
        usize::try_from(region.end()).unwrap(),
        start + "Population".len()
    );

    let lowercase_text = "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; predicate population using v(): Boolean { true }\n";
    let lowercase = parse(lowercase_text, Limits::default());
    assert!(lowercase.is_admissible(), "{:?}", lowercase.diagnostics());

    for body in ["M::Population", "receiver.Population"] {
        let parsed = parse(&source(body), Limits::default());
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
    }
}

#[trace("TC-790")]
#[test]
fn invalid_model_productions_refuse_at_the_offending_token() {
    for (body, token) in [
        ("lookup<M::T>(p, r)", "}"),
        ("lookup<M::T>(p) absent refused", ")"),
        ("lookup<M::T>(p, r) absent bogus", "bogus"),
        ("lookup<M::T>(p, r, s) absent refused", ", s"),
        ("r.query(, 1)", ","),
        ("r.query(1,)", ")"),
        ("r.query(1 2)", "2"),
        ("r.3", "3"),
    ] {
        let text = source(body);
        let parsed = parse(&text, Limits::default());
        assert!(!parsed.is_admissible(), "{body}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{body}");
        assert_eq!(diagnostic.cause, CompleteCause::UnexpectedToken, "{body}");
        assert_eq!(diagnostic.source.identity, "model-syntax");
        assert_eq!(diagnostic.path, "model-syntax.native");
        let start = text.rfind(token).unwrap();
        let region = diagnostic.region.as_ref().expect("located refusal");
        assert_eq!(
            usize::try_from(region.start()).unwrap(),
            start,
            "{body}: {diagnostic:?}"
        );
        assert_eq!(
            usize::try_from(region.end()).unwrap(),
            start + token.split_whitespace().next().unwrap().len(),
            "{body}"
        );
        assert!(
            diagnostic.message.starts_with("expected "),
            "{diagnostic:?}"
        );
    }
}

#[trace("TC-790")]
#[test]
fn absence_words_remain_identifiers_outside_the_lookup_mode() {
    let text = "language \"ix:native\" edition \"1-draft\"; profile v = \"filament.value.complete/v1\"; function empty using v(undefined: Boolean, refused: Boolean): Boolean pure { undefined and refused }";
    let parsed = parse(text, Limits::default());
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    for word in [b"empty".as_slice(), b"undefined", b"refused"] {
        assert!(parsed
            .cst()
            .tokens()
            .iter()
            .any(|token| token.spelling() == word && token.kind() == TokenKind::Identifier));
    }
}

#[trace("TC-790")]
#[test]
fn model_sources_keep_the_existing_budget_refusals() {
    let text = source("r.query(lookup<M::T>(p, r) absent refused, 3)");
    for limits in [
        Limits::default().with_tokens(1),
        Limits::default().with_nodes(1),
        Limits::default().with_work_units(0),
    ] {
        let diagnostic = match fsl_cst::parse(
            SourceIdentity::new("test", "model-syntax"),
            "model-syntax.native",
            text.as_bytes(),
            limits,
        ) {
            Ok(parsed) => {
                assert!(!parsed.is_admissible());
                assert_eq!(parsed.effective_limits(), limits);
                parsed.diagnostics()[0].clone()
            }
            Err(diagnostic) => *diagnostic,
        };
        assert_eq!(diagnostic.code, CompleteCode::StageLimitExceeded);
        assert!(diagnostic.limit().is_some());
    }
    let refusal = fsl_cst::parse(
        SourceIdentity::new("test", "model-syntax"),
        "model-syntax.native",
        text.as_bytes(),
        Limits::default().with_source_bytes(text.len() - 1),
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
}

/// FR-1000-AC-1 (TC-2833): `M::T#"s1"` parses as one reference-literal node
/// and formats unchanged; a literal with no quoted key or no type refuses at
/// the offending token.
#[trace("TC-2833", "FR-1000-AC-1", "FSpec-TC-965", "FSpec-FR-480-AC-1")]
#[test]
#[ignore = "QSL-2166: the reference-literal production is not in the grammar yet"]
fn a_reference_literal_parses_as_one_node() {
    let literal = "M::T#\"s1\"";
    let text = source(&format!("self = {literal}"));
    let parsed = parse(&text, Limits::default());
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
    let start = text.find(literal).unwrap();
    let literals: Vec<Span> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| format!("{:?}", node.production()) == "ReferenceLiteral")
        .map(|node| node.span())
        .collect();
    assert_eq!(
        literals,
        [Span {
            start,
            end: start + literal.len()
        }]
    );
    let formatted = fsl_cst::format::format(&parsed).expect("formats");
    let reparsed = parse(&formatted, Limits::default());
    assert_eq!(
        fsl_cst::format::format(&reparsed).expect("formats again"),
        formatted
    );
    for (malformed, offending) in [("M::T#s1", "s1"), ("M::T#", "}"), ("#\"s1\"", "#")] {
        let text = source(&format!("self = {malformed}"));
        let parsed = parse(&text, Limits::default());
        assert!(!parsed.is_admissible(), "{malformed}");
        let diagnostic = parsed.diagnostics().first().expect("a diagnostic");
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{malformed}");
        assert_eq!(
            diagnostic.cause,
            CompleteCause::UnexpectedToken,
            "{malformed}"
        );
        let region = diagnostic.region.as_ref().expect("a located diagnostic");
        let from = text.find(malformed).unwrap();
        let at = from + text[from..].find(offending).unwrap();
        assert_eq!(usize::try_from(region.start()).unwrap(), at, "{malformed}");
    }
}

/// The first diagnostic of `text`, whether S1 recovers or stops.
fn first_diagnostic(text: &str) -> fsl_cst::CompleteDiagnostic {
    match fsl_cst::parse(
        SourceIdentity::new("test", "model-syntax"),
        "model-syntax.native",
        text.as_bytes(),
        Limits::default(),
    ) {
        Ok(parsed) => {
            assert!(!parsed.is_admissible(), "{text}");
            parsed.diagnostics()[0].clone()
        }
        Err(diagnostic) => *diagnostic,
    }
}

/// FR-1000-AC-6 (TC-2952): a reference literal is written solid with a
/// string key; each malformed spelling refuses with its exact cause at the
/// offending token, `#` inside a string or comment is text, and the admitted
/// literal's node holds the qualified name, `#` and the string in order.
#[trace("TC-2952", "FR-1000-AC-6", "FSpec-TC-2002", "FSpec-FR-480-AC-6")]
#[test]
#[ignore = "QSL-2166: the reference-literal production, its solid spelling and its refusals are not in the grammar yet"]
fn a_malformed_reference_literal_refuses_at_the_offending_token() {
    // (spelling, offending text, nth occurrence of it in the spelling, cause)
    for (malformed, offending, nth, cause) in [
        ("M::T #\"s1\"", "#", 0, CompleteCause::UnexpectedToken),
        ("(x)#\"s1\"", "#", 0, CompleteCause::UnexpectedToken),
        ("f(x)#\"s1\"", "#", 0, CompleteCause::UnexpectedToken),
        ("M::T#\"a\"#\"b\"", "#", 1, CompleteCause::UnexpectedToken),
        ("M::T# \"s1\"", "\"s1\"", 0, CompleteCause::UnexpectedToken),
        ("M::T#1", "1", 0, CompleteCause::UnexpectedToken),
        ("M::T#)", ")", 0, CompleteCause::UnexpectedToken),
        ("M::T#\"\\q\"", "\"\\q\"", 0, CompleteCause::InvalidEscape),
    ] {
        let text = source(&format!("self = {malformed}"));
        let diagnostic = first_diagnostic(&text);
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{malformed}");
        assert_eq!(diagnostic.cause, cause, "{malformed}");
        let region = diagnostic.region.as_ref().expect("a located diagnostic");
        let from = text.find(malformed).unwrap();
        let at = from
            + malformed
                .match_indices(offending)
                .nth(nth)
                .expect("offending text")
                .0;
        assert_eq!(usize::try_from(region.start()).unwrap(), at, "{malformed}");
    }

    let truncated = source("self = M::T#");
    let truncated = &truncated[..truncated.find("M::T#").unwrap() + "M::T#".len()];
    let diagnostic = first_diagnostic(truncated);
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedEnd)
    );

    for body in ["self = \"a#b\"", "true // a#b\n", "self = M::T#\"\""] {
        let text = source(body);
        let parsed = parse(&text, Limits::default());
        assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
        if !body.contains("M::T#") {
            assert!(
                parsed
                    .cst()
                    .tokens()
                    .iter()
                    .all(|token| token.spelling() != b"#"),
                "{body}"
            );
        }
    }

    let text = source("self = M::T#\"s1\"");
    let parsed = parse(&text, Limits::default());
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let cst = parsed.cst();
    let node = cst
        .nodes()
        .iter()
        .find(|node| format!("{:?}", node.production()) == "ReferenceLiteral")
        .expect("one reference-literal node");
    let children: Vec<String> = node
        .children()
        .iter()
        .map(|child| match *child {
            fsl_cst::CstElement::Node(index) => {
                format!("{:?}", cst.nodes()[index].production())
            }
            fsl_cst::CstElement::Token(index) => {
                String::from_utf8_lossy(cst.tokens()[index].spelling()).into_owned()
            }
        })
        .collect();
    assert_eq!(children, ["QualifiedName", "#", "\"s1\""]);
}
