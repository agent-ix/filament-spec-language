// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-249's checked input lowering for a task-derived timed subject.

use std::collections::BTreeMap;

use filament_exact::{Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::check::{
    Arrival, CheckedScheduleClaim, Criticality, ModeSwitch, Release, SchedulingPolicy, Task,
    TaskSet, TimeSourceKind, Wcet,
};

use super::{GuardedStep, TimedState, TimedStop, TimedSystem};

/// One checked release binding retained by the task-product lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductRelease<'a> {
    /// The checked `release S.t on M::o;` declaration.
    pub release: &'a Release,
    /// The checked task selected by that declaration.
    pub task: &'a Task,
    /// The canonical operation spelling used by timed source transitions.
    pub operation: String,
}

/// One checked mode-switch binding resolved to this timed subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductModeSwitch<'a> {
    /// The checked declaration.
    pub declaration: &'a ModeSwitch,
    /// The canonical operation spelling used by timed source transitions.
    pub operation: String,
}

/// One checked task lowered to a bounded family of product slots. `releases`
/// indexes [`TaskProductPlan::releases`] so multiple source operations share
/// the task's one arrival phase and slot bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductTask {
    /// The task's source-order index in its checked task set.
    pub task_index: usize,
    /// The candidate number of simultaneously live instances, `ceil(D/T)+1`.
    pub max_jobs: u64,
    /// Indices of source operations that activate this task, in declaration
    /// order.
    pub releases: Vec<usize>,
}

/// An auxiliary unit-rate clock introduced by the task-product lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductClock {
    /// The exact name used to associate FR-418 constraints with this clock.
    pub name: String,
    /// The clock's role in the checked task automaton.
    pub kind: TaskProductClockKind,
    /// The task-set source-order index that owns the clock.
    pub task_index: usize,
    /// The finite task slot, for jitter, service and deadline clocks.
    pub slot: Option<u64>,
}

/// The task-product role of an auxiliary clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskProductClockKind {
    /// Nominal activation spacing for one task.
    Arrival,
    /// Selected release delay for one pending job slot.
    Jitter,
    /// Unit-rate execution received by one running job slot.
    Service,
    /// Time since nominal activation for one job slot.
    Deadline,
}

/// An action in the task-product scheduler (FSpec FR-419).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskTransitionAction {
    /// A source transition nominally activates one task job.
    Activate,
    /// The job becomes available after its selected release jitter.
    Release,
    /// The scheduler starts running the job.
    Dispatch,
    /// The scheduler stops the job because a higher-ranked job is ready.
    Preempt,
    /// The scheduler starts a previously preempted job again.
    Resume,
    /// The job has received its selected total demand.
    Complete,
    /// A source mode change increases a live high-criticality job's allowance.
    BudgetChange,
    /// The job's nominal deadline is reached with positive remaining work.
    DeadlineMiss,
}

/// Exact identity and provenance of one derived task-scheduler step
/// (FSpec FR-181/FR-419).
///
/// The identity repeats immutable job fields on each step. `job` is the
/// zero-based occurrence among all nominal activations of the task, regardless
/// of which bound source operation activated it. `slot` and `generation`
/// identify its storage while live. Certificate edges use FR-418's finite
/// template instead.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTransitionIdentity {
    /// The scheduler action.
    pub action: TaskTransitionAction,
    /// The checked task-set name.
    pub task_set: String,
    /// The checked task name.
    pub task: String,
    /// The exact activation occurrence shared across this task's release operations.
    pub job: u64,
    /// The finite slot that stores this live job.
    pub slot: u64,
    /// The number of times this slot has been reused.
    pub generation: u64,
    /// The source operation whose transition activated the job.
    pub release: ModelTransition,
    /// The source transition that enabled this scheduler step, when present.
    pub source_transition: Option<ModelTransition>,
    /// The exact demand selected for this job.
    pub demand: Rational,
    /// The exact release delay selected for this job.
    pub jitter: Rational,
    /// The absolute deadline measured from nominal activation.
    pub deadline: Rational,
    /// The declared priority, absent when the task has none.
    pub priority: Option<u64>,
    /// The exact high-mode allowance increase on a budget-change step only.
    pub budget_delta: Option<Rational>,
}

impl crate::simulation::SubjectTransition for TaskTransitionIdentity {
    type Fairness = filament_semantic_value::fairness::FairnessConstraint;
}

/// A transition on the lowered task-product subject, preserving the existing
/// source-transition and FR-364 task-transition wire arms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskProductTransition {
    /// A source transition that activates no checked task.
    Source(ModelTransition),
    /// A source-synchronized or autonomous scheduler action.
    Task(TaskTransitionIdentity),
    /// The timed source is quiescent and lets time elapse forever without a
    /// discrete source or scheduler step (FR-181 terminal stutter).
    TerminalStutter,
}

impl crate::simulation::SubjectTransition for TaskProductTransition {
    type Fairness = filament_semantic_value::fairness::FairnessConstraint;

    fn formula_lasso_trace_position(prefix: &[Self]) -> Option<u64> {
        prefix
            .iter()
            .position(|step| {
                matches!(
                    step,
                    Self::Task(identity)
                        if identity.action == TaskTransitionAction::DeadlineMiss
                )
            })
            .and_then(|position| u64::try_from(position).ok())
    }
}

/// Checked task-set inputs joined to one timed model for FR-249 lowering.
///
/// This is the typed boundary between S3's checked declarations and EN-6's
/// derived subject. Release declarations remain in source order; the plan
/// carries the selected checked policy and no caller-supplied scheduler
/// semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductPlan<'a> {
    /// The selected checked task set.
    pub task_set: &'a TaskSet,
    /// The exact checked claim whose identity selects the policy.
    pub claim: &'a CheckedScheduleClaim,
    /// The policy derived from the checked claim.
    pub policy: SchedulingPolicy,
    /// The model alias whose timed transitions drive this product.
    pub model: String,
    /// The selected task set's source release bindings, in source order.
    pub releases: Vec<TaskProductRelease<'a>>,
    /// Checked high-mode switches for this source model, in declaration order.
    pub mode_switches: Vec<TaskProductModeSwitch<'a>>,
    /// The finite per-task slot families used by FR-249 lowering.
    pub tasks: Vec<TaskProductTask>,
}

/// A selected task claim lowered over an existing dense timed model.
///
/// The wrapper keeps the source timed subject and checked task plan together
/// so EN-6 and replay rebuild from the same model, task set and policy.
pub struct TaskProductSystem<'s, 'a> {
    source: TimedSystem<'s, 'a>,
    plan: TaskProductPlan<'a>,
    source_clock_maxima: Vec<Rational>,
}

/// One live task instance in the concrete task-product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductJob {
    /// The task's source-order index in its checked task set.
    pub task_index: usize,
    /// The bounded storage slot occupied by this instance.
    pub slot: u64,
    /// The number of times this slot was reused.
    pub generation: u64,
    /// The zero-based activation occurrence for this task.
    pub occurrence: u64,
    /// Source operation whose checked release activated this job.
    pub release: ModelTransition,
    /// The exact selected demand retained by this job.
    pub demand: Rational,
    /// The selected release jitter retained by this job.
    pub jitter: Rational,
    /// Absolute deadline measured from nominal activation.
    pub deadline: Rational,
    /// Nominal activation time.
    pub activated_at: Rational,
    /// Work already received from the processor.
    pub service: Rational,
    /// Whether the job is currently available to the scheduler.
    pub ready: bool,
    /// The execution allowance available to this live job.
    pub allowance_phase: AllowancePhase,
}

