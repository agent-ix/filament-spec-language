// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418's task automaton: a syntactic lowering of a
//! [`TaskProductPlan`] over its timed source to a timed automaton whose
//! locations are source discrete state x scheduler control and whose edges
//! carry guards, resets, diagonal guards, bounded subtractions and finite
//! template labels.
//!
//! The lowering is data. It depends on no difference-bound-matrix code, so the
//! engine's DBM (`fsl-analyze`) and the checker's DBM (`fsl-replay`) each
//! interpret the same [`TaskEdge`] and [`LocationInfo`] values with their own
//! zone operations. [`TaskEdge::fire`] and [`LocationInfo::admits_delay`] are
//! the point semantics of that data over one exact valuation; they are the
//! reference the cross-check test compares with [`TaskProductSystem`].
//!
//! # Clocks
//!
//! Clock index `0` is the fixed reference clock, so a valuation holds
//! `clocks().len() + 1` values and `v[0]` is zero. Clocks follow FR-418's
//! canonical order: source clock places, task arrival clocks, pending-release
//! jitter clocks, then each task slot's service and nominal-deadline clocks
//! (the order of [`TaskProductPlan::auxiliary_clocks`]).
//!
//! Every clock has rate one. A slot's service clock simulates a stopwatch
//! with bounded subtraction: it is reset when the job is first dispatched,
//! runs at unit rate while the job is suspended, and every completion of a
//! job subtracts that job's fixed work from the service clock of every started
//! job, so a job's service clock equals its received service whenever it runs.
//! A clock that the control does not use (an unstarted job's service clock, a
//! free slot) is dead: [`LocationInfo::live`] is `false` and a consumer holds
//! it at zero, as it does a source clock of an absent object.
//!
//! # Locations and the live-job renaming quotient
//!
//! A [`TaskControl`] lists each task's live jobs in deadline-clock age order,
//! oldest first. A job's rank is its position in that list, so activation
//! occurrence and slot generation never enter a location. A job keeps its
//! storage slot while it lives, because its clocks are the slot's clocks; a
//! new job takes the lowest free slot, and a task whose slots are all live has
//! no activation edge: the attempt is an [`Overflow`], never a location.
//! [`TaskLocation::key`] is the finite key. Its fields are plain data so the
//! certificate's `TaskProductKey` can be built from it field by field.
//!
//! # Deadline misses end the lowering
//!
//! A deadline-miss edge targets a terminal location: its [`LocationInfo`] stops
//! time, and [`TaskAutomaton::successors`] of it returns
//! [`TaskAutomatonRefusal::PostMiss`]. Past a miss the service-clock encoding is
//! not exact (a suspended job's clock can exceed its ceiling, so a bounded
//! subtraction would fail and the automaton would time-lock where the concrete
//! product continues). A schedulability proof needs no miss to be reachable, so
//! a checker that reaches a miss edge must not certify: the item is refuted by
//! the concrete search or stays `incomplete`, never `proved`.
//!
//! # Template ties and unreachable shapes
//!
//! A task has at most `ceil(D/T)` live jobs before its first miss, one fewer
//! than its `ceil(D/T)+1` slots, so an [`Overflow`] needs a prior miss and
//! the lowering refuses past a miss first; the overflow data is reported for
//! completeness and a checker treats a reachable one as stopping certification.
//!
//! When several jobs miss at one instant the miss template names the first in
//! task-then-rank order; the concrete product names the first in activation
//! order, which a quotient node does not hold. Targets are equal; only the
//! label can differ. One operation releasing the same task twice is refused
//! at check time (`DuplicateRelease`, fsl-semantics), and the lowering refuses
//! it too ([`TaskAutomatonRefusal::DuplicateRelease`]) instead of guessing.
//!
//! # Policy
//!
//! The policy is the plan's, derived from the checked claim. Fixed priority
//! orders ready jobs by declared priority then age. Earliest deadline first
//! orders them by absolute deadline, which is a diagonal comparison of two
//! deadline clocks, then by canonical task name, then by age. The scheduler
//! actions are urgent and prioritised miss, complete, preempt, dispatch or
//! resume; the lowering makes their guards mutually exclusive, so at most one
//! scheduler edge is enabled at a valuation, as in the concrete product.

use std::collections::BTreeMap;

use filament_exact::{Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::check::{ClockOp, SchedulingPolicy, Wcet};

use super::task_product::source_clock_maxima;
use super::{
    CriticalityMode, ModelSimState, PlaceConstraint, TaskProductClock, TaskProductClockKind,
    TaskProductState, TaskProductSystem, TaskTransitionAction, TimedStop,
};
use crate::simulation::StateKey;

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// One difference constraint `x_left - x_right < bound` (or `<=`). Clock
/// index `0` is the reference clock, so `x <= c` is `(x, 0, <=, c)` and
/// `x >= c` is `(0, x, <=, -c)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockAtom {
    /// The minuend clock index.
    pub left: usize,
    /// The subtrahend clock index.
    pub right: usize,
    /// `<` when set, `<=` otherwise.
    pub strict: bool,
    /// The exact constant, in the model's unit.
    pub bound: Rational,
}

impl ClockAtom {
    /// `x <= bound`, or `x < bound` when `strict`.
    pub fn upper(clock: usize, strict: bool, bound: Rational) -> Self {
        Self {
            left: clock,
            right: 0,
            strict,
            bound,
        }
    }

    /// `x >= bound`, or `x > bound` when `strict`.
    pub fn lower(clock: usize, strict: bool, bound: &Rational) -> Self {
        Self {
            left: 0,
            right: clock,
            strict,
            bound: bound.neg(),
        }
    }

    /// Whether the constraint compares two clocks.
    pub fn is_diagonal(&self) -> bool {
        self.left != 0 && self.right != 0
    }

    /// The constraint's complement, `x_right - x_left <=' -bound`.
    #[must_use]
    pub fn negated(&self) -> Self {
        Self {
            left: self.right,
            right: self.left,
            strict: !self.strict,
            bound: self.bound.neg(),
        }
    }

    /// Whether the valuation `v` (index `0` is the reference clock) satisfies
    /// the constraint.
    pub fn holds(&self, v: &[Rational]) -> bool {
        let difference = v[self.left].sub(&v[self.right]);
        if self.strict {
            difference < self.bound
        } else {
            difference <= self.bound
        }
    }
}

/// A guard in disjunctive normal form: a union of convex conjunctions. No
/// alternative is false; one empty alternative is true.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Guard {
    alternatives: Vec<Vec<ClockAtom>>,
}

impl Guard {
    /// The guard no valuation satisfies.
    pub fn falsity() -> Self {
        Self::default()
    }

