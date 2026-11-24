// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2850: the exact engine's limit, cancel and fault exits settle as their
//! own outcome, never as `unsupported`/`not-yet-implemented`.
use super::*;
use crate::long_run_certificate::availability_with_claims;
use fsl_analyze::analyze::terminal_records;

const NOT_YET: &str = "unsupported_construct/not-yet-implemented";

fn long_run(name: &str, predicate: &str, bound: &str) -> String {
    format!(
        "probabilistic {name} using inf on M over (s: M::Server) under Steady on origin \
         {{ long_run_fraction holds({predicate}) {bound} }}\n"
    )
}

fn diagnostic_of(record: fsl_analyze::analyze::AnalyzeRecord) -> (TerminalValue, Option<String>) {
    let terminal = terminal_records(vec![record]).remove(0);
    (
        terminal.value().clone(),
        terminal.diagnostic().map(|code| code.to_string()),
    )
}

/// A positive-probability undefined product predicate refutes a long-run
/// claim only after its evidence replays against the checked subject.
#[trace("FSpec-FR-411-AC-12", "FSpec-FR-413-AC-7")]
#[test]
fn a_long_run_claim_with_an_undefined_predicate_replays_its_path() {
    let claim = "probabilistic Dropped using inf on M over (c: M::Coin) under Even on origin \
                 { long_run_fraction holds(c.v = 0) >= 0.5 }";
    let world = World::new(&prob_world::coin(claim));
    let record = settle(&world, "Dropped", ModelCheckLimits::default());
    assert_replayed_undefined(record, &world, "Dropped");
}

