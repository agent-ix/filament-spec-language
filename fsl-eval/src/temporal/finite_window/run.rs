// SPDX-License-Identifier: AGPL-3.0-or-later
//! The finite-window evaluator (FR-501): demand each activation's node
//! visits, charging the meter and reading the letters, then lift the formula
//! over the possibility sets and classify the result.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::temporal::facts::ClauseFacts;
use filament_exact::{Charge, ChargePoint, Incomplete, Meter};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::{Activation, CaptureSite, TemporalProfile};

use super::identity::{Block, IdentityStream};
use super::lift::{Candidate, Cell, Lifter, Part, Poss, ScanFrame, Variant};
use super::outcome::{
    Assessment, Basis, IndeterminateReason, Premises, SupportPair, Valuation, WindowEvaluation,
    WindowVerdict,
};
use super::trace::{covers, LowerBoundary, TimestampedTrace};
use crate::temporal::ledger::ProgressClosure;
use crate::temporal::letters::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, UndefinedCause,
};
use crate::temporal::outcome::{TemporalRefusal, UndefinedEvaluation};
use crate::temporal::plan::{fault, slot, Direction, Node, Plan, Window};
use crate::temporal::position::TemporalPosition;
use crate::temporal::window::WindowLimitReached;

const VISIT: ChargePoint = ChargePoint::TemporalVisit;

/// One unit of the work of a finite-window evaluation at the
/// `temporal.visit` charge point, named for the FR-523 counters it spends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowUnit {
    /// A temporal operator node visited at its activation.
    Step,
    /// One position scanned by an operator node.
    Scan,
    /// One atomic `holds` read at an observed position.
    Lookup,
    /// One atomic `holds` read at a silent sample (FR-504), which is no
    /// observation and no evaluated position visit.
    SilentLookup,
    /// A temporal operator node visited at a silent sample.
    SilentStep,
    /// One silent sample scanned by an operator node.
    SilentScan,
}

/// Where an evaluation charges its `temporal.visit` units instead of the
/// bare meter (FR-523): the temporal evidence call's counters, which refuse
/// a unit before the meter sees it.
pub trait WindowCharge {
    /// Charge one unit of work.
    ///
    /// # Errors
    ///
    /// The `Incomplete` that stops the evaluation; a refused unit is not
    /// performed.
    fn charge(&mut self, unit: WindowUnit) -> Result<(), Incomplete>;

    /// Charge one aggregate window read against both the call's counters and
    /// this evaluation's meter. Implementations must leave the counters
    /// unchanged when the meter refuses the visit.
    fn aggregate_window_visit(&mut self, meter: &mut Meter) -> Result<(), Incomplete>;
}

/// Why a run stopped before it finished every activation.
enum Stop {
    Incomplete(Incomplete),
    WindowLimit(WindowLimitReached),
    /// An `on each` guard that cannot be established at a position.
    GuardUnresolved(TemporalPosition),
    Undefined(UndefinedEvaluation),
    /// A `holds` condition, an `on each` guard or a capture was refused at
    /// this position (FR-327 "Refused condition").
    /// The atom class for a refused `holds` condition, or `None` for an
    /// `on each` guard or a capture.
    ConditionRefused(TemporalPosition, Option<usize>),
    Refused(TemporalRefusal),
}

impl Stop {
    /// The stop of a letter that failed at `at`: its refusal is the refused
    /// condition at `at`.
    fn at(stop: LetterStop, at: TemporalPosition, atom_class: Option<usize>) -> Self {
        match stop {
            LetterStop::Refused => Self::ConditionRefused(at, atom_class),
            other => other.into(),
        }
    }
}

impl From<Incomplete> for Stop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<InternalFault> for Stop {
    fn from(internal: InternalFault) -> Self {
        Self::Refused(TemporalRefusal::Fault(internal))
    }
}

impl From<LetterStop> for Stop {
    fn from(stop: LetterStop) -> Self {
        match stop {
            LetterStop::Incomplete(incomplete) => Self::Incomplete(incomplete),
            LetterStop::WindowLimit(limit) => Self::WindowLimit(limit),
            LetterStop::Window(refusal) => Self::Refused(TemporalRefusal::Window(refusal)),
            LetterStop::Fault(internal) => Self::Refused(TemporalRefusal::Fault(internal)),
            // Every read of a letter goes through `Stop::at`, which knows
            // the position the refusal is returned at.
            LetterStop::Refused => Self::Refused(TemporalRefusal::Fault(fault(
                "refused-condition-has-its-position",
            ))),
        }
    }
}

/// How an activation ended.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Ending {
    True,
    False,
    Pending { deadline: u64 },
    Indeterminate(IndeterminateReason),
}

pub(super) struct Finished {
    pub(super) anchor: usize,
    result: Ending,
    closure: Vec<(usize, usize)>,
    pub(super) established: bool,
    poss: Poss,
    /// The length of the run's journal when the activation ended.
    pub(super) journal: usize,
}

/// An activation summarized once no later position can change it (FR-820):
/// its result and its decision support.
#[derive(Clone, Debug)]
pub(super) struct Summary {
    anchor: usize,
    result: Ending,
    support: Vec<SupportPair>,
}

/// What the activations before the evaluated ones contributed: nothing for a
/// batch run, the summarized activations of a carried state (FR-820).
#[derive(Clone, Debug, Default)]
pub(super) struct Prior {
    /// The first activation that ended false.
    first_false: Option<Summary>,
    /// The first activation that ended open, before any false one.
    first_open: Option<Summary>,
    /// The union of the decision supports of the true activations, before
    /// any false or open one.
    holds: std::collections::BTreeSet<SupportPair>,
}

impl Prior {
    /// Add the activation `finished`, which no later position changes.
    pub(super) fn summarize<L: TemporalLetters>(
        &mut self,
        run: &Run<'_, L>,
        finished: &Finished,
    ) -> Result<(), InternalFault> {
        if self.first_false.is_some() {
            return Ok(());
        }
        let summary = || -> Result<Summary, InternalFault> {
            Ok(Summary {
                anchor: finished.anchor,
                result: finished.result.clone(),
                support: run.support(finished)?,
            })
        };
        match finished.result {
            Ending::False => self.first_false = Some(summary()?),
            Ending::Pending { .. } | Ending::Indeterminate(_) => {
                if self.first_open.is_none() {
                    self.first_open = Some(summary()?);
                }
            }
            Ending::True => {
                if self.first_open.is_none() {
                    self.holds.extend(run.support(finished)?);
                }
            }
        }
        Ok(())
    }