    /// The guard every valuation satisfies.
    pub fn truth() -> Self {
        Self {
            alternatives: vec![Vec::new()],
        }
    }

    /// One conjunction of `atoms`.
    pub fn conjunction(atoms: Vec<ClockAtom>) -> Self {
        Self {
            alternatives: vec![atoms],
        }
    }

    /// The convex alternatives whose union the guard is.
    pub fn alternatives(&self) -> &[Vec<ClockAtom>] {
        &self.alternatives
    }

    /// Whether no valuation can satisfy the guard.
    pub fn is_falsity(&self) -> bool {
        self.alternatives.is_empty()
    }

    /// The conjunction of two guards.
    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        let mut alternatives = Vec::new();
        for left in &self.alternatives {
            for right in &other.alternatives {
                let mut joined = left.clone();
                for atom in right {
                    if !joined.contains(atom) {
                        joined.push(atom.clone());
                    }
                }
                alternatives.push(joined);
            }
        }
        Self { alternatives }
    }

    /// The disjunction of two guards.
    #[must_use]
    pub fn or(&self, other: &Self) -> Self {
        let mut alternatives = self.alternatives.clone();
        alternatives.extend(other.alternatives.iter().cloned());
        Self { alternatives }
    }

    /// The complement guard.
    #[must_use]
    pub fn negated(&self) -> Self {
        let mut complement = Self::truth();
        for alternative in &self.alternatives {
            let atoms = Self {
                alternatives: alternative
                    .iter()
                    .map(|atom| vec![atom.negated()])
                    .collect(),
            };
            complement = complement.and(&atoms);
        }
        complement
    }

    /// Whether the valuation `v` satisfies some alternative.
    pub fn holds(&self, v: &[Rational]) -> bool {
        self.alternatives
            .iter()
            .any(|alternative| alternative.iter().all(|atom| atom.holds(v)))
    }
}

/// The role of one automaton clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockRole {
    /// A source clock place, by its index in `TimedSystem::places`.
    Source {
        /// The place index.
        place: usize,
    },
    /// A task-product clock.
    Task(TaskProductClock),
}

/// One automaton clock; its DBM index is its position in
/// [`TaskAutomaton::clocks`] plus one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomatonClock {
    /// The clock's name: `src<place>` for a source place, otherwise the
    /// task-product clock name.
    pub name: String,
    /// Its role.
    pub role: ClockRole,
    /// The ceiling the refined-region relation uses, `M_x`.
    pub ceiling: Rational,
}

/// A diagonal predicate `left - right <op> constant` whose truth value the
/// refined-region relation preserves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomatonDiagonal {
    /// The left clock index.
    pub left: usize,
    /// The comparison: `>` or `>=`.
    pub strict: bool,
    /// The right clock index.
    pub right: usize,
    /// The exact constant.
    pub constant: Rational,
}

/// Where a live job is in the scheduler.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JobPhase {
    /// Activated, waiting out its release jitter.
    Pending,
    /// Available to the scheduler.
    Ready,
    /// Receiving service.
    Running,
}

/// One live job in a location, identified by task and rank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveJob {
    /// The task's source-order index in its task set.
    pub task_index: usize,
    /// The storage slot whose clocks the job uses.
    pub slot: u64,
    /// The source release transition that activated it.
    pub release: ModelTransition,
    /// Its scheduler phase.
    pub phase: JobPhase,
    /// Whether the job has received service, so its service clock is live.
    pub started: bool,
}

/// The scheduler control of a location.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskControl {
    /// Per task, whether its arrival clock is live: a release occurred.
    pub arrival_started: Vec<bool>,
    /// Live jobs ordered by task then rank (deadline-clock age, oldest first).
    pub jobs: Vec<LiveJob>,
    /// The monotone criticality mode.
    pub mode: CriticalityMode,
    /// Whether the first deadline miss has been taken.
    pub missed: bool,
}

impl TaskControl {
    /// The zero-based rank of the job at `position` among its task's jobs.
    pub fn rank(&self, position: usize) -> u64 {
        let task = self.jobs[position].task_index;
        let rank = self.jobs[..position]
            .iter()
            .filter(|job| job.task_index == task)
            .count();
        u64::try_from(rank).unwrap_or(u64::MAX)
    }
}

/// One live job of a [`LocationKey`], plain data.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LiveJobKey {
    /// The task's source-order index.
    pub task_index: usize,
    /// The job's rank among its task's live jobs.
    pub rank: u64,
    /// The slot bound to the rank.
    pub slot: u64,
    /// The scheduler phase.
    pub phase: JobPhase,
    /// Whether the service clock is live.
    pub started: bool,
    /// The activating source transition's canonical JSON.
    pub release: String,
}

/// The finite key of a location: the complete source-plus-scheduler product
/// key modulo the live-job renaming, as plain data.
///
/// Its source part is the FR-181 source `StateKey` octets. The certificate
/// layer builds its `TaskProductKey` from these fields one for one (source
/// octets, arrival phases, jobs as task, rank, slot, phase and started,
/// criticality mode, miss latch) and takes its canonical octets from that
/// type; nothing here parses or re-encodes them.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LocationKey {
    /// The source discrete state's key octets.
    pub source: Vec<u8>,
    /// Per task, whether its arrival clock is live.
    pub arrival_started: Vec<bool>,
    /// The live jobs by task then rank.
    pub jobs: Vec<LiveJobKey>,
    /// Whether the criticality mode is high.
    pub high_mode: bool,
    /// Whether a deadline miss has been taken.
    pub missed: bool,
}

/// A location: a source discrete state and the scheduler control.
#[derive(Clone, Debug)]
pub struct TaskLocation {
    source: ModelSimState,
    source_key: StateKey,
    control: TaskControl,
}

impl TaskLocation {
    /// The source discrete state.
    pub fn source(&self) -> &ModelSimState {
        &self.source
    }

    /// The source state's canonical key.
    pub fn source_key(&self) -> &StateKey {
        &self.source_key
    }

    /// The scheduler control.
    pub fn control(&self) -> &TaskControl {
        &self.control
    }

    /// The finite location key.
    ///
    /// # Errors
    ///
    /// [`TaskAutomatonRefusal::Identity`] when a release transition has no
    /// canonical identity.
    pub fn key(&self) -> Result<LocationKey, TaskAutomatonRefusal> {
        let mut jobs = Vec::new();
        for (position, job) in self.control.jobs.iter().enumerate() {
            let identity = crate::simulation::model::canonical_json(&job.release)
                .and_then(|value| serde_json::to_string(&value).ok())
                .ok_or(TaskAutomatonRefusal::Identity)?;
            jobs.push(LiveJobKey {
                task_index: job.task_index,
                rank: self.control.rank(position),
                slot: job.slot,
                phase: job.phase,
                started: job.started,
                release: identity,
            });
        }
        Ok(LocationKey {
            source: self.source_key.as_bytes().to_vec(),
            arrival_started: self.control.arrival_started.clone(),
            jobs,
            high_mode: self.control.mode == CriticalityMode::Hi,
            missed: self.control.missed,
        })
    }
}

