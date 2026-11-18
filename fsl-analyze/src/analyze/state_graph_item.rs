// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over a state-graph claim (`possible`, `always possible`,
//! `unique path`; FR-628, FSpec FR-390): built from bytes as
//! [`super::ClauseSubject`] builds an authored clause's, so the checked
//! package, the request's subject binding and the item's budgets are
//! everything it needs.
//!
//! The run decides every instance of the claim (FR-125 Instancing) and settles
//! each one (FR-169), replaying its evidence through `replay_model_graph`
//! (FR-170). A request item carries one terminal record, so the instance that
//! decides the item is the first refuted one, else the first that is not
//! proved, else the first.

use filament_exact::{Cancel, ObjectReference, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::model_subject::{ModelInstance, ModelSubject};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    GraphEvidence, GraphEvidenceEnvelope, GraphReplayResult, ModelTraceSubject, ObligationIdentity,
    ProfileSelection, ReplayLimits, ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplayResult,
    ReplaySource, RequestTargetWire, StateGraphInstanceOutcome, StateGraphOutcome,
    StateGraphPropertyForm, StateGraphRecord, TerminalValue, TrapClosure, WitnessPath,
};
use fsl_route::StateGraphFormKind;
use fsl_semantics::check::{Capability, CheckedStateGraphClaim, StateGraphForm};

use super::items::{model_refusal, INVALID_VALUE};
use super::{
    AnalyzeItem, AnalyzeLimits, EngineReport, GraphReport, Refusal, SourceProvision,
    SubjectBinding, TemporalItem,
};
use crate::model_check::manifest as en1;
use crate::model_check::{
    check_and_settle_state_graph, StateGraphItem, StateGraphPipelineRefusal, StateGraphRequest,
};

/// The property form of a checked claim.
fn property_form(form: &StateGraphForm) -> StateGraphPropertyForm {
    match StateGraphFormKind::of(form) {
        StateGraphFormKind::Possible => StateGraphPropertyForm::Possible,
        StateGraphFormKind::AlwaysPossible => StateGraphPropertyForm::AlwaysPossible,
        StateGraphFormKind::UniquePath => StateGraphPropertyForm::UniquePath,
    }
}

/// The EN-1 item that decides the state-graph claim at `key` over the
/// binding, with the item's budgets, or `None` when the package holds no
/// such claim.
pub(super) fn state_graph_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let claim = package
        .graph()
        .state_graph_claims()
        .iter()
        .find(|claim| claim.occurrence() == Some(key))?;
    let (binding, form) = (binding.clone(), form.clone());
    let family =
        crate::claim_family::claim_family(package, fsl_route::request::ItemKind::Formula, key);
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            fsl_route::Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                run(
                    &Run {
                        package,
                        binding: &binding,
                        limits,
                        provision,
                        claim,
                        form: &form,
                    },
                    cancel,
                )
            },
        )
        .claimed(key.clone()),
    )
}

/// What one state-graph run reads.
#[derive(Clone, Copy)]
struct Run<'r, 'a> {
    package: &'a CheckedPackage,
    binding: &'r SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'r SourceProvision,
    claim: &'a CheckedStateGraphClaim,
    form: &'r TemporalItem,
}

impl<'a> Run<'_, 'a> {
    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            input: fsl_eval::simulation::model::ModelCheckInput::source(self.package),
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: self.form.max_candidates,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }
}

