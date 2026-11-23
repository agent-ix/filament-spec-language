// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's item adapter (FR-281, FR-203) builds and solves the supported
//! probabilistic products, writes the corresponding FR-201 document, and
//! settles `proved` only after independent source-subject checking accepts it.
//!
//! Each closed document profile owns its graph, binding and workload admission.
//! Unsupported forms remain explicit unsupported dispositions. Missed bounds
//! settle `refuted` only when FR-202 replay independently reproduces the
//! native path set or subsystem certificate; a solved value alone never does.

mod candidate;
mod elapsed;
mod long_run;
mod measure;
mod subsystem;
mod undefined;

use filament_exact::{Cancel, ObjectReference, Rational};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_replay::statistical::SubjectLimits;
use fsl_replay::{
    check_probability_certificate_document, claim_binding, probability_proof_document,
    write_probability_certificate, write_probability_proof_document, ExactBinding,
    ExactProbBuildIssue, ExactProbBuildRecord, ExactProbabilityCertificate,
    ExactProbabilityCertificateCheck, ExactProbabilityGraphError, ExactProbabilityValue, Extremum,
    ModelCheckLimit, ProbabilityBoundSide, ProbabilityCertificateDocument,
    ProbabilityDocumentError, ProbabilityDocumentLimits, ProbabilityObjective,
    ProbabilityProductKey, TerminalRecord, TimedProbabilityForm, TimedProbabilityObjective,
    TimedProbabilityRoute,
};
use fsl_replay::{
    replay_probabilistic_witness, replay_probabilistic_witness_from_package,
    ProbabilisticCounterexample, ProbabilityWitnessError, ProbabilityWitnessPath,
    ProbabilityWitnessReplay, ProbabilityWitnessStep, ReproducedProbabilityWitness,
};
use fsl_replay::{CertificateLocus, CertificateRule};
use fsl_replay::{InconclusiveCause, ProbabilityCertificateLocus, ProbabilityCertificateRule};
use fsl_route::Mode;
use fsl_semantics::check::{
    Bound, Capability, DeclarationFamily, Event, ProbForm, ProbScheduler, ReachTarget,
};

use super::items::{admission, BROKEN, NOT_YET};
use super::timed_item::subject_refusal;
use super::{AnalyzeItem, EngineReport, Refusal};
use crate::exact_probabilistic::{
    build_retaining_monitor, decide_always, decide_bounded_event, decide_expected_reward,
    decide_reachability, decide_until, manifest as en5, settle_product_refusal, ExtRational,
    FhError, FhResult, ProbBuild, ProbProduct, ProductRefusal, ProductRequest, UnboundedError,
    UnboundedObjective,
};

/// The certificate EN-5 produced and the core's checker accepted for a
/// `proved` item (FR-281-AC-1: the record carries the certificate).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityEvidence {
    /// One exact value for every state of the checker's graph.
    pub certificate: ExactProbabilityCertificate,
    /// The checked value at the initial state.
    pub value: Rational,
    /// The authored bound side that was checked.
    pub side: ProbabilityBoundSide,
}

/// A reproduced path set's FR-286 spelling, written where the subject's system
/// is at hand.
pub type ProbabilityWire = Result<serde_json::Value, fsl_replay::CounterexampleWireError>;

/// A reproduced path set with its spelling.
pub(crate) type SpelledWitness = (Box<ReproducedProbabilityWitness>, ProbabilityWire);

/// What EN-5 reports for one item.
pub(crate) enum ExactReport {
    /// Only independently reproduced path evidence can settle refuted.
    Refuted(Box<ReproducedProbabilityWitness>, Box<ProbabilityWire>),
    /// A semantic replay disagreed with the produced path evidence.
    ReplayParity,
    /// Replay refused the witness's semantic content.
    ReplayRefused,
    /// The core rejected a probability value at a complete product state.
    Rejected {
        rule: CertificateRule,
        at: CertificateLocus,
    },
    /// The checker accepted the solver's certificate.
    Proved {
        /// The checker's acceptance, the only evidence a `proved` record
        /// is built from.
        check: ExactProbabilityCertificateCheck,
        /// The scheduler extremum the value is, absent for a workload.
        extremum: Option<Extremum>,
        /// The claim's one `over` instance, absent without `over`.
        binding: Option<ExactBinding>,
        /// The certificate the record carries.
        evidence: Option<Box<ExactProbabilityEvidence>>,
    },
    /// The checker accepted both documents of a sound interval (FR-1042):
    /// the claim side's and its counterpart's.
    ProvedBounds {
        /// The accepted document on the claim's side.
        claim: ExactProbabilityCertificateCheck,
        /// The accepted document of the other kind.
        counterpart: ExactProbabilityCertificateCheck,
        /// How the interval was produced.
        method: fsl_replay::BoundsMethod,
        /// The scheduler extremum the interval bounds, absent for a workload.
        extremum: Option<Extremum>,
        /// The claim's one `over` instance, absent without `over`.
        binding: Option<ExactBinding>,
        /// The claim side's certificate the record carries.
        evidence: Box<ExactProbabilityEvidence>,
    },
    /// A construction overflow or actual allocator refusal exhausted resources.
    ResourceExhausted,
    /// FR-203's map settled a stop of the build or the check.
    Settled(ExactProbBuildRecord),
}

impl ExactReport {
    /// The terminal record and the evidence it carries.
    pub(crate) fn settle(
        self,
        index: RequestIndex,
        backend: BackendId,
    ) -> (
        TerminalRecord,
        Option<Box<ExactProbabilityEvidence>>,
        Option<SpelledWitness>,
    ) {
        let (record, evidence) = match self {
            Self::Refuted(reproduced, wire) => {
                return (
                    TerminalRecord::refuted_by_probability_witness(
                        index,
                        backend,
                        &reproduced.evidence().item,
                        &reproduced,
                    )
                    .expect("the token's own claim matches"),
                    None,
                    Some((reproduced, *wire)),
                );
            }
            Self::ReplayParity => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::ReplayParity(fsl_replay::DisagreementCause::NoValue {
                        proved: fsl_replay::Verdict::from_category(
                            filament_semantic_value::catalog::Category::Violation,
                        ),
                        replayed: fsl_replay::Verdict::from_category(
                            filament_semantic_value::catalog::Category::Inconclusive,
                        ),
                    }),
                ),
                None,
            ),
            Self::ReplayRefused => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::ReplayRefused(
                        fsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                    ),
                ),
                None,
            ),
            Self::Rejected { rule, at } => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::CertificateRejected { rule, at },
                ),
                None,
            ),
            Self::Proved {
                check,
                extremum,
                binding,
                evidence,
            } => match TerminalRecord::proved_by_exact_certificate(
                index,
                backend.clone(),
                &check,
                extremum,
                binding,
            ) {
                Some(record) => (record, evidence),
                None => (TerminalRecord::failed_with(index, backend, BROKEN), None),
            },
            Self::ProvedBounds {
                claim,
                counterpart,
                method,
                extremum,
                binding,
                evidence,
            } => match TerminalRecord::proved_by_value_bounds(
                index,
                backend.clone(),
                &claim,
                &counterpart,
                method,
                extremum,
                binding,
            ) {
                Some(record) => (record, Some(evidence)),
                None => (TerminalRecord::failed_with(index, backend, BROKEN), None),
            },
            Self::ResourceExhausted => (
                TerminalRecord::incomplete(
                    index,
                    backend,
                    fsl_replay::IncompleteCause::ResourceExhausted,
                ),
                None,
            ),
            Self::Settled(record) => (record.into_terminal(index, backend), None),
        };
        (record, evidence, None)
    }
}

/// The `probabilistic-satisfaction` item that runs `request` on EN-5 and
/// settles it by FR-203. It is `Supported(EN-5)` until the caller's
/// negotiation says otherwise ([`AnalyzeItem::disposed`]).
pub fn exact_probabilistic_item<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, None, None),
    )
}

/// [`exact_probabilistic_item`] whose certificate `tamper` alters after the
/// solver built it and before the core's checker sees it: the way a test
/// shows a rejected certificate never settles `proved`.
#[cfg(feature = "test-support")]
pub fn exact_probabilistic_item_tampered<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
    tamper: impl Fn(&mut ExactProbabilityCertificate) + 'a,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, Some(&tamper), None),
    )
}

const ZERO_WEIGHT_COMPONENT: filament_semantic_value::catalog::CatalogCode =
    filament_semantic_value::catalog::CatalogCode::new(
        "unsupported_construct",
        "zero-weight-component",
    );

fn not_yet() -> EngineReport {
    EngineReport::Refused(Refusal::Unsupported(NOT_YET))
}

/// How a failed unbounded solve settles: a reached limit names itself, any
/// other fault is the engine's.
fn unbounded_failure(
    error: UnboundedError,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    match error {
        UnboundedError::MaxPolicyIterations => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                value: limits.max_policy_iterations,
            },
            subject,
        ),
        UnboundedError::MaxRationalBits => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                value: limits.max_rational_bits,
            },
            subject,
        ),
        _ => broken(),
    }
}

fn broken() -> EngineReport {
    EngineReport::Refused(Refusal::Failed(BROKEN))
}

/// A finite-horizon solve that held its probabilities only as outward-rounded
/// intervals for a form whose interval is not certified: the certificate carries
/// exact rationals, so the run stopped at the caller's `max_rational_bits`.
fn rational_bits_limit(
    limits: &crate::exact_probabilistic::ExactProbLimits,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    settled(
        &ExactProbBuildIssue::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: limits.max_rational_bits,
        },
        subject,
    )
}

/// How a failed claim-binding, claim-identity or checker-graph step settles: a
/// cancel or a reached state or edge budget names itself; the forms the bounded
/// graph does not build (a cycle, a multi-state draw, several initial states or
/// bindings, another claim form) are `not-yet-implemented`; the rest are the
/// engine's own faults.
fn graph_failure(
    error: ExactProbabilityGraphError,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    let configured = |bound: usize| u64::try_from(bound).unwrap_or(u64::MAX);
    match error {
        ExactProbabilityGraphError::Cancelled(cause) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        ExactProbabilityGraphError::LimitReached => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: configured(limits.model.limits.max_states),
            },
            subject,
        ),
        ExactProbabilityGraphError::EdgeLimitReached | ExactProbabilityGraphError::TooManyEdges => {
            settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxTransitions,
                    value: configured(limits.model.limits.max_transitions),
                },
                subject,
            )
        }
        ExactProbabilityGraphError::Cycle
        | ExactProbabilityGraphError::UnsupportedDraw
        | ExactProbabilityGraphError::UnsupportedInitial
        | ExactProbabilityGraphError::UnsupportedBinding
        | ExactProbabilityGraphError::UnsupportedClaimForm => not_yet(),
        ExactProbabilityGraphError::RequiresBound => EngineReport::Refused(Refusal::RequiresBound),
        ExactProbabilityGraphError::ExtentLimit => EngineReport::Refused(Refusal::Incomplete(
            fsl_replay::IncompleteCause::ResourceExhausted,
        )),
        ExactProbabilityGraphError::AutomatonStateLimit { limit } => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxAutomatonStates,
                value: limit,
            },
            subject,
        ),
        // A refusal whose cause the checker graph does not keep (a key that
        // will not encode, an undecided or refused expansion): not an engine
        // fault, since a model can cause it.
        ExactProbabilityGraphError::ExpansionRefused => not_yet(),
        ExactProbabilityGraphError::MissingSuccessor
        | ExactProbabilityGraphError::InvalidDigitalKey
        | ExactProbabilityGraphError::ClaimNotBound => broken(),
    }
}

/// How a failed qualified bounded monitor settles: a formula outside the
/// admitted shape is `not-yet-implemented`, the monitor's state budget is the
/// product's `max_automaton_states` (as the product build reports it), and any
/// other refusal is the engine's.
fn bounded_monitor_failure(
    error: fsl_eval::automaton::QualifiedBoundedError,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    use fsl_eval::automaton::QualifiedBoundedError;
    match error {
        QualifiedBoundedError::Translation(_) => not_yet(),
        QualifiedBoundedError::Automaton(error) => {
            build_refusal(crate::exact_probabilistic::automaton_refusal(error), subject)
        }
        QualifiedBoundedError::Key(_) => broken(),
    }
}

/// Dyadic refinement reached its maximum precision with the threshold still inside
/// `[lower, upper]`.
fn precision_budget(lower: Rational, upper: Rational, subject: SubjectLimits<'_>) -> EngineReport {
    settled(&ExactProbBuildIssue::PrecisionBudget { lower, upper }, subject)
}

fn settled(issue: &ExactProbBuildIssue, subject: SubjectLimits<'_>) -> EngineReport {
    EngineReport::Exact(Box::new(ExactReport::Settled(
        fsl_replay::settle_exact_prob_build_issue(issue, subject),
    )))
}

