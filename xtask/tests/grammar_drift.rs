// SPDX-License-Identifier: AGPL-3.0-or-later
//! Grammar drift between FSpec's shared grammar and the complete-V1 parser:
//! each `ebnf` production name of `proposals/filament-v1/shared-grammar.md`
//! maps to a parser `Production` or to a named allow-list entry.

use std::collections::BTreeSet;

use fsl_cst::Production;
use ix_trace_rs::trace;

/// The shared grammar's published path inside the standard's pinned package.
const SHARED_GRAMMAR: &str = "proposals/filament-v1/shared-grammar.md";
const TC_1010: &str = "spec/test-cases/TC-1010-grammar-production-inventory-is-self-consistent.md";

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

/// The lexical terminals allowed by FSpec FR-503-AC-2, repeated in TC-1010.
fn shared_grammar_lexical_terminals() -> BTreeSet<String> {
    let text = published_file(TC_1010)
        .unwrap_or_else(|| panic!("{TC_1010}: not published by the pinned standard"));
    let expected = text
        .lines()
        .skip_while(|line| *line != "## Expected Results")
        .skip(1)
        .find(|line| line.contains("every right-hand-side name is defined or is one of"))
        .unwrap_or_else(|| panic!("{TC_1010}: expected terminal list is missing"));
    let names = expected
        .split_once("or is one of")
        .and_then(|(_, rest)| rest.split_once(". The"))
        .map(|(names, _)| names)
        .unwrap_or_else(|| panic!("{TC_1010}: expected terminal list is malformed"));
    let terminals: BTreeSet<String> = names
        .split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, name)| name.to_owned())
        .collect();
    assert!(
        !terminals.is_empty(),
        "{TC_1010}: expected terminal list is empty"
    );
    terminals
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
    // The names correspond but the alternatives differ: the standard accepts cfail, ctimeout and
    // cexpire, which the parser's StepSource does not (QSL-2967).
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
    // The shared grammar's unit-term and dimension-term have identical
    // syntax; the parser uses its existing DimensionTerm production for both.
    ("unit-term", "DimensionTerm"),
];

/// FR-600's standard-only allow-list: a standard production with no parser
/// production, its reason, and the requirement clause that justifies it.
const STANDARD_ONLY: &[(&str, &str, &str)] = &[
    (
        "object-member",
        "inlined alternative (FR-600-AC-1; FSpec FR-610): inlined into ObjectDeclaration; FSpec FR-610-AC-1",
        "FSpec FR-610-AC-1",
    ),
    (
        "absence-mode",
        "inlined alternative (FR-600 Behavior): three keyword alternatives the parser writes in place; FSpec FR-153-AC-4",
        "FSpec FR-153-AC-4",
    ),
    (
        "abstraction-binding",
        "inlined alternative (FR-600 Behavior): choice the parser writes in place inside AbstractionDeclaration; FSpec FR-610-AC-1",
        "FSpec FR-610-AC-1",
    ),
    (
        "always-possible-clause",
        "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim; FR-165-AC-1",
        "FR-165-AC-1",
    ),
    (
        "attempt-type",
        "inlined alternative (FR-600 Behavior): alias of type-name, recognized as TypeName; FR-218-AC-1",
        "FR-218-AC-1",
    ),
    (
        "bound-op",
        "inlined alternative (FR-600 Behavior): two operator alternatives the parser writes in place; FR-392-AC-1",
        "FR-392-AC-1",
    ),
    (
        "call-argument",
        "inlined alternative (FR-600 Behavior): choice between a lambda and an expression, written in place; FR-194-AC-12",
        "FR-194-AC-12",
    ),
    (
        "component-clause",
        "inlined alternative (FR-600 Behavior): two-identifier form the parser writes in place in the composition members; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "composition-member",
        "inlined alternative (FR-600 Behavior): choice whose alternatives are CompositionGuarantee, CompositionRely, CompositionDepends and CompositionSupplies; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "composition-rule",
        "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place inside CompositionDeclaration; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "control-form",
        "inlined alternative (FR-600 Behavior): choice of control forms the parser writes in place; FR-218-AC-1",
        "FR-218-AC-1",
    ),
    (
        "graph-over",
        "inlined alternative (FR-600 Behavior): optional clause the parser writes in place inside the state-graph claims; FR-165-AC-1",
        "FR-165-AC-1",
    ),
    (
        "guarantee-member",
        "inlined alternative (FR-600 Behavior): choice whose alternatives are GuaranteeProgress and GuaranteeEnvironment; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "lambda",
        "inlined alternative (FR-600 Behavior): single-parameter form the parser writes in place inside the call argument; FR-194-AC-12",
        "FR-194-AC-12",
    ),
    (
        "loop-proof",
        "inlined alternative (FR-600 Behavior): two-block form the parser writes in place; FR-218-AC-1",
        "FR-218-AC-1",
    ),
    (
        "loss-selector",
        "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place; FR-218-AC-1",
        "FR-218-AC-1",
    ),
    (
        "policy",
        "inlined alternative (FR-600 Behavior): keyword choice the parser writes in place; FR-246-AC-1",
        "FR-246-AC-1",
    ),
    (
        "schedulability-clause",
        "inlined alternative (FR-600 Behavior): choice whose alternatives are SchedulableDeclaration and ResponseDeclaration; FR-246-AC-1",
        "FR-246-AC-1",
    ),
    (
        "possible-clause",
        "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim; FR-165-AC-1",
        "FR-165-AC-1",
    ),
    (
        "power",
        "inlined alternative (FR-600 Behavior): precedence level the parser writes in place in the expression ladder; FR-258-AC-1",
        "FR-258-AC-1",
    ),
    (
        "protocol-reference",
        "inlined alternative (FR-600 Behavior): two-token form the parser writes in place; FR-880-AC-3",
        "FR-880-AC-3",
    ),
    (
        "rust-field",
        "named alias of string in the shared grammar (not one of FR-503's five lexical terminals), written as a string token in the parser; FSpec FR-503-AC-2",
        "FSpec FR-503-AC-2",
    ),
    (
        "rust-name",
        "named alias of string in the shared grammar (not one of FR-503's five lexical terminals), written as a string token in the parser; FSpec FR-503-AC-2",
        "FSpec FR-503-AC-2",
    ),
    (
        "rust-path",
        "named alias of string in the shared grammar (not one of FR-503's five lexical terminals), written as a string token in the parser; FSpec FR-503-AC-2",
        "FSpec FR-503-AC-2",
    ),
    (
        "rust-receiver",
        "named alias of string in the shared grammar (not one of FR-503's five lexical terminals), written as a string token in the parser; FSpec FR-503-AC-2",
        "FSpec FR-503-AC-2",
    ),
    (
        "synthesis-objective",
        "inlined alternative (FR-600 Behavior): optional clause the parser writes in place inside SynthesisDeclaration; FR-754-AC-1",
        "FR-754-AC-1",
    ),
    (
        "temporal-relation-unbounded",
        "inlined alternative (FR-600 Behavior): precedence level the parser writes in place of the unbounded temporal ladder; FR-126-AC-1",
        "FR-126-AC-1",
    ),
    (
        "temporal-unary-unbounded",
        "inlined alternative (FR-600 Behavior): precedence level the parser writes in place of the unbounded temporal ladder; FR-126-AC-1",
        "FR-126-AC-1",
    ),
    (
        "unique-path-clause",
        "inlined alternative (FR-600 Behavior): alternative of StateGraphClaim; FR-165-AC-1",
        "FR-165-AC-1",
    ),
];

