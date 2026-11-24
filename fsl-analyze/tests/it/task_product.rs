// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249's concrete task-product lowering, scheduler and miss identity.

use filament_semantic_value::model_transition::ModelTransition;
use fsl_analyze::zone_check::{
    TaskProductSearchLimits, TaskProductSearchOutcome, TaskProductSearchStop, check_task_product,
    check_task_product_with_cancel,
};
use fsl_eval::simulation::model::{
    TaskProductSystem, TaskProductTrace, TaskProductTraceStep, TaskProductTransition,
    TaskTransitionAction, TimedSystem,
};
use fsl_replay::{
    CounterexampleKind, Lasso, ModelStep, TaskQueueBound, TemporalCounterexample,
    task_temporal_wire,
};
use ix_trace_rs::trace;

use crate::timed_world::{DENSE, document, world};

fn world_with_task(work: u32, deadline: u32) -> crate::timed_world::World {
    world_with_task_bounds(work, work, deadline)
}

fn world_with_task_bounds(bcet: u32, wcet: u32, deadline: u32) -> crate::timed_world::World {
    let body = format!(
        "{DENSE}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and self.late = (pre(self.phase) = 3) and (pre(self.phase) = 1 implies self.phase = 2) and (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= 3 ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         taskset Tasks {{ task command periodic 4 ms deadline {deadline} ms bcet {bcet} ms wcet {wcet} ms priority 1 lo; }}\n\
         release Tasks.command on M::Call::send;\n\
         schedulable TaskClaim on Tasks under fixed_priority;\n"
    );
    world(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        &body,
    )
}

fn world_with_two_releases() -> crate::timed_world::World {
    let body = format!(
        "{DENSE}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and self.late = (pre(self.phase) = 3) and (pre(self.phase) = 1 implies self.phase = 2) and (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= 3 ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         taskset Tasks {{ task alpha periodic 4 ms deadline 2 ms bcet 1 ms wcet 1 ms priority 1 lo; task beta periodic 4 ms deadline 2 ms bcet 1 ms wcet 1 ms priority 2 lo; }}\n\
         release Tasks.beta on M::Call::send;\n\
         release Tasks.alpha on M::Call::send;\n\
         schedulable TaskClaim on Tasks under fixed_priority;\n"
    );
    world(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        &body,
    )
}

fn world_with_hi_mode_switch(mode_operation: &str) -> crate::timed_world::World {
    world_with_hi_mode_switch_order(Some(mode_operation), false)
}

fn world_with_hi_mode_switch_order(
    mode_operation: Option<&str>,
    reverse_release_order: bool,
) -> crate::timed_world::World {
    let releases = if reverse_release_order {
        "release Tasks.worker on M::Call::send;\nrelease Tasks.command on M::Call::send;\n"
    } else {
        "release Tasks.command on M::Call::send;\nrelease Tasks.worker on M::Call::send;\n"
    };
    let mode_declaration = mode_operation.map_or_else(String::new, |operation| {
        format!("mode Tasks hi on M::Call::{operation};\n")
    });
    let body = format!(
        "{DENSE}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and self.late = (pre(self.phase) = 3) and (pre(self.phase) = 1 implies self.phase = 2) and (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= 3 ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         taskset Tasks {{ task command periodic 4 ms deadline 4 ms bcet 3 ms wcet 2 ms priority 1 hi wcet_hi 3 ms; task worker periodic 4 ms deadline 4 ms bcet 3 ms wcet 2 ms priority 2 hi wcet_hi 3 ms; }}\n\
         {releases}\
         {mode_declaration}\
         schedulable TaskClaim on Tasks under fixed_priority;\n"
    );
    world(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        &body,
    )
}

