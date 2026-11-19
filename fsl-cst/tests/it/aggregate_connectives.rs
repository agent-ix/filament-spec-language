// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-982 (TC-2776, TC-2777): a window-aggregate threshold ends at the next
//! `and`, `or` or `implies`, an aggregate atom stands in any temporal operand
//! position, and `holds(count where ..)` is no aggregate atom.
use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

const ATOM: &str = "count where holds(s.ok) over past[0,1]";

fn clause(body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile t = \"filament.temporal.complete/v1\";\n\
         temporal T using t over (s: M::S) on origin {{ {body} }}"
    )
}

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:aggregate-connectives".into(),
        },
        "aggregate-connectives.native",
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

fn refused_at(text: &str, at: usize) {
    let parsed = parse(text);
    let first = parsed.diagnostics().first().expect("refused");
    assert_eq!(
        (first.code, first.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
        "{text}"
    );
    assert_eq!(first.byte_span().expect("located").start, at, "{text}");
}

/// FR-982-AC-1: `and`, `or` and `implies` after the threshold are temporal
/// connectives; an arithmetic threshold stays whole.
#[trace("TC-2776", "FR-982-AC-1")]
#[test]
fn every_connective_after_the_threshold_is_temporal() {
    for (connective, production) in [
        ("and", Production::TemporalConjunction),
        ("or", Production::TemporalDisjunction),
        ("implies", Production::TemporalImplication),
    ] {
        let formula = format!("{ATOM} >= 1 {connective} holds(s.done)");
        let text = clause(&formula);
        let parsed = parse(&text);
        assert!(
            parsed.is_admissible(),
            "{formula}: {:?}",
            parsed.diagnostics()
        );
        assert_eq!(
            texts(&text, &parsed, Production::AggregateAtom),
            [format!("{ATOM} >= 1").as_str()]
        );
        assert!(texts(&text, &parsed, production).contains(&formula.as_str()));
    }
    let text = clause(&format!("{ATOM} >= s.k + 1"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::AggregateAtom),
        [format!("{ATOM} >= s.k + 1").as_str()]
    );
}

/// FR-982-AC-2: a bare value operand after any connective refuses at its
/// first token.
#[trace("TC-2776", "FR-982-AC-2")]
#[test]
fn a_value_operand_after_a_connective_refuses() {
    for connective in ["and", "or", "implies"] {
        let text = clause(&format!("{ATOM} >= s.k {connective} s.ok"));
        let at = text.rfind("s.ok").expect("the operand");
        refused_at(&text, at);
    }
}

/// FR-982-AC-3: a parenthesized threshold, an atom as a middle operand, and
/// the refusal of `count where` inside `holds(...)`.
#[trace("TC-2777", "FR-982-AC-3")]
#[test]
fn an_aggregate_atom_stands_in_any_temporal_position() {
    let text = clause(&format!("{ATOM} >= (s.k and s.ok)"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::AggregateAtom),
        [format!("{ATOM} >= (s.k and s.ok)").as_str()]
    );
    let text = clause(&format!("holds(s.done) and {ATOM} >= 1 and holds(s.ok)"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::AggregateAtom),
        [format!("{ATOM} >= 1").as_str()]
    );
    let text = clause(&format!("always holds({ATOM} >= 1)"));
    refused_at(&text, text.find("where").expect("the word"));
}
