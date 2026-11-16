// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495 / FR-062: direct mod narrows retain the consumer's claim bound and
//! conversion occurrence through request construction and replay.

use filament_checked::node::NodeKind;
use filament_exact::{Cancel, Integer, IntegerInterval, NodeKey, ValueType};
use filament_semantic_value::catalog::Category;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::{BackendId, SourceIdentity};
use fsl_replay::spine::default_accounting;
use fsl_replay::{
    replay, run_with_replay_context, Call, CallArgument, CallerLimits, CanonicalAssignment,
    DependencyInput, DisagreementCause, EvaluatedValue, InconclusiveCause, ObjectEnvironment,
    ReplayLimits, ReplayResult, TerminalRecord, TerminalValue, Value, Verdict, WitnessValue,
};
use fsl_route::request::items_from_requirements;
use fsl_route::Registry;
use fsl_semantics::check::{
    BindingValue, BodyTerm, CheckedGraph, LeafTerm, LiteralValue, MemberTerm, NodeTag,
    PackageDeclarations,
};
use ix_trace_rs::trace;
use std::collections::BTreeMap;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile d = \"filament.value.integer-division.euclidean/v1\";\n\
    function acc using v(r: Int[0, 20]): Boolean pure { true }\n\
    function accw using v(r: Integer): Boolean pure { true }\n\
    function equal using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }\n\
    function wider using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 20] pure { a mod b }\n\
    function integer using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { a mod b }\n\
    function arg using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure { acc(a mod b) }\n\
    function ifb using v(a: Int[0, 9], c: Integer): Int[0, 9] pure { if c != 0 then a mod c else 0 }\n\
    function letb using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 20] pure { let r = a mod b in r }\n\
    function argw using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure { accw(a mod b) }\n\
    function cmp using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure { a mod b >= 0 }\n\
    function bound_value using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { let r = a mod b in r }\n\
    function derived using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { (a mod b) + 1 }\n\
    function derivedb using v(a: Int[0, 100], b: Int[1, 12]): Int[1, 12] pure { (a mod b) + 1 }\n\
    function qb using v(a: Int[-100, 100], b: Int[1, 12]): Int[-100, 100] pure { a div b }\n\
    function rb using v(a: Int[-100, 100], b: Int[1, 12]): Int[-11, 11] pure { a rem b }\n\
    function qi using v(a: Int[-100, 100], b: Int[1, 12]): Integer pure { a div b }\n\
    function ri using v(a: Int[-100, 100], b: Int[1, 12]): Integer pure { a rem b }\n\
    function qlet using v(a: Int[-100, 100], b: Int[1, 12]): Int[-100, 100] pure { let r = a div b in r }\n\
    function control using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 20] pure { let r = if a > 0 then a mod b else 0 in r }\n\
    function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n\
    function big using v(n: Integer): Integer pure { n + 1 }\n\
    function ifw using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { if a > 0 then a mod b else 0 }\n\
    function letif using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { let r = if a > 0 then a mod b else 0 in r }\n\
    function letargw using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure { let r = a mod b in accw(r) }\n\
    function letargif using v(a: Int[0, 100], b: Int[1, 12]): Boolean pure { let r = if a > 0 then a mod b else 0 in accw(r) }\n";

fn interval(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("valid interval"))
}

fn graph_type(graph: &CheckedGraph, key: NodeKey) -> ValueType {
    let node = graph
        .semantic_graph()
        .node(key)
        .expect("the type node is retained in the graph");
    match (node.node_tag(), node.semantic_form()) {
        (NodeTag::ScalarType, "integer") => ValueType::Integer,
        (NodeTag::BoundedDomain, "integer_range") => {
            let base = graph
                .semantic_graph()
                .node(node.semantic_type().expect("range base"))
                .expect("integer base node");
            assert_eq!(
                (base.node_tag(), base.semantic_form()),
                (NodeTag::ScalarType, "integer")
            );
            let BodyTerm::Aggregate(body) = node.body() else {
                panic!("integer range body")
            };
            let endpoints = body
                .members
                .iter()
                .map(|member| {
                    let MemberTerm::Binding(binding) = member else {
                        panic!("range endpoint binding")
                    };
                    let BindingValue::Leaf(LeafTerm::Literal {
                        value: LiteralValue::Integer(value),
                        ..
                    }) = &binding.value
                    else {
                        panic!("integer endpoint")
                    };
                    (binding.name.as_str(), value.clone())
                })
                .collect::<BTreeMap<_, _>>();
            assert_eq!(endpoints.len(), 2);
            ValueType::Int(
                IntegerInterval::new(endpoints["min"].clone(), endpoints["max"].clone())
                    .expect("nonempty integer range"),
            )
        }
        _ => panic!("an integer bound names an integer type node"),
    }
}

