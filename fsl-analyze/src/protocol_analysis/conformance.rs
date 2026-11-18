// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-991: replay evidence carried by a global-conformance result.

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::model_subject::ProtocolSubject;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::protocol::{
    live_instance_bound, ProtocolRefusal, ProtocolSystem, ProtocolTransition, Ra, Sc, StepKind,
    SubjectMemory, Tso,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_replay::TemporalCounterexample;
use fsl_replay::{
    CounterexampleKind, FamilyPayload, ProtocolConformanceOutput, ReplayLimits, TerminalRecord,
};

/// The independently settled global-conformance record and its FR-990 output.
///
/// The terminal record remains the authority for item settlement. The output
/// carries the obligation-level findings and closure evidence alongside it.
pub(crate) struct ConformanceReport {
    pub(crate) terminal: TerminalRecord,
    pub(crate) output: ProtocolConformanceOutput,
    pub(crate) counterexample: Option<serde_json::Value>,
}

fn recording_payload(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut document: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("recording document is not valid JSON: {error}"))?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "recording document must be an object".to_owned())?;
    object.remove("identity");
    serde_json::to_vec(&document).map_err(|error| format!("recording document: {error}"))
}

/// Build the recording-mode `global-conformance` result from the checked
/// protocol and its provisioned FR-737 bytes.
fn recording_result(
    index: fsl_foundation::RequestIndex,
    package: &fsl_package::CheckedPackage,
    subject: Option<&fsl_eval::model_subject::ProtocolSubject<'_>>,
    recording_identity: Option<&fsl_semantics::model::observation::DocumentRef>,
    recording: &[u8],
    population: Option<serde_json::Value>,
    cancel: &Cancel,
) -> crate::analyze::EngineReport {
    use base64::Engine as _;
    use fsl_replay::protocol_analysis::ProtocolAnalysisSettlement;
    use fsl_replay::{
        Certification, InconclusiveCause, ProofBasis, ResultLabel, SettlementBasis, TerminalRecord,
        TerminalValue,
    };

    // The command-level provision wrapper owns the document identity. The
    // FR-737 recording payload passed to its strict assessor has only the
    // recording schema members.
    let recording = match recording_payload(recording) {
        Ok(recording) => recording,
        Err(detail) => {
            return crate::analyze::EngineReport::Refused(
                crate::analyze::Refusal::DeclinedWithDetail(
                    fsl_replay::ProofRefusalCause::InvalidInput,
                    fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    [("detail".to_owned(), serde_json::json!(detail))]
                        .into_iter()
                        .collect(),
                ),
            );
        }
    };
    let population = match population {
        Some(population) => match serde_json::from_value::<
            Vec<crate::analyze::document::ActivatedRolePopulation>,
        >(population)
        {
            Ok(population) => Some(population),
            Err(error) => {
                return crate::analyze::EngineReport::Refused(
                    crate::analyze::Refusal::DeclinedWithDetail(
                        fsl_replay::ProofRefusalCause::InvalidInput,
                        fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        CatalogCode::new("invalid_runtime_input", "invalid-value"),
                        [("detail".to_owned(), serde_json::json!(error.to_string()))]
                            .into_iter()
                            .collect(),
                    ),
                );
            }
        },
        None => None,
    };
    let assessment = match super::recording::assess_recording_with_population(
        package,
        &recording,
        population.as_deref(),
    ) {
        Ok(assessment) => assessment,
        Err(refusal) => {
            let detail = [("detail".to_owned(), serde_json::json!(refusal.to_string()))]
                .into_iter()
                .chain(
                    refusal
                        .position()
                        .map(|position| ("position".to_owned(), serde_json::json!(position))),
                )
                .collect();
            return crate::analyze::EngineReport::Refused(
                crate::analyze::Refusal::DeclinedWithDetail(
                    fsl_replay::ProofRefusalCause::InvalidInput,
                    fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                    refusal.code(),
                    detail,
                ),
            );
        }
    };
    let Some(recording_identity) = recording_identity else {
        return crate::analyze::EngineReport::Refused(crate::analyze::Refusal::DeclinedWithDetail(
            fsl_replay::ProofRefusalCause::InvalidInput,
            fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            [(
                "detail".to_owned(),
                serde_json::json!("recording has no provisioned identity"),
            )]
            .into_iter()
            .collect(),
        ));
    };
    let replayed = if assessment.global.label == "refuted" {
        match (subject, assessment.trace_position) {
            (Some(subject), Some(trace_position)) => match replay_counterexample(
                subject,
                &ConformanceCounterexample::Recording {
                    recording: recording.to_vec(),
                    trace_position,
                },
                ReplayLimits::default(),
                cancel,
            ) {
                Ok(ConformanceReplay::Reproduced) => true,
                Ok(ConformanceReplay::ReplayParity { .. }) => {
                    return crate::analyze::EngineReport::Refused(crate::analyze::Refusal::Failed(
                        filament_semantic_value::catalog::CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        ),
                    ));
                }
                Err(refusal) => {
                    return crate::analyze::EngineReport::Refused(
                        crate::analyze::Refusal::Declined(
                            fsl_replay::ProofRefusalCause::InvalidInput,
                            fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                            refusal.code(),
                        ),
                    );
                }
            },
            _ => false,
        }
    } else {
        false
    };
    let mut output = recording_output(assessment.clone(), recording_identity);
    output.replay = replayed.then_some(fsl_replay::ConformanceReplayOutput { reproduced: true });
    let terminal = match assessment.global.label.as_str() {
        "proved" => TerminalRecord::of_protocol_analysis(
            index,
            fsl_foundation::BackendId::new(super::manifest::BACKEND_ID),
            &ProtocolAnalysisSettlement {
                value: TerminalValue::Proved {
                    basis: ProofBasis::Exhaustive,
                    certification: Certification::Uncertified,
                },
                label: ResultLabel::Proved,
                basis: SettlementBasis::ClosedScope,
                execution: fsl_replay::Execution::Completed,
                truth: fsl_replay::Truth::Satisfied,
            },
        ),
        "refuted" => TerminalRecord::of_protocol_analysis(
            index,
            fsl_foundation::BackendId::new(super::manifest::BACKEND_ID),
            &ProtocolAnalysisSettlement {
                value: TerminalValue::Refuted,
                label: ResultLabel::Refuted,
                basis: SettlementBasis::DecisiveCounterexample,
                execution: fsl_replay::Execution::Completed,
                truth: fsl_replay::Truth::Violated,
            },
        ),
        _ => TerminalRecord::inconclusive(
            index,
            fsl_foundation::BackendId::new(super::manifest::BACKEND_ID),
            InconclusiveCause::OpenHistory,
        ),
    };
    crate::analyze::EngineReport::GlobalConformance(ConformanceReport {
        terminal,
        output,
        counterexample: (assessment.global.label == "refuted").then(|| {
            serde_json::json!({
                "kind": "recording",
                "recording": base64::engine::general_purpose::STANDARD.encode(recording),
                "trace_position": assessment.trace_position,
            })
        }),
    })
}

