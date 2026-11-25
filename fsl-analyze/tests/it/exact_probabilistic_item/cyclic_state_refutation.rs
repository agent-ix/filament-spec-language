// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3162 to TC-3165: unwindowed `always` and `until` over a cyclic subject
//! are refuted through the event-avoidance subsystem certificate, on either
//! bound side, and never by a path-set search.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fsl_analyze::exact_probabilistic::{
    product_construction_counts, reset_product_construction_counts,
};
use fsl_replay::{
    ProbabilisticCounterexample, ProbabilisticSchedulerChoice, ProbabilityWitnessError,
    ProbabilityWitnessReplay, ProductAction,
};
use serde_json::{json, Value};

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

fn claim(name: &str, scheduler: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using inf on M over (x: M::Item) under {scheduler} on origin {{ {body} }}\n"
    )
}

fn item(phases: i64, operations: Vec<prob_world::Operation>, body: String) -> prob_world::Model {
    let package = "test/cyclic-state";
    prob_world::Model {
        package,
        object: "Item",
        population: "items",
        key: "x",
        integers: vec![("Phase", 0, phases)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations,
        body,
        initial: json!({"phase": {"integer": "0"}}),
    }
}

fn operation(name: &'static str, random: bool) -> prob_world::Operation {
    prob_world::Operation {
        name,
        parameters: if random {
            vec![("r", "ix://test/cyclic-state/Phase".into())]
        } else {
            vec![]
        },
        modifies: vec!["phase"],
        deletes: false,
    }
}

/// Settle `name`, counting the path-set searches it starts.
fn settle_counted(world: &World, name: &str) -> (fsl_analyze::analyze::AnalyzeRecord, usize) {
    reset_product_construction_counts();
    let record = settle(world, name, ModelCheckLimits::default());
    (record, product_construction_counts().path_searches)
}

fn refuted(world: &World, name: &str) -> ProbabilisticCounterexample {
    let (record, searches) = settle_counted(world, name);
    assert_eq!(
        searches, 0,
        "{name}: no path-set search on a cyclic subject"
    );
    assert_eq!(
        record
            .terminal()
            .unwrap_or_else(|| panic!("{name}: {:?}", record.settlement))
            .value(),
        &TerminalValue::Refuted,
        "{name}"
    );
    let found = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the replayed counterexample stays in the result")
        .evidence()
        .clone();
    assert!(found.paths.is_empty());
    found
}

/// A claim that holds over a cyclic subject is not refuted, emits no
/// counterexample and starts no path-set search. FR-202-AC-36/37 leave its
/// proof-side outcome to the proof profile: the unwindowed cyclic
/// state-predicate profile (QSL-2924) proves it, so it settles `proved`.
fn holds_unrefuted(world: &World, name: &str) {
    let (record, searches) = settle_counted(world, name);
    assert_eq!(searches, 0, "{name}");
    assert!(
        matches!(
            record
                .terminal()
                .unwrap_or_else(|| panic!("{name}: {:?}", record.settlement))
                .value(),
            TerminalValue::Proved { .. }
        ),
        "{name}: {:?}",
        record.settlement
    );
    assert!(record.probabilistic_counterexample.is_none(), "{name}");
}

fn document(found: &ProbabilisticCounterexample) -> Value {
    serde_json::from_slice(found.subsystem.as_ref().unwrap().as_bytes()).unwrap()
}

fn key_of(encoded: &str) -> ProbabilityProductKey {
    ProbabilityProductKey::from_canonical_bytes(URL_SAFE_NO_PAD.decode(encoded).unwrap())
}

fn encoded(key: &ProbabilityProductKey) -> String {
    URL_SAFE_NO_PAD.encode(key.as_bytes())
}

/// The named rows of a lower certificate, as (key, value, rank).
fn rows(document: &Value) -> Vec<(ProbabilityProductKey, Value, Value)> {
    let ranks = document["ranking"].as_array().unwrap();
    document["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let rank = ranks
                .iter()
                .find(|rank| rank["state"] == row["state"])
                .expect("each named state is ranked")["rank"]
                .clone();
            (
                key_of(row["state"].as_str().unwrap()),
                row["value"].clone(),
                rank,
            )
        })
        .collect()
}

