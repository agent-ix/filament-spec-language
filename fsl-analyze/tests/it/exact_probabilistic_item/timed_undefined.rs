// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked replay of timed probabilistic undefined paths.

use super::*;
use serde_json::Value;

/// Timed formula, Until, elapsed-quantile and every-scheduler undefined routes
/// retain their complete pre-read control, product key, cause and time.
#[trace(
    "FR-202-AC-24",
    "FR-202-AC-25",
    "FR-196-AC-13",
    "FR-203-AC-5",
    "FR-202-AC-26",
    "FSpec-FR-411-AC-12",
    "FSpec-FR-413-AC-7",
    "FSpec-FR-413-AC-11"
)]
#[test]
fn timed_always_undefined_replays_at_any_threshold_and_rejects_decided_control() {
    let mut document: Value = serde_json::from_slice(&crate::timed_world::document(
        &["x"],
        &[("drop", &[]), ("keep", &[])],
    ))
    .expect("the timed fixture emits JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = serde_json::json!([{
        "identity": crate::timed_world::ident("Call/drop/b"),
        "name": "b",
        "typeRef": crate::timed_world::ident("Phase"),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": crate::timed_world::ident("Call/drop/b"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [crate::timed_world::ident("Call/drop/b")]
        }}
    }]);
    operation["frame"]["deletes"] = serde_json::json!([crate::timed_world::ident("Call")]);
    let body = format!(
        "{}\
         post Drop using v on M::Call::drop {{ (b = 0 and count(o in M::calls where true) = 0) or (b = 1 and count(o in M::calls where true) = 1) }}\n\
         random Call::drop(b) ~ {{ 0: 1, 1: 1 }};\n\
         delay M::Call::drop ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::drop = 1; weight Call::keep = 0; }}\n\
         probabilistic UndefinedHalf using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n\
         probabilistic UndefinedSmall using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.01 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n\
         probabilistic UndefinedUntil using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ holds(c.phase = 0) until[0 ms, 3 ms] holds(c.phase = 1) ] }}\n\
         probabilistic UndefinedElapsed using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ quantile 0.5 of (elapsed from holds(c.phase = 0) until holds(c.phase = 1) within 3 ms) <= 1 ms }}\n\
         probabilistic UndefinedEvery using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n\
         probabilistic UndefinedWeightedLongRun using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ long_run_fraction holds(c.phase = 0) weighted by delay >= 0.5 }}\n\
         probabilistic UndefinedMeanTime using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ expected elapsed until holds(c.phase = 1) <= 1 ms }}\n\
         probabilistic AlreadyDecided using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 1) ] }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document.to_string().into_bytes(),
        "profile timed = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let mut records = Vec::new();
    for name in [
        "UndefinedHalf",
        "UndefinedSmall",
        "UndefinedUntil",
        "UndefinedElapsed",
        "UndefinedEvery",
        "UndefinedWeightedLongRun",
        "UndefinedMeanTime",
    ] {
        let claim = world
            .package
            .graph()
            .probabilistic_model()
            .claim(name)
            .expect("the timed claim checks");
        let request = fsl_analyze::exact_probabilistic::ProductRequest {
            subject: fsl_eval::model_subject::ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        };
        let item = exact_probabilistic_item(RequestIndex::new(0), request);
        records.extend(analyze_items(&engines, vec![item], &Cancel::new()));
    }
    assert_eq!(records.len(), 7);
    for record in &records {
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Refuted,
            "record: {record:#?}"
        );
        let witness = record
            .probabilistic_counterexample
            .as_ref()
            .expect("both thresholds reproduce the same undefined path");
        assert_eq!(witness.mass(), &ratio(1, 2));
        assert_eq!(witness.undefined().unwrap().evaluation.site.position, 1);
    }
    let witness = records[0]
        .probabilistic_counterexample
        .as_ref()
        .expect("undefined path is replayed");
    let undefined = witness.undefined().expect("the cause is reproduced");
    let initial_key: Value = serde_json::from_slice(undefined.path.initial_key.as_bytes()).unwrap();
    assert_eq!(
        initial_key[5],
        serde_json::json!(["deadline", "always", "pending"])
    );
    assert_eq!(
        initial_key[4][1][5],
        serde_json::json!(["deadline", "3", "0"])
    );
    assert_eq!(undefined.path.paths.len(), 1);
    assert_eq!(undefined.path.paths[0].steps.len(), 1);
    assert_eq!(undefined.path.paths[0].steps[0].elapsed, ratio(1, 1));
    assert_eq!(undefined.evaluation.site.position, 1);
    assert_eq!(
        undefined.evaluation.site.time.as_ref().unwrap().stamp,
        ratio(1, 1)
    );
    assert_eq!(
        undefined.evaluation.site.time.as_ref().unwrap().clocks,
        Vec::<Rational>::new()
    );
    assert_eq!(witness.mass(), &ratio(1, 2));
    assert_eq!(
        witness.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"],
        "absent-key"
    );
    let key: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
    assert_eq!(key[5], serde_json::json!(["deadline", "always", "pending"]));
    assert_eq!(key[4][1][5], serde_json::json!(["deadline", "3", "1"]));
    assert_eq!(key[4][1][4][0][1], "1");

    let system = fsl_eval::simulation::model::ModelSystem::new(world.request()).unwrap();
    let decided = world
        .package
        .graph()
        .probabilistic_model()
        .claim("AlreadyDecided")
        .unwrap();
    let mut forged = undefined.clone();
    forged.path.item = fsl_replay::probabilistic_claim_identity(decided).unwrap();
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &forged.path.item,
            &forged,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        ),
        Ok(fsl_replay::ProbabilityWitnessReplay::Parity)
    ));

    let mut limits = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    limits.max_witness_paths = 0;
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &undefined.path.item,
            undefined,
            &limits,
            &Cancel::new(),
        ),
        Err(fsl_replay::ProbabilityWitnessError::Limit {
            limit: fsl_replay::ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
        })
    ));

    let mut forged_position = undefined.clone();
    forged_position.evaluation.site.position = 0;
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &forged_position.path.item,
            &forged_position,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        ),
        Ok(fsl_replay::ProbabilityWitnessReplay::Parity)
    ));
}

