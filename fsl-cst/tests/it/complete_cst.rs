// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lossless complete-V1 CST construction and recovery -- the layer-1-only
//! subset of the root crate's former `complete_cst.rs`:
//! these tests call only `fsl_cst::parse` and fsl-cst types, never the root
//! crate's `complete::{apply_edit, apply_edits}` incremental-editing API,
//! which stays covered by the root's own `tests/it/complete_cst.rs`.
use std::collections::BTreeSet;

use fsl_cst::{CompleteCause, CompleteCode, Limits, Production, TokenClass};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const SOURCE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\r\n",
    "// exact trivia remains authored\r\n",
    "profile Complete = \"filament.value.complete/v1\";\r\n",
    "import \"acme/base\" as Base;\r\n",
    "model M = \"acme/model\";\r\n",
    "dimension Length = M::Length;\r\n",
    "unit meter: M::Length = rational(1, 1) * M::meter;\r\n",
    "ordered enum Color { Red = \"red\", Blue, }\r\n",
    "record Reading { amount: Decimal[-2, 4; 18, 4; nearest-even]; label: Option<Text[0, 32; nfc]>?; }\r\n",
    "tuple Pair(Integer, Reference<M::Thing>);\r\n",
    "type Readings = Sequence<Reading>[0, 100];\r\n",
    "function choose using Complete (x: Integer): Integer pure { if true then x else 0 }\r\n",
    "predicate Good using Complete (x: Integer): Boolean { choose(x) >= 0 }\r\n",
    "relation Same using Complete over (left: M::Execution, right: M::Execution) { true }\r\n",
    "hyper Secret using Complete over traces in M::Execution bounded 2 forall trace a exists trace b { true }\r\n",
    "hybrid Plant using Complete { initial { mode Idle when { true }; } mode Idle invariant { true } flow { M::x' = 0; } transition to Idle when { true } reset { true }; }\r\n",
    "synthesis Find using Complete grammar M::Grammar domain M::Domain satisfies { true };\r\n",
    "verify Portfolio using Complete { check Safety claim M::Claim method M::Method domain M::Domain depends [Earlier] bound 10; }",
);

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:complete-cst".into(),
    }
}

