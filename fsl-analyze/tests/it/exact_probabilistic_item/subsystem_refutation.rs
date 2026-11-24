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

fn avail2_claim(name: &str, scheduler: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using inf on M over (s: M::Server) under {scheduler} on origin {{ {body} }}\n"
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
    let key = |state: &fsl_eval::simulation::model::ModelSimState| {
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

/// Resolve a named operation in the reachable, one-step `Avail2` chain.
fn call_named(world: &World, name: &str) -> ProductAction {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let mut states = system.initial().to_vec();
    for state in states.clone() {
        if let Ok(actions) = system.actions(&state).unwrap() {
            for action in actions {
                for draw in action.draws {
                    states.extend(draw.successors.into_iter().map(|(_, post)| post));
                }
            }
        }
    }
    states.sort_by_key(|state| system.state_key(state).unwrap().0);
    states.dedup_by(|left, right| {
        system.state_key(left).unwrap().0 == system.state_key(right).unwrap().0
    });
    for state in states {
        if let Ok(actions) = system.actions(&state).unwrap() {
            if let Some(action) = actions
                .into_iter()
                .find(|action| action.identity.operation.contains(name))
            {
                return ProductAction::Identity(Schedule::of(&system, &action.identity).unwrap());
            }
        }
    }
    panic!("{name} is enabled in the reachable model");
}

fn fast_avail2_witness(world: &World) -> ProbabilisticCounterexample {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("AtLeast")).unwrap();
    let binding = claim_binding(&system, world.claim("AtLeast"))
        .unwrap()
        .unwrap();
    let (up, down) = keys(world, "AtLeast");
    let rational = |n: i64, d: i64| Rational::new(Integer::from(n), Integer::from(d)).unwrap();
    let row = |state: &ProbabilityProductKey, value: Rational| fsl_replay::ExactProbabilityValue {
        state: state.clone(),
        value,
    };
    let tick = call_named(world, "tick");
    let fast = call_named(world, "repair_fast");
    let mut values = vec![
        row(&up, rational(1800, 1801)),
        row(&down, rational(1800, 1801)),
    ];
    values.sort_by(|a, b| a.state.cmp(&b.state));
    // The document's canonical first key is down, whose bias is zero.
    let mut bias = vec![row(&up, rational(2000, 1801)), row(&down, rational(0, 1))];
    bias.sort_by(|a, b| a.state.cmp(&b.state));
    let mut policy = vec![
        fsl_replay::LongRunPolicyRow {
            state: up.clone(),
            choice: tick.clone(),
        },
        fsl_replay::LongRunPolicyRow {
            state: down.clone(),
            choice: fast.clone(),
        },
    ];
    policy.sort_by(|a, b| a.state.cmp(&b.state));
    let certificate = fsl_replay::long_run_probability_certificate_document(
        &system,
        claims,
        &item,
        values,
        vec![fsl_replay::LongRunProbabilityComponent {
            value: rational(1800, 1801),
            gain: rational(1800, 1801),
            bias,
            policy: Some(policy),
        }],
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let subsystem = fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
        &certificate,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let initial = system.initial()[0].clone();
    let mut scheduler = vec![
        (
            up.clone(),
            ProbabilisticSchedulerChoice::Deterministic(tick),
        ),
        (down, ProbabilisticSchedulerChoice::Deterministic(fast)),
    ];
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    ProbabilisticCounterexample {
        item,
        binding: Some(binding.reference),
        instance: None,
        initial,
        initial_key: up,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    }
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
    broken["components"][0]["bias"][1]["value"] = q(2, 1);
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
    let mut values = [(down.clone(), q(1, 1), 1), (up.clone(), q(0, 1), 0)];
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
    // Never calling with `ok` no longer reaches `up`: the upper certificate's
    // values no longer cover exactly the rebuilt chain.
    assert_eq!(
        changed.terminal().expect("a terminal record").value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(fsl_replay::CertificateMember::Values),
        })
    );
    assert!(changed.probabilistic_counterexample.is_none());
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

    // `down` is the event-free bottom component itself: its value is 1.
    let mut halved = doc.clone();
    set_value(&mut halved, &down, q(1, 2));
    rejected(
        replay(&world, &offering(&found, &halved)),
        ProbabilityCertificateRule::InequalityFails,
        &down,
    );
    // Equality: over a chain avoiding the event with probability 1/2, the
    // checked bound 1/2 against `>= 1/2` sits at `1 - θ` and does not refute.
    let split_world = World::new(&split(
        "probabilistic Event using inf on M over (x: M::Item) under every scheduler on origin \
         { probability >= rational(3,4) [ eventually holds(x.phase = 1) ] }\n\
         probabilistic Half using inf on M over (x: M::Item) under every scheduler on origin \
         { probability >= rational(1,2) [ eventually holds(x.phase = 1) ] }\n",
    ));
    let split_found = evidence(&refuted(&split_world, "Event"));
    let at_half = reidentified(&split_world, "Half", &split_found);
    rejected(
        replay(&split_world, &at_half),
        ProbabilityCertificateRule::ThresholdMissed,
        &split_found.initial_key,
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

/// An unwindowed `always` under a workload whose calls fail 1 time in 1,000
/// has exact value 0: the path masses `(999/1000)^n` of a path-set search
/// never close the gap, so the claim is refuted by the rejecting-union
/// `lower` certificate over the cyclic chain instead: no path-set search
/// starts (FR-202-AC-21's cyclic `always` clause), and the counterexample
/// carries no path.
#[trace("FR-202-AC-21")]
#[test]
fn a_rarely_failing_workload_always_is_refuted_by_its_rejecting_union() {
    let world = World::new(&architecture(
        true,
        &format!(
            "random Service::call(ok) ~ {{ true: 999, false: 1 }};\n\
             workload Traffic on M {{ weight Service::call = 1; }}\n{}",
            claim(
                "StaysUp",
                "Traffic",
                "probability >= 0.99 [ always holds(s.up) ]"
            ),
        ),
    ));
    let (up, down) = keys(&world, "StaysUp");
    fsl_analyze::exact_probabilistic::reset_product_construction_counts();
    let record = refuted(&world, "StaysUp");
    assert_eq!(
        fsl_analyze::exact_probabilistic::product_construction_counts().path_searches,
        0,
        "no path-set search on the cyclic subject"
    );
    decisive(&record);
    let found = evidence(&record);
    assert!(found.scheduler.is_empty());
    assert!(found.paths.is_empty());
    let doc = document(&found);
    assert_eq!(doc["kind"], "lower");
    assert_eq!(doc["objective"]["base"]["event"], "always");
    let mut expected = [(up.clone(), q(1, 1), 1), (down.clone(), q(1, 1), 0)];
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        doc["values"],
        Value::Array(
            expected
                .iter()
                .map(|(key, value, _)| json!({"state": encoded(key), "value": value}))
                .collect()
        )
    );
    assert_eq!(
        doc["ranking"],
        Value::Array(
            expected
                .iter()
                .map(|(key, _, rank)| json!({"state": encoded(key), "rank": rank}))
                .collect()
        )
    );
    reproduced(replay(&world, &found));
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
        integers: vec![("Phase", 0, 2)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![
            prob_world::Operation {
                name: "go",
                parameters: vec![("r", format!("ix://{package}/Phase"))],
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
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let binding = claim_binding(&system, world.claim("Event"))
        .unwrap()
        .unwrap();
    let successors = system
        .actions(&system.initial()[0])
        .unwrap()
        .unwrap()
        .into_iter()
        .flat_map(|action| action.draws)
        .flat_map(|draw| draw.successors)
        .map(|(_, post)| {
            let raw = system.state_key(&post).unwrap().0;
            ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(successors.len(), 2);
    // The producer names the positive states; the event state counts 0.
    let named = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| key_from(row["state"].as_str().unwrap()))
        .collect::<Vec<_>>();
    let a = named
        .iter()
        .find(|key| **key != s)
        .expect("the avoiding state is named")
        .clone();
    let e = successors
        .into_iter()
        .find(|key| *key != a)
        .expect("the event state");
    let mut expected = [(s.clone(), q(1, 2), 1), (a.clone(), q(1, 1), 0)];
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(
        doc["values"],
        Value::Array(
            expected
                .iter()
                .map(|(key, value, _)| json!({"state": encoded(key), "value": value}))
                .collect()
        )
    );
    assert_eq!(
        doc["ranking"],
        Value::Array(
            expected
                .iter()
                .map(|(key, _, rank)| json!({"state": encoded(key), "rank": rank}))
                .collect()
        )
    );
    // Both `s` and `a` make their one choice under the witness scheduler.
    assert_eq!(found.scheduler.len(), 2);
    reproduced(replay(&world, &found));

    // FSpec FR-412-AC-24's certificate also names the event state at 0.
    let mut full = doc.clone();
    insert_row(&mut full, &e, q(0, 1), 0);
    reproduced(replay(&world, &offering(&found, &full)));

    let mut above = full.clone();
    set_value(&mut above, &s, q(3, 4));
    rejected(
        replay(&world, &offering(&found, &above)),
        ProbabilityCertificateRule::InequalityFails,
        &s,
    );
    let mut event = full.clone();
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
            "{CALLS}{}{}",
            claim(
                "Bounded",
                "every scheduler",
                "expected accumulate calls until holds(s.up) <= 1"
            ),
            claim(
                "AtLeastOne",
                "every scheduler",
                "expected accumulate calls until holds(s.up) >= 1"
            ),
        ),
    ));
    let (down, _) = keys(&world, "Bounded");
    let record = refuted(&world, "Bounded");
    decisive(&record);
    // The infinite expectation is carried by no exact result value.
    let terminal = record.terminal().unwrap();
    assert!(terminal.exact_certificate().is_none());
    assert!(terminal.exact_certificate_document().is_none());
    assert!(wire(&record).get("value").is_none());
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

    // The proof side keeps its inclusive claim-side rule: the minimum 1
    // proves `>= 1` at equality.
    let at_one = settle(&world, "AtLeastOne", ModelCheckLimits::default());
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = at_one.terminal().unwrap().value()
    else {
        panic!("{:?}", at_one.settlement);
    };
    assert_eq!(value.value(), &ratio(1, 1));
}

fn key_from(encoded: &str) -> ProbabilityProductKey {
    ProbabilityProductKey::from_canonical_bytes(URL_SAFE_NO_PAD.decode(encoded).unwrap())
}

/// Name one more state, keeping both row groups in complete-key order.
fn insert_row(document: &mut Value, key: &ProbabilityProductKey, value: Value, rank: u64) {
    for (member, row) in [
        ("values", json!({"state": encoded(key), "value": value})),
        ("ranking", json!({"state": encoded(key), "rank": rank})),
    ] {
        let rows = document[member].as_array_mut().unwrap();
        rows.push(row);
        rows.sort_by(|left, right| {
            key_from(left["state"].as_str().unwrap())
                .cmp(&key_from(right["state"].as_str().unwrap()))
        });
    }
}

/// FR-203: a replayed refutation settles `refuted` on the
/// `decisive-counterexample` basis with truth false.
fn decisive(record: &fsl_analyze::analyze::AnalyzeRecord) {
    let axes = fsl_replay::OutcomeAxes::of_record(record.terminal().unwrap().value(), false);
    assert_eq!(
        (axes.truth, axes.settlement),
        (
            Some(fsl_replay::TruthAxis::False),
            Some(fsl_replay::SettlementBasis::DecisiveCounterexample)
        )
    );
}

/// `found`'s certificate rows written again for the claim `name`.
fn reidentified(
    world: &World,
    name: &str,
    found: &ProbabilisticCounterexample,
) -> ProbabilisticCounterexample {
    let doc = document(found);
    let rational = |value: &Value| {
        Rational::new(
            value["numerator"].as_str().unwrap().parse().unwrap(),
            value["denominator"].as_str().unwrap().parse().unwrap(),
        )
        .unwrap()
    };
    let values = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| fsl_replay::ExactProbabilityValue {
            state: key_from(row["state"].as_str().unwrap()),
            value: rational(&row["value"]),
        })
        .collect::<Vec<_>>();
    let ranking = doc.get("ranking").map(|rows| {
        rows.as_array()
            .unwrap()
            .iter()
            .map(|row| {
                (
                    key_from(row["state"].as_str().unwrap()),
                    row["rank"].as_u64().unwrap(),
                )
            })
            .collect::<Vec<_>>()
    });
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    let mut offered = found.clone();
    offered.item = item.clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::value_bound(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            if doc["kind"] == "upper" {
                fsl_replay::SubsystemBoundKind::Upper
            } else {
                fsl_replay::SubsystemBoundKind::Lower
            },
            &values,
            ranking.as_deref(),
            limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    offered
}

/// FR-202's checked-subject subsystem entry: "exhaustion and cancellation
/// retain the actual typed `incomplete` cause" (no AC states it yet).
#[trace("FR-202")]
#[test]
fn the_subsystem_entry_keeps_typed_limit_and_cancellation_causes() {
    use std::sync::atomic::AtomicUsize;
    let world = World::new(&architecture(
        false,
        &claim(
            "ComesUp",
            "every scheduler",
            "probability >= 0.5 [ eventually holds(s.up) ]",
        ),
    ));
    let found = evidence(&refuted(&world, "ComesUp"));
    let replay_under = |meter: filament_exact::ScalarLimits,
                        limits: &fsl_analyze::exact_probabilistic::ExactProbLimits,
                        cancel: &Cancel| {
        let mut simulation = world.subject().simulation;
        simulation.meter = meter;
        let system = ModelSystem::new(simulation).unwrap();
        fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &found.item,
            &found,
            limits,
            cancel,
        )
    };
    let defaults = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    // A handle that cancels itself at its `at`-th meter charge.
    let cancelling_at = |at: usize| {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let charges = Arc::new(AtomicUsize::new(0));
        let (handle, counted) = (slot.clone(), charges.clone());
        let cancel = Cancel::observing(move || {
            if counted.fetch_add(1, Ordering::SeqCst) + 1 == at {
                if let Some(cancel) = handle.get() {
                    cancel.cancel(CancelCause::Requested);
                }
            }
        });
        slot.set(cancel.clone()).expect("set once");
        (cancel, charges)
    };
    let (uncancelled, charges) = cancelling_at(usize::MAX);
    assert!(matches!(
        replay_under(prob_world::UNLIMITED, &defaults, &uncancelled),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
    let total = charges.load(Ordering::SeqCst);
    assert!(total > 1, "the entry charges its own meter");
    // Cancellation spread over the replay, through the entry's own checks
    // up to its last charge before the final acceptance poll, keeps its
    // cause. The last charge lies inside the entry, after every check.
    let mut points = (1..total).step_by((total / 64).max(1)).collect::<Vec<_>>();
    points.push(total);
    for at in points {
        let (cancel, _) = cancelling_at(at);
        let replayed = replay_under(prob_world::UNLIMITED, &defaults, &cancel);
        assert!(
            matches!(
                replayed,
                Err(ProbabilityWitnessError::Graph(
                    ExactProbabilityGraphError::Cancelled(CancelCause::Requested)
                ))
            ),
            "cancel at charge {at} of {total}: {replayed:?}"
        );
    }
    // Each work budget below the replay's need stops it at that budget.
    let mut work = 1;
    loop {
        let meter = filament_exact::ScalarLimits {
            work_units: work,
            ..prob_world::UNLIMITED
        };
        match replay_under(meter, &defaults, &Cancel::new()) {
            Ok(ProbabilityWitnessReplay::Reproduced(_)) => break,
            replayed => assert!(
                matches!(
                    replayed,
                    Err(ProbabilityWitnessError::Limit {
                        limit: ModelCheckLimit::EvaluationMeter {
                            counter: filament_exact::LimitKind::WorkUnits
                        },
                        value,
                    }) if value == work
                ),
                "work_units {work}: {replayed:?}"
            ),
        }
        work *= 2;
    }
    assert!(work > 1, "the entry spends work units");
    // Each working-storage budget below the replay's need stops it there.
    let mut storage = 1;
    loop {
        let mut limits = defaults;
        limits.document.storage_bytes = storage;
        match replay_under(prob_world::UNLIMITED, &limits, &Cancel::new()) {
            Ok(ProbabilityWitnessReplay::Reproduced(_)) => break,
            replayed => assert!(
                matches!(
                    replayed,
                    Err(ProbabilityWitnessError::Limit {
                        limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                        value,
                    }) if value == storage
                ),
                "storage_bytes {storage}: {replayed:?}"
            ),
        }
        storage *= 2;
    }
    assert!(storage > 1, "the entry retains working storage");
    let mut one_state = defaults;
    one_state.model.limits.max_states = 1;
    let replayed = replay_under(prob_world::UNLIMITED, &one_state, &Cancel::new());
    assert!(
        matches!(
            replayed,
            Err(ProbabilityWitnessError::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 1,
            })
        ),
        "{replayed:?}"
    );
}

/// A refutation witness for an `Every` long-run claim over the architecture
/// service, written by hand: the down state's only reachable successor under
/// `call(false)` is itself, so the induced chain is one component of value 0.
fn scheduled_long_run(
    world: &World,
    name: &str,
    scheduler: Vec<(ProbabilityProductKey, ProbabilisticSchedulerChoice)>,
    // The evidence of the chain that stays up (value 1) rather than down (0).
    stays_up: bool,
) -> ProbabilisticCounterexample {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    let (down, up) = keys(world, name);
    let rational = |n: i64| Rational::new(Integer::from(n), Integer::from(1i64)).unwrap();
    let row = |state: &ProbabilityProductKey, value: Rational| fsl_replay::ExactProbabilityValue {
        state: state.clone(),
        value,
    };
    let (values, component) = if stays_up {
        // Equation h(down) = 0 - 1 + h(up), h(up) = h(up): the first key is 0.
        let mut bias = if down < up {
            vec![row(&down, rational(0)), row(&up, rational(1))]
        } else {
            vec![row(&up, rational(0)), row(&down, rational(-1))]
        };
        bias.sort_by(|a, b| a.state.cmp(&b.state));
        let mut values = vec![row(&down, rational(1)), row(&up, rational(1))];
        values.sort_by(|a, b| a.state.cmp(&b.state));
        (
            values,
            fsl_replay::LongRunProbabilityComponent {
                value: rational(1),
                gain: rational(1),
                bias,
                policy: Some({
                    let mut rows = vec![
                        fsl_replay::LongRunPolicyRow {
                            state: down.clone(),
                            choice: call(world, true),
                        },
                        fsl_replay::LongRunPolicyRow {
                            state: up.clone(),
                            choice: call_staying_up(world),
                        },
                    ];
                    rows.sort_by(|a, b| a.state.cmp(&b.state));
                    rows
                }),
            },
        )
    } else {
        (
            vec![row(&down, rational(0))],
            fsl_replay::LongRunProbabilityComponent {
                value: rational(0),
                gain: rational(0),
                bias: vec![row(&down, rational(0))],
                policy: Some(vec![fsl_replay::LongRunPolicyRow {
                    state: down.clone(),
                    choice: call(world, false),
                }]),
            },
        )
    };
    let document = fsl_replay::long_run_probability_certificate_document(
        &system,
        claims,
        &item,
        values,
        vec![component],
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    assert_eq!(
        document.optimization,
        fsl_replay::LongRunOptimization::Minimum
    );
    let subsystem = fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
        &document,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let binding = claim_binding(&system, world.claim(name)).unwrap().unwrap();
    ProbabilisticCounterexample {
        item,
        binding: Some(binding.reference),
        instance: None,
        initial: system.initial()[0].clone(),
        initial_key: down,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    }
}

/// The scheduled identity of the call that keeps the up state up.
fn call_staying_up(world: &World) -> ProductAction {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = &system.initial()[0];
    let start = system.state_key(initial).unwrap().0;
    let up = system
        .actions(initial)
        .unwrap()
        .unwrap()
        .into_iter()
        .flat_map(|action| action.draws)
        .flat_map(|draw| draw.successors)
        .map(|(_, post)| post)
        .find(|post| system.state_key(post).unwrap().0 != start)
        .expect("one call reaches the up state");
    let up_key = system.state_key(&up).unwrap().0;
    let action = system
        .actions(&up)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| {
            action.draws.iter().all(|draw| {
                draw.successors
                    .iter()
                    .all(|(_, post)| system.state_key(post).unwrap().0 == up_key)
            })
        })
        .expect("a call keeps the service up");
    ProductAction::Identity(Schedule::of(&system, &action.identity).unwrap())
}

fn policy_malformed(replayed: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>) {
    match replayed {
        Err(ProbabilityWitnessError::CertificateRejected(rejection)) => assert_eq!(
            rejection,
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member("policy"),
            }
        ),
        other => panic!("expected malformed policy: {other:?}"),
    }
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-27")]
#[test]
fn an_every_scheduler_long_run_refutation_is_replayed_over_the_chain_its_policy_induces() {
    let world = World::new(&architecture(
        false,
        &claim(
            "Fails",
            "every scheduler",
            "long_run_fraction holds(s.up) >= rational(1,2)",
        ),
    ));
    let (down, up) = keys(&world, "Fails");
    let stay = || {
        vec![(
            down.clone(),
            ProbabilisticSchedulerChoice::Deterministic(call(&world, false)),
        )]
    };

    // The deterministic policy keeps the service down: value 0 < 1/2.
    reproduced(replay(
        &world,
        &scheduled_long_run(&world, "Fails", stay(), false),
    ));

    // A witness whose induced value (1, staying up) meets the bound is not a
    // refutation: inequality-fails at the initial state.
    let climb = vec![
        (
            down.clone(),
            ProbabilisticSchedulerChoice::Deterministic(call(&world, true)),
        ),
        (
            up.clone(),
            ProbabilisticSchedulerChoice::Deterministic(call_staying_up(&world)),
        ),
    ];
    let mut climb = climb;
    climb.sort_by(|a, b| a.0.cmp(&b.0));
    rejected(
        replay(&world, &scheduled_long_run(&world, "Fails", climb, true)),
        ProbabilityCertificateRule::InequalityFails,
        &down,
    );

    // A randomised row, a choice naming no action and a missing row for a
    // reachable undecided state are each malformed at `policy`.
    let half = Rational::new(Integer::from(1i64), Integer::from(2i64)).unwrap();
    policy_malformed(replay(
        &world,
        &scheduled_long_run(
            &world,
            "Fails",
            vec![(
                down.clone(),
                ProbabilisticSchedulerChoice::Randomized(vec![
                    (call(&world, false), half.clone()),
                    (call(&world, true), half),
                ]),
            )],
            false,
        ),
    ));
    policy_malformed(replay(
        &world,
        &scheduled_long_run(
            &world,
            "Fails",
            vec![(
                down.clone(),
                ProbabilisticSchedulerChoice::Deterministic(ProductAction::TerminalStutter),
            )],
            false,
        ),
    ));
    policy_malformed(replay(
        &world,
        &scheduled_long_run(&world, "Fails", Vec::new(), false),
    ));
}

fn toggling(declarations: &str) -> prob_world::Model {
    prob_world::Model {
        package: "test/toggling",
        object: "Service",
        population: "services",
        key: "checkout",
        integers: vec![],
        fields: vec![("up", prob_world::BOOLEAN.into())],
        operations: vec![
            prob_world::Operation {
                name: "fail",
                parameters: vec![],
                modifies: vec!["up"],
                deletes: false,
            },
            prob_world::Operation {
                name: "repair",
                parameters: vec![],
                modifies: vec!["up"],
                deletes: false,
            },
        ],
        body: format!(
            "post Fails using v on M::Service::fail {{ self.up = false }}\n\
             post Repairs using v on M::Service::repair {{ self.up = true }}\n{declarations}"
        ),
        initial: json!({"up": {"boolean": true}}),
    }
}

/// The scheduled identity of operation `op` at `state` and the state it reaches.
fn take(
    world: &World,
    state: &fsl_eval::simulation::model::ModelSimState,
    op: &str,
) -> (ProductAction, fsl_eval::simulation::model::ModelSimState) {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let action = system
        .actions(state)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity.operation.contains(op))
        .expect("the operation is enabled");
    let post = action.draws[0].successors[0].1.clone();
    (
        ProductAction::Identity(Schedule::of(&system, &action.identity).unwrap()),
        post,
    )
}

/// A scheduler witness that fails the service at once and forever: one
/// component, the down state, of value 0.
fn failing_witness(world: &World, name: &str) -> ProbabilisticCounterexample {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    let binding = claim_binding(&system, world.claim(name)).unwrap().unwrap();
    let key = |state: &fsl_eval::simulation::model::ModelSimState| {
        let raw = system.state_key(state).unwrap().0;
        ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap()
    };
    let start = system.initial()[0].clone();
    let (fail_at_start, down) = take(world, &start, "fail");
    let (fail_at_down, _) = take(world, &down, "fail");
    let (up_key, down_key) = (key(&start), key(&down));
    let zero = Rational::new(Integer::from(0i64), Integer::from(1i64)).unwrap();
    let row = |state: &ProbabilityProductKey| fsl_replay::ExactProbabilityValue {
        state: state.clone(),
        value: zero.clone(),
    };
    let mut values = vec![row(&up_key), row(&down_key)];
    values.sort_by(|a, b| a.state.cmp(&b.state));
    let document = fsl_replay::long_run_probability_certificate_document(
        &system,
        claims,
        &item,
        values,
        vec![fsl_replay::LongRunProbabilityComponent {
            value: zero.clone(),
            gain: zero.clone(),
            bias: vec![row(&down_key)],
            policy: Some(vec![fsl_replay::LongRunPolicyRow {
                state: down_key.clone(),
                choice: fail_at_down.clone(),
            }]),
        }],
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let subsystem = fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
        &document,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let mut scheduler = vec![
        (
            up_key.clone(),
            ProbabilisticSchedulerChoice::Deterministic(fail_at_start),
        ),
        (
            down_key,
            ProbabilisticSchedulerChoice::Deterministic(fail_at_down),
        ),
    ];
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    ProbabilisticCounterexample {
        item,
        binding: Some(binding.reference),
        instance: None,
        initial: start,
        initial_key: up_key,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    }
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-27", "FSpec-FR-411-AC-18")]
#[test]
fn a_fairness_set_accepts_a_deterministic_witness_only_when_every_bottom_component_is_fair() {
    let world = World::new(&toggling(&format!(
        "{}{}",
        claim(
            "Free",
            "every scheduler",
            "long_run_fraction holds(s.up) >= rational(1,2)"
        ),
        claim(
            "Weak",
            "every scheduler fair weak Service::repair",
            "long_run_fraction holds(s.up) >= rational(1,2)"
        ),
    )));
    // No fairness set: failing forever is a legitimate scheduler, value 0.
    reproduced(replay(&world, &failing_witness(&world, "Free")));
    // Weakly fair `repair` is enabled at every state of the failing chain's
    // bottom component and never taken: the induced chain is unfair, so the
    // witness is rejected at the initial state.
    let (up, _) = {
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let binding = claim_binding(&system, world.claim("Weak"))
            .unwrap()
            .unwrap();
        let raw = system.state_key(&system.initial()[0]).unwrap().0;
        (
            ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap(),
            (),
        )
    };
    rejected(
        replay(&world, &failing_witness(&world, "Weak")),
        ProbabilityCertificateRule::InequalityFails,
        &up,
    );
    // The fair infimum is not attained by a deterministic policy here, so the
    // analyzer produces no witness and does not settle the claim refuted.
    let record = settle(&world, "Weak", ModelCheckLimits::default());
    assert_ne!(
        record.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted)
    );
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-27", "FSpec-FR-411-AC-18")]
#[test]
fn an_every_scheduler_long_run_claim_settles_refuted_with_its_scheduler_witness() {
    let world = World::new(&architecture(
        false,
        &format!(
            "{}{}",
            claim(
                "Fails",
                "every scheduler",
                "long_run_fraction holds(s.up) >= rational(1,2)"
            ),
            claim(
                "FairFails",
                "every scheduler fair weak Service::call",
                "long_run_fraction holds(s.up) >= rational(1,2)"
            ),
        ),
    ));
    let (down, _) = keys(&world, "Fails");
    for name in ["Fails", "FairFails"] {
        let record = refuted(&world, name);
        let found = evidence(&record);
        assert_eq!(found.initial_key, down, "{name}");
        // The scheduler keeps the service down, the document is the minimum
        // over every scheduler and the replay reproduced it.
        assert_eq!(
            found.scheduler,
            vec![(
                down.clone(),
                ProbabilisticSchedulerChoice::Deterministic(call(&world, false))
            )],
            "{name}"
        );
        let doc = document(&found);
        assert_eq!(doc["kind"], "long-run", "{name}");
        assert_eq!(doc["objective"]["optimization"], "minimum", "{name}");
        assert_eq!(
            doc["objective"]["fairness"].as_array().unwrap().len(),
            usize::from(name == "FairFails"),
            "{name}"
        );
        assert_eq!(doc["components"][0]["value"], q(0, 1), "{name}");
        reproduced(replay(&world, &found));
    }
}

#[trace("TC-3191", "FR-202-AC-45")]
#[test]
fn avail2_every_scheduler_long_run_refutations_replay_their_induced_chains() {
    let world = World::new(&prob_world::avail2(&format!(
        "{}{}{}{}",
        avail2_claim(
            "AtLeast",
            "every scheduler",
            "long_run_fraction holds(s.up) >= rational(9993, 10000)"
        ),
        avail2_claim(
            "AtMost",
            "every scheduler",
            "long_run_fraction holds(s.up) <= rational(999, 1000)"
        ),
        avail2_claim(
            "Proof",
            "every scheduler",
            "long_run_fraction holds(s.up) >= rational(1, 2)"
        ),
        avail2_claim(
            "EventPosition",
            "every scheduler",
            "long_run_fraction holds(s.up) >= rational(9993, 10000)"
        )
        .replace("using inf", "using ev"),
    )));

    let slow = evidence(&refuted(&world, "AtLeast"));
    let slow_document = document(&slow);
    assert_eq!(slow_document["kind"], "long-run");
    assert_eq!(slow_document["objective"]["optimization"], "minimum");
    assert_eq!(slow_document["components"][0]["value"], q(1000, 1001));
    assert_eq!(slow_document["components"][0]["gain"], q(1000, 1001));
    assert!(format!("{:?}", slow.scheduler).contains("repair_slow"));
    let (up, down) = keys(&world, "AtLeast");
    let bias = slow_document["components"][0]["bias"].as_array().unwrap();
    assert_eq!(
        bias.iter()
            .find(|row| row["state"] == encoded(&down))
            .unwrap()["value"],
        q(0, 1)
    );
    assert_eq!(
        bias.iter()
            .find(|row| row["state"] == encoded(&up))
            .unwrap()["value"],
        q(2000, 1001)
    );
    reproduced(replay(&world, &slow));

    let fast_at_least = fast_avail2_witness(&world);
    let fast_document = document(&fast_at_least);
    let mut wrong_fast_certificate = slow_document.clone();
    wrong_fast_certificate["components"][0]["policy"] =
        fast_document["components"][0]["policy"].clone();
    let fast_with_slow_certificate = offering(&fast_at_least, &wrong_fast_certificate);
    let wrong_replay = replay(&world, &fast_with_slow_certificate);
    assert!(
        matches!(
            &wrong_replay,
            Err(ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    at: ProbabilityCertificateLocus::ProductState(_),
                }
            ))
        ),
        "the slow certificate with fast policy is rejected: {wrong_replay:?}"
    );

    let fast = evidence(&refuted(&world, "AtMost"));
    assert!(format!("{:?}", fast.scheduler).contains("repair_fast"));
    assert_eq!(document(&fast)["components"][0]["value"], q(1800, 1801));
    reproduced(replay(&world, &fast));

    let event_position = evidence(&refuted(&world, "EventPosition"));
    assert!(format!("{:?}", event_position.scheduler).contains("repair_slow"));
    assert_eq!(
        document(&event_position)["components"][0]["value"],
        q(1000, 1001)
    );
    reproduced(replay(&world, &event_position));
    assert!(matches!(
        replay(&world, &offering(&event_position, &slow_document)),
        Err(ProbabilityWitnessError::Refused)
    ));

    assert_eq!(
        document(&fast_at_least)["components"][0]["value"],
        q(1800, 1801)
    );
    let proof_record = settle(&world, "Proof", ModelCheckLimits::default());
    assert!(matches!(
        proof_record.terminal().unwrap().value(),
        TerminalValue::Proved { .. }
    ));
    assert!(proof_record.probabilistic_counterexample.is_none());
    let proof_bytes = proof_record
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .expect("the accepted FR-201 MDP long-run proof has a document");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Proof")).unwrap();
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            proof_bytes,
            limits(),
            &Cancel::new(),
        ),
        Ok(fsl_replay::ExactProbabilityCertificateCheck::Accepted(_))
    ));
    assert!(!matches!(
        replay(&world, &fast_at_least),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));

    let mut missing_scheduler = slow.clone();
    missing_scheduler.scheduler.clear();
    assert!(!matches!(
        replay(&world, &missing_scheduler),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
    let mut randomized_scheduler = slow.clone();
    let slow_row = randomized_scheduler
        .scheduler
        .iter_mut()
        .find(|(_, choice)| format!("{choice:?}").contains("repair_slow"))
        .expect("the counterexample names repair_slow");
    slow_row.1 = ProbabilisticSchedulerChoice::Randomized(vec![
        (
            call_named(&world, "repair_slow"),
            Rational::new(Integer::from(1i64), Integer::from(2i64)).unwrap(),
        ),
        (
            call_named(&world, "repair_fast"),
            Rational::new(Integer::from(1i64), Integer::from(2i64)).unwrap(),
        ),
    ]);
    assert!(!matches!(
        replay(&world, &randomized_scheduler),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));

    let equality = World::new(&prob_world::avail2(&avail2_claim(
        "Equality",
        "every scheduler",
        "long_run_fraction holds(s.up) >= rational(1000, 1001)",
    )));
    let equality_record = settle(&equality, "Equality", ModelCheckLimits::default());
    assert_ne!(
        equality_record.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted)
    );
    assert!(equality_record.probabilistic_counterexample.is_none());
}
