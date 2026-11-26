// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed checked-subject replay and EN-5 refutation integration.
use crate::prob_world::{self, World};
use crate::source_value_support::source_color;
use filament_exact::{Cancel, CancelCause, Integer, Rational, Undefined, Value};
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::StateKey;
use fsl_eval::temporal::UndefinedCause;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    probabilistic_undefined_wire, probabilistic_wire, replay_probabilistic_witness,
    CounterexampleWireError, ExactProbLimits, ExactProbabilityGraphError, IncompleteCause,
    ModelCheckLimit, ModelCheckLimits, ProbabilisticCounterexample, ProbabilisticValueSchema,
    ProbabilityUndefinedWitness, ProbabilityWitnessError, ProbabilityWitnessReplay, TerminalValue,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use fsl_semantics::check::{Bound, EnumBinding};
use ix_trace_rs::trace;
use serde_json::json;

const CLAIMS: &str = "
probabilistic TooMuch using inf on M over (m: M::Msg) under every scheduler on origin
{ probability <= 0.5 [ eventually holds(m.delivered) ] }
probabilistic TooLittle using inf on M over (m: M::Msg) under every scheduler on origin
{ probability >= 0.97 [ eventually holds(m.delivered) ] }
";

mod certificate_boundary_cases;
fn ratio(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}
fn evidence(world: &World, claim: &str) -> ProbabilisticCounterexample {
    let engines = Engines::fsl().unwrap();
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(claim, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    assert!(record.exact_probabilistic.is_none());
    record
        .probabilistic_counterexample
        .expect("reproduced evidence accompanies refuted")
        .evidence()
        .clone()
}
fn replay(
    world: &World,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    replay_probabilistic_witness(
        system.source_package().expect("a source-route system"),
        &system,
        &witness.item,
        witness,
        limits,
        cancel,
    )
}
fn mass(result: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>) -> Rational {
    let ProbabilityWitnessReplay::Reproduced(token) = result.expect("replay completes") else {
        panic!("must reproduce")
    };
    token.mass().clone()
}
#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn real_link_refutations_replay_target_hits_and_terminal_nonreach() {
    let world = World::new(&prob_world::link(CLAIMS));
    for (claim, expected) in [("TooMuch", ratio(9, 10)), ("TooLittle", ratio(1, 25))] {
        let witness = evidence(&world, claim);
        let limits = ExactProbLimits::default();
        assert_eq!(
            mass(replay(&world, &witness, &limits, &Cancel::new())),
            expected
        );
        assert_eq!(
            mass(replay(&world, &witness, &limits, &Cancel::new())),
            expected
        );
    }
}
#[trace("TC-770", "FR-286-AC-14")]
#[trace("FSpec-TC-431", "FSpec-FR-413-AC-17")]
#[trace("TC-471", "FR-120-AC-41")]
#[test]
fn a_refuted_exact_probabilistic_item_is_spelled_and_its_path_set_replays() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().unwrap();
    let records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("TooMuch", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let (terminals, counterexamples) = fsl_analyze::analyze::split_records(records, &world.package);
    let [item] = counterexamples.as_slice() else {
        panic!("one refuted item");
    };
    let wire = item.wire.as_ref().expect("a path set is spelled");
    assert_eq!(wire["type"], "probabilistic");
    assert_eq!(wire["initial_state"], 0);
    assert!(wire["binding"].is_object(), "the over binding");
    assert_eq!(wire["bound"]["comparison"], "less-or-equal");
    assert_eq!(wire["bound"]["type"], "probability");
    assert_eq!(wire["bound"].as_object().unwrap().len(), 3);
    assert_eq!(wire["bound"]["threshold"]["numerator"], "1");
    assert_eq!(wire["bound"]["threshold"]["denominator"], "2");
    assert_eq!(wire["evidence"]["type"], "path-set");
    let paths = wire["evidence"]["paths"].as_array().expect("paths");
    assert!(!paths.is_empty());
    for step in paths.iter().flat_map(|path| path.as_array().unwrap()) {
        assert_eq!(step["transition"]["type"], "transition");
        assert!(step["post_state"]["key"].is_string());
        assert!(step["random"].is_array());
    }
    let entries = wire["scheduler"]["entries"]
        .as_array()
        .expect("every scheduler");
    // FR-120-AC-41: each scheduled identity writes its operation as the
    // declaration identity text, one string.
    assert!(entries.iter().all(|entry| entry["state"]["key"].is_string()
        && matches!(
            entry["choice"]["operation"].as_str(),
            Some(
                r#"{"node":"ix://test/link/Msg/send_a","package":"test/link"}"#
                    | r#"{"node":"ix://test/link/Msg/send_b","package":"test/link"}"#
            )
        )));
    let document = fsl_analyze::analyze::analyze_document(&terminals, &counterexamples)
        .expect("the refuted item carries its counterexample");
    let written = serde_json::to_value(&document).expect("serializes");
    assert_eq!(written["items"][0]["counterexample"], *wire);
    // Replay reproduces the spelled evidence's path set.
    let fsl_analyze::analyze::Counterexample::Probabilistic(witness) = &item.counterexample else {
        panic!("a probabilistic counterexample");
    };
    // FR-413: one entry per product state key of the witness scheduler, each
    // key once.
    assert_eq!(entries.len(), witness.evidence().scheduler.len());
    let keys: std::collections::BTreeSet<String> = entries
        .iter()
        .map(|entry| entry["state"]["key"].as_str().expect("a key").to_owned())
        .collect();
    assert_eq!(keys.len(), entries.len());
    assert_eq!(
        mass(replay(
            &world,
            witness.evidence(),
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(9, 10)
    );
}

#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn semantic_mutations_cannot_obtain_a_reproduced_token() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let limits = ExactProbLimits::default();
    let mut changed = witness.clone();
    changed.paths[0].steps[0]
        .action
        .as_mut()
        .unwrap()
        .operation
        .push_str("-disabled");
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].draw = Some(vec![Value::Integer(Integer::from(7_i64))]);
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].post_key = witness.initial_key.clone();
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].transition = Some(
        filament_semantic_value::model_transition::ModelTransition::new(
            "\"disabled\"".to_owned(),
            None,
            vec![],
            quire_canonical::Limits::new(1024),
        )
        .expect("canonical syntax with a disabled semantic identity"),
    );
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].post = witness.initial.clone();
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.binding = None;
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.scheduler[0].1 = fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
        fsl_replay::ProductAction::TerminalStutter,
    );
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
}
#[trace("TC-637", "FR-202-AC-11", "FR-202-AC-21")]
#[test]
fn randomized_scheduler_entry_without_a_fairness_set_has_no_token() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let limits = ExactProbLimits::default();
    assert!(replay(&world, &witness, &limits, &Cancel::new()).is_ok());
    let fsl_replay::ProbabilisticSchedulerChoice::Deterministic(action) =
        witness.scheduler[0].1.clone()
    else {
        panic!("the produced scheduler is deterministic");
    };
    for distribution in [
        vec![(action.clone(), ratio(1, 1))],
        vec![
            (action.clone(), ratio(1, 2)),
            (fsl_replay::ProductAction::TerminalStutter, ratio(1, 2)),
        ],
    ] {
        let mut changed = witness.clone();
        changed.scheduler[0].1 = fsl_replay::ProbabilisticSchedulerChoice::Randomized(distribution);
        assert!(matches!(
            replay(&world, &changed, &limits, &Cancel::new()),
            Err(ProbabilityWitnessError::Refused)
        ));
    }
}

