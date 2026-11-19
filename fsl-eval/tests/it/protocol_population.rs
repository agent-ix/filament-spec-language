// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-883 (TC-2507): evaluate population reads in protocol slots that may
//! read model state, while keeping the protocol guard boundary at S3.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use filament_exact::ScalarLimits;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc, StepKind};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{DocumentProvision, ObservationLimits, document_reference};
use ix_trace_rs::trace;
use serde_json::{Value as Json, json};

#[path = "../../../fsl-semantics/tests/support/tc1070.rs"]
mod tc1070;

const METER: ScalarLimits = ScalarLimits {
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
protocol Census using P over (view: M::Node) on origin {
  capture populationCount: Integer = size(allInstances<M::Node>(M::otherNodes));
  role Seller on M::Node;
  role Buyer on M::Node;
  channel Notes from Seller to Buyer carries M::Flag
      ordering unordered delivery at-least-once capacity 1 overflow block;
  run sequence Main {
    send Sent via Notes as (sent: M::Flag) {
      size(allInstances<M::Node>(M::otherNodes)) >= 1 and populationCount = 2
    };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;

const LOOKUP_SOURCE: &str = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Lookup using P over (view: M::Node) on origin {
  capture found: Option<M::Node> = lookup<M::Node>(M::nodes, view) absent empty;
  capture missing: Option<M::Node> = lookup<M::Node>(M::otherNodes, view) absent empty;
  role Observer on M::Node;
  run sequence Main { }
  finish Closed as (closed: M::Flag) { true };
}
"#;

const CHECK_GUARD_SOURCE: &str = r#"
language "ix:native" edition "1-draft";
profile P = "filament.protocol.finite-global/v1";
profile S = "filament.state.graph/v1";
model M = "test/orders";
protocol Guard using P over (view: M::Node) on origin {
  role Observer on M::Node;
  run sequence Main {
    check HasNodes using S { size(allInstances<M::Node>(M::nodes)) > 0 };
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;

/// Build a checked package with an additional population of `Node` values.
fn package(source: &str) -> (CheckedPackage, fsl_semantics::model::intake::PackageInput) {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2216", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");

    let mut domain = tc1070::document();
    domain["populations"]
        .as_array_mut()
        .expect("population list")
        .push(json!({
            "identity": tc1070::model::id("otherNodes"),
            "displayName": "otherNodes",
            "kind": {"module": tc1070::model::BUSINESS, "name": "population"},
            "members": [tc1070::model::id("Node")],
            "extent": "closed",
            "origin": tc1070::model::origin("otherNodes")
        }));
    let domain_bytes = serde_json::to_vec(&domain).expect("domain serializes");
    let packages = package_input([domain_bytes.as_slice()]).expect("domain admits");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the model");
    let checked = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "qsl-2216".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the protocol package checks: {refusals:?}")),
    );
    (checked, packages)
}

/// Run a protocol over one node and list `other_nodes` in the current state.
fn run_protocol(
    source: &str,
    protocol_name: &str,
    roles: &[&str],
    other_nodes: &[&str],
) -> (Json, bool) {
    let (package, packages) = package(source);
    let population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let other_population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("otherNodes"),
    };
    let mut populations = vec![json!({
        "population": tc1070::model::id("nodes"),
        "complete": true,
        "objects": [{"key":"node-1", "type":tc1070::model::id("Node"), "fields":{"total":{"integer":"5"}}}]
    })];
    let objects: Vec<Json> = other_nodes
        .iter()
        .map(|key| {
            json!({
                "key": key,
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": "5"}}
            })
        })
        .collect();
    populations.push(json!({
        "population": tc1070::model::id("otherNodes"),
        "complete": true,
        "objects": objects
    }));
    let snapshot = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority":"test", "identity":"qsl-2216-start"},
        "observation": "current",
        "model": {"identity": tc1070::model::PACKAGE},
        "anchor": {"kind":"initialization", "name":"start"},
        "populations": populations
    }))
    .expect("snapshot serializes");
    let initial = document_reference(&snapshot).expect("snapshot has identity");
    let snapshots: DocumentProvision = BTreeMap::from([(initial.clone(), snapshot)]);
    let request = ModelSimulationRequest {
        emit: fsl_package::EmitLimits::fixture(),
        input: fsl_eval::simulation::model::ModelCheckInput::source(&package),
        packages: &packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![initial],
        universes: vec![
            PopulationUniverse {
                population: population.clone(),
                keys: vec!["node-1".to_owned()],
            },
            PopulationUniverse {
                population: other_population,
                keys: other_nodes.iter().map(|key| (*key).to_owned()).collect(),
            },
        ],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: METER,
    };
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let checked = package
        .graph()
        .protocol(protocol_name)
        .expect("protocol checked");
    let roles = roles
        .iter()
        .map(|role| RoleBinding {
            role: (*role).to_owned(),
            model: tc1070::model::PACKAGE.to_owned(),
        })
        .collect();
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: protocol_name.to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles,
        capacities: Vec::new(),
        memory: ResolvedMemory::resolve(checked, &[]).expect("sequential consistency selected"),
    };
    let system = ProtocolSystem::new(model, &subject, Sc).expect("protocol subject built");
    // `ProtocolSystem` borrows package, request data and subject; return only
    // its reachable result so those inputs can remain local to this helper.
    let graph = system
        .reachable(NonZeroUsize::new(1).expect("one worker"))
        .expect("finite protocol reachability");
    let capture = system
        .capture_bytes(&graph.states[0].state)
        .expect("initial captures have typed canonical forms");
    let (_, bytes) = capture.first().expect("one protocol instance");
    let captures = serde_json::from_slice(bytes).expect("captures are canonical JSON");
    let has_send = graph.states.iter().any(|state| {
        state
            .steps
            .iter()
            .any(|(transition, _)| transition.kind() == StepKind::Send)
    });
    (captures, has_send)
}