/// The single named state other than the initial one.
fn other(found: &ProbabilisticCounterexample) -> ProbabilityProductKey {
    let named = rows(&document(found));
    let others = named
        .iter()
        .filter(|(key, _, _)| key != &found.initial_key)
        .collect::<Vec<_>>();
    assert_eq!(others.len(), 1, "{named:?}");
    others[0].0.clone()
}

fn expect_rows(
    found: &ProbabilisticCounterexample,
    mut expected: Vec<(ProbabilityProductKey, Value, u64)>,
) {
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    let doc = document(found);
    assert_eq!(doc["kind"], "lower");
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
}

/// The same rows as a counterexample-role certificate for another claim.
fn offered_for(
    world: &World,
    name: &str,
    found: &ProbabilisticCounterexample,
    edit: impl FnOnce(&mut Vec<(ProbabilityProductKey, Rational, u64)>),
) -> ProbabilisticCounterexample {
    let mut named = rows(&document(found))
        .into_iter()
        .map(|(key, value, rank)| {
            let value = Rational::new(
                value["numerator"].as_str().unwrap().parse().unwrap(),
                value["denominator"].as_str().unwrap().parse().unwrap(),
            )
            .unwrap();
            (key, value, rank.as_u64().unwrap())
        })
        .collect::<Vec<_>>();
    edit(&mut named);
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    let values = named
        .iter()
        .map(|(state, value, _)| fsl_replay::ExactProbabilityValue {
            state: state.clone(),
            value: value.clone(),
        })
        .collect::<Vec<_>>();
    let ranking = named
        .iter()
        .map(|(state, _, rank)| (state.clone(), *rank))
        .collect::<Vec<_>>();
    let mut offered = found.clone();
    offered.item = item.clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::value_bound(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            fsl_replay::SubsystemBoundKind::Lower,
            &values,
            Some(&ranking),
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

/// `ok` (phase 0) keeps P and moves to `ok` or `down` (phase 1) with 1/2
/// each; `down` breaks P and returns to `ok`.
fn health(predicate: &str, claims: &str) -> World {
    World::new(&item(
        1,
        vec![operation("tick", true), operation("repair", false)],
        format!(
            "pre Tick using v on M::Item::tick {{ self.phase = 0 }}\n\
             post Ticked using v on M::Item::tick {{ self.phase = r }}\n\
             pre Repair using v on M::Item::repair {{ self.phase = 1 }}\n\
             post Repaired using v on M::Item::repair {{ self.phase = 0 }}\n\
             random Item::tick(r) ~ {{ 0: 1, 1: 1 }};\n\
             workload W on M {{ weight Item::tick = 1; weight Item::repair = 1; }}\n{}{claims}",
            claim(
                "Always",
                "W",
                &format!("probability >= 0.9 [ always holds({predicate}) ]")
            ),
        ),
    ))
}

#[trace(
    "TC-3162",
    "FR-202-AC-36",
    "FR-202-AC-21",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn a_cyclic_workload_always_is_refuted_by_its_rejecting_union_without_a_path_search() {
    let world = health("x.phase = 0", "");
    let found = refuted(&world, "Always");
    assert!(found.scheduler.is_empty());
    // `refuted`, `decisive-counterexample`, retaining the replayed evidence;
    // neither `unsupported` nor `incomplete`.
    let record = settle(&world, "Always", ModelCheckLimits::default());
    let axes = fsl_replay::OutcomeAxes::of_record(record.terminal().unwrap().value(), false);
    assert_eq!(
        (axes.truth, axes.settlement),
        (
            Some(fsl_replay::TruthAxis::False),
            Some(fsl_replay::SettlementBasis::DecisiveCounterexample)
        )
    );
    assert!(record.probabilistic_counterexample.is_some());
    // The search counter is live: an acyclic counterpart, whose `ok` breaks
    // P once and then stops, does start the path-set search.
    let acyclic = World::new(&item(
        1,
        vec![operation("fail", false)],
        format!(
            "pre Fail using v on M::Item::fail {{ self.phase = 0 }}\n\
             post Failed using v on M::Item::fail {{ self.phase = 1 }}\n\
             workload W on M {{ weight Item::fail = 1; }}\n{}",
            claim(
                "Always",
                "W",
                "probability >= 0.9 [ always holds(x.phase = 0) ]"
            ),
        ),
    ));
    let (control, searches) = settle_counted(&acyclic, "Always");
    assert_eq!(control.terminal().unwrap().value(), &TerminalValue::Refuted);
    assert!(searches > 0, "the acyclic subject is refuted by a path set");
    let ok = found.initial_key.clone();
    let down = other(&found);
    expect_rows(
        &found,
        vec![(ok.clone(), q(1, 1), 1), (down.clone(), q(1, 1), 0)],
    );
    let doc = document(&found);
    assert_eq!(
        doc["objective"]["base"]["profile"],
        "filament.probability-objective/state-predicate/v1"
    );
    assert_eq!(doc["objective"]["base"]["event"], "always");
    assert!(matches!(
        replay(&world, &found),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
    let tenth = offered_for(&world, "Always", &found, |rows| {
        rows.iter_mut().find(|row| row.0 == ok).unwrap().1 = ratio(1, 10);
    });
    rejected(
        replay(&world, &tenth),
        ProbabilityCertificateRule::ThresholdMissed,
        &ok,
    );
    let bytes = tenth.subsystem.clone();
    let settled = settle_tampered(&world, "Always", move |witness| {
        witness.subsystem = bytes.clone();
    });
    assert_eq!(
        settled.terminal().unwrap().value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ThresholdMissed,
            at: CertificateLocus::ProbabilityProductState(ok.clone()),
        })
    );

    let kept = health("x.phase >= 0", "");
    holds_unrefuted(&kept, "Always");
}

/// `s` (0) keeps P and moves to `t` (1) or `u` (2) with 1/2 each; `t` breaks
/// P and returns to `s`; `u` keeps P and loops.
fn branch(claims: &str) -> World {
    World::new(&item(
        2,
        vec![
            operation("go", true),
            operation("back", false),
            operation("stay", false),
        ],
        format!(
            "pre Go using v on M::Item::go {{ self.phase = 0 }}\n\
             post Went using v on M::Item::go {{ self.phase = r }}\n\
             pre Back using v on M::Item::back {{ self.phase = 1 }}\n\
             post Backed using v on M::Item::back {{ self.phase = 0 }}\n\
             pre Stay using v on M::Item::stay {{ self.phase = 2 }}\n\
             post Stayed using v on M::Item::stay {{ self.phase = 2 }}\n\
             random Item::go(r) ~ {{ 1: 1, 2: 1 }};\n\
             workload W on M {{ weight Item::go = 1; weight Item::back = 1; weight Item::stay = 1; }}\n{claims}"
        ),
    ))
}

#[trace(
    "TC-3163",
    "FR-202-AC-37",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn cyclic_always_is_refuted_on_either_side_and_never_at_equality() {
    let always = |name: &str, bound: &str| {
        claim(
            name,
            "W",
            &format!("probability {bound} [ always holds(x.phase != 1) ]"),
        )
    };
    let world = branch(
        &[
            always("AtLeastHalf", ">= rational(1,2)"),
            always("AtMostHalf", "<= rational(1,2)"),
            always("AtLeast", ">= rational(3,5)"),
            always("AtMost", "<= rational(2,5)"),
        ]
        .concat(),
    );
    holds_unrefuted(&world, "AtLeastHalf");
    holds_unrefuted(&world, "AtMostHalf");

    let rejecting = refuted(&world, "AtLeast");
    let s = rejecting.initial_key.clone();
    let t = other(&rejecting);
    expect_rows(
        &rejecting,
        vec![(s.clone(), q(1, 2), 1), (t.clone(), q(1, 1), 0)],
    );
    let accepting = refuted(&world, "AtMost");
    let u = other(&accepting);
    expect_rows(
        &accepting,
        vec![(s.clone(), q(1, 2), 1), (u.clone(), q(1, 1), 0)],
    );

    for (name, found) in [("AtLeastHalf", &rejecting), ("AtMostHalf", &accepting)] {
        rejected(
            replay(&world, &offered_for(&world, name, found, |_| {})),
            ProbabilityCertificateRule::ThresholdMissed,
            &s,
        );
    }
    let accepted_u = offered_for(&world, "AtLeast", &rejecting, |rows| {
        rows.push((u.clone(), ratio(1, 1), 1));
    });
    rejected(
        replay(&world, &accepted_u),
        ProbabilityCertificateRule::RankBroken,
        &u,
    );
}

/// Every scheduler: `ok` (0) keeps P with `stay` to itself or `leave` to
/// `down` (1); `down` breaks P and returns.
#[trace("TC-3164", "FR-202-AC-38", "FSpec-FR-412-AC-25", "FSpec-FR-413-AC-18")]
#[test]
fn cyclic_always_over_every_scheduler_carries_its_witness_scheduler() {
    let always = |name: &str, bound: &str| {
        claim(
            name,
            "every scheduler",
            &format!("probability {bound} [ always holds(x.phase = 0) ]"),
        )
    };
    let world = World::new(&item(
        1,
        vec![
            operation("stay", false),
            operation("leave", false),
            operation("back", false),
        ],
        format!(
            "pre Stay using v on M::Item::stay {{ self.phase = 0 }}\n\
             post Stayed using v on M::Item::stay {{ self.phase = 0 }}\n\
             pre Leave using v on M::Item::leave {{ self.phase = 0 }}\n\
             post Left using v on M::Item::leave {{ self.phase = 1 }}\n\
             pre Back using v on M::Item::back {{ self.phase = 1 }}\n\
             post Backed using v on M::Item::back {{ self.phase = 0 }}\n{}{}",
            always("AtLeast", ">= 0.9"),
            always("AtMost", "<= rational(1,2)"),
        ),
    ));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = &system.initial()[0];
    let schedule = |name: &str| {
        let action = system
            .actions(initial)
            .unwrap()
            .unwrap()
            .into_iter()
            .find(|action| action.identity.operation.contains(name))
            .unwrap_or_else(|| panic!("{name} is enabled at ok"));
        ProductAction::Identity(fsl_replay::Schedule::of(&system, &action.identity).unwrap())
    };
    let (stay, leave) = (schedule("stay"), schedule("leave"));

    let found = refuted(&world, "AtLeast");
    let ok = found.initial_key.clone();
    assert_eq!(
        found.scheduler,
        vec![(
            ok.clone(),
            ProbabilisticSchedulerChoice::Deterministic(leave.clone())
        )]
    );
    let down = other(&found);
    expect_rows(
        &found,
        vec![(ok.clone(), q(1, 1), 1), (down.clone(), q(1, 1), 0)],
    );

    let mut missing = found.clone();
    missing.scheduler.clear();
    assert!(matches!(
        replay(&world, &missing),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut randomized = found.clone();
    randomized.scheduler[0].1 = ProbabilisticSchedulerChoice::Randomized(vec![
        (stay.clone(), ratio(1, 2)),
        (leave.clone(), ratio(1, 2)),
    ]);
    assert!(matches!(
        replay(&world, &randomized),
        Err(ProbabilityWitnessError::Refused)
    ));
    // Under `stay`, `{ok}` is the induced chain's only component and it
    // accepts, so the rejecting union is empty. AC-36's certificate names the
    // unreached `down` and cannot cover the chain at all.
    let mut staying = found.clone();
    staying.scheduler[0].1 = ProbabilisticSchedulerChoice::Deterministic(stay.clone());
    rejected_member(replay(&world, &staying), "values");
    // Restricted to `ok`, it is checked over that chain: `ok` keeps value 1
    // only by returning to itself, so it has no progress towards a rejecting
    // state.
    let mut only_ok = offered_for(&world, "AtLeast", &found, |rows| {
        rows.retain(|row| row.0 == ok);
    });
    only_ok.scheduler[0].1 = ProbabilisticSchedulerChoice::Deterministic(stay.clone());
    rejected(
        replay(&world, &only_ok),
        ProbabilityCertificateRule::RankBroken,
        &ok,
    );

    let kept = refuted(&world, "AtMost");
    assert_eq!(
        kept.scheduler,
        vec![(
            ok.clone(),
            ProbabilisticSchedulerChoice::Deterministic(stay)
        )]
    );
    expect_rows(&kept, vec![(ok, q(1, 1), 0)]);
}

#[trace(
    "TC-3165",
    "FR-202-AC-39",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn cyclic_until_is_refuted_through_its_classified_components() {
    // Two A-and-not-B states alternate: no decided state, so the component
    // rejects and the until value is 0.
    let alternating = World::new(&item(
        2,
        vec![operation("flip", false)],
        format!(
            "pre Flip using v on M::Item::flip {{ self.phase < 2 }}\n\
             post Flipped using v on M::Item::flip {{ (pre(self.phase) = 0 and self.phase = 1) or (pre(self.phase) = 1 and self.phase = 0) }}\n\
             workload W on M {{ weight Item::flip = 1; }}\n{}",
            claim(
                "Until",
                "W",
                "probability >= rational(1,2) [ holds(x.phase < 2) until holds(x.phase = 2) ]"
            ),
        ),
    ));
    let found = refuted(&alternating, "Until");
    let first = found.initial_key.clone();
    let second = other(&found);
    expect_rows(&found, vec![(first, q(1, 1), 0), (second, q(1, 1), 0)]);

    // `s` (0, A) moves to itself 1/2, `b` (1, B) 1/4 and `n` (2, neither)
    // 1/4; `b` and `n` return to `s`, yet absorb as decided states.
    let until = |name: &str, bound: &str| {
        claim(
            name,
            "W",
            &format!("probability {bound} [ holds(x.phase = 0) until holds(x.phase = 1) ]"),
        )
    };
    let world = World::new(&item(
        2,
        vec![operation("go", true), operation("back", false)],
        format!(
            "pre Go using v on M::Item::go {{ self.phase = 0 }}\n\
             post Went using v on M::Item::go {{ self.phase = r }}\n\
             pre Back using v on M::Item::back {{ self.phase > 0 }}\n\
             post Backed using v on M::Item::back {{ self.phase = 0 }}\n\
             random Item::go(r) ~ {{ 0: 2, 1: 1, 2: 1 }};\n\
             workload W on M {{ weight Item::go = 1; weight Item::back = 1; }}\n{}{}{}{}",
            until("AtLeastHalf", ">= rational(1,2)"),
            until("AtMostHalf", "<= rational(1,2)"),
            until("AtLeast", ">= rational(3,5)"),
            until("AtMost", "<= rational(2,5)"),
        ),
    ));
    let rejecting = refuted(&world, "AtLeast");
    let s = rejecting.initial_key.clone();
    let n = other(&rejecting);
    expect_rows(&rejecting, vec![(s.clone(), q(1, 2), 1), (n, q(1, 1), 0)]);
    let accepting = refuted(&world, "AtMost");
    let b = other(&accepting);
    expect_rows(&accepting, vec![(s.clone(), q(1, 2), 1), (b, q(1, 1), 0)]);
    for (name, found) in [("AtLeastHalf", &rejecting), ("AtMostHalf", &accepting)] {
        rejected(
            replay(&world, &offered_for(&world, name, found, |_| {})),
            ProbabilityCertificateRule::ThresholdMissed,
            &s,
        );
    }
}

#[trace(
    "TC-3171",
    "FR-202-AC-41",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn a_cyclic_eventually_at_most_is_refuted_through_its_accepting_union() {
    // `s` (0) misses E and moves to `s` or `e` (1) with 1/2 each; `e`
    // satisfies E and returns to `s`.
    let world = World::new(&item(
        1,
        vec![operation("go", true), operation("back", false)],
        format!(
            "pre Go using v on M::Item::go {{ self.phase = 0 }}\n\
             post Went using v on M::Item::go {{ self.phase = r }}\n\
             pre Back using v on M::Item::back {{ self.phase = 1 }}\n\
             post Backed using v on M::Item::back {{ self.phase = 0 }}\n\
             random Item::go(r) ~ {{ 0: 1, 1: 1 }};\n\
             workload W on M {{ weight Item::go = 1; weight Item::back = 1; }}\n{}",
            claim(
                "Eventually",
                "W",
                "probability <= rational(1,2) [ eventually holds(x.phase = 1) ]"
            ),
        ),
    ));
    let found = refuted(&world, "Eventually");
    assert!(found.scheduler.is_empty());
    let s = found.initial_key.clone();
    let e = other(&found);
    expect_rows(&found, vec![(s.clone(), q(1, 1), 1), (e, q(1, 1), 0)]);
    let half = offered_for(&world, "Eventually", &found, |rows| {
        rows.iter_mut().find(|row| row.0 == s).unwrap().1 = ratio(1, 2);
    });
    rejected(
        replay(&world, &half),
        ProbabilityCertificateRule::ThresholdMissed,
        &s,
    );
    let bytes = half.subsystem.clone();
    let settled = settle_tampered(&world, "Eventually", move |witness| {
        witness.subsystem = bytes.clone();
    });
    assert_eq!(
        settled.terminal().unwrap().value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ThresholdMissed,
            at: CertificateLocus::ProbabilityProductState(s),
        })
    );
    assert!(settled.probabilistic_counterexample.is_none());
}

/// Phase 0 and phase 1 alternate deterministically under a workload.
fn alternating(claims: &str) -> World {
    World::new(&item(
        1,
        vec![operation("step", false), operation("ret", false)],
        format!(
            "pre Step using v on M::Item::step {{ self.phase = 0 }}\n\
             post Stepped using v on M::Item::step {{ self.phase = 1 }}\n\
             pre Ret using v on M::Item::ret {{ self.phase = 1 }}\n\
             post Returned using v on M::Item::ret {{ self.phase = 0 }}\n\
             workload W on M {{ weight Item::step = 1; weight Item::ret = 1; }}\n{claims}"
        ),
    ))
}

#[trace(
    "TC-3172",
    "FR-202-AC-42",
    "FR-202-AC-21",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn initial_decided_cyclic_subjects_are_refuted_not_unsupported() {
    // Until: the initial `b` (phase 0) satisfies B and not A; `s` (phase 1)
    // satisfies A and not B. Always: the initial `f` (phase 0) breaks P.
    let world = alternating(
        &(claim(
            "Until",
            "W",
            "probability <= rational(1,2) [ holds(x.phase = 1) until holds(x.phase = 0) ]",
        ) + &claim(
            "Always",
            "W",
            "probability >= rational(1,2) [ always holds(x.phase = 1) ]",
        )),
    );
    for name in ["Until", "Always"] {
        let found = refuted(&world, name);
        assert!(found.scheduler.is_empty());
        expect_rows(&found, vec![(found.initial_key.clone(), q(1, 1), 0)]);
        // The path-set entry itself admits no cycle and reproduces nothing.
        let mut paths = found.clone();
        paths.subsystem = None;
        let replayed = replay(&world, &paths);
        assert!(
            matches!(
                replayed,
                Err(ProbabilityWitnessError::Graph(
                    ExactProbabilityGraphError::Cycle
                ))
            ),
            "{name}: {replayed:?}"
        );
    }
}

#[trace(
    "TC-3172",
    "FR-202-AC-46",
    "FSpec-TC-433",
    "FSpec-FR-412-AC-25",
    "FSpec-FR-412-AC-28",
    "FSpec-FR-413-AC-18"
)]
#[test]
fn a_repeatable_operation_cycle_refutes_always_and_empty_event_without_path_search() {
    let world = World::new(&item(
        1,
        vec![operation("call", false)],
        format!(
            "pre Call using v on M::Item::call {{ self.phase = 0 }}\n\
             post Called using v on M::Item::call {{ self.phase = 0 }}\n\
             workload W on M {{ weight Item::call = 1; }}\n{}{}",
            claim(
                "Always",
                "W",
                "probability >= rational(1,2) [ always holds(x.phase = 1) ]",
            ),
            claim(
                "EmptyEvent",
                "W",
                "probability >= rational(1,2) [ eventually holds(x.phase = 1) ]",
            ),
        ),
    ));

    for name in ["Always", "EmptyEvent"] {
        let found = refuted(&world, name);
        assert!(found.scheduler.is_empty(), "{name}");
        assert!(found.paths.is_empty(), "{name}");
        expect_rows(&found, vec![(found.initial_key.clone(), q(1, 1), 0)]);
        assert!(
            matches!(
                replay(&world, &found),
                Ok(ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{name}: subsystem replay reproduces the cyclic certificate"
        );

        let mut path_set = found;
        path_set.subsystem = None;
        assert!(
            matches!(
                replay(&world, &path_set),
                Err(ProbabilityWitnessError::Graph(
                    ExactProbabilityGraphError::Cycle
                ))
            ),
            "{name}: the path-set entry reports the raw graph cycle"
        );
    }
}

#[trace("FSpec-TC-433", "FSpec-FR-412-AC-28", "FR-202-AC-46")]
#[test]
fn a_fair_minimum_ranking_over_the_repeatable_operation_cycle_is_malformed() {
    let fair_claim = claim(
        "FairEmpty",
        "every scheduler fair strong Item::call",
        "probability >= rational(1,2) [ eventually holds(x.phase = 1) ]",
    );
    let world = World::new(&item(
        1,
        vec![operation("call", false)],
        format!(
            "pre Call using v on M::Item::call {{ self.phase = 0 }}\n\
             post Called using v on M::Item::call {{ self.phase = 0 }}\n{}",
            fair_claim
        ),
    ));

    let checked_claim = world.claim("FairEmpty");
    let item_identity = fsl_replay::probabilistic_claim_identity(checked_claim).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let binding = claim_binding(&system, checked_claim).unwrap().unwrap();
    let raw = system
        .state_key(
            &system.initial()[0],
            fsl_foundation::IdentityLimits::default(),
        )
        .unwrap()
        .0;
    let initial = ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap();
    let malformed = json!({
        "identity": {
            "node_id": fsl_foundation::node_wire::node_key_to_json(item_identity.node()).unwrap(),
            "model": item_identity.model().map(|model| model.as_str()),
            "ordinal": item_identity.ordinal(),
        },
        "objective": {
            "profile": "filament.probability-objective/value-bounds/v1",
            "base": {
                "profile": "filament.probability-objective/cyclic-mdp-fair-eventual/v1",
            },
        },
        "kind": "lower",
        "values": [],
        "ranking": [{"state": encoded(&initial), "rank": 1}],
    });
    let malformed = fsl_foundation::canonical_json::encode(&malformed).unwrap();
    let subsystem = fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
        &malformed,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let witness = ProbabilisticCounterexample {
        item: item_identity,
        binding: Some(binding.reference),
        instance: None,
        initial: system.initial()[0].clone(),
        initial_key: initial,
        scheduler: Vec::new(),
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    assert!(matches!(
        replay(&world, &witness),
        Err(ProbabilityWitnessError::CertificateRejected(
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member("ranking"),
            }
        ))
    ));
    let initial_claims = format!(
        "pre Call using v on M::Item::call {{ self.phase = 0 }}\n\
         post Called using v on M::Item::call {{ self.phase = 0 }}\n\
         workload W on M {{ weight Item::call = 1; }}\n{}",
        claim(
            "Always",
            "W",
            "probability >= rational(1,2) [ always holds(x.phase = 1) ]",
        )
    );
    let initial_world = World::new(&item(1, vec![operation("call", false)], initial_claims));
    let initial_verdict = refuted(&initial_world, "Always");
    assert!(
        matches!(
            replay(&initial_world, &initial_verdict),
            Ok(ProbabilityWitnessReplay::Reproduced(_))
        ),
        "rejecting the fair-minimum ranking does not alter the initial refutation"
    );
}

fn rejected_member(
    replayed: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>,
    member: &'static str,
) {
    match replayed {
        Err(ProbabilityWitnessError::CertificateRejected(rejection)) => assert_eq!(
            rejection,
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member(member),
            }
        ),
        other => panic!("expected Malformed at {member}: {other:?}"),
    }
}