/// The recording arm of one global-conformance request.
pub(crate) fn global_recording_item<'a>(
    index: fsl_foundation::RequestIndex,
    package: &'a fsl_package::CheckedPackage,
    subject: Option<fsl_eval::model_subject::ProtocolSubject<'a>>,
    recording_identity: Option<fsl_semantics::model::observation::DocumentRef>,
    recording: Option<&'a [u8]>,
    population: Option<serde_json::Value>,
) -> crate::analyze::AnalyzeItem<'a> {
    crate::analyze::AnalyzeItem::new(
        index,
        fsl_semantics::check::Capability::GlobalConformance,
        Some(fsl_semantics::check::DeclarationFamily::Protocol),
        fsl_route::Mode::Bounded,
        super::manifest::BACKEND_ID,
        move |cancel| match recording {
            Some(recording) => recording_result(
                index,
                package,
                subject.as_ref(),
                recording_identity.as_ref(),
                recording,
                population,
                cancel,
            ),
            None => crate::analyze::EngineReport::Refused(crate::analyze::Refusal::Declined(
                fsl_replay::ProofRefusalCause::InvalidInput,
                fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                filament_semantic_value::catalog::CatalogCode::new(
                    "invalid_runtime_input",
                    "missing-member",
                ),
            )),
        },
    )
}

/// Decline a global request before analysis when a static role has no
/// selected model implementation.
pub(crate) fn missing_role_item(
    index: fsl_foundation::RequestIndex,
    role: &str,
) -> crate::analyze::AnalyzeItem<'static> {
    let role = role.to_owned();
    crate::analyze::AnalyzeItem::new(
        index,
        fsl_semantics::check::Capability::GlobalConformance,
        Some(fsl_semantics::check::DeclarationFamily::Protocol),
        fsl_route::Mode::Bounded,
        super::manifest::BACKEND_ID,
        move |_| {
            let detail = [("role".to_owned(), serde_json::json!(role))]
                .into_iter()
                .collect();
            crate::analyze::EngineReport::Refused(crate::analyze::Refusal::DeclinedWithDetail(
                fsl_replay::ProofRefusalCause::IncompleteInput,
                fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::MissingDeclaration),
                filament_semantic_value::catalog::CatalogCode::new(
                    "missing_declaration",
                    "missing-name",
                ),
                detail,
            ))
        },
    )
}

