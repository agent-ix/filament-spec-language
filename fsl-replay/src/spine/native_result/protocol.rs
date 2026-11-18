// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-24, FSpec FR-352 "Protocol-analysis result": the
//! `native-run-result/2` record of one local-projection or realizability
//! result, its writer and its strict reader.
//!
//! The record carries the result's key, its FR-360 label with the FR-241,
//! FR-242 and FR-243 axes, its FR-331 result entry and, on a decisive
//! counterexample, its FR-351 record (FR-574). It does not carry the
//! projected graph, the strategy table or the counterexample body. A request
//! refused before analysis and an engine failure return no result and so
//! have no record.

use filament_contract_model::CheckedNodeId;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{NativeRunReadError, read_typed};
use crate::model_check::settle::{Execution, ResultLabel, Truth};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
    UnavailabilityCause,
};
use crate::protocol_analysis::{
    self as analysis, NodeTextError, ProtocolAnalysisKey, ProtocolAnalysisSettlement,
    UnsupportedSlug, node_text, read_node_text,
};
use crate::result::{CauseCodecError, ProtocolWitnessWire, SeparatingWitnessRecord};

/// The FSpec FR-290 kind a protocol-analysis record reports. It is the kind
/// of the record's key, so the two cannot disagree in a written record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtocolAnalysisKind {
    /// `local-projection` (FSpec FR-174).
    LocalProjection,
    /// `realizability` (FSpec FR-175).
    Realizability,
}

impl ProtocolAnalysisKind {
    fn of(key: &ProtocolAnalysisKey) -> Self {
        match key {
            ProtocolAnalysisKey::Projection(_) => Self::LocalProjection,
            ProtocolAnalysisKey::Realizability(_) => Self::Realizability,
        }
    }
}

/// The proof basis of a protocol analysis's `proved` entry: FR-331's
/// `exhaustive`, the only basis either kind settles with (FR-573).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProtocolProofBasis {
    /// `exhaustive`.
    Exhaustive {},
}

/// A closed FR-331 inconclusive cause of a protocol analysis (FSpec FR-331
/// "Result entries", FSL FR-573).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProtocolInconclusiveCause {
    /// A horizon was reached with no counterexample; `depth` is the key's
    /// `max_depth`.
    BoundReached {
        /// The key's `max_depth`.
        depth: u64,
    },
    /// Realizability only: an enabled protocol activation reached the
    /// authored live-instance cap.
    InstanceBoundReached {
        /// The authored `max_live_instances` request bound.
        max_live_instances: u64,
    },
    /// The counterexample did not replay.
    ReplayDisagreed {},
    /// Realizability only: a premise the claim does not state.
    PremiseNotStated {},
    /// Realizability only: no refutation is forced.
    UnforcedRefutation {},
    /// Realizability only: the strategy table failed its replay walk.
    StrategyReplayFailed {},
    /// Realizability only: an open delivery range was cut.
    CutCount {},
    /// Realizability only: an evaluation was left undecided.
    UndecidedEvaluation {},
}

impl ProtocolInconclusiveCause {
    /// Whether the cause is admitted for `kind` (FSpec FR-331: the
    /// realizability-only causes are refused on another kind's entry).
    fn admitted_for(self, kind: ProtocolAnalysisKind) -> bool {
        match self {
            Self::BoundReached { .. } | Self::ReplayDisagreed {} => true,
            Self::PremiseNotStated {}
            | Self::UnforcedRefutation {}
            | Self::StrategyReplayFailed {}
            | Self::CutCount {}
            | Self::UndecidedEvaluation {}
            | Self::InstanceBoundReached { .. } => kind == ProtocolAnalysisKind::Realizability,
        }
    }