/// An expected-reward claim's positive-probability undefined target predicate
/// also produces checked-subject undefined evidence.
#[trace("FSpec-FR-411-AC-12", "FSpec-FR-413-AC-7")]
#[test]
fn an_expected_reward_claim_with_an_undefined_predicate_replays_its_path() {
    let claim =
        "probabilistic RewardUndefined using inf on M over (c: M::Coin) under Even on origin \
                { expected accumulate cost until holds(c.v = 1) <= 1 }";
    let model = prob_world::coin(&format!("reward cost on Coin::drop = 1;\n{claim}"));
    let world = World::new(&model);
    let product = fsl_analyze::exact_probabilistic::build_product(
        world.request("RewardUndefined", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the product builder accepts the expected-reward claim");
    assert!(
        matches!(
            &product,
            fsl_analyze::exact_probabilistic::ProbBuild::Undefined(_)
        ),
        "the coin's delete draw must reach the undefined predicate path: {product:?}"
    );
    let record = settle(&world, "RewardUndefined", ModelCheckLimits::default());
    assert_replayed_undefined(record, &world, "RewardUndefined");
}

/// FSpec FR-411-AC-12 and FR-413-AC-7: replay rejects a path that reaches an
/// expected-reward `until` target at position 1, then continues to an
/// undefined read at position 2.
#[trace("FSpec-FR-411-AC-12", "FSpec-FR-413-AC-7")]
#[test]
fn expected_reward_undefined_replay_rejects_a_path_after_until_decides() {
    use fsl_replay::{replay_probabilistic_undefined, ProbabilityWitnessReplay};

    let claims = "probabilistic Open using inf on M over (c: M::Coin) under Even on origin \
        { long_run_fraction holds(c.armed) >= 0.5 }\n\
        probabilistic Decided using inf on M over (c: M::Coin) under Even on origin \
        { expected accumulate cost until holds(c.armed) <= 1 }";
    let world = World::new(&prob_world::armed_coin(&format!(
        "reward cost on Coin::arm = 1;\nreward cost on Coin::drop = 1;\n{claims}"
    )));
    let mut witness = undefined_evidence(&world, "Open");
    assert_eq!(witness.evaluation.site.position, 2);
    assert_eq!(witness.path.paths[0].steps.len(), 2);
    witness.path.item = fsl_replay::probabilistic_claim_identity(world.claim("Decided")).unwrap();
    witness.evaluation.site.locus =
        fsl_replay::UndefinedLocus::ClaimPredicate(fsl_replay::ClaimPredicate::Until);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = replay_probabilistic_undefined(
        &world.package,
        &system,
        &witness.path.item,
        &witness,
        &world.request("Decided", ModelCheckLimits::default()).limits,
        &Cancel::new(),
    );
    assert!(matches!(replay, Ok(ProbabilityWitnessReplay::Parity)));
}

/// Long-run and expected-reward replay reject forged positions, repeated
/// product keys, exhausted path limits and cancellation.
#[trace("FSpec-FR-411-AC-12", "FSpec-FR-413-AC-7")]
#[test]
fn long_run_and_expected_reward_undefined_replay_reject_adverse_evidence() {
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_replay::{
        replay_probabilistic_undefined, ExactProbabilityGraphError, ProbabilisticSchedulerChoice,
        ProbabilityWitnessError, ProbabilityWitnessReplay, ProductAction, Schedule,
    };

    let claims =
        "probabilistic LongRun using inf on M over (c: M::Coin) under every scheduler on origin \
        { long_run_fraction holds(c.v = 1) >= 0.5 }\n\
        probabilistic Reward using inf on M over (c: M::Coin) under every scheduler on origin \
        { expected accumulate cost until holds(c.v = 1) <= 1 }";
    let mut model = prob_world::coin(&format!("reward cost on Coin::drop = 1;\n{claims}"));
    model.operations.push(prob_world::Operation {
        name: "keep",
        parameters: vec![],
        modifies: vec![],
        deletes: false,
    });
    model.body = model.body.replace(
        "weight Coin::drop = 1;",
        "weight Coin::drop = 1; weight Coin::keep = 0;",
    );
    let world = World::new(&model);
    let system = ModelSystem::new(world.subject().simulation).unwrap();

    for claim in ["LongRun", "Reward"] {
        let witness = undefined_evidence(&world, claim);
        let limits = world.request(claim, ModelCheckLimits::default()).limits;
        let replay = |witness: &fsl_replay::ProbabilityUndefinedWitness,
                      limits: &fsl_analyze::exact_probabilistic::ExactProbLimits,
                      cancel: &Cancel| {
            replay_probabilistic_undefined(
                &world.package,
                &system,
                &witness.path.item,
                witness,
                limits,
                cancel,
            )
        };

        let mut wrong_position = witness.clone();
        wrong_position.evaluation.site.position += 1;
        assert!(matches!(
            replay(&wrong_position, &limits, &Cancel::new()),
            Ok(ProbabilityWitnessReplay::Parity)
        ));

        let initial = system.initial().remove(0);
        let keep = system
            .actions_with_cancel(&initial, &Cancel::new())
            .unwrap()
            .unwrap()
            .into_iter()
            .find(|action| action.identity.operation.contains("keep"))
            .expect("the self-loop operation is enabled");
        let draw = keep
            .draws
            .into_iter()
            .next()
            .expect("one deterministic draw");
        let (transition, post) = draw
            .successors
            .into_iter()
            .next()
            .expect("the self-loop has one successor");
        let mut repeated = witness.clone();
        let first = fsl_replay::ProbabilityWitnessStep {
            choice: ProductAction::Identity(Schedule::of(&system, &keep.identity).unwrap()),
            action: Some(keep.identity),
            transition: Some(transition),
            draw: Some(draw.random),
            post,
            post_key: repeated.path.initial_key.clone(),
            elapsed: ratio(0, 1),
        };
        repeated.path.paths[0].steps.insert(0, first);
        repeated.evaluation.site.position += 1;
        assert_eq!(repeated.evaluation.site.position, 2);
        assert_eq!(repeated.path.paths[0].steps.len(), 2);
        repeated.path.scheduler[0].1 = ProbabilisticSchedulerChoice::Deterministic(
            repeated.path.paths[0].steps[0].choice.clone(),
        );
        assert!(matches!(
            replay(&repeated, &limits, &Cancel::new()),
            Ok(ProbabilityWitnessReplay::Parity)
        ));

        let mut no_paths = limits;
        no_paths.max_witness_paths = 0;
        assert!(matches!(
            replay(&witness, &no_paths, &Cancel::new()),
            Err(ProbabilityWitnessError::Limit {
                limit: fsl_replay::ModelCheckLimit::ExactProbMaxWitnessPaths,
                value: 0
            })
        ));
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        assert!(matches!(
            replay(&witness, &limits, &cancel),
            Err(ProbabilityWitnessError::Graph(
                ExactProbabilityGraphError::Cancelled(CancelCause::Requested)
            ))
        ));
    }
}

fn assert_replayed_undefined(
    record: fsl_analyze::analyze::AnalyzeRecord,
    world: &World,
    claim: &str,
) {
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    let token = record
        .probabilistic_counterexample
        .as_ref()
        .expect("replayed undefined path");
    let undefined = token.undefined().expect("undefined evidence");
    assert_eq!(undefined.evaluation.site.position, 1);
    assert_eq!(undefined.path.paths[0].steps.len(), 1);
    assert_eq!(token.mass(), &ratio(1, 2));
    assert_eq!(
        token.wire.as_ref().unwrap()["evidence"]["undefined"]["cause"],
        "absent-key"
    );
    let expected_bound = match claim {
        "Dropped" => serde_json::json!({
            "type": "long-run-fraction",
            "comparison": "greater-or-equal",
            "threshold": {"numerator": "1", "denominator": "2"}
        }),
        "RewardUndefined" => serde_json::json!({
            "type": "expected-total",
            "comparison": "less-or-equal",
            "threshold": {"numerator": "1", "denominator": "1"}
        }),
        _ => panic!("unexpected undefined claim {claim}"),
    };
    assert_eq!(token.wire.as_ref().unwrap()["bound"], expected_bound);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = fsl_replay::replay_probabilistic_undefined(
        &world.package,
        &system,
        &undefined.path.item,
        undefined,
        &world.request(claim, ModelCheckLimits::default()).limits,
        &Cancel::new(),
    );
    assert!(matches!(
        replay,
        Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
    ));
}

/// A cancel at any poll of a long-run run settles incomplete or with its
/// verdict, never as an unbuilt form or a fault.
#[trace("TC-763", "FR-281-AC-11")]
#[test]
fn cancelling_a_long_run_claim_at_any_poll_is_never_unbuilt() {
    let world = availability_with_claims(
        long_run("Lower", "s.phase = 0", ">= rational(1800,1801)"),
        "",
    );
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let run = |cancel: &Cancel| {
        let item = exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Lower", ModelCheckLimits::default()),
        );
        diagnostic_of(analyze_items(&engines, vec![item], cancel).remove(0))
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    let (control, _) = run(&live);
    assert!(
        matches!(control, TerminalValue::Proved { .. }),
        "{control:?}"
    );
    let total = live.charges();
    let stride = (total / 40).max(1);
    let mut stopped = 0_u32;
    for at in (1..=total).step_by(stride as usize) {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == at {
                weak.upgrade()
                    .expect("the run owns the slot")
                    .get()
                    .expect("the slot is initialized")
                    .cancel(CancelCause::Requested);
            }
        });
        slot.set(cancel.clone()).expect("initialized once");
        let (value, diagnostic) = run(&cancel);
        assert_ne!(diagnostic.as_deref(), Some(NOT_YET), "poll {at}: {value:?}");
        assert!(
            matches!(
                value,
                TerminalValue::Proved { .. } | TerminalValue::Incomplete(_)
            ),
            "poll {at} of {total}: {value:?} {diagnostic:?}"
        );
        stopped += u32::from(matches!(value, TerminalValue::Incomplete(_)));
    }
    assert!(stopped > 0, "some poll must land inside the run");
}
