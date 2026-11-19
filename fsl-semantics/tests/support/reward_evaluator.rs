// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test-only evaluator for S3 fixtures with no reward-value assertion.

use filament_exact::{Integer, Meter, Value};
use fsl_semantics::check::CheckedGraph;
use fsl_semantics::check::{Reward, RewardEvaluationFailure, RewardEvaluator};

pub(crate) struct ZeroRewardEvaluator;

impl RewardEvaluator for ZeroRewardEvaluator {
    fn evaluate(
        &self,
        _graph: &CheckedGraph,
        _reward: &Reward,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Value, RewardEvaluationFailure> {
        Ok(Value::Integer(Integer::from(0_i64)))
    }
}