/// The every-behavior arm of a global-conformance request.
pub(crate) fn global_behavior_item<'a>(
    index: fsl_foundation::RequestIndex,
    package: &'a fsl_package::CheckedPackage,
    binding: crate::analyze::SubjectBinding<'a>,
    limits: crate::analyze::AnalyzeLimits,
    protocol: &'a fsl_semantics::check::CheckedProtocol,
    form: &crate::analyze::TemporalItem,
) -> crate::analyze::AnalyzeItem<'a> {
    let form = form.clone();
    crate::analyze::AnalyzeItem::new(
        index,
        fsl_semantics::check::Capability::GlobalConformance,
        Some(fsl_semantics::check::DeclarationFamily::Protocol),
        fsl_route::Mode::Bounded,
        super::manifest::BACKEND_ID,
        move |cancel| {
            let subject_builder = super::finite_replay::ReplaySubject {
                package,
                binding: &binding,
                limits: &limits,
                protocol,
            };
            let instances = match subject_builder.instances() {
                Ok(instances) => instances,
                Err(_) => {
                    return crate::analyze::EngineReport::Refused(crate::analyze::Refusal::Failed(
                        filament_semantic_value::catalog::CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        ),
                    ));
                }
            };
            let Some(over) = instances.into_iter().next() else {
                return crate::analyze::EngineReport::Refused(crate::analyze::Refusal::Failed(
                    filament_semantic_value::catalog::CatalogCode::new(
                        "runtime_invariant",
                        "established-invariant-broken",
                    ),
                ));
            };
            let subject = match subject_builder.subject(over.clone()) {
                Ok(subject) => subject,
                // FSpec FR-785: a symbolic capacity with no binding.
                Err(super::finite_replay::ReplayFault::UnboundCapacity) => {
                    return crate::analyze::EngineReport::Refused(
                        crate::analyze::Refusal::RequiresBound,
                    );
                }
                Err(_) => return internal_fault(),
            };
            let replay_subject = match subject_builder.subject(over.clone()) {
                Ok(subject) => subject,
                Err(_) => return internal_fault(),
            };
            let certificate_subject = match subject_builder.subject(over) {
                Ok(subject) => subject,
                Err(_) => return internal_fault(),
            };
            let scan_subject = match subject_builder
                .instances()
                .ok()
                .and_then(|mut instances| (!instances.is_empty()).then(|| instances.remove(0)))
                .and_then(|over| subject_builder.subject(over).ok())
            {
                Some(subject) => subject,
                None => return internal_fault(),
            };
            every_behavior_result(
                index,
                protocol,
                &form,
                subject,
                replay_subject,
                certificate_subject,
                scan_subject,
                cancel,
            )
        },
    )
}

