// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual source protocol admission and native replay of the supplied prefix.

use filament_exact::{Cancel, Meter, Value as NativeValue};
use fsl_eval::model_subject::{ModelSubject, ProtocolSubject as NativeSubject};
use fsl_eval::simulation::model::{
    ModelCheckInput, ModelSimulationRequest, ModelSystem, ProtocolDefinition,
};
use fsl_eval::simulation::protocol::{
    check_protocol_bounds, live_instance_bound, Capacities, CapacityRefusal, MemoryModel,
    ProtocolLoopProofKind, ProtocolLoopProofTarget, ProtocolRefusal, ProtocolSystem, Sc,
    SubjectMemory,
};
use fsl_eval::simulation::{TransitionSystem, BOUND_POSITIONS};

use super::{NativeContextError, ReconstructedContext};
use crate::replay_context::{
    ContextError, NativeReplayOutput, ProtocolSubject, ReplayFamily, RestoreBudget,
    SourceDescriptor,
};
use crate::{ReplaySource, Witness};

impl ReconstructedContext {
    /// Admit the actual protocol subject and reproduce its supplied native
    /// transition prefix, returning a fresh independent replay result.
    pub fn replay_protocol_output(self) -> Result<NativeReplayOutput, NativeContextError> {
        let Self {
            context,
            common,
            compiled,
            caller,
            consumer,
        } = self;
        if context.family != ReplayFamily::ProtocolTemporal {
            return Err(ContextError::invalid("/family", "expected protocol-temporal").into());
        }
        let packet = context
            .envelope
            .as_ref()
            .ok_or_else(|| ContextError::invalid("/envelope", "missing protocol witness packet"))?;
        let source: SourceDescriptor =
            serde_json::from_value(packet.source.clone()).map_err(ContextError::from)?;
        if source.kind != "witness" {
            return Err(ContextError::invalid(
                "/envelope/source/kind",
                "expected original protocol witness",
            )
            .into());
        }
        let witness = Witness::parse(source.transcript).map_err(|cause| {
            ContextError::invalid("/envelope/source/transcript", cause.to_string())
        })?;
        let original =
            common.bind_source(ReplaySource::Witness(witness), context.certificate.clone())?;
        let selected: ProtocolSubject =
            serde_json::from_value(context.subject.clone()).map_err(ContextError::from)?;
        let package = compiled.checked.package();
        let protocol = package
            .graph()
            .protocol(selected.protocol_name())
            .ok_or_else(|| {
                ContextError::invalid(
                    "/subject/protocol",
                    "protocol is absent from the actual source graph",
                )
            })?;
        let (subject, _) = selected.native(&ProtocolDefinition::Source(protocol))?;
        let input = crate::replay_context::TemporalInput::read(&context.payload)?;
        let loop_target = if input.kind.as_str() == Some("loop-proof") {
            let proof = crate::model_check::replay::protocol_readback::loop_proof(&input)?;
            Some(ProtocolLoopProofTarget {
                repeat: fsl_semantics::check::ProtocolNodeId(proof.repeat),
                kind: match proof.obligation {
                    crate::LoopProofObligation::Invariant => ProtocolLoopProofKind::Invariant,
                    crate::LoopProofObligation::Termination => ProtocolLoopProofKind::Termination,
                },
            })
        } else {
            None
        };
        let snapshots = original.byte_provision().snapshots();
        let simulation = ModelSimulationRequest {
            input: ModelCheckInput::source(package),
            packages: &compiled.packages,
            normalization: caller.spine.model,
            snapshots: &snapshots,
            initial: subject.model.initial.clone(),
            universes: subject.model.universes.clone(),
            observation: caller.observation,
            expansion: subject.model.expansion,
            meter: consumer.accounting(),
            emit: caller.spine.package.emit(),
        };
        let model =
            ModelSystem::new(simulation.clone()).map_err(crate::ReplayRefusal::Admission)?;
        let capacities = Capacities::admit(protocol, &subject.capacities)
            .map_err(|cause| native_protocol_refusal(ProtocolRefusal::Capacity(cause)))?;
        if let Some(missing) = capacities.unbound().first() {
            return Err(native_protocol_refusal(ProtocolRefusal::Capacity(
                CapacityRefusal::Missing {
                    name: missing.name.clone(),
                },
            )));
        }
        check_protocol_bounds(&model, protocol, &capacities, package, BOUND_POSITIONS).map_err(
            |cause| {
                crate::ReplayRefusal::ModelTrace(Box::new(crate::ModelTraceRefusal::NotSimulated(
                    Box::new(cause),
                )))
            },
        )?;
        let native = NativeSubject {
            model: ModelSubject {
                simulation,
                proof_bounds: Vec::new(),
            },
            protocol: subject.protocol.clone(),
            over: subject.over.clone(),
            roles: subject.roles.clone(),
            capacities: subject.capacities.clone(),
            memory: subject.memory.clone(),
        };
        let readback = ProtocolReadback {
            compiled: &compiled,
            original: &original,
            caller: &caller,
            consumer: &consumer,
            subject: &subject,
            wire_instance_bound: fsl_eval::simulation::protocol::activates_on_each(protocol)
                .then_some(live_instance_bound(subject.max_live_instances)),
            packet,
            supplied: &context.payload,
        };
        match SubjectMemory::of(
            &subject.memory,
            subject.max_store_buffer,
            subject.max_messages,
        ) {
            SubjectMemory::Sc => readback.run(ProtocolSystem::new_with_loop_proof(
                model,
                &native,
                Sc,
                loop_target,
            )),
            SubjectMemory::Tso(memory) => readback.run(ProtocolSystem::new_with_loop_proof(
                model,
                &native,
                memory,
                loop_target,
            )),
            SubjectMemory::Ra(memory) => readback.run(ProtocolSystem::new_with_loop_proof(
                model,
                &native,
                memory,
                loop_target,
            )),
        }
    }
}

