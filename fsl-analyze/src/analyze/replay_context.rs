// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capture the actual producer replay inputs without changing its settlement.

use std::cell::RefCell;
use std::collections::BTreeMap;

use fsl_package::CheckedPackage;
use fsl_replay::replay_context::{
    ContextDocument, ContextError, DocumentKind, ModelSubject, ProtocolSubject, ReplayContext,
    ReplayFamily,
};
use fsl_replay::{ReplayRequestWire, WitnessEnvelope};
use serde_json::value::RawValue;
use serde_json::Value;

/// A producer's retained serialized context or its explicit retention failure.
/// The native terminal result remains independent of this output artifact.
#[derive(Clone, Debug)]
pub struct CapturedReplayContext {
    raw: Option<Box<RawValue>>,
    failure: Option<String>,
}

impl PartialEq for CapturedReplayContext {
    fn eq(&self, other: &Self) -> bool {
        self.raw.as_deref().map(RawValue::get) == other.raw.as_deref().map(RawValue::get)
            && self.failure == other.failure
    }
}
impl Eq for CapturedReplayContext {}

impl CapturedReplayContext {
    fn from_result(result: Result<ReplayContext, ContextError>) -> Self {
        Self::from_raw_result(result.and_then(|context| context.to_raw()))
    }

    fn from_raw_result(result: Result<Box<RawValue>, ContextError>) -> Self {
        match result {
            Ok(raw) => Self {
                raw: Some(raw),
                failure: None,
            },
            Err(error) => Self {
                raw: None,
                failure: Some(error.to_string()),
            },
        }
    }

    /// Original raw context bytes for the producer's raw outcome writer.
    pub fn raw(&self) -> Option<&RawValue> {
        self.raw.as_deref()
    }

    /// Transfer retained bytes to the external output carrier unchanged.
    pub fn into_raw(self) -> Option<Box<RawValue>> {
        self.raw
    }

