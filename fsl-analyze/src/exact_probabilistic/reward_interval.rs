// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-198/FR-1042 interval iteration for finite expected-reward values.

use filament_exact::{Integer, Rational, Value};
use num_bigint::BigInt;
use num_integer::Integer as BigInteger;

use super::{Dyadic, ExactProbLimits, ProbProduct, ProductKind, UnboundedError};

/// Threshold-directed outcome for expected total reward.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpectedRewardOutcome {
    /// The exact solver established the value (including positive infinity).
    Exact(super::UnboundedResult),
    /// Outward-rounded endpoints separate the requested threshold.
    Bounds {
        /// Lower endpoints in product-state order.
        lower: Vec<Dyadic>,
        /// Upper endpoints in product-state order.
        upper: Vec<Dyadic>,
        /// Dyadic precision used by the interval.
        precision_bits: u32,
        /// Bellman sweeps performed across all precision levels.
        sweeps: u64,
    },
    /// The interval still straddles the threshold at the precision limit.
    PrecisionBudget {
        /// Lower endpoint at the initial state.
        lower: Dyadic,
        /// Upper endpoint at the initial state.
        upper: Dyadic,
    },
}

/// Use monotone Bellman iteration to bound a single-action product's expected total reward.
///
/// MDP policy selection remains in the exact solver; this interval routine is
/// also valid for a product whose states each have one semantic action.
pub fn decide_expected_reward_threshold(
    product: &ProbProduct,
    target: &[bool],
    reward: &str,
    threshold: &Rational,
    limits: &ExactProbLimits,
) -> Result<ExpectedRewardOutcome, UnboundedError> {
    if target.len() != product.states.len()
        || limits.precision_bits == 0
        || limits.max_precision_bits < limits.precision_bits
    {
        return Err(UnboundedError::InvalidState);
    }
    let count = target.len();
    let mut rows = vec![Vec::new(); count];
    for edge in &product.edges {
        let Some(row) = rows.get_mut(edge.from.index()) else {
            return Err(UnboundedError::InvalidState);
        };
        if edge.to.index() >= count {
            return Err(UnboundedError::InvalidState);
        }
        row.push(edge);
    }
    if product.kind == ProductKind::Mdp
        && rows.iter().any(|edges| {
            edges
                .first()
                .is_some_and(|first| edges.iter().any(|edge| edge.choice != first.choice))
        })
    {
        return Err(UnboundedError::InvalidState);
    }
    if has_non_target_bottom_component(&rows, target) {
        let exact = super::decide_expected_reward(
            product,
            target,
            reward,
            super::UnboundedObjective::Min,
            limits,
        )?;
        return Ok(ExpectedRewardOutcome::Exact(exact));
    }
    let mut can_reach = target.to_vec();
    loop {
        let old = can_reach.clone();
        for state in 0..count {
            if !can_reach[state]
                && rows[state]
                    .iter()
                    .any(|edge| !edge.probability.is_zero() && old[edge.to.index()])
            {
                can_reach[state] = true;
            }
        }
        if can_reach == old {
            break;
        }
    }
    if can_reach
        .iter()
        .enumerate()
        .any(|(i, reaches)| !target[i] && !reaches)
    {
        let exact = super::decide_expected_reward(
            product,
            target,
            reward,
            super::UnboundedObjective::Min,
            limits,
        )?;
        return Ok(ExpectedRewardOutcome::Exact(exact));
    }
    let initial = product
        .initial
        .first()
        .map(|(_, _, state)| state.index())
        .unwrap_or(0);
    let mut smallest_probability: Option<Rational> = None;
    let mut largest_reward = Rational::from_integer(Integer::from(0_i64));
    for (index, edges) in rows.iter().enumerate() {
        if target[index] {
            continue;
        }
        if edges.is_empty() {
            return Err(UnboundedError::MissingAction);
        }
        let mass = edges
            .iter()
            .fold(Rational::from_integer(Integer::from(0_i64)), |sum, edge| {
                sum.add(&edge.probability)
            });
        if mass != Rational::from_integer(Integer::from(1_i64)) {
            return Err(UnboundedError::InvalidProbability);
        }
        for edge in edges {
            if edge.probability > Rational::from_integer(Integer::from(0_i64))
                && smallest_probability
                    .as_ref()
                    .is_none_or(|current| edge.probability < *current)
            {
                smallest_probability = Some(edge.probability.clone());
            }
            let edge_reward = edge
                .rewards
                .iter()
                .find(|(name, _)| name.as_str() == reward)
                .map(|(_, value)| match value {
                    Value::Integer(value) => Ok(Rational::from_integer(value.clone())),
                    Value::Quantity(value) => Ok(value.magnitude().clone()),
                    _ => Err(UnboundedError::MissingReward),
                })
                .transpose()?
                .unwrap_or_else(|| Rational::from_integer(Integer::from(0_i64)));
            if edge_reward < Rational::from_integer(Integer::from(0_i64)) {
                return Err(UnboundedError::MissingReward);
            }
            largest_reward = largest_reward.max(edge_reward);
        }
    }
    let Some(min_probability) = smallest_probability else {
        return Err(UnboundedError::InvalidProbability);
    };
    let steps = u32::try_from(count.max(1)).map_err(|_| UnboundedError::MaxRationalBits)?;
    let escape_probability = (0..steps)
        .fold(Rational::from_integer(Integer::from(1_i64)), |value, _| {
            value.mul(&min_probability)
        });
    let upper_seed = largest_reward
        .mul(&Rational::from_integer(Integer::from(count.max(1))))
        .div(&escape_probability)
        .ok_or(UnboundedError::InvalidProbability)?;
    let zero = Rational::from_integer(Integer::from(0_i64));
    let mut precision = limits.precision_bits;
    let mut total_sweeps = 0_u64;
    loop {
        let mut lower = vec![zero.clone(); count];
        let mut upper = vec![upper_seed.clone(); count];
        for state in 0..count {
            if target[state] {
                lower[state] = zero.clone();
                upper[state] = zero.clone();
            }
        }
        for _ in 0..limits.max_iterations {
            let mut next_lower = lower.clone();
            let mut next_upper = upper.clone();
            for state in 0..count {
                if target[state] {
                    continue;
                }
                let mut low = zero.clone();
                let mut high = zero.clone();
                for edge in &rows[state] {
                    let reward_value = edge
                        .rewards
                        .iter()
                        .find(|(name, _)| name.as_str() == reward)
                        .map(|(_, value)| match value {
                            Value::Integer(value) => Ok(Rational::from_integer(value.clone())),
                            Value::Quantity(value) => Ok(value.magnitude().clone()),
                            _ => Err(UnboundedError::MissingReward),
                        })
                        .transpose()?
                        .unwrap_or_else(|| zero.clone());
                    low = low.add(
                        &edge
                            .probability
                            .mul(&reward_value.add(&lower[edge.to.index()])),
                    );
                    high = high.add(
                        &edge
                            .probability
                            .mul(&reward_value.add(&upper[edge.to.index()])),
                    );
                }
                next_lower[state] = round(&low, precision, false);
                next_upper[state] = round(&high, precision, true);
            }
            lower = next_lower;
            upper = next_upper;
            total_sweeps = total_sweeps.saturating_add(1);
            if lower[initial] > *threshold || upper[initial] < *threshold {
                return Ok(ExpectedRewardOutcome::Bounds {
                    lower: lower
                        .iter()
                        .map(|value| dyadic(value, precision, false))
                        .collect(),
                    upper: upper
                        .iter()
                        .map(|value| dyadic(value, precision, true))
                        .collect(),
                    precision_bits: precision,
                    sweeps: total_sweeps,
                });
            }
        }
        if precision >= limits.max_precision_bits {
            return Ok(ExpectedRewardOutcome::PrecisionBudget {
                lower: dyadic(&lower[initial], precision, false),
                upper: dyadic(&upper[initial], precision, true),
            });
        }
        precision = precision.saturating_mul(2).min(limits.max_precision_bits);
    }
}

