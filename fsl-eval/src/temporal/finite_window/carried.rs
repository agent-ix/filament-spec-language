// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820: the finite-window evaluator (FR-501) carried across a trace that
//! grows, so an evaluation reads only the positions an open activation can
//! still read.
//!
//! An observation a later step admits is stamped above every watermark
//! asserted so far (FR-643 "Order"), so it sorts after every position an
//! earlier evaluation read. An activation whose window the progress has
//! established reads no later position and no later progress changes it:
//! [`WindowState`] summarizes it ([`Prior`]: the first false activation, the
//! first open one, the union of the true ones' decision supports) and keeps
//! only the readings it made at positions a later activation can still read.
//! The positions before the earliest one an unsummarized activation (or a
//! later one) can read are frozen into the result identity's chain and
//! dropped. Each evaluation is FR-501's run over the positions still held,
//! from the summary, so the report equals the batch report over the prefix:
//! the activations, readings and identity blocks it combines are the ones
//! the batch run computes, in the same order.

use std::collections::BTreeMap;

use filament_exact::Meter;
use filament_semantic_value::catalog::InternalFault;

use super::identity::IdentityStream;
use super::outcome::WindowEvaluation;
use super::run::{anchors, reaches, Anchors, Prior, Run, Seed, Tail};
use super::trace::{
    canonical, TimestampedObservation, TimestampedTrace, WindowClock, WindowProgress,
};
use crate::temporal::facts::ClauseFacts;
use crate::temporal::letters::TemporalLetters;
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::plan::{fault, slot, Direction, Node, Plan, Window};

/// FR-820's carried state of one finite-window clause over one timestamped
/// or progressed trace.
#[derive(Clone, Debug)]
pub struct WindowState {
    clause_identity: String,
    clock: WindowClock,
    /// The positions held, in canonical order from the absolute index
    /// `base`; the last `fresh` were admitted since the last evaluation.
    positions: Vec<TimestampedObservation>,
    fresh: usize,
    base: usize,
    /// The stamp of position 0.
    first: Option<u64>,
    /// The first activation anchor the summary does not cover.
    cursor: usize,
    prior: Prior,
    /// The readings of the summarized activations at the held positions,
    /// and the first undefined reading of all of them.
    seed: Seed,
    identity: IdentityStream,
    /// The stop at an established activation, which every later evaluation
    /// returns.
    stopped: Option<Result<Tail, TemporalRefusal>>,
    /// The ticks before an anchor that its activation can read.
    back: i128,
    /// The ticks after an anchor that its activation, and the windows of
    /// every instance below it, can read: once the progress establishes
    /// them, no later position or progress changes the activation.
    ahead: i128,
}

/// The ticks the formula of `plan` can read from an anchor, backward
/// (`future` false) or forward: the greatest sum, along a nesting path, of
/// the upper bounds of the operators in that direction, each widened by the
/// spread `1 + margin` a counter cover lets a candidate straddle its bound
/// by. Forward, the margin that establishes the deepest window is added.
fn span(plan: &Plan, margin: i128, future: bool) -> Result<i128, InternalFault> {
    let spread = if margin == 0 {
        0
    } else {
        margin.saturating_add(1)
    };
    let mut back = vec![0_i128; plan.nodes.len()];
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
            deepest = deepest.max(slot(&back, operand)?);
        }
        let own = match node {
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (direction, Window::Closed { upper, .. })
                    if (direction == Direction::Future) == future =>
                {
                    i128::from(upper).saturating_add(spread)
                }
                (_, Window::Closed { .. }) => 0,
                (_, Window::From { .. }) => return Err(fault("finite-window-interval-is-bounded")),
            },
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => 0,
        };
        *back
            .get_mut(index)
            .ok_or_else(|| fault("plan-index-in-arena"))? = deepest.saturating_add(own);
    }
    let root = slot(&back, plan.root)?;
    Ok(if future {
        root.saturating_add(margin)
    } else {
        root
    })
}