    /// Why context retention failed; never a replacement terminal verdict.
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

pub(super) type CaptureSlot = RefCell<Option<CapturedReplayContext>>;
pub(super) struct ModelRecording<'a, 'runtime> {
    pub(super) slot: &'a CaptureSlot,
    pub(super) system: &'a fsl_eval::simulation::model::ModelSystem<'runtime>,
    pub(super) cancel: &'a filament_exact::Cancel,
    pub(super) over: Option<filament_exact::ObjectReference>,
    pub(super) terminal: Result<Option<fsl_eval::simulation::StateKey>, ContextError>,
}

pub(super) fn timed_terminal(
    timed: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
    payload: &fsl_replay::TemporalCounterexample,
    provision: &super::SourceProvision,
    cancel: &filament_exact::Cancel,
) -> Result<Option<fsl_eval::simulation::StateKey>, ContextError> {
    if !payload.prefix.is_empty()
        || !payload
            .lasso
            .as_ref()
            .is_some_and(|lasso| lasso.terminal_stutter)
    {
        return Ok(None);
    }
    let owner = provision.normalization_owner().ok_or_else(|| {
        ContextError::invalid(
            "/payload",
            "original timed admission owner was not retained",
        )
    })?;
    let mut budget = fsl_replay::NormalizationReferenceBudget::source(
        owner,
        fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
    );
    let result = (|| {
        let initial = timed
            .initial_with_model_indices(&mut budget, cancel)
            .map_err(|cause| ContextError::invalid("/payload/initial_state", cause.to_string()))?;
        let (_, state) = initial
            .into_iter()
            .nth(payload.initial_state)
            .ok_or_else(|| {
                ContextError::invalid(
                    "/payload/initial_state",
                    "actual timed initial state is absent",
                )
            })?;
        timed
            .state_key(&state)
            .map(Some)
            .map_err(|cause| ContextError::invalid("/payload", format!("{cause:?}")))
    })();
    if let Some(cause) = budget.take_normalization_failure() {
        return Err(ContextError::invalid(
            "/payload",
            format!("normalization owner stopped: {cause:?}"),
        ));
    }
    result
}
pub(super) type ProtocolCapture = RefCell<
    Option<(
        ReplayRequestWire,
        WitnessEnvelope<
            fsl_replay::TemporalCounterexample<fsl_eval::simulation::protocol::ProtocolTransition>,
        >,
        fsl_package::Emission,
        Result<Option<fsl_eval::simulation::StateKey>, ContextError>,
    )>,
>;

pub(super) struct ProtocolRecording<'a, 'runtime> {
    pub(super) slot: &'a CaptureSlot,
    pub(super) system: &'a fsl_eval::simulation::model::ModelSystem<'runtime>,
    pub(super) cancel: &'a filament_exact::Cancel,
    pub(super) terminal: Result<Option<fsl_eval::simulation::StateKey>, ContextError>,
}

pub(super) fn retain_terminal_key(
    key: &fsl_eval::simulation::StateKey,
    provision: &super::SourceProvision,
    cancel: &filament_exact::Cancel,
) -> Result<fsl_eval::simulation::StateKey, ContextError> {
    use filament_exact::{IntegerDecimalBudget, Reservation};
    cancel.poll();
    if let Some(cause) = cancel.tripped() {
        return Err(ContextError::invalid(
            "/payload",
            format!("native terminal key retention cancelled: {cause:?}"),
        ));
    }
    let owner = provision.normalization_owner().ok_or_else(|| {
        ContextError::invalid(
            "/payload",
            "original protocol admission owner was not retained",
        )
    })?;
    let mut budget = fsl_replay::NormalizationReferenceBudget::source(
        owner,
        fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
    );
    let requested = key.as_bytes().len();
    let work = u64::try_from(requested).map_err(|_| ContextError::Allocation { requested })?;
    if budget.work(work).is_err() {
        return Err(ContextError::invalid(
            "/payload",
            format!(
                "normalization owner stopped: {:?}",
                budget.take_normalization_failure()
            ),
        ));
    }
    budget
        .reserve(requested)
        .map_err(|_| ContextError::Allocation { requested })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(requested)
        .map_err(|_| ContextError::Allocation { requested })?;
    bytes.extend_from_slice(key.as_bytes());
    Ok(fsl_eval::simulation::StateKey::from_canonical_bytes(bytes))
}

pub(super) struct HybridInputs<'a> {
    pub package: &'a CheckedPackage,
    pub binding: super::SubjectBinding<'a>,
    pub limits: super::AnalyzeLimits,
    pub provision: &'a super::SourceProvision,
}

pub(super) struct HybridCapture<'a, 'inputs> {
    pub inputs: &'inputs HybridInputs<'a>,
    pub claim: &'inputs fsl_foundation::source::provenance::OccurrenceKey,
    pub form: &'inputs super::HybridItem,
    pub backend: &'inputs fsl_foundation::BackendId,
    pub budget: fsl_replay::SolverBudget,
    pub emitted: &'inputs [u8],
    pub report: &'inputs super::hybrid_item::HybridReport,
}

