// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeMap;

use super::{identity, source};
use filament_semantic_value::call::InputRefusal;
use fsl_replay::{
    CanonicalAssignment, DependencyInput, EnvelopeSource, EvaluatedValue, Identifier,
    InputSettlement, QualifiedName, ReplayLimits, ReplayPackage, ReplayRefusal, ReplayRequestWire,
    ReplayResult, RequestTargetWire, StateEnvironment, WitnessValue, call_site, replay,
};
use ix_trace_rs::trace;

const PROGRAM: &str = "function positive using v(x: Int[0, 9]): Boolean pure { x > 0 }\nfunction compareInputs using v(left: Int[0, 9], right: Int[0, 9]): Boolean pure { left > right }\nfunction flag using v(x: Boolean): Boolean pure { not x }\nfunction foreign using v(y: Int[0, 9]): Boolean pure { y = 0 }";

fn qualified(name: &str) -> QualifiedName {
    QualifiedName::new(vec![Identifier::new(name).unwrap()]).unwrap()
}

fn request(function: &str, supplied: &[(&str, WitnessValue)]) -> ReplayRequestWire {
    let text = source(PROGRAM);
    let selected = qualified(function);
    let located = call_site(
        identity(),
        "replay-boundaries".to_owned(),
        "bindings.native",
        text.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &DependencyInput::default(),
        &selected,
    )
    .unwrap();
    let assignments = supplied
        .iter()
        .map(|(name, value)| CanonicalAssignment {
            parameter: located
                .site
                .parameters
                .iter()
                .find(|(parameter, _)| parameter.as_str() == *name)
                .unwrap()
                .1
                .clone(),
            value: value.clone(),
        })
        .collect();
    ReplayRequestWire {
        profile_selections: Vec::new(),
        package: ReplayPackage::read(&located.package, ReplayLimits::default().input_bytes)
            .unwrap(),
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::function(selected, located.site.declaration.clone()),
        source: EnvelopeSource::Input(assignments).into(),
        obligation_identity: located.site.obligation_identity().clone(),
        backend: "boundary-producer".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: fsl_replay::spine::default_accounting(10000),
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            "agent-ix".to_owned(),
            "test:lowering-intake-boundaries".to_owned(),
            text.into_bytes(),
        )],
        documents: Vec::new(),
    }
}

fn assert_value(request: ReplayRequestWire, expected: bool) {
    let result = replay(request, ReplayLimits::default())
        .expect("the source and actual parameter ids recompile and bind");
    let ReplayResult::Input(result) = result else {
        panic!("the input arm must stay input: {result:?}");
    };
    assert_eq!(result.value(), Some(EvaluatedValue::Boolean(expected)));
    if !expected {
        assert_eq!(
            result.settlement(),
            InputSettlement::ReproducedWithoutWitness
        );
        assert_eq!(result.category().exit_code(), 10);
        assert!(result.disagreement().is_none());
    } else {
        assert_eq!(result.settlement(), InputSettlement::Inconclusive);
        assert_eq!(result.category().exit_code(), 0);
        assert!(
            result.disagreement().is_some(),
            "a true predicate does not reproduce a refutation"
        );
    }
}

#[test]
#[trace("FR-121-AC-1", "FR-098-AC-1", "FR-098-AC-2")]
fn located_parameter_ids_bind_the_actual_predicate_in_a_source_only_provision() {
    assert_value(
        request("positive", &[("x", WitnessValue::Integer(0))]),
        false,
    );
    assert_value(
        request("positive", &[("x", WitnessValue::Integer(1))]),
        true,
    );
}

#[test]
#[trace("FR-098-AC-2")]
fn input_assignments_bind_by_node_identity_in_either_request_order() {
    assert_value(
        request(
            "compareInputs",
            &[
                ("right", WitnessValue::Integer(3)),
                ("left", WitnessValue::Integer(8)),
            ],
        ),
        true,
    );
    assert_value(
        request(
            "compareInputs",
            &[
                ("left", WitnessValue::Integer(3)),
                ("right", WitnessValue::Integer(8)),
            ],
        ),
        false,
    );
}

#[test]
#[trace("FR-098-AC-2")]
fn boolean_input_assignments_require_boolean_values_instead_of_integer_aliases() {
    assert_value(
        request("flag", &[("x", WitnessValue::Boolean(true))]),
        false,
    );
    assert_value(
        request("flag", &[("x", WitnessValue::Boolean(false))]),
        true,
    );
    let refusal = replay(
        request("flag", &[("x", WitnessValue::Integer(1))]),
        ReplayLimits::default(),
    )
    .unwrap_err();
    assert!(
        matches!(
            refusal.input_refusal(),
            Some(InputRefusal::WrongValueKind { parameter: 0 })
        ),
        "{refusal:?}"
    );
}

#[test]
#[trace("FR-121-AC-1", "FR-098-AC-2")]
fn omitting_a_located_parameter_is_a_typed_unbound_refusal() {
    let refusal = replay(request("positive", &[]), ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(refusal, ReplayRefusal::UnboundParameter(_)),
        "{refusal:?}"
    );
}

#[test]
#[trace("FR-098-AC-2")]
fn another_declared_functions_parameter_cannot_bind_the_selected_predicate() {
    let text = source(PROGRAM);
    let foreign = call_site(
        identity(),
        "replay-boundaries".to_owned(),
        "bindings.native",
        text.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &DependencyInput::default(),
        &qualified("foreign"),
    )
    .unwrap();
    let foreign_parameter = foreign.site.parameters[0].1.clone();
    let mut wire = request("positive", &[("x", WitnessValue::Integer(0))]);
    wire.source = EnvelopeSource::Input(vec![CanonicalAssignment {
        parameter: foreign_parameter.clone(),
        value: WitnessValue::Integer(0),
    }])
    .into();
    let refusal = replay(wire, ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(refusal, ReplayRefusal::UnknownParameter(ref parameter) if parameter == &foreign_parameter),
        "{refusal:?}"
    );
}
