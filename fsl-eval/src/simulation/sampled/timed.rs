// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-253/FR-254: exact conditioning of timed delay laws over enabled windows.

use filament_exact::{Integer, Rational};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::DelayDistribution;
use ix_trace_rs::implements;

use super::super::bounds::{exp_neg, ln, significant_floor};
use super::super::choice::{uniform_grid_value, weighted_choice_counted, ChoicePoint};
use super::super::model::DelayInterval;

/// Why a delay law has no supported draw in an enabled window.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TimedDelayRefusal {
    /// No support point or positive probability mass intersects the window.
    #[error("the delay distribution has no mass in its window")]
    NoMass,
    /// A uniform distribution over an unbounded window has no finite law.
    #[error("a uniform delay has an unbounded window")]
    UnboundedUniform,
    /// The requested uniform grid exponent is above the sampler's 256-bit
    /// draw domain.
    #[error("delay grid exponent {q} exceeds 256")]
    GridExponent {
        /// The refused exponent.
        q: u32,
    },
    /// An exact-arithmetic invariant broke.
    #[error("internal fault: {0:?}")]
    Internal(InternalFault),
}

impl From<InternalFault> for TimedDelayRefusal {
    fn from(fault: InternalFault) -> Self {
        Self::Internal(fault)
    }
}

/// One delay drawn from a law, and what drawing it used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedDelayDraw {
    /// The drawn delay.
    pub delay: Rational,
    /// The probability of this draw: a discrete delay's weight over the
    /// enabled weights, `1/n` over `n` points, `2^-q` on the grid, and 1
    /// where the delay is determined.
    pub probability: Rational,
    /// Whether the draw used the sampler choice it was given, so the next
    /// draw or tie-break takes the next choice index.
    pub consumed_choice: bool,
}

impl TimedDelayDraw {
    /// A delay the law and the windows determine, with no choice.
    fn determined(delay: Rational) -> Self {
        Self {
            delay,
            probability: Rational::from_integer(Integer::one()),
            consumed_choice: false,
        }
    }

    /// A delay taken from the grid value of one choice. At `q = 0` the grid
    /// is the one value 0 and no choice is used.
    fn on_grid(delay: Rational, q: u32) -> Self {
        Self {
            delay,
            probability: Rational::from_integer(Integer::one())
                .divided_by_power_of_two(u64::from(q)),
            consumed_choice: q > 0,
        }
    }
}

/// Draw one exact delay from `distribution` conditioned on the union of
/// `windows`, using `at` as its one sampler choice (FR-253/FR-254). The
/// delay returned is always one the windows include.
#[implements("FSpec-FR-420-AC-2")]
pub fn draw_timed_delay(
    distribution: &DelayDistribution,
    windows: &[DelayInterval],
    at: ChoicePoint,
    q: u32,
) -> Result<TimedDelayDraw, TimedDelayRefusal> {
    match distribution {
        DelayDistribution::Discrete(support) => {
            let (delays, weights): (Vec<_>, Vec<_>) = support
                .iter()
                .filter(|(delay, _)| windows.iter().any(|window| window.contains(delay)))
                .map(|(delay, weight)| (delay, weight.clone()))
                .unzip();
            draw_listed(&delays, &weights, at)
        }
        DelayDistribution::Uniform => draw_uniform(&ranges(windows, None)?, at, q),
        DelayDistribution::UniformRange { a, b } => {
            let ranges = ranges(windows, Some((a, b)))?;
            // A law over `[a, b]` with `a < b` has no mass on points alone.
            if a < b && !ranges.iter().any(DelayInterval::has_length) {
                return Err(TimedDelayRefusal::NoMass);
            }
            draw_uniform(&ranges, at, q)
        }
        DelayDistribution::Exponential { rate } => {
            let ranges = ranges(windows, None)?;
            let delay = conditioned_exponential(rate, &ranges, &grid(at, q)?, q)?;
            Ok(TimedDelayDraw::on_grid(delay, q))
        }
    }
}

