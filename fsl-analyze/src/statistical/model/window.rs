// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-407 "Time windows" and FR-420 "Windows on an idle tail": the
//! positions a timed sample's reading with a time horizon reads, and the
//! decisions over every completion of a dense idle tail (FR-254).

use filament_exact::{Integer, Rational};
use fsl_eval::simulation::model::{
    TimedRaceStop, TimedSample, TimedSampleStop, TimedState, TimedStop,
};
use fsl_semantics::check::{
    BinaryTemporal, Bound as CheckedBound, CheckedFormula, CheckedInterval, FormulaNode,
    FormulaNodeId, UnaryTemporal,
};
use ix_trace_rs::implements;

use super::{broken, compares, Ended};
use crate::statistical::SampleHalt;
use fsl_replay::statistical::MeasureValue;

/// The positions of one sample's window `[0, h]`.
pub(super) struct TimedWindow {
    /// The sample, with its steps and its crossing step.
    pub(super) sample: TimedSample,
    /// The sampled positions, then a tick source's stutter positions.
    pub(super) states: Vec<TimedState>,
    /// The time stamp of each of `states`.
    pub(super) stamps: Vec<Rational>,
    /// Whether a dense idle tail enters the window before its end, after the
    /// last of `states`: it holds any number `m >= 0` of stutter positions.
    pub(super) dense_tail: bool,
}

/// The halt of a refused timed sample, or `None` where a tick subject's step
/// has an undecided successor, which ends a sample as a sample.
pub(super) fn sample_halt(
    stop: TimedSampleStop,
    max_sample_steps: u64,
) -> Result<Option<TimedWindow>, SampleHalt> {
    Err(match stop {
        TimedSampleStop::Race(TimedRaceStop::NotStochastic { state, identity }) => {
            SampleHalt::NotStochastic { state, identity }
        }
        TimedSampleStop::Race(TimedRaceStop::Timed(TimedStop::Expansion(stop))) => {
            SampleHalt::Expansion(stop)
        }
        TimedSampleStop::TimeLockedSample { state } => SampleHalt::TimeLockedSample { state },
        TimedSampleStop::MaxSampleSteps => SampleHalt::MaxSampleSteps {
            value: max_sample_steps,
        },
        TimedSampleStop::Step(stop) => match Ended::from(*stop) {
            Ended::Halt(halt) => halt,
            Ended::Undecided => return Ok(None),
        },
        TimedSampleStop::Race(_) => broken(),
    })
}

/// Whether every completion of a dense idle tail gives the unweighted
/// fraction of `held` window positions out of `positions` one value, or its
/// event one truth: `(held + m * tail) / (positions + m)` over every `m >= 0`,
/// where `tail` is whether the predicate holds in the idle state.
///
/// Returns the value at `m = 0` where every completion reads alike, and
/// `None` where two completions differ.
#[implements("FR-254-AC-7")]
#[implements("FR-254-AC-8")]
pub(super) fn tail_fraction(
    held: u64,
    positions: u64,
    tail: bool,
    event: Option<&CheckedBound>,
) -> Option<Rational> {
    let first = Rational::from_integer(Integer::from(held))
        .div(&Rational::from_integer(Integer::from(positions)))?;
    // The value moves monotonically from `first` towards `limit` and never
    // reaches it, unless the two are equal.
    let limit = Rational::from_integer(Integer::from(u64::from(tail)));
    if first == limit {
        return Some(first);
    }
    let event = event?;
    let rising = limit > first;
    let eventually = match event {
        // Values just under a rising limit are at most `c` exactly when the
        // limit is; just over a falling one, only when the limit is below `c`.
        CheckedBound::AtMost(c) => {
            if rising {
                limit <= *c
            } else {
                limit < *c
            }
        }
        CheckedBound::AtLeast(c) => {
            if rising {
                limit > *c
            } else {
                limit >= *c
            }
        }
    };
    (compares(event, &MeasureValue::Value(first.clone())) == eventually).then_some(first)
}

