// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analysis declarations retain their grammatical roles, source order and spans.

use fsl_cst::{CompleteCause, CompleteCode, CstElement, CstNode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(declarations: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\n// λ distinguishes byte offsets\n{declarations}")
}

fn parse(source: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:analysis-production-forms"),
        "analysis-production-forms.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("authored UTF-8 and source metadata")
}

fn admitted(source: &str) -> ParsedSource {
    let parsed = parse(source);
    assert!(
        parsed.is_admissible(),
        "{source}: {:?}",
        parsed.diagnostics()
    );
    assert_eq!(parsed.cst().render(), source.as_bytes());
    parsed
}

fn text<'s>(source: &'s str, node: &CstNode) -> &'s str {
    let Span { start, end } = node.span();
    &source[start..end]
}

fn node<'p>(
    parsed: &'p ParsedSource,
    source: &str,
    kind: Production,
    expected: &str,
) -> &'p CstNode {
    let mut matches = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|n| n.production() == kind && text(source, n) == expected);
    let found = matches
        .next()
        .unwrap_or_else(|| panic!("missing {kind:?}: {expected}"));
    assert!(matches.next().is_none(), "duplicate {kind:?}: {expected}");
    assert_eq!(
        parsed.cst().render_node(found).unwrap(),
        expected.as_bytes()
    );
    found
}

fn children<'s>(
    parsed: &ParsedSource,
    source: &'s str,
    parent: &CstNode,
    kind: Production,
) -> Vec<&'s str> {
    parent
        .children()
        .iter()
        .filter_map(|element| match element {
            CstElement::Node(index) => Some(&parsed.cst().nodes()[*index]),
            CstElement::Token(_) => None,
        })
        .filter(|child| child.production() == kind)
        .map(|child| text(source, child))
        .collect()
}