fn run(read: &Run<'_, '_>, cancel: &Cancel) -> EngineReport {
    let Run { claim, form, .. } = *read;
    let Ok(system) = ModelSystem::new(read.simulation()) else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let mut model_limits = form.limits;
    if let Some(samples) = form.witness_samples {
        model_limits.witness_samples = samples;
    }
    let request = StateGraphRequest {
        subject: ModelSubject {
            simulation: read.simulation(),
            proof_bounds: form.proof_bounds.clone(),
        },
        temporal_clock_selections: Vec::new(),
        items: vec![StateGraphItem::new(claim)],
        reductions: form.reductions.clone(),
        deadlock_freedom: false,
        max_depth: form.max_depth,
        limits: model_limits,
        seed: form.seed,
    };
    let replay = |instance: &StateGraphInstanceOutcome| {
        replay_instance(read, &system, instance).map_err(|refusal| *refusal)
    };
    let settled = match check_and_settle_state_graph(request, cancel, replay) {
        Ok(settled) => settled,
        Err(StateGraphPipelineRefusal::Engine(refusal)) => {
            return EngineReport::Refused(model_refusal(&refusal));
        }
        Err(StateGraphPipelineRefusal::HorizonOutOfRange) => {
            return EngineReport::Refused(Refusal::Declined(
                fsl_replay::ProofRefusalCause::InvalidInput,
                fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                INVALID_VALUE,
            ));
        }
    };
    let (Some(records), Some(instances)) = (settled.records.first(), settled.run.items.first())
    else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let chosen = decisive(records);
    let (Some(record), Some(instance)) = (records.get(chosen), instances.get(chosen)) else {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    };
    let property = property_form(claim.form());
    let parameter = claim.over().map(|over| over.name.as_str());
    let spelled = evidence_of(&system, claim, instance).map(|evidence| {
        fsl_replay::graph_evidence_wire(
            &evidence,
            property,
            parameter,
            instance.over.as_ref(),
            &instance.sampled,
            claim.occurrence(),
        )
    });
    let refuted = matches!(record.value, TerminalValue::Refuted);
    // A refuted record and a proof by witnesses (FR-628 Outputs) each carry
    // their evidence; one that cannot be spelled is a fault of the engine, never
    // a record without it.
    let witnessed = matches!(record.value, TerminalValue::Proved { .. })
        && matches!(
            instance.outcome,
            StateGraphOutcome::Witnessed { .. }
                | StateGraphOutcome::HoldsReduced {
                    witnesses: Some(_),
                    ..
                }
        );
    let wire = spelled.and_then(Result::ok);
    if (refuted || witnessed) && wire.is_none() {
        return EngineReport::Refused(Refusal::Failed(super::items::BROKEN));
    }
    let (counterexample, evidence) = if refuted {
        (wire, None)
    } else {
        (None, wire.filter(|_| witnessed))
    };
    let graph = &settled.run.graph;
    let exploration = fsl_replay::RunStats {
        product_states: graph.stats.nodes,
        product_edges: graph.stats.edges,
        depth: graph.stats.depth,
        ..fsl_replay::RunStats::default()
    };
    EngineReport::StateGraph(Box::new(GraphReport {
        record: record.clone(),
        exploration,
        counterexample,
        evidence,
    }))
}

/// The index of the instance that decides the item.
fn decisive(records: &[StateGraphRecord]) -> usize {
    records
        .iter()
        .position(|record| matches!(record.value, TerminalValue::Refuted))
        .or_else(|| {
            records
                .iter()
                .position(|record| !matches!(record.value, TerminalValue::Proved { .. }))
        })
        .unwrap_or(0)
}

