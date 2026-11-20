// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-265 (ADR-031 SW-1 to SW-4, SW-9): derive a state clause's settlement
//! basis and, when that basis is decisive, its one separating witness
//! record, from its S6a evaluation. The FR-109 clause run
//! (`spine::check_clause`) and the FR-122 replay
//! (`execute::replay_state_clause`) both call [`derive_separating_witness`],
//! so producer and replay derive the record the same way.

use filament_checked::node::{Connective, ConnectiveOrigin, NodeKind, Visit};
use filament_eval::FamilyOutcome;
use filament_exact::{Outcome, Value};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::location::ExpressionLocation;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{CheckedGraph, CheckedNode, CheckedStateClause};

use fsl_eval::value::{ClauseEvaluation, StopReport};

use crate::proof_result::SettlementBasis;
use crate::result::SeparatingWitnessRecord;
use crate::{
    bounds::ReplayLimits,
    execute::prepare_state_clause,
    result::SeparationRefusal,
    spine::{CallOutcome, CallRefusal, CallValue, ClauseDisposition},
    ReplayRefusal, ReplayRequestWire, StateClauseCounterexample, WitnessEnvelope,
};
use fsl_foundation::Code;
use fsl_semantics::model::observation::OutOfRange;
use std::borrow::Cow;

/// FR-269's refusal from the public state-clause witness derivation entry.
/// Each variant retains its typed cause and request locus.
#[derive(Debug, thiserror::Error)]
pub enum DerivationRefusal {
    /// FR-122 refused before the clause evaluation settled.
    #[error(transparent)]
    Replay(#[from] ReplayRefusal),
    /// FR-106 admitted an out-of-range integer in the post snapshot.
    #[error("invalid_runtime_input/invalid-value at /family_payload/observation")]
    RangeViolation(Vec<OutOfRange>),
    /// The replay executor or S6a evaluator reached a broken invariant.
    #[error("runtime_invariant: {0:?}")]
    Fault(InternalFault),
    /// The state-clause evaluation returned a catalog refusal.
    #[error("{} / {} at /family_payload/observation", .0.code, .0.cause)]
    Refused(SeparationRefusal),
    /// The evaluation meter ran out before the clause completed.
    #[error("resource_exhausted/insufficient-next-charge at /family_payload/observation")]
    Exhausted,
    /// The clause's evaluation ended undefined.
    #[error("undefined_expression/{cause} at /family_payload/observation: {expression:?}")]
    Undefined {
        /// The expression where evaluation became undefined.
        expression: ExpressionLocation,
        /// The native undefined cause.
        cause: &'static str,
    },
    /// The clause evaluated true, so the supplied observation is not a counterexample.
    #[error("invalid_runtime_input/invalid-value at /family_payload/observation")]
    NotRefuted,
}

impl DerivationRefusal {
    /// The catalog code for this refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::Replay(refusal) => refusal.code(),
            Self::RangeViolation(_) | Self::NotRefuted => Code::InvalidRuntimeInput,
            Self::Fault(_) => Code::RuntimeInvariant,
            Self::Refused(refusal) => {
                Code::from_code(&refusal.code).unwrap_or(Code::RuntimeInvariant)
            }
            Self::Exhausted => Code::ResourceExhausted,
            Self::Undefined { .. } => Code::UndefinedExpression,
        }
    }

    /// The catalog cause, when this refusal has one.
    pub fn cause(&self) -> Option<&str> {
        match self {
            Self::Replay(refusal) => refusal.cause(),
            Self::RangeViolation(_) | Self::NotRefuted => Some("invalid-value"),
            Self::Fault(_) => None,
            Self::Refused(refusal) => Some(&refusal.cause),
            Self::Exhausted => Some("insufficient-next-charge"),
            Self::Undefined { cause, .. } => Some(cause),
        }
    }

    /// The wire pointer associated with this refusal, when one exists.
    pub fn pointer(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::Replay(refusal) => refusal.pointer(),
            Self::Fault(_) => None,
            Self::RangeViolation(_)
            | Self::Refused(_)
            | Self::Exhausted
            | Self::Undefined { .. }
            | Self::NotRefuted => Some(Cow::Borrowed("/family_payload/observation")),
        }
    }
}

