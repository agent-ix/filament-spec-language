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

/// Draw one exact delay from `distribution` conditioned on the union of
/// `windows` (in time order), using `at` as its one sampler choice
/// (FR-253/FR-254).
pub fn draw_timed_delay(
    distribution: &DelayDistribution,
    windows: &[DelayInterval],
    at: ChoicePoint,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    match distribution {
        DelayDistribution::Discrete(support) => {
            let (delays, weights): (Vec<_>, Vec<_>) = support
                .iter()
                .filter(|(delay, _)| windows.iter().any(|window| window.contains(delay)))
                .map(|(delay, weight)| (delay.clone(), weight.clone()))
                .unzip();
            if delays.is_empty() {
                return Err(TimedDelayRefusal::NoMass);
            }
            if delays.len() == 1 {
                return Ok(delays[0].clone());
            }
            let (selected, _) = weighted_choice_counted(at, &weights);
            let index = selected.map_err(|_| TimedDelayRefusal::NoMass)?;
            delays.get(index).cloned().ok_or(TimedDelayRefusal::NoMass)
        }
        DelayDistribution::Uniform => {
            let ranges = ranges(windows, None)?;
            draw_uniform(&ranges, at, q)
        }
        DelayDistribution::UniformRange { a, b } => {
            let ranges = ranges(windows, Some((a, b)))?;
            draw_uniform(&ranges, at, q)
        }
        DelayDistribution::Exponential { rate } => {
            let ranges = ranges(windows, None)?;
            conditioned_exponential(rate, &ranges, &grid(at, q)?)
        }
    }
}

/// A set of clipped intervals represented by their finite ends.
fn ranges(
    windows: &[DelayInterval],
    clip: Option<(&Rational, &Rational)>,
) -> Result<Vec<(Rational, Option<Rational>)>, TimedDelayRefusal> {
    let mut ranges = Vec::new();
    for window in windows {
        let (mut lower, _) = window.lower();
        let mut upper = window.upper().map(|(upper, _)| upper);
        if let Some((a, b)) = clip {
            if lower < a {
                lower = a;
            }
            if upper.is_none_or(|upper| upper > b) {
                upper = Some(b);
            }
        }
        if upper.is_some_and(|upper| upper < lower) {
            continue;
        }
        ranges.push((lower.clone(), upper.cloned()));
    }
    if ranges.is_empty() {
        Err(TimedDelayRefusal::NoMass)
    } else {
        Ok(union(ranges))
    }
}

/// The union of `ranges` in time order: the parts of a disjunctive guard
/// may overlap, and a law is conditioned on each instant once.
fn union(mut ranges: Vec<(Rational, Option<Rational>)>) -> Vec<(Rational, Option<Rational>)> {
    ranges.sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut merged: Vec<(Rational, Option<Rational>)> = Vec::with_capacity(ranges.len());
    for (lower, upper) in ranges {
        match merged.last_mut() {
            Some((_, end)) if end.as_ref().is_none_or(|end| lower <= *end) => {
                if end.is_some()
                    && upper
                        .as_ref()
                        .is_none_or(|upper| Some(upper) > end.as_ref())
                {
                    *end = upper;
                }
            }
            _ => merged.push((lower, upper)),
        }
    }
    merged
}

