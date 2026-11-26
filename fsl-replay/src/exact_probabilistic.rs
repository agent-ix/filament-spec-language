// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's product-build refusal settlement (FR-203).

use filament_exact::{CancelCause, Rational};
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::simulation::model::{ClauseOutcome, RegionCause};
use fsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use fsl_foundation::{BackendId, RequestIndex};

use crate::model_check::limits::{meter_bound, ModelCheckLimit};
use crate::proof_result::{
    IncompleteCause, InconclusiveCause, TerminalRecord, TerminalValue, UnavailabilityCause,
};
use crate::statistical::SubjectLimits;

/// The build result's settlement-relevant cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactProbBuildIssue {
    /// An exact-product budget stopped exploration.
    Limit {
        /// The request limit reached.
        limit: ModelCheckLimit,
        /// The configured value.
        value: u64,
    },
    /// The request was cancelled.
    Cancelled(CancelCause),
    /// Dyadic refinement reached its maximum precision with this interval
    /// still containing the threshold.
    PrecisionBudget {
        /// Lower endpoint.
        lower: Rational,
        /// Upper endpoint.
        upper: Rational,
    },
    /// The workload does not define a Markov chain.
    NotMarkov(Box<fsl_eval::simulation::model::NotMarkov>),
    /// A product successor's contract was undecided.
    UndecidedSuccessor,
    /// State expansion failed or reached a subject-owned limit.
    Expansion(ExpansionStop),
    /// A claim predicate was refused or incomplete.
    Predicate(ClauseOutcome),
    /// An internal product invariant, automaton translation, or evaluator
    /// invariant failed.
    BrokenInvariant,
    /// A refusal carrying its own catalog diagnostic.
    Failed(CatalogCode),
    /// A timed-product form the digital engine recognizes but does not admit.
    Unsupported(UnavailabilityCause),
    /// A timed claim that needs a region process that is not built (ADR-038
    /// RL-3).
    RegionNotBuilt(RegionCause),
}

/// One settled EN-5 build issue. It is always non-refuted: a counterexample
/// can settle only after FR-202 replays its evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbBuildRecord {
    value: TerminalValue,
    diagnostic: Option<CatalogCode>,
    region: Option<RegionCause>,
}

impl ExactProbBuildRecord {
    /// Why the claim leaves digital clocks, when the run stopped because the
    /// region process it needs is not built.
    pub fn region_cause(&self) -> Option<RegionCause> {
        self.region
    }

    /// The FR-331 result kind this stop settles.
    pub fn value(&self) -> &TerminalValue {
        &self.value
    }

    /// Its one diagnostic, when this is a failed result.
    pub fn diagnostic(&self) -> Option<CatalogCode> {
        self.diagnostic
    }

    /// Build this non-refuted EN-5 result's terminal record.
    pub fn into_terminal(self, request_index: RequestIndex, backend: BackendId) -> TerminalRecord {
        match (self.value, self.diagnostic) {
            (TerminalValue::Incomplete(cause), None) => {
                TerminalRecord::incomplete(request_index, backend, cause)
            }
            (TerminalValue::Inconclusive(cause), None) => {
                TerminalRecord::inconclusive(request_index, backend, cause)
            }
            (TerminalValue::Unsupported(cause), None) => {
                TerminalRecord::unsupported(request_index, backend, cause)
            }
            (TerminalValue::Failed, Some(diagnostic)) => {
                TerminalRecord::failed_with(request_index, backend, diagnostic)
            }
            _ => unreachable!("an exact build issue has a non-refuted terminal value"),
        }
    }
}