/// FR-269 (ADR-031 SW-9): re-run a state clause's replay front half and
/// derive its decisive witness without reading the payload's `witness`.
/// The clause's evaluation and witness derivation are shared with
/// [`crate::replay_state_clause`] through `prepare_state_clause`.
pub fn derive_state_clause_witness(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<StateClauseCounterexample>,
    replay_limits: ReplayLimits,
) -> Result<Option<SeparatingWitnessRecord>, DerivationRefusal> {
    let prepared = prepare_state_clause(wire, envelope, replay_limits)?;
    let range_violations = prepared
        .check
        .observations
        .as_ref()
        .and_then(|observations| observations.post.as_ref())
        .map(|post| post.out_of_range.clone())
        .unwrap_or_default();
    if !range_violations.is_empty() {
        return Err(DerivationRefusal::RangeViolation(range_violations));
    }

    let report = prepared.check.report;
    match report.disposition {
        ClauseDisposition::Admit(failure)
        | ClauseDisposition::SnapshotAdmission { failure, .. } => {
            Err(DerivationRefusal::Replay(ReplayRefusal::Admission(failure)))
        }
        ClauseDisposition::EvaluateFault(fault) => Err(DerivationRefusal::Fault(fault)),
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(false))) => {
            Ok(report.witness)
        }
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(true))) => {
            Err(DerivationRefusal::NotRefuted)
        }
        ClauseDisposition::Evaluate(CallOutcome::Completed(_)) => Err(DerivationRefusal::Fault(
            InternalFault::new("replay", "state-clause-completes-a-boolean"),
        )),
        ClauseDisposition::Evaluate(CallOutcome::Refused(refusal)) => {
            Err(DerivationRefusal::Refused(separation_refusal(refusal)))
        }
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(_)) => {
            Err(DerivationRefusal::Exhausted)
        }
        ClauseDisposition::Evaluate(CallOutcome::Undefined { reason, location }) => location
            .map_or_else(
                || {
                    Err(DerivationRefusal::Fault(InternalFault::new(
                        "replay",
                        "undefined-evaluation-is-located",
                    )))
                },
                |expression| {
                    Err(DerivationRefusal::Undefined {
                        expression,
                        cause: reason,
                    })
                },
            ),
        ClauseDisposition::MissingName { .. } => Err(DerivationRefusal::Fault(InternalFault::new(
            "replay",
            "state-clause-resolved-by-name",
        ))),
        ClauseDisposition::Compile(_)
        | ClauseDisposition::UnknownLanguage { .. }
        | ClauseDisposition::NotAPredicate { .. }
        | ClauseDisposition::ArgumentRefusal(_)
        | ClauseDisposition::Temporal(_)
        | ClauseDisposition::TemporalRefusal(_)
        | ClauseDisposition::TraceRefused(_)
        | ClauseDisposition::ProgressContradiction(_)
        | ClauseDisposition::FrameViolation(_)
        | ClauseDisposition::OperationContractViolation(_)
        | ClauseDisposition::TemporalValuation(_) => Err(DerivationRefusal::Fault(
            InternalFault::new("replay", "clause-check-reports-admission-or-evaluation"),
        )),
    }
}

