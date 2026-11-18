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
    Capacities, MemoryModel, ProtocolRefusal, ProtocolSystem, ResolvedMemory, SubjectMemory,
    live_instance_bound, send_record_json, transition_json,
};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedProtocol;
use serde_json::Value;

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::manifest::BACKEND_ID;
use crate::analyze::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding};

/// The FSpec FR-433 trace document a `finite-replay` item names: a format
/// word, its declared `{authority, identity}` (the request selects the
/// document by it), the index of the initial state it starts from, and its
/// steps, spelled as a protocol counterexample's `prefix`.
#[derive(Clone, Debug)]
pub(crate) struct SuppliedTrace {
    /// The index of the initial state the trace starts from.
    initial_state: usize,
    /// The trace's steps, in order.
    steps: Vec<SuppliedStep>,
}

/// One step of a supplied trace: its transition identity, the key of its
/// post-state and, for a `send`, its send record.
#[derive(Clone, Debug)]
struct SuppliedStep {
    transition: Value,
    post_state: PostState,
    send_record: Option<Value>,
}

/// A state key as the counterexample wire spells it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct PostState {
    key: String,
}

/// The format word of a supplied protocol trace.
const FORMAT: &str = "filament.protocol-trace/v1";

/// The members of a trace document, in the order they are read.
const DOCUMENT_MEMBERS: [&str; 4] = ["format", "identity", "initial_state", "prefix"];

/// The members of a trace step, in the order they are read.
const STEP_MEMBERS: [&str; 3] = ["transition", "post_state", "send_record"];

/// What a wire-shape defect of a supplied trace is (FR-941 "Read").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DefectKind {
    /// A required member is absent.
    MissingMember,
    /// A member the spelling does not have is present.
    UnknownMember,
    /// A member holds a value of another JSON kind.
    WrongValueKind,
    /// A member holds a value of the right kind that is not admitted: a
    /// format word other than the trace format, or a document that is not
    /// JSON.
    InvalidValue,
}

/// Why a supplied trace document is declined before any step replays
/// (FR-941 "Read", FSpec FR-433 "Wire"): the first defect, its kind, the
/// JSON pointer of the member it is at and, inside a step, that step's
/// position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TraceDefect {
    kind: DefectKind,
    path: String,
    position: Option<usize>,
}

impl TraceDefect {
    /// A defect of `kind` at the document member `member`.
    fn at(kind: DefectKind, member: &str) -> Self {
        Self {
            kind,
            path: format!("/{member}"),
            position: None,
        }
    }

    /// A defect of `kind` at the member `member` of the step at `position`.
    fn in_step(kind: DefectKind, position: usize, member: &str) -> Self {
        Self {
            kind,
            path: format!("/prefix/{position}/{member}"),
            position: Some(position),
        }
    }

    /// A document that is not JSON, or not provisioned: no member names it.
    pub(crate) fn unreadable() -> Self {
        Self {
            kind: DefectKind::InvalidValue,
            path: String::new(),
            position: None,
        }
    }

    /// The `invalid_runtime_input` cause word of the decline.
    pub(crate) fn cause(&self) -> &'static str {
        match self.kind {
            DefectKind::MissingMember => "missing-member",
            DefectKind::UnknownMember => "unknown-member",
            DefectKind::WrongValueKind => "wrong-value-kind",
            DefectKind::InvalidValue => "invalid-value",
        }
    }

    /// The position of the step the defect is in, if it is in one.
    pub(crate) fn position(&self) -> Option<usize> {
        self.position
    }

    /// The JSON pointer of the defect inside the trace document; empty for
    /// the whole document.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

/// The first member of `object` not in `known`, in key order.
fn unknown<'o>(object: &'o serde_json::Map<String, Value>, known: &[&str]) -> Option<&'o str> {
    object
        .keys()
        .map(String::as_str)
        .find(|member| !known.contains(member))
}

impl SuppliedTrace {
    /// The trace `bytes` hold. The whole document is read before any step
    /// replays, so the answer never depends on how far a replay got: a
    /// missing, extra or mistyped member, and a `send` step without a
    /// `send_record` or another step with one, are each a wire-shape defect,
    /// and the first one is reported (FR-941 "Read", FSpec FR-433-AC-8).
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, TraceDefect> {
        use DefectKind::{InvalidValue, MissingMember, UnknownMember, WrongValueKind};
        let document: Value =
            serde_json::from_slice(bytes).map_err(|_| TraceDefect::unreadable())?;
        let Value::Object(document) = document else {
            return Err(TraceDefect {
                kind: WrongValueKind,
                path: String::new(),
                position: None,
            });
        };
        match document.get("format") {
            None => return Err(TraceDefect::at(MissingMember, "format")),
            Some(Value::String(format)) if format == FORMAT => {}
            Some(Value::String(_)) => return Err(TraceDefect::at(InvalidValue, "format")),
            Some(_) => return Err(TraceDefect::at(WrongValueKind, "format")),
        }
        match document.get("identity") {
            None => return Err(TraceDefect::at(MissingMember, "identity")),
            Some(Value::Object(_)) => {}
            Some(_) => return Err(TraceDefect::at(WrongValueKind, "identity")),
        }
        let initial_state = match document.get("initial_state") {
            None => 0,
            Some(value) => value
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| TraceDefect::at(WrongValueKind, "initial_state"))?,
        };
        let prefix = match document.get("prefix") {
            None => return Err(TraceDefect::at(MissingMember, "prefix")),
            Some(Value::Array(prefix)) => prefix,
            Some(_) => return Err(TraceDefect::at(WrongValueKind, "prefix")),
        };
        if let Some(member) = unknown(&document, &DOCUMENT_MEMBERS) {
            return Err(TraceDefect::at(UnknownMember, member));
        }
        let steps = prefix
            .iter()
            .enumerate()
            .map(|(position, step)| SuppliedStep::read(position, step))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            initial_state,
            steps,
        })
    }
}