/// The FR-393 evidence an instance's outcome settles from, bound to the
/// instance it was found for.
fn evidence_of(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    instance: &StateGraphInstanceOutcome,
) -> Option<GraphEvidence> {
    let over = bound_instance(system, claim, instance.over.as_ref())?;
    let witnesses = |paths: &[fsl_replay::GraphWitness]| {
        paths
            .iter()
            .map(|found| (WitnessPath::Path(found.path.clone()), found.source.clone()))
            .collect()
    };
    match &instance.outcome {
        StateGraphOutcome::Witnessed { paths, .. }
        | StateGraphOutcome::WitnessedUnchecked { paths, .. }
        | StateGraphOutcome::HoldsReduced {
            witnesses: Some(paths),
            ..
        } => Some(GraphEvidence::Witness {
            paths: witnesses(paths),
            over: over.clone(),
        }),
        StateGraphOutcome::Trapped { stem } => Some(GraphEvidence::Trap {
            stem: stem.clone(),
            closure: TrapClosure::StateGraph,
            over: over.clone(),
        }),
        StateGraphOutcome::PathPair {
            stem,
            first,
            second,
        } => Some(GraphEvidence::PathPair {
            stem: stem.clone(),
            first: first.clone(),
            second: second.clone(),
            over: over.clone(),
        }),
        StateGraphOutcome::Undefined { stem, undefined } => Some(GraphEvidence::Undefined {
            stem: stem.clone(),
            undefined: undefined.clone(),
            over: over.clone(),
        }),
        _ => None,
    }
}

/// The instance `reference` is the binding of: `Some(None)` for a claim with
/// no `over` parameter, `None` when the reference names no instance.
fn bound_instance(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    reference: Option<&ObjectReference>,
) -> Option<Option<ModelInstance>> {
    let (Some(over), Some(reference)) = (claim.over(), reference) else {
        return Some(None);
    };
    system
        .instances(&ValueType::Reference(over.object.clone()))
        .ok()?
        .into_iter()
        .find(|instance| {
            system
                .bound_reference(over.object.clone(), instance)
                .is_ok_and(|bound| &bound == reference)
        })
        .map(Some)
}

/// FR-170's replay of an instance's evidence over the same source.
fn replay_instance(
    read: &Run<'_, '_>,
    system: &ModelSystem<'_>,
    instance: &StateGraphInstanceOutcome,
) -> Result<GraphReplayResult, Box<ReplayRefusal>> {
    let fault =
        |cause: &'static str| Box::new(ReplayRefusal::Fault(InternalFault::new("analyze", cause)));
    let evidence =
        evidence_of(system, read.claim, instance).ok_or_else(|| fault("graph-evidence"))?;
    let occurrence = read
        .claim
        .occurrence()
        .cloned()
        .ok_or_else(|| fault("graph-claim-occurrence"))?;
    let node = occurrence.node().clone();
    let emission =
        fsl_package::emit_checked_with_temporal_selections(read.package, &[], read.limits.emit)
            .map_err(|_| fault("package-emission"))?;
    let package = ReplayPackage::read(
        emission.package().bytes(),
        ReplayLimits::default().input_bytes,
    )
    .map_err(|_| fault("package-emission"))?;
    let obligation = ObligationIdentity::new(node.clone(), None, 0);
    let envelope = GraphEvidenceEnvelope::new(
        obligation.clone(),
        occurrence,
        node,
        Vec::<ProfileSelection>::new(),
        read.limits.accounting,
        Vec::new(),
        fsl_foundation::BackendId::new(en1::BACKEND_ID),
        evidence,
    );
    let wire = ReplayRequestWire {
        package,
        profile_selections: Vec::new(),
        dependencies: super::request::dependency_wires(read.package),
        replay_target: RequestTargetWire::default(),
        source: ReplaySource::ModelGraph(Box::new(envelope)),
        obligation_identity: obligation,
        backend: en1::BACKEND_ID.to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: read.limits.accounting,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: read.provision.wire_entries(),
        documents: read
            .binding
            .packages
            .values()
            .chain(read.binding.snapshots.values())
            .cloned()
            .collect(),
    };
    let subject = ModelTraceSubject {
        timed: None,
        initial: read.binding.initial.clone(),
        universes: read.binding.universes.clone(),
        expansion: ExpansionLimits {
            max_candidates: read.form.max_candidates,
        },
    };
    match fsl_replay::replay_model_graph(wire, &subject, ReplayLimits::default())
        .map_err(Box::new)?
    {
        ReplayResult::ModelGraph(replayed) => Ok(replayed),
        _ => Err(fault("graph-replay-result")),
    }
}
