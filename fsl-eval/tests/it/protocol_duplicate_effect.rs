// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-894-AC-1: message, delivery and business-effect occurrence identities
//! remain separate in the protocol evaluator.

use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroUsize;

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{DeliveryIdentity, ProtocolSystem, Sc, StepKind};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{
    document_reference, DocumentProvision, DocumentRef, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::json;

#[path = "../../../fsl-semantics/tests/support/tc1070.rs"]
#[allow(dead_code, clippy::duplicate_mod)]
mod tc1070;

const METER: filament_exact::ScalarLimits = filament_exact::ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const SOURCE: &str = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Purchase using P over (view: M::Node) on origin {
  role Buyer on M::Node;
  role Seller on M::Node;
  channel Charge from Buyer to Seller carries M::Flag
      ordering unordered delivery at-least-once capacity 2 overflow block;
  run sequence Main {
    send Sent via Charge as (sent: M::Flag) { true };
    receive First via Charge of Main::Sent as (first: M::Flag) { true };
    receive Second via Charge of Main::Sent as (second: M::Flag) { true };
    attempt Pay by Seller on M::Node::step contracts []
        as (tried: M::Flag) { true };
    effect Paid of Main::Pay as (applied: M::Receipt) { true };
  }
  finish Closed as (closed: M::Receipt) { true };
}
"#;

fn fixture() -> (
    CheckedPackage,
    fsl_semantics::model::intake::PackageInput,
    DocumentProvision,
    DocumentRef,
) {
    let source = SOURCE;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2208", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source read");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("unit forms built");
    let domain_bytes = serde_json::to_vec(&tc1070::document()).expect("domain serializes");
    let packages = package_input([domain_bytes.as_slice()]).expect("domain package admitted");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("domain model selected");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "qsl-2208".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("unit assembled")
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "qsl-2208-start".to_owned(),
    };
    let snapshot = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "qsl-2208-start"},
        "observation": "current",
        "model": {"identity": tc1070::model::PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": tc1070::model::id("nodes"),
            "complete": true,
            "objects": [{
                "key": "node-1",
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": "5"}}
            }]
        }]
    }))
    .expect("snapshot serializes");
    let source_ref = document_reference(&snapshot).expect("snapshot has identity");
    let snapshots = BTreeMap::from([(source_ref, snapshot)]);
    (package, packages, snapshots, identity)
}

fn reachable() -> fsl_eval::simulation::protocol::Reachable<Sc> {
    let (package, packages, snapshots, initial) = fixture();
    let population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let request = ModelSimulationRequest {
        emit: fsl_package::EmitLimits::fixture(),
        package: &package,
        packages: &packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![initial],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: vec!["node-1".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: METER,
    };
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let checked = package
        .graph()
        .protocol("Purchase")
        .expect("protocol checked");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Purchase".to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles: ["Buyer", "Seller"]
            .into_iter()
            .map(|role| RoleBinding {
                role: role.to_owned(),
                model: tc1070::model::PACKAGE.to_owned(),
            })
            .collect(),
        memory: fsl_eval::simulation::protocol::ResolvedMemory::resolve(checked, &[])
            .expect("sc memory selected"),
    };
    ProtocolSystem::new(model, &subject, Sc)
        .expect("protocol subject built")
        .reachable(NonZeroUsize::new(1).expect("one worker"))
        .expect("finite protocol reachability")
}

/// Find one path that sends, duplicates, receives both delivery copies and
/// establishes the single effect occurrence.
fn duplicate_then_effect(
    graph: &fsl_eval::simulation::protocol::Reachable<Sc>,
) -> Option<(Vec<DeliveryIdentity>, Vec<u64>)> {
    let mut queue = VecDeque::from([(
        0_usize,
        None::<fsl_eval::simulation::protocol::MessageOccurrence>,
        Vec::<DeliveryIdentity>::new(),
        Vec::<u64>::new(),
        false,
    )]);
    let mut seen = std::collections::BTreeSet::new();
    while let Some((state, message, deliveries, effects, duplicated)) = queue.pop_front() {
        let signature = (
            state,
            message.clone(),
            deliveries.clone(),
            effects.clone(),
            duplicated,
        );
        if !seen.insert(signature) {
            continue;
        }
        for (transition, next) in &graph.states[state].steps {
            let mut message = message.clone();
            let mut deliveries = deliveries.clone();
            let mut effects = effects.clone();
            let mut duplicated = duplicated;
            match transition.kind() {
                StepKind::Send => message = transition.message().cloned(),
                StepKind::Duplicate => {
                    let delivery = transition.delivery()?.clone();
                    if Some(delivery.message()) != message.as_ref() || delivery.index() != 1 {
                        continue;
                    }
                    deliveries.push(delivery);
                    duplicated = true;
                }
                StepKind::Receive => {
                    let Some(delivery) = transition.delivery().cloned() else {
                        return None;
                    };
                    if Some(delivery.message()) != message.as_ref() {
                        continue;
                    }
                    deliveries.push(delivery);
                }
                StepKind::Event if transition.business_effect().is_some() => {
                    effects.push(transition.business_effect()?.ordinal());
                    if duplicated
                        && deliveries.iter().any(|delivery| delivery.index() == 0)
                        && deliveries.iter().any(|delivery| delivery.index() == 1)
                    {
                        return Some((deliveries, effects));
                    }
                }
                _ => {}
            }
            queue.push_back((*next, message, deliveries, effects, duplicated));
        }
    }
    None
}

#[test]
#[trace("TC-2528", "FR-894-AC-1")]
fn a_duplicate_delivery_is_distinct_but_establishes_one_effect_occurrence() {
    let graph = reachable();
    let (deliveries, effects) = duplicate_then_effect(&graph)
        .expect("the channel can duplicate one send before both receives");
    assert_eq!(deliveries.len(), 3, "one duplicate and two receives");
    let duplicated = &deliveries[0];
    assert_eq!(duplicated.index(), 1);
    assert!(deliveries.iter().any(|delivery| delivery.index() == 0));
    assert_eq!(
        deliveries
            .iter()
            .map(|delivery| delivery.message().clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        1,
        "both delivery indices name one send occurrence"
    );
    assert_eq!(effects, [0], "one effect occurrence is established");

    for reached in &graph.states {
        for (transition, _) in &reached.steps {
            if let Some(effect) = transition.business_effect() {
                assert_eq!(effect.ordinal(), 0);
                let identity = fsl_eval::simulation::protocol::transition_json(transition)
                    .expect("effect identity serializes");
                assert!(identity.get("business_effect").is_some());
                assert!(identity.get("record").is_none(), "payload is not identity");
            }
        }
    }
}
