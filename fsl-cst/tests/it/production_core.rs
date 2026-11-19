// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable core-production cases grounded in the shared complete-V1 grammar.

use fsl_cst::{CompleteCause, CompleteCode, CstElement, CstNode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";";
const PROFILE: &str = "profile v = \"filament.value.complete/v1\";";

fn unit(declarations: &str) -> String {
    format!("{HEADER}\n{PROFILE}\n{declarations}")
}

fn parse(source: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:production-core"),
        "production-core.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("authored UTF-8 and source metadata")
}

fn accepted(source: &str) -> ParsedSource {
    let parsed = parse(source);
    assert!(
        parsed.is_admissible(),
        "{source}: {:?}",
        parsed.diagnostics()
    );
    parsed
}

fn spelling<'a>(source: &'a str, node: &CstNode) -> &'a str {
    let Span { start, end } = node.span();
    &source[start..end]
}

fn one<'a>(source: &str, parsed: &'a ParsedSource, kind: Production, text: &str) -> &'a CstNode {
    let mut matching = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == kind && spelling(source, node) == text);
    let node = matching
        .next()
        .unwrap_or_else(|| panic!("missing {kind:?}: {text}"));
    assert!(matching.next().is_none(), "duplicate {kind:?}: {text}");
    assert_eq!(parsed.cst().render_node(node).unwrap(), text.as_bytes());
    node
}

fn children<'a>(
    source: &'a str,
    parsed: &ParsedSource,
    node: &CstNode,
    kind: Production,
) -> Vec<&'a str> {
    node.children()
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(index) => Some(&parsed.cst().nodes()[*index]),
            CstElement::Token(_) => None,
        })
        .filter(|child| child.production() == kind)
        .map(|child| spelling(source, child))
        .collect()
}