impl SuppliedStep {
    /// The step at `position` that `step` spells.
    fn read(position: usize, step: &Value) -> Result<Self, TraceDefect> {
        use DefectKind::{MissingMember, UnknownMember, WrongValueKind};
        let defect = |kind, member| TraceDefect::in_step(kind, position, member);
        let Value::Object(step) = step else {
            return Err(TraceDefect {
                kind: WrongValueKind,
                path: format!("/prefix/{position}"),
                position: Some(position),
            });
        };
        let transition = match step.get("transition") {
            None => return Err(defect(MissingMember, "transition")),
            Some(transition @ Value::Object(_)) => transition.clone(),
            Some(_) => return Err(defect(WrongValueKind, "transition")),
        };
        let post_state = match step.get("post_state") {
            None => return Err(defect(MissingMember, "post_state")),
            Some(Value::Object(post_state)) => {
                let key = match post_state.get("key") {
                    None => return Err(defect(MissingMember, "post_state/key")),
                    Some(Value::String(key)) => key.clone(),
                    Some(_) => return Err(defect(WrongValueKind, "post_state/key")),
                };
                if let Some(member) = unknown(post_state, &["key"]) {
                    return Err(defect(UnknownMember, &format!("post_state/{member}")));
                }
                PostState { key }
            }
            Some(_) => return Err(defect(WrongValueKind, "post_state")),
        };
        let send = transition.get("step").and_then(Value::as_str) == Some("send");
        let send_record = match (send, step.get("send_record")) {
            (true, None) => return Err(defect(MissingMember, "send_record")),
            (false, Some(_)) => return Err(defect(UnknownMember, "send_record")),
            (true, Some(record @ Value::Object(_))) => Some(record.clone()),
            (true, Some(_)) => return Err(defect(WrongValueKind, "send_record")),
            (false, None) => None,
        };
        if let Some(member) = unknown(step, &STEP_MEMBERS) {
            return Err(defect(UnknownMember, member));
        }
        Ok(Self {
            transition,
            post_state,
            send_record,
        })
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
    /// The protocol has a symbolic capacity, and a supplied trace's subject
    /// binds no capacity: requires-bound (FSpec FR-785).
    UnboundCapacity,
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
    pub(crate) fn simulation(&self) -> ModelSimulationRequest<'a> {
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
    pub(crate) fn roles(&self) -> Vec<RoleBinding> {
        let selected: std::collections::BTreeSet<String> = self
            .binding
            .universes
            .iter()
            .map(|universe| universe.population.package.clone())
            .collect();
        let sole = (selected.len() == 1)
            .then(|| selected.iter().next().cloned())
            .flatten();
        self.protocol
            .roles
            .iter()
            .filter(|role| !role.replicated)
            .filter_map(|role| {
                let named = role
                    .model_selection
                    .as_ref()
                    .and_then(|name| self.package.graph().model_package_identity(name))
                    .filter(|package| selected.contains(*package))
                    .map(str::to_owned);
                let contextual = role
                    .context
                    .as_ref()
                    .map(|context| context.original().package())
                    .filter(|package| selected.contains(*package))
                    .map(str::to_owned);
                Some(RoleBinding {
                    role: role.name.clone(),
                    model: named.or(contextual).or_else(|| sole.clone())?,
                })
            })
            .collect()
    }

    /// The subject with `over` bound to `instance`.
    pub(crate) fn subject(&self, over: ModelInstance) -> Result<ProtocolSubject<'a>, ReplayFault> {
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
            // An analyze request's subject states no capacity binding;
            // `replay` returns requires-bound for a symbolic capacity first.
            capacities: Vec::new(),
            memory,
        })
    }

    /// The instances the protocol's `over` input can be bound to.
    pub(crate) fn instances(&self) -> Result<Vec<ModelInstance>, ReplayFault> {
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
        // FSpec FR-785: an analyze request's subject carries no capacity
        // binding, so a symbolic capacity is an unbounded root here.
        let capacities = Capacities::admit(self.protocol, &[]).map_err(|_| ReplayFault::Broken)?;
        if !capacities.unbound().is_empty() {
            return Err(ReplayFault::UnboundCapacity);
        }
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
        /// The JSON pointer of a wire-shape defect inside the trace
        /// document (FR-941 "Read").
        path: Option<String>,
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
                        path: Some(defect.path().to_owned()).filter(|path| !path.is_empty()),
                    });
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
                        path: None,
                    })
                }
                Ok(ReplayVerdict::NoInitialState) => {
                    EngineReport::FiniteReplay(FiniteReplayReport::Declined {
                        cause: "invalid-value",
                        position: None,
                        path: Some("/initial_state".to_owned()),
                    })
                }
                Err(ReplayFault::Admission(failure)) => {
                    EngineReport::Refused(crate::analyze::admission(&failure))
                }
                Err(ReplayFault::Stopped(stop)) => {
                    EngineReport::Refused(Refusal::Failed(stop.cause))
                }
                Err(ReplayFault::UnboundCapacity) => EngineReport::Refused(Refusal::RequiresBound),
                Err(ReplayFault::Protocol | ReplayFault::Broken) => fault(),
            }
        },
    )
}
