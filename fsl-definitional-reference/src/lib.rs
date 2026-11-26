// SPDX-License-Identifier: AGPL-3.0-or-later
//! Support for an exact, deliberately small semantic reference used by checker tests.
//!
//! This crate is a test oracle, never a verdict route. Its dependency surface
//! is restricted to checked packages and data crates so it cannot inherit a
//! producer, analyzer, or checker decision. Unsupported expressions are
//! refused rather than delegated to production semantics.

#![forbid(unsafe_code)]

use filament_checked::node::NodeKind;
use filament_exact::{Meter, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{
    CheckedGraph, ProbForm, ProbScheduler, Reward, RewardEvaluationFailure, RewardEvaluator,
    StateClauseKind,
};
use fsl_semantics::model::domain_package::{DomainPackageRecord, NativeValueType, ValueTypeRef};
use ix_trace_rs::implements;

mod interpreter;
pub use interpreter::{evaluate_boolean, ReferenceClockState, ReferenceFiniteState};

/// Maximum horizon enumerated by the reference's finite-horizon solver.
pub const MAX_HORIZON_STEPS: usize = 256;

/// Scheduler quantifiers understood independently of the source claim syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceSchedulerQuantifier {
    /// A claim required to hold under every enabled scheduler.
    Every,
    /// A claim whose truth asks whether some scheduler can achieve the bound.
    Some,
    /// A claim evaluated under this fixed, declared workload.
    Workload(String),
}

/// The scheduler quantifier represented by a checked probabilistic claim.
pub fn scheduler_quantifier(scheduler: &ProbScheduler) -> ReferenceSchedulerQuantifier {
    match scheduler {
        ProbScheduler::Every => ReferenceSchedulerQuantifier::Every,
        ProbScheduler::Workload(name) => ReferenceSchedulerQuantifier::Workload(name.clone()),
    }
}

/// Refuses a quantile when its scheduler semantics are outside the implemented
/// quantile solver. `Some` is the AC-19 refusal; `Every` remains distinct from
/// a fixed declared workload.
#[implements("FR-1051-AC-19")]
pub fn validate_quantile_scheduler(
    scheduler: &ReferenceSchedulerQuantifier,
) -> Result<(), ReferenceRefusal> {
    match scheduler {
        ReferenceSchedulerQuantifier::Some => Err(ReferenceRefusal {
            construct: "quantile claim over some scheduler".to_owned(),
        }),
        ReferenceSchedulerQuantifier::Every => Ok(()),
        ReferenceSchedulerQuantifier::Workload(_) => Err(ReferenceRefusal {
            construct: "quantile over an unsupported declared workload".to_owned(),
        }),
    }
}

/// A state-indexed action relation for a finite, exact probabilistic model.
/// Each action is a finite distribution over state indices. The target set is
/// supplied separately to the reachability query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TinyMdp {
    actions: Vec<Vec<Vec<(usize, filament_exact::Rational)>>>,
}

impl TinyMdp {
    /// Builds a model from state, action and transition rows. Every state must
    /// have at least one action; distributions must be nonempty, in range,
    /// nonnegative and sum exactly to one.
    #[implements("FR-1051-AC-19")]
    pub fn new(
        actions: Vec<Vec<Vec<(usize, filament_exact::Rational)>>>,
    ) -> Result<Self, ReferenceRefusal> {
        for state in &actions {
            if state.is_empty() {
                return Err(ReferenceRefusal {
                    construct: "state with no available action".to_owned(),
                });
            }
            for action in state {
                if action.is_empty() {
                    return Err(ReferenceRefusal {
                        construct: "empty probabilistic support".to_owned(),
                    });
                }
                let mut total =
                    filament_exact::Rational::from_integer(filament_exact::Integer::zero());
                for (destination, probability) in action {
                    if *destination >= actions.len() {
                        return Err(ReferenceRefusal {
                            construct: "transition destination outside finite state space"
                                .to_owned(),
                        });
                    }
                    if probability
                        < &filament_exact::Rational::from_integer(filament_exact::Integer::zero())
                    {
                        return Err(ReferenceRefusal {
                            construct: "negative probability weight".to_owned(),
                        });
                    }
                    total = total.add(probability);
                }
                if total != filament_exact::Rational::from_integer(filament_exact::Integer::one()) {
                    return Err(ReferenceRefusal {
                        construct: "probability weights do not sum to one".to_owned(),
                    });
                }
            }
        }
        if actions.is_empty() {
            return Err(ReferenceRefusal {
                construct: "empty finite state space".to_owned(),
            });
        }
        Ok(Self { actions })
    }