/// Which execution budget a HI task job may consume.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllowancePhase {
    /// The job may execute through `C(LO)` only.
    Lo,
    /// The high-mode switch granted the job `C(HI)`.
    Hi,
}

/// The task set's monotone criticality mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CriticalityMode {
    /// Initial mode.
    Lo,
    /// Entered on the checked high-mode source operation.
    Hi,
}

#[derive(Clone, Debug)]
struct PendingBudgetChange {
    task_index: usize,
    slot: u64,
    generation: u64,
    occurrence: u64,
    source_transition: ModelTransition,
}

/// A concrete source-plus-scheduler state for task-product replay.
#[derive(Clone, Debug)]
pub struct TaskProductState {
    source: TimedState,
    now: Rational,
    arrivals: Vec<Option<Rational>>,
    occurrences: Vec<u64>,
    generations: BTreeMap<(usize, u64), u64>,
    jobs: Vec<TaskProductJob>,
    running: Option<(usize, u64)>,
    missed: bool,
    criticality_mode: CriticalityMode,
    pending_budget_changes: Vec<PendingBudgetChange>,
}

impl TaskProductState {
    /// The underlying timed-model state.
    pub fn source(&self) -> &TimedState {
        &self.source
    }

    /// Each task's last nominal activation time, absent before its first.
    pub fn arrivals(&self) -> &[Option<Rational>] {
        &self.arrivals
    }

    /// Current absolute task-product time.
    pub fn now(&self) -> &Rational {
        &self.now
    }

    /// Live task jobs in activation order.
    pub fn jobs(&self) -> &[TaskProductJob] {
        &self.jobs
    }

    /// The monotone criticality mode derived from checked source declarations.
    pub fn criticality_mode(&self) -> CriticalityMode {
        self.criticality_mode
    }

    /// The running job's task index and finite slot, if any.
    pub fn running(&self) -> Option<(usize, u64)> {
        self.running
    }

    /// Whether a live job has reached its absolute deadline with positive
    /// remaining demand.
    pub fn deadline_missed(&self) -> bool {
        self.missed
    }
}

/// A source transition synchronized with zero or more task activations.
#[derive(Clone, Debug)]
pub struct TaskProductSourceStep {
    /// The source-model transition.
    pub source: GuardedStep,
    /// The resulting concrete task-product state.
    pub state: TaskProductState,
    /// One activation identity per checked release declaration on the source
    /// operation.
    pub activations: Vec<TaskTransitionIdentity>,
    /// The step selected a concrete demand endpoint from an interval. Search
    /// may replay this path, but must retain an unresolved frontier for the
    /// unenumerated values in that interval.
    pub unresolved_demand: bool,
}

impl TaskProductSourceStep {
    /// The transition identities replay records for this synchronized move.
    /// An operation releasing several tasks has one activation identity per
    /// released task; an operation releasing none keeps its source identity.
    pub fn transitions(&self) -> Vec<TaskProductTransition> {
        if self.activations.is_empty() {
            vec![TaskProductTransition::Source(
                self.source.transition().clone(),
            )]
        } else {
            self.activations
                .iter()
                .cloned()
                .map(TaskProductTransition::Task)
                .collect()
        }
    }
}

/// A zero-time scheduler transition in the task-product replay path.
#[derive(Clone, Debug)]
pub struct TaskProductDerivedStep {
    /// The identity and provenance of the derived action.
    pub identity: TaskTransitionIdentity,
    /// The resulting task-product state.
    pub state: TaskProductState,
}

impl TaskProductDerivedStep {
    /// The FR-364 replay transition carried by this scheduler move.
    pub fn transition(&self) -> TaskProductTransition {
        TaskProductTransition::Task(self.identity.clone())
    }
}

/// One exact-delay move in a task-derived trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductTraceStep {
    /// The exact time before the transition.
    pub delay: Rational,
    /// The source or task transition identity.
    pub transition: TaskProductTransition,
}

/// A replayable prefix over a task-product subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductTrace {
    /// The source subject's initial-state index.
    pub initial_state: usize,
    /// The exact-delay transition prefix.
    pub steps: Vec<TaskProductTraceStep>,
    /// The start position of a finite, positive-delay divergent loop, when
    /// the witness uses one.
    pub lasso_entry: Option<usize>,
}

/// Why a task-product trace failed exact replay.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductReplayRefusal {
    /// The source subject has no initial state at the recorded index.
    #[error("the task-product trace names no initial state")]
    InitialState,
    /// A recorded member differs from the one the rebuilt product derives
    /// (FSpec FR-364 "Comparison with the re-derived step").
    #[error("the task-product trace differs from the rebuilt product at {member}")]
    Member {
        /// The recorded step the pointer is relative to; `None` for a
        /// root-level pointer such as `/loop`.
        step: Option<usize>,
        /// The JSON pointer of the differing member, relative to the step.
        member: String,
        /// The supplied value.
        supplied: String,
        /// The value the rebuilt product derives.
        recomputed: String,
    },
    /// The recorded source or scheduler transition is not enabled.
    #[error("the task-product transition is not enabled at its recorded state")]
    Transition,
    /// Source expansion or delay evaluation stopped before replay completed.
    #[error("the timed source stopped while replaying the task product")]
    TimedSource(super::TimedStop),
    /// A source activation selected continuous task data outside the exact
    /// fixed-work, zero-jitter replay slice.
    #[error("the task product has an unresolved demand or jitter choice")]
    UnresolvedChoice,
}

/// Why a concrete task-product state could not be keyed for bounded search.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductStateKeyRefusal {
    /// The source model state has no canonical timed-state key.
    #[error("the timed source state has no canonical key")]
    Source(#[from] crate::simulation::EncodingRefusal),
    /// A checked task transition identity has no canonical wire value.
    #[error("a checked task transition has no canonical identity")]
    TransitionIdentity,
}

/// Why a concrete task-product move could not be expanded exactly.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductMoveRefusal {
    /// A non-zero jitter interval requires a selected concrete jitter value.
    #[error("the task release jitter needs an explicit concrete choice")]
    VariableJitter,
    /// Source-model step expansion stopped before returning successors.
    #[error("the timed source could not enumerate its enabled steps")]
    TimedSource(super::TimedStop),
    /// A task's unbounded activation occurrence counter overflowed.
    #[error("the task activation occurrence counter overflowed")]
    OccurrenceOverflow,
}

impl<'s, 'a> TaskProductSystem<'s, 'a> {
    /// Selects `claim_name` from the checked package and lowers its task
    /// releases over `source`'s dense timed model.
    ///
    /// # Errors
    ///
    /// [`TaskProductPlanRefusal`] when the source is not dense-time or the
    /// checked task claim and release bindings do not resolve to the source.
    pub fn new(
        source: TimedSystem<'s, 'a>,
        claim_name: &str,
    ) -> Result<Self, TaskProductPlanRefusal> {
        let plan = TaskProductPlan::new(&source, claim_name)?;
        let source_clock_maxima = source_clock_maxima(&source);
        Ok(Self {
            source,
            plan,
            source_clock_maxima,
        })
    }

    /// The checked source timed system.
    pub fn source(&self) -> &TimedSystem<'s, 'a> {
        &self.source
    }