fn run_timed(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let Some(source_package) = simulation.source_package() else {
        return not_yet();
    };
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let bound = match &claim.form {
        ProbForm::Probability {
            bound,
            event:
                Event::Formula {
                    timed_horizon: Some(_),
                    ..
                },
        } => bound,
        _ => return not_yet(),
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, meter);
    let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let system = match admitted {
        Ok(system) => system,
        Err(failure) => return build_refusal(ProductRefusal::Admission(failure), subject),
    };
    let timed = match fsl_eval::simulation::model::TimedSystem::new(
        &system,
        claim.subject.alias.as_str(),
    ) {
        Ok(timed) => timed,
        Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
    };
    let route = if crate::exact_probabilistic::timed_region_route(&timed, claim) {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let certificate_scale =
        match crate::exact_probabilistic::timed_certificate_scale(&system, &timed, claim) {
            Ok(scale) => scale,
            Err(refusal) => return build_refusal(refusal, subject),
        };
    let product_request = ProductRequest {
        subject: request.subject,
        claim,
        limits,
    };
    let built = match build_retaining_monitor(product_request, cancel) {
        Ok(built) => built,
        Err(refusal) => return build_refusal(refusal, subject),
    };
    let (built, _) = built;
    let ProbBuild::Built(product) = built else {
        return not_yet();
    };
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    let mut interval: Option<Interval> = None;
    let (values, policy) = match decide_bounded_event(&product, bound, &limits) {
        Ok(FhResult::Exact { values, policy }) => (values, policy),
        Ok(FhResult::Bounds {
            lower,
            upper,
            precision_bits,
            policy,
        }) => {
            let values = lower.iter().map(|end| end.rational()).collect();
            interval = Some(Interval {
                lower,
                upper,
                precision_bits,
            });
            (values, policy)
        }
        Err(FhError::PolicyIterationLimit) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                    value: limits.max_policy_iterations,
                },
                subject,
            );
        }
        Ok(FhResult::PrecisionBudget { lower, upper, .. }) => {
            return precision_budget(lower.rational(), upper.rational(), subject);
        }
        Err(_) => return broken(),
    };
    let Some(value) = values.get(start.index()) else {
        return broken();
    };
    // An interval that does not straddle the threshold decides the claim: its
    // lower end meets a `>=` bound, its upper end a `<=` bound.
    let holds = match (&interval, bound) {
        (Some(interval), Bound::AtMost(threshold)) => interval
            .upper
            .get(start.index())
            .is_some_and(|end| end.rational() <= *threshold),
        (_, Bound::AtLeast(threshold)) => value >= threshold,
        (None, Bound::AtMost(threshold)) => value <= threshold,
    };
    let values = values
        .into_iter()
        .map(ExtRational::Finite)
        .collect::<Vec<_>>();
    let identity = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(identity) => identity,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    if !holds {
        // FR-202: `Bounds` never becomes a bounded refutation, so the exact
        // budget is what stopped the run.
        if interval.is_some() {
            return rational_bits_limit(&limits, subject);
        }
        return replay_timed_refutation(
            &system,
            claim,
            &product,
            &policy,
            *start,
            bound,
            binding.as_ref().map(|binding| &binding.reference),
            &limits,
            cancel,
            subject,
            tamper_witness,
        );
    }
    let mut targets = product
        .decided
        .iter()
        .filter(|(_, verdict)| *verdict == crate::exact_probabilistic::MonitorVerdict::Accept)
        .filter_map(|(state, _)| product.states[state.index()].certificate_state.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    // A workload leaves the scheduler no choice when the product is a chain
    // and the delay choices of FR-204 otherwise; the checker's own graph
    // decides which the document states, so the other spelling is the one
    // retry, and only when the checker names the spelling as the mismatch (a
    // route, unit, scale, form or target mismatch is a different error).
    let residual = match bound {
        Bound::AtLeast(_) => ProbabilityObjective::WorkloadMinimum,
        Bound::AtMost(_) => ProbabilityObjective::WorkloadMaximum,
    };
    let candidates = match &claim.scheduler {
        ProbScheduler::Every => vec![match bound {
            Bound::AtLeast(_) => ProbabilityObjective::Minimum,
            Bound::AtMost(_) => ProbabilityObjective::Maximum,
        }],
        ProbScheduler::Workload(_)
            if product.kind == crate::exact_probabilistic::ProductKind::Dtmc =>
        {
            vec![ProbabilityObjective::Workload, residual]
        }
        ProbScheduler::Workload(_) => vec![residual, ProbabilityObjective::Workload],
    };
    // Write `certificate` as an exact document, or as `side`'s end of an
    // interval, and have the checker accept or reject it.
    let verify = |certificate: &ExactProbabilityCertificate,
                  side: Option<fsl_replay::SubsystemBoundKind>,
                  limits: ProbabilityDocumentLimits| {
        let mut optimization = candidates[0];
        let mut checked = Err(ProbabilityDocumentError::OptimizationMismatch);
        for candidate in candidates.iter().copied() {
            optimization = candidate;
            let document = ProbabilityCertificateDocument {
                cyclic_dtmc: false,
                identity: identity.clone(),
                targets: targets.clone(),
                event: None,
                failures: Vec::new(),
                reward: None,
                optimization,
                timed: Some(TimedProbabilityObjective {
                    route,
                    unit: timed.source().unit_node.clone(),
                    scale: certificate_scale,
                    form: TimedProbabilityForm::Deadline,
                }),
                values: certificate.values.clone(),
                ranking: Vec::new(),
            };
            checked = (|| {
                let bytes = match side {
                    Some(side) => {
                        fsl_replay::write_value_bounds_certificate(&document, side, limits, cancel)?
                    }
                    None => write_probability_certificate(&document, limits, cancel)?,
                };
                check_probability_certificate_document(
                    &system,
                    source_package.graph().probabilistic_model().claims(),
                    &identity,
                    &bytes,
                    limits,
                    cancel,
                )
            })();
            if !matches!(checked, Err(ProbabilityDocumentError::OptimizationMismatch)) {
                break;
            }
        }
        (checked, optimization)
    };
    let extremum_of = |optimization| match optimization {
        ProbabilityObjective::Minimum | ProbabilityObjective::WorkloadMinimum => {
            Some(Extremum::Min)
        }
        ProbabilityObjective::Maximum | ProbabilityObjective::WorkloadMaximum => {
            Some(Extremum::Max)
        }
        ProbabilityObjective::Workload => None,
    };
    if let Some(first) = interval {
        return raise_until_accepted(&product, bound, &limits, first, |interval| {
            let mut extremum = None;
            let mut documents = Vec::with_capacity(2);
            for (side, ends) in [
                (fsl_replay::SubsystemBoundKind::Lower, &interval.lower),
                (fsl_replay::SubsystemBoundKind::Upper, &interval.upper),
            ] {
                let ends = ends
                    .iter()
                    .map(|end| ExtRational::Finite(end.rational()))
                    .collect::<Vec<_>>();
                let mut certificate = match certificate_of(
                    &product,
                    None,
                    &ends,
                    fsl_replay::ProbabilityDocumentIoLimits {
                        bytes: document_limits.bytes,
                        storage_bytes: document_limits.storage_bytes,
                    },
                    cancel,
                ) {
                    Ok(certificate) => certificate,
                    Err(error) => return Ok(document_failure(error, subject)),
                };
                if let Some(tamper) = tamper {
                    tamper(&mut certificate);
                }
                let (checked, optimization) = verify(&certificate, Some(side), document_limits);
                let claim_side = matches!(
                    (side, bound),
                    (fsl_replay::SubsystemBoundKind::Lower, Bound::AtLeast(_))
                        | (fsl_replay::SubsystemBoundKind::Upper, Bound::AtMost(_))
                );
                if claim_side {
                    extremum = extremum_of(optimization);
                }
                documents.push(checked.map(|check| (certificate, check)));
            }
            let mut documents = documents.into_iter();
            let (Some(lower), Some(upper)) = (documents.next(), documents.next()) else {
                return Ok(broken());
            };
            finish_interval(
                bound,
                lower,
                upper,
                binding.as_ref(),
                extremum,
                interval.precision_bits,
                subject,
            )
        });
    }
    let mut certificate = match certificate_of(
        &product,
        None,
        &values,
        fsl_replay::ProbabilityDocumentIoLimits {
            bytes: document_limits.bytes,
            storage_bytes: document_limits.storage_bytes,
        },
        cancel,
    ) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let (checked, optimization) = verify(&certificate, None, document_limits);
    checked_report(
        checked,
        certificate,
        binding,
        extremum_of(optimization),
        subject,
    )
}

fn mean_time_ranking(
    product: &ProbProduct,
    targets: &[bool],
    included: &[bool],
    policy: &[Option<usize>],
) -> Option<Vec<(ProbabilityProductKey, u64)>> {
    if targets.len() != product.states.len()
        || included.len() != product.states.len()
        || policy.len() != product.states.len()
    {
        return None;
    }
    let mut ranks = vec![None; product.states.len()];
    for (index, target) in targets.iter().enumerate() {
        if included[index] && *target {
            ranks[index] = Some(0_u64);
        }
    }
    while included
        .iter()
        .zip(&ranks)
        .any(|(included, rank)| *included && rank.is_none())
    {
        let mut changed = false;
        for index in 0..ranks.len() {
            if !included[index] || ranks[index].is_some() {
                continue;
            }
            let outgoing = product.outgoing(product.ids().nth(index)?);
            let mut choices: Vec<(&crate::exact_probabilistic::ProductAction, Vec<u64>)> =
                Vec::new();
            for edge in outgoing {
                let choice = match choices
                    .iter_mut()
                    .find(|(choice, _)| same_mean_time_choice(choice, &edge.choice))
                {
                    Some((_, choice)) => choice,
                    None => {
                        choices.push((&edge.choice, Vec::new()));
                        &mut choices.last_mut()?.1
                    }
                };
                if edge.probability > Rational::from_integer(0_i64.into())
                    && included.get(edge.to.index()).copied().unwrap_or(false)
                {
                    if let Some(next_rank) = ranks.get(edge.to.index()).copied().flatten() {
                        choice.push(next_rank);
                    }
                }
            }
            let Some(selected) = policy[index] else {
                continue;
            };
            let (_, lower) = choices.get(selected)?;
            let Some(lower) = lower.iter().copied().min() else {
                continue;
            };
            ranks[index] = Some(lower.checked_add(1)?);
            changed = true;
        }
        if !changed {
            return None;
        }
    }
    let mut entries = product
        .states
        .iter()
        .zip(ranks)
        .zip(included)
        .filter(|(_, included)| **included)
        .map(|((state, rank), _)| Some((state.certificate_state.clone()?, rank?)))
        .collect::<Option<Vec<_>>>()?;
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Some(entries)
}

fn mean_time_reachable_before_target(product: &ProbProduct, targets: &[bool]) -> Option<Vec<bool>> {
    if targets.len() != product.states.len() {
        return None;
    }
    let mut included = vec![false; product.states.len()];
    let mut pending = product
        .initial
        .iter()
        .map(|(_, _, state)| state.index())
        .collect::<Vec<_>>();
    while let Some(index) = pending.pop() {
        let seen = included.get_mut(index)?;
        if *seen {
            continue;
        }
        *seen = true;
        if targets[index] {
            continue;
        }
        for edge in product.outgoing(product.ids().nth(index)?) {
            pending.push(edge.to.index());
        }
    }
    Some(included)
}

fn same_mean_time_choice(
    left: &crate::exact_probabilistic::ProductAction,
    right: &crate::exact_probabilistic::ProductAction,
) -> bool {
    use crate::exact_probabilistic::ProductAction;
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(_), ProductAction::PostState(_))
        | (ProductAction::TerminalStutter, ProductAction::TerminalStutter)
        | (ProductAction::UnitDelay, ProductAction::UnitDelay)
        | (ProductAction::EnterIdleDelay, ProductAction::EnterIdleDelay)
        | (ProductAction::IdleDelay, ProductAction::IdleDelay)
        | (ProductAction::IdleObserve, ProductAction::IdleObserve) => true,
        (
            ProductAction::WorkloadResolve {
                identity: left_identity,
                delay: left_delay,
            },
            ProductAction::WorkloadResolve {
                identity: right_identity,
                delay: right_delay,
            },
        ) => left_identity == right_identity && left_delay == right_delay,
        (ProductAction::FreeDelays(left), ProductAction::FreeDelays(right)) => left == right,
        _ => false,
    }
}

