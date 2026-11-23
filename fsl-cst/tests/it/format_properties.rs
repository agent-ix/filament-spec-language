// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2058: the formatter is a fixed point, keeps the significant token stream
//! and the comments, and does not change the production tree, on units derived
//! from the parser's own grammar table.
//!
//! The units are the grammar-table derivations (one bounded complete unit per
//! production, from `fsl_cst::grammar::generate`) with their layout rewritten at
//! random: every gap between significant tokens is replaced by a random choice of
//! whitespace shapes and line comments, and one unit in eight loses a random token
//! (a near miss the parser may refuse; those cases are skipped and counted).
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

use fsl_cst::cst::{CstElement, CstNode, LosslessCst, TokenClass};
use fsl_cst::grammar::generate::{covering_derivations, Derivation};
use fsl_cst::{Limits, ParsedSource, Production};
use fsl_foundation::source::SourceIdentity;
use fsl_proptest_policy as prop;
use proptest::prelude::*;

const GAPS: &[&str] = &[
    " ", "\n", "\t", "   \n   ", "\r\n", " // c\n", "\n// c\n", "  \n\n  ",
];

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:format-properties".into(),
    }
}

fn parse(source: &str) -> Option<ParsedSource> {
    fsl_cst::parse(
        identity(),
        "unit.native",
        source.as_bytes(),
        Limits::default(),
    )
    .ok()
}

/// One derived unit with a random layout and an optional dropped token.
#[derive(Clone, Debug)]
struct Unit {
    /// Index into the derivation table.
    derivation: usize,
    /// Gap choices; a missing entry means a single space.
    gaps: Vec<u8>,
    /// A token to delete (modulo the token count), for near-miss units.
    drop: Option<u16>,
}

fn units(count: usize) -> BoxedStrategy<Unit> {
    (
        0..count,
        proptest::collection::vec(0u8..GAPS.len() as u8, 0..400),
        prop_oneof![7 => Just(None), 1 => (0u16..2000).prop_map(Some)],
    )
        .prop_map(|(derivation, gaps, drop)| Unit {
            derivation,
            gaps,
            drop,
        })
        .boxed()
}

/// Whether each significant token is glued to the next one in the source (hex digits).
fn glued(parsed: &ParsedSource) -> Vec<bool> {
    let tokens: Vec<_> = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Token)
        .collect();
    tokens
        .windows(2)
        .map(|pair| pair[0].span().end == pair[1].span().start)
        .chain([false])
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

/// Render a derivation with its layout replaced. `None` when the derivation does not lex.
fn render(derivations: &[Derivation], unit: &Unit) -> Option<String> {
    let base = parse(&derivations[unit.derivation].source)?;
    let mut tokens = significant(&base);
    let mut glue = glued(&base);
    if let Some(drop) = unit.drop {
        if tokens.len() > 1 {
            let at = usize::from(drop) % tokens.len();
            tokens.remove(at);
            glue.remove(at);
        }
    }
    let mut out = String::new();
    for (index, token) in tokens.iter().enumerate() {
        out.push_str(std::str::from_utf8(token).ok()?);
        if glue[index] {
            continue;
        }
        let gap = usize::from(unit.gaps.get(index).copied().unwrap_or(0));
        let gap = GAPS[gap];
        out.push_str(if index + 1 == tokens.len() && !gap.ends_with('\n') {
            "\n"
        } else {
            gap
        });
    }
    Some(out)
}

#[derive(Debug, PartialEq, Eq)]
enum Tree {
    Node(Production, Vec<Tree>),
    Token(Vec<u8>),
}

fn tree(cst: &LosslessCst, node: &CstNode) -> Tree {
    Tree::Node(
        node.production(),
        node.children()
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(index) => Some(tree(cst, &cst.nodes()[*index])),
                CstElement::Token(index) => {
                    let token = &cst.tokens()[*index];
                    (token.class() == TokenClass::Token)
                        .then(|| Tree::Token(token.spelling().to_vec()))
                }
            })
            .collect(),
    )
}

fn comments(parsed: &ParsedSource) -> Vec<Vec<u8>> {
    parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == TokenClass::Comment)
        .map(|token| token.spelling().trim_ascii_end().to_vec())
        .collect()
}

