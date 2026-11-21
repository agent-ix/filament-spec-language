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
    receive First via Charge of Main::Sent as (receivedOne: M::Flag) { true };
    receive Second via Charge of Main::Sent as (receivedTwo: M::Flag) { true };
    attempt Pay by Seller on M::Node::step contracts []
        as (tried: M::Flag) { true };
    effect Paid of Main::Pay as (applied: M::Receipt) { true };
  }
  finish Closed as (closed: M::Receipt) { true };
}
"#;

fn fixture(
    source: &str,
    include_second: bool,
    related: bool,
) -> (
    CheckedPackage,
    fsl_semantics::model::intake::PackageInput,
    DocumentProvision,
    DocumentRef,
) {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2208", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source read");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("unit forms built");
    let mut domain = tc1070::document();
    domain["types"]
        .as_array_mut()
        .expect("domain types")
        .push(tc1070::model::declaration(
            "Link",
            "record",
            json!({
                "fields": [
                    tc1070::model::field("Link", "source", &tc1070::model::id("Node")),
                    tc1070::model::field("Link", "target", &tc1070::model::id("Node")),
                ]
            }),
        ));
    let domain_bytes = serde_json::to_vec(&domain).expect("domain serializes");
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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "qsl-2208-start".to_owned(),
    };
    let mut document = json!({
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
    });
    if include_second {
        document["populations"][0]["objects"]
            .as_array_mut()
            .expect("population objects")
            .push(json!({
                "key": "node-2",
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": "6"}}
            }));
    }
    if related {
        document["relationships"] = json!([{
            "relationship": tc1070::model::id("relationship/owns"),
            "complete": true,
            "instances": [{
                "source": {"population": tc1070::model::id("nodes"), "key": "node-2"},
                "target": {"population": tc1070::model::id("nodes"), "key": "node-1"}
            }]
        }]);
    }
    let snapshot = serde_json::to_vec(&document).expect("snapshot serializes");
    let source_ref = document_reference(&snapshot).expect("snapshot has identity");
    let snapshots = BTreeMap::from([(source_ref, snapshot)]);
    (package, packages, snapshots, identity)
}

fn reachable(source: &str) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    reachable_with_nodes(source, false, false)
}

fn reachable_with_relation(
    source: &str,
    include_second: bool,
) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    reachable_with_nodes(source, include_second, true)
}

fn reachable_with_nodes(
    source: &str,
    include_second: bool,
    related: bool,
) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    let (package, packages, snapshots, initial) = fixture(source, include_second, related);
    let population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let request = ModelSimulationRequest {
        emit: fsl_package::EmitLimits::fixture(),
        input: fsl_eval::simulation::model::ModelCheckInput::source(&package),
        packages: &packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![initial],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: if include_second {
                vec!["node-1".to_owned(), "node-2".to_owned()]
            } else {
                vec!["node-1".to_owned()]
            },
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
        capacities: Vec::new(),
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
) -> Option<(Vec<DeliveryIdentity>, Vec<DeliveryIdentity>, Vec<u64>)> {
    let mut queue = VecDeque::from([(
        0_usize,
        None::<fsl_eval::simulation::protocol::MessageOccurrence>,
        Vec::<DeliveryIdentity>::new(),
        Vec::<DeliveryIdentity>::new(),
        Vec::<u64>::new(),
        false,
    )]);
    let mut seen = std::collections::BTreeSet::new();
    while let Some((state, message, copies, receives, effects, duplicated)) = queue.pop_front() {
        let signature = (
            state,
            message.clone(),
            copies.clone(),
            receives.clone(),
            effects.clone(),
            duplicated,
        );
        if !seen.insert(signature) {
            continue;
        }
        for (transition, next) in &graph.states[state].steps {
            let mut message = message.clone();
            let mut copies = copies.clone();
            let mut receives = receives.clone();
            let mut effects = effects.clone();
            let mut duplicated = duplicated;
            match transition.kind() {
                StepKind::Send => message = transition.message().cloned(),
                StepKind::Duplicate => {
                    let delivery = transition.delivery()?.clone();
                    if Some(delivery.message()) != message.as_ref() || delivery.index() != 1 {
                        continue;
                    }
                    copies.push(delivery);
                    duplicated = true;
                }
                StepKind::Receive => {
                    let delivery = transition.delivery().cloned()?;
                    if Some(delivery.message()) != message.as_ref() {
                        continue;
                    }
                    receives.push(delivery);
                }
                StepKind::Event if transition.business_effect().is_some() => {
                    effects.push(transition.business_effect()?.ordinal());
                    if duplicated
                        && receives.iter().any(|delivery| delivery.index() == 0)
                        && receives.iter().any(|delivery| delivery.index() == 1)
                    {
                        return Some((copies, receives, effects));
                    }
                }
                _ => {}
            }
            queue.push_back((*next, message, copies, receives, effects, duplicated));
        }
    }
    None
}

