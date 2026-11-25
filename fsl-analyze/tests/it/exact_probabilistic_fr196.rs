// SPDX-License-Identifier: AGPL-3.0-or-later
//! Focused FR-196 acceptance coverage isolated from FR191's shared tests.

use base64::Engine as _;
use filament_exact::Cancel;
use fsl_analyze::exact_probabilistic::{build_product, ProbBuild, ProductRefusal, ProductStop};
use fsl_eval::simulation::model::ModelSystem;
use fsl_replay::{
    ActivationPhase, ModelCheckLimit, ModelCheckLimits, ProbabilityDocumentIoLimits,
    ProbabilityDocumentLimits, ProbabilityProductKey,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::prob_world::{self, Model, Operation, World};

const TERMINATES: &str = "probabilistic Terminates using inf on M over (p: M::Proc) \
     under every scheduler on origin { probability >= 0.99 [ eventually holds(p.done) ] }";

fn conditional_model() -> World {
    const PACKAGE: &str = "test/fr196-conditional";
    let operations = vec![
        Operation {
            name: "a",
            parameters: vec![("success", prob_world::BOOLEAN.to_owned())],
            modifies: vec!["activated", "holds", "dead"],
            deletes: false,
        },
        Operation {
            name: "b",
            parameters: vec![("success", prob_world::BOOLEAN.to_owned())],
            modifies: vec!["activated", "holds", "dead"],
            deletes: false,
        },
    ];
    World::new(&Model {
        package: PACKAGE,
        object: "T",
        population: "ts",
        key: "x",
        integers: vec![],
        fields: vec![
            ("activated", prob_world::BOOLEAN.to_owned()),
            ("holds", prob_world::BOOLEAN.to_owned()),
            ("dead", prob_world::BOOLEAN.to_owned()),
        ],
        operations,
        body: "pre AReady using v on M::T::a { not self.activated and not self.dead }\n\
             post AResult using v on M::T::a { self.activated = success and self.holds and self.dead = not success }\n\
             pre BReady using v on M::T::b { not self.activated and not self.dead }\n\
             post BResult using v on M::T::b { self.activated and self.holds = success and not self.dead }\n\
             random T::a(success) ~ { true: 1, false: 3 };\n\
             random T::b(success) ~ { true: 1, false: 1 };\n\
             terminal Done using v on M when { self.activated or self.dead };\n\
             workload Even on M { weight T::a = 1; weight T::b = 1; }\n\
             probabilistic Workload using inf on M under Even on each (x: M::T) \
         when (x.activated) { probability >= rational(3, 5) \
             [ eventually holds(x.holds) ] }\n".to_owned(),
        initial: json!({
            "activated": {"boolean": false},
            "holds": {"boolean": false},
            "dead": {"boolean": false}
        }),
    })
}

fn model_system<'a>(
    world: &'a World,
    claim_name: &str,
) -> (
    ModelSystem<'a>,
    &'a fsl_semantics::check::CheckedProbabilisticClaim,
) {
    let request = world.request(claim_name, ModelCheckLimits::default());
    let claim = world.claim(claim_name);
    let system = ModelSystem::new(request.subject.simulation).expect("the checked model admits");
    (system, claim)
}

/// FR-196-AC-16's workload half of TC-3060: the independently specified
/// masses are 3/8 and 5/8, so the checked conditional value is exactly 3/5.
#[trace("TC-3060")]
#[test]
fn conditional_workload_has_literal_three_fifths_value() {
    let world = conditional_model();
    let (system, claim) = model_system(&world, "Workload");
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let claims = world.package.graph().probabilistic_model().claims();
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let outcome = fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
        .expect("the finite workload model settles");
    let fsl_replay::OnEachOutcome::Holds { document } = outcome else {
        panic!("the independently calculated conditional value is 3/5");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical JSON");
    assert_eq!(document["conditional"]["mode"], "ratio");
    let rows = |part: &str| {
        let entries = document["conditional"][part]["values"]
            .as_array()
            .expect("exact value rows")
            .iter()
            .map(|row| {
                (
                    row["state"].as_str().expect("canonical state key"),
                    (
                        row["value"]["numerator"].as_str().expect("numerator"),
                        row["value"]["denominator"].as_str().expect("denominator"),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let unique = entries
            .iter()
            .map(|(state, _)| *state)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            entries.len(),
            unique.len(),
            "{part} has no duplicate state rows"
        );
        entries.into_iter().collect::<BTreeMap<_, _>>()
    };
    let numerator = rows("numerator");
    let denominator = rows("denominator");
    assert_eq!(
        numerator.keys().collect::<Vec<_>>(),
        denominator.keys().collect::<Vec<_>>(),
        "the two exact parts name the same phase states"
    );

    let initial = system
        .initial_states()
        .first()
        .expect("the model has its literal initial state");
    let (initial_model, _) = system.state_key(initial, fsl_foundation::IdentityLimits::default()).expect("initial model key");
    let instance = system
        .reference_model()
        .object_reference(
            &fsl_replay::DeclarationKey {
                package: "test/fr196-conditional".to_owned(),
                node: "ix://test/fr196-conditional/T".to_owned(),
            },
            "x",
        )
        .expect("the declared initial instance reference");
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.bytes,
        storage_bytes: limits.storage_bytes,
    };
    let expected_initial_waiting_key = ProbabilityProductKey::for_model_state(None, &initial_model)
        .expect("literal initial model state key")
        .with_activation(&instance, ActivationPhase::Waiting, io, &Cancel::new())
        .expect("initial key in the on-each waiting phase");
    let expected_state = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(expected_initial_waiting_key.as_bytes());
    assert_eq!(
        numerator.get(expected_state.as_str()).copied(),
        Some(("3", "8")),
        "the initial waiting state has numerator mass 3/8"
    );
    assert_eq!(
        denominator.get(expected_state.as_str()).copied(),
        Some(("5", "8")),
        "the same initial waiting state has activation mass 5/8"
    );
}

/// TC-631's unbounded exact-only event product carries genuine absent
/// monitor and accumulator members, rather than synthesized control values.
#[trace("TC-631")]
#[test]
fn exact_only_event_keys_keep_none_controls() {
    let world = World::new(&prob_world::process(TERMINATES));
    let result = build_product(
        world.request("Terminates", ModelCheckLimits::default()),
        &Cancel::new(),
    );
    let ProbBuild::Built(product) = result.expect("the exact-only event product builds") else {
        panic!("the request builds a product");
    };
    assert_eq!(product.labels, [[false], [true]]);
    assert!(product.states.iter().all(|key| {
        key.monitor.is_none() && key.accumulator.is_none() && key.pending.is_none()
    }));
}

/// FR-196-AC-9's transition-budget clause: a stop before the first edge
/// returns only the typed refusal, with no default or truncated product key.
#[trace("TC-631")]
#[test]
fn a_product_budget_stop_returns_no_partial_key() {
    let world = World::new(&prob_world::process(TERMINATES));
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 0;
    let result = build_product(world.request("Terminates", limits), &Cancel::new());
    assert!(
        matches!(
            &result,
            Err(ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxTransitions,
                value: 0,
            }))
        ),
        "expected exact_prob.max_transitions=0 refusal, received {result:?}"
    );
}
