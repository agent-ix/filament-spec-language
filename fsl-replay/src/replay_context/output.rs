// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fresh native replay outcomes. Producer history is never used as evidence.

use filament_semantic_value::catalog::Category;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::Value;

use super::ReplayFamily;
use crate::{
    CounterexampleKind, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceValue, Operation,
    OutcomeDocument, OutcomeStage, ReplayResult,
};

/// Failure to encode a fresh native result; no partial reproduction is output.
#[derive(Debug, thiserror::Error)]
pub enum ContextOutputError {
    /// The existing native replay result codec refused a value.
    #[error(transparent)]
    Native(#[from] crate::CauseCodecError),
    /// JSON output could not be written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A native result and its owning family cannot be represented together.
    #[error("native replay output invariant failed: {field}")]
    Invariant {
        /// The mismatched native result member.
        field: &'static str,
    },
}

/// One freshly executed native replay outcome, ready for the external driver.
#[derive(Debug)]
pub struct NativeReplayOutput {
    document: Box<RawValue>,
    category: Category,
    outcome: OutcomeDocument,
}

#[derive(Serialize)]
struct TemporalRow {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    value: Option<bool>,
    trace_position: Option<String>,
    undefined: Option<Value>,
    disagreement: Option<Value>,
    limit: Option<Limit>,
}

#[derive(Serialize)]
struct Limit {
    limit: &'static str,
    value: String,
    count: Option<String>,
}

impl NativeReplayOutput {
    /// The raw fresh `filament-outcome/1` document.
    pub fn as_raw(&self) -> &RawValue {
        &self.document
    }

    /// Native category of this independent execution.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The actual typed fresh native result, including its replay row.
    pub fn outcome(&self) -> &OutcomeDocument {
        &self.outcome
    }

    /// Transfer the typed fresh native document to the command dispatcher.
    pub fn into_outcome(self) -> OutcomeDocument {
        self.outcome
    }

    /// Transfer the fresh output without a parse/re-emit step.
    pub fn into_raw(self) -> Box<RawValue> {
        self.document
    }

    pub(super) fn hybrid(result: ReplayResult) -> Result<Self, ContextOutputError> {
        if !matches!(&result, ReplayResult::Hybrid(_)) {
            return Err(ContextOutputError::Invariant {
                field: "hybrid-arm",
            });
        }
        let category = result.category();
        let row: Value = serde_json::from_str(&result.to_function_json()?)?;
        let document = OutcomeDocument::from_replay_with_row(&result, row);
        Ok(Self {
            document: serde_json::value::to_raw_value(&document)?,
            category,
            outcome: document,
        })
    }

    pub(super) fn temporal(
        family: ReplayFamily,
        result: &ModelTraceReplayResult,
        original_kind: &CounterexampleKind,
        original_kind_wire: &Value,
    ) -> Result<Self, ContextOutputError> {
        let arm = match family {
            ReplayFamily::ModelTemporal => "model-trace",
            ReplayFamily::ProtocolTemporal => "protocol-trace",
            ReplayFamily::Hybrid => {
                return Err(ContextOutputError::Invariant {
                    field: "temporal-arm",
                })
            }
        };
        let (category, row) = match &result.settlement {
            ModelTraceSettlement::Reproduced {
                value,
                trace_position,
            } => {
                let (value, undefined) = match value {
                    ModelTraceValue::Truth(value) => (Some(*value), None),
                    ModelTraceValue::Undefined(found) => {
                        let CounterexampleKind::UndefinedEvaluation(original) = original_kind
                        else {
                            return Err(ContextOutputError::Invariant {
                                field: "undefined-kind",
                            });
                        };
                        if found != original {
                            return Err(ContextOutputError::Invariant {
                                field: "undefined-evaluation",
                            });
                        }
                        // The owning family codec supplied this form, and the
                        // fresh native replay just established typed equality.
                        (None, Some(original_kind_wire.clone()))
                    }
                    ModelTraceValue::HyperUndefined(_) => {
                        return Err(ContextOutputError::Invariant {
                            field: "hyper-undefined-in-temporal-arm",
                        })
                    }
                };
                (
                    Category::Success,
                    TemporalRow {
                        arm,
                        settlement: "reproduced-with-evaluated-witness",
                        category: "success",
                        value,
                        trace_position: trace_position.map(|position| position.to_string()),
                        undefined,
                        disagreement: None,
                        limit: None,
                    },
                )
            }
            ModelTraceSettlement::Inconclusive(cause) => (
                Category::Inconclusive,
                TemporalRow {
                    arm,
                    settlement: "inconclusive",
                    category: "inconclusive",
                    value: None,
                    trace_position: None,
                    undefined: None,
                    disagreement: Some(serde_json::from_str(&cause.to_json(None)?)?),
                    limit: None,
                },
            ),
            ModelTraceSettlement::Stopped { reached, count } => (
                Category::Incomplete,
                TemporalRow {
                    arm,
                    settlement: "incomplete",
                    category: "incomplete",
                    value: None,
                    trace_position: None,
                    undefined: None,
                    disagreement: None,
                    limit: Some(Limit {
                        limit: reached.limit.as_str(),
                        value: reached.value.to_string(),
                        count: count.map(|count| count.to_string()),
                    }),
                },
            ),
        };
        let row = serde_json::to_value(row)?;
        let outcome = OutcomeDocument::new(Operation::Replay, Some(OutcomeStage::S8), category)
            .with_replay_row(row);
        let document = serde_json::value::to_raw_value(&outcome)?;
        Ok(Self {
            document,
            category,
            outcome,
        })
    }
}
