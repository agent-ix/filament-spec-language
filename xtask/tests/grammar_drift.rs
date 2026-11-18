// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2107: compare the shared EBNF inventory with the parser inventory.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use fsl_cst::Production;
use ix_trace_rs::trace;

const SHARED_GRAMMAR: &str = "proposals/filament-v1/shared-grammar.md";

/// A written production whose parser counterpart has a different name.
const CORRESPONDENCE: &[(&str, &str)] = &[
    ("abstraction-binding", "ObjectBinding"),
    ("abstraction-decl", "AbstractionDeclaration"),
    ("alias-decl", "AliasDeclaration"),
    ("always-possible-clause", "StateGraphClaim"),
    ("attempt-type", "TypeName"),
    ("absence-mode", "Primary"),
    ("binder", "PopulationBinder"),
    ("bound-op", "Comparison"),
    ("complete-unit", "CompleteUnit"),
    ("composed-unit", "CompleteUnit"),
    ("component-clause", "CompositionComponent"),
    ("component-decl", "CompositionComponent"),
    ("composition-decl", "CompositionDeclaration"),
    ("composition-member", "CompositionGuarantee"),
    ("delay-decl", "DelayDeclaration"),
    ("delay-dist", "DelayDistribution"),
    ("dimension-decl", "DimensionDeclaration"),
    ("enum-decl", "EnumDeclaration"),
    ("expr", "Expression"),
    ("environment-premise", "GuaranteeEnvironment"),
    ("field-value", "FieldValue"),
    ("function", "FunctionDeclaration"),
    ("guarantee-decl", "GuaranteeDeclaration"),
    ("guarantee-member", "GuaranteeInitial"),
    ("hybrid-decl", "HybridDeclaration"),
    ("initial-binding", "GuaranteeInitial"),
    ("import-decl", "ImportDeclaration"),
    ("node-ref", "NodeReference"),
    ("population-binding", "PopulationBinding"),
    ("possible-clause", "PossibleClause"),
    ("policy", "SchedulerClause"),
    ("progress-premise", "GuaranteeProgress"),
    ("random-decl", "RandomDeclaration"),
    ("record-decl", "RecordDeclaration"),
    ("release-decl", "ReleaseDeclaration"),
    ("reward-decl", "RewardDeclaration"),
    ("rust-field", "FieldBinding"),
    ("rust-name", "ParameterBinding"),
    ("rust-path", "ObjectBinding"),
    ("rust-receiver", "FrameBinding"),
    ("signed-int", "SignedInteger"),
    ("signed-quantity", "SignedQuantity"),
    ("state-graph-clause", "StateGraphClaim"),
    ("synthesis-decl", "SynthesisDeclaration"),
    ("taskset-decl", "TasksetDeclaration"),
    ("temporal-expr", "TemporalExpression"),
    ("terminal-decl", "TerminalDeclaration"),
    ("time-decl", "TimeDeclaration"),
    ("tuple-decl", "TupleDeclaration"),
    ("type-ref", "TypeReference"),
    ("union-decl", "UnionDeclaration"),
    ("unit-decl", "UnitDeclaration"),
    ("unit-term", "UnitExpression"),
    ("quantity-unit", "UnitExpression"),
    ("urgent-decl", "UrgentDeclaration"),
    ("window", "PastWindow"),
    ("workload-decl", "WorkloadDeclaration"),
];