/// FR-202-AC-11 (TC-637): this path-set replay profile refuses more than one
/// initial state or more than one claim binding without producing a token.
#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn multiple_initials_and_instances_are_unsupported_without_a_token() {
    let model = prob_world::link(CLAIMS);
    let world = World::new(&model);
    let witness = evidence(&world, "TooMuch");

    let mut delivered = model.initial.clone();
    delivered["delivered"] = serde_json::json!({"boolean": true});
    let multiple_initials = World::with_initials(&model, &[model.initial.clone(), delivered]);
    assert!(matches!(
        replay(
            &multiple_initials,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::UnsupportedInitial
        ))
    ));
    let multiple_instances = World::with_objects(&model, &["m", "n"]);
    assert!(matches!(
        replay(
            &multiple_instances,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::UnsupportedBinding
        ))
    ));
}

#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn duplicate_paths_live_prefixes_and_threshold_equality_have_no_token() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let mut changed = witness.clone();
    changed.paths[0]
        .steps
        .push(witness.paths[0].steps[0].clone());
    assert!(
        matches!(
            replay(
                &world,
                &changed,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Ok(ProbabilityWitnessReplay::Parity)
        ),
        "a step after the first target cannot extend an accepted path"
    );
    let mut changed = witness.clone();
    changed.paths.push(changed.paths[0].clone());
    assert!(matches!(
        replay(
            &world,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let nonreach = evidence(&world, "TooLittle");
    let mut changed = nonreach.clone();
    changed.paths[0].steps.pop();
    assert!(matches!(
        replay(
            &world,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let equal_world = World::new(&prob_world::link(&CLAIMS.replace("<= 0.5", "<= 0.9")));
    let mut equal = witness;
    // Use the equal-threshold claim's exact checked identity and scheduler binding.
    equal.item = fsl_package::ObligationIdentity::new(
        equal_world
            .claim("TooMuch")
            .occurrence()
            .unwrap()
            .node()
            .clone(),
        None,
        0,
    );
    assert!(matches!(
        replay(
            &equal_world,
            &equal,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
}
#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn witness_limits_and_final_acceptance_cancellation_never_yield_tokens() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let limited = ExactProbLimits {
        max_witness_paths: 0,
        ..ExactProbLimits::default()
    };
    assert!(matches!(
        replay(&world, &witness, &limited, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0
        })
    ));
    let live = Cancel::new();
    let _count = live.count_charges();
    assert_eq!(
        mass(replay(&world, &witness, &ExactProbLimits::default(), &live)),
        ratio(9, 10)
    );
    let final_poll = live.charges();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let count = Arc::new(AtomicU32::new(0));
        let observed = Arc::clone(&count);
        let cancel = Cancel::observing(move || {
            if observed.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert!(
            matches!(replay(&world,&witness,&ExactProbLimits::default(),&cancel), Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(found))) if found == cause)
        );
        assert_eq!(count.load(Ordering::Relaxed), final_poll);
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let engines = Engines::fsl().unwrap();
        let record = analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("TooMuch", ModelCheckLimits::default()),
            )],
            &cancel,
        )
        .remove(0);
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause })
        );
        assert!(record.probabilistic_counterexample.is_none());
    }
}

