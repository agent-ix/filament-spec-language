// SPDX-License-Identifier: AGPL-3.0-or-later
//! Replay-owned cancellation behavior for TC-757.

use std::collections::BTreeMap;

use filament_exact::{Cancel, CancelCause};
use filament_semantic_value::supplied_stop::{SuppliedCause, SuppliedEvent, SuppliedPhase};
use fsl_replay::{
    CallSiteUnit, CanonicalAssignment, DependencyInput, Identifier, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource, RequestClocks,
    RequestTargetWire, SourceIdentity, StateEnvironment, WitnessValue, replay_with_cancel,
};
use ix_trace_rs::trace;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function p using v(x: Int[0, 9]): Boolean pure { x > 3 }\n";

/// TC-757 partial: replay stops at the first supplied-value charge after a
/// handle is cancelled, preserving the installed cause and producing no arm.
#[trace("TC-757")]
#[test]
fn replay_preserves_cancellation_at_the_first_conversion_charge() {
    let source = SourceIdentity::new("agent-ix", "test:replay-cancellation");
    let name = QualifiedName::new(vec![Identifier::new("p").expect("function name")])
        .expect("qualified function name");
    let site = fsl_replay::call_site(
        source.clone(),
        CallSiteUnit::new("fixture".to_owned(), "unit.native", SOURCE.as_bytes()),
        [],
        &DependencyInput::default(),
        &RequestClocks::default(),
        &name,
    )
    .expect("the replay fixture compiles");
    let wire = ReplayRequestWire {
        certificate: None,
        profile_selections: vec![],
        package: ReplayPackage::read(&site.package, ReplayLimits::default().input_bytes)
            .expect("the call-site package reads"),
        dependencies: vec![],
        replay_target: RequestTargetWire::function(name, site.site.declaration.clone()),
        source: ReplaySource::Input(vec![CanonicalAssignment {
            parameter: site.site.parameters[0].1.clone(),
            value: WitnessValue::Integer(5),
        }]),
        obligation_identity: site.site.obligation_identity().clone(),
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: filament_exact::ScalarLimits::default(),
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(
            "agent-ix".to_owned(),
            "test:replay-cancellation".to_owned(),
            SOURCE.as_bytes().to_vec(),
        )],
        documents: vec![],
    };

    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let stop = match replay_with_cancel(wire.clone(), ReplayLimits::default(), &cancel) {
            Err(ReplayRefusal::Supplied(stop)) => stop,
            other => panic!("cancelled replay must stop at conversion, got {other:?}"),
        };
        assert_eq!(stop.cause, SuppliedCause::Cancelled(cause));
        assert_eq!(stop.limit(), None);
        assert_eq!(stop.context.phase, SuppliedPhase::Conversion);
        assert_eq!(stop.context.parameter, Some(0));
        assert_eq!(stop.context.event, Some(SuppliedEvent::WitnessPreflight));
        assert_eq!(stop.context.consumed, 0);
    }
}