fn has_non_target_bottom_component(rows: &[Vec<&super::ProbEdge>], target: &[bool]) -> bool {
    let count = rows.len();
    let mut forward = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for (source, edges) in rows.iter().enumerate() {
        for edge in edges {
            if edge.probability.is_zero() {
                continue;
            }
            forward[source].push(edge.to.index());
            reverse[edge.to.index()].push(source);
        }
    }
    let mut seen = vec![false; count];
    let mut order = Vec::with_capacity(count);
    for root in 0..count {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((state, next)) = stack.pop() {
            if next < forward[state].len() {
                stack.push((state, next + 1));
                let successor = forward[state][next];
                if !seen[successor] {
                    seen[successor] = true;
                    stack.push((successor, 0));
                }
            } else {
                order.push(state);
            }
        }
    }
    seen.fill(false);
    for root in order.into_iter().rev() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut component = vec![root];
        let mut stack = vec![root];
        while let Some(state) = stack.pop() {
            for predecessor in &reverse[state] {
                if !seen[*predecessor] {
                    seen[*predecessor] = true;
                    component.push(*predecessor);
                    stack.push(*predecessor);
                }
            }
        }
        let members = component
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let bottom = component.iter().all(|state| {
            forward[*state]
                .iter()
                .all(|successor| members.contains(successor))
        });
        if bottom && component.iter().all(|state| !target[*state]) {
            return true;
        }
    }
    false
}