    fn of(cause: &analysis::InconclusiveCause, max_depth: u64) -> Self {
        match cause {
            analysis::InconclusiveCause::Horizon => Self::BoundReached { depth: max_depth },
            analysis::InconclusiveCause::PremiseNotStated { .. } => Self::PremiseNotStated {},
            analysis::InconclusiveCause::ReplayDisagreed => Self::ReplayDisagreed {},
            analysis::InconclusiveCause::Unforced => Self::UnforcedRefutation {},
            analysis::InconclusiveCause::StrategyReplayFailed => Self::StrategyReplayFailed {},
            analysis::InconclusiveCause::CutCount => Self::CutCount {},
            analysis::InconclusiveCause::Undecided => Self::UndecidedEvaluation {},
            analysis::InconclusiveCause::InstanceBoundReached { max_live_instances } => {
                Self::InstanceBoundReached {
                    max_live_instances: *max_live_instances,
                }
            }
        }
    }
}

/// The catalog code every FR-575 slug sits under.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UnsupportedCode {
    UnsupportedProjection,
}

/// The `detail` of an `unsupported` diagnostic (FSL FR-574 Outputs).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnsupportedDetail {
    /// The construct's checked node id, as its canonical text.
    node: String,
    /// The named policy of an `unsupported-join`, or the `fsl-eval` cause.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    reason: Option<String>,
}

/// The FR-272 diagnostic of an `unsupported` entry: an FR-575 slug under
/// `unsupported_projection`, naming the construct's node and, where the
/// slug has one, the named policy or the `fsl-eval` cause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedDiagnostic {
    code: UnsupportedCode,
    cause: UnsupportedSlug,
    detail: UnsupportedDetail,
}

impl UnsupportedDiagnostic {
    /// The FR-575 slug.
    #[must_use]
    pub fn slug(&self) -> UnsupportedSlug {
        self.cause
    }

    /// The construct's checked node id.
    ///
    /// # Errors
    ///
    /// [`NodeTextError`] for a node text that is not canonical, which the
    /// strict reader has already refused.
    pub fn node(&self) -> Result<CheckedNodeId, NodeTextError> {
        read_node_text(&self.detail.node)
    }

    /// The named policy of an `unsupported-join`, or the `fsl-eval` cause.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.detail.reason.as_deref()
    }
}

/// The `limit-reached` cause of a run a limit stopped: the limit, the value
/// in force and the request member that raises it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProtocolIncompleteCause {
    /// `limit-reached{limit, value, setting}`.
    LimitReached {
        /// The limit's name.
        limit: String,
        /// The value in force.
        value: u64,
        /// The request member that raises it.
        setting: String,
    },
}

/// The item's FSpec FR-331 result entry without its `request_index`,
/// `backend`, `kind` and `counterexample` members (FSpec FR-352
/// `settlement`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProtocolSettlementEntry {
    /// `proved`, with its basis and certification.
    Proved {
        /// The proof basis.
        basis: ProtocolProofBasis,
        /// The certification.
        certification: Certification,
    },
    /// `refuted`.
    Refuted {},
    /// `inconclusive`, with its cause.
    Inconclusive {
        /// The cause.
        cause: ProtocolInconclusiveCause,
    },
    /// `unsupported`, with its diagnostic.
    Unsupported {
        /// The diagnostic.
        diagnostic: UnsupportedDiagnostic,
    },
    /// `incomplete`, with its `limit-reached` cause.
    Incomplete {
        /// The cause.
        cause: ProtocolIncompleteCause,
    },
}

