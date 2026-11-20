// SPDX-License-Identifier: AGPL-3.0-or-later
//! Returned state-graph evidence admitted and replayed through the public facade.

use super::*;
use fsl_replay::{
    GraphEvidence, GraphReplaySettlement, ModelTraceRefusal, ReplayLimits, ReplayRefusal,
    WitnessPath,
};

fn returned(starts: &[Vec<i64>], sampled: bool) -> (World, ModelInstance, Vec<GraphWitness>) {
    let world = config_world_over("", REACHES_TWO, &["a"], starts);
    let binding = ModelInstance {
        population: world.subject(&["a"], UNLIMITED).simulation.universes[0]
            .population
            .clone(),
        key: "a".to_owned(),
    };
    let limits = if sampled {
        ModelCheckLimits::default()
    } else {
        no_samples()
    };
    let result = run(world.request(
        &["a"],
        vec![StateGraphItem::for_binding(
            world.claim("ReachesTwo"),
            binding.clone(),
        )],
        limits,
    ));
    let [instance] = result.items[0].as_slice() else {
        panic!("one bound item")
    };
    assert_eq!(instance.over.as_ref(), Some(&world.reference("a")));
    let paths = witnesses(&instance.outcome).to_vec();
    (world, binding, paths)
}

fn evidence(paths: &[GraphWitness], over: &ModelInstance) -> GraphEvidence {
    GraphEvidence::Witness {
        paths: paths
            .iter()
            .map(|witness| {
                (
                    WitnessPath::Path(witness.path.clone()),
                    witness.source.clone(),
                )
            })
            .collect(),
        over: Some(over.clone()),
    }
}

#[trace("FR-170-AC-1", "FR-170-AC-4")]
#[test]
fn sampled_and_explored_returned_paths_replay_their_evaluated_targets() {
    for sampled in [false, true] {
        let (world, over, paths) = returned(&[vec![0], vec![1]], sampled);
        assert_eq!(paths.len(), 2);
        for (index, witness) in paths.iter().enumerate() {
            assert_eq!(
                usize::try_from(witness.path.initial).expect("a small index"),
                index
            );
            assert_eq!(
                positions(&world, &["a"], &witness.path)
                    .last()
                    .map(|state| world.value(state, "a", "versionNumber")),
                Some(2)
            );
            match (&witness.source, sampled) {
                (WitnessSource::Sampled(_), true) | (WitnessSource::Explored, false) => {}
                other => {
                    panic!("the selected discovery method must return its own source: {other:?}")
                }
            }
        }
        let first = world
            .replay_evidence(
                world.claim("ReachesTwo"),
                evidence(&paths, &over),
                &["a"],
                ReplayLimits::default(),
            )
            .expect("returned evidence admits");
        assert_eq!(first.settlement, GraphReplaySettlement::Reproduced);
        assert_eq!(
            first.evaluated_predicates,
            [(fsl_semantics::check::PredicateRole::Target, true); 2]
        );
        let second = world
            .replay_evidence(
                world.claim("ReachesTwo"),
                evidence(&paths, &over),
                &["a"],
                ReplayLimits::default(),
            )
            .expect("the same evidence admits");
        assert_eq!(second, first);
    }
}

#[trace("FR-170-AC-3")]
#[test]
fn truncating_a_returned_path_preserves_admission_but_disagrees_on_its_target() {
    let (world, over, mut paths) = returned(&[vec![0]], false);
    assert_eq!(paths[0].path.steps.len(), 2);
    paths[0].path.steps.pop();
    let replay = world
        .replay_evidence(
            world.claim("ReachesTwo"),
            evidence(&paths, &over),
            &["a"],
            ReplayLimits::default(),
        )
        .expect("the truncated path is still a real path");
    assert!(matches!(
        replay.settlement,
        GraphReplaySettlement::Inconclusive(fsl_replay::DisagreementCause::Verdicts { .. })
    ));
    assert_eq!(
        replay.evaluated_predicates,
        [(fsl_semantics::check::PredicateRole::Target, false)]
    );
}