    /// The position identities the summaries cite.
    pub(super) fn cited(&self) -> usize {
        let support = |summary: &Option<Summary>| summary.as_ref().map_or(0, |s| s.support.len());
        support(&self.first_false) + support(&self.first_open) + self.holds.len()
    }
}

/// One change to the run's readings, with the value it replaced, so a
/// carried state can put back the readings as they stood after its last
/// summarized activation.
#[derive(Clone, Debug)]
pub(super) enum Entry {
    Cell((usize, usize), Option<Cell>),
    Undefined((usize, usize), Option<UndefinedCause>),
}

/// What ended the activations early, if anything did.
#[derive(Clone, Debug)]
pub(super) enum Tail {
    Done,
    GuardUnresolved(TemporalPosition),
    Undefined(UndefinedEvaluation),
    Incomplete(Incomplete),
    WindowLimit(WindowLimitReached),
}

pub(super) struct Run<'a, L: TemporalLetters> {
    clause: &'a ClauseFacts,
    trace: &'a TimestampedTrace,
    /// The absolute index of the trace's first position.
    base: usize,
    /// The activation anchors the run evaluates.
    anchors: Anchors,
    /// Every change to `cells` and `undefined`, in order.
    pub(super) journal: Vec<Entry>,
    /// The anchor of the activation being evaluated.
    pub(super) current: usize,
    plan: Plan,
    /// Each node's future reach, in ticks: the greatest sum of the upper
    /// bounds of future operators along a nesting path below it.
    reach: Vec<i128>,
    stamps: Vec<i128>,
    margin: i128,
    watermark: i128,
    letters: &'a mut L,
    meter: &'a mut Meter,
    /// The caller's own charging of the visits, when it has one.
    units: Option<&'a mut dyn WindowCharge>,
    visited: HashSet<(usize, usize)>,
    /// The atom each node stands for: nodes whose conditions are the same
    /// atom share one valuation at a position (FSpec FR-160).
    classes: Vec<usize>,
    /// The valuations read, by (position, atom class).
    cells: HashMap<(usize, usize), Cell>,
    undefined: BTreeMap<(usize, usize), UndefinedCause>,
    frames: HashMap<(usize, usize), ScanFrame>,
    /// The `temporal.visit` charges admitted so far.
    visits: u64,
}

/// Evaluate `clause`, selected under
/// `filament.temporal.timestamped-event.finite-window/v1`, over the timestamped
/// `trace` (FR-501).
///
/// A definite `Holds` or `Violated` is returned whenever the window is
/// closed and the evidence through it is complete, or a decisive witness or
/// counterexample every admissible continuation preserves exists;
/// `Pending` and `Indeterminate` only for an obligation that is genuinely
/// unresolved. The charge count depends on the trace and its progress, not on
/// the valuations.
///
/// # Errors
///
/// A [`TemporalRefusal`] for a clause under another profile, equal stamps of
/// positions that take part in an order-sensitive operator without a
/// comparable order key, or a letter that fails.
pub fn evaluate_finite_window<L: TemporalLetters>(
    clause: &ClauseFacts,
    trace: &TimestampedTrace,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<WindowEvaluation, TemporalRefusal> {
    run_window(clause, trace, letters, meter, None, None)
}

/// [`evaluate_finite_window`] for the one activation anchored at the
/// admitted position `anchor` (an `on each` clause activates once per
/// instance, FR-516), with every `temporal.visit` unit charged through
/// `units` before it reaches the meter (FR-523). An `on origin` clause
/// activates at position 0 whatever `anchor` names.
///
/// # Errors
///
/// As [`evaluate_finite_window`].
pub fn evaluate_finite_window_at<L: TemporalLetters>(
    clause: &ClauseFacts,
    trace: &TimestampedTrace,
    anchor: usize,
    letters: &mut L,
    units: &mut dyn WindowCharge,
    meter: &mut Meter,
) -> Result<WindowEvaluation, TemporalRefusal> {
    run_window(clause, trace, letters, meter, Some(units), Some(anchor))
}

fn run_window<'a, L: TemporalLetters>(
    clause: &'a ClauseFacts,
    trace: &'a TimestampedTrace,
    letters: &'a mut L,
    meter: &'a mut Meter,
    units: Option<&'a mut dyn WindowCharge>,
    only: Option<usize>,
) -> Result<WindowEvaluation, TemporalRefusal> {
    let anchors = anchors(clause, trace, only);
    let identity = IdentityStream::default();
    let mut run = Run::new(
        clause,
        trace,
        letters,
        meter,
        units,
        anchors,
        Seed::default(),
    )?;
    let prior = Prior::default();
    let (done, tail) = run.drive(&prior)?;
    run.settle(&prior, &done, tail, &identity)
        .map_err(TemporalRefusal::Fault)
}

/// The activation anchors of `clause` over the whole of `trace`: position 0
/// for an `on origin` clause, every observation for an `on each` one (silent
/// samples are no observation, FR-504), or the one `only` names.
pub(super) fn anchors(
    clause: &ClauseFacts,
    trace: &TimestampedTrace,
    only: Option<usize>,
) -> Anchors {
    match (clause.activation(), only) {
        (Activation::Origin, _) => Anchors {
            range: 0..1,
            guarded: false,
        },
        // An anchored run is one instance's single activation: its guard is
        // the instance's, established when the instance was created.
        (Activation::Each { .. }, Some(anchor)) => Anchors {
            range: anchor..anchor.saturating_add(1),
            guarded: false,
        },
        (Activation::Each { guarded }, None) => Anchors {
            range: 0..trace.observed,
            guarded,
        },
    }
}

/// The activations a run evaluates, and whether each reads its `on each`
/// guard first.
#[derive(Clone, Debug)]
pub(super) struct Anchors {
    pub(super) range: std::ops::Range<usize>,
    pub(super) guarded: bool,
}