pub(super) fn hybrid(capture: HybridCapture<'_, '_>) -> Option<CapturedReplayContext> {
    let found = capture.report.counterexample.as_ref()?;
    let result = (|| {
        let inputs = capture.inputs;
        let graph = inputs.package.graph();
        let position = graph
            .hybrid_claim_sites()
            .iter()
            .position(|site| {
                site.identity == *capture.claim.node() && &site.claim == capture.claim.origin()
            })
            .ok_or_else(|| {
                ContextError::invalid("/subject/claim", "actual checked claim is absent")
            })?;
        let site = &graph.hybrid_claim_sites()[position];
        let checked = graph
            .hybrid_claims()
            .get(position)
            .ok_or_else(|| ContextError::invalid("/subject/claim", "checked claim is absent"))?;
        let model = graph
            .hybrid_models()
            .get(checked.model.get())
            .ok_or_else(|| ContextError::invalid("/subject/model", "checked model is absent"))?;
        let name = filament_exact::Identifier::new(site.name.as_str()).map_err(|_| {
            ContextError::invalid("/request/replay_target", "actual claim is unnamed")
        })?;
        let name = fsl_replay::QualifiedName::new(vec![name])
            .map_err(|error| ContextError::invalid("/request/replay_target", error.to_string()))?;
        let certificate = capture
            .report
            .certificate
            .as_ref()
            .map(|certificate| certificate.form.clone());
        let wire = ReplayRequestWire {
            package: fsl_replay::ReplayPackage::read(
                capture.emitted,
                inputs.limits.replay.input_bytes,
            )
            .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?,
            profile_selections: Vec::new(),
            dependencies: super::request::dependency_wires(inputs.package),
            replay_target: fsl_replay::RequestTargetWire::state_clause(name),
            source: fsl_replay::ReplaySource::Hybrid(found.clone()),
            certificate: certificate.clone(),
            obligation_identity: fsl_package::ObligationIdentity::from_claim_occurrence(
                capture.claim,
            ),
            backend: capture.backend.as_str().to_owned(),
            state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
            accounting_limits: inputs.limits.accounting,
            stage_limits: BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: capture
                .form
                .proof_bounds
                .iter()
                .cloned()
                .map(fsl_replay::DeclaredDomain::new)
                .collect(),
            byte_provision: inputs.provision.wire_entries(),
            documents: inputs
                .binding
                .packages
                .values()
                .chain(inputs.binding.snapshots.values())
                .cloned()
                .collect(),
        };
        let native_wire =
            fsl_semantics::check::wire_id::WireGraph::of_package_bytes(capture.emitted)
                .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?;
        let (request, source_bytes) = fsl_replay::replay_context::request_wire(&wire)?;
        let subject = serde_json::json!({
            "claim": {"node": native_wire.wire_value(capture.claim.node()).map_err(|error| ContextError::invalid("/subject/claim/node", error.to_string()))?,
                "occurrence": {"role": capture.claim.origin().role().as_str(), "ordinal": capture.claim.origin().ordinal()}},
            "model": model.name.as_str(), "proof_bounds": request.declared_domains,
            "budget": {"max_solver_time": capture.budget.time_seconds().map(|value| value.to_string()), "max_solver_steps": capture.budget.steps().map(|value| value.to_string())},
            "options": capture.form.options,
            "deadline_ms": match inputs.limits.deadline { super::DeadlineSupply::Unbounded => None, super::DeadlineSupply::Milliseconds(value) => Some(value.to_string()) },
        });
        let payload = capture
            .report
            .wire
            .clone()
            .map_err(|error| ContextError::invalid("/payload", error.to_string()))?;
        Ok(ReplayContext {
            request,
            source_bytes,
            documents: documents(inputs.package, &inputs.binding, inputs.provision)?,
            family: ReplayFamily::Hybrid,
            payload,
            envelope: None,
            subject,
            certificate,
        })
    })();
    Some(CapturedReplayContext::from_result(result))
}

