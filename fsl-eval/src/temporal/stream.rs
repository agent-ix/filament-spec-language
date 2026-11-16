// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820: the TemporalTrace evaluator in streaming form. A [`ClauseStream`]
//! reads a finite trace one position at a time and answers, after any
//! position, the outcome [`evaluate`](super::evaluate) gives over the trace of
//! every position read so far, under the clause's closed (bounded profile) or
//! prefix (infinite-trace profile) reading.
//!
//! **Cells.** When position `m` is read, every node of the plan gets its cell
//! there: its value at `m` as far as the positions read decide it, a residual
//! over the positions still to come. A residual is Kleene's three values, a
//! `not`, an `and` or an `or` of residuals, or a future scan at the frontier
//! with its window shifted (formula progression). Reading the next position
//! progresses each residual: every future scan at the frontier is expanded
//! into its operands' cells there and the scan one position on.
//!
//! **Past operators** carry their operands' cells: a closed window keeps the
//! cells of its last `upper + 1` positions, an open one keeps the cells of the
//! last `lower` positions and one running value over everything older
//! (`once` folds `or`, `since` folds `w or (g and acc)`). A closed past window
//! reads positions before the origin, where every atom is false (FSpec
//! FR-092): the stream starts that many positions before 0, as far as the
//! formula's left floor makes a difference, and a window reaching further
//! reads its oldest cell, which equals every older one.
//!
//! **Activations.** Each activation's residual is progressed with the rest;
//! activations whose residuals are equal are one class, kept at its least
//! position, so a clause whose obligations repeat holds one residual for all
//! of them. A residual that reaches a value is dropped, a `false` one
//! keeping its position as the violation.
//!
//! **The reading of a prefix.** The batch run reads the trace's positions
//! followed by a sentinel whose atoms are `false` (closed) or unknown
//! (prefix). [`ClauseStream::outcome`] progresses a copy of the state over
//! the sentinel for the formula's past reach plus one positions, after which
//! every node's value is the same at every later position (ADR-018 SM-8), so
//! a future scan still open reads its operand's value there.
//!
//! Each read and each node visit is one `temporal.visit` work unit on the
//! caller's meter, so the work of one position is bounded by the plan and
//! the residuals the activations hold, not by the positions already read.

use std::collections::VecDeque;

use filament_exact::{Charge, ChargePoint, Meter};
use filament_semantic_value::fairness::FairnessConstraint;
use fsl_semantics::check::{Activation, CheckedTemporalClause, TemporalProfile};
use serde::{Deserialize, Serialize};

use super::engine::{PositionReader, PositionStop, Stop, Tri};
use super::letters::TemporalLetters;
use super::outcome::{
    InsufficientDataEvaluation, TemporalOutcome, TemporalRefusal, UndefinedEvaluation,
};
use super::plan::{fault, slot, Base, Direction, Node, Plan, Scan, Window};
use super::position::TemporalPosition;
use super::window::{WindowLimitReached, WindowRefusal};

const VISIT: ChargePoint = ChargePoint::TemporalVisit;

/// A node's value at a position as far as the positions read decide it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Res {
    Tri(Tri),
    Not(Box<Res>),
    And(Vec<Res>),
    Or(Vec<Res>),
    /// The future scan `node` at the frontier, its window starting `lower`
    /// positions after it and ending `upper` after it (`None`: open).
    Scan {
        node: usize,
        lower: u64,
        upper: Option<u64>,
    },
}

fn not(res: Res) -> Res {
    match res {
        Res::Tri(value) => Res::Tri(value.negate()),
        Res::Not(inner) => *inner,
        other => Res::Not(Box::new(other)),
    }
}

