// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked-claim validation for the FR-410 statistical result bound.

use serde_json::Value;

use super::VerdictBound;

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
    let object = bound
        .as_object()
        .ok_or(StatisticalBoundReadError::Malformed)?;
    let has_level = checked_claim.level.is_some();
    if object.len() != if has_level { 4 } else { 3 }
        || !object.get("type").is_some_and(Value::is_string)
        || !object.get("comparison").is_some_and(Value::is_string)
        || !object.get("threshold").is_some_and(valid_rational)
        || object
            .get("level")
            .is_some_and(|level| !valid_rational(level))
        || object.contains_key("level") != has_level
    {
        return Err(StatisticalBoundReadError::Malformed);
    }

    if bound != &crate::verdict_bound_wire(checked_claim) {
        return Err(StatisticalBoundReadError::Mismatch);
    }
    Ok(())
}

fn valid_rational(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 2 {
        return false;
    }
    let Some(numerator) = object.get("numerator").and_then(Value::as_str) else {
        return false;
    };
    let Some(denominator) = object.get("denominator").and_then(Value::as_str) else {
        return false;
    };
    let (Ok(numerator), Ok(denominator)) = (
        numerator.parse::<filament_exact::Integer>(),
        denominator.parse::<filament_exact::Integer>(),
    ) else {
        return false;
    };
    filament_exact::Rational::new(numerator, denominator).is_ok()
}