fn draw_uniform(
    ranges: &[(Rational, Option<Rational>)],
    at: ChoicePoint,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    if ranges.iter().any(|(_, upper)| upper.is_none()) {
        return Err(TimedDelayRefusal::UnboundedUniform);
    }
    let total = ranges.iter().fold(
        Rational::from_integer(Integer::zero()),
        |sum, (lower, upper)| sum.add(&upper.as_ref().expect("finite range").sub(lower)),
    );
    if total.is_zero() {
        if ranges.len() == 1 {
            return Ok(ranges[0].0.clone());
        }
        let weights = vec![Rational::from_integer(Integer::one()); ranges.len()];
        let (selected, _) = weighted_choice_counted(at, &weights);
        return selected
            .map(|index| ranges[index].0.clone())
            .map_err(|_| TimedDelayRefusal::NoMass);
    }
    let u = grid(at, q)?;
    let mut remaining = total.mul(&u);
    for (lower, upper) in ranges {
        let length = upper.as_ref().expect("finite range").sub(lower);
        if remaining <= length {
            return Ok(lower.add(&remaining));
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
/// law is read the same however late the window starts. An interval
/// `[a, b]` has mass `E(a) - E(b)` with `E(t) = e^-(rate · (t - first))`.
#[implements("FSpec-FR-420-AC-2")]
fn conditioned_exponential(
    rate: &Rational,
    ranges: &[(Rational, Option<Rational>)],
    u: &Rational,
) -> Result<Rational, TimedDelayRefusal> {
    // A continuous law has no mass on points.
    let ranges: Vec<_> = ranges
        .iter()
        .filter(|(lower, upper)| upper.as_ref() != Some(lower))
        .collect();
    let Some((first, _)) = ranges.first().copied() else {
        return Err(TimedDelayRefusal::NoMass);
    };
    let survival = |at: &Rational| -> Result<Rational, TimedDelayRefusal> {
        Ok(exp_neg(&rate.mul(&at.sub(first)))?.lower)
    };
    let zero = Rational::from_integer(Integer::zero());
    let one = Rational::from_integer(Integer::one());
    let mut intervals = Vec::with_capacity(ranges.len());
    let mut total = zero.clone();
    for (lower, upper) in ranges.iter().copied() {
        let from = survival(lower)?;
        let to = match upper {
            Some(upper) => survival(upper)?,
            None => zero.clone(),
        };
        total = total.add(&from.sub(&to));
        intervals.push((lower, upper.as_ref(), from, to));
    }
    // A single unbounded interval needs no exponential: its survival value
    // is exactly `1 - u`.
    let exact = intervals.len() == 1 && intervals[0].1.is_none();
    let mut remaining = total.mul(u);
    let last = intervals.len() - 1;
    for (index, (lower, upper, from, to)) in intervals.into_iter().enumerate() {
        let mass = from.sub(&to);
        if remaining > mass && index < last {
            remaining = remaining.sub(&mass);
            continue;
        }
        let mut value = from.sub(&remaining);
        if value <= zero {
            // Only a bounded interval's far end is left within the precision.
            return upper.cloned().ok_or(TimedDelayRefusal::NoMass);
        }
        if !exact {
            value = significant_floor(&value, SURVIVAL_BITS)?;
        }
        let inverse = one.div(&value).ok_or(TimedDelayRefusal::NoMass)?;
        let elapsed = ln(&inverse)?.upper;
        let delay = first.add(&elapsed.div(rate).ok_or(TimedDelayRefusal::NoMass)?);
        return Ok(match upper {
            _ if delay < *lower => lower.clone(),
            Some(upper) if delay > *upper => upper.clone(),
            _ => delay,
        });
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

    /// The delay of rate 1 at `u`, checked to lie in `[low, high]`, each a
    /// count of ten-millionths or millionths over `scale`.
    fn within(
        ranges: &[(Rational, Option<Rational>)],
        u: &Rational,
        low: i64,
        high: i64,
        scale: i64,
    ) {
        let delay = conditioned_exponential(&exact(1, 1), ranges, u).expect("a delay");
        assert!(
            delay >= exact(low, scale) && delay <= exact(high, scale),
            "{delay:?} is outside [{low}, {high}] / {scale}"
        );
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn an_exponential_delay_is_the_inverse_of_the_law_conditioned_on_the_window() {
        // Over [0, 1] at u = 1/2: -ln((1 + e^-1) / 2) = 0.37988549304...
        let bounded = [(exact(0, 1), Some(exact(1, 1)))];
        within(&bounded, &exact(1, 2), 3_798_854, 3_798_855, 10_000_000);
        // Over [0, 1] and [2, 3] at u = 15/16: the mass is
        // (1 - e^-1) + (e^-2 - e^-3) = 0.71766877369..., and the survival value
        // e^-2 - (u · mass - (1 - e^-1)) = 0.09464136672..., whose
        // -ln is 2.35766062...
        let split = [
            (exact(0, 1), Some(exact(1, 1))),
            (exact(2, 1), Some(exact(3, 1))),
        ];
        within(&split, &exact(15, 16), 2_357_660, 2_357_662, 1_000_000);
        // Over [0, inf) at u = 1 - 2^-20: 20 ln 2 = 13.86294361119...
        let unbounded = [(exact(0, 1), None)];
        within(
            &unbounded,
            &exact(1_048_575, 1_048_576),
            13_862_943,
            13_862_944,
            1_000_000,
        );
        // A window that starts later shifts the same law: 5 + 0.379885...
        let late = [(exact(5, 1), Some(exact(6, 1)))];
        within(&late, &exact(1, 2), 53_798_854, 53_798_855, 10_000_000);
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn a_window_of_overlapping_parts_counts_each_instant_once() {
        let range = |lower: i64, upper: i64| (exact(lower, 1), Some(exact(upper, 1)));
        assert_eq!(
            union(vec![range(4, 5), range(0, 2), range(0, 1), range(2, 3)]),
            vec![range(0, 3), range(4, 5)]
        );
        assert_eq!(
            union(vec![(exact(1, 1), None), range(0, 2), range(3, 4)]),
            vec![(exact(0, 1), None)]
        );
    }

    #[trace("FSpec-FR-420-AC-2")]
    #[test]
    fn an_exponential_law_has_no_mass_on_points() {
        let points = [(exact(1, 1), Some(exact(1, 1)))];
        assert_eq!(
            conditioned_exponential(&exact(1, 1), &points, &exact(1, 2)),
            Err(TimedDelayRefusal::NoMass)
        );
    }
}
