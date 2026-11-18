// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over a protocol subject read from an analyze request (FSL
//! FR-1033): an authored temporal clause, or the derived `deadlock-freedom`
//! or `race-freedom` item of the protocol, decided under the memory model each
//! outermost `parallel` resolves to (FR-219) and replayed under it (FR-217).
//!
//! Like [`super::ClauseSubject`], the item is built from borrows of the
//! caller's inputs; the model system, the clause letters and the protocol
//! subject are built inside its work, where they live as long as the run, its
//! settlement and its replay.

use filament_semantic_value::catalog::{CatalogCode, InternalFault};
use fsl_eval::model_subject::{ModelInstance, ModelLetters, ProtocolSubject};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_eval::simulation::protocol::{
    classify_accesses, ProtocolTransition, ResolvedMemory,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    replay_protocol_trace_with_temporal_selections, DeclineCode, ModelCheckItem,
    ModelTraceReplayResult, ModelTraceSubject, ProofRefusalCause, ProtocolCertificateRequest,
    ProtocolTraceSubject, ReplayLimits, ReplayRefusal, ReplayTargetMembers,
    RequestTargetWire, TemporalCounterexample,
};
use fsl_route::request::ItemKind;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, TemporalClauseNode};

use super::clause_item::{clause_name, replay_wire, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::derived_item::{derived_target, NoLetters};
use super::items::{admission, settle_protocol_item, ProtocolItem, BROKEN};
use super::request::ProtocolBinding;
use super::{
    AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SourceProvision, SubjectBinding,
    TemporalItem,
};
use crate::claim_family::claim_family;
use crate::model_check::{manifest as en1, ProtocolCheckRequest};
use crate::protocol_analysis::subject::{
    protocol_subject, SubjectBindings, SubjectPackage, SubjectRefusal,
};

/// The witness transcript of a derived `race-freedom` counterexample: it
/// carries no values, the payload being the racing prefix (FR-128).
const RACE_TRANSCRIPT: &str = "<<<assertion|model_check|race|>>>";

/// The witness transcript of a derived `deadlock-freedom` counterexample.
const DEADLOCK_TRANSCRIPT: &str = "<<<assertion|model_check|deadlock|>>>";

/// The object `over` binds, as the model's population key and object key; the
/// population's declaration key is the universe's when the request supplied
/// one, else the key its identity spells.
fn over_instance(binding: &SubjectBinding<'_>, over: &(String, String)) -> ModelInstance {
    let (population, key) = over;
    let declaration = binding
        .universes
        .iter()
        .find(|universe| universe.population.node == *population)
        .map(|universe| universe.population.clone())
        .unwrap_or_else(|| filament_checked::key::DeclarationKey {
            package: population
                .strip_prefix("ix://")
                .and_then(|rest| rest.rsplit_once('/'))
                .map_or_else(String::new, |(package, _)| package.to_owned()),
            node: population.clone(),
        });
    ModelInstance {
        population: declaration,
        key: key.clone(),
    }
}

fn simulation<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: &AnalyzeLimits,
    max_candidates: u64,
) -> ModelSimulationRequest<'a> {
    ModelSimulationRequest {
        package,
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits { max_candidates },
        meter: limits.accounting,
        emit: limits.emit,
    }
}

fn subject_of<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: &AnalyzeLimits,
    max_candidates: u64,
    protocol: &ProtocolBinding,
    over: &ModelInstance,
) -> Result<ProtocolSubject<'a>, SubjectRefusal> {
    protocol_subject(
        SubjectPackage {
            package,
            packages: binding.packages,
            snapshots: binding.snapshots,
        },
        SubjectBindings {
            protocol: protocol.protocol.clone(),
            over: over.clone(),
            roles: protocol.roles.clone(),
            capacities: protocol.capacities.clone(),
            memory: protocol.memory.clone(),
            initial: binding.initial.clone(),
            universes: binding.universes.clone(),
            normalization: limits.normalization,
            observation: limits.observation,
            expansion: ExpansionLimits { max_candidates },
            meter: limits.accounting,
            emit: limits.emit,
        },
    )
}

