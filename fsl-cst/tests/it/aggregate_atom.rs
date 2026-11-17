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
