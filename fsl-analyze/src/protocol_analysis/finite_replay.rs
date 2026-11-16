// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 "Supplied trace refusals": the `finite-replay` engine of a
//! protocol claim. It replays a finite protocol trace the caller supplies,
//! written in FSpec FR-433's spelling, by the re-execution that replays the
//! engine's own counterexamples (FR-217): each step's transition identity
//! must be enabled at the replayed state and reach the post-state key the
//! step records. It builds no transition system of its own; the one
//! `fsl-eval` protocol system steps the trace.

use base64::Engine as _;
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_eval::simulation::protocol::{
    live_instance_bound, send_record_json, transition_json, MemoryModel, ProtocolRefusal,
    ProtocolSystem, ResolvedMemory, SubjectMemory,
};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedProtocol;
use serde::Deserialize;
use serde_json::Value;

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::manifest::BACKEND_ID;
use crate::analyze::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding};

/// The FSpec FR-433 trace document a `finite-replay` item names.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SuppliedTrace {
    /// The document's format word.
    format: String,
    /// The document's declared `{authority, identity}`; the request selects
    /// the document by it.
    #[serde(rename = "identity")]
    _identity: Value,
    /// The index of the initial state the trace starts from.
    #[serde(default)]
    initial_state: usize,
    /// The trace's steps, in order.
    steps: Vec<SuppliedStep>,
}

/// One step of a supplied trace: its transition identity, the key of its
/// post-state and, for a `send`, its send record.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppliedStep {
    transition: Value,
    post_state: PostState,
    #[serde(default)]
    send_record: Option<Value>,
}

/// A state key as the counterexample wire spells it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct PostState {
    key: String,
}

/// The format word of a supplied protocol trace.
const FORMAT: &str = "filament.protocol-trace/v1";

/// Why a supplied trace document is declined before any step replays
/// (FSpec FR-433 "Wire").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TraceDefect {
    /// The document is not a trace in FR-433's spelling: another format, or
    /// a missing, extra or mistyped document member.
    Malformed,
    /// The `send` step at this position carries no `send_record`.
    MissingSendRecord(usize),
    /// The non-`send` step at this position carries a `send_record`.
    ExtraSendRecord(usize),
}

impl TraceDefect {
    /// The `invalid_runtime_input` cause word of the decline.
    pub(crate) fn cause(self) -> &'static str {
        match self {
            Self::Malformed => "invalid-value",
            Self::MissingSendRecord(_) => "missing-member",
            Self::ExtraSendRecord(_) => "unknown-member",
        }
    }

    /// The position of the step the defect is at, if it is at one.
    pub(crate) fn position(self) -> Option<usize> {
        match self {
            Self::Malformed => None,
            Self::MissingSendRecord(position) | Self::ExtraSendRecord(position) => Some(position),
        }
    }
}

impl SuppliedTrace {
    /// The trace `bytes` hold. The whole document is read before any step
    /// replays: another format, a missing, extra or mistyped member, and a
    /// `send` step without a `send_record` or another step with one are
    /// each a wire-shape defect (FSpec FR-433 "Wire", FR-433-AC-8).
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, TraceDefect> {
        let trace: Self = serde_json::from_slice(bytes).map_err(|_| TraceDefect::Malformed)?;
        if trace.format != FORMAT {
            return Err(TraceDefect::Malformed);
        }
        for (position, step) in trace.steps.iter().enumerate() {
            let send = step.transition.get("step").and_then(Value::as_str) == Some("send");
            match (send, step.send_record.is_some()) {
                (true, false) => return Err(TraceDefect::MissingSendRecord(position)),
                (false, true) => return Err(TraceDefect::ExtraSendRecord(position)),
                _ => {}
            }
        }
        Ok(trace)
    }
}

/// How a supplied trace settled over the protocol.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ReplayVerdict {
    /// Every step replayed; the count of steps.
    Replayed(usize),
    /// The step at this position is not enabled at the replayed state, or
    /// reaches another post-state than it records.
    StaleStep(usize),
    /// The trace names an initial state the subject does not have.
    NoInitialState,
}

