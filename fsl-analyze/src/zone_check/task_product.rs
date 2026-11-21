// SPDX-License-Identifier: AGPL-3.0-or-later
//! The dedicated EN-6 bounded search seam for a checked task-derived timed
//! subject. Source-only `TimedSystem` searches remain in `search.rs`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Cancel, CancelCause, Rational};
use fsl_eval::simulation::model::{
    TaskProductMoveRefusal, TaskProductState, TaskProductStateKeyRefusal, TaskProductSystem,
    TaskProductTrace, TaskProductTraceStep,
};

/// The effective work budget for one task-product search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskProductSearchLimits {
    /// Product nodes expanded, including source and scheduler successor
    /// enumeration.
    pub max_work: u64,
}

/// Why the dedicated task-product search returned no settled result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskProductSearchStop {
    /// The configured task-product expansion budget was reached.
    WorkLimit(u64),
    /// A continuous demand or jitter choice needs bounded path selection.
    UnresolvedChoice,
    /// A due task release cannot be materialized because every candidate
    /// slot for that task is occupied. This is an open frontier, never a
    /// concrete overflow transition or a refutation.
    QueueOverflow,
    /// The explored event graph closed without proving finite product
    /// closure under the FR-418 refined-region relation.
    RegionClosureRequired,
    /// The caller cancelled this search.
    Cancelled(CancelCause),
}

/// An unexpanded task-product node retained by the bounded search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductSearchFrontier {
    /// The finite source-plus-scheduler state key.
    pub state_key: Vec<u8>,
    /// The number of task-product transitions from an initial state.
    pub depth: u64,
    /// The concrete transition prefix to replay this reachable node.
    pub trace: TaskProductTrace,
    /// Exact time advanced after the last transition in `trace` to reach this
    /// node. Search states may end between discrete transitions.
    pub final_delay: Rational,
    /// Why the node remains open.
    pub stop: TaskProductSearchStop,
}

/// A product path that reaches a concrete miss and a replayable time-divergent
/// continuation from the miss prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductSearchRefutation {
    /// The exact miss prefix plus a repeated product state, replayed against
    /// the rebuilt task product under FR-237/FR-364.
    pub trace: TaskProductTrace,
}

/// The result of the dedicated task-product search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskProductSearchOutcome {
    /// The product search found a concrete deadline miss and a positive-delay
    /// repeated product key.
    Refuted(TaskProductSearchRefutation),
    /// No checked result could be settled; the frontier is never proof
    /// evidence.
    Stopped {
        /// The reason the bounded route stopped.
        cause: TaskProductSearchStop,
        /// The open nodes retained at the stop.
        frontier: Vec<TaskProductSearchFrontier>,
        /// The number of nodes expanded.
        expanded: u64,
        /// The number of product edges retained.
        transitions: u64,
    },
}

/// Why a task-product request could not enter the dedicated search.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductSearchRefusal {
    /// A product node could not be encoded for coverage.
    #[error("a task-product state has no canonical finite key")]
    StateKey(#[from] TaskProductStateKeyRefusal),
    /// The timed source could not enumerate its exact next event boundaries.
    #[error("the timed source stopped while building task-product successors")]
    TimedSource(fsl_eval::simulation::model::TimedStop),
    /// An internal task-product expansion fault occurred.
    #[error("the task-product search could not enumerate an exact successor")]
    ProductExpansion,
}

struct Pending {
    state: TaskProductState,
    trace: TaskProductTrace,
    delay_since_step: Rational,
    depth: u64,
    lasso_history: Vec<LassoVisit>,
}

#[derive(Clone)]
struct LassoVisit {
    state_key: Vec<u8>,
    now: Rational,
    trace_position: usize,
}

/// Search one task-product subject under a bounded event expansion.
///
/// This provider returns a refutation only when a concrete task path misses a
/// deadline and its trace closes a positive-delay repeated state. Exhaustion
/// or unresolved dynamics always returns `Stopped`; it never proves absence
/// of a miss.
pub fn check_task_product(
    product: &TaskProductSystem<'_, '_>,
    limits: TaskProductSearchLimits,
) -> Result<TaskProductSearchOutcome, TaskProductSearchRefusal> {
    check_task_product_with_cancel(product, limits, &Cancel::new())
}

