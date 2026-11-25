// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (ruling U7): the shared task-automaton lowering agrees with
//! the concrete `TaskProductSystem` at sampled rational-time states, and its
//! certification gate exposes the dual-WCET mode-switch exclusion (ruling U6).

use std::collections::BTreeSet;

use filament_exact::{NonZeroInteger, Rational};
use fsl_eval::simulation::model::{
    CertificationRefusal, EdgeKind, JobPhase, TaskAutomaton, TaskAutomatonRefusal, TaskEdge,
    TaskProductState, TaskProductSystem, TaskTemplate, TaskTransitionAction,
    TaskTransitionIdentity, TimedSystem,
};

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

pub(crate) fn world_for(set: &str, tasks: &[&str], declarations: &str) -> World {
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

/// `p` runs long; `q` arrives later with an earlier absolute deadline.
fn preempting_pair() -> World {
    world_for(
        "Q",
        &["p", "q"],
        "taskset Q {\n\
         task p periodic 10 ms deadline 10 ms bcet 4 ms wcet 4 ms priority 2 lo;\n\
         task q sporadic 5 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         }\n\
         schedulable QFp on Q under fixed_priority;\n\
         schedulable QEdf on Q under edf;\n",
    )
}

/// Both tasks released at once share an absolute deadline.
fn tied_pair() -> World {
    world_for(
        "T",
        &["a", "b"],
        "taskset T {\n\
         task a periodic 6 ms deadline 6 ms bcet 1 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms bcet 2 ms wcet 2 ms priority 2 lo;\n\
         }\n\
         schedulable TFp on T under fixed_priority;\n\
         schedulable TEdf on T under edf;\n",
    )
}

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

/// What a walk exercised, so a walk that never reaches a case fails.
#[derive(Default)]
struct Coverage {
    compared: usize,
    actions: BTreeSet<String>,
    /// States with two live jobs of different tasks at one absolute deadline.
    ties: usize,
    /// States whose live clocks hold two distinct nonzero fractional parts.
    fraction_orders: usize,
    /// States where a scheduler action is enabled (an invariant boundary).
    boundaries: usize,
}

fn fractional_part(value: &Rational) -> Rational {
    let denominator = NonZeroInteger::try_from(value.denominator()).expect("nonzero denominator");
    let (_, remainder) = value.numerator().div_mod_floor(denominator);
    Rational::new(remainder, value.denominator().clone()).expect("nonzero denominator")
}

fn template_label(template: &TaskTemplate, missing: usize) -> String {
    if template.action == TaskTransitionAction::DeadlineMiss && missing > 1 {
        // Simultaneous misses: the concrete product names the first in
        // activation order, a quotient node holds none (module doc).
        return format!("{:?}", template.action);
    }
    format!(
        "{:?}|{}|{}|{}|{:?}|{:?}|{:?}",
        template.action,
        template.task,
        template.job_rank,
        template.slot,
        template.release,
        template.source_transition,
        template.priority
    )
}

/// The rank of the live job `(task, slot)` among its task's jobs in `state`,
/// oldest first.
fn concrete_rank(
    product: &TaskProductSystem<'_, '_>,
    state: &TaskProductState,
    task: &str,
    slot: u64,
) -> u64 {
    let tasks = &product.plan().task_set.tasks;
    let mut same: Vec<_> = state
        .jobs()
        .iter()
        .filter(|job| tasks[job.task_index].name == task)
        .collect();
    same.sort_by(|a, b| {
        a.activated_at
            .cmp(&b.activated_at)
            .then(a.occurrence.cmp(&b.occurrence))
    });
    u64::try_from(
        same.iter()
            .position(|job| job.slot == slot)
            .expect("the identity names a live job"),
    )
    .expect("small")
}

fn identity_label(
    product: &TaskProductSystem<'_, '_>,
    labelled_in: &TaskProductState,
    identity: &TaskTransitionIdentity,
    missing: usize,
) -> String {
    if identity.action == TaskTransitionAction::DeadlineMiss && missing > 1 {
        return format!("{:?}", identity.action);
    }
    format!(
        "{:?}|{}|{}|{}|{:?}|{:?}|{:?}",
        identity.action,
        identity.task,
        concrete_rank(product, labelled_in, &identity.task, identity.slot),
        identity.slot,
        identity.release,
        identity.source_transition,
        identity.priority
    )
}

/// Walks the concrete product from the initial state along pseudo-random
/// moves and, at each visited state, asserts that the automaton's enabled
/// edges (target, valuation and template label), delays and successors agree
/// with the concrete product's.
fn cross_check(world: &World, claim: &str, seeds: u64, depth: usize) -> Coverage {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    automaton.certification().expect("fixed-work set");
    let mut coverage = Coverage::default();
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
            let missing = state
                .jobs()
                .iter()
                .filter(|job| state.now() >= &job.deadline && job.service < job.demand)
                .count();
            let jobs = state.jobs();
            if jobs.iter().any(|a| {
                jobs.iter()
                    .any(|b| a.task_index != b.task_index && a.deadline == b.deadline)
            }) {
                coverage.ties += 1;
            }
            #[allow(
                clippy::mutable_key_type,
                reason = "exact rational ordering is immutable despite the integer representation cache"
            )]
            let fractions: BTreeSet<Rational> = v
                .iter()
                .skip(1)
                .filter(|value| !value.is_integer())
                .map(fractional_part)
                .collect();
            if fractions.len() >= 2 {
                coverage.fraction_orders += 1;
            }
            if info.urgent.holds(&v) {
                coverage.boundaries += 1;
            }

            // Discrete moves: edges against concrete source and scheduler steps.
            let successors = automaton.successors(&location).expect("successors");
            assert!(successors.overflows.is_empty());
            let mut symbolic = Vec::new();
            for edge in &successors.edges {
                if let Some(next) = automaton.fire(edge, &v).expect("fire") {
                    let (label, action) = match &edge.kind {
                        EdgeKind::Source {
                            activations,
                            transition,
                        } if activations.is_empty() => {
                            (format!("source|{transition:?}"), "Source".to_owned())
                        }
                        EdgeKind::Source { activations, .. } => (
                            activations
                                .iter()
                                .map(|template| template_label(template, missing))
                                .collect::<Vec<_>>()
                                .join(";"),
                            "Activate".to_owned(),
                        ),
                        EdgeKind::Scheduler(template) => (
                            template_label(template, missing),
                            format!("{:?}", template.action),
                        ),
                    };
                    coverage.actions.insert(action);
                    symbolic.push((edge.target.key().expect("key"), next, label));
                }
            }
            let mut concrete = Vec::new();
            let scheduler = product.scheduler_steps(&state);
            let source = product.source_steps(&state).expect("source steps");
            for step in &scheduler {
                let (target, next) = automaton.abstract_state(&step.state).expect("abstraction");
                let label = identity_label(&product, &state, &step.identity, missing);
                concrete.push((target.key().expect("key"), next, label));
            }
            for step in &source {
                let (target, next) = automaton.abstract_state(&step.state).expect("abstraction");
                let label = if step.activations.is_empty() {
                    format!("source|{:?}", step.source.transition())
                } else {
                    step.activations
                        .iter()
                        .map(|identity| identity_label(&product, &step.state, identity, missing))
                        .collect::<Vec<_>>()
                        .join(";")
                };
                concrete.push((target.key().expect("key"), next, label));
            }
            symbolic.sort();
            symbolic.dedup();
            concrete.sort();
            concrete.dedup();
            assert_eq!(symbolic, concrete, "discrete successors agree at {state:?}");
            coverage.compared += 1;

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
                    coverage.compared += 1;
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
    coverage
}

