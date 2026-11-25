// SPDX-License-Identifier: AGPL-3.0-or-later
//! Returned state-graph evidence admitted and replayed through the public facade.

use super::*;
use crate::terminal_two_models::handoff_package;
use filament_semantic_value::checking::CheckingLimits;
use fsl_eval::simulation::model::PackageModelInput;
use fsl_replay::{GraphReplaySettlement, ModelTraceRefusal, ReplayRefusal, WitnessPath};
use fsl_semantics::model::accounting::ModelNormalizationLimits;

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

const HALF: &str = "constraint Half using v on Config::ConfigVersion at current { \
     quotient(2 - self.versionNumber) >= 1 }\n";
const UNDEFINED_GUARD: &str = "pre Guard using v on Config::ConfigVersion::attemptUpdate { \
     quotient(2 - self.versionNumber) >= 1 }\n";
/// `Stop` leaves `versionNumber = 2` with no enabled transition, where the
/// terminal predicate divides by zero.
const UNDEFINED_TERMINAL: &str =
    "pre Stop using v on Config::ConfigVersion::attemptUpdate { self.versionNumber < 2 }\n\
     terminal Done using v on Config when { quotient(2 - self.versionNumber) >= 0 };\n";

/// An undefined guard, `terminal when` predicate and state constraint each
/// refute through a replay that reads the expression from the evidence's
/// locus; evidence naming another transition, stem or cause disagrees.
#[trace("TC-612", "FR-170-AC-7")]
#[test]
fn undefined_subject_evidence_replays_from_its_expression_locus() {
    for (extra, keys, constraint) in [
        (HALF, &["a"][..], Some("Half")),
        (UNDEFINED_GUARD, AB, None),
        (UNDEFINED_TERMINAL, &["a"][..], None),
    ] {
        let world =
            config_replayable_kernel_world(extra, REACHES_TWO, keys, &[vec![0; keys.len()]]);
        let claim = world.claim("ReachesTwo");
        let over = ModelInstance {
            population: world.subject(keys, UNLIMITED).simulation.universes[0]
                .population
                .clone(),
            key: "a".to_owned(),
        };
        let replay = |stem: &ModelPath, undefined: &fsl_replay::GraphUndefined| {
            world.replay_evidence(
                claim,
                GraphEvidence::Undefined {
                    stem: stem.clone(),
                    undefined: undefined.clone(),
                    over: Some(over.clone()),
                },
                keys,
                ReplayLimits::default(),
            )
        };
        let mut request = world.request(
            keys,
            vec![StateGraphItem::for_binding(claim, over.clone())],
            no_samples(),
        );
        request.reductions.constraint = constraint.map(str::to_owned);
        let mut offered = None;
        let settled = check_and_settle_state_graph(request, &Cancel::new(), |instance| {
            let StateGraphOutcome::Undefined { stem, undefined } = &instance.outcome else {
                panic!("undefined: {:?}", instance.outcome);
            };
            offered = Some((stem.clone(), undefined.clone()));
            replay(stem, undefined)
        })
        .expect("the subject is checked");
        let (stem, undefined) = offered.expect("an undefined evaluation is replayed");
        assert_eq!(stem.steps.len(), 2, "{extra}");
        assert_eq!(undefined.predicate, None);
        assert_eq!(undefined.transition.is_some(), extra == UNDEFINED_GUARD);
        let record = &settled.records[0][0];
        assert_eq!(record.value, fsl_replay::TerminalValue::Refuted, "{extra}");
        assert_eq!(
            record.basis,
            fsl_replay::SettlementBasis::DecisiveCounterexample
        );
        assert_eq!(record.undefined.as_ref(), Some(&undefined));

        let disagrees = |stem: &ModelPath, undefined: &fsl_replay::GraphUndefined| {
            let replayed = replay(stem, undefined).expect("the evidence is admitted");
            assert!(
                matches!(
                    replayed.settlement,
                    GraphReplaySettlement::Inconclusive(
                        fsl_replay::DisagreementCause::Verdicts { .. }
                    )
                ),
                "{extra}: {:?}",
                replayed.settlement
            );
        };
        let mut shortened = stem.clone();
        shortened.steps.pop();
        disagrees(&shortened, &undefined);
        let mut other_cause = undefined.clone();
        other_cause.cause =
            fsl_eval::temporal::UndefinedCause::Kernel(filament_exact::Undefined::NoneValue);
        disagrees(&stem, &other_cause);
        if let Some(guarded) = &undefined.transition {
            let system = system(&world, keys);
            let other = system
                .successors(&system.initial()[0])
                .expect("expands")
                .successors
                .into_iter()
                .map(|(transition, _)| transition)
                .find(|transition| transition != guarded)
                .expect("the other object's update");
            let mut renamed = undefined.clone();
            renamed.transition = Some(other);
            disagrees(&stem, &renamed);
        }
    }
}

/// Over a checked package read from its handoff bytes, an undefined guard,
/// `terminal when` predicate and state constraint are each reported at the
/// locus the package holds for that expression.
#[trace("TC-612", "FR-170-AC-7")]
#[test]
fn a_package_subject_reports_each_undefined_expression_at_its_locus() {
    let mut loci = Vec::new();
    for (extra, keys, constraint) in [
        (HALF, &["a"][..], Some("Half")),
        (UNDEFINED_GUARD, AB, None),
        (UNDEFINED_TERMINAL, &["a"][..], None),
    ] {
        let world =
            config_replayable_kernel_world(extra, REACHES_TWO, keys, &[vec![0; keys.len()]]);
        let handoff = handoff_package(&world.package, world.packages());
        let bare = handoff.bare();
        let input =
            PackageModelInput::of(&bare, world.packages(), ModelNormalizationLimits::default())
                .expect("the package route reads the unit");
        let occurrence = world
            .claim("ReachesTwo")
            .occurrence()
            .expect("a checked claim has an occurrence");
        let claim = input
            .state_graph_claim(occurrence, CheckingLimits::default().nodes())
            .expect("the package rebuilds the claim");
        let subject = || {
            let mut subject = world.subject(keys, UNLIMITED);
            subject.simulation.input = input.input();
            subject
        };
        let mut request = world.request(keys, Vec::new(), no_samples());
        request.subject = subject();
        request.temporal_clock_selections = Vec::new();
        request.items = vec![StateGraphItem::for_binding(
            &claim,
            bound_to(&world, keys, "a"),
        )];
        request.reductions.constraint = constraint.map(str::to_owned);
        let result = run(request);
        let StateGraphOutcome::Undefined { stem, undefined } = &result.items[0][0].outcome else {
            panic!("undefined: {:?}", result.items[0][0].outcome);
        };
        assert_eq!(stem.steps.len(), 2, "{extra}");
        let system = ModelSystem::new(subject().simulation).expect("the package admits");
        let expected = if let Some(name) = constraint {
            system.state_clause_locus(name)
        } else if extra == UNDEFINED_GUARD {
            system.state_clause_locus("Guard")
        } else {
            let dead_end = system
                .successors(&system.initial()[0])
                .and_then(|first| system.successors(&first.successors[0].1))
                .expect("expands")
                .successors
                .remove(0)
                .1;
            let reading = system.terminal_reading(&dead_end, false).expect("reads");
            system.terminal_locus(&reading.undefined.expect("`Done` is undefined").0)
        };
        assert!(expected.is_some(), "{extra}");
        assert_eq!(undefined.locus, expected, "{extra}");
        loci.push(expected);
    }
    assert!(loci[0] != loci[1] && loci[1] != loci[2] && loci[0] != loci[2]);
}
