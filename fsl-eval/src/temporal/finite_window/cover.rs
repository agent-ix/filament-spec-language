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

use filament_observation::authority::partial::EventTimeInterval;
use filament_observation::authority::Limits;
use filament_observation::Identity;

use super::trace::WindowClock;

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

/// The suffix FR-944 gives the unit identity of an owner interval: the
/// quarter-tick scale, in which a stamp `k` of the clock unit is `4k`.
const QUARTER_SUFFIX: &str = "/4";

/// The identity every part of a [`Domain`] takes when nothing is bound to
/// it, which the owner would otherwise refuse as blank.
const UNBOUND: &str = "tick";

/// The owner domain of the intervals of one comparison (FR-944 "Partial
/// values and interval order"): the clock binding name, the profile identity
/// and the clock unit, whose identity takes the suffix [`QUARTER_SUFFIX`].
/// Intervals of two clocks, profiles or units are different domains, which the
/// owner refuses to order, so a comparison across them is not made.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Domain<'a> {
    clock: &'a str,
    profile: &'a str,
    unit: &'a str,
}

impl<'a> Domain<'a> {
    /// The domain of the clock binding `clock`.
    pub(super) fn of(clock: &'a WindowClock) -> Self {
        Self {
            clock: &clock.name,
            profile: &clock.profile,
            unit: &clock.unit,
        }
    }

