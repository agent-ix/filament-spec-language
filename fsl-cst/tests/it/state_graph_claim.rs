// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-graph recognition distinguishes syntactic placement from S3 admission.

use fsl_cst::{CstElement, ParsedSource, Production, TokenClass};
use ix_trace_rs::trace;

use super::claim_syntax;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Possible,
    AlwaysPossible,
    UniquePath,
}

struct Row {
    accepted: &'static str,
    kind: Kind,
    over: bool,
    rejected: &'static str,
    anchor: &'static str,
    token: &'static str,
    expected: &'static str,
}

// Current shared-grammar state-graph distinctions. Temporal predicates and
// leading fairness are recognized at S1; their semantic refusal belongs to S3.
const ROWS: [Row; 13] = [
    Row {
        accepted: "possible Win using v over (g: G::Game) { g.phase = G::Phase::Won }",
        kind: Kind::Possible,
        over: true,
        rejected: "possible Win using v (g: G::Game) { g.phase = G::Phase::Won }",
        anchor: "(g:",
        token: "(",
        expected: "expected `{`",
    },
    Row {
        accepted: "always possible Recover using v from (not x.lost) { x.ok }",
        kind: Kind::AlwaysPossible,
        over: false,
        rejected: "always Recover using v from (not x.lost) { x.ok }",
        anchor: "Recover",
        token: "Recover",
        expected: "expected `possible`",
    },
    Row {
        accepted: "unique path One using v over (j: M::Job) from (not j.a) to (j.done);",
        kind: Kind::UniquePath,
        over: true,
        rejected: "unique One using v over (j: M::Job) from (not j.a) to (j.done);",
        anchor: "One",
        token: "One",
        expected: "expected `path`",
    },
    Row {
        accepted:
            "possible Conjunction using v over (c: C::Cfg) { c.n = 1 and eventually c.n = 2 }",
        kind: Kind::Possible,
        over: true,
        rejected:
            "possible Conjunction using v over (c: C::Cfg) { if eventually c.a then c.b else c.c }",
        anchor: "eventually",
        token: "eventually",
        expected: "`eventually` is a reserved word",
    },
    Row {
        accepted: "possible Negated using v { not eventually[0,5] c.n = 2 }",
        kind: Kind::Possible,
        over: false,
        rejected: "possible Negated using v { not eventually[0,5) c.n = 2 }",
        anchor: ") c.n",
        token: ")",
        expected: "expected `]`",
    },
    Row {
        accepted: "possible Grouped using v { (eventually c.n = 2) }",
        kind: Kind::Possible,
        over: false,
        rejected: "possible Grouped using v { (eventually c.n = 2 }",
        anchor: "}",
        token: "}",
        expected: "expected `)`",
    },
    Row {
        accepted: "possible Until using v { c.a until c.b }",
        kind: Kind::Possible,
        over: false,
        rejected: "possible Until using v { c.a until c.b until c.c }",
        anchor: "until c.c",
        token: "until",
        expected: "expected `}`",
    },
    Row {
        accepted: "always possible FromTemporal using v from (always c.ok) { c.done }",
        kind: Kind::AlwaysPossible,
        over: false,
        rejected:
            "always possible FromTemporal using v from (fair weak M::Cfg::step; c.ok) { c.done }",
        anchor: "fair",
        token: "fair",
        expected: "`fair` is a reserved word",
    },
    Row {
        accepted: "always possible Fair using v { fair weak M::Cfg::step; c.done }",
        kind: Kind::AlwaysPossible,
        over: false,
        rejected: "always possible Fair using v { c.done; fair weak M::Cfg::step }",
        anchor: "; fair",
        token: ";",
        expected: "expected `}`",
    },
    Row {
        accepted: "possible OrdinaryNot using v over (c: C::Cfg) { not c.n = 2 }",
        kind: Kind::Possible,
        over: true,
        rejected: "possible OrdinaryNot using v over (c: C::Cfg) { c.n = eventually c.n }",
        anchor: "eventually",
        token: "eventually",
        expected: "`eventually` is a reserved word",
    },
    Row {
        accepted: "possible Unbounded using v over (c: C::Cfg) { eventually[1,*] c.done }",
        kind: Kind::Possible,
        over: true,
        rejected: "possible Unbounded using v over (c: C::Cfg) { eventually[1,*,] c.done }",
        anchor: ",]",
        token: ",",
        expected: "expected `]`",
    },
    Row {
        accepted: "always possible NoFrom using v { x.done }",
        kind: Kind::AlwaysPossible,
        over: false,
        rejected: "always possible NoFrom v { x.done }",
        anchor: "v {",
        token: "v",
        expected: "expected `using`",
    },
    Row {
        accepted: "unique path NoOver using v from (c.a) to (c.done);",
        kind: Kind::UniquePath,
        over: false,
        rejected: "unique path NoOver using v from (c.a) to (c.done) { c.done }",
        anchor: "{ c.done",
        token: "{",
        expected: "expected `;`",
    },
];

