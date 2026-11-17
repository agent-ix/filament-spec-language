// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timestamped trace and its progress (FR-501 Inputs).

use super::cover::{ClockCover, Extent, StampInterval};
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

/// FR-501 "Late-data cutoff": how late an observation may arrive after its
/// event instant and still be read (FSpec FR-094).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LateCutoff {
    /// An observation with event instant `t` arriving at `A > t + lateness`
    /// is late.
    AllowedLateness {
        /// The lateness, in ticks.
        lateness: u64,
    },
    /// An observation with event instant `t <= cutoff` arriving at `A >
    /// cutoff` is late.
    FixedInstant {
        /// The cutoff instant, in ticks.
        cutoff: u64,
    },
}

impl LateCutoff {
    /// Whether an observation whose event instant is `instant` and that
    /// arrived at the watermark `arrival` is late.
    pub fn is_late(self, instant: u64, arrival: u64) -> bool {
        match self {
            Self::AllowedLateness { lateness } => {
                i128::from(arrival) > i128::from(instant) + i128::from(lateness)
            }
            Self::FixedInstant { cutoff } => instant <= cutoff && arrival > cutoff,
        }
    }

    /// The watermark a window with deadline `deadline` needs before it is
    /// established under this cutoff: `D + lateness`, or `max(D, cutoff)`.
    fn settles(self, deadline: i128) -> i128 {
        match self {
            Self::AllowedLateness { lateness } => deadline.saturating_add(i128::from(lateness)),
            Self::FixedInstant { cutoff } => deadline.max(i128::from(cutoff)),
        }
    }
}

/// One observation a late-data cutoff excluded, as the result identity
/// reads it (FR-966 "Identity").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LateObservation {
    /// The observation's identity (FR-106).
    pub observation: String,
    /// Its nominal stamp.
    pub stamp: u64,
    /// Its interval stamp, when it has one.
    pub interval: Option<StampInterval>,
    /// Its arrival watermark.
    pub arrival: u64,
}

/// The late-data inputs of a trace (FR-501 "Late-data cutoff", FR-966
/// "Identity"): the cutoff, the arrival watermark of every read
/// observation in canonical order, the observations it excluded and the
/// late ones a revisioned repair admitted (FR-526).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Lateness {
    /// The cutoff, when one is selected.
    pub cutoff: Option<LateCutoff>,
    /// The arrival watermark of each read position that states or is
    /// given one, in canonical order.
    pub arrivals: Vec<Option<u64>>,
    /// The observations the cutoff excluded, in input order.
    pub late: Vec<LateObservation>,
    /// The identities of the late observations a repair admitted.
    pub admitted: Vec<String>,
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
    /// Each position's interval stamp, in canonical order (FR-501 "Bounded
    /// cover and interval stamps").
    pub(super) intervals: Vec<Option<StampInterval>>,
    /// The late-data inputs.
    pub(super) lateness: Lateness,
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
        let positions = positions
            .into_iter()
            .map(|position| (position, None))
            .collect();
        Self::admit_uncertain(clause_identity, clock, positions, progress)
    }

    /// [`Self::admit`] for observations that may carry their own interval
    /// stamp, which overrides the cover for that observation (FR-501
    /// "Bounded cover and interval stamps"). Canonical order sorts by each
    /// observation's earliest instant, then its latest, then its stamp, the
    /// order key and the observation identity; under one cover for every
    /// stamp that is the stamp order.
    ///
    /// # Errors
    ///
    /// As [`Self::admit`].
    pub fn admit_uncertain(
        clause_identity: impl Into<String>,
        clock: WindowClock,
        positions: Vec<(TimestampedObservation, Option<StampInterval>)>,
        progress: WindowProgress,
    ) -> Result<Self, TemporalRefusal> {
        if positions.is_empty() {
            return Err(TemporalRefusal::EmptyTrace);
        }
        // Without an interval stamp the canonical order is the stamp order
        // FR-820's carried trace keeps, so the trace is the one it grows.
        if positions.iter().all(|(_, interval)| interval.is_none()) {
            let mut trace = Self::open(clause_identity, clock, progress);
            trace.extend(positions.into_iter().map(|(position, _)| position).collect(), progress);
            return Ok(trace);
        }
        let extent = |(position, interval): &(TimestampedObservation, Option<StampInterval>)| {
            interval.map_or_else(|| clock.cover.extent(position.stamp), Extent::of_interval)
        };
        let mut order: Vec<usize> = (0..positions.len()).collect();
        order.sort_by(|left, right| {
            let (Some(left_entry), Some(right_entry)) =
                (positions.get(*left), positions.get(*right))
            else {
                return std::cmp::Ordering::Equal;
            };
            let (left_extent, right_extent) = (extent(left_entry), extent(right_entry));
            let (left, right) = (&left_entry.0, &right_entry.0);
            left_extent
                .lo()
                .cmp(&right_extent.lo())
                .then_with(|| left_extent.hi().cmp(&right_extent.hi()))
                .then_with(|| left.stamp.cmp(&right.stamp))
                .then_with(|| match (&left.order_key, &right.order_key) {
                    (Some(left), Some(right)) => left.cmp(right),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| left.observation.cmp(&right.observation))
        });
        let (admitted, intervals): (Vec<TimestampedObservation>, Vec<Option<StampInterval>>) =
            order
                .iter()
                .filter_map(|index| positions.get(*index).cloned())
                .unzip();
        let mut trace = Self {
            intervals,
            lateness: Lateness::default(),
            observed: admitted.len(),
            positions: admitted,
            origin: order,
            base: 0,
            first: 0,
            progress,
            asserted: progress,
            clock,
            clause_identity: clause_identity.into(),
        };
        trace.first = trace.positions.first().map_or(0, |position| position.stamp);
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
            intervals: Vec::new(),
            lateness: Lateness::default(),
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
        self.intervals.truncate(len);
        self.observed = self.observed.min(len);
    }

    /// This trace read under the late-data inputs `lateness` (FR-501
    /// "Late-data cutoff"): its positions are the on-time observations, and
    /// the cutoff delays establishment and enters the result identity.
    #[must_use]
    pub fn with_lateness(mut self, lateness: Lateness) -> Self {
        self.lateness = lateness;
        self
    }

    /// The instants the observation at the canonical `position` may lie at:
    /// its interval stamp, or the cover of its stamp.
    pub(super) fn extent(&self, position: usize) -> Option<Extent> {
        let observed = self.positions.get(position)?;
        Some(self.intervals.get(position).copied().flatten().map_or_else(
            || self.clock.cover.extent(observed.stamp),
            Extent::of_interval,
        ))
    }

    /// Whether the observation at `position` carries its own interval stamp.
    pub(super) fn has_interval(&self, position: usize) -> bool {
        self.intervals.get(position).copied().flatten().is_some()
    }

    /// The establishment margin: the cover's, or the largest upper widening
    /// `latest - t` of an interval stamp, whichever is greater (FR-501
    /// "Bounded cover and interval stamps").
    pub(super) fn margin(&self) -> i128 {
        self.positions
            .iter()
            .zip(&self.intervals)
            .filter_map(|(observed, interval)| {
                interval.map(|interval| i128::from(interval.latest) - i128::from(observed.stamp))
            })
            .fold(self.clock.cover.margin(), i128::max)
    }

    /// The watermark an activation whose window ends at `need` waits for:
    /// `need` itself, or later under a late-data cutoff (FR-501 "Late-data
    /// cutoff").
    pub(super) fn settling(&self, need: i128) -> i128 {
        self.lateness
            .cutoff
            .map_or(need, |cutoff| cutoff.settles(need))
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