impl ProtocolSettlementEntry {
    fn of(value: &TerminalValue, max_depth: u64) -> Result<Self, ProtocolRecordError> {
        Ok(match value {
            TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification,
            } => Self::Proved {
                basis: ProtocolProofBasis::Exhaustive {},
                certification: *certification,
            },
            TerminalValue::Refuted => Self::Refuted {},
            TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth }) => {
                Self::Inconclusive {
                    cause: ProtocolInconclusiveCause::BoundReached { depth: *depth },
                }
            }
            TerminalValue::Inconclusive(InconclusiveCause::ProtocolAnalysis(cause)) => {
                Self::Inconclusive {
                    cause: ProtocolInconclusiveCause::of(cause, max_depth),
                }
            }
            TerminalValue::Unsupported(UnavailabilityCause::ProtocolAnalysis(cause)) => {
                Self::Unsupported {
                    diagnostic: UnsupportedDiagnostic {
                        code: UnsupportedCode::UnsupportedProjection,
                        cause: cause.slug,
                        detail: UnsupportedDetail {
                            node: node_text(&cause.node)?,
                            reason: cause.detail.clone(),
                        },
                    },
                }
            }
            TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }) => {
                Self::Incomplete {
                    cause: ProtocolIncompleteCause::LimitReached {
                        limit: limit.as_str().to_owned(),
                        value: *value,
                        setting: limit.setting().to_owned(),
                    },
                }
            }
            _ => return Err(ProtocolRecordError::NotProtocolSettlement),
        })
    }

    /// The FR-360 label this entry's `result` corresponds to.
    fn label(&self) -> ResultLabel {
        match self {
            Self::Proved { .. } => ResultLabel::Proved,
            Self::Refuted {} => ResultLabel::Refuted,
            Self::Inconclusive { .. } => ResultLabel::Inconclusive,
            Self::Unsupported { .. } => ResultLabel::Unsupported,
            Self::Incomplete { .. } => ResultLabel::Failed,
        }
    }
}

