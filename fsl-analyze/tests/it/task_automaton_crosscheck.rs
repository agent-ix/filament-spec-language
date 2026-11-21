// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (ruling U7): the shared task-automaton lowering agrees with
//! the concrete `TaskProductSystem` at sampled rational-time states, and its
//! certification gate exposes the dual-WCET mode-switch exclusion (ruling U6).

use filament_exact::Rational;
use fsl_eval::simulation::model::{
    CertificationRefusal, EdgeKind, JobPhase, LocationKey, TaskAutomaton, TaskAutomatonRefusal,
    TaskProductState, TaskProductSystem, TaskTransitionAction, TimedSystem,
};
use ix_trace_rs::trace;

use crate::timed_world::{world, World, DENSE};

fn sources(set: &str, tasks: &[&str]) -> String {
    tasks
        .iter()
        .map(|task| {
            format!(
                "post Fire_{task} using v on M::Call::fire_{task} {{ self.phase = pre(self.phase) }}\n\
                 release {set}.{task} on M::Call::fire_{task};\n"
            )
        })
        .collect()
}

fn world_for(set: &str, tasks: &[&str], declarations: &str) -> World {
    let operations: Vec<(String, &[&str])> = tasks
        .iter()
        .map(|task| (format!("fire_{task}"), &["phase"][..]))
        .collect();
    let extra: Vec<(&str, &[&str])> = operations
        .iter()
        .map(|(name, fields)| (name.as_str(), *fields))
        .collect();
    let body = format!("{DENSE}{declarations}{}", sources(set, tasks));
    world(
        &crate::zone_search::rpc_document_with(&extra),
        crate::zone_search::TIMED,
        &body,
    )
}

fn set_b(third: u32) -> World {
    world_for(
        "B",
        &["t1", "t2", "t3"],
        &format!(
            "taskset B {{\n\
             task t1 periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
             task t2 periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
             task t3 periodic 10 ms deadline 10 ms bcet {third} ms wcet {third} ms priority 3 lo;\n\
             }}\n\
             schedulable BFp on B under fixed_priority;\n\
             schedulable BEdf on B under edf;\n"
        ),
    )
}

fn sporadic_pair() -> World {
    world_for(
        "P",
        &["a", "b"],
        "taskset P {\n\
         task a sporadic 3 ms deadline 5 ms bcet 2 ms wcet 2 ms priority 1 lo;\n\
         task b periodic 4 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         schedulable PFp on P under fixed_priority;\n\
         schedulable PEdf on P under edf;\n",
    )
}

type Point = (LocationKey, Vec<Rational>);

fn overloaded_pair() -> World {
    world_for(
        "O",
        &["x", "y"],
        "taskset O {\n\
         task x periodic 4 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo;\n\
         task y periodic 5 ms deadline 5 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         schedulable OFp on O under fixed_priority;\n",
    )
}

fn rational(n: i64, d: i64) -> Rational {
    crate::zone_search::exact(n, d)
}

fn sorted(mut points: Vec<Point>) -> Vec<Point> {
    points.sort();
    points.dedup();
    points
}

