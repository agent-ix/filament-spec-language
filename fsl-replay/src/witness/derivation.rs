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
    execute::{prepare_state_clause, separation_refusal},
    result::SeparationRefusal,
    spine::{CallOutcome, CallValue, ClauseDisposition},
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
    pub fn code(&self) -> &str {
        match self {
            Self::Replay(refusal) => refusal.code().as_str(),
            Self::RangeViolation(_) | Self::NotRefuted => Code::InvalidRuntimeInput.as_str(),
            Self::Fault(_) => Code::RuntimeInvariant.as_str(),
            Self::Refused(refusal) => &refusal.code,
            Self::Exhausted => Code::ResourceExhausted.as_str(),
            Self::Undefined { .. } => Code::UndefinedExpression.as_str(),
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
    let check = prepared.check;
    classify_with_range_violations(
        range_violations,
        check.report.disposition,
        check.report.witness,
        check.stopped_call,
    )
}

fn classify_clause_outcome(
    disposition: ClauseDisposition,
    witness: Option<SeparatingWitnessRecord>,
    stopped_call: Option<ExpressionLocation>,
) -> Result<Option<SeparatingWitnessRecord>, DerivationRefusal> {
    match disposition {
        ClauseDisposition::Admit(failure)
        | ClauseDisposition::SnapshotAdmission { failure, .. } => {
            Err(DerivationRefusal::Replay(ReplayRefusal::Admission(failure)))
        }
        ClauseDisposition::EvaluateFault(fault) => Err(DerivationRefusal::Fault(fault)),
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(false))) => {
            Ok(witness)
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
        ClauseDisposition::Evaluate(
            CallOutcome::Incomplete(_)
            | CallOutcome::RelationshipIncomplete(_)
            | CallOutcome::RelationshipLimit(_),
        ) => Err(DerivationRefusal::Exhausted),
        ClauseDisposition::Evaluate(CallOutcome::Undefined { reason, location }) => location
            .map_or_else(
                || {
                    Err(DerivationRefusal::Fault(InternalFault::new(
                        "replay",
                        "undefined-evaluation-is-located",
                    )))
                },
                |inner| {
                    // TC-3121 row 5: an undefined result inside a called
                    // body names the clause expression that ended undefined,
                    // the clause's call, keeping the body's cause.
                    Err(DerivationRefusal::Undefined {
                        expression: stopped_call.unwrap_or(inner),
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
        | ClauseDisposition::ExtractionClauseCount { .. }
        | ClauseDisposition::NotAPredicate { .. }
        | ClauseDisposition::ArgumentRefusal(_)
        | ClauseDisposition::Temporal(_)
        | ClauseDisposition::PackageAdmission(_)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// TC-3121: an evaluator invariant fault remains the typed internal
    /// fault rather than becoming a missing witness or a replay refusal.
    #[ix_trace_rs::trace("TC-3121", "FR-269-AC-8")]
    #[test]
    fn in_crate_fault_disposition_remains_a_derivation_fault() {
        let fault = InternalFault::new("S6a", "checked-program-invariant");
        let error = classify_clause_outcome(ClauseDisposition::EvaluateFault(fault), None, None)
            .expect_err("an in-crate fault is a typed derivation refusal");
        assert_eq!(error.code(), Code::RuntimeInvariant.as_str());
        assert!(matches!(error, DerivationRefusal::Fault(_)));
    }

    /// TC-3121: an admitted range violation takes precedence over a clause
    /// evaluator fault, while the same fault without a range violation stays
    /// a runtime-invariant refusal.
    #[ix_trace_rs::trace("TC-3121", "FR-269-AC-8")]
    #[test]
    fn range_violation_precedes_an_in_crate_fault() {
        let fault = InternalFault::new("S6a", "checked-program-invariant");
        let observed = filament_exact::Integer::from(-1_i64);
        let object_type = filament_exact::EffectiveId::new(
            None,
            filament_exact::OriginalDeclarationKey::new("agent-ix/config-version", "ConfigVersion"),
        );
        let range_violation = OutOfRange {
            object: filament_exact::ObjectReference::new(
                filament_exact::UniverseId::new(crate::fixture_node(1)),
                object_type.clone(),
                filament_exact::ObjectId::new("child").unwrap(),
            ),
            field: filament_semantic_value::declaration::FieldRef::new(
                object_type,
                "versionNumber",
            ),
            index: None,
            range: filament_exact::IntegerInterval::new(
                filament_exact::Integer::from(0_i64),
                filament_exact::Integer::from(1000_i64),
            )
            .unwrap(),
            observed,
            steps: Vec::new(),
        };
        let error = classify_with_range_violations(
            vec![range_violation.clone()],
            ClauseDisposition::EvaluateFault(fault),
            None,
            None,
        )
        .expect_err("range admission takes precedence over clause evaluation");
        assert!(matches!(
            error,
            DerivationRefusal::RangeViolation(violations) if violations == vec![range_violation]
        ));
    }

    /// TC-3121: the undefined evaluator outcome keeps its native cause and
    /// source expression location at the derivation boundary.
    #[ix_trace_rs::trace("TC-3121", "FR-269-AC-7")]
    #[test]
    fn undefined_disposition_keeps_location_and_cause() {
        let location = ExpressionLocation::at(
            filament_semantic_value::location::ExpressionOwner::StateClause {
                clause: "UndefinedDivision".to_owned(),
                index: 0,
            },
            &[0],
        );
        let error = classify_clause_outcome(
            ClauseDisposition::Evaluate(CallOutcome::Undefined {
                reason: "division-by-zero",
                location: Some(location.clone()),
            }),
            None,
            None,
        )
        .expect_err("an undefined evaluation has no witness");
        assert_eq!(error.code(), Code::UndefinedExpression.as_str());
        assert_eq!(error.cause(), Some("division-by-zero"));
        assert!(matches!(
            error,
            DerivationRefusal::Undefined { expression, cause }
                if expression == location && cause == "division-by-zero"
        ));
    }

    /// TC-3121 row 5: an undefined result inside a called function's body
    /// names the clause's call that ended undefined and keeps the body's
    /// cause.
    #[ix_trace_rs::trace("TC-3121", "FR-269-AC-7")]
    #[test]
    fn undefined_inside_a_called_body_names_the_clause_call() {
        let inner = ExpressionLocation::at(
            filament_semantic_value::location::ExpressionOwner::Body {
                function: "u".to_owned(),
                index: 0,
            },
            &[0],
        );
        let call = ExpressionLocation::at(
            filament_semantic_value::location::ExpressionOwner::StateClause {
                clause: "UndefinedDomain".to_owned(),
                index: 0,
            },
            &[0, 0],
        );
        let error = classify_clause_outcome(
            ClauseDisposition::Evaluate(CallOutcome::Undefined {
                reason: "division-by-zero",
                location: Some(inner),
            }),
            None,
            Some(call.clone()),
        )
        .expect_err("an undefined evaluation has no witness");
        assert!(matches!(
            error,
            DerivationRefusal::Undefined { expression, cause }
                if expression == call && cause == "division-by-zero"
        ));
    }
}

fn classify_with_range_violations(
    range_violations: Vec<OutOfRange>,
    disposition: ClauseDisposition,
    witness: Option<SeparatingWitnessRecord>,
    stopped_call: Option<ExpressionLocation>,
) -> Result<Option<SeparatingWitnessRecord>, DerivationRefusal> {
    if !range_violations.is_empty() {
        return Err(DerivationRefusal::RangeViolation(range_violations));
    }
    classify_clause_outcome(disposition, witness, stopped_call)
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
type DecisionEnd<'e> = (
    Option<&'e StopReport>,
    Option<(&'e fsl_eval::value::ComparisonReport, Vec<OccurrenceKey>)>,
);

fn decision_end<'e>(
    graph: &CheckedGraph,
    root: CheckedNode<'_>,
    evaluation: &'e ClauseEvaluation,
) -> Result<DecisionEnd<'e>, InternalFault> {
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
        match decisive_step(node.kind(), || decision(node))? {
            DecisiveStep::Child(next) => node = node.at(*next),
            DecisiveStep::Quantifier => return Ok(evaluation.stop(node.location())),
            DecisiveStep::End => return Ok(None),
        }
    }
}

/// The same decision walk over the admitted evaluator projection. Locations
/// are the projection's actual checker paths, including repeated visits.
pub(crate) fn decisive_package_stop<'e>(
    projection: &fsl_package::ReaderProtocolProjection,
    evaluation: &'e ClauseEvaluation,
) -> Result<Option<&'e StopReport>, InternalFault> {
    let fault = || InternalFault::new("witness", "decision-path-node-recorded");
    let mut node = projection.root();
    loop {
        let kind = projection.kind(node).ok_or_else(fault)?;
        let location = projection.location(node).ok_or_else(fault)?;
        match decisive_step(kind, || evaluation.decision(location).ok_or_else(fault))? {
            DecisiveStep::Child(next) => node = *next,
            DecisiveStep::Quantifier => return Ok(evaluation.stop(location)),
            DecisiveStep::End => return Ok(None),
        }
    }
}

enum DecisiveStep<'a, N> {
    Child(&'a N),
    Quantifier,
    End,
}

fn decisive_step<N, L>(
    kind: &NodeKind<N, L>,
    decision: impl FnOnce() -> Result<bool, InternalFault>,
) -> Result<DecisiveStep<'_, N>, InternalFault> {
    let next = match kind {
        NodeKind::Let { body, .. } => body,
        NodeKind::If {
            then, otherwise, ..
        } => {
            if decision()? {
                then
            } else {
                otherwise
            }
        }
        NodeKind::Not(operand) => operand,
        // A connective the checker synthesized (a float ordering's
        // composition) records no decision: the path ends there.
        NodeKind::Connective(_, ConnectiveOrigin::Synthesized, _, _) => {
            return Ok(DecisiveStep::End)
        }
        NodeKind::Connective(connective, ConnectiveOrigin::Authored, left, right) => {
            let left_value = decision()?;
            let stopped_on_left = match connective {
                Connective::And | Connective::Implies => !left_value,
                Connective::Or => left_value,
            };
            if stopped_on_left {
                left
            } else {
                right
            }
        }
        NodeKind::Query {
            visit: Visit::Forall | Visit::Exists,
            ..
        } => return Ok(DecisiveStep::Quantifier),
        // ADR-031 SW-2: "The path ends at the first node of any other
        // kind" -- the rule itself is the catch-all, so a node kind
        // added later ends the walk too.
        _ => return Ok(DecisiveStep::End),
    };
    Ok(DecisiveStep::Child(next))
}

fn closed_scope() -> Derived {
    Derived {
        basis: SettlementBasis::ClosedScope,
        record: None,
    }
}
