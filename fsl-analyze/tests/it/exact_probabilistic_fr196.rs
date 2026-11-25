// SPDX-License-Identifier: AGPL-3.0-or-later
//! Focused FR-196 acceptance coverage isolated from FR191's shared tests.

use base64::Engine as _;
use filament_exact::Cancel;
use fsl_analyze::exact_probabilistic::{ProbBuild, ProductRefusal, ProductStop, build_product};
use fsl_eval::simulation::model::ModelSystem;
use fsl_replay::{
    ActivationPhase, ModelCheckLimit, ModelCheckLimits, ProbabilityDocumentIoLimits,
    ProbabilityDocumentLimits, ProbabilityProductKey,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

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
        body: format!(
            "pre AReady using v on M::T::a {{ not self.activated and not self.dead }}\n\
             post AResult using v on M::T::a {{ self.activated = success and self.holds and self.dead = not success }}\n\
             pre BReady using v on M::T::b {{ not self.activated and not self.dead }}\n\
             post BResult using v on M::T::b {{ self.activated and self.holds = success and not self.dead }}\n\
             random T::a(success) ~ {{ true: 1, false: 3 }};\n\
             random T::b(success) ~ {{ true: 1, false: 1 }};\n\
             terminal Done using v on M when {{ self.activated or self.dead }};\n\
             workload Even on M {{ weight T::a = 1; weight T::b = 1; }}\n\
             probabilistic Workload using inf on M under Even on each (x: M::T) \
         when (x.activated) {{ probability >= rational(3, 5) \
             [ eventually holds(x.holds) ] }}\n"
        ),
        initial: json!({
            "activated": {"boolean": false},
            "holds": {"boolean": false},
            "dead": {"boolean": false}
        }),
    })
}

fn reset_conditioned_model() -> World {
    const PACKAGE: &str = "test/fr196-reset-conditioned";
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
        Operation {
            name: "stay",
            parameters: vec![],
            modifies: vec!["activated", "holds", "dead"],
            deletes: false,
        },
    ];
    let claims = concat!(
        "probabilistic Min using inf on M under every scheduler on each (x: M::T) when (x.activated) { probability >= rational(1, 2) [ eventually holds(x.holds) ] }\n",
        "probabilistic Max using inf on M under every scheduler on each (x: M::T) when (x.activated) { probability <= rational(9, 10) [ eventually holds(x.holds) ] }\n",
    );
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
        body: format!(
            "pre AReady using v on M::T::a {{ not self.activated and not self.dead }}\n\
             post AResult using v on M::T::a {{ self.activated = success and self.holds = success and self.dead = not success }}\n\
             pre BReady using v on M::T::b {{ not self.activated and not self.dead }}\n\
             post BResult using v on M::T::b {{ self.activated and self.holds = success and not self.dead }}\n\
             pre StayReady using v on M::T::stay {{ not self.activated and not self.dead }}\n\
             post Stayed using v on M::T::stay {{ not self.activated and not self.holds and not self.dead }}\n\
             random T::a(success) ~ {{ true: 1, false: 3 }};\n\
             random T::b(success) ~ {{ true: 1, false: 1 }};\n\
             terminal Done using v on M when {{ self.activated or self.dead }};\n{claims}"
        ),
        initial: json!({
            "activated": {"boolean": false},
            "holds": {"boolean": false},
            "dead": {"boolean": false}
        }),
    })
}