/// The readings a run starts from: none for a batch run, those of the
/// summarized activations for a carried state (FR-820).
#[derive(Clone, Debug, Default)]
pub(super) struct Seed {
    pub(super) cells: HashMap<(usize, usize), Cell>,
    pub(super) undefined: BTreeMap<(usize, usize), UndefinedCause>,
}

impl<'a, L: TemporalLetters> Run<'a, L> {
    /// A run of `clause` over `trace` evaluating the activations at
    /// `anchors`, from the readings `seed`.
    ///
    /// # Errors
    ///
    /// A clause under another profile, or a fault.
    pub(super) fn new(
        clause: &'a ClauseFacts,
        trace: &'a TimestampedTrace,
        letters: &'a mut L,
        meter: &'a mut Meter,
        units: Option<&'a mut dyn WindowCharge>,
        anchors: Anchors,
        seed: Seed,
    ) -> Result<Self, TemporalRefusal> {
        if !matches!(
            clause.profile(),
            TemporalProfile::TimestampedEventFiniteWindow
                | TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
        ) {
            return Err(TemporalRefusal::NotFiniteWindowProfile {
                profile: clause.profile(),
            });
        }
        let plan = clause
            .formula()
            .plan()
            .map_err(TemporalRefusal::Fault)?
            .clone();
        let reach = reaches(&plan).map_err(TemporalRefusal::Fault)?;
        let classes = (0..plan.nodes.len())
            .map(|index| {
                let atom = matches!(plan.nodes.get(index), Some(Node::Atom));
                match clause.formula().node_id(index) {
                    Some(id) if atom => letters.atom_class(id),
                    _ => index,
                }
            })
            .collect();
        Ok(Run {
            clause,
            trace,
            base: trace.base,
            current: anchors.range.start,
            anchors,
            journal: Vec::new(),
            plan,
            reach,
            stamps: trace
                .positions
                .iter()
                .map(|position| i128::from(position.stamp))
                .collect(),
            margin: trace.margin(),
            watermark: trace.progress.watermark.inclusive(),
            letters,
            meter,
            units,
            visited: HashSet::new(),
            classes,
            cells: seed.cells,
            undefined: seed.undefined,
            frames: HashMap::new(),
            visits: 0,
        })
    }

    /// Evaluate every activation, in anchor order, after the ones `prior`
    /// summarizes.
    ///
    /// # Errors
    ///
    /// The refusal that ends the run.
    pub(super) fn drive(
        &mut self,
        prior: &Prior,
    ) -> Result<(Vec<Finished>, Tail), TemporalRefusal> {
        let mut done = Vec::new();
        let tail = match self.activations(&mut done) {
            Ok(()) => Tail::Done,
            Err(Stop::Incomplete(incomplete)) => Tail::Incomplete(incomplete),
            Err(Stop::WindowLimit(limit)) => Tail::WindowLimit(limit),
            Err(Stop::GuardUnresolved(at)) => Tail::GuardUnresolved(at),
            Err(Stop::Undefined(undefined)) => Tail::Undefined(undefined),
            // In evaluation order: an atom undefined at an earlier position, or
            // an activation that ended false before the refused condition, is
            // returned instead of it.
            Err(Stop::ConditionRefused(at, atom_class)) => {
                let refused = usize::try_from(at.index()).unwrap_or(usize::MAX);
                // Only an earlier undefined atom or a lower-class atom at the
                // refused position precedes this refusal. `read_atom` can retain
                // lower-class valuations outside the activation's closure, so
                // position alone cannot establish precedence for guards/captures.
                if let Some(((position, _), cause)) =
                    self.undefined.iter().find(|((position, class), _)| {
                        *position < refused
                            || (*position == refused
                                && atom_class.is_some_and(|refused_class| *class < refused_class))
                    })
                {
                    Tail::Undefined(UndefinedEvaluation {
                        at: position_of(*position).map_err(TemporalRefusal::Fault)?,
                        cause: cause.clone(),
                    })
                } else if prior.first_false.is_some()
                    || done
                        .iter()
                        .any(|activation| activation.result == Ending::False)
                {
                    Tail::Done
                } else {
                    return Err(TemporalRefusal::ConditionRefused { at });
                }
            }
            Err(Stop::Refused(refusal)) => return Err(refusal),
        };
        Ok((done, tail))
    }
}

/// The future reach of every node.
pub(super) fn reaches(plan: &Plan) -> Result<Vec<i128>, InternalFault> {
    let mut reach = vec![0_i128; plan.nodes.len()];
    for index in (0..plan.nodes.len()).rev() {
        let node = slot(&plan.nodes, index)?;
        let operands: [Option<usize>; 2] = match node {
            Node::Constant(_) | Node::Atom => [None, None],
            Node::Not(operand) => [Some(operand), None],
            Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                [Some(left), Some(right)]
            }
            Node::Scan(scan) => [scan.guard, Some(scan.witness)],
        };
        let mut deepest = 0_i128;
        for operand in operands.into_iter().flatten() {
            if operand <= index {
                return Err(fault("operands-follow-operator"));
            }
            deepest = deepest.max(
                *reach
                    .get(operand)
                    .ok_or_else(|| fault("plan-index-in-arena"))?,
            );
        }
        let own = match node {
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (Direction::Future, Window::Closed { upper, .. }) => i128::from(upper),
                (Direction::Past, Window::Closed { .. }) => 0,
                (_, Window::From { .. }) => return Err(fault("finite-window-interval-is-bounded")),
            },
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => 0,
        };
        *reach
            .get_mut(index)
            .ok_or_else(|| fault("plan-index-in-arena"))? = deepest.saturating_add(own);
    }
    Ok(reach)
}

/// How far an activation's window reaches from its anchor tick: the
/// greatest sum of the upper bounds of the clause's past operators along a
/// nesting path, and the same sum of its future operators (FR-501
/// "Established window"). An activation anchored at tick `a` reads instants
/// in `[a - past, a + future]`, which names the anchors a late observation
/// falls in (FR-501 "Late-data cutoff").
///
/// # Errors
///
/// [`TemporalRefusal::Fault`] for a clause whose plan does not form.
pub fn window_reach(clause: &ClauseFacts) -> Result<(u64, u64), TemporalRefusal> {
    let plan = clause.formula().plan().map_err(TemporalRefusal::Fault)?;
    let future = reaches(plan).map_err(TemporalRefusal::Fault)?;
    let past = past_reaches(plan).map_err(TemporalRefusal::Fault)?;
    let at_root = |reach: &[i128]| {
        reach
            .get(plan.root)
            .and_then(|value| u64::try_from(*value).ok())
            .ok_or_else(|| TemporalRefusal::Fault(fault("reach-in-u64")))
    };
    Ok((at_root(&past)?, at_root(&future)?))
}