fn round(value: &Rational, precision: u32, upward: bool) -> Rational {
    dyadic(value, precision, upward).rational()
}

fn dyadic(value: &Rational, precision: u32, upward: bool) -> Dyadic {
    let scale = BigInt::from(1_u8) << precision;
    let numerator = value.numerator().as_big() * &scale;
    let denominator = value.denominator().as_big();
    let mantissa = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    Dyadic {
        mantissa,
        exponent: precision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::{ProbEdge, ProbKey, ProbStateId, ProductAction};

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
    }

    #[test]
    fn retry_reward_bounds_enclose_two_and_separate_three_halves() {
        let reward = filament_exact::Identifier::new("R").unwrap();
        let edge = |from, to, probability| ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability,
            elapsed: rational(0, 1),
            rewards: vec![(reward.clone(), Value::Integer(Integer::from(1_i64)))],
        };
        let product = ProbProduct {
            states: (0..2)
                .map(|state| ProbKey {
                    binding: None,
                    model: vec![state],
                    monitor: None,
                    accumulator: None,
                    pending: None,
                    certificate_state: None,
                    timed_history: None,
                })
                .collect(),
            kind: ProductKind::Dtmc,
            edges: vec![edge(0, 0, rational(1, 2)), edge(0, 1, rational(1, 2))],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: Vec::new(),
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let limits = ExactProbLimits {
            precision_bits: 4,
            max_precision_bits: 16,
            max_rational_bits: 1,
            ..ExactProbLimits::default()
        };
        let ExpectedRewardOutcome::Bounds {
            lower,
            upper,
            precision_bits,
            ..
        } = decide_expected_reward_threshold(
            &product,
            &[false, true],
            "R",
            &rational(3, 2),
            &limits,
        )
        .unwrap()
        else {
            panic!("the retry chain should separate the threshold by interval iteration");
        };
        assert!(lower[0].rational() >= rational(3, 2));
        assert!(lower[0].rational() <= rational(2, 1));
        assert!(upper[0].rational() >= rational(2, 1));
        assert!(precision_bits <= limits.max_precision_bits);
    }
}