fn behavior_report(
    index: fsl_foundation::RequestIndex,
    protocol: &fsl_semantics::check::CheckedProtocol,
    record: fsl_replay::ModelCheckRecord<ProtocolTransition>,
    reported_counterexample: Option<serde_json::Value>,
) -> crate::analyze::EngineReport {
    use fsl_replay::{
        ConformanceClosureOutput, ConformanceGlobalOutput, ConformanceObligationOutput,
        ProtocolConformanceOutput, TerminalRecord,
    };

    let label = if matches!(&record.value, fsl_replay::TerminalValue::Incomplete(_)) {
        "incomplete"
    } else {
        match record.label {
            fsl_replay::ResultLabel::Proved => "proved",
            fsl_replay::ResultLabel::Refuted => "refuted",
            fsl_replay::ResultLabel::Inconclusive => "inconclusive",
            fsl_replay::ResultLabel::Unsupported => "unsupported",
            fsl_replay::ResultLabel::Failed => "failed",
        }
    };
    let closed = label == "proved";
    let refuted = label == "refuted";
    let incomplete = label == "incomplete";
    let verdict = if closed {
        "satisfied"
    } else if refuted {
        "violated"
    } else {
        "pending"
    };
    let cause = refuted.then(|| "counterexample".to_owned());
    let witness = refuted.then_some(serde_json::json!(0));
    let mut obligations: Vec<_> = protocol
        .edges
        .iter()
        .map(|edge| ConformanceObligationOutput {
            obligation: serde_json::json!({
                "kind": "causal-order",
                "from": format!("{:?}", edge.from.node),
                "to": format!("{:?}", edge.to.node),
            }),
            verdict: verdict.to_owned(),
            cause: cause.clone(),
            basis: "by-construction".to_owned(),
            witness: witness.clone(),
        })
        .collect();
    obligations.extend(protocol.controls.controls.iter().filter_map(|control| {
        matches!(
            control.kind,
            fsl_forms::ProtocolNodeKind::Send
                | fsl_forms::ProtocolNodeKind::Receive
                | fsl_forms::ProtocolNodeKind::Attempt
                | fsl_forms::ProtocolNodeKind::Effect
                | fsl_forms::ProtocolNodeKind::Event
        )
        .then(|| ConformanceObligationOutput {
            obligation: serde_json::json!({
                "kind": "enabledness",
                "node": format!("{}::{}", protocol.name, control.path.join("::")),
            }),
            verdict: verdict.to_owned(),
            cause: cause.clone(),
            basis: "by-construction".to_owned(),
            witness: witness.clone(),
        })
    }));
    obligations.extend(
        protocol
            .channels
            .iter()
            .map(|channel| ConformanceObligationOutput {
                obligation: serde_json::json!({"kind": "channel-premise", "channel": channel.name}),
                verdict: "satisfied".to_owned(),
                cause: None,
                basis: "by-construction".to_owned(),
                witness: None,
            }),
    );
    obligations.extend(protocol.controls.controls.iter().filter_map(|control| {
        matches!(
            control.kind,
            fsl_forms::ProtocolNodeKind::Choice
                | fsl_forms::ProtocolNodeKind::Parallel
                | fsl_forms::ProtocolNodeKind::Repeat
                | fsl_forms::ProtocolNodeKind::Await
                | fsl_forms::ProtocolNodeKind::Check
        )
        .then(|| ConformanceObligationOutput {
            obligation: serde_json::json!({
                "kind": "control",
                "node": format!("{}::{}", protocol.name, control.path.join("::")),
            }),
            verdict: verdict.to_owned(),
            cause: cause.clone(),
            basis: "explored".to_owned(),
            witness: witness.clone(),
        })
    }));
    obligations.extend(
        protocol
            .awaits
            .iter()
            .map(|awaited| ConformanceObligationOutput {
                obligation: serde_json::json!({
                    "kind": "window",
                    "node": format!("{:?}", awaited.node),
                    "clock": awaited.clock,
                }),
                verdict: verdict.to_owned(),
                cause: cause.clone(),
                basis: "explored".to_owned(),
                witness: witness.clone(),
            }),
    );
    obligations.extend(
        protocol
            .obligations
            .compensations
            .iter()
            .map(|compensation| ConformanceObligationOutput {
                obligation: serde_json::json!({
                    "kind": "compensation",
                    "template": compensation.name,
                }),
                verdict: verdict.to_owned(),
                cause: cause.clone(),
                basis: "explored".to_owned(),
                witness: witness.clone(),
            }),
    );
    obligations.extend(protocol.requirements.iter().map(|requirement| {
        ConformanceObligationOutput {
            obligation: serde_json::json!({
                "kind": "requirement",
                "temporal": requirement.temporal,
            }),
            verdict: verdict.to_owned(),
            cause: cause.clone(),
            basis: "explored".to_owned(),
            witness: witness.clone(),
        }
    }));
    obligations.extend(
        protocol
            .roles
            .iter()
            .map(|role| ConformanceObligationOutput {
                obligation: serde_json::json!({"kind": "protocol-instance", "role": role.name}),
                verdict: verdict.to_owned(),
                cause: cause.clone(),
                basis: "explored".to_owned(),
                witness: witness.clone(),
            }),
    );
    obligations.push(ConformanceObligationOutput {
        obligation: serde_json::json!({"kind": "closure", "node": protocol.name}),
        verdict: verdict.to_owned(),
        cause,
        basis: "explored".to_owned(),
        witness,
    });
    let counterexample = refuted.then(|| {
        reported_counterexample.unwrap_or_else(|| serde_json::json!({"kind": "deadlock"}))
    });
    let terminal = record
        .to_terminal(
            index,
            fsl_foundation::BackendId::new(super::manifest::BACKEND_ID),
        )
        .unwrap_or_else(|_| {
            TerminalRecord::failed(
                index,
                fsl_foundation::BackendId::new(super::manifest::BACKEND_ID),
            )
        });
    crate::analyze::EngineReport::GlobalConformance(ConformanceReport {
        terminal,
        output: ProtocolConformanceOutput {
            obligations,
            closure: ConformanceClosureOutput {
                state: if closed {
                    "closed"
                } else if incomplete {
                    "incomplete"
                } else {
                    "open"
                }
                .to_owned(),
                premises: std::iter::once(serde_json::json!({
                    "kind": "execution",
                    "node": protocol
                        .finish
                        .as_ref()
                        .map(|(_, finish)| format!("{}::{finish}", protocol.name))
                        .unwrap_or_else(|| protocol.name.clone())
                }))
                .chain(
                    protocol
                        .controls
                        .controls
                        .iter()
                        .filter(|control| {
                            matches!(
                                control.kind,
                                fsl_forms::ProtocolNodeKind::Case
                                    | fsl_forms::ProtocolNodeKind::Branch
                            )
                        })
                        .map(|control| {
                            serde_json::json!({
                                "kind": "branch",
                                "node": format!("{}::{}", protocol.name, control.path.join("::")),
                            })
                        }),
                )
                .collect(),
                producer: "engine".to_owned(),
            },
            global: ConformanceGlobalOutput {
                label: label.to_owned(),
                basis: if closed {
                    "closed-scope"
                } else if refuted {
                    "decisive-counterexample"
                } else if incomplete {
                    "unavailable"
                } else {
                    "unsettled"
                }
                .to_owned(),
                complete: closed,
                cause: if incomplete {
                    Some("limit-reached".to_owned())
                } else if !closed && !refuted {
                    Some("open-history".to_owned())
                } else {
                    None
                },
            },
            projections: Some(
                protocol
                    .roles
                    .iter()
                    .map(|role| fsl_replay::ConformanceProjectionOutput {
                        role: role.name.clone(),
                        label: label.to_owned(),
                        counterexample: None,
                    })
                    .collect(),
            ),
            recording: None,
            replay: refuted.then_some(fsl_replay::ConformanceReplayOutput { reproduced: true }),
        },
        counterexample,
    })
}