/// FR-418's finite `TaskTransitionTemplate`, with the task's index kept for
/// the consumer's table lookups.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTemplate {
    /// The scheduler action.
    pub action: TaskTransitionAction,
    /// The task-set name.
    pub task_set: String,
    /// The task name.
    pub task: String,
    /// The task's source-order index.
    pub task_index: usize,
    /// The job's rank among its task's live jobs. On an activation it names
    /// the new job in the target location; otherwise the affected job in the
    /// source location.
    pub job_rank: u64,
    /// The slot bound to the rank.
    pub slot: u64,
    /// The source release transition.
    pub release: ModelTransition,
    /// The source transition enabling the action; absent for scheduler
    /// actions with no source transition.
    pub source_transition: Option<ModelTransition>,
    /// The declared priority, absent when the task declares none.
    pub priority: Option<u64>,
}

/// `clock := value`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockReset {
    /// The clock index.
    pub clock: usize,
    /// The new value.
    pub value: Rational,
}

/// A source clock carried from one place to another by a collection write:
/// `to := from`, every source read before any write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockCarry {
    /// The clock index written.
    pub to: usize,
    /// The clock index read.
    pub from: usize,
}

/// A bounded subtraction `clock := clock - amount`, enabled only in
/// `amount <= clock <= ceiling` (FR-418 `bounded-subtraction-diagonal/v1`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedSubtraction {
    /// The clock index updated.
    pub clock: usize,
    /// The positive constant subtracted: the completed job's fixed work.
    pub amount: Rational,
    /// The clock's ceiling `M_x`.
    pub ceiling: Rational,
    /// The task-scheduler edge the row belongs to.
    pub template: TaskTemplate,
}

/// What an edge is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EdgeKind {
    /// A source transition, with the templates of the jobs it activates
    /// (empty when it releases no task), in task-name order.
    Source {
        /// The source transition identity.
        transition: ModelTransition,
        /// One `Activate` template per released task.
        activations: Vec<TaskTemplate>,
    },
    /// A task-scheduler action, labelled by its template.
    Scheduler(TaskTemplate),
}

/// One edge of the task automaton.
#[derive(Clone, Debug)]
pub struct TaskEdge {
    /// What the edge is.
    pub kind: EdgeKind,
    /// The guard, including every diagonal guard and, for a scheduler action,
    /// the exclusion of every higher-priority urgent action.
    pub guard: Guard,
    /// Constant resets, applied after the carries.
    pub resets: Vec<ClockReset>,
    /// Source clock carries.
    pub carries: Vec<ClockCarry>,
    /// Bounded subtractions on service clocks.
    pub subtractions: Vec<BoundedSubtraction>,
    /// The target location; its [`LocationInfo`] invariant also constrains
    /// the valuation after the update.
    pub target: TaskLocation,
}

/// A release that would need a slot past `max_jobs` (FR-418: overflow, not a
/// node). The step has no target and certification stops where it is enabled.
#[derive(Clone, Debug)]
pub struct Overflow {
    /// The source transition whose release has no free slot.
    pub transition: ModelTransition,
    /// The task whose slots are all live.
    pub task_index: usize,
    /// Where the attempt is enabled.
    pub guard: Guard,
}

/// The enabled edges of a location.
#[derive(Clone, Debug)]
pub struct Successors {
    /// Source edges in canonical transition order, then scheduler edges in
    /// priority order (release, miss, complete, preempt, dispatch or resume).
    pub edges: Vec<TaskEdge>,
    /// The attempted allocations past a task's slot bound.
    pub overflows: Vec<Overflow>,
}

/// Time at a location: the invariant, urgency and live clocks.
#[derive(Clone, Debug)]
pub struct LocationInfo {
    /// Every constraint time may not leave: source invariants plus the task
    /// arrival, jitter, service and deadline bounds.
    pub invariant: Vec<ClockAtom>,
    /// The source blocks time (an urgent state or a tick source).
    pub time_stopped: bool,
    /// Where a scheduler action is enabled; no positive delay starts there.
    pub urgent: Guard,
    /// Per clock index (`live[0]` is the reference clock, always live)
    /// whether the control uses the clock.
    pub live: Vec<bool>,
}

impl LocationInfo {
    /// Whether the invariant holds at `v`.
    pub fn invariant_holds(&self, v: &[Rational]) -> bool {
        self.invariant.iter().all(|atom| atom.holds(v))
    }

    /// Whether a positive delay of `delay` is admitted from `v`: the source
    /// permits time, no scheduler action is enabled at `v`, and the invariant
    /// holds at `v + delay`.
    pub fn admits_delay(&self, v: &[Rational], delay: &Rational) -> bool {
        if self.time_stopped || self.urgent.holds(v) {
            return false;
        }
        let delayed = delay_valuation(self, v, delay);
        self.invariant_holds(&delayed)
    }
}

/// `v` after `delay`: every live clock advances; a dead clock stays zero.
pub fn delay_valuation(info: &LocationInfo, v: &[Rational], delay: &Rational) -> Vec<Rational> {
    v.iter()
        .enumerate()
        .map(|(index, value)| {
            if index == 0 {
                zero()
            } else if info.live[index] {
                value.add(delay)
            } else {
                zero()
            }
        })
        .collect()
}

/// Why a closed-finite-product certificate cannot cover this task automaton.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CertificationRefusal {
    /// A task has no exact fixed-work demand (FR-419).
    #[error("a task has no exact fixed-work demand")]
    NotFixedWork,
    /// A dual-WCET task set with a mode switch: `closed-finite-product/v1`
    /// does not certify it, so the item settles `incomplete` with
    /// `bounded-search-incomplete{reason: certificate-unchecked}` (FR-418).
    #[error("a dual-WCET task set with a mode switch is not certified")]
    DualWcetModeSwitch,
    /// Two tasks of a fixed-priority claim share a priority, which claim
    /// admission rejects.
    #[error("two tasks share a fixed priority")]
    EqualPriorities,
}