/// A tied digital race retains and replays every identity in its exact order.
#[trace(
    "FR-202-AC-24",
    "FR-202-AC-25",
    "FR-196-AC-13",
    "FR-203-AC-5",
    "FR-202-AC-26",
    "FSpec-FR-411-AC-12",
    "FSpec-FR-413-AC-7",
    "FSpec-FR-413-AC-11"
)]
#[test]
fn timed_undefined_replays_every_hop_of_a_tied_race() {
    let mut document: Value = serde_json::from_slice(&crate::timed_world::document(
        &["x"],
        &[("aa_keep", &[]), ("zz_drop", &[])],
    ))
    .expect("the timed fixture emits JSON");
    let operation = &mut document["types"][1]["operations"][1];
    operation["params"] = serde_json::json!([{
        "identity": crate::timed_world::ident("Call/zz_drop/b"),
        "name": "b",
        "typeRef": crate::timed_world::ident("Phase"),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": crate::timed_world::ident("Call/zz_drop/b"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [crate::timed_world::ident("Call/zz_drop/b")]
        }}
    }]);
    operation["frame"]["deletes"] = serde_json::json!([crate::timed_world::ident("Call")]);
    let body = format!(
        "{}\
         post Drop using v on M::Call::zz_drop {{ (b = 0 and count(o in M::calls where true) = 0) or (b = 1 and count(o in M::calls where true) = 1) }}\n\
         random Call::zz_drop(b) ~ {{ 0: 1, 1: 1 }};\n\
         delay M::Call::aa_keep ~ discrete {{ 1 ms: 1 }};\n\
         delay M::Call::zz_drop ~ discrete {{ 1 ms: 1 }};\n\
         workload Tied on M {{ weight Call::aa_keep = 1; weight Call::zz_drop = 1; }}\n\
         probabilistic TiedUndefined using timed on M over (c: M::Call) under Tied on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document.to_string().into_bytes(),
        "profile timed = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("TiedUndefined")
        .expect("the tied timed claim checks");
    let request = fsl_analyze::exact_probabilistic::ProductRequest {
        subject: fsl_eval::model_subject::ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim,
        limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
    };
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    let witness = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the first undefined read has a positive path");
    let undefined = witness.undefined().expect("the tied path replays");
    let steps = &undefined.path.paths[0].steps;
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].action.as_ref().unwrap().operation, "Call/aa_keep");
    assert_eq!(steps[1].action.as_ref().unwrap().operation, "Call/zz_drop");
    assert_eq!(steps[0].elapsed, ratio(1, 1));
    assert_eq!(steps[1].elapsed, ratio(0, 1));
    assert_eq!(undefined.evaluation.site.position, 2);
    assert_eq!(undefined.evaluation.site.time.as_ref().unwrap().stamp, ratio(1, 1));
    assert_eq!(witness.mass(), &ratio(1, 2));
    let intermediate: Value = serde_json::from_slice(steps[0].post_key.as_bytes()).unwrap();
    let final_key: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
    assert_eq!(intermediate[5], serde_json::json!(["deadline", "always", "pending"]));
    assert_eq!(final_key[5], serde_json::json!(["deadline", "always", "pending"]));
    assert_eq!(final_key[4][1][4][0][1], "1");
    assert_eq!(witness.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"], "absent-key");
}