#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn workload_replay_preserves_weight_shares_and_emits_no_scheduler() {
    let world = World::new(&prob_world::link(
        &CLAIMS
            .replace("under every scheduler", "under Even")
            .replace(">= 0.97", ">= 0.99"),
    ));
    use fsl_analyze::exact_probabilistic::{
        build_product, decide_reachability, produce_witness, ProbBuild, UnboundedObjective,
        WitnessScope,
    };
    let request = world.request("TooMuch", ModelCheckLimits::default());
    let limits = request.limits;
    let ProbBuild::Built(product) = build_product(request, &Cancel::new()).unwrap() else {
        panic!("defined product");
    };
    let target = product.labels.iter().map(|row| row[0]).collect::<Vec<_>>();
    let solved = decide_reachability(
        &product,
        &target,
        UnboundedObjective::Min,
        &limits,
        &Cancel::new(),
    )
    .expect("workload solver");
    assert_eq!(
        solved.values[product.initial[0].2.index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(391, 400))
    );
    assert!(produce_witness(
        &product,
        &solved.policy,
        WitnessScope::Workload,
        product.initial[0].2,
        &target,
        &ratio(1, 2),
        &limits
    )
    .expect("workload witness producer")
    .is_some());
    let witness = evidence(&world, "TooMuch");
    assert!(witness.scheduler.is_empty());
    assert_eq!(
        mass(replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(17, 20)
    );
    let nonreach = evidence(&world, "TooLittle");
    assert!(nonreach.scheduler.is_empty());
    assert_eq!(
        mass(replay(
            &world,
            &nonreach,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(3, 200)
    );
}

#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn claim_and_package_mismatches_are_refused() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let other = fsl_package::ObligationIdentity::new(
        world
            .claim("TooLittle")
            .occurrence()
            .unwrap()
            .node()
            .clone(),
        None,
        0,
    );
    assert!(matches!(
        replay_probabilistic_witness(
            system.source_package().expect("a source-route system"),
            &system,
            &other,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let other_world = World::new(&prob_world::link(&CLAIMS.replace("<= 0.5", "<= 0.4")));
    let other_system = ModelSystem::new(other_world.subject().simulation).unwrap();
    assert!(matches!(
        replay_probabilistic_witness(
            other_system
                .source_package()
                .expect("a source-route system"),
            &system,
            &witness.item,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let ProbabilityWitnessReplay::Reproduced(token) = replay(
        &world,
        &witness,
        &ExactProbLimits::default(),
        &Cancel::new(),
    )
    .unwrap() else {
        panic!("positive control");
    };
    assert!(fsl_replay::TerminalRecord::refuted_by_probability_witness(
        RequestIndex::new(0),
        fsl_foundation::BackendId::new("en-5"),
        &other,
        &token
    )
    .is_none());
}

#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn two_thirds_first_hits_and_nonreach_leaves_cross_one_half() {
    let mut model = prob_world::link(&CLAIMS.replace(">= 0.97", ">= 0.5"));
    model.body = model
        .body
        .replace("< 2", "< 1")
        .replace("attempts = 2", "attempts = 1")
        .replace("true: 1, false: 9", "true: 1, false: 2")
        .replace("true: 1, false: 4", "true: 2, false: 1");
    let world = World::new(&model);
    for claim in ["TooMuch", "TooLittle"] {
        let witness = evidence(&world, claim);
        assert_eq!(
            mass(replay(
                &world,
                &witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            )),
            ratio(2, 3)
        );
    }
}

#[trace("TC-637", "FR-202-AC-10", "FR-203-AC-3")]
#[test]
fn replay_parity_and_refusal_settle_inconclusive_without_evidence() {
    use fsl_analyze::analyze::exact_probabilistic_item_witness_tampered;
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().unwrap();
    for duplicate in [true, false] {
        let item = exact_probabilistic_item_witness_tampered(
            RequestIndex::new(0),
            world.request("TooMuch", ModelCheckLimits::default()),
            move |witness| {
                if duplicate {
                    witness.paths.push(witness.paths[0].clone());
                } else {
                    witness.paths[0].steps[0]
                        .action
                        .as_mut()
                        .unwrap()
                        .operation
                        .push_str("-disabled");
                }
            },
        );
        let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
        if duplicate {
            assert!(matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::ReplayParity(_))
            ));
        } else {
            assert!(matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::ReplayRefused(
                    fsl_foundation::diagnostic::Code::InvalidRuntimeInput
                ))
            ));
        }
        assert!(record.probabilistic_counterexample.is_none());
        assert!(record.exact_probabilistic.is_none());
        let outcome = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(
            record.terminal().unwrap(),
        ))
        .unwrap();
        assert!(outcome.get("measured").is_none(), "{outcome}");
    }
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn graph_arithmetic_limits_and_cyclic_models_cannot_reproduce() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let mut limits = ExactProbLimits::default();
    limits.model.limits = fsl_eval::simulation::Limits::default().with_max_states(0);
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::LimitReached
        ))
    ));
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_transitions = 0;
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::EdgeLimitReached
        ))
    ));
    let limits = ExactProbLimits {
        max_rational_bits: 3,
        ..ExactProbLimits::default()
    };
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: 3
        })
    ));
    let mut model = prob_world::link(CLAIMS);
    model.body = model.body.replace(
        "self.attempts - pre(self.attempts) = 1",
        "self.attempts - pre(self.attempts) = 0",
    );
    let cyclic = World::new(&model);
    let mut changed = witness;
    changed.item = fsl_package::ObligationIdentity::new(
        cyclic.claim("TooMuch").occurrence().unwrap().node().clone(),
        None,
        0,
    );
    assert!(matches!(
        replay(
            &cyclic,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::Cycle
        ))
    ));
    model.initial["delivered"] = serde_json::json!({"boolean": true});
    model.body = model
        .body
        .replace("not self.delivered and ", "")
        .replace("self.delivered or ", "");
    let target_cycle = World::new(&model);
    changed.item = fsl_package::ObligationIdentity::new(
        target_cycle
            .claim("TooMuch")
            .occurrence()
            .unwrap()
            .node()
            .clone(),
        None,
        0,
    );
    assert!(
        matches!(
            replay(
                &target_cycle,
                &changed,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Err(ProbabilityWitnessError::Graph(
                ExactProbabilityGraphError::Cycle
            ))
        ),
        "an initial target cannot conceal its reachable continuation cycle"
    );
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn cancellation_at_the_en5_replay_acceptance_boundary_has_no_evidence() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().unwrap();
    let run = |cancel: &Cancel| {
        analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("TooMuch", ModelCheckLimits::default()),
            )],
            cancel,
        )
        .remove(0)
    };
    let live = Cancel::new();
    let _count = live.count_charges();
    assert_eq!(
        run(&live).terminal().unwrap().value(),
        &TerminalValue::Refuted
    );
    let final_poll = live.charges();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let seen = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&seen);
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        let record = run(&cancel);
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause })
        );
        assert!(record.probabilistic_counterexample.is_none());
        assert!(record.exact_probabilistic.is_none());
        assert_eq!(seen.load(Ordering::Relaxed), final_poll);
    }
}

