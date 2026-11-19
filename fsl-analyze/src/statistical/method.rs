// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-408's fixed-sample (Okamoto) and sequential (SPRT) methods over
//! one test's `α'`, `β'` and the claim's `ι` (ADR-024 ST-4, ST-5, ST-9), in
//! exact rationals with every rounding outward, toward undecided.

use filament_exact::{Charge, ChargePoint, Integer, Meter, Rational};
use filament_semantic_value::catalog::InternalFault;
use fsl_replay::statistical::{Decision, UndecidedCause};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive, Zero};

use super::bounds::{ceil, int, ln, quotient, Bounds};
use super::claim::{Bound, Comparison};

const BINOMIAL_SCALE: InternalFault = InternalFault::new("statistical-binomial", "positive-scale");
const BINOMIAL_DENOMINATOR: InternalFault =
    InternalFault::new("statistical-binomial", "positive-denominator");
const BINOMIAL_MODE: InternalFault = InternalFault::new("statistical-binomial", "mode-in-range");
const BINOMIAL_METER: InternalFault =
    InternalFault::new("statistical-binomial", "clause-meter-exhausted");

/// Decide `bound` over the interval `[estimate − half_width, estimate +
/// half_width]` (ADR-024 ST-4): a `>= θ` bound accepts when the interval
/// lies at or above `θ` and rejects when it lies below; a `<= θ` bound is
/// symmetric; the strict `< θ` bound accepts only when the interval lies
/// strictly below `θ` and rejects as `<= θ` does. Anything else is
/// undecided.
pub(crate) fn interval_decision(
    bound: &Bound,
    estimate: &Rational,
    half_width: &Rational,
) -> (Decision, (Rational, Rational)) {
    let lower = estimate.sub(half_width);
    let upper = estimate.add(half_width);
    let theta = &bound.threshold;
    let (accepted, rejected) = match bound.comparison {
        Comparison::AtLeast => (lower >= *theta, upper < *theta),
        Comparison::AtMost => (upper <= *theta, lower > *theta),
        Comparison::Below => (upper < *theta, lower > *theta),
    };
    let decision = if accepted {
        Decision::Accepted
    } else if rejected {
        Decision::Rejected
    } else {
        Decision::Undecided(UndecidedCause::IndifferenceRegion)
    };
    (decision, (lower, upper))
}

/// The lesser of `α'` and `β'`.
pub(crate) fn least<'r>(alpha: &'r Rational, beta: &'r Rational) -> &'r Rational {
    if alpha <= beta {
        alpha
    } else {
        beta
    }
}

/// Okamoto's sample count `N = ⌈ln(1 / min(α', β')) / (2 ι²)⌉`, with the
/// upper bound of the logarithm, rounded up.
pub(crate) fn okamoto_sample_count(
    alpha: &Rational,
    beta: &Rational,
    iota: &Rational,
) -> Result<Integer, InternalFault> {
    let log = ln(&quotient(&int(1), least(alpha, beta))?)?.upper;
    Ok(ceil(&quotient(&log, &int(2).mul(&iota.mul(iota)))?))
}

/// The Clopper–Pearson method uses Okamoto's fixed sample count (ADR-024
/// ST-11).
pub(crate) fn clopper_pearson_sample_count(
    alpha: &Rational,
    beta: &Rational,
    iota: &Rational,
) -> Result<Integer, InternalFault> {
    okamoto_sample_count(alpha, beta, iota)
}

/// A fixed-point interval `[lower, upper] / scale` for a nonnegative value.
#[derive(Clone, Debug)]
struct BinaryInterval {
    lower: BigInt,
    upper: BigInt,
    scale: BigInt,
}

impl BinaryInterval {
    fn zero(scale: &BigInt) -> Self {
        Self {
            lower: BigInt::zero(),
            upper: BigInt::zero(),
            scale: scale.clone(),
        }
    }

