// SPDX-License-Identifier: AGPL-3.0-or-later
//! S3's port to the one S6a reward expression evaluator (FR-185).

use std::string::String;
use std::vec::Vec;

use filament_exact::{Meter, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::diagnostic::Code;

use super::{CheckedGraph, Reward};

/// A kernel evaluation failure passed unchanged from S6a to S3.
///
/// Fields retain the evaluator's structured diagnostic payload. The reward
/// checker supplies the reward's source region when it turns this into an S3
/// refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RewardEvaluationFailure {
    /// The stable FSL diagnostic code associated with the kernel result.
    pub diagnostic_code: Code,
    /// The evaluator's original catalog code and cause.
    pub catalog: CatalogCode,
    /// The evaluator's original structured fields, in stable order.
    pub fields: Vec<(&'static str, String)>,
}

/// The S6a evaluator S3 uses to check a reward over its finite arguments.
///
/// This port keeps expression evaluation out of layer 3. Implementations
/// evaluate the checked reward body against `graph` and charge `meter` for
/// kernel work. Undefined and refused outcomes are returned as their original
/// diagnostic record, never as a value.
pub trait RewardEvaluator {
    /// Evaluate `reward` for one argument vector against the assembled graph.
    fn evaluate(
        &self,
        graph: &CheckedGraph,
        reward: &Reward,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Value, RewardEvaluationFailure>;
}