#[trace("TC-637", "FR-202-AC-9", "FR-202-AC-11")]
#[test]
fn merged_draws_expand_into_actual_vectors_with_a_witness_storage_limit() {
    let mut model = prob_world::link(CLAIMS);
    model.body = model
        .body
        .replace("self.delivered = (not lost)", "self.delivered = true");
    let world = World::new(&model);
    let witness = evidence(&world, "TooMuch");
    assert_eq!(witness.paths.len(), 2);
    assert_eq!(
        mass(replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(1, 1)
    );
    assert!(witness.paths[0].steps[0]
        .draw
        .as_deref()
        .zip(witness.paths[1].steps[0].draw.as_deref())
        .is_some_and(|(left, right)| !fsl_eval::simulation::model::same_values(left, right)));
    let engines = Engines::fsl().unwrap();
    let mut request = world.request("TooMuch", ModelCheckLimits::default());
    request.limits.max_witness_paths = 1;
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 1,
            required: None,
        })
    );
    assert!(record.probabilistic_counterexample.is_none());
}

#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn fix_round_poisoned_model_and_ordinal_in_both_copies_are_refused() {
    let world = World::new(&prob_world::link(CLAIMS));
    let original = evidence(&world, "TooMuch");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    for (model, ordinal) in [
        (Some(fsl_package::ModelIdentity::new("wrong-model")), 0),
        (None, 123),
        (Some(fsl_package::ModelIdentity::new("wrong-model")), 123),
    ] {
        let mut witness = original.clone();
        let poisoned =
            fsl_package::ObligationIdentity::new(original.item.node().clone(), model, ordinal);
        witness.item = poisoned.clone();
        assert!(
            matches!(
                replay_probabilistic_witness(
                    system.source_package().expect("a source-route system"),
                    &system,
                    &poisoned,
                    &witness,
                    &ExactProbLimits::default(),
                    &Cancel::new()
                ),
                Err(ProbabilityWitnessError::Refused)
            ),
            "two agreeing caller-owned identities cannot authorize a different checked obligation"
        );
    }
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn fix_round_producer_path_exhaustion_settles_incomplete_without_evidence() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().unwrap();
    let mut request = world.request("TooMuch", ModelCheckLimits::default());
    request.limits.max_witness_paths = 0;
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(
        record
            .terminal()
            .expect("producer exhaustion must settle an incomplete terminal record")
            .value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
            required: None,
        })
    );
    assert!(record.probabilistic_counterexample.is_none());
    assert!(record.exact_probabilistic.is_none());
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn fix_round_fanout_checks_state_storage_before_queuing_successors() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    for maximum in [0, 1] {
        let mut limits = ExactProbLimits::default();
        limits.model.limits.max_states = maximum;
        limits.model.limits.max_transitions = maximum;
        assert!(matches!(replay(&world,&witness,&limits,&Cancel::new()),Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::LimitReached))),"state admission must stop before another fanout edge can exhaust the transition budget");
    }
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn fix_round_producer_frontier_and_rational_stops_follow_a_successful_real_solve() {
    use fsl_analyze::exact_probabilistic::{
        build_product, decide_reachability, produce_witness, ProbBuild, UnboundedObjective,
        WitnessError, WitnessScope,
    };
    let world = World::new(&prob_world::link(CLAIMS));
    let request = world.request("TooMuch", ModelCheckLimits::default());
    let ample = request.limits;
    let ProbBuild::Built(product) =
        build_product(request, &Cancel::new()).expect("the real model builds under ample limits")
    else {
        panic!("defined product");
    };
    let target = product.labels.iter().map(|row| row[0]).collect::<Vec<_>>();
    let solved = decide_reachability(
        &product,
        &target,
        UnboundedObjective::Max,
        &ample,
        &Cancel::new(),
    )
    .expect("the real product solves before producer-only limits apply");
    let start = product.initial[0].2;
    let produce = |limits: &ExactProbLimits| {
        produce_witness(
            &product,
            &solved.policy,
            WitnessScope::EveryScheduler,
            start,
            &target,
            &ratio(1, 2),
            limits,
        )
    };
    assert!(
        matches!(produce(&ample), Ok(Some(_))),
        "positive control: the same policy has sufficient finite evidence"
    );
    let mut path = ample;
    path.max_witness_paths = 0;
    let mut frontier = ample;
    frontier.model.limits.max_transitions = 2;
    let mut arithmetic = ample;
    arithmetic.max_rational_bits = 3;
    for (limits, expected, value) in [
        (path, ModelCheckLimit::ExactProbMaxWitnessPaths, 0),
        (frontier, ModelCheckLimit::ExactProbMaxTransitions, 2),
        (arithmetic, ModelCheckLimit::ExactProbMaxRationalBits, 3),
    ] {
        match produce(&limits) {
            Err(WitnessError::Limit {
                limit,
                value: actual,
            }) => {
                assert_eq!(limit, expected);
                assert_eq!(actual, value);
            }
            other => panic!(
                "a successful build/solve must expose the actual producer stop, got {other:?}"
            ),
        }
    }
}