/// Provenance: QSL-2216.
#[trace("TC-2507", "FR-883-AC-1")]
#[test]
fn all_instances_populates_a_protocol_capture_and_send_constraint() {
    let (captures, has_send) = run_protocol(
        SOURCE,
        "Census",
        &["Seller", "Buyer"],
        &["other-1", "other-2"],
    );
    assert_eq!(
        captures["entries"][0]["value"]["value"],
        json!("2"),
        "{captures}"
    );
    assert!(has_send, "the populated send constraint is enabled");

    let (captures, has_send) = run_protocol(SOURCE, "Census", &["Seller", "Buyer"], &[]);
    assert_eq!(captures["entries"][0]["value"]["value"], json!("0"));
    assert!(!has_send, "the empty-population send is disabled");
}

/// Provenance: QSL-2216.
#[trace("TC-2507", "FR-883-AC-2")]
#[test]
fn lookup_in_a_protocol_capture_matches_present_and_empty_absent_results() {
    let (captures, _) = run_protocol(LOOKUP_SOURCE, "Lookup", &["Observer"], &[]);
    assert_eq!(captures["entries"][0]["key"], json!("found"));
    assert_eq!(
        captures["entries"][0]["value"]["type"],
        json!("option"),
        "{captures}"
    );
    assert_eq!(
        captures["entries"][0]["value"]["value"]["type"],
        json!("reference")
    );
    assert_eq!(
        captures["entries"][0]["value"]["value"]["identity"],
        json!("node-1"),
        "{captures}"
    );
    assert_eq!(captures["entries"][1]["key"], json!("missing"));
    assert_eq!(captures["entries"][1]["value"]["type"], json!("option"));
    assert!(captures["entries"][1]["value"]["value"].is_null());
}

/// Provenance: QSL-2216.
#[trace("TC-2507", "FR-883-AC-4")]
#[test]
fn all_instances_in_a_check_guard_is_refused_at_checking() {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2216", "guard.native"),
        "guard.native",
        CHECK_GUARD_SOURCE.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let domain_bytes = serde_json::to_vec(&tc1070::document()).expect("domain serializes");
    let packages = package_input([domain_bytes.as_slice()]).expect("domain admits");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the model");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qsl-2216".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check_with_protocol_results(CheckMode::Linked, CheckingLimits::default())
    .expect("the protocol refusal is retained in its result");
    let refusals: Vec<_> = graph
        .protocol_refusals()
        .iter()
        .flat_map(|result| result.refusals.iter())
        .collect();
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == fsl_foundation::Code::IllTyped
                && refusal.cause.cause() == Some("operator-ineligible")
        }),
        "expected FR-883's S3 slot refusal, got {refusals:?}"
    );
}
