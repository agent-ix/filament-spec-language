// SPDX-License-Identifier: AGPL-3.0-or-later
//! Support for an exact, deliberately small semantic reference used by checker tests.
//!
//! This crate is a test oracle, never a verdict route. Its dependency surface
//! is restricted to checked packages and data crates so it cannot inherit a
//! producer, analyzer, or checker decision. The interpreter admits only
//! literal reward bodies today; unsupported expressions are refused rather
//! than delegated to the production evaluator.

#![forbid(unsafe_code)]

use fsl_semantics::check::{CheckedGraph, Reward, RewardEvaluationFailure, RewardEvaluator};
use filament_checked::node::NodeKind;
use filament_exact::{Meter, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;

/// Reference-owned evaluator supplied when package checking needs a reward
/// evaluator. It only admits reward bodies in the tiny literal fragment.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceRewardEvaluator;

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
                catalog: CatalogCode::new("reward", "unsupported_expression"),
                fields: vec![(
                    "construct",
                    "reward outside literal rational fragment or invalid arity".into(),
                )],
            }),
        }
    }
}