fn separation_refusal(refusal: CallRefusal) -> SeparationRefusal {
    let (code, fields) = match refusal {
        CallRefusal::Record { code, fields, .. } => (code, fields),
        CallRefusal::Family { code, .. } => (code, std::collections::BTreeMap::new()),
    };
    SeparationRefusal {
        code: code.code().to_owned(),
        cause: code.cause().to_owned(),
        fields: fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

/// FR-265's output for a clause that completed a Boolean: the basis, and
/// the record exactly when the basis is decisive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Derived {
    /// `decisive-counterexample`, `decisive-witness` or `closed-scope`.
    pub(crate) basis: SettlementBasis,
    /// The separating witness record, present exactly when `basis` is
    /// decisive.
    pub(crate) record: Option<SeparatingWitnessRecord>,
}

/// FR-265: walk `clause`'s decision path from its claim root over
/// `evaluation`'s recorded decisions (ADR-031 SW-2): into the body of a
/// `let`, the selected branch of an `if`, the operand of `not`, the operand
/// of `and` or `or` on which evaluation stopped (the first `false` operand
/// of a `false` `and`, the first `true` operand of a `true` `or`, otherwise
/// the right operand), and for `implies` the antecedent when it is `false`,
/// otherwise the consequent. The walk ends at the first node of any other
/// kind, which includes a connective the checker synthesized for a float
/// ordering (it records no decision), and descends the claim's tree, so it
/// visits each node at most once.
/// A walk that ends at a `forall` or `exists` with a stop report settles
/// the decisive basis for the clause's truth and builds the record from
/// that report; any other end settles `closed-scope` with no record. The
/// basis follows the clause's truth, not the polarity at the end of the
/// path (ADR-031 SW-3).
///
/// `Ok(None)` when the evaluation completed no Boolean (FR-266 gives that
/// result basis `unavailable`). It reads only the clause, the evaluation's
/// recorded decisions and stop reports, and evaluates nothing.
///
/// # Errors
///
/// An `InternalFault` when a node the walk reaches has no recorded
/// decision: every such node is on the claim's own level and was evaluated,
/// so S6a recorded it.
pub(crate) fn derive_separating_witness(
    graph: &CheckedGraph,
    clause: &CheckedStateClause,
    evaluation: &ClauseEvaluation,
) -> Result<Option<Derived>, InternalFault> {
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) =
        &evaluation.evaluation.outcome
    else {
        return Ok(None);
    };
    let (stop, comparison) = decision_end(graph, clause.body(), evaluation)?;
    Ok(Some(match (stop, comparison) {
        (Some(report), _) => Derived {
            basis: SettlementBasis::decisive(*truth),
            record: Some(SeparatingWitnessRecord::from_stop(report)),
        },
        (None, Some((report, path))) => match evaluation.receiver_path() {
            Some(root) => Derived {
                basis: SettlementBasis::decisive(*truth),
                record: Some(SeparatingWitnessRecord::from_comparison(
                    report,
                    path,
                    root.clone(),
                )),
            },
            None => closed_scope(),
        },
        (None, None) => closed_scope(),
    }))
}