/// The past reach of every node: [`reaches`] with the roles of past and
/// future operators exchanged.
fn past_reaches(plan: &Plan) -> Result<Vec<i128>, InternalFault> {
    let mut reach = vec![0_i128; plan.nodes.len()];
    for index in (0..plan.nodes.len()).rev() {
        let node = slot(&plan.nodes, index)?;
        let operands: [Option<usize>; 2] = match node {
            Node::Constant(_) | Node::Atom => [None, None],
            Node::Not(operand) => [Some(operand), None],
            Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                [Some(left), Some(right)]
            }
            Node::Scan(scan) => [scan.guard, Some(scan.witness)],
        };
        let mut deepest = 0_i128;
        for operand in operands.into_iter().flatten() {
            deepest = deepest.max(slot(&reach, operand)?);
        }
        let own = match node {
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (Direction::Past, Window::Closed { upper, .. }) => i128::from(upper),
                (Direction::Future, Window::Closed { .. }) => 0,
                (_, Window::From { .. }) => return Err(fault("finite-window-interval-is-bounded")),
            },
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => 0,
        };
        *reach
            .get_mut(index)
            .ok_or_else(|| fault("plan-index-in-arena"))? = deepest.saturating_add(own);
    }
    Ok(reach)
}

fn position_of(index: usize) -> Result<TemporalPosition, InternalFault> {
    u64::try_from(index)
        .map(TemporalPosition::new)
        .map_err(|_| fault("position-in-u64"))
}