    /// Maximum probability of reaching a target within `horizon` steps,
    /// allowing deterministic scheduler choices to depend on both state and
    /// steps remaining. An iterative dynamic program keeps the best exact
    /// expectation at each horizon.
    #[implements("FR-1051-AC-19")]
    pub fn some_scheduler_reachability_within(
        &self,
        initial: usize,
        targets: &[bool],
        horizon: usize,
    ) -> Result<filament_exact::Rational, ReferenceRefusal> {
        if horizon > MAX_HORIZON_STEPS {
            return Err(ReferenceRefusal {
                construct: "horizon exceeds the reference step bound".to_owned(),
            });
        }
        if initial >= self.actions.len() || targets.len() != self.actions.len() {
            return Err(ReferenceRefusal {
                construct: "initial state or target set outside finite state space".to_owned(),
            });
        }
        let zero = filament_exact::Rational::from_integer(filament_exact::Integer::zero());
        let one = filament_exact::Rational::from_integer(filament_exact::Integer::one());
        let mut values: Vec<_> = targets
            .iter()
            .map(|target| if *target { one.clone() } else { zero.clone() })
            .collect();
        for _ in 0..horizon {
            let mut next = values.clone();
            for (state, actions) in self.actions.iter().enumerate() {
                if targets[state] {
                    continue;
                }
                next[state] = actions
                    .iter()
                    .map(|action| {
                        action
                            .iter()
                            .fold(zero.clone(), |sum, (destination, probability)| {
                                sum.add(&probability.mul(&values[*destination]))
                            })
                    })
                    .max()
                    .expect("construction requires at least one action per state");
            }
            values = next;
        }
        Ok(values[initial].clone())
    }

    /// Maximum exact accumulated state-action reward over a finite horizon.
    /// `rewards[state][action]` must have one entry per available action.
    #[implements("FR-1051-AC-19")]
    pub fn some_scheduler_expected_reward_within(
        &self,
        initial: usize,
        rewards: &[Vec<filament_exact::Rational>],
        horizon: usize,
    ) -> Result<filament_exact::Rational, ReferenceRefusal> {
        if horizon > MAX_HORIZON_STEPS {
            return Err(ReferenceRefusal {
                construct: "horizon exceeds the reference step bound".to_owned(),
            });
        }
        if initial >= self.actions.len()
            || rewards.len() != self.actions.len()
            || rewards
                .iter()
                .zip(&self.actions)
                .any(|(state_rewards, actions)| state_rewards.len() != actions.len())
        {
            return Err(ReferenceRefusal {
                construct: "reward table outside finite state/action space".to_owned(),
            });
        }
        let zero = filament_exact::Rational::from_integer(filament_exact::Integer::zero());
        let mut values = vec![zero; self.actions.len()];
        for _ in 0..horizon {
            let next = self
                .actions
                .iter()
                .enumerate()
                .map(|(state, actions)| {
                    actions
                        .iter()
                        .enumerate()
                        .map(|(action_index, action)| {
                            action.iter().fold(
                                rewards[state][action_index].clone(),
                                |sum, (destination, probability)| {
                                    sum.add(&probability.mul(&values[*destination]))
                                },
                            )
                        })
                        .max()
                        .expect("model construction requires one action per state")
                })
                .collect();
            values = next;
        }
        Ok(values[initial].clone())
    }
}

