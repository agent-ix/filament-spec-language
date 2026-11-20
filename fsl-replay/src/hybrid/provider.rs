// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750's in-process hybrid provider seam. The trait, the typed request and
//! the answer live beside [`SolverResult`] so a provider crate below layer A
//! implements [`HybridProvider`] without depending on `fsl-analyze`.

use std::collections::BTreeMap;
use std::fmt;

use filament_exact::Cancel;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{
    CheckedGraph, CheckedHybridClaim, CheckedHybridModel, ModeIndex, VariableIndex,
};
use serde_json::Value;

use super::{SolverBudget, SolverCoverage, SolverMethod, SolverOutcome, SolverResult, SolverStop};
use crate::hybrid_replay::{
    read_certificate, CertificateRefusal, Expect, ExpressionSite, HybridCertificate,
    HybridCounterexample, RefusalCode, TermLocus,
};
use crate::proof_result::{
    read_plugin_basis, read_plugin_hybrid_counterexample_over, read_plugin_inconclusive_cause_over,
    InconclusiveCause, PluginResultRefusal, ProofBasis, ValueFault,
};

/// One hybrid item's solve request (FR-750): the checked claim and the model
/// it ranges over, the resolved `hybrid.limits`, the FSpec FR-331 `options`
/// and the request's deadline.
#[derive(Clone, Debug)]
pub struct HybridSolveRequest<'a> {
    /// The checked package's graph, which a provider reads an entry against
    /// ([`HybridAnswer::read_entry`]).
    pub graph: &'a CheckedGraph,
    /// The graph of the package emitted for this request, which spells and
    /// reads a `Locus` node id ([`HybridAnswer::read_entry`]).
    pub wire: &'a WireGraph,
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
/// it. FR-751's checker is a later slice (QSL-2531), so the certificate is
/// carried on the answer unchecked, `analyze` does not write it to the
/// outcome, and the item settles `uncertified`.
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
pub enum HybridVerdict {
    /// Proved, inconclusive or incomplete: settled by FR-250's map.
    Solved(SolverResult),
    /// A claimed refutation: settled only by FR-450's exact replay. A
    /// claimed refutation is never folded into [`SolverOutcome::Unknown`],
    /// which FR-250 settles `solver-inconclusive`, because replay parity
    /// needs the claim.
    Refuted(Box<HybridCounterexample>),
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

    /// Refuse an item's `options` whose value lies outside the domain of an
    /// option the provider publishes (FR-750, FSpec FR-779). `analyze` calls
    /// it after FR-967's `unknown-option` check and before [`Self::solve`],
    /// so a refused item runs no solver step. The default admits every value.
    ///
    /// # Errors
    ///
    /// [`OptionRefusal`] naming the least refused option and its value.
    fn check_options(&self, _options: &BTreeMap<String, String>) -> Result<(), OptionRefusal> {
        Ok(())
    }
}

/// An option value outside its domain, as [`HybridProvider::check_options`]
/// refuses it: the item settles `invalid-request`
/// `invalid_capability`/`invalid-option-value` (FSpec FR-779-AC-2).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("option `{option}` does not admit the value `{value}`")]
pub struct OptionRefusal {
    /// The option's name.
    pub option: String,
    /// The refused value, as the item wrote it.
    pub value: String,
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
    /// The claim names no model of the graph, so no entry can be read
    /// against it.
    #[error("the claim names no hybrid model of the graph: {0:?}")]
    UnknownModel(fsl_semantics::check::HybridModelIndex),
}