fn run_timed_mean(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
) -> EngineReport {
    let claim = request.claim;
    let ProbForm::MeanTime { bound, .. } = &claim.form else {
        return not_yet();
    };
    if !matches!(claim.scheduler, ProbScheduler::Every) || !claim.fairness.is_empty() {
        return not_yet();
    }
    let simulation = request.subject.simulation.clone();
    let Some(source_package) = simulation.source_package() else {
        return not_yet();
    };
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, meter);
    let system = match ModelSystem::new_with_cancel(simulation.clone(), cancel) {
        Ok(system) => system,
        Err(failure) => return build_refusal(ProductRefusal::Admission(failure), subject),
    };
    let timed = match fsl_eval::simulation::model::TimedSystem::new(
        &system,
        claim.subject.alias.as_str(),
    ) {
        Ok(timed) => timed,
        Err(refusal) => return EngineReport::Refused(subject_refusal(&refusal)),
    };
    let route = if crate::exact_probabilistic::timed_region_route(&timed, claim) {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let scale = match crate::exact_probabilistic::timed_certificate_scale(&system, &timed, claim) {
        Ok(scale) => scale,
        Err(refusal) => {
            return build_refusal(refusal, subject);
        }
    };
    let built = match build_retaining_monitor(
        ProductRequest {
            subject: request.subject,
            claim,
            limits,
        },
        cancel,
    ) {
        Ok((built, _)) => built,
        Err(refusal) => {
            return build_refusal(refusal, subject);
        }
    };
    let ProbBuild::Built(product) = built else {
        return not_yet();
    };
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    let target = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect::<Vec<_>>();
    if target.len() != product.states.len() {
        return broken();
    }
    let (objective, extremum) = match bound {
        Bound::AtLeast(_) => (UnboundedObjective::Min, Some(Extremum::Min)),
        Bound::AtMost(_) => (UnboundedObjective::Max, Some(Extremum::Max)),
    };
    let solved = match decide_expected_reward(
        &product,
        &target,
        crate::exact_probabilistic::ELAPSED_REWARD,
        objective,
        &limits,
    ) {
        Ok(solved) => solved,
        Err(UnboundedError::MaxPolicyIterations) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                    value: limits.max_policy_iterations,
                },
                subject,
            );
        }
        Err(UnboundedError::MaxRationalBits) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxRationalBits,
                    value: limits.max_rational_bits,
                },
                subject,
            );
        }
        Err(_) => {
            return broken();
        }
    };
    if solved
        .values
        .iter()
        .any(|value| matches!(value, ExtRational::PlusInfinity))
    {
        return document_failure(ProbabilityDocumentError::MixedInfiniteCoverage, subject);
    }
    let Some(ExtRational::Finite(value)) = solved.values.get(start.index()) else {
        return not_yet();
    };
    let holds = match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    };
    if !holds {
        return not_yet();
    }
    let identity = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(identity) => identity,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let values = solved.values;
    let mut certificate = match certificate_of(
        &product,
        None,
        &values,
        fsl_replay::ProbabilityDocumentIoLimits {
            bytes: document_limits.bytes,
            storage_bytes: document_limits.storage_bytes,
        },
        cancel,
    ) {
        Ok(certificate) => certificate,
        Err(error) => {
            return document_failure(error, subject);
        }
    };
    let included = match mean_time_reachable_before_target(&product, &target) {
        Some(included) => included,
        None => return broken(),
    };
    let included_keys = product
        .states
        .iter()
        .zip(&included)
        .filter(|(_, included)| **included)
        .filter_map(|(state, _)| state.certificate_state.clone())
        .collect::<std::collections::BTreeSet<_>>();
    certificate
        .values
        .retain(|value| included_keys.contains(&value.state));
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let mut targets = product
        .labels
        .iter()
        .enumerate()
        .filter(|(index, labels)| included[*index] && labels.first().copied().unwrap_or(false))
        .filter_map(|(index, _)| product.states[index].certificate_state.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    let ranking = match mean_time_ranking(&product, &target, &included, &solved.policy) {
        Some(ranking) => ranking,
        None => return not_yet(),
    };
    let optimization = match objective {
        UnboundedObjective::Min => ProbabilityObjective::Minimum,
        UnboundedObjective::Max => ProbabilityObjective::Maximum,
    };
    let document = ProbabilityCertificateDocument {
        cyclic_dtmc: false,
        identity: identity.clone(),
        targets,
        event: None,
        failures: Vec::new(),
        reward: None,
        optimization,
        timed: Some(TimedProbabilityObjective {
            route,
            unit: timed.source().unit_node.clone(),
            scale,
            form: TimedProbabilityForm::ExpectedElapsed,
        }),
        values: certificate.values.clone(),
        ranking,
    };
    let checked = (|| {
        let bytes = write_probability_certificate(&document, document_limits, cancel)?;
        check_probability_certificate_document(
            &system,
            source_package.graph().probabilistic_model().claims(),
            &identity,
            &bytes,
            document_limits,
            cancel,
        )
    })();
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    checked_report(checked, certificate, binding, extremum, subject)
}

pub(super) fn run_package_exact(request: ProductRequest<'_>, cancel: &Cancel) -> EngineReport {
    run(request, cancel, None, None)
}

fn run(
    mut request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    if matches!(
        &request.claim.form,
        ProbForm::Quantile {
            measure: fsl_semantics::check::BoundedMeasure::Elapsed { .. },
            ..
        }
    ) {
        return elapsed::run(request, cancel, tamper);
    }
    if request.claim.bounded_measure() {
        return measure::run(request, cancel, tamper, tamper_witness);
    }
    if matches!(&request.claim.form, ProbForm::MeanTime { .. }) {
        return run_timed_mean(request, cancel, tamper);
    }
    if matches!(
        &request.claim.form,
        ProbForm::Probability {
            event: Event::Formula {
                timed_horizon: Some(_),
                ..
            },
            ..
        }
    ) {
        return run_timed(request, cancel, tamper, tamper_witness);
    }
    if matches!(request.claim.form, ProbForm::LongRunFraction { .. }) {
        return long_run::run(request, cancel, tamper, tamper_witness);
    }
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let probabilistic = simulation.input.probabilistic;
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let (bound, bounded_formula) = match &claim.form {
        ProbForm::Reach {
            bound,
            target: ReachTarget::Eventually(_) | ReachTarget::Until(..) | ReachTarget::Always(_),
        } => (bound, None),
        ProbForm::ExpectedReward { bound, .. } => (bound, None),
        ProbForm::Probability {
            bound,
            event:
                Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
        } => (bound, Some(formula)),
        _ => return not_yet(),
    };
    let state_predicate = matches!(
        &claim.form,
        ProbForm::Reach {
            target: ReachTarget::Until(..) | ReachTarget::Always(_),
            ..
        }
    );
    let expected_reward = matches!(&claim.form, ProbForm::ExpectedReward { .. });
    if (state_predicate || expected_reward)
        && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
    {
        return not_yet();
    }
    let bounded_fair = !claim.fairness.is_empty()
        && bounded_formula.is_some()
        && matches!(claim.scheduler, ProbScheduler::Every);
    if !claim.fairness.is_empty() && !bounded_fair {
        return not_yet();
    }
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, meter);
    let reward_system = if expected_reward {
        let item = match fsl_replay::probabilistic_claim_identity(claim) {
            Ok(item) => item,
            Err(error) => return graph_failure(error, &limits, subject),
        };
        let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
        if let Some(cause) = cancel.tripped() {
            return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
        }
        let system = match admitted {
            Ok(system) => system,
            Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
        };
        if let Err(error) = fsl_replay::admit_expected_reward_certificate_subject(
            &system,
            probabilistic.claims(),
            &item,
            document_limits,
            cancel,
        ) {
            return document_failure(error, subject);
        }
        Some(system)
    } else {
        None
    };
    if let Some(formula) = bounded_formula {
        let Some(profile) =
            fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        else {
            // The checker refuses a non-temporal profile (`not-temporal-profile`,
            // fsl-semantics probabilistic/claim.rs), so a checked claim has one.
            return broken();
        };
        if !fsl_eval::automaton::QualifiedBoundedMonitor::supports_observations(
            formula.formula(),
            profile,
        ) {
            return not_yet();
        }
    }
    let (objective, extremum) = match (&claim.scheduler, bound) {
        (ProbScheduler::Every, Bound::AtLeast(_)) => (UnboundedObjective::Min, Some(Extremum::Min)),
        (ProbScheduler::Every, Bound::AtMost(_)) => (UnboundedObjective::Max, Some(Extremum::Max)),
        (ProbScheduler::Workload(_), _) => (UnboundedObjective::Min, None),
    };

    let eventual = matches!(
        claim.form,
        ProbForm::Reach {
            target: ReachTarget::Eventually(_),
            ..
        }
    );
    let mut candidate_budget = (expected_reward || eventual || bounded_fair)
        .then(|| candidate::CandidateBudget::new(document_limits, cancel));
    let built = if let Some(budget) = candidate_budget.as_mut() {
        use crate::exact_probabilistic::{
            build_retaining_monitor_with_budget, BudgetedProductError,
        };
        let identity = budget.identity;
        let mut admission = budget.admission();
        let proof_bounds = std::mem::take(&mut request.subject.proof_bounds);
        match build_retaining_monitor_with_budget(request, identity, &mut admission, cancel) {
            Ok(built) => built,
            Err(BudgetedProductError::Product(error)) => return build_refusal(error, subject),
            Err(BudgetedProductError::Key(error)) => {
                return document_failure(admission.refusal(error), subject);
            }
            Err(BudgetedProductError::Undefined(_)) => {
                // The proof-only budgeted builder does not retain a failing path.
                // Re-explore only that case under the same finite state, edge,
                // expansion and evaluation limits; keys are bounded by the
                // model identity owner, then admitted again by native replay.
                let undefined_request = ProductRequest {
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation: simulation.clone(),
                        proof_bounds,
                    },
                    claim,
                    limits,
                };
                match build_retaining_monitor(undefined_request, cancel) {
                    Ok(built) => built,
                    Err(error) => return build_refusal(error, subject),
                }
            }
        }
    } else {
        match build_retaining_monitor(request, cancel) {
            Ok(built) => built,
            Err(error) => return build_refusal(error, subject),
        }
    };
    let (product, monitor) = match built {
        (ProbBuild::Built(product), monitor) => (product, monitor),
        (ProbBuild::Undefined(path), monitor) => {
            return undefined::run(
                simulation,
                claim,
                path,
                monitor,
                &limits,
                cancel,
                subject,
                tamper_witness,
            )
        }
    };
    if let Some(formula) = bounded_formula {
        let Some(profile) =
            fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        else {
            // The checker refuses a non-temporal profile (`not-temporal-profile`,
            // fsl-semantics probabilistic/claim.rs), so a checked claim has one.
            return broken();
        };
        if !fsl_eval::automaton::QualifiedBoundedMonitor::supports(formula.formula(), profile) {
            return not_yet();
        }
    }
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    if bounded_formula.is_some()
        && product
            .states
            .iter()
            .any(|state| state.pending.is_some() || state.accumulator.is_some())
    {
        return not_yet();
    }
    let target: Vec<bool> = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect();
    let mut interval: Option<Interval> = None;
    let (values, policy) = if bounded_formula.is_some() {
        let outcome = if bounded_fair {
            crate::exact_probabilistic::decide_fair_bounded_event(&product, bound, &limits)
                .map(|result| result.outcome)
        } else {
            decide_bounded_event(&product, bound, &limits)
        };
        match outcome {
            Ok(FhResult::Exact { values, policy }) => (
                values.into_iter().map(ExtRational::Finite).collect(),
                policy,
            ),
            Ok(FhResult::Bounds {
                lower,
                upper,
                precision_bits,
                policy,
            }) if !bounded_fair && candidate_budget.is_none() => {
                let values = lower
                    .iter()
                    .map(|end| ExtRational::Finite(end.rational()))
                    .collect();
                interval = Some(Interval {
                    lower,
                    upper,
                    precision_bits,
                });
                (values, policy)
            }
            Ok(FhResult::Bounds { .. }) => return rational_bits_limit(&limits, subject),
            Ok(FhResult::PrecisionBudget { lower, upper, .. }) => {
                return precision_budget(lower.rational(), upper.rational(), subject);
            }
            Err(FhError::PolicyIterationLimit) => {
                return settled(
                    &ExactProbBuildIssue::Limit {
                        limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                        value: limits.max_policy_iterations,
                    },
                    subject,
                );
            }
            Err(_) => return broken(),
        }
    } else {
        let result = match &claim.form {
            ProbForm::Reach {
                target: ReachTarget::Until(..),
                ..
            } => {
                let until = product
                    .labels
                    .iter()
                    .map(|labels| labels.get(1).copied().unwrap_or(false))
                    .collect::<Vec<_>>();
                decide_until(&product, &target, &until, objective, &limits)
            }
            ProbForm::Reach {
                target: ReachTarget::Always(_),
                ..
            } => decide_always(&product, &target, objective, &limits),
            ProbForm::ExpectedReward { reward, .. } => {
                decide_expected_reward(&product, &target, reward.as_str(), objective, &limits)
            }
            _ => decide_reachability(&product, &target, objective, &limits),
        };
        let solved = match result {
            Ok(solved) => solved,
            Err(error) => return unbounded_failure(error, &limits, subject),
        };
        (solved.values, solved.policy)
    };
    // A positive-infinite expectation fails a finite `<=` bound (FR-203-AC-15);
    // it is refuted only through a replayed finite lower certificate.
    let holds = match (values.get(start.index()), bound) {
        _ if interval.is_some() => {
            let ends = interval.as_ref().and_then(|interval| {
                Some((
                    interval.lower.get(start.index())?.rational(),
                    interval.upper.get(start.index())?.rational(),
                ))
            });
            let Some((lower, upper)) = ends else {
                return broken();
            };
            match bound {
                Bound::AtLeast(threshold) => lower >= *threshold,
                Bound::AtMost(threshold) => upper <= *threshold,
            }
        }
        (Some(ExtRational::Finite(value)), Bound::AtLeast(threshold)) => value >= threshold,
        (Some(ExtRational::Finite(value)), Bound::AtMost(threshold)) => value <= threshold,
        (Some(ExtRational::PlusInfinity), Bound::AtMost(_)) if expected_reward => false,
        (Some(ExtRational::PlusInfinity), _) if expected_reward => {
            return document_failure(ProbabilityDocumentError::MixedInfiniteCoverage, subject);
        }
        _ => return broken(),
    };
    if holds
        && expected_reward
        && values
            .iter()
            .any(|value| matches!(value, ExtRational::PlusInfinity))
    {
        return document_failure(ProbabilityDocumentError::MixedInfiniteCoverage, subject);
    }
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let admitted = match reward_system {
        Some(system) => Ok(system),
        None => ModelSystem::new_with_cancel(simulation, cancel),
    };
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let Ok(system) = admitted else {
        return broken();
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let reference = binding.as_ref().map(|bound| &bound.reference);
    if !holds {
        if interval.is_some() {
            return rational_bits_limit(&limits, subject);
        }
        if bounded_fair {
            return not_yet();
        }
        let mut own_budget = candidate::CandidateBudget::new(document_limits, cancel);
        let mut subsystem_refutation = || {
            let budget = match candidate_budget.as_mut() {
                Some(budget) => budget,
                None => &mut own_budget,
            };
            subsystem::refute(
                &system,
                claim,
                &product,
                &policy,
                *start,
                &values,
                reference,
                &limits,
                cancel,
                subject,
                budget,
                tamper_witness,
            )
        };
        if expected_reward {
            return subsystem_refutation();
        }
        // The path-set search never starts on a cyclic subject (FR-202): an
        // unwindowed `eventually`, `always` or `until` there is refuted by
        // the event-avoidance lower certificate for its violation.
        if let ProbForm::Reach { .. } = &claim.form {
            match fsl_replay::admit_acyclic_probabilistic_refutation_subject(
                &system, &limits, cancel,
            ) {
                Ok(true) => {}
                Ok(false) => return subsystem_refutation(),
                Err(error) => return document_failure(error, subject),
            }
        }
        let bounded_monitor = match (bounded_formula, monitor) {
            (Some(formula), Some(monitor)) => {
                let context =
                    match fsl_replay::probabilistic_formula_monitor_context(claim, reference) {
                        Ok(context) => context,
                        Err(error) => return graph_failure(error, &limits, subject),
                    };
                match fsl_eval::automaton::QualifiedBoundedMonitor::from_monitor(
                    formula.formula(),
                    &context,
                    monitor,
                    limits.model.max_automaton_states,
                ) {
                    Ok(monitor) => Some(monitor),
                    Err(error) => return bounded_monitor_failure(error, subject),
                }
            }
            (None, _) => None,
            (Some(_), None) => return broken(),
        };
        return replay_refutation(
            &system,
            claim,
            &product,
            &policy,
            *start,
            &target,
            reference,
            bounded_monitor.as_ref(),
            &limits,
            cancel,
            subject,
            tamper_witness,
        );
    }
    if let Some(interval) = interval {
        let (Some(formula), Some(monitor)) = (bounded_formula, monitor) else {
            return broken();
        };
        let context = match fsl_replay::probabilistic_formula_monitor_context(claim, reference) {
            Ok(context) => context,
            Err(error) => return graph_failure(error, &limits, subject),
        };
        let monitor = match fsl_eval::automaton::QualifiedBoundedMonitor::from_monitor(
            formula.formula(),
            &context,
            monitor,
            limits.model.max_automaton_states,
        ) {
            Ok(monitor) => monitor,
            Err(error) => return bounded_monitor_failure(error, subject),
        };
        let subject_of = IntervalSubject {
            system: &system,
            claims: probabilistic.claims(),
            item: &item,
            product: &product,
            reference,
            monitor: &monitor,
            bound,
            extremum,
            binding: binding.as_ref(),
        };
        return raise_until_accepted(&product, bound, &limits, interval, |interval| {
            settle_interval(
                &subject_of,
                interval,
                document_limits,
                cancel,
                tamper,
                subject,
            )
        });
    }
    let certificate = match (bounded_formula, monitor) {
        (Some(formula), Some(monitor)) => {
            let context = match fsl_replay::probabilistic_formula_monitor_context(claim, reference)
            {
                Ok(context) => context,
                Err(error) => return graph_failure(error, &limits, subject),
            };
            let monitor = match fsl_eval::automaton::QualifiedBoundedMonitor::from_monitor(
                formula.formula(),
                &context,
                monitor,
                limits.model.max_automaton_states,
            ) {
                Ok(monitor) => monitor,
                Err(error) => return bounded_monitor_failure(error, subject),
            };
            bounded_certificate_of(
                &product,
                reference,
                &values,
                &monitor,
                fsl_replay::ProbabilityDocumentIoLimits {
                    bytes: document_limits.bytes,
                    storage_bytes: document_limits.storage_bytes,
                },
                cancel,
                candidate_budget.as_mut(),
            )
        }
        (None, _) if candidate_budget.is_some() => candidate::values(
            &product,
            reference,
            &values,
            candidate_budget
                .as_mut()
                .expect("quantitative candidate budget"),
        ),
        (None, _) => certificate_of(
            &product,
            reference,
            &values,
            if state_predicate || expected_reward {
                fsl_replay::ProbabilityDocumentIoLimits {
                    bytes: document_limits.bytes,
                    storage_bytes: document_limits.storage_bytes,
                }
            } else {
                fsl_replay::ProbabilityDocumentIoLimits::default()
            },
            cancel,
        ),
        (Some(_), None) => return broken(),
    };
    let mut certificate = match certificate {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let checked = (|| {
        let claims = probabilistic.claims();
        let document = probability_proof_document(
            &system,
            claims,
            &item,
            &certificate,
            document_limits,
            cancel,
        )?;
        certificate.values = document.document().values.clone();
        let bytes = write_probability_proof_document(&document, document_limits, cancel)?;
        let check = check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            document_limits,
            cancel,
        )?;
        Ok::<_, ProbabilityDocumentError>(check)
    })();
    checked_report(checked, certificate, binding, extremum, subject)
}

