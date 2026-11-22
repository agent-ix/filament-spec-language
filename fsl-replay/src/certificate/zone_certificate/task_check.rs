// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418's check of a fixed-work task-product certificate.
//!
//! The checker rebuilds the task automaton itself ([`TaskAutomaton`], the
//! lowering `fsl-eval` shares with the producer, as `ModelSystem` is shared),
//! explores it with its own difference-bound matrices into the closed
//! refined-region quotient ([`explore_task_product`]), and matches the
//! certificate against that quotient. It reads nothing from the certificate
//! but its nodes, edges, `initials` and `admitted`; every number the
//! exploration uses is recomputed, and the `task_product` member is only
//! compared with the recomputation.
//!
//! The quotient's vertices are (task product key, refined region). A
//! vertex's time successors are its explicit positive-delay edges, so a
//! certificate node's zone is held to be closed under time by checking each
//! vertex it holds against those edges. The admitted witness's cycle must
//! advance time by a fixed positive amount each round (a clock guarded
//! above the value its last in-cycle reset gave it, the ordinary checker's
//! `advancing_cycle` rule), so a Zeno region cycle is not accepted. A
//! `locked` node is rejected on the weaker over-approximation, any
//! quotient cycle through a positive delay reachable from it, which can
//! only reject more.
//!
//! A source operation that releases several tasks at once is one source
//! edge of the lowering with all its activations, so its certificate edge
//! keeps the source identity; an operation releasing one task is the
//! `activate` template edge (the published ZC-19 shape). This is v1,
//! pending arch's ruling on splitting a multi-release operation into one
//! source edge plus a template edge per activation in every interleaving:
//! S3's `EdgeKind::Source` is one atomic edge whose target already holds
//! every activated job, so (a) needs S3 to expose the intermediate
//! locations, and would be a follow-up here.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use filament_exact::{Cancel, Integer, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{components, EdgeLabel, FairnessError, Graph, Identity};
use fsl_eval::simulation::model::{
    AllowancePhase, CertificationRefusal, ClockAtom, ClockRole, EdgeKind, JobPhase, LocationInfo,
    Successors, TaskAutomaton, TaskAutomatonRefusal, TaskEdge, TaskLocation, TaskProductPlan,
    TaskProductSystem, TaskTemplate, TimedSystem,
};
use fsl_eval::simulation::StateKey;
use fsl_semantics::check::CheckedScheduleClaim;

use super::super::task_product::{
    same_source_transition, TaskArrivalPhase, TaskClockCeiling, TaskClosureEvidence,
    TaskClosureMethod, TaskDiagonalConstraint, TaskDiagonalOperator, TaskJobPhase, TaskLiveJob,
    TaskProductCertificate, TaskProductKey, TaskQueueBound, TaskSchedulerControl, TaskSubtraction,
    TaskTransitionTemplate,
};
use super::super::zone::{Coverage, DiagonalPredicate, Exact, TargetShape, Zone, ZoneError};
use super::{
    at_edge, at_node, fault, malformed, reject, scaled, CertEdge, CertificateCheckLimits,
    CertificateLocus, CertificateMember, CertificateRule, CheckerFault, DivergentTarget, Halt,
    IncompleteCause, ModelCheckLimit, Step, ZoneCertificate, ZoneCertificateRefusal,
    ZoneCertificateVerdict,
};

/// The work meter: one unit per elementary step, polling `cancel`.
struct Work<'c> {
    used: Cell<u64>,
    limit: u64,
    cancel: &'c Cancel,
    cancelled: Cell<bool>,
}

impl<'c> Work<'c> {
    fn new(limit: u64, cancel: &'c Cancel) -> Self {
        Self {
            used: Cell::new(0),
            limit,
            cancel,
            cancelled: Cell::new(false),
        }
    }

    /// Spends one unit; `false` when the limit or a cancel stopped the check.
    fn charge(&self) -> bool {
        if self.cancel.poll() {
            self.cancelled.set(true);
            return false;
        }
        let used = self.used.get().saturating_add(1);
        self.used.set(used);
        used <= self.limit
    }

    fn stop<O>(&self) -> Halt<O> {
        if self.cancelled.get() {
            return match self.cancel.cause() {
                Some(source) => Halt::Verdict(ZoneCertificateVerdict::Stopped(
                    IncompleteCause::Cancelled { source },
                )),
                None => fault(CheckerFault::CancellationWithoutCause),
            };
        }
        Halt::Verdict(ZoneCertificateVerdict::Stopped(
            IncompleteCause::LimitReached {
                limit: ModelCheckLimit::ZoneMaxCheckWork,
                value: self.limit,
            },
        ))
    }
}

/// Why the exploration stopped.
#[derive(Debug)]
pub enum QuotientStop {
    /// The work limit or a cancel stopped it.
    Work,
    /// A constant of the model is not an integer at the certificate's scale.
    Scale,
    /// The lowering cannot certify the set, or an allocation past a task's
    /// slot bound is reachable.
    Uncertified,
    /// The lowering refused for another reason.
    Refused(TaskAutomatonRefusal),
    /// The checker broke an invariant of its own.
    Fault(CheckerFault),
}

impl From<ZoneError> for QuotientStop {
    fn from(_: ZoneError) -> Self {
        Self::Fault(CheckerFault::ZoneShape)
    }
}

impl From<TaskAutomatonRefusal> for QuotientStop {
    fn from(refusal: TaskAutomatonRefusal) -> Self {
        match refusal {
            TaskAutomatonRefusal::Uncertified(_) => Self::Uncertified,
            other => Self::Refused(other),
        }
    }
}

/// The identity of a recomputed edge: a source transition, or a finite
/// task-transition template.
#[derive(Clone, Debug)]
pub enum QuotientIdentity {
    /// A source transition that activates at most several tasks at once.
    Source(ModelTransition),
    /// A task-scheduler action or the activation of one task.
    Template(TaskTransitionTemplate),
}

impl QuotientIdentity {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Source(left), Self::Source(right)) => same_source_transition(left, right),
            (Self::Template(left), Self::Template(right)) => left.same_identity(right),
            _ => false,
        }
    }

    /// Whether the certificate edge carries this identity.
    pub fn labels(&self, edge: &CertEdge) -> bool {
        match self {
            Self::Source(transition) => {
                edge.task_transition.is_none()
                    && edge
                        .transition
                        .as_ref()
                        .is_some_and(|own| same_source_transition(own, transition))
            }
            Self::Template(template) => {
                edge.transition.is_none()
                    && edge
                        .task_transition
                        .as_ref()
                        .is_some_and(|own| own.same_identity(template))
            }
        }
    }
}

/// One vertex of the closed quotient: a task product key and a refined
/// region.
#[derive(Clone, Debug)]
pub struct QuotientVertex {
    /// The task product state key octets.
    pub key: StateKey,
    /// The refined region, in FR-418's clock order.
    pub zone: Zone,
    /// Whether the control has taken a deadline miss.
    pub missed: bool,
}

