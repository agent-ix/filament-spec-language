// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public consumer coverage for the call replay package pairing key.

use fsl_replay::{
    run_with_replay_context, BackendId, Call, CallArgument, CallerLimits, Cancel,
    CanonicalAssignment, DependencyInput, DomainKey, DomainKind, FiniteBound, Integer, NodeKey,
    ObjectEnvironment, PluginReplayRefusal, ProofBound, ScalarLimits, SourceIdentity,
    TerminalValue, Value, WitnessValue,
};
use ix_trace_rs::trace;
use std::collections::BTreeMap;

const R: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
function r using v(x: Int[0, 9]): Integer pure { 10 mod (x + 1) }\n";

const R2: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.value.complete/v1\";\n\
function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n\
function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n\
function r using v(x: Int[0, 9]): Integer pure { 10 mod (x + 1) }\n\
function s using v(x: Int[0, 9]): Integer pure { x + 2 }\n";

fn call() -> Call {
    Call {
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
        supplied: filament_semantic_value::supplied_stop::SuppliedLimits::default(),
        backend: filament_eval::ExecutionBackendChoice::Interpreter,
        live_occurrences: 1_048_576,
    }
}

fn run(source: &str) -> fsl_replay::RunWithReplayContext {
    run_with_replay_context(
        SourceIdentity::new("agent-ix", "fr404:fixture"),
        "fixture".to_owned(),
        "unit.native",
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        CallerLimits::default(),
        &call(),
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("fixture call compiles and runs")
}

fn bound_and_assignment() -> (Vec<ProofBound>, Vec<CanonicalAssignment>) {
    let parameter = NodeKey::new(
        "fixture",
        vec![filament_exact::PathSegment::Text("7".to_owned())],
        7,
    )
    .expect("parameter fixture key");
    let bounds = vec![ProofBound::new(
        DomainKey::Node {
            node: parameter.clone(),
            path: vec![],
        },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
    )
    .unwrap()];
    let assignments = vec![CanonicalAssignment {
        parameter,
        value: WitnessValue::Integer(3),
    }];
    (bounds, assignments)
}

/// FR-404-AC-5: equal occurrence keys in different packages do not make a
/// cross-run item valid; each run builds a request with its own bytes/id.
#[trace("TC-1173", "FR-404-AC-5")]
#[test]
fn package_id_binds_call_members_and_refuses_foreign_run_items() {
    let first = run(R);
    let second = run(R2);
    assert_ne!(first.package_id(), second.package_id());
    let first_member = &first.requirements().members()[0];
    let second_member = &second.requirements().members()[0];
    assert_eq!(first_member.occurrence(), second_member.occurrence());

    let (bounds, assignments) = bound_and_assignment();
    let first_wire = first
        .context()
        .request(
            first.package_id(),
            first_member.request_index(),
            &bounds,
            &BackendId::new("acme"),
            assignments.clone(),
        )
        .expect("first run owns its item");
    let second_wire = second
        .context()
        .request(
            second.package_id(),
            second_member.request_index(),
            &bounds,
            &BackendId::new("acme"),
            assignments.clone(),
        )
        .expect("second run owns its item");
    assert_eq!(first_wire.package_id(), first.package_id());
    assert_eq!(second_wire.package_id(), second.package_id());
    assert_eq!(first_wire.byte_provision[0].2, R.as_bytes());
    assert_eq!(second_wire.byte_provision[0].2, R2.as_bytes());

    let refusal = first
        .context()
        .request(
            second.package_id(),
            second_member.request_index(),
            &bounds,
            &BackendId::new("acme"),
            assignments,
        )
        .err()
        .expect("foreign package pairing key refuses before request construction");
    assert_eq!(
        refusal,
        PluginReplayRefusal::ForeignRun {
            received: second.package_id(),
            run: first.package_id(),
        }
    );
    let diagnostic = refusal.diagnostic(second_member.request_index().get());
    assert_eq!(diagnostic.code(), "invalid-request");
    assert_eq!(diagnostic.cause(), None);
    let record = refusal.record(second_member.request_index(), BackendId::new("acme"));
    assert_eq!(record.value(), &TerminalValue::Failed);
    assert_eq!(record.backend(), Some(&BackendId::new("acme")));
}
