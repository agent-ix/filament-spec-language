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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NotDigitisable {
    /// The rule the declined part breaks.
    pub rule: NotDigitisableRule,
    /// The byte range, in the claim's source unit, of the declined part.
    pub span: Span,
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
