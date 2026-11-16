// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495 / FR-062: direct mod narrows retain the consumer's claim bound and
//! conversion occurrence through request construction and replay.

use filament_exact::{Cancel, Integer, IntegerInterval, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::{BackendId, SourceIdentity};
use fsl_replay::spine::default_accounting;
use fsl_replay::{
    replay, run_with_replay_context, Call, CallArgument, CallerLimits, CanonicalAssignment,
    DependencyInput, InconclusiveCause, ObjectEnvironment, ReplayLimits, TerminalRecord,
    TerminalValue, Value, WitnessValue,
};
use fsl_route::request::items_from_requirements;
use fsl_route::Registry;
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;
use std::collections::BTreeMap;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function equal using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }\n\
    function wider using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 20] pure { a mod b }\n\
    function integer using v(a: Int[0, 100], b: Int[1, 12]): Integer pure { a mod b }\n";

#[trace("TC-2210", "FR-495-AC-17", "FR-062-AC-13", "FR-404-AC-2")]
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
    assert_eq!(items.len(), 3);

    for (function, expected_type, expected_narrow) in [
        (
            "equal",
            ValueType::Int(IntegerInterval::new(0_i64.into(), 11_i64.into()).unwrap()),
            true,
        ),
        (
            "wider",
            ValueType::Int(IntegerInterval::new(0_i64.into(), 20_i64.into()).unwrap()),
            true,
        ),
        ("integer", ValueType::Integer, false),
    ] {
        let call = Call {
            clocks: Default::default(),
            function: function.to_owned(),
            arguments: vec![
                CallArgument {
                    parameter: "a".to_owned(),
                    value: Value::Integer(Integer::from(100_i64)),
                },
                CallArgument {
                    parameter: "b".to_owned(),
                    value: Value::Integer(Integer::from(12_i64)),
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
        let [member] = run.requirements().members() else {
            panic!("one mod requirement for {function}");
        };
        let item = items
            .get(member.request_index().get())
            .expect("the requirement item is present");
        assert_eq!(item.occurrence(), member.occurrence(), "{function}");
        let record = graph
            .requirements()
            .get(member.occurrence())
            .expect("the same occurrence has a checked requirement record");
        let expected_bound_node = record.result_bound().node();
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
                value: WitnessValue::Integer(100),
            },
            CanonicalAssignment {
                parameter: parameters[1].clone(),
                value: WitnessValue::Integer(12),
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