fn canonical_model_payload(
    inputs: &super::clause_item::ReplayInputs<'_, '_>,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    payload: &fsl_replay::TemporalCounterexample,
    over: Option<&filament_exact::ObjectReference>,
    terminal: Option<&fsl_eval::simulation::StateKey>,
    system: &fsl_eval::simulation::model::ModelSystem<'_>,
    cancel: &filament_exact::Cancel,
) -> Result<Value, ContextError> {
    use filament_exact::IntegerDecimalBudget;
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
    let owner = inputs.provision.normalization_owner().ok_or_else(|| {
        ContextError::invalid(
            "/payload",
            "original model admission owner was not retained",
        )
    })?;
    let mut budget = fsl_replay::NormalizationReferenceBudget::source(
        owner,
        fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
    );
    let result = (|| {
        let catalog = fsl_package::ReferenceCatalog::from_checked_model(
            inputs.package,
            system.reference_model(),
            &mut budget,
        )
        .map_err(|cause| ContextError::invalid("/payload", cause.to_string()))?;
        let codec = fsl_replay::ReferenceCodec::new(&catalog, system)
            .map_err(|cause| ContextError::invalid("/payload", cause.to_string()))?;
        let initial_terminal = if terminal.is_none()
            && inputs.timed.is_none()
            && payload.prefix.is_empty()
            && payload
                .lasso
                .as_ref()
                .is_some_and(|lasso| lasso.terminal_stutter)
        {
            budget
                .work(u64::try_from(inputs.binding.initial.len()).map_err(|_| {
                    ContextError::invalid("/subject/initial", "initial state count overflow")
                })?)
                .map_err(|_| {
                    ContextError::invalid(
                        "/payload",
                        "normalization owner denied initial state admission",
                    )
                })?;
            let initial = TransitionSystem::initial(system);
            let state = initial.get(payload.initial_state).ok_or_else(|| {
                ContextError::invalid(
                    "/payload/initial_state",
                    "actual initial model state is absent",
                )
            })?;
            Some(
                SubjectSystem::canonical_state_key(system, state)
                    .map_err(|cause| ContextError::invalid("/payload", format!("{cause:?}")))?,
            )
        } else {
            None
        };
        codec
            .scope(
                &mut budget,
                cancel,
                usize::MAX,
                quire_canonical::Limits::new(u64::MAX),
            )
            .temporal_wire(
                payload,
                over,
                Some(claim),
                terminal.or(initial_terminal.as_ref()),
            )
            .map_err(|cause| ContextError::invalid("/payload", cause.to_string()))
    })();
    if let Some(cause) = budget.take_normalization_failure() {
        return Err(ContextError::invalid(
            "/payload",
            format!("normalization owner stopped: {cause:?}"),
        ));
    }
    result
}

fn documents(
    package: &CheckedPackage,
    binding: &super::SubjectBinding<'_>,
    provision: &super::SourceProvision,
) -> Result<Vec<ContextDocument>, ContextError> {
    let mut documents = binding
        .packages
        .values()
        .map(|bytes| ContextDocument {
            kind: DocumentKind::DomainPackage,
            selection: None,
            bytes: bytes.clone(),
        })
        .chain(binding.snapshots.values().map(|bytes| ContextDocument {
            kind: DocumentKind::Snapshot,
            selection: None,
            bytes: bytes.clone(),
        }))
        .collect::<Vec<_>>();
    let mut pending = vec![package];
    let mut retained = std::collections::BTreeSet::new();
    while let Some(checked) = pending.pop() {
        for identity in checked.dependency_selections().keys() {
            if !retained.insert(identity.clone()) {
                continue;
            }
            let dependency = checked.dependencies().get(identity).ok_or_else(|| {
                ContextError::invalid("/documents", "selected checked dependency is absent")
            })?;
            let bytes = provision
                .library_packages()
                .and_then(|packages| packages.get(identity))
                .ok_or_else(|| {
                    ContextError::invalid(
                        "/documents",
                        "genuine dependency package handle was not retained at acquisition",
                    )
                })?;
            documents.push(ContextDocument {
                kind: DocumentKind::DependencyPackage,
                selection: Some(identity.to_string()),
                bytes: bytes.bytes().to_vec(),
            });
            pending.push(dependency);
        }
    }
    Ok(documents)
}

