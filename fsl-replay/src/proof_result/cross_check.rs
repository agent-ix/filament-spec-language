// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-290's SMT row: settle the SMT backend's result beside its independent
//! cross-check, row for row with FSpec FR-349's cross-check table. The
//! backend runs both solvers; this module settles what they reported.

use fsl_foundation::{BackendId, RequestIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Certification, InconclusiveCause, ProofBasis, TerminalRecord, TerminalValue};
use crate::SmtProofSettlement;

/// A solver's result for the shared SMT query, in FSpec FR-331's spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CrossCheckVerdict {
    /// The query's claim holds.
    Proved,
    /// A counterexample exists.
    Refuted,
    /// The solver did not decide.
    Inconclusive,
}

/// The independent solver's run on the same query: FSpec FR-331's
/// `cross_check` member of a `proved` result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossCheck {
    /// The cross-checking tool, for example `z3`.
    #[serde(deserialize_with = "tool_name")]
    pub tool: String,
    /// The result the tool's run alone settles.
    pub verdict: CrossCheckVerdict,
}

/// What the SMT backend reports about its cross-check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrossCheckRun {
    /// The cross-check ran.
    Ran(CrossCheck),
    /// No cross-check ran: FSpec FR-349's `cross-check-absent{tool}` warning.
    Absent {
        /// The cross-checking tool that did not run.
        tool: String,
    },
}

/// A warning a terminal record keeps, in FSpec FR-331's `Warning` spelling.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", deny_unknown_fields)]
pub enum RecordedWarning {
    /// FSpec FR-349: the SMT result was settled with no cross-check.
    #[serde(rename = "cross-check-absent")]
    CrossCheckAbsent {
        /// The cross-checking tool that did not run.
        #[serde(deserialize_with = "tool_name")]
        tool: String,
    },
}

/// Why a cross-check could not be settled against a record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CrossCheckRefusal {
    /// The report names no tool.
    #[error("the cross-check names no tool")]
    EmptyTool,
    /// FSpec FR-331 gives `cross_check` to no such proof: it is `trusted`, or
    /// its basis is neither `bounded-complete` nor `inductive`.
    #[error("the proof is not an SMT proof a cross-check applies to")]
    NotAnSmtProof,
    /// The cross-check table has no row for a verdict beside a result other
    /// than `proved`, `refuted` or `inconclusive`.
    #[error("the cross-check table has no row for this result")]
    NoTableRow,
}

fn tool_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let tool = String::deserialize(deserializer)?;
    if tool.is_empty() {
        return Err(serde::de::Error::custom("the cross-check names no tool"));
    }
    Ok(tool)
}

impl InconclusiveCause {
    /// Read a serialized `cross-check-disagreement{tool, primary,
    /// cross_check}` cause (FR-286, `kind` spelling) back into its variant.
    /// `None` for another `kind`, a missing or unknown member, an empty
    /// `tool` or a result outside FSpec FR-331's three.
    pub fn read_cross_check_disagreement(cause: &Value) -> Option<Self> {
        #[derive(Deserialize)]
        enum Kind {
            #[serde(rename = "cross-check-disagreement")]
            CrossCheckDisagreement,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(rename = "kind")]
            _kind: Kind,
            #[serde(deserialize_with = "tool_name")]
            tool: String,
            primary: CrossCheckVerdict,
            cross_check: CrossCheckVerdict,
        }
        let Wire {
            tool,
            primary,
            cross_check,
            ..
        } = Wire::deserialize(cause).ok()?;
        Some(Self::CrossCheckDisagreement {
            tool,
            primary,
            cross_check,
        })
    }
}

impl TerminalRecord {
    /// The record of the SMT backend's proof settlement: its value, with the
    /// certification FR-314's check alone gave it, under `backend`.
    pub fn from_smt_settlement(
        request_index: RequestIndex,
        backend: BackendId,
        settlement: &SmtProofSettlement,
    ) -> Self {
        Self::with_backend(request_index, backend, settlement.value().clone())
    }

