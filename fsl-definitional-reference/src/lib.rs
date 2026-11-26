// SPDX-License-Identifier: AGPL-3.0-or-later
//! Support for an exact, deliberately small semantic reference used by checker tests.
//!
//! This crate is a test oracle, never a verdict route. Its dependency surface
//! is restricted to checked packages and data crates so it cannot inherit a
//! producer, analyzer, or checker decision. The interpreter admits only
//! literal reward bodies today; unsupported expressions are refused rather
//! than delegated to the production evaluator.

#![forbid(unsafe_code)]

use fsl_semantics::check::{
    CheckedGraph, Reward, RewardEvaluationFailure, RewardEvaluator, StateClauseKind,
};
use filament_checked::node::NodeKind;
use filament_exact::{Meter, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;
use ix_trace_rs::implements;

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
#[implements("FR-1051-AC-14")]
pub fn validate_conjunctive_guards(graph: &CheckedGraph) -> Result<(), ReferenceRefusal> {
    for model in graph.timed_models() {
        if model.guards().iter().any(|guard| guard.parts().len() > 1) {
            return Err(ReferenceRefusal {
                construct: "disjunctive timed guard",
            });
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