fn child_kinds(parsed: &ParsedSource, parent: &CstNode) -> Vec<Production> {
    parent
        .children()
        .iter()
        .filter_map(|element| match element {
            CstElement::Node(index) => Some(parsed.cst().nodes()[*index].production()),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn declaration_span(source: &str, declaration: &str, found: &CstNode) {
    let start = source.find(declaration).expect("authored declaration");
    assert_eq!(
        found.span(),
        Span {
            start,
            end: start + declaration.len()
        }
    );
}

fn refuses_at(declaration: &str, marker: &str, width: usize) {
    let source = unit(declaration);
    let parsed = parse(&source);
    assert!(!parsed.is_admissible(), "{source}");
    let first = parsed
        .diagnostics()
        .first()
        .expect("located syntax refusal");
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

fn probability(form: &str) -> String {
    unit(&format!(
        "probabilistic Claim using p on System under policy on origin {{ {form} }}"
    ))
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn random_choices_keep_values_weights_trailing_comma_and_declaration_span() {
    let declaration = "random Model::Order::submit(amount) ~ { 0: rational(1,4), 1: 0.25, 2: decimal(5,1), 3: 0, };";
    let source = unit(declaration);
    let parsed = admitted(&source);
    let random = node(&parsed, &source, Production::RandomDeclaration, declaration);
    declaration_span(&source, declaration, random);
    assert_eq!(
        children(&parsed, &source, random, Production::QualifiedName),
        ["Model::Order::submit"]
    );
    let choices = ["0: rational(1,4)", "1: 0.25", "2: decimal(5,1)", "3: 0"];
    assert_eq!(
        children(&parsed, &source, random, Production::WeightedValue),
        choices
    );
    for (choice, value, weight) in [
        (choices[0], "0", "rational(1,4)"),
        (choices[1], "1", "0.25"),
        (choices[2], "2", "decimal(5,1)"),
        (choices[3], "3", "0"),
    ] {
        let alternative = node(&parsed, &source, Production::WeightedValue, choice);
        assert_eq!(
            child_kinds(&parsed, alternative),
            [Production::Expression, Production::ExactValue]
        );
        assert_eq!(
            children(&parsed, &source, alternative, Production::Expression),
            [value]
        );
        assert_eq!(
            children(&parsed, &source, alternative, Production::ExactValue),
            [weight]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn workload_entries_and_reward_expressions_keep_operation_names_in_order() {
    let workload = "workload Typical on System { weight Model::Order::submit = rational(3,4); weight Model::Order::retry = decimal(25,2); }";
    let reward = "reward effort on Model::Order::submit = self.total + amount;";
    let source = unit(&format!("{workload}\n{reward}"));
    let parsed = admitted(&source);
    let work = node(&parsed, &source, Production::WorkloadDeclaration, workload);
    let entries = [
        "weight Model::Order::submit = rational(3,4);",
        "weight Model::Order::retry = decimal(25,2);",
    ];
    assert_eq!(
        children(&parsed, &source, work, Production::WorkloadWeight),
        entries
    );
    for (entry, operation, weight) in [
        (entries[0], "Model::Order::submit", "rational(3,4)"),
        (entries[1], "Model::Order::retry", "decimal(25,2)"),
    ] {
        let row = node(&parsed, &source, Production::WorkloadWeight, entry);
        assert_eq!(
            children(&parsed, &source, row, Production::QualifiedName),
            [operation]
        );
        assert_eq!(
            children(&parsed, &source, row, Production::ExactValue),
            [weight]
        );
    }
    let reward = node(&parsed, &source, Production::RewardDeclaration, reward);
    assert_eq!(
        child_kinds(&parsed, reward),
        [Production::QualifiedName, Production::Expression]
    );
    assert_eq!(
        children(&parsed, &source, reward, Production::QualifiedName),
        ["Model::Order::submit"]
    );
    assert_eq!(
        children(&parsed, &source, reward, Production::Expression),
        ["self.total + amount"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn delay_distribution_alternatives_keep_quantity_bounds_and_exact_parameters() {
    for (distribution, quantities, values) in [
        ("uniform", vec![], vec![]),
        (
            "uniform[1 ms, rational(5,2) ms]",
            vec!["1 ms", "rational(5,2) ms"],
            vec![],
        ),
        ("exponential(decimal(3,2))", vec![], vec!["decimal(3,2)"]),
    ] {
        let declaration = format!("delay Model::Order::submit ~ {distribution};");
        let source = unit(&declaration);
        let parsed = admitted(&source);
        let delay = node(&parsed, &source, Production::DelayDeclaration, &declaration);
        assert_eq!(
            children(&parsed, &source, delay, Production::QualifiedName),
            ["Model::Order::submit"]
        );
        assert_eq!(
            children(&parsed, &source, delay, Production::DelayDistribution),
            [distribution]
        );
        let dist = node(
            &parsed,
            &source,
            Production::DelayDistribution,
            distribution,
        );
        assert_eq!(
            children(&parsed, &source, dist, Production::QuantityLiteral),
            quantities
        );
        assert_eq!(
            children(&parsed, &source, dist, Production::ExactValue),
            values
        );
    }
    let distribution = "discrete { 0 ms: rational(1,4), 2 ms: 0.75, }";
    let source = unit(&format!("delay Model::Order::submit ~ {distribution};"));
    let parsed = admitted(&source);
    let dist = node(
        &parsed,
        &source,
        Production::DelayDistribution,
        distribution,
    );
    assert_eq!(
        children(&parsed, &source, dist, Production::WeightedDelay),
        ["0 ms: rational(1,4)", "2 ms: 0.75"]
    );
    for (row, quantity, weight) in [
        ("0 ms: rational(1,4)", "0 ms", "rational(1,4)"),
        ("2 ms: 0.75", "2 ms", "0.75"),
    ] {
        let weighted = node(&parsed, &source, Production::WeightedDelay, row);
        assert_eq!(
            children(&parsed, &source, weighted, Production::QuantityLiteral),
            [quantity]
        );
        assert_eq!(
            children(&parsed, &source, weighted, Production::ExactValue),
            [weight]
        );
        let quantity = node(&parsed, &source, Production::QuantityLiteral, quantity);
        assert_eq!(
            children(&parsed, &source, quantity, Production::QualifiedName),
            ["ms"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_random_workload_reward_and_delay_slots_refuse_the_offending_token() {
    for (declaration, marker, width) in [
        ("random Model::pick(seed) { 0: 1 };", "{ 0", 1),
        ("random Model::pick(seed) ~ { 0: };", "};", 1),
        ("random Model::pick(seed) ~ { true 1 };", "1 };", 1),
        ("random Model::pick(seed) ~ { };", "};", 1),
        ("workload Typical on System { }", "}", 1),
        (
            "workload Typical on System { weight Model::tick = ; }",
            "; }",
            1,
        ),
        ("reward effort on Model::tick = ;", ";", 1),
        ("delay Model::tick ~ gaussian;", "gaussian", 8),
        ("delay Model::tick ~ exponential();", ");", 1),
        ("delay Model::tick ~ uniform[1 ms, ];", "];", 1),
    ] {
        refuses_at(declaration, marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn probabilistic_headers_keep_observation_scheduler_activation_and_confidence_roles() {
    let declaration = "probabilistic Reliable using p on System over (state: Model::Order) under every scheduler fair weak each Model::Order::submit fair strong Model::Order::retry on each (trigger: Model::Order) when (trigger.ready) with alpha 0.05, beta rational(1,20), indifference decimal(1,2) { probability >= 0.9 [eventually[0,3] holds(state.ready)] }";
    let source = unit(declaration);
    let parsed = admitted(&source);
    let claim = node(
        &parsed,
        &source,
        Production::ProbabilisticClause,
        declaration,
    );
    declaration_span(&source, declaration, claim);
    assert_eq!(
        child_kinds(&parsed, claim),
        [
            Production::Parameter,
            Production::SchedulerClause,
            Production::Activation,
            Production::Confidence,
            Production::ProbabilisticForm
        ]
    );
    assert_eq!(
        children(&parsed, &source, claim, Production::Parameter),
        ["state: Model::Order"]
    );
    let scheduler_text =
        "under every scheduler fair weak each Model::Order::submit fair strong Model::Order::retry";
    let scheduler = node(
        &parsed,
        &source,
        Production::SchedulerClause,
        scheduler_text,
    );
    assert_eq!(
        children(&parsed, &source, scheduler, Production::FairnessItem),
        [
            "fair weak each Model::Order::submit",
            "fair strong Model::Order::retry"
        ]
    );
    let activation = node(
        &parsed,
        &source,
        Production::Activation,
        "on each (trigger: Model::Order) when (trigger.ready)",
    );
    assert_eq!(
        children(&parsed, &source, activation, Production::Parameter),
        ["trigger: Model::Order"]
    );
    assert_eq!(
        children(&parsed, &source, activation, Production::Expression),
        ["trigger.ready"]
    );
    let confidence = node(
        &parsed,
        &source,
        Production::Confidence,
        "with alpha 0.05, beta rational(1,20), indifference decimal(1,2)",
    );
    let items = [
        "alpha 0.05",
        "beta rational(1,20)",
        "indifference decimal(1,2)",
    ];
    assert_eq!(
        children(&parsed, &source, confidence, Production::ConfidenceItem),
        items
    );
    for (item, value) in [
        (items[0], "0.05"),
        (items[1], "rational(1,20)"),
        (items[2], "decimal(1,2)"),
    ] {
        let item = node(&parsed, &source, Production::ConfidenceItem, item);
        assert_eq!(
            children(&parsed, &source, item, Production::ExactValue),
            [value]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn scheduler_and_probability_event_alternatives_keep_their_distinct_children() {
    for scheduler in ["under policy", "under every scheduler"] {
        let declaration = format!("probabilistic Bound using p on System {scheduler} on origin {{ probability <= 1 [holds(ready)] }}");
        let source = unit(&declaration);
        let parsed = admitted(&source);
        let clause = node(
            &parsed,
            &source,
            Production::ProbabilisticClause,
            &declaration,
        );
        assert_eq!(
            child_kinds(&parsed, clause),
            [
                Production::SchedulerClause,
                Production::Activation,
                Production::ProbabilisticForm
            ]
        );
        node(&parsed, &source, Production::SchedulerClause, scheduler);
        let event = node(&parsed, &source, Production::Event, "holds(ready)");
        assert_eq!(
            child_kinds(&parsed, event),
            [Production::TemporalExpression]
        );
    }
    let event_text = "steps from holds(started) until holds(done) within 7 <= 4";
    let form = format!("probability >= rational(3,4) [{event_text}]");
    let source = probability(&form);
    let parsed = admitted(&source);
    let form = node(&parsed, &source, Production::ProbabilisticForm, &form);
    assert_eq!(
        children(&parsed, &source, form, Production::ExactValue),
        ["rational(3,4)"]
    );
    let event = node(&parsed, &source, Production::Event, event_text);
    assert_eq!(
        child_kinds(&parsed, event),
        [Production::Measure, Production::Expression]
    );
    assert_eq!(
        children(&parsed, &source, event, Production::Expression),
        ["4"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn quantile_measures_keep_start_end_horizon_and_result_bound_separate() {
    for operation in ["accumulate effort", "steps", "elapsed"] {
        for horizon in ["12", "12 ms"] {
            let measure =
                format!("{operation} from holds(started) until holds(done) within {horizon}");
            let form = format!("quantile 0.95 of ({measure}) <= cap + 1");
            let source = probability(&form);
            let parsed = admitted(&source);
            let quantile = node(&parsed, &source, Production::ProbabilisticForm, &form);
            assert_eq!(
                children(&parsed, &source, quantile, Production::ExactValue),
                ["0.95"]
            );
            assert_eq!(
                children(&parsed, &source, quantile, Production::Measure),
                [measure.as_str()]
            );
            assert_eq!(
                children(&parsed, &source, quantile, Production::Expression),
                ["cap + 1"]
            );
            let measure = node(&parsed, &source, Production::Measure, &measure);
            assert_eq!(
                children(&parsed, &source, measure, Production::Expression),
                ["started", "done"]
            );
            assert_eq!(
                children(&parsed, &source, measure, Production::Horizon),
                [horizon]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn fraction_mean_and_long_run_forms_keep_optional_weighting_and_zero_origin() {
    for weight in ["", " weighted by effort", " weighted by delay"] {
        let fraction = format!("fraction holds(ready) over [0, 9 ms]{weight}");
        let form = format!("mean of ({fraction}) >= 0.5");
        let source = probability(&form);
        let parsed = admitted(&source);
        let mean = node(&parsed, &source, Production::ProbabilisticForm, &form);
        assert_eq!(
            children(&parsed, &source, mean, Production::Fraction),
            [fraction.as_str()]
        );
        assert_eq!(
            children(&parsed, &source, mean, Production::ExactValue),
            ["0.5"]
        );
        let fraction = node(&parsed, &source, Production::Fraction, &fraction);
        assert_eq!(
            children(&parsed, &source, fraction, Production::Expression),
            ["ready"]
        );
        assert_eq!(
            children(&parsed, &source, fraction, Production::Horizon),
            ["9 ms"]
        );
        assert_eq!(
            children(&parsed, &source, fraction, Production::Weighting),
            if weight.is_empty() {
                vec![]
            } else {
                vec![weight.trim()]
            }
        );
        let form = format!("long_run_fraction holds(ready){weight} <= 0.75");
        let source = probability(&form);
        let parsed = admitted(&source);
        let long_run = node(&parsed, &source, Production::ProbabilisticForm, &form);
        assert_eq!(
            children(&parsed, &source, long_run, Production::Expression),
            ["ready"]
        );
        assert_eq!(
            children(&parsed, &source, long_run, Production::ExactValue),
            ["0.75"]
        );
    }
    let fraction = "fraction holds(ready) over [0, 4]";
    let source = probability(&format!("quantile rational(1,2) of ({fraction}) <= limit"));
    let parsed = admitted(&source);
    let measure = node(&parsed, &source, Production::Measure, fraction);
    assert_eq!(child_kinds(&parsed, measure), [Production::Fraction]);
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn expected_reward_and_elapsed_bounds_preserve_full_quantity_expressions() {
    for (operation, bound) in [("accumulate effort", "cap + 2"), ("elapsed", "10 ms")] {
        let form = format!("expected {operation} until holds(done) <= {bound}");
        let source = probability(&form);
        let parsed = admitted(&source);
        let expected = node(&parsed, &source, Production::ProbabilisticForm, &form);
        assert_eq!(
            children(&parsed, &source, expected, Production::Expression),
            ["done", bound]
        );
        if operation == "elapsed" {
            let quantity = node(&parsed, &source, Production::QuantityLiteral, bound);
            assert_eq!(
                children(&parsed, &source, quantity, Production::ExactValue),
                ["10"]
            );
            assert_eq!(
                children(&parsed, &source, quantity, Production::QualifiedName),
                ["ms"]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_probability_headers_and_form_slots_report_the_token_location() {
    // `every` is contextual: without `scheduler`, it names a scheduler.
    let source = unit("probabilistic Q using p on System under every on origin { probability <= 1 [holds(true)] }");
    let parsed = admitted(&source);
    let scheduler = node(&parsed, &source, Production::SchedulerClause, "under every");
    assert!(children(&parsed, &source, scheduler, Production::FairnessItem).is_empty());

    for (declaration, marker, width) in [
        ("probabilistic Q using p on System under every scheduler fair maybe Model::tick on origin { probability <= 1 [holds(true)] }", "maybe", 5),
        ("probabilistic Q using p on System under policy on origin with gamma 0.1 { probability <= 1 [holds(true)] }", "gamma", 5),
        ("probabilistic Q using p on System under policy on origin with alpha { probability <= 1 [holds(true)] }", "{ probability", 1),
        ("probabilistic Q using p on System under policy on origin { probability < 0.5 [holds(true)] }", "< 0.5", 1),
        ("probabilistic Q using p on System under policy on origin { quantile 0.5 (steps from holds(a) until holds(b) within 3) <= 2 }", "(steps", 1),
        ("probabilistic Q using p on System under policy on origin { mean of (elapsed) <= 1 }", "elapsed)", 7),
        ("probabilistic Q using p on System under policy on origin { mean of (fraction holds(ready) over [1, 4]) <= 0.5 }", "1, 4", 1),
        ("probabilistic Q using p on System under policy on origin { expected elapsed holds(done) <= 10 ms }", "holds(done)", 5),
    ] { refuses_at(declaration, marker, width); }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn refinement_rows_preserve_header_qualification_row_order_and_projection_roles() {
    let declaration = "refinement Projection using v abstract Spec::step concrete Impl { population Spec::rows from Impl::items; object Spec::Row from Impl::Item { amount = self.cost; active = self.ready; } history Spec::Row.seen: Int[0,9] = 0 { on Impl::Item::push: seen + 1; on Impl::Item::clear: 0; } step Impl::Item::push -> Spec::Row::accept(self, _, payloadArg); assume fair weak each Impl::Item::push; ensure fair strong whole Spec::Row::accept; }";
    let source = unit(declaration);
    let parsed = admitted(&source);
    let refinement = node(&parsed, &source, Production::Refinement, declaration);
    declaration_span(&source, declaration, refinement);
    assert_eq!(
        child_kinds(&parsed, refinement),
        [
            Production::PopulationRow,
            Production::ObjectRow,
            Production::HistoryRow,
            Production::StepRow,
            Production::FairnessRow,
            Production::FairnessRow
        ]
    );
    let population = node(
        &parsed,
        &source,
        Production::PopulationRow,
        "population Spec::rows from Impl::items;",
    );
    assert_eq!(
        children(&parsed, &source, population, Production::PopulationName),
        ["Spec::rows", "Impl::items"]
    );
    let object = node(
        &parsed,
        &source,
        Production::ObjectRow,
        "object Spec::Row from Impl::Item { amount = self.cost; active = self.ready; }",
    );
    assert_eq!(
        children(&parsed, &source, object, Production::TypeName),
        ["Spec::Row", "Impl::Item"]
    );
    assert_eq!(
        children(&parsed, &source, object, Production::FieldRow),
        ["amount = self.cost;", "active = self.ready;"]
    );
    let history = node(&parsed, &source, Production::HistoryRow, "history Spec::Row.seen: Int[0,9] = 0 { on Impl::Item::push: seen + 1; on Impl::Item::clear: 0; }");
    assert_eq!(
        children(&parsed, &source, history, Production::TypeName),
        ["Spec::Row"]
    );
    assert_eq!(
        children(&parsed, &source, history, Production::TypeReference),
        ["Int[0,9]"]
    );
    assert_eq!(
        children(&parsed, &source, history, Production::HistoryLiteral),
        ["0"]
    );
    assert_eq!(
        children(&parsed, &source, history, Production::HistoryUpdate),
        ["on Impl::Item::push: seen + 1;", "on Impl::Item::clear: 0;"]
    );
    let update = node(
        &parsed,
        &source,
        Production::HistoryUpdate,
        "on Impl::Item::push: seen + 1;",
    );
    assert_eq!(
        children(&parsed, &source, update, Production::OperationName),
        ["Impl::Item::push"]
    );
    assert_eq!(
        children(&parsed, &source, update, Production::Expression),
        ["seen + 1"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn history_initializers_keep_each_literal_form_without_eating_update_blocks() {
    for initial in [
        "-3",
        "true",
        "false",
        "none",
        "\"boot\"",
        "rational(1,2)",
        "decimal(5,1)",
        "Impl::Seed",
    ] {
        let row = format!(
            "history Spec::Row.saved: Integer = {initial} {{ on Impl::Row::next: saved; }}"
        );
        let source = unit(&format!(
            "refinement State using v abstract Spec concrete Impl {{ {row} }}"
        ));
        let parsed = admitted(&source);
        let history = node(&parsed, &source, Production::HistoryRow, &row);
        assert_eq!(
            children(&parsed, &source, history, Production::HistoryLiteral),
            [initial]
        );
        assert_eq!(
            children(&parsed, &source, history, Production::HistoryUpdate),
            ["on Impl::Row::next: saved;"]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn refinement_step_sources_and_targets_keep_actions_wildcards_and_arguments() {
    for action in [
        "",
        "fork ",
        "join ",
        "cend ",
        "duplicate ",
        "lose ",
        "spawn ",
        "retire ",
        "activate ",
    ] {
        let source_text = format!("{action}Impl::Worker::advance");
        let row = format!("step {source_text} -> stutter;");
        let source = unit(&format!(
            "refinement Steps using v abstract Spec concrete Impl {{ {row} }}"
        ));
        let parsed = admitted(&source);
        let step = node(&parsed, &source, Production::StepRow, &row);
        assert_eq!(
            children(&parsed, &source, step, Production::StepSource),
            [source_text.as_str()]
        );
        assert_eq!(
            children(&parsed, &source, step, Production::StepTarget),
            ["stutter"]
        );
        let start = node(&parsed, &source, Production::StepSource, &source_text);
        assert_eq!(
            children(&parsed, &source, start, Production::QualifiedName),
            ["Impl::Worker::advance"]
        );
    }
    for target in [
        "any",
        "node Spec::Ready",
        "node Spec::Ready(_, item.amount)",
        "Spec::Row::accept(self, _, item.amount)",
    ] {
        let row = format!("step memory tso -> {target};");
        let source = unit(&format!(
            "refinement Steps using v abstract Spec concrete Impl {{ {row} }}"
        ));
        let parsed = admitted(&source);
        let step = node(&parsed, &source, Production::StepRow, &row);
        assert_eq!(
            children(&parsed, &source, step, Production::StepSource),
            ["memory tso"]
        );
        let target_node = node(&parsed, &source, Production::StepTarget, target);
        assert_eq!(
            children(&parsed, &source, target_node, Production::StepArgument),
            if target.contains('(') {
                vec!["_", "item.amount"]
            } else {
                vec![]
            }
        );
        if target.starts_with("Spec::") {
            assert_eq!(
                children(&parsed, &source, target_node, Production::OperationName),
                ["Spec::Row::accept"]
            );
            assert_eq!(
                children(&parsed, &source, target_node, Production::Expression),
                ["self"]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn refinement_fairness_rows_keep_assumption_strength_and_optional_granularity() {
    for obligation in ["assume", "ensure"] {
        for strength in ["weak", "strong"] {
            for grain in ["", "each ", "whole "] {
                let row = format!("{obligation} fair {strength} {grain}Impl::Row::advance;");
                let source = unit(&format!(
                    "refinement Fair using v abstract Spec concrete Impl {{ {row} }}"
                ));
                let parsed = admitted(&source);
                let fairness = node(&parsed, &source, Production::FairnessRow, &row);
                assert_eq!(
                    children(&parsed, &source, fairness, Production::OperationName),
                    ["Impl::Row::advance"]
                );
            }
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_refinement_rows_refuse_at_the_missing_role() {
    for (declaration, marker, width) in [
        ("refinement BrokenProjection using v abstract Spec concrete Impl { population Spec::rows Impl::rows; }", "Impl::rows", 4),
        ("refinement BrokenProjection using v abstract Spec concrete Impl { object Spec::Row from Impl::Row { amount ; } }", "; }", 1),
        ("refinement BrokenProjection using v abstract Spec concrete Impl { history Spec::Row.saved Integer = 0 { } }", "Integer =", 7),
        ("refinement BrokenProjection using v abstract Spec concrete Impl { step Impl::Row::advance -> ; }", "; }", 1),
        ("refinement BrokenProjection using v abstract Spec concrete Impl { assume fair weak each ; }", "; }", 1),
    ] { refuses_at(declaration, marker, width); }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn finite_hyper_domains_keep_bounds_and_quantifier_order() {
    let declaration = "hyper Compare using h over traces in Views::Runs bounded 6 forall trace left exists trace right { left.ready implies right.ready }";
    let source = unit(declaration);
    let parsed = admitted(&source);
    let hyper = node(&parsed, &source, Production::HyperClause, declaration);
    assert_eq!(
        child_kinds(&parsed, hyper),
        [
            Production::TraceDomain,
            Production::Quantifier,
            Production::Quantifier,
            Production::Block
        ]
    );
    let domain = node(
        &parsed,
        &source,
        Production::TraceDomain,
        "traces in Views::Runs bounded 6",
    );
    assert_eq!(
        children(&parsed, &source, domain, Production::QualifiedName),
        ["Views::Runs"]
    );
    assert_eq!(
        children(&parsed, &source, hyper, Production::Quantifier),
        ["forall trace left", "exists trace right"]
    );
    let body = node(
        &parsed,
        &source,
        Production::Block,
        "{ left.ready implies right.ready }",
    );
    assert_eq!(
        children(&parsed, &source, body, Production::Expression),
        ["left.ready implies right.ready"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn behaviour_hyper_scopes_keep_quantifier_fairness_alignment_and_step_labels() {
    let declaration = "hyper Public using h over behaviours (view: Model::State) forall trace left of Model fair weak each Model::Row::advance; exists trace right of Other fair strong whole Other::Row::advance; align skip { Model::Row::idle, Other::Row::idle } match { left.step.op = right.step.op and left.step.receiver != stutter } { always holds(view.amount @ left = view.amount @ right) }";
    let source = unit(declaration);
    let parsed = admitted(&source);
    let hyper = node(&parsed, &source, Production::HyperClause, declaration);
    declaration_span(&source, declaration, hyper);
    assert_eq!(
        child_kinds(&parsed, hyper),
        [
            Production::BehavioursDomain,
            Production::Quantifier,
            Production::Quantifier,
            Production::AlignSkip,
            Production::MatchBlock,
            Production::TemporalExpression
        ]
    );
    let domain = node(
        &parsed,
        &source,
        Production::BehavioursDomain,
        "behaviours (view: Model::State)",
    );
    assert_eq!(
        children(&parsed, &source, domain, Production::Parameter),
        ["view: Model::State"]
    );
    let quantifiers = [
        "forall trace left of Model fair weak each Model::Row::advance;",
        "exists trace right of Other fair strong whole Other::Row::advance;",
    ];
    assert_eq!(
        children(&parsed, &source, hyper, Production::Quantifier),
        quantifiers
    );
    for (quantifier, fairness) in [
        (quantifiers[0], "fair weak each Model::Row::advance;"),
        (quantifiers[1], "fair strong whole Other::Row::advance;"),
    ] {
        let q = node(&parsed, &source, Production::Quantifier, quantifier);
        assert_eq!(
            children(&parsed, &source, q, Production::Fairness),
            [fairness]
        );
    }
    let align = node(
        &parsed,
        &source,
        Production::AlignSkip,
        "align skip { Model::Row::idle, Other::Row::idle }",
    );
    assert_eq!(
        children(&parsed, &source, align, Production::QualifiedName),
        ["Model::Row::idle", "Other::Row::idle"]
    );
    let labels: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|n| n.production() == Production::StepLabel)
        .map(|n| text(&source, n))
        .collect();
    assert_eq!(
        labels,
        ["left.step.op", "right.step.op", "left.step.receiver"]
    );
    for (indexed, base) in [
        ("view.amount @ left", "view.amount"),
        ("view.amount @ right", "view.amount"),
    ] {
        let read = node(&parsed, &source, Production::IndexedRead, indexed);
        assert_eq!(
            children(&parsed, &source, read, Production::HyperPostfix),
            [base]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1")]
fn bare_behaviours_and_indexed_arithmetic_preserve_hyper_precedence() {
    let predicate = "(view.amount @ left + view.offset @ right) * 2 < view.total @ left";
    let source = unit(&format!("hyper Arithmetic using h over behaviours forall trace left forall trace right {{ always holds({predicate}) }}"));
    let parsed = admitted(&source);
    let domain = node(&parsed, &source, Production::BehavioursDomain, "behaviours");
    assert!(children(&parsed, &source, domain, Production::Parameter).is_empty());
    let comparison = node(&parsed, &source, Production::HyperComparison, predicate);
    assert_eq!(
        children(&parsed, &source, comparison, Production::HyperSum),
        [
            "(view.amount @ left + view.offset @ right) * 2",
            "view.total @ left"
        ]
    );
    let product = node(
        &parsed,
        &source,
        Production::HyperProduct,
        "(view.amount @ left + view.offset @ right) * 2",
    );
    assert_eq!(
        children(&parsed, &source, product, Production::HyperUnary),
        ["(view.amount @ left + view.offset @ right)", "2"]
    );
    let sum = node(
        &parsed,
        &source,
        Production::HyperSum,
        "view.amount @ left + view.offset @ right",
    );
    assert_eq!(
        children(&parsed, &source, sum, Production::HyperProduct),
        ["view.amount @ left", "view.offset @ right"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn malformed_hyper_domains_quantifiers_and_matches_refuse_at_the_slot() {
    for (declaration, marker, width) in [
        (
            "hyper Q using h over traces in Views::Runs bounded forall trace a { true }",
            "forall trace",
            6,
        ),
        (
            "hyper Q using h over behaviours forall trace { holds(true) }",
            "{ holds",
            1,
        ),
        (
            "hyper Q using h over behaviours forall trace a match state.ready { holds(true) }",
            "state.ready",
            5,
        ),
    ] {
        refuses_at(declaration, marker, width);
    }
}

// Pending QSL-2599: the empty align-skip syntax has no settled required oracle.
// Original input, outside active coverage until that ambiguity is resolved:
// hyper Q using h over behaviours forall trace a align skip { } { holds(true) }

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn synthesis_declarations_keep_grammar_domain_and_predicate_order() {
    for (grammar, domain, predicate) in [
        (
            "Candidates::Terms",
            "Inputs::Bounded",
            "candidate.cost <= cap",
        ),
        ("Other::Trees", "Space::Rows", "candidate.ready and enabled"),
    ] {
        let declaration = format!("synthesis Search using s grammar {grammar} domain {domain} satisfies {{ {predicate} }};");
        let source = unit(&declaration);
        let parsed = admitted(&source);
        let synthesis = node(
            &parsed,
            &source,
            Production::SynthesisDeclaration,
            &declaration,
        );
        declaration_span(&source, &declaration, synthesis);
        assert_eq!(
            child_kinds(&parsed, synthesis),
            [
                Production::QualifiedName,
                Production::QualifiedName,
                Production::Block
            ]
        );
        assert_eq!(
            children(&parsed, &source, synthesis, Production::QualifiedName),
            [grammar, domain]
        );
        let body = node(
            &parsed,
            &source,
            Production::Block,
            &format!("{{ {predicate} }}"),
        );
        assert_eq!(
            children(&parsed, &source, body, Production::Expression),
            [predicate]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
#[trace("TC-2104", "FR-754-AC-1")]
fn synthesis_objectives_preserve_direction_objective_and_predicate_roles() {
    for (direction, objective) in [
        ("minimize", "candidate.cost"),
        ("maximize", "candidate.score + bonus"),
    ] {
        let declaration = format!("synthesis Search using s grammar Candidates::Terms domain Inputs::Rows {direction} {objective} satisfies {{ candidate.ready }};");
        let source = unit(&declaration);
        let parsed = admitted(&source);
        let synthesis = node(
            &parsed,
            &source,
            Production::SynthesisDeclaration,
            &declaration,
        );
        declaration_span(&source, &declaration, synthesis);
        assert_eq!(
            children(&parsed, &source, synthesis, Production::QualifiedName),
            ["Candidates::Terms", "Inputs::Rows"]
        );
        let objective_node = node(&parsed, &source, Production::Expression, objective);
        let start = source.find(&format!("{direction} {objective}")).unwrap() + direction.len() + 1;
        assert_eq!(
            objective_node.span(),
            Span {
                start,
                end: start + objective.len()
            }
        );
        let body = node(&parsed, &source, Production::Block, "{ candidate.ready }");
        assert_eq!(
            children(&parsed, &source, synthesis, Production::Block),
            ["{ candidate.ready }"]
        );
        assert_eq!(
            children(&parsed, &source, body, Production::Expression),
            ["candidate.ready"]
        );
        assert!(objective_node.span().end < body.span().start);
    }
}

/// FR-754-AC-1: `minimize` and `maximize` are keywords only where an
/// objective may stand, so each stays usable as a name everywhere else.
#[test]
#[trace("FSpec-FR-339-AC-1")]
#[trace("TC-2104", "FR-754-AC-1")]
fn minimize_and_maximize_are_names_outside_the_objective_position() {
    let declaration = "type minimize = Int[0, 9];\n\
         synthesis maximize using s grammar minimize domain minimize \
         maximize minimize satisfies { maximize >= 3 };";
    let source = unit(declaration);
    let parsed = admitted(&source);
    node(
        &parsed,
        &source,
        Production::SynthesisDeclaration,
        declaration.split_once('\n').unwrap().1,
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-4")]
fn incomplete_synthesis_selectors_and_predicates_refuse_at_the_slot() {
    for (declaration, marker, width) in [
        ("synthesis Q using s grammar Candidates::Terms domain satisfies { true };", "satisfies {", 9),
        ("synthesis Q using s grammar Candidates::Terms domain Inputs::Rows satisfies ;", ";", 1),
        ("synthesis Q using s grammar Candidates::Terms domain Inputs::Rows satisfies { true } extra", "extra", 5),
    ] { refuses_at(declaration, marker, width); }
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn analysis_declarations_missing_final_semicolons_refuse_at_end_of_source() {
    for declaration in [
        "random Model::choose(seed) ~ { true: 1 }",
        "reward effort on Model::advance = 2",
        "delay Model::advance ~ uniform",
        "synthesis Search using s grammar Candidates::Terms domain Inputs::Rows satisfies { true }",
    ] {
        let source = unit(declaration);
        let parsed = parse(&source);
        assert!(!parsed.is_admissible(), "{source}");
        let first = parsed.diagnostics().first().expect("end-of-source refusal");
        assert_eq!(
            (first.code, first.cause),
            (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedEnd),
            "{source}: {first:?}"
        );
        assert_eq!(
            first.byte_span(),
            Some(Span {
                start: source.len(),
                end: source.len()
            }),
            "{source}: {first:?}"
        );
    }
}
