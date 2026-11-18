// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over a derived `deadlock-freedom` request item (FR-626,
//! FR-124, FSpec FR-368), built from bytes as [`super::ClauseSubject`] builds
//! an authored clause's: the checked package, the request's subject binding
//! and the item's budgets are everything it needs.
//!
//! The item decides that every reachable state either enables a step or
//! satisfies a `terminal` declaration of the unit, so it reads no `holds`
//! atom; its counterexample replays through `replay_model_trace` over the
//! same source, named by the subject's `model_root` anchor (FR-128).

use std::collections::BTreeSet;

use filament_exact::{Identifier, Meter};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ModelSubject};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimState, ModelSimulationRequest};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    CertificateRequest, ModelCheckItem, QualifiedName, ReplayRefusal, ReplayTargetMembers,
    RequestTargetWire, TemporalCounterexample,
};
use fsl_route::request::ItemKind;
use fsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use fsl_semantics::check::FormulaNodeId;

use super::clause_item::{replay_over, ReplayInputs, ReplayShape};
use super::items::{joined_replay, run_model_item, CounterexampleReplayFn, ModelItem};
use super::{AnalyzeItem, AnalyzeLimits, SourceProvision, SubjectBinding, TemporalItem};
use crate::claim_family::claim_family;
use crate::model_check::manifest as en1;
use crate::model_check::ModelCheckRequest;

/// The witness transcript of a derived deadlock-freedom counterexample: it
/// carries no values, the payload being the finite prefix (FR-128).
pub(super) const DEADLOCK_TRANSCRIPT: &str = "<<<assertion|model_check|deadlock|>>>";

/// The replay target a derived item names: it has no declaration of its own,
/// and the replay reads only the item's node and occurrence key.
const DERIVED_TARGET: &str = "derived_item";

/// The letters of a derived item: it has no `holds` atom and no activation,
/// so a read of either is a fault of the engine, never a reading.
pub(super) struct NoLetters;

impl NoLetters {
    fn fault() -> AtomStop {
        AtomStop::Fault(InternalFault::new(
            "analyze",
            "a-derived-item-reads-no-atom",
        ))
    }
}

impl ModelLetters for NoLetters {
    fn atom(
        &self,
        _atom: FormulaNodeId,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Err(Self::fault())
    }

    fn activation(
        &self,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Err(Self::fault())
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }

    fn reads(&self, _atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }
}

/// The replay target of a derived item.
pub(super) fn derived_target() -> Option<QualifiedName> {
    QualifiedName::new(vec![Identifier::new(DERIVED_TARGET).ok()?]).ok()
}

/// The EN-1 item that decides the derived deadlock-freedom item anchored at
/// `claim` (the `model_root` occurrence of the subject's one selection) over
/// the binding, with the item's budgets. A derived item carries no `max_depth` (FSpec
/// FR-368-AC-20), so the search has no horizon.
pub(super) fn deadlock_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let simulation = move |binding: &SubjectBinding<'a>| ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(package),
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
    let target = derived_target()?;
    let (binding, form, claim) = (binding.clone(), form.clone(), claim.clone());
    let replay_binding = binding.clone();
    let request = ModelCheckRequest {
        temporal_clock_selections: &[],
        subject: subject(&binding),
        item: ModelCheckItem::DeadlockFreedom,
        max_depth: None,
        limits: form.limits,
        reductions: form.reductions.clone(),
    };
    let certificate = CertificateRequest {
        temporal_clock_selections: &[],
        subject: subject(&binding),
        item: ModelCheckItem::DeadlockFreedom,
        smt_encoding: form.smt_encoding.unwrap_or_default(),
    };
    let replayed = claim.clone();
    let replay: Box<CounterexampleReplayFn<'a>> =
        Box::new(move |payload: &TemporalCounterexample| {
            replay_derived(
                &ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &replay_binding,
                    limits: &limits,
                    provision,
                    max_candidates: form.max_candidates,
                    timed: None,
                },
                &replayed,
                &target,
                payload,
            )
        });
    let replay = joined_replay(replay, package.graph().terminal_keys());
    let family = claim_family(package, ItemKind::DeadlockFreedom, &claim);
    Some(
        run_model_item(
            index,
            family,
            ModelItem {
                request,
                certificate,
                replay,
            },
        )
        .claimed(claim),
    )
}

/// FR-128's replay of a derived deadlock-freedom counterexample, in the
/// envelope that names the item by its anchor.
fn replay_derived(
    inputs: &ReplayInputs<'_, '_>,
    claim: &OccurrenceKey,
    target: &QualifiedName,
    payload: &TemporalCounterexample,
) -> Result<fsl_replay::ModelTraceReplayResult, ReplayRefusal> {
    replay_over(
        inputs,
        ReplayShape {
            target: RequestTargetWire::function(target.clone(), claim.clone()),
            members: ReplayTargetMembers::function(target.clone()),
            node: claim.node().clone(),
            occurrence: claim.clone(),
            transcript: DEADLOCK_TRANSCRIPT,
        },
        payload,
        &NoLetters,
    )
}