impl<L: TemporalLetters> Run<'_, L> {
    fn charge(&mut self, unit: WindowUnit) -> Result<(), Stop> {
        match self.units.as_deref_mut() {
            Some(units) => units.charge(unit)?,
            None => self.meter.charge(Charge::new(VISIT))?,
        }
        self.visits = self.visits.saturating_add(1);
        Ok(())
    }

    /// The atom class of `node`: itself unless another node is the same atom.
    fn class(&self, node: usize) -> usize {
        self.classes.get(node).copied().unwrap_or(node)
    }

    fn stamp(&self, position: usize) -> Result<i128, InternalFault> {
        let index = position
            .checked_sub(self.base)
            .ok_or_else(|| fault("position-is-held"))?;
        slot(&self.stamps, index)
    }

    /// The stamp of the held `position`.
    pub(super) fn stamp_at(&self, position: usize) -> Result<i128, InternalFault> {
        self.stamp(position)
    }

    /// One past the last position the run reads.
    fn end(&self) -> usize {
        self.base.saturating_add(self.stamps.len())
    }

    /// Set the valuation of `key`, journaling the one it replaces.
    fn set_cell(&mut self, key: (usize, usize), cell: Cell) {
        let prior = self.cells.insert(key, cell);
        self.journal.push(Entry::Cell(key, prior));
    }

    /// The run's readings as they stood when the journal had `len` entries.
    pub(super) fn seed_at(&self, len: usize) -> Seed {
        let mut seed = Seed {
            cells: self.cells.clone(),
            undefined: self.undefined.clone(),
        };
        for entry in self.journal.iter().skip(len).rev() {
            match entry {
                Entry::Cell(key, Some(cell)) => {
                    seed.cells.insert(*key, *cell);
                }
                Entry::Cell(key, None) => {
                    seed.cells.remove(key);
                }
                Entry::Undefined(key, Some(cause)) => {
                    seed.undefined.insert(*key, cause.clone());
                }
                Entry::Undefined(key, None) => {
                    seed.undefined.remove(key);
                }
            }
        }
        seed
    }

    /// The instants the observation at `position` may lie at.
    fn extent(&self, position: usize) -> Result<super::cover::Extent, InternalFault> {
        position
            .checked_sub(self.base)
            .and_then(|index| self.trace.extent(index))
            .ok_or_else(|| fault("position-in-trace"))
    }

    /// Whether the window of an activation (or sub-instance) at `tick`
    /// reaching `reach` ticks ahead is established (FR-501 "Established
    /// window").
    fn established(&self, tick: i128, reach: i128) -> bool {
        let need = tick.saturating_add(reach).saturating_add(self.margin);
        covers(self.trace.progress.completeness, need)
            && self.watermark >= self.trace.settling(need)
    }

    /// Every activation in position order. `done` receives each activation
    /// that ended before the run stopped.
    fn activations(&mut self, done: &mut Vec<Finished>) -> Result<(), Stop> {
        let guarded = self.anchors.guarded;
        for anchor in self.anchors.range.clone() {
            self.current = anchor;
            let at = position_of(anchor)?;
            if guarded {
                match self
                    .letters
                    .activation(at, &mut *self.meter)
                    .map_err(|stop| Stop::at(stop, at, None))?
                {
                    AtomReading::True => {}
                    AtomReading::False => continue,
                    // A guard that cannot be established is an incomplete
                    // activation, never a false guard (FR-044-AC-7).
                    AtomReading::Undefined(_) => return Err(Stop::GuardUnresolved(at)),
                }
            }
            let captured = match self.clause.capture_site() {
                CaptureSite::Never => false,
                CaptureSite::Origin => anchor == 0,
                CaptureSite::Activations => true,
            };
            if captured {
                if let CaptureReading::Undefined { cause, .. } = self
                    .letters
                    .captures(at, &mut *self.meter)
                    .map_err(|stop| Stop::at(stop, at, None))?
                {
                    return Err(Stop::Undefined(UndefinedEvaluation { at, cause }));
                }
            }
            let closure = self.demand(anchor)?;
            let undefined = self.undefined.iter().find(|(key, _)| {
                closure
                    .iter()
                    .any(|(node, at)| *at == key.0 && self.class(*node) == key.1)
            });
            if let Some((&(position, _), cause)) = undefined {
                let undefined = UndefinedEvaluation {
                    at: position_of(position)?,
                    cause: cause.clone(),
                };
                // An activation that is false whatever the undefined atoms
                // read is returned instead of them (FR-327 "Undefined
                // evaluation"): the claim fails at the first refuting
                // position.
                // The letter at an undefined atom's position has no value,
                // and none after it is read (ADR-018 UE-1): only a position
                // before it can refute the activation.
                let saved = self.cells.clone();
                for (key, cell) in &mut self.cells {
                    if key.0 >= position {
                        *cell = Cell::Missing;
                    }
                }
                let finished = self.classify(anchor, closure);
                self.cells = saved;
                let finished = finished?;
                if finished.result != Ending::False {
                    return Err(Stop::Undefined(undefined));
                }
                done.push(finished);
                continue;
            }
            done.push(self.classify(anchor, closure)?);
        }
        Ok(())
    }

    /// Visit every node the activation at `anchor` depends on, charging each
    /// temporal visit once; the visits already made by an earlier activation
    /// are not repeated. Returns the dependency closure, operators before
    /// operands.
    fn demand(&mut self, anchor: usize) -> Result<Vec<(usize, usize)>, Stop> {
        let root = (self.plan.root, anchor);
        let mut queue = VecDeque::from([root]);
        let mut seen: HashSet<(usize, usize)> = HashSet::from([root]);
        let mut closure = Vec::new();
        while let Some((node, position)) = queue.pop_front() {
            closure.push((node, position));
            let mut next: Vec<(usize, usize)> = Vec::new();
            match slot(&self.plan.nodes, node)? {
                Node::Constant(_) => {}
                Node::Atom => {
                    if self.visited.insert((node, position)) {
                        self.read_atom(node, position)?;
                    }
                }
                Node::Not(operand) => next.push((operand, position)),
                Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                    next.push((left, position));
                    next.push((right, position));
                }
                Node::Scan(scan) => {
                    if self.visited.insert((node, position)) {
                        let frame = self.scan(node, position)?;
                        self.frames.insert((node, position), frame);
                    }
                    let frame = self
                        .frames
                        .get(&(node, position))
                        .ok_or_else(|| fault("scan-frame-visited"))?;
                    for candidate in &frame.candidates {
                        next.push((scan.witness, candidate.position));
                        if let Some(guard) = scan.guard {
                            next.push((guard, candidate.position));
                        }
                    }
                }
            }
            for key in next {
                if seen.insert(key) {
                    queue.push_back(key);
                }
            }
        }
        closure.sort_by_key(|key| std::cmp::Reverse(key.0));
        Ok(closure)
    }

    /// Read the valuation of the atom class `class` at the admitted position
    /// `member`, unless it was read.
    fn read_cell(&mut self, member: usize, class: usize) -> Result<(), Stop> {
        if self.cells.contains_key(&(member, class)) {
            return Ok(());
        }
        let id = self
            .clause
            .formula()
            .node_id(class)
            .ok_or_else(|| fault("atom-has-node-id"))?;
        let at = position_of(member)?;
        let Run {
            letters,
            meter,
            units,
            visits,
            ..
        } = self;
        let mut aggregate_visit = |meter: &mut Meter| {
            if let Some(units) = units.as_deref_mut() {
                units
                    .aggregate_window_visit(meter)
                    .map_err(LetterStop::Incomplete)?;
            } else {
                meter.charge(Charge::new(VISIT))?;
            }
            *visits = visits.saturating_add(1);
            Ok(())
        };
        match letters
            .valuation_with_window_visits(id, at, meter, &mut aggregate_visit)
            .map_err(|stop| Stop::at(stop, at, Some(class)))?
        {
            Valuation::True => self.set_cell((member, class), Cell::True),
            Valuation::False => self.set_cell((member, class), Cell::False),
            Valuation::Missing => self.set_cell((member, class), Cell::Missing),
            Valuation::Undefined(cause) => {
                self.set_cell((member, class), Cell::Missing);
                let prior = self.undefined.insert((member, class), cause);
                self.journal.push(Entry::Undefined((member, class), prior));
            }
        }
        Ok(())
    }

    /// Read the atom `node` at `position`. Observations that share a stamp
    /// are readings of one instant, so every position of the stamp is read:
    /// when two of them give differing valuations of the atom, it is
    /// `ConflictingValuation` at every position of that stamp (FR-501
    /// "Verdicts"), whatever order they were listed in.
    fn read_atom(&mut self, node: usize, position: usize) -> Result<(), Stop> {
        let silent = position >= self.trace.observed;
        self.charge(if silent {
            WindowUnit::SilentLookup
        } else {
            WindowUnit::Lookup
        })?;
        let class = self.class(node);
        let stamp = self.stamp(position)?;
        let mut group = Vec::new();
        for (index, candidate) in self.stamps.iter().enumerate() {
            if *candidate == stamp {
                group.push(self.base.saturating_add(index));
            }
        }
        // The letter at a position is the value of every atom there, read in
        // clause-node order (ADR-018 UE-1): the atoms before `class` are read
        // first, so an undefined one is known when `class` is refused.
        let mut classes: Vec<usize> = (0..self.plan.nodes.len())
            .filter(|index| matches!(self.plan.nodes.get(*index), Some(Node::Atom)))
            .map(|index| self.class(index))
            .filter(|other| *other <= class)
            .collect();
        classes.sort_unstable();
        classes.dedup();
        for member in &group {
            for other in &classes {
                self.read_cell(*member, *other)?;
            }
        }
        let held = |cell: Option<&Cell>, want: Cell| cell == Some(&want);
        let differ = group
            .iter()
            .any(|member| held(self.cells.get(&(*member, class)), Cell::True))
            && group
                .iter()
                .any(|member| held(self.cells.get(&(*member, class)), Cell::False));
        if differ {
            for member in &group {
                if let Some(cell @ (Cell::True | Cell::False)) =
                    self.cells.get(&(*member, class)).copied()
                {
                    self.cells.insert((*member, class), Cell::Conflicting);
                    self.journal.push(Entry::Cell((*member, class), Some(cell)));
                }
            }
        }
        Ok(())
    }

    /// One visit of the scan `node` at `position`: scan every position in the
    /// scan's direction (one charge each), keep the ones whose stamp lies in
    /// the interval, and read what a continuation could still add.
    fn scan(&mut self, node: usize, position: usize) -> Result<ScanFrame, Stop> {
        let silent = position >= self.trace.observed;
        self.charge(if silent {
            WindowUnit::SilentStep
        } else {
            WindowUnit::Step
        })?;
        let Node::Scan(scan) = slot(&self.plan.nodes, node)? else {
            return Err(fault("scan-node").into());
        };
        let Window::Closed { lower, upper } = scan.window else {
            return Err(fault("finite-window-interval-is-bounded").into());
        };
        let forward = scan.base.direction() == Direction::Future;
        let order: Vec<usize> = if forward {
            (position..self.end()).collect()
        } else {
            (self.base..=position).rev().collect()
        };
        let anchor = self.stamp(position)?;
        let anchor_extent = self.extent(position)?;
        let mut candidates: Vec<Candidate> = Vec::new();
        for candidate in order {
            self.charge(if candidate >= self.trace.observed {
                WindowUnit::SilentScan
            } else {
                WindowUnit::Scan
            })?;
            let part = match super::cover::Extent::eligible(
                anchor_extent,
                self.extent(candidate)?,
                candidate == position,
                forward,
                lower,
                upper,
            ) {
                super::cover::Containment::True => Part::Sure,
                super::cover::Containment::Indeterminate => Part::Maybe,
                super::cover::Containment::False => continue,
            };
            candidates.push(Candidate {
                position: candidate,
                part,
            });
        }
        if scan.guard.is_some() {
            self.require_order(&candidates)?;
        }
        let first = i128::from(self.trace.first);
        let (unestablished, history) = if forward {
            let reach = *self
                .reach
                .get(node)
                .ok_or_else(|| fault("plan-index-in-arena"))?;
            (!self.established(anchor, reach), false)
        } else {
            (
                candidates.is_empty()
                    && !covers(
                        self.trace.progress.completeness,
                        anchor - i128::from(lower) + self.margin,
                    ),
                self.trace.progress.lower_boundary == LowerBoundary::HistoryCutoff
                    && anchor - i128::from(upper) - self.margin < first,
            )
        };
        Ok(ScanFrame {
            candidates,
            unestablished,
            history,
        })
    }

    /// FSpec FR-090-AC-4: two positions of equal stamp that both take part in
    /// an order-sensitive operator need a usable order (FR-501): distinct keys
    /// of one order authority. A key repeated within its authority orders
    /// nothing, and the observation identity never breaks the tie. The refusal
    /// names the later position.
    ///
    /// Under a bounded cover, and for an observation with its own interval
    /// stamp, two observations whose intervals overlap are such a pair as
    /// equal stamps are (FR-501 "Bounded cover and interval stamps").
    fn require_order(&self, candidates: &[Candidate]) -> Result<(), Stop> {
        let bounded = matches!(
            self.trace.clock.cover,
            super::cover::ClockCover::Bounded { .. }
        );
        for (index, before) in candidates.iter().enumerate() {
            for after in candidates.iter().skip(index + 1) {
                self.require_pair_order(before, after, bounded)?;
            }
        }
        Ok(())
    }

    /// [`Self::require_order`] for one pair of candidates.
    fn require_pair_order(
        &self,
        before: &Candidate,
        after: &Candidate,
        bounded: bool,
    ) -> Result<(), Stop> {
        let interval = |position: usize| {
            position
                .checked_sub(self.base)
                .is_some_and(|index| self.trace.has_interval(index))
        };
        let uncertain = bounded || interval(before.position) || interval(after.position);
        let tied = if uncertain {
            self.extent(before.position)?
                .overlaps(self.extent(after.position)?)
        } else {
            self.stamp(before.position)? == self.stamp(after.position)?
        };
        if !tied {
            return Ok(());
        }
        let key = |position: usize| {
            position
                .checked_sub(self.base)
                .and_then(|index| self.trace.positions.get(index))
                .and_then(|p| p.order_key.as_ref())
        };
        let ordered = matches!(
            (key(before.position), key(after.position)),
            (Some(left), Some(right))
                if left.authority == right.authority && left.key != right.key
        );
        if !ordered {
            return Err(Stop::Refused(TemporalRefusal::UnorderedTie {
                position: position_of(before.position.max(after.position))?,
            }));
        }
        Ok(())
    }

    fn lifter(&self) -> Lifter<'_> {
        Lifter {
            plan: &self.plan,
            classes: &self.classes,
            cells: &self.cells,
            frames: &self.frames,
        }
    }

    /// Lift the activation at `anchor` and classify it.
    fn classify(&self, anchor: usize, closure: Vec<(usize, usize)>) -> Result<Finished, Stop> {
        let root = (self.plan.root, anchor);
        let poss = self
            .lifter()
            .exact(&closure, root, &Variant::default())
            .ok_or_else(|| fault("root-lifts"))?;
        let tick = self.stamp(anchor)?;
        let reach = slot(&self.reach, self.plan.root)?;
        let established = self.established(tick, reach);
        let settles = self
            .trace
            .settling(tick.saturating_add(reach).saturating_add(self.margin));
        // FR-501 "Late-data cutoff": under a cutoff an observation may still
        // arrive on time until `W` covers the window's deadline plus the
        // lateness, so no verdict is reached before then.
        let result = if self.trace.lateness.cutoff.is_some() && self.watermark < settles {
            Ending::Pending {
                deadline: u64::try_from(settles).map_err(|_| fault("deadline-in-u64"))?,
            }
        } else if poss.can_true && !poss.can_false {
            Ending::True
        } else if poss.can_false && !poss.can_true {
            Ending::False
        } else if !poss.can_true && !poss.can_false {
            return Err(Stop::Refused(TemporalRefusal::Fault(fault(
                "possibility-set-is-not-empty",
            ))));
        } else if self.watermark < settles {
            Ending::Pending {
                deadline: u64::try_from(settles).map_err(|_| fault("deadline-in-u64"))?,
            }
        } else {
            Ending::Indeterminate(self.reason(&closure, root)?)
        };
        Ok(Finished {
            anchor,
            result,
            closure,
            established,
            poss,
            journal: self.journal.len(),
        })
    }

    /// Why a closed window's two-valued result stays open: the cause whose
    /// repair would settle it, history first, then completeness, then the
    /// first valuation or straddled bound in position order (FR-501
    /// "Verdicts").
    fn reason(
        &self,
        closure: &[(usize, usize)],
        root: (usize, usize),
    ) -> Result<IndeterminateReason, InternalFault> {
        let lifter = self.lifter();
        let settles = |variant: &Variant| {
            lifter
                .exact(closure, root, variant)
                .is_some_and(Poss::is_singleton)
        };
        let frames = || {
            closure
                .iter()
                .filter_map(|key| self.frames.get(key))
                .collect::<Vec<_>>()
        };
        let any_history = frames().iter().any(|frame| frame.history);
        let any_unestablished = frames().iter().any(|frame| frame.unestablished);
        let repair = |no_history, no_unestablished| Variant {
            no_history,
            no_unestablished,
            ..Variant::default()
        };
        if any_history && settles(&repair(true, false)) {
            return Ok(IndeterminateReason::MissingHistory);
        }
        if any_unestablished && settles(&repair(false, true)) {
            return Ok(IndeterminateReason::IncompleteInput);
        }
        // Valuation sources and straddled positions, in position order.
        let mut cells: Vec<(usize, usize, Cell)> = closure
            .iter()
            .filter(|(node, _)| matches!(self.plan.nodes.get(*node), Some(Node::Atom)))
            .filter_map(|(node, position)| {
                self.cells
                    .get(&(*position, self.class(*node)))
                    .filter(|cell| cell.ambiguous())
                    .map(|cell| (*position, self.class(*node), *cell))
            })
            .collect();
        cells.sort_unstable_by_key(|(position, node, _)| (*position, *node));
        cells.dedup_by_key(|(position, node, _)| (*position, *node));
        let mut straddled: Vec<usize> = frames()
            .iter()
            .flat_map(|frame| frame.candidates.iter())
            .filter(|candidate| candidate.part == Part::Maybe)
            .map(|candidate| candidate.position)
            .collect();
        straddled.sort_unstable();
        straddled.dedup();
        let mut sources: Vec<(usize, Option<(usize, Cell)>)> = cells
            .iter()
            .map(|(position, node, cell)| (*position, Some((*node, *cell))))
            .chain(straddled.iter().map(|position| (*position, None)))
            .collect();
        sources.sort_by_key(|(position, source)| (*position, source.map(|(node, _)| node)));
        let describe = |position: usize, source: Option<(usize, Cell)>| match source {
            Some((node, Cell::Conflicting)) => {
                let atom = self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?;
                Ok(IndeterminateReason::ConflictingValuation {
                    position: position_of(position)?,
                    atom,
                })
            }
            Some((node, _)) => {
                let atom = self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?;
                Ok(IndeterminateReason::MissingValuation {
                    position: position_of(position)?,
                    atom,
                })
            }
            None => Ok(IndeterminateReason::StraddlesBound),
        };
        for (position, source) in &sources {
            let deciding = match source {
                Some((node, _)) => [Poss::TRUE, Poss::FALSE].into_iter().any(|poss| {
                    settles(&Variant {
                        fixed: vec![((*position, *node), poss)],
                        ..Variant::default()
                    })
                }),
                None => [true, false].into_iter().any(|takes_part| {
                    settles(&Variant {
                        settled: Some((*position, takes_part)),
                        ..Variant::default()
                    })
                }),
            };
            if deciding {
                return describe(*position, *source);
            }
        }
        // No single repair settles it: report the first cause present.
        if any_history {
            return Ok(IndeterminateReason::MissingHistory);
        }
        if any_unestablished {
            return Ok(IndeterminateReason::IncompleteInput);
        }
        match sources.first() {
            Some((position, source)) => describe(*position, *source),
            None => Err(fault("two-valued-result-has-a-cause")),
        }
    }

    /// The decision support of `activation`: the `(position, atom)` pairs
    /// whose single valuation, flipped or settled, or whose position, other
    /// than the anchor, removed, changes the lifted result of the activation.
    pub(super) fn support(&self, activation: &Finished) -> Result<Vec<SupportPair>, InternalFault> {
        let lifter = self.lifter();
        let root = (self.plan.root, activation.anchor);
        let closure = &activation.closure;
        let definite = activation.poss.is_singleton();
        let changes = |variant: &Variant| {
            lifter
                .exact(closure, root, variant)
                .is_some_and(|poss| poss != activation.poss)
        };
        let mut atoms: Vec<(usize, usize)> = closure
            .iter()
            .filter(|(node, _)| matches!(self.plan.nodes.get(*node), Some(Node::Atom)))
            .map(|(node, position)| (*position, *node))
            .collect();
        atoms.sort_unstable();
        atoms.dedup();
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for &(position, node) in &atoms {
            let Some(cell) = self.cells.get(&(position, self.class(node))) else {
                continue;
            };
            let perturbed: &[Poss] = match cell {
                Cell::True => &[Poss::FALSE],
                Cell::False => &[Poss::TRUE],
                Cell::Missing | Cell::Conflicting => &[Poss::TRUE, Poss::FALSE],
            };
            let valuation = perturbed.iter().any(|poss| {
                changes(&Variant {
                    fixed: vec![((position, self.class(node)), *poss)],
                    ..Variant::default()
                })
            });
            let removal = definite
                && position != activation.anchor
                && changes(&Variant {
                    removed: Some(position),
                    ..Variant::default()
                });
            if valuation || removal {
                pairs.push((position, node));
            }
        }
        let mut support = Vec::new();
        for (position, node) in pairs {
            support.push(SupportPair {
                position: position_of(position)?,
                atom: self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?,
            });
        }
        Ok(support)
    }

    /// Assemble the verdict and report from the activations `prior`
    /// summarizes, the activations that ended and what stopped the run. The
    /// result identity hashes `identity`'s frozen blocks, then a block for
    /// each position the run holds.
    pub(super) fn settle(
        &self,
        prior: &Prior,
        done: &[Finished],
        tail: Tail,
        identity: &IdentityStream,
    ) -> Result<WindowEvaluation, InternalFault> {
        let first_false = done.iter().find(|a| a.result == Ending::False);
        let first_open = done
            .iter()
            .find(|a| matches!(a.result, Ending::Pending { .. } | Ending::Indeterminate(_)));
        let scope_closed = self.trace.asserted.scope == ProgressClosure::Closed;
        let (verdict, basis, support) = if let Some(summary) = &prior.first_false {
            (
                WindowVerdict::Violated {
                    at: position_of(summary.anchor)?,
                },
                if scope_closed {
                    Basis::ClosedScope
                } else {
                    Basis::DecisiveCounterexample
                },
                summary.support.clone(),
            )
        } else if let Some(activation) = first_false {
            let basis = if activation.established && scope_closed {
                Basis::ClosedScope
            } else {
                Basis::DecisiveCounterexample
            };
            (
                WindowVerdict::Violated {
                    at: position_of(activation.anchor)?,
                },
                basis,
                self.support(activation)?,
            )
        } else {
            match tail {
                Tail::GuardUnresolved(at) => (
                    WindowVerdict::ActivationIncomplete { at },
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::Undefined(undefined) => (
                    WindowVerdict::Undefined(undefined),
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::Incomplete(incomplete) => (
                    WindowVerdict::Incomplete(incomplete),
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::WindowLimit(limit) => (
                    WindowVerdict::WindowLimitReached(limit),
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::Done => {
                    let open = match (&prior.first_open, first_open) {
                        (Some(summary), _) => {
                            Some((summary.anchor, &summary.result, summary.support.clone()))
                        }
                        (None, Some(activation)) => Some((
                            activation.anchor,
                            &activation.result,
                            self.support(activation)?,
                        )),
                        (None, None) => None,
                    };
                    match open {
                        Some((anchor, result, support)) => {
                            let at = position_of(anchor)?;
                            let (verdict, basis) = match result {
                                Ending::Pending { deadline } => (
                                    if self.clause.profile()
                                        == TemporalProfile::EventPositionFalseExtension
                                    {
                                        WindowVerdict::PendingNextEvent { at }
                                    } else {
                                        WindowVerdict::Pending {
                                            at,
                                            deadline: *deadline,
                                        }
                                    },
                                    Basis::Unsettled,
                                ),
                                Ending::Indeterminate(reason) => (
                                    WindowVerdict::Indeterminate {
                                        at,
                                        reason: reason.clone(),
                                    },
                                    Basis::Unavailable,
                                ),
                                Ending::True | Ending::False => {
                                    return Err(fault("open-activation-is-open"))
                                }
                            };
                            (verdict, basis, support)
                        }
                        None => {
                            // Every summarized activation is established.
                            let established = done.iter().all(|a| a.established);
                            let basis = if established && scope_closed {
                                Basis::ClosedScope
                            } else {
                                Basis::DecisiveWitness
                            };
                            let mut support = prior.holds.clone();
                            for activation in done {
                                support.extend(self.support(activation)?);
                            }
                            (WindowVerdict::Holds, basis, support.into_iter().collect())
                        }
                    }
                }
            }
        };
        let assessment = match verdict {
            WindowVerdict::Incomplete(_)
            | WindowVerdict::WindowLimitReached(_)
            | WindowVerdict::ActivationIncomplete { .. } => Assessment::Failed,
            WindowVerdict::Holds
            | WindowVerdict::Violated { .. }
            | WindowVerdict::Pending { .. }
            | WindowVerdict::PendingNextEvent { .. }
            | WindowVerdict::Indeterminate { .. }
            | WindowVerdict::Undefined(_) => Assessment::Completed,
        };
        let progress = self.trace.asserted;
        let readings = self.readings();
        let blocks = self.blocks(&readings, self.base..self.end())?;
        let identity = identity.identity(
            &self.trace.clause_identity,
            self.clause.profile().identity(),
            &self.trace.clock,
            &blocks,
            &progress,
            &self.trace.lateness,
            self.meter.limits(),
        )?;
        Ok(WindowEvaluation {
            verdict,
            basis,
            support,
            premises: Premises {
                scope: progress.scope,
                execution: progress.execution,
                assessment,
                completeness: progress.completeness,
                watermark: progress.watermark.inclusive(),
                lower_boundary: progress.lower_boundary,
                cover: self.trace.clock.cover,
                unit: self.trace.clock.unit.clone(),
            },
            identity,
            visits: self.visits,
        })
    }

    /// Every valuation the run holds, by position, each position's in atom
    /// class order.
    pub(super) fn readings(&self) -> BTreeMap<usize, Vec<(usize, Cell)>> {
        let mut readings: BTreeMap<usize, Vec<(usize, Cell)>> = BTreeMap::new();
        for ((position, class), cell) in &self.cells {
            readings.entry(*position).or_default().push((*class, *cell));
        }
        for cells in readings.values_mut() {
            cells.sort_unstable_by_key(|(class, _)| *class);
        }
        readings
    }

    /// The identity blocks of the held positions in `range`, with
    /// `readings`.
    pub(super) fn blocks<'r>(
        &'r self,
        readings: &'r BTreeMap<usize, Vec<(usize, Cell)>>,
        range: std::ops::Range<usize>,
    ) -> Result<Vec<Block<'r>>, InternalFault> {
        range
            .map(|position| {
                let index = position
                    .checked_sub(self.base)
                    .ok_or_else(|| fault("position-is-held"))?;
                let observation = self
                    .trace
                    .positions
                    .get(index)
                    .ok_or_else(|| fault("position-is-held"))?;
                Ok(Block {
                    observation,
                    interval: self.trace.intervals.get(index).copied().flatten(),
                    arrival: self.trace.lateness.arrivals.get(index).copied().flatten(),
                    readings: readings.get(&position).map_or(&[], Vec::as_slice),
                })
            })
            .collect()
    }

    /// Whether the progress establishes `ahead` ticks after the held
    /// `anchor`, so no later position or progress changes its activation
    /// (FR-820).
    pub(super) fn settled(&self, anchor: usize, ahead: i128) -> Result<bool, InternalFault> {
        let need = self.stamp(anchor)?.saturating_add(ahead);
        Ok(covers(self.trace.progress.completeness, need) && self.watermark >= self.trace.settling(need))
    }
}