/// Why the lowering could not produce a location's successors.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskAutomatonRefusal {
    /// The task set is outside the closed finite product.
    #[error("{0}")]
    Uncertified(#[from] CertificationRefusal),
    /// Source expansion stopped.
    #[error("the timed source stopped")]
    Source(TimedStop),
    /// A source state has no canonical key.
    #[error("a source state has no canonical key")]
    Key(#[from] crate::simulation::EncodingRefusal),
    /// A release transition has no canonical identity.
    #[error("a release transition has no canonical identity")]
    Identity,
    /// The clock layout does not hold a slot the control names.
    #[error("the task clock layout has no such slot")]
    Layout,
    /// The location follows a deadline miss; the lowering is not exact there.
    #[error("successors past a deadline miss are not modelled")]
    PostMiss,
    /// One source operation releases the same task twice.
    #[error("one source operation releases a task twice")]
    DuplicateRelease,
}

#[derive(Clone, Copy, Debug)]
struct SlotClocks {
    jitter: Option<usize>,
    service: usize,
    deadline: usize,
}

/// A task product lowered to a timed automaton.
pub struct TaskAutomaton<'t, 's, 'a> {
    product: &'t TaskProductSystem<'s, 'a>,
    clocks: Vec<AutomatonClock>,
    source_places: usize,
    arrival: Vec<Option<usize>>,
    slots: Vec<Vec<SlotClocks>>,
}

impl<'t, 's, 'a> TaskAutomaton<'t, 's, 'a> {
    /// Lowers `product`. Construction always succeeds; [`Self::certification`]
    /// states whether a closed-finite-product certificate can cover the set and
    /// every edge query refuses a set it cannot.
    pub fn new(product: &'t TaskProductSystem<'s, 'a>) -> Self {
        let plan = product.plan();
        let source = product.source();
        let maxima = source_clock_maxima(source);
        let mut clocks = Vec::new();
        for (index, place) in source.places().iter().enumerate() {
            clocks.push(AutomatonClock {
                name: format!("src{index}"),
                role: ClockRole::Source { place: index },
                ceiling: maxima
                    .get(place.clock().index())
                    .cloned()
                    .unwrap_or_else(zero),
            });
        }
        let source_places = clocks.len();
        let mut arrival = vec![None; plan.tasks.len()];
        let mut slots: Vec<Vec<SlotClocks>> = plan.tasks.iter().map(|_| Vec::new()).collect();
        let mut jitters: BTreeMap<(usize, u64), usize> = BTreeMap::new();
        let mut services: BTreeMap<(usize, u64), usize> = BTreeMap::new();
        for aux in plan.auxiliary_clocks() {
            let task = &plan.task_set.tasks[aux.task_index];
            let ceiling = match aux.kind {
                TaskProductClockKind::Arrival => match &task.arrival {
                    fsl_semantics::check::Arrival::Periodic { period } => period.clone(),
                    fsl_semantics::check::Arrival::Sporadic { min_interarrival } => {
                        min_interarrival.clone()
                    }
                },
                TaskProductClockKind::Jitter => task.jitter.clone(),
                TaskProductClockKind::Deadline => task.deadline.clone(),
                TaskProductClockKind::Service => {
                    let work = task
                        .fixed_work()
                        .cloned()
                        .unwrap_or_else(|| match &task.wcet {
                            Wcet::Single(work) => work.clone(),
                            Wcet::Dual { hi, .. } => hi.clone(),
                        });
                    if work > task.deadline {
                        work
                    } else {
                        task.deadline.clone()
                    }
                }
            };
            let index = clocks.len() + 1;
            match (aux.kind, aux.slot) {
                (TaskProductClockKind::Arrival, _) => arrival[aux.task_index] = Some(index),
                (TaskProductClockKind::Jitter, Some(slot)) => {
                    jitters.insert((aux.task_index, slot), index);
                }
                (TaskProductClockKind::Service, Some(slot)) => {
                    services.insert((aux.task_index, slot), index);
                }
                (TaskProductClockKind::Deadline, Some(slot)) => {
                    if let Some(service) = services.get(&(aux.task_index, slot)).copied() {
                        slots[aux.task_index].push(SlotClocks {
                            jitter: jitters.get(&(aux.task_index, slot)).copied(),
                            service,
                            deadline: index,
                        });
                    }
                }
                _ => {}
            }
            clocks.push(AutomatonClock {
                name: aux.name.clone(),
                role: ClockRole::Task(aux),
                ceiling,
            });
        }
        Self {
            product,
            clocks,
            source_places,
            arrival,
            slots,
        }
    }