    fn one(scale: &BigInt) -> Self {
        Self {
            lower: scale.clone(),
            upper: scale.clone(),
            scale: scale.clone(),
        }
    }

    fn add_assign(&mut self, other: &Self) {
        self.lower += &other.lower;
        self.upper += &other.upper;
    }

    fn multiplied_by_ratio(
        &self,
        numerator: &BigInt,
        denominator: &BigInt,
    ) -> Result<Self, InternalFault> {
        if denominator <= &BigInt::zero() || numerator < &BigInt::zero() {
            return Err(BINOMIAL_DENOMINATOR);
        }
        Ok(Self {
            lower: (&self.lower * numerator).div_floor(denominator),
            upper: (&self.upper * numerator).div_ceil(denominator),
            scale: self.scale.clone(),
        })
    }
}

/// Which inclusive binomial tail is being bounded.
#[derive(Clone, Copy)]
enum Tail {
    /// `Pr(X >= k)`.
    Upper,
    /// `Pr(X <= k)`.
    Lower,
}

/// The precision specified by ADR-024 ST-9: 96 guard bits plus the ceiling
/// of `log2(n + 1)`.
fn binomial_precision(n: u64) -> u64 {
    96 + u64::from(u64::BITS - n.leading_zeros())
}

/// One outward-rounded binomial tail. The fixed-point weights are scaled
/// relative to the largest term in the requested tail, which preserves
/// relative precision even for a small tail.
fn binomial_tail(
    k: u64,
    n: u64,
    p: &Rational,
    tail_kind: Tail,
    meter: &mut Meter,
) -> Result<BinaryInterval, InternalFault> {
    if k > n {
        return Err(BINOMIAL_MODE);
    }
    meter
        .charge(Charge::new(ChargePoint::CollectionVisit).work(Integer::from(n.saturating_add(1))))
        .map_err(|_| BINOMIAL_METER)?;
    let scale = BigInt::one() << binomial_precision(n);
    let zero = int(0);
    let one = int(1);
    if *p < zero || *p > one {
        return Err(BINOMIAL_DENOMINATOR);
    }
    if *p == zero {
        let probability = match tail_kind {
            Tail::Upper => u64::from(k == 0),
            Tail::Lower => 1,
        };
        return Ok(if probability == 1 {
            BinaryInterval::one(&scale)
        } else {
            BinaryInterval::zero(&scale)
        });
    }
    if *p == one {
        let probability = match tail_kind {
            Tail::Upper => 1,
            Tail::Lower => u64::from(k == n),
        };
        return Ok(if probability == 1 {
            BinaryInterval::one(&scale)
        } else {
            BinaryInterval::zero(&scale)
        });
    }

    let probability_numerator = p.numerator().as_big();
    let probability_denominator = p.denominator().as_big();
    let failure_numerator = probability_denominator - probability_numerator;
    let mode_product = (BigInt::from(n) + BigInt::one()) * probability_numerator;
    let mode = mode_product
        .div_floor(probability_denominator)
        .to_u64()
        .ok_or(BINOMIAL_MODE)?
        .min(n);
    let start = match tail_kind {
        Tail::Upper => k.max(mode),
        Tail::Lower => k.min(mode),
    };
    let in_tail = |index: u64| match tail_kind {
        Tail::Upper => index >= k,
        Tail::Lower => index <= k,
    };

    let mut total = BinaryInterval::zero(&scale);
    let mut selected = BinaryInterval::zero(&scale);
    let mut add = |index: u64, weight: &BinaryInterval| {
        total.add_assign(weight);
        if in_tail(index) {
            selected.add_assign(weight);
        }
    };

    let start_weight = BinaryInterval::one(&scale);
    add(start, &start_weight);

    let mut weight = start_weight;
    for index in start..n {
        let numerator = BigInt::from(n - index) * probability_numerator;
        let denominator = BigInt::from(index + 1) * &failure_numerator;
        weight = weight.multiplied_by_ratio(&numerator, &denominator)?;
        add(index + 1, &weight);
    }

    weight = BinaryInterval::one(&scale);
    for index in (1..=start).rev() {
        let numerator = BigInt::from(index) * &failure_numerator;
        let denominator = BigInt::from(n - index + 1) * probability_numerator;
        weight = weight.multiplied_by_ratio(&numerator, &denominator)?;
        add(index - 1, &weight);
    }
    drop(add);

    if total.lower <= BigInt::zero() {
        return Err(BINOMIAL_SCALE);
    }
    let lower = (&selected.lower * &scale).div_floor(&total.upper);
    let upper = (&selected.upper * &scale)
        .div_ceil(&total.lower)
        .min(scale.clone());
    Ok(BinaryInterval {
        lower,
        upper,
        scale,
    })
}