/// Walks the concrete product from every initial state along pseudo-random
/// moves and, at each visited state, asserts that the automaton's enabled
/// edges, delays and successors agree with the concrete product's. Returns
/// the number of discrete steps and delays compared and the actions seen.
fn cross_check(world: &World, claim: &str, seeds: u64, depth: usize) -> (usize, Vec<String>) {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    automaton.certification().expect("fixed-work set");
    let mut compared = 0;
    let mut actions = Vec::new();
    for seed in 0..seeds {
        let mut rng = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let mut pick = |bound: usize| {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from((rng >> 33) % u64::try_from(bound.max(1)).expect("small"))
                .expect("small")
        };
        let mut state: TaskProductState = product.initial().remove(0);
        for _ in 0..depth {
            if state.deadline_missed() {
                break;
            }
            let (location, v) = automaton.abstract_state(&state).expect("abstraction");
            let info = automaton.location_info(&location).expect("location info");
            assert!(
                info.invariant_holds(&v),
                "the abstraction satisfies the invariant"
            );

            // Discrete moves: edges against concrete source and scheduler steps.
            let successors = automaton.successors(&location).expect("successors");
            assert!(successors.overflows.is_empty());
            let mut symbolic = Vec::new();
            for edge in &successors.edges {
                if let Some(next) = automaton.fire(edge, &v).expect("fire") {
                    symbolic.push((edge.target.key().expect("key"), next));
                    actions.push(match &edge.kind {
                        EdgeKind::Source { activations, .. } if !activations.is_empty() => {
                            "Activate".to_owned()
                        }
                        EdgeKind::Source { .. } => "Source".to_owned(),
                        EdgeKind::Scheduler(template) => format!("{:?}", template.action),
                    });
                }
            }
            let mut concrete = Vec::new();
            let scheduler = product.scheduler_steps(&state);
            let source = product.source_steps(&state).expect("source steps");
            for next in scheduler
                .iter()
                .map(|step| &step.state)
                .chain(source.iter().map(|step| &step.state))
            {
                let (target, next) = automaton.abstract_state(next).expect("abstraction");
                concrete.push((target.key().expect("key"), next));
            }
            assert_eq!(
                sorted(symbolic),
                sorted(concrete),
                "discrete successors agree at {state:?}"
            );
            compared += 1;

            // Delays at the event boundaries, half-way to them and past them.
            let boundaries = product.next_event_delays(&state).expect("boundaries");
            let mut candidates = vec![rational(1, 3)];
            for boundary in &boundaries {
                candidates.push(boundary.clone());
                candidates.push(boundary.div(&rational(2, 1)).expect("half"));
                candidates.push(boundary.add(&rational(1, 3)));
            }
            let mut admitted = Vec::new();
            for delay in &candidates {
                let reached = product.delay(&state, delay).expect("source delay");
                assert_eq!(
                    reached.is_some(),
                    info.admits_delay(&v, delay),
                    "delay {delay:?} admitted alike at {state:?}"
                );
                if let Some(reached) = reached {
                    let (target, point) = automaton.abstract_state(&reached).expect("abstraction");
                    assert_eq!(target.key().expect("key"), location.key().expect("key"));
                    assert_eq!(
                        point,
                        fsl_eval::simulation::model::delay_valuation(&info, &v, delay),
                        "delayed valuation agrees"
                    );
                    admitted.push(reached);
                    compared += 1;
                }
            }

            // Take one concrete move: a scheduler step if one is enabled,
            // else a source step or an admitted delay.
            if let Some(step) = scheduler.into_iter().next() {
                state = step.state;
                continue;
            }
            let moves = source.len() + admitted.len();
            if moves == 0 {
                break;
            }
            let chosen = pick(moves);
            state = if chosen < source.len() {
                source.into_iter().nth(chosen).expect("source step").state
            } else {
                admitted.swap_remove(chosen - source.len())
            };
        }
    }
    (compared, actions)
}

fn assert_saw(actions: &[String], wanted: &[&str]) {
    for action in wanted {
        assert!(
            actions.iter().any(|seen| seen == action),
            "the walk exercised {action}; saw {:?}",
            {
                let mut seen = actions.to_vec();
                seen.sort();
                seen.dedup();
                seen
            }
        );
    }
}

/// FR-418-AC-12, AC-13: fixed-priority successors (release, dispatch,
/// preemption, resumption, completion and its bounded subtractions) of the
/// task automaton equal the concrete product's at sampled rational states.
#[trace("TC-371", "FR-418-AC-12")]
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn fixed_priority_symbolic_successors_agree_with_the_concrete_product() {
    let (compared, actions) = cross_check(&set_b(4), "BFp", 24, 60);
    assert!(compared > 200, "compared {compared}");
    assert_saw(
        &actions,
        &["Activate", "Dispatch", "Preempt", "Resume", "Complete"],
    );
}

