// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750's in-process hybrid provider seam. The trait, the typed request and
//! the answer live beside [`SolverResult`] so a provider crate below layer A
//! implements [`HybridProvider`] without depending on `fsl-analyze`.

use std::collections::BTreeMap;
use std::fmt;

use filament_exact::Cancel;
use fsl_semantics::check::{CheckedHybridClaim, CheckedHybridModel};
use serde_json::Value;

use super::{SolverBudget, SolverCoverage, SolverMethod, SolverOutcome, SolverResult};
use crate::proof_result::{
    read_plugin_basis, read_plugin_inconclusive_cause_over, InconclusiveCause, PluginResultRefusal,
    ProofBasis,
};

/// One hybrid item's solve request (FR-750): the checked claim and the model
/// it ranges over, the resolved `hybrid.limits`, the FSpec FR-331 `options`
/// and the request's deadline.
#[derive(Clone, Debug)]
pub struct HybridSolveRequest<'a> {
    /// The checked claim.
    pub claim: &'a CheckedHybridClaim,
    /// The model the claim's `over` parameter names.
    pub model: &'a CheckedHybridModel,
    /// The limits the solver runs under: each member the request left unset
    /// already takes the serving descriptor's published bound.
    pub budget: SolverBudget,
    /// The item's FSpec FR-331 `options`, each a string; an absent option
    /// takes the descriptor's published default.
    pub options: BTreeMap<String, String>,
    /// The request's deadline in milliseconds.
    pub deadline_ms: u64,
}

/// What a provider returns for one request: FR-250's solver result and, when
/// the provider returned one, its FSpec FR-652 certificate as the entry wrote
/// it, for FR-751's check.
#[derive(Clone, Debug, PartialEq)]
pub struct HybridAnswer {
    /// The provider's result.
    pub result: SolverResult,
    /// The certificate member of the provider's result entry, unchecked.
    pub certificate: Option<Value>,
}

/// A provider that decides hybrid items in process (FR-750). It reads the
/// request and the cancellation token and returns an answer; FSL settles the
/// answer by FR-250 and replays its traces by FR-450.
pub trait HybridProvider: fmt::Debug + Send + Sync {
    /// Decide one request.
    fn solve(&self, request: &HybridSolveRequest<'_>, cancel: &Cancel) -> HybridAnswer;
}

/// Why a hybrid result entry was not read into an answer.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HybridEntryRefusal {
    /// FSpec FR-331 refuses the entry at this pointer.
    #[error("the hybrid result entry was refused: {0:?}")]
    Entry(PluginResultRefusal),
    /// The entry's `result` is not one this reader turns into a
    /// [`SolverResult`].
    #[error("a `{0}` hybrid result entry is not read into a solver result")]
    NotRead(String),
}

impl HybridAnswer {
    /// Read one FSpec FR-331 hybrid result entry into an answer: a `proved`
    /// entry by its `basis`, an `inconclusive` one by its `cause` (a
    /// `solver-inconclusive` trace is read against `model`), and the entry's
    /// `certificate`, kept as written. The member names are the wire's, so
    /// this reader is a string edge.
    ///
    /// # Errors
    ///
    /// [`HybridEntryRefusal::Entry`] for an entry FSpec FR-331 refuses, with
    /// its cause and pointer; [`HybridEntryRefusal::NotRead`] for a `result`
    /// that is not turned into a solver result.
    #[ix_trace_rs::string_edge]
    pub fn read_entry(
        entry: &Value,
        model: &CheckedHybridModel,
    ) -> Result<Self, HybridEntryRefusal> {
        let result = entry.get("result").and_then(Value::as_str).unwrap_or("");
        let solver = match result {
            "proved" => {
                let basis = entry.get("basis").ok_or_else(|| {
                    HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
                        pointer: "/basis".to_owned(),
                    })
                })?;
                match read_plugin_basis(basis).map_err(HybridEntryRefusal::Entry)? {
                    ProofBasis::BoundedSolver {
                        method,
                        horizon,
                        jumps,
                        exhaustive,
                    } => SolverResult {
                        method,
                        outcome: if method == SolverMethod::DeltaComplete {
                            SolverOutcome::Unsat
                        } else {
                            SolverOutcome::EnclosureDisjoint
                        },
                        coverage: SolverCoverage::Bounded {
                            horizon,
                            jumps,
                            exhaustive,
                        },
                    },
                    ProofBasis::InductiveSolver { method } => SolverResult {
                        method,
                        outcome: SolverOutcome::EnclosureDisjoint,
                        coverage: SolverCoverage::Inductive,
                    },
                    _ => return Err(HybridEntryRefusal::NotRead("proved".to_owned())),
                }
            }
            "inconclusive" => {
                let cause = entry.get("cause").ok_or_else(|| {
                    HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
                        pointer: "/cause".to_owned(),
                    })
                })?;
                let outcome = match read_plugin_inconclusive_cause_over(cause, Some(model))
                    .map_err(HybridEntryRefusal::Entry)?
                {
                    InconclusiveCause::NoInitialState => SolverOutcome::NoInitialState,
                    InconclusiveCause::SolverInconclusive { trace, .. } => {
                        SolverOutcome::Unknown(trace)
                    }
                    _ => SolverOutcome::Unknown(None),
                };
                SolverResult {
                    method: SolverMethod::Unclassified,
                    outcome,
                    coverage: SolverCoverage::Inductive,
                }
            }
            other => return Err(HybridEntryRefusal::NotRead(other.to_owned())),
        };
        Ok(Self {
            result: solver,
            certificate: entry.get("certificate").cloned(),
        })
    }
}