impl WindowState {
    /// FR-820: open the carried state of `clause` over a trace stamped by
    /// `clock`.
    ///
    /// # Errors
    ///
    /// A fault for a plan that does not lower.
    pub fn open(
        clause: &ClauseFacts,
        clause_identity: impl Into<String>,
        clock: WindowClock,
    ) -> Result<Self, TemporalRefusal> {
        let plan = clause.formula().plan().map_err(TemporalRefusal::Fault)?;
        reaches(plan).map_err(TemporalRefusal::Fault)?;
        let margin = clock.cover.margin();
        let back = span(plan, margin, false).map_err(TemporalRefusal::Fault)?;
        let ahead = span(plan, margin, true).map_err(TemporalRefusal::Fault)?;
        Ok(Self {
            clause_identity: clause_identity.into(),
            clock,
            positions: Vec::new(),
            fresh: 0,
            base: 0,
            first: None,
            cursor: 0,
            prior: Prior::default(),
            seed: Seed::default(),
            identity: IdentityStream::default(),
            stopped: None,
            back,
            ahead,
        })
    }

    /// Admit `observation`, stamped above every watermark an earlier
    /// evaluation read.
    pub fn admit(&mut self, observation: TimestampedObservation) {
        self.positions.push(observation);
        self.fresh = self.fresh.saturating_add(1);
    }

    /// The number of positions the state holds.
    pub fn retained(&self) -> usize {
        self.positions.len()
    }

    /// The position identities the summary cites.
    pub fn cited(&self) -> usize {
        self.prior.cited()
    }

    /// The number of observations admitted.
    pub fn len(&self) -> usize {
        self.base.saturating_add(self.positions.len())
    }

    /// Whether no observation was admitted.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The held observation at the absolute `position`, in canonical order.
    pub fn observation(&self, position: usize) -> Option<&TimestampedObservation> {
        position
            .checked_sub(self.base)
            .and_then(|index| self.positions.get(index))
    }