/// Draw one of finitely many `delays` by exact `weights`; one delay uses no
/// choice.
fn draw_listed(
    delays: &[&Rational],
    weights: &[Rational],
    at: ChoicePoint,
) -> Result<TimedDelayDraw, TimedDelayRefusal> {
    if let [delay] = delays {
        return Ok(TimedDelayDraw::determined((*delay).clone()));
    }
    if delays.is_empty() {
        return Err(TimedDelayRefusal::NoMass);
    }
    let (selected, _) = weighted_choice_counted(at, weights);
    let index = selected.map_err(|_| TimedDelayRefusal::NoMass)?;
    let (Some(delay), Some(weight)) = (delays.get(index), weights.get(index)) else {
        return Err(TimedDelayRefusal::NoMass);
    };
    let total = weights
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |sum, weight| {
            sum.add(weight)
        });
    Ok(TimedDelayDraw {
        delay: (*delay).clone(),
        probability: weight.div(&total).ok_or(TimedDelayRefusal::NoMass)?,
        consumed_choice: true,
    })
}

/// The delays of `windows`, within `clip` where the law has a support, as
/// disjoint intervals in time order.
fn ranges(
    windows: &[DelayInterval],
    clip: Option<(&Rational, &Rational)>,
) -> Result<Vec<DelayInterval>, TimedDelayRefusal> {
    let clip = clip.map(|(a, b)| DelayInterval::closed(a.clone(), b.clone()));
    let ranges: Vec<_> = windows
        .iter()
        .map(|window| {
            clip.as_ref()
                .map_or_else(|| window.clone(), |clip| window.meet(clip))
        })
        .filter(|range| !range.is_empty())
        .collect();
    if ranges.is_empty() {
        Err(TimedDelayRefusal::NoMass)
    } else {
        Ok(DelayInterval::union(ranges))
    }
}

/// `delay`, or where it is at or past an excluded end of `range`, the delay
/// `2^-(q+1)` of the range's length inside that end (`unbounded` stands for
/// the length of an unbounded range). An included end is kept.
fn inside(delay: Rational, range: &DelayInterval, q: u32, unbounded: &Rational) -> Rational {
    let (lower, lower_closed) = range.lower();
    let upper = range.upper();
    let length = upper.map_or_else(|| unbounded.clone(), |(upper, _)| upper.sub(lower));
    let step = length.divided_by_power_of_two(u64::from(q) + 1);
    if delay <= *lower {
        return if lower_closed {
            lower.clone()
        } else {
            lower.add(&step)
        };
    }
    match upper {
        Some((upper, closed)) if delay >= *upper => {
            if closed {
                upper.clone()
            } else {
                upper.sub(&step)
            }
        }
        _ => delay,
    }
}

fn draw_uniform(
    ranges: &[DelayInterval],
    at: ChoicePoint,
    q: u32,
) -> Result<TimedDelayDraw, TimedDelayRefusal> {
    if ranges.iter().any(|range| range.upper().is_none()) {
        return Err(TimedDelayRefusal::UnboundedUniform);
    }
    if ranges.iter().any(DelayInterval::has_length) {
        let delay = conditioned_uniform(ranges, &grid(at, q)?, q)?;
        return Ok(TimedDelayDraw::on_grid(delay, q));
    }
    // A set of zero length is uniform over its points.
    let points: Vec<_> = ranges.iter().map(|range| range.lower().0).collect();
    let weights = vec![Rational::from_integer(Integer::one()); points.len()];
    draw_listed(&points, &weights, at)
}

/// The uniform law over the bounded `ranges` at the grid value `u` in
/// `[0, 1)`: uniform in length, so a point beside an interval has no mass.
#[implements("FSpec-FR-420-AC-2")]
fn conditioned_uniform(
    ranges: &[DelayInterval],
    u: &Rational,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    let long: Vec<_> = ranges
        .iter()
        .filter(|range| range.has_length())
        .filter_map(|range| {
            let (upper, _) = range.upper()?;
            Some((range, upper.sub(range.lower().0)))
        })
        .collect();
    let zero = Rational::from_integer(Integer::zero());
    let total = long
        .iter()
        .fold(zero.clone(), |sum, (_, length)| sum.add(length));
    let mut remaining = total.mul(u);
    for (range, length) in long {
        if remaining <= length {
            return Ok(inside(range.lower().0.add(&remaining), range, q, &zero));
        }
        remaining = remaining.sub(&length);
    }
    Err(TimedDelayRefusal::NoMass)
}

fn grid(at: ChoicePoint, q: u32) -> Result<Rational, TimedDelayRefusal> {
    uniform_grid_value(at, q).ok_or(TimedDelayRefusal::GridExponent { q })
}