fn assert_saw(coverage: &Coverage, wanted: &[&str]) {
    for action in wanted {
        assert!(
            coverage.actions.contains(*action),
            "the walk exercised {action}; saw {:?}",
            coverage.actions
        );
    }
    assert!(
        coverage.fraction_orders > 0,
        "the walk reached a fraction-order point"
    );
    assert!(coverage.boundaries > 0, "the walk reached a boundary point");
}

/// FR-418 ruling U7: fixed-priority successors (release, dispatch,
/// preemption, resumption, completion and its bounded subtractions) of the
/// task automaton equal the concrete product's, labels included, at sampled
/// rational states.
#[test]
fn fixed_priority_symbolic_successors_agree_with_the_concrete_product() {
    let coverage = cross_check(&set_b(4), "BFp", 24, 60);
    assert!(coverage.compared > 200, "compared {}", coverage.compared);
    assert_saw(
        &coverage,
        &["Activate", "Dispatch", "Preempt", "Resume", "Complete"],
    );
}

/// FR-418 ruling U7: earliest-deadline-first successors, decided by diagonal
/// deadline-clock guards and the task-name tie, agree with the concrete
/// product's, including preemption and resumption and an equal absolute
/// deadline across tasks.
#[test]
fn edf_symbolic_successors_agree_with_the_concrete_product() {
    let coverage = cross_check(&preempting_pair(), "QEdf", 40, 60);
    assert_saw(
        &coverage,
        &["Activate", "Dispatch", "Preempt", "Resume", "Complete"],
    );
    let tied = cross_check(&tied_pair(), "TEdf", 24, 40);
    assert!(tied.ties > 0, "an equal absolute deadline was reached");
    assert_saw(&tied, &["Activate", "Dispatch", "Complete"]);
    let sporadic = cross_check(&sporadic_pair(), "PEdf", 40, 60);
    assert_saw(&sporadic, &["Activate", "Dispatch", "Complete"]);
}

