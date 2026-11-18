// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-206-AC-13 (TC-2990): the protocol evaluator never advances shared
//! state through an observe-only step. A `receive` by an observe-only role and
//! a `recommend` keep the model state, and the `recommend` step carries its
//! recommendation record.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{ProtocolSystem, Sc};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{
    DocumentProvision, DocumentRef, ObservationLimits, document_reference,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

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
protocol Watch using P over (view: M::Node) on origin {
  observe role Auditor on M::Node;
  role Seller on M::Node;
  channel Note from Seller to Auditor carries M::Flag
      ordering unordered delivery at-least-once capacity 1 overflow block;
  run sequence Main {
    send Sent via Note as (sent: M::Flag) { true };
    receive Seen via Note of Main::Sent as (seen: M::Flag) { true };
    recommend Advise by Auditor on M::Node::step contracts []
        as (advice: M::Flag) { true };
  }
  finish Closed as (closed: M::Receipt) { true };
}
"#;

fn fixture(
    source: &str,
) -> (
    CheckedPackage,
    fsl_semantics::model::intake::PackageInput,
    DocumentProvision,
    DocumentRef,
) {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2211", "unit.native"),
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
            "qsl-2211".to_owned(),
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
        identity: "qsl-2211-start".to_owned(),
    };
    let snapshot = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "qsl-2211-start"},
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

fn reachable(source: &str) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    let (package, packages, snapshots, initial) = fixture(source);
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
            keys: vec!["node-1".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: METER,
    };
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let checked = package.graph().protocol("Watch").expect("protocol checked");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Watch".to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles: ["Auditor", "Seller"]
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

/// The `semantic` member of a protocol state key: the model state.
fn model_state(key: &fsl_eval::simulation::StateKey) -> Value {
    let key: Value = serde_json::from_slice(key.as_bytes()).expect("a JCS state key");
    key["semantic"].clone()
}

#[test]
#[trace("TC-2990", "FR-206-AC-13", "FSpec-TC-2019", "FSpec-FR-822-AC-5")]
fn an_observe_only_step_leaves_state_unchanged() {
    let graph = reachable(SOURCE);
    let mut recommends = 0_usize;
    let mut observed = 0_usize;
    for state in &graph.states {
        for (transition, next) in &state.steps {
            let Some(node) = transition.node() else {
                continue;
            };
            if !(node.ends_with("Advise") || node.ends_with("Seen")) {
                continue;
            }
            observed += 1;
            if node.ends_with("Advise") {
                assert_eq!(format!("{:?}", transition.kind()), "Recommend", "{node}");
                let recommendation = transition.recommendation().expect("typed recommendation");
                assert_eq!(recommendation.node, "Main/Advise");
                assert_eq!(recommendation.role, "Auditor");
                assert_eq!(recommendation.operation, "step");
                assert!(recommendation.arguments.is_empty());
                recommends += 1;
            }
            assert_eq!(
                model_state(&state.key),
                model_state(&graph.states[*next].key),
                "an observe-only step at {node} keeps the model state"
            );
        }
    }
    assert!(recommends > 0, "the recommend control is stepped");
    assert!(observed > recommends, "the observe-only receive is stepped");
}
