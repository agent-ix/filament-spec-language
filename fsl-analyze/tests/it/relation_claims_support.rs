// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared relation claim fixtures span split integration test binaries"
)]
//! Shared model-relation fixture helpers for integration targets.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedHyper, CheckedModelRelation, check_hyper_clauses};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{PackageInput, package_input};
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use serde_json::Value as Json;

use crate::counter::UNLIMITED;
use crate::hyper_world::{self, Vault};

pub(crate) const DETERMINISTIC: &str = "relation Det using v over (x: V::Vault::step, y: V::Vault::step) \
    { x.i = y.i and pre(x.self.h) = pre(y.self.h) implies x.self.l = y.self.l }";

pub(crate) fn package_and_relation(
    vault: Vault,
    text: &str,
) -> (CheckedPackage, CheckedModelRelation, Vec<u8>) {
    let document = hyper_world::document(false, false);
    package_and_relation_with_document(document, vault, text)
}

pub(crate) fn package_and_relation_with_document(
    document: Vec<u8>,
    vault: Vault,
    text: &str,
) -> (CheckedPackage, CheckedModelRelation, Vec<u8>) {
    let documents = [document.as_slice()];
    let unit = hyper_world::unit_text(None, vault, false, text);
    let graph = hyper_world::declarations(&documents, &unit)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .expect("the relation unit checks");
    let package = CheckedPackage::link(graph);
    let declarations = hyper_world::declarations(&documents, &unit);
    let mut checked = check_hyper_clauses(&declarations);
    let Some(Ok(CheckedHyper::Relation(relation))) = checked.pop() else {
        panic!("the relation is checked: {checked:?}");
    };
    assert!(checked.is_empty(), "one checked relation");
    (package, *relation, document)
}

pub(crate) fn subjects<'a>(
    package: &'a CheckedPackage,
    package_input: &'a PackageInput,
    snapshots: &'a DocumentProvision,
    initial: &[DocumentRef],
) -> Vec<ModelSubject<'a>> {
    (0..1)
        .map(|_| ModelSubject {
            simulation: ModelSimulationRequest {
                input: fsl_eval::simulation::model::ModelCheckInput::source(package),
                packages: package_input,
                normalization: ModelNormalizationLimits::default(),
                snapshots,
                initial: initial.to_vec(),
                universes: vec![PopulationUniverse {
                    population: hyper_world::key("vaults"),
                    keys: vec!["v".to_owned()],
                }],
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
                meter: UNLIMITED,
                emit: fsl_package::EmitLimits::fixture(),
            },
            proof_bounds: Vec::new(),
        })
        .collect()
}

pub(crate) fn initial_world(
    document: &[u8],
) -> (
    fsl_semantics::model::intake::PackageInput,
    DocumentProvision,
    Vec<DocumentRef>,
) {
    let packages = package_input([document]).expect("the package input admits");
    let mut snapshots = DocumentProvision::default();
    let initial = vec![
        hyper_world::snapshot(&mut snapshots, "zero", &[("v", (0, 0))], None, None),
        hyper_world::snapshot(&mut snapshots, "one", &[("v", (1, 0))], None, None),
    ];
    (packages, snapshots, initial)
}

pub(crate) fn step_with_result_document() -> Vec<u8> {
    let mut document: Json =
        serde_json::from_slice(&hyper_world::document(false, false)).expect("fixture JSON");
    document["types"][1]["operations"][0]["returns"] = serde_json::json!({
        "typeRef": hyper_world::ident("Bit"),
        "nullable": false,
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
    });
    serde_json::to_vec(&document).expect("the result-bearing fixture serializes")
}