fn checked_report(
    checked: Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError>,
    certificate: ExactProbabilityCertificate,
    binding: Option<fsl_replay::ClaimBinding>,
    extremum: Option<Extremum>,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    match checked {
        Ok(check) => match &check {
            ExactProbabilityCertificateCheck::Accepted(accepted) => {
                let evidence = Box::new(ExactProbabilityEvidence {
                    certificate,
                    value: accepted.value().clone(),
                    side: accepted.side(),
                });
                let binding = binding.map(|bound| ExactBinding {
                    population: bound.instance.population.node,
                    key: bound.instance.key,
                });
                EngineReport::Exact(Box::new(ExactReport::Proved {
                    check,
                    extremum,
                    binding,
                    evidence: Some(evidence),
                }))
            }
            ExactProbabilityCertificateCheck::AcceptedVacuous(_) => {
                EngineReport::Exact(Box::new(ExactReport::Proved {
                    check,
                    extremum: None,
                    binding: None,
                    evidence: None,
                }))
            }
            ExactProbabilityCertificateCheck::Rejected(rejection) => {
                let rule = match rejection.rule {
                    ProbabilityCertificateRule::InequalityFails => CertificateRule::InequalityFails,
                    ProbabilityCertificateRule::ThresholdMissed => CertificateRule::ThresholdMissed,
                    ProbabilityCertificateRule::Malformed => CertificateRule::Malformed,
                    ProbabilityCertificateRule::RankMissing => CertificateRule::RankMissing,
                    ProbabilityCertificateRule::RankBroken => CertificateRule::RankBroken,
                };
                match &rejection.at {
                    ProbabilityCertificateLocus::ProductState(at) => {
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: CertificateLocus::ProbabilityProductState(at.clone()),
                        }))
                    }
                    ProbabilityCertificateLocus::ProductPart { key, part } => {
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: CertificateLocus::ProbabilityProductPart {
                                key: key.clone(),
                                part: *part,
                            },
                        }))
                    }
                    ProbabilityCertificateLocus::Member(name) => {
                        let Some(member) = fsl_replay::CertificateMember::ALL
                            .iter()
                            .find(|member| member.as_str() == *name)
                        else {
                            return broken();
                        };
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: CertificateLocus::CertificateMember(*member),
                        }))
                    }
                }
            }
        },
        Err(error) => document_failure(error, subject),
    }
}

fn document_failure(error: ProbabilityDocumentError, subject: SubjectLimits<'_>) -> EngineReport {
    match error {
        ProbabilityDocumentError::Cancelled(cause) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        ProbabilityDocumentError::Limit { limit, bound, .. } => settled(
            &ExactProbBuildIssue::Limit {
                limit: limit.model_limit(),
                value: bound,
            },
            subject,
        ),
        ProbabilityDocumentError::Accounting(incomplete) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: incomplete.limit_kind,
                },
                value: incomplete.limit,
            },
            subject,
        ),
        ProbabilityDocumentError::IntegerText { bit_bound, .. } => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                value: bit_bound,
            },
            subject,
        ),
        ProbabilityDocumentError::KeyEncoding(fsl_eval::simulation::EncodingRefusal::Limit(
            limit,
        )) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::IdentityInputBytes,
                value: limit.configured_bound(),
            },
            subject,
        ),
        ProbabilityDocumentError::KeyEncoding(
            fsl_eval::simulation::EncodingRefusal::Allocation { .. },
        )
        | ProbabilityDocumentError::KeySizeOverflow => {
            EngineReport::Exact(Box::new(ExactReport::ResourceExhausted))
        }
        ProbabilityDocumentError::KeyEncoding(
            fsl_eval::simulation::EncodingRefusal::NoEncoding(_),
        ) => EngineReport::Refused(Refusal::Declined(
            fsl_replay::ProofRefusalCause::InvalidInput,
            fsl_replay::DeclineCode::Fsl(fsl_foundation::diagnostic::Code::InvalidRuntimeInput),
            filament_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "invalid-value",
            ),
        )),
        ProbabilityDocumentError::KeyDenied => broken(),
        ProbabilityDocumentError::Build(issue) => settled(&issue, subject),
        ProbabilityDocumentError::ZeroWeightComponent { .. } => {
            EngineReport::Refused(Refusal::Unsupported(ZERO_WEIGHT_COMPONENT))
        }
        ProbabilityDocumentError::WorkloadFreeDelays(refusal) => {
            EngineReport::Refused(Refusal::Unsupported(refusal.code()))
        }
        ProbabilityDocumentError::MixedInfiniteCoverage => EngineReport::Refused(
            Refusal::Unsupported(filament_semantic_value::catalog::CatalogCode::new(
                "unsupported_construct",
                "mixed-infinite-coverage",
            )),
        ),
        ProbabilityDocumentError::Unsupported => not_yet(),
        // The product build reports a first undefined read itself (`ProbBuild::Undefined`);
        // one the checker finds after a build that saw none is a disagreement between them.
        ProbabilityDocumentError::UndefinedPredicate(_)
        | ProbabilityDocumentError::MalformedMember(_)
        | ProbabilityDocumentError::Malformed
        | ProbabilityDocumentError::MissingMember(_)
        | ProbabilityDocumentError::Noncanonical
        | ProbabilityDocumentError::IdentityMismatch
        | ProbabilityDocumentError::ObjectiveMismatch
        | ProbabilityDocumentError::OptimizationMismatch
        | ProbabilityDocumentError::Graph(_) => broken(),
    }
}

/// A finite-horizon interval EN-5 certifies at both ends instead of writing
/// it as an exact rational (FR-1042).
struct Interval {
    lower: Vec<crate::exact_probabilistic::Dyadic>,
    upper: Vec<crate::exact_probabilistic::Dyadic>,
    precision_bits: u32,
}