fn native_protocol_refusal(cause: ProtocolRefusal) -> NativeContextError {
    crate::ReplayRefusal::ModelTrace(Box::new(crate::ModelTraceRefusal::Protocol(cause))).into()
}

struct ProtocolReadback<'a> {
    compiled: &'a crate::execute::Recompiled,
    original: &'a crate::ReplayRequest,
    caller: &'a crate::limits::CallerLimits,
    consumer: &'a crate::replay_context::ConsumerLimits,
    subject: &'a crate::ProtocolTraceSubject,
    wire_instance_bound: Option<u64>,
    packet: &'a crate::replay_context::EnvelopeDescriptor,
    supplied: &'a serde_json::Value,
}

impl ProtocolReadback<'_> {
    fn run<M: MemoryModel>(
        &self,
        system: Result<ProtocolSystem<'_, M>, ProtocolRefusal>,
    ) -> Result<NativeReplayOutput, NativeContextError> {
        let Self {
            compiled,
            original,
            caller,
            consumer,
            subject,
            wire_instance_bound,
            packet,
            supplied,
        } = *self;
        let system = system
            .map_err(native_protocol_refusal)?
            .with_max_live_instances(live_instance_bound(subject.max_live_instances));
        let input = crate::replay_context::TemporalInput::read(supplied)?;
        let initial = system.initial();
        let state = initial.get(input.initial_state).ok_or_else(|| {
            ContextError::invalid(
                "/payload/initial_state",
                "initial state index is not admitted",
            )
        })?;
        let NativeValue::Reference(over) = state.over() else {
            return Err(ContextError::invalid(
                "/subject/over",
                "actual protocol initial state has no reference binding",
            )
            .into());
        };
        let wire_subject = crate::ProtocolWire {
            protocol: subject.protocol.clone(),
            over: over.clone(),
            roles: subject
                .roles
                .iter()
                .map(|role| (role.role.clone(), role.model.clone()))
                .collect(),
            capacities: subject
                .capacities
                .iter()
                .map(|capacity| (capacity.name.clone(), capacity.value))
                .collect(),
            max_live_instances: wire_instance_bound,
        };
        let claim = packet.occurrence(&original.to_wire())?;
        let source = compiled.checked.package();
        let model_index = system
            .initial_model_index(input.initial_state)
            .ok_or_else(|| {
                ContextError::invalid(
                    "/payload/initial_state",
                    "protocol initial state has no original model snapshot",
                )
            })?;
        let mut meter = Meter::new(consumer.accounting());
        let mut budget = RestoreBudget::new(&mut meter);
        let cancel = Cancel::new();
        let node = source
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.identity() == *claim.node() && node.claim() == claim.origin());
        let letters = node
            .map(|node| system.model().clause_letters(node))
            .transpose()
            .map_err(|_| {
                ContextError::invalid(
                    "/subject",
                    "actual protocol clause letters could not be built",
                )
            })?;
        let restored = crate::replay_context::with_source_reference_codec(
            system.model(),
            compiled.emitted.emission(),
            &compiled.packages,
            caller.spine.imports,
            caller.spine.package.emit(),
            &mut budget,
            |codec, budget| {
                codec
                    .restore_model_over_binding(
                        &supplied["subject"]["over"],
                        Some(&subject.over),
                        model_index,
                        budget,
                        &cancel,
                        usize::MAX,
                    )
                    .map_err(crate::replay_context::ReferenceContextError::from)?;
                if !input.over_binding.is_null() {
                    codec
                        .restore_model_over_binding(
                            &input.over_binding,
                            Some(&subject.over),
                            model_index,
                            budget,
                            &cancel,
                            usize::MAX,
                        )
                        .map_err(crate::replay_context::ReferenceContextError::from)?;
                }
                Ok(crate::model_check::replay::protocol_readback::admit(
                    &system,
                    supplied,
                    &wire_subject,
                    &claim,
                    codec,
                    budget,
                    &cancel,
                    |input, resolved, budget| {
                        crate::model_check::replay::protocol_readback::metadata(
                            &system,
                            input,
                            resolved,
                            subject,
                            node,
                            letters.as_ref(),
                            budget,
                        )
                    },
                ))
            },
        );
        if let Some(stop) = budget.into_stop() {
            return Err(stop.into());
        }
        let payload = restored??;
        let envelope = packet.reconstruct(&original.to_wire(), payload, consumer.replay())?;
        let no_letters = crate::model_check::replay::reconstruction::NoLetters;
        let letters: &dyn fsl_eval::model_subject::ModelLetters = letters.as_ref().map_or(
            &no_letters as &dyn fsl_eval::model_subject::ModelLetters,
            |letters| letters,
        );
        let result = crate::model_check::replay::reconstruction::protocol(
            compiled,
            subject,
            &system,
            &envelope,
            letters,
            consumer.accounting(),
        )?;
        Ok(NativeReplayOutput::temporal(
            ReplayFamily::ProtocolTemporal,
            &result,
            &envelope.family_payload().kind,
            &supplied["kind"],
        )?)
    }
}
