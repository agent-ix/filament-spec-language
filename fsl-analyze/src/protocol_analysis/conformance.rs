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
        ConformanceCounterexample::Recording {
            recording,
            trace_position,
        } => {
            let protocol = subject
                .model
                .simulation
                .package
                .graph()
                .protocols()
                .iter()
                .find(|protocol| protocol.name == subject.protocol)
                .ok_or_else(|| {
                    ConformanceReplayRefusal::invalid(
                        format!("checked protocol `{}` is absent", subject.protocol),
                        None,
                    )
                })?;
            fsl_replay::BoundExceeded::check(recording.len(), limits)
                .map_err(|error| ConformanceReplayRefusal::invalid(error.to_string(), None))?;
            let assessment = super::recording::assess_recording(protocol, recording)
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

/// Convert a recording assessment to the shared FR-990 item representation.
pub(crate) fn recording_output(
    assessment: super::recording::RecordingAssessment,
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
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[trace("TC-2805", "FR-990-AC-1")]
    #[test]
    fn recording_assessment_maps_to_the_shared_conformance_output() {
        let output = recording_output(super::super::recording::RecordingAssessment {
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
            global: super::super::recording::RecordingGlobalResult {
                label: "refuted".to_owned(),
                basis: "violation".to_owned(),
                complete: false,
                cause: Some("violation".to_owned()),
            },
            trace_position: Some(3),
        });

        assert_eq!(output.obligations[0].verdict, "violated");
        assert_eq!(output.obligations[0].cause.as_deref(), Some("unexpected"));
        assert_eq!(output.obligations[0].witness, Some(serde_json::json!([3])));
        assert_eq!(output.closure.state, "open");
        assert_eq!(output.closure.producer, "recording");
        assert_eq!(output.global.label, "refuted");
        assert!(!output.global.complete);
        assert!(output.projections.is_none());
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
        let expansion = system
            .successors(&state)
            .map_err(|error| ConformanceReplayRefusal::invalid(format!("{error:?}"), Some(position)))?;
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
