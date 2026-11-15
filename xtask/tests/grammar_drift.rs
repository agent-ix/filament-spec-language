// SPDX-License-Identifier: AGPL-3.0-or-later
//! Grammar drift between FSpec's shared grammar and the complete-V1 parser:
//! each `ebnf` production name of `proposals/filament-v1/shared-grammar.md`
//! maps to a parser `Production` or to a named allow-list entry.

use std::collections::BTreeSet;

use fsl_cst::Production;
use ix_trace_rs::trace;

/// The shared grammar's published path inside the standard's pinned package.
const SHARED_GRAMMAR: &str = "proposals/filament-v1/shared-grammar.md";

/// The bytes of a standard file, read only through the pinned
/// `filament_specification` crate (FR-760, FSpec FR-660): no environment
/// variable, no sibling checkout, no filesystem path.
fn published_file(path: &str) -> Option<&'static str> {
    // QSL-2107: until the workspace takes `filament-specification-qualification`
    // as a tagged dev-dependency (FR-760), this is the one place that becomes
    // `filament_specification::published_file(path)`.
    panic!("{path}: the pinned filament_specification crate is not a dependency yet (FR-760)")
}

/// Every production name defined in an `ebnf` fence of the shared grammar.
fn shared_grammar_productions() -> BTreeSet<String> {
    let text = published_file(SHARED_GRAMMAR)
        .unwrap_or_else(|| panic!("{SHARED_GRAMMAR}: not published by the pinned standard"));
    let mut names = BTreeSet::new();
    let mut in_ebnf = false;
    for line in text.lines() {
        if line.starts_with("```") {
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
        if !name.is_empty()
            && !line.starts_with(' ')
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
        {
            names.insert(name.to_owned());
        }
    }
    names
}

/// The parser's production names.
fn parser_productions() -> BTreeSet<String> {
    Production::all()
        .iter()
        .map(|production| format!("{production:?}"))
        .collect()
}

/// FSpec FR-610: the object declaration productions map to the parser's,
/// and `object-member`, a choice the parser inlines into
/// `ObjectDeclaration`, is the one allow-listed name.
#[trace("FSpec-FR-610-AC-1", "FSpec-TC-1440", "FR-569-AC-1", "TC-1408")]
#[test]
#[ignore = "QSL-2107: the grammar drift gate maps the standard's object productions; not implemented yet"]
fn the_object_declaration_productions_are_mapped() {
    const MAPPED: [(&str, &str); 9] = [
        ("object-decl", "ObjectDeclaration"),
        ("population-member", "ObjectPopulation"),
        ("operation-member", "ObjectOperation"),
        ("frame-clause", "FrameClause"),
        ("field-target", "FieldTarget"),
        ("delete-target", "DeleteTarget"),
        ("type-name", "TypeName"),
        ("operation-name", "OperationName"),
        ("population-name", "PopulationName"),
    ];
    const INLINED: [&str; 1] = ["object-member"];
    let standard = shared_grammar_productions();
    let parser = parser_productions();
    for (name, production) in MAPPED {
        assert!(
            standard.contains(name),
            "the shared grammar defines `{name}`"
        );
        assert!(
            parser.contains(production),
            "`{name}` maps to the parser production `{production}`"
        );
    }
    for name in INLINED {
        assert!(
            standard.contains(name),
            "the shared grammar defines `{name}`"
        );
    }
}

/// FR-600's correspondence table: a standard production name and the parser
/// production that recognizes it, where the PascalCase spelling of the name
/// is not the variant's spelling.
const CORRESPONDENCE: &[(&str, &str)] = &[
    ("object-decl", "ObjectDeclaration"),
    ("population-member", "ObjectPopulation"),
    ("operation-member", "ObjectOperation"),
    ("composed-unit", "CompleteUnit"),
    ("complete-unit", "CompleteUnit"),
    ("import-decl", "ImportDeclaration"),
    ("type-ref", "TypeReference"),
    ("expr", "Expression"),
    ("population-bound", "PopulationBound"),
    ("delivery-interval", "DeliveryInterval"),
    ("quantified-clock-bound", "QuantifiedClockBound"),
    ("protocol-requirement", "ProtocolRequirement"),
];

/// FR-600's standard-only allow-list: a standard production with no parser
/// production, and why.
const STANDARD_ONLY: &[(&str, &str)] = &[("object-member", "inlined into ObjectDeclaration")];

/// FR-600's parser-only allow-list: a parser production with no standard
/// production, and why.
const PARSER_ONLY: &[(&str, &str)] = &[];

/// The PascalCase spelling of a kebab-case production name.
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

/// Every disagreement between the two inventories under the table and the
/// allow-lists, each naming its production and side.
fn disagreements(
    standard: &BTreeSet<String>,
    parser: &BTreeSet<String>,
    correspondence: &[(&str, &str)],
    standard_only: &[(&str, &str)],
    parser_only: &[(&str, &str)],
) -> Vec<String> {
    let mapped = |name: &str| -> String {
        correspondence
            .iter()
            .find(|(standard_name, _)| *standard_name == name)
            .map(|(_, production)| (*production).to_owned())
            .unwrap_or_else(|| pascal(name))
    };
    let mut found = Vec::new();
    let mut matched = BTreeSet::new();
    for name in standard {
        let production = mapped(name);
        if parser.contains(&production) {
            matched.insert(production);
        } else if !standard_only.iter().any(|(entry, _)| entry == name) {
            found.push(format!(
                "standard production `{name}` has no parser production"
            ));
        }
    }
    for production in parser {
        if !matched.contains(production)
            && !parser_only.iter().any(|(entry, _)| entry == production)
        {
            found.push(format!(
                "parser production `{production}` has no standard production"
            ));
        }
    }
    for (name, production) in correspondence {
        if !standard.contains(*name) {
            found.push(format!(
                "stale correspondence entry `{name}`: not in the standard"
            ));
        }
        if !parser.contains(*production) {
            found.push(format!(
                "stale correspondence entry `{production}`: not in the parser"
            ));
        }
    }
    for (name, _) in standard_only {
        if !standard.contains(*name) {
            found.push(format!("stale standard-only entry `{name}`"));
        }
    }
    for (production, _) in parser_only {
        if !parser.contains(*production) {
            found.push(format!("stale parser-only entry `{production}`"));
        }
    }
    found
}

/// FR-600-AC-1: every standard production and every parser production is
/// matched or allow-listed with a reason.
#[trace("TC-1500", "FR-600-AC-1", "FSpec-TC-1010", "FSpec-FR-503-AC-1")]
#[test]
#[ignore = "QSL-2107: the two production inventories do not yet correspond; not implemented yet"]
fn every_standard_and_parser_production_is_matched_or_allow_listed() {
    let found = disagreements(
        &shared_grammar_productions(),
        &parser_productions(),
        CORRESPONDENCE,
        STANDARD_ONLY,
        PARSER_ONLY,
    );
    assert!(found.is_empty(), "grammar drift:\n{}", found.join("\n"));
}

/// FR-600-AC-2: an invented standard name, a removed parser production and a
/// stale entry each fail naming the production and its side.
#[trace("TC-1500", "FR-600-AC-2")]
#[test]
#[ignore = "QSL-2107: the drift gate does not yet detect each disagreement; not implemented yet"]
fn drift_and_stale_entries_are_reported_by_name() {
    let mut standard = shared_grammar_productions();
    let parser = parser_productions();
    standard.insert("invented-form".to_owned());
    let found = disagreements(
        &standard,
        &parser,
        CORRESPONDENCE,
        STANDARD_ONLY,
        PARSER_ONLY,
    );
    assert!(
        found.contains(&"standard production `invented-form` has no parser production".to_owned())
    );

    let standard = shared_grammar_productions();
    let mut parser = parser_productions();
    parser.remove("DeliveryInterval");
    let found = disagreements(
        &standard,
        &parser,
        CORRESPONDENCE,
        STANDARD_ONLY,
        PARSER_ONLY,
    );
    assert!(found.iter().any(|line| line.contains("delivery-interval")));
    assert!(found
        .contains(&"stale correspondence entry `DeliveryInterval`: not in the parser".to_owned()));
}

/// FR-600-AC-3: the shared grammar is read from the pinned standard crate,
/// which publishes it, and a path it does not publish yields nothing.
#[trace("TC-1500", "FR-600-AC-3")]
#[test]
#[ignore = "QSL-2107: the drift gate does not yet read the shared grammar through the pinned standard crate; not implemented yet"]
fn the_shared_grammar_is_read_through_the_pinned_standard_crate() {
    assert!(published_file(SHARED_GRAMMAR).is_some_and(|text| text.contains("```ebnf")));
    assert!(published_file("proposals/filament-v1/no-such-grammar.md").is_none());
}

/// FSpec FR-502 and FR-501: the productions the standard gained from the
/// parser, and the optional `clock`, exist on both sides.
#[trace(
    "FSpec-TC-1009",
    "FSpec-FR-502-AC-1",
    "FSpec-FR-502-AC-2",
    "FSpec-FR-502-AC-3",
    "FSpec-FR-502-AC-4"
)]
#[test]
#[ignore = "QSL-2107: the drift gate does not yet map the population bound, quantified clock bound, delivery interval and protocol requirement; not implemented yet"]
fn the_language_forms_recorded_in_the_standard_are_mapped() {
    let standard = shared_grammar_productions();
    let parser = parser_productions();
    for (name, production) in [
        ("population-bound", "PopulationBound"),
        ("quantified-clock-bound", "QuantifiedClockBound"),
        ("delivery-interval", "DeliveryInterval"),
        ("protocol-requirement", "ProtocolRequirement"),
    ] {
        assert!(
            standard.contains(name),
            "the shared grammar defines `{name}`"
        );
        assert!(
            parser.contains(production),
            "`{name}` maps to `{production}`"
        );
    }
}