/// A parser production whose spelling identifies a subrule written inline
/// under a shared-grammar production.
const PARSER_SUBPRODUCTIONS: &[(&str, &str)] = &[
    ("CaseArm", "case-arm"),
    ("CaseBinder", "case-arm"),
    ("CaseExpression", "expr"),
    ("ScrutineeExpression", "expr"),
    ("ScrutineeImplication", "implication"),
    ("ScrutineeDisjunction", "disjunction"),
    ("ScrutineeConjunction", "conjunction"),
    ("ScrutineeComparison", "comparison"),
    ("ScrutineeSum", "sum"),
    ("ScrutineeProduct", "product"),
    ("ScrutineeUnary", "unary"),
    ("ScrutineePostfix", "postfix"),
    ("ScrutineePrimary", "primary"),
    ("HyperExpression", "hyper-clause"),
    ("HyperImplication", "hyper-clause"),
    ("HyperDisjunction", "hyper-clause"),
    ("HyperConjunction", "hyper-clause"),
    ("HyperComparison", "hyper-clause"),
    ("HyperSum", "hyper-clause"),
    ("HyperProduct", "hyper-clause"),
    ("HyperUnary", "hyper-clause"),
    ("HyperPostfix", "hyper-clause"),
    ("HyperPrimary", "hyper-clause"),
    ("StateGraphBody", "graph-body"),
    ("StateGraphClaim", "possible-clause"),
    ("StateGraphConjunction", "graph-conjunction"),
    ("StateGraphDisjunction", "graph-disjunction"),
    ("StateGraphImplication", "graph-implication"),
    ("StateGraphPredicate", "graph-predicate"),
    ("StateGraphRelation", "graph-relation"),
    ("StateGraphUnary", "graph-unary"),
    ("PastWindow", "window"),
    ("UnitExpression", "quantity-unit"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Side {
    Standard,
    Parser,
}

#[derive(Clone, Copy, Debug)]
struct Divergence {
    side: Side,
    production: &'static str,
    register_row: &'static str,
    ticket: &'static str,
    reason: &'static str,
}

/// Known unmatched production names, each with the issue that owns their
/// reconciliation. The standard-only object member is inlined by the parser.
const DIVERGENCES: &[Divergence] = &[
    Divergence {
        side: Side::Parser,
        production: "PopulationBound",
        register_row: "G-249",
        ticket: "STD-694",
        reason: "the parser production awaits standard inventory reconciliation",
    },
    Divergence {
        side: Side::Parser,
        production: "QuantifiedClockBound",
        register_row: "G-250",
        ticket: "STD-694",
        reason: "the parser production awaits standard inventory reconciliation",
    },
    Divergence {
        side: Side::Parser,
        production: "DeliveryInterval",
        register_row: "G-251",
        ticket: "STD-694",
        reason: "the parser production awaits standard inventory reconciliation",
    },
    Divergence {
        side: Side::Parser,
        production: "ObjectDeclaration",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits an object declaration into dedicated productions",
    },
    Divergence {
        side: Side::Parser,
        production: "ObjectPopulation",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits object populations into dedicated productions",
    },
    Divergence {
        side: Side::Parser,
        production: "ObjectOperation",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits object operations into dedicated productions",
    },
    Divergence {
        side: Side::Parser,
        production: "FrameClause",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits frame clauses into dedicated productions",
    },
    Divergence {
        side: Side::Parser,
        production: "FieldTarget",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits field targets into dedicated productions",
    },
    Divergence {
        side: Side::Parser,
        production: "DeleteTarget",
        register_row: "G-257",
        ticket: "STD-698",
        reason: "the parser splits delete targets into dedicated productions",
    },
];

/// Read the one shared grammar document from an explicitly supplied FSpec
/// checkout. `None` is an error; it is never interpreted as a passing skip.
fn read_shared_grammar(fspec_dir: Option<&Path>) -> Result<String, String> {
    let root = fspec_dir.ok_or_else(|| "FSPEC_DIR is not set".to_owned())?;
    let path = root.join(SHARED_GRAMMAR);
    fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))
}

/// The production names declared by top-level `name =` or `name =/` lines
/// inside EBNF fences.
fn grammar_productions(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_ebnf = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_ebnf = line.trim() == "```ebnf";
            continue;
        }
        if !in_ebnf {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !line.starts_with(' ')
            && !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '-')
        {
            names.insert(name.to_owned());
        }
    }
    names
}