/// Search one task-product subject, polling the caller's cancellation handle
/// between canonical and general product expansions.
pub fn check_task_product_with_cancel(
    product: &TaskProductSystem<'_, '_>,
    limits: TaskProductSearchLimits,
    cancel: &Cancel,
) -> Result<TaskProductSearchOutcome, TaskProductSearchRefusal> {
    if cancel.poll() {
        return Ok(cancelled(cancel));
    }
    let configured_work = limits.max_work;
    let (preferred_refutation, preferred_work) = if product.plan().has_fixed_work_demand()
        && product
            .plan()
            .task_set
            .tasks
            .iter()
            .all(|task| task.jitter.is_zero())
    {
        canonical_refutation(product, configured_work.min(512), cancel)?
    } else {
        (None, 0)
    };
    if let Some(refutation) = preferred_refutation {
        return Ok(TaskProductSearchOutcome::Refuted(refutation));
    }
    let limits = TaskProductSearchLimits {
        max_work: configured_work.saturating_sub(preferred_work),
    };
    // Process concrete events by absolute time so the first refutation is the
    // earliest deadline miss; within a timestamp, continue one path before
    // branching across every same-time transition ordering.
    let mut queue = BTreeMap::<Rational, VecDeque<Pending>>::new();
    for (initial_state, state) in product.initial().into_iter().enumerate() {
        enqueue(
            &mut queue,
            Pending {
                state,
                trace: TaskProductTrace {
                    initial_state,
                    steps: Vec::new(),
                    lasso_entry: None,
                },
                delay_since_step: zero(),
                depth: 0,
                lasso_history: Vec::new(),
            },
        );
    }
    let mut visited = BTreeSet::new();
    let mut expanded = 0_u64;
    let mut transitions = 0_u64;
    let mut unresolved = Vec::new();
    while let Some(mut pending) = pop_earliest(&mut queue) {
        if cancel.poll() {
            return Ok(cancelled(cancel));
        }
        let queue_cap_blocked = product
            .queue_cap_blocked(&pending.state)
            .map_err(TaskProductSearchRefusal::TimedSource)?;
        // A deadline miss becomes a refutation only once this same path has
        // a replayable positive-time continuation. The miss latch remains in
        // the state while the search follows the product to its loop entry.
        if pending.state.deadline_missed() && pending.delay_since_step.is_zero() {
            // Start a continuation only after all currently enabled source
            // releases have been settled. Otherwise an irrelevant source
            // self-loop can close the lasso before the next periodic release
            // and put that activation in the loop instead of the miss stem.
            let activation_due = match product.source_steps(&pending.state) {
                Ok(steps) => steps.iter().any(|step| !step.activations.is_empty()),
                Err(_) => true,
            };
            if !activation_due && !queue_cap_blocked {
                let lasso_key = product.lasso_state_key(&pending.state)?;
                if let Some(visit) =
                    pending.lasso_history.iter().rev().find(|visit| {
                        visit.state_key == lasso_key && pending.state.now() > &visit.now
                    })
                {
                    pending.trace.lasso_entry = Some(visit.trace_position);
                    return Ok(TaskProductSearchOutcome::Refuted(
                        TaskProductSearchRefutation {
                            trace: pending.trace,
                        },
                    ));
                }
                pending.lasso_history.push(LassoVisit {
                    state_key: lasso_key,
                    now: pending.state.now().clone(),
                    trace_position: pending.trace.steps.len(),
                });
            }
        }
        let key = product.state_key(&pending.state)?;
        if !pending.state.deadline_missed() || pending.delay_since_step.is_zero() {
            // Before the first miss, exact observationally equal product
            // states have identical futures. Keep only their earliest visit;
            // the time-ordered queue therefore retains the earliest possible
            // deadline-miss path while collapsing repeated periodic schedules.
            // After a miss, preserve each path's continuation history until a
            // replayable lasso or queue-growth witness is found.
            if !visited.insert(key.clone()) {
                continue;
            }
        }
        if queue_cap_blocked {
            unresolved.push(TaskProductSearchFrontier {
                state_key: key.clone(),
                depth: pending.depth,
                trace: pending.trace.clone(),
                final_delay: pending.delay_since_step.clone(),
                stop: TaskProductSearchStop::QueueOverflow,
            });
        }
        if expanded >= limits.max_work {
            let frontier = std::iter::once(TaskProductSearchFrontier {
                state_key: key,
                depth: pending.depth,
                trace: pending.trace,
                final_delay: pending.delay_since_step,
                stop: TaskProductSearchStop::WorkLimit(limits.max_work),
            })
            .chain(unresolved)
            .collect();
            return Ok(TaskProductSearchOutcome::Stopped {
                cause: TaskProductSearchStop::WorkLimit(configured_work),
                frontier,
                expanded: preferred_work.saturating_add(expanded),
                transitions,
            });
        }
        expanded += 1;
        match product.source_steps(&pending.state) {
            Ok(steps) => {
                let source_key = product.source().state_key(pending.state.source()).ok();
                let canonical_release = steps
                    .iter()
                    .filter(|step| step.activations.len() == 1)
                    .filter(|step| {
                        source_key.as_ref().is_some_and(|key| {
                            product
                                .source()
                                .state_key(step.state.source())
                                .ok()
                                .as_ref()
                                == Some(key)
                        })
                    })
                    .map(|step| step.activations[0].task.as_str())
                    .min()
                    .map(str::to_owned);
                for step in steps {
                    if step.activations.len() == 1
                        && canonical_release.as_ref().is_some_and(|canonical| {
                            step.activations[0].task.as_str() != canonical
                                && source_key.as_ref().is_some_and(|key| {
                                    product
                                        .source()
                                        .state_key(step.state.source())
                                        .ok()
                                        .as_ref()
                                        == Some(key)
                                })
                        })
                    {
                        // Equivalent self-loop releases commute across task
                        // identities. Keep their declaration order once so
                        // simultaneous releases do not permute the queue.
                        continue;
                    }
                    if step.unresolved_demand {
                        unresolved.push(TaskProductSearchFrontier {
                            state_key: key.clone(),
                            depth: pending.depth,
                            trace: pending.trace.clone(),
                            final_delay: pending.delay_since_step.clone(),
                            stop: TaskProductSearchStop::UnresolvedChoice,
                        });
                    }
                    let identities = step.transitions();
                    if !identities.is_empty() {
                        let mut trace = pending.trace.clone();
                        for (index, identity) in identities.iter().cloned().enumerate() {
                            trace.steps.push(TaskProductTraceStep {
                                delay: if index == 0 {
                                    pending.delay_since_step.clone()
                                } else {
                                    zero()
                                },
                                transition: identity,
                            });
                        }
                        let identity_count = u64::try_from(identities.len()).unwrap_or(u64::MAX);
                        enqueue(
                            &mut queue,
                            Pending {
                                state: step.state.clone(),
                                trace,
                                delay_since_step: zero(),
                                depth: pending.depth.saturating_add(identity_count),
                                lasso_history: pending.lasso_history.clone(),
                            },
                        );
                        transitions = transitions.saturating_add(identity_count);
                    }
                }
            }
            Err(TaskProductMoveRefusal::VariableJitter) => {
                unresolved.push(TaskProductSearchFrontier {
                    state_key: key.clone(),
                    depth: pending.depth,
                    trace: pending.trace.clone(),
                    final_delay: pending.delay_since_step.clone(),
                    stop: TaskProductSearchStop::UnresolvedChoice,
                });
            }
            Err(TaskProductMoveRefusal::OccurrenceOverflow) => {
                return Err(TaskProductSearchRefusal::ProductExpansion);
            }
            Err(TaskProductMoveRefusal::TimedSource(stop)) => {
                return Err(TaskProductSearchRefusal::TimedSource(stop));
            }
        }
        for step in product.scheduler_steps(&pending.state) {
            let mut trace = pending.trace.clone();
            trace.steps.push(TaskProductTraceStep {
                delay: pending.delay_since_step.clone(),
                transition: step.transition(),
            });
            enqueue(
                &mut queue,
                Pending {
                    state: step.state,
                    trace,
                    delay_since_step: zero(),
                    depth: pending.depth.saturating_add(1),
                    lasso_history: pending.lasso_history.clone(),
                },
            );
            transitions = transitions.saturating_add(1);
        }
        let delays = product
            .next_event_delays(&pending.state)
            .map_err(TaskProductSearchRefusal::TimedSource)?;
        for delay in delays {
            let Some(state) = product
                .delay(&pending.state, &delay)
                .map_err(TaskProductSearchRefusal::TimedSource)?
            else {
                continue;
            };
            let pending_delay = pending.delay_since_step.add(&delay);
            enqueue(
                &mut queue,
                Pending {
                    state,
                    trace: pending.trace.clone(),
                    delay_since_step: pending_delay,
                    depth: pending.depth,
                    lasso_history: pending.lasso_history.clone(),
                },
            );
        }
        if pending.state.deadline_missed()
            && !queue_cap_blocked
            && pending.state.jobs().is_empty()
            && product.scheduler_steps(&pending.state).is_empty()
            && product
                .source()
                .steps(pending.state.source())
                .map_err(TaskProductSearchRefusal::TimedSource)?
                .is_empty()
            && product
                .source()
                .is_quiescent(pending.state.source().discrete())
                .map_err(TaskProductSearchRefusal::TimedSource)?
        {
            let next_bound = product
                .next_event_delays(&pending.state)
                .map_err(TaskProductSearchRefusal::TimedSource)?
                .into_iter()
                .min();
            let positive_delay = next_bound
                .and_then(|bound| {
                    bound.div(&Rational::from_integer(filament_exact::Integer::from(
                        2_i64,
                    )))
                })
                .unwrap_or_else(one);
            let entry = pending.trace.steps.len();
            let mut trace = pending.trace;
            trace.steps.push(TaskProductTraceStep {
                delay: pending.delay_since_step.add(&positive_delay),
                transition: fsl_eval::simulation::model::TaskProductTransition::TerminalStutter,
            });
            trace.lasso_entry = Some(entry);
            if product.replay_trace_states(&trace).is_ok() {
                return Ok(TaskProductSearchOutcome::Refuted(
                    TaskProductSearchRefutation { trace },
                ));
            }
        }
    }
    Ok(TaskProductSearchOutcome::Stopped {
        cause: if unresolved
            .iter()
            .any(|frontier| frontier.stop == TaskProductSearchStop::QueueOverflow)
        {
            TaskProductSearchStop::QueueOverflow
        } else if unresolved.is_empty() {
            TaskProductSearchStop::RegionClosureRequired
        } else {
            TaskProductSearchStop::UnresolvedChoice
        },
        frontier: unresolved,
        expanded: preferred_work.saturating_add(expanded),
        transitions,
    })
}