/// Why a replay reached no verdict.
#[derive(Debug)]
pub(crate) enum ReplayFault {
    /// The protocol system could not be built.
    Protocol,
    /// The model admission refused the subject.
    Admission(Box<fsl_semantics::model::observation::AdmissionFailure>),
    /// An expansion stopped before the step could be decided.
    Stopped(fsl_eval::simulation::ExpansionStop),
    /// An invariant of the system broke.
    Broken,
}

/// The state-key spelling of the counterexample wire.
fn state_wire(key: &StateKey) -> PostState {
    PostState {
        key: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes()),
    }
}

/// Re-execute `trace` through `system` from the initial state it names.
fn replay_steps<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    trace: &SuppliedTrace,
) -> Result<ReplayVerdict, ReplayFault> {
    use fsl_eval::simulation::SubjectSystem;
    let Some(mut state) = system.initial().into_iter().nth(trace.initial_state) else {
        return Ok(ReplayVerdict::NoInitialState);
    };
    for (position, step) in trace.steps.iter().enumerate() {
        let expansion = system.successors(&state).map_err(ReplayFault::Stopped)?;
        let mut reached = None;
        for (transition, post) in expansion.successors {
            if transition_json(&transition).as_ref() != Some(&step.transition)
                || transition.send_record().and_then(send_record_json) != step.send_record
            {
                continue;
            }
            let key = system
                .canonical_state_key(&post)
                .map_err(|_| ReplayFault::Broken)?;
            if state_wire(&key) == step.post_state {
                reached = Some(post);
                break;
            }
        }
        match reached {
            Some(post) => state = post,
            None => return Ok(ReplayVerdict::StaleStep(position)),
        }
    }
    Ok(ReplayVerdict::Replayed(trace.steps.len()))
}

/// The bindings the claim's protocol subject is over: the model simulation
/// request of the analyze subject, and the protocol the claim names.
pub(crate) struct ReplaySubject<'a, 'b> {
    pub(crate) package: &'a CheckedPackage,
    pub(crate) binding: &'b SubjectBinding<'a>,
    pub(crate) limits: &'b AnalyzeLimits,
    pub(crate) protocol: &'a CheckedProtocol,
}

impl<'a> ReplaySubject<'a, '_> {
    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            package: self.package,
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: u64::MAX,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }

    /// The package identity each static role is played by: the model its
    /// `on` names, else the subject's one selected model.
    fn roles(&self) -> Vec<RoleBinding> {
        let selections = self.package.graph().model_selections();
        let sole = match selections {
            [only] => Some(only.identity.clone()),
            _ => None,
        };
        self.protocol
            .roles
            .iter()
            .filter(|role| !role.replicated)
            .filter_map(|role| {
                let named = role.model_selection.as_ref().and_then(|name| {
                    selections
                        .iter()
                        .find(|selection| selection.identity == *name)
                        .map(|selection| selection.identity.clone())
                });
                Some(RoleBinding {
                    role: role.name.clone(),
                    model: named.or_else(|| sole.clone())?,
                })
            })
            .collect()
    }

    /// The subject with `over` bound to `instance`.
    fn subject(&self, over: ModelInstance) -> Result<ProtocolSubject<'a>, ReplayFault> {
        let memory =
            ResolvedMemory::resolve(self.protocol, &[]).map_err(|_| ReplayFault::Broken)?;
        Ok(ProtocolSubject {
            model: ModelSubject {
                simulation: self.simulation(),
                proof_bounds: Vec::new(),
            },
            protocol: self.protocol.name.clone(),
            over,
            roles: self.roles(),
            memory,
        })
    }

    /// The instances the protocol's `over` input can be bound to.
    fn instances(&self) -> Result<Vec<ModelInstance>, ReplayFault> {
        let input = self
            .protocol
            .slots
            .binders
            .first()
            .filter(|binder| binder.node.is_none())
            .ok_or(ReplayFault::Broken)?;
        let model = ModelSystem::new(self.simulation())
            .map_err(|failure| ReplayFault::Admission(Box::new(failure)))?;
        model
            .instances(&input.value_type)
            .map_err(|_| ReplayFault::Broken)
    }

    /// Replay `trace` with `over` bound to `instance`.
    fn replay_over(
        &self,
        instance: ModelInstance,
        trace: &SuppliedTrace,
    ) -> Result<ReplayVerdict, ReplayFault> {
        let subject = self.subject(instance)?;
        let model = ModelSystem::new(self.simulation())
            .map_err(|failure| ReplayFault::Admission(Box::new(failure)))?;
        let built = |_refusal: ProtocolRefusal| ReplayFault::Protocol;
        let live = live_instance_bound(None);
        match SubjectMemory::of(&subject.memory, None, None) {
            SubjectMemory::Sc => replay_steps(
                &ProtocolSystem::new(model, &subject, fsl_eval::simulation::protocol::Sc)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
            ),
            SubjectMemory::Tso(memory) => replay_steps(
                &ProtocolSystem::new(model, &subject, memory)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
            ),
            SubjectMemory::Ra(memory) => replay_steps(
                &ProtocolSystem::new(model, &subject, memory)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
            ),
        }
    }

    /// Replay `trace` over each instance the protocol's `over` input can be
    /// bound to. The trace replays when any binding replays it; otherwise
    /// the verdict is the stale step that came furthest.
    pub(crate) fn replay(&self, trace: &SuppliedTrace) -> Result<ReplayVerdict, ReplayFault> {
        let mut furthest: Option<ReplayVerdict> = None;
        for instance in self.instances()? {
            match self.replay_over(instance, trace)? {
                replayed @ ReplayVerdict::Replayed(_) => return Ok(replayed),
                ReplayVerdict::StaleStep(position) => {
                    let ahead = match furthest {
                        Some(ReplayVerdict::StaleStep(held)) => position > held,
                        _ => true,
                    };
                    if ahead {
                        furthest = Some(ReplayVerdict::StaleStep(position));
                    }
                }
                ReplayVerdict::NoInitialState => {
                    furthest.get_or_insert(ReplayVerdict::NoInitialState);
                }
            }
        }
        Ok(furthest.unwrap_or(ReplayVerdict::NoInitialState))
    }
}

