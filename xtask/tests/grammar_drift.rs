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
    filament_specification::published_file(path).and_then(|bytes| std::str::from_utf8(bytes).ok())
}

/// Every production name defined in an `ebnf` fence of the shared grammar.
fn shared_grammar_productions() -> BTreeSet<String> {
    let text = published_file(SHARED_GRAMMAR)
        .unwrap_or_else(|| panic!("{SHARED_GRAMMAR}: not published by the pinned standard"));
    let mut names = BTreeSet::new();
    let mut in_ebnf = false;
    let lines: Vec<&str> = text.lines().collect();
    for (index, line) in lines.iter().copied().enumerate() {
        if line.starts_with("```") {
            in_ebnf = line.trim() == "```ebnf";
            continue;
        }
        if !in_ebnf {
            continue;
        }
        // The shared grammar writes `probabilistic-form` and `schedulability-clause`
        // with the `=` on the line after the name; the name still defines one.
        let continued = lines
            .get(index + 1)
            .is_some_and(|next| next.trim_start().starts_with('=') && next.starts_with(' '));
        let Some((name, _)) = line.split_once('=').or(continued.then_some((line, ""))) else {
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
    ("abstraction-decl", "AbstractionDeclaration"),
    ("alias-decl", "AliasDeclaration"),
    ("binder", "PopulationBinder"),
    ("component-decl", "CompositionComponent"),
    ("composition-decl", "CompositionDeclaration"),
    ("delay-decl", "DelayDeclaration"),
    ("delay-dist", "DelayDistribution"),
    ("dimension-decl", "DimensionDeclaration"),
    ("enum-decl", "EnumDeclaration"),
    ("environment-premise", "GuaranteeEnvironment"),
    ("function", "FunctionDeclaration"),
    ("graph-body", "StateGraphBody"),
    ("graph-conjunction", "StateGraphConjunction"),
    ("graph-disjunction", "StateGraphDisjunction"),
    ("graph-implication", "StateGraphImplication"),
    ("graph-predicate", "StateGraphPredicate"),
    ("graph-relation", "StateGraphRelation"),
    ("graph-unary", "StateGraphUnary"),
    ("guarantee-decl", "GuaranteeDeclaration"),
    ("hybrid-decl", "HybridDeclaration"),
    ("initial-binding", "GuaranteeInitial"),
    ("mode-switch-decl", "ModeSwitchDeclaration"),
    ("node-ref", "NodeReference"),
    ("progress-premise", "GuaranteeProgress"),
    ("protocol-step-row", "StepRow"),
    ("protocol-step-source", "StepSource"),
    ("protocol-step-target", "StepTarget"),
    ("quantity-unit", "UnitExpression"),
    ("random-decl", "RandomDeclaration"),
    ("record-decl", "RecordDeclaration"),
    ("release-decl", "ReleaseDeclaration"),
    ("reward-decl", "RewardDeclaration"),
    ("signed-int", "SignedInteger"),
    ("state-graph-clause", "StateGraphClaim"),
    ("synthesis-decl", "SynthesisDeclaration"),
    ("taskset-decl", "TasksetDeclaration"),
    ("temporal-expr", "TemporalExpression"),
    ("terminal-decl", "TerminalDeclaration"),
    ("time-decl", "TimeDeclaration"),
    ("tuple-decl", "TupleDeclaration"),
    ("union-decl", "UnionDeclaration"),
    ("unit-decl", "UnitDeclaration"),
    ("urgent-decl", "UrgentDeclaration"),
    ("window", "PastWindow"),
    ("workload-decl", "WorkloadDeclaration"),
];

/// FR-600's standard-only allow-list: a standard production with no parser
/// production, and why.
const STANDARD_ONLY: &[(&str, &str)] = &[
    ("object-member", "inlined alternative (FR-600 Behavior, FSpec FR-610): inlined into ObjectDeclaration"),
    ("absence-mode", "inlined alternative (FR-600 Behavior): three keyword alternatives the parser writes in place"),
    ("abstraction-binding", "inlined alternative (FR-600 Behavior): choice the parser writes in place inside AbstractionDeclaration"),
    ("always-possible-clause", "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim"),
    ("attempt-type", "inlined alternative (FR-600 Behavior): alias of type-name, recognized as TypeName"),
    ("bound-op", "inlined alternative (FR-600 Behavior): two operator alternatives the parser writes in place"),
    ("call-argument", "inlined alternative (FR-600 Behavior): choice between a lambda and an expression, written in place"),
    ("component-clause", "inlined alternative (FR-600 Behavior): two-identifier form the parser writes in place in the composition members"),
    ("composition-member", "inlined alternative (FR-600 Behavior): choice whose alternatives are CompositionGuarantee, CompositionRely, CompositionDepends and CompositionSupplies"),
    ("composition-rule", "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place inside CompositionDeclaration"),
    ("control-form", "inlined alternative (FR-600 Behavior): choice of control forms the parser writes in place"),
    ("graph-over", "inlined alternative (FR-600 Behavior): optional clause the parser writes in place inside the state-graph claims"),
    ("guarantee-member", "inlined alternative (FR-600 Behavior): choice whose alternatives are GuaranteeProgress and GuaranteeEnvironment"),
    ("lambda", "inlined alternative (FR-600 Behavior): single-parameter form the parser writes in place inside the call argument"),
    ("loop-proof", "inlined alternative (FR-600 Behavior): two-block form the parser writes in place"),
    ("loss-selector", "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place"),
    ("policy", "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place"),
    ("schedulability-clause", "inlined alternative (FR-600 Behavior): choice whose alternatives are SchedulableDeclaration and ResponseDeclaration"),
    ("possible-clause", "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim"),
    ("power", "inlined alternative (FR-600 Behavior): precedence level the parser writes in place in the expression ladder"),
    ("protocol-reference", "inlined alternative (FR-600 Behavior): two-token form the parser writes in place"),
    ("rust-field", "lexical terminal (FR-600 Behavior): a string position, a string token in the parser"),
    ("rust-name", "lexical terminal (FR-600 Behavior): a string position, a string token in the parser"),
    ("rust-path", "lexical terminal (FR-600 Behavior): a string position, a string token in the parser"),
    ("rust-receiver", "lexical terminal (FR-600 Behavior): a string position, a string token in the parser"),
    ("synthesis-objective", "inlined alternative (FR-600 Behavior): optional clause the parser writes in place inside SynthesisDeclaration"),
    ("temporal-relation-unbounded", "inlined alternative (FR-600 Behavior): precedence level the parser writes in place of the unbounded temporal ladder"),
    ("temporal-unary-unbounded", "inlined alternative (FR-600 Behavior): precedence level the parser writes in place of the unbounded temporal ladder"),
    ("unique-path-clause", "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim"),
    ("unit-term", "inlined alternative (FR-600 Behavior): term of a unit expression, written in place inside UnitExpression"),
];

/// FR-600's parser-only allow-list: a parser production with no standard
/// production, and why.
const PARSER_ONLY: &[(&str, &str)] = &[
    ("AggregateKind", "inlined alternatives (FR-600 Behavior) the standard writes in place inside aggregate-term"),
    ("AllocationDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("CaseBinder", "parser-internal helper: one binder of a case arm, written in place by the standard (FR-600 Behavior)"),
    ("CaseExpression", "parser-internal helper: the standard writes the case form inside expr (FR-600 Behavior)"),
    ("CompositionDepends", "inlined alternative (FR-600 Behavior) of the standard composition-member"),
    ("CompositionGuarantee", "inlined alternative (FR-600 Behavior) of the standard composition-member"),
    ("CompositionRely", "inlined alternative (FR-600 Behavior) of the standard composition-member"),
    ("CompositionSupplies", "inlined alternative (FR-600 Behavior) of the standard composition-member"),
    ("ConnectionDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainContract", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainField", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainInvariant", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainMultiplicity", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainObject", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainOperation", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainPopulation", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainRecordDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("DomainRelationship", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("EnumValue", "parser-internal helper: an enum member reference, written in place by the standard (FR-600 Behavior)"),
    ("EventDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("ExtensionDeclaration", "extension-only production (FR-600 Behavior): the declarations an installed extension adds (FR-354)"),
    ("FairnessRow", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("FieldRow", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("HistoryLiteral", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("HistoryRow", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("HistoryUpdate", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("HyperComparison", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperConjunction", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperDisjunction", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperExpression", "parser-internal helper with no source meaning (FR-600 Behavior): the root of the hyper body ladder, which the standard writes as expr (FSpec FR-395)"),
    ("HyperImplication", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperPostfix", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperPrimary", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperProduct", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperSum", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("HyperUnary", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as expr (FSpec FR-395)"),
    ("InterfaceDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("MachineDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("MapDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("ModelUnit", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("NamespaceDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("ObjectRow", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("PartDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("PopulationCall", "parser-internal helper: the population call form the standard writes inside primary (FR-600 Behavior)"),
    ("PopulationRow", "refinement row of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("PortDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("ProcessDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("Refinement", "refinement declaration of FSpec FR-375; FSpec FR-503 keeps prose-only forms out of the production inventory, so FR-600 Behavior records it as parser-only"),
    ("ResponseDeclaration", "inlined alternative (FR-600 Behavior) of the standard schedulability-clause"),
    ("SchedulableDeclaration", "inlined alternative (FR-600 Behavior) of the standard schedulability-clause"),
    ("ScrutineeComparison", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeConjunction", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeDisjunction", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeExpression", "parser-internal helper with no source meaning (FR-600 Behavior): the root of the case scrutinee ladder, which the standard writes as expr"),
    ("ScrutineeImplication", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineePostfix", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineePrimary", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeProduct", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeSum", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("ScrutineeUnary", "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr"),
    ("StepArgument", "named by the standard only in prose (FSpec FR-375) with no ebnf definition; the standard-side gap is QSL-2960"),
    ("StepDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("StepSumObjective", "parser-internal helper: the step-sum form of a synthesis objective, written inside expr by the standard (FR-600 Behavior)"),
    ("StoreDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("TransitionDeclaration", "extension-only production (FR-600 Behavior): a form of the domain-package model unit of FR-710, which the shared grammar does not define"),
    ("TupleValue", "parser-internal helper: a tuple value, written in place inside primary by the standard (FR-600 Behavior)"),
];

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
#[ignore = "QSL-2960: the pinned standard uses decimal-literal, step-target and step-argument without an ebnf definition and outside the five lexical terminals; a filament-specification change"]
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