    /// The checked task plan and derived scheduler policy.
    pub fn plan(&self) -> &TaskProductPlan<'a> {
        &self.plan
    }

    /// Initial source-plus-scheduler states. Task clocks start at zero and no
    /// task job exists until a checked source release transition occurs.
    pub fn initial(&self) -> Vec<TaskProductState> {
        self.source
            .initial()
            .into_iter()
            .map(|source| TaskProductState {
                source,
                now: task_zero(),
                arrivals: vec![None; self.plan.tasks.len()],
                occurrences: vec![0; self.plan.tasks.len()],
                generations: BTreeMap::new(),
                jobs: Vec::new(),
                running: None,
                missed: false,
                criticality_mode: CriticalityMode::Lo,
                pending_budget_changes: Vec::new(),
            })
            .collect()
    }

    /// Advances source and scheduler time together by an admissible delay.
    /// The currently running job alone receives service; every deadline and
    /// activation phase advances with the unit-rate clocks.
    pub fn delay(
        &self,
        state: &TaskProductState,
        delay: &Rational,
    ) -> Result<Option<TaskProductState>, super::TimedStop> {
        // Scheduler actions are urgent: dispatch, preemption, completion and
        // deadline-miss transitions happen at their enabling instant. Do not
        // let a concrete delay skip an enabled action and invent a miss.
        if !delay.is_zero() && !self.scheduler_steps(state).is_empty() {
            return Ok(None);
        }
        let mut scheduler_boundaries = Vec::new();
        for job in &state.jobs {
            // Once the first miss is latched, deadline transitions are
            // disabled for the rest of this counterexample continuation.
            // Later deadlines therefore do not split or forbid its delays.
            if !state.missed && job.deadline > state.now {
                scheduler_boundaries.push(job.deadline.sub(&state.now));
            }
            let ready_at = job.activated_at.add(&job.jitter);
            if !job.ready && ready_at > state.now {
                scheduler_boundaries.push(ready_at.sub(&state.now));
            }
            if state.running == Some((job.task_index, job.slot)) && job.service < job.demand {
                scheduler_boundaries.push(job.demand.sub(&job.service));
                let task = &self.plan.task_set.tasks[job.task_index];
                if task.criticality == Criticality::Hi && job.allowance_phase == AllowancePhase::Lo
                {
                    if let Wcet::Dual { lo, .. } = &task.wcet {
                        if job.service < *lo {
                            scheduler_boundaries.push(lo.sub(&job.service));
                        }
                    }
                }
            }
        }
        if scheduler_boundaries
            .into_iter()
            .min()
            .is_some_and(|boundary| delay > &boundary)
        {
            return Ok(None);
        }
        let Some(source) = self.source.delay(&state.source, delay)? else {
            return Ok(None);
        };
        let now = state.now.add(delay);
        for (task_index, last) in state.arrivals.iter().enumerate() {
            let Some(last) = last else {
                continue;
            };
            let task = &self.plan.task_set.tasks[task_index];
            let periodic = matches!(&task.arrival, Arrival::Periodic { .. });
            let interarrival = match &task.arrival {
                Arrival::Periodic { period } => period,
                Arrival::Sporadic { min_interarrival } => min_interarrival,
            };
            if periodic && last.add(interarrival) < now {
                return Ok(None);
            }
        }
        let mut reached = state.clone();
        reached.source = source;
        reached.now = now.clone();
        for job in &mut reached.jobs {
            if state.running == Some((job.task_index, job.slot)) {
                job.service = job.service.add(delay);
            }
            let ready_at = job.activated_at.add(&job.jitter);
            job.ready = now >= ready_at;
        }
        Ok(Some(reached))
    }

    /// Exact positive delay boundaries at which a source transition, task
    /// arrival, job completion or deadline can change the product's enabled
    /// moves. A symbolic EN-6 provider uses these as its concrete replay
    /// refinement points while maintaining zones over the corresponding
    /// clocks.
    pub fn next_event_delays(
        &self,
        state: &TaskProductState,
    ) -> Result<Vec<Rational>, super::TimedStop> {
        let mut delays = Vec::new();
        let positive = task_zero();
        let mut offer = |delay: Rational| {
            if delay > positive && !delays.contains(&delay) {
                delays.push(delay);
            }
        };
        if let Some((bound, _)) = self.source.admissible_delays(&state.source)?.upper() {
            offer(bound.clone());
        }
        for window in self.source.scheduled_windows(&state.source)? {
            for interval in window.intervals {
                let (lower, _) = interval.lower();
                offer(lower.clone());
                if let Some((upper, _)) = interval.upper() {
                    offer(upper.clone());
                }
            }
        }
        for (task_index, last) in state.arrivals.iter().enumerate() {
            let Some(last) = last else {
                continue;
            };
            let task = &self.plan.task_set.tasks[task_index];
            let interarrival = match &task.arrival {
                Arrival::Periodic { period } => period,
                Arrival::Sporadic { min_interarrival } => min_interarrival,
            };
            offer(last.add(interarrival).sub(&state.now));
        }
        for job in &state.jobs {
            if !state.missed {
                offer(job.deadline.sub(&state.now));
            }
            if state.running == Some((job.task_index, job.slot)) {
                offer(job.demand.sub(&job.service));
                let task = &self.plan.task_set.tasks[job.task_index];
                if task.criticality == Criticality::Hi && job.allowance_phase == AllowancePhase::Lo
                {
                    if let Wcet::Dual { lo, .. } = &task.wcet {
                        if job.service < *lo {
                            offer(lo.sub(&job.service));
                        }
                    }
                }
            }
            offer(job.activated_at.add(&job.jitter).sub(&state.now));
        }
        delays.sort();
        Ok(delays)
    }

    /// A finite observational key for concrete task-product search. Source
    /// clocks retain their exact rational values through the largest checked
    /// unary guard or invariant bound; values above that bound cannot change
    /// source enabledness or delay admissibility and share an overflow class.
    /// Absolute task time, job occurrence and slot generation are quotiented
    /// out; relative task phases, live slots, selected work and scheduler
    /// control remain in the key. Replay traces retain their exact states.
    pub fn state_key(
        &self,
        state: &TaskProductState,
    ) -> Result<Vec<u8>, TaskProductStateKeyRefusal> {
        let (_, source_key) = self.source.system().state_key(state.source.discrete())?;
        let mut key = source_key.hex();
        for (place, value) in self
            .source
            .places()
            .iter()
            .zip(state.source.clocks().values())
        {
            key.push('|');
            if !self.source.live(place, state.source.discrete()) {
                key.push('-');
                continue;
            }
            let Some(maximum) = self.source_clock_maxima.get(place.clock().index()) else {
                return Err(TaskProductStateKeyRefusal::TransitionIdentity);
            };
            if value > maximum {
                // No source guard or invariant distinguishes clock values
                // beyond the largest checked constant for this field.
                key.push('>');
            } else {
                key.push('=');
                push_rational_key(&mut key, value);
            }
        }
        // The vector reflects source-transition ordering; that order does
        // not change future scheduling, so key live jobs by their stable
        // task slot to merge equivalent release interleavings.
        let mut jobs: Vec<_> = state.jobs.iter().collect();
        jobs.sort_by_key(|job| (job.task_index, job.slot));
        let rational_text = |value: &Rational| {
            let mut text = String::new();
            push_rational_key(&mut text, value);
            text
        };
        // One writer shared with FR-418's symbolic node key.
        let control = super::task_key::TaskKeyControl {
            arrivals: state
                .arrivals
                .iter()
                .map(|arrival| {
                    arrival
                        .as_ref()
                        .map(|last| rational_text(&state.now.sub(last)))
                })
                .collect(),
            jobs: jobs
                .iter()
                .map(|job| super::task_key::TaskKeyJob {
                    task: job.task_index,
                    rank: None,
                    slot: job.slot,
                    concrete: vec![
                        rational_text(&job.demand),
                        rational_text(&job.jitter),
                        rational_text(&job.deadline.sub(&state.now)),
                        rational_text(&job.service),
                    ],
                    high_allowance: job.allowance_phase == AllowancePhase::Hi,
                    ready: job.ready,
                    running: state.running == Some((job.task_index, job.slot)),
                    release: &job.release,
                })
                .collect(),
            queue: None,
            high_mode: state.criticality_mode == CriticalityMode::Hi,
            pending: state
                .pending_budget_changes
                .iter()
                .map(|pending| super::task_key::TaskKeyPending {
                    task: pending.task_index,
                    index: pending.slot,
                    generation_occurrence: Some((pending.generation, pending.occurrence)),
                    source_transition: &pending.source_transition,
                })
                .collect(),
            missed: state.missed,
        };
        super::task_key::write_task_control(&mut key, &control)
            .map_err(|_| TaskProductStateKeyRefusal::TransitionIdentity)?;
        Ok(key.into_bytes())
    }

    /// The task-product lasso equivalence used by FR-364 replay. It keeps
    /// source control and refined clock regions, arrival phases, and the
    /// multiset of live job semantics, while alpha-renaming live slots,
    /// generations and occurrences. Each job remains a separate row, so
    /// equal jobs are never merged.
    pub fn lasso_state_key(
        &self,
        state: &TaskProductState,
    ) -> Result<Vec<u8>, TaskProductStateKeyRefusal> {
        let (_, source_key) = self.source.system().state_key(state.source.discrete())?;
        let mut key = source_key.hex();
        for (place, value) in self
            .source
            .places()
            .iter()
            .zip(state.source.clocks().values())
        {
            key.push('|');
            if !self.source.live(place, state.source.discrete()) {
                key.push('-');
                continue;
            }
            let Some(maximum) = self.source_clock_maxima.get(place.clock().index()) else {
                return Err(TaskProductStateKeyRefusal::TransitionIdentity);
            };
            if value > maximum {
                key.push('>');
            } else {
                key.push('=');
                push_rational_key(&mut key, value);
            }
        }
        key.push('|');
        key.push(match state.criticality_mode {
            CriticalityMode::Lo => 'l',
            CriticalityMode::Hi => 'h',
        });
        for pending in &state.pending_budget_changes {
            key.push(':');
            key.push_str(&pending.task_index.to_string());
            key.push('@');
            key.push_str(&pending.slot.to_string());
            key.push('#');
            key.push_str(&pending.generation.to_string());
            key.push('@');
            key.push_str(&pending.occurrence.to_string());
            let identity = crate::simulation::model::canonical_json(&pending.source_transition)
                .ok_or(TaskProductStateKeyRefusal::TransitionIdentity)?;
            key.push(':');
            key.push_str(
                &serde_json::to_string(&identity)
                    .map_err(|_| TaskProductStateKeyRefusal::TransitionIdentity)?,
            );
        }
        for (task_index, arrival) in state.arrivals.iter().enumerate() {
            key.push_str(&task_index.to_string());
            key.push(':');
            match arrival {
                Some(last) => push_rational_key(&mut key, &state.now.sub(last)),
                None => key.push('-'),
            }
            key.push(';');
        }
        let mut jobs = Vec::with_capacity(state.jobs.len());
        for job in &state.jobs {
            let task = self
                .plan
                .task_set
                .tasks
                .get(job.task_index)
                .ok_or(TaskProductStateKeyRefusal::TransitionIdentity)?;
            let release = crate::simulation::model::canonical_json(&job.release)
                .ok_or(TaskProductStateKeyRefusal::TransitionIdentity)?;
            jobs.push(format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                job.task_index,
                job.demand,
                job.jitter,
                job.deadline.sub(&state.now),
                job.service,
                task.priority
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                match job.allowance_phase {
                    AllowancePhase::Lo => "lo",
                    AllowancePhase::Hi => "hi",
                },
                if job.ready { 'r' } else { 'p' },
                if state.running == Some((job.task_index, job.slot)) {
                    'x'
                } else {
                    '-'
                },
                serde_json::to_string(&release)
                    .map_err(|_| TaskProductStateKeyRefusal::TransitionIdentity)?
            ));
        }
        jobs.sort_unstable();
        for job in jobs {
            key.push('|');
            key.push_str(&job);
        }
        key.push(if state.missed { '!' } else { '.' });
        Ok(key.into_bytes())
    }

    /// Every enabled source transition, synchronized with the checked task
    /// activations attached to its operation. Exact fixed work and zero
    /// jitter are handled directly; interval choices are returned as typed
    /// refusals for the bounded task-search path.
    pub fn source_steps(
        &self,
        state: &TaskProductState,
    ) -> Result<Vec<TaskProductSourceStep>, TaskProductMoveRefusal> {
        if !state.pending_budget_changes.is_empty() {
            return Ok(Vec::new());
        }
        let mut found = Vec::new();
        for (source, reached) in self
            .source
            .steps(&state.source)
            .map_err(TaskProductMoveRefusal::TimedSource)?
        {
            let mut next = state.clone();
            next.source = reached;
            let enters_hi = state.criticality_mode == CriticalityMode::Lo
                && self.plan.switches_mode_on(source.operation());
            if enters_hi {
                next.criticality_mode = CriticalityMode::Hi;
                let mut preexisting: Vec<_> = state
                    .jobs
                    .iter()
                    .filter(|job| {
                        self.plan.task_set.tasks[job.task_index].criticality == Criticality::Hi
                    })
                    .collect();
                preexisting.sort_by(|left, right| {
                    self.plan.task_set.tasks[left.task_index]
                        .name
                        .cmp(&self.plan.task_set.tasks[right.task_index].name)
                        .then_with(|| left.activated_at.cmp(&right.activated_at))
                        .then_with(|| left.occurrence.cmp(&right.occurrence))
                });
                next.pending_budget_changes = preexisting
                    .into_iter()
                    .map(|job| PendingBudgetChange {
                        task_index: job.task_index,
                        slot: job.slot,
                        generation: job.generation,
                        occurrence: job.occurrence,
                        source_transition: source.transition().clone(),
                    })
                    .collect();
            }
            let mut activations = Vec::new();
            let mut enabled = true;
            let mut unresolved_demand = false;
            for release in self.plan.releases_for_operation(source.operation()) {
                let bound = self.plan.tasks.iter().find(|bound| {
                    self.plan.task_set.tasks[bound.task_index].name == release.task.name
                });
                let Some(bound) = bound else {
                    continue;
                };
                let task = &self.plan.task_set.tasks[bound.task_index];
                let interarrival = match &task.arrival {
                    Arrival::Periodic { period } => period,
                    Arrival::Sporadic { min_interarrival } => min_interarrival,
                };
                if let Some(last) = state
                    .arrivals
                    .get(bound.task_index)
                    .and_then(Option::as_ref)
                {
                    let elapsed = state.now.sub(last);
                    let spacing_holds = match &task.arrival {
                        Arrival::Periodic { .. } => elapsed == *interarrival,
                        Arrival::Sporadic { .. } => elapsed >= *interarrival,
                    };
                    if !spacing_holds {
                        enabled = false;
                        break;
                    }
                }
                let demand = match task.fixed_work() {
                    Some(work) => work.clone(),
                    None => {
                        // The upper endpoint is one concrete admitted demand
                        // choice. EN-6 explores it as a replayable path and
                        // separately marks the remaining interval unresolved;
                        // it cannot use this endpoint to prove closure.
                        unresolved_demand = true;
                        match &task.wcet {
                            fsl_semantics::check::Wcet::Single(work) => work.clone(),
                            fsl_semantics::check::Wcet::Dual { hi, .. } => hi.clone(),
                        }
                    }
                };
                if !task.jitter.is_zero() {
                    return Err(TaskProductMoveRefusal::VariableJitter);
                }
                let Some(slot) = (0..bound.max_jobs).find(|slot| {
                    !state
                        .jobs
                        .iter()
                        .any(|job| job.task_index == bound.task_index && job.slot == *slot)
                }) else {
                    enabled = false;
                    break;
                };
                let generation = state
                    .generations
                    .get(&(bound.task_index, slot))
                    .copied()
                    .unwrap_or(0);
                let occurrence = state.occurrences[bound.task_index];
                let deadline = state.now.add(&task.deadline);
                let identity = TaskTransitionIdentity {
                    action: TaskTransitionAction::Activate,
                    task_set: self.plan.task_set.name.clone(),
                    task: task.name.clone(),
                    job: occurrence,
                    slot,
                    generation,
                    release: source.transition().clone(),
                    source_transition: Some(source.transition().clone()),
                    demand: demand.clone(),
                    jitter: task.jitter.clone(),
                    deadline: deadline.clone(),
                    priority: task.priority,
                    budget_delta: None,
                };
                next.arrivals[bound.task_index] = Some(state.now.clone());
                next.occurrences[bound.task_index] = occurrence
                    .checked_add(1)
                    .ok_or(TaskProductMoveRefusal::OccurrenceOverflow)?;
                next.generations
                    .insert((bound.task_index, slot), generation);
                next.jobs.push(TaskProductJob {
                    task_index: bound.task_index,
                    slot,
                    generation,
                    occurrence,
                    demand,
                    jitter: task.jitter.clone(),
                    deadline,
                    activated_at: state.now.clone(),
                    release: source.transition().clone(),
                    service: task_zero(),
                    ready: true,
                    allowance_phase: if task.criticality == Criticality::Hi
                        && next.criticality_mode == CriticalityMode::Hi
                    {
                        AllowancePhase::Hi
                    } else {
                        AllowancePhase::Lo
                    },
                });
                activations.push(identity);
            }
            if enabled {
                // One source move can release several tasks. The wire trace
                // represents that atomic move as activation steps ordered by
                // task name, regardless of declaration order.
                activations.sort_by(|left, right| left.task.cmp(&right.task));
                found.push(TaskProductSourceStep {
                    source,
                    state: next,
                    activations,
                    unresolved_demand,
                });
            }
        }
        Ok(found)
    }

    /// Whether a currently enabled source release is blocked only because
    /// its task has occupied every finite candidate slot. Such a boundary is
    /// search-frontier evidence; it cannot be used to close a task lasso.
    pub fn queue_cap_blocked(&self, state: &TaskProductState) -> Result<bool, TimedStop> {
        if !state.pending_budget_changes.is_empty() {
            return Ok(false);
        }
        for (source, _) in self.source.steps(&state.source)? {
            for release in self.plan.releases_for_operation(source.operation()) {
                let Some(bound) = self.plan.tasks.iter().find(|bound| {
                    self.plan.task_set.tasks[bound.task_index].name == release.task.name
                }) else {
                    continue;
                };
                let task = &self.plan.task_set.tasks[bound.task_index];
                if let Some(last) = state
                    .arrivals
                    .get(bound.task_index)
                    .and_then(Option::as_ref)
                {
                    let interarrival = match &task.arrival {
                        Arrival::Periodic { period } => period,
                        Arrival::Sporadic { min_interarrival } => min_interarrival,
                    };
                    if &state.now.sub(last) < interarrival {
                        continue;
                    }
                }
                let live_jobs = state
                    .jobs
                    .iter()
                    .filter(|job| job.task_index == bound.task_index)
                    .count();
                if u64::try_from(live_jobs).unwrap_or(u64::MAX) >= bound.max_jobs {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Enabled zero-time scheduler actions: completion, deadline miss,
    /// preemption, dispatch and resumption in policy order.
    pub fn scheduler_steps(&self, state: &TaskProductState) -> Vec<TaskProductDerivedStep> {
        if let Some(pending) = state.pending_budget_changes.first() {
            let Some(index) = state.jobs.iter().position(|job| {
                job.task_index == pending.task_index
                    && job.slot == pending.slot
                    && job.generation == pending.generation
                    && job.occurrence == pending.occurrence
            }) else {
                return Vec::new();
            };
            let job = &state.jobs[index];
            let delta = match &self.plan.task_set.tasks[job.task_index].wcet {
                Wcet::Dual { lo, hi } => hi.sub(lo),
                Wcet::Single(_) => return Vec::new(),
            };
            let mut reached = state.clone();
            reached.jobs[index].allowance_phase = AllowancePhase::Hi;
            reached.pending_budget_changes.remove(0);
            let mut identity = self.identity(job, TaskTransitionAction::BudgetChange);
            identity.source_transition = Some(pending.source_transition.clone());
            identity.budget_delta = Some(delta);
            return vec![TaskProductDerivedStep {
                identity,
                state: reached,
            }];
        }
        if !state.missed {
            for job in &state.jobs {
                if state.now >= job.deadline && job.service < job.demand {
                    let mut reached = state.clone();
                    reached.missed = true;
                    return vec![TaskProductDerivedStep {
                        identity: self.identity(job, TaskTransitionAction::DeadlineMiss),
                        state: reached,
                    }];
                }
            }
        }
        if let Some((task_index, slot)) = state.running {
            if let Some(index) = state
                .jobs
                .iter()
                .position(|job| job.task_index == task_index && job.slot == slot)
            {
                let job = &state.jobs[index];
                if job.service >= job.demand {
                    let mut reached = state.clone();
                    reached.jobs.remove(index);
                    reached.running = None;
                    let Some(generation) = job.generation.checked_add(1) else {
                        return Vec::new();
                    };
                    reached.generations.insert((task_index, slot), generation);
                    return vec![TaskProductDerivedStep {
                        identity: self.identity(job, TaskTransitionAction::Complete),
                        state: reached,
                    }];
                }
            }
        }
        if let Some((task_index, slot)) = state.running {
            let Some(index) = state
                .jobs
                .iter()
                .position(|job| job.task_index == task_index && job.slot == slot)
            else {
                return Vec::new();
            };
            let job = &state.jobs[index];
            let task = &self.plan.task_set.tasks[task_index];
            if task.criticality == Criticality::Hi
                && job.allowance_phase == AllowancePhase::Lo
                && matches!(&task.wcet, Wcet::Dual { lo, .. } if job.service >= *lo)
            {
                let mut reached = state.clone();
                reached.running = None;
                return vec![TaskProductDerivedStep {
                    identity: self.identity(job, TaskTransitionAction::Preempt),
                    state: reached,
                }];
            }
        }
        let best = state
            .jobs
            .iter()
            .filter(|job| {
                job.ready
                    && (self.plan.task_set.tasks[job.task_index].criticality != Criticality::Hi
                        || job.allowance_phase == AllowancePhase::Hi
                        || !matches!(
                            &self.plan.task_set.tasks[job.task_index].wcet,
                            Wcet::Dual { lo, .. } if job.service >= *lo
                        ))
            })
            .min_by(|left, right| self.job_order(left, right));
        let Some(best) = best else {
            return Vec::new();
        };
        if let Some(running) = state.running {
            if running == (best.task_index, best.slot) {
                return Vec::new();
            }
            let current = state
                .jobs
                .iter()
                .find(|job| job.task_index == running.0 && job.slot == running.1);
            let Some(current) = current else {
                return Vec::new();
            };
            if self.job_order(best, current).is_lt() {
                let mut reached = state.clone();
                reached.running = None;
                return vec![TaskProductDerivedStep {
                    identity: self.identity(current, TaskTransitionAction::Preempt),
                    state: reached,
                }];
            }
            return Vec::new();
        }
        let action = if best.service.is_zero() {
            TaskTransitionAction::Dispatch
        } else {
            TaskTransitionAction::Resume
        };
        let mut reached = state.clone();
        reached.running = Some((best.task_index, best.slot));
        vec![TaskProductDerivedStep {
            identity: self.identity(best, action),
            state: reached,
        }]
    }

    /// Replays a source-plus-scheduler prefix from the rebuilt initial state.
    /// This is the concrete FR-237 seam for fixed-work, zero-jitter task
    /// products; every delay, source transition and derived action is
    /// re-evaluated against the checked model and task declarations.
    pub fn replay_trace(
        &self,
        trace: &TaskProductTrace,
    ) -> Result<TaskProductState, TaskProductReplayRefusal> {
        self.replay_trace_states(trace)?
            .pop()
            .ok_or(TaskProductReplayRefusal::InitialState)
    }

    /// Replays a source-plus-scheduler prefix and returns every state from
    /// its selected initial state through its last transition.
    pub fn replay_trace_states(
        &self,
        trace: &TaskProductTrace,
    ) -> Result<Vec<TaskProductState>, TaskProductReplayRefusal> {
        self.replay_trace_observed(trace, |_, _| Ok(()))
    }

    /// Replays a prefix like [`Self::replay_trace_states`], calling `observe`
    /// with each recorded step's index and its post-state before the next
    /// step is derived, so a caller's post-state comparison refuses at the
    /// first failing step in step order (FSpec FR-364 "Comparison with the
    /// re-derived step").
    ///
    /// # Errors
    ///
    /// [`TaskProductReplayRefusal`] at the first step whose delay, derived
    /// identity or observed post-state differs from the rebuilt product's.
    pub fn replay_trace_observed(
        &self,
        trace: &TaskProductTrace,
        mut observe: impl FnMut(usize, &TaskProductState) -> Result<(), TaskProductReplayRefusal>,
    ) -> Result<Vec<TaskProductState>, TaskProductReplayRefusal> {
        let mut state = self
            .initial()
            .into_iter()
            .nth(trace.initial_state)
            .ok_or(TaskProductReplayRefusal::InitialState)?;
        let mut positions = vec![state.clone()];
        let mut step_index = 0;
        while step_index < trace.steps.len() {
            let recorded = &trace.steps[step_index];
            state = self
                .delay(&state, &recorded.delay)
                .map_err(TaskProductReplayRefusal::TimedSource)?
                .ok_or_else(|| {
                    member_refusal(
                        step_index,
                        "/delay",
                        &recorded.delay,
                        "a delay the rebuilt product admits",
                    )
                })?;
            let consumed = match &recorded.transition {
                TaskProductTransition::Task(identity)
                    if identity.action == TaskTransitionAction::Activate =>
                {
                    self.check_static(step_index, identity)?;
                    let moves = self
                        .source_steps(&state)
                        .map_err(|_| TaskProductReplayRefusal::UnresolvedChoice)?;
                    let candidate = Self::activation_move(&moves, identity).ok_or_else(|| {
                        member_refusal(
                            step_index,
                            "/transition",
                            "activate",
                            "no enabled activation",
                        )
                    })?;
                    let count = candidate.activations.len();
                    for (offset, derived) in candidate.activations.iter().enumerate() {
                        let at = step_index + offset;
                        let supplied = match trace.steps.get(at) {
                            Some(TaskProductTraceStep {
                                transition: TaskProductTransition::Task(supplied),
                                ..
                            }) if supplied.action == TaskTransitionAction::Activate => supplied,
                            other => {
                                return Err(member_refusal(
                                    at,
                                    "/transition",
                                    other.map_or_else(
                                        || "end of trace".to_owned(),
                                        |step| format!("{:?}", step.transition),
                                    ),
                                    format!("activate {}#{}", derived.task, derived.job),
                                ));
                            }
                        };
                        if offset > 0 {
                            let group_delay = &trace.steps[at].delay;
                            if !group_delay.is_zero() {
                                return Err(member_refusal(at, "/delay", group_delay, "0"));
                            }
                            self.check_static(at, supplied)?;
                        }
                        compare_identity(at, supplied, derived)?;
                    }
                    state = candidate.state.clone();
                    count
                }
                TaskProductTransition::TerminalStutter => {
                    let quiescent = self
                        .source
                        .is_quiescent(state.source.discrete())
                        .map_err(TaskProductReplayRefusal::TimedSource)?;
                    let source_steps = self
                        .source
                        .steps(&state.source)
                        .map_err(TaskProductReplayRefusal::TimedSource)?;
                    if recorded.delay <= task_zero() {
                        return Err(member_refusal(
                            step_index,
                            "/delay",
                            &recorded.delay,
                            "a positive terminal-stutter delay",
                        ));
                    }
                    if !quiescent
                        || !source_steps.is_empty()
                        || !state.jobs.is_empty()
                        || step_index + 1 != trace.steps.len()
                        || trace.lasso_entry != Some(step_index)
                    {
                        return Err(member_refusal(
                            step_index,
                            "/transition",
                            "terminal-stutter",
                            "loop entry is not terminal",
                        ));
                    }
                    // The common step prelude has already applied the
                    // positive terminal-stutter delay. This marker closes
                    // time divergence from a quiescent product state; it
                    // does not advance the clocks a second time.
                    1
                }
                TaskProductTransition::Source(identity) => {
                    let moves = self
                        .source_steps(&state)
                        .map_err(|_| TaskProductReplayRefusal::UnresolvedChoice)?;
                    if let Some(found) = moves.iter().find(|step| {
                        step.activations.is_empty() && step.source.transition() == identity
                    }) {
                        state = found.state.clone();
                    } else if let Some(activating) = moves.iter().find(|step| {
                        !step.activations.is_empty() && step.source.transition() == identity
                    }) {
                        let first = &activating.activations[0];
                        return Err(member_refusal(
                            step_index,
                            "/transition/type",
                            "transition",
                            format!("task-transition activate {}#{}", first.task, first.job),
                        ));
                    } else {
                        return Err(member_refusal(
                            step_index,
                            "/transition",
                            "transition",
                            "no enabled source transition",
                        ));
                    }
                    1
                }
                TaskProductTransition::Task(identity) => {
                    self.check_static(step_index, identity)?;
                    let derived = self.scheduler_steps(&state);
                    let mut same_action = derived
                        .iter()
                        .filter(|step| step.identity.action == identity.action)
                        .peekable();
                    if same_action.peek().is_none() {
                        return Err(member_refusal(
                            step_index,
                            "/transition",
                            format!("{:?}", identity.action),
                            "no enabled step with this action",
                        ));
                    }
                    let candidates: Vec<_> = same_action.collect();
                    let chosen = candidates
                        .iter()
                        .find(|step| {
                            step.identity.task == identity.task && step.identity.job == identity.job
                        })
                        .unwrap_or(&candidates[0]);
                    compare_identity(step_index, identity, &chosen.identity)?;
                    state = chosen.state.clone();
                    1
                }
            };
            for offset in 0..consumed {
                positions.push(state.clone());
                observe(step_index + offset, &state)?;
            }
            step_index += consumed;
        }
        if let Some(entry) = trace.lasso_entry {
            if matches!(
                trace.steps.last().map(|step| &step.transition),
                Some(TaskProductTransition::TerminalStutter)
            ) {
                return Ok(positions);
            }
            let start = positions
                .get(entry)
                .ok_or(TaskProductReplayRefusal::Transition)?;
            let last = trace.steps.len().saturating_sub(1);
            let entry_key = self
                .lasso_state_key(start)
                .map_err(|_| TaskProductReplayRefusal::Transition)?;
            let exit_key = self
                .lasso_state_key(&state)
                .map_err(|_| TaskProductReplayRefusal::Transition)?;
            if entry_key != exit_key {
                return Err(member_refusal(
                    last,
                    "/post_state",
                    "the last loop step's state",
                    "does not return to the loop entry",
                ));
            }
            let elapsed = state.now.sub(&start.now);
            if elapsed <= task_zero() {
                return Err(TaskProductReplayRefusal::Member {
                    step: None,
                    member: "/loop".to_owned(),
                    supplied: format!("loop elapsed {elapsed}"),
                    recomputed: "a positive loop elapsed time".to_owned(),
                });
            }
        }
        Ok(positions)
    }

    /// The enabled activating move a recorded `activate` step belongs to: the
    /// move whose group holds that task's job, else the first move whose
    /// first activation names the task, else the first activating move.
    fn activation_move<'m>(
        moves: &'m [TaskProductSourceStep],
        supplied: &TaskTransitionIdentity,
    ) -> Option<&'m TaskProductSourceStep> {
        let activating = || moves.iter().filter(|step| !step.activations.is_empty());
        activating()
            .find(|step| {
                step.activations
                    .iter()
                    .any(|a| a.task == supplied.task && a.job == supplied.job)
            })
            .or_else(|| activating().find(|step| step.activations[0].task == supplied.task))
            .or_else(|| activating().next())
    }

    /// FSpec FR-364's static checks on a supplied task identity: the task
    /// set and task exist, a budget change names a `hi` task and the priority
    /// is the declared one.
    fn check_static(
        &self,
        step: usize,
        supplied: &TaskTransitionIdentity,
    ) -> Result<(), TaskProductReplayRefusal> {
        let set = &self.plan.task_set;
        if supplied.task_set != set.name {
            return Err(member_refusal(
                step,
                "/transition/taskset",
                &supplied.task_set,
                &set.name,
            ));
        }
        let Some(task) = set.tasks.iter().find(|task| task.name == supplied.task) else {
            return Err(member_refusal(
                step,
                "/transition/task",
                &supplied.task,
                "a task declared by the task set",
            ));
        };
        if supplied.action == TaskTransitionAction::BudgetChange
            && task.criticality != Criticality::Hi
        {
            return Err(member_refusal(
                step,
                "/transition/task",
                &supplied.task,
                format!("{} is not a hi task", task.name),
            ));
        }
        if supplied.priority != task.priority {
            return Err(member_refusal(
                step,
                "/transition/priority",
                format!("{:?}", supplied.priority),
                format!("{:?}", task.priority),
            ));
        }
        Ok(())
    }

    fn identity(
        &self,
        job: &TaskProductJob,
        action: TaskTransitionAction,
    ) -> TaskTransitionIdentity {
        let task = &self.plan.task_set.tasks[job.task_index];
        TaskTransitionIdentity {
            action,
            task_set: self.plan.task_set.name.clone(),
            task: task.name.clone(),
            job: job.occurrence,
            slot: job.slot,
            generation: job.generation,
            release: job.release.clone(),
            source_transition: None,
            demand: job.demand.clone(),
            jitter: job.jitter.clone(),
            deadline: job.deadline.clone(),
            priority: task.priority,
            budget_delta: None,
        }
    }

    pub(super) fn job_order(
        &self,
        left: &TaskProductJob,
        right: &TaskProductJob,
    ) -> std::cmp::Ordering {
        let left_task = &self.plan.task_set.tasks[left.task_index];
        let right_task = &self.plan.task_set.tasks[right.task_index];
        match self.plan.policy {
            SchedulingPolicy::FixedPriority | SchedulingPolicy::Amc => left_task
                .priority
                .cmp(&right_task.priority)
                .then_with(|| left.activated_at.cmp(&right.activated_at))
                .then_with(|| left.occurrence.cmp(&right.occurrence)),
            SchedulingPolicy::Edf => left
                .deadline
                .cmp(&right.deadline)
                .then_with(|| left_task.name.cmp(&right_task.name))
                .then_with(|| left.activated_at.cmp(&right.activated_at))
                .then_with(|| left.occurrence.cmp(&right.occurrence)),
        }
    }
}

/// Why checked task-product inputs could not be joined for a claim.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductPlanRefusal {
    /// The selected package does not contain the named checked schedule claim.
    #[error("the package does not contain the named checked task claim")]
    MissingClaim,
    /// The selected checked claim names no task set.
    #[error("the checked task claim has no checked task set")]
    MissingTaskSet,
    /// A release on the selected model does not map to one of its timed
    /// operation identities.
    #[error("a checked task release does not map to the selected timed model")]
    UnknownTimedOperation,
    /// The selected timed model has a tick source; FR-249 lowers only dense
    /// time task products.
    #[error("task automaton lowering requires a dense-time source")]
    RequiresDenseTime,
    /// A checked task's candidate slot bound does not fit the finite slot
    /// index used by the task product.
    #[error("a checked task's finite candidate slot bound overflows")]
    TaskQueueBoundOverflow,
}

