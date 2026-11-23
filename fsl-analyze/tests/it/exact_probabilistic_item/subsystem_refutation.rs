// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3156 to TC-3160: failing long-run, expected-reward and unwindowed
//! `eventually` claims settle `refuted` only after FR-202 replay reproduces
//! their subsystem certificate over the rebuilt workload or scheduler chain.
//!
//! The model is the architecture example's service: `call(ok)` sets `up` to
//! `ok`; the workload draws `ok` 99:1 and every call costs 1.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fsl_replay::{
    ProbabilisticCounterexample, ProbabilisticSchedulerChoice, ProbabilityWitnessError,
    ProbabilityWitnessReplay, ProductAction, Schedule,
};
use serde_json::{json, Value};

const TRAFFIC: &str = "random Service::call(ok) ~ { true: 99, false: 1 };\n\
                       workload Traffic on M { weight Service::call = 1; }\n";
const CALLS: &str = "reward calls on Service::call = 1;\n";

fn architecture(up: bool, declarations: &str) -> prob_world::Model {
    prob_world::Model {
        package: "test/architecture",
        object: "Service",
        population: "services",
        key: "checkout",
        integers: vec![],
        fields: vec![("up", prob_world::BOOLEAN.into())],
        operations: vec![prob_world::Operation {
            name: "call",
            parameters: vec![("ok", prob_world::BOOLEAN.into())],
            modifies: vec!["up"],
            deletes: false,
        }],
        body: format!(
            "post UpFollowsCall using v on M::Service::call {{ self.up = ok }}\n{declarations}"
        ),
        initial: json!({"up": {"boolean": up}}),
    }
}

fn claim(name: &str, scheduler: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using inf on M over (s: M::Service) under {scheduler} on origin {{ {body} }}\n"
    )
}

fn limits() -> fsl_replay::ProbabilityDocumentLimits {
    fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 16_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

fn q(numerator: i64, denominator: i64) -> Value {
    json!({"numerator": numerator.to_string(), "denominator": denominator.to_string()})
}

fn encoded(key: &ProbabilityProductKey) -> String {
    URL_SAFE_NO_PAD.encode(key.as_bytes())
}

/// The complete product keys of the initial state and of the other model
/// state, reached by one call.
fn keys(world: &World, name: &str) -> (ProbabilityProductKey, ProbabilityProductKey) {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let binding = claim_binding(&system, world.claim(name)).unwrap().unwrap();
    let key = |state| {
        let raw = system.state_key(state).unwrap().0;
        ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap()
    };
    let initial = &system.initial()[0];
    let start = key(initial);
    let other = system
        .actions(initial)
        .unwrap()
        .unwrap()
        .into_iter()
        .flat_map(|action| action.draws)
        .flat_map(|draw| draw.successors)
        .map(|(_, post)| key(&post))
        .find(|candidate| candidate != &start)
        .expect("one call reaches the other state");
    (start, other)
}

/// The scheduled identity of `call(ok)` at the initial state.
fn call(world: &World, ok: bool) -> ProductAction {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = &system.initial()[0];
    let action = system
        .actions(initial)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.draws.iter().any(|draw| {
                draw.successors.iter().any(|(_, post)| {
                    system.state_key(post).unwrap().0 != system.state_key(initial).unwrap().0
                }) == ok
            })
        })
        .expect("call(ok) is enabled");
    ProductAction::Identity(Schedule::of(&system, &action.identity).unwrap())
}

fn refuted(world: &World, name: &str) -> fsl_analyze::analyze::AnalyzeRecord {
    let record = settle(world, name, ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("{name} settles: {:?}", record.settlement));
    assert_eq!(terminal.value(), &TerminalValue::Refuted, "{name}");
    assert!(record.exact_probabilistic.is_none());
    record
}

fn evidence(record: &fsl_analyze::analyze::AnalyzeRecord) -> ProbabilisticCounterexample {
    record
        .probabilistic_counterexample
        .as_ref()
        .expect("the replayed counterexample stays in the result")
        .evidence()
        .clone()
}