/// What a bounded interval's two documents are written and checked over.
struct IntervalSubject<'a> {
    system: &'a ModelSystem<'a>,
    claims: &'a [fsl_semantics::check::CheckedProbabilisticClaim],
    item: &'a fsl_package::ObligationIdentity,
    product: &'a ProbProduct,
    reference: Option<&'a ObjectReference>,
    monitor: &'a fsl_eval::automaton::QualifiedBoundedMonitor,
    bound: &'a Bound,
    extremum: Option<Extremum>,
    binding: Option<&'a fsl_replay::ClaimBinding>,
}

/// FR-1042: settle an interval's documents, and when a self-checked document
/// is rejected raise the precision (doubling, up to `max_precision_bits`) and
/// try again. The last rejection settles when the cap is reached.
fn raise_until_accepted(
    product: &ProbProduct,
    bound: &Bound,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    mut interval: Interval,
    mut settle: impl FnMut(&Interval) -> Result<EngineReport, EngineReport>,
) -> EngineReport {
    loop {
        let rejected = match settle(&interval) {
            Ok(report) => return report,
            Err(rejected) => rejected,
        };
        let raised = interval
            .precision_bits
            .saturating_mul(2)
            .min(limits.max_precision_bits);
        if raised <= interval.precision_bits {
            return rejected;
        }
        let next = crate::exact_probabilistic::ExactProbLimits {
            precision_bits: raised,
            ..limits.clone()
        };
        match decide_bounded_event(product, bound, &next) {
            Ok(FhResult::Bounds {
                lower,
                upper,
                precision_bits,
                ..
            }) if precision_bits > interval.precision_bits => {
                interval = Interval {
                    lower,
                    upper,
                    precision_bits,
                };
            }
            _ => return rejected,
        }
    }
}

/// FR-1042: certify a decided interval with its `lower` and `upper` documents
/// and settle by the claim's side. Each document is checked before it is
/// reported, and a rejection of either settles `inconclusive`.
fn settle_interval(
    subject_of: &IntervalSubject<'_>,
    interval: &Interval,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    subject: SubjectLimits<'_>,
) -> Result<EngineReport, EngineReport> {
    use fsl_replay::SubsystemBoundKind;
    let io = fsl_replay::ProbabilityDocumentIoLimits {
        bytes: limits.bytes,
        storage_bytes: limits.storage_bytes,
    };
    let mut sides = Vec::with_capacity(2);
    for (side, ends) in [
        (SubsystemBoundKind::Lower, &interval.lower),
        (SubsystemBoundKind::Upper, &interval.upper),
    ] {
        let values = ends
            .iter()
            .map(|end| ExtRational::Finite(end.rational()))
            .collect::<Vec<_>>();
        let checked = (|| {
            let mut certificate = bounded_certificate_of(
                subject_of.product,
                subject_of.reference,
                &values,
                subject_of.monitor,
                io,
                cancel,
                None,
            )?;
            if let Some(tamper) = tamper {
                tamper(&mut certificate);
            }
            let document = probability_proof_document(
                subject_of.system,
                subject_of.claims,
                subject_of.item,
                &certificate,
                limits,
                cancel,
            )?;
            certificate.values = document.document().values.clone();
            let bytes =
                fsl_replay::write_value_bounds_proof_document(&document, side, limits, cancel)?;
            let check = check_probability_certificate_document(
                subject_of.system,
                subject_of.claims,
                subject_of.item,
                &bytes,
                limits,
                cancel,
            )?;
            Ok::<_, ProbabilityDocumentError>((certificate, check))
        })();
        sides.push((side, checked));
    }
    let mut documents = sides.into_iter().map(|(_, checked)| checked);
    let (Some(lower), Some(upper)) = (documents.next(), documents.next()) else {
        return Ok(broken());
    };
    finish_interval(
        subject_of.bound,
        lower,
        upper,
        subject_of.binding,
        subject_of.extremum,
        interval.precision_bits,
        subject,
    )
}

/// An interval end's certificate and the checker's verdict on its document.
type IntervalDocument = Result<
    (
        ExactProbabilityCertificate,
        ExactProbabilityCertificateCheck,
    ),
    ProbabilityDocumentError,
>;

/// Settle a decided interval from its two checked documents: `proved` with
/// `ValueBounds` only when the claim side's document and its counterpart are
/// both accepted, and a rejection of either as that document's rejection.
fn finish_interval(
    bound: &Bound,
    lower: IntervalDocument,
    upper: IntervalDocument,
    binding: Option<&fsl_replay::ClaimBinding>,
    extremum: Option<Extremum>,
    precision_bits: u32,
    subject: SubjectLimits<'_>,
) -> Result<EngineReport, EngineReport> {
    let (claim, counterpart) = match bound {
        Bound::AtLeast(_) => (lower, upper),
        Bound::AtMost(_) => (upper, lower),
    };
    let (certificate, claim_check) = match claim {
        Ok(accepted) => accepted,
        Err(error) => return Ok(document_failure(error, subject)),
    };
    let (_, counterpart_check) = match counterpart {
        Ok(accepted) => accepted,
        Err(error) => return Ok(document_failure(error, subject)),
    };
    let accepted = |check: &ExactProbabilityCertificateCheck| {
        matches!(check, ExactProbabilityCertificateCheck::Accepted(_))
    };
    let rejected = if !accepted(&claim_check) {
        Some(claim_check.clone())
    } else if !accepted(&counterpart_check) {
        Some(counterpart_check.clone())
    } else {
        None
    };
    if let Some(check) = rejected {
        return Err(checked_report(
            Ok(check),
            certificate,
            binding.cloned(),
            extremum,
            subject,
        ));
    }
    let ExactProbabilityCertificateCheck::Accepted(accepted_claim) = &claim_check else {
        return Ok(broken());
    };
    let evidence = Box::new(ExactProbabilityEvidence {
        certificate,
        value: accepted_claim.value().clone(),
        side: accepted_claim.side(),
    });
    let binding = binding.map(|bound| ExactBinding {
        population: bound.instance.population.node.clone(),
        key: bound.instance.key.clone(),
    });
    Ok(EngineReport::Exact(Box::new(ExactReport::ProvedBounds {
        claim: claim_check,
        counterpart: counterpart_check,
        method: fsl_replay::BoundsMethod::BackwardInduction { precision_bits },
        extremum,
        binding,
        evidence,
    })))
}

fn bounded_certificate_of(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
    monitor: &fsl_eval::automaton::QualifiedBoundedMonitor,
    io: fsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
    mut budget: Option<&mut candidate::CandidateBudget<'_>>,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if values.len() != product.states.len() {
        return Err(ExactProbabilityGraphError::MissingSuccessor.into());
    }
    if let Some(budget) = budget.as_mut() {
        budget.reserve(
            product
                .states
                .len()
                .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
        )?;
    }
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        if state.accumulator.is_some() || state.pending.is_some() {
            return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
        }
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        let key = match budget.as_mut() {
            Some(budget) => {
                let key = bounded_monitor_key(state, monitor)?;
                let identity = budget.identity;
                let mut admission = budget.admission();
                ProbabilityProductKey::for_formula_state_with_budget(
                    binding,
                    &state.model,
                    &key,
                    identity,
                    &mut admission,
                    cancel,
                )
                .map_err(|error| admission.refusal(error))?
            }
            None => bounded_product_key(state, binding, monitor, io, cancel)?,
        };
        if let Some(budget) = budget.as_mut() {
            budget.reserve(candidate::rational_bytes(value))?;
        }
        entries.push(ExactProbabilityValue {
            state: key,
            value: value.clone(),
        });
    }
    if let Some(budget) = budget {
        candidate::sort(&mut entries, |row| &row.state, &mut budget.meter, cancel)?;
    } else {
        entries.sort_by(|left, right| left.state.cmp(&right.state));
    }
    if entries
        .windows(2)
        .any(|pair| pair[0].state == pair[1].state)
    {
        return Err(ExactProbabilityGraphError::ExpansionRefused.into());
    }
    Ok(ExactProbabilityCertificate { values: entries })
}

fn bounded_product_key(
    state: &crate::exact_probabilistic::ProbKey,
    binding: Option<&ObjectReference>,
    monitor: &fsl_eval::automaton::QualifiedBoundedMonitor,
    io: fsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let key = bounded_monitor_key(state, monitor)?;
    ProbabilityProductKey::for_formula_state(binding, &state.model, &key, io, cancel)
}

fn bounded_monitor_key(
    state: &crate::exact_probabilistic::ProbKey,
    monitor: &fsl_eval::automaton::QualifiedBoundedMonitor,
) -> Result<fsl_eval::automaton::AutomatonStateKey, ProbabilityDocumentError> {
    use crate::exact_probabilistic::{FormulaState, MonitorState, MonitorVerdict};
    let key = match state.monitor {
        Some(MonitorState::Formula(FormulaState::Running(state))) => monitor.key(state),
        Some(MonitorState::Formula(FormulaState::Decided(verdict))) => {
            monitor.decision_key(verdict == MonitorVerdict::Accept)
        }
        _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
    }
    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    Ok(key)
}

fn build_refusal(refusal: ProductRefusal, subject: SubjectLimits<'_>) -> EngineReport {
    match settle_product_refusal(refusal, subject) {
        Ok(record) => EngineReport::Exact(Box::new(ExactReport::Settled(record))),
        Err(ProductRefusal::Admission(failure)) => EngineReport::Refused(admission(&failure)),
        Err(ProductRefusal::NotSimulated(fsl_eval::simulation::NotSimulated::RequiresBound(_))) => {
            EngineReport::Refused(Refusal::RequiresBound)
        }
        Err(ProductRefusal::NotStochastic { .. }) => not_yet(),
        Err(ProductRefusal::WorkloadFreeDelays(refusal)) => {
            EngineReport::Refused(Refusal::Unsupported(refusal.code()))
        }
        Err(_) => broken(),
    }
}

/// FR-201's certificate for `product`: the solved value of every state, under
/// the key the checker gives the same model state.
fn certificate_of(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
    io: fsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if values.len() != product.states.len() {
        return Err(ExactProbabilityGraphError::MissingSuccessor.into());
    }
    let mut storage = 0_u64;
    let rows = u64::try_from(product.states.len()).unwrap_or(u64::MAX);
    reserve_certificate_storage(
        &mut storage,
        io.storage_bytes,
        rows.saturating_mul(std::mem::size_of::<ExactProbabilityValue>() as u64),
    )?;
    for value in values {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        reserve_certificate_storage(
            &mut storage,
            io.storage_bytes,
            value
                .numerator()
                .magnitude_bits()
                .div_ceil(8)
                .saturating_add(value.denominator().magnitude_bits().div_ceil(8)),
        )?;
    }
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        let key = match &state.certificate_state {
            Some(key) => key.clone(),
            None => ProbabilityProductKey::for_model_state_with_limits(
                binding,
                &state.model,
                fsl_replay::ProbabilityDocumentIoLimits {
                    storage_bytes: io.storage_bytes.saturating_sub(storage),
                    ..io
                },
                cancel,
            )?,
        };
        reserve_certificate_storage(&mut storage, io.storage_bytes, key.as_bytes().len() as u64)?;
        entries.push(ExactProbabilityValue {
            state: key,
            value: value.clone(),
        });
    }
    entries.sort_unstable_by(|a, b| a.state.cmp(&b.state));
    if entries
        .windows(2)
        .any(|pair| pair[0].state == pair[1].state)
    {
        return Err(ExactProbabilityGraphError::ExpansionRefused.into());
    }
    Ok(ExactProbabilityCertificate { values: entries })
}

fn reserve_certificate_storage(
    used: &mut u64,
    bound: u64,
    bytes: u64,
) -> Result<(), ProbabilityDocumentError> {
    let actual = used.saturating_add(bytes);
    if actual > bound {
        return Err(ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        });
    }
    *used = actual;
    Ok(())
}