    /// FR-501's evaluation of `clause` over every observation admitted, with
    /// the `silent` samples after them (FR-504), under the effective
    /// `progress` the evaluator establishes windows by and the `asserted`
    /// progress the report names. `letters` reads the positions in canonical
    /// order by their absolute index.
    ///
    /// # Errors
    ///
    /// As [`super::evaluate_finite_window`]; the state is then unchanged.
    pub fn evaluate<L: TemporalLetters>(
        &mut self,
        clause: &ClauseFacts,
        silent: &[TimestampedObservation],
        progress: WindowProgress,
        asserted: WindowProgress,
        letters: &mut L,
        meter: &mut Meter,
    ) -> Result<WindowEvaluation, TemporalRefusal> {
        let mut positions = self.positions.clone();
        let kept = positions.len().saturating_sub(self.fresh);
        if let Some(fresh) = positions.get_mut(kept..) {
            fresh.sort_by(canonical);
        }
        if let (Some(last), Some(next)) = (
            kept.checked_sub(1).and_then(|index| positions.get(index)),
            positions.get(kept),
        ) {
            if canonical(last, next) != std::cmp::Ordering::Less {
                return Err(TemporalRefusal::Fault(fault(
                    "carried-observation-is-above-the-watermark",
                )));
            }
        }
        let first = self
            .first
            .or_else(|| positions.first().map(|position| position.stamp));
        let Some(first) = first else {
            return Err(TemporalRefusal::EmptyTrace);
        };
        let observed = self.base.saturating_add(positions.len());
        let held = positions.len();
        positions.extend(silent.iter().cloned());
        let trace = TimestampedTrace {
            positions,
            origin: Vec::new(),
            progress,
            asserted,
            observed,
            clock: self.clock.clone(),
            clause_identity: self.clause_identity.clone(),
            base: self.base,
            first,
        };
        let whole = anchors(clause, &trace, None);
        let range = match &self.stopped {
            Some(_) => whole.range.end..whole.range.end,
            None => self.cursor.max(whole.range.start)..whole.range.end,
        };
        let mut run = Run::new(
            clause,
            &trace,
            letters,
            meter,
            None,
            Anchors {
                range: range.clone(),
                guarded: whole.guarded,
            },
            self.seed.clone(),
        )?;
        let (done, tail) = match &self.stopped {
            Some(Ok(tail)) => (Vec::new(), tail.clone()),
            Some(Err(refusal)) => return Err(refusal.clone()),
            None => run.drive(&self.prior)?,
        };
        let evaluation = run
            .settle(&self.prior, &done, tail.clone(), &self.identity)
            .map_err(TemporalRefusal::Fault)?;

        // Summarize the activations no later position or progress changes.
        let mut prior = self.prior.clone();
        let mut cursor = range.start;
        let mut journal = 0;
        let mut stopped = self.stopped.clone();
        let ahead = self.ahead;
        let settled = |anchor: usize| run.settled(anchor, ahead).map_err(TemporalRefusal::Fault);
        for finished in &done {
            if !settled(finished.anchor)? {
                break;
            }
            prior
                .summarize(&run, finished)
                .map_err(TemporalRefusal::Fault)?;
            journal = finished.journal;
            cursor = finished.anchor.saturating_add(1);
        }
        let next = match (done.iter().find(|f| f.anchor >= cursor), &tail) {
            (Some(open), _) => open.anchor,
            (None, Tail::Done) => range.end,
            (None, _) => run.current,
        };
        // Anchors skipped by a false guard between them.
        while cursor < next && settled(cursor)? {
            cursor = cursor.saturating_add(1);
        }
        let permanent = stopped.is_none()
            && cursor == run.current
            && !matches!(tail, Tail::Done | Tail::Incomplete(_))
            && settled(run.current)?;
        if permanent {
            stopped = Some(Ok(tail));
            journal = run.journal.len();
        }
        let seed = if stopped.is_some() && self.stopped.is_some() {
            self.seed.clone()
        } else {
            run.seed_at(journal)
        };

        // Freeze the positions no later activation reads.
        let frontier = if stopped.is_some() {
            observed
        } else {
            let floor = progress.watermark.inclusive().saturating_add(1);
            let low = if cursor < observed {
                run.stamp_at(cursor)
                    .map_err(TemporalRefusal::Fault)?
                    .min(floor)
            } else {
                floor
            }
            .saturating_sub(self.back);
            let mut frontier = self.base;
            while frontier < cursor.min(observed)
                && run.stamp_at(frontier).map_err(TemporalRefusal::Fault)? < low
            {
                frontier = frontier.saturating_add(1);
            }
            frontier
        };
        let mut identity = self.identity.clone();
        let readings = {
            let mut readings: BTreeMap<usize, Vec<(usize, super::lift::Cell)>> = BTreeMap::new();
            for ((position, class), cell) in &seed.cells {
                if *position < frontier {
                    readings.entry(*position).or_default().push((*class, *cell));
                }
            }
            for cells in readings.values_mut() {
                cells.sort_unstable_by_key(|(class, _)| *class);
            }
            readings
        };
        for block in run
            .blocks(&readings, self.base..frontier)
            .map_err(TemporalRefusal::Fault)?
        {
            identity.freeze(&block).map_err(TemporalRefusal::Fault)?;
        }
        let mut seed = seed;
        let first_undefined = seed
            .undefined
            .iter()
            .next()
            .map(|(key, cause)| (*key, cause.clone()));
        seed.cells.retain(|(position, _), _| *position >= frontier);
        seed.undefined
            .retain(|(position, _), _| *position >= frontier);
        if let Some((key, cause)) = first_undefined {
            seed.undefined.insert(key, cause);
        }
        let mut positions = trace.positions;
        positions.truncate(held);
        let drop = frontier.saturating_sub(self.base).min(positions.len());
        positions.drain(..drop);

        self.positions = positions;
        self.fresh = 0;
        self.base = frontier;
        self.first = Some(first);
        self.cursor = cursor;
        self.prior = prior;
        self.seed = seed;
        self.identity = identity;
        self.stopped = stopped;
        Ok(evaluation)
    }
}
