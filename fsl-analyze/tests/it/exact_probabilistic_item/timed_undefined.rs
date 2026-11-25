// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked replay of timed probabilistic undefined paths.

use super::*;
use serde_json::Value;

const DROP_OPERATION: &str = r#"{"node":"ix://test/timed/Call/drop","package":"test/timed"}"#;
const AA_DROP_OPERATION: &str = r#"{"node":"ix://test/timed/Call/aa_drop","package":"test/timed"}"#;
const AA_KEEP_OPERATION: &str = r#"{"node":"ix://test/timed/Call/aa_keep","package":"test/timed"}"#;
const ZZ_DROP_OPERATION: &str = r#"{"node":"ix://test/timed/Call/zz_drop","package":"test/timed"}"#;
const ZZ_KEEP_OPERATION: &str = r#"{"node":"ix://test/timed/Call/zz_keep","package":"test/timed"}"#;

/// Timed formula, Until, elapsed-quantile and every-scheduler undefined routes
/// retain their complete pre-read control, product key, cause and time.
#[trace(
    "FR-202-AC-24",
    "FR-202-AC-25",
    "FR-196-AC-13",
    "FR-203-AC-5",
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
         reward R on M::Call::drop = 0;\n\
         delay M::Call::drop ~ discrete {{ 1 ms: 1 }};\n\
         delay M::Call::keep ~ discrete {{ 1 ms: 1 }};\n\
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
         probabilistic UndefinedElapsed using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ quantile 0.5 of (elapsed from holds(c.phase = 0) until holds(c.phase = 1) within 3 ms) <= 1 ms }}\n\
         probabilistic UndefinedElapsedRegion using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ quantile 0.5 of (elapsed from holds(c.phase = 0) until holds(c.phase = 1) within 3 ms) >= 1 ms }}\n\
         probabilistic UndefinedEvery using timed on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n\
         probabilistic UndefinedWeightedLongRun using inf on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ long_run_fraction holds(c.phase = 0) weighted by delay >= 0.5 }}\n\
         probabilistic UndefinedEveryWeightedLongRun using inf on M over (c: M::Call) under every scheduler on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ long_run_fraction holds(c.phase = 0) weighted by R >= 0.5 }}\n\
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
        "profile timed = \"filament.temporal.timed/v1\";\nprofile inf = \"filament.temporal.infinite-trace/v1\";\n",
        &body,
    );
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let mut records = Vec::new();
    let names = [
        "UndefinedHalf",
        "UndefinedSmall",
        "UndefinedUntil",
        "UndefinedElapsed",
        "UndefinedElapsedRegion",
        "UndefinedEvery",
        "UndefinedWeightedLongRun",
        "UndefinedEveryWeightedLongRun",
        "UndefinedMeanTime",
    ];
    for name in names {
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
        if matches!(name, "UndefinedElapsed" | "UndefinedElapsedRegion") {
            let product = fsl_analyze::exact_probabilistic::build_product(
                fsl_analyze::exact_probabilistic::ProductRequest {
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation: world.request(),
                        proof_bounds: Vec::new(),
                    },
                    claim,
                    limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
                },
                &Cancel::new(),
            );
            assert!(
                matches!(
                    product,
                    Ok(fsl_analyze::exact_probabilistic::ProbBuild::Undefined(_))
                ),
                "elapsed measure build did not stop at its undefined read: {product:#?}"
            );
        }
        if matches!(
            name,
            "UndefinedWeightedLongRun" | "UndefinedEveryWeightedLongRun"
        ) {
            let product = fsl_analyze::exact_probabilistic::build_product(
                fsl_analyze::exact_probabilistic::ProductRequest {
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation: world.request(),
                        proof_bounds: Vec::new(),
                    },
                    claim,
                    limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
                },
                &Cancel::new(),
            );
            assert!(
                matches!(
                    product,
                    Ok(fsl_analyze::exact_probabilistic::ProbBuild::Undefined(_))
                ),
                "weighted long-run build did not stop at its undefined read: {product:#?}"
            );
        }
        let item = exact_probabilistic_item(RequestIndex::new(0), request);
        records.extend(analyze_items(&engines, vec![item], &Cancel::new()));
    }
    assert_eq!(records.len(), 9);
    for (name, record) in names.iter().zip(&records) {
        assert_eq!(
            record
                .terminal()
                .unwrap_or_else(|| panic!("{name} did not settle: {record:#?}"))
                .value(),
            &TerminalValue::Refuted,
            "{name} record: {record:#?}"
        );
        let witness = record
            .probabilistic_counterexample
            .as_ref()
            .unwrap_or_else(|| panic!("{name} has no replayed undefined path"));
        assert_eq!(witness.mass(), &ratio(1, 2));
        let undefined = witness.undefined().expect("the undefined read is replayed");
        let product_key: Value =
            serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
        assert_eq!(
            product_key[0], "filament.probability-product-key/v1",
            "{name} key schema"
        );
        let expected_route = if *name == "UndefinedElapsedRegion" {
            "region"
        } else {
            "digital"
        };
        assert_eq!(product_key[4][1][0], expected_route, "{name} timed route");
        if *name != "UndefinedElapsedRegion" {
            assert_eq!(product_key[4][1][4][0][1], "1", "{name} clock value");
        }
        let expected_monitor = match *name {
            "UndefinedHalf" | "UndefinedSmall" | "UndefinedEvery" => {
                serde_json::json!(["deadline", "always", "pending"])
            }
            "UndefinedUntil" => serde_json::json!(["deadline", "until", "pending"]),
            "UndefinedElapsed" => serde_json::json!(["measure", null, "active"]),
            "UndefinedElapsedRegion" => serde_json::json!(["measure", null, "active"]),
            "UndefinedWeightedLongRun" | "UndefinedEveryWeightedLongRun" | "UndefinedMeanTime" => {
                Value::Null
            }
            _ => panic!("unexpected timed undefined fixture: {name}"),
        };
        assert_eq!(product_key[5], expected_monitor, "{name} monitor key");
        assert_eq!(undefined.evaluation.site.position, 1, "{name}");
        assert_eq!(
            undefined.evaluation.site.time.as_ref().unwrap().stamp,
            ratio(1, 1),
            "{name} pre-read time"
        );
        assert_eq!(undefined.path.paths[0].steps.len(), 1, "{name} path length");
        assert_eq!(
            undefined.path.paths[0].steps[0].elapsed,
            ratio(1, 1),
            "{name} transition duration"
        );
        assert_eq!(
            undefined.path.paths[0].steps[0]
                .action
                .as_ref()
                .unwrap()
                .operation,
            DROP_OPERATION,
            "{name} undefined-producing action"
        );
        assert_eq!(
            witness.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"],
            serde_json::json!("absent-key"),
            "{name} undefined cause"
        );
        let every_scheduler = matches!(
            *name,
            "UndefinedElapsed"
                | "UndefinedElapsedRegion"
                | "UndefinedEvery"
                | "UndefinedEveryWeightedLongRun"
                | "UndefinedMeanTime"
        );
        assert_eq!(
            undefined.path.scheduler.len(),
            usize::from(every_scheduler),
            "{name} scheduler rows"
        );
        if every_scheduler {
            assert_eq!(
                undefined.path.scheduler[0].1,
                fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
                    undefined.path.paths[0].steps[0].choice.clone()
                ),
                "{name} selected scheduler action"
            );
        }
    }
    let every = records[5]
        .probabilistic_counterexample
        .as_ref()
        .expect("Every's selected drop path is replayed");
    let every_undefined = every
        .undefined()
        .expect("Every has a checked undefined path");
    let system = fsl_eval::simulation::model::ModelSystem::new(world.request()).unwrap();
    let initial = system.initial().remove(0);
    let keep = system
        .actions_with_cancel(&initial, &Cancel::new())
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity.operation == "Call/keep")
        .expect("keep is enabled beside drop");
    let mut forged_keep = every_undefined.clone();
    forged_keep.path.scheduler[0].1 = fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
        fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(&system, &keep.identity).unwrap(),
        ),
    );
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &forged_keep.path.item,
            &forged_keep,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        ),
        Err(fsl_replay::ProbabilityWitnessError::Refused)
    ));
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
    let cancelled = Cancel::new();
    cancelled.cancel(filament_exact::CancelCause::Requested);
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &undefined.path.item,
            undefined,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &cancelled,
        ),
        Err(fsl_replay::ProbabilityWitnessError::Graph(
            fsl_replay::ExactProbabilityGraphError::Cancelled(
                filament_exact::CancelCause::Requested
            )
        ))
    ));

    let mut forged_timed_key = undefined.clone();
    forged_timed_key.path.paths[0].steps[0].post_key = forged_timed_key.path.initial_key.clone();
    assert!(matches!(
        fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &forged_timed_key.path.item,
            &forged_timed_key,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        ),
        Ok(fsl_replay::ProbabilityWitnessReplay::Parity)
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
    "FR-196-AC-13",
    "FR-203-AC-5",
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
             {{ probability >= 0.5 [ always[0 ms, 3 ms) holds(c.phase = 0) ] }}\n",
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
    assert_eq!(
        steps[0].action.as_ref().unwrap().operation,
        AA_KEEP_OPERATION
    );
    assert_eq!(
        steps[1].action.as_ref().unwrap().operation,
        ZZ_DROP_OPERATION
    );
    assert_eq!(steps[0].elapsed, ratio(1, 1));
    assert_eq!(steps[1].elapsed, ratio(0, 1));
    assert_eq!(undefined.evaluation.site.position, 2);
    assert_eq!(
        undefined.evaluation.site.time.as_ref().unwrap().stamp,
        ratio(1, 1)
    );
    assert_eq!(witness.mass(), &ratio(1, 2));
    let intermediate: Value = serde_json::from_slice(steps[0].post_key.as_bytes()).unwrap();
    let final_key: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
    assert_eq!(
        intermediate[5],
        serde_json::json!(["deadline", "always", "pending"])
    );
    assert_eq!(
        final_key[5],
        serde_json::json!(["deadline", "always", "pending"])
    );
    assert_eq!(final_key[4][1][4][0][1], "1");
    assert_eq!(
        witness.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"],
        "absent-key"
    );
}