fn witness_payload_bytes(
    work: crate::exact_probabilistic::WitnessWork<'_>,
    system: &ModelSystem<'_>,
    mut post_state_bytes: impl FnMut(
        &crate::exact_probabilistic::ProbKey,
    ) -> Result<usize, Box<ExactProbBuildIssue>>,
) -> Result<usize, Box<ExactProbBuildIssue>> {
    use crate::exact_probabilistic::{ProductAction, WitnessWork};
    let mut action_bytes = |action: &ProductAction| -> Result<usize, Box<ExactProbBuildIssue>> {
        match action {
            ProductAction::Identity(identity) => system
                .scheduled_identity_key(identity)
                .map(|bytes| bytes.len())
                .map_err(|stop| Box::new(ExactProbBuildIssue::Expansion(stop))),
            ProductAction::TerminalStutter => Ok(0),
            ProductAction::PostState(target) => post_state_bytes(target),
            _ => Err(Box::new(ExactProbBuildIssue::BrokenInvariant)),
        }
    };
    match work {
        WitnessWork::Visit | WitnessWork::MaterializedStep => Ok(0),
        WitnessWork::Rational(probability) => {
            Ok([probability.numerator(), probability.denominator()]
                .iter()
                .map(|integer| {
                    usize::try_from(integer.as_big().bits().div_ceil(8)).unwrap_or(usize::MAX)
                })
                .fold(0usize, usize::saturating_add))
        }
        WitnessWork::Slots(bytes) => Ok(bytes),
        WitnessWork::Action(action) => action_bytes(action),
        WitnessWork::Step {
            action,
            draw,
            probability,
            rewards,
        } => {
            let mut bytes = action_bytes(action)?;
            let mut value_bytes =
                |value: &filament_exact::Value| -> Result<(), Box<ExactProbBuildIssue>> {
                    let key = system
                        .value_key(value)
                        .map_err(|_| Box::new(ExactProbBuildIssue::BrokenInvariant))?;
                    bytes = bytes.saturating_add(key.len());
                    Ok(())
                };
            for value in draw.into_iter().flatten() {
                value_bytes(value)?;
            }
            for (_, value) in rewards {
                value_bytes(value)?;
            }
            bytes = bytes.saturating_add(draw.map_or(0, |draw| {
                draw.len()
                    .saturating_mul(std::mem::size_of::<filament_exact::Value>())
            }));
            for (name, _) in rewards {
                bytes =
                    bytes
                        .saturating_add(name.as_str().len())
                        .saturating_add(std::mem::size_of::<(
                            filament_exact::Identifier,
                            filament_exact::Value,
                        )>());
            }
            for integer in [probability.numerator(), probability.denominator()] {
                bytes = bytes.saturating_add(
                    usize::try_from(integer.as_big().bits().div_ceil(8)).unwrap_or(usize::MAX),
                );
            }
            Ok(bytes)
        }
    }
}

fn witness_charge(
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    bytes: usize,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
) -> Result<(), Box<ExactProbBuildIssue>> {
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return Err(Box::new(ExactProbBuildIssue::Cancelled(cause)));
        }
    }
    *storage = storage.saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    if *storage > limits.document.storage_bytes {
        return Err(Box::new(ExactProbBuildIssue::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: limits.document.storage_bytes,
        }));
    }
    meter
        .charge(filament_exact::Charge::new(
            filament_exact::ChargePoint::CollectionVisit,
        ))
        .map_err(|incomplete| {
            if let Some(cause) = cancel.tripped() {
                return Box::new(ExactProbBuildIssue::Cancelled(cause));
            }
            Box::new(ExactProbBuildIssue::Predicate(
                fsl_eval::simulation::model::ClauseOutcome::Incomplete(incomplete),
            ))
        })
}

#[allow(clippy::too_many_arguments)]
fn replay_refutation(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    target: &[bool],
    binding: Option<&ObjectReference>,
    monitor: Option<&fsl_eval::automaton::QualifiedBoundedMonitor>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{
        produce_witness, FormulaState, MonitorState, MonitorVerdict, ProductAction, WitnessScope,
    };
    let mut producer_meter =
        filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut producer_storage = 0u64;
    // Classify direct state boundaries in one pass over the source-labelled
    // product. The replay core independently derives the same outcomes.
    let state_outcomes = match &claim.form {
        ProbForm::Reach {
            target: target @ (ReachTarget::Until(..) | ReachTarget::Always(_)),
            ..
        } => {
            if let Err(issue) = witness_charge(
                &mut producer_meter,
                &mut producer_storage,
                product.states.len().saturating_mul(
                    std::mem::size_of::<bool>() + std::mem::size_of::<Option<bool>>(),
                ),
                limits,
                cancel,
            ) {
                return settled(&issue, subject);
            }
            let mut terminal = Vec::new();
            let mut outcomes = Vec::new();
            if terminal.try_reserve(product.states.len()).is_err()
                || outcomes.try_reserve(product.states.len()).is_err()
            {
                return settled(
                    &ExactProbBuildIssue::Limit {
                        limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                        value: limits.document.storage_bytes,
                    },
                    subject,
                );
            }
            terminal.resize(product.states.len(), true);
            for edge in &product.edges {
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    0,
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                if !matches!(
                    edge.choice,
                    ProductAction::Identity(_) | ProductAction::TerminalStutter
                ) {
                    return not_yet();
                }
                if !matches!(edge.choice, ProductAction::TerminalStutter) {
                    let Some(terminal) = terminal.get_mut(edge.from.index()) else {
                        return broken();
                    };
                    *terminal = false;
                }
            }
            for (index, labels) in product.labels.iter().enumerate() {
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    0,
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                let Some(terminal) = terminal.get(index).copied() else {
                    return broken();
                };
                let first = labels.first().copied().unwrap_or(false);
                let verdict = match target {
                    ReachTarget::Until(..) => {
                        if labels.get(1).copied().unwrap_or(false) {
                            Some(true)
                        } else if !first || terminal {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    ReachTarget::Always(_) => {
                        if !first {
                            Some(false)
                        } else if terminal {
                            Some(true)
                        } else {
                            None
                        }
                    }
                    ReachTarget::Eventually(_) => return broken(),
                };
                outcomes.push(verdict);
            }
            Some(outcomes)
        }
        _ => None,
    };
    let (accepting, past) = match (&claim.form, state_outcomes.as_deref()) {
        (ProbForm::Reach { bound, .. }, Some(outcomes)) => (
            outcomes
                .iter()
                .map(|outcome| *outcome == Some(matches!(bound, Bound::AtMost(_))))
                .collect(),
            match bound {
                Bound::AtMost(threshold) => threshold.clone(),
                Bound::AtLeast(threshold) => Rational::from_integer(1_i64.into()).sub(threshold),
            },
        ),
        (ProbForm::Probability { bound, .. }, _) if monitor.is_some() => (
            product
                .states
                .iter()
                .map(|state| {
                    matches!(
                        (bound, state.monitor),
                        (
                            Bound::AtMost(_),
                            Some(MonitorState::Formula(FormulaState::Decided(
                                MonitorVerdict::Accept
                            )))
                        ) | (
                            Bound::AtLeast(_),
                            Some(MonitorState::Formula(FormulaState::Decided(
                                MonitorVerdict::Reject
                            )))
                        )
                    )
                })
                .collect(),
            match bound {
                Bound::AtMost(threshold) => threshold.clone(),
                Bound::AtLeast(threshold) => Rational::from_integer(1_i64.into()).sub(threshold),
            },
        ),
        (
            ProbForm::Reach {
                bound: Bound::AtMost(threshold),
                ..
            },
            _,
        ) => (target.to_vec(), threshold.clone()),
        (
            ProbForm::Reach {
                bound: Bound::AtLeast(threshold),
                ..
            },
            _,
        ) => (
            product
                .states
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    !target.get(index).copied().unwrap_or(false)
                        && product
                            .edges
                            .iter()
                            .filter(|edge| edge.from.index() == index)
                            .all(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
                })
                .collect(),
            Rational::from_integer(1_i64.into()).sub(threshold),
        ),
        _ => return not_yet(),
    };
    let scope = match claim.scheduler {
        ProbScheduler::Every => WitnessScope::EveryScheduler,
        ProbScheduler::Workload(_) => WitnessScope::Workload,
    };
    let stopped = state_outcomes
        .as_ref()
        .map(|outcomes| outcomes.iter().map(Option::is_some).collect::<Vec<_>>());
    let mut producer_issue = None;
    let mut control = |work: crate::exact_probabilistic::WitnessWork<'_>| {
        let result = witness_payload_bytes(work, system, |_| {
            Err(Box::new(ExactProbBuildIssue::BrokenInvariant))
        })
        .and_then(|bytes| {
            witness_charge(
                &mut producer_meter,
                &mut producer_storage,
                bytes,
                limits,
                cancel,
            )
        });
        result.map_err(|issue| {
            producer_issue = Some(issue);
            crate::exact_probabilistic::WitnessError::InvalidState
        })
    };
    let mut produce = |policy: &[Option<usize>]| {
        if let Some(stopped) = stopped.as_deref() {
            crate::exact_probabilistic::produce_witness_stopping(
                product,
                policy,
                scope,
                initial,
                &accepting,
                Some(stopped),
                &past,
                limits,
                &mut control,
            )
        } else {
            produce_witness(product, policy, scope, initial, &accepting, &past, limits)
        }
    };
    let mut produced = produce(policy);
    if matches!(produced, Ok(None)) {
        // Actions tied at the optimal value need not reach a failure: a
        // minimizing scheduler may loop where it is free to lose. Whatever
        // scheduler's failure paths outweigh `past` refutes the bound, so
        // retry with the one that reaches a failure most, never continuing
        // past a state a direct event has already decided.
        let retried = match stopped.as_deref() {
            None => decide_reachability(product, &accepting, UnboundedObjective::Max, limits),
            Some(stopped) => {
                let open = stopped.iter().map(|stopped| !stopped).collect::<Vec<_>>();
                decide_until(product, &open, &accepting, UnboundedObjective::Max, limits)
            }
        };
        match retried {
            Ok(result) => produced = produce(&result.policy),
            Err(error) => return unbounded_failure(error, limits, subject),
        }
    }
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
        }
    }
    if let Some(issue) = producer_issue {
        return settled(&issue, subject);
    }
    let produced = match produced {
        Ok(Some(witness)) => witness,
        Ok(None) => return not_yet(),
        Err(crate::exact_probabilistic::WitnessError::Limit { limit, value }) => {
            return settled(&ExactProbBuildIssue::Limit { limit, value }, subject);
        }
        Err(_) => return broken(),
    };
    replay_produced_witness(
        system,
        claim,
        product,
        produced,
        initial,
        binding,
        monitor,
        limits,
        cancel,
        subject,
        tamper,
        producer_meter,
        producer_storage,
    )
}