#[trace(
    "TC-2210",
    "TC-2270",
    "FR-495-AC-17",
    "FR-495-AC-21",
    "FR-495-AC-22",
    "FR-062-AC-16",
    "FR-062-AC-13",
    "FR-404-AC-2"
)]
#[test]
fn modulo_claim_bound_context_and_replay_agree_for_each_consumer() {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:modulo-claim-bounds"),
        "modulo.native",
        SOURCE.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let source_ref = parsed.source().reference().clone();
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &BTreeMap::new(),
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    let graph =
        PackageDeclarations::assemble(source_ref, "fixture".to_owned(), unit, models, Vec::new())
            .expect("the unit assembles")
            .check(CheckMode::Linked, CheckingLimits::default())
            .expect("the unit checks");
    let items = items_from_requirements(graph.requirements(), &Registry::new(), None);
    for (index, expected) in [(19, interval(0, 10)), (20, ValueType::Integer)] {
        let mut pending = vec![graph.function_state(index).expect("RR function").body];
        let addition = loop {
            let node = pending.pop().expect("RR addition exists");
            if matches!(node.kind(), NodeKind::Arithmetic(..)) {
                break graph.node_key(node).expect("RR addition keyed");
            }
            pending.extend(node.children());
        };
        let (_, record) = graph
            .requirements()
            .iter()
            .find(|(key, _)| key.node() == addition)
            .expect("RR record");
        assert_eq!(record.result_bound().value_type(), &expected);
        assert_eq!(
            graph_type(&graph, record.result_bound().node()),
            expected,
            "RR bound names one type twice"
        );
    }
    for (index, function, expected_type, checked_type, expected_narrow, binder_narrow) in [
        (2, "equal", interval(0, 11), interval(0, 11), true, false),
        (3, "wider", interval(0, 20), interval(0, 11), true, false),
        (4, "integer", interval(0, 11), interval(0, 11), false, false),
        (5, "arg", interval(0, 20), interval(0, 11), true, false),
        (6, "ifb", interval(0, 9), interval(0, 9), true, false),
        (7, "letb", interval(0, 11), interval(0, 11), false, true),
        (8, "argw", interval(0, 11), interval(0, 11), false, false),
        (9, "cmp", interval(0, 11), interval(0, 11), false, false),
        (
            10,
            "bound_value",
            interval(0, 11),
            interval(0, 11),
            false,
            false,
        ),
        (
            11,
            "derived",
            interval(0, 11),
            interval(0, 11),
            false,
            false,
        ),
        (
            12,
            "derivedb",
            interval(0, 11),
            interval(0, 11),
            false,
            false,
        ),
        (
            13,
            "qb",
            interval(-100, 100),
            interval(-100, 100),
            true,
            false,
        ),
        (14, "rb", interval(-11, 11), interval(0, 11), true, false),
        (
            15,
            "qi",
            interval(-100, 100),
            interval(-100, 100),
            false,
            false,
        ),
        (16, "ri", interval(0, 11), interval(0, 11), false, false),
        (
            17,
            "qlet",
            interval(-100, 100),
            interval(-100, 100),
            false,
            true,
        ),
        (18, "control", interval(0, 11), interval(0, 11), false, true),
        (21, "ifw", interval(0, 11), interval(0, 11), false, false),
        (22, "letif", interval(0, 11), interval(0, 11), false, false),
        (
            23,
            "letargw",
            interval(0, 11),
            interval(0, 11),
            false,
            false,
        ),
        (
            24,
            "letargif",
            interval(0, 11),
            interval(0, 11),
            false,
            false,
        ),
    ] {
        let body = graph.function_state(index).expect("function checked").body;
        let mut pending = vec![body];
        let mut division_key = None;
        let mut addition_key = None;
        let mut direct_narrow = None;
        let mut narrows = 0;
        while let Some(node) = pending.pop() {
            assert!(
                !matches!(node.kind(), NodeKind::ConvertScalar(..)),
                "{function} needs no synthetic Integer conversion"
            );
            if matches!(node.kind(), NodeKind::If { .. }) && function != "ifb" {
                let lowered = graph
                    .semantic_graph()
                    .node(graph.node_key(node).expect("control keyed").clone())
                    .expect("control lowered");
                assert_eq!(
                    graph_type(&graph, lowered.semantic_type().expect("control type")),
                    ValueType::Integer,
                    "{function} control uses lowered leaf hull"
                );
            }
            if matches!(node.kind(), NodeKind::Arithmetic(..)) {
                addition_key = graph.node_key(node).cloned();
            }
            if matches!(node.kind(), NodeKind::IntegerDivision { .. }) {
                assert_eq!(
                    node.value_type(),
                    &checked_type,
                    "{function} checked interval"
                );
                let key = graph.node_key(node).expect("division keyed").clone();
                let lowered = graph
                    .semantic_graph()
                    .node(key.clone())
                    .expect("division lowered");
                let result = lowered.semantic_type().expect("application has type");
                assert_eq!(
                    graph_type(&graph, result),
                    ValueType::Integer,
                    "{function} lowered catalog type"
                );
                division_key = Some(key);
            }
            if let NodeKind::Coerce(operand, _) = node.kind() {
                let operand = node.at(*operand);
                if matches!(
                    operand.kind(),
                    NodeKind::IntegerDivision { .. } | NodeKind::Local(_)
                ) {
                    narrows += 1;
                    let narrow_key = graph.node_key(node).expect("coercion keyed").clone();
                    let lowered = graph
                        .semantic_graph()
                        .node(narrow_key.clone())
                        .expect("narrow lowered");
                    assert!(
                        matches!(lowered.body(), BodyTerm::Application(application) if application.operation.identity() == "filament.op.numeric.narrow"),
                        "{function} coercion emits a narrow"
                    );
                    if binder_narrow {
                        assert!(
                            !graph
                                .requirements()
                                .keys()
                                .any(|key| key.node() == &narrow_key),
                            "{function} binder narrow holds no record"
                        );
                    } else {
                        direct_narrow =
                            Some((narrow_key, lowered.semantic_type().expect("narrow type")));
                    }
                    assert_eq!(
                        matches!(operand.kind(), NodeKind::Local(_)),
                        binder_narrow,
                        "{function} narrow operand"
                    );
                }
            }
            pending.extend(node.children());
        }
        assert_eq!(
            narrows,
            usize::from(expected_narrow || binder_narrow),
            "{function} narrow count"
        );
        let division_key = division_key.expect("one division");
        if let Some(addition) = addition_key {
            let (_, record) = graph
                .requirements()
                .iter()
                .find(|(key, record)| key.node() == &addition && record.owner() == Some(function))
                .expect("addition record");
            let bounded = function == "derivedb";
            assert_eq!(
                record.result_bound().value_type(),
                &if bounded {
                    interval(1, 12)
                } else {
                    ValueType::Integer
                },
                "{function} addition bound"
            );
            assert_eq!(
                record.conversion_context().is_some(),
                bounded,
                "{function} addition context"
            );
        }
        let (a, second_name, b): (i64, &str, i64) = if function == "ifb" {
            (9, "c", 4)
        } else {
            (100, "b", 12)
        };
        let call = Call {
            clocks: Default::default(),
            function: function.to_owned(),
            arguments: vec![
                CallArgument {
                    parameter: "a".to_owned(),
                    value: Value::Integer(Integer::from(a)),
                },
                CallArgument {
                    parameter: second_name.to_owned(),
                    value: Value::Integer(Integer::from(b)),
                },
            ],
            accounting: default_accounting(1_000_000),
            supplied: filament_semantic_value::supplied_stop::SuppliedLimits::default(),
            backend: filament_eval::ExecutionBackendChoice::Interpreter,
        };
        let run = run_with_replay_context(
            SourceIdentity::new("agent-ix", "test:modulo-claim-bounds"),
            "fixture".to_owned(),
            "modulo.native",
            SOURCE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            CallerLimits::default(),
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("the selected function runs");
        let member = run
            .requirements()
            .members()
            .iter()
            .find(|member| member.occurrence().node() == &division_key)
            .expect("division requirement");
        let item = items
            .get(member.request_index().get())
            .expect("the requirement item is present");
        assert_eq!(item.occurrence(), member.occurrence(), "{function}");
        let record = graph
            .requirements()
            .get(member.occurrence())
            .expect("the same occurrence has a checked requirement record");
        let expected_bound_node = record.result_bound().node();
        if let Some((narrow, target)) = direct_narrow {
            assert_eq!(
                record
                    .conversion_context()
                    .expect("direct narrow context")
                    .node(),
                &narrow,
                "{function} context names its direct narrow"
            );
            assert_eq!(
                expected_bound_node, target,
                "{function} bound node is its narrow target"
            );
        }
        assert_eq!(item.result_bound(), &expected_bound_node, "{function}");
        assert_eq!(
            item.conversion_context(),
            record.conversion_context(),
            "{function} request preserves the checked conversion context"
        );
        assert_eq!(
            record.result_bound().value_type(),
            &expected_type,
            "{function} records the consumer's result bound"
        );
        assert_eq!(
            graph_type(&graph, expected_bound_node),
            expected_type,
            "{function} bound and type node agree"
        );
        assert_eq!(
            record.path_condition().len(),
            usize::from(matches!(
                function,
                "ifb" | "control" | "ifw" | "letif" | "letargif"
            )),
            "{function} guards"
        );
        assert!(
            record.path_condition().iter().all(|guard| guard.holds()),
            "{function} division runs in the true branch"
        );
        assert_eq!(
            record.conversion_context().is_some(),
            expected_narrow,
            "{function} conversion context"
        );

        let plugin = BackendId::new("acme");
        let function_node = graph
            .semantic_graph()
            .node(
                graph
                    .function_identity(function)
                    .expect("the selected function is declared"),
            )
            .expect("the function is lowered");
        let parameters = function_node
            .function_parameters()
            .expect("a function node");
        let assignments = vec![
            CanonicalAssignment {
                parameter: parameters[0].clone(),
                value: WitnessValue::Integer(i128::from(a)),
            },
            CanonicalAssignment {
                parameter: parameters[1].clone(),
                value: WitnessValue::Integer(i128::from(b)),
            },
        ];
        let wire = run
            .context()
            .request(
                member.request_index(),
                item.proof_bounds(),
                &plugin,
                assignments,
            )
            .expect("the occurrence-bound request builds");
        assert_eq!(
            wire.replay_target.occurrence_key.as_ref(),
            Some(member.occurrence())
        );
        let replayed = replay(wire, ReplayLimits::default());
        let ReplayResult::Input(result) = replayed.as_ref().expect("request replays") else {
            panic!("input replay")
        };
        assert_eq!(
            result.disagreement(),
            Some(&DisagreementCause::Verdicts {
                proved: Verdict::from_category(Category::Violation),
                replayed: Verdict::from_category(Category::Success),
            }),
            "{function} replay verdicts"
        );
        assert_eq!(
            result.value(),
            Some(EvaluatedValue::Boolean(true)),
            "{function} replay value"
        );
        let terminal =
            TerminalRecord::from_plugin_counterexample(member.request_index(), plugin, &replayed);
        assert!(
            matches!(
                terminal.value(),
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
            ),
            "{function} request replays against the same bounded result: {terminal:?}"
        );
    }
}