fn internal_fault() -> crate::analyze::EngineReport {
    crate::analyze::EngineReport::Refused(crate::analyze::Refusal::Failed(
        filament_semantic_value::catalog::CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken",
        ),
    ))
}

fn pending_finish_counterexample(
    subject: &ProtocolSubject<'_>,
    limits: fsl_replay::ModelCheckLimits,
) -> Option<TemporalCounterexample<ProtocolTransition>> {
    let model = ModelSystem::new(subject.model.simulation.clone()).ok()?;
    match SubjectMemory::of(&subject.memory, None, None) {
        SubjectMemory::Sc => {
            let system = ProtocolSystem::new(model, subject, Sc).ok()?;
            pending_finish_in::<Sc>(&system, limits)
        }
        SubjectMemory::Tso(memory) => {
            let system = ProtocolSystem::new(model, subject, memory).ok()?;
            pending_finish_in::<Tso>(&system, limits)
        }
        SubjectMemory::Ra(memory) => {
            let system = ProtocolSystem::new(model, subject, memory).ok()?;
            pending_finish_in::<Ra>(&system, limits)
        }
    }
}

fn pending_finish_in<M: fsl_eval::simulation::protocol::MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    limits: fsl_replay::ModelCheckLimits,
) -> Option<TemporalCounterexample<ProtocolTransition>> {
    use std::collections::{BTreeSet, VecDeque};

    struct Pending<M: fsl_eval::simulation::protocol::MemoryModel> {
        initial_state: usize,
        state: fsl_eval::simulation::protocol::ProtocolState<M>,
        prefix: Vec<fsl_replay::ModelStep<ProtocolTransition>>,
    }

    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    for (initial_state, state) in system.initial().into_iter().enumerate() {
        let (bytes, _) = system.state_key(&state).ok()?;
        if seen.insert((initial_state, bytes)) {
            queue.push_back(Pending {
                initial_state,
                state,
                prefix: Vec::new(),
            });
        }
    }
    let mut expanded = 0_u64;
    let mut transitions = 0_u64;
    while let Some(Pending {
        initial_state,
        state,
        prefix,
    }) = queue.pop_front()
    {
        if expanded >= u64::try_from(limits.limits.max_states).unwrap_or(u64::MAX) {
            return None;
        }
        expanded += 1;
        let expansion = system.successors(&state).ok()?;
        if system.has_pending_exactly_once_delivery(&state)
            && expansion
                .successors
                .iter()
                .any(|(transition, _)| transition.kind() == StepKind::Finish)
        {
            return Some(TemporalCounterexample {
                initial_state,
                prefix,
                over_binding: None,
                fairness: Vec::new(),
                kind: CounterexampleKind::Deadlock,
                lasso: None,
                blocked: Vec::new(),
                refinement: None,
                final_delay: None,
            });
        }
        for (transition, next) in expansion.successors {
            if transitions >= u64::try_from(limits.limits.max_transitions).unwrap_or(u64::MAX) {
                return None;
            }
            transitions += 1;
            let (bytes, key) = system.state_key(&next).ok()?;
            if seen.insert((initial_state, bytes)) {
                let mut next_prefix = prefix.clone();
                next_prefix.push(fsl_replay::ModelStep {
                    transition,
                    post_state: key,
                    delay: None,
                });
                queue.push_back(Pending {
                    initial_state,
                    state: next,
                    prefix: next_prefix,
                });
            }
        }
    }
    None
}