/// Reference-owned evaluator supplied when package checking needs a reward
/// evaluator. It only admits reward bodies in the tiny literal fragment.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceRewardEvaluator;

/// A construct the tiny reference does not define. Callers must not attach a
/// truth value to a package refused with this error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceRefusal {
    /// The unsupported construct, in stable diagnostic wording.
    pub construct: String,
}

/// Refuses an operation guard containing disjunction.
///
/// The fragment admits conjunctive guards. This check walks the checked
/// operation clauses from [`CheckedGraph`]; it does not parse or reinterpret
/// source text.
#[implements("FR-1051-AC-19")]
pub fn validate_conjunctive_guards(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    for model in graph.timed_models() {
        for guard in model.guards() {
            if guard.parts().len() > 1 {
                return Err(ReferenceRefusal {
                    construct: "disjunctive timed guard".to_owned(),
                });
            }
        }
    }
    for clause in graph.state_clauses() {
        if clause.kind() != StateClauseKind::Precondition || clause.operation().is_none() {
            continue;
        }
        let mut pending = vec![clause.body()];
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                NodeKind::Connective(filament_checked::node::Connective::Or, _, _, _)
            ) {
                return Err(ReferenceRefusal {
                    construct: "disjunctive operation guard".to_owned(),
                });
            }
            pending.extend(node.children());
        }
    }
    Ok(())
}