/// FR-203's exhaustive map for FR-196's build stops and predicate refusals.
pub fn settle_exact_prob_build_issue(
    issue: &ExactProbBuildIssue,
    subject: SubjectLimits<'_>,
) -> ExactProbBuildRecord {
    let incomplete = |limit, value| ExactProbBuildRecord {
        value: TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit,
            value,
            required: None,
        }),
        diagnostic: None,
        region: None,
    };
    let failed = |diagnostic| ExactProbBuildRecord {
        value: TerminalValue::Failed,
        diagnostic: Some(diagnostic),
        region: None,
    };
    match issue {
        ExactProbBuildIssue::Limit { limit, value } => incomplete(*limit, *value),
        ExactProbBuildIssue::Cancelled(source) => ExactProbBuildRecord {
            value: TerminalValue::Incomplete(IncompleteCause::Cancelled { source: *source }),
            diagnostic: None,
            region: None,
        },
        ExactProbBuildIssue::PrecisionBudget { lower, upper } => ExactProbBuildRecord {
            value: TerminalValue::Incomplete(IncompleteCause::PrecisionBudget {
                lower: Box::new(crate::proof_result::ExactRational::new(lower.clone())),
                upper: Box::new(crate::proof_result::ExactRational::new(upper.clone())),
            }),
            diagnostic: None,
            region: None,
        },
        ExactProbBuildIssue::NotMarkov(cause) => ExactProbBuildRecord {
            value: TerminalValue::Unsupported(UnavailabilityCause::NotMarkov(cause.clone())),
            diagnostic: None,
            region: None,
        },
        ExactProbBuildIssue::UndecidedSuccessor => ExactProbBuildRecord {
            value: TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor),
            diagnostic: None,
            region: None,
        },
        ExactProbBuildIssue::Expansion(stop) => match stop.limit {
            Some(ExpansionLimit::EvaluationMeter(counter)) => incomplete(
                ModelCheckLimit::EvaluationMeter { counter },
                meter_bound(subject.meter, counter),
            ),
            Some(ExpansionLimit::MaxCandidates) => incomplete(
                ModelCheckLimit::MaxCandidates,
                subject.expansion.max_candidates,
            ),
            None if stop.is_internal_failure() => failed(BROKEN),
            None => failed(stop.cause),
        },
        ExactProbBuildIssue::Predicate(outcome) => predicate(outcome, incomplete, failed),
        ExactProbBuildIssue::BrokenInvariant => failed(BROKEN),
        ExactProbBuildIssue::Failed(diagnostic) => failed(*diagnostic),
        // FSpec FR-421: the run stops `incomplete`, with no verdict. FSpec
        // FR-331's closed cause set has no member for an unbuilt region
        // process; the record carries the cause beside the nearest member,
        // dense dynamics the run cannot resolve.
        ExactProbBuildIssue::RegionNotBuilt(cause) => ExactProbBuildRecord {
            value: TerminalValue::Incomplete(IncompleteCause::UnresolvedDynamics),
            diagnostic: None,
            region: Some(*cause),
        },
        ExactProbBuildIssue::Unsupported(cause) => ExactProbBuildRecord {
            value: TerminalValue::Unsupported(cause.clone()),
            diagnostic: None,
            region: None,
        },
    }
}

const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