/// What a `finite-replay` item reports.
pub(crate) enum FiniteReplayReport {
    /// Every step replayed: the number of steps.
    Replayed(u64),
    /// The trace is declined `invalid-input` with the `invalid_runtime_input`
    /// cause word `cause`, naming the step at `position` when the decline is
    /// at one.
    Declined {
        /// The `invalid_runtime_input` cause word.
        cause: &'static str,
        /// The position of the step the decline names.
        position: Option<usize>,
    },
}

fn fault() -> EngineReport {
    EngineReport::Refused(Refusal::Failed(CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    )))
}

/// The `finite-replay` item at `index` that replays `trace` over the
/// protocol `protocol` of `package` (FR-636). A trace document with a wire
/// defect is declined before any step replays (FSpec FR-433-AC-8).
pub(crate) fn finite_replay_item<'a>(
    index: RequestIndex,
    package: &'a CheckedPackage,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    protocol: &'a CheckedProtocol,
    trace: Result<SuppliedTrace, TraceDefect>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::FiniteReplay,
        Some(DeclarationFamily::Protocol),
        Mode::Bounded,
        BACKEND_ID,
        move |_| {
            let trace = match trace {
                Ok(trace) => trace,
                Err(defect) => {
                    return EngineReport::FiniteReplay(FiniteReplayReport::Declined {
                        cause: defect.cause(),
                        position: defect.position(),
                    })
                }
            };
            let subject = ReplaySubject {
                package,
                binding: &binding,
                limits: &limits,
                protocol,
            };
            match subject.replay(&trace) {
                Ok(ReplayVerdict::Replayed(steps)) => EngineReport::FiniteReplay(
                    FiniteReplayReport::Replayed(u64::try_from(steps).unwrap_or(u64::MAX)),
                ),
                Ok(ReplayVerdict::StaleStep(position)) => {
                    EngineReport::FiniteReplay(FiniteReplayReport::Declined {
                        cause: "invalid-value",
                        position: Some(position),
                    })
                }
                Ok(ReplayVerdict::NoInitialState) => {
                    EngineReport::FiniteReplay(FiniteReplayReport::Declined {
                        cause: "invalid-value",
                        position: None,
                    })
                }
                Err(ReplayFault::Admission(failure)) => {
                    EngineReport::Refused(crate::analyze::admission(&failure))
                }
                Err(ReplayFault::Stopped(stop)) => {
                    EngineReport::Refused(Refusal::Failed(stop.cause))
                }
                Err(ReplayFault::Protocol | ReplayFault::Broken) => fault(),
            }
        },
    )
}