/// Refuses checked constructs outside the currently executable tiny fragment.
/// Every refusal occurs before any semantic truth value can be returned.
#[implements("FR-1051-AC-19")]
pub fn validate_tiny_fragment(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    validate_conjunctive_guards(graph)?;
    for package in graph
        .selected_model_views()
        .values()
        .map(|view| view.domain_package())
        .chain(graph.source_packages())
    {
        if package.records.iter().any(|record| {
            matches!(
                record,
                DomainPackageRecord::FieldMember(field)
                    if matches!(
                        &field.value_type,
                        ValueTypeRef::Native(NativeValueType::Integer)
                    )
            )
        }) {
            return Err(ReferenceRefusal {
                construct: "unbounded integer field".to_owned(),
            });
        }
    }
    for model in graph.timed_models() {
        let fractional_guard = model
            .guards()
            .iter()
            .flat_map(|guard| guard.parts())
            .flat_map(|part| part.clocks())
            .any(|clock| !clock.bound().is_integer());
        let fractional_invariant = model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds())
            .any(|clock| !clock.bound().is_integer());
        if fractional_guard || fractional_invariant {
            return Err(ReferenceRefusal {
                construct: "clock compared with a non-integer constant".to_owned(),
            });
        }
        if model
            .resets()
            .iter()
            .flat_map(|resets| resets.resets())
            .any(|reset| !reset.value().is_zero())
        {
            return Err(ReferenceRefusal {
                construct: "clock reset to a nonzero constant".to_owned(),
            });
        }
    }
    for model in graph.hybrid_models() {
        if model
            .modes
            .iter()
            .flat_map(|mode| &mode.rates)
            .any(|rate| !matches!(rate, fsl_semantics::check::HybridTerm::Constant(_)))
        {
            return Err(ReferenceRefusal {
                construct: "hybrid mode with a non-constant rate".to_owned(),
            });
        }
    }
    for claim in graph.probabilistic_model().claims() {
        if matches!(&claim.form, ProbForm::Quantile { .. }) {
            validate_quantile_scheduler(&scheduler_quantifier(&claim.scheduler))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TinyMdp;
    use filament_exact::{Integer, Rational};
    use ix_trace_rs::trace;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("hand-derived probabilities have nonzero denominators")
    }

    /// The first action is best with one step left, while the other is best
    /// with two: a scheduler must observe the remaining horizon.
    #[trace("FR-1051-AC-19")]
    #[test]
    fn horizon_reachability_uses_remaining_steps_in_scheduler_choice() {
        // At state 0, action A reaches target 2 with 3/5 immediately. Action
        // B reaches state 1, whose sole action reaches target with 9/10.
        // Thus the hand values are 3/5 at horizon one and 9/10 at horizon
        // two; a stationary choice at state 0 cannot achieve both.
        let model = TinyMdp::new(vec![
            vec![
                vec![(2, ratio(3, 5)), (3, ratio(2, 5))],
                vec![(1, ratio(1, 1))],
            ],
            vec![vec![(2, ratio(9, 10)), (3, ratio(1, 10))]],
            vec![vec![(2, ratio(1, 1))]],
            vec![vec![(3, ratio(1, 1))]],
        ])
        .expect("all finite distributions sum exactly to one");
        let targets = [false, false, true, false];

        assert_eq!(
            model
                .some_scheduler_reachability_within(0, &targets, 1)
                .expect("valid initial state and target set"),
            ratio(3, 5)
        );
        assert_eq!(
            model
                .some_scheduler_reachability_within(0, &targets, 2)
                .expect("valid initial state and target set"),
            ratio(9, 10)
        );
    }

    #[trace("FR-1051-AC-19")]
    #[test]
    fn finite_horizon_reward_uses_exact_transition_weights() {
        let model = TinyMdp::new(vec![
            vec![vec![(1, ratio(1, 2)), (2, ratio(1, 2))]],
            vec![vec![(1, ratio(1, 1))]],
            vec![vec![(2, ratio(1, 1))]],
        ])
        .expect("each exact support sums to one");
        // One step earns zero. On step two, state 1 earns 4 and state 2
        // earns 0, so the exact two-step expected reward is 1/2 * 4 = 2.
        let rewards = vec![vec![ratio(0, 1)], vec![ratio(4, 1)], vec![ratio(0, 1)]];
        assert_eq!(
            model
                .some_scheduler_expected_reward_within(0, &rewards, 2)
                .expect("finite exact reward is supported"),
            ratio(2, 1)
        );
    }

    #[trace("FR-1051-AC-19")]
    #[test]
    fn horizon_solver_refuses_steps_past_its_declared_bound() {
        let model = TinyMdp::new(vec![vec![vec![(0, ratio(1, 1))]]])
            .expect("the self-loop distribution is exact");
        let refusal = model
            .some_scheduler_reachability_within(0, &[false], super::MAX_HORIZON_STEPS + 1)
            .expect_err("the reference refuses horizons beyond its bound");
        assert_eq!(
            refusal.construct,
            "horizon exceeds the reference step bound"
        );
    }

    #[trace("FR-1051-AC-19")]
    #[test]
    fn quantile_refuses_some_but_preserves_every_scheduler_quantifiers() {
        use super::{validate_quantile_scheduler, ReferenceSchedulerQuantifier};

        assert_eq!(
            validate_quantile_scheduler(&ReferenceSchedulerQuantifier::Some)
                .expect_err("AC-19's some-scheduler quantile is refused")
                .construct,
            "quantile claim over some scheduler"
        );
        assert!(validate_quantile_scheduler(&ReferenceSchedulerQuantifier::Every).is_ok());
        assert!(
            validate_quantile_scheduler(&ReferenceSchedulerQuantifier::Workload("Even".into()))
                .is_err()
        );
    }
}

impl RewardEvaluator for ReferenceRewardEvaluator {
    fn evaluate(
        &self,
        _graph: &CheckedGraph,
        reward: &Reward,
        arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Value, RewardEvaluationFailure> {
        let expression = reward.expression();
        match expression.kind() {
            NodeKind::Literal(literal)
                if matches!(
                    &literal.0,
                    Value::Integer(_) | Value::Rational(_) | Value::Quantity(_)
                ) && arguments.len() == reward.parameters().len() =>
            {
                Ok(literal.0.clone())
            }
            _ => Err(RewardEvaluationFailure {
                diagnostic_code: Code::UnsupportedConstruct,
                catalog: CatalogCode::new("unsupported_construct", "not-yet-implemented"),
                fields: vec![("what", "reward outside literal rational fragment".into())],
            }),
        }
    }
}