impl<'a> TaskProductPlan<'a> {
    /// Joins `claim_name` to the timed system's checked task set and release
    /// declarations. The task set and claim must already have passed S3.
    ///
    /// # Errors
    ///
    /// Returns [`TaskProductPlanRefusal`] when the selected model is not
    /// dense-time, the named checked claim or its checked task set is absent,
    /// or a release lacks its canonical operation identity.
    pub fn new(
        timed: &TimedSystem<'_, 'a>,
        claim_name: &str,
    ) -> Result<Self, TaskProductPlanRefusal> {
        if !matches!(timed.source().source, TimeSourceKind::Dense) {
            return Err(TaskProductPlanRefusal::RequiresDenseTime);
        }
        let checked = timed
            .system()
            .input()
            .source_package()
            .ok_or(TaskProductPlanRefusal::MissingTaskSet)?
            .graph()
            .task_sets();
        let claim = checked
            .claims()
            .iter()
            .find(|claim| claim.name == claim_name)
            .ok_or(TaskProductPlanRefusal::MissingClaim)?;
        let task_set = checked
            .task_sets()
            .iter()
            .find(|task_set| task_set.name == claim.task_set)
            .ok_or(TaskProductPlanRefusal::MissingTaskSet)?;
        let releases = checked
            .releases()
            .iter()
            .filter(|release| {
                release.task_set() == task_set.name
                    && release.operation().model() == timed.model().model()
            })
            .map(|release| {
                let task = task_set
                    .tasks
                    .iter()
                    .find(|task| task.name == release.task())
                    .ok_or(TaskProductPlanRefusal::MissingTaskSet)?;
                let declaring = release.operation().declaring();
                let operation_name = release.operation().name();
                let operation = timed
                    .system()
                    .operations()
                    .into_iter()
                    .find(|(owner, name, _)| owner == &declaring && *name == operation_name)
                    .map(|(_, _, operation)| operation)
                    .ok_or(TaskProductPlanRefusal::UnknownTimedOperation)?
                    .to_owned();
                Ok(TaskProductRelease {
                    release,
                    task,
                    operation,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mode_switches = checked
            .mode_switches()
            .iter()
            .filter(|switch| {
                switch.task_set() == task_set.name
                    && switch.operation().model() == timed.model().model()
            })
            .map(|declaration| {
                let declaring = declaration.operation().declaring();
                let operation_name = declaration.operation().name();
                let operation = timed
                    .system()
                    .operations()
                    .into_iter()
                    .find(|(owner, name, _)| owner == &declaring && *name == operation_name)
                    .map(|(_, _, operation)| operation)
                    .ok_or(TaskProductPlanRefusal::UnknownTimedOperation)?
                    .to_owned();
                Ok(TaskProductModeSwitch {
                    declaration,
                    operation,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let tasks = task_set
            .tasks
            .iter()
            .enumerate()
            .map(|(task_index, task)| {
                let interarrival = match &task.arrival {
                    Arrival::Periodic { period } => period,
                    Arrival::Sporadic { min_interarrival } => min_interarrival,
                };
                let max_jobs = candidate_max_jobs(&interarrival, &task.deadline)
                    .ok_or(TaskProductPlanRefusal::TaskQueueBoundOverflow)?;
                let releases = releases
                    .iter()
                    .enumerate()
                    .filter_map(|(release_index, release)| {
                        (release.task.name == task.name).then_some(release_index)
                    })
                    .collect();
                Ok(TaskProductTask {
                    task_index,
                    max_jobs,
                    releases,
                })
            })
            .collect::<Result<Vec<_>, TaskProductPlanRefusal>>()?;
        let policy: SchedulingPolicy = claim.policy();
        Ok(Self {
            task_set,
            claim,
            policy,
            model: timed.model().model().to_owned(),
            releases,
            mode_switches,
            tasks,
        })
    }

    /// The releases activated by one source transition, in checked source
    /// declaration order. EN-6 uses this lookup while lowering each guarded
    /// source edge into the product's derived task edges.
    pub fn releases_for_operation<'p>(
        &'p self,
        operation: &'p str,
    ) -> impl Iterator<Item = &'p TaskProductRelease<'a>> + 'p {
        self.releases
            .iter()
            .filter(move |release| release.operation == operation)
    }

    /// Whether one checked source operation enters high mode for this product.
    pub fn switches_mode_on(&self, operation: &str) -> bool {
        self.mode_switches
            .iter()
            .any(|switch| switch.operation == operation)
    }

    /// Whether every task has the exact fixed-work demand required by the
    /// current FR-418 finite-product gate. The caller still has to prove
    /// complete source-plus-scheduler closure before constructing a holding
    /// certificate.
    pub fn has_fixed_work_demand(&self) -> bool {
        self.task_set
            .tasks
            .iter()
            .all(|task| task.fixed_work().is_some())
    }

    /// Auxiliary clocks in FSpec FR-418 order: every active task's arrival
    /// clock, then each finite job slot's jitter, service and nominal-deadline
    /// clocks. Source-model clocks remain first in the enclosing timed
    /// subject. All returned clocks advance at unit rate.
    pub fn auxiliary_clocks(&self) -> Vec<TaskProductClock> {
        let mut clocks = Vec::new();
        for (task, bound) in self.task_set.tasks.iter().zip(&self.tasks) {
            if bound.releases.is_empty() {
                continue;
            }
            clocks.push(TaskProductClock {
                name: format!("{}.arrival", task.name),
                kind: TaskProductClockKind::Arrival,
                task_index: bound.task_index,
                slot: None,
            });
        }
        for (task, bound) in self.task_set.tasks.iter().zip(&self.tasks) {
            if bound.releases.is_empty() || task.jitter == Rational::from_integer(Integer::zero()) {
                continue;
            }
            for slot in 0..bound.max_jobs {
                clocks.push(TaskProductClock {
                    name: format!("{}.slot{slot}.jitter", task.name),
                    kind: TaskProductClockKind::Jitter,
                    task_index: bound.task_index,
                    slot: Some(slot),
                });
            }
        }
        for (task, bound) in self.task_set.tasks.iter().zip(&self.tasks) {
            if bound.releases.is_empty() {
                continue;
            }
            for slot in 0..bound.max_jobs {
                clocks.push(TaskProductClock {
                    name: format!("{}.slot{slot}.service", task.name),
                    kind: TaskProductClockKind::Service,
                    task_index: bound.task_index,
                    slot: Some(slot),
                });
                clocks.push(TaskProductClock {
                    name: format!("{}.slot{slot}.deadline", task.name),
                    kind: TaskProductClockKind::Deadline,
                    task_index: bound.task_index,
                    slot: Some(slot),
                });
            }
        }
        clocks
    }
}

pub(super) fn source_clock_maxima(source: &TimedSystem<'_, '_>) -> Vec<Rational> {
    let mut maxima = vec![task_zero(); source.model().clocks().len()];
    for guard in source.model().guards() {
        for part in guard.parts() {
            for constraint in part.clocks() {
                if let Some(maximum) = maxima.get_mut(constraint.clock().index()) {
                    if constraint.bound() > maximum {
                        *maximum = constraint.bound().clone();
                    }
                }
            }
        }
    }
    for invariant in source.model().invariants() {
        for constraint in invariant.bounds() {
            if let Some(maximum) = maxima.get_mut(constraint.clock().index()) {
                if constraint.bound() > maximum {
                    *maximum = constraint.bound().clone();
                }
            }
        }
    }
    maxima
}

/// FR-419's finite candidate job cap, `ceil(D / T) + 1`.
fn candidate_max_jobs(interarrival: &Rational, deadline: &Rational) -> Option<u64> {
    let periods = deadline.div(interarrival)?;
    let denominator = filament_exact::NonZeroInteger::try_from(periods.denominator()).ok()?;
    let (floor, remainder) = periods.numerator().div_mod_floor(denominator);
    let ceiling = if remainder.is_zero() {
        floor
    } else {
        floor.add(&Integer::one())
    };
    ceiling.to_u64()?.checked_add(1)
}

fn task_zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn push_rational_key(key: &mut String, value: &Rational) {
    key.push_str(&value.numerator().to_string());
    key.push('/');
    key.push_str(&value.denominator().to_string());
}

/// A member-level replay refusal at `member`, a JSON pointer relative to the
/// recorded step at `step`.
fn member_refusal(
    step: usize,
    member: &str,
    supplied: impl std::fmt::Display,
    recomputed: impl std::fmt::Display,
) -> TaskProductReplayRefusal {
    TaskProductReplayRefusal::Member {
        step: Some(step),
        member: member.to_owned(),
        supplied: supplied.to_string(),
        recomputed: recomputed.to_string(),
    }
}

/// Compares a supplied identity with the re-derived one member by member in
/// FSpec FR-364's schema order, refusing at the first difference.
fn compare_identity(
    step: usize,
    supplied: &TaskTransitionIdentity,
    derived: &TaskTransitionIdentity,
) -> Result<(), TaskProductReplayRefusal> {
    macro_rules! member {
        ($name:literal, $field:ident) => {
            if supplied.$field != derived.$field {
                return Err(member_refusal(
                    step,
                    concat!("/transition/", $name),
                    format!("{:?}", supplied.$field),
                    format!("{:?}", derived.$field),
                ));
            }
        };
    }
    macro_rules! shown {
        ($name:literal, $field:ident) => {
            if supplied.$field != derived.$field {
                return Err(member_refusal(
                    step,
                    concat!("/transition/", $name),
                    &supplied.$field,
                    &derived.$field,
                ));
            }
        };
    }
    member!("taskset", task_set);
    member!("task", task);
    member!("job", job);
    member!("slot", slot);
    member!("generation", generation);
    member!("release", release);
    member!("source_transition", source_transition);
    shown!("demand", demand);
    shown!("jitter", jitter);
    shown!("deadline", deadline);
    member!("priority", priority);
    member!("budget_delta", budget_delta);
    Ok(())
}
