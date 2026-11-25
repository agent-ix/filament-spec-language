// SPDX-License-Identifier: AGPL-3.0-or-later
//! Root-only consumer route for the FR-404 call replay facade.

use fsl_replay::{
    run_with_replay_context, BackendId, Call, CallArgument, CallValue, CallerLimits, Cancel,
    CanonicalAssignment, DependencyInput, DomainKey, DomainKind, FiniteBound, Integer, NodeKey,
    ObjectEnvironment, PathSegment, ProofBound, RequestIndex, ScalarLimits, SourceIdentity,
    SuppliedLimits, TerminalValue, Value, WitnessValue,
};
use std::collections::BTreeMap;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
function r using v(x: Int[0, 9]): Integer pure { 10 mod (x + 1) }\n";

#[test]
fn public_root_consumer_runs_builds_and_settles() {
    let call = Call {
        clocks: Default::default(),
        wire_arguments: Vec::new(),
        canonical_values: false,
        snapshot: None,
        function: "q".to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        accounting: ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: 1_000_000,
            result_units: u64::MAX,
        },
        supplied: SuppliedLimits::default(),
        backend: fsl_replay::ExecutionBackendChoice::Interpreter,
        live_occurrences: 1_048_576,
    };
    let result = run_with_replay_context(
        SourceIdentity::new("agent-ix", "fr404:root-consumer"),
        "fixture".to_owned(),
        "unit.native",
        SOURCE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        CallerLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("the call runs");
    assert!(matches!(
        result.outcome(),
        fsl_replay::CallOutcome::Completed(CallValue::Integer(value)) if *value == Integer::from(4_i64)
    ));
    assert_eq!(result.requirements().members().len(), 1);

    let member = &result.requirements().members()[0];
    let parameter = NodeKey::new(
        "fixture",
        vec![PathSegment::Text("fixture-parameter".to_owned())],
        7,
    )
    .expect("a fixture parameter key");
    let bound = ProofBound::new(
        DomainKey::Node {
            node: parameter.clone(),
            path: Vec::new(),
        },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64))
            .expect("integer range"),
    )
    .expect("proof bound");
    let plugin = BackendId::new("acme");
    let request = result
        .context()
        .request(
            result.package_id(),
            member.request_index(),
            &[bound.clone()],
            &plugin,
            vec![CanonicalAssignment {
                parameter,
                value: WitnessValue::Integer(9),
            }],
        )
        .expect("the consumer builds a member request");
    assert_eq!(request.package_id(), result.package_id());
    assert_eq!(
        request.replay_target.occurrence_key.as_ref(),
        Some(member.occurrence())
    );

    let foreign_index = RequestIndex::new(99);
    let refusal = result
        .context()
        .request(
            result.package_id(),
            foreign_index,
            &[bound],
            &plugin,
            Vec::new(),
        )
        .err()
        .expect("foreign request index refuses");
    assert!(matches!(
        &refusal,
        fsl_replay::PluginReplayRefusal::NotACallItem { request_index }
            if *request_index == foreign_index
    ));
    let record = refusal.record(foreign_index, plugin.clone());
    assert_eq!(record.value(), &TerminalValue::Failed);
    assert_eq!(record.backend(), Some(&plugin));
}