fn accepted(index: usize) -> (String, ParsedSource) {
    let row = &ROWS[index];
    let text = claim_syntax::unit(row.accepted);
    let parsed = claim_syntax::admitted(&text);
    let claims = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::StateGraphClaim)
        .collect::<Vec<_>>();
    assert_eq!(claims.len(), 1, "{}", row.accepted);
    let claim = claims[0];
    assert_eq!(&text[claim.span().start..claim.span().end], row.accepted);
    let tokens = claim
        .children()
        .iter()
        .filter_map(|element| match element {
            CstElement::Token(index) => Some(&parsed.cst().tokens()[*index]),
            CstElement::Node(_) => None,
        })
        .filter(|token| token.class() == TokenClass::Token)
        .map(|token| std::str::from_utf8(token.spelling()).expect("authored UTF8 token"))
        .collect::<Vec<_>>();
    let kind = match tokens.as_slice() {
        ["possible", ..] => Kind::Possible,
        ["always", "possible", ..] => Kind::AlwaysPossible,
        ["unique", "path", ..] => Kind::UniquePath,
        _ => panic!("wrong claim prefix: {tokens:?}"),
    };
    assert_eq!(kind, row.kind);
    assert_eq!(tokens.contains(&"over"), row.over);
    assert_eq!(
        claim_syntax::node_texts(&text, &parsed, Production::StateGraphBody).len(),
        usize::from(kind != Kind::UniquePath)
    );
    (text, parsed)
}

fn rejected(index: usize) {
    let row = &ROWS[index];
    claim_syntax::refused(row.rejected, row.anchor, row.token, row.expected);
}

mod each_row_of_the_distinguishing_table_has_an_accepting_form {
    use super::*;
    #[trace("FR-165-AC-4")]
    #[test]
    fn possible_over() {
        super::accepted(0);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn always_from() {
        super::accepted(1);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn unique_over() {
        super::accepted(2);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn temporal_conjunction() {
        super::accepted(3);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn negated_interval() {
        super::accepted(4);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn temporal_group() {
        super::accepted(5);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn binary_until() {
        super::accepted(6);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn temporal_from() {
        super::accepted(7);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn leading_fairness() {
        super::accepted(8);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn ordinary_not_expression() {
        let (text, parsed) = super::accepted(9);
        assert_eq!(
            claim_syntax::node_texts(&text, &parsed, Production::Comparison),
            ["not c.n = 2"]
        );
        assert!(claim_syntax::node_texts(&text, &parsed, Production::Unary).contains(&"not c.n"));
        assert!(claim_syntax::node_texts(&text, &parsed, Production::StateGraphUnary).is_empty());
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn unbounded_interval() {
        super::accepted(10);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn always_no_from() {
        super::accepted(11);
    }
    #[trace("FR-165-AC-4")]
    #[test]
    fn unique_no_over() {
        super::accepted(12);
    }
}

mod each_row_has_a_near_miss_refused_at_its_token {
    use super::*;
    #[trace("FR-002-AC-6")]
    #[test]
    fn possible_requires_body_after_optional_over() {
        super::rejected(0);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn always_requires_possible() {
        super::rejected(1);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn unique_requires_path() {
        super::rejected(2);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn temporal_if_condition() {
        super::rejected(3);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn interval_closing_bracket() {
        super::rejected(4);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn group_closing_parenthesis() {
        super::rejected(5);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn second_unparenthesized_until() {
        super::rejected(6);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn fairness_in_from() {
        super::rejected(7);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn fairness_after_predicate() {
        super::rejected(8);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn temporal_comparison_operand() {
        super::rejected(9);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn interval_extra_separator() {
        super::rejected(10);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn always_requires_using() {
        super::rejected(11);
    }
    #[trace("FR-002-AC-6")]
    #[test]
    fn unique_requires_semicolon() {
        super::rejected(12);
    }

    #[trace("FR-165-AC-6")]
    #[test]
    fn unique_path_has_no_body_for_a_following_fairness_constraint() {
        let good = "unique path One using v from (c.a) to (c.d);";
        let text = claim_syntax::unit(good);
        let parsed = claim_syntax::admitted(&text);
        assert_eq!(
            claim_syntax::node_texts(&text, &parsed, Production::StateGraphClaim),
            [good]
        );
        let bad = format!("{good} fair weak M::Cfg::step");
        claim_syntax::refused_at(&bad, "fair", "fair");
    }
}