    /// The task product this automaton lowers.
    pub fn product(&self) -> &'t TaskProductSystem<'s, 'a> {
        self.product
    }

    /// Every clock in FR-418 order; DBM index `i + 1` is `clocks()[i]`.
    pub fn clocks(&self) -> &[AutomatonClock] {
        &self.clocks
    }

    /// The number of leading source clock places in [`Self::clocks`].
    pub fn source_clocks(&self) -> usize {
        self.source_places
    }

    /// The DBM index of `task_index`'s arrival clock, when it has one.
    pub fn arrival_clock(&self, task_index: usize) -> Option<usize> {
        self.arrival.get(task_index).copied().flatten()
    }

    /// The DBM indices of a slot's jitter (when the task has jitter), service
    /// and deadline clocks.
    pub fn slot_clocks(
        &self,
        task_index: usize,
        slot: u64,
    ) -> Option<(Option<usize>, usize, usize)> {
        let slot = usize::try_from(slot).ok()?;
        self.slots
            .get(task_index)?
            .get(slot)
            .map(|clocks| (clocks.jitter, clocks.service, clocks.deadline))
    }

    /// Whether a closed-finite-product certificate (`closed-finite-product/v1`)
    /// can cover this task set. A dual-WCET set with a mode switch is not
    /// certified (ruling U6); the checker's policy check refuses it from this
    /// value.
    ///
    /// # Errors
    ///
    /// The [`CertificationRefusal`] naming why not.
    pub fn certification(&self) -> Result<(), CertificationRefusal> {
        let plan = self.product.plan();
        let dual = plan
            .task_set
            .tasks
            .iter()
            .any(|task| matches!(task.wcet, Wcet::Dual { .. }));
        if dual && !plan.mode_switches.is_empty() {
            return Err(CertificationRefusal::DualWcetModeSwitch);
        }
        if !plan.has_fixed_work_demand() {
            return Err(CertificationRefusal::NotFixedWork);
        }
        if matches!(
            plan.policy,
            SchedulingPolicy::FixedPriority | SchedulingPolicy::Amc
        ) {
            let tasks = &plan.task_set.tasks;
            for (index, task) in tasks.iter().enumerate() {
                if tasks[..index]
                    .iter()
                    .any(|other| other.priority == task.priority)
                {
                    return Err(CertificationRefusal::EqualPriorities);
                }
            }
        }
        Ok(())
    }

    /// The diagonal predicates the guards read, whose truth the refined-region
    /// relation preserves: under EDF, for each pair of tasks' slots in clock
    /// order, `d_i - d_j > D_i - D_j` and `d_i - d_j >= D_i - D_j` over the
    /// deadline clocks. Fixed priority reads none.
    pub fn diagonals(&self) -> Vec<AutomatonDiagonal> {
        let plan = self.product.plan();
        let mut found = Vec::new();
        if plan.policy != SchedulingPolicy::Edf {
            return found;
        }
        let mut deadlines = Vec::new();
        for (task_index, slots) in self.slots.iter().enumerate() {
            for slot in slots {
                deadlines.push((task_index, slot.deadline));
            }
        }
        deadlines.sort_by_key(|(_, index)| *index);
        for (position, (left_task, left)) in deadlines.iter().enumerate() {
            for (right_task, right) in &deadlines[position + 1..] {
                if left_task == right_task {
                    continue;
                }
                let constant = plan.task_set.tasks[*left_task]
                    .deadline
                    .sub(&plan.task_set.tasks[*right_task].deadline);
                for strict in [true, false] {
                    found.push(AutomatonDiagonal {
                        left: *left,
                        strict,
                        right: *right,
                        constant: constant.clone(),
                    });
                }
            }
        }
        found
    }

    /// The initial locations, one per initial source state in FR-361 order,
    /// each with its initial valuation (index `0` is the reference clock).
    ///
    /// # Errors
    ///
    /// [`TaskAutomatonRefusal::Key`] when a source state has no key.
    pub fn initial(&self) -> Result<Vec<(TaskLocation, Vec<Rational>)>, TaskAutomatonRefusal> {
        let tasks = self.product.plan().tasks.len();
        let mut found = Vec::new();
        for state in self.product.source().initial() {
            let (_, source_key) = self.product.source().system().state_key(state.discrete())?;
            let location = TaskLocation {
                source: state.discrete().clone(),
                source_key,
                control: TaskControl {
                    arrival_started: vec![false; tasks],
                    jobs: Vec::new(),
                    mode: CriticalityMode::Lo,
                    missed: false,
                },
            };
            let mut v = vec![zero(); self.clocks.len() + 1];
            for (place, value) in state.clocks().values().iter().enumerate() {
                v[place + 1] = value.clone();
            }
            let mut live = vec![true; self.clocks.len() + 1];
            for (index, place) in self.product.source().places().iter().enumerate() {
                live[index + 1] = self.product.source().live(place, &location.source);
            }
            for (index, value) in v.iter_mut().enumerate() {
                if !live[index] {
                    *value = zero();
                }
            }
            found.push((location, v));
        }
        Ok(found)
    }

    fn slot_of(&self, task_index: usize, slot: u64) -> Result<SlotClocks, TaskAutomatonRefusal> {
        usize::try_from(slot)
            .ok()
            .and_then(|slot| self.slots.get(task_index)?.get(slot))
            .copied()
            .ok_or(TaskAutomatonRefusal::Layout)
    }

    fn job_clocks(&self, job: &LiveJob) -> Result<SlotClocks, TaskAutomatonRefusal> {
        self.slot_of(job.task_index, job.slot)
    }

    fn ceiling(&self, clock: usize) -> Result<Rational, TaskAutomatonRefusal> {
        clock
            .checked_sub(1)
            .and_then(|index| self.clocks.get(index))
            .map(|clock| clock.ceiling.clone())
            .ok_or(TaskAutomatonRefusal::Layout)
    }

    fn work(&self, job: &LiveJob) -> Rational {
        self.product.plan().task_set.tasks[job.task_index]
            .fixed_work()
            .cloned()
            .unwrap_or_else(zero)
    }

    /// Time at `location`: its invariant, urgency and live clocks.
    ///
    /// # Errors
    ///
    /// [`TaskAutomatonRefusal`] when the set is uncertified or the source
    /// stops.
    pub fn location_info(
        &self,
        location: &TaskLocation,
    ) -> Result<LocationInfo, TaskAutomatonRefusal> {
        let plan = self.product.plan();
        let source = self.product.source();
        let mut invariant = Vec::new();
        for constraint in source
            .invariants(&location.source)
            .map_err(TaskAutomatonRefusal::Source)?
        {
            invariant.extend(place_atoms(&constraint));
        }
        let mut live = vec![false; self.clocks.len() + 1];
        live[0] = true;
        for (index, place) in source.places().iter().enumerate() {
            live[index + 1] = source.live(place, &location.source);
        }
        for (task_index, started) in location.control.arrival_started.iter().enumerate() {
            let Some(clock) = self.arrival_clock(task_index) else {
                continue;
            };
            if !started {
                continue;
            }
            live[clock] = true;
            let task = &plan.task_set.tasks[task_index];
            if let fsl_semantics::check::Arrival::Periodic { period } = &task.arrival {
                invariant.push(ClockAtom::upper(clock, false, period.clone()));
            }
        }
        for job in &location.control.jobs {
            let clocks = self.job_clocks(job)?;
            let task = &plan.task_set.tasks[job.task_index];
            live[clocks.deadline] = true;
            invariant.push(ClockAtom::upper(
                clocks.deadline,
                false,
                task.deadline.clone(),
            ));
            if job.phase == JobPhase::Pending {
                if let Some(jitter) = clocks.jitter {
                    live[jitter] = true;
                    invariant.push(ClockAtom::upper(jitter, false, task.jitter.clone()));
                }
            }
            if job.started {
                live[clocks.service] = true;
            }
            if job.phase == JobPhase::Running {
                invariant.push(ClockAtom::upper(clocks.service, false, self.work(job)));
            }
        }
        if location.control.missed {
            // A deadline miss is terminal: time stops and nothing follows.
            return Ok(LocationInfo {
                invariant: Vec::new(),
                time_stopped: true,
                urgent: Guard::falsity(),
                live,
            });
        }
        let time_stopped = source
            .is_urgent(&location.source)
            .map_err(TaskAutomatonRefusal::Source)?;
        let urgent = self.scheduler_edges(location)?.iter().fold(
            Guard::falsity(),
            |union, edge| match &edge.kind {
                EdgeKind::Scheduler(template)
                    if template.action != TaskTransitionAction::Release =>
                {
                    union.or(&edge.guard)
                }
                _ => union,
            },
        );
        Ok(LocationInfo {
            invariant,
            time_stopped,
            urgent,
            live,
        })
    }

    /// Every edge enabled by some valuation at `location`, with the
    /// attempted allocations past a slot bound.
    ///
    /// # Errors
    ///
    /// [`TaskAutomatonRefusal`] when the set is uncertified or the source
    /// stops.
    pub fn successors(&self, location: &TaskLocation) -> Result<Successors, TaskAutomatonRefusal> {
        let (mut edges, overflows) = self.source_edges(location)?;
        edges.extend(self.scheduler_edges(location)?);
        Ok(Successors { edges, overflows })
    }

    /// The valuation after `edge` fires from `v`, or `None` when the guard,
    /// a subtraction's range or the target invariant fails. Dead clocks of
    /// the target are held at zero.
    ///
    /// # Errors
    ///
    /// As [`Self::location_info`] for the target.
    pub fn fire(
        &self,
        edge: &TaskEdge,
        v: &[Rational],
    ) -> Result<Option<Vec<Rational>>, TaskAutomatonRefusal> {
        if !edge.guard.holds(v) {
            return Ok(None);
        }
        let mut next = v.to_vec();
        for carry in &edge.carries {
            next[carry.to] = v[carry.from].clone();
        }
        for reset in &edge.resets {
            next[reset.clock] = reset.value.clone();
        }
        for subtraction in &edge.subtractions {
            let value = &v[subtraction.clock];
            if value < &subtraction.amount || value > &subtraction.ceiling {
                return Ok(None);
            }
            next[subtraction.clock] = value.sub(&subtraction.amount);
        }
        let info = self.location_info(&edge.target)?;
        normalise(&info, &mut next);
        Ok(info.invariant_holds(&next).then_some(next))
    }

    /// The location and valuation of a concrete task-product state: the
    /// abstraction the cross-check compares edges against. Arrival clocks are
    /// elapsed time since the last activation, deadline clocks elapsed time
    /// since nominal activation, and a started job's service clock its
    /// received service plus the service of each started job that precedes it
    /// in policy order and is live, which is what completions have yet to
    /// subtract.
    ///
    /// # Errors
    ///
    /// [`TaskAutomatonRefusal::Key`] when the source state has no key.
    pub fn abstract_state(
        &self,
        state: &TaskProductState,
    ) -> Result<(TaskLocation, Vec<Rational>), TaskAutomatonRefusal> {
        let (_, source_key) = self
            .product
            .source()
            .system()
            .state_key(state.source().discrete())?;
        let mut order: Vec<usize> = (0..state.jobs().len()).collect();
        order.sort_by(|left, right| {
            let (left, right) = (&state.jobs()[*left], &state.jobs()[*right]);
            left.task_index
                .cmp(&right.task_index)
                .then_with(|| left.activated_at.cmp(&right.activated_at))
                .then_with(|| left.occurrence.cmp(&right.occurrence))
        });
        let running = state.running();
        let started = |job: &super::TaskProductJob| {
            !job.service.is_zero() || running == Some((job.task_index, job.slot))
        };
        let jobs: Vec<LiveJob> = order
            .iter()
            .map(|index| {
                let job = &state.jobs()[*index];
                LiveJob {
                    task_index: job.task_index,
                    slot: job.slot,
                    release: job.release.clone(),
                    phase: if running == Some((job.task_index, job.slot)) {
                        JobPhase::Running
                    } else if job.ready {
                        JobPhase::Ready
                    } else {
                        JobPhase::Pending
                    },
                    started: started(job),
                }
            })
            .collect();
        let location = TaskLocation {
            source: state.source().discrete().clone(),
            source_key,
            control: TaskControl {
                arrival_started: state.arrivals().iter().map(Option::is_some).collect(),
                jobs,
                mode: state.criticality_mode(),
                missed: state.deadline_missed(),
            },
        };
        let mut v = vec![zero(); self.clocks.len() + 1];
        for (place, value) in state.source().clocks().values().iter().enumerate() {
            v[place + 1] = value.clone();
        }
        for (task_index, last) in state.arrivals().iter().enumerate() {
            if let (Some(last), Some(clock)) = (last, self.arrival_clock(task_index)) {
                v[clock] = state.now().sub(last);
            }
        }
        for job in state.jobs() {
            let (jitter, service, deadline) = self
                .slot_clocks(job.task_index, job.slot)
                .ok_or(TaskAutomatonRefusal::Identity)?;
            let age = state.now().sub(&job.activated_at);
            v[deadline] = age.clone();
            if let (Some(jitter), false) = (jitter, job.ready) {
                v[jitter] = age;
            }
            if started(job) {
                let mut value = job.service.clone();
                if running != Some((job.task_index, job.slot)) {
                    for other in state.jobs() {
                        if started(other)
                            && self.product.job_order(other, job).is_lt()
                            && (other.task_index, other.slot) != (job.task_index, job.slot)
                        {
                            value = value.add(&other.service);
                        }
                    }
                }
                v[service] = value;
            }
        }
        let info = self.location_info(&location)?;
        normalise(&info, &mut v);
        Ok((location, v))
    }

    fn template(
        &self,
        action: TaskTransitionAction,
        control: &TaskControl,
        position: usize,
        release: &ModelTransition,
        source_transition: Option<&ModelTransition>,
    ) -> TaskTemplate {
        let plan = self.product.plan();
        let job = &control.jobs[position];
        let task = &plan.task_set.tasks[job.task_index];
        TaskTemplate {
            action,
            task_set: plan.task_set.name.clone(),
            task: task.name.clone(),
            task_index: job.task_index,
            job_rank: control.rank(position),
            slot: job.slot,
            release: release.clone(),
            source_transition: source_transition.cloned(),
            priority: task.priority,
        }
    }

    fn source_edges(
        &self,
        location: &TaskLocation,
    ) -> Result<(Vec<TaskEdge>, Vec<Overflow>), TaskAutomatonRefusal> {
        self.certification()?;
        if location.control.missed {
            return Err(TaskAutomatonRefusal::PostMiss);
        }
        let plan = self.product.plan();
        let mut edges = Vec::new();
        let mut overflows = Vec::new();
        for step in self
            .product
            .source()
            .guarded_steps(&location.source)
            .map_err(TaskAutomatonRefusal::Source)?
        {
            if step.parts().is_empty() {
                continue;
            }
            let mut guard = Guard {
                alternatives: step
                    .parts()
                    .iter()
                    .map(|part| part.iter().flat_map(place_atoms).collect())
                    .collect(),
            };
            let mut control = location.control.clone();
            let mut resets: Vec<ClockReset> = step
                .resets()
                .iter()
                .map(|(place, value)| ClockReset {
                    clock: place + 1,
                    value: value.clone(),
                })
                .collect();
            let carries: Vec<ClockCarry> = step
                .carries()
                .iter()
                .map(|(to, from)| ClockCarry {
                    to: to + 1,
                    from: from + 1,
                })
                .collect();
            let mut released: Vec<(usize, usize)> = Vec::new();
            let mut overflowing = None;
            let mut seen: Vec<usize> = Vec::new();
            for release in plan.releases_for_operation(step.operation()) {
                let Some(task_index) = plan.tasks.iter().position(|bound| {
                    plan.task_set.tasks[bound.task_index].name == release.task.name
                }) else {
                    continue;
                };
                if seen.contains(&task_index) {
                    return Err(TaskAutomatonRefusal::DuplicateRelease);
                }
                seen.push(task_index);
                let task = &plan.task_set.tasks[task_index];
                let Some(arrival) = self.arrival_clock(task_index) else {
                    continue;
                };
                if control.arrival_started[task_index] {
                    guard = guard.and(&spacing_guard(task, arrival));
                }
                let live: Vec<u64> = control
                    .jobs
                    .iter()
                    .filter(|job| job.task_index == task_index)
                    .map(|job| job.slot)
                    .collect();
                let Some(slot) =
                    (0..plan.tasks[task_index].max_jobs).find(|slot| !live.contains(slot))
                else {
                    overflowing.get_or_insert(task_index);
                    continue;
                };
                let clocks = self.slot_of(task_index, slot)?;
                control.arrival_started[task_index] = true;
                resets.push(ClockReset {
                    clock: arrival,
                    value: zero(),
                });
                resets.push(ClockReset {
                    clock: clocks.deadline,
                    value: zero(),
                });
                resets.push(ClockReset {
                    clock: clocks.service,
                    value: zero(),
                });
                let pending = clocks.jitter.is_some();
                if let Some(jitter) = clocks.jitter {
                    resets.push(ClockReset {
                        clock: jitter,
                        value: zero(),
                    });
                }
                let after = control
                    .jobs
                    .iter()
                    .rposition(|job| job.task_index == task_index)
                    .map_or_else(
                        || {
                            control
                                .jobs
                                .iter()
                                .position(|job| job.task_index > task_index)
                                .unwrap_or(control.jobs.len())
                        },
                        |last| last + 1,
                    );
                control.jobs.insert(
                    after,
                    LiveJob {
                        task_index,
                        slot,
                        release: step.transition().clone(),
                        phase: if pending {
                            JobPhase::Pending
                        } else {
                            JobPhase::Ready
                        },
                        started: false,
                    },
                );
                released.push((task_index, after));
            }
            if let Some(task_index) = overflowing {
                overflows.push(Overflow {
                    transition: step.transition().clone(),
                    task_index,
                    guard,
                });
                continue;
            }
            if control.mode == CriticalityMode::Lo && plan.switches_mode_on(step.operation()) {
                control.mode = CriticalityMode::Hi;
            }
            let (_, source_key) = self.product.source().system().state_key(step.post())?;
            let target = TaskLocation {
                source: step.post().clone(),
                source_key,
                control,
            };
            // Template ranks and slots name the new job in the target.
            let mut activations: Vec<TaskTemplate> = released
                .iter()
                .filter_map(|(task_index, _)| {
                    let position = target
                        .control
                        .jobs
                        .iter()
                        .rposition(|job| job.task_index == *task_index)?;
                    Some(self.template(
                        TaskTransitionAction::Activate,
                        &target.control,
                        position,
                        step.transition(),
                        Some(step.transition()),
                    ))
                })
                .collect();
            activations.sort_by(|left, right| left.task.cmp(&right.task));
            edges.push(TaskEdge {
                kind: EdgeKind::Source {
                    transition: step.transition().clone(),
                    activations,
                },
                guard,
                resets,
                carries,
                subtractions: Vec::new(),
                target,
            });
        }
        Ok((edges, overflows))
    }

    /// Whether the job at `better` precedes the job at `worse` in policy
    /// order, as a guard over deadline clocks.
    fn before(
        &self,
        control: &TaskControl,
        better: usize,
        worse: usize,
    ) -> Result<Guard, TaskAutomatonRefusal> {
        let plan = self.product.plan();
        let (left, right) = (&control.jobs[better], &control.jobs[worse]);
        if left.task_index == right.task_index {
            // The same task's jobs are ordered by age: the list order.
            return Ok(if better < worse {
                Guard::truth()
            } else {
                Guard::falsity()
            });
        }
        let (left_task, right_task) = (
            &plan.task_set.tasks[left.task_index],
            &plan.task_set.tasks[right.task_index],
        );
        Ok(match plan.policy {
            SchedulingPolicy::FixedPriority | SchedulingPolicy::Amc => {
                if left_task.priority < right_task.priority {
                    Guard::truth()
                } else {
                    Guard::falsity()
                }
            }
            SchedulingPolicy::Edf => {
                let (left_clock, right_clock) = (
                    self.job_clocks(left)?.deadline,
                    self.job_clocks(right)?.deadline,
                );
                // Absolute deadline is `now - d + D`, so the left job's is
                // earlier exactly when `d_l - d_r > D_l - D_r`.
                let constant = left_task.deadline.sub(&right_task.deadline);
                let ties_to_left = left_task.name < right_task.name;
                Guard::conjunction(vec![ClockAtom {
                    left: right_clock,
                    right: left_clock,
                    strict: !ties_to_left,
                    bound: constant.neg(),
                }])
            }
        })
    }

    fn scheduler_edges(
        &self,
        location: &TaskLocation,
    ) -> Result<Vec<TaskEdge>, TaskAutomatonRefusal> {
        self.certification()?;
        if location.control.missed {
            return Err(TaskAutomatonRefusal::PostMiss);
        }
        let control = &location.control;
        let mut edges = Vec::new();
        let edge = |kind: TaskTemplate,
                    guard: Guard,
                    resets: Vec<ClockReset>,
                    subtractions: Vec<BoundedSubtraction>,
                    target: TaskControl| TaskEdge {
            kind: EdgeKind::Scheduler(kind),
            guard,
            resets,
            carries: Vec::new(),
            subtractions,
            target: TaskLocation {
                source: location.source.clone(),
                source_key: location.source_key.clone(),
                control: target,
            },
        };
        // A pending job becomes ready at any instant within its jitter.
        for (position, job) in control.jobs.iter().enumerate() {
            if job.phase != JobPhase::Pending {
                continue;
            }
            let mut target = control.clone();
            target.jobs[position].phase = JobPhase::Ready;
            edges.push(edge(
                self.template(
                    TaskTransitionAction::Release,
                    control,
                    position,
                    &job.release,
                    None,
                ),
                Guard::truth(),
                Vec::new(),
                Vec::new(),
                target,
            ));
        }
        // Raw urgent guards, then each made exclusive of the higher classes.
        let mut miss_raw = Vec::new();
        for job in &control.jobs {
            let clocks = self.job_clocks(job)?;
            let task = &self.product.plan().task_set.tasks[job.task_index];
            let mut atoms = vec![ClockAtom::lower(clocks.deadline, false, &task.deadline)];
            if job.phase == JobPhase::Running {
                atoms.push(ClockAtom::upper(clocks.service, true, self.work(job)));
            }
            miss_raw.push(Guard::conjunction(atoms));
        }
        let miss_any = miss_raw
            .iter()
            .fold(Guard::falsity(), |union, guard| union.or(guard));
        let mut earlier = Guard::falsity();
        for (position, raw) in miss_raw.iter().enumerate() {
            let job = &control.jobs[position];
            let mut target = control.clone();
            target.missed = true;
            edges.push(edge(
                self.template(
                    TaskTransitionAction::DeadlineMiss,
                    control,
                    position,
                    &job.release,
                    None,
                ),
                raw.and(&earlier.negated()),
                Vec::new(),
                Vec::new(),
                target,
            ));
            earlier = earlier.or(raw);
        }
        let running = control
            .jobs
            .iter()
            .position(|job| job.phase == JobPhase::Running);
        let not_miss = miss_any.negated();
        let mut complete_raw = Guard::falsity();
        if let Some(position) = running {
            let job = &control.jobs[position];
            let clocks = self.job_clocks(job)?;
            let work = self.work(job);
            complete_raw = Guard::conjunction(vec![ClockAtom::lower(clocks.service, false, &work)]);
            let mut target = control.clone();
            target.jobs.remove(position);
            let template = self.template(
                TaskTransitionAction::Complete,
                control,
                position,
                &job.release,
                None,
            );
            let subtractions = control
                .jobs
                .iter()
                .enumerate()
                .filter(|(other, candidate)| *other != position && candidate.started)
                .map(|(_, candidate)| {
                    let service = self.job_clocks(candidate)?.service;
                    Ok(BoundedSubtraction {
                        clock: service,
                        amount: work.clone(),
                        ceiling: self.ceiling(service)?,
                        template: template.clone(),
                    })
                })
                .collect::<Result<Vec<_>, TaskAutomatonRefusal>>()?;
            edges.push(edge(
                template,
                complete_raw.and(&not_miss),
                Vec::new(),
                subtractions,
                target,
            ));
        }
        let eligible: Vec<usize> = control
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| job.phase != JobPhase::Pending)
            .map(|(position, _)| position)
            .collect();
        let best = |position: usize| -> Result<Guard, TaskAutomatonRefusal> {
            let mut guard = Guard::truth();
            for other in eligible.iter().filter(|other| **other != position) {
                guard = guard.and(&self.before(control, position, *other)?);
            }
            Ok(guard)
        };
        let exclusive = not_miss.and(&complete_raw.negated());
        if let Some(position) = running {
            let job = &control.jobs[position];
            let mut better = Guard::falsity();
            for other in eligible.iter().filter(|other| **other != position) {
                better = better.or(&best(*other)?);
            }
            let clocks = self.job_clocks(job)?;
            let template = self.template(
                TaskTransitionAction::Preempt,
                control,
                position,
                &job.release,
                None,
            );
            for (zero_service, keeps_clock) in [(true, false), (false, true)] {
                let atom = if zero_service {
                    ClockAtom::upper(clocks.service, false, zero())
                } else {
                    ClockAtom::lower(clocks.service, true, &zero())
                };
                let mut target = control.clone();
                target.jobs[position].phase = JobPhase::Ready;
                target.jobs[position].started = keeps_clock;
                edges.push(edge(
                    template.clone(),
                    better.and(&exclusive).and(&Guard::conjunction(vec![atom])),
                    Vec::new(),
                    Vec::new(),
                    target,
                ));
            }
        } else {
            for position in &eligible {
                let job = &control.jobs[*position];
                let clocks = self.job_clocks(job)?;
                let guard = best(*position)?.and(&not_miss);
                let mut target = control.clone();
                target.jobs[*position].phase = JobPhase::Running;
                target.jobs[*position].started = true;
                if job.started {
                    for (action, atom) in [
                        (
                            TaskTransitionAction::Dispatch,
                            ClockAtom::upper(clocks.service, false, zero()),
                        ),
                        (
                            TaskTransitionAction::Resume,
                            ClockAtom::lower(clocks.service, true, &zero()),
                        ),
                    ] {
                        edges.push(edge(
                            self.template(action, control, *position, &job.release, None),
                            guard.and(&Guard::conjunction(vec![atom])),
                            Vec::new(),
                            Vec::new(),
                            target.clone(),
                        ));
                    }
                } else {
                    edges.push(edge(
                        self.template(
                            TaskTransitionAction::Dispatch,
                            control,
                            *position,
                            &job.release,
                            None,
                        ),
                        guard,
                        vec![ClockReset {
                            clock: clocks.service,
                            value: zero(),
                        }],
                        Vec::new(),
                        target,
                    ));
                }
            }
        }
        Ok(edges)
    }
}