impl BinaryInterval {
    fn upper_le(&self, threshold: &Rational, strict: bool) -> bool {
        let left = &self.upper * threshold.denominator().as_big();
        let right = threshold.numerator().as_big() * &self.scale;
        if strict {
            left < right
        } else {
            left <= right
        }
    }
}

/// The exact Clopper–Pearson decision for one binomial count. Only the
/// outward upper tail bounds participate in the decision (ADR-024 ST-9,
/// ST-11).
pub(crate) fn clopper_pearson_decision(
    bound: &Bound,
    held: u64,
    samples: u64,
    alpha: &Rational,
    beta: &Rational,
    meter: &mut Meter,
) -> Result<Decision, InternalFault> {
    let lower = binomial_tail(held, samples, &bound.threshold, Tail::Lower, meter)?;
    let upper = binomial_tail(held, samples, &bound.threshold, Tail::Upper, meter)?;
    let (accepted, rejected) = match bound.comparison {
        Comparison::AtLeast => (upper.upper_le(beta, false), lower.upper_le(alpha, false)),
        Comparison::AtMost => (lower.upper_le(beta, false), upper.upper_le(alpha, false)),
        Comparison::Below => (lower.upper_le(beta, true), upper.upper_le(alpha, false)),
    };
    Ok(if accepted && rejected {
        Decision::Undecided(UndecidedCause::IndifferenceRegion)
    } else if accepted {
        Decision::Accepted
    } else if rejected {
        Decision::Rejected
    } else {
        Decision::Undecided(UndecidedCause::IndifferenceRegion)
    })
}

/// The outward Clopper–Pearson interval found with 64 bisection steps
/// (ADR-024 ST-11). The lower endpoint uses `β`; the upper endpoint uses
/// `α`.
pub(crate) fn clopper_pearson_interval(
    held: u64,
    samples: u64,
    alpha: &Rational,
    beta: &Rational,
    meter: &mut Meter,
) -> Result<(Rational, Rational), InternalFault> {
    if samples == 0 || held > samples {
        return Err(BINOMIAL_MODE);
    }
    let mut lower = int(0);
    let mut upper = int(1);
    if held > 0 {
        for _ in 0..64 {
            let middle = lower.add(&upper).divided_by_power_of_two(1);
            let tail = binomial_tail(held, samples, &middle, Tail::Upper, meter)?;
            if tail.upper_le(beta, false) {
                lower = middle;
            } else {
                upper = middle;
            }
        }
    }
    let lower_endpoint = lower;

    lower = int(0);
    upper = int(1);
    if held < samples {
        for _ in 0..64 {
            let middle = lower.add(&upper).divided_by_power_of_two(1);
            let tail = binomial_tail(held, samples, &middle, Tail::Lower, meter)?;
            if tail.upper_le(alpha, false) {
                upper = middle;
            } else {
                lower = middle;
            }
        }
    }
    Ok((lower_endpoint, upper))
}

/// SPRT's thresholds, each rounded away from zero: the rejection threshold
/// the upper bound of `ln(1/α')`, the acceptance threshold the lower bound
/// of `ln(β')`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SprtThresholds {
    pub(crate) reject: Rational,
    pub(crate) accept: Rational,
}