fn every_behavior_result(
    index: fsl_foundation::RequestIndex,
    protocol: &fsl_semantics::check::CheckedProtocol,
    form: &crate::analyze::TemporalItem,
    subject: fsl_eval::model_subject::ProtocolSubject<'_>,
    replay_subject: fsl_eval::model_subject::ProtocolSubject<'_>,
    certificate_subject: fsl_eval::model_subject::ProtocolSubject<'_>,
    scan_subject: fsl_eval::model_subject::ProtocolSubject<'_>,
    cancel: &Cancel,
) -> crate::analyze::EngineReport {
    let wire_simulation = subject.model.simulation.clone();
    let wire_protocol = subject.protocol.clone();
    let wire_over = subject.over.clone();
    let wire_roles: Vec<_> = subject
        .roles
        .iter()
        .map(|binding| (binding.role.clone(), binding.model.clone()))
        .collect();
    let wire_capacities: Vec<_> = subject
        .capacities
        .iter()
        .map(|binding| (binding.name.clone(), binding.value))
        .collect();
    let item = fsl_replay::ModelCheckItem::DeadlockFreedom;
    let request = crate::model_check::ProtocolCheckRequest {
        subject,
        temporal_clock_selections: &[],
        item,
        max_depth: form.max_depth,
        max_store_buffer: None,
        max_messages: None,
        ignore_memory_constraints: false,
        max_live_instances: None,
        limits: form.limits,
    };
    let certificate = fsl_replay::ProtocolCertificateRequest {
        subject: certificate_subject,
        temporal_clock_selections: &[],
        item,
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    let outcome = match crate::model_check::check_protocol(request, cancel) {
        Ok(outcome) => outcome,
        Err(_) => return internal_fault(),
    };
    let pending_delivery = pending_finish_counterexample(&scan_subject, form.limits);
    let outcome = match pending_delivery {
        Some(counterexample) => fsl_replay::ModelCheckOutcome::Violated {
            counterexample: Box::new(counterexample),
            stats: Default::default(),
        },
        None => outcome,
    };
    let replay_limits = ReplayLimits::default();
    let replay = |counterexample: &TemporalCounterexample<ProtocolTransition>| {
        let payload = ConformanceCounterexample::Protocol(counterexample.clone());
        match replay_counterexample(&replay_subject, &payload, replay_limits, cancel) {
            Ok(ConformanceReplay::Reproduced) => Ok(fsl_replay::ModelTraceReplayResult {
                settlement: fsl_replay::ModelTraceSettlement::Reproduced {
                    value: fsl_replay::ModelTraceValue::Truth(false),
                    trace_position: None,
                },
                hints: Vec::new(),
                terminals: Default::default(),
            }),
            _ => {
                let refusal = counterexample
                    .prefix
                    .last()
                    .map(|step| fsl_replay::ModelTraceRefusal::ProtocolNotEnabled {
                        step: counterexample.prefix.len().saturating_sub(1),
                        transition: Box::new(step.transition.clone()),
                    })
                    .unwrap_or(fsl_replay::ModelTraceRefusal::InitialState {
                        index: counterexample.initial_state,
                        count: counterexample.initial_state,
                    });
                Err(fsl_replay::ReplayRefusal::ModelTrace(Box::new(refusal)))
            }
        }
    };
    let record =
        fsl_replay::settle_protocol_check(&outcome, &form.limits, &certificate, cancel, replay);
    let counterexample = record.counterexample.as_ref().and_then(|counterexample| {
        let over = fsl_eval::simulation::protocol::over_reference(
            wire_simulation,
            &wire_protocol,
            &wire_over,
        )?;
        let subject = fsl_replay::ProtocolWire {
            protocol: wire_protocol,
            over,
            roles: wire_roles,
            capacities: wire_capacities,
            max_live_instances: None,
        };
        fsl_replay::protocol_temporal_wire(counterexample, &subject, None).ok()
    });
    behavior_report(index, protocol, record, counterexample)
}

/// A global-conformance counterexample that can be replayed from a checked subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceCounterexample {
    /// A protocol-system counterexample over every behaviour.
    Protocol(TemporalCounterexample<ProtocolTransition>),
    /// The recording bytes and index of the first violating observation.
    Recording {
        /// The FSpec FR-737 recording document bytes.
        recording: Vec<u8>,
        /// Index of the first violating observation in causal order.
        trace_position: usize,
    },
}

/// The result of replaying a global-conformance counterexample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceReplay {
    /// Replay reproduced the refutation.
    Reproduced,
    /// Replay was valid but did not preserve the claimed refutation.
    ReplayParity {
        /// What differed between the result and replay.
        detail: String,
    },
}

/// Why a global-conformance counterexample could not be replayed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code:?}/{cause}: {detail}")]
pub struct ConformanceReplayRefusal {
    code: CatalogCode,
    cause: String,
    position: Option<usize>,
    detail: String,
}

impl ConformanceReplayRefusal {
    fn invalid(detail: impl Into<String>, position: Option<usize>) -> Self {
        Self {
            code: CatalogCode::new("invalid_runtime_input", "invalid-value"),
            cause: "invalid-value".to_owned(),
            position,
            detail: detail.into(),
        }
    }

    fn recording(refusal: super::recording::RecordingRefusal) -> Self {
        Self {
            code: refusal.code(),
            cause: refusal.cause().to_owned(),
            position: refusal.position(),
            detail: refusal.to_string(),
        }
    }

    /// The FSpec catalog code for the refusal.
    pub fn code(&self) -> CatalogCode {
        self.code
    }

    /// The FSpec catalog cause.
    pub fn cause(&self) -> &str {
        &self.cause
    }

    /// The transition or observation index named by the refusal, if any.
    pub fn position(&self) -> Option<usize> {
        self.position
    }
}

