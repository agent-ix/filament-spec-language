// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-625 and FR-951: the EN-1 item over an `invariant` state clause in a
//! request that names a model subject, built from bytes as the derived
//! deadlock-freedom item is: the checked package, the request's subject
//! binding and the item's budgets are everything it needs.
//!
//! The item decides `always holds(I)`, `I` the invariant's body over every
//! object of a state whose type conforms to its context type. Its proof is
//! certified under the invariant key-context root, and its counterexample,
//! a finite prefix to the first state where `I` is false or undefined,
//! replays through `replay_model_trace` naming the invariant as its
//! `state_clause` target (FR-128).

use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    CertificateRequest, ModelCheckItem, QualifiedName, ReplayTargetMembers, RequestTargetWire,
    TemporalCounterexample,
};
use fsl_semantics::check::DeclarationFamily;

use super::clause_item::{replay_over, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::derived_item::NoLetters;
use super::items::{run_model_item, CounterexampleReplayFn, ModelItem};
use super::{AnalyzeItem, AnalyzeLimits, SourceProvision, SubjectBinding, TemporalItem};
use crate::model_check::manifest as en1;
use crate::model_check::ModelCheckRequest;

/// The EN-1 item that decides the `invariant` state clause at `claim` (its
/// `state_clause` node and `claim` occurrence) over the binding, with the
/// budgets of `form`; `None` when `claim` names no checked invariant.
pub(super) fn invariant_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let clause = package.graph().state_clauses().iter().find(|clause| {
        clause.is_invariant()
            && clause.identity() == *claim.node()
            && clause.claim() == claim.origin()
    })?;
    let name = QualifiedName::new(vec![filament_exact::Identifier::new(clause.name()).ok()?]).ok()?;
    let simulation = move |binding: &SubjectBinding<'a>| ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits {
            max_candidates: form.max_candidates,
        },
        meter: limits.accounting,
        emit: limits.emit,
    };
    let subject = |binding: &SubjectBinding<'a>| ModelSubject {
        simulation: simulation(binding),
        proof_bounds: form.proof_bounds.clone(),
    };
    let item = ModelCheckItem::Invariant { clause };
    let request = ModelCheckRequest {
        temporal_clock_selections: &[],
        subject: subject(binding),
        item,
        max_depth: form.max_depth,
        limits: form.limits,
        reductions: form.reductions.clone(),
    };
    let certificate = CertificateRequest {
        temporal_clock_selections: &[],
        subject: subject(binding),
        item,
        smt_encoding: form.smt_encoding.unwrap_or_default(),
    };
    let replay_binding = binding.clone();
    let max_candidates = form.max_candidates;
    let occurrence = claim.clone();
    let replay: Box<CounterexampleReplayFn<'a>> =
        Box::new(move |payload: &TemporalCounterexample| {
            replay_over(
                &ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &replay_binding,
                    limits: &limits,
                    provision,
                    max_candidates,
                    timed: None,
                },
                ReplayShape {
                    target: RequestTargetWire::state_clause(name.clone()),
                    members: ReplayTargetMembers::state_clause(name.clone()),
                    node: occurrence.node().clone(),
                    occurrence: occurrence.clone(),
                    transcript: CLAUSE_TRANSCRIPT,
                },
                payload,
                &NoLetters,
            )
        });
    Some(
        run_model_item(
            index,
            Some(DeclarationFamily::StateModel),
            ModelItem {
                request,
                certificate,
                replay,
            },
        )
        .claimed(claim.clone()),
    )
}
