// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-875: an `on each` probabilistic claim settled one instance at a time.
//!
//! The replay crate owns the per-instance phase graphs, the FR-201
//! `conditional` ratio certificate and the instance refutation witness; this
//! adapter runs them under the item's limits and settles through the same
//! checked-report and replayed-refutation path as every exact item.

use filament_exact::Cancel;
use fsl_eval::simulation::model::ModelSystem;
use fsl_replay::statistical::SubjectLimits;
use fsl_replay::{
    check_probability_certificate_document, on_each_outcome, replay_on_each_witness,
    ExactProbBuildIssue, ExactProbabilityCertificate, OnEachOutcome, ProbabilisticCounterexample,
    ProbabilityDocumentLimits,
};

use super::{
    broken, checked_report, document_failure, finish_replay, graph_failure, settled, EngineReport,
    ExactReport, ProductRequest, Refusal,
};

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let probabilistic = simulation.input.probabilistic;
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, meter);
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let admitted = ModelSystem::new_with_cancel(simulation, cancel);
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let Ok(system) = admitted else {
        return broken();
    };
    let claims = probabilistic.claims();
    let outcome = match on_each_outcome(
        &system,
        claims,
        &item,
        document_limits,
        limits.max_witness_paths,
        cancel,
    ) {
        Ok(outcome) => outcome,
        Err(error) => return failure(error, &limits, subject),
    };
    match outcome {
        OnEachOutcome::NoInstance => EngineReport::Exact(Box::new(ExactReport::NoInstance)),
        OnEachOutcome::Holds { document } => {
            let checked = check_probability_certificate_document(
                &system,
                claims,
                &item,
                &document,
                document_limits,
                cancel,
            );
            let certificate = match &checked {
                Ok(fsl_replay::ExactProbabilityCertificateCheck::Accepted(accepted)) => {
                    accepted.certificate().clone()
                }
                _ => ExactProbabilityCertificate { values: Vec::new() },
            };
            checked_report(checked, certificate, None, None, subject)
        }
        OnEachOutcome::Fails { evidence } => {
            let mut witness = *evidence;
            if let Some(tamper) = tamper_witness {
                tamper(&mut witness);
            }
            let replayed = match replay_on_each_witness(
                &system,
                claims,
                &item,
                &witness,
                document_limits,
                limits.max_witness_paths,
                cancel,
            ) {
                Ok(replayed) => replayed,
                Err(fsl_replay::ProbabilityDocumentError::Unsupported) => return super::not_yet(),
                Err(error) => return failure(error, &limits, subject),
            };
            finish_replay(
                Ok(replayed),
                claim,
                &system,
                &witness,
                &[],
                &limits,
                subject,
            )
        }
    }
}

/// How a failed on-each run settles: the graph and budget causes as every
/// exact route settles them (FR-741 Budgets), an undefined guard or target
/// read as an item this route does not yet refute by undefined path, and the
/// rest through the shared document failures.
fn failure(
    error: fsl_replay::ProbabilityDocumentError,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    match error {
        fsl_replay::ProbabilityDocumentError::Graph(error) => graph_failure(error, limits, subject),
        // The reset-construction optimum, a cyclic phase graph, a form other than
        // unbounded `eventually`, a timed claim and a fairness set are not built.
        fsl_replay::ProbabilityDocumentError::Unsupported => super::not_yet(),
        fsl_replay::ProbabilityDocumentError::UndefinedPredicate(_) => super::not_yet(),
        error => document_failure(error, subject),
    }
}