/// Replay `counterexample` from the already-built protocol subject.
///
/// The replay reads no source text and does not recompile the subject.
///
/// # Errors
///
/// [`ConformanceReplayRefusal`] when the counterexample is malformed or the
/// subject cannot build its protocol system.
pub fn replay_counterexample(
    subject: &ProtocolSubject<'_>,
    counterexample: &ConformanceCounterexample,
    limits: ReplayLimits,
    cancel: &Cancel,
) -> Result<ConformanceReplay, ConformanceReplayRefusal> {
    match counterexample {
        ConformanceCounterexample::Protocol(counterexample) => {
            let bytes = counterexample.measured_bytes();
            fsl_replay::BoundExceeded::check(bytes, limits)
                .map_err(|error| ConformanceReplayRefusal::invalid(error.to_string(), None))?;
            match SubjectMemory::of(&subject.memory, None, None) {
                SubjectMemory::Sc => {
                    let model =
                        ModelSystem::new(subject.model.simulation.clone()).map_err(|error| {
                            ConformanceReplayRefusal::invalid(
                                format!("model admission: {error:?}"),
                                None,
                            )
                        })?;
                    let system = ProtocolSystem::new(model, subject, Sc)
                        .map_err(protocol_refusal)?
                        .with_max_live_instances(live_instance_bound(None));
                    if replay_pending_finish(&system, counterexample) {
                        Ok(ConformanceReplay::Reproduced)
                    } else {
                        replay_protocol(&system, counterexample, cancel)
                    }
                }
                SubjectMemory::Tso(memory) => {
                    let model =
                        ModelSystem::new(subject.model.simulation.clone()).map_err(|error| {
                            ConformanceReplayRefusal::invalid(
                                format!("model admission: {error:?}"),
                                None,
                            )
                        })?;
                    let system = ProtocolSystem::new(model, subject, memory)
                        .map_err(protocol_refusal)?
                        .with_max_live_instances(live_instance_bound(None));
                    if replay_pending_finish(&system, counterexample) {
                        Ok(ConformanceReplay::Reproduced)
                    } else {
                        replay_protocol(&system, counterexample, cancel)
                    }
                }
                SubjectMemory::Ra(memory) => {
                    let model =
                        ModelSystem::new(subject.model.simulation.clone()).map_err(|error| {
                            ConformanceReplayRefusal::invalid(
                                format!("model admission: {error:?}"),
                                None,
                            )
                        })?;
                    let system = ProtocolSystem::new(model, subject, memory)
                        .map_err(protocol_refusal)?
                        .with_max_live_instances(live_instance_bound(None));
                    if replay_pending_finish(&system, counterexample) {
                        Ok(ConformanceReplay::Reproduced)
                    } else {
                        replay_protocol(&system, counterexample, cancel)
                    }
                }
            }
        }
        ConformanceCounterexample::Recording {
            recording,
            trace_position,
        } => {
            fsl_replay::BoundExceeded::check(recording.len(), limits)
                .map_err(|error| ConformanceReplayRefusal::invalid(error.to_string(), None))?;
            let assessment =
                super::recording::assess_recording(subject.model.simulation.package, recording)
                    .map_err(ConformanceReplayRefusal::recording)?;
            if assessment.global.label == "refuted"
                && assessment.trace_position == Some(*trace_position)
            {
                Ok(ConformanceReplay::Reproduced)
            } else {
                Ok(ConformanceReplay::ReplayParity {
                    detail: format!(
                        "recording assessment was {} at {:?}, expected refuted at {}",
                        assessment.global.label, assessment.trace_position, trace_position
                    ),
                })
            }
        }
    }
}

fn replay_pending_finish<M: fsl_eval::simulation::protocol::MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    counterexample: &TemporalCounterexample<ProtocolTransition>,
) -> bool {
    if !matches!(counterexample.kind, CounterexampleKind::Deadlock) {
        return false;
    }
    let Some(mut state) = system
        .initial()
        .into_iter()
        .nth(counterexample.initial_state)
    else {
        return false;
    };
    for step in &counterexample.prefix {
        let Ok(expansion) = system.successors(&state) else {
            return false;
        };
        let Some((_, next)) = expansion.successors.into_iter().find(|(transition, next)| {
            if transition != &step.transition {
                return false;
            }
            system
                .state_key(next)
                .is_ok_and(|(_, key)| key == step.post_state)
        }) else {
            return false;
        };
        state = next;
    }
    let Ok(expansion) = system.successors(&state) else {
        return false;
    };
    system.has_pending_exactly_once_delivery(&state)
        && expansion
            .successors
            .iter()
            .any(|(transition, _)| transition.kind() == StepKind::Finish)
}

