// SPDX-License-Identifier: AGPL-3.0-or-later
//! The clock binding's cover of a stamp and the comparisons it supports
//! (FR-501 "Counter cover"; FSpec FR-160 "Counter-tick intervals and
//! endpoints"; ADR-026 MN-2).
//!
//! Under [`ClockCover::Exact`] a stamp is the instant itself and a difference
//! of two stamps is an integer. Under [`ClockCover::Counter`] a counter
//! reading `k` covers the half-open interval `[k, k+1)` of the counter unit,
//! widened by the uncertainty `u` to `[k-u, k+1+u)`, so the difference of the
//! covers of two different readings is an open interval whose endpoints are
//! integers, and a comparison against a bound is decided only when every pair
//! of instants in the two covers agrees. Under [`ClockCover::Bounded`] a stamp
//! `t` covers the closed interval `[t-u, t+u]`, and an observation's own
//! [`StampInterval`] overrides the cover for it (FR-501 "Bounded cover and
//! interval stamps"): every comparison reads the two observations' extents.

/// What a clock binding declares a stamp to cover (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClockCover {
    /// A stamp is the instant itself (the default).
    Exact,
    /// A counter reading `k` covers `[k, k+1)` widened by `uncertainty` ticks
    /// on each side.
    Counter {
        /// The widening `u`, in ticks.
        uncertainty: u64,
    },
    /// FR-501 "Bounded cover and interval stamps": a point stamp `t` covers
    /// the closed interval `[t-u, t+u]` (FSpec FR-160 "Timestamp uncertainty
    /// beyond counter ticks").
    Bounded {
        /// The widening `u`, in ticks.
        uncertainty: u64,
    },
}

/// The answer of a comparison over covers: every pair of instants agrees, or
/// the covers straddle the bound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Containment {
    /// Every pair of instants satisfies the comparison.
    True,
    /// No pair of instants satisfies it.
    False,
    /// Some pairs do and some do not: the covers straddle the bound.
    Indeterminate,
}

impl Containment {
    /// The conjunction of two comparisons: `False` when either is, `True`
    /// only when both are.
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            (Self::True | Self::Indeterminate, Self::Indeterminate)
            | (Self::Indeterminate, Self::True) => Self::Indeterminate,
        }
    }
}

/// A difference of two extents: `lo .. hi` with each end open or closed, a
/// single point when `lo == hi` and both ends are closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Difference {
    lo: i128,
    hi: i128,
    lo_open: bool,
    hi_open: bool,
}

impl Difference {
    /// The exact difference `value`.
    const fn point(value: i128) -> Self {
        Self {
            lo: value,
            hi: value,
            lo_open: false,
            hi_open: false,
        }
    }

    /// `d >= bound` for every `d`, for no `d`, or for some.
    fn at_least(self, bound: i128) -> Containment {
        if self.lo >= bound {
            Containment::True
        } else if self.hi < bound || (self.hi == bound && self.hi_open) {
            Containment::False
        } else {
            Containment::Indeterminate
        }
    }

    /// `d <= bound` for every `d`, for no `d`, or for some.
    fn at_most(self, bound: i128) -> Containment {
        if self.hi <= bound {
            Containment::True
        } else if self.lo > bound || (self.lo == bound && self.lo_open) {
            Containment::False
        } else {
            Containment::Indeterminate
        }
    }
}

impl ClockCover {
    /// The margin `m` an establishment adds to a clause's future reach:
    /// `2u` under a counter cover, `u` under a bounded one and 0 under an
    /// exact one (FR-501 "Established window", "Bounded cover and interval
    /// stamps").
    pub fn margin(self) -> i128 {
        match self {
            Self::Exact => 0,
            Self::Counter { uncertainty } => 2 * i128::from(uncertainty),
            Self::Bounded { uncertainty } => i128::from(uncertainty),
        }
    }

    /// The instants the cover gives the reading `stamp`: `[t, t]` under
    /// `Exact`, `[k-u, k+1+u)` under `Counter` and `[t-u, t+u]` under
    /// `Bounded`.
    pub(super) fn extent(self, stamp: u64) -> Extent {
        let stamp = i128::from(stamp);
        match self {
            Self::Exact => Extent {
                lo: stamp,
                hi: stamp,
                lo_open: false,
                hi_open: false,
            },
            Self::Counter { uncertainty } => Extent {
                lo: stamp - i128::from(uncertainty),
                hi: stamp + 1 + i128::from(uncertainty),
                lo_open: false,
                hi_open: true,
            },
            Self::Bounded { uncertainty } => Extent {
                lo: stamp - i128::from(uncertainty),
                hi: stamp + i128::from(uncertainty),
                lo_open: false,
                hi_open: false,
            },
        }
    }