/// The cut points of a formula whose temporal operators each have a bounded
/// time interval and no temporal operator inside them, or `None` for any
/// other formula.
#[implements("FR-254-AC-7")]
#[implements("FR-254-AC-8")]
pub(super) fn flat_interval_bounds(formula: &CheckedFormula) -> Option<Vec<Rational>> {
    fn is_state(nodes: &[FormulaNode], id: FormulaNodeId) -> bool {
        match nodes.get(id.index()) {
            Some(FormulaNode::Constant(_) | FormulaNode::Holds { .. }) => true,
            Some(FormulaNode::Not(operand)) => is_state(nodes, *operand),
            Some(
                FormulaNode::And(left, right)
                | FormulaNode::Or(left, right)
                | FormulaNode::Implies(left, right),
            ) => is_state(nodes, *left) && is_state(nodes, *right),
            _ => false,
        }
    }
    let nodes = formula.nodes();
    let mut bounds = Vec::new();
    for found in nodes {
        let interval = match found {
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Not(_)
            | FormulaNode::And(..)
            | FormulaNode::Or(..)
            | FormulaNode::Implies(..) => continue,
            FormulaNode::Unary {
                operator: UnaryTemporal::Eventually | UnaryTemporal::Always,
                interval,
                operand,
            } if is_state(nodes, *operand) => interval,
            FormulaNode::Binary {
                operator: BinaryTemporal::Until | BinaryTemporal::Release,
                interval,
                left,
                right,
            } if is_state(nodes, *left) && is_state(nodes, *right) => interval,
            _ => return None,
        };
        let Some(CheckedInterval::Timed(interval)) = interval else {
            return None;
        };
        bounds.push(interval.lower().clone());
        bounds.push(interval.upper()?.clone());
    }
    Some(bounds)
}

/// The most interval pieces a completion search enumerates: `2^12` readings.
const MAX_PIECES: usize = 12;

/// The stutter stamps one of which stands for each piece of `(from, to]` the
/// `bounds` cut it into: every stutter stamp of a piece lies in the same
/// intervals, so a formula with no nested temporal operator reads each
/// completion as one choice of which pieces hold a stutter. `None` where
/// there are more than [`MAX_PIECES`].
#[implements("FR-254-AC-7")]
#[implements("FR-254-AC-8")]
pub(super) fn piece_stamps(
    bounds: &[Rational],
    from: &Rational,
    to: &Rational,
) -> Option<Vec<Rational>> {
    let mut cuts: Vec<Rational> = bounds
        .iter()
        .filter(|bound| *bound > from && *bound < to)
        .cloned()
        .collect();
    cuts.push(to.clone());
    cuts.sort();
    cuts.dedup();
    if cuts.len().saturating_mul(2) > MAX_PIECES {
        return None;
    }
    let two = Rational::from_integer(Integer::from(2u64));
    let mut previous = from.clone();
    let mut pieces = Vec::with_capacity(cuts.len() * 2);
    for cut in cuts {
        pieces.push(previous.add(&cut).div(&two)?);
        pieces.push(cut.clone());
        previous = cut;
    }
    Some(pieces)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a ratio")
    }

    /// One position of two holds and the idle state does not, so the value is
    /// `1 / (2 + m)`, falling from `1/2` towards 0 and never reaching it.
    #[test]
    fn a_falling_fraction_is_decided_only_where_every_completion_agrees() {
        let read = |event: CheckedBound| tail_fraction(1, 2, false, Some(&event));
        let first = Some(rational(1, 2));
        // True for every `m`.
        assert_eq!(read(CheckedBound::AtMost(rational(3, 5))), first);
        assert_eq!(read(CheckedBound::AtLeast(rational(0, 1))), first);
        // False for every `m`: the value never reaches 0.
        assert_eq!(read(CheckedBound::AtMost(rational(0, 1))), first);
        assert_eq!(read(CheckedBound::AtLeast(rational(3, 5))), first);
        // The truth changes with `m`.
        assert_eq!(read(CheckedBound::AtMost(rational(2, 5))), None);
        assert_eq!(read(CheckedBound::AtLeast(rational(2, 5))), None);
        // A mean reads the value itself, which changes with `m`.
        assert_eq!(tail_fraction(1, 2, false, None), None);
    }
}