/// Holds every dead clock of `info` at zero.
fn normalise(info: &LocationInfo, v: &mut [Rational]) {
    for (index, value) in v.iter_mut().enumerate() {
        if !info.live[index] {
            *value = zero();
        }
    }
}

fn place_atoms(constraint: &PlaceConstraint) -> Vec<ClockAtom> {
    let clock = constraint.place() + 1;
    let bound = constraint.bound();
    match constraint.op() {
        ClockOp::Less => vec![ClockAtom::upper(clock, true, bound.clone())],
        ClockOp::LessOrEqual => vec![ClockAtom::upper(clock, false, bound.clone())],
        ClockOp::Equal => vec![
            ClockAtom::upper(clock, false, bound.clone()),
            ClockAtom::lower(clock, false, bound),
        ],
        ClockOp::GreaterOrEqual => vec![ClockAtom::lower(clock, false, bound)],
        ClockOp::Greater => vec![ClockAtom::lower(clock, true, bound)],
    }
}

/// The activation spacing a task's next release needs: exactly its period
/// for a periodic task, at least its minimum interarrival for a sporadic one.
fn spacing_guard(task: &fsl_semantics::check::Task, arrival: usize) -> Guard {
    match &task.arrival {
        fsl_semantics::check::Arrival::Periodic { period } => Guard::conjunction(vec![
            ClockAtom::upper(arrival, false, period.clone()),
            ClockAtom::lower(arrival, false, period),
        ]),
        fsl_semantics::check::Arrival::Sporadic { min_interarrival } => {
            Guard::conjunction(vec![ClockAtom::lower(arrival, false, min_interarrival)])
        }
    }
}