/// FR-418-AC-12: earliest-deadline-first successors, decided by diagonal
/// deadline-clock guards and the task-name tie, agree with the concrete
/// product's.
#[trace("TC-371", "FR-418-AC-12")]
#[test]
fn edf_symbolic_successors_agree_with_the_concrete_product() {
    let (compared, actions) = cross_check(&set_b(4), "BEdf", 24, 60);
    assert!(compared > 200, "compared {compared}");
    assert_saw(&actions, &["Activate", "Dispatch", "Complete"]);
    let (_, sporadic) = cross_check(&sporadic_pair(), "PEdf", 24, 60);
    assert_saw(&sporadic, &["Activate", "Dispatch", "Complete"]);
}

/// FR-418-AC-13: a sporadic and a periodic task under both policies, and an
/// overloaded set whose first deadline miss is an edge, agree too.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn sporadic_and_overloaded_sets_agree_with_the_concrete_product() {
    let (compared, actions) = cross_check(&sporadic_pair(), "PFp", 24, 60);
    assert!(compared > 100, "compared {compared}");
    assert_saw(&actions, &["Activate", "Dispatch", "Complete"]);
    let (_, overloaded) = cross_check(&overloaded_pair(), "OFp", 24, 30);
    assert_saw(&overloaded, &["DeadlineMiss"]);
}

/// FR-418 ruling U6: a dual-WCET task set with a mode switch is not covered
/// by `closed-finite-product/v1`; the lowering exposes the refusal so the
/// checker's policy check can settle `certificate-unchecked`, and it builds no
/// edges for it.
#[trace("TC-371", "FR-418-AC-21")]
#[test]
fn dual_wcet_mode_switch_is_not_certified() {
    let body_tasks = "taskset D {\n\
         task command periodic 4 ms deadline 4 ms bcet 2 ms wcet 2 ms priority 1 hi wcet_hi 3 ms;\n\
         task worker periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         mode D hi on M::Call::fire_command;\n\
         schedulable DFp on D under fixed_priority;\n";
    let world = world_for("D", &["command", "worker"], body_tasks);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "DFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    assert_eq!(
        automaton.certification(),
        Err(CertificationRefusal::DualWcetModeSwitch)
    );
    let (initial, _) = automaton.initial().expect("initial").remove(0);
    assert_eq!(
        automaton.successors(&initial).err(),
        Some(TaskAutomatonRefusal::Uncertified(
            CertificationRefusal::DualWcetModeSwitch
        ))
    );
}

/// FR-418-AC-13: the lowering is data in FR-418 clock order, with the
/// bounded subtractions and diagonals the guards use, and an activation
/// names the new job's rank and slot in its target.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn the_automaton_exposes_clocks_subtractions_diagonals_and_templates() {
    let world = set_b(4);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "BEdf").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let names: Vec<&str> = automaton.clocks().iter().map(|c| c.name.as_str()).collect();
    let aux: Vec<String> = product
        .plan()
        .auxiliary_clocks()
        .into_iter()
        .map(|clock| clock.name)
        .collect();
    assert_eq!(&names[automaton.source_clocks()..], aux.as_slice());
    assert!(!automaton.diagonals().is_empty());
    assert!(automaton
        .diagonals()
        .iter()
        .all(|d| d.left != 0 && d.right != 0));

    let (mut location, mut v) = automaton.initial().expect("initial").remove(0);
    // Activate t1 then t2 at time zero (one source edge each).
    for task in ["t1", "t2"] {
        let successors = automaton.successors(&location).expect("successors");
        let edge = successors
            .edges
            .iter()
            .find(|edge| {
                matches!(&edge.kind, EdgeKind::Source { activations, .. }
                    if activations.iter().any(|a| a.task == task))
            })
            .expect("the release is enabled");
        let EdgeKind::Source { activations, .. } = &edge.kind else {
            unreachable!()
        };
        assert_eq!((activations[0].job_rank, activations[0].slot), (0, 0));
        v = automaton.fire(edge, &v).expect("fire").expect("enabled");
        location = edge.target.clone();
    }
    // Dispatch t1 (earliest deadline), run it to completion: a bounded
    // subtraction applies to no started job yet, so dispatch t2 afterwards.
    let dispatch = automaton
        .successors(&location)
        .expect("successors")
        .edges
        .into_iter()
        .find(|edge| matches!(&edge.kind, EdgeKind::Scheduler(t) if t.task == "t1" && t.action == TaskTransitionAction::Dispatch))
        .expect("t1 dispatches first");
    v = automaton
        .fire(&dispatch, &v)
        .expect("fire")
        .expect("enabled");
    assert!(dispatch
        .target
        .control()
        .jobs
        .iter()
        .any(|job| job.phase == JobPhase::Running));
}