/// A discrete edge of the quotient, by identity index.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct QuotientEdge {
    /// The source vertex.
    pub from: usize,
    /// The index of the edge's identity in [`TaskProductQuotient::identities`].
    pub identity: usize,
    /// The target vertex.
    pub to: usize,
    /// The index of the edge's clock effect in `TaskProductQuotient::effects`.
    pub effect: usize,
}

/// What one enabled alternative of an edge does to the clocks: the lower
/// bounds its guard imposes, the clocks it resets to a constant and the
/// clocks it assigns from another clock. A cycle advances time when a clock
/// is guarded above the value its last reset in the cycle gave it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeEffect {
    /// `(clock, bound, strict)`: the guard requires `clock > bound` when
    /// strict, else `clock >= bound`.
    pub lowers: Vec<(usize, Integer, bool)>,
    /// `(clock, value)`: reset to a constant.
    pub resets: Vec<(usize, Integer)>,
    /// Clocks assigned from another clock, whose value the cycle test does
    /// not know.
    pub assigned: Vec<usize>,
}

/// One initial symbolic state and the regions it meets.
#[derive(Clone, Debug)]
pub struct QuotientInitial {
    /// The task product key.
    pub key: StateKey,
    /// The zone of the zero valuation held to its invariants.
    pub zone: Zone,
    /// The vertices of the regions the zone meets.
    pub vertices: Vec<usize>,
}

/// The checker's own closed refined-region quotient of a task automaton.
#[derive(Clone, Debug)]
pub struct TaskProductQuotient {
    /// The vertices, in discovery order.
    pub vertices: Vec<QuotientVertex>,
    /// The distinct edge identities.
    pub identities: Vec<QuotientIdentity>,
    /// The distinct clock effects of the discrete edges.
    pub effects: Vec<EdgeEffect>,
    /// The zero-delay edges.
    pub discrete: Vec<QuotientEdge>,
    /// The positive-delay edges, a vertex to a vertex of the same location.
    pub time: Vec<(usize, usize)>,
    /// The initial symbolic states that admit a valuation.
    pub initials: Vec<QuotientInitial>,
    /// The bounded subtractions of every reachable edge, once each.
    pub subtractions: Vec<TaskSubtraction>,
}

fn template_of(template: &TaskTemplate) -> TaskTransitionTemplate {
    TaskTransitionTemplate {
        action: template.action.into(),
        task_set: template.task_set.clone(),
        task: template.task.clone(),
        job_rank: template.job_rank,
        slot: template.slot,
        release: template.release.clone(),
        source_transition: template.source_transition.clone(),
        priority: template.priority,
    }
}

fn identity_of(edge: &TaskEdge) -> QuotientIdentity {
    match &edge.kind {
        EdgeKind::Source {
            transition,
            activations,
        } => match activations.as_slice() {
            [one] => QuotientIdentity::Template(template_of(one)),
            _ => QuotientIdentity::Source(transition.clone()),
        },
        EdgeKind::Scheduler(template) => QuotientIdentity::Template(template_of(template)),
    }
}

/// The complete source-plus-scheduler product key of a location. Its queue
/// lists the ready jobs in `(task, rank)` order under every policy: the
/// policy order is carried by the region's diagonal truth, not the key.
fn product_key(location: &TaskLocation) -> Option<StateKey> {
    let control = location.control();
    let arrivals = control
        .arrival_started
        .iter()
        .map(|started| {
            if *started {
                TaskArrivalPhase::Arrived
            } else {
                TaskArrivalPhase::NotArrived
            }
        })
        .collect();
    let mut jobs = Vec::new();
    let mut queue = Vec::new();
    for (position, job) in control.jobs.iter().enumerate() {
        let rank = control.rank(position);
        let phase = match job.phase {
            JobPhase::Pending => TaskJobPhase::Pending,
            JobPhase::Ready => TaskJobPhase::Ready,
            JobPhase::Running => TaskJobPhase::Running,
        };
        if phase == TaskJobPhase::Ready {
            queue.push((job.task_index, rank));
        }
        jobs.push(TaskLiveJob {
            task: job.task_index,
            rank,
            slot: job.slot,
            phase,
            started: job.started,
            allowance: AllowancePhase::Lo,
            release: job.release.clone(),
        });
    }
    queue.sort_unstable();
    TaskProductKey::new(
        location.source_key().clone(),
        TaskSchedulerControl {
            arrivals,
            jobs,
            queue,
            mode: control.mode,
            pending_budget_changes: Vec::new(),
            missed: control.missed,
        },
    )
    .ok()?
    .octets()
    .ok()
}

/// A location's time behaviour and edges, by key.
struct LocationData {
    info: LocationInfo,
    successors: Option<Successors>,
}

struct Explorer<'t, 's, 'a, 'c> {
    automaton: &'t TaskAutomaton<'t, 's, 'a>,
    scale: &'c Integer,
    work: &'c Work<'c>,
    dim: usize,
    ceilings: Vec<Integer>,
    diagonals: Vec<DiagonalPredicate>,
    cache: BTreeMap<StateKey, Rc<LocationData>>,
    locations: Vec<TaskLocation>,
    index: BTreeMap<(StateKey, Zone), usize>,
    quotient: TaskProductQuotient,
}

type Explored<T> = Result<T, QuotientStop>;

/// The scaled ceilings and refined-region diagonals of `automaton`.
fn region_parameters(
    automaton: &TaskAutomaton<'_, '_, '_>,
    scale: &Integer,
) -> Explored<(Vec<Integer>, Vec<DiagonalPredicate>)> {
    let mut ceilings = Vec::new();
    for clock in automaton.clocks() {
        ceilings.push(scaled(&clock.ceiling, scale).ok_or(QuotientStop::Scale)?);
    }
    let mut diagonals = Vec::new();
    for diagonal in automaton.diagonals() {
        diagonals.push(DiagonalPredicate {
            left: diagonal.left,
            right: diagonal.right,
            value: scaled(&diagonal.constant, scale).ok_or(QuotientStop::Scale)?,
            // `x - y > c` is the negation of `x - y <= c`, and `>=` of `<`.
            strict: !diagonal.strict,
        });
    }
    Ok((ceilings, diagonals))
}

impl Explorer<'_, '_, '_, '_> {
    fn sc(&self, value: &Rational) -> Explored<Integer> {
        scaled(value, self.scale).ok_or(QuotientStop::Scale)
    }

    fn atoms(&self, zone: &mut Zone, atoms: &[ClockAtom]) -> Explored<()> {
        for atom in atoms {
            let bound = self.sc(&atom.bound)?;
            if atom.is_diagonal() {
                zone.constrain_diagonal(atom.left, atom.right, &bound, atom.strict)?;
            } else {
                zone.constrain(atom.left, atom.right, &bound, atom.strict)?;
            }
        }
        Ok(())
    }