/// The conjunction (`conj`) or disjunction of `parts`: nested joins of the
/// same kind are flattened, the unit is dropped, the absorbing value
/// absorbs, one unknown is kept and equal parts are kept once.
fn join(conj: bool, parts: impl IntoIterator<Item = Res>) -> Res {
    let (unit, zero) = if conj {
        (Tri::True, Tri::False)
    } else {
        (Tri::False, Tri::True)
    };
    let mut items: Vec<Res> = Vec::new();
    let mut unknown = false;
    for part in parts {
        let nested = match part {
            Res::And(inner) if conj => inner,
            Res::Or(inner) if !conj => inner,
            other => vec![other],
        };
        for item in nested {
            match item {
                Res::Tri(value) if value == zero => return Res::Tri(zero),
                Res::Tri(value) if value == unit => {}
                Res::Tri(_) => unknown = true,
                other => {
                    if !items.contains(&other) {
                        items.push(other);
                    }
                }
            }
        }
    }
    if unknown {
        if items.is_empty() {
            return Res::Tri(Tri::Unknown);
        }
        items.insert(0, Res::Tri(Tri::Unknown));
    }
    match items.len() {
        0 => Res::Tri(unit),
        1 if !unknown => items.pop().unwrap_or(Res::Tri(unit)),
        _ if conj => Res::And(items),
        _ => Res::Or(items),
    }
}

/// What a past scan carries from one position to the next.
#[derive(Clone, Debug, Deserialize, Serialize)]
enum Carry {
    None,
    /// A closed window: the operand cells of its last `upper + 1` positions,
    /// newest last, operands negated for a dual.
    Window {
        witness: VecDeque<Res>,
        guard: VecDeque<Res>,
    },
    /// An open window `[lower, *]`: the operand cells of the last `lower`
    /// positions and the value over every older position from 0.
    Running {
        witness: VecDeque<Res>,
        guard: VecDeque<Res>,
        value: Option<Res>,
    },
}

/// One class of open activations: their least position and their common
/// residual.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Open {
    first: u64,
    res: Res,
}

/// How the trace past its positions reads.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Reading {
    /// A bounded profile's false extension (FSpec FR-090).
    Closed,
    /// An infinite-trace prefix (FR-328).
    Prefix,
}

/// Why a position could not be read; the stream is unchanged.
#[derive(Debug)]
pub enum StreamStop {
    /// The meter ran out or the caller cancelled.
    Incomplete(filament_exact::Incomplete),
}

/// The outcome a read that stopped fixes for every longer prefix.
#[derive(Clone, Debug)]
enum Ended {
    WindowLimit(WindowLimitReached),
    Window(WindowRefusal),
    Fault(filament_semantic_value::catalog::InternalFault),
}

/// FR-820: a clause's evaluation carried from one position to the next.
#[derive(Clone, Debug)]
pub struct ClauseStream {
    plan: Plan,
    reading: Reading,
    /// Whether activations are evaluated: a prefix decides only a safety
    /// formula (FR-328).
    evaluates: bool,
    activation: Activation,
    /// The positions an `on origin` bounded clause reads.
    read: Option<u64>,
    fairness: Option<FairnessConstraint>,
    /// The sentinel positions [`Self::outcome`] reads.
    tail: u64,
    cells: Vec<Res>,
    carries: Vec<Carry>,
    /// The real positions read.
    len: u64,
    opens: Vec<Open>,
    /// The least activation that is false for good.
    violated: Option<u64>,
    /// Whether an activation read unknown for good or was skipped short of
    /// data.
    unknown: bool,
    stopped: Option<u64>,
    undefined: Option<UndefinedEvaluation>,
    refused: Option<TemporalPosition>,
    insufficient: Option<InsufficientDataEvaluation>,
    ended: Option<Ended>,
}

/// A count of positions as `u64`.
fn count(value: u128) -> Result<u64, TemporalRefusal> {
    u64::try_from(value).map_err(|_| TemporalRefusal::Fault(fault("stream-reach-in-u64")))
}