fn document(evidence: &ProbabilisticCounterexample) -> Value {
    serde_json::from_slice(evidence.subsystem.as_ref().unwrap().as_bytes()).unwrap()
}

fn wire(record: &fsl_analyze::analyze::AnalyzeRecord) -> Value {
    record
        .probabilistic_counterexample
        .as_ref()
        .unwrap()
        .wire
        .clone()
        .expect("the counterexample spells")
}

fn offering(
    evidence: &ProbabilisticCounterexample,
    document: &Value,
) -> ProbabilisticCounterexample {
    let mut offered = evidence.clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
            &serde_json::to_vec(document).unwrap(),
            limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    offered
}

fn replay(
    world: &World,
    offered: &ProbabilisticCounterexample,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        offered,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    )
}

fn rejected(
    replayed: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>,
    rule: ProbabilityCertificateRule,
    at: &ProbabilityProductKey,
) {
    match replayed {
        Err(ProbabilityWitnessError::CertificateRejected(rejection)) => assert_eq!(
            rejection,
            ProbabilityCertificateRejection {
                rule,
                at: ProbabilityCertificateLocus::ProductState(at.clone()),
            }
        ),
        other => panic!("expected {rule:?}: {other:?}"),
    }
}

fn reproduced(replayed: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>) {
    assert!(
        matches!(replayed, Ok(ProbabilityWitnessReplay::Reproduced(_))),
        "{replayed:?}"
    );
}

fn set_value(document: &mut Value, key: &ProbabilityProductKey, value: Value) {
    let state = encoded(key);
    document["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == state)
        .expect("a named state")["value"] = value;
}

fn set_rank(document: &mut Value, key: &ProbabilityProductKey, rank: u64) {
    let state = encoded(key);
    document["ranking"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == state)
        .expect("a ranked state")["rank"] = json!(rank);
}

/// The item's settlement when the produced counterexample is altered before
/// replay.
fn settle_tampered(
    world: &World,
    name: &str,
    tamper: impl Fn(&mut ProbabilisticCounterexample) + 'static,
) -> fsl_analyze::analyze::AnalyzeRecord {
    let item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        world.request(name, ModelCheckLimits::default()),
        tamper,
    );
    analyze_items(&Engines::fsl().unwrap(), vec![item], &Cancel::new()).remove(0)
}

fn replay_refused(record: &fsl_analyze::analyze::AnalyzeRecord) {
    assert_eq!(
        record.terminal().expect("a terminal record").value(),
        &TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
            fsl_foundation::diagnostic::Code::InvalidRuntimeInput
        ))
    );
    assert!(record.probabilistic_counterexample.is_none());
}

fn certificate_rejected(
    record: &fsl_analyze::analyze::AnalyzeRecord,
    rule: CertificateRule,
    at: &ProbabilityProductKey,
) {
    assert_eq!(
        record.terminal().expect("a terminal record").value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule,
            at: CertificateLocus::ProbabilityProductState(at.clone()),
        })
    );
    assert!(record.probabilistic_counterexample.is_none());
}

