// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over an untimed refinement declaration (FR-630, FR-144),
//! built from bytes as [`super::ClauseSubject`] builds an authored clause's:
//! the checked package, the request's subject binding and the item's budgets
//! are everything it needs.
//!
//! The run is FR-142's safety half and FR-143's liveness half over the
//! explicit-state product ([`check_refinement`]); FR-144's rows settle it.
//! A holding safety half settles `proved` only once FR-149's core checker
//! accepts its simulation certificate, and a violated half settles `refuted`
//! only once FR-145's replay reproduces its counterexample over the same
//! subjects.

use filament_exact::Cancel;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use filament_semantic_value::catalog::InternalFault;
use fsl_replay::{
    open_systems, replay_refinement_trace, settle_refinement, settle_refinement_refusal,
    RefinementTraceSettlement, SimulationCertificateRequest, TemporalCounterexample,
};
use fsl_semantics::model::state::ModelState;
use serde_json::{json, Value};
use fsl_semantics::check::{AbstractSide, Capability, CheckedRefinement, DeclarationFamily};
use fsl_semantics::model::observation::DocumentRef;

use super::items::model_refusal;
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, SubjectBinding, TemporalItem};
use crate::model_check::manifest as en1;
use crate::model_check::{check_refinement, ModelCheckRefusal, RefinementCheckRequest};

/// The checked refinement whose requirement record `claim` keys, when the
/// package declares one.
pub(super) fn refinement_at<'p>(
    package: &'p CheckedPackage,
    claim: &OccurrenceKey,
) -> Option<&'p CheckedRefinement> {
    package
        .graph()
        .refinements()
        .iter()
        .find(|refinement| refinement.requirement_occurrence() == Some(claim))
}

/// The EN-1 refinement item at `claim` over the binding, with the item's
/// budgets, or `None` when the package holds no model-to-model refinement
/// keyed there. A protocol abstract side is FR-147's arm, which this item
/// does not build.
pub(super) fn refinement_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let refinement = refinement_at(package, claim)?;
    if !matches!(refinement.abstract_side(), AbstractSide::Model) {
        return None;
    }
    let concrete = ModelSimulationRequest {
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
    // FR-630 "Request item": the item's `abstract_subject` names the
    // abstract initial snapshots; absent, the concrete subject's are read
    // in the abstract model. The declaring package holds the abstract
    // model's selection (ADR-020 RM-1).
    let abstract_initial: Vec<DocumentRef> = form
        .abstract_initial
        .clone()
        .unwrap_or_else(|| binding.initial.clone());
    let form = form.clone();
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::RelationClaim),
            fsl_route::Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| run(refinement, package, &concrete, abstract_initial, &form, cancel),
        )
        .claimed(claim.clone()),
    )
}

/// FR-142/FR-143's run of `refinement` and FR-144's settlement of it.
fn run(
    refinement: &CheckedRefinement,
    package: &CheckedPackage,
    concrete: &ModelSimulationRequest<'_>,
    abstract_initial: Vec<DocumentRef>,
    form: &TemporalItem,
    cancel: &Cancel,
) -> EngineReport {
    let checked = check_refinement(
        RefinementCheckRequest {
            refinement,
            concrete: ModelSubject {
                simulation: concrete.clone(),
                proof_bounds: form.proof_bounds.clone(),
            },
            abstract_package: package,
            abstract_initial: abstract_initial.clone(),
            abstract_protocol_binding: None,
            max_depth: form.max_depth,
            limits: form.limits,
        },
        cancel,
    );
    let outcome = match checked {
        Ok(outcome) => outcome,
        Err(ModelCheckRefusal::Fault(fault)) => {
            return EngineReport::Model(Box::new(
                settle_refinement_refusal(&fault, &form.limits).record,
            ))
        }
        Err(refusal) => return EngineReport::Refused(model_refusal(&refusal)),
    };
    let certified = SimulationCertificateRequest {
        refinement,
        concrete: concrete.clone(),
        abstract_package: package,
        abstract_initial: abstract_initial.clone(),
    };
    // FR-630 "Outputs": the replay that reproduces a counterexample gives
    // the abstract image of each of its positions.
    let mut steps = None;
    let replay = |payload: &TemporalCounterexample| {
        let systems = open_systems(refinement, concrete, package, abstract_initial.clone())
            .map_err(|_| {
                fsl_replay::ReplayRefusal::Fault(InternalFault::new(
                    "analyze",
                    "the-refinement-subjects-reopen",
                ))
            })?;
        let settled = replay_refinement_trace(refinement, &systems, payload)?;
        if let RefinementTraceSettlement::Reproduced { mapped, .. } = &settled {
            steps = Some(counterexample_steps(refinement, &systems, mapped)?);
        }
        Ok(settled)
    };
    let settled = settle_refinement(&outcome, &form.limits, &certified, cancel, replay);
    EngineReport::Refinement(Box::new(settled.record), steps)
}

/// A reproduced refinement counterexample's members: `steps`, for each position, from
/// 0 to the failing one, the abstract state it maps to in FSpec FR-181's
/// typed canonical form, and `step_simulations`, the refinement's FR-146 step records by their
/// occurrence (role `generated`, ordinal 0 for `Initial`, row position + 1
/// for a `Step`).
fn counterexample_steps(
    refinement: &CheckedRefinement,
    systems: &fsl_replay::RefinementSystems<'_>,
    mapped: &[ModelState],
) -> Result<Value, fsl_replay::ReplayRefusal> {
    let fault =
        |invariant| fsl_replay::ReplayRefusal::Fault(InternalFault::new("analyze", invariant));
    let positions = mapped
        .iter()
        .enumerate()
        .map(|(position, state)| {
            let (_, key) = systems
                .abstract_system
                .state_key_of(state)
                .map_err(|_| fault("a-mapped-state-encodes"))?;
            let state: Value = serde_json::from_slice(key.as_bytes())
                .map_err(|_| fault("a-mapped-state-key-is-json"))?;
            Ok(json!({"position": position, "abstract_state": state}))
        })
        .collect::<Result<Vec<_>, fsl_replay::ReplayRefusal>>()?;
    let records: Vec<Value> = refinement
        .step_simulations()
        .iter()
        .map(|entry| {
            json!({"role": entry.occurrence().origin().role().to_string(),
                "ordinal": entry.occurrence().origin().ordinal()})
        })
        .collect();
    Ok(json!({"steps": positions, "step_simulations": records}))
}