impl ClauseStream {
    /// Open the stream of `clause` over a finite trace, reading the
    /// positions before the origin a closed past window reads.
    ///
    /// # Errors
    ///
    /// A profile the position evaluator does not read (finite-window,
    /// timed), as [`evaluate`](super::evaluate) refuses it, or an
    /// incomplete reading of the positions before the origin.
    pub fn open(
        clause: &CheckedTemporalClause,
        meter: &mut Meter,
    ) -> Result<Result<Self, StreamStop>, TemporalRefusal> {
        let profile = clause.profile();
        let reading = match profile {
            TemporalProfile::EventPositionFalseExtension
            | TemporalProfile::FixedSampleFalseExtension => Reading::Closed,
            TemporalProfile::InfiniteTrace => Reading::Prefix,
            TemporalProfile::TimestampedEventFiniteWindow => {
                return Err(TemporalRefusal::TraceCarriesNoStamps { profile })
            }
            TemporalProfile::Timed => return Err(TemporalRefusal::ProfileNotEvaluated { profile }),
        };
        let plan = Plan::new(clause.formula()).map_err(TemporalRefusal::Fault)?;
        let evaluates =
            reading == Reading::Closed || plan.is_safety().map_err(TemporalRefusal::Fault)?;
        let reach = plan.past_reach(1, None).map_err(TemporalRefusal::Fault)?;
        let tail =
            count(slot(&reach, plan.root).map_err(TemporalRefusal::Fault)?)?.saturating_add(1);
        let mut before = 0_u64;
        let mut carries = Vec::with_capacity(plan.nodes.len());
        for node in &plan.nodes {
            carries.push(match node {
                Node::Scan(scan) if scan.base.direction() == Direction::Past => match scan.window {
                    Window::Closed { .. } => {
                        let floor = |operand: usize| slot(&plan.left_floor, operand);
                        let mut reach = floor(scan.witness).map_err(TemporalRefusal::Fault)?;
                        if let Some(guard) = scan.guard {
                            reach = reach.max(floor(guard).map_err(TemporalRefusal::Fault)?);
                        }
                        before = before.max(count(reach)?);
                        Carry::Window {
                            witness: VecDeque::new(),
                            guard: VecDeque::new(),
                        }
                    }
                    Window::From { .. } => Carry::Running {
                        witness: VecDeque::new(),
                        guard: VecDeque::new(),
                        value: None,
                    },
                },
                Node::Constant(_)
                | Node::Atom
                | Node::Not(_)
                | Node::And(..)
                | Node::Or(..)
                | Node::Implies(..)
                | Node::Scan(_) => Carry::None,
            });
        }
        let read = match (clause.activation(), clause.horizon()) {
            (Activation::Origin, Some(horizon)) => Some(horizon.saturating_add(1)),
            _ => None,
        };
        let fairness = match profile {
            TemporalProfile::InfiniteTrace => clause.fairness().first().cloned(),
            _ => None,
        };
        let mut stream = Self {
            cells: vec![Res::Tri(Tri::False); plan.nodes.len()],
            plan,
            reading,
            evaluates,
            activation: clause.activation(),
            read,
            fairness,
            tail,
            carries,
            len: 0,
            opens: Vec::new(),
            violated: None,
            unknown: false,
            stopped: None,
            undefined: None,
            refused: None,
            insufficient: None,
            ended: None,
        };
        // Positions before the origin read every atom false (FSpec FR-092).
        let before = i128::from(before);
        for position in -before..0 {
            if let Err(stop) = stream.step(position, |_| Tri::False, meter) {
                return match stop {
                    Stop::Incomplete(incomplete) => Ok(Err(StreamStop::Incomplete(incomplete))),
                    other => Err(refusal(other)),
                };
            }
        }
        Ok(Ok(stream))
    }

    /// The real positions read.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether no real position has been read.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The number of residuals the stream holds: one per open activation
    /// class and one per carried past-window cell.
    pub fn held(&self) -> u64 {
        let carried: usize = self
            .carries
            .iter()
            .map(|carry| match carry {
                Carry::None => 0,
                Carry::Window { witness, guard } => witness.len() + guard.len(),
                Carry::Running {
                    witness,
                    guard,
                    value,
                } => witness.len() + guard.len() + usize::from(value.is_some()),
            })
            .sum();
        u64::try_from(carried + self.opens.len()).unwrap_or(u64::MAX)
    }

