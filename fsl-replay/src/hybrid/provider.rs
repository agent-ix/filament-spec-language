// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750's in-process hybrid provider seam. The trait, the typed request and
//! the answer live beside [`SolverResult`] so a provider crate below layer A
//! implements [`HybridProvider`] without depending on `fsl-analyze`.

use std::collections::BTreeMap;
use std::fmt;

use filament_exact::Cancel;
use fsl_semantics::check::{CheckedHybridClaim, CheckedHybridModel, ModeIndex, VariableIndex};
use serde_json::Value;

use super::{
    SolverBudget, SolverCoverage, SolverMethod, SolverOutcome, SolverResult, SolverStop,
};
use crate::hybrid_replay::{ExpressionSite, HybridCounterexample, TermLocus};
use crate::proof_result::{
    read_plugin_basis, read_plugin_hybrid_counterexample, read_plugin_inconclusive_cause_over,
    InconclusiveCause, PluginResultRefusal, ProofBasis, ValueFault,
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
    /// The request's own deadline in milliseconds (`AnalyzeLimits.deadline`),
    /// `None` when the caller supplied none.
    pub deadline_ms: Option<u64>,
}

/// What a provider returns for one request (FR-750): its verdict and, when
/// the provider returned one, its FSpec FR-652 certificate as the entry wrote
/// it. FR-751's checker is a later slice, so the certificate is carried
/// unchecked and the item settles `uncertified`.
#[derive(Clone, Debug, PartialEq)]
pub struct HybridAnswer {
    /// What the provider answered.
    pub verdict: HybridVerdict,
    /// The certificate member of the provider's result entry, unchecked.
    pub certificate: Option<Value>,
}

/// What a hybrid provider can answer, one arm per settlement path, so no arm
/// stands in for another (FR-750).
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)] // a verdict is built once per item and moved, never collected
pub enum HybridVerdict {
    /// Proved, inconclusive or incomplete: settled by FR-250's map.
    Solved(SolverResult),
    /// A claimed refutation: settled only by FR-450's exact replay. A
    /// claimed refutation is never folded into [`SolverOutcome::Unknown`],
    /// which FR-250 settles `solver-inconclusive`, because replay parity
    /// needs the claim.
    Refuted(HybridCounterexample),
    /// A construct outside the provider's class.
    Unsupported(UnsupportedConstruct),
}