/// Convert an unlisted kebab-case name to its ordinary parser spelling.
fn pascal(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// The parser productions corresponding to one shared-grammar production.
fn parser_names_for(name: &str) -> Vec<String> {
    let mut names = CORRESPONDENCE
        .iter()
        .filter(|(standard, _)| *standard == name)
        .map(|(_, parser)| (*parser).to_owned())
        .chain(
            PARSER_SUBPRODUCTIONS
                .iter()
                .filter(|(_, standard)| *standard == name)
                .map(|(parser, _)| (*parser).to_owned()),
        )
        .collect::<Vec<_>>();
    let default = pascal(name);
    if Production::all()
        .iter()
        .any(|production| format!("{production:?}") == default)
    {
        names.push(default);
    }
    names.sort();
    names.dedup();
    names
}

/// Parser production names from the closed inventory.
fn parser_productions() -> BTreeSet<String> {
    Production::all()
        .iter()
        .map(|production| format!("{production:?}"))
        .collect()
}

fn ticket_id(ticket: &str) -> bool {
    let Some((team, number)) = ticket.split_once('-') else {
        return false;
    };
    !team.is_empty()
        && team.chars().all(|character| character.is_ascii_uppercase())
        && !number.is_empty()
        && number.chars().all(|character| character.is_ascii_digit())
}

fn allow_list_errors(
    divergences: &[Divergence],
    standard: &BTreeSet<String>,
    parser: &BTreeSet<String>,
) -> Vec<String> {
    let mapped_parser: BTreeSet<String> = standard
        .iter()
        .flat_map(|name| parser_names_for(name))
        .filter(|production| parser.contains(production))
        .collect();
    let mut errors = Vec::new();
    for divergence in divergences {
        if !ticket_id(divergence.ticket) {
            errors.push(format!(
                "allow-list entry `{}` has no ticket id",
                divergence.production
            ));
        }
        if !divergence
            .register_row
            .strip_prefix("G-")
            .is_some_and(|digits| {
                !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
            })
        {
            errors.push(format!(
                "allow-list entry `{}` has no grammar register row",
                divergence.production
            ));
        }
        if divergence.reason.trim().is_empty() {
            errors.push(format!(
                "allow-list entry `{}` has no reason",
                divergence.production
            ));
        }
        let still_diverges = match divergence.side {
            Side::Standard => {
                standard.contains(divergence.production)
                    && parser_names_for(divergence.production)
                        .iter()
                        .all(|production| !parser.contains(production))
            }
            Side::Parser => {
                parser.contains(divergence.production)
                    && !mapped_parser.contains(divergence.production)
            }
        };
        if !still_diverges {
            errors.push(format!(
                "stale allow-list entry `{}`: it no longer diverges",
                divergence.production
            ));
        }
    }
    errors
}

fn standard_drift(standard: &BTreeSet<String>, parser: &BTreeSet<String>) -> Vec<String> {
    let mut found = Vec::new();
    for name in standard {
        if parser_names_for(name)
            .iter()
            .any(|production| parser.contains(production))
        {
            continue;
        }
        if !DIVERGENCES
            .iter()
            .any(|entry| entry.side == Side::Standard && entry.production == name)
        {
            found.push(format!(
                "standard production `{name}` has no parser production"
            ));
        }
    }
    found
}

fn parser_drift(standard: &BTreeSet<String>, parser: &BTreeSet<String>) -> Vec<String> {
    let mapped: BTreeSet<String> = standard
        .iter()
        .flat_map(|name| parser_names_for(name))
        .filter(|production| parser.contains(production))
        .collect();
    parser
        .iter()
        .filter(|production| {
            !mapped.contains(*production)
                && !DIVERGENCES.iter().any(|entry| {
                    entry.side == Side::Parser && entry.production == production.as_str()
                })
        })
        .map(|production| format!("parser production `{production}` has no standard production"))
        .collect()
}

fn mapping_errors(standard: &BTreeSet<String>, parser: &BTreeSet<String>) -> Vec<String> {
    let mut found = Vec::new();
    for (standard_name, parser_name) in CORRESPONDENCE {
        if !standard.contains(*standard_name) {
            found.push(format!(
                "stale correspondence `{standard_name}`: not in the standard"
            ));
        }
        if !parser.contains(*parser_name) {
            found.push(format!(
                "stale correspondence `{parser_name}`: not in the parser"
            ));
        }
    }
    for (parser_name, standard_name) in PARSER_SUBPRODUCTIONS {
        if !standard.contains(*standard_name) {
            found.push(format!(
                "stale parser subproduction `{parser_name}`: `{standard_name}` is not in the standard"
            ));
        }
        if !parser.contains(*parser_name) {
            found.push(format!(
                "stale parser subproduction `{parser_name}`: not in the parser"
            ));
        }
    }
    found
}

fn drift_with(standard: &BTreeSet<String>, parser: &BTreeSet<String>) -> Vec<String> {
    let mut found = standard_drift(standard, parser);
    found.extend(parser_drift(standard, parser));
    found.extend(mapping_errors(standard, parser));
    found.extend(allow_list_errors(DIVERGENCES, standard, parser));
    found
}

fn configured_grammar() -> Result<String, String> {
    let root = std::env::var_os("FSPEC_DIR").map(std::path::PathBuf::from);
    read_shared_grammar(root.as_deref())
}

#[trace("QSL-2107")]
#[test]
fn every_standard_production_maps_to_a_parser_production() {
    let grammar = configured_grammar().expect("FSPEC_DIR must name an FSpec checkout");
    let standard = grammar_productions(&grammar);
    let parser = parser_productions();
    let mut found = standard_drift(&standard, &parser);
    found.extend(mapping_errors(&standard, &parser));
    assert!(
        !standard.is_empty(),
        "the shared grammar defines productions"
    );
    assert!(
        found.is_empty(),
        "standard grammar drift:\n{}",
        found.join("\n")
    );
}

#[trace("QSL-2107")]
#[test]
fn every_parser_production_appears_in_the_standard() {
    let grammar = configured_grammar().expect("FSPEC_DIR must name an FSpec checkout");
    let standard = grammar_productions(&grammar);
    let parser = parser_productions();
    let mut found = parser_drift(&standard, &parser);
    found.extend(mapping_errors(&standard, &parser));
    assert!(
        found.is_empty(),
        "parser grammar drift:\n{}",
        found.join("\n")
    );
}

#[trace("QSL-2107")]
#[test]
fn allow_list_entries_name_a_ticket_and_still_diverge() {
    let grammar = configured_grammar().expect("FSPEC_DIR must name an FSpec checkout");
    let standard = grammar_productions(&grammar);
    let parser = parser_productions();
    let found = allow_list_errors(DIVERGENCES, &standard, &parser);
    assert!(
        found.is_empty(),
        "grammar drift allow-list:\n{}",
        found.join("\n")
    );

    let mut repaired = standard.clone();
    repaired.insert("population-bound".to_owned());
    let stale = allow_list_errors(DIVERGENCES, &repaired, &parser);
    assert!(
        stale
            .iter()
            .any(|error| error.contains("PopulationBound") && error.contains("stale")),
        "a now-matched production must make its allow-list entry stale: {stale:?}"
    );
}

#[trace("QSL-2107")]
#[test]
fn a_renamed_production_fails() {
    let grammar = configured_grammar().expect("FSPEC_DIR must name an FSpec checkout");
    let renamed = grammar.replacen("header       =", "renamed-header =", 1);
    assert_ne!(grammar, renamed, "fixture changes a production name");
    let standard = grammar_productions(&renamed);
    let parser = parser_productions();
    let found = drift_with(&standard, &parser);
    assert!(
        found.iter().any(|error| error.contains("renamed-header")),
        "a changed production name fails by name: {found:?}"
    );
}

#[trace("QSL-2107")]
#[test]
fn a_missing_fspec_dir_is_an_error_not_a_skip() {
    let error = read_shared_grammar(None).expect_err("missing FSPEC_DIR is an error");
    assert!(
        error.contains("FSPEC_DIR"),
        "the error names the missing input: {error}"
    );
}