fn bound_reference_ids(
    graph: &fsl_eval::simulation::protocol::Reachable<Sc>,
    kind: StepKind,
) -> std::collections::BTreeSet<String> {
    graph
        .states
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(transition, _)| transition.kind() == kind)
        .filter_map(|(transition, _)| {
            fn first_reference(value: &filament_exact::Value) -> Option<String> {
                match value {
                    filament_exact::Value::Reference(reference) => {
                        Some(reference.object().as_str().to_owned())
                    }
                    filament_exact::Value::Composite(composite) => {
                        composite.slots().iter().find_map(|slot| match slot {
                            filament_exact::FieldValue::Present(value) => first_reference(value),
                            filament_exact::FieldValue::Absent
                            | filament_exact::FieldValue::Null => None,
                        })
                    }
                    _ => None,
                }
            }
            transition.binder_record().and_then(first_reference)
        })
        .collect()
}

#[test]
#[trace("FR-731-AC-1")]
fn a_related_by_clause_limits_protocol_event_bindings_to_its_relationship_instance() {
    let source = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Purchase using P over (view: M::Node) on origin {
  role Buyer on M::Node;
  role Seller on M::Node;
  relationship Owns = M::owns;
  run sequence Main {
    event Paid by Buyer as (paid: M::Node) related by Owns (paid, view) { true };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;
    let graph = reachable_with_relation(source, true);
    let paid = bound_reference_ids(&graph, StepKind::Event);
    assert_eq!(paid, ["node-2".to_owned()].into_iter().collect());

    let uncorrelated_source = source.replace(" related by Owns (paid, view)", "");
    let graph = reachable_with_nodes(&uncorrelated_source, true, false);
    let paid = bound_reference_ids(&graph, StepKind::Event);
    assert_eq!(
        paid,
        ["node-1".to_owned(), "node-2".to_owned()]
            .into_iter()
            .collect()
    );
}

#[test]
#[trace("TC-2023", "FR-731-AC-1")]
fn related_by_filters_send_and_receive_candidates() {
    let source = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Purchase using P over (view: M::Node) on origin {
  role Buyer on M::Node;
  role Seller on M::Node;
  relationship Owns = M::owns;
  channel Pay from Buyer to Seller carries M::Link
      ordering unordered delivery at-least-once capacity 2 overflow block;
  run sequence Main {
    send Sent via Pay as (sent: M::Link) { true };
    receive Got via Pay of Main::Sent as (got: M::Link) { true };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;
    let correlated_send = source.replace(
        "as (sent: M::Link) { true }",
        "as (sent: M::Link) related by Owns (sent.source, view) { true }",
    );
    let sends = reachable_with_relation(&correlated_send, true);
    assert_eq!(
        bound_reference_ids(&sends, StepKind::Send),
        ["node-2".to_owned()].into_iter().collect()
    );

    let correlated_receive = source.replace(
        "as (got: M::Link) { true }",
        "as (got: M::Link) related by Owns (got.source, view) { true }",
    );
    let receives = reachable_with_relation(&correlated_receive, true);
    assert_eq!(
        bound_reference_ids(&receives, StepKind::Send),
        ["node-1".to_owned(), "node-2".to_owned()]
            .into_iter()
            .collect()
    );
    assert_eq!(
        bound_reference_ids(&receives, StepKind::Receive),
        ["node-2".to_owned()].into_iter().collect()
    );
}

#[test]
#[trace("FR-731-AC-1")]
fn a_related_by_clause_limits_protocol_effect_bindings_to_its_relationship_instance() {
    let source = SOURCE
        .replace(
            "  channel Charge from Buyer",
            "  relationship Owns = M::owns;\n  channel Charge from Buyer",
        )
        .replace(
            "effect Paid of Main::Pay as (applied: M::Receipt) { true }",
            "effect Paid of Main::Pay as (applied: M::Link) \
             related by Owns (applied.source, view) { true }",
        );
    let graph = reachable_with_relation(&source, true);
    assert_eq!(
        bound_reference_ids(&graph, StepKind::Event),
        ["node-2".to_owned()].into_iter().collect()
    );
}

#[test]
#[trace("TC-2528", "FR-894-AC-1")]
fn a_duplicate_delivery_applies_its_effect_once() {
    let graph = reachable(SOURCE);
    let (copies, receives, effects) = duplicate_then_effect(&graph)
        .expect("the channel can duplicate one send before both receives");
    assert_eq!(copies.len(), 1, "one duplicated copy");
    assert_eq!(copies[0].index(), 1);
    assert_eq!(receives.len(), 2, "both copies were received once");
    assert_eq!(
        receives
            .iter()
            .map(DeliveryIdentity::index)
            .collect::<Vec<_>>(),
        [0, 1],
    );
    assert_eq!(
        copies
            .iter()
            .chain(&receives)
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