#[allow(clippy::too_many_arguments)]
fn replay_produced_witness(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    produced: crate::exact_probabilistic::ProducedWitness,
    initial: crate::exact_probabilistic::ProbStateId,
    binding: Option<&ObjectReference>,
    monitor: Option<&fsl_eval::automaton::QualifiedBoundedMonitor>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
    mut producer_meter: filament_exact::Meter,
    mut producer_storage: u64,
) -> EngineReport {
    use crate::exact_probabilistic::ProductAction;
    let measure = claim.bounded_measure();
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let key_of = |state: &crate::exact_probabilistic::ProbKey| {
        if let Some(key) = &state.certificate_state {
            return Ok(key.clone());
        }
        if let Some(monitor) = monitor {
            bounded_product_key(state, binding, monitor, limits.document, cancel)
        } else {
            ProbabilityProductKey::for_model_state_with_limits(
                binding,
                &state.model,
                limits.document,
                cancel,
            )
        }
    };
    let replay_choice = |choice: &ProductAction| canonical_action(system, product, choice, &key_of);
    let Some(initial_state) = product.states.get(initial.index()) else {
        return broken();
    };
    let initial_key = match key_of(initial_state) {
        Ok(key) => key,
        Err(error) => return document_failure(error, subject),
    };
    let mut scheduler = Vec::new();
    let mut spelled_scheduler = Vec::new();
    for (state, action) in produced.entries {
        // Terminal stutter carries no scheduler decision. Evidence paths
        // stop at first hits or independently checked nonreach leaves.
        if matches!(&action, ProductAction::TerminalStutter) {
            continue;
        }
        let Some(state) = product.states.get(state.index()) else {
            return broken();
        };
        let key = match key_of(state) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        if let ProductAction::Identity(identity) = &action {
            spelled_scheduler.push((
                fsl_eval::simulation::StateKey::from_canonical_bytes(key.as_bytes().to_vec()),
                identity.clone(),
            ));
        }
        let action = match replay_choice(&action) {
            Ok(action) => action,
            Err(error) => return document_failure(error, subject),
        };
        scheduler.push((key, action));
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let mut paths = Vec::new();
    let mut conversion_work = 0usize;
    let mut residual_used = std::collections::BTreeSet::new();
    for path in produced.paths {
        let mut partials = vec![(first.clone(), Vec::new())];
        let mut produced_steps = path.steps.into_iter();
        while let Some(mut step) = produced_steps.next() {
            if measure
                && product
                    .states
                    .get(step.to.index())
                    .is_some_and(|state| state.pending.is_some())
            {
                let Some(resolved) = produced_steps.next() else {
                    return broken();
                };
                if !matches!(step.action, ProductAction::Identity(_))
                    || !matches!(resolved.action, ProductAction::PostState(_))
                    || resolved.probability != Rational::from_integer(1_i64.into())
                {
                    return broken();
                }
                let Some(pending) = product.states.get(step.to.index()) else {
                    return broken();
                };
                let pending = match key_of(pending) {
                    Ok(key) => key,
                    Err(error) => return document_failure(error, subject),
                };
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    pending.as_bytes().len(),
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                residual_used.insert(pending);
                step.to = resolved.to;
            }
            if cancel.poll() {
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
            }
            if !measure && matches!(&step.action, ProductAction::TerminalStutter) {
                // Qualified replay derives false extension at an actual leaf;
                // closure consumes no model step or draw.
                continue;
            }
            let choice = match replay_choice(&step.action) {
                Ok(choice) => choice,
                Err(error) => return document_failure(error, subject),
            };
            let Some(to) = product.states.get(step.to.index()) else {
                return broken();
            };
            let post_key = match key_of(to) {
                Ok(key) => key,
                Err(error) => return document_failure(error, subject),
            };
            let mut next = Vec::new();
            for (state, steps) in partials {
                conversion_work = conversion_work.saturating_add(1);
                if conversion_work > limits.model.limits.max_transitions {
                    return settled(
                        &ExactProbBuildIssue::Limit {
                            limit: ModelCheckLimit::ExactProbMaxTransitions,
                            value: u64::try_from(limits.model.limits.max_transitions)
                                .unwrap_or(u64::MAX),
                        },
                        subject,
                    );
                }
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    std::mem::size_of::<ProbabilityWitnessStep>(),
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                let ProductAction::Identity(identity) = &step.action else {
                    if !matches!(
                        &choice,
                        fsl_replay::ProductAction::PostState(_)
                            | fsl_replay::ProductAction::TerminalStutter
                            | fsl_replay::ProductAction::UnitDelay
                            | fsl_replay::ProductAction::EnterIdleDelay
                            | fsl_replay::ProductAction::IdleDelay
                            | fsl_replay::ProductAction::IdleObserve
                    ) {
                        return not_yet();
                    }
                    let mut extended = steps.clone();
                    extended.push(ProbabilityWitnessStep {
                        choice: choice.clone(),
                        action: None,
                        transition: None,
                        draw: None,
                        post: state.clone(),
                        post_key: post_key.clone(),
                        elapsed: step.elapsed.clone(),
                    });
                    next.push((state, extended));
                    continue;
                };
                let result = system.actions_with_cancel(&state, cancel);
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
                let actions = match result {
                    Ok(Ok(actions)) => actions,
                    Err(stop) => return settled(&ExactProbBuildIssue::Expansion(stop), subject),
                    Ok(Err(_)) => return broken(),
                };
                let mut candidates = Vec::new();
                for action in actions {
                    if &action.identity != identity {
                        continue;
                    }
                    for draw in action.draws {
                        if step.draw.as_ref().is_some_and(|expected| {
                            !fsl_eval::simulation::model::same_values(expected, &draw.random)
                        }) {
                            continue;
                        }
                        for (transition, post) in draw.successors {
                            if system.state_key(&post).ok().map(|key| key.0).as_ref()
                                == Some(&to.model)
                            {
                                if let Err(issue) = witness_charge(
                                    &mut producer_meter,
                                    &mut producer_storage,
                                    post_key.as_bytes().len().saturating_mul(2),
                                    limits,
                                    cancel,
                                ) {
                                    return settled(&issue, subject);
                                }
                                if candidates.try_reserve(1).is_err() {
                                    return settled(
                                        &ExactProbBuildIssue::Limit {
                                            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                            value: limits.document.storage_bytes,
                                        },
                                        subject,
                                    );
                                }
                                candidates.push(ProbabilityWitnessStep {
                                    choice: choice.clone(),
                                    action: Some(identity.clone()),
                                    transition: Some(transition),
                                    draw: Some(draw.random.clone()),
                                    post,
                                    post_key: post_key.clone(),
                                    elapsed: step.elapsed.clone(),
                                });
                            }
                        }
                    }
                }
                if candidates.is_empty() {
                    return broken();
                }
                for candidate in candidates {
                    if u64::try_from(next.len().saturating_add(paths.len())).unwrap_or(u64::MAX)
                        >= limits.max_witness_paths
                    {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
                                value: limits.max_witness_paths,
                            },
                            subject,
                        );
                    }
                    if let Err(issue) = witness_charge(
                        &mut producer_meter,
                        &mut producer_storage,
                        steps
                            .len()
                            .saturating_add(1)
                            .saturating_mul(std::mem::size_of::<ProbabilityWitnessStep>())
                            .saturating_add(post_key.as_bytes().len()),
                        limits,
                        cancel,
                    ) {
                        return settled(&issue, subject);
                    }
                    if next.try_reserve(1).is_err() {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                value: limits.document.storage_bytes,
                            },
                            subject,
                        );
                    }
                    let mut extended = Vec::new();
                    if extended.try_reserve(steps.len().saturating_add(1)).is_err() {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                value: limits.document.storage_bytes,
                            },
                            subject,
                        );
                    }
                    extended.extend(steps.iter().cloned());
                    let post = candidate.post.clone();
                    extended.push(candidate);
                    next.push((post, extended));
                }
            }
            partials = next;
        }
        paths.extend(
            partials
                .into_iter()
                .map(|(_, steps)| ProbabilityWitnessPath { steps }),
        );
    }
    if monitor.is_some() || measure {
        // A finite-horizon policy only needs entries for represented choices.
        // The unbounded producer retains its continuation policy contract.
        let mut used = std::collections::BTreeSet::new();
        for path in &paths {
            let mut key = &initial_key;
            for step in &path.steps {
                used.insert(key.clone());
                key = &step.post_key;
            }
        }
        scheduler.retain(|(key, _)| used.contains(key) || residual_used.contains(key));
    }
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        initial: first.clone(),
        initial_key,
        scheduler: scheduler
            .into_iter()
            .map(|(key, action)| {
                (
                    key,
                    fsl_replay::ProbabilisticSchedulerChoice::Deterministic(action),
                )
            })
            .collect(),
        paths,
        subsystem: None,
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    finish_replay(
        replay_witness(system, &item, &witness, limits, cancel),
        claim,
        system,
        &witness,
        &spelled_scheduler,
        limits,
        subject,
    )
}

fn replay_witness(
    system: &ModelSystem<'_>,
    item: &fsl_package::ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    if let Some(package) = system.source_package() {
        return replay_probabilistic_witness(package, system, item, witness, limits, cancel);
    }
    let owner = system
        .input()
        .package_owner()
        .ok_or(ProbabilityWitnessError::Refused)?;
    replay_probabilistic_witness_from_package(owner, system, item, witness, limits, cancel)
}

/// The report a witness replay settles: refuted on a reproduction, otherwise
/// the typed reason it did not.
fn finish_replay(
    replayed: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    system: &ModelSystem<'_>,
    witness: &ProbabilisticCounterexample,
    spelled_scheduler: &[(
        fsl_eval::simulation::StateKey,
        fsl_eval::simulation::model::ScheduledIdentity,
    )],
    limits: &crate::exact_probabilistic::ExactProbLimits,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    match replayed {
        Ok(ProbabilityWitnessReplay::Reproduced(token)) => {
            let input = system.input();
            let quantile_bound;
            let bound = match &claim.form {
                ProbForm::Reach { bound, .. }
                | ProbForm::Probability { bound, .. }
                | ProbForm::MeanOfFraction { bound, .. }
                | ProbForm::LongRunFraction { bound, .. }
                | ProbForm::ExpectedReward { bound, .. } => bound,
                ProbForm::Quantile { q, threshold, .. } => {
                    quantile_bound = match threshold {
                        Bound::AtMost(_) => Bound::AtLeast(q.clone()),
                        Bound::AtLeast(_) => Bound::AtMost(q.clone()),
                    };
                    &quantile_bound
                }
                _ => return not_yet(),
            };
            let state_key = |state: &fsl_eval::simulation::model::ModelSimState| {
                system.state_key(state).ok().map(|(_, key)| key)
            };
            let wire = match input.enum_members {
                Some(enum_members) => fsl_replay::probabilistic_wire_with_enum_members(
                    witness,
                    bound,
                    input.types,
                    input.variant_types,
                    enum_members,
                    &state_key,
                    spelled_scheduler,
                ),
                None => fsl_replay::probabilistic_wire(
                    witness,
                    bound,
                    input.types,
                    input.variant_types,
                    &state_key,
                    spelled_scheduler,
                ),
            };
            EngineReport::Exact(Box::new(ExactReport::Refuted(token, Box::new(wire))))
        }
        Ok(ProbabilityWitnessReplay::Parity) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayParity))
        }
        Err(ProbabilityWitnessError::Build(issue)) => settled(&issue, subject),
        Err(ProbabilityWitnessError::CertificateRejected(rejection)) => checked_report(
            Ok(ExactProbabilityCertificateCheck::Rejected(rejection)),
            ExactProbabilityCertificate { values: Vec::new() },
            None,
            None,
            subject,
        ),
        Err(ProbabilityWitnessError::Refused) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayRefused))
        }
        Err(ProbabilityWitnessError::Limit { limit, value }) => {
            settled(&ExactProbBuildIssue::Limit { limit, value }, subject)
        }
        Err(ProbabilityWitnessError::Graph(error)) => graph_failure(error, limits, subject),
    }
}

fn canonical_action(
    system: &ModelSystem<'_>,
    product: &ProbProduct,
    choice: &crate::exact_probabilistic::ProductAction,
    key_of: &impl Fn(
        &crate::exact_probabilistic::ProbKey,
    ) -> Result<ProbabilityProductKey, ProbabilityDocumentError>,
) -> Result<fsl_replay::ProductAction, ProbabilityDocumentError> {
    // A stopped expansion keeps its budget; any other schedule failure is a
    // product the engine built that the system cannot write.
    let unscheduled = |error| match error {
        fsl_replay::ProductActionError::Stopped(stop) => {
            ProbabilityDocumentError::Build(Box::new(ExactProbBuildIssue::Expansion(stop)))
        }
        _ => ProbabilityDocumentError::Malformed,
    };
    use crate::exact_probabilistic::ProductAction as AnalyzerAction;
    Ok(match choice {
        AnalyzerAction::Identity(identity) => fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(system, identity).map_err(unscheduled)?,
        ),
        AnalyzerAction::PostState(target) => {
            let target = product
                .states
                .iter()
                .find(|state| *state == target.as_ref())
                .ok_or(ProbabilityDocumentError::Malformed)?;
            fsl_replay::ProductAction::PostState(key_of(target)?)
        }
        AnalyzerAction::TerminalStutter => fsl_replay::ProductAction::TerminalStutter,
        AnalyzerAction::UnitDelay => fsl_replay::ProductAction::UnitDelay,
        AnalyzerAction::EnterIdleDelay => fsl_replay::ProductAction::EnterIdleDelay,
        AnalyzerAction::IdleDelay => fsl_replay::ProductAction::IdleDelay,
        AnalyzerAction::IdleObserve => fsl_replay::ProductAction::IdleObserve,
        AnalyzerAction::WorkloadResolve { identity, delay } => {
            fsl_replay::ProductAction::WorkloadResolve {
                schedule: fsl_replay::Schedule::of(system, identity).map_err(unscheduled)?,
                delay: *delay,
            }
        }
        AnalyzerAction::FreeDelays(rows) => {
            fsl_replay::ProductAction::free_delays(system, rows).map_err(unscheduled)?
        }
    })
}