#[trace(
    "FSpec-TC-180",
    "FSpec-TC-222",
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-302-AC-1"
)]
#[test]
fn complete_source_builds_a_byte_exact_trivia_preserving_cst() {
    let parsed = fsl_cst::parse(
        identity(),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 and resource bounds");

    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    assert!(parsed.is_admissible());
    assert_eq!(parsed.cst().render(), SOURCE.as_bytes());
    assert_eq!(parsed.source().text(), SOURCE);
    // Distinct occurrences never collapse to one identity: nodes that share
    // one exact span (an `Expression` wrapping an `Implication` wrapping ...)
    // each resolve to themselves alone.
    let nodes = parsed.cst().nodes();
    let shared_span = nodes
        .iter()
        .enumerate()
        .find_map(|(index, node)| {
            nodes[index + 1..]
                .iter()
                .find(|other| other.span() == node.span())
                .map(|other| (node, other))
        })
        .expect("a production chain shares one span");
    assert_ne!(shared_span.0.identity(), shared_span.1.identity());
    for node in nodes {
        let resolved = parsed.cst().resolve(node.identity()).expect("own identity");
        assert_eq!(resolved.production(), node.production());
        assert_eq!(resolved.identity(), node.identity());
    }
    assert!(parsed
        .cst()
        .tokens()
        .iter()
        .any(|token| token.class() == TokenClass::Comment));
    assert!(parsed
        .cst()
        .tokens()
        .iter()
        .any(|token| token.class() == TokenClass::Whitespace));

    let productions: BTreeSet<_> = parsed
        .cst()
        .nodes()
        .iter()
        .map(|node| node.production())
        .collect();
    for expected in [
        Production::CompleteUnit,
        Production::ImportDeclaration,
        Production::DimensionDeclaration,
        Production::UnitDeclaration,
        Production::EnumDeclaration,
        Production::RecordDeclaration,
        Production::TupleDeclaration,
        Production::AliasDeclaration,
        Production::FunctionDeclaration,
        Production::RelationClause,
        Production::HyperClause,
        Production::HybridDeclaration,
        Production::HybridInitial,
        Production::InitialEntry,
        Production::SynthesisDeclaration,
        Production::VerificationPlan,
    ] {
        assert!(productions.contains(&expected), "missing {expected:?}");
    }
}

#[trace("FSpec-TC-222", "FSpec-FR-302-AC-1")]
#[test]
fn exact_syntax_node_limit_admits_the_boundary_and_refuses_one_less() {
    let baseline = fsl_cst::parse(
        identity(),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let node_count = baseline.cst().nodes().len();
    let exact = Limits {
        nodes: node_count,
        ..Limits::default()
    };
    assert!(
        fsl_cst::parse(identity(), "complete.native", SOURCE.as_bytes(), exact,)
            .unwrap()
            .is_admissible()
    );
    let below = Limits {
        nodes: node_count - 1,
        ..Limits::default()
    };
    assert_eq!(
        fsl_cst::parse(identity(), "complete.native", SOURCE.as_bytes(), below,)
            .unwrap_err()
            .code,
        // The node ceiling is a `SyntaxLimit` kind the catalog
        // admits, so it now reports `stage_limit_exceeded`.
        CompleteCode::StageLimitExceeded
    );
}

#[trace("FSpec-TC-222", "FSpec-FR-302-AC-1")]
#[test]
fn token_limit_charges_every_retained_cst_leaf_at_the_exact_boundary() {
    let baseline = fsl_cst::parse(
        identity(),
        "complete.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let leaf_count = baseline.cst().tokens().len();
    let exact = Limits {
        tokens: leaf_count,
        ..Limits::default()
    };
    let accepted = fsl_cst::parse(identity(), "complete.native", SOURCE.as_bytes(), exact).unwrap();
    assert_eq!(accepted.cst().tokens().len(), leaf_count);
    assert!(accepted.is_admissible());

    let first_excess = baseline.cst().tokens()[leaf_count - 1].span();
    let below = Limits {
        tokens: leaf_count - 1,
        ..Limits::default()
    };
    let refusal =
        fsl_cst::parse(identity(), "complete.native", SOURCE.as_bytes(), below).unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
    assert_eq!(refusal.byte_span().unwrap().start, first_excess.start);
    assert_eq!(refusal.byte_span().unwrap().end, first_excess.end);

    for (revision, source) in [
        ("whitespace-leaf", " "),
        ("comment-leaf", "// retained"),
        ("invalid-leaf", "@"),
    ] {
        let refusal = fsl_cst::parse(
            identity(),
            "complete.native",
            source.as_bytes(),
            Limits {
                tokens: 0,
                ..Limits::default()
            },
        )
        .unwrap_err();
        assert_eq!(refusal.code, CompleteCode::StageLimitExceeded, "{revision}");
        assert_eq!(refusal.byte_span().unwrap().start, 0, "{revision}");
        assert_eq!(refusal.byte_span().unwrap().end, source.len(), "{revision}");
    }
}

#[trace("FSpec-TC-222", "FSpec-FR-302-AC-1")]
#[test]
fn large_single_lexeme_refuses_at_the_first_excess_leaf() {
    let source = format!("0x{}", "f".repeat(900_000));
    let refusal = fsl_cst::parse(
        identity(),
        "large.native",
        source.as_bytes(),
        Limits {
            tokens: 8,
            ..Limits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::StageLimitExceeded);
    assert_eq!(refusal.byte_span().unwrap().start, 9);
    assert_eq!(refusal.byte_span().unwrap().end, 10);
}

#[trace("FSpec-TC-222", "FSpec-FR-302-AC-2")]
#[test]
fn invalid_source_retains_bytes_and_exposes_recovery_without_admission() {
    let source = "language \"ix:native\" edition \"1-draft\"; profile C = \"filament.value.complete/v1\"; record Broken { value: Integer }";
    let parsed = fsl_cst::parse(
        identity(),
        "broken.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("recoverable syntax is still a parsed source artifact");

    assert_eq!(parsed.cst().render(), source.as_bytes());
    assert!(!parsed.is_admissible());
    assert_eq!(
        (parsed.diagnostics()[0].code, parsed.diagnostics()[0].cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert!(!parsed.cst().recoveries().is_empty());
}

#[trace("FSpec-TC-222", "FSpec-FR-302-AC-3")]
#[test]
fn rendering_an_exchanged_clone_from_the_same_cst_succeeds() {
    let parsed = fsl_cst::parse(
        identity(),
        "clone.native",
        SOURCE.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let exchanged = parsed.cst().root().clone();
    assert_eq!(
        parsed.cst().render_node(&exchanged).unwrap(),
        SOURCE.as_bytes()
    );
}

/// TC-188 R04 (FR-143-AC-10): a sum declaration is a layer-1 parse refusal
/// at `variant`. Moved here from the root crate's `composite_values.rs`:
/// it calls only `fsl_cst::parse`.
#[trace("FSpec-TC-188", "FSpec-FR-143-AC-10")]
#[test]
fn r04_a_sum_declaration_is_invalid_syntax_at_variant() {
    let text = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"filament.value.complete/v1\";\nvariant V { A, B }";
    let parsed = fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:tc-188".into(),
        },
        "tc-188.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert!(!parsed.is_admissible());
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(
        diagnostic.byte_span().unwrap().start,
        text.find("variant").unwrap()
    );
}

/// FR-762: the header shared by the multi-error recovery units.
const RECOVERY_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n";

fn recover(body: &str) -> (String, fsl_cst::ParsedSource) {
    let text = format!("{RECOVERY_HEADER}{body}");
    let parsed = fsl_cst::parse(
        identity(),
        "recovery.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 within the S1 limits");
    (text, parsed)
}

fn byte_at(text: &str, needle: &str) -> usize {
    text.find(needle).expect("the needle is in the unit")
}

const THREE_RECORDS: &str = concat!(
    "record A { x: Integer y: Integer; }\n",
    "record B { u: Integer; v: Integer; }\n",
    "record C { z: ; }\n",
);

#[trace("TC-2148", "FR-762-AC-1")]
#[test]
#[ignore = "QSL-2195: declaration-level recovery after a syntax error not implemented yet"]
fn two_independent_syntax_errors_give_two_diagnostics() {
    let (text, parsed) = recover(THREE_RECORDS);
    assert!(!parsed.is_admissible());
    let found: Vec<_> = parsed
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.cause,
                diagnostic.byte_span().expect("located").start,
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                CompleteCode::InvalidSyntax,
                CompleteCause::UnexpectedToken,
                byte_at(&text, "y: Integer;")
            ),
            (
                CompleteCode::InvalidSyntax,
                CompleteCause::UnexpectedToken,
                text.rfind("; }").expect("the third record")
            ),
        ]
    );
}

#[trace("TC-2149", "FR-762-AC-2")]
#[test]
#[ignore = "QSL-2195: the lexer continuing after an invalid token not implemented yet"]
fn two_invalid_tokens_give_two_invalid_token_diagnostics() {
    let (text, parsed) =
        recover("record A { x: $; }\nrecord B { y: Integer; }\nrecord C { z: $; }\n");
    let first = byte_at(&text, "$");
    let second = text.rfind('$').expect("two dollars");
    let found: Vec<_> = parsed
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.cause,
                diagnostic.byte_span().expect("located").start,
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (CompleteCause::InvalidToken, first),
            (CompleteCause::InvalidToken, second)
        ]
    );
}

#[trace("TC-2150", "FR-762-AC-3")]
#[test]
#[ignore = "QSL-2195: declaration-level recovery after a syntax error not implemented yet"]
fn a_valid_declaration_between_broken_ones_is_recovered() {
    let (text, parsed) = recover(THREE_RECORDS);
    let second = "record B { u: Integer; v: Integer; }";
    let start = byte_at(&text, second);
    assert!(
        parsed.cst().nodes().iter().any(|node| {
            node.production() == Production::Declaration
                && node.span().start == start
                && node.span().end == start + second.len()
        }),
        "the second record is a recovered declaration"
    );
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

#[trace("TC-2151", "FR-762-AC-4")]
#[test]
#[ignore = "QSL-2195: declaration-level recovery after a syntax error not implemented yet"]
fn a_missing_semicolon_gives_one_diagnostic() {
    let (text, parsed) = recover("record A { x: Integer }\nrecord B { y: Integer; }\n");
    assert_eq!(parsed.diagnostics().len(), 1, "{:?}", parsed.diagnostics());
    let start = byte_at(&text, "record B");
    assert!(parsed
        .cst()
        .nodes()
        .iter()
        .any(|node| node.production() == Production::Declaration && node.span().start == start));
}

#[trace("TC-2152", "FR-762-AC-5")]
#[test]
fn a_single_error_keeps_its_one_diagnostic() {
    for (expression, second) in [("a < b < c", 6), ("a = b != c", 6), ("a < b + c = d", 10)] {
        let body = format!(
            "profile Complete = \"filament.value.complete/v1\";\n\
             function f using Complete(a: Integer, b: Integer, c: Integer, d: Integer): Boolean pure {{ {expression} }}\n"
        );
        let (text, parsed) = recover(&body);
        let at = byte_at(&text, expression) + second;
        let found: Vec<_> = parsed
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    diagnostic.byte_span().map(|span| span.start),
                )
            })
            .collect();
        assert_eq!(
            found,
            [(CompleteCode::InvalidSyntax, Some(at))],
            "{expression}"
        );
    }
}