    fn regions(&self, zone: &Zone) -> Explored<Vec<Zone>> {
        if zone.is_empty() {
            return Ok(Vec::new());
        }
        zone.region_normalize_charged(&self.ceilings, &self.diagonals, &mut || self.work.charge())?
            .ok_or(QuotientStop::Work)
    }

    fn data(&mut self, location: &TaskLocation) -> Explored<(StateKey, Rc<LocationData>)> {
        let key = product_key(location).ok_or(QuotientStop::Fault(CheckerFault::Automaton))?;
        if let Some(data) = self.cache.get(&key) {
            return Ok((key, Rc::clone(data)));
        }
        let info = self.automaton.location_info(location)?;
        let successors = if location.control().missed {
            None
        } else {
            Some(self.automaton.successors(location)?)
        };
        let data = Rc::new(LocationData { info, successors });
        self.cache.insert(key.clone(), Rc::clone(&data));
        Ok((key, data))
    }

    fn vertex(&mut self, location: &TaskLocation, key: &StateKey, zone: Zone) -> (usize, bool) {
        if let Some(&found) = self.index.get(&(key.clone(), zone.clone())) {
            return (found, false);
        }
        let id = self.quotient.vertices.len();
        self.quotient.vertices.push(QuotientVertex {
            key: key.clone(),
            zone: zone.clone(),
            missed: location.control().missed,
        });
        self.locations.push(location.clone());
        self.index.insert((key.clone(), zone), id);
        (id, true)
    }

    fn identity(&mut self, identity: QuotientIdentity) -> usize {
        match self
            .quotient
            .identities
            .iter()
            .position(|known| known.same(&identity))
        {
            Some(found) => found,
            None => {
                self.quotient.identities.push(identity);
                self.quotient.identities.len() - 1
            }
        }
    }

    fn pin_dead(&self, zone: &mut Zone, info: &LocationInfo) -> Explored<()> {
        let zero = Integer::zero();
        for clock in 1..self.dim {
            if !info.live[clock] {
                zone.reset(clock, &zero)?;
            }
        }
        Ok(())
    }

    fn initial(&mut self) -> Explored<VecDeque<usize>> {
        let mut queue = VecDeque::new();
        for (location, valuation) in self.automaton.initial()? {
            if !self.work.charge() {
                return Err(QuotientStop::Work);
            }
            let (key, data) = self.data(&location)?;
            let mut zone = Zone::zero(self.dim);
            for (clock, entry) in valuation.iter().enumerate().skip(1) {
                let value = self.sc(entry)?;
                if value != Integer::zero() {
                    zone.reset(clock, &value)?;
                }
            }
            self.atoms(&mut zone, &data.info.invariant)?;
            if zone.is_empty() {
                continue;
            }
            let mut vertices = Vec::new();
            for region in self.regions(&zone)? {
                let (id, new) = self.vertex(&location, &key, region);
                vertices.push(id);
                if new {
                    queue.push_back(id);
                }
            }
            self.quotient.initials.push(QuotientInitial {
                key,
                zone,
                vertices,
            });
        }
        Ok(queue)
    }

    fn expand(&mut self, at: usize, queue: &mut VecDeque<usize>) -> Explored<()> {
        if !self.work.charge() {
            return Err(QuotientStop::Work);
        }
        let location = self.locations[at].clone();
        let zone = self.quotient.vertices[at].zone.clone();
        let (key, data) = self.data(&location)?;
        let Some(successors) = &data.successors else {
            return Ok(());
        };
        self.delays(at, &location, &key, &zone, &data.info, queue)?;
        for edge in &successors.edges {
            let identity = self.identity(identity_of(edge));
            for alternative in edge.guard.alternatives() {
                if !self.work.charge() {
                    return Err(QuotientStop::Work);
                }
                let mut post = zone.clone();
                self.atoms(&mut post, alternative)?;
                for subtraction in &edge.subtractions {
                    if post.is_empty() {
                        break;
                    }
                    let amount = self.sc(&subtraction.amount)?;
                    let ceiling = self.sc(&subtraction.ceiling)?;
                    // FR-418: the subtraction of `c` from `x` is proved
                    // guarded by `c <= x <= M_x`; a valuation outside that
                    // range has no completion successor, which the
                    // exploration would otherwise drop silently.
                    if !within_range(&post, subtraction.clock, &amount, &ceiling)? {
                        return Err(QuotientStop::Uncertified);
                    }
                    post.subtract_bounded(subtraction.clock, &amount, &ceiling)?;
                }
                if post.is_empty() {
                    continue;
                }
                self.rows(edge)?;
                let carries: Vec<(usize, usize)> = edge
                    .carries
                    .iter()
                    .map(|carry| (carry.to, carry.from))
                    .collect();
                let mut resets = Vec::new();
                for reset in &edge.resets {
                    resets.push((reset.clock, self.sc(&reset.value)?));
                }
                let effect = self.effect(alternative, &carries, &resets)?;
                post.assign_clocks(&carries, &resets)?;
                let (target_key, target) = self.data(&edge.target)?;
                let zero = Integer::zero();
                for clock in 1..self.dim {
                    if !target.info.live[clock] {
                        post.reset(clock, &zero)?;
                    }
                }
                self.atoms(&mut post, &target.info.invariant)?;
                for region in self.regions(&post)? {
                    let (to, new) = self.vertex(&edge.target, &target_key, region);
                    if new {
                        queue.push_back(to);
                    }
                    self.quotient.discrete.push(QuotientEdge {
                        from: at,
                        identity,
                        to,
                        effect,
                    });
                }
            }
        }
        for overflow in &successors.overflows {
            for alternative in overflow.guard.alternatives() {
                let mut post = zone.clone();
                self.atoms(&mut post, alternative)?;
                if !post.is_empty() {
                    return Err(QuotientStop::Uncertified);
                }
            }
        }
        Ok(())
    }

    /// The index of the clock effect of one enabled alternative.
    fn effect(
        &mut self,
        alternative: &[ClockAtom],
        carries: &[(usize, usize)],
        resets: &[(usize, Integer)],
    ) -> Explored<usize> {
        let mut lowers = Vec::new();
        for atom in alternative {
            // `x >= c` is `0 - x <= -c`.
            if atom.left == 0 && atom.right != 0 {
                lowers.push((atom.right, self.sc(&atom.bound)?.negated(), atom.strict));
            }
        }
        let effect = EdgeEffect {
            lowers,
            resets: resets.to_vec(),
            assigned: carries.iter().map(|(to, _)| *to).collect(),
        };
        if let Some(found) = self.quotient.effects.iter().position(|own| own == &effect) {
            return Ok(found);
        }
        self.quotient.effects.push(effect);
        Ok(self.quotient.effects.len() - 1)
    }