/// FR-249-AC-1's task-derived release, service, completion and deadline-miss
/// identity use the checked task and source transition.
#[trace("FR-249-AC-1")]
#[test]
fn fixed_work_release_completes_or_emits_a_deadline_miss() {
    let world = world_with_task(1, 2);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let stopped = check_task_product(&product, TaskProductSearchLimits { max_work: 1 })
        .expect("task-product search is admitted");
    assert!(matches!(
        stopped,
        TaskProductSearchOutcome::Stopped {
            cause: TaskProductSearchStop::WorkLimit(1),
            expanded: 1,
            ..
        }
    ));
    let initial = product.initial().into_iter().next().expect("initial state");
    let activated = product
        .source_steps(&initial)
        .expect("exact task choices")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command");
    assert_eq!(activated.activations.len(), 1);
    assert_eq!(
        activated.activations[0].action,
        TaskTransitionAction::Activate
    );

    let dispatched = product
        .scheduler_steps(&activated.state)
        .into_iter()
        .next()
        .expect("ready job dispatches");
    assert_eq!(dispatched.identity.action, TaskTransitionAction::Dispatch);
    assert!(
        product
            .delay(&dispatched.state, &crate::zone_search::exact(2, 1))
            .expect("source permits the larger time span")
            .is_none(),
        "time cannot pass the one-unit completion event"
    );
    let events = product
        .next_event_delays(&dispatched.state)
        .expect("task and source event boundaries");
    assert!(events.contains(&crate::zone_search::exact(1, 1)));
    let elapsed = product
        .delay(&dispatched.state, &crate::zone_search::exact(1, 1))
        .expect("source remains admissible")
        .expect("one millisecond is admissible");
    let completed = product
        .scheduler_steps(&elapsed)
        .into_iter()
        .next()
        .expect("completed job leaves the product");
    assert_eq!(completed.identity.action, TaskTransitionAction::Complete);
    assert!(completed.state.jobs().is_empty());

    let world = world_with_task(2, 1);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let searched = check_task_product(&product, TaskProductSearchLimits { max_work: 4096 })
        .expect("bounded task-product exploration is admitted");
    if let TaskProductSearchOutcome::Refuted(refutation) = searched {
        let replayed = product
            .replay_trace(&refutation.trace)
            .unwrap_or_else(|error| {
                panic!(
                    "finite miss replay failed: {error:?}; trace={:#?}",
                    refutation.trace
                )
            });
        assert!(replayed.deadline_missed());
    }
    let initial = product.initial().into_iter().next().expect("initial state");
    let activated = product
        .source_steps(&initial)
        .expect("exact task choices")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command");
    let dispatched = product
        .scheduler_steps(&activated.state)
        .into_iter()
        .next()
        .expect("ready job dispatches");
    let at_deadline = product
        .delay(&dispatched.state, &crate::zone_search::exact(1, 1))
        .expect("source remains admissible")
        .expect("the deadline instant is admissible");
    let miss = product
        .scheduler_steps(&at_deadline)
        .into_iter()
        .next()
        .expect("deadline miss is an explicit product transition");
    assert_eq!(miss.identity.action, TaskTransitionAction::DeadlineMiss);
    assert_eq!(miss.identity.task, "command");
    assert_eq!(miss.identity.demand, crate::zone_search::exact(2, 1));
    assert!(miss.state.deadline_missed());
    let trace = TaskProductTrace {
        initial_state: 0,
        lasso_entry: None,
        steps: vec![
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: activated.transitions()[0].clone(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: dispatched.transition(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(1, 1),
                transition: miss.transition(),
            },
        ],
    };
    assert!(
        product
            .replay_trace(&trace)
            .expect("the derived miss replays")
            .deadline_missed()
    );
    let counterexample = TemporalCounterexample {
        initial_state: 0,
        prefix: vec![
            ModelStep {
                transition: activated.transitions()[0].clone().into(),
                post_state: product
                    .source()
                    .state_key(activated.state.source())
                    .expect("activation state key"),
                delay: Some(crate::zone_search::exact(0, 1)),
            },
            ModelStep {
                transition: dispatched.transition().into(),
                post_state: product
                    .source()
                    .state_key(dispatched.state.source())
                    .expect("dispatch state key"),
                delay: Some(crate::zone_search::exact(0, 1)),
            },
            ModelStep {
                transition: miss.transition().into(),
                post_state: product
                    .source()
                    .state_key(miss.state.source())
                    .expect("miss state key"),
                delay: Some(crate::zone_search::exact(1, 1)),
            },
        ],
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    let wire = task_temporal_wire(&counterexample, None, None).expect("FR-364 spelling");
    assert_eq!(wire["prefix"][0]["transition"]["type"], "task-transition");
    assert_eq!(wire["prefix"][2]["transition"]["action"], "deadline-miss");
}

/// The EN-6 task-product search preserves a caller cancellation instead of
/// reporting it as an ordinary exhausted frontier.
#[test]
fn task_product_search_preserves_cancellation() {
    let world = world_with_task(1, 2);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let cancel = filament_exact::Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Deadline);

    let outcome = check_task_product_with_cancel(
        &product,
        TaskProductSearchLimits { max_work: 4096 },
        &cancel,
    )
    .expect("cancellation is a search outcome");

    assert!(matches!(
        outcome,
        TaskProductSearchOutcome::Stopped {
            cause: TaskProductSearchStop::Cancelled(filament_exact::CancelCause::Deadline),
            ..
        }
    ));
}

/// FR-249-AC-3/15: replay validates the captured task identity and declared
/// priority instead of accepting a tampered miss step.
#[trace("FR-249-AC-3", "FR-249-AC-15")]
#[test]
fn task_product_replay_rejects_a_tampered_miss_identity() {
    let world = world_with_task(2, 1);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activated = product
        .source_steps(&initial)
        .expect("exact task choices")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command");
    let dispatched = product
        .scheduler_steps(&activated.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("ready job dispatches");
    let at_deadline = product
        .delay(&dispatched.state, &crate::zone_search::exact(1, 1))
        .expect("source admits the deadline")
        .expect("deadline instant is admissible");
    let missed = product
        .scheduler_steps(&at_deadline)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::DeadlineMiss)
        .expect("the deadline-miss step is explicit");
    let trace = TaskProductTrace {
        initial_state: 0,
        lasso_entry: None,
        steps: vec![
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: activated.transitions()[0].clone(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: dispatched.transition(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(1, 1),
                transition: missed.transition(),
            },
        ],
    };
    assert!(
        product
            .replay_trace(&trace)
            .expect("exact prefix replays")
            .deadline_missed()
    );
    let after_miss = product
        .delay(&missed.state, &crate::zone_search::exact(1, 1))
        .expect("the latched miss does not stop later scheduler behavior")
        .expect("the job can receive its remaining service");
    let completed = product
        .scheduler_steps(&after_miss)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Complete)
        .expect("the late job can complete after its miss");
    assert!(
        completed.state.deadline_missed(),
        "the first miss stays latched"
    );
    assert!(
        product
            .scheduler_steps(&completed.state)
            .iter()
            .all(|step| step.identity.action != TaskTransitionAction::DeadlineMiss)
    );

    for field in ["deadline", "demand", "priority"] {
        let mut tampered = trace.clone();
        let TaskProductTransition::Task(identity) = &mut tampered.steps[2].transition else {
            unreachable!("the last transition is the task miss");
        };
        match field {
            "deadline" => identity.deadline = crate::zone_search::exact(2, 1),
            "demand" => identity.demand = crate::zone_search::exact(3, 1),
            "priority" => identity.priority = Some(2),
            _ => unreachable!(),
        }
        assert!(product.replay_trace(&tampered).is_err(), "tampered {field}");
    }
}

/// FR-249 permits one source operation to release several tasks and orders
/// simultaneous jobs by their checked fixed priorities.
#[trace("FR-249-AC-1", "FR-249-AC-15")]
#[test]
fn one_source_step_can_activate_multiple_bound_tasks() {
    let world = world_with_two_releases();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("task activations are concrete")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates both tasks");
    assert_eq!(activation.activations.len(), 2);
    assert_eq!(activation.transitions().len(), 2);
    assert_eq!(
        activation
            .activations
            .iter()
            .map(|identity| identity.task.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
    let dispatch = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("the highest-priority released task dispatches first");
    assert_eq!(dispatch.identity.task, "alpha");
    let trace = TaskProductTrace {
        initial_state: 0,
        lasso_entry: None,
        steps: activation
            .transitions()
            .into_iter()
            .map(|transition| TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition,
            })
            .collect(),
    };
    let replayed = product
        .replay_trace(&trace)
        .expect("one source move replays its complete activation group");
    assert_eq!(replayed.jobs().len(), 2);
    let replay_states = product
        .replay_trace_states(&trace)
        .expect("each activation step replays");
    assert_eq!(
        product
            .state_key(&replay_states[1])
            .expect("first activation state has a key"),
        product
            .state_key(&replay_states[2])
            .expect("second activation carries the whole move's post-state")
    );

    let outcome = check_task_product(&product, TaskProductSearchLimits { max_work: 16 })
        .expect("the task-product search is admitted");
    assert!(!matches!(
        &outcome,
        TaskProductSearchOutcome::Stopped {
            cause: TaskProductSearchStop::UnresolvedChoice,
            ..
        }
    ));
}

/// FR-249-AC-5/FR-331: a finite candidate-slot overflow remains a replayable
/// frontier and cannot close the missed-deadline path as a finite lasso.
#[trace("TC-372-RT-25", "FR-249-AC-5", "FR-331-AC-1")]
#[test]
fn periodic_overload_keeps_queue_overflow_as_an_open_frontier() {
    let model_document = crate::zone_search::rpc_document_with(&[("fire_t", &["phase"])]);
    for policy in ["fixed_priority", "edf"] {
        let declarations = format!(
            "{DENSE}\
             taskset Overload {{ task t periodic 1 ms deadline 1 ms bcet 2 ms wcet 2 ms priority 1 lo; }}\n\
             schedulable OverloadFp on Overload under {policy};\n\
             post FireT using v on M::Call::fire_t {{ self.phase = pre(self.phase) }}\n\
             release Overload.t on M::Call::fire_t;\n"
        );
        let world = world(&model_document, crate::zone_search::TIMED, &declarations);
        let model = world.system();
        let timed = TimedSystem::new(&model, "M").expect("dense timed model");
        let product = TaskProductSystem::new(timed, "OverloadFp").expect("checked task product");
        let outcome = check_task_product(&product, TaskProductSearchLimits { max_work: 10_000 })
            .expect("bounded task search is admitted");
        let TaskProductSearchOutcome::Stopped {
            cause: TaskProductSearchStop::QueueOverflow,
            frontier,
            ..
        } = outcome
        else {
            panic!("{policy} slot overflow cannot settle a refutation: {outcome:?}");
        };
        let boundary = frontier
            .iter()
            .find(|node| node.stop == TaskProductSearchStop::QueueOverflow)
            .expect("the overflow boundary is retained");
        let trace_state = product
            .replay_trace(&boundary.trace)
            .expect("frontier path replays to the checked product");
        let state = if boundary.final_delay.is_zero() {
            trace_state
        } else {
            product
                .delay(&trace_state, &boundary.final_delay)
                .expect("frontier tail delay is admitted")
                .expect("frontier tail delay reaches the recorded node")
        };
        assert_eq!(
            product
                .state_key(&state)
                .expect("frontier state has a finite key"),
            boundary.state_key
        );
        let blocked = product
            .queue_cap_blocked(&state)
            .expect("the checked source transitions are available");
        assert!(
            blocked,
            "{policy} frontier state={:?}, final_delay={:?}, trace={:?}, task bounds={:?}",
            state,
            boundary.final_delay,
            boundary.trace.steps,
            product.plan().tasks
        );
    }
}

/// FR-249-AC-9/FR-364: a miss is not a refutation until its same-path lasso
/// repeats the product state with positive elapsed time.
#[trace("TC-3008", "FR-249-AC-9", "FR-237-AC-8")]
#[trace("TC-523", "FR-128-AC-11")]
#[test]
fn missed_deadline_settles_only_with_a_replayed_positive_time_lasso() {
    let world = world_with_task(2, 1);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let outcome = check_task_product(&product, TaskProductSearchLimits { max_work: 4096 })
        .expect("task-product search is admitted");
    let TaskProductSearchOutcome::Refuted(refutation) = outcome else {
        panic!("the recurring source supplies a replayable continuation: {outcome:?}");
    };
    let entry = refutation
        .trace
        .lasso_entry
        .expect("a miss alone is not a refutation");
    let states = product
        .replay_trace_states(&refutation.trace)
        .unwrap_or_else(|error| {
            panic!(
                "miss/lasso replay failed: {error:?}; trace={:#?}",
                refutation.trace
            )
        });
    assert!(
        states[entry].now() < states.last().expect("final state").now(),
        "the loop has positive elapsed time"
    );
    assert!(states.last().expect("final state").deadline_missed());
    let miss = refutation
        .trace
        .steps
        .iter()
        .position(|step| {
            matches!(
                &step.transition,
                TaskProductTransition::Task(identity)
                    if identity.action == TaskTransitionAction::DeadlineMiss
            )
        })
        .expect("the replayed prefix contains its first missed deadline");
    assert!(entry > miss, "the loop starts after the first miss");
    assert!(refutation.trace.steps[entry..].iter().all(|step| {
        !matches!(
            &step.transition,
            TaskProductTransition::Task(identity)
                if identity.action == TaskTransitionAction::DeadlineMiss
        )
    }));

    let typed_counterexample = TemporalCounterexample::<fsl_replay::TaskProductTransition> {
        initial_state: refutation.trace.initial_state,
        prefix: refutation
            .trace
            .steps
            .iter()
            .zip(states.iter().skip(1))
            .map(|(step, state)| ModelStep {
                transition: step.transition.clone().into(),
                post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(
                    product.state_key(state).expect("replayed state key"),
                ),
                delay: Some(step.delay.clone()),
            })
            .collect(),
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    fsl_replay::replay_task_product_trace(&product, &typed_counterexample)
        .expect("the typed task trace independently replays");

    // FR-128-AC-11: the task alphabet has no `closure-stutter` step, so the
    // typed entry refuses a listed one at its `/transition/type` before the
    // product expands: a first step whose key is wrong would otherwise
    // refuse at its `/post_state`.
    for site in [
        fsl_replay::StepSite::Prefix(0),
        fsl_replay::StepSite::Loop(0),
    ] {
        let mut closure = typed_counterexample.clone();
        closure.prefix[0].post_state =
            fsl_eval::simulation::StateKey::from_canonical_bytes(b"{}".to_vec());
        assert!(
            fsl_replay::replay_task_product_trace(&product, &closure).is_err(),
            "the control: the wrong key alone refuses"
        );
        closure.closure_stutters = vec![fsl_replay::ClosureStutter {
            site,
            post_state: closure.prefix[0].post_state.clone(),
            delay: None,
        }];
        assert_eq!(
            fsl_replay::replay_task_product_trace(&product, &closure),
            Err(fsl_replay::TaskProductTraceReplayRefusal::TraceAlphabet {
                pointer: format!("{}/transition/type", site.pointer()),
            })
        );
    }

    let initial = product.initial().into_iter().next().expect("initial state");
    let source_step = product
        .source_steps(&initial)
        .expect("enabled source transitions")
        .into_iter()
        .next()
        .expect("the task product has a source transition");
    let model_alphabet = TemporalCounterexample::<ModelTransition> {
        initial_state: 0,
        prefix: vec![ModelStep {
            transition: source_step.source.transition().clone(),
            post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(
                product
                    .state_key(&source_step.state)
                    .expect("source successor key"),
            ),
            delay: Some(crate::zone_search::exact(0, 1)),
        }],
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    assert_eq!(
        fsl_replay::replay_task_product_trace(&product, &model_alphabet),
        Err(fsl_replay::TaskProductTraceReplayRefusal::TraceAlphabet {
            pointer: "/prefix/0/transition/type".to_owned(),
        })
    );
}

/// FR-249-AC-11/FR-237: a quiescent product-terminal miss may use one
/// positive-delay terminal-stutter loop, with the delay applied exactly once.
#[trace("TC-704", "FR-249-AC-11")]
#[test]
fn terminal_stutter_replay_advances_once_from_a_quiescent_product_terminal() {
    let document = document(&[], &[("go", &["phase"])]);
    let body = format!(
        "{DENSE}\
         pre GoReady using v on M::Call::go {{ self.phase = 0 }}\n\
         post GoFired using v on M::Call::go {{ self.phase = 1 }}\n\
         taskset Once {{ task s sporadic 10 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo; }}\n\
         release Once.s on M::Call::go;\n\
         schedulable OnceClaim on Once under fixed_priority;\n"
    );
    let world = world(&document, crate::zone_search::TIMED, &body);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "OnceClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("source release is checked")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("go activates the task");
    let dispatch = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("the released job dispatches");
    let at_deadline = product
        .delay(&dispatch.state, &crate::zone_search::exact(2, 1))
        .expect("the source admits the deadline delay")
        .expect("the deadline is reachable");
    let miss = product
        .scheduler_steps(&at_deadline)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::DeadlineMiss)
        .expect("the first deadline is missed");
    let at_completion = product
        .delay(&miss.state, &crate::zone_search::exact(1, 1))
        .expect("time remains admissible after the miss")
        .expect("the job can complete");
    let complete = product
        .scheduler_steps(&at_completion)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Complete)
        .expect("the missed job eventually completes");
    let trace = TaskProductTrace {
        initial_state: 0,
        steps: vec![
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: activation.transitions()[0].clone(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(0, 1),
                transition: dispatch.transition(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(2, 1),
                transition: miss.transition(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(1, 1),
                transition: complete.transition(),
            },
            TaskProductTraceStep {
                delay: crate::zone_search::exact(1, 1),
                transition: TaskProductTransition::TerminalStutter,
            },
        ],
        lasso_entry: Some(4),
    };
    let states = product
        .replay_trace_states(&trace)
        .expect("the terminal loop replays with one positive delay");
    assert_eq!(states[4].now(), &crate::zone_search::exact(3, 1));
    assert_eq!(states[5].now(), &crate::zone_search::exact(4, 1));
    assert!(states[5].deadline_missed());
    assert!(states[5].jobs().is_empty());

    let mut zero_delay = trace;
    zero_delay.steps[4].delay = crate::zone_search::exact(0, 1);
    assert!(product.replay_trace_states(&zero_delay).is_err());
}

/// FR-419-AC-14/FR-364-AC-21: a checked mode operation grants only a live
/// HI job's budget, and replay derives the same phase and source identity.
#[trace("TC-704", "FR-249-AC-14", "FR-237-AC-9")]
#[test]
fn mode_switch_emits_and_replays_the_live_hi_budget_change() {
    let world = world_with_hi_mode_switch("reply");
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("task releases are concrete")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command");
    let dispatch = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("the HI job dispatches in LO mode");
    let at_low_allowance = product
        .delay(&dispatch.state, &crate::zone_search::exact(2, 1))
        .expect("the source admits time through C(LO)")
        .expect("the exact allowance boundary is reachable");
    assert_eq!(
        at_low_allowance.jobs()[0].service,
        crate::zone_search::exact(2, 1)
    );
    let exhaustion = product
        .scheduler_steps(&at_low_allowance)
        .into_iter()
        .next()
        .expect("C(LO) exhaustion preempts the running HI job");
    assert_eq!(exhaustion.identity.action, TaskTransitionAction::Preempt);
    assert_eq!(exhaustion.identity.source_transition, None);
    let at_reply = product
        .delay(&dispatch.state, &crate::zone_search::exact(1, 1))
        .expect("the dense source admits one millisecond")
        .expect("reply boundary is reachable");
    let mode_switch = product
        .source_steps(&at_reply)
        .expect("the checked mode transition is admitted")
        .into_iter()
        .find(|step| step.activations.is_empty())
        .expect("reply enters HI mode");
    assert_eq!(
        mode_switch.state.criticality_mode(),
        fsl_eval::simulation::model::CriticalityMode::Hi
    );
    let budget_change = product
        .scheduler_steps(&mode_switch.state)
        .into_iter()
        .next()
        .expect("a live HI job receives an urgent budget change");
    assert_eq!(
        budget_change.identity.action,
        TaskTransitionAction::BudgetChange
    );
    assert_eq!(budget_change.identity.task, "command");
    assert_eq!(
        budget_change.identity.budget_delta,
        Some(crate::zone_search::exact(1, 1))
    );
    assert_eq!(
        budget_change.identity.source_transition,
        Some(mode_switch.source.transition().clone())
    );
    assert_eq!(
        budget_change.state.jobs()[0].allowance_phase,
        fsl_eval::simulation::model::AllowancePhase::Hi
    );
    assert_ne!(
        product
            .state_key(&mode_switch.state)
            .expect("LO-phase state has a finite key"),
        product
            .state_key(&budget_change.state)
            .expect("HI-phase state has a finite key")
    );
    let second_budget_change = product
        .scheduler_steps(&budget_change.state)
        .into_iter()
        .next()
        .expect("the second preexisting HI job changes budget next");
    assert_eq!(
        second_budget_change.identity.action,
        TaskTransitionAction::BudgetChange
    );
    assert_eq!(second_budget_change.identity.task, "worker");
    assert_eq!(
        second_budget_change.identity.source_transition,
        Some(mode_switch.source.transition().clone())
    );
    let mut trace_steps: Vec<_> = activation
        .transitions()
        .into_iter()
        .map(|transition| TaskProductTraceStep {
            delay: crate::zone_search::exact(0, 1),
            transition,
        })
        .collect();
    trace_steps.extend([
        TaskProductTraceStep {
            delay: crate::zone_search::exact(0, 1),
            transition: dispatch.transition(),
        },
        TaskProductTraceStep {
            delay: crate::zone_search::exact(1, 1),
            transition: mode_switch.transitions()[0].clone(),
        },
        TaskProductTraceStep {
            delay: crate::zone_search::exact(0, 1),
            transition: budget_change.transition(),
        },
        TaskProductTraceStep {
            delay: crate::zone_search::exact(0, 1),
            transition: second_budget_change.transition(),
        },
    ]);
    let trace = TaskProductTrace {
        initial_state: 0,
        lasso_entry: None,
        steps: trace_steps,
    };
    let replayed = product
        .replay_trace(&trace)
        .expect("FR-237 derives the mode and budget phase from checked steps");
    assert_eq!(
        replayed.jobs()[0].allowance_phase,
        fsl_eval::simulation::model::AllowancePhase::Hi
    );
    assert_eq!(
        replayed.jobs()[1].allowance_phase,
        fsl_eval::simulation::model::AllowancePhase::Hi
    );
}

/// FR-249-AC-14: reversing release declaration order does not change the
/// task/activation/occurrence order of atomic budget changes.
#[trace("TC-704", "FR-249-AC-14")]
#[test]
fn budget_changes_keep_task_order_when_release_declarations_reverse() {
    let world = world_with_hi_mode_switch_order(Some("reply"), true);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("task releases are concrete")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates both tasks");
    assert_eq!(
        activation
            .activations
            .iter()
            .map(|identity| identity.task.as_str())
            .collect::<Vec<_>>(),
        ["command", "worker"]
    );
    let dispatch = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("the HI job dispatches in LO mode");
    let at_reply = product
        .delay(&dispatch.state, &crate::zone_search::exact(1, 1))
        .expect("the dense source admits one millisecond")
        .expect("reply boundary is reachable");
    let mode_switch = product
        .source_steps(&at_reply)
        .expect("the checked mode transition is admitted")
        .into_iter()
        .find(|step| step.activations.is_empty())
        .expect("reply enters HI mode");
    let first = product
        .scheduler_steps(&mode_switch.state)
        .into_iter()
        .next()
        .expect("first urgent budget change");
    let second = product
        .scheduler_steps(&first.state)
        .into_iter()
        .next()
        .expect("second urgent budget change");
    assert_eq!(first.identity.action, TaskTransitionAction::BudgetChange);
    assert_eq!(first.identity.task, "command");
    assert_eq!(second.identity.action, TaskTransitionAction::BudgetChange);
    assert_eq!(second.identity.task, "worker");
}

/// FR-419-AC-15: a HI job activated on the mode-entry operation starts with
/// its HI allowance and does not receive a second budget-change step.
#[trace("TC-704", "FR-249-AC-15")]
#[test]
fn hi_activation_on_mode_entry_starts_in_hi_phase() {
    let world = world_with_hi_mode_switch("send");
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("the checked mode transition is admitted")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command in HI mode");
    assert_eq!(
        activation.state.criticality_mode(),
        fsl_eval::simulation::model::CriticalityMode::Hi
    );
    assert_eq!(activation.activations.len(), 2);
    assert_eq!(
        activation
            .state
            .jobs()
            .iter()
            .map(|job| job.allowance_phase)
            .collect::<Vec<_>>(),
        [
            fsl_eval::simulation::model::AllowancePhase::Hi,
            fsl_eval::simulation::model::AllowancePhase::Hi
        ]
    );
    assert!(
        product
            .scheduler_steps(&activation.state)
            .iter()
            .all(|step| step.identity.action != TaskTransitionAction::BudgetChange)
    );
}

/// FR-249-AC-15: a source operation does not enter HI or change a HI job's
/// allowance when the checked package has no mode declaration.
#[trace("TC-704", "FR-249-AC-15")]
#[test]
fn no_mode_declaration_never_emits_budget_changes() {
    let world = world_with_hi_mode_switch_order(None, false);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    assert!(product.plan().mode_switches.is_empty());
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("task releases are concrete")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates both tasks");
    let dispatch = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("the HI job dispatches in LO mode");
    let at_reply = product
        .delay(&dispatch.state, &crate::zone_search::exact(1, 1))
        .expect("the dense source admits one millisecond")
        .expect("reply boundary is reachable");
    let source_step = product
        .source_steps(&at_reply)
        .expect("ordinary source steps remain enabled")
        .into_iter()
        .find(|step| step.activations.is_empty())
        .expect("reply has no task release");
    assert_eq!(
        source_step.state.criticality_mode(),
        fsl_eval::simulation::model::CriticalityMode::Lo
    );
    assert!(
        product
            .scheduler_steps(&source_step.state)
            .iter()
            .all(|step| step.identity.action != TaskTransitionAction::BudgetChange)
    );
}

/// FR-249-AC-1: S3's checked task declarations lower into the timed product
/// with each task's declared arrival period and relative deadline intact.
#[trace("FR-249-AC-1")]
#[test]
fn task_product_lowering_preserves_each_period_and_deadline() {
    let world = world_with_two_releases();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");

    let rows = TaskQueueBound::from_checked_task_set(product.plan().task_set)
        .expect("checked tasks fit the FR-418 queue schema");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].task, "alpha");
    assert_eq!(rows[0].interarrival, crate::zone_search::exact(4, 1));
    assert_eq!(rows[0].relative_deadline, crate::zone_search::exact(2, 1));
    assert_eq!(rows[1].task, "beta");
    assert_eq!(rows[1].interarrival, crate::zone_search::exact(4, 1));
    assert_eq!(rows[1].relative_deadline, crate::zone_search::exact(2, 1));
    assert_eq!(product.plan().tasks.len(), 2);
    assert_eq!(product.plan().releases.len(), 2);
}

/// FR-249-AC-1: source clocks above every checked guard/invariant constant
/// have identical task-product state identity, so unconstrained time does
/// not make the bounded event search infinite.
#[trace("FR-249-AC-1")]
#[test]
fn task_product_key_quotients_source_clocks_above_checked_bounds() {
    let world = world_with_task(1, 2);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let at_four = product
        .delay(&initial, &crate::zone_search::exact(4, 1))
        .expect("source admits time outside its active invariant")
        .expect("the source remains live");
    let at_ten = product
        .delay(&initial, &crate::zone_search::exact(10, 1))
        .expect("source admits unconstrained time")
        .expect("the source remains live");
    assert_eq!(
        product.state_key(&at_four).expect("state key"),
        product.state_key(&at_ten).expect("state key")
    );
    assert_eq!(
        product.lasso_state_key(&at_four).expect("lasso key"),
        product.lasso_state_key(&at_ten).expect("lasso key")
    );
    let one_more = crate::zone_search::exact(1, 1);
    let after_four = product
        .delay(&at_four, &one_more)
        .expect("source admits a successor delay beyond its checked bound")
        .expect("the source remains live");
    let after_ten = product
        .delay(&at_ten, &one_more)
        .expect("source admits the equivalent successor delay")
        .expect("the source remains live");
    assert_eq!(
        product.state_key(&after_four).expect("successor key"),
        product.state_key(&after_ten).expect("successor key")
    );
    let successors = |state: &fsl_eval::simulation::model::TaskProductState| {
        product
            .source_steps(state)
            .expect("enabled source steps")
            .iter()
            .map(|step| {
                let transitions = step
                    .transitions()
                    .into_iter()
                    .map(|mut transition| {
                        if let fsl_eval::simulation::model::TaskProductTransition::Task(identity) =
                            &mut transition
                        {
                            // Absolute deadlines are retained in replay
                            // provenance and shift with absolute task time;
                            // compare their observable relative value here.
                            identity.deadline = identity.deadline.sub(state.now());
                        }
                        transition
                    })
                    .collect::<Vec<_>>();
                (
                    transitions,
                    step.unresolved_demand,
                    product.state_key(&step.state).expect("successor key"),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(successors(&at_four), successors(&at_ten));
    assert_eq!(at_four.deadline_missed(), at_ten.deadline_missed());

    // Values below the maximum remain exact. In particular, the key does
    // not collapse fractional clock values or erase their ordering, which
    // guards and invariants may still observe.
    let at_one_third = product
        .delay(&initial, &crate::zone_search::exact(1, 3))
        .expect("source admits fractional time below its bound")
        .expect("the source remains live");
    let at_two_thirds = product
        .delay(&initial, &crate::zone_search::exact(2, 3))
        .expect("source admits a later fractional time below its bound")
        .expect("the source remains live");
    let one_third_key = product.state_key(&at_one_third).expect("state key");
    let two_thirds_key = product.state_key(&at_two_thirds).expect("state key");
    assert_ne!(one_third_key, two_thirds_key);
    assert!(one_third_key < two_thirds_key);
    assert_ne!(
        product.lasso_state_key(&at_one_third).expect("lasso key"),
        product.lasso_state_key(&at_two_thirds).expect("lasso key")
    );

    // Task outcomes and FR-418 certificate inputs are computed from the
    // concrete task state and checked plan; they do not consume this source
    // clock key. The successor assertion above checks runtime observations
    // represented by the quotiented source-clock component.
}

/// FR-249-AC-2: a concrete endpoint demand remains attached to the job across
/// service and scheduler transitions, while the search retains an unresolved
/// frontier for the rest of the admitted interval.
#[trace("FR-249-AC-2")]
#[test]
fn interval_work_is_replayed_as_a_concrete_choice_and_stays_incomplete() {
    let world = world_with_task_bounds(1, 2, 4);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let activation = product
        .source_steps(&initial)
        .expect("bounded demand endpoint is admitted")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("send activates command");
    assert!(activation.unresolved_demand);
    assert_eq!(
        activation.activations[0].demand,
        crate::zone_search::exact(2, 1)
    );
    let job = activation.state.jobs().first().expect("activated job");
    assert_eq!(job.demand, crate::zone_search::exact(2, 1));
    let dispatched = product
        .scheduler_steps(&activation.state)
        .into_iter()
        .find(|step| step.identity.action == TaskTransitionAction::Dispatch)
        .expect("endpoint job dispatches");
    let after_service = product
        .delay(&dispatched.state, &crate::zone_search::exact(1, 1))
        .expect("source admits service time")
        .expect("one unit of service is admissible");
    assert_eq!(
        after_service.jobs()[0].demand,
        crate::zone_search::exact(2, 1)
    );

    let outcome = check_task_product(&product, TaskProductSearchLimits { max_work: 64 })
        .expect("interval demand remains admitted");
    assert!(matches!(
        &outcome,
        TaskProductSearchOutcome::Stopped {
            frontier,
            ..
        } if frontier.iter().any(|entry| entry.stop == TaskProductSearchStop::UnresolvedChoice)
    ));
    let TaskProductSearchOutcome::Stopped { frontier, .. } = outcome else {
        unreachable!("interval demand keeps a frontier");
    };
    let unresolved = frontier
        .iter()
        .find(|entry| entry.stop == TaskProductSearchStop::UnresolvedChoice)
        .expect("unresolved demand has a reachable frontier");
    let replayed = product
        .replay_trace_states(&unresolved.trace)
        .expect("the retained prefix replays");
    assert_eq!(
        product
            .state_key(replayed.last().expect("initial state"))
            .expect("frontier key"),
        unresolved.state_key
    );
}

/// FSpec FR-364-AC-23's `Tick` fixture: one operation `tick` releases tasks
/// `b` and `a`, declared in `release_order`.
fn tick_world(release_order: &[&str]) -> crate::timed_world::World {
    let document = document(&[], &[("tick", &["phase"])]);
    let releases: String = release_order
        .iter()
        .map(|task| format!("release Two.{task} on M::Call::tick;\n"))
        .collect();
    let body = format!(
        "{DENSE}\
         pre TickReady using v on M::Call::tick {{ self.phase = 0 }}\n\
         post TickFired using v on M::Call::tick {{ self.phase = 1 }}\n\
         taskset Two {{ task a sporadic 10 ms deadline 2 ms bcet 3 ms wcet 3 ms priority 1 lo; \
         task b sporadic 10 ms deadline 10 ms bcet 1 ms wcet 1 ms priority 2 lo; }}\n\
         {releases}\
         schedulable TwoFp on Two under fixed_priority;\n"
    );
    world(&document, crate::zone_search::TIMED, &body)
}

/// The typed FR-364 counterexample of `product`'s earliest refutation: the
/// terminal-stutter step is carried only by its lasso, never in the prefix.
fn refutation_of(
    product: &TaskProductSystem<'_, '_>,
) -> TemporalCounterexample<fsl_replay::TaskProductTransition> {
    let TaskProductSearchOutcome::Refuted(refutation) =
        check_task_product(product, TaskProductSearchLimits { max_work: 4096 })
            .expect("task-product search is admitted")
    else {
        panic!("the product refutes");
    };
    let trace = refutation.trace;
    let states = product.replay_trace_states(&trace).expect("trace replays");
    let stutter = trace.steps.last().and_then(|step| {
        matches!(step.transition, TaskProductTransition::TerminalStutter)
            .then(|| step.delay.clone())
    });
    let kept = trace.steps.len() - usize::from(stutter.is_some());
    TemporalCounterexample {
        initial_state: trace.initial_state,
        prefix: trace.steps[..kept]
            .iter()
            .zip(states.iter().skip(1))
            .map(|(step, state)| ModelStep {
                transition: step.transition.clone().into(),
                post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(
                    product.state_key(state).expect("replayed state key"),
                ),
                delay: Some(step.delay.clone()),
            })
            .collect(),
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: trace.lasso_entry.map(|entry| Lasso {
            entry,
            terminal_stutter: stutter.is_some(),
            terminal_stutter_delay: stutter,
        }),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    }
}

fn refusal_pointer(
    product: &TaskProductSystem<'_, '_>,
    counterexample: &TemporalCounterexample<fsl_replay::TaskProductTransition>,
) -> (String, String, String) {
    match fsl_replay::replay_task_product_trace(product, counterexample) {
        Err(fsl_replay::TaskProductTraceReplayRefusal::InvalidRuntimeInput {
            pointer,
            supplied,
            recomputed,
        }) => (pointer, supplied, recomputed),
        other => panic!("expected an invalid-value refusal, got {other:?}"),
    }
}

fn task_step(
    counterexample: &mut TemporalCounterexample<fsl_replay::TaskProductTransition>,
    index: usize,
) -> &mut fsl_replay::TaskTransitionIdentity {
    match &mut counterexample.prefix[index].transition {
        fsl_replay::TaskProductTransition::Task(identity) => identity,
        other => panic!("step {index} is not a task step: {other:?}"),
    }
}

/// FR-249-AC-9 and FSpec FR-364-AC-23: a move that releases two tasks is
/// written as its `activate` steps in task-name order whatever the release
/// declaration order, so reversing the `Tick` declarations changes no step.
#[trace("TC-704", "FR-249-AC-9", "FR-237-AC-12")]
#[test]
fn reversed_release_declarations_write_the_same_synchronized_steps() {
    let steps_for = |order: &[&str]| {
        let world = tick_world(order);
        let model = world.system();
        let timed = TimedSystem::new(&model, "M").expect("dense timed model");
        let product = TaskProductSystem::new(timed, "TwoFp").expect("checked task product");
        refutation_of(&product)
    };
    let declared = steps_for(&["b", "a"]);
    let reversed = steps_for(&["a", "b"]);
    let tasks: Vec<_> = declared
        .prefix
        .iter()
        .take(2)
        .map(|step| match &step.transition {
            fsl_replay::TaskProductTransition::Task(identity) => {
                (identity.action, identity.task.clone())
            }
            other => panic!("the move is written as activations: {other:?}"),
        })
        .collect();
    assert_eq!(
        tasks,
        [
            (fsl_replay::TaskTransitionAction::Activate, "a".to_owned()),
            (fsl_replay::TaskTransitionAction::Activate, "b".to_owned())
        ]
    );
    assert_eq!(declared.prefix.len(), reversed.prefix.len());
    for (left, right) in declared.prefix.iter().zip(&reversed.prefix) {
        assert_eq!(left.delay, right.delay);
        match (&left.transition, &right.transition) {
            (
                fsl_replay::TaskProductTransition::Task(left),
                fsl_replay::TaskProductTransition::Task(right),
            ) => {
                assert_eq!(
                    (
                        left.action,
                        &left.task,
                        left.job,
                        &left.deadline,
                        &left.demand
                    ),
                    (
                        right.action,
                        &right.task,
                        right.job,
                        &right.deadline,
                        &right.demand
                    )
                );
            }
            (left, right) => assert_eq!(left, right),
        }
    }
}

/// FR-237-AC-12 and FSpec FR-364-AC-23/24: each tamper of the `Tick`
/// refutation refuses `invalid-value` at its member, naming the supplied and
/// the recomputed value; the untampered counterexample replays, with its
/// terminal stutter carried only by the lasso.
#[trace("TC-704", "FR-237-AC-12")]
#[test]
fn tampered_task_steps_refuse_at_their_member() {
    let world = tick_world(&["b", "a"]);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TwoFp").expect("checked task product");
    let original = refutation_of(&product);
    assert!(
        original
            .prefix
            .iter()
            .all(|step| step.transition != fsl_replay::TaskProductTransition::TerminalStutter)
    );
    fsl_replay::replay_task_product_trace(&product, &original)
        .expect("the untampered counterexample replays");

    let miss = original
        .prefix
        .iter()
        .position(|step| {
            matches!(
                &step.transition,
                fsl_replay::TaskProductTransition::Task(identity)
                    if identity.action == fsl_replay::TaskTransitionAction::DeadlineMiss
            )
        })
        .expect("the miss step");
    let member =
        |mutate: &dyn Fn(&mut TemporalCounterexample<fsl_replay::TaskProductTransition>)| {
            let mut tampered = original.clone();
            mutate(&mut tampered);
            refusal_pointer(&product, &tampered)
        };

    let (pointer, supplied, recomputed) = member(&|found| task_step(found, miss).slot += 1);
    assert_eq!(pointer, format!("/prefix/{miss}/transition/slot"));
    assert_ne!(supplied, recomputed);
    assert_eq!(
        member(&|found| task_step(found, 0).generation = 1).0,
        "/prefix/0/transition/generation"
    );
    assert_eq!(
        member(&|found| task_step(found, 0).task = "t9".to_owned()).0,
        "/prefix/0/transition/task"
    );
    assert_eq!(
        member(&|found| task_step(found, 0).source_transition = None).0,
        "/prefix/0/transition/source_transition"
    );
    assert_eq!(
        member(&|found| task_step(found, miss).deadline = crate::zone_search::exact(3, 1)).0,
        format!("/prefix/{miss}/transition/deadline")
    );
    assert_eq!(
        member(&|found| {
            found.prefix[0].post_state = fsl_eval::simulation::StateKey::from_canonical_bytes(
                product
                    .state_key(&product.initial()[0])
                    .expect("initial key"),
            );
        })
        .0,
        "/prefix/0/post_state"
    );
    // The move's activations swapped, the second removed, and a delay on the
    // second step of the move.
    assert_eq!(
        member(&|found| found.prefix.swap(0, 1)).0,
        "/prefix/0/transition/task"
    );
    assert_eq!(
        member(&|found| {
            found.prefix.remove(1);
        })
        .0,
        "/prefix/1/transition"
    );
    assert_eq!(
        member(&|found| found.prefix[1].delay = Some(crate::zone_search::exact(1, 1))).0,
        "/prefix/1/delay"
    );
    // The move written as its source `tick` step is not the activation.
    let tick = product
        .source_steps(&product.initial()[0])
        .expect("source steps")
        .into_iter()
        .find(|step| !step.activations.is_empty())
        .expect("tick activates");
    assert_eq!(
        member(&|found| {
            found.prefix[0].transition =
                fsl_replay::TaskProductTransition::Source(tick.source.transition().clone());
        })
        .0,
        "/prefix/0/transition/type"
    );
    // A zero-delay terminal stutter is refused at the loop's delay.
    assert_eq!(
        member(&|found| {
            if let Some(lasso) = found.lasso.as_mut() {
                lasso.terminal_stutter_delay = Some(crate::zone_search::exact(0, 1));
            }
        })
        .0,
        "/loop/0/delay"
    );
}

/// FR-237-AC-11: the source projection of a task trace, passed as a typed
/// `TemporalCounterexample<ModelTransition>` to `replay_model_trace` for the
/// task item, refuses `ModelTraceRefusal::TraceAlphabet` before any step
/// replays, although each projected step is enabled in the source model.
#[trace("TC-704", "FR-237-AC-11")]
#[test]
fn a_model_alphabet_payload_for_a_task_item_refuses_trace_alphabet() {
    let world = world_with_task(2, 1);
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "TaskClaim").expect("checked task product");
    let initial = product.initial().into_iter().next().expect("initial state");
    let source_step = product
        .source_steps(&initial)
        .expect("enabled source transitions")
        .into_iter()
        .next()
        .expect("the task product has a source transition");
    let projection = TemporalCounterexample::<ModelTransition> {
        initial_state: 0,
        prefix: vec![ModelStep {
            transition: source_step.source.transition().clone(),
            post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(
                product
                    .state_key(&source_step.state)
                    .expect("source successor key"),
            ),
            delay: Some(crate::zone_search::exact(0, 1)),
        }],
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: Vec::new(),
    };
    let claim = &world.package.graph().task_sets().claims()[0];
    let occurrence = claim.occurrence().expect("the claim has an occurrence");
    let request = world.request();
    let refused = fsl_replay::replay_model_trace(
        crate::model_settle::request(
            &world.package,
            &[],
            &world.document,
            world.snapshots.iter(),
            &world.unit,
        ),
        &fsl_replay::ModelTraceSubject {
            timed: Some(fsl_replay::TimedReplay {
                model: "M".to_owned(),
                exploration: fsl_eval::simulation::Limits::default(),
                max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
                digital: false,
            }),
            initial: request.initial.clone(),
            universes: request.universes.clone(),
            expansion: request.expansion,
            stutter_closure: false,
        },
        &crate::model_settle::envelope_at(
            projection,
            claim.node_id().expect("the claim has a node").clone(),
            occurrence,
        ),
        &crate::counter::Letters { atom: |_| None },
        fsl_replay::ReplayLimits::default(),
    );
    match refused {
        Err(fsl_replay::ReplayRefusal::ModelTrace(refusal)) => assert!(
            matches!(
                *refusal,
                fsl_replay::ModelTraceRefusal::TraceAlphabet {
                    required: "task",
                    supplied: "model"
                }
            ),
            "{refusal:?}"
        ),
        other => panic!("expected TraceAlphabet, got {other:?}"),
    }
}