#[trace("TC-3156", "FR-202-AC-31")]
#[test]
fn a_failing_workload_long_run_fraction_is_refuted_by_its_replayed_long_run_certificate() {
    let world = World::new(&architecture(
        false,
        &format!(
            "{TRAFFIC}{}{}",
            claim("Fails", "Traffic", "long_run_fraction holds(s.up) >= 0.995"),
            claim(
                "AtBound",
                "Traffic",
                "long_run_fraction holds(s.up) >= rational(99,100)"
            ),
        ),
    ));
    let (down, up) = keys(&world, "Fails");
    let record = refuted(&world, "Fails");
    let found = evidence(&record);
    assert!(found.scheduler.is_empty());
    assert!(found.paths.is_empty());
    assert_eq!(found.initial_key, down);
    let doc = document(&found);
    assert_eq!(doc["kind"], "long-run");
    assert_eq!(
        doc["values"],
        json!([
            {"state": encoded(&down.clone().min(up.clone())), "value": q(99, 100)},
            {"state": encoded(&down.clone().max(up.clone())), "value": q(99, 100)},
        ])
    );
    assert_eq!(doc["components"].as_array().unwrap().len(), 1);
    assert_eq!(doc["components"][0]["value"], q(99, 100));
    assert_eq!(doc["components"][0]["gain"], q(99, 100));
    let spelled = wire(&record);
    assert_eq!(spelled["evidence"]["type"], "subsystem");
    assert_eq!(spelled["evidence"]["certificates"][0], doc);
    assert!(spelled.get("scheduler").is_none());
    assert_eq!(
        spelled["bound"],
        json!({"comparison": ">=", "threshold": q(199, 200)})
    );

    // Equality at the bound proves; its proof-side document is not a
    // refutation under the strict comparison.
    let at_bound = settle(&world, "AtBound", ModelCheckLimits::default());
    let terminal = at_bound.terminal().unwrap();
    assert!(matches!(terminal.value(), TerminalValue::Proved { .. }));
    let proof = terminal.exact_certificate_document().unwrap().to_vec();
    let mut offered = found.clone();
    offered.item = fsl_replay::probabilistic_claim_identity(world.claim("AtBound")).unwrap();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(&proof, limits(), &Cancel::new())
            .unwrap(),
    ));
    rejected(
        replay(&world, &offered),
        ProbabilityCertificateRule::ThresholdMissed,
        &down,
    );

    // A changed bias fails its gain-bias equation over the rebuilt chain.
    let mut broken = doc.clone();
    broken["components"][0]["bias"][1]["value"] = q(1, 1);
    let replayed = replay(&world, &offering(&found, &broken));
    assert!(
        matches!(
            replayed,
            Err(ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "{replayed:?}"
    );
    reproduced(replay(&world, &found));

    // A workload counterexample carries no scheduler; no bare value refutes.
    let scheduled = settle_tampered(&world, "Fails", move |witness| {
        witness.scheduler = vec![(
            witness.initial_key.clone(),
            ProbabilisticSchedulerChoice::Deterministic(ProductAction::TerminalStutter),
        )];
    });
    replay_refused(&scheduled);
    let bare = settle_tampered(&world, "Fails", |witness| witness.subsystem = None);
    assert_ne!(
        bare.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted)
    );
    assert!(bare.probabilistic_counterexample.is_none());
}

#[trace("TC-3157", "FR-202-AC-32")]
#[test]
fn a_failing_workload_expected_total_is_refuted_by_a_strict_lower_certificate() {
    let world = World::new(&architecture(
        false,
        &format!(
            "{TRAFFIC}{CALLS}{}",
            claim(
                "Fails",
                "Traffic",
                "expected accumulate calls until holds(s.up) <= 1"
            ),
        ),
    ));
    let (down, up) = keys(&world, "Fails");
    let record = refuted(&world, "Fails");
    let found = evidence(&record);
    assert!(found.scheduler.is_empty());
    let doc = document(&found);
    assert_eq!(doc["kind"], "lower");
    assert_eq!(
        doc["objective"]["profile"],
        "filament.probability-objective/value-bounds/v1"
    );
    assert_eq!(
        doc["objective"]["base"]["profile"],
        "filament.probability-objective/dtmc-expected-reward/v1"
    );
    assert_eq!(doc["objective"]["base"]["optimization"], "workload");
    assert_eq!(doc["objective"]["base"]["targets"], json!([encoded(&up)]));
    assert!(doc.get("ranking").is_none());
    let mut expected = vec![(down.clone(), q(100, 99)), (up.clone(), q(0, 1))];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        doc["values"],
        Value::Array(
            expected
                .into_iter()
                .map(|(key, value)| json!({"state": encoded(&key), "value": value}))
                .collect()
        )
    );
    assert!(wire(&record).get("scheduler").is_none());

    let mut equal = doc.clone();
    set_value(&mut equal, &down, q(1, 1));
    rejected(
        replay(&world, &offering(&found, &equal)),
        ProbabilityCertificateRule::ThresholdMissed,
        &down,
    );
    let mut invalid = doc.clone();
    set_value(&mut invalid, &down, q(2, 1));
    rejected(
        replay(&world, &offering(&found, &invalid)),
        ProbabilityCertificateRule::InequalityFails,
        &down,
    );
    let mut target = doc.clone();
    set_value(&mut target, &up, q(1, 1));
    rejected(
        replay(&world, &offering(&found, &target)),
        ProbabilityCertificateRule::InequalityFails,
        &up,
    );
    let equality = settle_tampered(&world, "Fails", move |witness| {
        *witness = offering(witness, &equal);
    });
    certificate_rejected(&equality, CertificateRule::ThresholdMissed, &down);
    let bare = settle_tampered(&world, "Fails", |witness| witness.subsystem = None);
    assert_ne!(
        bare.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted)
    );
}