fn cancelled(cancel: &Cancel) -> TaskProductSearchOutcome {
    TaskProductSearchOutcome::Stopped {
        cause: TaskProductSearchStop::Cancelled(cancel.cause().unwrap_or(CancelCause::Requested)),
        frontier: Vec::new(),
        expanded: 0,
        transitions: 0,
    }
}

fn canonical_refutation(
    product: &TaskProductSystem<'_, '_>,
    budget: u64,
    cancel: &Cancel,
) -> Result<(Option<TaskProductSearchRefutation>, u64), TaskProductSearchRefusal> {
    let mut work = 0_u64;
    for (initial_state, mut state) in product.initial().into_iter().enumerate() {
        let mut trace = TaskProductTrace {
            initial_state,
            steps: Vec::new(),
            lasso_entry: None,
        };
        let mut delay_since_step = zero();
        let mut pre_miss = BTreeSet::new();
        let mut lasso_history = Vec::<LassoVisit>::new();
        while work < budget {
            if cancel.poll() {
                return Ok((None, work));
            }
            work = work.saturating_add(1);
            let queue_cap_blocked = product
                .queue_cap_blocked(&state)
                .map_err(TaskProductSearchRefusal::TimedSource)?;
            if state.deadline_missed() && delay_since_step.is_zero() && !queue_cap_blocked {
                let activation_due = product
                    .source_steps(&state)
                    .map_err(|_| TaskProductSearchRefusal::ProductExpansion)?
                    .iter()
                    .any(|step| !step.activations.is_empty());
                if !activation_due {
                    let key = product.lasso_state_key(&state)?;
                    if let Some(visit) = lasso_history
                        .iter()
                        .rev()
                        .find(|visit| visit.state_key == key && state.now() > &visit.now)
                    {
                        trace.lasso_entry = Some(visit.trace_position);
                        return Ok((Some(TaskProductSearchRefutation { trace }), work));
                    }
                    lasso_history.push(LassoVisit {
                        state_key: key,
                        now: state.now().clone(),
                        trace_position: trace.steps.len(),
                    });
                }
            }
            if !state.deadline_missed() {
                let key = product.state_key(&state)?;
                if !pre_miss.insert(key) {
                    break;
                }
            }

            let scheduler_steps = product.scheduler_steps(&state);
            if let Some(step) = scheduler_steps.into_iter().next() {
                trace.steps.push(TaskProductTraceStep {
                    delay: delay_since_step,
                    transition: step.transition(),
                });
                state = step.state;
                delay_since_step = zero();
                continue;
            }

            let source_steps = match product.source_steps(&state) {
                Ok(steps) => steps,
                Err(TaskProductMoveRefusal::VariableJitter) => break,
                Err(TaskProductMoveRefusal::OccurrenceOverflow) => {
                    return Err(TaskProductSearchRefusal::ProductExpansion);
                }
                Err(TaskProductMoveRefusal::TimedSource(stop)) => {
                    return Err(TaskProductSearchRefusal::TimedSource(stop));
                }
            };
            let mut activating = source_steps
                .iter()
                .filter(|step| !step.activations.is_empty())
                .collect::<Vec<_>>();
            activating
                .sort_by(|left, right| left.activations[0].task.cmp(&right.activations[0].task));
            if let Some(step) = activating.first() {
                let identities = step.transitions();
                for (index, identity) in identities.into_iter().enumerate() {
                    trace.steps.push(TaskProductTraceStep {
                        delay: if index == 0 {
                            delay_since_step.clone()
                        } else {
                            zero()
                        },
                        transition: identity,
                    });
                }
                state = step.state.clone();
                delay_since_step = zero();
                continue;
            }

            let mut reached = None;
            for delay in product
                .next_event_delays(&state)
                .map_err(TaskProductSearchRefusal::TimedSource)?
            {
                if let Some(next) = product
                    .delay(&state, &delay)
                    .map_err(TaskProductSearchRefusal::TimedSource)?
                {
                    reached = Some((next, delay));
                    break;
                }
            }
            let Some((next, delay)) = reached else {
                break;
            };
            state = next;
            delay_since_step = delay_since_step.add(&delay);
        }
        if work >= budget {
            break;
        }
    }
    Ok((None, work))
}

fn enqueue(queue: &mut BTreeMap<Rational, VecDeque<Pending>>, pending: Pending) {
    queue
        .entry(pending.state.now().clone())
        .or_default()
        .push_back(pending);
}

fn pop_earliest(queue: &mut BTreeMap<Rational, VecDeque<Pending>>) -> Option<Pending> {
    let time = queue.first_key_value()?.0.clone();
    let pending = queue.get_mut(&time)?.pop_back()?;
    if queue.get(&time).is_some_and(VecDeque::is_empty) {
        queue.remove(&time);
    }
    Some(pending)
}

fn zero() -> Rational {
    Rational::from_integer(filament_exact::Integer::zero())
}

fn one() -> Rational {
    Rational::from_integer(filament_exact::Integer::one())
}