/// FR-418-AC-13: a task with release jitter lowers to jitter clocks, a
/// pending phase and a release edge within the jitter window.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn jitter_lowers_to_a_pending_phase_and_a_release_edge() {
    let world = world_for(
        "J",
        &["a"],
        "taskset J {\n\
         task a periodic 6 ms deadline 6 ms bcet 1 ms wcet 1 ms priority 1 jitter 1 ms lo;\n\
         }\n\
         schedulable JFp on J under fixed_priority;\n",
    );
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "JFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let (location, v) = automaton.initial().expect("initial").remove(0);
    let successors = automaton.successors(&location).expect("successors");
    let activate = successors
        .edges
        .iter()
        .find(|edge| matches!(edge.kind, EdgeKind::Source { .. }))
        .expect("release");
    let v = automaton
        .fire(activate, &v)
        .expect("fire")
        .expect("enabled");
    let pending = &activate.target;
    assert_eq!(pending.control().jobs[0].phase, JobPhase::Pending);
    let (jitter, _, _) = automaton.slot_clocks(0, 0).expect("slot clocks");
    let jitter = jitter.expect("a jitter clock");
    let info = automaton.location_info(pending).expect("info");
    assert!(info.live[jitter]);
    assert!(info.admits_delay(&v, &rational(1, 1)));
    assert!(
        !info.admits_delay(&v, &rational(3, 2)),
        "jitter bounds time"
    );
    let release = automaton
        .successors(pending)
        .expect("successors")
        .edges
        .into_iter()
        .find(|edge| matches!(&edge.kind, EdgeKind::Scheduler(t) if t.action == TaskTransitionAction::Release))
        .expect("a release edge");
    assert_eq!(release.target.control().jobs[0].phase, JobPhase::Ready);
}

/// FR-418-AC-13: an allocation past a task's `ceil(D/T)+1` slots is an
/// overflow where it is enabled, never a location or an edge.
#[trace("TC-371", "FR-418-AC-13")]
#[test]
fn an_allocation_past_the_slot_bound_is_overflow_not_a_node() {
    let world = world_for(
        "W",
        &["a"],
        "taskset W {\n\
         task a periodic 1 ms deadline 1 ms bcet 5 ms wcet 5 ms priority 1 lo;\n\
         }\n\
         schedulable WFp on W under fixed_priority;\n",
    );
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "WFp").expect("checked task product");
    assert_eq!(product.plan().tasks[0].max_jobs, 2);
    let automaton = TaskAutomaton::new(&product);
    let (mut location, mut v) = automaton.initial().expect("initial").remove(0);
    let mut overflowed = false;
    for _ in 0..40 {
        let successors = automaton.successors(&location).expect("successors");
        if successors
            .overflows
            .iter()
            .any(|overflow| overflow.guard.holds(&v))
        {
            overflowed = true;
            assert!(!successors.edges.iter().any(|edge| matches!(
                &edge.kind,
                EdgeKind::Source { activations, .. } if !activations.is_empty()
            ) && automaton
                .fire(edge, &v)
                .expect("fire")
                .is_some()));
            break;
        }
        let fired = successors.edges.iter().find_map(|edge| {
            if matches!(&edge.kind, EdgeKind::Source { activations, .. } if activations.is_empty())
            {
                return None;
            }
            let next = automaton.fire(edge, &v).expect("fire")?;
            Some((edge.target.clone(), next))
        });
        if let Some((target, next)) = fired {
            (location, v) = (target, next);
            continue;
        }
        let info = automaton.location_info(&location).expect("info");
        let delay = rational(1, 1);
        assert!(info.admits_delay(&v, &delay));
        v = fsl_eval::simulation::model::delay_valuation(&info, &v, &delay);
    }
    assert!(overflowed, "a third live job needs a slot past max_jobs");
}