fn text(tokens: &[Vec<u8>]) -> String {
    tokens
        .iter()
        .map(|token| String::from_utf8_lossy(token).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which of the four formatter properties to check.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Law {
    Idempotent,
    Tokens,
    Comments,
    Tree,
}

/// The formatter under test: the real one, or a stub standing in for a defective one.
type Formatter = fn(&ParsedSource) -> Option<String>;

fn real_format(parsed: &ParsedSource) -> Option<String> {
    fsl_cst::format::format(parsed).ok()
}

/// A formatter that drops the first `;`.
fn dropping_format(parsed: &ParsedSource) -> Option<String> {
    real_format(parsed).map(|formatted| formatted.replacen(';', "", 1))
}

enum Verdict {
    Skipped,
    Held,
    Broken(String),
}

fn check(source: &str, law: Law, format: Formatter) -> Verdict {
    let Some(parsed) = parse(source) else {
        return Verdict::Skipped;
    };
    if !parsed.is_admissible() {
        return Verdict::Skipped;
    }
    let Some(once) = format(&parsed) else {
        return Verdict::Broken(format!("formatter refused an admissible unit:\n{source}"));
    };
    let Some(reparsed) = parse(&once) else {
        return Verdict::Broken(format!("formatted unit does not read:\n{once}"));
    };
    let broken = |what: &str, detail: String| {
        Verdict::Broken(format!(
            "{what}\n{detail}\n--- unit ---\n{source}\n--- format(unit) ---\n{once}"
        ))
    };
    match law {
        Law::Idempotent => {
            let Some(twice) = format(&reparsed) else {
                return broken("second format refused", String::new());
            };
            if twice == once {
                Verdict::Held
            } else {
                let at = once
                    .bytes()
                    .zip(twice.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(once.len().min(twice.len()));
                broken(
                    "format is not a fixed point",
                    format!("first difference at byte {at}"),
                )
            }
        }
        Law::Tokens => {
            let (before, after) = (significant(&parsed), significant(&reparsed));
            if before == after {
                Verdict::Held
            } else {
                let at = before
                    .iter()
                    .zip(&after)
                    .position(|(a, b)| a != b)
                    .unwrap_or(before.len().min(after.len()));
                broken(
                    "significant tokens changed",
                    format!(
                        "first differing token index {at}: {:?} vs {:?}\nbefore: {}\nafter: {}",
                        before
                            .get(at)
                            .map(|t| String::from_utf8_lossy(t).into_owned()),
                        after
                            .get(at)
                            .map(|t| String::from_utf8_lossy(t).into_owned()),
                        text(&before),
                        text(&after)
                    ),
                )
            }
        }
        Law::Comments => {
            let (before, after) = (comments(&parsed), comments(&reparsed));
            if before == after {
                Verdict::Held
            } else {
                broken(
                    "comments changed",
                    format!("{:?} vs {:?}", text(&before), text(&after)),
                )
            }
        }
        Law::Tree => {
            let (before, after) = (
                tree(parsed.cst(), parsed.cst().root()),
                tree(reparsed.cst(), reparsed.cst().root()),
            );
            if before == after {
                Verdict::Held
            } else {
                broken("production tree changed", String::new())
            }
        }
    }
}

fn run_law(suite: &'static str, law: Law) {
    let derivations = covering_derivations();
    let skipped = AtomicUsize::new(0);
    let cases = AtomicUsize::new(0);
    prop::run(
        suite,
        prop::config(256),
        &units(derivations.len()),
        |unit| {
            cases.fetch_add(1, Ordering::Relaxed);
            let Some(source) = render(&derivations, &unit) else {
                skipped.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            };
            match check(&source, law, real_format) {
                Verdict::Skipped => {
                    skipped.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                Verdict::Held => Ok(()),
                Verdict::Broken(why) => Err(TestCaseError::fail(why)),
            }
        },
    )
    .unwrap();
    let (skipped, cases) = (skipped.into_inner(), cases.into_inner());
    eprintln!("{suite}: {skipped} of {cases} cases skipped as refused near misses");
    assert!(
        skipped * 2 < cases,
        "{skipped} of {cases} cases skipped: the generator mostly produces refused units"
    );
}

/// Provenance: QSL-2058 P1(a).
#[test]
fn format_is_idempotent_on_generated_units() {
    run_law("format_is_idempotent_on_generated_units", Law::Idempotent);
}

/// Provenance: QSL-2058 P1(b).
#[test]
fn format_keeps_the_significant_token_stream() {
    run_law("format_keeps_the_significant_token_stream", Law::Tokens);
}

/// Provenance: QSL-2058 P1(c).
#[test]
fn format_keeps_comments_in_order() {
    run_law("format_keeps_comments_in_order", Law::Comments);
}

/// Provenance: QSL-2058 P1(d).
#[test]
fn reparse_of_formatted_unit_has_the_same_production_tree() {
    run_law(
        "reparse_of_formatted_unit_has_the_same_production_tree",
        Law::Tree,
    );
}

/// The production families QSL-2058 names, as `Production` variants.
const FAMILIES: &[Production] = &[
    Production::DimensionEquivalence,
    Production::UnitDeclaration,
    Production::EnumDeclaration,
    Production::RecordDeclaration,
    Production::TupleDeclaration,
    Production::DelayDeclaration,
    Production::HybridDeclaration,
    Production::RelationClause,
    Production::SynthesisDeclaration,
    Production::VerificationPlan,
    Production::Compensation,
    Production::Repetition,
    Production::AwaitControl,
    Production::EventNode,
    Production::TimeDeclaration,
    Production::TimeInvariant,
    Production::UrgentDeclaration,
    Production::StateGraphClaim,
];

/// Provenance: QSL-2058. Every production the parser can build is reached by a
/// generated unit, the named families included, and the four laws hold on every
/// one of those units (an exhaustive sweep, so coverage is not left to sampling).
#[test]
fn generated_units_reach_every_production_family() {
    let derivations = covering_derivations();
    let mut reached = BTreeSet::new();
    for (index, derivation) in derivations.iter().enumerate() {
        let unit = Unit {
            derivation: index,
            gaps: (0..400).map(|i| (i * 7 % GAPS.len()) as u8).collect(),
            drop: None,
        };
        let source = render(&derivations, &unit).expect("derivation lexes");
        let parsed = parse(&source).expect("reads");
        assert!(
            parsed.is_admissible(),
            "{:?} is not admissible: {:?}\n{source}",
            derivation.target,
            parsed.diagnostics()
        );
        reached.extend(parsed.cst().nodes().iter().map(|node| node.production()));
        for law in [Law::Idempotent, Law::Tokens, Law::Comments, Law::Tree] {
            if let Verdict::Broken(why) = check(&source, law, real_format) {
                panic!("{:?}: {why}", derivation.target);
            }
        }
    }
    let missing: Vec<_> = Production::all()
        .iter()
        // ExtensionDeclaration exists only when the unit is parsed with extension
        // forms; the base grammar the derivations walk does not contain it.
        .filter(|production| **production != Production::ExtensionDeclaration)
        .filter(|production| !reached.contains(production))
        .collect();
    assert!(
        missing.is_empty(),
        "productions never generated: {missing:?}"
    );
    let families: Vec<_> = FAMILIES.iter().filter(|p| !reached.contains(p)).collect();
    assert!(
        families.is_empty(),
        "families never generated: {families:?}"
    );
}

/// Provenance: QSL-2058 negative. A formatter that drops one token fails the
/// token property, and the failure shrinks to the plain layout of one derivation.
#[test]
fn dropped_token_is_caught() {
    let derivations = covering_derivations();
    let mut config = prop::config(256);
    config.failure_persistence = None;
    let outcome = prop::run(
        "dropped_token_is_caught",
        config,
        &units(derivations.len()),
        |unit| {
            let Some(source) = render(&derivations, &unit) else {
                return Ok(());
            };
            match check(&source, Law::Tokens, dropping_format) {
                Verdict::Broken(why) => Err(TestCaseError::fail(why)),
                _ => Ok(()),
            }
        },
    );
    match outcome {
        Err(proptest::test_runner::TestError::Fail(_, minimal)) => {
            assert!(
                minimal.drop.is_none() && minimal.gaps.iter().all(|gap| *gap == 0),
                "layout did not shrink: {minimal:?}"
            );
        }
        other => panic!("the dropped token went unnoticed: {other:?}"),
    }
}