    /// The bounded subtractions of an enabled edge, once each.
    fn rows(&mut self, edge: &TaskEdge) -> Explored<()> {
        for subtraction in &edge.subtractions {
            if !self.work.charge() {
                return Err(QuotientStop::Work);
            }
            let row = TaskSubtraction {
                transition: template_of(&subtraction.template),
                clock: self.automaton.clocks()[subtraction.clock - 1].name.clone(),
                constant: self.sc(&subtraction.amount)?,
            };
            let known = self.quotient.subtractions.iter().any(|own| {
                own.clock == row.clock
                    && own.constant == row.constant
                    && own.transition.same_identity(&row.transition)
            });
            if !known {
                self.quotient.subtractions.push(row);
            }
        }
        Ok(())
    }

    /// The positive-delay edges of the region at `at`: no delay from the part
    /// of it where a scheduler action is enabled or the source blocks time.
    fn delays(
        &mut self,
        at: usize,
        location: &TaskLocation,
        key: &StateKey,
        zone: &Zone,
        info: &LocationInfo,
        queue: &mut VecDeque<usize>,
    ) -> Explored<()> {
        if info.time_stopped {
            return Ok(());
        }
        let mut free = vec![zone.clone()];
        for alternative in info.urgent.alternatives() {
            let mut urgent = Zone::universe(self.dim);
            self.atoms(&mut urgent, alternative)?;
            if urgent.is_empty() {
                continue;
            }
            let mut rest = Vec::new();
            for piece in &free {
                if !self.work.charge() {
                    return Err(QuotientStop::Work);
                }
                rest.extend(piece.minus(&urgent)?);
            }
            free = rest;
        }
        free.retain(|piece| !piece.is_empty());
        if free.is_empty() {
            return Ok(());
        }
        for piece in &free {
            let mut later = piece.clone();
            later.up();
            self.pin_dead(&mut later, info)?;
            self.atoms(&mut later, &info.invariant)?;
            for region in self.regions(&later)? {
                if &region == zone {
                    continue;
                }
                let (to, new) = self.vertex(location, key, region);
                if new {
                    queue.push_back(to);
                }
                self.quotient.time.push((at, to));
            }
        }
        // A region whose every live clock is saturated lets time pass
        // without leaving it.
        let mut saturated = true;
        for clock in 1..self.dim {
            if !info.live[clock] {
                continue;
            }
            let mut within = zone.clone();
            within.constrain(clock, 0, &self.ceilings[clock - 1], false)?;
            if !within.is_empty() {
                saturated = false;
                break;
            }
        }
        if saturated {
            self.quotient.time.push((at, at));
        }
        Ok(())
    }
}

/// Explores `automaton` into its closed refined-region quotient under the
/// work limit `max_work`, polling `cancel`. Constants are read at `scale`.
///
/// # Errors
///
/// [`QuotientStop`] when the work limit or a cancel stops it, a constant is
/// not exact at `scale`, the set cannot be certified, or a lowering step is
/// refused.
pub fn explore_task_product(
    automaton: &TaskAutomaton<'_, '_, '_>,
    scale: &Integer,
    max_work: u64,
    cancel: &Cancel,
) -> Result<TaskProductQuotient, QuotientStop> {
    let work = Work::new(max_work, cancel);
    explore(automaton, scale, &work)
}

fn explore(
    automaton: &TaskAutomaton<'_, '_, '_>,
    scale: &Integer,
    work: &Work<'_>,
) -> Explored<TaskProductQuotient> {
    automaton
        .certification()
        .map_err(|_| QuotientStop::Uncertified)?;
    let (ceilings, diagonals) = region_parameters(automaton, scale)?;
    let mut explorer = Explorer {
        automaton,
        scale,
        work,
        dim: automaton.clocks().len() + 1,
        ceilings,
        diagonals,
        cache: BTreeMap::new(),
        locations: Vec::new(),
        index: BTreeMap::new(),
        quotient: TaskProductQuotient {
            vertices: Vec::new(),
            identities: Vec::new(),
            effects: Vec::new(),
            discrete: Vec::new(),
            time: Vec::new(),
            initials: Vec::new(),
            subtractions: Vec::new(),
        },
    };
    let mut queue = explorer.initial()?;
    while let Some(at) = queue.pop_front() {
        explorer.expand(at, &mut queue)?;
    }
    let mut quotient = explorer.quotient;
    quotient.discrete.sort_unstable();
    quotient.discrete.dedup();
    quotient.time.sort_unstable();
    quotient.time.dedup();
    Ok(quotient)
}

fn unchecked<T, O>() -> Step<T, O> {
    Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
        IncompleteCause::CertificateUnchecked,
    )))
}

/// The ceilings and diagonals the checker recompiles for `automaton`.
fn expected_clock_metadata(
    automaton: &TaskAutomaton<'_, '_, '_>,
    scale: &Integer,
) -> Explored<(Vec<TaskClockCeiling>, Vec<TaskDiagonalConstraint>)> {
    let mut ceilings = Vec::new();
    for clock in automaton.clocks() {
        if matches!(clock.role, ClockRole::Task(_)) {
            ceilings.push(TaskClockCeiling {
                clock: clock.name.clone(),
                maximum: scaled(&clock.ceiling, scale).ok_or(QuotientStop::Scale)?,
            });
        }
    }
    let name = |index: usize| automaton.clocks()[index - 1].name.clone();
    let mut diagonals = Vec::new();
    for diagonal in automaton.diagonals() {
        diagonals.push(TaskDiagonalConstraint {
            left: name(diagonal.left),
            operator: if diagonal.strict {
                TaskDiagonalOperator::Greater
            } else {
                TaskDiagonalOperator::GreaterOrEqual
            },
            right: name(diagonal.right),
            constant: scaled(&diagonal.constant, scale).ok_or(QuotientStop::Scale)?,
        });
    }
    Ok((ceilings, diagonals))
}

fn quotient_halt<O>(stop: QuotientStop, work: &Work<'_>) -> Halt<O> {
    match stop {
        QuotientStop::Work => work.stop(),
        QuotientStop::Scale => Halt::Verdict(ZoneCertificateVerdict::Rejected(
            super::CertificateRejection {
                rule: CertificateRule::Malformed,
                at: CertificateLocus::CertificateMember(CertificateMember::Scale),
            },
        )),
        QuotientStop::Uncertified => Halt::Verdict(ZoneCertificateVerdict::Stopped(
            IncompleteCause::CertificateUnchecked,
        )),
        QuotientStop::Refused(_) => super::fault(CheckerFault::Automaton),
        QuotientStop::Fault(kind) => super::fault(kind),
    }
}

/// Whether every valuation of `zone` has `amount <= clock <= ceiling`, the
/// range a bounded subtraction is proved over; `subtract_bounded` would
/// otherwise clip a valuation outside it away without a successor.
fn within_range(
    zone: &Zone,
    clock: usize,
    amount: &Integer,
    ceiling: &Integer,
) -> Result<bool, ZoneError> {
    let mut range = zone.clone();
    range.constrain(0, clock, &amount.negated(), false)?;
    range.constrain(clock, 0, ceiling, false)?;
    range.includes(zone)
}

