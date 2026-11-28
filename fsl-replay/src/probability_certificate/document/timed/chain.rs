// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checker's own solver for a refuting finite Markov chain (FR-922
//! "Behavior", FR-921).
//!
//! A refutation of an expected-elapsed claim states a finite chain: each state
//! has an exact rational delay and branches with exact rational
//! probabilities. The checker solves the chain's expected elapsed time to the
//! goal itself, by exact Gaussian elimination, and never reads a value from
//! the producer. Nothing here calls into the producer's code.

#![cfg_attr(
    not(test),
    allow(dead_code, reason = "the chain witness is wired in QSL-3495 step 6.5")
)]

use filament_exact::Rational;
use ix_trace_rs::implements;

/// One state of a stated chain.
#[derive(Clone, Debug)]
pub(super) struct ChainState {
    /// The exact delay taken in the state before its branches fire.
    pub(super) delay: Rational,
    /// Target state index and probability, per branch. A goal state has none.
    pub(super) branches: Vec<(usize, Rational)>,
    /// Whether the state is in the claim's target.
    pub(super) goal: bool,
}

/// Why a chain was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChainRefusal {
    /// A non-goal state's probabilities do not sum to 1, or one is negative.
    ProbabilitiesDoNotSumToOne,
    /// A branch names a state the chain does not have.
    TargetOutOfRange,
    /// A goal state has branches, or the chain has no initial state.
    Malformed,
    /// Some state cannot reach the goal: its expected time is infinite.
    Improper,
    /// The solve would exceed the work bound.
    LimitReached,
}

fn integer(value: i64) -> Rational {
    Rational::from_integer(value.into())
}

/// The expected elapsed time from `initial` to the goal, solved exactly.
///
/// `work_bound` caps the number of rational operations (cubic in the number
/// of non-goal states); a chain over it is `LimitReached`.
#[implements("FR-922-AC-7")]
pub(super) fn expected_elapsed(
    states: &[ChainState],
    initial: usize,
    work_bound: u64,
) -> Result<Rational, ChainRefusal> {
    let size = states.len();
    if initial >= size {
        return Err(ChainRefusal::Malformed);
    }
    for state in states {
        if state.goal {
            if !state.branches.is_empty() {
                return Err(ChainRefusal::Malformed);
            }
            continue;
        }
        let mut sum = integer(0);
        for (target, probability) in &state.branches {
            if *target >= size {
                return Err(ChainRefusal::TargetOutOfRange);
            }
            if probability.numerator().is_negative() {
                return Err(ChainRefusal::ProbabilitiesDoNotSumToOne);
            }
            sum = sum.add(probability);
        }
        if sum != integer(1) {
            return Err(ChainRefusal::ProbabilitiesDoNotSumToOne);
        }
    }
    if states[initial].goal {
        return Ok(integer(0));
    }
    // Unknowns: the non-goal states. Row i: E_i - sum p_ij E_j = delay_i.
    let unknowns: Vec<usize> = (0..size).filter(|at| !states[*at].goal).collect();
    let count = unknowns.len() as u64;
    if count.saturating_mul(count).saturating_mul(count) > work_bound {
        return Err(ChainRefusal::LimitReached);
    }
    let column_of = |state: usize| unknowns.iter().position(|at| *at == state);
    let width = unknowns.len() + 1;
    let mut rows: Vec<Vec<Rational>> = unknowns
        .iter()
        .map(|at| {
            let mut row = vec![integer(0); width];
            row[column_of(*at).unwrap_or(0)] = integer(1);
            for (target, probability) in &states[*at].branches {
                if let Some(column) = column_of(*target) {
                    row[column] = row[column].sub(probability);
                }
            }
            row[width - 1] = states[*at].delay.clone();
            row
        })
        .collect();
    for column in 0..unknowns.len() {
        // A zero pivot column means a closed class that never reaches the goal.
        let pivot = (column..rows.len())
            .find(|row| !rows[*row][column].is_zero())
            .ok_or(ChainRefusal::Improper)?;
        rows.swap(pivot, column);
        let lead = rows[column][column].clone();
        let pivot_row: Vec<Rational> = rows[column]
            .iter()
            .map(|cell| cell.div(&lead).ok_or(ChainRefusal::Improper))
            .collect::<Result<_, _>>()?;
        for (at, row) in rows.iter_mut().enumerate() {
            if at == column || row[column].is_zero() {
                continue;
            }
            let factor = row[column].clone();
            for (cell, pivot_cell) in row.iter_mut().zip(&pivot_row) {
                *cell = cell.sub(&factor.mul(pivot_cell));
            }
        }
        rows[column] = pivot_row;
    }
    let at = column_of(initial).ok_or(ChainRefusal::Malformed)?;
    Ok(rows[at][width - 1].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(numerator.into(), denominator.into()).expect("nonzero denominator")
    }

    fn state(delay: Rational, branches: Vec<(usize, Rational)>) -> ChainState {
        ChainState {
            delay,
            branches,
            goal: false,
        }
    }

    fn goal() -> ChainState {
        ChainState {
            delay: integer(0),
            branches: Vec::new(),
            goal: true,
        }
    }

    /// The retry loop of row H4 (design 5a): `S0 --49/25--> S1`, then half to
    /// the goal and half back to `S0`; `E = t + E/2`, `E = 2t = 98/25`.
    fn retry_loop(delay: Rational) -> Vec<ChainState> {
        vec![
            state(delay, vec![(1, integer(1))]),
            state(integer(0), vec![(2, ratio(1, 2)), (0, ratio(1, 2))]),
            goal(),
        ]
    }

    #[trace("FR-922-AC-6")]
    #[test]
    fn the_retry_loop_chain_solves_to_98_over_25() {
        let value = expected_elapsed(&retry_loop(ratio(49, 25)), 0, 1_000).expect("solves");
        assert_eq!(value, ratio(98, 25));
    }

    #[trace("FR-922-AC-6")]
    #[test]
    fn a_path_chain_sums_its_delays() {
        let path = vec![
            state(ratio(3, 2), vec![(1, integer(1))]),
            state(ratio(1, 4), vec![(2, integer(1))]),
            goal(),
        ];
        assert_eq!(expected_elapsed(&path, 0, 1_000), Ok(ratio(7, 4)));
    }

    #[trace("FR-922-AC-7")]
    #[test]
    fn a_branch_probability_that_does_not_sum_to_one_is_refused() {
        let chain = vec![state(integer(1), vec![(1, ratio(1, 2))]), goal()];
        assert_eq!(
            expected_elapsed(&chain, 0, 1_000),
            Err(ChainRefusal::ProbabilitiesDoNotSumToOne)
        );
    }

    #[trace("FR-922-AC-7")]
    #[test]
    fn a_closed_class_that_never_reaches_the_goal_is_improper() {
        let chain = vec![state(integer(1), vec![(0, integer(1))]), goal()];
        assert_eq!(
            expected_elapsed(&chain, 0, 1_000),
            Err(ChainRefusal::Improper)
        );
    }

    #[trace("FR-922-AC-7")]
    #[test]
    fn a_branch_to_a_missing_state_is_refused() {
        let chain = vec![state(integer(1), vec![(5, integer(1))]), goal()];
        assert_eq!(
            expected_elapsed(&chain, 0, 1_000),
            Err(ChainRefusal::TargetOutOfRange)
        );
    }

    #[trace("FR-922-AC-7")]
    #[test]
    fn a_chain_over_the_work_bound_is_limit_reached() {
        assert_eq!(
            expected_elapsed(&retry_loop(ratio(49, 25)), 0, 1),
            Err(ChainRefusal::LimitReached)
        );
    }
}