    /// Read the next position of the trace, which `source` holds at `at`.
    ///
    /// # Errors
    ///
    /// [`StreamStop::Incomplete`] when `meter` runs out; the stream is then
    /// unchanged.
    pub fn push<L: TemporalLetters>(
        &mut self,
        clause: &CheckedTemporalClause,
        source: &mut L,
        at: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<(), StreamStop> {
        let mut next = self.clone();
        match next.advance(clause, source, at, meter) {
            Ok(()) => {
                *self = next;
                Ok(())
            }
            Err(Stop::Incomplete(incomplete)) => Err(StreamStop::Incomplete(incomplete)),
            Err(stop) => {
                self.ended = Some(match stop {
                    Stop::WindowLimit(limit) => Ended::WindowLimit(limit),
                    Stop::Window(refusal) => Ended::Window(refusal),
                    Stop::Fault(internal) => Ended::Fault(internal),
                    Stop::Incomplete(_) | Stop::ConditionRefused(_) => {
                        Ended::Fault(fault("stream-read-stops-at-a-reading"))
                    }
                });
                self.len = self.len.saturating_add(1);
                Ok(())
            }
        }
    }

    fn advance<L: TemporalLetters>(
        &mut self,
        clause: &CheckedTemporalClause,
        source: &mut L,
        at: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<(), Stop> {
        let index = self.len;
        self.len = index.saturating_add(1);
        if self.ended.is_some() {
            return Ok(());
        }
        let reads = self.stopped.is_none() && self.read.is_none_or(|read| index < read);
        let guarded = matches!(self.activation, Activation::Each { guarded: true });
        let letter = if reads {
            let reader = PositionReader {
                formula: clause.formula(),
                atoms: &self.plan.atoms,
                guarded,
                captured: clause.capture_site(),
            };
            reader.read(source, at, TemporalPosition::new(index), meter)?
        } else {
            super::engine::PositionLetter::default()
        };
        if self.insufficient.is_none() {
            self.insufficient = letter.insufficient.clone();
        }
        match &letter.stop {
            None => {}
            Some(PositionStop::Undefined(undefined)) => {
                self.stopped = Some(index);
                self.undefined = Some(undefined.clone());
            }
            Some(PositionStop::Refused(position)) => {
                self.stopped = Some(index);
                self.refused = Some(*position);
            }
        }
        // From the first stopped position on, every atom reads unknown, as
        // does a position an `on origin` bounded clause does not read.
        let known = reads && self.stopped.is_none();
        let atoms = &letter.atoms;
        let position = i128::from(index);
        self.step(
            position,
            |node| match known {
                true => atoms
                    .iter()
                    .find(|(atom, _)| *atom == node)
                    .map_or(Tri::Unknown, |(_, value)| *value),
                false => Tri::Unknown,
            },
            meter,
        )?;
        self.settle_opens()?;
        let activates = self.evaluates
            && self.stopped.is_none()
            && self.violated.is_none()
            && match self.activation {
                Activation::Origin => index == 0,
                Activation::Each { guarded: false } => true,
                Activation::Each { guarded: true } => letter.when == Some(true),
            };
        if activates {
            if letter.short {
                self.unknown = true;
            } else {
                let res = slot_res(&self.cells, self.plan.root)?;
                self.opens.push(Open { first: index, res });
                self.settle_opens()?;
            }
        }
        Ok(())
    }

    /// Drop the activations whose residual reached a value, keep the least
    /// false one, and merge the classes whose residuals are equal.
    fn settle_opens(&mut self) -> Result<(), Stop> {
        let opens = std::mem::take(&mut self.opens);
        for open in opens {
            match open.res {
                Res::Tri(Tri::True) => {}
                Res::Tri(Tri::False) => {
                    self.violated = Some(self.violated.map_or(open.first, |at| at.min(open.first)));
                }
                Res::Tri(Tri::Unknown) => self.unknown = true,
                res => {
                    if !self.opens.iter().any(|kept| kept.res == res) {
                        self.opens.push(Open {
                            first: open.first,
                            res,
                        });
                    }
                }
            }
        }
        if let Some(at) = self.violated {
            self.opens.retain(|open| open.first < at);
        }
        Ok(())
    }

    /// Give every node its cell at `position`, whose atoms read `atom`, and
    /// progress every carried residual and open activation past it.
    fn step(
        &mut self,
        position: i128,
        atom: impl Fn(usize) -> Tri,
        meter: &mut Meter,
    ) -> Result<(), Stop> {
        for index in (0..self.plan.nodes.len()).rev() {
            meter.charge(Charge::new(VISIT))?;
            let cell = match slot(&self.plan.nodes, index)? {
                Node::Constant(value) => Res::Tri(Tri::of(value)),
                Node::Atom => Res::Tri(atom(index)),
                Node::Not(operand) => not(slot_res(&self.cells, operand)?),
                Node::And(left, right) => join(
                    true,
                    [slot_res(&self.cells, left)?, slot_res(&self.cells, right)?],
                ),
                Node::Or(left, right) => join(
                    false,
                    [slot_res(&self.cells, left)?, slot_res(&self.cells, right)?],
                ),
                Node::Implies(left, right) => join(
                    false,
                    [
                        not(slot_res(&self.cells, left)?),
                        slot_res(&self.cells, right)?,
                    ],
                ),
                Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                    (Direction::Future, Window::Closed { lower, upper }) => {
                        self.expand(index, scan, lower, Some(upper))?
                    }
                    (Direction::Future, Window::From { lower }) => {
                        self.expand(index, scan, lower, None)?
                    }
                    (Direction::Past, _) => self.past(index, scan, position, meter)?,
                },
            };
            *self
                .cells
                .get_mut(index)
                .ok_or_else(|| fault("plan-index-in-arena"))? = cell;
        }
        let mut opens = std::mem::take(&mut self.opens);
        for open in &mut opens {
            meter.charge(Charge::new(VISIT))?;
            let res = std::mem::replace(&mut open.res, Res::Tri(Tri::False));
            open.res = self.progress(res)?;
        }
        self.opens = opens;
        Ok(())
    }

    /// The future scan `node` at the newest position with its window
    /// `[lower, upper]` after it, from its operands' cells there.
    fn expand(&self, node: usize, scan: Scan, lower: u64, upper: Option<u64>) -> Result<Res, Stop> {
        if lower > 0 {
            return Ok(Res::Scan {
                node,
                lower: lower - 1,
                upper: upper.map(|upper| upper.saturating_sub(1)),
            });
        }
        let rest = match upper {
            Some(0) => None,
            Some(upper) => Some(Res::Scan {
                node,
                lower: 0,
                upper: Some(upper - 1),
            }),
            None => Some(Res::Scan {
                node,
                lower: 0,
                upper: None,
            }),
        };
        let witness = slot_res(&self.cells, scan.witness)?;
        let Some(rest) = rest else {
            return Ok(witness);
        };
        Ok(match (scan.base, scan.dual, scan.guard) {
            (Base::Eventually, dual, _) => join(dual, [witness, rest]),
            (Base::Until, dual, Some(guard)) => {
                let guard = slot_res(&self.cells, guard)?;
                join(dual, [witness, join(!dual, [guard, rest])])
            }
            (Base::Until, _, None) | (Base::Once | Base::Since, ..) => {
                return Err(fault("future-scan-shape").into())
            }
        })
    }

    /// Progress `res` past the newest position: each future scan at the
    /// frontier is expanded there.
    fn progress(&self, res: Res) -> Result<Res, Stop> {
        Ok(match res {
            Res::Tri(value) => Res::Tri(value),
            Res::Not(inner) => not(self.progress(*inner)?),
            Res::And(parts) => join(
                true,
                parts
                    .into_iter()
                    .map(|part| self.progress(part))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Res::Or(parts) => join(
                false,
                parts
                    .into_iter()
                    .map(|part| self.progress(part))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Res::Scan { node, lower, upper } => match slot(&self.plan.nodes, node)? {
                Node::Scan(scan) => self.expand(node, scan, lower, upper)?,
                Node::Constant(_)
                | Node::Atom
                | Node::Not(_)
                | Node::And(..)
                | Node::Or(..)
                | Node::Implies(..) => return Err(fault("residual-scan-is-a-scan").into()),
            },
        })
    }

    /// The past scan `node` at `position`: progress what it carries, take in
    /// its operands' cells there and read its window.
    fn past(
        &mut self,
        node: usize,
        scan: Scan,
        position: i128,
        meter: &mut Meter,
    ) -> Result<Res, Stop> {
        let operand = |cells: &[Res], index: usize| -> Result<Res, Stop> {
            let cell = slot_res(cells, index)?;
            Ok(if scan.dual { not(cell) } else { cell })
        };
        let witness_cell = operand(&self.cells, scan.witness)?;
        let guard_cell = match scan.guard {
            Some(guard) => Some(operand(&self.cells, guard)?),
            None => None,
        };
        let mut carry = std::mem::replace(
            self.carries
                .get_mut(node)
                .ok_or_else(|| fault("plan-index-in-arena"))?,
            Carry::None,
        );
        let since = scan.base == Base::Since;
        let base = match (&mut carry, scan.window) {
            (Carry::Window { witness, guard }, Window::Closed { lower, upper }) => {
                self.progress_all(witness, meter)?;
                self.progress_all(guard, meter)?;
                witness.push_back(witness_cell);
                if let Some(cell) = guard_cell {
                    guard.push_back(cell);
                }
                let keep = usize::try_from(upper.saturating_add(1)).unwrap_or(usize::MAX);
                while witness.len() > keep {
                    witness.pop_front();
                }
                while guard.len() > keep {
                    guard.pop_front();
                }
                let newest = witness.len().saturating_sub(1);
                let lower = usize::try_from(lower).unwrap_or(usize::MAX);
                let upper = usize::try_from(upper).unwrap_or(usize::MAX);
                // A window wholly before the oldest cell reads the oldest,
                // which every older position equals.
                let offsets = if lower > newest {
                    newest..=newest
                } else {
                    lower..=upper.min(newest)
                };
                let at = |cells: &VecDeque<Res>, offset: usize| {
                    cells
                        .get(newest - offset)
                        .cloned()
                        .ok_or_else(|| Stop::from(fault("past-window-cell")))
                };
                if since {
                    let mut acc = Res::Tri(Tri::False);
                    let mut held = Res::Tri(Tri::True);
                    for offset in offsets {
                        let witness_at = at(witness, offset)?;
                        acc = join(false, [acc, join(true, [witness_at, held.clone()])]);
                        held = join(true, [held, at(guard, offset)?]);
                    }
                    acc
                } else {
                    let mut parts = Vec::new();
                    for offset in offsets {
                        parts.push(at(witness, offset)?);
                    }
                    join(false, parts)
                }
            }
            (
                Carry::Running {
                    witness,
                    guard,
                    value,
                },
                Window::From { lower },
            ) => {
                self.progress_all(witness, meter)?;
                self.progress_all(guard, meter)?;
                if let Some(held) = value.take() {
                    *value = Some(self.progress(held)?);
                }
                witness.push_back(witness_cell);
                if let Some(cell) = guard_cell {
                    guard.push_back(cell);
                }
                let lower = usize::try_from(lower).unwrap_or(usize::MAX);
                while witness.len() > lower {
                    let oldest = position
                        - i128::try_from(witness.len() - 1)
                            .map_err(|_| Stop::from(fault("past-window-in-i128")))?;
                    let witness_at = witness
                        .pop_front()
                        .ok_or_else(|| Stop::from(fault("past-window-cell")))?;
                    let guard_at = if since {
                        Some(
                            guard
                                .pop_front()
                                .ok_or_else(|| Stop::from(fault("past-window-cell")))?,
                        )
                    } else {
                        None
                    };
                    // An open past window reads only the positions that
                    // exist (FSpec FR-092, FR-327-AC-14).
                    if oldest < 0 {
                        continue;
                    }
                    let older = value.take().unwrap_or(Res::Tri(Tri::False));
                    *value = Some(match guard_at {
                        Some(guard_at) => join(false, [witness_at, join(true, [guard_at, older])]),
                        None => join(false, [older, witness_at]),
                    });
                }
                value.clone().unwrap_or(Res::Tri(Tri::False))
            }
            (Carry::None | Carry::Window { .. } | Carry::Running { .. }, _) => {
                return Err(fault("past-scan-carries-its-window").into())
            }
        };
        *self
            .carries
            .get_mut(node)
            .ok_or_else(|| fault("plan-index-in-arena"))? = carry;
        Ok(if scan.dual { not(base) } else { base })
    }

    fn progress_all(&self, cells: &mut VecDeque<Res>, meter: &mut Meter) -> Result<(), Stop> {
        for cell in cells.iter_mut() {
            meter.charge(Charge::new(VISIT))?;
            let res = std::mem::replace(cell, Res::Tri(Tri::False));
            *cell = self.progress(res)?;
        }
        Ok(())
    }

    /// FR-327, FR-328: the outcome of the clause over every position read,
    /// as [`evaluate`](super::evaluate) gives it over that trace.
    ///
    /// # Errors
    ///
    /// The refusal the batch run gives over the same trace.
    pub fn outcome(&self, meter: &mut Meter) -> Result<TemporalOutcome, TemporalRefusal> {
        match self.reading_outcome(meter) {
            Ok(outcome) => outcome,
            Err(Stop::Incomplete(incomplete)) => Ok(TemporalOutcome::Incomplete(incomplete)),
            Err(other) => Err(refusal(other)),
        }
    }

    fn reading_outcome(
        &self,
        meter: &mut Meter,
    ) -> Result<Result<TemporalOutcome, TemporalRefusal>, Stop> {
        if self.len == 0 {
            return Ok(Err(TemporalRefusal::EmptyTrace));
        }
        if let Some(constraint) = &self.fairness {
            return Ok(Ok(TemporalOutcome::MissingFairnessPremise {
                constraint: constraint.clone(),
            }));
        }
        match &self.ended {
            None => {}
            Some(Ended::WindowLimit(limit)) => {
                return Ok(Ok(TemporalOutcome::WindowLimitReached(*limit)))
            }
            Some(Ended::Window(window)) => return Ok(Err(TemporalRefusal::Window(*window))),
            Some(Ended::Fault(internal)) => {
                return Ok(Err(TemporalRefusal::Fault(internal.clone())))
            }
        }
        let mut violated = self.violated;
        let mut every_true = !self.unknown;
        if self.evaluates && !self.opens.is_empty() {
            let sentinel = match self.reading {
                Reading::Closed => Tri::False,
                Reading::Prefix => Tri::Unknown,
            };
            let mut tail = self.clone();
            let start = i128::from(self.len);
            for offset in 0..i128::from(self.tail) {
                tail.step(start + offset, |_| sentinel, meter)?;
            }
            let values = tail.stationary()?;
            for open in &tail.opens {
                match evaluate(&open.res, &tail.plan, &values)? {
                    Tri::True => {}
                    Tri::False => {
                        violated = Some(violated.map_or(open.first, |at| at.min(open.first)));
                    }
                    Tri::Unknown => every_true = false,
                }
            }
        }
        if let Some(at) = violated {
            return Ok(Ok(TemporalOutcome::Violated {
                at: TemporalPosition::new(at),
            }));
        }
        if let Some(undefined) = &self.undefined {
            return Ok(Ok(TemporalOutcome::Undefined(undefined.clone())));
        }
        if let Some(at) = self.refused {
            return Ok(Err(TemporalRefusal::ConditionRefused { at }));
        }
        if let Some(insufficient) = &self.insufficient {
            return Ok(Ok(TemporalOutcome::InsufficientData(insufficient.clone())));
        }
        Ok(match self.reading {
            Reading::Prefix => Ok(TemporalOutcome::Pending),
            Reading::Closed if every_true => Ok(TemporalOutcome::Holds),
            Reading::Closed => Err(TemporalRefusal::Fault(fault("definite-trace-read-unknown"))),
        })
    }

    /// Each node's value at every position past the newest, once the
    /// sentinel has been read for the formula's past reach: its cell there,
    /// a future scan still open reading its witness's value.
    fn stationary(&self) -> Result<Vec<Tri>, Stop> {
        let mut values = vec![Tri::False; self.plan.nodes.len()];
        for index in (0..self.plan.nodes.len()).rev() {
            let value = evaluate(&slot_res(&self.cells, index)?, &self.plan, &values)?;
            *values
                .get_mut(index)
                .ok_or_else(|| fault("plan-index-in-arena"))? = value;
        }
        Ok(values)
    }
}

/// The part of a stream that changes from one position to the next, in its
/// state form. A stop's records are not written: the caller holds the
/// position that stopped and reads it again ([`ClauseStream::restore_stop`]).
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Carried {
    cells: Vec<Res>,
    carries: Vec<Carry>,
    len: u64,
    opens: Vec<Open>,
    violated: Option<u64>,
    unknown: bool,
    stopped: Option<u64>,
    failed: bool,
}

impl ClauseStream {
    /// The stream's state form, read back by [`Self::from_state`]; `None`
    /// for a stream whose records the state form does not carry (a data
    /// shortfall or a read that ended it).
    pub fn to_state(&self) -> Option<serde_json::Value> {
        if self.insufficient.is_some()
            || matches!(self.ended, Some(Ended::WindowLimit(_) | Ended::Window(_)))
        {
            return None;
        }
        serde_json::to_value(Carried {
            cells: self.cells.clone(),
            carries: self.carries.clone(),
            len: self.len,
            opens: self.opens.clone(),
            violated: self.violated,
            unknown: self.unknown,
            stopped: self.stopped,
            failed: self.ended.is_some(),
        })
        .ok()
    }

    /// The stream of `clause` [`Self::to_state`] wrote as `state`; `None` for
    /// a state that is not a state form of this clause's stream.
    pub fn from_state(
        clause: &CheckedTemporalClause,
        state: &serde_json::Value,
        meter: &mut Meter,
    ) -> Option<Self> {
        let carried: Carried = serde_json::from_value(state.clone()).ok()?;
        let mut stream = Self::open(clause, meter).ok()?.ok()?;
        if carried.cells.len() != stream.cells.len()
            || carried.carries.len() != stream.carries.len()
        {
            return None;
        }
        stream.cells = carried.cells;
        stream.carries = carried.carries;
        stream.len = carried.len;
        stream.opens = carried.opens;
        stream.violated = carried.violated;
        stream.unknown = carried.unknown;
        stream.stopped = carried.stopped;
        if carried.failed {
            stream.ended = Some(Ended::Fault(fault("stream-read-failed")));
        }
        Some(stream)
    }

    /// The position at which the reading stopped, when it did.
    pub fn stopped(&self) -> Option<u64> {
        self.stopped
    }

    /// Restore what the stopped position's reading recorded, read again by
    /// the caller: its undefined reading or the condition refused there.
    pub fn restore_stop(
        &mut self,
        undefined: Option<UndefinedEvaluation>,
        refused: Option<TemporalPosition>,
    ) {
        self.undefined = undefined;
        self.refused = refused;
    }
}

/// The value of `res` where every node reads its stationary `values`.
fn evaluate(res: &Res, plan: &Plan, values: &[Tri]) -> Result<Tri, Stop> {
    Ok(match res {
        Res::Tri(value) => *value,
        Res::Not(inner) => evaluate(inner, plan, values)?.negate(),
        Res::And(parts) => {
            let mut value = Tri::True;
            for part in parts {
                value = value.and(evaluate(part, plan, values)?);
            }
            value
        }
        Res::Or(parts) => {
            let mut value = Tri::False;
            for part in parts {
                value = value.or(evaluate(part, plan, values)?);
            }
            value
        }
        // Over a tail whose every position reads alike, a scan whose window
        // is not empty reads its witness: `until` and `release` too, their
        // window's first position needing no guard.
        Res::Scan { node, .. } => match slot(&plan.nodes, *node)? {
            Node::Scan(scan) => *values
                .get(scan.witness)
                .ok_or_else(|| fault("plan-index-in-arena"))?,
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => return Err(fault("residual-scan-is-a-scan").into()),
        },
    })
}

fn slot_res(cells: &[Res], index: usize) -> Result<Res, Stop> {
    cells
        .get(index)
        .cloned()
        .ok_or_else(|| fault("plan-index-in-arena").into())
}

fn refusal(stop: Stop) -> TemporalRefusal {
    match stop {
        Stop::ConditionRefused(at) => TemporalRefusal::ConditionRefused { at },
        Stop::Window(refusal) => TemporalRefusal::Window(refusal),
        Stop::Fault(internal) => TemporalRefusal::Fault(internal),
        Stop::Incomplete(_) | Stop::WindowLimit(_) => {
            TemporalRefusal::Fault(fault("stream-stop-is-a-refusal"))
        }
    }
}