pub(crate) fn sprt_thresholds(
    alpha: &Rational,
    beta: &Rational,
) -> Result<SprtThresholds, InternalFault> {
    Ok(SprtThresholds {
        reject: ln(&quotient(&int(1), alpha)?)?.upper,
        accept: ln(beta)?.lower,
    })
}

/// One test's sequential probability ratio test (ADR-024 ST-5): the bounds
/// on each sample's increment and the counts of holding and failing
/// samples, from which the lower and upper sums follow exactly.
#[derive(Clone, Debug)]
pub(crate) struct Sprt {
    holds: Bounds,
    fails: Bounds,
    thresholds: SprtThresholds,
    held: u64,
    failed: u64,
}

impl Sprt {
    /// The test of `bound` at `ι` with per-test `α'` and `β'`. For `>= θ`
    /// it tests `H0: p >= θ + ι` against `H1: p <= θ − ι`; a `<= θ` or
    /// `< θ` bound swaps the hypotheses. A holding sample adds
    /// `ln(p1 / p0)`, a failing one `ln((1 − p1) / (1 − p0))`, `p1` the
    /// hypothesis whose acceptance rejects the bound.
    pub(crate) fn new(
        bound: &Bound,
        iota: &Rational,
        alpha: &Rational,
        beta: &Rational,
    ) -> Result<Self, InternalFault> {
        let above = bound.threshold.add(iota);
        let below = bound.threshold.sub(iota);
        let (p0, p1) = match bound.comparison {
            Comparison::AtLeast => (above, below),
            Comparison::AtMost | Comparison::Below => (below, above),
        };
        let holds = ln(&quotient(&p1, &p0)?)?;
        let fails = ln(&quotient(&int(1).sub(&p1), &int(1).sub(&p0))?)?;
        Ok(Self {
            holds,
            fails,
            thresholds: sprt_thresholds(alpha, beta)?,
            held: 0,
            failed: 0,
        })
    }

    /// Count one sample, by whether its event held.
    pub(crate) fn count(&mut self, held: bool) {
        if held {
            self.held += 1;
        } else {
            self.failed += 1;
        }
    }

