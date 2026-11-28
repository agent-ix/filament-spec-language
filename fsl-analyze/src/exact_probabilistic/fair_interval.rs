// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1041/FR-1042 interval bounds for fair eventual probabilities.

use filament_exact::{Cancel, Integer, Rational};
use ix_trace_rs::implements;

use super::fair::maximal_fair_end_components_within;
use super::rows::Evaluation;
use super::{
    decide_until_threshold, Dyadic, ExactProbLimits, ProbProduct, UnboundedError,
    UnboundedObjective, UnboundedOutcome,
};

/// Bound a fair-minimum eventual probability through the complement event.
///
/// The complement of the minimum probability is the maximum probability of
/// reaching a fair end component of the product restricted to non-target
/// states (FR-200). Components are derived on those states, with enabledness
/// read from the complete action graph, as [`super::decide_fair_reachability`]
/// derives them; the numeric reachability interval is then produced by FR-198's
/// outward-rounded solver.
#[implements("FR-200-AC-11")]
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
    let evaluation = Evaluation::of(product).map_err(|_| UnboundedError::InvalidProbability)?;
    let target = evaluation
        .extend(target)
        .ok_or(UnboundedError::InvalidState)?;
    let domain = target
        .iter()
        .map(|is_target| !is_target)
        .collect::<Vec<_>>();
    let mut fair_targets = vec![false; target.len()];
    for component in
        maximal_fair_end_components_within(&evaluation.product, &domain).map_err(|error| {
            match error {
                super::FairProductError::Product(error) => error,
                _ => UnboundedError::InvalidState,
            }
        })?
    {
        for state in component.states {
            fair_targets[state.index()] = true;
        }
    }
    let avoidance_threshold = Rational::from_integer(Integer::from(1_i64)).sub(threshold);
    match decide_until_threshold(
        &evaluation.product,
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
            result.values = evaluation.restrict(result.values);
            Ok(UnboundedOutcome::Exact(result))
        }
        UnboundedOutcome::Bounds {
            lower,
            upper,
            precision_bits,
            sweeps,
            policy,
        } => {
            let projected_lower = evaluation.restrict(upper.iter().map(complement).collect());
            let projected_upper = evaluation.restrict(lower.iter().map(complement).collect());
            Ok(UnboundedOutcome::Bounds {
                lower: projected_lower,
                upper: projected_upper,
                precision_bits,
                sweeps,
                policy,
            })
        }
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
    use crate::exact_probabilistic::{
        decide_fair_reachability, ExtRational, ProbEdge, ProbKey, ProbStateId, ProductAction,
        ProductKind,
    };
    use ix_trace_rs::trace;

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

    /// FR-200 "Reach" with an infimum takes the fair end components of the
    /// product restricted to non-target states, so a component that runs
    /// through a target is none. State 0 steps to 1 or to the sink 2, `1/2`
    /// each; 1 steps to the target 3 and 3 steps back to 1. By hand the fair
    /// minimum is `1/2` at 0, 1 at 1 and at 3, and 0 at 2.
    #[trace("TC-635", "FR-200-AC-2")]
    #[test]
    fn a_cycle_through_a_target_is_no_fair_end_component() {
        let edge = |from, to, numerator, denominator| ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability: rational(numerator, denominator),
            elapsed: rational(0, 1),
            rewards: Vec::new(),
        };
        let product = ProbProduct {
            states: (0..4)
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
                edge(0, 1, 1, 2),
                edge(0, 2, 1, 2),
                edge(1, 3, 1, 1),
                edge(2, 2, 1, 1),
                edge(3, 1, 1, 1),
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: Vec::new(),
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let target = [false, false, false, true];
        let expected = [
            rational(1, 2),
            rational(1, 1),
            rational(0, 1),
            rational(1, 1),
        ];
        let limits = ExactProbLimits::default();

        let UnboundedOutcome::Bounds { lower, upper, .. } = decide_fair_reachability_threshold(
            &product,
            &target,
            &rational(2, 5),
            &limits,
            &Cancel::new(),
        )
        .unwrap() else {
            panic!("the fair value should be bounded by interval iteration");
        };
        let bounds = lower
            .iter()
            .zip(&upper)
            .map(|(lower, upper)| (lower.rational(), upper.rational()))
            .collect::<Vec<_>>();
        assert_eq!(
            bounds,
            expected
                .iter()
                .map(|value| (value.clone(), value.clone()))
                .collect::<Vec<_>>()
        );

        let exact = decide_fair_reachability(
            &product,
            None,
            &target,
            UnboundedObjective::Min,
            &limits,
            &Cancel::new(),
        )
        .unwrap();
        assert_eq!(exact.values, expected.map(ExtRational::Finite));
    }
}