fn active_cycle_conditioned_model() -> World {
    const PACKAGE: &str = "test/fr196-active-cycle";
    let operations = vec![
        Operation {
            name: "start",
            parameters: vec![],
            modifies: vec!["activated"],
            deletes: false,
        },
        Operation {
            name: "finish",
            parameters: vec![("success", prob_world::BOOLEAN.to_owned())],
            modifies: vec!["holds"],
            deletes: false,
        },
        Operation {
            name: "abandon",
            parameters: vec![],
            modifies: vec!["dead"],
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
        body: concat!(
            "pre StartReady using v on M::T::start { not self.activated }\n",
            "post Started using v on M::T::start { self.activated = true }\n",
            "pre FinishReady using v on M::T::finish { self.activated and not self.holds and not self.dead }\n",
            "post FinishResult using v on M::T::finish { self.holds = success }\n",
            "pre AbandonReady using v on M::T::abandon { self.activated and not self.holds and not self.dead }\n",
            "post Abandoned using v on M::T::abandon { self.dead = true }\n",
            "random T::finish(success) ~ { true: 1, false: 1 };\n",
            "terminal Done using v on M when { self.holds or self.dead };\n",
            "probabilistic CycleMin using inf on M under every scheduler on each (x: M::T) ",
            "when (x.activated) { probability >= rational(1, 100) [ eventually holds(x.holds) ] }\n",
            "probabilistic CycleMax using sup on M under every scheduler on each (x: M::T) ",
            "when (x.activated) { probability <= rational(99, 100) [ eventually holds(x.holds) ] }\n",
        )
        .to_owned(),
        initial: json!({
            "activated": {"boolean": false},
            "holds": {"boolean": false},
            "dead": {"boolean": false}
        }),
    })
}

fn reset_conditioned_scc_model(
    multi_root: bool,
    active_roots: [bool; 2],
    vacuous: bool,
    duplicate_root: bool,
) -> World {
    const PACKAGE: &str = "test/fr196-reset-scc";
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
        Operation {
            name: "cycle",
            parameters: vec![("next", prob_world::BOOLEAN.to_owned())],
            modifies: vec!["phase"],
            deletes: false,
        },
    ];
    let model = Model {
        package: PACKAGE,
        object: "T",
        population: "ts",
        key: "x",
        integers: vec![],
        fields: vec![
            ("activated", prob_world::BOOLEAN.to_owned()),
            ("holds", prob_world::BOOLEAN.to_owned()),
            ("dead", prob_world::BOOLEAN.to_owned()),
            ("phase", prob_world::BOOLEAN.to_owned()),
        ],
        operations,
        body: concat!(
            "pre AReady using v on M::T::a { not self.activated and not self.dead }\n",
            "post AResult using v on M::T::a { self.activated = success and self.holds = success and self.dead = not success }\n",
            "pre BReady using v on M::T::b { not self.activated and not self.dead }\n",
            "post BResult using v on M::T::b { self.activated and self.holds = success and not self.dead }\n",
            "pre CycleReady using v on M::T::cycle { not self.activated and not self.dead and self.phase != next }\n",
            "post CycleResult using v on M::T::cycle { self.phase = next }\n",
            "random T::a(success) ~ { true: 1, false: 3 };\n",
            "random T::b(success) ~ { true: 1, false: 1 };\n",
            "terminal Done using v on M when { self.activated or self.dead };\n",
            "probabilistic Min using inf on M under every scheduler on each (x: M::T) ",
            "when (x.activated) { probability >= rational(1, 2) ",
            "[ eventually holds(x.holds) ] }\n",
        )
        .to_owned(),
        initial: json!({
            "activated": {"boolean": false},
            "holds": {"boolean": false},
            "dead": {"boolean": false},
            "phase": {"boolean": false}
        }),
    };
    if multi_root {
        let mut first = model.initial.clone();
        first["activated"] = json!({"boolean": active_roots[0]});
        first["holds"] = json!({"boolean": active_roots[0]});
        if vacuous {
            first["dead"] = json!({"boolean": true});
        }
        let mut second = json!({
            "activated": {"boolean": active_roots[1]},
            "holds": {"boolean": active_roots[1]},
            "dead": {"boolean": vacuous},
            "phase": {"boolean": true}
        });
        if !vacuous {
            second["dead"] = json!({"boolean": false});
        }
        if duplicate_root {
            second = first.clone();
        }
        World::with_initials(
            &model,
            &[second, first],
        )
    } else {
        let mut initial = model.initial.clone();
        initial["activated"] = json!({"boolean": active_roots[0]});
        initial["holds"] = json!({"boolean": active_roots[0]});
        World::with_initials(&model, &[initial])
    }
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
#[trace("TC-3060", "FR-196-AC-16")]
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
        for (state, _) in &entries {
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(state)
                .expect("state key base64");
            let key: Value = serde_json::from_slice(&bytes).expect("canonical product key");
            assert_eq!(key.as_array().expect("key array").len(), 8);
        }
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
    let (initial_model, _) = system.state_key(initial).expect("initial model key");
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