/// FSpec FR-503-AC-1 and AC-2: the shared grammar's inventory is closed under
/// use: every `=/` names an earlier `=` definition, and every right-hand-side
/// name is defined or is a lexical terminal.
#[trace("FSpec-TC-1010", "FSpec-FR-503-AC-1", "FSpec-FR-503-AC-2")]
#[test]
#[ignore = "QSL-2107: the shared grammar is not yet read through the pinned standard crate and checked for closure; not implemented yet"]
fn every_name_the_shared_grammar_uses_is_defined() {
    const TERMINALS: [&str; 5] = ["ident", "member-name", "string", "uint", "EOF"];
    let text = published_file(SHARED_GRAMMAR).expect("published");
    let defined = shared_grammar_productions();
    assert!(!defined.is_empty());
    let mut in_ebnf = false;
    let mut undefined = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.starts_with("```") {
            in_ebnf = line.trim() == "```ebnf";
            continue;
        }
        if !in_ebnf {
            continue;
        }
        let body = if line.starts_with(' ') {
            line
        } else {
            line.split_once('=').map_or(line, |(_, rest)| rest)
        };
        let mut quoted = false;
        let mut word = String::new();
        for c in body.chars().chain(std::iter::once(' ')) {
            if c == '\'' || c == '"' {
                quoted = !quoted;
                word.clear();
            } else if !quoted && (c.is_ascii_alphanumeric() || c == '-') {
                word.push(c);
            } else {
                if !word.is_empty()
                    && word
                        .chars()
                        .next()
                        .is_some_and(|f| f.is_ascii_lowercase() || f == 'E')
                    && !defined.contains(&word)
                    && !TERMINALS.contains(&word.as_str())
                {
                    undefined.push(format!("line {}: `{word}`", number + 1));
                }
                word.clear();
            }
        }
    }
    assert!(
        undefined.is_empty(),
        "undefined names:\n{}",
        undefined.join("\n")
    );
}

/// FR-600-AC-1: every parser production the correspondence table and the
/// parser-only allow-list name exists in the parser, so a misspelled entry
/// cannot hide a production from the drift gate.
#[trace("TC-1500", "FR-600-AC-1")]
#[test]
fn every_parser_production_the_tables_name_exists() {
    let parser = parser_productions();
    let named = CORRESPONDENCE
        .iter()
        .map(|(_, production)| *production)
        .chain(PARSER_ONLY.iter().map(|(production, _)| *production));
    let missing = named
        .filter(|production| !parser.contains(*production))
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "not parser productions: {missing:?}");
}