/// The significant bits a conditioned survival value keeps, rounded down so
/// the delay rounds up. Its relative error `2^-95` is below FSpec FR-408's
/// `2^-64`.
const SURVIVAL_BITS: u64 = 96;

/// The exponential law of `rate` conditioned on the union of `ranges` (in
/// time order), at the grid value `u` in `[0, 1)`: the delay whose
/// conditional probability below it is `u` (FSpec FR-420).
///
/// Survival is measured from the first interval of positive length, so the
/// law is read the same however late the window starts. An interval from
/// `a` to `b` has mass `E(a) - E(b)` with `E(t) = e^-(rate · (t - first))`.
#[implements("FSpec-FR-420-AC-2")]
fn conditioned_exponential(
    rate: &Rational,
    ranges: &[DelayInterval],
    u: &Rational,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    // A continuous law has no mass on points.
    let ranges: Vec<_> = ranges.iter().filter(|range| range.has_length()).collect();
    let Some(first) = ranges.first().map(|range| range.lower().0) else {
        return Err(TimedDelayRefusal::NoMass);
    };
    let unbounded = Rational::from_integer(Integer::one())
        .div(rate)
        .ok_or(TimedDelayRefusal::NoMass)?;
    let survival = |at: &Rational| -> Result<Rational, TimedDelayRefusal> {
        Ok(exp_neg(&rate.mul(&at.sub(first)))?.lower)
    };
    let zero = Rational::from_integer(Integer::zero());
    let one = Rational::from_integer(Integer::one());
    let mut intervals = Vec::with_capacity(ranges.len());
    let mut total = zero.clone();
    for range in ranges {
        let from = survival(range.lower().0)?;
        let to = match range.upper() {
            Some((upper, _)) => survival(upper)?,
            None => zero.clone(),
        };
        total = total.add(&from.sub(&to));
        intervals.push((range, from, to));
    }
    // A single unbounded interval needs no exponential: its survival value
    // is exactly `1 - u`.
    let exact = intervals.len() == 1 && intervals[0].0.upper().is_none();
    let mut remaining = total.mul(u);
    let last = intervals.len() - 1;
    for (index, (range, from, to)) in intervals.into_iter().enumerate() {
        let mass = from.sub(&to);
        if remaining > mass && index < last {
            remaining = remaining.sub(&mass);
            continue;
        }
        let mut value = from.sub(&remaining);
        if value <= zero {
            // Only a bounded interval's far end is left within the precision.
            let (upper, _) = range.upper().ok_or(TimedDelayRefusal::NoMass)?;
            return Ok(inside(upper.clone(), range, q, &unbounded));
        }
        if !exact {
            value = significant_floor(&value, SURVIVAL_BITS)?;
        }
        let inverse = one.div(&value).ok_or(TimedDelayRefusal::NoMass)?;
        let elapsed = ln(&inverse)?.upper;
        let delay = first.add(&elapsed.div(rate).ok_or(TimedDelayRefusal::NoMass)?);
        return Ok(inside(delay, range, q, &unbounded));
    }
    Err(TimedDelayRefusal::NoMass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn exact(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
    }

    fn closed(lower: i64, upper: i64) -> DelayInterval {
        DelayInterval::closed(exact(lower, 1), exact(upper, 1))
    }

    /// From `lower` to `upper`, each end as its flag says.
    fn range(lower: (i64, bool), upper: (i64, bool)) -> DelayInterval {
        DelayInterval::between(
            (exact(lower.0, 1), lower.1),
            Some((exact(upper.0, 1), upper.1)),
        )
    }

    fn from(lower: i64, closed: bool) -> DelayInterval {
        DelayInterval::between((exact(lower, 1), closed), None)
    }

    /// The delay of rate 1 at `u`, checked to lie in `[low, high]`, each a
    /// count of ten-millionths or millionths over `scale`.
    fn within(ranges: &[DelayInterval], u: &Rational, low: i64, high: i64, scale: i64) {
        let delay = conditioned_exponential(&exact(1, 1), ranges, u, 64).expect("a delay");
        assert!(
            delay >= exact(low, scale) && delay <= exact(high, scale),
            "{delay:?} is outside [{low}, {high}] / {scale}"
        );
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn an_exponential_delay_is_the_inverse_of_the_law_conditioned_on_the_window() {
        // Over [0, 1] at u = 1/2: -ln((1 + e^-1) / 2) = 0.37988549304...
        let bounded = [closed(0, 1)];
        within(&bounded, &exact(1, 2), 3_798_854, 3_798_855, 10_000_000);
        // Over [0, 1] and [2, 3] at u = 15/16: the mass is
        // (1 - e^-1) + (e^-2 - e^-3) = 0.71766877369..., and the survival value
        // e^-2 - (u · mass - (1 - e^-1)) = 0.09464136672..., whose
        // -ln is 2.35766062...
        let split = [closed(0, 1), closed(2, 3)];
        within(&split, &exact(15, 16), 2_357_660, 2_357_662, 1_000_000);
        // Over [0, inf) at u = 1 - 2^-20: 20 ln 2 = 13.86294361119...
        let unbounded = [from(0, true)];
        within(
            &unbounded,
            &exact(1_048_575, 1_048_576),
            13_862_943,
            13_862_944,
            1_000_000,
        );
        // A window that starts later shifts the same law: 5 + 0.379885...
        let late = [closed(5, 6)];
        within(&late, &exact(1, 2), 53_798_854, 53_798_855, 10_000_000);
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn a_union_counts_each_instant_once_and_keeps_excluded_ones_excluded() {
        let union = DelayInterval::union;
        assert_eq!(
            union(vec![closed(4, 5), closed(0, 2), closed(0, 1), closed(2, 3)]),
            vec![closed(0, 3), closed(4, 5)]
        );
        assert_eq!(
            union(vec![from(1, true), closed(0, 2), closed(3, 4)]),
            vec![from(0, true)]
        );
        // [0, 1) and (1, 2] both exclude 1; either including it joins them.
        let below = range((0, true), (1, false));
        let above = range((1, false), (2, true));
        assert_eq!(
            union(vec![above.clone(), below.clone()]),
            vec![below.clone(), above.clone()]
        );
        assert_eq!(union(vec![closed(0, 1), above]), vec![closed(0, 2)]);
        assert_eq!(union(vec![below, closed(1, 2)]), vec![closed(0, 2)]);
        // (0, 2] and [0, 1] include 0.
        assert_eq!(
            union(vec![range((0, false), (2, true)), closed(0, 1)]),
            vec![closed(0, 2)]
        );
    }

    /// The one sampler choice at `q = 0` takes `u = 0`.
    fn draw_at_zero(
        distribution: &DelayDistribution,
        window: DelayInterval,
    ) -> Result<Rational, TimedDelayRefusal> {
        let at = ChoicePoint {
            seed: 7,
            trace: 0,
            step: 0,
            choice: 0,
        };
        draw_timed_delay(distribution, &[window], at, 0).map(|draw| draw.delay)
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn a_law_with_no_mass_in_the_window_draws_nothing() {
        let two_to_six = DelayDistribution::UniformRange {
            a: exact(2, 1),
            b: exact(6, 1),
        };
        // [0, 2) excludes 2; [0, 2] meets [2, 6] in one point of no mass.
        for window in [range((0, true), (2, false)), closed(0, 2)] {
            assert_eq!(
                draw_at_zero(&two_to_six, window),
                Err(TimedDelayRefusal::NoMass)
            );
        }
        let at_two = DelayDistribution::UniformRange {
            a: exact(2, 1),
            b: exact(2, 1),
        };
        assert_eq!(draw_at_zero(&at_two, closed(0, 2)), Ok(exact(2, 1)));
        assert_eq!(
            draw_at_zero(&at_two, range((0, true), (2, false))),
            Err(TimedDelayRefusal::NoMass)
        );
        // A window that is one point is uniform over that point.
        assert_eq!(
            draw_at_zero(&DelayDistribution::Uniform, closed(2, 2)),
            Ok(exact(2, 1))
        );
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn a_draw_reports_its_probability_and_whether_it_used_its_choice() {
        let at = ChoicePoint {
            seed: 7,
            trace: 0,
            step: 0,
            choice: 0,
        };
        let draw = |distribution: &DelayDistribution, windows: &[DelayInterval], q| {
            let drawn = draw_timed_delay(distribution, windows, at, q).expect("a draw");
            (drawn.probability, drawn.consumed_choice)
        };
        let two_points = [closed(1, 1), closed(3, 3)];
        let three_points = [closed(1, 1), closed(3, 3), closed(5, 5)];
        let at_two = DelayDistribution::UniformRange {
            a: exact(2, 1),
            b: exact(2, 1),
        };
        let rate = DelayDistribution::Exponential { rate: exact(1, 1) };
        for q in [0, 8] {
            // Uniform over finitely many points, at any grid.
            assert_eq!(
                draw(&DelayDistribution::Uniform, &two_points, q),
                (exact(1, 2), true)
            );
            assert_eq!(
                draw(&DelayDistribution::Uniform, &three_points, q),
                (exact(1, 3), true)
            );
            // One delay is determined, however long the window.
            assert_eq!(draw(&at_two, &[closed(0, 4)], q), (exact(1, 1), false));
            assert_eq!(
                draw(&DelayDistribution::Uniform, &[closed(2, 2)], q),
                (exact(1, 1), false)
            );
        }
        for law in [DelayDistribution::Uniform, rate] {
            assert_eq!(draw(&law, &[closed(0, 4)], 0), (exact(1, 1), false));
            assert_eq!(draw(&law, &[closed(0, 4)], 8), (exact(1, 256), true));
        }
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[trace("FSpec-FR-420-AC-2", "FSpec-FR-420-AC-9")]
    fn an_excluded_end_is_never_drawn() {
        // (2, 4] at u = 0, q = 0: half the length inside the open end.
        let open = range((2, false), (4, true));
        assert_eq!(
            draw_at_zero(&DelayDistribution::Uniform, open.clone()),
            Ok(exact(3, 1))
        );
        let rate = DelayDistribution::Exponential { rate: exact(1, 1) };
        assert_eq!(draw_at_zero(&rate, open), Ok(exact(3, 1)));
        // (2, inf) at rate 2: half of the mean 1/2 inside the open end.
        let rate = DelayDistribution::Exponential { rate: exact(2, 1) };
        assert_eq!(draw_at_zero(&rate, from(2, false)), Ok(exact(9, 4)));
        // An included lower end is drawn.
        assert_eq!(
            draw_at_zero(&DelayDistribution::Uniform, closed(2, 4)),
            Ok(exact(2, 1))
        );
        // [0, 1) and [2, 3] at u = 1/2 is the boundary at length 1: the open
        // end 1 is not drawn, but 1 - 1/4 at q = 1.
        let split = [range((0, true), (1, false)), closed(2, 3)];
        assert_eq!(
            conditioned_uniform(&split, &exact(1, 2), 1),
            Ok(exact(3, 4))
        );
        // The same boundary at an included end is drawn.
        let split = [closed(0, 1), closed(2, 3)];
        assert_eq!(
            conditioned_uniform(&split, &exact(1, 2), 1),
            Ok(exact(1, 1))
        );
        // A point beside an interval has no mass.
        let beside = [closed(0, 0), closed(2, 3)];
        assert_eq!(
            conditioned_uniform(&beside, &exact(0, 1), 1),
            Ok(exact(2, 1))
        );
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[trace("FSpec-FR-420-AC-9")]
    #[test]
    fn a_quantile_on_an_excluded_end_moves_in_by_two_to_the_minus_q_plus_one() {
        let at = ChoicePoint {
            seed: 7,
            trace: 0,
            step: 0,
            choice: 0,
        };
        // (2, 4] at u = 0, q = 2: the raw delay 2 is excluded, so the delay
        // is 2 + (4 - 2) / 2^(2+1) = 2 + 2/8 = 9/4.
        let open = range((2, false), (4, true));
        let drawn = conditioned_uniform(&[open], &exact(0, 1), 2).expect("a draw");
        assert_eq!(drawn, exact(9, 4));
        // (2, inf) at rate 2, u = 0, q = 0: 2 + (1/2) / 2^(0+1) = 2 + 1/4 = 9/4.
        let rate = DelayDistribution::Exponential { rate: exact(2, 1) };
        let drawn = draw_timed_delay(&rate, &[from(2, false)], at, 0)
            .expect("a draw")
            .delay;
        assert_eq!(drawn, exact(9, 4));
    }

    #[test]
    fn an_exponential_law_has_no_mass_on_points() {
        assert_eq!(
            conditioned_exponential(&exact(1, 1), &[closed(1, 1)], &exact(1, 2), 64),
            Err(TimedDelayRefusal::NoMass)
        );
    }
}