/// A construct of the model the provider's method does not decide (FR-750,
/// FSpec FR-193 "Solver result on the wire").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedConstruct {
    /// The recognized construct, as FSpec FR-272 names it.
    pub form: String,
    /// The flow, guard, reset, invariant or initial entry holding it.
    pub site: ExpressionSite,
    /// The construct within that expression.
    pub term: TermLocus,
    /// The mode of the model the construct is in.
    pub mode: ModeIndex,
    /// For a flow, the variable whose rate it is.
    pub variable: Option<VariableIndex>,
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
    /// An answer that carries no certificate.
    pub const fn solved(result: SolverResult) -> Self {
        Self {
            verdict: HybridVerdict::Solved(result),
            certificate: None,
        }
    }

    /// Read one FSpec FR-331 hybrid result entry into an answer, refusing an
    /// entry FSpec FR-331 refuses with that cause and pointer (FR-750):
    ///
    /// - `proved` into [`HybridVerdict::Solved`] by its `basis`;
    /// - `inconclusive` into `Solved` by its `cause` (`no-initial-state`,
    ///   `solver-inconclusive` with its trace read against `model`), and
    ///   `incomplete` by `timed-out` or a `max_solver_steps` `limit-reached`;
    /// - `refuted` into [`HybridVerdict::Refuted`]: its `counterexample` is a
    ///   position in `counterexamples`, the entry it names read against
    ///   `model`, and a position outside the array refuses
    ///   `invalid_result`/`invalid-value` at `/counterexample`.
    ///
    /// Every other admitted entry returns [`HybridEntryRefusal::NotRead`],
    /// and the item takes FR-290's settlement of that entry unchanged: a
    /// `failed` `tool-failure`, an `unsupported` entry, and every
    /// `inconclusive` or `incomplete` cause a [`SolverResult`] cannot hold
    /// (`insufficient-horizon`, `bound-reached`, ...), so no cause is folded
    /// into another. An `unsupported` `expression-form` entry is among them
    /// until the reader holds the checked graph that turns its `locus` back
    /// into an [`ExpressionSite`] and [`TermLocus`]. The member names are the
    /// wire's, so this reader is a string edge.
    ///
    /// # Errors
    ///
    /// [`HybridEntryRefusal::Entry`] for an entry FSpec FR-331 refuses, with
    /// its cause and pointer; [`HybridEntryRefusal::NotRead`] for an entry
    /// that is not a solver answer.
    #[ix_trace_rs::string_edge]
    pub fn read_entry(
        entry: &Value,
        counterexamples: &[Value],
        model: &CheckedHybridModel,
    ) -> Result<Self, HybridEntryRefusal> {
        let missing = |pointer: &str| {
            HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
                pointer: pointer.to_owned(),
            })
        };
        let result = entry.get("result").and_then(Value::as_str).unwrap_or("");
        let verdict = match result {
            "proved" => {
                let basis = entry.get("basis").ok_or_else(|| missing("/basis"))?;
                HybridVerdict::Solved(match read_plugin_basis(basis).map_err(HybridEntryRefusal::Entry)? {
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
                })
            }
            "inconclusive" => {
                let cause = entry.get("cause").ok_or_else(|| missing("/cause"))?;
                let outcome = match read_plugin_inconclusive_cause_over(cause, Some(model))
                    .map_err(HybridEntryRefusal::Entry)?
                {
                    InconclusiveCause::NoInitialState => SolverOutcome::NoInitialState,
                    InconclusiveCause::SolverInconclusive { trace, .. } => {
                        SolverOutcome::Unknown(trace)
                    }
                    _ => {
                        return Err(HybridEntryRefusal::NotRead(spelling_of(
                            "inconclusive",
                            cause,
                        )))
                    }
                };
                // An unclassified method over no coverage: the settlement of
                // these outcomes reads neither (FR-250).
                HybridVerdict::Solved(SolverResult {
                    method: SolverMethod::Unclassified,
                    outcome,
                    coverage: SolverCoverage::Inductive,
                })
            }
            "incomplete" => {
                let cause = entry.get("cause").ok_or_else(|| missing("/cause"))?;
                let stop = match cause.get("type").and_then(Value::as_str) {
                    Some("timed-out") => SolverStop::Time,
                    Some("limit-reached")
                        if cause.get("limit").and_then(Value::as_str)
                            == Some("max_solver_steps") =>
                    {
                        let value = cause.get("value").and_then(Value::as_u64).ok_or_else(|| {
                            HybridEntryRefusal::Entry(PluginResultRefusal::Malformed {
                                pointer: "/cause/value".to_owned(),
                                cause: ValueFault::WrongValueKind,
                            })
                        })?;
                        SolverStop::Steps(value)
                    }
                    _ => {
                        return Err(HybridEntryRefusal::NotRead(spelling_of(
                            "incomplete",
                            cause,
                        )))
                    }
                };
                HybridVerdict::Solved(SolverResult {
                    method: SolverMethod::Unclassified,
                    outcome: SolverOutcome::Stopped(stop),
                    coverage: SolverCoverage::Inductive,
                })
            }
            "refuted" => {
                let position = entry
                    .get("counterexample")
                    .ok_or_else(|| missing("/counterexample"))?;
                let malformed = |cause| {
                    HybridEntryRefusal::Entry(PluginResultRefusal::Malformed {
                        pointer: "/counterexample".to_owned(),
                        cause,
                    })
                };
                let index = position
                    .as_u64()
                    .ok_or_else(|| malformed(ValueFault::WrongValueKind))?;
                let found = usize::try_from(index)
                    .ok()
                    .and_then(|index| counterexamples.get(index))
                    .ok_or_else(|| malformed(ValueFault::InvalidValue))?;
                let named = serde_json::json!({ "counterexample": found });
                HybridVerdict::Refuted(
                    read_plugin_hybrid_counterexample(&named, Some(model))
                        .map_err(HybridEntryRefusal::Entry)?,
                )
            }
            other => return Err(HybridEntryRefusal::NotRead(other.to_owned())),
        };
        Ok(Self {
            verdict,
            certificate: entry.get("certificate").cloned(),
        })
    }
}

/// The entry's `result` and its cause's `type` spelling, for a refusal that
/// names what was not read.
fn spelling_of(result: &str, cause: &Value) -> String {
    match cause.get("type").and_then(Value::as_str) {
        Some(kind) => format!("{result}/{kind}"),
        None => result.to_owned(),
    }
}
