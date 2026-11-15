// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-622 (FSpec FR-521): the four independent outcome axes of a temporal
//! outcome, beside its one category. Activation, execution, truth and
//! settlement are separate facts a consumer acts on separately, so every
//! outcome that has them carries all four in one `axes` object.

use serde::{Deserialize, Serialize};

use crate::{SettlementBasis, TerminalValue};

/// Whether the obligation was activated (FR-516 `activation`).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationAxis {
    /// No trigger was admitted within a closed, complete scope.
    Inactive,
    /// The scope was open, or its trigger evidence missing or refused.
    Unknown,
    /// A trigger or the origin activated the obligation.
    Active,
    /// A fact that decides activation was not available.
    Unavailable,
}

/// How the surrounding execution ended (FR-516 `execution`).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionAxis {
    /// The execution completed.
    Completed,
    /// The execution failed or was stopped.
    Failed,
}

/// The truth value of the outcome (FR-516 `truth`).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TruthAxis {
    /// The obligation holds.
    True,
    /// The obligation does not hold.
    False,
    /// Declared progress could still resolve it.
    Pending,
}

/// The four outcome axes (FR-622): each member is decided by its own
/// inputs, and changing the input of one leaves the other three as they
/// were. `null` states that no value applies: no execution ran, no truth was
/// settled, or no settlement exists.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeAxes {
    /// The activation state; never `null`.
    pub activation: ActivationAxis,
    /// The execution disposition, `null` when no execution ran.
    #[serde(deserialize_with = "present")]
    pub execution: Option<ExecutionAxis>,
    /// The truth value, `null` when none was settled.
    #[serde(deserialize_with = "present")]
    pub truth: Option<TruthAxis>,
    /// What the truth rests on, `null` when nothing settled.
    #[serde(deserialize_with = "present")]
    pub settlement: Option<SettlementBasis>,
}

/// A member that must be written, possibly as `null`.
fn present<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(decoder)
}

impl OutcomeAxes {
    /// The axes FR-622 maps an analyze item's terminal record to. A record
    /// settled by a negotiation disposition is `unavailable` on every axis,
    /// as an `unsupported`, `declined` or `failed` record is.
    pub fn of_record(value: &TerminalValue, disposed: bool) -> Self {
        let unavailable = Self {
            activation: ActivationAxis::Unavailable,
            execution: None,
            truth: None,
            settlement: Some(SettlementBasis::Unavailable),
        };
        if disposed {
            return unavailable;
        }
        let active = |execution, truth, settlement| Self {
            activation: ActivationAxis::Active,
            execution: Some(execution),
            truth,
            settlement: Some(settlement),
        };
        match value {
            TerminalValue::Proved { .. } => active(
                ExecutionAxis::Completed,
                Some(TruthAxis::True),
                SettlementBasis::ClosedScope,
            ),
            TerminalValue::Refuted => active(
                ExecutionAxis::Completed,
                Some(TruthAxis::False),
                SettlementBasis::DecisiveCounterexample,
            ),
            TerminalValue::Inconclusive(_) => active(
                ExecutionAxis::Completed,
                None,
                SettlementBasis::Unsettled,
            ),
            TerminalValue::Incomplete(_) => {
                active(ExecutionAxis::Failed, None, SettlementBasis::Unsettled)
            }
            TerminalValue::Tested
            | TerminalValue::Measured(_)
            | TerminalValue::Declined { .. }
            | TerminalValue::Unsupported(_)
            | TerminalValue::Failed
            | TerminalValue::ToolFailed(_) => unavailable,
        }
    }
}