pub(super) fn model(
    inputs: &super::clause_item::ReplayInputs<'_, '_>,
    wire: &ReplayRequestWire,
    envelope: &WitnessEnvelope<fsl_replay::TemporalCounterexample>,
    recording: ModelRecording<'_, '_>,
) {
    let ModelRecording {
        slot,
        system,
        cancel,
        over,
        terminal,
    } = recording;
    let result = (|| {
        let terminal = terminal?;
        let payload = canonical_model_payload(
            inputs,
            envelope.occurrence_key(),
            envelope.family_payload(),
            over.as_ref(),
            terminal.as_ref(),
            system,
            cancel,
        )?;
        let (request, source_bytes) = fsl_replay::replay_context::request_wire(wire)?;
        let graph =
            fsl_semantics::check::wire_id::WireGraph::of_package_bytes(wire.package.bytes())
                .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?;
        let subject = fsl_replay::ModelTraceSubject {
            initial: inputs.binding.initial.clone(),
            universes: inputs.binding.universes.clone(),
            expansion: fsl_eval::simulation::model::ExpansionLimits {
                max_candidates: inputs.max_candidates,
            },
            timed: inputs.timed.clone(),
        };
        let context = ReplayContext {
            request,
            source_bytes,
            documents: documents(inputs.package, inputs.binding, inputs.provision)?,
            family: ReplayFamily::ModelTemporal,
            envelope: Some(fsl_replay::replay_context::envelope_wire(
                envelope,
                payload.clone(),
                &graph,
            )?),
            payload,
            subject: serde_json::to_value(ModelSubject::capture(
                &subject,
                &super::clause_item::clock_selections(inputs.package, inputs.provision),
                envelope.family_payload().over_binding.as_ref(),
            )?)?,
            certificate: wire.certificate.clone(),
        };
        context.authenticate_packet()?;
        Ok(context)
    })();
    slot.replace(Some(CapturedReplayContext::from_result(result)));
}

pub(super) fn protocol(
    inputs: &super::clause_item::ReplayInputs<'_, '_>,
    wire: &ReplayRequestWire,
    envelope: &WitnessEnvelope<
        fsl_replay::TemporalCounterexample<fsl_eval::simulation::protocol::ProtocolTransition>,
    >,
    subject: &fsl_replay::ProtocolTraceSubject,
    wire_subject: &fsl_replay::ProtocolWire,
    recording: ProtocolRecording<'_, '_>,
) -> Option<Box<Value>> {
    let ProtocolRecording {
        slot,
        system,
        cancel,
        terminal,
    } = recording;
    let mut spelled = None;
    let result = (|| {
        let terminal = terminal?;
        let payload = super::reference_output::with_source(
            system,
            inputs.provision.normalization_owner(),
            cancel,
            |scope| {
                scope.protocol_temporal_wire(
                    envelope.family_payload(),
                    wire_subject,
                    Some(envelope.occurrence_key()),
                    terminal.as_ref().or_else(|| {
                        envelope
                            .family_payload()
                            .prefix
                            .last()
                            .map(|step| &step.post_state)
                    }),
                )
            },
        )
        .map_err(|cause| ContextError::invalid("/payload", format!("{cause:?}")))?;
        let (request, source_bytes) = fsl_replay::replay_context::request_wire(wire)?;
        let graph =
            fsl_semantics::check::wire_id::WireGraph::of_package_bytes(wire.package.bytes())
                .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?;
        let context = ReplayContext {
            request,
            source_bytes,
            documents: documents(inputs.package, inputs.binding, inputs.provision)?,
            family: ReplayFamily::ProtocolTemporal,
            envelope: Some(fsl_replay::replay_context::envelope_wire(
                envelope,
                payload.clone(),
                &graph,
            )?),
            payload,
            subject: serde_json::to_value(ProtocolSubject::capture(
                subject,
                &super::clause_item::clock_selections(inputs.package, inputs.provision),
            )?)?,
            certificate: wire.certificate.clone(),
        };
        context.authenticate_packet()?;
        let raw = context.to_raw()?;
        spelled = Some(Box::new(context.payload));
        Ok(raw)
    })();
    slot.replace(Some(CapturedReplayContext::from_raw_result(result)));
    spelled
}