const FAIR_CLAIMS: &str = "
probabilistic TooMuchFair using inf on M over (m: M::Msg) under every scheduler fair weak Msg::send_a on origin
{ probability <= 0.5 [ eventually holds(m.delivered) ] }
probabilistic TooLittleFair using inf on M over (m: M::Msg) under every scheduler fair weak Msg::send_a on origin
{ probability >= 0.99 [ eventually holds(m.delivered) ] }
";

/// FR-200 / FR-413-AC-5: the fair refutation of both bound directions carries
/// evidence whose fairness replay accepts.
#[trace("TC-364", "FR-200", "FR-200-AC-4", "FR-413-AC-5")]
#[trace("FSpec-TC-364", "FSpec-FR-413-AC-5")]
#[test]
fn a_fair_every_scheduler_refutation_replays_through_the_fairness_check() {
    let world = World::new(&prob_world::link(FAIR_CLAIMS));
    for claim in ["TooMuchFair", "TooLittleFair"] {
        let witness = evidence(&world, claim);
        let limits = ExactProbLimits::default();
        assert!(
            matches!(
                replay(&world, &witness, &limits, &Cancel::new()),
                Ok(ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{claim}"
        );
    }
}

/// FR-413-AC-5: a witness the fairness walk cannot cover (no entry at a
/// reached state) is not a fair witness and settles `inconclusive`,
/// `replay-parity`, not a refusal.
#[trace("TC-364", "FR-413-AC-5", "FR-202-AC-3")]
#[trace("FSpec-TC-364", "FSpec-FR-413-AC-5")]
#[test]
fn an_unfair_witness_settles_inconclusive_replay_parity() {
    use fsl_analyze::analyze::exact_probabilistic_item_witness_tampered;
    let world = World::new(&prob_world::link(FAIR_CLAIMS));
    let witness = evidence(&world, "TooMuchFair");
    let limits = ExactProbLimits::default();
    let mut changed = witness.clone();
    changed.scheduler.remove(0);
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let engines = Engines::fsl().unwrap();
    let item = exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        world.request("TooMuchFair", ModelCheckLimits::default()),
        |witness| {
            witness.scheduler.remove(0);
        },
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    assert!(matches!(
        record.terminal().unwrap().value(),
        TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::ReplayParity(_))
    ));
    assert!(record.probabilistic_counterexample.is_none());
}

/// A probabilistic counterexample whose draw and scheduled argument are source
/// `enum` values writes both in FR-181's typed enum form, as a path set and as
/// an undefined path. This proves the spelling of `probabilistic_wire` and
/// `probabilistic_undefined_wire` only: the values are substituted into the
/// witness of a claim over a domain package's object, and no production path is
/// known to carry a source `enum` through that claim. The analyzer itself writes
/// a source `enum` for a source object's operation, in the tests below.
#[test]
fn source_enum_draws_and_arguments_are_spelled_in_probabilistic_counterexamples() {
    let world = World::new(&prob_world::link(&format!(
        "enum Color {{ red, blue }}\n{CLAIMS}"
    )));
    let mut witness = evidence(&world, "TooMuch");
    witness.paths.truncate(1);
    let step = witness
        .paths
        .first_mut()
        .and_then(|path| path.steps.first_mut())
        .expect("a witness step");
    step.draw = Some(vec![source_color(&world.package, "red")]);
    let mut identity = step.action.clone().expect("an identity step");
    identity.arguments = vec![source_color(&world.package, "blue")];
    let at = StateKey::from_canonical_bytes(b"at".to_vec());
    let scheduler = [(at.clone(), identity)];
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let state_key = |state: &ModelSimState| {
        system
            .state_key(state, fsl_foundation::IdentityLimits::default())
            .ok()
            .map(|(_, key)| key)
    };
    let bound = Bound::AtMost(ratio(1, 2));
    let scope = world.package.graph().scope();
    let color = |member: &str| {
        json!({"type": "enum", "name": r#"{"node":"Color","package":"package"}"#,
            "member": member})
    };
    let unspelled = Err(CounterexampleWireError::Unspelled("probabilistic value"));

    let path_set = |enums: &[EnumBinding]| {
        probabilistic_wire(
            &witness,
            &bound,
            "probability",
            &ProbabilisticValueSchema::new(scope.types(), scope.variant_types(), enums),
            &state_key,
            &scheduler,
        )
    };
    let written = path_set(scope.enums()).expect("the source enum values are spelled");
    assert_eq!(
        written["evidence"]["paths"][0][0]["random"],
        json!([color("red")])
    );
    assert_eq!(
        written["scheduler"]["entries"][0]["choice"]["arguments"],
        json!([color("blue")])
    );
    assert_eq!(path_set(&[]), unspelled);

    let undefined = ProbabilityUndefinedWitness {
        path: witness.clone(),
        evaluation: UndefinedEvaluation {
            site: UndefinedSite {
                position: 0,
                state_key: at,
                locus: UndefinedLocus::FractionValue,
                transition: None,
                time: None,
            },
            cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
        },
        product_state: witness.initial_key.clone(),
    };
    let claim = world
        .claim("TooMuch")
        .occurrence()
        .expect("the claim has an occurrence");
    let undefined_path = |enums: &[EnumBinding]| {
        probabilistic_undefined_wire(
            &undefined,
            claim,
            &bound,
            "probability",
            &ProbabilisticValueSchema::new(scope.types(), scope.variant_types(), enums),
            &state_key,
            &scheduler,
        )
    };
    let written = undefined_path(scope.enums()).expect("the source enum values are spelled");
    assert_eq!(
        written["evidence"]["path"][0]["random"],
        json!([color("red")])
    );
    assert_eq!(
        written["scheduler"]["entries"][0]["choice"]["arguments"],
        json!([color("blue")])
    );
    assert_eq!(undefined_path(&[]), unspelled);
}

/// The counterexample the analyzer writes for the claim `Seen` over a coin no
/// step changes, beside an object of the unit's own type `Lamp` whose one
/// operation is scheduled with the source `enum` member `Color::red` and draws
/// `Color::blue`. `profile` is the claim's temporal profile and `measure` its
/// bounded measure.
fn source_object_wire(profile: &str, measure: &str) -> serde_json::Value {
    let mut world = World::new(&prob_world::coin(&format!(
        "enum Color {{ red, blue }}\n\
         object Lamp {{ lit: Boolean; population lamps; \
         operation paint(c: Color, tint: Color) modifies self.lit; }}\n\
         pre Dark using v on Lamp::paint {{ not self.lit and tint = Color::red }}\n\
         post Lit using v on Lamp::paint {{ self.lit }}\n\
         pre Never using v on M::Coin::drop {{ false }}\n\
         random Lamp::paint(c) ~ {{ Color::blue: 1 }};\n\
         probabilistic Seen using {profile} on M over (c: M::Coin) \
         under every scheduler on origin\n{{ {measure} }}\n"
    )));
    world.add_source_object("lamps", "Lamp", "l", &json!({"lit": {"boolean": false}}));
    let record = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Seen", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    record
        .probabilistic_counterexample
        .as_ref()
        .unwrap_or_else(|| panic!("a counterexample: {:?}", record.settlement))
        .wire
        .clone()
        .expect("the source enum values are spelled")
}

/// The one-member value list holding the source `enum` member `member`.
fn color_list(member: &str) -> serde_json::Value {
    json!([{"type": "enum", "name": r#"{"node":"Color","package":"package"}"#,
        "member": member}])
}

/// A refuted reachability claim writes a source object operation's source
/// `enum` draw in FR-181's typed enum form.
#[test]
fn a_source_enum_draw_is_spelled_in_a_reachability_counterexample() {
    let wire = source_object_wire("inf", "probability >= 0.5 [ eventually holds(c.v = 1) ]");
    assert_eq!(
        wire["evidence"]["paths"][0][0]["random"],
        color_list("blue")
    );
}

/// A refuted mean claim writes a source object operation's source `enum`
/// scheduled argument in FR-181's typed enum form. Its evidence is a subsystem
/// certificate with no path, so it writes no draw.
#[test]
fn a_source_enum_argument_is_spelled_in_a_mean_counterexample() {
    let wire = source_object_wire("ev", "mean of (fraction holds(c.v = 1) over [0, 1]) >= 0.5");
    assert_eq!(
        wire["scheduler"]["entries"][0]["choice"]["arguments"],
        color_list("red")
    );
}

/// A claim refuted by an undefined read writes the source `enum` draw on its
/// path in FR-181's typed enum form. `paint` takes a coin by reference and its
/// postcondition removes every coin, so the claim's read of `c.v` is undefined
/// after that one step.
#[test]
fn a_source_enum_draw_is_spelled_in_an_undefined_counterexample() {
    let mut world = World::new(&prob_world::coin(
        "enum Color { red, blue }\n\
         object Lamp { lit: Boolean; population lamps; \
         operation paint(c: Color, coin: Reference<M::Coin>) modifies self.lit deletes coin; }\n\
         pre Dark using v on Lamp::paint { not self.lit }\n\
         post Lit using v on Lamp::paint \
         { self.lit and count(o in M::coins where true) = 0 }\n\
         pre Never using v on M::Coin::drop { false }\n\
         random Lamp::paint(c) ~ { Color::blue: 1 };\n\
         probabilistic Seen using ev on M over (c: M::Coin) under every scheduler on origin\n\
         { probability >= 0.5 [ always[0,1] holds(c.v = 0) ] }\n",
    ));
    world.add_source_object("lamps", "Lamp", "l", &json!({"lit": {"boolean": false}}));
    let record = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Seen", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    let wire = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap_or_else(|| panic!("a counterexample: {:?}", record.settlement))
        .wire
        .clone()
        .expect("the source enum values are spelled");
    assert_eq!(wire["evidence"]["type"], "undefined-path");
    assert_eq!(wire["evidence"]["path"][0]["random"], color_list("blue"));
}
