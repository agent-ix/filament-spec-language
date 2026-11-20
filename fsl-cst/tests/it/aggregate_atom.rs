// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-501 item 1: the threshold of a window-aggregate atom is a `sum`,
//! the operand a comparison takes, so a following `and`, `or` or `implies`
//! joins the atom to the next temporal operand and never becomes part of the
//! threshold.
use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

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
            identity: "test:aggregate-atom".into(),
        },
        "aggregate.native",
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

/// FSpec FR-501-AC-1: `... >= 1 and holds(s.done)` is one temporal
/// conjunction of the atom and `holds(s.done)`, and `>= s.k + 1` keeps the
/// arithmetic threshold.
#[trace("FSpec-TC-1002", "FSpec-FR-501-AC-1")]
#[test]
fn an_aggregate_followed_by_and_is_a_temporal_conjunction() {
    let text = clause("count where holds(s.ok) over past[0,1] >= 1 and holds(s.done)");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::AggregateAtom),
        ["count where holds(s.ok) over past[0,1] >= 1"]
    );
    assert!(texts(&text, &parsed, Production::TemporalConjunction)
        .contains(&"count where holds(s.ok) over past[0,1] >= 1 and holds(s.done)"));

    let text = clause("count where holds(s.ok) over past[0,1] >= s.k + 1");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&text, &parsed, Production::AggregateAtom),
        ["count where holds(s.ok) over past[0,1] >= s.k + 1"]
    );
}

/// FSpec FR-501-AC-1: `>= s.k and s.ok` is never the threshold
/// `(s.k and s.ok)`; the bare `s.ok` after `and` is no temporal operand, so
/// the unit refuses at that `s`.
#[trace("FSpec-TC-1002", "FSpec-FR-501-AC-1")]
#[test]
fn a_boolean_connective_never_joins_the_threshold() {
    let text = clause("count where holds(s.ok) over past[0,1] >= s.k and s.ok");
    let parsed = parse(&text);
    assert!(!parsed.is_admissible());
    let first = &parsed.diagnostics()[0];
    assert_eq!(
        (first.code, first.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(
        first.byte_span().expect("located").start,
        text.find("and s.ok").expect("the conjunct") + 4
    );
}

fn paired_atom(valid: &str, invalid: &str, anchor: &str, token: &str, expected: Option<&str>) {
    let good = format!("temporal NearMiss using v over (s: M::S) on origin {{ {valid} }}");
    let text = super::claim_syntax::unit(&good);
    let parsed = super::claim_syntax::admitted(&text);
    assert_eq!(
        super::claim_syntax::node_texts(&text, &parsed, Production::AggregateAtom),
        [valid]
    );
    let bad = format!("temporal NearMiss using v over (s: M::S) on origin {{ {invalid} }}");
    match expected {
        Some(expected) => super::claim_syntax::refused(&bad, anchor, token, expected),
        None => {
            super::claim_syntax::refused_at(&bad, anchor, token);
        }
    }
}

#[trace("FR-002-AC-6")]
#[test]
fn aggregate_kind_must_be_one_of_the_grammar_alternatives() {
    paired_atom(
        "count where holds(s.ok) over past[0,1] >= 1",
        "average where holds(s.ok) over past[0,1] >= 1",
        "average",
        "average",
        None,
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn past_window_requires_its_interval_after_past() {
    paired_atom(
        "sum of s.amount over past[0,2] >= 1",
        "sum of s.amount over past >= 1",
        ">=",
        ">=",
        Some("expected `[`"),
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn aggregate_atom_requires_a_comparison_after_the_complete_term() {
    paired_atom(
        "max of s.amount over past[0,2] >= 1",
        "max of s.amount over past[0,2] 1",
        "1 }",
        "1",
        Some("expected `<` or `<=` or `=` or `>` or `>=`"),
    );
}

#[trace("FR-002-AC-6")]
#[test]
fn past_window_requires_a_separator_between_its_two_horizons() {
    paired_atom(
        "min of s.amount over past[0,2] >= 1",
        "min of s.amount over past[0 2] >= 1",
        "2]",
        "2",
        Some("expected `,`"),
    );
}
