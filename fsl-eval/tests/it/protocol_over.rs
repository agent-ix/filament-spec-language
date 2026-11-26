// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-880-AC-8 and FR-882-AC-6: evaluate a compiled post-test repeat whose
//! guard reads the binder produced by its body.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use filament_exact::{EffectiveId, OriginalDeclarationKey, ScalarLimits};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{
    over_reference, ProtocolSystem, Reachable, ResolvedMemory, Sc, StepKind,
};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{document_reference, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;
use serde_json::json;

#[path = "../../../fsl-semantics/tests/support/tc1070.rs"]
#[allow(
    dead_code,
    clippy::duplicate_mod,
    reason = "the shared TC-1070 model fixture is used by sibling integration tests"
)]
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

const SOURCE: &str = r#"language "ix:native" edition "1-draft";
profile p = "filament.protocol.finite-global/v1";
model M = "test/orders";
protocol Flow using p over (view: M::Node) on origin {
  role Service on M::Node;
  run sequence Main {
    repeat Poll by Service visible (ready.ready) max 2 while { ready.ready }
      sequence Body { event Ready by Service as (ready: M::Flag) { true }; }
    exhausted sequence Done { }
  }
  finish End as (outcome: Boolean) { true };
}
"#;

struct Fixture {
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: fsl_semantics::model::observation::DocumentProvision,
    initial: DocumentRef,
}

fn fixture() -> Fixture {
    fixture_of(SOURCE, &tc1070::document())
}

fn fixture_of(source: &str, document: &serde_json::Value) -> Fixture {
    let domain = serde_json::to_vec(document).expect("domain fixture serializes");
    let packages = package_input([domain.as_slice()]).expect("domain package admitted");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-2220", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("unit parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("unit forms built");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("domain model selected");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qsl-2220".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("unit assembled");
    declarations
        .profile_selections
        .retain(|selection| selection.alias != "p");
    declarations.protocol_profile = None;
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let bytes = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "qsl-2220-repeat"},
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
    let snapshot_identity = document_reference(&bytes).expect("snapshot identity");
    let initial = DocumentRef {
        authority: "test".to_owned(),
        identity: "qsl-2220-repeat".to_owned(),
    };
    Fixture {
        package,
        packages,
        snapshots: BTreeMap::from([(snapshot_identity, bytes)]),
        initial,
    }
}

fn simulation(fixture: &Fixture) -> (ModelSimulationRequest<'_>, ModelInstance) {
    let population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let request = ModelSimulationRequest {
        emit: fsl_package::EmitLimits::fixture(),
        input: fsl_eval::simulation::model::ModelCheckInput::source(&fixture.package),
        packages: &fixture.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &fixture.snapshots,
        initial: vec![fixture.initial.clone()],
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
    let over = ModelInstance {
        population,
        key: "node-1".to_owned(),
    };
    (request, over)
}

fn system(fixture: &Fixture) -> ProtocolSystem<'_, Sc> {
    let (request, over) = simulation(fixture);
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let protocol = fixture.package.graph().protocol("Flow").expect("protocol");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Flow".to_owned(),
        over,
        roles: vec![RoleBinding {
            role: "Service".to_owned(),
            model: tc1070::model::PACKAGE.to_owned(),
        }],
        capacities: Vec::new(),
        memory: ResolvedMemory::resolve(protocol, &[]).expect("SC memory selected"),
    };
    ProtocolSystem::new(model, &subject, Sc).expect("protocol subject built")
}

fn terminal_event_paths(reachable: &Reachable<Sc>) -> Vec<Vec<String>> {
    fn visit(
        reachable: &Reachable<Sc>,
        at: usize,
        prefix: &mut Vec<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        let state = &reachable.states[at];
        if state.steps.is_empty() {
            paths.push(prefix.clone());
            return;
        }
        for (transition, next) in &state.steps {
            let named = match transition.kind() {
                StepKind::Event | StepKind::Finish => transition
                    .node()
                    .and_then(|node| node.rsplit('/').next())
                    .map(str::to_owned),
                _ => None,
            };
            let tracked = named.is_some();
            if let Some(name) = named {
                prefix.push(name);
            }
            visit(reachable, *next, prefix, paths);
            if tracked {
                prefix.pop();
            }
        }
    }

    let mut paths = Vec::new();
    visit(reachable, 0, &mut Vec::new(), &mut paths);
    paths
}

/// FR-880-AC-8 / FR-882-AC-6: exploration enters the body before evaluating
/// the post-test guard, then repeats only when the latest body binder is true.
#[trace("FR-880-AC-8", "FR-882-AC-6", "TC-2506")]
#[test]
fn the_newly_compiled_forms_are_explored() {
    let fixture = fixture();
    let system = system(&fixture);
    let reachable = system
        .reachable(
            NonZeroUsize::new(1).expect("one worker"),
            fsl_eval::simulation::Limits::default().max_states,
        )
        .expect("finite repeat exploration completes");
    let paths = terminal_event_paths(&reachable);
    assert!(
        paths
            .iter()
            .any(|steps| steps.iter().filter(|name| *name == "Ready").count() == 1),
        "a false latest binder exits after one body pass: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|steps| steps.iter().filter(|name| *name == "Ready").count() == 2),
        "a true latest binder permits the second pass: {paths:?}"
    );
    assert!(
        reachable.states.iter().any(|state| state.steps.is_empty()),
        "the finite exploration settles after the repeat"
    );
}

/// FR-125 "Instancing": the reference a protocol's `over (view: M::Base)`
/// binds at the key `node-1` has the type of the admitted record, `Node`.
#[trace("TC-520", "FR-125-AC-9")]
#[test]
fn a_protocol_over_binding_names_the_admitted_member_type() {
    let mut document = tc1070::document();
    let types = document["types"].as_array_mut().expect("types");
    let node_id = serde_json::Value::from(tc1070::model::id("Node"));
    let node = types
        .iter_mut()
        .find(|declared| declared["identity"] == node_id)
        .expect("Node is declared");
    node["supertypes"] = json!([tc1070::model::id("Base")]);
    let mut base = node.clone();
    base["identity"] = json!(tc1070::model::id("Base"));
    base["displayName"] = json!("Base");
    base["supertypes"] = json!([]);
    base["identityFields"] = json!([]);
    base["fields"] = json!([]);
    base["operations"] = json!([]);
    base["relationships"] = json!([]);
    base["origin"] = tc1070::model::origin("Base");
    types.push(base);
    let source = SOURCE.replace("over (view: M::Node)", "over (view: M::Base)");
    let fixture = fixture_of(&source, &document);

    let (request, over) = simulation(&fixture);
    let reference = over_reference(request, "Flow", &over).expect("the instance binds");

    assert_eq!(
        reference.object_type(),
        &EffectiveId::new(
            None,
            OriginalDeclarationKey::new(tc1070::model::PACKAGE, tc1070::model::id("Node")),
        )
    );
}
