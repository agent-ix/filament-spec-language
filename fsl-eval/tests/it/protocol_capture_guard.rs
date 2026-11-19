// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218-AC-17: protocol captures can drive guards during public reachability.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{ProtocolSystem, Sc, StepKind};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{DocumentRef, ObservationLimits, document_reference};
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
protocol Checkout using P over (input: M::Node) on origin {
  capture hasFunds: Boolean = input.total > 0;
  role Buyer on M::Node;
  run sequence Main {
    choice Decide by Buyer visible () {
      case funded when { hasFunds } sequence Funded {
        attempt Approve by Buyer on M::Node::step contracts []
            as (approved: M::Flag) { true };
      }
      case empty when { not hasFunds } sequence Empty {
        attempt Decline by Buyer on M::Node::step contracts []
            as (declined: M::Flag) { true };
      }
    }
  }
  finish Closed as (closed: M::Receipt) { true };
}
"#;

fn reachable(total: i64) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-1971", "unit.native"),
        "unit.native",
        SOURCE.as_bytes(),
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
            "qsl-1971".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("unit assembled")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "qsl-1971-start".to_owned(),
    };
    let snapshot = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "qsl-1971-start"},
        "observation": "current",
        "model": {"identity": tc1070::model::PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": tc1070::model::id("nodes"),
            "complete": true,
            "objects": [{
                "key": "node-1",
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": total.to_string()}}
            }]
        }]
    }))
    .expect("snapshot serializes");
    let source_ref = document_reference(&snapshot).expect("snapshot has identity");
    let snapshots = BTreeMap::from([(source_ref, snapshot)]);
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
        initial: vec![identity],
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
        .protocol("Checkout")
        .expect("protocol checked");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Checkout".to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles: ["Buyer"]
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

fn attempted_nodes(graph: &fsl_eval::simulation::protocol::Reachable<Sc>) -> Vec<String> {
    graph
        .states
        .iter()
        .flat_map(|state| state.steps.iter())
        .filter_map(|(transition, _)| {
            (transition.kind() == StepKind::Attempt)
                .then(|| transition.node().map(str::to_owned))
                .flatten()
        })
        .collect()
}

/// A visible protocol capture selects the only explored branch from the
/// initial model state, without listing the capture in `visible`.
#[trace("FR-218-AC-17")]
#[test]
fn a_guard_over_a_capture_is_explored() {
    let funded = attempted_nodes(&reachable(5));
    assert_eq!(funded.len(), 1, "{funded:?}");
    assert!(funded[0].ends_with("Approve"), "{funded:?}");

    let empty = attempted_nodes(&reachable(0));
    assert_eq!(empty.len(), 1, "{empty:?}");
    assert!(empty[0].ends_with("Decline"), "{empty:?}");
}
