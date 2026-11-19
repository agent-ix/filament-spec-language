// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable numeric, query, object, state, time and temporal parser cases.

use fsl_cst::{
    CompleteCause, CompleteCode, CstElement, CstNode, Limits, ParsedSource, Production, TokenKind,
};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(body: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile t = \"filament.temporal.complete/v1\";\n{body}")
}

fn expression(body: &str) -> String {
    unit(&format!(
        "function inspect using v(): Integer pure {{ {body} }}"
    ))
}

fn parse(source: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:production-source-forms"),
        "production-source-forms.native",
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
    let mut nodes = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == kind && spelling(source, node) == text);
    let node = nodes
        .next()
        .unwrap_or_else(|| panic!("missing {kind:?}: {text}"));
    assert!(nodes.next().is_none(), "duplicate {kind:?}: {text}");
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
        .filter(|node| node.production() == kind)
        .map(|node| spelling(source, node))
        .collect()
}

fn refused(source: &str, marker: &str, width: usize) {
    let parsed = parse(source);
    assert!(!parsed.is_admissible(), "{source}");
    let first = parsed.diagnostics().first().expect("specified refusal");
    assert_eq!(
        (first.code, first.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
        "{source}: {first:?}"
    );
    let start = source.rfind(marker).expect("authored offending token");
    assert_eq!(
        first.byte_span(),
        Some(Span {
            start,
            end: start + width
        }),
        "{source}: {first:?}"
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn exact_number_alternatives_keep_signed_components_and_scales() {
    for (value, expected) in [
        ("rational(-12, 7)", vec!["-12", "7"]),
        ("rational(12, -7)", vec!["12", "-7"]),
        ("decimal(-25, 2)", vec!["-25"]),
        ("decimal(0, 0)", vec!["0"]),
    ] {
        let source = expression(value);
        let parsed = accepted(&source);
        let number = one(&source, &parsed, Production::ExactNumber, value);
        assert_eq!(
            children(&source, &parsed, number, Production::SignedInteger),
            expected
        );
    }
    for value in ["0", "125", "0.125", "1.0"] {
        let source = expression(value);
        let parsed = accepted(&source);
        one(&source, &parsed, Production::Primary, value);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn bit_exact_float_constructors_keep_every_digit_inside_their_width() {
    for (value, kind, bits) in [
        ("float32(bits: 0x3f800000)", Production::Hex32, "0x3f800000"),
        (
            "float64(bits: 0x3ff0000000000000)",
            Production::Hex64,
            "0x3ff0000000000000",
        ),
    ] {
        let source = expression(value);
        let parsed = accepted(&source);
        let float = one(&source, &parsed, Production::FloatValue, value);
        assert_eq!(children(&source, &parsed, float, kind), [bits]);
        let hex = one(&source, &parsed, kind, bits);
        let digits = children(&source, &parsed, hex, Production::HexDigit).concat();
        assert_eq!(digits, bits.strip_prefix("0x").unwrap());
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_numeric_constructor_arguments_refuse_at_the_unfilled_slot() {
    for (value, marker) in [
        ("rational(3,)", ")"),
        ("decimal(5,)", ")"),
        ("float32(bits: 0x1234567)", ")"),
        ("float64(bits: 0x123456789abcdef)", ")"),
    ] {
        refused(&expression(value), marker, 1);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn reference_and_escaped_text_literals_keep_original_utf8_byte_spans() {
    let value = "Store::Row#\"id//not-a-comment\"";
    let source = expression(value);
    let parsed = accepted(&source);
    let reference = one(&source, &parsed, Production::ReferenceLiteral, value);
    assert_eq!(
        children(&source, &parsed, reference, Production::QualifiedName),
        ["Store::Row"]
    );
    let literal = r#""β//kept\n\u03bb""#;
    let source = expression(literal);
    let parsed = accepted(&source);
    one(&source, &parsed, Production::Primary, literal);
    let token = parsed
        .cst()
        .tokens()
        .iter()
        .find(|token| token.spelling() == literal.as_bytes())
        .expect("one literal token");
    assert_eq!(token.kind(), TokenKind::Text);
    let start = source.find(literal).unwrap();
    assert_eq!(
        token.span(),
        Span {
            start,
            end: start + literal.len()
        }
    );
    assert_eq!(parsed.cst().render(), source.as_bytes());
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn collection_query_binders_projections_and_fold_identities_keep_their_roles() {
    for operation in ["map", "collect", "filter", "flatMap", "forall", "exists"] {
        let query = format!("{operation}(item in values: item.ready)");
        let source = expression(&query);
        let parsed = accepted(&source);
        let call = one(&source, &parsed, Production::CollectionCall, &query);
        assert_eq!(
            children(&source, &parsed, call, Production::Expression),
            ["values", "item.ready"]
        );
    }
    for operation in ["flatten", "dropFirst"] {
        let query = format!("{operation}(values)");
        let source = expression(&query);
        let parsed = accepted(&source);
        let call = one(&source, &parsed, Production::CollectionCall, &query);
        assert_eq!(
            children(&source, &parsed, call, Production::Expression),
            ["values"]
        );
    }
    for operation in ["fold", "reduce"] {
        for (suffix, expected) in [
            ("", vec!["values", "acc + item"]),
            (", identity: 0", vec!["values", "acc + item", "0"]),
        ] {
            let query = format!("{operation}<Total>(acc, item in values: acc + item{suffix})");
            let source = expression(&query);
            let parsed = accepted(&source);
            let call = one(&source, &parsed, Production::CollectionCall, &query);
            assert_eq!(
                children(&source, &parsed, call, Production::QualifiedName),
                ["Total"]
            );
            assert_eq!(
                children(&source, &parsed, call, Production::Expression),
                expected
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn population_queries_preserve_population_names_filters_and_projection_ranges() {
    for operation in ["count", "exists", "all"] {
        for filter in ["", " where row.ready"] {
            let binder = format!("row in Store::rows{filter}");
            let query = format!("{operation}({binder})");
            let source = expression(&query);
            let parsed = accepted(&source);
            let call = one(&source, &parsed, Production::PopulationCall, &query);
            assert_eq!(
                children(&source, &parsed, call, Production::PopulationBinder),
                [binder.as_str()]
            );
            let bound = one(&source, &parsed, Production::PopulationBinder, &binder);
            assert_eq!(
                children(&source, &parsed, bound, Production::PopulationName),
                ["Store::rows"]
            );
            assert_eq!(
                children(&source, &parsed, bound, Production::Expression),
                if filter.is_empty() {
                    vec![]
                } else {
                    vec!["row.ready"]
                }
            );
        }
    }
    for operation in ["sum", "only"] {
        let query = format!("{operation}(row in rows: row.amount)");
        let source = expression(&query);
        let parsed = accepted(&source);
        let call = one(&source, &parsed, Production::PopulationCall, &query);
        assert_eq!(
            children(&source, &parsed, call, Production::Expression),
            ["row.amount"]
        );
    }
    let query = "seq(i in 0 .. 3: i + 1)";
    let source = expression(query);
    let parsed = accepted(&source);
    let call = one(&source, &parsed, Production::PopulationCall, query);
    assert_eq!(
        children(&source, &parsed, call, Production::Expression),
        ["0", "3", "i + 1"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn model_lookup_absence_modes_and_typed_queries_keep_argument_boundaries() {
    for mode in ["undefined", "empty", "refused"] {
        let query = format!("lookup<Store::Row>(rows, key) absent {mode}");
        let source = expression(&query);
        let parsed = accepted(&source);
        let call = one(&source, &parsed, Production::Primary, &query);
        assert_eq!(
            children(&source, &parsed, call, Production::TypeReference),
            ["Store::Row"]
        );
        assert_eq!(
            children(&source, &parsed, call, Production::Expression),
            ["rows", "key"]
        );
    }
    for query in [
        "convert<Integer>(amount)",
        "allInstances<Store::Row>(rows)",
        "size<Total>(values)",
    ] {
        let source = expression(query);
        let parsed = accepted(&source);
        let call = one(&source, &parsed, Production::Primary, query);
        assert_eq!(
            children(&source, &parsed, call, Production::Expression),
            if query.starts_with("convert") {
                vec!["amount"]
            } else if query.starts_with("allInstances") {
                vec!["rows"]
            } else {
                vec!["values"]
            }
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_query_binders_and_lookup_modes_refuse_at_the_required_slot() {
    for (query, marker, width) in [
        ("map(item values: item)", "values", 6),
        ("fold<Total>(acc item in values: item)", "item in", 4),
        ("sum(row in rows)", ")", 1),
        ("lookup<Store::Row>(rows, key) absent bogus", "bogus", 5),
    ] {
        refused(&expression(query), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn source_objects_keep_inheritance_identity_fields_and_population_members() {
    let source = unit("object Empty {}\nabstract object Derived extends Base, Store::Root identity (key, tag) { key: Integer; tag: Text[0, 8; nfc]?; population local; symmetric population peers; }");
    let parsed = accepted(&source);
    let empty = one(
        &source,
        &parsed,
        Production::ObjectDeclaration,
        "object Empty {}",
    );
    assert!(children(&source, &parsed, empty, Production::Field).is_empty());
    let declaration = "abstract object Derived extends Base, Store::Root identity (key, tag) { key: Integer; tag: Text[0, 8; nfc]?; population local; symmetric population peers; }";
    let object = one(&source, &parsed, Production::ObjectDeclaration, declaration);
    assert_eq!(
        children(&source, &parsed, object, Production::TypeName),
        ["Base", "Store::Root"]
    );
    assert_eq!(
        children(&source, &parsed, object, Production::Field),
        ["key: Integer;", "tag: Text[0, 8; nfc]?;"]
    );
    assert_eq!(
        children(&source, &parsed, object, Production::ObjectPopulation),
        ["population local;", "symmetric population peers;"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn operation_members_keep_optional_results_redefinitions_and_each_frame_kind() {
    let simple = "operation clear();";
    let complex = "operation revise(amount: Integer, target: Reference<Row>): Boolean redefines Base::revise modifies self.key, Row.key, key creates Row, Other deletes self, target;";
    let source = unit(&format!("object Row {{ {simple} {complex} }}"));
    let parsed = accepted(&source);
    let clear = one(&source, &parsed, Production::ObjectOperation, simple);
    assert!(children(&source, &parsed, clear, Production::Parameter).is_empty());
    assert!(children(&source, &parsed, clear, Production::TypeReference).is_empty());
    let operation = one(&source, &parsed, Production::ObjectOperation, complex);
    assert_eq!(
        children(&source, &parsed, operation, Production::Parameter),
        ["amount: Integer", "target: Reference<Row>"]
    );
    assert_eq!(
        children(&source, &parsed, operation, Production::TypeReference),
        ["Boolean"]
    );
    assert_eq!(
        children(&source, &parsed, operation, Production::FrameClause),
        [
            "modifies self.key, Row.key, key",
            "creates Row, Other",
            "deletes self, target"
        ]
    );
    let fields = one(
        &source,
        &parsed,
        Production::FrameClause,
        "modifies self.key, Row.key, key",
    );
    assert_eq!(
        children(&source, &parsed, fields, Production::FieldTarget),
        ["self.key", "Row.key", "key"]
    );
    let deletions = one(
        &source,
        &parsed,
        Production::FrameClause,
        "deletes self, target",
    );
    assert_eq!(
        children(&source, &parsed, deletions, Production::DeleteTarget),
        ["self", "target"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_object_lists_and_frames_refuse_the_missing_member() {
    for (declaration, marker, width) in [
        ("object Row extends , Base {}", ", Base", 1),
        ("object Row identity () {}", ")", 1),
        (
            "object Row { operation revise(amount: Integer,) ; }",
            ")",
            1,
        ),
        ("object Row { operation revise() redefines Base; }", ";", 1),
        ("object Row { operation revise() modifies ; }", ";", 1),
        ("object Row { population ; }", ";", 1),
    ] {
        refused(&unit(declaration), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn state_clause_and_operation_body_alternatives_keep_targets_and_predicates() {
    for keyword in ["invariant", "constraint"] {
        let declaration =
            format!("{keyword} Ready using v on Store::Row at current {{ self.ready }}");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let clause = one(&source, &parsed, Production::StateClause, &declaration);
        assert_eq!(
            children(&source, &parsed, clause, Production::TypeName),
            ["Store::Row"]
        );
        assert_eq!(
            children(&source, &parsed, clause, Production::Block),
            ["{ self.ready }"]
        );
    }
    for keyword in ["pre", "post"] {
        let declaration = format!("{keyword} Ready using v on Store::Row::revise {{ true }}");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let clause = one(&source, &parsed, Production::StateClause, &declaration);
        assert_eq!(
            children(&source, &parsed, clause, Production::OperationName),
            ["Store::Row::revise"]
        );
    }
    for args in ["", "amount", "amount, target"] {
        let declaration = format!("body Revise using v on Row::revise({args}) {{ true }}");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let body = one(&source, &parsed, Production::OperationBody, &declaration);
        assert_eq!(
            children(&source, &parsed, body, Production::QualifiedName),
            ["Row::revise"]
        );
        assert_eq!(
            children(&source, &parsed, body, Production::Block),
            ["{ true }"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn population_bounds_and_terminal_alternatives_keep_their_subjects() {
    for (name, population, maximum) in [("Empty", "rows", 0), ("Small", "Store::rows", 2)] {
        let declaration = format!("bound {name} on {population}[{maximum}];");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let bound = one(&source, &parsed, Production::PopulationBound, &declaration);
        assert_eq!(
            children(&source, &parsed, bound, Production::PopulationName),
            [population]
        );
    }
    for closing in ["any", "when { true }"] {
        let declaration = format!("terminal End using v on Store {closing};");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let terminal = one(
            &source,
            &parsed,
            Production::TerminalDeclaration,
            &declaration,
        );
        assert_eq!(
            children(&source, &parsed, terminal, Production::Block),
            if closing == "any" {
                vec![]
            } else {
                vec!["{ true }"]
            }
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_state_targets_and_population_bounds_refuse_the_offending_token() {
    for (declaration, marker, width) in [
        ("invariant Ready using v on Row { true }", "{", 1),
        ("pre Ready using v on Row { true }", "{", 1),
        (
            "body Revise using v on Row::revise(amount,) { true }",
            ")",
            1,
        ),
        ("bound Small on rows[];", "]", 1),
        ("terminal End using v on Store;", ";", 1),
    ] {
        refused(&unit(declaration), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn dense_and_tick_time_declarations_and_urgency_keep_their_distinct_members() {
    for (declaration, names, quantities) in [
        ("time Wall dense unit ms;", vec!["ms"], vec![]),
        (
            "time Logical tick Plant::tick period 2 ms unit ms;",
            vec!["Plant::tick", "ms"],
            vec!["2 ms"],
        ),
    ] {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let time = one(&source, &parsed, Production::TimeDeclaration, declaration);
        assert_eq!(
            children(&source, &parsed, time, Production::QualifiedName),
            names
        );
        assert_eq!(
            children(&source, &parsed, time, Production::QuantityLiteral),
            quantities
        );
    }
    for declaration in [
        "urgent Immediate when { true };",
        "urgent Critical Plant::tick;",
    ] {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let urgent = one(&source, &parsed, Production::UrgentDeclaration, declaration);
        if declaration.contains("when") {
            assert_eq!(
                children(&source, &parsed, urgent, Production::Block),
                ["{ true }"]
            );
        } else {
            assert_eq!(
                children(&source, &parsed, urgent, Production::QualifiedName),
                ["Plant::tick"]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn clock_bound_conjunctions_keep_ordinary_and_quantified_bounds_separate() {
    for quantifier in ["forall", "exists"] {
        let quantified = format!("{quantifier}(item in rows: item.age < 2 ms)");
        let declaration = format!(
            "time invariant Limit when {{ ready }} {{ self.age <= 5 ms and {quantified} }}"
        );
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let invariant = one(&source, &parsed, Production::TimeInvariant, &declaration);
        assert_eq!(
            children(&source, &parsed, invariant, Production::Block),
            ["{ ready }"]
        );
        assert_eq!(
            children(&source, &parsed, invariant, Production::ClockBound),
            ["self.age <= 5 ms"]
        );
        assert_eq!(
            children(
                &source,
                &parsed,
                invariant,
                Production::QuantifiedClockBound
            ),
            [quantified.as_str()]
        );
        let bound = one(
            &source,
            &parsed,
            Production::QuantifiedClockBound,
            &quantified,
        );
        assert_eq!(
            children(&source, &parsed, bound, Production::Postfix),
            ["item.age"]
        );
        assert_eq!(
            children(&source, &parsed, bound, Production::QuantityLiteral),
            ["2 ms"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_time_and_clock_bound_forms_refuse_at_their_required_slot() {
    for (declaration, marker, width) in [
        ("time Wall dense;", ";", 1),
        ("time Logical tick Plant::tick period unit ms;", "unit", 4),
        ("time invariant Limit when { true } {}", "}", 1),
        (
            "time invariant Limit when { true } { self.age >= 3 ms }",
            ">=",
            2,
        ),
        ("urgent Critical;", ";", 1),
    ] {
        refused(&unit(declaration), marker, width);
    }
}

fn temporal(formula: &str) -> String {
    unit(&format!(
        "temporal Progress using t over (state: Plant::State) on origin {{ {formula} }}"
    ))
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn activation_options_and_repeated_fairness_and_captures_keep_their_scope() {
    for activation in [
        "on origin",
        "on each (trigger: Plant::Event)",
        "on each (trigger: Plant::Event) when (trigger.ready)",
    ] {
        let source = unit(&format!("temporal Progress using t over (state: Plant::State) clock \"sensor\" {activation} {{ fair Plant::tick; fair weak whole Plant::step; fair strong each Plant::send; capture saved: Boolean = state.ready; capture initial: Integer = state.count; holds(saved) }}"));
        let parsed = accepted(&source);
        let clause = parsed
            .cst()
            .nodes()
            .iter()
            .find(|node| node.production() == Production::TemporalClause)
            .expect("temporal clause");
        assert_eq!(
            children(&source, &parsed, clause, Production::Activation),
            [activation]
        );
        assert_eq!(
            children(&source, &parsed, clause, Production::Fairness),
            [
                "fair Plant::tick;",
                "fair weak whole Plant::step;",
                "fair strong each Plant::send;"
            ]
        );
        assert_eq!(
            children(&source, &parsed, clause, Production::Capture),
            [
                "capture saved: Boolean = state.ready;",
                "capture initial: Integer = state.count;"
            ]
        );
        let active = one(&source, &parsed, Production::Activation, activation);
        assert_eq!(
            children(&source, &parsed, active, Production::Expression),
            if activation.contains("when") {
                vec!["trigger.ready"]
            } else {
                vec![]
            }
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn temporal_unary_operators_keep_step_and_timed_interval_alternatives() {
    for operator in ["eventually", "always", "once", "historically"] {
        for interval in [
            "",
            "[0, 2]",
            "[0, *]",
            "[1 ms, 3 ms]",
            "(1 ms, 3 ms]",
            "[1 ms, 3 ms)",
            "(1 ms, 3 ms)",
        ] {
            let formula = format!("{operator}{interval} holds(state.ready)");
            let source = temporal(&formula);
            let parsed = accepted(&source);
            let unary = one(&source, &parsed, Production::TemporalUnary, &formula);
            assert_eq!(
                children(&source, &parsed, unary, Production::TemporalUnary),
                ["holds(state.ready)"]
            );
            if !interval.is_empty() {
                let kind = if interval.contains("ms") {
                    Production::TimedInterval
                } else {
                    Production::Interval
                };
                assert_eq!(children(&source, &parsed, unary, kind), [interval]);
            }
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn temporal_binary_and_boolean_operators_keep_their_operand_precedence() {
    for operator in ["until", "release", "since", "triggered"] {
        for interval in ["", "[0, 3]", "[1 ms, 3 ms)"] {
            let formula = format!("holds(state.ready) {operator}{interval} holds(state.done)");
            let source = temporal(&formula);
            let parsed = accepted(&source);
            let relation = one(&source, &parsed, Production::TemporalRelation, &formula);
            assert_eq!(
                children(&source, &parsed, relation, Production::TemporalUnary),
                ["holds(state.ready)", "holds(state.done)"]
            );
        }
    }
    let formula = "holds(state.ready) or holds(state.done) and not holds(state.failed) implies holds(state.safe)";
    let source = temporal(formula);
    let parsed = accepted(&source);
    let implication = one(&source, &parsed, Production::TemporalImplication, formula);
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::TemporalDisjunction
        ),
        ["holds(state.ready) or holds(state.done) and not holds(state.failed)"]
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::TemporalImplication
        ),
        ["holds(state.safe)"]
    );
    let disjunction = one(
        &source,
        &parsed,
        Production::TemporalDisjunction,
        "holds(state.ready) or holds(state.done) and not holds(state.failed)",
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            disjunction,
            Production::TemporalConjunction
        ),
        [
            "holds(state.ready)",
            "holds(state.done) and not holds(state.failed)"
        ]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_temporal_headers_guards_captures_and_intervals_refuse_locally() {
    for (declaration,marker,width) in [("temporal P using t on origin { holds(true) }","on origin",2),("temporal P using t over (s: M::S) on each () { holds(true) }",") {",1),("temporal P using t over (s: M::S) on each (x: M::S) when x.ready { holds(true) }","x.ready",1),("temporal P using t over (s: M::S) on origin { capture saved: Boolean = ; holds(true) }","; holds",1),("temporal P using t over (s: M::S) on origin { eventually[0 2] holds(true) }","2]",1)] {
        refused(&unit(declaration),marker,width);
    }
}