/// `max_certificate_edges` and `max_certificate_bytes`, as the ordinary reader
/// applies them before it reads anything.
fn budgets<O>(certificate: &ZoneCertificate<O>, limits: CertificateCheckLimits) -> Step<(), O> {
    let stop = |limit, value| {
        Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
            IncompleteCause::LimitReached { limit, value },
        )))
    };
    let edges = u64::try_from(certificate.edges.len()).unwrap_or(u64::MAX);
    if edges > limits.max_certificate_edges {
        return stop(
            ModelCheckLimit::CertificateMaxEdges,
            limits.max_certificate_edges,
        );
    }
    if super::certificate_bytes(certificate) > limits.max_certificate_bytes {
        return stop(
            ModelCheckLimit::CertificateMaxBytes,
            limits.max_certificate_bytes,
        );
    }
    Ok(())
}

/// The certificate's own shape: members the task-product form carries, and
/// indices within them.
fn shape<O>(certificate: &ZoneCertificate<O>, dim: usize) -> Step<(), O> {
    if certificate.scale <= Integer::zero() {
        return malformed(CertificateMember::Scale);
    }
    if !certificate.lu_bounds.lower.is_empty() || !certificate.lu_bounds.upper.is_empty() {
        return malformed(CertificateMember::Bounds);
    }
    if certificate.initials.is_empty() {
        return malformed(CertificateMember::Initials);
    }
    let nodes = certificate.nodes.len();
    let edges = certificate.edges.len();
    if certificate.initials.iter().any(|&node| node >= nodes)
        || certificate.state_graph.is_some()
        || certificate.refinement.is_some()
        || certificate.nodes.iter().any(|node| {
            node.zone.dim() != dim || node.automaton_state != 0 || node.product.is_some()
        })
    {
        return malformed(CertificateMember::Nodes);
    }
    if certificate.components.is_some() {
        return malformed(CertificateMember::Components);
    }
    if certificate.divergence.is_some() {
        return malformed(CertificateMember::Divergence);
    }
    if certificate.non_zeno.is_some() {
        return reject(
            CertificateRule::NonZenoMismatch,
            CertificateLocus::NonZenoMember,
        );
    }
    if certificate.edges.iter().any(|edge| {
        edge.from >= nodes
            || edge.to >= nodes
            || edge.transition.is_some() == edge.task_transition.is_some()
            || edge.part.is_some()
            || edge.horizon.is_some()
            || edge.silent.is_some()
    }) {
        return malformed(CertificateMember::Edges);
    }
    let Some(admitted) = &certificate.admitted else {
        return malformed(CertificateMember::Admitted);
    };
    let target_ok = match &admitted.target {
        DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => *node < nodes,
        DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
    };
    if admitted.start >= nodes || admitted.path.iter().any(|&edge| edge >= edges) || !target_ok {
        return malformed(CertificateMember::Edges);
    }
    Ok(())
}

/// The static part of the `task_product` member against its recompilation.
fn compare_member<O>(
    automaton: &TaskAutomaton<'_, '_, '_>,
    certificate: &ZoneCertificate<O>,
    work: &Work<'_>,
) -> Step<(), O> {
    let Some(member) = &certificate.task_product else {
        return malformed(CertificateMember::TaskProduct);
    };
    let plan = automaton.product().plan();
    let rows = TaskQueueBound::from_checked_task_set(plan.task_set);
    let cap = rows.as_ref().and_then(|rows| {
        rows.iter()
            .try_fold(0_u64, |sum, row| sum.checked_add(row.max_jobs))
    });
    let (ceilings, diagonals) = expected_clock_metadata(automaton, &certificate.scale)
        .map_err(|stop| quotient_halt(stop, work))?;
    let fixed = rows
        .as_ref()
        .is_some_and(|rows| rows.iter().all(|row| row.work_lower == row.work_upper));
    let matches = member.abstraction == TaskProductCertificate::ABSTRACTION
        && rows.as_ref() == Some(&member.queue_bounds)
        && fixed
        && member.time_progress.method == TaskClosureMethod::ClosedFiniteProduct
        && member.time_progress.closure == TaskClosureEvidence::RecomputeAllReachableSuccessors
        && Some(member.time_progress.live_job_cap) == cap
        && member.clock_ceilings == ceilings
        && member.diagonal_constraints == diagonals;
    if matches {
        Ok(())
    } else {
        malformed(CertificateMember::TaskProduct)
    }
}