/// Why a protocol-analysis result was not written as a record.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolRecordError {
    /// The terminal value is not one a protocol analysis settles.
    #[error("the settlement is not a protocol-analysis settlement")]
    NotProtocolSettlement,
    /// The witness's presence disagrees with the basis.
    #[error("the separating witness record's presence disagrees with the basis")]
    BasisWitness,
    /// The record would not read back: the reader's refusal.
    #[error("the record does not read back: {0}")]
    Refused(Box<NativeRunReadError>),
    /// A node id does not encode.
    #[error(transparent)]
    Node(#[from] NodeTextError),
    /// The FR-351 record is not of the protocol family.
    #[error("invalid separating witness: {0}")]
    Witness(#[from] CauseCodecError),
    /// The record could not be represented as JSON.
    #[error("cannot serialize the protocol-analysis record: {0}")]
    Json(#[from] serde_json::Error),
    /// The canonical reader refused the producer document.
    #[error("cannot read the protocol-analysis record for canonical encoding: {0}")]
    Read(#[from] quire_canonical::ReadError),
    /// The canonical encoder could not write the record.
    #[error("cannot encode the protocol-analysis record canonically: {0}")]
    Canonical(#[from] quire_canonical::Error),
}

/// A FSpec FR-352 protocol-analysis record: the result of one
/// local-projection or realizability analysis. Writing one and reading it
/// back restores an equal record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAnalysisRecord {
    key: ProtocolAnalysisKey,
    label: ResultLabel,
    execution: Execution,
    truth: Truth,
    settlement: ProtocolSettlementEntry,
    basis: SettlementBasis,
    witness: Option<ProtocolWitnessWire>,
}

impl ProtocolAnalysisRecord {
    /// The record of a result whose key is `key` and whose FR-573
    /// settlement is `settlement`; `witness` is the counterexample's FR-351
    /// record, present exactly for a decisive basis.
    ///
    /// # Errors
    ///
    /// [`ProtocolRecordError`] when the settlement is not a protocol
    /// analysis's, the witness disagrees with the basis or is not of the
    /// protocol family, or the record breaks a rule the strict reader
    /// enforces.
    pub fn new(
        key: ProtocolAnalysisKey,
        settlement: &ProtocolAnalysisSettlement,
        witness: Option<&SeparatingWitnessRecord>,
    ) -> Result<Self, ProtocolRecordError> {
        if settlement.basis.is_decisive() != witness.is_some() {
            return Err(ProtocolRecordError::BasisWitness);
        }
        let entry = ProtocolSettlementEntry::of(&settlement.value, max_depth(&key))?;
        let record = Self {
            label: settlement.label,
            execution: settlement.execution,
            truth: settlement.truth,
            basis: settlement.basis,
            settlement: entry,
            witness: witness.map(ProtocolWitnessWire::of).transpose()?,
            key,
        };
        record
            .validate()
            .map_err(|refusal| ProtocolRecordError::Refused(Box::new(refusal)))?;
        Ok(record)
    }

    /// The analysis kind.
    #[must_use]
    pub fn kind(&self) -> ProtocolAnalysisKind {
        ProtocolAnalysisKind::of(&self.key)
    }

    /// The derived-artifact key.
    #[must_use]
    pub fn key(&self) -> &ProtocolAnalysisKey {
        &self.key
    }

    /// The FR-360 label.
    #[must_use]
    pub fn label(&self) -> ResultLabel {
        self.label
    }

    /// The FR-241 execution disposition.
    #[must_use]
    pub fn execution(&self) -> Execution {
        self.execution
    }

    /// The FR-242 truth disposition.
    #[must_use]
    pub fn truth(&self) -> Truth {
        self.truth
    }

    /// The FR-243 settlement basis.
    #[must_use]
    pub fn basis(&self) -> SettlementBasis {
        self.basis
    }

    /// The FR-331 result entry.
    #[must_use]
    pub fn settlement(&self) -> &ProtocolSettlementEntry {
        &self.settlement
    }

    /// Whether the record carries an FR-351 record.
    #[must_use]
    pub fn has_witness(&self) -> bool {
        self.witness.is_some()
    }

    /// The deciding node of the FR-351 record.
    #[must_use]
    pub fn deciding_node(&self) -> Option<CheckedNodeId> {
        self.witness
            .as_ref()
            .and_then(ProtocolWitnessWire::deciding_node)
    }

    /// The trace position of the FR-351 record.
    #[must_use]
    pub fn trace_position(&self) -> Option<u64> {
        self.witness
            .as_ref()
            .and_then(ProtocolWitnessWire::trace_position)
    }

    /// Whether the record's FR-351 record is `record` on the wire: the same
    /// deciding element, value path and trace position.
    #[must_use]
    pub fn witness_is(&self, record: &SeparatingWitnessRecord) -> bool {
        ProtocolWitnessWire::of(record).is_ok_and(|wire| self.witness.as_ref() == Some(&wire))
    }

    /// This record as RFC 8785 `native-run-result/2` bytes.
    ///
    /// # Errors
    ///
    /// [`ProtocolRecordError`] when the record does not encode, which no
    /// record built by [`Self::new`] or read by the strict reader does.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtocolRecordError> {
        let document = DocumentWire {
            format: RunResultFormat::V2,
            analysis: AnalysisWire {
                kind: self.kind(),
                key: RawValue::from_string(self.key.to_json()?)?,
                label: self.label,
                execution: self.execution,
                truth: self.truth,
                settlement: self.settlement.clone(),
            },
            basis: self.basis,
            witness: self.witness.clone(),
        };
        let json = serde_json::to_vec(&document)?;
        let parsed = quire_canonical::read(&json, u64::MAX)?;
        Ok(quire_canonical::to_vec(
            &parsed.root(),
            quire_canonical::Limits::new(u64::MAX),
        )?)
    }

    /// Read a protocol-analysis document whose identifier the caller has
    /// already checked, refusing with a located diagnostic naming the member
    /// (FSpec FR-352 "Protocol-analysis result").
    pub(super) fn read(bytes: &[u8]) -> Result<Self, NativeRunReadError> {
        let document: DocumentWire = read_typed(bytes)?;
        let analysis = document.analysis;
        let key = ProtocolAnalysisKey::from_json(analysis.key.get())
            .map_err(NativeRunReadError::AnalysisKey)?;
        if analysis.kind != ProtocolAnalysisKind::of(&key) {
            return Err(invalid("analysis.kind", "the kind is not the key's kind"));
        }
        let record = Self {
            key,
            label: analysis.label,
            execution: analysis.execution,
            truth: analysis.truth,
            settlement: analysis.settlement,
            basis: document.basis,
            witness: document.witness,
        };
        record.validate()?;
        Ok(record)
    }

    /// The rules of FSpec FR-352 "Protocol-analysis result" beyond the
    /// closed member shapes.
    fn validate(&self) -> Result<(), NativeRunReadError> {
        use Execution as E;
        use ResultLabel as L;
        use SettlementBasis as B;
        use Truth as T;
        match (self.label, self.execution, self.truth, self.basis) {
            (L::Failed, E::Failed, _, _) => {
                return Err(invalid(
                    "analysis.execution",
                    "an engine failure has no protocol-analysis record",
                ));
            }
            (L::Proved, E::Completed, T::Satisfied, B::ClosedScope)
            | (L::Refuted, E::Completed, T::Violated, B::DecisiveCounterexample)
            | (L::Inconclusive, E::Completed, T::Pending, B::Unsettled)
            | (L::Unsupported, E::Unsupported, T::Unavailable, B::Unavailable)
            | (L::Failed, E::ResourceIncomplete, T::Unavailable, B::Unavailable) => {}
            _ => {
                return Err(invalid(
                    "analysis.label",
                    "label, execution, truth and basis are not one FR-360 combination",
                ));
            }
        }
        if self.settlement.label() != self.label {
            return Err(invalid(
                "analysis.settlement.result",
                "the result does not correspond to the label",
            ));
        }
        match &self.settlement {
            ProtocolSettlementEntry::Inconclusive { cause } => {
                if !cause.admitted_for(self.kind()) {
                    return Err(invalid(
                        "analysis.settlement.cause",
                        "the cause is not admitted for the record's kind",
                    ));
                }
                if let ProtocolInconclusiveCause::BoundReached { depth } = cause {
                    if *depth != max_depth(&self.key) {
                        return Err(invalid(
                            "analysis.settlement.cause.depth",
                            "the depth is not the key's max_depth",
                        ));
                    }
                }
            }
            ProtocolSettlementEntry::Unsupported { diagnostic } => {
                read_node_text(&diagnostic.detail.node).map_err(|_| {
                    invalid(
                        "analysis.settlement.diagnostic.detail.node",
                        "the node is not a node id's canonical text",
                    )
                })?;
            }
            ProtocolSettlementEntry::Proved { .. }
            | ProtocolSettlementEntry::Refuted {}
            | ProtocolSettlementEntry::Incomplete { .. } => {}
        }
        match &self.witness {
            Some(witness) if self.basis.is_decisive() => witness.validate()?,
            None if !self.basis.is_decisive() => {}
            _ => return Err(invalid("witness", "presence disagrees with basis")),
        }
        Ok(())
    }
}

fn invalid(member: &'static str, reason: &'static str) -> NativeRunReadError {
    NativeRunReadError::Invalid { member, reason }
}

fn max_depth(key: &ProtocolAnalysisKey) -> u64 {
    match key {
        ProtocolAnalysisKey::Projection(key) => key.bounds.max_depth,
        ProtocolAnalysisKey::Realizability(key) => key.bounds.max_depth,
    }
}

/// A member present with a value; `null` is refused by the value's type.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// The one identifier of the wire, which the dispatching reader checked
/// before choosing this form.
#[derive(Serialize, Deserialize)]
enum RunResultFormat {
    #[serde(rename = "native-run-result/2")]
    V2,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentWire {
    format: RunResultFormat,
    analysis: AnalysisWire,
    basis: SettlementBasis,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    witness: Option<ProtocolWitnessWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalysisWire {
    kind: ProtocolAnalysisKind,
    key: Box<RawValue>,
    label: ResultLabel,
    execution: Execution,
    truth: Truth,
    settlement: ProtocolSettlementEntry,
}

#[cfg(test)]
mod tests;