/// Whether the protocol subject `protocol` binds has a non-atomic shared
/// access (ADR-025 MA-2): the condition under which its race-freedom item is
/// derived (FR-224). A subject EN-1 cannot classify derives none.
pub(super) fn has_non_atomic_access(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    limits: &AnalyzeLimits,
    protocol: &ProtocolBinding,
) -> bool {
    let Some(over) = protocol.over.as_ref() else {
        return false;
    };
    let over = over_instance(binding, over);
    let Some(checked) = package.graph().protocol(&protocol.protocol) else {
        return false;
    };
    let Ok(memory) = ResolvedMemory::resolve(checked, &protocol.memory) else {
        return false;
    };
    let Ok(model) = ModelSystem::new(simulation(
        package,
        binding,
        limits,
        super::clause_item::DEFAULT_MAX_CANDIDATES,
    )) else {
        return false;
    };
    classify_accesses(&model, checked, &memory, &over, &protocol.capacities)
        .is_ok_and(|accesses| !accesses.non_atomic().is_empty())
}

/// The `temporal-satisfaction` item that decides the claim at `claim` over
/// the protocol subject `protocol` binds: the authored clause there, or the
/// derived item `form.item_kind` names anchored there. `None` when the
/// package holds neither.
pub(super) fn protocol_clause_item<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
    protocol: &ProtocolBinding,
) -> Option<AnalyzeItem<'a>> {
    let node: Option<&'a TemporalClauseNode> = match form.item_kind {
        ItemKind::Formula => Some(
            package
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.identity() == *claim.node() && node.claim() == claim.origin())?,
        ),
        ItemKind::RaceFreedom | ItemKind::DeadlockFreedom => None,
        ItemKind::TimeLockFreedom | ItemKind::Refinement => return None,
    };
    if let Some(node) = node {
        clause_name(node).ok()?;
    }
    package.graph().protocol(&protocol.protocol)?;
    let family = claim_family(package, form.item_kind, claim);
    let (binding, form, protocol, claim) = (
        binding.clone(),
        form.clone(),
        protocol.clone(),
        claim.clone(),
    );
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                let Some((population, key)) = protocol.over.clone() else {
                    let mut detail = serde_json::Map::new();
                    detail.insert("member".to_owned(), serde_json::json!("over"));
                    return EngineReport::Refused(Refusal::DeclinedWithDetail(
                        ProofRefusalCause::InvalidInput,
                        DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        CatalogCode::new("invalid_runtime_input", "missing-member"),
                        detail,
                    ));
                };
                let over = over_instance(&binding, &(population, key));
                let subject = |max_candidates| {
                    subject_of(package, &binding, &limits, max_candidates, &protocol, &over)
                };
                let refused = |refusal: &SubjectRefusal| {
                    EngineReport::Refused(match refusal {
                        SubjectRefusal::UnknownProtocol(_) | SubjectRefusal::Memory(_) => {
                            Refusal::Declined(
                                ProofRefusalCause::InvalidInput,
                                DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                                CatalogCode::new("invalid_runtime_input", "invalid-value"),
                            )
                        }
                    })
                };
                let (run_subject, certificate_subject) =
                    match (subject(form.max_candidates), subject(form.max_candidates)) {
                        (Ok(run), Ok(certificate)) => (run, certificate),
                        (Err(refusal), _) | (_, Err(refusal)) => return refused(&refusal),
                    };
                let system = match ModelSystem::new(simulation(
                    package,
                    &binding,
                    &limits,
                    form.max_candidates,
                )) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let clause_letters;
                let letters: &dyn ModelLetters = match node {
                    Some(node) => {
                        clause_letters = match system.clause_letters(node) {
                            Ok(letters) => letters,
                            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                        };
                        &clause_letters
                    }
                    None => &NoLetters,
                };
                let item = || match node {
                    Some(node) => ModelCheckItem::Clause {
                        clause: node.clause(),
                        letters,
                    },
                    None if form.item_kind == ItemKind::RaceFreedom => ModelCheckItem::RaceFreedom,
                    None => ModelCheckItem::DeadlockFreedom,
                };
                let trace = ProtocolTraceSubject {
                    model: ModelTraceSubject {
                        timed: None,
                        initial: binding.initial.clone(),
                        universes: binding.universes.clone(),
                        expansion: ExpansionLimits {
                            max_candidates: form.max_candidates,
                        },
                    },
                    protocol: protocol.protocol.clone(),
                    over: over.clone(),
                    roles: protocol.roles.clone(),
                    capacities: protocol.capacities.clone(),
                    memory: run_subject.memory.clone(),
                    max_store_buffer: form.max_store_buffer,
                    max_messages: form.max_messages,
                    max_live_instances: None,
                };
                let inputs = ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &binding,
                    limits: &limits,
                    provision,
                    max_candidates: form.max_candidates,
                    timed: None,
                };
                let replay = |payload: &TemporalCounterexample<ProtocolTransition>| {
                    replay_protocol(&inputs, &trace, node, &claim, payload, letters)
                };
                settle_protocol_item(
                    ProtocolItem {
                        request: ProtocolCheckRequest {
                            temporal_clock_selections: &[],
                            subject: run_subject,
                            item: item(),
                            max_depth: if node.is_some() { form.max_depth } else { None },
                            max_store_buffer: form.max_store_buffer,
                            max_messages: form.max_messages,
                            ignore_memory_constraints: false,
                            max_live_instances: None,
                            limits: form.limits,
                        },
                        certificate: ProtocolCertificateRequest {
                            temporal_clock_selections: &[],
                            subject: certificate_subject,
                            item: item(),
                            max_store_buffer: form.max_store_buffer,
                            max_messages: form.max_messages,
                            max_live_instances: None,
                        },
                        replay: Box::new(replay),
                    },
                    cancel,
                )
            },
        )
        .claimed(claim_for(node, &claim)),
    )
}

