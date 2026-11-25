// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2238: shared concrete tuples agree between EN-1 and FR-315's SMT form.

use filament_exact::{ObjectId, ObjectReference, Value};
use fsl_analyze::model_check::{
    check_model_relation, ModelRelationCheckRequest, ModelRelationOutcome,
};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use fsl_replay::ModelCheckLimits;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::DocumentProvision;
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{self, Vault};
use crate::relation_claims_support as relation_claims;

#[trace("TC-604", "FR-179-AC-2")]
#[test]
fn explicit_relation_refutation_matches_the_smt_query_for_its_tuple() {
    let declaration = "relation SameInput using v over \
        (x: V::Vault::step, y: V::Vault::step) { x.i = y.i }";
    let (package, relation, document) =
        relation_claims::package_and_relation(Vault::Secure, declaration);
    let documents = [document.as_slice()];
    let packages = package_input(documents).expect("the package input admits");
    let mut snapshots = DocumentProvision::default();
    let initial = vec![
        hyper_world::snapshot(&mut snapshots, "zero", &[("v", (0, 0))], None, None),
        hyper_world::snapshot(&mut snapshots, "one", &[("v", (1, 0))], None, None),
    ];
    let subject = ModelSubject {
        simulation: ModelSimulationRequest {
            input: fsl_eval::simulation::model::ModelCheckInput::source(&package),
            packages: &packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &snapshots,
            initial,
            universes: vec![PopulationUniverse {
                population: hyper_world::key("vaults"),
                keys: vec!["v".to_owned()],
            }],
            observation: fsl_semantics::model::observation::ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
            emit: fsl_package::EmitLimits::fixture(),
        },
        proof_bounds: Vec::new(),
    };
    let outcome = check_model_relation(
        ModelRelationCheckRequest {
            relation: &relation,
            package: &package,
            subjects: vec![ModelSubject {
                simulation: subject.simulation.clone(),
                proof_bounds: Vec::new(),
            }],
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &filament_exact::Cancel::new(),
    )
    .expect("the explicit relation is admitted");
    let ModelRelationOutcome::Refuted { counterexample, .. } = outcome else {
        panic!("the relation finds unequal inputs: {outcome:?}");
    };

    let mut arguments = Vec::new();
    let (_, receiver_type) = &relation.body_parameters()[0];
    let filament_exact::ValueType::Reference(receiver_type) = receiver_type else {
        panic!("the relation's receiver parameter is a reference");
    };
    let receiver = ObjectReference::new(
        crate::fixture_ids::universe_id("relation-smt"),
        receiver_type.clone(),
        ObjectId::new("v").expect("nonempty object identity"),
    );
    for (trace, (variable, _)) in counterexample.traces.iter().zip(relation.executions()) {
        let Some(fsl_replay::HyperStep::Model(step)) = trace.steps.last() else {
            panic!("each relation trace ends with a transition");
        };
        arguments.push(Value::Reference(receiver.clone()));
        let (_, argument_type) = &relation.body_parameters()[arguments.len()];
        let value: serde_json::Value = serde_json::from_str(&step.transition.arguments()[0])
            .expect("the canonical integer argument parses");
        let integer = value["value"]
            .as_str()
            .expect("integer wire value is a decimal string")
            .parse::<i64>()
            .expect("fixture integer fits i64");
        let argument = Value::Integer(filament_exact::Integer::from(integer));
        assert!(argument_type.admits(&argument));
        arguments.push(argument);
        assert!(trace.variable == *variable);
    }
    let query = fsl_eval::smt::encode_relation_tuple(&relation, &subject, &arguments)
        .expect("the relation tuple is in the FR-315 encoding fragment");
    let query = String::from_utf8(query).expect("the SMT query is UTF-8");
    assert!(
        query.contains("(assert (not (= 0 1)))"),
        "the false tuple has a satisfiable refutation query: {query}"
    );
}
