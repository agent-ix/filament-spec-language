// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable value, dimension, graph, hybrid and implementation-binding forms.

use fsl_cst::{CompleteCause, CompleteCode, CstElement, CstNode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(body: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{body}")
}

fn expression(body: &str) -> String {
    unit(&format!("function inspect(): Integer pure {{ {body} }}"))
}

fn parse(source: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:production-value-forms"),
        "production-value-forms.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("authored UTF-8 source")
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

fn refused(source: &str, marker: &str, width: usize) {
    let parsed = parse(source);
    assert!(!parsed.is_admissible(), "{source}");
    let diagnostic = parsed.diagnostics().first().expect("located refusal");
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken),
        "{source}: {diagnostic:?}"
    );
    let start = source.rfind(marker).expect("authored offending token");
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start,
            end: start + width
        }),
        "{source}: {diagnostic:?}"
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn dimension_products_quotients_and_signed_exponents_keep_their_operands() {
    for (declaration, terms) in [
        ("dimension Length;", vec![]),
        ("dimension Speed = Length / Time;", vec!["Length", "Time"]),
        (
            "dimension Acceleration = Length * Time^-2 / Mass^3;",
            vec!["Length", "Time^-2", "Mass^3"],
        ),
    ] {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let node = one(
            &source,
            &parsed,
            Production::DimensionDeclaration,
            declaration,
        );
        assert_eq!(
            children(&source, &parsed, node, Production::DimensionTerm),
            terms
        );
    }
    let source = unit("type Force = Quantity<kg * m * s^-2, Integer>;");
    let parsed = accepted(&source);
    let units = one(
        &source,
        &parsed,
        Production::UnitExpression,
        "kg * m * s^-2",
    );
    assert_eq!(
        children(&source, &parsed, units, Production::DimensionTerm),
        ["kg", "m", "s^-2"]
    );
    let inverse = one(&source, &parsed, Production::DimensionTerm, "s^-2");
    assert_eq!(
        children(&source, &parsed, inverse, Production::SignedInteger),
        ["-2"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn dimension_equivalence_and_unit_offsets_keep_each_exact_ratio_separate() {
    let declaration = "dimension R::Time ~ F::Time by 1 R::s = rational(1000, 1) F::ms;";
    let source = unit(declaration);
    let parsed = accepted(&source);
    let equivalence = one(
        &source,
        &parsed,
        Production::DimensionEquivalence,
        declaration,
    );
    assert_eq!(
        children(&source, &parsed, equivalence, Production::QualifiedName),
        ["R::Time", "F::Time"]
    );
    assert_eq!(
        children(&source, &parsed, equivalence, Production::RatioTerm),
        ["1 R::s", "rational(1000, 1) F::ms"]
    );
    for suffix in [
        "",
        " * Kelvin",
        " + decimal(27315, 2)",
        " * Kelvin + decimal(27315, 2)",
    ] {
        let declaration = format!("unit Celsius: Temperature = rational(1, 1){suffix};");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let node = one(&source, &parsed, Production::UnitDeclaration, &declaration);
        let mut exact = vec!["rational(1, 1)"];
        if suffix.contains('+') {
            exact.push("decimal(27315, 2)");
        }
        assert_eq!(
            children(&source, &parsed, node, Production::ExactNumber),
            exact
        );
        let names = if suffix.contains('*') {
            vec!["Temperature", "Kelvin"]
        } else {
            vec!["Temperature"]
        };
        assert_eq!(
            children(&source, &parsed, node, Production::QualifiedName),
            names
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn missing_dimension_and_unit_operands_refuse_at_the_unfilled_position() {
    for (declaration, marker, width) in [
        ("dimension Rate = Length / ;", ";", 1),
        ("dimension Rate = Time^;", ";", 1),
        ("dimension A ~ B by 1 a = ;", ";", 1),
        ("unit scale: Length = rational(1, 1) * ;", ";", 1),
        ("unit scale: Length = rational(1, 1) + ;", ";", 1),
        ("type Rate = Quantity<m * >;", ">", 1),
    ] {
        refused(&unit(declaration), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn scalar_builtins_and_qualified_calls_keep_argument_and_target_nodes() {
    for value in ["true", "false", "null", "none", "absent", "self", "result"] {
        let source = expression(value);
        let parsed = accepted(&source);
        one(&source, &parsed, Production::Primary, value);
    }
    for name in ["present", "value", "deref", "ref", "pre"] {
        let value = format!("{name}(input.member)");
        let source = expression(&value);
        let parsed = accepted(&source);
        let primary = one(&source, &parsed, Production::Primary, &value);
        assert_eq!(
            children(&source, &parsed, primary, Production::Expression),
            ["input.member"]
        );
    }
    for (value, arguments) in [("Text()", vec![]), ("Text(65, 66)", vec!["65", "66"])] {
        let source = expression(value);
        let parsed = accepted(&source);
        let primary = one(&source, &parsed, Production::Primary, value);
        assert_eq!(
            children(&source, &parsed, primary, Production::Expression),
            arguments
        );
    }
    let source = expression("Pair(9, 4)");
    let parsed = accepted(&source);
    let tuple = one(&source, &parsed, Production::TupleValue, "Pair(9, 4)");
    assert_eq!(
        children(&source, &parsed, tuple, Production::Expression),
        ["9", "4"]
    );
    assert_eq!(
        children(&source, &parsed, tuple, Production::QualifiedName),
        ["Pair"]
    );
    let source = expression("protocol Store::Transfer");
    let parsed = accepted(&source);
    let primary = one(
        &source,
        &parsed,
        Production::Primary,
        "protocol Store::Transfer",
    );
    assert_eq!(
        children(&source, &parsed, primary, Production::QualifiedName),
        ["Store::Transfer"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn graph_navigation_queries_preserve_receiver_target_and_relation_argument_roles() {
    for name in ["contains", "removeOne"] {
        let formula = format!("{name}(values, selected)");
        let source = expression(&formula);
        let parsed = accepted(&source);
        let primary = one(&source, &parsed, Production::Primary, &formula);
        assert_eq!(
            children(&source, &parsed, primary, Production::Expression),
            ["values", "selected"]
        );
    }
    let source = expression("reaches(start, goal, Store::parent)");
    let parsed = accepted(&source);
    let primary = one(
        &source,
        &parsed,
        Production::Primary,
        "reaches(start, goal, Store::parent)",
    );
    assert_eq!(
        children(&source, &parsed, primary, Production::Expression),
        ["start", "goal"]
    );
    assert_eq!(
        children(&source, &parsed, primary, Production::QualifiedName),
        ["Store::parent"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn boolean_and_comparison_precedence_preserves_ordered_operands_and_unary_binding() {
    for operator in ["=", "!=", "<", "<=", ">", ">="] {
        let formula = format!("a + b {operator} c - d");
        let source = expression(&formula);
        let parsed = accepted(&source);
        let comparison = one(&source, &parsed, Production::Comparison, &formula);
        assert_eq!(
            children(&source, &parsed, comparison, Production::Sum),
            ["a + b", "c - d"]
        );
    }
    let formula = "not ready or enabled and -score > minimum";
    let source = expression(formula);
    let parsed = accepted(&source);
    let disjunction = one(&source, &parsed, Production::Disjunction, formula);
    assert_eq!(
        children(&source, &parsed, disjunction, Production::Conjunction),
        ["not ready", "enabled and -score > minimum"]
    );
    let conjunction = one(
        &source,
        &parsed,
        Production::Conjunction,
        "enabled and -score > minimum",
    );
    assert_eq!(
        children(&source, &parsed, conjunction, Production::Comparison),
        ["enabled", "-score > minimum"]
    );
    let negation = one(&source, &parsed, Production::Unary, "not ready");
    assert_eq!(
        children(&source, &parsed, negation, Production::Unary),
        ["ready"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn case_scrutinee_specialized_precedence_and_arm_binders_preserve_structure() {
    let scrutinee = "a implies b or c and d < e + f * -g";
    let formula =
        format!("case {scrutinee} {{ Item::Both(left, right): left + right; Item::Empty: 0; }}");
    let source = expression(&formula);
    let parsed = accepted(&source);
    let implication = one(
        &source,
        &parsed,
        Production::ScrutineeImplication,
        scrutinee,
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::ScrutineeDisjunction
        ),
        ["a"]
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::ScrutineeImplication
        ),
        ["b or c and d < e + f * -g"]
    );
    for (kind, text, child_kind, expected) in [
        (
            Production::ScrutineeDisjunction,
            "b or c and d < e + f * -g",
            Production::ScrutineeConjunction,
            vec!["b", "c and d < e + f * -g"],
        ),
        (
            Production::ScrutineeConjunction,
            "c and d < e + f * -g",
            Production::ScrutineeComparison,
            vec!["c", "d < e + f * -g"],
        ),
        (
            Production::ScrutineeComparison,
            "d < e + f * -g",
            Production::ScrutineeSum,
            vec!["d", "e + f * -g"],
        ),
        (
            Production::ScrutineeSum,
            "e + f * -g",
            Production::ScrutineeProduct,
            vec!["e", "f * -g"],
        ),
        (
            Production::ScrutineeProduct,
            "f * -g",
            Production::ScrutineeUnary,
            vec!["f", "-g"],
        ),
    ] {
        let node = one(&source, &parsed, kind, text);
        assert_eq!(children(&source, &parsed, node, child_kind), expected);
    }
    let arm = one(
        &source,
        &parsed,
        Production::CaseArm,
        "Item::Both(left, right): left + right;",
    );
    assert_eq!(
        children(&source, &parsed, arm, Production::CaseBinder),
        ["left", "right"]
    );
    let source = expression("case (Payload { key: 7 }) { Item::One(x): x; }");
    let parsed = accepted(&source);
    one(
        &source,
        &parsed,
        Production::RecordValue,
        "Payload { key: 7 }",
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_builtin_arities_and_case_binders_refuse_at_the_incomplete_slot() {
    for (formula, marker, width) in [
        ("present()", ")", 1),
        ("reaches(start, goal, )", ")", 1),
        ("contains(values,)", ")", 1),
        ("case input { Item::One(): 1; }", "): 1", 1),
        ("case input { Item::Both(a,): a; }", "): a", 1),
        ("a < b < c", "< c", 1),
    ] {
        refused(&expression(formula), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn reference_literal_separators_and_nonstring_keys_refuse_at_the_exact_token() {
    for (formula, marker, width) in [
        ("Store::Row #\"r\"", "#", 1),
        ("Store::Row //gap\n#\"r\"", "#", 1),
        ("Store::Row#1", "1", 1),
        ("Store::Row#\"a\"#\"b\"", "#\"b\"", 1),
    ] {
        refused(&expression(formula), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn execution_relations_keep_two_and_repeated_named_execution_bindings() {
    for bindings in [
        vec!["left: Plant::Run", "right: Plant::Run"],
        vec!["left: Plant::Run", "right: Plant::Run", "third: Other::Run"],
    ] {
        let declaration = format!(
            "relation Same using v over ({}) {{ left = right }}",
            bindings.join(", ")
        );
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let relation = one(&source, &parsed, Production::RelationClause, &declaration);
        assert_eq!(
            children(&source, &parsed, relation, Production::ExecutionBinding),
            bindings
        );
        assert_eq!(
            children(&source, &parsed, relation, Production::Block),
            ["{ left = right }"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn graph_claim_options_keep_bound_subjects_sources_destinations_and_fairness() {
    for over in ["", " over (s: Plant::State)"] {
        for (declaration, predicates) in [
            (
                format!("possible Reach using v{over} {{ s.ready }}"),
                vec![],
            ),
            (
                format!("always possible Return using v{over} from (s.ready) {{ s.done }}"),
                vec!["s.ready"],
            ),
            (
                format!("unique path Only using v{over} from (s.ready) to (s.done);"),
                vec!["s.ready", "s.done"],
            ),
        ] {
            let source = unit(&declaration);
            let parsed = accepted(&source);
            let claim = one(&source, &parsed, Production::StateGraphClaim, &declaration);
            assert_eq!(
                children(&source, &parsed, claim, Production::StateGraphPredicate),
                predicates
            );
            assert_eq!(
                children(&source, &parsed, claim, Production::Parameter),
                if over.is_empty() {
                    vec![]
                } else {
                    vec!["s: Plant::State"]
                }
            );
        }
    }
    let declaration = "possible Reach using v { fair weak each Plant::tick; fair strong whole Plant::send; s.ready }";
    let source = unit(declaration);
    let parsed = accepted(&source);
    let body = one(
        &source,
        &parsed,
        Production::StateGraphBody,
        "{ fair weak each Plant::tick; fair strong whole Plant::send; s.ready }",
    );
    assert_eq!(
        children(&source, &parsed, body, Production::Fairness),
        [
            "fair weak each Plant::tick;",
            "fair strong whole Plant::send;"
        ]
    );
    assert_eq!(
        children(&source, &parsed, body, Production::StateGraphPredicate),
        ["s.ready"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn graph_predicate_bindings_conditionals_and_temporal_operators_keep_their_tree() {
    let predicate =
        "let saved = s.ready in if saved then eventually[0, 2] s.done else not s.failed";
    let source = unit(&format!("possible Reach using v {{ {predicate} }}"));
    let parsed = accepted(&source);
    let binding = one(&source, &parsed, Production::StateGraphPredicate, predicate);
    assert_eq!(
        children(&source, &parsed, binding, Production::Expression),
        ["s.ready"]
    );
    assert_eq!(
        children(&source, &parsed, binding, Production::StateGraphPredicate),
        ["if saved then eventually[0, 2] s.done else not s.failed"]
    );
    let conditional = one(
        &source,
        &parsed,
        Production::StateGraphPredicate,
        "if saved then eventually[0, 2] s.done else not s.failed",
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            conditional,
            Production::StateGraphPredicate
        ),
        ["eventually[0, 2] s.done", "not s.failed"]
    );
    for operator in ["until", "release", "since", "triggered"] {
        let predicate = format!("s.ready {operator}[0, 3] s.done");
        let source = unit(&format!("possible Reach using v {{ {predicate} }}"));
        let parsed = accepted(&source);
        let relation = one(&source, &parsed, Production::StateGraphRelation, &predicate);
        assert_eq!(
            children(&source, &parsed, relation, Production::StateGraphUnary),
            ["s.ready", "s.done"]
        );
        assert_eq!(
            children(&source, &parsed, relation, Production::Interval),
            ["[0, 3]"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn hybrid_modes_keep_initial_guards_ordered_equations_and_reset_transitions() {
    for initial in [
        "",
        "initial { mode Rest when { x = 0 }; mode Moving when { x > 0 }; }",
    ] {
        let rest = "mode Rest invariant { x >= 0 } flow { x' = 0; }";
        let moving = "mode Moving invariant { x <= 10 } flow { x' = speed; speed' = -1; } transition to Rest when { x = 10 } reset { x = 0 };";
        let declaration = format!("hybrid Motion using v {{ {initial} {rest} {moving} }}");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let hybrid = one(
            &source,
            &parsed,
            Production::HybridDeclaration,
            &declaration,
        );
        assert_eq!(
            children(&source, &parsed, hybrid, Production::HybridMode),
            [rest, moving]
        );
        assert_eq!(
            children(&source, &parsed, hybrid, Production::HybridInitial),
            if initial.is_empty() {
                vec![]
            } else {
                vec![initial]
            }
        );
        if !initial.is_empty() {
            let initial = one(&source, &parsed, Production::HybridInitial, initial);
            assert_eq!(
                children(&source, &parsed, initial, Production::InitialEntry),
                ["mode Rest when { x = 0 };", "mode Moving when { x > 0 };"]
            );
        }
        let mode = one(&source, &parsed, Production::HybridMode, moving);
        assert_eq!(
            children(&source, &parsed, mode, Production::Equation),
            ["x' = speed;", "speed' = -1;"]
        );
        assert_eq!(
            children(&source, &parsed, mode, Production::Block),
            ["{ x <= 10 }", "{ x = 10 }", "{ x = 0 }"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn abstraction_bindings_keep_rust_paths_field_mapping_and_parameter_order() {
    let object = "type Plant::State to \"crate::State\" { ready to \"ready_flag\"; count to \"count_value\"; };";
    let frame = "operation Plant::State::advance to \"crate::advance\" receiver \"state\" (amount to \"delta\", enabled to \"permit\");";
    let declaration = format!("abstraction Native using v {{ {object} type Plant::Empty to \"crate::Empty\"; population Plant::states to \"crate::states\"; {frame} operation Plant::State::clear to \"crate::clear\" receiver \"state\" (); }}");
    let source = unit(&declaration);
    let parsed = accepted(&source);
    let object = one(&source, &parsed, Production::ObjectBinding, object);
    assert_eq!(
        children(&source, &parsed, object, Production::FieldBinding),
        ["ready to \"ready_flag\";", "count to \"count_value\";"]
    );
    let frame = one(&source, &parsed, Production::FrameBinding, frame);
    assert_eq!(
        children(&source, &parsed, frame, Production::ParameterBinding),
        ["amount to \"delta\"", "enabled to \"permit\""]
    );
    assert_eq!(
        children(&source, &parsed, frame, Production::OperationName),
        ["Plant::State::advance"]
    );
    one(
        &source,
        &parsed,
        Production::PopulationBinding,
        "population Plant::states to \"crate::states\";",
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn task_sets_preserve_signed_timing_fields_criticality_and_claim_routes() {
    let empty = unit("taskset Empty {}");
    let parsed = accepted(&empty);
    let taskset = one(
        &empty,
        &parsed,
        Production::TasksetDeclaration,
        "taskset Empty {}",
    );
    assert!(children(&empty, &parsed, taskset, Production::Task).is_empty());
    for (task, quantities) in [
        ("task Basic periodic 10 ms deadline 8 ms wcet 2 ms lo;", vec!["10 ms", "8 ms", "2 ms"]),
        ("task Full sporadic 20 ms deadline 18 ms bcet 1 ms wcet 3 ms priority 2 jitter -1 ms blocking 2 ms hi wcet_hi 5 ms;", vec!["20 ms", "18 ms", "1 ms", "3 ms", "-1 ms", "2 ms", "5 ms"]),
        ("task Low periodic 10 ms deadline 9 ms wcet 3 ms lo wcet_hi 4 ms;", vec!["10 ms", "9 ms", "3 ms", "4 ms"]),
    ] {
        let source = unit(&format!("taskset Tasks {{ {task} }}"));
        let parsed = accepted(&source);
        let task = one(&source, &parsed, Production::Task, task);
        assert_eq!(children(&source, &parsed, task, Production::SignedQuantity), quantities);
    }
    for policy in ["fixed_priority", "edf", "amc"] {
        let declaration = format!("schedulable Feasible on Tasks under {policy};");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        one(
            &source,
            &parsed,
            Production::SchedulableDeclaration,
            &declaration,
        );
    }
    for (declaration, kind) in [
        (
            "response Bound on Tasks.Full <= 7 ms under fixed_priority;",
            Production::ResponseDeclaration,
        ),
        (
            "release Tasks.Full on Plant::tick;",
            Production::ReleaseDeclaration,
        ),
    ] {
        let source = unit(declaration);
        let parsed = accepted(&source);
        let node = one(&source, &parsed, kind, declaration);
        if kind == Production::ResponseDeclaration {
            assert_eq!(
                children(&source, &parsed, node, Production::QuantityLiteral),
                ["7 ms"]
            );
        } else {
            assert_eq!(
                children(&source, &parsed, node, Production::QualifiedName),
                ["Plant::tick"]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn missing_graph_hybrid_abstraction_and_task_members_refuse_locally() {
    for (declaration, marker, width) in [
        (
            "relation Same using v over (left: Plant::Run) { true }",
            ") {",
            1,
        ),
        ("unique path Only using v from (true) to ();", ");", 1),
        ("hybrid Motion using v {}", "}", 1),
        (
            "hybrid Motion using v { mode Rest invariant { true } flow {} }",
            "} }",
            1,
        ),
        ("abstraction Native using v {}", "}", 1),
        (
            "abstraction Native using v { type Plant::State to \"State\" {}; }",
            "}; }",
            1,
        ),
        (
            "taskset Tasks { task Full periodic 10 ms deadline 9 ms wcet 2 ms hi; }",
            "; }",
            1,
        ),
        ("response Bound on Tasks.Full <= 7 ms under edf;", "edf", 3),
    ] {
        refused(&unit(declaration), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn guarantee_premises_keep_initial_progress_and_environment_members() {
    let declaration = "guarantee Runs of Progress using Native { initial to \"crate::initial\"; progress Plant::tick decreases \"crate::rank\"; progress Plant::stop bound 5; environment Fair by \"operator\" fair weak whole Plant::tick; environment Safe by \"hardware\" assumes Plant::Valid; }";
    let source = unit(declaration);
    let parsed = accepted(&source);
    let guarantee = one(
        &source,
        &parsed,
        Production::GuaranteeDeclaration,
        declaration,
    );
    assert_eq!(
        children(&source, &parsed, guarantee, Production::GuaranteeInitial),
        ["initial to \"crate::initial\";"]
    );
    assert_eq!(
        children(&source, &parsed, guarantee, Production::GuaranteeProgress),
        [
            "progress Plant::tick decreases \"crate::rank\";",
            "progress Plant::stop bound 5;"
        ]
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            guarantee,
            Production::GuaranteeEnvironment
        ),
        [
            "environment Fair by \"operator\" fair weak whole Plant::tick;",
            "environment Safe by \"hardware\" assumes Plant::Valid;"
        ]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn component_compositions_keep_rule_alternatives_and_each_premise_kind() {
    let members = "component First from P using Native; component Second from Q using Other; component Third from R using Last; guarantee First::Runs; rely First::Ready on Second::Safe; depends First::Live on Third::Progress; supplies fair strong each Plant::tick; environment Fair by \"operator\" assumes Plant::Valid;";
    for rule in [
        "assume-guarantee",
        "protocol-refinement",
        "refinement-transfer",
        "interface",
    ] {
        let declaration = format!("compose Combined rule {rule} {{ {members} }}");
        let source = unit(&declaration);
        let parsed = accepted(&source);
        let composition = one(
            &source,
            &parsed,
            Production::CompositionDeclaration,
            &declaration,
        );
        assert_eq!(
            children(
                &source,
                &parsed,
                composition,
                Production::CompositionComponent
            ),
            [
                "component First from P using Native;",
                "component Second from Q using Other;",
                "component Third from R using Last;"
            ]
        );
        for (kind, text) in [
            (Production::CompositionGuarantee, "guarantee First::Runs;"),
            (
                Production::CompositionRely,
                "rely First::Ready on Second::Safe;",
            ),
            (
                Production::CompositionDepends,
                "depends First::Live on Third::Progress;",
            ),
            (
                Production::CompositionSupplies,
                "supplies fair strong each Plant::tick;",
            ),
        ] {
            one(&source, &parsed, kind, text);
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn missing_guarantee_initial_and_second_component_refuse_at_the_member_boundary() {
    for (declaration, marker, width) in [
        ("guarantee Runs of Progress using Native {}", "}", 1),
        ("guarantee Runs of Progress using Native { initial to \"init\"; progress Plant::tick bound ; }", "; }", 1),
        ("compose Combined rule interface { component First from P using Native; }", "}", 1),
    ] { refused(&unit(declaration), marker, width); }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
#[ignore = "QSL-2620: qualified empty calls are refused before primary construction"]
fn qualified_zero_argument_calls_keep_target_and_empty_arguments() {
    let source = expression("Store::construct()");
    let parsed = accepted(&source);
    let primary = one(&source, &parsed, Production::Primary, "Store::construct()");
    assert_eq!(
        children(&source, &parsed, primary, Production::QualifiedName),
        ["Store::construct"]
    );
    assert!(children(&source, &parsed, primary, Production::Expression).is_empty());
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
#[ignore = "QSL-2621: a gap after reference # reports a zero-width key locus"]
fn separated_reference_keys_refuse_at_the_complete_string_token() {
    for value in ["Store::Row# \"r\"", "Store::Row#//gap\n\"r\""] {
        refused(&expression(value), "\"r\"", 3);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn clock_types_and_unannotated_let_bindings_keep_their_declared_contexts() {
    let source =
        unit("type Timer = Clock; function inspect(): Integer pure { let saved = 7 in saved }");
    let parsed = accepted(&source);
    let alias = one(
        &source,
        &parsed,
        Production::AliasDeclaration,
        "type Timer = Clock;",
    );
    assert_eq!(
        children(&source, &parsed, alias, Production::TypeReference),
        ["Clock"]
    );
    let binding = one(
        &source,
        &parsed,
        Production::Expression,
        "let saved = 7 in saved",
    );
    assert!(children(&source, &parsed, binding, Production::TypeReference).is_empty());
    assert_eq!(
        children(&source, &parsed, binding, Production::Expression),
        ["7", "saved"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn graph_boolean_and_unbounded_unary_forms_keep_their_precedence() {
    let predicate = "eventually a or b and not c implies d";
    let declaration = format!("always possible Reach using v {{ {predicate} }}");
    let source = unit(&declaration);
    let parsed = accepted(&source);
    let claim = one(&source, &parsed, Production::StateGraphClaim, &declaration);
    assert!(children(&source, &parsed, claim, Production::StateGraphPredicate).is_empty());
    let implication = one(
        &source,
        &parsed,
        Production::StateGraphImplication,
        predicate,
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::StateGraphDisjunction
        ),
        ["eventually a or b and not c"]
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            implication,
            Production::StateGraphImplication
        ),
        ["d"]
    );
    let disjunction = one(
        &source,
        &parsed,
        Production::StateGraphDisjunction,
        "eventually a or b and not c",
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            disjunction,
            Production::StateGraphConjunction
        ),
        ["eventually a", "b and not c"]
    );
    for operator in ["eventually", "always", "once", "historically"] {
        let predicate = format!("{operator} (s.ready or s.done)");
        let source = unit(&format!("possible Reach using v {{ {predicate} }}"));
        let parsed = accepted(&source);
        let unary = one(&source, &parsed, Production::StateGraphUnary, &predicate);
        assert_eq!(
            children(&source, &parsed, unary, Production::StateGraphUnary),
            ["(s.ready or s.done)"]
        );
        assert!(children(&source, &parsed, unary, Production::Interval).is_empty());
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn minimal_hybrid_guarantee_and_composition_bodies_keep_only_authored_members() {
    let mode = "mode Rest invariant { true } flow { x' = 0; }";
    let declaration = format!("hybrid Motion using v {{ {mode} }}");
    let source = unit(&declaration);
    let parsed = accepted(&source);
    let hybrid = one(
        &source,
        &parsed,
        Production::HybridDeclaration,
        &declaration,
    );
    assert_eq!(
        children(&source, &parsed, hybrid, Production::HybridMode),
        [mode]
    );
    let mode = one(&source, &parsed, Production::HybridMode, mode);
    assert_eq!(
        children(&source, &parsed, mode, Production::Block),
        ["{ true }"]
    );
    let declaration = "guarantee Runs of Progress using Native { initial to \"crate::initial\"; }";
    let source = unit(declaration);
    let parsed = accepted(&source);
    let guarantee = one(
        &source,
        &parsed,
        Production::GuaranteeDeclaration,
        declaration,
    );
    assert_eq!(
        children(&source, &parsed, guarantee, Production::GuaranteeInitial),
        ["initial to \"crate::initial\";"]
    );
    assert!(children(&source, &parsed, guarantee, Production::GuaranteeProgress).is_empty());
    assert!(children(
        &source,
        &parsed,
        guarantee,
        Production::GuaranteeEnvironment
    )
    .is_empty());
    let declaration = "compose Combined rule interface { component First from P using Native; component Second from Q using Other; }";
    let source = unit(declaration);
    let parsed = accepted(&source);
    let composition = one(
        &source,
        &parsed,
        Production::CompositionDeclaration,
        declaration,
    );
    assert_eq!(
        children(
            &source,
            &parsed,
            composition,
            Production::CompositionComponent
        ),
        [
            "component First from P using Native;",
            "component Second from Q using Other;"
        ]
    );
    assert!(children(
        &source,
        &parsed,
        composition,
        Production::CompositionGuarantee
    )
    .is_empty());
}

fn temporal(body: &str) -> String {
    unit(&format!(
        "temporal Window using v over (s: Plant::State) on origin {{ {body} }}"
    ))
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn temporal_aggregate_kinds_keep_projection_filter_and_window_members() {
    for (kind, kind_expressions) in [
        ("count", vec![]),
        ("sum", vec![]),
        ("min", vec![]),
        ("max", vec![]),
        ("fraction holds(s.ready)", vec!["s.ready"]),
        ("quantile rational(9, 10)", vec![]),
    ] {
        for options in [
            "",
            " of s.value",
            " where holds(s.ready)",
            " of s.value where holds(s.ready)",
        ] {
            let term = format!("{kind}{options} over past[0, 3] min_count 2");
            let source = temporal(&format!("{term} >= 1"));
            let parsed = accepted(&source);
            let aggregate = one(&source, &parsed, Production::AggregateTerm, &term);
            let expected: Vec<&str> = match options {
                "" => vec![],
                " of s.value" => vec!["s.value"],
                " where holds(s.ready)" => vec!["s.ready"],
                _ => vec!["s.value", "s.ready"],
            };
            assert_eq!(
                children(&source, &parsed, aggregate, Production::Expression),
                expected
            );
            assert_eq!(
                children(&source, &parsed, aggregate, Production::PastWindow),
                ["[0, 3]"]
            );
            let kind_node = one(&source, &parsed, Production::AggregateKind, kind);
            assert_eq!(
                children(&source, &parsed, kind_node, Production::Expression),
                kind_expressions
            );
            if kind.starts_with("quantile") {
                assert_eq!(
                    children(&source, &parsed, kind_node, Production::ExactValue),
                    ["rational(9, 10)"]
                );
            }
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn aggregate_thresholds_and_step_or_time_windows_preserve_boolean_boundaries() {
    for window in ["[0, 0]", "[1 ms, 3 ms]"] {
        for operator in ["<", "<=", "=", ">=", ">"] {
            let atom = format!("count over past{window} {operator} s.limit + 1");
            let formula = format!("{atom} and holds(s.ready)");
            let source = temporal(&formula);
            let parsed = accepted(&source);
            let conjunction = one(&source, &parsed, Production::TemporalConjunction, &formula);
            assert_eq!(
                children(&source, &parsed, conjunction, Production::TemporalRelation),
                [atom.as_str(), "holds(s.ready)"]
            );
            let aggregate = one(&source, &parsed, Production::AggregateAtom, &atom);
            assert_eq!(
                children(&source, &parsed, aggregate, Production::Sum),
                ["s.limit + 1"]
            );
            let window_node = one(&source, &parsed, Production::PastWindow, window);
            assert_eq!(
                children(&source, &parsed, window_node, Production::Horizon),
                if window.contains("ms") {
                    vec!["1 ms", "3 ms"]
                } else {
                    vec!["0", "0"]
                }
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_aggregates_and_unwrapped_boolean_thresholds_refuse_locally() {
    for (formula, marker, width) in [
        ("count where holds() over past[0, 3] >= 1", ") over", 1),
        ("sum of over past[0, 3] >= 1", "over past", 4),
        ("count over past[0,] >= 1", "]", 1),
        ("count over past[0, 3] min_count >= 1", ">=", 2),
        ("count over past[0, 3] >= s.limit and s.ready", "s.ready", 1),
    ] {
        refused(&temporal(formula), marker, width);
    }
}