impl HybridAnswer {
    /// The answer's certificate, read against the form the entry takes (FSpec
    /// FR-652 "Checking order"): a proof takes the form of its basis, a refutation
    /// or a `solver-inconclusive` entry carrying a trace takes `validated-witness`,
    /// and no other entry carries one.
    ///
    /// # Errors
    ///
    /// [`CertificateRefusal`] for a certificate the reader cannot read.
    pub fn read_certificate(
        &self,
        graph: &CheckedGraph,
        claim: &CheckedHybridClaim,
    ) -> Result<Option<HybridCertificate>, CertificateRefusal> {
        let Some(value) = &self.certificate else {
            return Ok(None);
        };
        let model = graph
            .hybrid_models()
            .get(claim.model.get())
            .ok_or_else(|| CertificateRefusal {
                pointer: "/certificate".to_owned(),
                code: RefusalCode::InvalidValue,
            })?;
        let expect = match &self.verdict {
            HybridVerdict::Refuted(claimed) => Some(Expect::Witness(&claimed.trace)),
            HybridVerdict::Solved(result) => match (&result.outcome, &result.coverage) {
                (
                    SolverOutcome::Unsat | SolverOutcome::EnclosureDisjoint,
                    SolverCoverage::Bounded { .. },
                ) => Some(Expect::Bounded),
                (
                    SolverOutcome::Unsat | SolverOutcome::EnclosureDisjoint,
                    SolverCoverage::Inductive,
                ) => Some(Expect::Inductive),
                (SolverOutcome::Unknown(Some(trace)), _) => Some(Expect::Witness(trace)),
                _ => None,
            },
            HybridVerdict::Unsupported(_) => None,
        };
        let Some(expect) = expect else {
            return Err(CertificateRefusal {
                pointer: "/certificate".to_owned(),
                code: RefusalCode::UnknownMember,
            });
        };
        read_certificate(model, expect, value).map(Some)
    }

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
        graph: &CheckedGraph,
        claim: &CheckedHybridClaim,
        wire: &WireGraph,
    ) -> Result<Self, HybridEntryRefusal> {
        let model = graph
            .hybrid_models()
            .get(claim.model.get())
            .ok_or(HybridEntryRefusal::UnknownModel(claim.model))?;
        let missing = |pointer: &str| {
            HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
                pointer: pointer.to_owned(),
            })
        };
        let result = entry.get("result").and_then(Value::as_str).unwrap_or("");
        let verdict = match result {
            "proved" => {
                let basis = entry.get("basis").ok_or_else(|| missing("/basis"))?;
                HybridVerdict::Solved(
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
                    },
                )
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
                        let value =
                            cause.get("value").and_then(Value::as_u64).ok_or_else(|| {
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
                HybridVerdict::Refuted(Box::new(
                    read_plugin_hybrid_counterexample_over(&named, graph, claim, wire)
                        .map_err(HybridEntryRefusal::Entry)?,
                ))
            }
            "unsupported" => match read_unsupported(entry, graph, claim, model, wire)? {
                Some(construct) => HybridVerdict::Unsupported(construct),
                None => return Err(HybridEntryRefusal::NotRead("unsupported".to_owned())),
            },
            other => return Err(HybridEntryRefusal::NotRead(other.to_owned())),
        };
        let answer = Self {
            verdict,
            certificate: entry.get("certificate").cloned(),
        };
        answer
            .read_certificate(graph, claim)
            .map_err(|refusal| HybridEntryRefusal::Entry(refusal.into()))?;
        Ok(answer)
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

/// The `unsupported_construct`/`expression-form` construct of an `unsupported`
/// entry, its `detail` read against `model` and its `locus` resolved through
/// `graph` (FR-750). `None` for an entry with another diagnostic, which is not
/// a solver answer. A `mode`, `variable` or `locus` the model does not have is
/// the provider's malformed result, `invalid_result`/`invalid-value` at that
/// member of `/diagnostic/detail`.
fn read_unsupported(
    entry: &Value,
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    model: &CheckedHybridModel,
    wire: &WireGraph,
) -> Result<Option<UnsupportedConstruct>, HybridEntryRefusal> {
    let diagnostic = entry.get("diagnostic");
    let spelled = |member: &str| {
        diagnostic
            .and_then(|d| d.get(member))
            .and_then(Value::as_str)
    };
    if spelled("code") != Some("unsupported_construct")
        || spelled("cause") != Some("expression-form")
    {
        return Ok(None);
    }
    let invalid = |member: &str, cause| {
        HybridEntryRefusal::Entry(PluginResultRefusal::Malformed {
            pointer: format!("/diagnostic/detail/{member}"),
            cause,
        })
    };
    let detail = diagnostic.and_then(|d| d.get("detail")).ok_or_else(|| {
        HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
            pointer: "/diagnostic/detail".to_owned(),
        })
    })?;
    let text = |member: &str| -> Result<&str, HybridEntryRefusal> {
        match detail.get(member) {
            None => Err(HybridEntryRefusal::Entry(
                PluginResultRefusal::MissingMember {
                    pointer: format!("/diagnostic/detail/{member}"),
                },
            )),
            Some(value) => value
                .as_str()
                .ok_or_else(|| invalid(member, ValueFault::WrongValueKind)),
        }
    };
    let form = text("form")?.to_owned();
    let mode_name = text("mode")?;
    let mode = model
        .modes
        .iter()
        .position(|mode| mode.name.as_str() == mode_name)
        .ok_or_else(|| invalid("mode", ValueFault::InvalidValue))?;
    let variable = match detail.get("variable") {
        None => None,
        Some(_) => {
            let name = text("variable")?;
            Some(
                model
                    .variables
                    .iter()
                    .position(|variable| variable.name.as_str() == name)
                    .ok_or_else(|| invalid("variable", ValueFault::InvalidValue))?,
            )
        }
    };
    let locus = detail.get("locus").ok_or_else(|| {
        HybridEntryRefusal::Entry(PluginResultRefusal::MissingMember {
            pointer: "/diagnostic/detail/locus".to_owned(),
        })
    })?;
    let position = graph
        .hybrid_claims()
        .iter()
        .position(|candidate| candidate == claim)
        .ok_or(HybridEntryRefusal::UnknownModel(claim.model))?;
    let found = super::locus::hybrid_locus_site(graph, wire, position, mode, variable, locus)
        .ok_or_else(|| invalid("locus", ValueFault::InvalidValue))?;
    Ok(Some(UnsupportedConstruct {
        form,
        site: found.site,
        term: found.term,
        mode: ModeIndex::new(mode),
        variable: variable.map(VariableIndex::new),
    }))
}