    /// This record, the SMT backend's own settlement of one item, settled
    /// beside its cross-check by FR-290's table.
    ///
    /// With no cross-check the value stands and the record keeps the absence
    /// warning. A certified proof stands whatever the verdict and records it;
    /// an uncertified proof stands only on `proved`. A refutation or an
    /// inconclusive result stands only on the same verdict, with no
    /// `cross_check`. Every other pair is `inconclusive` with
    /// [`InconclusiveCause::CrossCheckDisagreement`], and carries none of the
    /// primary result's evidence.
    ///
    /// # Errors
    ///
    /// [`CrossCheckRefusal`] for an unnamed tool, for a verdict beside a
    /// proof FSpec FR-331 gives no `cross_check`, and for a verdict beside a
    /// result the table has no row for.
    pub fn with_cross_check(mut self, run: CrossCheckRun) -> Result<Self, CrossCheckRefusal> {
        let (CrossCheckRun::Ran(CrossCheck { tool, .. }) | CrossCheckRun::Absent { tool }) = &run;
        if tool.is_empty() {
            return Err(CrossCheckRefusal::EmptyTool);
        }
        let check = match run {
            CrossCheckRun::Ran(check) => check,
            CrossCheckRun::Absent { tool } => {
                self.cross_check = Some(CrossCheckRun::Absent { tool });
                return Ok(self);
            }
        };
        let primary = match &self.value {
            TerminalValue::Proved {
                basis: ProofBasis::BoundedComplete { .. } | ProofBasis::Inductive { .. },
                certification: Certification::Certified | Certification::Uncertified,
            } => CrossCheckVerdict::Proved,
            TerminalValue::Proved { .. } => return Err(CrossCheckRefusal::NotAnSmtProof),
            TerminalValue::Refuted => CrossCheckVerdict::Refuted,
            TerminalValue::Inconclusive(_) => CrossCheckVerdict::Inconclusive,
            TerminalValue::Tested
            | TerminalValue::Measured(_)
            | TerminalValue::Declined { .. }
            | TerminalValue::Unsupported(_)
            | TerminalValue::Incomplete(_)
            | TerminalValue::Failed
            | TerminalValue::ToolFailed(_) => return Err(CrossCheckRefusal::NoTableRow),
        };
        let certified = matches!(
            self.value,
            TerminalValue::Proved {
                certification: Certification::Certified,
                ..
            }
        );
        if certified || check.verdict == primary {
            if primary == CrossCheckVerdict::Proved {
                self.cross_check = Some(CrossCheckRun::Ran(check));
            }
            return Ok(self);
        }
        let disagreement = TerminalValue::Inconclusive(InconclusiveCause::CrossCheckDisagreement {
            tool: check.tool,
            primary,
            cross_check: check.verdict,
        });
        let mut record = Self::from_value(self.request_index, disagreement);
        record.backend = self.backend;
        Ok(record)
    }

    /// The independent solver's result a `proved` record holds.
    pub fn cross_check(&self) -> Option<&CrossCheck> {
        match &self.cross_check {
            Some(CrossCheckRun::Ran(check)) => Some(check),
            Some(CrossCheckRun::Absent { .. }) | None => None,
        }
    }

    /// The warning of a record settled with no cross-check.
    pub fn cross_check_warning(&self) -> Option<RecordedWarning> {
        match &self.cross_check {
            Some(CrossCheckRun::Absent { tool }) => {
                Some(RecordedWarning::CrossCheckAbsent { tool: tool.clone() })
            }
            Some(CrossCheckRun::Ran(_)) | None => None,
        }
    }

    /// The bytes the record's cross-check report retains.
    pub(super) fn cross_check_bytes(&self) -> usize {
        match &self.cross_check {
            Some(CrossCheckRun::Ran(CrossCheck { tool, .. }) | CrossCheckRun::Absent { tool }) => {
                tool.len()
            }
            None => 0,
        }
    }
}