#[trace("TC-3158", "FR-202-AC-33")]
#[test]
fn an_every_scheduler_expected_total_carries_its_scheduler_and_a_proper_upper_certificate() {
    let at_target = World::new(&architecture(
        true,
        &format!(
            "{CALLS}{}",
            claim(
                "Zero",
                "every scheduler",
                "expected accumulate calls until holds(s.up) >= 1"
            ),
        ),
    ));
    let record = refuted(&at_target, "Zero");
    let found = evidence(&record);
    let up = found.initial_key.clone();
    // The initial state already satisfies the target: no choice is made.
    assert!(found.scheduler.is_empty());
    let doc = document(&found);
    assert_eq!(doc["kind"], "upper");
    assert_eq!(
        doc["objective"]["base"]["profile"],
        "filament.probability-objective/cyclic-mdp-expected-reward/v1"
    );
    assert_eq!(doc["objective"]["base"]["optimization"], "minimum");
    assert_eq!(
        doc["values"],
        json!([{"state": encoded(&up), "value": q(0, 1)}])
    );
    assert_eq!(doc["ranking"], json!([{"state": encoded(&up), "rank": 0}]));

    let world = World::new(&architecture(
        false,
        &format!(
            "{CALLS}{}",
            claim(
                "Two",
                "every scheduler",
                "expected accumulate calls until holds(s.up) >= 2"
            ),
        ),
    ));
    let (down, up) = keys(&world, "Two");
    let record = refuted(&world, "Two");
    let found = evidence(&record);
    let selected = call(&world, true);
    assert_eq!(
        found.scheduler,
        vec![(
            down.clone(),
            ProbabilisticSchedulerChoice::Deterministic(selected)
        )]
    );
    let doc = document(&found);
    assert_eq!(doc["kind"], "upper");
    let mut values = vec![(down.clone(), q(1, 1), 1), (up.clone(), q(0, 1), 0)];
    values.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        doc["values"],
        Value::Array(
            values
                .iter()
                .map(|(key, value, _)| json!({"state": encoded(key), "value": value}))
                .collect()
        )
    );
    assert_eq!(
        doc["ranking"],
        Value::Array(
            values
                .iter()
                .map(|(key, _, rank)| json!({"state": encoded(key), "rank": rank}))
                .collect()
        )
    );
    assert_eq!(
        wire(&record)["scheduler"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let omitted = settle_tampered(&world, "Two", |witness| witness.scheduler.clear());
    replay_refused(&omitted);
    let stay = call(&world, false);
    let changed = settle_tampered(&world, "Two", move |witness| {
        witness.scheduler[0].1 = ProbabilisticSchedulerChoice::Deterministic(stay.clone());
    });
    certificate_rejected(&changed, CertificateRule::InequalityFails, &down);
    let mut equal = doc.clone();
    set_value(&mut equal, &down, q(2, 1));
    rejected(
        replay(&world, &offering(&found, &equal)),
        ProbabilityCertificateRule::ThresholdMissed,
        &down,
    );
    let mut improper = doc.clone();
    set_rank(&mut improper, &down, 0);
    rejected(
        replay(&world, &offering(&found, &improper)),
        ProbabilityCertificateRule::RankBroken,
        &down,
    );
}

#[trace("TC-3159", "FR-202-AC-34")]
#[test]
fn an_every_scheduler_eventually_without_random_is_refuted_by_event_avoidance() {
    let world = World::new(&architecture(
        false,
        &claim(
            "ComesUp",
            "every scheduler",
            "probability >= 0.5 [ eventually holds(s.up) ]",
        ),
    ));
    let (down, _) = keys(&world, "ComesUp");
    let record = refuted(&world, "ComesUp");
    let found = evidence(&record);
    let stay = call(&world, false);
    assert_eq!(
        found.scheduler,
        vec![(
            down.clone(),
            ProbabilisticSchedulerChoice::Deterministic(stay)
        )]
    );
    assert!(found.paths.is_empty());
    let doc = document(&found);
    assert_eq!(doc["kind"], "lower");
    assert_eq!(
        doc["objective"]["base"]["profile"],
        "filament.probability-objective/cyclic-mdp-eventual/v1"
    );
    assert_eq!(doc["objective"]["base"]["optimization"], "minimum");
    assert_eq!(
        doc["values"],
        json!([{"state": encoded(&down), "value": q(1, 1)}])
    );
    assert_eq!(
        doc["ranking"],
        json!([{"state": encoded(&down), "rank": 0}])
    );
    assert_eq!(wire(&record)["evidence"]["type"], "subsystem");

    let mut equal = doc.clone();
    set_value(&mut equal, &down, q(1, 2));
    rejected(
        replay(&world, &offering(&found, &equal)),
        ProbabilityCertificateRule::ThresholdMissed,
        &down,
    );
    let omitted = settle_tampered(&world, "ComesUp", |witness| witness.scheduler.clear());
    replay_refused(&omitted);
    // Calling with `ok` reaches the event: the rebuilt chain's only bottom
    // component contains an event state and avoids nothing.
    let go = call(&world, true);
    let changed = settle_tampered(&world, "ComesUp", move |witness| {
        witness.scheduler[0].1 = ProbabilisticSchedulerChoice::Deterministic(go.clone());
    });
    certificate_rejected(&changed, CertificateRule::InequalityFails, &down);
}

/// FSpec FR-412-AC-24's chain: `s` reaches event `e` and the self-looping
/// event-free `a` with probability 1/2 each.
fn split(claims: &str) -> prob_world::Model {
    let package = "test/avoidance";
    prob_world::Model {
        package,
        object: "Item",
        population: "items",
        key: "x",
        integers: vec![("Phase", 0, 2), ("Pick", 1, 2)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![
            prob_world::Operation {
                name: "go",
                parameters: vec![("r", format!("ix://{package}/Pick"))],
                modifies: vec!["phase"],
                deletes: false,
            },
            prob_world::Operation {
                name: "stay",
                parameters: vec![],
                modifies: vec!["phase"],
                deletes: false,
            },
        ],
        body: format!(
            "pre Go using v on M::Item::go {{ self.phase = 0 }}\n\
             post Went using v on M::Item::go {{ self.phase = r }}\n\
             pre Stay using v on M::Item::stay {{ self.phase = 2 }}\n\
             post Stayed using v on M::Item::stay {{ self.phase = 2 }}\n\
             random Item::go(r) ~ {{ 1: 1, 2: 1 }};\n{claims}"
        ),
        initial: json!({"phase": {"integer": "0"}}),
    }
}

#[trace("FSpec-TC-432", "FSpec-FR-412-AC-24", "FSpec-FR-413-AC-16")]
#[test]
fn event_avoidance_derives_the_event_free_bottom_component_itself() {
    let world = World::new(&split(
        "probabilistic Event using inf on M over (x: M::Item) under every scheduler on origin \
         { probability >= rational(3,4) [ eventually holds(x.phase = 1) ] }\n",
    ));
    let record = refuted(&world, "Event");
    let found = evidence(&record);
    let doc = document(&found);
    let s = found.initial_key.clone();
    let named = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["state"].as_str().unwrap().to_owned(),
                row["value"].clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(named.len(), 3);
    let root = encoded(&s);
    let value = |state: &str| {
        named
            .iter()
            .find(|(key, _)| key == state)
            .map(|(_, value)| value.clone())
            .unwrap()
    };
    assert_eq!(value(&root), q(1, 2));
    let others = named
        .iter()
        .filter(|(key, _)| key != &root)
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    assert!(
        others.contains(&q(1, 1)) && others.contains(&q(0, 1)),
        "{others:?}"
    );
    let state_of = |wanted: &Value| -> ProbabilityProductKey {
        let (key, _) = named
            .iter()
            .find(|(key, value)| key != &root && value == wanted)
            .unwrap();
        ProbabilityProductKey::from_canonical_bytes(URL_SAFE_NO_PAD.decode(key).unwrap())
    };
    let (a, e) = (state_of(&q(1, 1)), state_of(&q(0, 1)));
    let ranks = doc["ranking"].as_array().unwrap();
    let rank = |key: &ProbabilityProductKey| {
        ranks
            .iter()
            .find(|row| row["state"] == encoded(key))
            .unwrap()["rank"]
            .clone()
    };
    assert_eq!(
        (rank(&s), rank(&a), rank(&e)),
        (json!(1), json!(0), json!(0))
    );
    // Both `s` and `a` make their one choice under the witness scheduler.
    assert_eq!(found.scheduler.len(), 2);
    reproduced(replay(&world, &found));

    let mut above = doc.clone();
    set_value(&mut above, &s, q(3, 4));
    rejected(
        replay(&world, &offering(&found, &above)),
        ProbabilityCertificateRule::InequalityFails,
        &s,
    );
    let mut event = doc.clone();
    set_value(&mut event, &e, q(1, 1));
    rejected(
        replay(&world, &offering(&found, &event)),
        ProbabilityCertificateRule::InequalityFails,
        &e,
    );
    let mut flat = doc.clone();
    set_rank(&mut flat, &s, 0);
    rejected(
        replay(&world, &offering(&found, &flat)),
        ProbabilityCertificateRule::RankBroken,
        &s,
    );
    let mut quarter = doc.clone();
    set_value(&mut quarter, &s, q(1, 4));
    rejected(
        replay(&world, &offering(&found, &quarter)),
        ProbabilityCertificateRule::ThresholdMissed,
        &s,
    );
    let mut unscheduled = found.clone();
    unscheduled.scheduler.clear();
    assert!(
        matches!(
            replay(&world, &unscheduled),
            Err(ProbabilityWitnessError::Refused)
        ),
        "a missing scheduler"
    );
}

#[trace("TC-3160", "FR-202-AC-35", "FR-203-AC-15")]
#[test]
fn an_infinite_expected_total_is_refuted_by_a_finite_lower_certificate() {
    let world = World::new(&architecture(
        false,
        &format!(
            "{CALLS}{}",
            claim(
                "Bounded",
                "every scheduler",
                "expected accumulate calls until holds(s.up) <= 1"
            ),
        ),
    ));
    let (down, _) = keys(&world, "Bounded");
    let record = refuted(&world, "Bounded");
    let found = evidence(&record);
    assert_eq!(
        found.scheduler,
        vec![(
            down.clone(),
            ProbabilisticSchedulerChoice::Deterministic(call(&world, false))
        )]
    );
    let doc = document(&found);
    assert_eq!(doc["kind"], "lower");
    assert_eq!(doc["objective"]["base"]["optimization"], "maximum");
    assert!(doc.get("ranking").is_none());
    // Never reaching the target from `down` has probability 1, so the
    // finite bound is (1/1 + 1) * 1 = 2 > 1.
    assert_eq!(
        doc["values"],
        json!([{"state": encoded(&down), "value": q(2, 1)}])
    );

    let mut equal = doc.clone();
    set_value(&mut equal, &down, q(1, 1));
    rejected(
        replay(&world, &offering(&found, &equal)),
        ProbabilityCertificateRule::ThresholdMissed,
        &down,
    );
    let bare = settle_tampered(&world, "Bounded", |witness| witness.subsystem = None);
    assert_ne!(
        bare.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted)
    );
    assert!(bare.probabilistic_counterexample.is_none());
}