    /// The domain of a cover compared where no clock is bound: every
    /// identity is `tick`. Only [`ClockCover`]'s own comparison methods,
    /// which take no clock, read it; a trace always reads its own clock.
    const UNBOUND: Domain<'static> = Domain {
        clock: UNBOUND,
        profile: UNBOUND,
        unit: UNBOUND,
    };

    /// An interval of the quarter-tick endpoints `earliest` and `latest` in
    /// this domain.
    fn interval(self, earliest: i128, latest: i128) -> EventTimeInterval {
        let identity = |text: &str| {
            Identity::new(if text.trim().is_empty() {
                UNBOUND
            } else {
                text
            })
        };
        // The endpoints are ordered (an open end moves inward by one quarter
        // tick and an extent spans at least one whole tick or is closed) and
        // below 2^68 in magnitude, far inside the owner's string bound; the
        // identities are non-blank.
        EventTimeInterval::new(
            identity(self.clock),
            identity(self.profile),
            Identity::new(format!("{}{QUARTER_SUFFIX}", self.unit)),
            earliest,
            latest,
            Limits::owner_max(),
        )
        .expect("a quarter-tick extent is a valid owner interval")
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
            Self::Exact => Extent::new(stamp, stamp, false, false),
            Self::Counter { uncertainty } => Extent::new(
                stamp - i128::from(uncertainty),
                stamp + 1 + i128::from(uncertainty),
                false,
                true,
            ),
            Self::Bounded { uncertainty } => Extent::new(
                stamp - i128::from(uncertainty),
                stamp + i128::from(uncertainty),
                false,
                false,
            ),
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
    /// stamps"). Canonical order is FSL's own sort key over whole-tick
    /// endpoints, not an order of two intervals, so it stays out of the
    /// owner's `relation_to`.
    pub fn order_span(self, stamp: u64, interval: Option<StampInterval>) -> (i128, i128) {
        let extent = interval.map_or_else(|| self.extent(stamp), Extent::of_interval);
        (extent.lo, extent.hi)
    }

    /// FSpec FR-160's `within_bound`: whether `ack - request <= bound` holds
    /// for every pair of instants in the covers of the readings `request_tick`
    /// and `ack_tick`, for none, or for some. It reads the owner's order of
    /// `ack` against `request` shifted by the bound (FR-944). No clock is bound here, so the
    /// owner intervals take the `tick` domain (`tick/4`); a trace compares its
    /// extents in its own clock's domain.
    pub fn within_bound(self, bound: i64, request_tick: u64, ack_tick: u64) -> Containment {
        Extent::within_bound(
            self.extent(request_tick),
            self.extent(ack_tick),
            bound,
            Domain::UNBOUND,
        )
    }

    /// FSpec FR-160's `definitely_before`: whether every instant in the cover
    /// of the reading `a` lies below every instant in the cover of `b`,
    /// as the owner's `relation_to` answers it (FR-944). As for
    /// [`Self::within_bound`], the `tick` domain stands in where no clock is
    /// bound.
    pub fn definitely_before(self, a: u64, b: u64) -> bool {
        self.extent(a).before(self.extent(b), Domain::UNBOUND)
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
///
/// Every comparison reads the extent as FR-944's quarter-tick interval:
/// scaling is uniform, a closed endpoint `c` is `4c`, an open lower end `a`
/// is `4a + 1` and an open upper end `b` is `4b - 1`. An open end moves
/// inward by less than the distance to the next whole tick, so no scaled
/// endpoint crosses an integer threshold `4b` unless the true one does. The
/// scaling runs in `i128` from `u64` stamps and `i64` bounds, so it cannot
/// overflow and has no refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Extent {
    /// The whole-tick endpoints, which canonical order sorts by.
    lo: i128,
    hi: i128,
    /// The quarter-tick endpoints of the closed interval.
    earliest: i128,
    latest: i128,
}

impl Extent {
    /// The extent `[lo, hi]` of whole ticks with each end open or closed.
    fn new(lo: i128, hi: i128, lo_open: bool, hi_open: bool) -> Self {
        Self {
            lo,
            hi,
            earliest: 4 * lo + i128::from(lo_open),
            latest: 4 * hi - i128::from(hi_open),
        }
    }

    /// The instants `interval` covers.
    pub(super) fn of_interval(interval: StampInterval) -> Self {
        Self::new(
            i128::from(interval.earliest),
            i128::from(interval.latest),
            interval.earliest_open,
            interval.latest_open,
        )
    }

    /// The least instant, which canonical order sorts by first.
    pub(super) fn lo(self) -> i128 {
        self.lo
    }

    /// The greatest instant, which canonical order sorts by second.
    pub(super) fn hi(self) -> i128 {
        self.hi
    }

    /// The owner interval of this extent, its quarter-tick endpoints moved
    /// by `shift` quarter ticks, in `domain` (FR-944). Nothing writes it into
    /// an owner document.
    fn owner_interval(self, domain: Domain<'_>, shift: i128) -> EventTimeInterval {
        domain.interval(self.earliest + shift, self.latest + shift)
    }

    /// The owner's answer for the order of `left` against `right`.
    fn relation(
        left: &EventTimeInterval,
        right: &EventTimeInterval,
    ) -> filament_observation::authority::partial::OrderPossibilities {
        left.relation_to(right, Limits::owner_max())
            .expect("two extents of one unit share an owner domain")
    }

    /// FSpec FR-160's `within_bound` on the extents of a request and its
    /// acknowledgement: the order of `ack` against `request` shifted by
    /// `bound` whole ticks (FR-944): `True` when it admits no `after`,
    /// `False` when it admits neither `before` nor `equal`, and
    /// `Indeterminate` otherwise.
    pub(super) fn within_bound(
        request: Self,
        ack: Self,
        bound: i64,
        domain: Domain<'_>,
    ) -> Containment {
        let order = Self::relation(
            &ack.owner_interval(domain, 0),
            &request.owner_interval(domain, 4 * i128::from(bound)),
        );
        if !order.after() {
            Containment::True
        } else if !order.before() && !order.equal() {
            Containment::False
        } else {
            Containment::Indeterminate
        }
    }

    /// Whether every instant of `self` lies below every instant of `other`.
    pub(super) fn before(self, other: Self, domain: Domain<'_>) -> bool {
        Self::relation(
            &self.owner_interval(domain, 0),
            &other.owner_interval(domain, 0),
        )
        .definitely_before()
    }

    /// Whether the two extents share an instant: neither lies wholly before
    /// the other, which is the owner's `equal`.
    pub(super) fn overlaps(self, other: Self, domain: Domain<'_>) -> bool {
        Self::relation(
            &self.owner_interval(domain, 0),
            &other.owner_interval(domain, 0),
        )
        .equal()
    }

    /// Whether the reading `candidate` lies at distance `[lower, upper]`
    /// from the reading `anchor`, forward when `forward` and backward
    /// otherwise: for every pair of instants, for none, or for some. A
    /// reading's distance to itself is exactly 0 (the same instant).
    ///
    /// The difference `later - earlier` has the scaled endpoints
    /// `later.earliest - earlier.latest` and `later.latest -
    /// earlier.earliest`. `at_least lower` is decided against `4 * lower`:
    /// every pair when the scaled lower end reaches it, no pair when the
    /// scaled upper end is below it. `at_most upper` mirrors it. No owner
    /// interval with an unbounded end is formed (FR-944).
    pub(super) fn eligible(
        anchor: Self,
        candidate: Self,
        same_position: bool,
        forward: bool,
        lower: u64,
        upper: u64,
    ) -> Containment {
        let (earlier, later) = if forward {
            (anchor, candidate)
        } else {
            (candidate, anchor)
        };
        let (least, greatest) = if same_position {
            (0, 0)
        } else {
            (
                later.earliest - earlier.latest,
                later.latest - earlier.earliest,
            )
        };
        let (lower, upper) = (4 * i128::from(lower), 4 * i128::from(upper));
        let at_least = if least >= lower {
            Containment::True
        } else if greatest < lower {
            Containment::False
        } else {
            Containment::Indeterminate
        };
        let at_most = if greatest <= upper {
            Containment::True
        } else if least > upper {
            Containment::False
        } else {
            Containment::Indeterminate
        };
        at_least.and(at_most)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn interval(earliest: u64, latest: u64, earliest_open: bool, latest_open: bool) -> Extent {
        Extent::of_interval(StampInterval {
            earliest,
            latest,
            earliest_open,
            latest_open,
        })
    }

    /// FR-944-AC-6: `at_least` and `at_most` decided on the scaled
    /// endpoints against `4b`.
    #[trace("TC-2624", "FR-944-AC-6")]
    #[test]
    fn one_sided_bounds_are_decided_on_the_scaled_endpoints() {
        let anchor = interval(0, 0, false, false);
        let reading = |cover: ClockCover, k| cover.extent(k);
        let counter = |u| ClockCover::Counter { uncertainty: u };
        let far = 1_000;
        let eligible = |candidate, lower, upper| {
            Extent::eligible(anchor, candidate, false, true, lower, upper)
        };
        // at_least 2
        assert_eq!(eligible(reading(counter(0), 2), 2, far), Containment::True);
        assert_eq!(eligible(reading(counter(0), 1), 2, far), Containment::False);
        assert_eq!(
            eligible(reading(counter(1), 2), 2, far),
            Containment::Indeterminate
        );
        // at_most 2
        assert_eq!(eligible(reading(counter(0), 1), 0, 2), Containment::True);
        assert_eq!(eligible(reading(counter(0), 3), 0, 2), Containment::False);
        assert_eq!(
            eligible(reading(counter(0), 2), 0, 2),
            Containment::Indeterminate
        );
        // A candidate of interval (1, 2] and one of [0, 1).
        assert_eq!(
            eligible(interval(1, 2, true, false), 1, far),
            Containment::True
        );
        assert_eq!(
            eligible(interval(0, 1, false, true), 1, far),
            Containment::False
        );
        assert_eq!(
            eligible(interval(0, 1, false, true), 0, 0),
            Containment::Indeterminate
        );
        // Against an anchor of interval (0, 1] a candidate of [3, 4).
        let open_anchor = interval(0, 1, true, false);
        let candidate = interval(3, 4, false, true);
        assert_eq!(
            Extent::eligible(open_anchor, candidate, false, true, 0, 3),
            Containment::Indeterminate
        );
        assert_eq!(
            Extent::eligible(open_anchor, candidate, false, true, 2, far),
            Containment::True
        );
        // Backward reads the anchor as the later end.
        assert_eq!(
            Extent::eligible(open_anchor, candidate, false, false, 0, far),
            Containment::False
        );
        assert_eq!(
            Extent::eligible(anchor, anchor, true, true, 0, 0),
            Containment::True
        );
    }

    /// FR-944-AC-5 on the extents: a doubly-open difference whose supremum is
    /// the bound plus one is indeterminate.
    #[trace("TC-2624", "FR-944-AC-5")]
    #[test]
    fn a_doubly_open_difference_is_indeterminate_at_the_bound_plus_one() {
        let request = interval(0, 1, true, false);
        assert_eq!(
            Extent::within_bound(request, interval(3, 4, false, true), 3, Domain::UNBOUND),
            Containment::Indeterminate
        );
        assert_eq!(
            Extent::within_bound(request, interval(2, 3, false, true), 3, Domain::UNBOUND),
            Containment::True
        );
    }

    /// FR-944-AC-7: the scaled endpoints are exact at the edge of the `u64`
    /// reading and `i64` bound domain, and nothing refuses.
    #[trace("TC-2624", "FR-944-AC-7")]
    #[test]
    fn the_scaled_endpoints_are_exact_at_the_domain_edge() {
        let max = u64::MAX;
        let cover = ClockCover::Counter { uncertainty: max };
        let widest = cover.extent(max);
        assert_eq!(
            (widest.earliest, widest.latest),
            (0, 147_573_952_589_676_412_923)
        );
        let zero = cover.extent(0);
        assert_eq!(
            (zero.earliest, zero.latest),
            (-73_786_976_294_838_206_460, 73_786_976_294_838_206_463)
        );
        for (bound, scaled) in [
            (i64::MAX, 36_893_488_147_419_103_228_i128),
            (i64::MIN, -36_893_488_147_419_103_232),
        ] {
            assert_eq!(4 * i128::from(bound), scaled);
            // Neither a refusal nor a wrap: the owner answers.
            let _ = cover.within_bound(bound, 0, max);
        }
        assert!(!cover.definitely_before(0, max));
    }

    fn clock(name: &str, profile: &str, unit: &str) -> WindowClock {
        WindowClock {
            name: name.to_owned(),
            profile: profile.to_owned(),
            unit: unit.to_owned(),
            cover: ClockCover::Exact,
        }
    }

    /// FR-944: an owner interval is in its clock's own domain (clock binding
    /// name, profile identity and the unit with `/4`), so the intervals of
    /// two different clocks are not comparable, and the bare `tick` domain
    /// is no clock's.
    #[trace("TC-2624", "FR-944-AC-1")]
    #[test]
    fn intervals_of_different_clocks_are_different_owner_domains() {
        let extent = ClockCover::Exact.extent(3);
        let base = clock("c", "p", "ms");
        let own = |clock: &WindowClock| extent.owner_interval(Domain::of(clock), 0);
        let comparable = |left: &WindowClock, right: &WindowClock| {
            own(left)
                .relation_to(&own(right), Limits::owner_max())
                .is_ok()
        };
        assert!(comparable(&base, &clock("c", "p", "ms")));
        assert!(!comparable(&base, &clock("d", "p", "ms")), "another clock");
        assert!(
            !comparable(&base, &clock("c", "q", "ms")),
            "another profile"
        );
        assert!(!comparable(&base, &clock("c", "p", "us")), "another unit");
        let bare = extent.owner_interval(Domain::UNBOUND, 0);
        assert!(own(&base).relation_to(&bare, Limits::owner_max()).is_err());
        assert_eq!(own(&base).unit().as_str(), "ms/4");
        assert_eq!(own(&base).clock_identity().as_str(), "c");
        assert_eq!(own(&base).clock_revision().as_str(), "p");
        assert_eq!(bare.unit().as_str(), "tick/4");
    }
}