#[trace("TC-2153", "FR-762-AC-6")]
#[trace("FSpec-TC-1649", "FSpec-FR-272-AC-30")]
#[test]
#[ignore = "QSL-2195: the s1.diagnostics bound and the diagnostic-list-cut diagnostic not implemented yet"]
fn the_diagnostic_list_is_cut_at_its_bound() {
    let body: String = (0..150)
        .map(|n| format!("record R{n} {{ z: ; }}\n"))
        .collect();
    let (text, parsed) = recover(&body);
    let diagnostics = parsed.diagnostics();
    assert_eq!(diagnostics.len(), 101);
    assert!(diagnostics[..100]
        .iter()
        .all(|diagnostic| diagnostic.cause == CompleteCause::UnexpectedToken));
    let cut = &diagnostics[100];
    assert_eq!(cut.code, CompleteCode::InvalidSyntax);
    assert_eq!(cut.cause.as_str(), "diagnostic-list-cut");
    let record = byte_at(&text, "record R100 ");
    assert_eq!(
        cut.byte_span().expect("located").start,
        record + text[record..].find("; }").expect("the offending token")
    );
    assert!(cut.message.contains("s1.diagnostics") && cut.message.contains("100"));
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.limit().is_none()));
}

#[trace("TC-2154", "FR-762-AC-7")]
#[test]
#[ignore = "QSL-2195: declaration-level recovery after a syntax error not implemented yet"]
fn a_recovered_unit_is_never_admitted() {
    let (_, parsed) = recover(THREE_RECORDS);
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics().len(), 2);
}