    /// FR-966 "The classified instant": the upper end of the uncertainty
    /// of an observation with nominal stamp `stamp` and, when it has one,
    /// `interval`: the interval's `latest` whatever its flag; otherwise `t +
    /// u` under `Bounded`, `k + u` under `Counter` (the last whole tick of
    /// its half-open cover) and `t` under `Exact`.
    pub fn latest_instant(self, stamp: u64, interval: Option<StampInterval>) -> u64 {
        if let Some(interval) = interval {
            return interval.latest;
        }
        match self {
            Self::Exact => stamp,
            Self::Counter { uncertainty } | Self::Bounded { uncertainty } => {
                stamp.saturating_add(uncertainty)
            }
        }
    }

    /// The earliest and the latest instant of an observation with nominal
    /// stamp `stamp` and, when it has one, `interval`: what canonical order
    /// sorts by before the stamp (FR-501 "Bounded cover and interval
    /// stamps").
    pub fn order_span(self, stamp: u64, interval: Option<StampInterval>) -> (i128, i128) {
        let extent = interval.map_or_else(|| self.extent(stamp), Extent::of_interval);
        (extent.lo, extent.hi)
    }

    /// FSpec FR-160's `within_bound`: whether `ack - request <= bound` holds
    /// for every pair of instants in the covers of the readings `request_tick`
    /// and `ack_tick`, for none, or for some.
    pub fn within_bound(self, bound: i64, request_tick: u64, ack_tick: u64) -> Containment {
        Extent::difference(self.extent(request_tick), self.extent(ack_tick))
            .at_most(i128::from(bound))
    }

    /// FSpec FR-160's `definitely_before`: whether every instant in the cover
    /// of the reading `a` lies below every instant in the cover of `b`.
    pub fn definitely_before(self, a: u64, b: u64) -> bool {
        self.extent(a).before(self.extent(b))
    }
}

/// FR-501 "Bounded cover and interval stamps": one observation's own
/// uncertainty interval `[earliest, latest]`, with an open or closed flag at
/// each end, which overrides the trace's cover for that observation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StampInterval {
    /// The earliest instant.
    pub earliest: u64,
    /// The latest instant.
    pub latest: u64,
    /// Whether `earliest` itself is excluded.
    pub earliest_open: bool,
    /// Whether `latest` itself is excluded.
    pub latest_open: bool,
}

impl StampInterval {
    /// FR-966 "Interval stamp admission": whether the interval is well
    /// formed (`earliest <= latest`, and not a single instant with an open
    /// end) and holds the nominal stamp `stamp`.
    pub fn admits(self, stamp: u64) -> bool {
        let formed = self.earliest < self.latest
            || (self.earliest == self.latest && !self.earliest_open && !self.latest_open);
        let above = if self.earliest_open {
            stamp > self.earliest
        } else {
            stamp >= self.earliest
        };
        let below = if self.latest_open {
            stamp < self.latest
        } else {
            stamp <= self.latest
        };
        formed && above && below
    }
}

/// The instants one observation may lie at: its interval stamp, or the
/// cover of its nominal stamp.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Extent {
    lo: i128,
    hi: i128,
    lo_open: bool,
    hi_open: bool,
}

impl Extent {
    /// The instants `interval` covers.
    pub(super) fn of_interval(interval: StampInterval) -> Self {
        Self {
            lo: i128::from(interval.earliest),
            hi: i128::from(interval.latest),
            lo_open: interval.earliest_open,
            hi_open: interval.latest_open,
        }
    }

    /// The least instant, which canonical order sorts by first.
    pub(super) fn lo(self) -> i128 {
        self.lo
    }

    /// The greatest instant, which canonical order sorts by second.
    pub(super) fn hi(self) -> i128 {
        self.hi
    }

    /// The difference `later - earlier` over every pair of instants of the
    /// two extents (FSpec FR-160's difference interval).
    fn difference(earlier: Self, later: Self) -> Difference {
        Difference {
            lo: later.lo - earlier.hi,
            hi: later.hi - earlier.lo,
            lo_open: later.lo_open || earlier.hi_open,
            hi_open: later.hi_open || earlier.lo_open,
        }
    }

    /// Whether every instant of `self` lies below every instant of `other`.
    pub(super) fn before(self, other: Self) -> bool {
        self.hi < other.lo || (self.hi == other.lo && (self.hi_open || other.lo_open))
    }

    /// Whether the two extents share an instant: neither lies wholly before
    /// the other.
    pub(super) fn overlaps(self, other: Self) -> bool {
        !self.before(other) && !other.before(self)
    }

    /// Whether the reading `candidate` lies at distance `[lower, upper]`
    /// from the reading `anchor`, forward when `forward` and backward
    /// otherwise: for every pair of instants, for none, or for some. A
    /// reading's distance to itself is exactly 0 (the same instant).
    pub(super) fn eligible(
        anchor: Self,
        candidate: Self,
        same_position: bool,
        forward: bool,
        lower: u64,
        upper: u64,
    ) -> Containment {
        let difference = if same_position {
            Difference::point(0)
        } else if forward {
            Self::difference(anchor, candidate)
        } else {
            Self::difference(candidate, anchor)
        };
        difference
            .at_least(i128::from(lower))
            .and(difference.at_most(i128::from(upper)))
    }
}
