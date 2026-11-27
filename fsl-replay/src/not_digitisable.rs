// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-272's `not_digitisable` payload member, the one type the
//! digital-clock route's writer (FSL FR-243) and the strict result-entry
//! reader ([`crate::planner::read_result_entry`]) share. It travels in the
//! `detail` of an `invalid_runtime_input`/`invalid-value` diagnostic and on
//! no other.

use fsl_foundation::diagnostic::CatalogCode;
use fsl_foundation::Span;
use fsl_semantics::check::{CheckedStateGraphClaim, StateGraphForm};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};

/// Why digitisation does not preserve an item's verdict: FSL FR-243's closed
/// set of `rule` spellings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotDigitisableRule {
    /// (b): an atom reads a clock.
    ClockAtom,
    /// (c): a second `eventually`, `until`, `once` or `since` carries an
    /// interval other than `[0, *]`.
    SecondBoundedOperator,
    /// (d): such an interval has a nonzero lower bound and an upper bound.
    TwoSidedInterval,
    /// (e): such an operator is nested where its position is not fixed.
    NestedBoundedOperator,
    /// (f): a fairness constraint over a subject with a clock constraint.
    FairnessOverClockConstraint,
    /// An `always possible` state-graph item.
    AlwaysPossible,
    /// A `unique path` state-graph item.
    UniquePath,
}

/// The `not_digitisable` member `{rule, span: {start, end}}`.
///
/// Reading refuses a span whose `start` exceeds its `end`: such a range names
/// no part (FSpec FR-272), so no read value carries one.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, try_from = "WrittenNotDigitisable")]
pub struct NotDigitisable {
    /// The rule the declined part breaks.
    pub rule: NotDigitisableRule,
    /// The byte range, in the claim's source unit, of the declined part.
    pub span: Span,
}

/// The member as written, before its span is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenNotDigitisable {
    rule: NotDigitisableRule,
    span: Span,
}

impl TryFrom<WrittenNotDigitisable> for NotDigitisable {
    type Error = String;

    fn try_from(written: WrittenNotDigitisable) -> Result<Self, String> {
        if written.span.start > written.span.end {
            return Err(format!(
                "span start {} is after its end {}",
                written.span.start, written.span.end
            ));
        }
        Ok(Self {
            rule: written.rule,
            span: written.span,
        })
    }
}

impl NotDigitisable {
    /// The member's FSpec FR-272 payload name.
    pub const MEMBER: &'static str = "not_digitisable";
    /// The one code and cause whose detail carries the member.
    pub const CATALOG: CatalogCode = CatalogCode::new("invalid_runtime_input", "invalid-value");

    /// The state-graph half of FSL FR-243's rule, read by the digital-clock
    /// route and by its replay: `always possible` and `unique path` quantify
    /// over every reachable timed state, which an integer-clock reading does
    /// not visit. `possible` is preserved.
    #[must_use]
    #[implements("FR-243-AC-6")]
    pub fn of_state_graph(claim: &CheckedStateGraphClaim) -> Option<Self> {
        let rule = match claim.form() {
            StateGraphForm::Possible { .. } => return None,
            StateGraphForm::AlwaysPossible { .. } => NotDigitisableRule::AlwaysPossible,
            StateGraphForm::UniquePath { .. } => NotDigitisableRule::UniquePath,
        };
        Some(Self {
            rule,
            span: claim.span(),
        })
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::NotDigitisable;

    #[trace("TC-2290", "FR-243-AC-5")]
    #[test]
    fn a_span_ending_before_it_starts_is_not_read() {
        let read = |start: u32, end: u32| {
            serde_json::from_value::<NotDigitisable>(
                json!({"rule": "two-sided-interval", "span": {"start": start, "end": end}}),
            )
        };
        assert!(read(3, 3).is_ok());
        assert!(read(3, 9).is_ok());
        assert!(read(9, 3).is_err());
    }
}
