// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timestamped trace and its progress (FR-501 Inputs).

use super::cover::ClockCover;
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::position::TemporalPosition;
use crate::temporal::window::{PastWindow, WindowTrace};
use filament_exact::{Integer, Rational};

/// Whether the completeness assertion covers every tick up to `tick`.
pub(super) fn covers(completeness: ProgressCompleteness, tick: i128) -> bool {
    match completeness {
        ProgressCompleteness::Complete { through } => i128::from(through) >= tick,
        ProgressCompleteness::Incomplete => false,
    }
}

/// Whether the trace's lower boundary is the origin of the history or only a
/// cutoff within it (FR-501 "Verdicts").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LowerBoundary {
    /// History before the first position is empty.
    AuthoritativeOrigin,
    /// History before the first position exists but was not supplied.
    HistoryCutoff,
}

/// A progress watermark as the supplier states it (FSpec FR-094).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Watermark {
    /// Progress covers every tick up to and including the value.
    Inclusive(u64),
    /// Progress covers every tick strictly below `end`, supplied without a
    /// FSpec FR-289 mapping that proves inclusive coverage of `end`: it is
    /// read as the inclusive watermark `end - 1`, and as no progress at all
    /// when `end` is 0.
    HalfOpen {
        /// The exclusive end.
        end: u64,
    },
}

impl Watermark {
    /// The greatest tick progress covers, or -1 when it covers none.
    pub(super) fn inclusive(self) -> i128 {
        match self {
            Self::Inclusive(tick) => i128::from(tick),
            Self::HalfOpen { end } => i128::from(end) - 1,
        }
    }
}

/// An order key: an order-authority identity and a key within it. Distinct
/// keys of one authority order two observations; keys of different
/// authorities, and a key repeated within one authority, do not (FSpec
/// FR-090-AC-4, FR-501 "usable order").
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OrderKey {
    /// The order authority.
    pub authority: String,
    /// The key within the authority.
    pub key: i64,
}

/// One admitted observation of a timestamped trace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimestampedObservation {
    /// The stamp: a non-negative count of ticks of the clock unit.
    pub stamp: u64,
    /// The order key, when the observation has one.
    pub order_key: Option<OrderKey>,
    /// The identity of the admitted observation; it enters the result
    /// identity so that a change of any valuation changes it.
    pub observation: String,
}

/// The progress a timestamped trace asserts (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowProgress {
    /// The watermark `W`.
    pub watermark: Watermark,
    /// The completeness assertion.
    pub completeness: ProgressCompleteness,
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The lower-boundary kind.
    pub lower_boundary: LowerBoundary,
}

/// The clock binding's unit and cover.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WindowClock {
    /// The declared timestamp unit.
    pub unit: String,
    /// The declared cover of a stamp.
    pub cover: ClockCover,
}

/// A non-empty timestamped trace in canonical order (FR-501 Inputs, "Canonical
/// order").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampedTrace {
    pub(super) positions: Vec<TimestampedObservation>,
    pub(super) origin: Vec<usize>,
    pub(super) progress: WindowProgress,
    /// The progress the request asserted, which the report and the result
    /// identity name. It is `progress` itself for a timestamped trace; a
    /// progressed trace (FR-504) reads an effective `progress` of its own.
    pub(super) asserted: WindowProgress,
    /// How many leading positions are observations: the positions after them
    /// are silent samples (FR-504), which no activation anchors on.
    pub(super) observed: usize,
    pub(super) clock: WindowClock,
    pub(super) clause_identity: String,
    /// The absolute index of `positions[0]`: 0 for a whole trace, the first
    /// position a carried state still holds otherwise (FR-820).
    pub(super) base: usize,
    /// The stamp of the trace's position 0, which a cutoff's history reads.
    pub(super) first: u64,
}

impl TimestampedTrace {
    /// A window trace over this call's positions or exact clock stamps,
    /// selected by the aggregate's package window kind.
    pub(crate) fn aggregate_window_trace(
        &self,
        window: &PastWindow,
    ) -> Result<WindowTrace, crate::temporal::window::WindowRefusal> {
        match window {
            PastWindow::Positions { .. } => Ok(WindowTrace::positions(
                u64::try_from(self.len()).unwrap_or(u64::MAX),
            )),
            PastWindow::Duration { .. } => WindowTrace::timed(
                self.positions
                    .iter()
                    .map(|position| Rational::from_integer(Integer::from(position.stamp)))
                    .collect(),
            ),
        }
    }

    /// Admit `positions`, given in any order. A trace with no position is
    /// [`TemporalRefusal::EmptyTrace`], refused before any position is
    /// visited. The positions are put in canonical order (FR-501 "Canonical
    /// order"): ascending stamp, then ascending order-authority identity (an
    /// observation without an order key after every keyed one), then
    /// ascending key within the authority, then ascending observation
    /// identity. The listing order decides nothing: the admitted position of
    /// an observation is its index in that order.
    pub fn admit(
        clause_identity: impl Into<String>,
        clock: WindowClock,
        positions: Vec<TimestampedObservation>,
        progress: WindowProgress,
    ) -> Result<Self, TemporalRefusal> {
        if positions.is_empty() {
            return Err(TemporalRefusal::EmptyTrace);
        }
        let mut trace = Self::open(clause_identity, clock, progress);
        trace.extend(positions, progress);
        Ok(trace)
    }