/// Follow the clause's decision path and return its stopped quantifier, or
/// its authored comparison and the occurrence keys on the path to it.
fn decision_end<'e>(
    graph: &CheckedGraph,
    root: CheckedNode<'_>,
    evaluation: &'e ClauseEvaluation,
) -> Result<
    (
        Option<&'e StopReport>,
        Option<(&'e fsl_eval::value::ComparisonReport, Vec<OccurrenceKey>)>,
    ),
    InternalFault,
> {
    let mut node = root;
    let mut path = Vec::new();
    loop {
        let next = match node.kind() {
            NodeKind::Let { body, .. } => Some(*body),
            NodeKind::If {
                then, otherwise, ..
            } => {
                let condition = recorded_decision(evaluation, node)?;
                Some(if condition { *then } else { *otherwise })
            }
            NodeKind::Not(operand) => Some(*operand),
            NodeKind::Connective(_, ConnectiveOrigin::Synthesized, _, _) => {
                return Ok((None, None));
            }
            NodeKind::Connective(connective, ConnectiveOrigin::Authored, left, right) => {
                let left_value = recorded_decision(evaluation, node)?;
                let stopped_on_left = match connective {
                    Connective::And | Connective::Implies => !left_value,
                    Connective::Or => left_value,
                };
                Some(if stopped_on_left { *left } else { *right })
            }
            NodeKind::Query {
                visit: Visit::Forall | Visit::Exists,
                ..
            } => {
                return Ok((evaluation.stop(node.location()), None));
            }
            _ => {
                let Some(report) = evaluation.comparison(node.location()) else {
                    return Ok((None, None));
                };
                path.push(occurrence_at(graph, node.location()).ok_or_else(|| {
                    InternalFault::new("witness", "decision-path-node-has-occurrence")
                })?);
                return Ok((None, Some((report, path))));
            }
        };
        path.push(
            occurrence_at(graph, node.location()).ok_or_else(|| {
                InternalFault::new("witness", "decision-path-node-has-occurrence")
            })?,
        );
        let Some(next) = next else {
            return Err(InternalFault::new(
                "witness",
                "decision-path-node-has-child",
            ));
        };
        node = node.at(next);
    }
}

fn recorded_decision(
    evaluation: &ClauseEvaluation,
    node: CheckedNode<'_>,
) -> Result<bool, InternalFault> {
    evaluation
        .decision(node.location())
        .ok_or_else(|| InternalFault::new("witness", "decision-path-node-recorded"))
}

fn occurrence_at(
    graph: &CheckedGraph,
    location: &filament_semantic_value::location::ExpressionLocation,
) -> Option<OccurrenceKey> {
    graph
        .occurrences()
        .find(|(_, origin, at)| *at == location && origin.role().as_str() == "expression")
        .map(|(key, origin, _)| OccurrenceKey::new(key.clone(), origin))
}

/// FR-265's decision-path walk from `root` over `evaluation`'s recorded
/// decisions (ADR-031 SW-2), shared by a state clause's claim and a temporal
/// `holds` atom's condition (FR-283 `decisive`): the stop report of the
/// `forall` or `exists` the walk ends at, or `None` when it ends at any
/// other node or at a quantifier that did not stop.
///
/// # Errors
///
/// An `InternalFault` when a node the walk reaches has no recorded decision.
pub(crate) fn decisive_stop<'e>(
    root: CheckedNode<'_>,
    evaluation: &'e ClauseEvaluation,
) -> Result<Option<&'e StopReport>, InternalFault> {
    let decision = |node: CheckedNode<'_>| {
        evaluation
            .decision(node.location())
            .ok_or_else(|| InternalFault::new("witness", "decision-path-node-recorded"))
    };
    let mut node = root;
    loop {
        let next = match node.kind() {
            NodeKind::Let { body, .. } => body,
            NodeKind::If {
                then, otherwise, ..
            } => {
                if decision(node)? {
                    then
                } else {
                    otherwise
                }
            }
            NodeKind::Not(operand) => operand,
            // A connective the checker synthesized (a float ordering's
            // composition) records no decision: the path ends there.
            NodeKind::Connective(_, ConnectiveOrigin::Synthesized, _, _) => return Ok(None),
            NodeKind::Connective(connective, ConnectiveOrigin::Authored, left, right) => {
                let left_value = decision(node)?;
                let stopped_on_left = match connective {
                    Connective::And | Connective::Implies => !left_value,
                    Connective::Or => left_value,
                };
                if stopped_on_left { left } else { right }
            }
            NodeKind::Query {
                visit: Visit::Forall | Visit::Exists,
                ..
            } => return Ok(evaluation.stop(node.location())),
            // ADR-031 SW-2: "The path ends at the first node of any other
            // kind" -- the rule itself is the catch-all, so a node kind
            // added later ends the walk too.
            _ => return Ok(None),
        };
        node = node.at(*next);
    }
}

fn closed_scope() -> Derived {
    Derived {
        basis: SettlementBasis::ClosedScope,
        record: None,
    }
}