#[trace("FR-170-AC-4")]
#[test]
fn missing_initial_paths_and_out_of_range_initials_are_typed_refusals() {
    let (world, over, paths) = returned(&[vec![0], vec![1]], false);
    let refused = |paths: &[GraphWitness]| {
        let error = world
            .replay_evidence(
                world.claim("ReachesTwo"),
                evidence(paths, &over),
                &["a"],
                ReplayLimits::default(),
            )
            .expect_err("malformed evidence must not settle");
        assert_eq!(error.code().as_str(), "invalid_runtime_input");
        assert_eq!(error.cause(), Some("invalid-value"));
        let ReplayRefusal::ModelTrace(reason) = error else {
            panic!("expected typed model trace refusal")
        };
        reason
    };
    assert!(matches!(
        *refused(&paths[..1]),
        ModelTraceRefusal::InitialState { index: 1, count: 2 }
    ));
    let mut bad = paths.clone();
    bad[0].path.initial = 2;
    assert!(matches!(
        *refused(&bad),
        ModelTraceRefusal::InitialState { index: 2, count: 2 }
    ));
}

#[trace("FR-170-AC-4")]
#[test]
fn replacing_a_recorded_edge_target_by_a_different_real_state_is_refused() {
    let (world, over, mut paths) = returned(&[vec![0]], false);
    let second = paths[0].path.steps[1].post_state.clone();
    paths[0].path.steps[0].post_state = second.clone();
    let error = world
        .replay_evidence(
            world.claim("ReachesTwo"),
            evidence(&paths, &over),
            &["a"],
            ReplayLimits::default(),
        )
        .expect_err("an enabled edge cannot reach its second successor in one step");
    assert_eq!(error.code().as_str(), "stale_dependency");
    assert_eq!(error.cause(), Some("content-mismatch"));
    let ReplayRefusal::ModelTrace(reason) = error else {
        panic!("expected typed trace refusal")
    };
    assert!(
        matches!(*reason, ModelTraceRefusal::PostState { step: 0, recorded, .. } if recorded == second)
    );
}

#[trace("FR-170-AC-3", "FR-170-AC-8")]
#[test]
fn a_returned_trap_settles_from_its_real_closure_and_a_shortened_stem_names_the_target() {
    let world = game_world(false, false, CAN_STILL_WIN);
    let result = run(world.request(&["g"], vec![world.item("CanStillWin")], no_samples()));
    let [instance] = result.items[0].as_slice() else {
        panic!("one game instance")
    };
    let StateGraphOutcome::Trapped { stem } = &instance.outcome else {
        panic!("the lost branch traps")
    };
    assert_eq!(instance.over.as_ref(), Some(&world.reference("g")));
    let over = ModelInstance {
        population: world.subject(&["g"], UNLIMITED).simulation.universes[0]
            .population
            .clone(),
        key: "g".to_owned(),
    };
    assert_eq!(names(&stem.steps), ["play", "lose"]);
    let mut observed = None;
    let record = fsl_replay::settle_state_graph(
        instance,
        &fsl_replay::StateGraphMethod {
            limits: no_samples(),
            max_depth: None,
            seed: DEFAULT_WITNESS_SEED,
        },
        &world.subject(&["g"], UNLIMITED),
        world.claim("CanStillWin"),
        &crate::fill::tcb_01_event_position_selection(&world.package),
        &Cancel::new(),
        |offered| {
            let StateGraphOutcome::Trapped { stem } = &offered.outcome else {
                panic!("settlement must replay its returned trap")
            };
            let replay = world.replay_evidence(
                world.claim("CanStillWin"),
                GraphEvidence::Trap {
                    stem: stem.clone(),
                    closure: fsl_replay::TrapClosure::StateGraph,
                    over: Some(over.clone()),
                },
                &["g"],
                ReplayLimits::default(),
            )?;
            observed = Some(replay.clone());
            Ok(replay)
        },
    );
    let replay = observed.expect("a trap must replay before settlement");
    assert_eq!(replay.settlement, GraphReplaySettlement::Reproduced);
    assert_eq!(replay.closure_nodes, Some(1));
    assert_eq!(replay.first_target, None);
    assert_eq!(record.value, fsl_replay::TerminalValue::Refuted);
    assert_eq!(record.basis, fsl_replay::SettlementBasis::ClosedScope);

    let mut shortened = stem.clone();
    shortened.steps.pop();
    let replay = world
        .replay_evidence(
            world.claim("CanStillWin"),
            GraphEvidence::Trap {
                stem: shortened,
                closure: fsl_replay::TrapClosure::StateGraph,
                over: Some(over),
            },
            &["g"],
            ReplayLimits::default(),
        )
        .expect("the shorter stem is a real path to Mid");
    assert!(matches!(
        replay.settlement,
        GraphReplaySettlement::Inconclusive(fsl_replay::DisagreementCause::Verdicts { .. })
    ));
    assert_eq!(replay.closure_nodes, Some(3));
    assert_eq!(
        replay.first_target,
        Some(state_key_where(&world, &["g"], |state| world
            .value(state, "g", "phase")
            == 3))
    );
}
