// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1041/FR-1042 interval bounds for fair eventual probabilities.

use filament_exact::{Cancel, Integer, Rational};

use super::{
    decide_until_threshold, maximal_fair_end_components, Dyadic, ExactProbLimits, ProbProduct,
    UnboundedError, UnboundedObjective, UnboundedOutcome,
};

/// Bound a fair-minimum eventual probability through the complement event.
///
/// The complement of the minimum probability is the maximum probability of
/// reaching a fair end component outside the target set. Fair components are
/// derived with the product's complete enabled-action graph; the numeric
/// reachability interval is then produced by FR-198's outward-rounded solver.
pub fn decide_fair_reachability_threshold(
    product: &ProbProduct,
    target: &[bool],
    threshold: &Rational,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<UnboundedOutcome, UnboundedError> {
    if target.len() != product.states.len() {
        return Err(UnboundedError::InvalidState);
    }
    let mut fair_targets = vec![false; target.len()];
    for component in
        maximal_fair_end_components(product, &product.fairness).map_err(|error| match error {
            super::FairProductError::Product(error) => error,
            _ => UnboundedError::InvalidState,
        })?
    {
        for state in component.states {
            if !target[state.index()] {
                fair_targets[state.index()] = true;
            }
        }
    }
    let domain = target
        .iter()
        .map(|is_target| !is_target)
        .collect::<Vec<_>>();
    let avoidance_threshold = Rational::from_integer(Integer::from(1_i64)).sub(threshold);
    match decide_until_threshold(
        product,
        &domain,
        &fair_targets,
        UnboundedObjective::Max,
        &avoidance_threshold,
        limits,
        cancel,
    )? {
        UnboundedOutcome::Exact(mut result) => {
            for value in &mut result.values {
                if let super::ExtRational::Finite(value) = value {
                    *value = Rational::from_integer(Integer::from(1_i64)).sub(value);
                }
            }
            Ok(UnboundedOutcome::Exact(result))
        }
        UnboundedOutcome::Bounds {
            lower,
            upper,
            precision_bits,
            sweeps,
            policy,
        } => Ok(UnboundedOutcome::Bounds {
            lower: upper.iter().map(|bound| complement(bound)).collect(),
            upper: lower.iter().map(|bound| complement(bound)).collect(),
            precision_bits,
            sweeps,
            policy,
        }),
        UnboundedOutcome::PrecisionBudget { lower, upper } => {
            Ok(UnboundedOutcome::PrecisionBudget {
                lower: complement(&upper),
                upper: complement(&lower),
            })
        }
        stopped @ UnboundedOutcome::Stopped(_) => Ok(stopped),
    }
}

fn complement(value: &Dyadic) -> Dyadic {
    Dyadic {
        mantissa: (BigInt::from(1_u8) << value.exponent) - &value.mantissa,
        exponent: value.exponent,
    }
}

use num_bigint::BigInt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::{ProbEdge, ProbKey, ProbStateId, ProductAction, ProductKind};

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
    }

    #[test]
    fn fair_minimum_bounds_enclose_one_half_and_reach_two_fifths() {
        let edge = |from, to| ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability: rational(1, 2),
            elapsed: rational(0, 1),
            rewards: Vec::new(),
        };
        let product = ProbProduct {
            states: (0..3)
                .map(|state| ProbKey {
                    binding: None,
                    model: vec![state],
                    monitor: None,
                    accumulator: None,
                    pending: None,
                    certificate_state: None,
                })
                .collect(),
            kind: ProductKind::Mdp,
            edges: vec![
                edge(0, 1),
                edge(0, 2),
                ProbEdge {
                    from: ProbStateId(1),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(2),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(1, 1),
                    elapsed: rational(0, 1),
                    rewards: Vec::new(),
                },
            ],
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
        let UnboundedOutcome::Bounds { lower, upper, .. } = decide_fair_reachability_threshold(
            &product,
            &[false, true, false],
            &rational(2, 5),
            &limits,
            &Cancel::new(),
        )
        .unwrap() else {
            panic!("the fair value should be bounded by interval iteration");
        };
        assert!(lower[0].rational() >= rational(2, 5));
        assert!(lower[0].rational() <= rational(1, 2));
        assert!(upper[0].rational() >= rational(1, 2));
    }
}