/// Parse `body` after the recovery header under `limits`.
fn recover_under(body: &str, limits: Limits) -> Result<fsl_cst::ParsedSource, String> {
    let text = format!("{RECOVERY_HEADER}{body}");
    fsl_cst::parse(identity(), "recovery.native", text.as_bytes(), limits)
        .map_err(|diagnostic| diagnostic.to_string())
}

#[trace("TC-2187", "FR-762-AC-8")]
#[trace("FSpec-TC-1649", "FSpec-FR-272-AC-30")]
#[test]
#[ignore = "QSL-2195: the work-budget cut of the recovery report not implemented yet"]
fn recovery_stops_at_the_work_budget() {
    let records_first_error = |work: usize| {
        recover_under(THREE_RECORDS, Limits::default().with_work_units(work)).is_ok_and(|parsed| {
            parsed
                .diagnostics()
                .first()
                .is_some_and(|diagnostic| diagnostic.cause == CompleteCause::UnexpectedToken)
        })
    };
    let (mut low, mut high) = (0_usize, Limits::default().work_units);
    assert!(records_first_error(high));
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if records_first_error(middle) {
            high = middle;
        } else {
            low = middle;
        }
    }
    let parsed = recover_under(THREE_RECORDS, Limits::default().with_work_units(high))
        .expect("the first error is recorded");
    assert!(!parsed.is_admissible());
    let last = parsed.diagnostics().last().expect("diagnostics");
    assert_eq!(last.cause.as_str(), "diagnostic-list-cut");
    assert!(last.message.contains("s1.work_units"), "{}", last.message);
    assert!(parsed
        .diagnostics()
        .iter()
        .all(|diagnostic| diagnostic.limit().is_none()));
}

#[trace("TC-2187", "FR-762-AC-9")]
#[test]
#[ignore = "QSL-2195: header recovery not implemented yet"]
fn header_recovery_resumes_at_the_next_declaration() {
    let body = "profile v = ;\nrecord A { x: Integer y: Integer; }\n";
    let (text, parsed) = recover(body);
    let starts: Vec<usize> = parsed
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.byte_span().expect("located").start)
        .collect();
    assert_eq!(
        starts,
        [byte_at(&text, "= ;") + 2, byte_at(&text, "y: Integer;")]
    );
    let start = byte_at(&text, "record A");
    assert!(parsed
        .cst()
        .nodes()
        .iter()
        .any(|node| node.production() == Production::Declaration && node.span().start == start));
}

#[trace("TC-2187", "FR-762-AC-10")]
#[test]
#[ignore = "QSL-2195: declaration-level recovery after a syntax error not implemented yet"]
fn an_error_with_no_resumption_point_is_last() {
    let (text, parsed) = recover("record A { x: Integer; }\nrecord B { z: ; }\n");
    let diagnostics = parsed.diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let last = diagnostics.last().expect("one diagnostic");
    assert_eq!(
        last.byte_span().expect("located").start,
        text.rfind("; }").expect("in the unit")
    );
    assert_ne!(last.cause.as_str(), "diagnostic-list-cut");
}