/// FR-418 ruling U7: a sporadic and a periodic task under fixed priority
/// agree, with preemption and resumption, and an overloaded set whose first
/// deadline miss is an edge agrees too.
#[test]
fn sporadic_and_overloaded_sets_agree_with_the_concrete_product() {
    let coverage = cross_check(&preempting_pair(), "QFp", 40, 60);
    assert_saw(
        &coverage,
        &["Activate", "Dispatch", "Preempt", "Resume", "Complete"],
    );
    let pair = cross_check(&sporadic_pair(), "PFp", 40, 60);
    assert_saw(&pair, &["Activate", "Dispatch", "Preempt", "Complete"]);
    let overloaded = cross_check(&overloaded_pair(), "OFp", 24, 30);
    assert!(overloaded.actions.contains("DeadlineMiss"));
}

/// FR-418 ruling U7, post-miss refusal: the lowering never continues past a
/// deadline miss. With x (priority 1, period 6, deadline 2, work 6) and y
/// (priority 2, period 10, deadline 5, work 1), y runs, x preempts it at 1/2
/// and misses at 5/2; past the miss y's service clock would exceed its
/// ceiling, so the miss location is terminal and its successors are refused
/// rather than the automaton time-locking.
#[test]
fn the_lowering_refuses_successors_past_a_deadline_miss() {
    let world = world_for(
        "M2",
        &["x", "y"],
        "taskset M2 {\n\
         task x periodic 6 ms deadline 2 ms bcet 6 ms wcet 6 ms priority 1 lo;\n\
         task y periodic 10 ms deadline 5 ms bcet 1 ms wcet 1 ms priority 2 lo;\n\
         }\n\
         schedulable MFp on M2 under fixed_priority;\n",
    );
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "MFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let (mut location, mut v) = automaton.initial().expect("initial").remove(0);
    let take = |location: &mut fsl_eval::simulation::model::TaskLocation,
                v: &mut Vec<Rational>,
                wanted: &dyn Fn(&TaskEdge) -> bool| {
        let edge = automaton
            .successors(location)
            .expect("successors")
            .edges
            .into_iter()
            .find(|edge| wanted(edge) && automaton.fire(edge, v).expect("fire").is_some())
            .expect("the wanted edge is enabled");
        *v = automaton.fire(&edge, v).expect("fire").expect("enabled");
        *location = edge.target;
    };
    let activates = |task: &'static str| {
        move |edge: &TaskEdge| {
            matches!(&edge.kind, EdgeKind::Source { activations, .. }
                if activations.iter().any(|a| a.task == task))
        }
    };
    let action = |action: TaskTransitionAction, task: &'static str| move |edge: &TaskEdge| matches!(&edge.kind, EdgeKind::Scheduler(t) if t.action == action && t.task == task);
    let delay = |location: &fsl_eval::simulation::model::TaskLocation,
                 v: &mut Vec<Rational>,
                 by: Rational| {
        let info = automaton.location_info(location).expect("info");
        assert!(info.admits_delay(v, &by), "delay {by:?} is admitted");
        *v = fsl_eval::simulation::model::delay_valuation(&info, v, &by);
    };
    take(&mut location, &mut v, &activates("y"));
    take(
        &mut location,
        &mut v,
        &action(TaskTransitionAction::Dispatch, "y"),
    );
    delay(&location, &mut v, rational(1, 2));
    take(&mut location, &mut v, &activates("x"));
    take(
        &mut location,
        &mut v,
        &action(TaskTransitionAction::Preempt, "y"),
    );
    take(
        &mut location,
        &mut v,
        &action(TaskTransitionAction::Dispatch, "x"),
    );
    delay(&location, &mut v, rational(2, 1));
    take(
        &mut location,
        &mut v,
        &action(TaskTransitionAction::DeadlineMiss, "x"),
    );
    assert!(location.control().missed);
    assert_eq!(
        automaton.successors(&location).err(),
        Some(TaskAutomatonRefusal::PostMiss)
    );
    let info = automaton.location_info(&location).expect("miss info");
    assert!(info.time_stopped, "a miss location stops time");
}

/// FR-418 ruling U6 (settlement is S4): a dual-WCET task set with a mode switch is not covered
/// by `closed-finite-product/v1`; the lowering exposes the refusal so the
/// checker's policy check can settle `certificate-unchecked`, and it builds no
/// edges for it.
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

/// FR-418 ruling U7 (lowering data, not yet a certificate check): the lowering is data in FR-418 clock order, with the
/// bounded subtractions and diagonals the guards use, and an activation
/// names the new job's rank and slot in its target.
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
    let _ = automaton
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

/// Lowering-only test, not certified: release jitter has no concrete oracle
/// (`TaskProductSystem` refuses it), so this checks only the structure: a task with release jitter lowers to jitter clocks, a
/// pending phase and a release edge within the jitter window.
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
    let automaton = TaskAutomaton::new_structural_jitter(&product);
    assert_eq!(
        TaskAutomaton::new(&product).certification(),
        Err(CertificationRefusal::Jitter),
        "v1 certification is jitter-free"
    );
    assert_eq!(
        TaskAutomaton::new(&product)
            .successors(&automaton.initial().expect("initial").remove(0).0)
            .err(),
        Some(TaskAutomatonRefusal::Uncertified(
            CertificationRefusal::Jitter
        ))
    );
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