/// The certificate's own subtraction table against the reachable one.
fn compare_subtractions<O>(
    certificate: &ZoneCertificate<O>,
    quotient: &TaskProductQuotient,
    work: &Work<'_>,
) -> Step<(), O> {
    let Some(member) = &certificate.task_product else {
        return malformed(CertificateMember::TaskProduct);
    };
    let listed = &member.subtractions;
    let same = |left: &TaskSubtraction, right: &TaskSubtraction| {
        left.clock == right.clock
            && left.constant == right.constant
            && left.transition.same_identity(&right.transition)
    };
    let covered = |from: &[TaskSubtraction], other: &[TaskSubtraction]| -> Step<bool, O> {
        for row in from {
            if !work.charge() {
                return Err(work.stop());
            }
            if !other.iter().any(|own| same(row, own)) {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let equal = listed.len() == quotient.subtractions.len()
        && covered(listed, &quotient.subtractions)?
        && covered(&quotient.subtractions, listed)?;
    if equal {
        Ok(())
    } else {
        malformed(CertificateMember::TaskProduct)
    }
}

/// FSpec FR-418: checks a fixed-work task-product certificate over
/// `automaton`, the checker's own lowering of the claim `identity` named.
pub(super) fn check<O: Clone + Eq>(
    automaton: &TaskAutomaton<'_, '_, '_>,
    certificate: &ZoneCertificate<O>,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Step<(), O> {
    // The policy check: a set the lowering cannot certify settles
    // `certificate-unchecked`, whatever the certificate says.
    if let Err(
        CertificationRefusal::DualWcetModeSwitch
        | CertificationRefusal::NotFixedWork
        | CertificationRefusal::EqualPriorities
        | CertificationRefusal::Jitter,
    ) = automaton.certification()
    {
        return unchecked();
    }
    let work = Work::new(limits.max_check_work, cancel);
    budgets(certificate, limits)?;
    let dim = automaton.clocks().len() + 1;
    shape(certificate, dim)?;
    compare_member(automaton, certificate, &work)?;
    let quotient =
        explore(automaton, &certificate.scale, &work).map_err(|stop| quotient_halt(stop, &work))?;
    compare_subtractions(certificate, &quotient, &work)?;
    let (ceilings, diagonals) = region_parameters(automaton, &certificate.scale)
        .map_err(|stop| quotient_halt(stop, &work))?;
    let mut matcher = Matcher {
        certificate,
        quotient: &quotient,
        work: &work,
        ceilings,
        diagonals,
        shapes: vec![None; certificate.nodes.len()],
        leaving: vec![Vec::new(); certificate.nodes.len()],
        used: vec![false; certificate.edges.len()],
        pairs: BTreeSet::new(),
        initial_nodes: Vec::new(),
    };
    for (index, edge) in certificate.edges.iter().enumerate() {
        matcher.leaving[edge.from].push(index);
    }
    matcher.assign()?;
    matcher.canonical()?;
    let into = quotient_graph(&quotient, &work, cancel)?;
    matcher.locked(&into)?;
    matcher.admitted(&into)
}

struct Matcher<'q, 'c, O> {
    certificate: &'c ZoneCertificate<O>,
    quotient: &'q TaskProductQuotient,
    work: &'q Work<'q>,
    ceilings: Vec<Integer>,
    diagonals: Vec<DiagonalPredicate>,
    /// Whether the node's zone is established as a union of whole regions.
    shapes: Vec<Option<TargetShape>>,
    leaving: Vec<Vec<usize>>,
    used: Vec<bool>,
    /// The `(node, vertex)` pairs the certificate's edges assign.
    pairs: BTreeSet<(usize, usize)>,
    /// The node and vertices of each initial state's regions.
    initial_nodes: Vec<(usize, Vec<usize>)>,
}

impl<O: Clone + Eq> Matcher<'_, '_, O> {
    fn spend(&self) -> Step<(), O> {
        if self.work.charge() {
            Ok(())
        } else {
            Err(self.work.stop())
        }
    }

    /// Whether `node`'s zone is a union of whole refined regions, which the
    /// checker establishes by walking the regions it meets; a node that is
    /// not one is malformed.
    fn shape_of(&mut self, node: usize) -> Step<TargetShape, O> {
        if let Some(shape) = self.shapes[node] {
            return Ok(shape);
        }
        let zone = &self.certificate.nodes[node].zone;
        let coverage = zone.region_covers(
            zone,
            &self.ceilings,
            &self.diagonals,
            TargetShape::Arbitrary,
            &mut || self.work.charge(),
        )?;
        match coverage {
            Coverage::Covered => {
                self.shapes[node] = Some(TargetShape::UnionOfRegions);
                Ok(TargetShape::UnionOfRegions)
            }
            Coverage::NotCovered => malformed(CertificateMember::Nodes),
            Coverage::Exhausted => Err(self.work.stop()),
        }
    }

    /// Whether `zone` closed under the refined-region relation lies in
    /// `node`'s zone.
    fn covers(&mut self, zone: &Zone, node: usize) -> Step<bool, O> {
        let shape = self.shape_of(node)?;
        let coverage = zone.region_covers(
            &self.certificate.nodes[node].zone,
            &self.ceilings,
            &self.diagonals,
            shape,
            &mut || self.work.charge(),
        )?;
        match coverage {
            Coverage::Covered => Ok(true),
            Coverage::NotCovered => Ok(false),
            Coverage::Exhausted => Err(self.work.stop()),
        }
    }

    fn missing<T>(&self, node: usize) -> Step<T, O> {
        reject(CertificateRule::SuccessorMissing, at_node(node))
    }

    /// Initial coverage, then the fixpoint of `(node, vertex)` pairs along
    /// the certificate's edges.
    fn assign(&mut self) -> Step<(), O> {
        let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); self.quotient.vertices.len()];
        for edge in &self.quotient.discrete {
            out[edge.from].push((edge.identity, edge.to));
        }
        let mut delay: Vec<Vec<usize>> = vec![Vec::new(); self.quotient.vertices.len()];
        for &(from, to) in &self.quotient.time {
            delay[from].push(to);
        }
        let initials = &self.quotient.initials;
        if self.certificate.initials.len() > initials.len() {
            return malformed(CertificateMember::Initials);
        }
        let mut queue = VecDeque::new();
        for (position, initial) in initials.iter().enumerate() {
            self.spend()?;
            let missing = || {
                reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(initial.key.clone()),
                )
            };
            let Some(&node) = self.certificate.initials.get(position) else {
                return missing();
            };
            if self.certificate.nodes[node].state_key != initial.key
                || !self.covers(&initial.zone, node)?
            {
                return missing();
            }
            self.initial_nodes.push((node, initial.vertices.clone()));
            for &vertex in &initial.vertices {
                if self.pairs.insert((node, vertex)) {
                    queue.push_back((node, vertex));
                }
            }
        }
        while let Some((node, vertex)) = queue.pop_front() {
            self.spend()?;
            let here = &self.quotient.vertices[vertex];
            if here.missed {
                if self.certificate.nodes[node].locked {
                    return unchecked();
                }
                return reject(CertificateRule::BadState, at_node(node));
            }
            for &next in &delay[vertex] {
                self.spend()?;
                let zone = &self.quotient.vertices[next].zone;
                if !self.covers(zone, node)? {
                    return self.missing(node);
                }
                if self.pairs.insert((node, next)) {
                    queue.push_back((node, next));
                }
            }
            for &(identity, next) in &out[vertex] {
                self.spend()?;
                let successor = &self.quotient.vertices[next];
                let mut hit = None;
                for position in 0..self.leaving[node].len() {
                    self.spend()?;
                    let index = self.leaving[node][position];
                    let edge = &self.certificate.edges[index];
                    let target = edge.to;
                    if self.quotient.identities[identity].labels(edge)
                        && self.certificate.nodes[target].state_key == successor.key
                        && self.covers(&successor.zone, target)?
                    {
                        hit = Some((index, target));
                        break;
                    }
                }
                let Some((edge, target)) = hit else {
                    return self.missing(node);
                };
                self.used[edge] = true;
                if self.pairs.insert((target, next)) {
                    queue.push_back((target, next));
                }
            }
        }
        Ok(())
    }

    /// The certificate holds exactly the reachable closure.
    fn canonical(&self) -> Step<(), O> {
        let reached: BTreeSet<usize> = self.pairs.iter().map(|&(node, _)| node).collect();
        if let Some(node) = (0..self.certificate.nodes.len()).find(|node| !reached.contains(node)) {
            return reject(CertificateRule::UnreachedNode, at_node(node));
        }
        if let Some(edge) = self.used.iter().position(|used| !used) {
            return reject(CertificateRule::ExtraEdge, at_edge(edge));
        }
        Ok(())
    }

    /// A `locked` node must have no time-divergent continuation. Divergence
    /// here is over-approximated by a quotient cycle through a positive
    /// delay (a Zeno cycle counts), which can only reject more: it is sound
    /// for this direction and never decides an acceptance.
    fn locked(&self, into: &Divergence) -> Step<(), O> {
        for &(node, vertex) in &self.pairs {
            self.spend()?;
            if self.certificate.nodes[node].locked && into.reaches[vertex] {
                return reject(CertificateRule::BadState, at_node(node));
            }
        }
        Ok(())
    }

    /// The vertices of `now` and every vertex a run of delays reaches.
    fn close(&self, into: &Divergence, now: &BTreeSet<usize>) -> Step<BTreeSet<usize>, O> {
        let mut all = now.clone();
        let mut stack: Vec<usize> = now.iter().copied().collect();
        while let Some(vertex) = stack.pop() {
            for &next in &into.delay[vertex] {
                self.spend()?;
                if all.insert(next) {
                    stack.push(next);
                }
            }
        }
        Ok(all)
    }

    /// The quotient edges out of `vertex` that the certificate edge `index`
    /// labels and whose pair the certificate holds.
    fn taken<'s>(
        &'s self,
        into: &'s Divergence,
        vertex: usize,
        index: usize,
    ) -> impl Iterator<Item = &'s QuotientEdge> + 's {
        let edge = &self.certificate.edges[index];
        into.by_from[vertex]
            .iter()
            .map(|&position| &self.quotient.discrete[position])
            .filter(move |step| {
                self.quotient.identities[step.identity].labels(edge)
                    && self.pairs.contains(&(edge.to, step.to))
            })
    }

    /// The TD-6 witness, replayed on the quotient.
    fn admitted(&self, into: &Divergence) -> Step<(), O> {
        let Some(witness) = &self.certificate.admitted else {
            return malformed(CertificateMember::Admitted);
        };
        let fails = |node: usize| reject(CertificateRule::WitnessFails, at_node(node));
        let mut at = witness.start;
        let mut now: BTreeSet<usize> = self
            .initial_nodes
            .iter()
            .filter(|(node, _)| *node == at)
            .flat_map(|(_, vertices)| vertices.iter().copied())
            .collect();
        if now.is_empty() {
            return fails(at);
        }
        for &index in &witness.path {
            let edge = &self.certificate.edges[index];
            if edge.from != at || !self.used[index] {
                return fails(at);
            }
            now = self.step(into, &now, index)?;
            at = edge.to;
            if now.is_empty() {
                return fails(at);
            }
        }
        let holds = match &witness.target {
            DivergentTarget::Quiescent(node) => {
                *node == at
                    && self
                        .close(into, &now)?
                        .iter()
                        .any(|&vertex| into.idles[vertex])
            }
            DivergentTarget::Terminal(_) => false,
            DivergentTarget::Cycle(cycle) => self.repeats(into, at, cycle, &now)?,
        };
        if holds {
            Ok(())
        } else {
            fails(at)
        }
    }

    /// The vertices after delay and then the certificate edge `index`.
    fn step(
        &self,
        into: &Divergence,
        now: &BTreeSet<usize>,
        index: usize,
    ) -> Step<BTreeSet<usize>, O> {
        let mut next = BTreeSet::new();
        for vertex in self.close(into, now)? {
            for step in self.taken(into, vertex, index) {
                self.spend()?;
                next.insert(step.to);
            }
        }
        Ok(next)
    }

    /// Whether `cycle` is a closed walk of the quotient through a vertex of
    /// `start` along which time advances by a fixed positive amount each
    /// round: a clock guarded above the value its last reset in the cycle
    /// gave it, so no Zeno cycle (a never-reset clock held open by an
    /// invariant while another is reset every round) passes.
    fn repeats(
        &self,
        into: &Divergence,
        at: usize,
        cycle: &[usize],
        start: &BTreeSet<usize>,
    ) -> Step<bool, O> {
        let edges = &self.certificate.edges;
        let chained = !cycle.is_empty()
            && edges[cycle[0]].from == at
            && cycle
                .windows(2)
                .all(|pair| edges[pair[0]].to == edges[pair[1]].from)
            && edges[cycle[cycle.len() - 1]].to == at
            && cycle.iter().all(|&index| self.used[index]);
        if !chained {
            return Ok(false);
        }
        for &vertex in &self.close(into, start)? {
            let mut effects = Vec::new();
            if self.walk(into, cycle, 0, vertex, vertex, &mut effects)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether the rest of `cycle` from `position` closes at `origin` with an
    /// advancing sequence of clock effects.
    fn walk(
        &self,
        into: &Divergence,
        cycle: &[usize],
        position: usize,
        current: usize,
        origin: usize,
        effects: &mut Vec<usize>,
    ) -> Step<bool, O> {
        let reached = self.close(into, &BTreeSet::from([current]))?;
        if position == cycle.len() {
            return Ok(reached.contains(&origin) && self.advances(effects));
        }
        for vertex in reached {
            for step in self.taken(into, vertex, cycle[position]) {
                self.spend()?;
                effects.push(step.effect);
                let found = self.walk(into, cycle, position + 1, step.to, origin, effects)?;
                effects.pop();
                if found {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Whether some edge of the cycle guards a clock `x >= b` (or `x > b`)
    /// with `b` above the value `r` of the last reset of `x` before it,
    /// going back around the cycle; an assignment from another clock, or no
    /// reset at all, proves nothing.
    fn advances(&self, effects: &[usize]) -> bool {
        let length = effects.len();
        let data = &self.quotient.effects;
        for (position, &effect) in effects.iter().enumerate() {
            for (clock, bound, _) in &data[effect].lowers {
                for back in 1..=length {
                    let earlier = &data[effects[(position + length - back) % length]];
                    if earlier.assigned.contains(clock) {
                        break;
                    }
                    if let Some((_, value)) = earlier
                        .resets
                        .iter()
                        .rev()
                        .find(|(reset, _)| reset == clock)
                    {
                        if bound > value {
                            return true;
                        }
                        break;
                    }
                }
            }
        }
        false
    }
}

/// The quotient's time-divergence facts.
struct Divergence {
    /// Whether a vertex on a cycle through a positive delay is reachable
    /// from the vertex.
    reaches: Vec<bool>,
    /// Whether time can pass at the vertex forever without leaving it.
    idles: Vec<bool>,
    delay: Vec<Vec<usize>>,
    /// The positions in `TaskProductQuotient::discrete` of each vertex's
    /// outgoing edges.
    by_from: Vec<Vec<usize>>,
}

/// The strongly connected components of the quotient, and with them the
/// vertices from which a cycle through a positive delay is reachable.
fn quotient_graph<O>(
    quotient: &TaskProductQuotient,
    work: &Work<'_>,
    cancel: &Cancel,
) -> Step<Divergence, O> {
    let spend = || {
        if work.charge() {
            Ok(())
        } else {
            Err(work.stop())
        }
    };
    let count = quotient.vertices.len();
    let mut graph = Graph::new();
    let vertices: Vec<_> = (0..count)
        .map(|_| graph.add_vertex(std::iter::empty::<Identity>()))
        .collect();
    let mut from: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut delay: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut by_from: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (position, edge) in quotient.discrete.iter().enumerate() {
        spend()?;
        graph
            .add_edge(vertices[edge.from], vertices[edge.to], EdgeLabel::Stutter)
            .map_err(|_| fault(CheckerFault::Graph))?;
        from[edge.to].push(edge.from);
        by_from[edge.from].push(position);
    }
    for &(source, target) in &quotient.time {
        spend()?;
        graph
            .add_edge(vertices[source], vertices[target], EdgeLabel::Stutter)
            .map_err(|_| fault(CheckerFault::Graph))?;
        from[target].push(source);
        delay[source].push(target);
    }
    let within: BTreeSet<_> = vertices.iter().copied().collect();
    let found = components(&graph, &within, cancel).map_err(|error| match error {
        FairnessError::Cancelled => match cancel.cause() {
            Some(source) => Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::Cancelled { source },
            )),
            None => fault(CheckerFault::CancellationWithoutCause),
        },
        _ => fault(CheckerFault::Graph),
    })?;
    let mut component = vec![usize::MAX; count];
    for (number, members) in found.iter().enumerate() {
        for vertex in members.vertices() {
            spend()?;
            component[vertex.index()] = number;
        }
    }
    let mut cyclic = vec![false; found.len()];
    for &(source, target) in &quotient.time {
        spend()?;
        if component[source] == component[target] {
            cyclic[component[source]] = true;
        }
    }
    let mut reaches: Vec<bool> = component.iter().map(|&own| cyclic[own]).collect();
    let mut stack: Vec<usize> = (0..count).filter(|&vertex| reaches[vertex]).collect();
    while let Some(vertex) = stack.pop() {
        for &before in &from[vertex] {
            spend()?;
            if !reaches[before] {
                reaches[before] = true;
                stack.push(before);
            }
        }
    }
    let idles = (0..count)
        .map(|vertex| delay[vertex].contains(&vertex))
        .collect();
    Ok(Divergence {
        reaches,
        idles,
        delay,
        by_from,
    })
}

/// FSpec FR-418: the verdict for the schedulability claim `claim`, whose
/// task product is lowered over `timed` by the checker itself.
pub(super) fn schedulability<'s, 'a, O: Clone + Eq>(
    timed: TimedSystem<'s, 'a>,
    claim: &CheckedScheduleClaim,
    certificate: &ZoneCertificate<O>,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<O>> {
    let product = TaskProductSystem::new(timed, &claim.name)
        .map_err(|refusal| ZoneCertificateRefusal::TaskProductPlan(Box::new(refusal)))?;
    let automaton = TaskAutomaton::new(&product);
    match check(&automaton, certificate, limits, cancel) {
        Ok(()) => Ok(ZoneCertificateVerdict::Accepted),
        Err(Halt::Verdict(verdict)) => Ok(verdict),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

/// The `task_product` member the checker recomputes for `automaton` at
/// `scale` and the reachable subtractions of `quotient`: the candidate a
/// producer states and [`check`] compares member by member. `None` when the
/// set has no finite product (a task without singleton work, an overflowing
/// cap) or a constant is not an integer at `scale`.
pub fn recomputed_member(
    automaton: &TaskAutomaton<'_, '_, '_>,
    scale: &Integer,
    quotient: &TaskProductQuotient,
) -> Option<TaskProductCertificate> {
    let rows = TaskQueueBound::from_checked_task_set(automaton.product().plan().task_set)?;
    let (ceilings, diagonals) = expected_clock_metadata(automaton, scale).ok()?;
    TaskProductCertificate::closed_finite_product(
        rows,
        ceilings,
        diagonals,
        quotient.subtractions.clone(),
    )
}

/// Which check a schedulability certificate takes.
pub(super) enum Route {
    /// `closed-finite-product/v1`: [`schedulability`].
    FiniteProduct,
    /// `closed-no-release-product/v1`: the ordinary source check, with the
    /// release bindings (source operation names) none of which may be
    /// enabled at a reached node.
    NoRelease(Vec<String>),
    /// The member alone settles it.
    Verdict(ZoneCertificateVerdict),
}

/// FR-418: reads the `task_product` member's method and, for the no-release
/// closure, compares the member with its recomputation (ZC-27, ZC-29).
pub(super) fn route<O>(
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedScheduleClaim,
    certificate: &ZoneCertificate<O>,
) -> Result<Route, ZoneCertificateRefusal<O>> {
    let broken = || {
        Ok(Route::Verdict(ZoneCertificateVerdict::Rejected(
            super::CertificateRejection {
                rule: CertificateRule::Malformed,
                at: CertificateLocus::CertificateMember(CertificateMember::TaskProduct),
            },
        )))
    };
    let Some(member) = &certificate.task_product else {
        return broken();
    };
    match member.time_progress.method {
        TaskClosureMethod::ClosedFiniteProduct => Ok(Route::FiniteProduct),
        TaskClosureMethod::ClosedNoReleaseProduct => {
            let plan = TaskProductPlan::new(timed, &claim.name)
                .map_err(|refusal| ZoneCertificateRefusal::TaskProductPlan(Box::new(refusal)))?;
            let rows = TaskQueueBound::from_checked_task_set(plan.task_set);
            let matches = member.abstraction == TaskProductCertificate::ABSTRACTION
                && rows.as_ref() == Some(&member.queue_bounds)
                && !member.queue_bounds.is_empty()
                && member.time_progress.live_job_cap == 0
                && member.time_progress.closure
                    == TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease
                && member.clock_ceilings.is_empty()
                && member.diagonal_constraints.is_empty()
                && member.subtractions.is_empty();
            if !matches {
                return broken();
            }
            Ok(Route::NoRelease(
                plan.releases
                    .iter()
                    .map(|release| release.operation.clone())
                    .collect(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-clock zone `low <= x <= high`.
    fn between(low: i64, high: i64) -> Zone {
        let mut zone = Zone::universe(2);
        zone.constrain(0, 1, &Integer::from(-low), false)
            .expect("lower");
        zone.constrain(1, 0, &Integer::from(high), false)
            .expect("upper");
        zone
    }

    /// FR-418: a bounded subtraction is proved over `c <= x <= M_x`; a zone
    /// reaching outside that range is not within it, so the exploration
    /// settles `Uncertified` instead of clipping the valuation away.
    #[test]
    fn a_zone_outside_the_subtraction_range_is_not_within_it() {
        let (amount, ceiling) = (Integer::from(2_i64), Integer::from(5_i64));
        assert!(within_range(&between(2, 5), 1, &amount, &ceiling).unwrap());
        assert!(within_range(&between(3, 4), 1, &amount, &ceiling).unwrap());
        assert!(!within_range(&between(1, 4), 1, &amount, &ceiling).unwrap());
        assert!(!within_range(&between(3, 6), 1, &amount, &ceiling).unwrap());
        assert!(!within_range(&Zone::universe(2), 1, &amount, &ceiling).unwrap());
    }
}