    /// The decision the sums reach, if any: rejected when the lower sum
    /// reaches the rejection threshold, accepted when the upper sum reaches
    /// the acceptance threshold.
    pub(crate) fn decision(&self) -> Option<Decision> {
        let held = Rational::from_integer(Integer::from(self.held));
        let failed = Rational::from_integer(Integer::from(self.failed));
        let lower = held
            .mul(&self.holds.lower)
            .add(&failed.mul(&self.fails.lower));
        if lower >= self.thresholds.reject {
            return Some(Decision::Rejected);
        }
        let upper = held
            .mul(&self.holds.upper)
            .add(&failed.mul(&self.fails.upper));
        (upper <= self.thresholds.accept).then_some(Decision::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use filament_exact::{LimitKind, ScalarLimits};
    use ix_trace_rs::trace;

    use super::*;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        quotient(&int(numerator), &int(denominator)).expect("nonzero denominator")
    }

    fn decimal(digits: &str, scale: u64) -> Rational {
        let value: Integer = digits.parse().expect("decimal digits");
        Rational::from_integer(value).divided_by_power_of_ten(scale)
    }

    fn meter(work_units: u64) -> Meter {
        Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units,
            result_units: u64::MAX,
        })
    }

    /// FR-189-AC-16: the exact binomial tails decide strictly more counts
    /// than Okamoto at the same sample count, and remain undecided in the
    /// gap between their error thresholds.
    #[trace("TC-624", "FR-189-AC-16")]
    #[test]
    fn clopper_pearson_decisions_match_the_fixed_sample_vector() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(9, 10),
        };
        let alpha = ratio(1, 100);
        let beta = ratio(1, 100);
        let cases = [
            (231, Decision::Accepted),
            (219, Decision::Accepted),
            (218, Decision::Undecided(UndecidedCause::IndifferenceRegion)),
            (197, Decision::Undecided(UndecidedCause::IndifferenceRegion)),
            (196, Decision::Rejected),
            (184, Decision::Rejected),
        ];
        let mut meter = meter(u64::MAX);
        for (held, expected) in cases {
            assert_eq!(
                clopper_pearson_decision(&bound, held, 231, &alpha, &beta, &mut meter)
                    .expect("binomial tails"),
                expected,
                "held count {held}"
            );
        }
    }

    /// ADR-024 ST-11: both true tails leave the result undecided.
    #[test]
    fn clopper_pearson_both_tails_true_is_undecided() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(1, 2),
        };
        let alpha = ratio(9, 10);
        let beta = ratio(9, 10);
        let mut meter = meter(u64::MAX);
        assert_eq!(
            clopper_pearson_decision(&bound, 3, 6, &alpha, &beta, &mut meter)
                .expect("binomial tails"),
            Decision::Undecided(UndecidedCause::IndifferenceRegion)
        );
    }

    /// FR-189 charges N+1 work units for each tail, including every
    /// repeated tail evaluation in the interval bisection.
    #[test]
    fn clopper_pearson_charges_each_tail_term() {
        let alpha = ratio(1, 100);
        let beta = ratio(1, 100);
        let mut meter = meter(30_160);
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(9, 10),
        };
        clopper_pearson_decision(&bound, 219, 231, &alpha, &beta, &mut meter)
            .expect("two N+1 tails");
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 464);
        clopper_pearson_interval(219, 231, &alpha, &beta, &mut meter).expect("128 bisection tails");
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 30_160);
    }

    /// FR-189-AC-16: the outward 64-step interval brackets the published
    /// Clopper–Pearson endpoints at `k = 219`, `N = 231`.
    #[trace("TC-624", "FR-189-AC-16")]
    #[test]
    fn clopper_pearson_known_intervals() {
        let alpha = ratio(1, 100);
        let beta = ratio(1, 100);
        let mut meter = meter(u64::MAX);
        let (lower, upper) =
            clopper_pearson_interval(219, 231, &alpha, &beta, &mut meter).expect("interval");
        let published_lower = decimal("90351472792271070929", 20);
        let published_upper = decimal("97621175445884490995", 20);
        let printed_unit = int(1).divided_by_power_of_ten(20);
        let precision = int(1).divided_by_power_of_two(64);

        assert!(lower <= published_lower.add(&printed_unit));
        assert!(
            published_lower.sub(&lower) < precision.add(&printed_unit),
            "lower endpoint is too far below the published value: {lower}"
        );
        assert!(upper >= published_upper);
        assert!(
            upper.sub(&published_upper) < precision.add(&printed_unit),
            "upper endpoint is too far above the published value: {upper}"
        );
    }

    /// `N` is 23,026 at `α' = β' = ι = 1/100`, 26,492 at `α' = 1/200` (two
    /// tests) and 9,210,341 at `ι = 1/2000` (ADR-024 §7.1's `NoFault`).
    #[trace("TC-624", "FR-189-AC-2")]
    #[test]
    fn okamoto_sample_counts_match_the_spec_vectors() {
        let hundredth = ratio(1, 100);
        let n = |alpha: &Rational, iota: &Rational| {
            okamoto_sample_count(alpha, alpha, iota).expect("sample count")
        };
        assert_eq!(n(&hundredth, &hundredth), Integer::from(23_026_u64));
        assert_eq!(n(&ratio(1, 200), &hundredth), Integer::from(26_492_u64));
        assert_eq!(n(&hundredth, &ratio(1, 2000)), Integer::from(9_210_341_u64));
    }

    /// The thresholds bound `±ln 100` outward, each within `2^-64` of it.
    #[trace("TC-624", "FR-189-AC-2")]
    #[test]
    fn sprt_thresholds_bound_ln_100_outward_within_two_to_the_minus_64() {
        let thresholds = sprt_thresholds(&ratio(1, 100), &ratio(1, 100)).expect("thresholds");
        // ln 100 = 4.605170185988091368035982909368728415202202977257…
        let digits: Integer = "4605170185988091368035982909368728415202"
            .parse()
            .expect("digits");
        let ln_lower = Rational::from_integer(digits).divided_by_power_of_ten(39);
        let ln_upper = ln_lower.add(&int(1).divided_by_power_of_ten(39));
        let precision = int(1).divided_by_power_of_two(64);
        assert!(thresholds.reject >= ln_upper);
        assert!(thresholds.reject.sub(&ln_lower) < precision);
        assert!(thresholds.accept <= ln_lower.neg());
        assert!(ln_upper.neg().sub(&thresholds.accept) < precision);
    }

    #[test]
    fn the_interval_rule_follows_each_comparison() {
        let half = ratio(1, 2);
        let tenth = ratio(1, 10);
        let bound = |comparison| Bound {
            comparison,
            threshold: half.clone(),
        };
        let decide = |comparison, estimate: Rational| {
            interval_decision(&bound(comparison), &estimate, &tenth).0
        };
        let undecided = Decision::Undecided(UndecidedCause::IndifferenceRegion);
        assert_eq!(
            decide(Comparison::AtLeast, ratio(6, 10)),
            Decision::Accepted
        );
        assert_eq!(
            decide(Comparison::AtLeast, ratio(39, 100)),
            Decision::Rejected
        );
        assert_eq!(decide(Comparison::AtLeast, ratio(55, 100)), undecided);
        assert_eq!(decide(Comparison::AtMost, ratio(4, 10)), Decision::Accepted);
        assert_eq!(
            decide(Comparison::AtMost, ratio(61, 100)),
            Decision::Rejected
        );
        // The strict bound never accepts an interval touching θ.
        assert_eq!(decide(Comparison::Below, ratio(4, 10)), undecided);
        assert_eq!(
            decide(Comparison::Below, ratio(39, 100)),
            Decision::Accepted
        );
        assert_eq!(
            decide(Comparison::Below, ratio(61, 100)),
            Decision::Rejected
        );
    }

    #[test]
    fn sprt_decides_by_the_two_sums() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(95, 100),
        };
        let hundredth = ratio(1, 100);
        let mut test = Sprt::new(&bound, &hundredth, &hundredth, &hundredth).expect("sprt");
        // ADR-024 §7.2: with no failing sample SPRT accepts after 219.
        for _ in 0..218 {
            test.count(true);
            assert_eq!(test.decision(), None);
        }
        test.count(true);
        assert_eq!(test.decision(), Some(Decision::Accepted));
        let mut failing = Sprt::new(&bound, &hundredth, &hundredth, &hundredth).expect("sprt");
        // A failing sample adds ln 1.5 ≈ 0.4055; ln 100 / ln 1.5 ≈ 11.36.
        for _ in 0..11 {
            failing.count(false);
            assert_eq!(failing.decision(), None);
        }
        failing.count(false);
        assert_eq!(failing.decision(), Some(Decision::Rejected));
    }

    /// FSpec FR-408-AC-7: a sum within the rounding error of a threshold
    /// gives no decision. With `p0 = 2/5`, `p1 = 1/5` a holding sample adds
    /// exactly `ln(1/2)`, so four of them reach `ln(β') = ln(1/16)` exactly;
    /// the outward sums do not, and the test decides only at the fifth.
    #[test]
    fn a_sum_on_the_threshold_within_rounding_is_undecided() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(3, 10),
        };
        let sixteenth = ratio(1, 16);
        let mut test = Sprt::new(&bound, &ratio(1, 10), &sixteenth, &sixteenth).expect("sprt");
        for _ in 0..4 {
            test.count(true);
        }
        assert_eq!(test.decision(), None);
        test.count(true);
        assert_eq!(test.decision(), Some(Decision::Accepted));
    }
}