/// FR-600's parser-only allow-list: a parser production with no standard
/// production, its reason, and the requirement clause that justifies it.
const PARSER_ONLY: &[(&str, &str, &str)] = &[
    (
        "AggregateKind",
        "inlined alternatives (FR-600 Behavior) the standard writes in place inside aggregate-term; FR-392-AC-1",
        "FR-392-AC-1",
    ),
    (
        "AllocationDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "CaseBinder",
        "parser-internal helper: one binder of a case arm, written in place by the standard (FR-600 Behavior); FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "CaseExpression",
        "parser-internal helper: the standard writes the case form inside expr (FR-600 Behavior); FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "CompositionDepends",
        "inlined alternative (FR-600 Behavior) of the standard composition-member; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "CompositionGuarantee",
        "inlined alternative (FR-600 Behavior) of the standard composition-member; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "CompositionRely",
        "inlined alternative (FR-600 Behavior) of the standard composition-member; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "CompositionSupplies",
        "inlined alternative (FR-600 Behavior) of the standard composition-member; FR-530-AC-1",
        "FR-530-AC-1",
    ),
    (
        "ConnectionDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainContract",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainField",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainInvariant",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainMultiplicity",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainObject",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainOperation",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainPopulation",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainRecordDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "DomainRelationship",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "EnumValue",
        "parser-internal helper: an enum member reference, written in place by the standard (FR-600 Behavior); FR-313-AC-2",
        "FR-313-AC-2",
    ),
    (
        "EventDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "ExtensionDeclaration",
        "extension-only production (FR-600 Behavior): the declarations an installed extension adds (FR-354); FR-354-AC-1",
        "FR-354-AC-1",
    ),
    (
        "FairnessRow",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-6",
        "FSpec FR-375-AC-6",
    ),
    (
        "FieldRow",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-1",
        "FSpec FR-375-AC-1",
    ),
    (
        "HistoryLiteral",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-9",
        "FSpec FR-375-AC-9",
    ),
    (
        "HistoryRow",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-5",
        "FSpec FR-375-AC-5",
    ),
    (
        "HistoryUpdate",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-5",
        "FSpec FR-375-AC-5",
    ),
    (
        "HyperComparison",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperConjunction",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperDisjunction",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperExpression",
        "parser-internal helper with no source meaning (FR-600 Behavior): the root of the hyper body ladder, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperImplication",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperPostfix",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperPrimary",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperProduct",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperSum",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "HyperUnary",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of the hyper body, which the standard writes as a block or a temporal-expr (hyper-clause of the shared grammar; FSpec FR-395); FSpec FR-395-AC-1",
        "FSpec FR-395-AC-1",
    ),
    (
        "InterfaceDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "MachineDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "MapDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "ModelUnit",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "NamespaceDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "ObjectRow",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-1",
        "FSpec FR-375-AC-1",
    ),
    (
        "PartDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "PopulationCall",
        "parser-internal helper: the population call form the standard writes inside primary (FR-600 Behavior); FR-139-AC-1",
        "FR-139-AC-1",
    ),
    (
        "PopulationRow",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-1",
        "FSpec FR-375-AC-1",
    ),
    (
        "PortDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "ProcessDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "Refinement",
        "the standard has no ebnf production for this refinement form (FSpec FR-375 describes it in prose); QSL-2968; FSpec FR-375-AC-1",
        "FSpec FR-375-AC-1",
    ),
    (
        "ResponseDeclaration",
        "inlined alternative (FR-600 Behavior) of the standard schedulability-clause; FR-246-AC-1",
        "FR-246-AC-1",
    ),
    (
        "SchedulableDeclaration",
        "inlined alternative (FR-600 Behavior) of the standard schedulability-clause; FR-246-AC-1",
        "FR-246-AC-1",
    ),
    (
        "ScrutineeComparison",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeConjunction",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeDisjunction",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeExpression",
        "parser-internal helper with no source meaning (FR-600 Behavior): the root of the case scrutinee ladder, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeImplication",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineePostfix",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineePrimary",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeProduct",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeSum",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "ScrutineeUnary",
        "parser-internal helper with no source meaning (FR-600 Behavior): a precedence level of a case scrutinee, which the standard writes as expr; FR-318-AC-1",
        "FR-318-AC-1",
    ),
    (
        "StepArgument",
        "the standard has no ebnf production for this form (named in prose by FSpec FR-375); QSL-2960; FSpec FR-375-AC-4",
        "FSpec FR-375-AC-4",
    ),
    (
        "StepDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "StepSumObjective",
        "parser-internal helper: the step-sum form of a synthesis objective, written inside expr by the standard (FR-600 Behavior); FR-754-AC-11",
        "FR-754-AC-11",
    ),
    (
        "StoreDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "TransitionDeclaration",
        "FSL-owned form (FR-710): a form of the domain-package model unit, which the shared grammar does not define; FR-710-AC-3",
        "FR-710-AC-3",
    ),
    (
        "TupleValue",
        "parser-internal helper: a tuple value, written in place inside primary by the standard (FR-600 Behavior); FR-313-AC-2",
        "FR-313-AC-2",
    ),
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
    standard_only: &[(&str, &str, &str)],
    parser_only: &[(&str, &str, &str)],
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
        } else if !standard_only.iter().any(|(entry, _, _)| entry == name) {
            found.push(format!(
                "standard production `{name}` has no parser production"
            ));
        }
    }
    for production in parser {
        if !matched.contains(production)
            && !parser_only.iter().any(|(entry, _, _)| entry == production)
        {
            found.push(format!(
                "parser production `{production}` has no standard production"
            ));
        }
    }
    let mut keys = BTreeSet::new();
    for (name, _) in correspondence {
        if !keys.insert(*name) {
            found.push(format!("duplicate correspondence entry `{name}`"));
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
    for (name, _, _) in standard_only {
        if !standard.contains(*name) {
            found.push(format!("stale standard-only entry `{name}`"));
        } else if parser.contains(&mapped(name)) {
            found.push(format!(
                "stale standard-only entry `{name}`: the parser has `{}`",
                mapped(name)
            ));
        }
    }
    for (production, _, _) in parser_only {
        if !parser.contains(*production) {
            found.push(format!("stale parser-only entry `{production}`"));
        }
        if matched.contains(*production) {
            found.push(format!(
                "stale parser-only entry `{production}`: a standard production matches it"
            ));
        }
    }
    found
}

/// FR-600-AC-1: every standard production and every parser production is
/// matched or allow-listed with a reason.
#[trace("TC-1500", "FR-600-AC-1", "FSpec-TC-1010", "FSpec-FR-503-AC-1")]
#[test]
fn every_standard_and_parser_production_is_matched_or_allow_listed() {
    for (name, reason, clause) in STANDARD_ONLY.iter().chain(PARSER_ONLY) {
        assert!(
            reason.contains(clause) && *clause != "FR-600-AC-1",
            "allow-list entry `{name}` must cite its production-specific clause `{clause}`"
        );
    }
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
    assert!(
        found.contains(
            &"stale correspondence entry `DeliveryInterval`: not in the parser".to_owned()
        )
    );
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
fn every_name_the_shared_grammar_uses_is_defined() {
    let text = published_file(SHARED_GRAMMAR).expect("published");
    let defined = shared_grammar_productions();
    let terminals = shared_grammar_lexical_terminals();
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
                    && !terminals.contains(&word)
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
        .chain(PARSER_ONLY.iter().map(|(production, _, _)| *production));
    let missing = named
        .filter(|production| !parser.contains(*production))
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "not parser productions: {missing:?}");
}