/// FR-196-AC-16: the every-scheduler reset process returns the unreachable
/// post-a state to s0, admits Reset in the stay end component, and certifies
/// the proper conditioned extrema 1/2 and 1.
#[trace("TC-3060", "FR-196-AC-16")]
#[test]
fn reset_conditioned_extrema_are_proper_and_checker_verified() {
    let world = reset_conditioned_model();
    let claims = world.package.graph().probabilistic_model().claims();
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let run = |name: &str| {
        let claim = world.claim(name);
        let system = ModelSystem::new(
            world
                .request(name, ModelCheckLimits::default())
                .subject
                .simulation,
        )
        .expect("the reset model admits");
        let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
        let outcome =
            fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
                .expect("the reset-conditioned optimum is produced");
        let fsl_replay::OnEachOutcome::Holds { document } = outcome else {
            panic!("the conditional extremum has an optimum certificate");
        };
        let decoded: Value = serde_json::from_slice(&document).expect("canonical certificate");
        assert_eq!(decoded["conditional"]["mode"], "optimum");
        assert_eq!(
            decoded["objective"]["optimization"],
            if name == "Min" { "minimum" } else { "maximum" }
        );
        let reset = decoded["conditional"]["reset"]["values"]
            .as_array()
            .expect("reset values");
        assert_eq!(
            reset.len(),
            2,
            "s0 and the unreachable after-a waiting state are covered"
        );
        let values = reset
            .iter()
            .map(|row| {
                (
                    row["value"]["numerator"].as_str().expect("numerator"),
                    row["value"]["denominator"].as_str().expect("denominator"),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        if name == "Min" {
            assert_eq!(values, std::collections::BTreeSet::from([("1", "2")]));
        } else {
            assert_eq!(values, std::collections::BTreeSet::from([("1", "1")]));
        }
        let policy = decoded["conditional"]["reset"]["policy"]
            .as_array()
            .expect("proper reset policy");
        assert_eq!(policy.len(), 2, "the policy covers both waiting states");
        assert_eq!(
            policy
                .iter()
                .filter(|row| row["choice"] == json!(["reset"]))
                .count(),
            1,
            "the unreachable after-a state takes the reset action"
        );
        let checked = fsl_replay::check_probability_certificate_document(
            &system,
            claims,
            &item,
            &document,
            limits,
            &Cancel::new(),
        )
        .expect("the independent checker reads the optimum");
        (decoded, checked)
    };
    let (_, accepted) = run("Min");
    assert!(matches!(
        accepted,
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
    ));
    let (_, rejected) = run("Max");
    assert!(matches!(
        rejected,
        fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
            if rejection.rule == fsl_replay::ProbabilityCertificateRule::ThresholdMissed
    ));
}

/// FR-196-AC-16: active post extrema distinguish a geometric success loop
/// from a scheduler action that abandons to a non-target terminal.
#[trace("TC-3060", "FR-196-AC-16")]
#[test]
fn active_cycle_post_minimum_and_maximum_are_produced_and_checked() {
    let world = active_cycle_conditioned_model();
    let claims = world.package.graph().probabilistic_model().claims();
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    for (name, expected_root) in [("CycleMin", ("0", "1")), ("CycleMax", ("1", "1"))] {
        let claim = world.claim(name);
        let system = ModelSystem::new(
            world
                .request(name, ModelCheckLimits::default())
                .subject
                .simulation,
        )
        .expect("the active-cycle model admits");
        let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
        let outcome = fsl_replay::on_each_outcome(
            &system,
            claims,
            &item,
            limits,
            64,
            &Cancel::new(),
        )
        .expect("the cyclic post optimum is produced");
        let fsl_replay::OnEachOutcome::Holds { document } = outcome else {
            panic!("the cyclic conditioned extremum has a certificate");
        };
        let decoded: Value = serde_json::from_slice(&document).expect("proof document");
        let post = decoded["conditional"]["post"]["values"]
            .as_array()
            .expect("post value rows");
        let mut root = None;
        let mut target = None;
        let mut abandoned = None;
        let values = post
            .iter()
            .map(|row| {
                let encoded = row["state"].as_str().expect("post state");
                let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(encoded)
                    .expect("product key base64");
                let key: Value = serde_json::from_slice(&bytes).expect("product key JSON");
                let fields = key[4][1]["semantic"]["entries"][0]["value"]["entries"][0]["value"]["fields"]
                    .as_array()
                    .expect("embedded object fields");
                let field = |name: &str| {
                    fields
                        .iter()
                        .find(|field| field["name"] == name)
                        .and_then(|field| field["value"]["value"].as_bool())
                        .expect("Boolean model field")
                };
                let value = (
                    row["value"]["numerator"].as_str().expect("numerator"),
                    row["value"]["denominator"].as_str().expect("denominator"),
                );
                if field("activated") && !field("holds") && !field("dead") {
                    root = Some(encoded.to_owned());
                } else if field("holds") {
                    target = Some((encoded.to_owned(), value));
                } else if field("dead") {
                    abandoned = Some((encoded.to_owned(), value));
                }
                (encoded.to_owned(), value)
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let root = root.expect("the active conditioned root has a post row");
        assert_eq!(values[&root], expected_root, "literal active root extremum for {name}");
        let (target, target_value) = target.expect("the success state has a post row");
        let (abandoned, abandoned_value) = abandoned.expect("the abandoned state has a post row");
        assert_eq!(values[&target], ("1", "1"), "literal target value for {name}");
        assert_eq!(target_value, ("1", "1"), "literal target row for {name}");
        assert_eq!(values[&abandoned], ("0", "1"), "literal abandon value for {name}");
        assert_eq!(abandoned_value, ("0", "1"), "literal abandon row for {name}");
        assert!(matches!(
            fsl_replay::check_probability_certificate_document(
                &system,
                claims,
                &item,
                &document,
                limits,
                &Cancel::new(),
            )
            .expect("independent checker reads the cyclic post proof"),
            fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
                if rejection.rule == fsl_replay::ProbabilityCertificateRule::ThresholdMissed
        ));
    }
}

/// FR-196-AC-16: the two-state waiting SCC has a reset candidate at both
/// states; a policy may reset from the non-root member to the root and then
/// make progress to activation.
#[trace("FR-196-AC-16", "FR-412-AC-41")]
#[test]
fn reset_action_covers_every_state_in_a_waiting_scc() {
    let world = reset_conditioned_scc_model(false, [false, false], false, false);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the SCC reset model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("the proper SCC optimum settles")
    else {
        panic!("the conditioned SCC optimum has a certificate");
    };
    let mut document: Value = serde_json::from_slice(&document).expect("canonical proof");
    let reset_values = document["conditional"]["reset"]["values"]
        .as_array()
        .expect("reset value rows");
    assert_eq!(
        reset_values.len(),
        4,
        "both SCC phases and both reachable post-a waiting states are covered"
    );
    for row in reset_values {
        assert_eq!(
            (
                row["value"]["numerator"].as_str(),
                row["value"]["denominator"].as_str()
            ),
            (Some("1"), Some("2")),
            "each SCC phase has the literal conditioned minimum 1/2"
        );
    }
    let initial = system.initial_states().first().expect("initial state");
    let (model_bytes, _) = system.state_key(initial).expect("initial model key");
    let instance = system
        .reference_model()
        .object_reference(
            &fsl_replay::DeclarationKey {
                package: "test/fr196-reset-scc".to_owned(),
                node: "ix://test/fr196-reset-scc/T".to_owned(),
            },
            "x",
        )
        .expect("initial on-each binding");
    let root_key = ProbabilityProductKey::for_model_state(None, &model_bytes)
        .expect("root product key")
        .with_activation(
            &instance,
            ActivationPhase::Waiting,
            ProbabilityDocumentIoLimits {
                bytes: limits.bytes,
                storage_bytes: limits.storage_bytes,
            },
            &Cancel::new(),
        )
        .expect("root waiting key");
    let root_state = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(root_key.as_bytes());
    let policy = document["conditional"]["reset"]["policy"]
        .as_array_mut()
        .expect("policy rows");
    assert_eq!(
        policy.len(),
        4,
        "the deterministic policy covers every waiting state"
    );
    let root_index = policy
        .iter()
        .position(|row| row["state"] == root_state)
        .expect("the policy names the initial SCC member");
    assert_ne!(
        policy[root_index]["choice"],
        json!(["reset"]),
        "the root policy makes progress into the active graph"
    );
    for index in 0..policy.len() {
        if index != root_index {
            policy[index]["choice"] = json!(["reset"]);
        }
    }
    assert_eq!(
        policy
            .iter()
            .filter(|row| row["choice"] == json!(["reset"]))
            .count(),
        3,
        "both non-root SCC members and the two dead waiting states carry Reset"
    );
    let ranks = document["conditional"]["reset"]["ranking"]
        .as_array_mut()
        .expect("reset ranks");
    ranks[root_index]["rank"] = json!(1);
    for index in 0..ranks.len() {
        if index != root_index {
            ranks[index]["rank"] = json!(2);
        }
    }
    let bytes = serde_json::to_vec(&document).expect("proof re-encodes");
    let checked = fsl_replay::check_probability_certificate_document(
        &system,
        claims,
        &item,
        &bytes,
        limits,
        &Cancel::new(),
    )
    .expect("checker reads the reset policy");
    assert!(
        matches!(
            checked,
            fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
        ),
        "the checker independently admits Reset from the other SCC member: {checked:?}"
    );
}

/// A reset process spanning multiple initial roots carries the root index in
/// each complete product key, including states reached from both roots.
#[trace(
    "FR-196-AC-18",
    "FR-412-AC-35",
    "FR-412-AC-36",
    "FR-412-AC-37",
    "FR-412-AC-38",
    "FR-412-AC-39",
    "FR-412-AC-40"
)]
#[test]
fn reset_extrema_keep_converged_rows_root_indexed_and_check_each_root() {
    let world = reset_conditioned_scc_model(true, [false, false], false, false);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the multi-root reset model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let mut state_limited = limits;
    state_limited.states = 1;
    assert!(matches!(
        fsl_replay::on_each_outcome(
            &system,
            claims,
            &item,
            state_limited,
            64,
            &Cancel::new()
        ),
        Err(fsl_replay::ProbabilityDocumentError::Graph(
            fsl_replay::ExactProbabilityGraphError::LimitReached
        ))
    ));
    let mut storage_limited = limits;
    storage_limited.storage_bytes = 0;
    assert!(matches!(
        fsl_replay::on_each_outcome(
            &system,
            claims,
            &item,
            storage_limited,
            64,
            &Cancel::new()
        ),
        Err(fsl_replay::ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::StorageBytes,
            ..
        })
    ));
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("the two-root reset optimum settles")
    else {
        panic!("the conditioned optimum has a certificate");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical proof");
    let conditional = &document["conditional"];
    let reset = &conditional["reset"];
    let declared_initial_keys = system
        .initial_states()
        .iter()
        .map(|state| {
            system
                .state_key(state)
                .expect("initial model key")
                .0
        })
        .collect::<Vec<_>>();
    assert!(
        declared_initial_keys
            .windows(2)
            .any(|pair| pair[0] > pair[1]),
        "fixture declaration order opposes canonical initial-key order"
    );
    let mut canonical_initial_keys = declared_initial_keys.clone();
    canonical_initial_keys.sort();
    canonical_initial_keys.dedup();
    let root_for_initial = canonical_initial_keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index as u64))
        .collect::<BTreeMap<_, _>>();
    let read_keys = |part: &str, member: &str| {
        conditional[part][member]
            .as_array()
            .expect("key rows")
            .iter()
            .map(|row| {
                let text = row["state"].as_str().expect("encoded product key");
                let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(text)
                    .expect("base64 product key");
                let key: Value = serde_json::from_slice(&bytes).expect("canonical product key");
                (row, key)
            })
            .collect::<Vec<_>>()
    };
    let mut indexed_states = BTreeMap::<Vec<u8>, BTreeMap<u64, usize>>::new();
    let mut seen_initial_roots = BTreeMap::<Vec<u8>, BTreeSet<u64>>::new();
    for (part, member) in [
        ("post", "values"),
        ("post", "ranking"),
        ("reset", "values"),
        ("reset", "ranking"),
        ("reset", "policy"),
    ] {
        for (row, key) in read_keys(part, member) {
            let key = key.as_array().expect("product key array");
            assert_eq!(key.len(), 9, "every reset-process key has root index");
            let root = key[8].as_u64().expect("canonical integer root index");
            assert!(root < 2, "root index is from FR-101 order");
            assert!(row.get("root").is_none(), "the row shape has no extra root field");
            let state = key[4].as_array().expect("embedded model-state key");
            let state_bytes = serde_json::to_vec(&state[1]).expect("model-state key bytes");
            if root_for_initial.contains_key(&state_bytes) {
                seen_initial_roots
                    .entry(state_bytes)
                    .or_default()
                    .insert(root);
            }
            let shared = serde_json::to_vec(&key[..8]).expect("legacy key prefix");
            *indexed_states.entry(shared).or_default().entry(root).or_default() += 1;
        }
    }
    assert!(
        indexed_states.values().any(|owners| owners.len() == 2),
        "a converged product state coexists once under each initial root"
    );
    for (key, expected) in root_for_initial {
        assert!(
            seen_initial_roots
                .get(&key)
                .is_some_and(|indexes| indexes.contains(&expected)),
            "each canonical initial root has its own indexed root key"
        );
    }
    let policy = reset["policy"].as_array().expect("policy rows");
    let mut reset_roots = std::collections::BTreeSet::new();
    for row in policy.iter().filter(|row| row["choice"] == json!(["reset"])) {
        let key: Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(row["state"].as_str().expect("policy state"))
                .expect("policy key base64"),
        )
        .expect("policy key JSON");
        let root = key.as_array().and_then(|parts| parts[8].as_u64()).expect("root index");
        assert!(root < 2);
        reset_roots.insert(root);
    }
    assert_eq!(reset_roots, std::collections::BTreeSet::from([0, 1]));
    let encoded = serde_json::to_vec(&document).expect("proof bytes");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &encoded, limits, &Cancel::new()
        )
        .expect("independent root-index checker"),
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
    ));

    for member in ["values", "ranking", "policy"] {
        let mut missing_root = document.clone();
        let rows = missing_root["conditional"]["reset"][member]
            .as_array_mut()
            .expect("reset rows");
        let root_zero = rows
            .iter()
            .position(|row| {
                let Some(text) = row["state"].as_str() else {
                    return false;
                };
                let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(text)
                else {
                    return false;
                };
                let Ok(key) = serde_json::from_slice::<Value>(&bytes) else {
                    return false;
                };
                key.as_array()
                    .and_then(|parts| parts.get(8))
                    .and_then(Value::as_u64)
                    == Some(0)
            })
            .expect("a root-zero row exists");
        let text = rows[root_zero]["state"].as_str().expect("state key");
        let mut key: Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(text)
                .expect("state key base64"),
        )
        .expect("state key JSON");
        key.as_array_mut().expect("key array").pop();
        rows[root_zero]["state"] = Value::String(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&key).expect("eight-member key")),
        );
        let missing_root = serde_json::to_vec(&missing_root).expect("mutated proof");
        let result = fsl_replay::check_probability_certificate_document(
            &system,
            claims,
            &item,
            &missing_root,
            limits,
            &Cancel::new(),
        )
        .expect("missing root is a typed rejection");
        assert!(matches!(
            result,
            fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
                if rejection.rule == fsl_replay::ProbabilityCertificateRule::Malformed
                    && rejection.at == fsl_replay::ProbabilityCertificateLocus::Member(member)
        ), "missing root in {member} is malformed at that exact member: {result:?}");
    }

    let mut omitted_reset = document.clone();
    omitted_reset["conditional"]["reset"]["values"] = json!([]);
    omitted_reset["conditional"]["reset"]["ranking"] = json!([]);
    omitted_reset["conditional"]["reset"]["policy"] = json!([]);
    let omitted_reset = serde_json::to_vec(&omitted_reset).expect("proof without reset rows");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system,
            claims,
            &item,
            &omitted_reset,
            limits,
            &Cancel::new()
        )
        .expect("omitted reset rows are a typed rejection"),
        fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
            if rejection.rule == fsl_replay::ProbabilityCertificateRule::Malformed
                && rejection.at == fsl_replay::ProbabilityCertificateLocus::Member("values")
    ));
}