fn predicate(
    outcome: &ClauseOutcome,
    incomplete: impl Fn(ModelCheckLimit, u64) -> ExactProbBuildRecord,
    failed: impl Fn(CatalogCode) -> ExactProbBuildRecord,
) -> ExactProbBuildRecord {
    match outcome {
        ClauseOutcome::KernelRefused(refusal) => match (refusal.code(), refusal.cause()) {
            (Some(code), Some(cause)) => failed(CatalogCode::new(code, cause)),
            _ => failed(BROKEN),
        },
        ClauseOutcome::FamilyRefused(code) | ClauseOutcome::Stopped(code) => failed(*code),
        ClauseOutcome::Incomplete(value) => incomplete(
            ModelCheckLimit::EvaluationMeter {
                counter: value.limit_kind,
            },
            value.limit,
        ),
        ClauseOutcome::KernelUndefined(_)
        | ClauseOutcome::FamilyUndefined(_)
        | ClauseOutcome::NotBoolean => failed(BROKEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{Integer, IntegerInterval, ScalarLimits};
    use fsl_eval::simulation::model::ExpansionLimits;
    use ix_trace_rs::trace;

    fn not_markov() -> fsl_eval::simulation::model::NotMarkov {
        let transition = fsl_eval::simulation::model::ScheduledIdentity {
            operation: r#""tick""#.to_owned(),
            receiver: filament_exact::ObjectReference::new(
                crate::fixture_universe(1),
                crate::fixture_effective(2),
                filament_exact::ObjectId::new("n").expect("a key"),
            ),
            arguments: Vec::new(),
        };
        let drawn_transition = filament_semantic_value::model_transition::ModelTransition::new(
            transition.operation.clone(),
            Some(
                fsl_eval::simulation::model::reference_canonical_text(&transition.receiver)
                    .expect("a receiver"),
            ),
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("a transition");
        fsl_eval::simulation::model::NotMarkov {
            state: fsl_eval::simulation::StateKey::from_canonical_bytes(b"\"healthy\"".to_vec()),
            transition,
            drawn_transition,
            random: Vec::new(),
            post_states: 0,
            post_state_examples: Vec::new(),
        }
    }

    /// Invalid count/evidence combinations cannot become a terminal cause,
    /// whether built directly, settled by EN-5, or used by the planner writer.
    #[trace("TC-622", "FR-187-AC-3", "TC-638", "FR-203-AC-7")]
    #[test]
    fn malformed_not_markov_evidence_fails_before_terminal_serialization() {
        use crate::outcome::OutcomeItem;
        use fsl_eval::simulation::StateKey;

        let key = |bytes: &[u8]| StateKey::from_canonical_bytes(bytes.to_vec());
        let a = key(b"1");
        let b = key(b"2");
        let c = key(b"3");
        let cases = [
            (1, Vec::new()),
            (0, vec![a.clone(), b.clone()]),
            (2, Vec::new()),
            (2, vec![a.clone()]),
            (2, vec![a.clone(), a.clone()]),
            (2, vec![b.clone(), a.clone()]),
            (4, vec![a, b, c]),
        ];
        let meter = crate::AccountingLimits::default().0;
        for (count, examples) in cases {
            let mut cause = not_markov();
            cause.post_states = count;
            cause.post_state_examples = examples;
            let unsupported = UnavailabilityCause::NotMarkov(Box::new(cause.clone()));
            let settled = settle_exact_prob_build_issue(
                &ExactProbBuildIssue::NotMarkov(Box::new(cause)),
                SubjectLimits {
                    expansion: ExpansionLimits { max_candidates: 64 },
                    meter: &meter,
                },
            )
            .into_terminal(RequestIndex::new(0), BackendId::new("en-5"));
            let direct = TerminalRecord::unsupported(
                RequestIndex::new(0),
                BackendId::new("en-5"),
                unsupported.clone(),
            );
            let planner = TerminalRecord::from_value(
                RequestIndex::new(0),
                TerminalValue::Unsupported(unsupported),
            );
            for record in [settled, direct, planner] {
                assert_eq!(record.value(), &TerminalValue::Failed, "count {count}");
                assert_eq!(record.diagnostic(), Some(BROKEN));
                let wire = serde_json::to_value(OutcomeItem::from_terminal(&record)).unwrap();
                assert_eq!(wire["result"], "failed");
                assert_eq!(wire["cause"]["kind"], "established-invariant-broken");
                assert!(wire["cause"].get("post_state_count").is_none(), "{wire}");
            }
        }
    }

    #[trace("TC-638", "FR-203-AC-6", "FR-203-AC-7")]
    #[test]
    fn build_stops_and_refusals_have_the_fr203_terminal_map() {
        let meter = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units: 4_096,
            result_units: 64,
        };
        let subject = SubjectLimits {
            expansion: ExpansionLimits { max_candidates: 2 },
            meter: &meter,
        };
        let cases = [
            (
                ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxStates,
                    value: 1_000,
                },
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::ExactProbMaxStates,
                    value: 1_000,
                    required: None,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::NotMarkov(Box::new(not_markov())),
                TerminalValue::Unsupported(UnavailabilityCause::NotMarkov(Box::new(not_markov()))),
                None,
            ),
            (
                ExactProbBuildIssue::UndecidedSuccessor,
                TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: Some(ExpansionLimit::MaxCandidates),
                }),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxCandidates,
                    value: 2,
                    required: None,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: None,
                }),
                TerminalValue::Failed,
                Some(CatalogCode::new(
                    "resource_exhausted",
                    "insufficient-next-charge",
                )),
            ),
            (
                ExactProbBuildIssue::BrokenInvariant,
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Cancelled(CancelCause::Requested),
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::PrecisionBudget {
                    lower: filament_exact::Rational::new(1_i64.into(), 3_i64.into())
                        .expect("a nonzero denominator"),
                    upper: filament_exact::Rational::new(2_i64.into(), 3_i64.into())
                        .expect("a nonzero denominator"),
                },
                TerminalValue::Incomplete(IncompleteCause::PrecisionBudget {
                    lower: Box::new(crate::proof_result::ExactRational::new(
                        filament_exact::Rational::new(1_i64.into(), 3_i64.into())
                            .expect("a nonzero denominator"),
                    )),
                    upper: Box::new(crate::proof_result::ExactRational::new(
                        filament_exact::Rational::new(2_i64.into(), 3_i64.into())
                            .expect("a nonzero denominator"),
                    )),
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: Some(ExpansionLimit::EvaluationMeter(
                        filament_exact::LimitKind::WorkUnits,
                    )),
                }),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: filament_exact::LimitKind::WorkUnits,
                    },
                    value: 4_096,
                    required: None,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::KernelRefused(
                    filament_exact::Refusal::IntegerOutOfDomain {
                        target: Box::new(IntegerInterval::spanning(
                            Integer::from(0_i64),
                            Integer::from(1_i64),
                        )),
                    },
                )),
                TerminalValue::Failed,
                Some(CatalogCode::new("integer_out_of_domain", "outside-domain")),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::KernelRefused(
                    filament_exact::Refusal::CheckedInvariant {
                        cause: filament_exact::CheckedInvariantCause::NativeIntegerCapacityBreach,
                    },
                )),
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::NotBoolean),
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Failed(CatalogCode::new(
                    "invalid_runtime_input",
                    "invalid-value",
                )),
                TerminalValue::Failed,
                Some(CatalogCode::new("invalid_runtime_input", "invalid-value")),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::Incomplete(
                    filament_exact::Incomplete {
                        limit_kind: filament_exact::LimitKind::WorkUnits,
                        limit: 9,
                        consumed: 9,
                        next_charge: filament_exact::Integer::from(1_i64),
                        charge_point: filament_exact::ChargePoint::TextInputBytes,
                    },
                )),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: filament_exact::LimitKind::WorkUnits,
                    },
                    value: 9,
                    required: None,
                }),
                None,
            ),
        ];

        for (issue, expected, diagnostic) in cases {
            let record = settle_exact_prob_build_issue(&issue, subject);
            assert_eq!(record.value(), &expected);
            assert_eq!(record.diagnostic(), diagnostic);
            assert_ne!(
                record.value().category(),
                filament_semantic_value::catalog::Category::Success
            );
        }
    }

    #[trace("TC-639", "FR-204-AC-2", "FR-204-AC-4")]
    #[test]
    fn digital_product_unsupported_causes_keep_their_wire_identities() {
        let meter = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units: 4_096,
            result_units: 64,
        };
        let subject = SubjectLimits {
            expansion: ExpansionLimits { max_candidates: 2 },
            meter: &meter,
        };
        let cases = [
            (
                UnavailabilityCause::StrictClockConstraint {
                    span: fsl_foundation::Span { start: 0, end: 1 },
                },
                "strict-clock-constraint",
            ),
            (
                UnavailabilityCause::DelayDistribution {
                    operation: "M::Call::send".to_owned(),
                },
                "delay-distribution",
            ),
            (
                UnavailabilityCause::ZeroDelayCycle {
                    states: vec!["state-1".to_owned()],
                },
                "zero-delay-cycle",
            ),
            (
                UnavailabilityCause::TimedFormShape {
                    span: fsl_foundation::Span { start: 0, end: 1 },
                    claim: None,
                    member: None,
                },
                "timed-form-shape",
            ),
        ];

        for (cause, wire_name) in cases {
            let record = settle_exact_prob_build_issue(
                &ExactProbBuildIssue::Unsupported(cause.clone()),
                subject,
            );
            assert_eq!(record.value(), &TerminalValue::Unsupported(cause.clone()));
            let TerminalValue::Unsupported(settled) = record.value() else {
                unreachable!("an unsupported build issue stays unsupported")
            };
            assert_eq!(settled.as_str(), wire_name);
        }
    }
}
