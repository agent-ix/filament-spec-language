// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-991: replay evidence carried by a global-conformance result.

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::model_subject::ProtocolSubject;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::protocol::{
    ProtocolRefusal, ProtocolSystem, ProtocolTransition, Sc, SubjectMemory, live_instance_bound,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_replay::TemporalCounterexample;
use fsl_replay::{CounterexampleKind, ProtocolConformanceOutput, ReplayLimits, TerminalRecord};

/// The independently settled global-conformance record and its FR-990 output.
///
/// The terminal record remains the authority for item settlement. The output
/// carries the obligation-level findings and closure evidence alongside it.
pub(crate) struct ConformanceReport {
    pub(crate) terminal: TerminalRecord,
    pub(crate) output: ProtocolConformanceOutput,
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
                    replay_protocol(&system, counterexample, cancel)
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
                    replay_protocol(&system, counterexample, cancel)
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
                    replay_protocol(&system, counterexample, cancel)
                }
            }
        }
        ConformanceCounterexample::Recording { .. } => Ok(ConformanceReplay::ReplayParity {
            detail: "recording replay is provided by the recording assessment module".to_owned(),
        }),
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
            ConformanceReplayRefusal::invalid(error.to_string(), Some(position))
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
                    error.to_string(),
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