/// If every initial root is already active, the optimum has no reset process
/// and keeps the legacy eight-member product-key bytes.
#[trace("FR-196-AC-18", "FR-412-AC-41")]
#[test]
fn active_multi_root_optimum_keeps_legacy_key_shape_without_reset() {
    let world = reset_conditioned_scc_model(true, [true, true], false, false);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the active multi-root model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("the active-root optimum settles")
    else {
        panic!("the active-root optimum holds");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical proof");
    let reset = &document["conditional"]["reset"];
    for member in ["values", "ranking", "policy"] {
        assert_eq!(reset[member].as_array().map(Vec::len), Some(0));
    }
    for row in document["conditional"]["post"]["values"]
        .as_array()
        .expect("post values")
    {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(row["state"].as_str().expect("post state"))
            .expect("product key base64");
        let key: Value = serde_json::from_slice(&bytes).expect("product key JSON");
        assert_eq!(key.as_array().expect("key array").len(), 8);
    }
    let instance = system
        .reference_model()
        .object_reference(
            &fsl_replay::DeclarationKey {
                package: "test/fr196-reset-scc".to_owned(),
                node: "ix://test/fr196-reset-scc/T".to_owned(),
            },
            "x",
        )
        .expect("initial on-each binding");
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.bytes,
        storage_bytes: limits.storage_bytes,
    };
    let expected = system
        .initial_states()
        .iter()
        .map(|initial| {
            let (model, _) = system.state_key(initial).expect("initial model key");
            let key = ProbabilityProductKey::for_model_state(None, &model)
                .expect("legacy product key")
                .with_activation(&instance, ActivationPhase::Active, io, &Cancel::new())
                .expect("legacy active key");
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes())
        })
        .collect::<std::collections::BTreeSet<_>>();
    let actual = document["conditional"]["post"]["values"]
        .as_array()
        .expect("post values")
        .iter()
        .map(|row| row["state"].as_str().expect("post key").to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected, "no-reset keys preserve legacy bytes exactly");
    let bytes = serde_json::to_vec(&document).expect("proof bytes");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &bytes, limits, &Cancel::new()
        )
        .expect("independent no-reset checker"),
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