/// Convert a recording assessment to the shared FR-990 item representation.
pub(crate) fn recording_output(
    assessment: super::recording::RecordingAssessment,
    recording_identity: &fsl_semantics::model::observation::DocumentRef,
) -> ProtocolConformanceOutput {
    use fsl_replay::{
        ConformanceClosureOutput, ConformanceGlobalOutput, ConformanceObligationOutput,
    };

    ProtocolConformanceOutput {
        obligations: assessment
            .obligations
            .into_iter()
            .map(|obligation| ConformanceObligationOutput {
                obligation: serde_json::to_value(obligation.obligation)
                    .expect("recording obligation IDs serialize"),
                verdict: match obligation.verdict {
                    super::recording::RecordingVerdict::Satisfied => "satisfied",
                    super::recording::RecordingVerdict::Violated => "violated",
                    super::recording::RecordingVerdict::Pending => "pending",
                }
                .to_owned(),
                cause: obligation
                    .cause
                    .map(|cause| serde_json::to_value(cause).expect("recording causes serialize"))
                    .and_then(|cause| cause.as_str().map(str::to_owned)),
                basis: "recording".to_owned(),
                witness: (!obligation.witness.is_empty())
                    .then(|| serde_json::json!(obligation.witness)),
            })
            .collect(),
        closure: ConformanceClosureOutput {
            state: assessment.closure.state,
            premises: assessment
                .closure
                .premises
                .into_iter()
                .map(|premise| serde_json::to_value(premise).expect("premises serialize"))
                .collect(),
            producer: assessment.closure.producer.to_owned(),
        },
        global: ConformanceGlobalOutput {
            label: assessment.global.label,
            basis: assessment.global.basis,
            complete: assessment.global.complete,
            cause: assessment.global.cause,
        },
        projections: None,
        recording: Some(fsl_replay::ConformanceRecordingIdentity {
            authority: recording_identity.authority.clone(),
            identity: recording_identity.identity.clone(),
        }),
        replay: None,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[trace("TC-2805", "FR-990-AC-1")]
    #[test]
    fn recording_assessment_maps_to_the_shared_conformance_output() {
        let output = recording_output(
            super::super::recording::RecordingAssessment {
                obligations: vec![super::super::recording::RecordingObligation {
                    obligation: super::super::recording::RecordingObligationId {
                        kind: "enabledness".to_owned(),
                        node: Some("Flow::Main::Ship".to_owned()),
                        instance: Some("w-1".to_owned()),
                    },
                    verdict: super::super::recording::RecordingVerdict::Violated,
                    cause: Some(super::super::recording::RecordingCause::Unexpected),
                    witness: vec![3],
                }],
                closure: super::super::recording::RecordingClosure {
                    state: "open".to_owned(),
                    premises: Vec::new(),
                    producer: "recording",
                },
                facts: Vec::new(),
                global: super::super::recording::RecordingGlobalResult {
                    label: "refuted".to_owned(),
                    basis: "violation".to_owned(),
                    complete: false,
                    cause: Some("violation".to_owned()),
                },
                trace_position: Some(3),
            },
            &fsl_semantics::model::observation::DocumentRef {
                authority: "test".to_owned(),
                identity: "recording".to_owned(),
            },
        );

        assert_eq!(output.obligations[0].verdict, "violated");
        assert_eq!(output.obligations[0].cause.as_deref(), Some("unexpected"));
        assert_eq!(output.obligations[0].witness, Some(serde_json::json!([3])));
        assert_eq!(output.closure.state, "open");
        assert_eq!(output.closure.producer, "recording");
        assert_eq!(output.global.label, "refuted");
        assert!(!output.global.complete);
        assert!(output.projections.is_none());
        assert_eq!(
            output
                .recording
                .as_ref()
                .map(|identity| identity.identity.as_str()),
            Some("recording")
        );
        assert!(output.replay.is_none());
    }
}

fn protocol_refusal(refusal: ProtocolRefusal) -> ConformanceReplayRefusal {
    ConformanceReplayRefusal::invalid(format!("protocol admission: {refusal:?}"), None)
}

fn replay_protocol<M: fsl_eval::simulation::protocol::MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    counterexample: &TemporalCounterexample<ProtocolTransition>,
    cancel: &Cancel,
) -> Result<ConformanceReplay, ConformanceReplayRefusal> {
    let mut state = system
        .initial()
        .into_iter()
        .nth(counterexample.initial_state)
        .ok_or_else(|| ConformanceReplayRefusal::invalid("initial state is out of range", None))?;
    for (position, step) in counterexample.prefix.iter().enumerate() {
        if cancel.tripped().is_some() {
            return Err(ConformanceReplayRefusal::invalid(
                "replay cancelled",
                Some(position),
            ));
        }
        let expansion = system.successors(&state).map_err(|error| {
            ConformanceReplayRefusal::invalid(format!("{error:?}"), Some(position))
        })?;
        let mut matched_identity = false;
        let mut next = None;
        for (transition, candidate) in expansion.successors {
            if transition != step.transition {
                continue;
            }
            matched_identity = true;
            let (_, key) = system.state_key(&candidate).map_err(|error| {
                ConformanceReplayRefusal::invalid(error.to_string(), Some(position))
            })?;
            if key == step.post_state {
                next = Some(candidate);
                break;
            }
        }
        state = next.ok_or_else(|| {
            let detail = if matched_identity {
                "transition reaches a different state"
            } else {
                "transition is not enabled at this position"
            };
            ConformanceReplayRefusal::invalid(detail, Some(position))
        })?;
    }
    if matches!(counterexample.kind, CounterexampleKind::Deadlock) {
        return if !system
            .successors(&state)
            .map_err(|error| {
                ConformanceReplayRefusal::invalid(
                    format!("{error:?}"),
                    Some(counterexample.prefix.len()),
                )
            })?
            .successors
            .is_empty()
        {
            Ok(ConformanceReplay::ReplayParity {
                detail: "the counterexample ends before a deadlock".to_owned(),
            })
        } else {
            Ok(ConformanceReplay::Reproduced)
        };
    }
    Ok(ConformanceReplay::ReplayParity {
        detail: "the counterexample refutation kind is not supported by this replay arm".to_owned(),
    })
}