/// A precomputed immediate hop after the first undefined read is excluded
/// from the witness path and cannot advance its site or control.
#[trace(
    "FR-196-AC-13",
    "FR-203-AC-5",
    "FSpec-FR-411-AC-12",
    "FSpec-FR-413-AC-7",
    "FSpec-FR-413-AC-11"
)]
#[test]
fn timed_undefined_tied_chain_stops_at_its_first_undefined_hop() {
    let mut document: Value = serde_json::from_slice(&crate::timed_world::document(
        &["x"],
        &[("aa_drop", &[]), ("zz_keep", &[])],
    ))
    .expect("the timed fixture emits JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = serde_json::json!([{
        "identity": crate::timed_world::ident("Call/aa_drop/b"),
        "name": "b",
        "typeRef": crate::timed_world::ident("Phase"),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": crate::timed_world::ident("Call/aa_drop/b"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [crate::timed_world::ident("Call/aa_drop/b")]
        }}
    }]);
    operation["frame"]["deletes"] = serde_json::json!([]);
    operation["frame"]["deletes_receiver"] = serde_json::json!(true);
    let body = format!(
        "{}\
         post Drop using v on M::Call::aa_drop {{ b = 0 and count(o in M::calls where true) = 0 }}\n\
         random Call::aa_drop(b) ~ {{ 0: 1 }};\n\
         delay M::Call::aa_drop ~ discrete {{ 1 ms: 1 }};\n\
         delay M::Call::zz_keep ~ discrete {{ 1 ms: 1 }};\n\
         workload Tied on M {{ weight Call::aa_drop = 1; weight Call::zz_keep = 1; }}\n\
         probabilistic FirstUndefined using timed on M over (c: M::Call) under Tied on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms) holds(c.phase = 0) ] }}\n",
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
        .claim("FirstUndefined")
        .expect("the tied timed claim checks");
    let system = fsl_eval::simulation::model::ModelSystem::new(world.request())
        .expect("the tied timed model admits");
    assert!(
        system.source_package().is_some(),
        "the replay package is retained"
    );
    assert!(
        matches!(
            &claim.form,
            fsl_semantics::check::ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula {
                    timed_horizon: Some(_),
                    ..
                },
                ..
            }
        ),
        "the timed formula dispatch is selected: {claim:#?}"
    );
    let initial = system.initial().remove(0);
    let initially_enabled = system
        .actions_with_cancel(&initial, &Cancel::new())
        .unwrap()
        .unwrap();
    assert!(
        initially_enabled
            .iter()
            .any(|action| action.identity.operation == AA_DROP_OPERATION)
    );
    assert!(
        initially_enabled
            .iter()
            .any(|action| action.identity.operation == ZZ_KEEP_OPERATION)
    );
    let drop = system
        .actions_with_cancel(&initial, &Cancel::new())
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity.operation == AA_DROP_OPERATION)
        .expect("the first tied hop is enabled");
    let zero_draw = drop
        .draws
        .into_iter()
        .find(|draw| {
            matches!(
                draw.random.as_slice(),
                [filament_exact::Value::Integer(value)] if value == &Integer::from(0_i64)
            )
        })
        .expect("drop(0) removes the bound object");
    let (_, after_drop) = zero_draw
        .successors
        .into_iter()
        .next()
        .expect("drop(0) has one successor");
    assert!(
        system
            .actions_with_cancel(&after_drop, &Cancel::new())
            .unwrap()
            .unwrap()
            .iter()
            .all(|action| action.identity.operation != ZZ_KEEP_OPERATION),
        "aa_drop deletes c, so the already-tied zz_keep hop is unavailable"
    );
    let limits = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    let product = fsl_analyze::exact_probabilistic::build_product(
        fsl_analyze::exact_probabilistic::ProductRequest {
            subject: fsl_eval::model_subject::ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: limits.clone(),
        },
        &Cancel::new(),
    )
    .expect("the first undefined timed read is a product outcome");
    let fsl_analyze::exact_probabilistic::ProbBuild::Undefined(path) = product else {
        panic!("expected the undefined read at the first tied hop: {product:#?}");
    };
    assert_eq!(path.position, 1);
    assert_eq!(path.steps.len(), 1);
    assert_eq!(
        path.steps[0].action.as_ref().unwrap().operation,
        AA_DROP_OPERATION
    );
    // The equal-weight race selects aa_drop(c) first with probability 1/2;
    // its precomputed unavailable suffix does not price this undefined read.
    assert_eq!(path.probability, ratio(1, 2));

    let engines = Engines::fsl().expect("the linked engines are admitted");
    let records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            fsl_analyze::exact_probabilistic::ProductRequest {
                subject: fsl_eval::model_subject::ModelSubject {
                    simulation: world.request(),
                    proof_bounds: Vec::new(),
                },
                claim,
                limits,
            },
        )],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(
        record.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted),
        "{record:#?}"
    );
    let witness = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the first undefined hop replays");
    assert_eq!(witness.mass(), &ratio(1, 2));
    let undefined = witness
        .undefined()
        .expect("the undefined read is reproduced");
    assert_eq!(undefined.path.paths[0].steps.len(), 1);
    assert_eq!(
        undefined.path.paths[0].steps[0]
            .action
            .as_ref()
            .unwrap()
            .operation,
        AA_DROP_OPERATION
    );
    assert_eq!(undefined.evaluation.site.position, 1);
    assert_eq!(
        undefined.evaluation.site.time.as_ref().unwrap().stamp,
        ratio(1, 1)
    );
}