fn claim_for(node: Option<&TemporalClauseNode>, claim: &OccurrenceKey) -> OccurrenceKey {
    node.map_or_else(
        || claim.clone(),
        |node| OccurrenceKey::new(node.identity(), node.claim().clone()),
    )
}

/// FR-217's replay of `payload` over the same source and protocol subject,
/// in the envelope that names the authored clause, or the derived item by its
/// protocol anchor.
fn replay_protocol(
    inputs: &ReplayInputs<'_, '_>,
    trace: &ProtocolTraceSubject,
    node: Option<&TemporalClauseNode>,
    claim: &OccurrenceKey,
    payload: &TemporalCounterexample<ProtocolTransition>,
    letters: &dyn ModelLetters,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
    let shape = match node {
        Some(node) => {
            let name = clause_name(node).map_err(|_| fault("clause-name"))?;
            ReplayShape {
                target: RequestTargetWire::state_clause(name.clone()),
                members: ReplayTargetMembers::state_clause(name),
                node: node.identity(),
                occurrence: OccurrenceKey::new(node.identity(), node.claim().clone()),
                transcript: CLAUSE_TRANSCRIPT,
            }
        }
        None => {
            let target = derived_target().ok_or_else(|| fault("derived-target"))?;
            ReplayShape {
                target: RequestTargetWire::function(target.clone(), claim.clone()),
                members: ReplayTargetMembers::function(target),
                node: claim.node().clone(),
                occurrence: claim.clone(),
                transcript: if matches!(payload.kind, fsl_replay::CounterexampleKind::Race(_)) {
                    RACE_TRANSCRIPT
                } else {
                    DEADLOCK_TRANSCRIPT
                },
            }
        }
    };
    let (wire, envelope) = replay_wire(inputs, shape, payload)?;
    replay_protocol_trace_with_temporal_selections(
        wire,
        trace,
        &envelope,
        letters,
        &[],
        ReplayLimits::default(),
    )
}