/// Test seam altering produced typed evidence before independent replay.
#[cfg(feature = "test-support")]
pub fn exact_probabilistic_item_witness_tampered<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
    tamper: impl Fn(&mut ProbabilisticCounterexample) + 'a,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, None, Some(&tamper)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsl_replay::{ProbabilityCertificateRejection, TerminalValue};
    use ix_trace_rs::trace;

    #[trace("TC-1197", "FR-201-AC-6", "FR-203-AC-3")]
    #[test]
    fn threshold_rejection_retains_its_rule_and_complete_product_key() {
        let key = ProbabilityProductKey::for_model_state(
            None,
            include_bytes!(
                "../../../tools/fsl-probability-check/tests/fixtures/link.model-key-0.json"
            ),
        )
        .expect("the key encodes");
        let (terminal, evidence, counterexample) = ExactReport::Rejected {
            rule: CertificateRule::ThresholdMissed,
            at: CertificateLocus::ProbabilityProductState(key.clone()),
        }
        .settle(RequestIndex::new(0), BackendId::new("en-5"));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::ThresholdMissed,
                at: CertificateLocus::ProbabilityProductState(key),
            })
        );
        assert!(evidence.is_none());
        assert!(counterexample.is_none());
    }
    #[trace("FR-203-AC-11")]
    #[test]
    fn an_engine_proved_assertion_without_checker_acceptance_fails_invariant() {
        let check = ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::InequalityFails,
            at: ProbabilityCertificateLocus::Member("values"),
        });
        let (terminal, evidence, counterexample) = ExactReport::Proved {
            check,
            extremum: None,
            binding: None,
            evidence: None,
        }
        .settle(RequestIndex::new(0), BackendId::new("en-5"));
        assert_eq!(terminal.value(), &TerminalValue::Failed);
        assert_eq!(terminal.diagnostic(), Some(BROKEN));
        assert!(terminal.exact_certificate_document().is_none());
        assert!(evidence.is_none() && counterexample.is_none());
    }

    #[trace("FR-203-AC-11")]
    #[test]
    fn an_inequality_rejection_settles_inconclusive_at_the_same_complete_key() {
        let key = ProbabilityProductKey::for_model_state(
            None,
            include_bytes!(
                "../../../tools/fsl-probability-check/tests/fixtures/link.model-key-0.json"
            ),
        )
        .unwrap();
        let (terminal, evidence, counterexample) = ExactReport::Rejected {
            rule: CertificateRule::InequalityFails,
            at: CertificateLocus::ProbabilityProductState(key.clone()),
        }
        .settle(RequestIndex::new(0), BackendId::new("en-5"));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::InequalityFails,
                at: CertificateLocus::ProbabilityProductState(key),
            })
        );
        assert!(terminal.exact_certificate_document().is_none());
        assert!(evidence.is_none() && counterexample.is_none());
    }
}

/// FR-923: refute a timed deadline claim with a path set over the timed
/// product, which the checker's own replay of the same product must reproduce.
///
/// The product's policy picks, per state, the choice that reaches the decided
/// states the bound forbids; the paths are read from it. Each path step
/// carries the product-state key it reaches (the key is what the checker
/// replays) and, where the step takes an operation, the operation, draw and
/// post-state it resolves to in the model.
#[allow(clippy::too_many_arguments)]
fn replay_timed_refutation(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    bound: &Bound,
    binding: Option<&ObjectReference>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{produce_witness, ProductAction, WitnessScope};
    use fsl_eval::probabilistic::{DeadlinePhase, MonitorState};
    let forbidden = match bound {
        Bound::AtMost(_) => DeadlinePhase::Accept,
        Bound::AtLeast(_) => DeadlinePhase::Reject,
    };
    let accepting = product
        .states
        .iter()
        .map(|state| {
            matches!(
                state.monitor,
                Some(MonitorState::Deadline { phase, .. }) if phase == forbidden
            )
        })
        .collect::<Vec<_>>();
    let past = match bound {
        Bound::AtMost(threshold) => threshold.clone(),
        Bound::AtLeast(threshold) => Rational::from_integer(1_i64.into()).sub(threshold),
    };
    let scope = match claim.scheduler {
        ProbScheduler::Every => WitnessScope::EveryScheduler,
        ProbScheduler::Workload(_) => WitnessScope::Workload,
    };
    let produced = match produce_witness(product, policy, scope, initial, &accepting, &past, limits)
    {
        Ok(Some(produced)) => produced,
        Ok(None) => return not_yet(),
        Err(crate::exact_probabilistic::WitnessError::Limit { limit, value }) => {
            return settled(&ExactProbBuildIssue::Limit { limit, value }, subject);
        }
        Err(_) => return broken(),
    };
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let key_of = |state: &crate::exact_probabilistic::ProbKey| {
        state
            .certificate_state
            .clone()
            .ok_or(ProbabilityDocumentError::Malformed)
    };
    let Some(initial_state) = product.states.get(initial.index()) else {
        return broken();
    };
    let initial_key = match key_of(initial_state) {
        Ok(key) => key,
        Err(error) => return document_failure(error, subject),
    };
    let mut paths = Vec::with_capacity(produced.paths.len());
    for path in produced.paths {
        let mut previous_key = initial_key.clone();
        let mut state = first.clone();
        let mut steps = Vec::with_capacity(path.steps.len());
        for step in path.steps {
            if cancel.poll() {
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
            }
            let Some(to) = product.states.get(step.to.index()) else {
                return broken();
            };
            let post_key = match key_of(to) {
                Ok(key) => key,
                Err(error) => return document_failure(error, subject),
            };
            let choice = match canonical_action(system, product, &step.action, &key_of) {
                Ok(choice) => choice,
                Err(error) => return document_failure(error, subject),
            };
            let (action, transition, draw, post, resume) = match &step.action {
                ProductAction::Identity(identity)
                | ProductAction::WorkloadResolve { identity, .. } => {
                    let wanted = timed_key_discrete(&post_key);
                    let result = system.actions_with_cancel(&state, cancel);
                    if let Some(cause) = cancel.tripped() {
                        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                    }
                    let actions = match result {
                        Ok(Ok(actions)) => actions,
                        Err(stop) => {
                            return settled(&ExactProbBuildIssue::Expansion(stop), subject);
                        }
                        Ok(Err(_)) => return broken(),
                    };
                    let found = actions
                        .into_iter()
                        .filter(|action| &action.identity == identity)
                        .flat_map(|action| action.draws)
                        .filter(|draw| {
                            step.draw.as_ref().is_none_or(|expected| {
                                fsl_eval::simulation::model::same_values(expected, &draw.random)
                            })
                        })
                        .find_map(|draw| {
                            draw.successors.into_iter().find_map(|(transition, post)| {
                                let key = system.state_key(&post).ok()?.0;
                                let discrete: serde_json::Value =
                                    serde_json::from_slice(&key).ok()?;
                                (Some(&discrete) == wanted.as_ref())
                                    .then(|| (transition, draw.random.clone(), post))
                            })
                        });
                    match found {
                        Some((transition, random, post)) => {
                            let resume = post.clone();
                            (
                                Some(identity.clone()),
                                Some(transition),
                                Some(random),
                                post,
                                resume,
                            )
                        }
                        None => {
                            // A clock-guarded operation is not enabled at the bare
                            // model state; take its step from the timed state the
                            // previous key names.
                            let (transition, post, resume) = match timed_step_for(
                                system,
                                claim,
                                &state,
                                &previous_key,
                                identity,
                                &step.elapsed,
                                wanted.as_ref(),
                            ) {
                                Ok(found) => found,
                                Err(failure) => {
                                    return timed_step_failure(failure, cancel, subject);
                                }
                            };
                            (
                                Some(identity.clone()),
                                Some(transition),
                                step.draw.clone(),
                                post,
                                resume,
                            )
                        }
                    }
                }
                _ => (None, None, None, state.clone(), state.clone()),
            };
            let next_key = post_key.clone();
            steps.push(ProbabilityWitnessStep {
                choice,
                action,
                transition,
                draw,
                post: post.clone(),
                post_key,
                elapsed: step.elapsed,
            });
            state = resume;
            previous_key = next_key;
        }
        paths.push(ProbabilityWitnessPath { steps });
    }
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        initial: first.clone(),
        initial_key,
        scheduler: Vec::new(),
        paths,
        subsystem: None,
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    finish_replay(
        replay_witness(system, &item, &witness, limits, cancel),
        claim,
        system,
        &witness,
        &[],
        limits,
        subject,
    )
}

/// The discrete model-state component of a timed product-state key.
fn timed_key_discrete(key: &ProbabilityProductKey) -> Option<serde_json::Value> {
    let root: serde_json::Value = serde_json::from_slice(key.as_bytes()).ok()?;
    root.get(4)?.get(1)?.get(1).cloned()
}

/// The model step of the operation a timed digital witness step took, found
/// from the timed state its previous key names: the clocks as keyed (placed
/// by clock identity), or those delayed by the step's elapsed time first.
///
/// The product resolves every tied zero-time operation of an instant in one
/// step, so the step's model state can lie several operations after the first.
/// The search follows a chain whose FIRST operation is `identity`'s and
/// returns that operation's transition, the model state right after it (what a
/// single recorded transition produces) and the model state at the end of the
/// chain (what the product key names, and where the next step starts).
/// A previous key that is a region key, or a chain still open after `CHAIN`
/// operations, is [`TimedStepFailure::Unbuilt`] (a region refutation through a
/// clock-guarded step therefore settles not-yet); a step expansion or key
/// encoding that stops names its stop; only a search that exhausted every
/// chain without reaching the wanted state is a fault.
fn timed_step_for(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    state: &fsl_eval::simulation::model::ModelSimState,
    previous: &ProbabilityProductKey,
    identity: &fsl_eval::simulation::model::ScheduledIdentity,
    elapsed: &Rational,
    wanted: Option<&serde_json::Value>,
) -> Result<
    (
        filament_semantic_value::model_transition::ModelTransition,
        fsl_eval::simulation::model::ModelSimState,
        fsl_eval::simulation::model::ModelSimState,
    ),
    TimedStepFailure,
> {
    use fsl_eval::simulation::model::{TimedState, TimedSystem};
    let valuation = fsl_replay::digital_key_clocks(system, claim, previous)
        .ok_or(TimedStepFailure::Unbuilt)?;
    let timed = TimedSystem::new(system, claim.subject.alias.as_str())
        .map_err(TimedStepFailure::Subject)?;
    // A tie among the zero-time operations of one instant: the product never
    // resolves more of them in one step than a small model has operations, and
    // each level multiplies the frontier, so the search stays short.
    const CHAIN: usize = 4;
    type First = Option<(
        filament_semantic_value::model_transition::ModelTransition,
        fsl_eval::simulation::model::ModelSimState,
    )>;
    type Found = (
        filament_semantic_value::model_transition::ModelTransition,
        fsl_eval::simulation::model::ModelSimState,
        fsl_eval::simulation::model::ModelSimState,
    );
    // The chain found, or whether the search stopped with chains still open.
    let reaches = |from: TimedState| -> Result<Result<Found, bool>, TimedStepFailure> {
        let mut frontier: Vec<(TimedState, First)> = vec![(from, None)];
        for level in 0..CHAIN {
            let mut next = Vec::new();
            for (at, first) in frontier {
                let steps = timed.steps(&at).map_err(TimedStepFailure::Stopped)?;
                for (step, post) in steps {
                    let first = if level == 0 {
                        if step.operation() != identity.operation
                            || step.receiver() != &identity.receiver
                        {
                            continue;
                        }
                        Some((step.transition().clone(), post.discrete().clone()))
                    } else {
                        first.clone()
                    };
                    let key = system
                        .state_key(post.discrete())
                        .map_err(|error| {
                            TimedStepFailure::Key(ProbabilityDocumentError::KeyEncoding(error))
                        })?
                        .0;
                    let discrete: serde_json::Value =
                        serde_json::from_slice(&key).map_err(|_| TimedStepFailure::Fault)?;
                    if Some(&discrete) == wanted {
                        let (transition, after_first) = first.ok_or(TimedStepFailure::Fault)?;
                        return Ok(Ok((transition, after_first, post.discrete().clone())));
                    }
                    next.push((post, first));
                }
            }
            frontier = next;
        }
        Ok(Err(!frontier.is_empty()))
    };
    let mut open = false;
    for clocks in [valuation.delayed(elapsed), valuation] {
        match reaches(TimedState::new(state.clone(), clocks))? {
            Ok(found) => return Ok(found),
            Err(still_open) => open |= still_open,
        }
    }
    Err(if open {
        TimedStepFailure::Unbuilt
    } else {
        TimedStepFailure::Fault
    })
}

/// Why a timed witness step could not be realized in the model.
enum TimedStepFailure {
    /// A region key, or a zero-time chain longer than the search follows: a
    /// generality of the replay that is not built.
    Unbuilt,
    /// The timed subject could not be built again.
    Subject(fsl_eval::simulation::model::TimedSubjectRefusal),
    /// A step expansion stopped.
    Stopped(fsl_eval::simulation::model::TimedStop),
    /// A state key did not encode.
    Key(ProbabilityDocumentError),
    /// Every chain was searched and none reaches the model state the engine
    /// produced.
    Fault,
}

/// How a timed step the replay could not realize settles.
fn timed_step_failure(
    failure: TimedStepFailure,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    use fsl_eval::simulation::model::TimedStop;
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    match failure {
        TimedStepFailure::Unbuilt => not_yet(),
        TimedStepFailure::Subject(refusal) => EngineReport::Refused(subject_refusal(&refusal)),
        TimedStepFailure::Stopped(TimedStop::MaxClocks { max_clocks }) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::MaxClocks,
                value: max_clocks,
            },
            subject,
        ),
        TimedStepFailure::Stopped(TimedStop::Expansion(stop)) => {
            settled(&ExactProbBuildIssue::Expansion(stop), subject)
        }
        TimedStepFailure::Stopped(_) => settled(&ExactProbBuildIssue::UndecidedSuccessor, subject),
        TimedStepFailure::Key(error) => document_failure(error, subject),
        TimedStepFailure::Fault => broken(),
    }
}