/// A rational delay selects the region certificate and preserves its exact
/// pre-read time through checked replay.
#[trace(
    "FR-196-AC-13",
    "FR-203-AC-5",
    "FSpec-FR-411-AC-12",
    "FSpec-FR-413-AC-7",
    "FSpec-FR-413-AC-11"
)]
#[test]
fn timed_region_undefined_replays_its_exact_clock_key() {
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
         delay M::Call::drop ~ discrete {{ rational(1, 2) ms: 1 }};\n\
         delay M::Call::keep ~ discrete {{ rational(1, 2) ms: 1 }};\n\
         workload Even on M {{ weight Call::drop = 1; weight Call::keep = 0; }}\n\
         probabilistic RegionUndefined using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms) holds(c.phase = 0) ] }}\n",
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
        .claim("RegionUndefined")
        .expect("the rational-delay claim checks");
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            fsl_analyze::exact_probabilistic::ProductRequest {
                subject: fsl_eval::model_subject::ModelSubject {
                    simulation: world.request(),
                    proof_bounds: Vec::new(),
                },
                claim,
                limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            },
        )],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    let witness = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the region undefined path is replayed");
    assert_eq!(witness.mass(), &ratio(1, 2));
    let undefined = witness.undefined().expect("the region cause is reproduced");
    assert_eq!(undefined.path.paths[0].steps.len(), 1);
    assert_eq!(undefined.path.paths[0].steps[0].elapsed, ratio(1, 2));
    assert_eq!(undefined.evaluation.site.position, 1);
    assert_eq!(
        undefined.evaluation.site.time.as_ref().unwrap().stamp,
        ratio(1, 2)
    );
    let key: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
    assert_eq!(key[4][1][0], "region");
    assert_eq!(key[4][1][4][0][1], "1");
    assert_eq!(
        witness.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"],
        "absent-key"
    );
}