fn refused(source: &str, code: CompleteCode, cause: CompleteCause, span: Span) {
    let parsed = parse(source);
    assert!(!parsed.is_admissible(), "{source}");
    let diagnostic = parsed.diagnostics().first().expect("a located refusal");
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (code, cause),
        "{source}: {diagnostic:?}"
    );
    assert_eq!(
        diagnostic.byte_span(),
        Some(span),
        "{source}: {diagnostic:?}"
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2", "FSpec-FR-339-AC-6")]
fn unit_prelude_repetitions_preserve_selections_and_utf8_spans() {
    for source in [HEADER.to_owned(), format!("{HEADER}\n{PROFILE}")] {
        let parsed = accepted(&source);
        one(&source, &parsed, Production::Header, HEADER);
        assert_eq!(parsed.cst().render(), source.as_bytes());
        assert!(children(
            &source,
            &parsed,
            parsed.cst().root(),
            Production::Declaration
        )
        .is_empty());
    }
    let source = format!("{HEADER}\r\n// λ keeps byte offsets distinct from character offsets\r\n{PROFILE}\r\nprofile other = \"acme/other\";\r\nimport \"acme/base\";\r\nimport \"acme/extra\" as Extra;\r\nmodel First = \"acme/first\";\r\nmodel Second = \"acme/second\";\r\ntype Flag = Boolean;\r\ntype Count = Integer;\r\n");
    let parsed = accepted(&source);
    assert_eq!(parsed.cst().render(), source.as_bytes());
    let selections = parsed.selections();
    assert_eq!(
        selections
            .profiles
            .iter()
            .map(|s| (s.alias.as_str(), s.identity.as_str()))
            .collect::<Vec<_>>(),
        [("v", "filament.value.complete/v1"), ("other", "acme/other")]
    );
    assert_eq!(
        selections
            .imports
            .iter()
            .map(|s| (s.alias.as_deref(), s.identity.as_str()))
            .collect::<Vec<_>>(),
        [(None, "acme/base"), (Some("Extra"), "acme/extra")]
    );
    assert_eq!(
        selections
            .models
            .iter()
            .map(|s| (s.alias.as_str(), s.identity.as_str()))
            .collect::<Vec<_>>(),
        [("First", "acme/first"), ("Second", "acme/second")]
    );
    for selected in &selections.models {
        assert_eq!(
            &source[selected.identity_span.start..selected.identity_span.end],
            format!("\"{}\"", selected.identity)
        );
        assert_eq!(
            &source[selected.span.start..selected.span.end],
            format!("model {} = \"{}\";", selected.alias, selected.identity)
        );
    }
    assert_eq!(
        children(
            &source,
            &parsed,
            parsed.cst().root(),
            Production::Declaration
        ),
        ["type Flag = Boolean;", "type Count = Integer;"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_prelude_members_refuse_the_missing_member_position() {
    for declaration in [
        "profile missing = ;",
        "import \"acme/base\" as ;",
        "model Missing = ;",
    ] {
        let source = format!("{HEADER}\n{declaration}");
        let start = source.rfind(';').unwrap();
        refused(
            &source,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            Span {
                start,
                end: start + 1,
            },
        );
    }
    let source = HEADER.trim_end_matches(';');
    refused(
        source,
        CompleteCode::InvalidSyntax,
        CompleteCause::UnexpectedEnd,
        Span {
            start: source.len(),
            end: source.len(),
        },
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn value_declarations_keep_ordered_members_payloads_and_optional_markers() {
    let declarations = [
        (
            "enum Switch { Off }",
            Production::EnumDeclaration,
            Production::EnumMember,
            vec!["Off"],
        ),
        (
            "ordered enum Signal { Stop = \"halt\", Wait, Go, }",
            Production::EnumDeclaration,
            Production::EnumMember,
            vec!["Stop = \"halt\"", "Wait", "Go"],
        ),
        (
            "record Pair { first: Integer; second: Option<Boolean>?; }",
            Production::RecordDeclaration,
            Production::Field,
            vec!["first: Integer;", "second: Option<Boolean>?;"],
        ),
        (
            "tuple Single(Boolean);",
            Production::TupleDeclaration,
            Production::TypeReference,
            vec!["Boolean"],
        ),
        (
            "tuple Pair(Integer, Reference<Store::Row>);",
            Production::TupleDeclaration,
            Production::TypeReference,
            vec!["Integer", "Reference<Store::Row>"],
        ),
        (
            "union Outcome { Empty, Item(Integer), Pair(Boolean, Integer), }",
            Production::UnionDeclaration,
            Production::UnionMember,
            vec!["Empty", "Item(Integer)", "Pair(Boolean, Integer)"],
        ),
    ];
    for (declaration, kind, member, expected) in declarations {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let node = one(&source, &parsed, kind, declaration);
        assert_eq!(
            children(&source, &parsed, node, member),
            expected,
            "{declaration}"
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn nonempty_declaration_lists_and_member_separators_refuse_malformed_neighbours() {
    for (declaration, marker) in [
        ("enum Empty {}", "}"),
        ("record Empty {}", "}"),
        ("union Empty {}", "}"),
        ("tuple Empty();", ")"),
        ("tuple Bad(Integer,);", ")"),
        ("union Bad { Value() }", ")"),
        ("enum Bad { Value = true }", "true"),
        ("record Bad { left: Integer, right: Boolean; }", ", right"),
    ] {
        let source = unit(declaration);
        let start = source.rfind(marker).unwrap();
        let width = if marker == ", right" { 1 } else { marker.len() };
        refused(
            &source,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            Span {
                start,
                end: start + width,
            },
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn type_reference_alternatives_retain_the_full_authored_type() {
    for ty in [
        "Boolean",
        "Integer",
        "Named",
        "Store::result",
        "Int[-4, 7]",
        "Rational[-4, 7; 1, 9]",
        "Decimal[-8, 8; 0, 3; exact]",
        "Float32",
        "Float64[nearest-even]",
        "Text[0, 12; nfc]",
        "Option<Option<Boolean>>",
        "Sequence<Integer>",
        "Sequence<Integer>[0, 0]",
        "Set<Boolean>",
        "Bag<Integer>[1, 3]",
        "OrderedSet<Integer>[0, 9]",
        "Map<Integer, Boolean>",
        "Map<Integer, Boolean>[0, 4]",
        "Reference<Store::Row>",
        "Population<Store::Row>",
        "Population<Store::Row>[0]",
        "Quantity<Units::m>",
        "Quantity<Units::m * Units::s^-2, Rational[-4, 4; 1, 8]>",
    ] {
        let declaration = format!("type Captured = {ty};");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let alias = one(&source, &parsed, Production::AliasDeclaration, &declaration);
        assert_eq!(
            children(&source, &parsed, alias, Production::TypeReference),
            [ty],
            "{ty}"
        );
        one(&source, &parsed, Production::TypeReference, ty);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn nested_type_members_are_attached_to_their_own_container() {
    let inner = "Sequence<Option<Reference<Store::Row>>>[0, 3]";
    let outer = format!("Map<Text[0, 4; nfc], {inner}>[0, 7]");
    let source = unit(&format!("type Nested = {outer};"));
    let parsed = accepted(&source);
    let map = one(&source, &parsed, Production::TypeReference, &outer);
    assert_eq!(
        children(&source, &parsed, map, Production::TypeReference),
        ["Text[0, 4; nfc]", inner]
    );
    let sequence = one(&source, &parsed, Production::TypeReference, inner);
    assert_eq!(
        children(&source, &parsed, sequence, Production::TypeReference),
        ["Option<Reference<Store::Row>>"]
    );
    let option = one(
        &source,
        &parsed,
        Production::TypeReference,
        "Option<Reference<Store::Row>>",
    );
    assert_eq!(
        children(&source, &parsed, option, Production::TypeReference),
        ["Reference<Store::Row>"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn rounding_and_text_profiles_are_exact_grammar_members() {
    for mode in [
        "exact",
        "toward-zero",
        "toward-positive",
        "toward-negative",
        "nearest-even",
        "nearest-away",
    ] {
        let source = unit(&format!("type Fixed = Decimal[-3, 3; 0, 2; {mode}];"));
        let parsed = accepted(&source);
        one(&source, &parsed, Production::RoundingMode, mode);
    }
    for profile in [
        "unicode-scalars",
        "nfc",
        "nfd",
        "nfkc",
        "nfkd",
        "binary-utf8",
    ] {
        let source = unit(&format!("type Label = Text[0, 12; {profile}];"));
        let parsed = accepted(&source);
        one(&source, &parsed, Production::TextProfile, profile);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_type_arguments_refuse_at_the_unfilled_argument_or_separator() {
    for (ty, marker) in [
        ("Map<Integer>", ">"),
        ("Map<Integer,, Boolean>", ", Boolean"),
        ("Sequence<Integer>[0,]", "]"),
        ("Int[-, 5]", ", 5"),
        ("Decimal[0, 1; 0, 2]", "]"),
        ("Reference<>", ">"),
        ("Population<Store::Row>[]", "]"),
    ] {
        let source = unit(&format!("type Broken = {ty};"));
        let start = source.rfind(marker).unwrap();
        refused(
            &source,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            Span {
                start,
                end: start + 1,
            },
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn function_options_and_parameter_repetitions_keep_their_own_nodes() {
    for (declaration, parameters, measure) in [
        ("function zero(): Integer pure { 0 }", vec![], None),
        (
            "function one using v(x: Integer): Integer pure { x }",
            vec!["x: Integer"],
            None,
        ),
        (
            "function two using v(x: Integer, y: Boolean): Integer pure decreases(x) { x }",
            vec!["x: Integer", "y: Boolean"],
            Some("x"),
        ),
    ] {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let node = one(
            &source,
            &parsed,
            Production::FunctionDeclaration,
            declaration,
        );
        assert_eq!(
            children(&source, &parsed, node, Production::Parameter),
            parameters
        );
        assert_eq!(
            children(&source, &parsed, node, Production::Expression),
            measure.into_iter().collect::<Vec<_>>()
        );
        let body = if parameters.is_empty() {
            "{ 0 }"
        } else {
            "{ x }"
        };
        assert_eq!(children(&source, &parsed, node, Production::Block), [body]);
    }
    let source = unit("predicate Ready(): Boolean { true }");
    let parsed = accepted(&source);
    let predicate = one(
        &source,
        &parsed,
        Production::Predicate,
        "predicate Ready(): Boolean { true }",
    );
    assert!(children(&source, &parsed, predicate, Production::Parameter).is_empty());
    assert_eq!(
        children(&source, &parsed, predicate, Production::Block),
        ["{ true }"]
    );
}

fn expression(text: &str) -> String {
    unit(&format!(
        "function inspect using v(): Integer pure {{ {text} }}"
    ))
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn expression_precedence_attaches_products_before_sums_and_implications_to_the_right() {
    for op in ["*", "/", "div", "rem", "mod"] {
        let product = format!("a {op} b");
        let formula = format!("{product} + c");
        let source = expression(&formula);
        let parsed = accepted(&source);
        let sum = one(&source, &parsed, Production::Sum, &formula);
        assert_eq!(
            children(&source, &parsed, sum, Production::Product),
            [product.as_str(), "c"]
        );
        let product = one(&source, &parsed, Production::Product, &product);
        assert_eq!(
            children(&source, &parsed, product, Production::Unary),
            ["a", "b"]
        );
    }
    let formula = "a implies b implies c";
    let source = expression(formula);
    let parsed = accepted(&source);
    let implication = one(&source, &parsed, Production::Implication, formula);
    assert_eq!(
        children(&source, &parsed, implication, Production::Disjunction),
        ["a"]
    );
    assert_eq!(
        children(&source, &parsed, implication, Production::Implication),
        ["b implies c"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FR-002-AC-5")]
fn powers_and_grouping_keep_their_authored_operand_spans() {
    for (formula, base, exponent) in [("-x^2", "x", "2"), ("(-x)^-2", "(-x)", "-2")] {
        let source = expression(formula);
        let parsed = accepted(&source);
        let power_text = if formula.starts_with('-') {
            "x^2"
        } else {
            formula
        };
        let power = one(&source, &parsed, Production::Power, power_text);
        assert_eq!(
            children(&source, &parsed, power, Production::Postfix),
            [base]
        );
        assert_eq!(
            children(&source, &parsed, power, Production::SignedInteger),
            [exponent]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn let_if_and_case_members_preserve_binder_and_branch_boundaries() {
    let formula = "let keep: Integer = 7 in if ok then keep else 0";
    let source = expression(formula);
    let parsed = accepted(&source);
    let binding = one(&source, &parsed, Production::Expression, formula);
    assert_eq!(
        children(&source, &parsed, binding, Production::TypeReference),
        ["Integer"]
    );
    assert_eq!(
        children(&source, &parsed, binding, Production::Expression),
        ["7", "if ok then keep else 0"]
    );
    let conditional = one(
        &source,
        &parsed,
        Production::Expression,
        "if ok then keep else 0",
    );
    assert_eq!(
        children(&source, &parsed, conditional, Production::Expression),
        ["ok", "keep", "0"]
    );
    let formula = "case tag { Result::Good(payload): payload; Result::Bad: 0; }";
    let source = expression(formula);
    let parsed = accepted(&source);
    let case = one(&source, &parsed, Production::CaseExpression, formula);
    assert_eq!(
        children(&source, &parsed, case, Production::ScrutineeExpression),
        ["tag"]
    );
    assert_eq!(
        children(&source, &parsed, case, Production::CaseArm),
        ["Result::Good(payload): payload;", "Result::Bad: 0;"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn collection_and_record_value_repetitions_preserve_element_order() {
    for constructor in ["sequence", "set", "bag", "orderedSet"] {
        for (values, expected) in [
            ("", vec![]),
            ("4", vec!["4"]),
            ("4, 2, 9", vec!["4", "2", "9"]),
        ] {
            let value = format!("{constructor}[{values}]");
            let source = expression(&value);
            let parsed = accepted(&source);
            let collection = one(&source, &parsed, Production::CollectionValue, &value);
            assert_eq!(
                children(&source, &parsed, collection, Production::Expression),
                expected
            );
        }
    }
    let value = "Payload { high: 9, low: 2 }";
    let source = expression(value);
    let parsed = accepted(&source);
    let record = one(&source, &parsed, Production::RecordValue, value);
    assert_eq!(
        children(&source, &parsed, record, Production::FieldValue),
        ["high: 9", "low: 2"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn postfix_calls_and_indices_stay_inside_their_receiver_chain() {
    let formula = "root.child.method(1, 2)[0].value";
    let source = expression(formula);
    let parsed = accepted(&source);
    let postfix = one(&source, &parsed, Production::Postfix, formula);
    assert_eq!(
        children(&source, &parsed, postfix, Production::Primary),
        ["root"]
    );
    assert_eq!(
        children(&source, &parsed, postfix, Production::Expression),
        ["1", "2", "0"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_expression_neighbours_refuse_the_offending_token() {
    for (formula, marker) in [
        ("if ok then 1 2", "2"),
        ("let item = 1 2", "2"),
        ("sequence[1,]", "]"),
        ("root[]", "]"),
        ("x^2^3", "^3"),
        ("Payload { high: 9, }", "}"),
    ] {
        let source = expression(formula);
        let start = source.find(formula).unwrap() + formula.rfind(marker).unwrap();
        refused(
            &source,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            Span {
                start,
                end: start + 1,
            },
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-7")]
fn trailing_words_and_punctuation_have_distinct_located_refusals() {
    let declaration = "function valid using v(): Boolean pure { true }";
    for (tail, code, cause) in [
        (
            "unselected",
            CompleteCode::UnsupportedConstruct,
            CompleteCause::DeclarationForm,
        ),
        (
            "}",
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
        ),
    ] {
        let source = unit(&format!("{declaration} {tail}"));
        let start = source.rfind(tail).unwrap();
        refused(
            &source,
            code,
            cause,
            Span {
                start,
                end: start + tail.len(),
            },
        );
    }
}