    /// FR-820: a trace with no position yet, which [`Self::extend`] grows
    /// as a carried state admits its positions. It is no admitted trace
    /// until it holds one.
    pub fn open(
        clause_identity: impl Into<String>,
        clock: WindowClock,
        progress: WindowProgress,
    ) -> Self {
        Self {
            observed: 0,
            base: 0,
            first: 0,
            positions: Vec::new(),
            origin: Vec::new(),
            progress,
            asserted: progress,
            clock,
            clause_identity: clause_identity.into(),
        }
    }

    /// FR-820: admit `positions`, listed after every position admitted
    /// already, under the `progress` now asserted. The result equals
    /// [`Self::admit`] over the whole listing. Positions that sort after the
    /// trace's last one in canonical order (FR-643 "Order" keeps a carried
    /// trace's later positions there) are appended, at work proportional to
    /// them; otherwise the whole listing is put in canonical order again.
    pub fn extend(&mut self, positions: Vec<TimestampedObservation>, progress: WindowProgress) {
        let offset = self.positions.len();
        let mut order: Vec<usize> = (0..positions.len()).collect();
        order.sort_by(
            |left, right| match (positions.get(*left), positions.get(*right)) {
                (Some(left), Some(right)) => canonical(left, right),
                _ => std::cmp::Ordering::Equal,
            },
        );
        let after = match (
            self.positions.last(),
            order.first().and_then(|at| positions.get(*at)),
        ) {
            (Some(last), Some(next)) => canonical(last, next) != std::cmp::Ordering::Greater,
            (None, _) | (_, None) => true,
        };
        if after {
            for index in order {
                if let Some(position) = positions.get(index) {
                    self.positions.push(position.clone());
                    self.origin.push(offset.saturating_add(index));
                }
            }
        } else {
            let mut listing: Vec<Option<TimestampedObservation>> = vec![None; offset];
            for (position, origin) in self.positions.drain(..).zip(self.origin.drain(..)) {
                if let Some(slot) = listing.get_mut(origin) {
                    *slot = Some(position);
                }
            }
            listing.extend(positions.into_iter().map(Some));
            let listing: Vec<TimestampedObservation> = listing.into_iter().flatten().collect();
            let mut order: Vec<usize> = (0..listing.len()).collect();
            order.sort_by(
                |left, right| match (listing.get(*left), listing.get(*right)) {
                    (Some(left), Some(right)) => canonical(left, right),
                    _ => std::cmp::Ordering::Equal,
                },
            );
            self.positions = order
                .iter()
                .filter_map(|index| listing.get(*index).cloned())
                .collect();
            self.origin = order;
        }
        self.first = self.positions.first().map_or(0, |position| position.stamp);
        self.observed = self.positions.len();
        self.progress = progress;
        self.asserted = progress;
    }

    /// FR-820: keep only the first `len` positions in canonical order, as a
    /// carried state undoing a call keeps the positions it had; a progressed
    /// trace's silent samples go with them. The trace is read again only
    /// once [`Self::extend`] or [`Self::extend_progressed`] has set its
    /// progress.
    pub fn truncate(&mut self, len: usize) {
        self.positions.truncate(len);
        self.origin.truncate(len);
        self.observed = self.observed.min(len);
    }

    /// The number of positions that are observations; the rest are silent
    /// samples.
    pub fn observed(&self) -> usize {
        self.observed
    }

    /// The number of positions.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the trace has no position; an admitted trace never has none.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The listing index of the observation at the canonical `position`: the
    /// observation a letter reads there is the one listed at that index.
    pub fn supplied_index(&self, position: TemporalPosition) -> Option<usize> {
        usize::try_from(position.index())
            .ok()
            .and_then(|index| self.origin.get(index).copied())
    }

    /// The admitted positions, in order.
    pub fn positions(&self) -> &[TimestampedObservation] {
        &self.positions
    }

    /// The progress asserted.
    pub fn progress(&self) -> &WindowProgress {
        &self.progress
    }

    /// The clock binding.
    pub fn clock(&self) -> &WindowClock {
        &self.clock
    }

    /// The progress the request asserted, which the report names.
    pub fn asserted(&self) -> &WindowProgress {
        &self.asserted
    }

    /// The identity of the clause the trace is read for.
    pub fn clause_identity(&self) -> &str {
        &self.clause_identity
    }
}

/// FR-501 "Canonical order": ascending stamp, then ascending order-authority
/// identity (an observation without an order key after every keyed one),
/// then ascending key within the authority, then ascending observation
/// identity.
pub(super) fn canonical(
    left: &TimestampedObservation,
    right: &TimestampedObservation,
) -> std::cmp::Ordering {
    left.stamp
        .cmp(&right.stamp)
        .then_with(|| match (&left.order_key, &right.order_key) {
            (Some(left), Some(right)) => left.cmp(right),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .then_with(|| left.observation.cmp(&right.observation))
}
