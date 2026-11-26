// SPDX-License-Identifier: AGPL-3.0-or-later
//! Support for an exact, deliberately small semantic reference used by checker tests.
//!
//! This crate is a test oracle, never a verdict route. Its dependency surface
//! is restricted to checked packages and data crates so it cannot inherit a
//! producer, analyzer, or checker decision. The interpreter admits only
//! literal reward bodies today; unsupported expressions are refused rather
//! than delegated to the production evaluator.

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
    pub fn new(
        actions: Vec<Vec<Vec<(usize, filament_exact::Rational)>>>,
    ) -> Result<Self, ReferenceRefusal> {
        for state in &actions {
            if state.is_empty() {
                return Err(ReferenceRefusal {
                    construct: "state with no available action",
                });
            }
            for action in state {
                if action.is_empty() {
                    return Err(ReferenceRefusal {
                        construct: "empty probabilistic support",
                    });
                }
                let mut total =
                    filament_exact::Rational::from_integer(filament_exact::Integer::zero());
                for (destination, probability) in action {
                    if *destination >= actions.len() {
                        return Err(ReferenceRefusal {
                            construct: "transition destination outside finite state space",
                        });
                    }
                    if probability
                        < &filament_exact::Rational::from_integer(filament_exact::Integer::zero())
                    {
                        return Err(ReferenceRefusal {
                            construct: "negative probability weight",
                        });
                    }
                    total = total.add(probability);
                }
                if total != filament_exact::Rational::from_integer(filament_exact::Integer::one()) {
                    return Err(ReferenceRefusal {
                        construct: "probability weights do not sum to one",
                    });
                }
            }
        }
        if actions.is_empty() {
            return Err(ReferenceRefusal {
                construct: "empty finite state space",
            });
        }
        Ok(Self { actions })
    }

    /// Maximum probability of reaching a target within `horizon` steps,
    /// allowing deterministic scheduler choices to depend on both state and
    /// steps remaining. The recursion enumerates every available action at
    /// each `(state, remaining)` pair and keeps the best exact expectation.
    #[implements("FR-1051-AC-19")]
    pub fn some_scheduler_reachability_within(
        &self,
        initial: usize,
        targets: &[bool],
        horizon: usize,
    ) -> Result<filament_exact::Rational, ReferenceRefusal> {
        if initial >= self.actions.len() || targets.len() != self.actions.len() {
            return Err(ReferenceRefusal {
                construct: "initial state or target set outside finite state space",
            });
        }
        fn value(
            model: &TinyMdp,
            state: usize,
            targets: &[bool],
            remaining: usize,
        ) -> filament_exact::Rational {
            if targets[state] {
                return filament_exact::Rational::from_integer(filament_exact::Integer::one());
            }
            if remaining == 0 {
                return filament_exact::Rational::from_integer(filament_exact::Integer::zero());
            }
            model.actions[state]
                .iter()
                .map(|action| {
                    action.iter().fold(
                        filament_exact::Rational::from_integer(filament_exact::Integer::zero()),
                        |sum, (destination, probability)| {
                            sum.add(&probability.mul(&value(
                                model,
                                *destination,
                                targets,
                                remaining - 1,
                            )))
                        },
                    )
                })
                .max()
                .expect("construction requires at least one action per state")
        }
        Ok(value(self, initial, targets, horizon))
    }
}

/// Reference-owned evaluator supplied when package checking needs a reward
/// evaluator. It only admits reward bodies in the tiny literal fragment.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceRewardEvaluator;

/// A construct the tiny reference does not define. Callers must not attach a
/// truth value to a package refused with this error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceRefusal {
    /// The unsupported construct, in stable diagnostic wording.
    pub construct: &'static str,
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
                    construct: "disjunctive timed guard",
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
                    construct: "disjunctive operation guard",
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
                construct: "unbounded integer field",
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
                construct: "clock compared with a non-integer constant",
            });
        }
        if model
            .resets()
            .iter()
            .flat_map(|resets| resets.resets())
            .any(|reset| !reset.value().is_zero())
        {
            return Err(ReferenceRefusal {
                construct: "clock reset to a nonzero constant",
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
                construct: "hybrid mode with a non-constant rate",
            });
        }
    }
    for claim in graph.probabilistic_model().claims() {
        if matches!(&claim.form, ProbForm::Quantile { .. })
            && matches!(&claim.scheduler, ProbScheduler::Workload(_))
        {
            return Err(ReferenceRefusal {
                construct: "quantile claim over some scheduler",
            });
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
