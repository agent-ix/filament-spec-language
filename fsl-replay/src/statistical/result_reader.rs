// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked-claim validation for the FR-410 statistical result bound.

use serde_json::Value;

use super::VerdictBound;
use crate::shared_wire::ProbabilisticBound;

/// Why a statistical result's claim bound could not be trusted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StatisticalBoundReadError {
    /// The result has no `bound` member.
    #[error("the statistical result has no bound")]
    Missing,
    /// The `bound` member does not have the FR-413 shape or valid rational
    /// values.
    #[error("the statistical result bound is malformed")]
    Malformed,
    /// The well-formed bound differs from the checked claim.
    #[error("the statistical result bound differs from the checked claim")]
    Mismatch,
}

/// Verify a serialized FR-410 statistical result's `bound` against the
/// bound of the checked claim that requested it.
///
/// This is deliberately a validation step at the result-reading boundary:
/// a well-formed bound is still refused when it names another claim. The
/// input is the verdict object (the value containing `decision`, `bound`,
/// `basis`, and the remaining FR-410 members).
pub fn verify_statistical_result_bound(
    result: &Value,
    checked_claim: &VerdictBound,
) -> Result<(), StatisticalBoundReadError> {
    let bound = result
        .as_object()
        .ok_or(StatisticalBoundReadError::Malformed)?
        .get("bound")
        .ok_or(StatisticalBoundReadError::Missing)?;
    // A `level` on the wrong form is a shape fault, not another claim.
    if bound.get("level").is_some() != checked_claim.level.is_some() {
        return Err(StatisticalBoundReadError::Malformed);
    }
    let written =
        ProbabilisticBound::read(bound).map_err(|_| StatisticalBoundReadError::Malformed)?;
    if written != ProbabilisticBound::from(checked_claim) {
        return Err(StatisticalBoundReadError::Mismatch);
    }
    Ok(())
}