/// Duplicate declaration roots coalesce to one canonical FR-101 root, so a
/// reset certificate keeps the legacy eight-member keys.
#[trace("FR-196-AC-18", "FR-412-AC-41")]
#[test]
fn duplicate_initial_roots_keep_legacy_reset_key_bytes() {
    let world = reset_conditioned_scc_model(true, [false, false], false, true);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the duplicate-root model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("one canonical root has an optimum")
    else {
        panic!("the single canonical root has an optimum certificate");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical proof");
    for part in ["post", "reset"] {
        for member in ["values", "ranking", "policy"] {
            if let Some(rows) = document["conditional"][part][member].as_array() {
                for row in rows {
                    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                        .decode(row["state"].as_str().expect("state key"))
                        .expect("state base64");
                    let key: Value = serde_json::from_slice(&bytes).expect("product key");
                    assert_eq!(key.as_array().expect("key array").len(), 8);
                }
            }
        }
    }
    let bytes = serde_json::to_vec(&document).expect("proof bytes");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &bytes, limits, &Cancel::new()
        )
        .expect("independent checker coalesces duplicate roots"),
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

/// A root that is already active contributes post evidence only; the waiting
/// root's reset policy is interpreted against that root's own initial state.
#[trace(
    "FR-196-AC-18",
    "FR-412-AC-35",
    "FR-412-AC-36",
    "FR-412-AC-37",
    "FR-412-AC-38",
    "FR-412-AC-39"
)]
#[test]
fn mixed_initial_roots_keep_distinct_post_and_reset_results() {
    let world = reset_conditioned_scc_model(true, [true, false], false, false);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the mixed-root model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("the mixed-root optimum settles")
    else {
        panic!("the mixed-root optimum has a certificate");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical proof");
    let root_of = |row: &Value| {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(row["state"].as_str().expect("state key"))
            .expect("state base64");
        let key: Value = serde_json::from_slice(&bytes).expect("product key");
        key.as_array()
            .and_then(|parts| parts.get(8))
            .and_then(Value::as_u64)
            .expect("root-indexed nine-member key")
    };
    let conditional = &document["conditional"];
    let post = conditional["post"]["values"].as_array().expect("post rows");
    let reset = conditional["reset"]["values"].as_array().expect("reset rows");
    let post_roots = post.iter().map(root_of).collect::<std::collections::BTreeSet<_>>();
    let reset_roots = reset.iter().map(root_of).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(post_roots, std::collections::BTreeSet::from([0, 1]));
    assert_eq!(reset_roots.len(), 1);
    assert!(reset_roots.is_subset(&post_roots));
    let values_by_root = post.iter().fold(BTreeMap::<u64, BTreeSet<(String, String)>>::new(), |mut values, row| {
        values.entry(root_of(row)).or_default().insert((
            row["value"]["numerator"].as_str().expect("numerator").to_owned(),
            row["value"]["denominator"].as_str().expect("denominator").to_owned(),
        ));
        values
    });
    assert_eq!(values_by_root.len(), 2);
    assert_ne!(values_by_root[&0], values_by_root[&1]);
    let policy = conditional["reset"]["policy"].as_array().expect("reset policy");
    let policy_roots = policy.iter().map(root_of).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(policy_roots, reset_roots, "the reset rows use only the waiting root");
    let bytes = serde_json::to_vec(&document).expect("proof bytes");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &bytes, limits, &Cancel::new()
        )
        .expect("checker replays each root against its own initial state"),
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

/// A multi-root process with no reachable activation is vacuous and has no
/// reset process or root-indexed product rows.
#[trace("FR-196-AC-18", "FR-412-AC-41")]
#[test]
fn vacuous_multi_root_claim_emits_no_reset_keys() {
    let world = reset_conditioned_scc_model(true, [false, false], true, false);
    let claim = world.claim("Min");
    let system = ModelSystem::new(
        world
            .request("Min", ModelCheckLimits::default())
            .subject
            .simulation,
    )
    .expect("the vacuous multi-root model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(claim).expect("claim identity");
    let limits = ProbabilityDocumentLimits::for_exact(
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let fsl_replay::OnEachOutcome::Holds { document } =
        fsl_replay::on_each_outcome(&system, claims, &item, limits, 64, &Cancel::new())
            .expect("the claim is vacuous")
    else {
        panic!("the no-activation process is vacuous");
    };
    let document: Value = serde_json::from_slice(&document).expect("canonical vacuous proof");
    assert_eq!(document["conditional"]["mode"], "vacuous");
    assert_eq!(document["values"], json!([]));
    assert!(document["conditional"].get("post").is_none());
    assert!(document["conditional"].get("reset").is_none());
    let bytes = serde_json::to_vec(&document).expect("proof bytes");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &bytes, limits, &Cancel::new()
        )
        .expect("independent vacuity checker"),
        fsl_replay::ExactProbabilityCertificateCheck::AcceptedVacuous(_)
    ));
}

/// TC-631's unbounded exact-only event product carries genuine absent
/// monitor and accumulator members, rather than synthesized control values.
#[trace("TC-631", "FR-196-AC-9")]
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
#[trace("TC-631", "FR-196-AC-9")]
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
