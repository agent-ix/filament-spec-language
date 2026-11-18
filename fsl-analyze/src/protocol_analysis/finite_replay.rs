// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 "Supplied trace refusals": the `finite-replay` engine of a
//! protocol claim. It replays a finite protocol trace the caller supplies,
//! written in FSpec FR-433's spelling, by the re-execution that replays the
//! engine's own counterexamples (FR-217): each step's transition identity
//! must be enabled at the replayed state and reach the post-state key the
//! step records. It builds no transition system of its own; the one
//! `fsl-eval` protocol system steps the trace.

use base64::Engine as _;
use filament_exact::{Identifier, ScalarLimits};
use fsl_eval::model_subject::{
    ModelInstance, ModelLetters, ModelSubject, ProtocolSubject, RoleBinding,
};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_eval::simulation::protocol::{
    live_instance_bound, over_reference, send_record_json, transition_json, Capacities,
    MemoryModel, ProtocolFairness, ProtocolRefusal, ProtocolSystem, ProtocolTransition,
    ResolvedMemory, SubjectMemory,
};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_eval::temporal::{TemporalOutcome, TemporalRefusal};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::CheckedPackage;
use fsl_replay::{
    evaluate_protocol_trace, settle_protocol_check, undefined_finding, CounterexampleKind,
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord, ModelStep,
    ModelTraceSubject, ProtocolCertificateRequest, ProtocolTraceEnd, ProtocolTraceSubject,
    ProtocolWire, QualifiedName, ReplayTargetMembers, RequestTargetWire, TemporalCounterexample,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use fsl_semantics::check::{CheckedProtocol, TemporalClauseNode};
use serde_json::Value;

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::manifest::BACKEND_ID;
use crate::analyze::{
    derived_target, replay_protocol_over, AnalyzeItem, AnalyzeLimits, EngineReport, NoLetters,
    ProtocolReplayInputs, Refusal, ReplayShape, SourceProvision, SubjectBinding,
};

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
#[derive(Debug)]
pub(crate) enum ReplayVerdict {
    /// Every step replayed and no expression was undefined at a replayed
    /// state; the count of steps, for a claim with no required clause.
    Replayed(usize),
    /// The step at this position is not enabled at the replayed state, or
    /// reaches another post-state than it records.
    StaleStep(usize),
    /// The trace names an initial state the subject does not have.
    NoInitialState,
    /// A protocol expression was undefined at a replayed state (FR-941
    /// "Undefined steps").
    Undefined(Box<UndefinedStep>),
    /// Every step replayed and the claim's required clauses were read over
    /// the replayed behaviour (FR-941 "Formula claims").
    Claimed(Box<Claimed>),
}

/// The first protocol expression that evaluated undefined at a replayed
/// state, and the counterexample that names it.
#[derive(Debug)]
pub(crate) struct UndefinedStep {
    /// The supplied trace's steps up to that state, with the undefined
    /// evaluation at its position.
    counterexample: TemporalCounterexample<ProtocolTransition>,
    /// The object the protocol's `over` input was bound to.
    over: ModelInstance,
}

/// The reading of the claim's required clauses over a replayed behaviour.
#[derive(Debug)]
pub(crate) struct Claimed {
    /// The index, among the required clauses, of the clause that decides:
    /// the first violated or undefined, else the first not `holds`; `None`
    /// when every clause holds.
    deciding: Option<usize>,
    /// The deciding clause's reading, or `holds`.
    evaluation: Result<TemporalOutcome, TemporalRefusal>,
    /// The number of steps replayed.
    steps: usize,
    /// The counterexample of a violated or undefined reading: the supplied
    /// trace through the position the evaluator names.
    counterexample: Option<TemporalCounterexample<ProtocolTransition>>,
    /// The object the protocol's `over` input was bound to.
    over: ModelInstance,
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

/// What a claim's required clauses add to a replay (FR-941 "Formula claims").
struct Formula<'c, 'a> {
    /// The required clauses, in `requires` order.
    required: &'c [&'a TemporalClauseNode],
    /// The object the protocol's `over` input is bound to, which each
    /// required clause's `over` parameter is bound to.
    instance: &'c ModelInstance,
    /// The evaluation budget.
    accounting: ScalarLimits,
}

/// The counterexample over the first `steps` steps of a replay, of `kind`.
fn behaviour(
    initial_state: usize,
    steps: &[ModelStep<ProtocolTransition>],
    over_binding: Option<ModelInstance>,
    fairness: Vec<ProtocolFairness>,
    kind: CounterexampleKind,
) -> TemporalCounterexample<ProtocolTransition> {
    TemporalCounterexample {
        initial_state,
        prefix: steps.to_vec(),
        over_binding,
        fairness,
        kind,
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    }
}

/// One required clause's reading over the replayed behaviour.
struct Reading {
    evaluation: Result<TemporalOutcome, TemporalRefusal>,
    /// The position and locus of the first undefined letter.
    undefined: Option<(u64, UndefinedLocus)>,
}

/// FR-941 "Formula claims": `clause` read over the finite behaviour
/// `payload` replays, by the one temporal evaluator.
fn read_clause<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    node: &TemporalClauseNode,
    payload: &TemporalCounterexample<ProtocolTransition>,
    accounting: ScalarLimits,
) -> Result<Reading, ReplayFault> {
    let letters = system
        .model()
        .clause_letters(node)
        .map_err(|_| ReplayFault::Broken)?;
    match evaluate_protocol_trace(system, payload, node.clause(), &letters, accounting)
        .map_err(|_| ReplayFault::Broken)?
    {
        ProtocolTraceEnd::Evaluated(run) => Ok(Reading {
            evaluation: run.outcome,
            undefined: run.undefined,
        }),
        ProtocolTraceEnd::Stopped(stop) => Err(ReplayFault::Stopped(stop)),
    }
}

/// The fairness set the clause carries over the protocol system, which a
/// replayed counterexample of the clause must record (FR-212).
fn fairness_of<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    node: &TemporalClauseNode,
) -> Result<Vec<ProtocolFairness>, ReplayFault> {
    let clause = node.clause();
    let protocol_fairness: Vec<_> = clause
        .protocol_fairness()
        .map(|(fairness, _)| fairness.clone())
        .collect();
    system
        .fairness_set_with_targets(clause.fairness(), &protocol_fairness)
        .map_err(|_| ReplayFault::Broken)
}

/// Whether the reading refutes its clause: violated, or undefined.
fn refutes(reading: &Reading) -> bool {
    matches!(
        reading.evaluation,
        Ok(TemporalOutcome::Violated { .. } | TemporalOutcome::Undefined(_))
    )
}

/// FR-941 "Formula claims": each required clause in `requires` order over the
/// replayed behaviour; the first violated or undefined clause decides, else
/// the first that is not `holds`, else the claim holds.
fn read_claim<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    trace: &SuppliedTrace,
    steps: &[ModelStep<ProtocolTransition>],
    keys: &[StateKey],
    formula: &Formula<'_, '_>,
) -> Result<Claimed, ReplayFault> {
    let over = formula.instance.clone();
    let mut decided: Option<(usize, Reading, Vec<ProtocolFairness>)> = None;
    for (index, node) in formula.required.iter().enumerate() {
        let fairness = fairness_of(system, node)?;
        let payload = behaviour(
            trace.initial_state,
            steps,
            Some(over.clone()),
            fairness.clone(),
            CounterexampleKind::Formula,
        );
        let reading = read_clause(system, node, &payload, formula.accounting)?;
        let holds = matches!(reading.evaluation, Ok(TemporalOutcome::Holds));
        let replace = match &decided {
            None => !holds,
            Some((_, held, _)) => refutes(&reading) && !refutes(held),
        };
        if replace {
            decided = Some((index, reading, fairness));
        }
    }
    let Some((index, reading, fairness)) = decided else {
        return Ok(Claimed {
            deciding: None,
            evaluation: Ok(TemporalOutcome::Holds),
            steps: steps.len(),
            counterexample: None,
            over,
        });
    };
    let node = formula.required.get(index).ok_or(ReplayFault::Broken)?;
    let counterexample = match &reading.evaluation {
        Ok(TemporalOutcome::Violated { at }) => {
            // The counterexample is the supplied trace through the first false
            // activation, as long as that prefix still violates the clause
            // there; otherwise it is the whole trace.
            let through = usize::try_from(at.index())
                .ok()
                .and_then(|at| steps.get(..at))
                .ok_or(ReplayFault::Broken)?;
            let shortened = behaviour(
                trace.initial_state,
                through,
                Some(over.clone()),
                fairness.clone(),
                CounterexampleKind::Formula,
            );
            let shortest = read_clause(system, node, &shortened, formula.accounting)?;
            Some(
                if matches!(
                    &shortest.evaluation,
                    Ok(TemporalOutcome::Violated { at: held }) if held == at
                ) {
                    shortened
                } else {
                    behaviour(
                        trace.initial_state,
                        steps,
                        Some(over.clone()),
                        fairness,
                        CounterexampleKind::Formula,
                    )
                },
            )
        }
        Ok(TemporalOutcome::Undefined(undefined)) => {
            let position = undefined.at.index();
            let (named, locus) = reading.undefined.clone().ok_or(ReplayFault::Broken)?;
            if named != position {
                return Err(ReplayFault::Broken);
            }
            let at = usize::try_from(position).map_err(|_| ReplayFault::Broken)?;
            let through = steps.get(..at).ok_or(ReplayFault::Broken)?;
            let state_key = keys.get(at).ok_or(ReplayFault::Broken)?.clone();
            Some(behaviour(
                trace.initial_state,
                through,
                Some(over.clone()),
                fairness,
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                    site: UndefinedSite {
                        position,
                        state_key,
                        locus,
                        transition: None,
                        time: None,
                    },
                    cause: undefined.cause.clone(),
                }),
            ))
        }
        _ => None,
    };
    Ok(Claimed {
        deciding: Some(index),
        evaluation: reading.evaluation,
        steps: steps.len(),
        counterexample,
        over,
    })
}

/// Re-execute `trace` through `system` from the initial state it names, then
/// read the claim's required clauses over the replayed behaviour.
///
/// At each replayed state, position 0 through the last, the protocol
/// expressions the system evaluates when it expands the state are read first
/// (FR-941 "Undefined steps"): the first undefined one refutes the item
/// there, before a later stale step is reached.
fn replay_steps<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    trace: &SuppliedTrace,
    formula: &Formula<'_, '_>,
) -> Result<ReplayVerdict, ReplayFault> {
    use fsl_eval::simulation::SubjectSystem;
    let Some(mut state) = system.initial().into_iter().nth(trace.initial_state) else {
        return Ok(ReplayVerdict::NoInitialState);
    };
    let key_of = |state: &_| {
        system
            .canonical_state_key(state)
            .map_err(|_| ReplayFault::Broken)
    };
    let mut keys = vec![key_of(&state)?];
    let mut steps: Vec<ModelStep<ProtocolTransition>> = Vec::with_capacity(trace.steps.len());
    for position in 0..=trace.steps.len() {
        let expansion = system.successors(&state).map_err(ReplayFault::Stopped)?;
        if let Some((locus, guard_of, cause)) = undefined_finding(&expansion.findings) {
            let state_key = keys.get(position).ok_or(ReplayFault::Broken)?.clone();
            return Ok(ReplayVerdict::Undefined(Box::new(UndefinedStep {
                counterexample: behaviour(
                    trace.initial_state,
                    &steps,
                    None,
                    Vec::new(),
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: u64::try_from(position).map_err(|_| ReplayFault::Broken)?,
                            state_key,
                            locus,
                            transition: guard_of,
                            time: None,
                        },
                        cause,
                    }),
                ),
                over: formula.instance.clone(),
            })));
        }
        let Some(step) = trace.steps.get(position) else {
            break;
        };
        let mut reached = None;
        for (transition, post) in expansion.successors {
            if transition_json(&transition).as_ref() != Some(&step.transition)
                || transition.send_record().and_then(send_record_json) != step.send_record
            {
                continue;
            }
            let key = key_of(&post)?;
            if state_wire(&key) == step.post_state {
                reached = Some((transition, post, key));
                break;
            }
        }
        match reached {
            Some((transition, post, key)) => {
                steps.push(ModelStep {
                    transition,
                    post_state: key.clone(),
                    delay: None,
                });
                keys.push(key);
                state = post;
            }
            None => return Ok(ReplayVerdict::StaleStep(position)),
        }
    }
    if formula.required.is_empty() {
        return Ok(ReplayVerdict::Replayed(steps.len()));
    }
    read_claim(system, trace, &steps, &keys, formula)
        .map(|claimed| ReplayVerdict::Claimed(Box::new(claimed)))
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
        // FSpec FR-785: an analyze request's subject carries no capacity
        // binding, so a symbolic capacity is an unbounded root of every
        // subject built here.
        let capacities = Capacities::admit(self.protocol, &[]).map_err(|_| ReplayFault::Broken)?;
        if !capacities.unbound().is_empty() {
            return Err(ReplayFault::UnboundCapacity);
        }
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
            // No capacity binding: a symbolic capacity returned above.
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

    /// The temporal clauses the protocol requires (FR-218), in `requires`
    /// order.
    pub(crate) fn required(&self) -> Result<Vec<&'a TemporalClauseNode>, ReplayFault> {
        self.protocol
            .requirements
            .iter()
            .map(|requirement| {
                self.package
                    .graph()
                    .temporal_clauses()
                    .get(requirement.temporal)
                    .ok_or(ReplayFault::Broken)
            })
            .collect()
    }

    /// Replay `trace` with `over` bound to `instance`.
    fn replay_over(
        &self,
        instance: ModelInstance,
        trace: &SuppliedTrace,
    ) -> Result<ReplayVerdict, ReplayFault> {
        let subject = self.subject(instance.clone())?;
        let model = ModelSystem::new(self.simulation())
            .map_err(|failure| ReplayFault::Admission(Box::new(failure)))?;
        let built = |_refusal: ProtocolRefusal| ReplayFault::Protocol;
        let live = live_instance_bound(None);
        let required = self.required()?;
        let formula = Formula {
            required: &required,
            instance: &instance,
            accounting: self.limits.accounting,
        };
        match SubjectMemory::of(&subject.memory, None, None) {
            SubjectMemory::Sc => replay_steps(
                &ProtocolSystem::new(model, &subject, fsl_eval::simulation::protocol::Sc)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
                &formula,
            ),
            SubjectMemory::Tso(memory) => replay_steps(
                &ProtocolSystem::new(model, &subject, memory)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
                &formula,
            ),
            SubjectMemory::Ra(memory) => replay_steps(
                &ProtocolSystem::new(model, &subject, memory)
                    .map_err(built)?
                    .with_max_live_instances(live),
                trace,
                &formula,
            ),
        }
    }

    /// Replay `trace` over each instance the protocol's `over` input can be
    /// bound to. The trace replays when any binding replays it; otherwise
    /// the verdict is the stale step that came furthest.
    pub(crate) fn replay(&self, trace: &SuppliedTrace) -> Result<ReplayVerdict, ReplayFault> {
        let mut furthest: Option<usize> = None;
        let mut no_initial = false;
        for instance in self.instances()? {
            match self.replay_over(instance, trace)? {
                ReplayVerdict::StaleStep(position) => {
                    furthest = Some(furthest.map_or(position, |held| held.max(position)));
                }
                ReplayVerdict::NoInitialState => no_initial = true,
                decided @ (ReplayVerdict::Replayed(_)
                | ReplayVerdict::Undefined(_)
                | ReplayVerdict::Claimed(_)) => return Ok(decided),
            }
        }
        Ok(match furthest {
            Some(position) => ReplayVerdict::StaleStep(position),
            None => {
                let _ = no_initial;
                ReplayVerdict::NoInitialState
            }
        })
    }
}

/// A refuted `finite-replay` claim settled as an EN-1 record over the
/// protocol subject: the record carries the counterexample and its replay.
pub(crate) struct RefutedClaim {
    pub(crate) record: Box<ModelCheckRecord<ProtocolTransition>>,
    pub(crate) subject: Option<ProtocolWire>,
}

/// FR-941's reading of a formula claim, beside the terminal record its kind
/// settles.
pub(crate) struct ClaimReport {
    /// The number of steps replayed.
    pub(crate) steps: u64,
    /// The deciding clause's reading: `holds`, or the evaluator's result.
    pub(crate) evaluation: Result<TemporalOutcome, TemporalRefusal>,
    /// The refutation, for a violated or undefined reading.
    pub(crate) refuted: Option<RefutedClaim>,
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
    /// A required clause was read over the replayed behaviour (FR-941
    /// "Formula claims").
    Claim(Box<ClaimReport>),
    /// A protocol expression was undefined at a replayed state (FR-941
    /// "Undefined steps"): the refutation, and where the expression is.
    UndefinedStep {
        /// The refutation.
        refuted: Box<RefutedClaim>,
        /// `{position, state_key, locus, node}` of the undefined expression.
        at: Value,
    },
}

fn fault() -> EngineReport {
    EngineReport::Refused(Refusal::Failed(CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    )))
}

/// The replay transcript a finite-replay counterexample carries: the claim's
/// own, with no values (FR-128).
const TRANSCRIPT: &str = "<<<assertion|model_check|clause|>>>";

/// FR-941 "Undefined steps": the `where` of the undefined expression the
/// counterexample names, `{position, state_key, locus, node}`.
fn undefined_where(counterexample: &TemporalCounterexample<ProtocolTransition>) -> Option<Value> {
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        return None;
    };
    let site = &evaluation.site;
    let (locus, node) = match &site.locus {
        UndefinedLocus::Protocol(node) => ("protocol", Some(node.clone())),
        UndefinedLocus::Contract(clause) => ("contract", Some(clause.clone())),
        _ => ("expression", None),
    };
    let mut at = serde_json::json!({
        "position": site.position,
        "state_key": {"key": state_wire(&site.state_key).key},
        "locus": locus,
    });
    if let (Some(node), Some(members)) = (node, at.as_object_mut()) {
        members.insert("node".to_owned(), Value::String(node));
    }
    Some(at)
}

/// Everything a finite-replay item needs once its trace has been read.
struct Run<'a, 'b> {
    package: &'a CheckedPackage,
    binding: &'b SubjectBinding<'a>,
    limits: &'b AnalyzeLimits,
    provision: &'a SourceProvision,
    protocol: &'a CheckedProtocol,
    subject: &'b ReplaySubject<'a, 'b>,
}

impl Run<'_, '_> {
    /// Settle `counterexample`, found with the protocol's `over` bound to
    /// `over`, for the required `clause` (`None` for a protocol expression
    /// undefined at a replayed state), as an EN-1 record over the protocol
    /// subject whose replay is FR-217's.
    fn refute(
        &self,
        over: ModelInstance,
        counterexample: TemporalCounterexample<ProtocolTransition>,
        clause: Option<&TemporalClauseNode>,
        cancel: &filament_exact::Cancel,
    ) -> Result<RefutedClaim, ReplayFault> {
        let subject = self.subject.subject(over.clone())?;
        let roles: Vec<(String, String)> = subject
            .roles
            .iter()
            .map(|binding| (binding.role.clone(), binding.model.clone()))
            .collect();
        let trace_subject = ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: self.binding.initial.clone(),
                universes: self.binding.universes.clone(),
                expansion: ExpansionLimits {
                    max_candidates: u64::MAX,
                },
            },
            protocol: self.protocol.name.clone(),
            over: over.clone(),
            roles: subject.roles.clone(),
            capacities: Vec::new(),
            memory: subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        };
        let model = ModelSystem::new(self.subject.simulation())
            .map_err(|failure| ReplayFault::Admission(Box::new(failure)))?;
        let clause_letters = match clause {
            Some(node) => Some(
                model
                    .clause_letters(node)
                    .map_err(|_| ReplayFault::Broken)?,
            ),
            None => None,
        };
        let letters: &dyn ModelLetters = match &clause_letters {
            Some(letters) => letters,
            None => &NoLetters,
        };
        let item = match (&clause_letters, clause) {
            (Some(letters), Some(node)) => ModelCheckItem::Clause {
                clause: node.clause(),
                letters,
            },
            _ => ModelCheckItem::DeadlockFreedom,
        };
        let certificate = ProtocolCertificateRequest {
            subject: self.subject.subject(over.clone())?,
            temporal_clock_selections: &[],
            item,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        };
        let shape = match clause {
            Some(node) => {
                let name = QualifiedName::new(vec![
                    Identifier::new(node.clause().name()).map_err(|_| ReplayFault::Broken)?
                ])
                .map_err(|_| ReplayFault::Broken)?;
                ReplayShape {
                    target: RequestTargetWire::state_clause(name.clone()),
                    members: ReplayTargetMembers::state_clause(name),
                    node: node.identity(),
                    occurrence: OccurrenceKey::new(node.identity(), node.claim().clone()),
                    transcript: TRANSCRIPT,
                }
            }
            None => {
                let anchor = self
                    .package
                    .graph()
                    .protocol_node_keys(&self.protocol.name)
                    .and_then(|keys| keys.clause.clone())
                    .ok_or(ReplayFault::Broken)?;
                let target = derived_target().ok_or(ReplayFault::Broken)?;
                let occurrence = OccurrenceKey::new(
                    anchor.clone(),
                    filament_exact::Origin::new(filament_exact::Role::new("declaration"), 0),
                );
                ReplayShape {
                    target: RequestTargetWire::function(target.clone(), occurrence.clone()),
                    members: ReplayTargetMembers::function(target),
                    node: anchor,
                    occurrence,
                    transcript: TRANSCRIPT,
                }
            }
        };
        let inputs = ProtocolReplayInputs {
            package: self.package,
            binding: self.binding,
            limits: self.limits,
            provision: self.provision,
            subject: trace_subject,
            backend: BACKEND_ID,
        };
        let outcome = ModelCheckOutcome::Violated {
            counterexample: Box::new(counterexample),
            stats: Default::default(),
        };
        let record = settle_protocol_check(
            &outcome,
            &ModelCheckLimits::default(),
            &certificate,
            cancel,
            move |payload| replay_protocol_over(&inputs, shape, payload, letters),
        );
        let on_each = self.protocol_activates_on_each();
        let wire = record.counterexample.as_ref().and_then(|_| {
            over_reference(self.subject.simulation(), &self.protocol.name, &over).map(|over| {
                ProtocolWire {
                    protocol: self.protocol.name.clone(),
                    over,
                    roles,
                    capacities: Vec::new(),
                    max_live_instances: on_each.then_some(live_instance_bound(None)),
                }
            })
        });
        Ok(RefutedClaim {
            record: Box::new(record),
            subject: wire,
        })
    }

    fn protocol_activates_on_each(&self) -> bool {
        fsl_eval::simulation::protocol::activates_on_each(self.protocol)
    }
}

/// The engine report of a fault of the replay.
fn report_fault(error: ReplayFault) -> EngineReport {
    match error {
        ReplayFault::Admission(failure) => {
            EngineReport::Refused(crate::analyze::admission(&failure))
        }
        ReplayFault::Stopped(stop) => EngineReport::Refused(Refusal::Failed(stop.cause)),
        ReplayFault::UnboundCapacity => EngineReport::Refused(Refusal::RequiresBound),
        ReplayFault::Protocol | ReplayFault::Broken => fault(),
    }
}

fn declined(cause: &'static str, position: Option<usize>, path: Option<String>) -> EngineReport {
    EngineReport::FiniteReplay(FiniteReplayReport::Declined {
        cause,
        position,
        path,
    })
}

/// The `finite-replay` item at `index` that replays `trace` over the
/// protocol `protocol` of `package` (FR-636). A trace document with a wire
/// defect is declined before any step replays (FSpec FR-433-AC-8).
pub(crate) fn finite_replay_item<'a>(
    index: RequestIndex,
    package: &'a CheckedPackage,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    protocol: &'a CheckedProtocol,
    trace: Result<SuppliedTrace, TraceDefect>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::FiniteReplay,
        Some(DeclarationFamily::Protocol),
        Mode::Bounded,
        BACKEND_ID,
        move |cancel| {
            let trace = match trace {
                Ok(trace) => trace,
                Err(defect) => {
                    return declined(
                        defect.cause(),
                        defect.position(),
                        Some(defect.path().to_owned()).filter(|path| !path.is_empty()),
                    );
                }
            };
            let subject = ReplaySubject {
                package,
                binding: &binding,
                limits: &limits,
                protocol,
            };
            let run = Run {
                package,
                binding: &binding,
                limits: &limits,
                provision,
                protocol,
                subject: &subject,
            };
            match subject.replay(&trace) {
                Ok(ReplayVerdict::Replayed(steps)) => EngineReport::FiniteReplay(
                    FiniteReplayReport::Replayed(u64::try_from(steps).unwrap_or(u64::MAX)),
                ),
                Ok(ReplayVerdict::StaleStep(position)) => {
                    declined("invalid-value", Some(position), None)
                }
                Ok(ReplayVerdict::NoInitialState) => {
                    declined("invalid-value", None, Some("/initial_state".to_owned()))
                }
                Ok(ReplayVerdict::Undefined(step)) => {
                    let UndefinedStep {
                        counterexample,
                        over,
                    } = *step;
                    let Some(at) = undefined_where(&counterexample) else {
                        return fault();
                    };
                    match run.refute(over, counterexample, None, cancel) {
                        Ok(refuted) => {
                            EngineReport::FiniteReplay(FiniteReplayReport::UndefinedStep {
                                refuted: Box::new(refuted),
                                at,
                            })
                        }
                        Err(error) => report_fault(error),
                    }
                }
                Ok(ReplayVerdict::Claimed(claimed)) => {
                    let Claimed {
                        deciding,
                        evaluation,
                        steps,
                        counterexample,
                        over,
                    } = *claimed;
                    let refuted = match counterexample {
                        Some(counterexample) => {
                            let required = match subject.required() {
                                Ok(required) => required,
                                Err(error) => return report_fault(error),
                            };
                            let node = deciding.and_then(|index| required.get(index)).copied();
                            match run.refute(over, counterexample, node, cancel) {
                                Ok(refuted) => Some(refuted),
                                Err(error) => return report_fault(error),
                            }
                        }
                        None => None,
                    };
                    EngineReport::FiniteReplay(FiniteReplayReport::Claim(Box::new(ClaimReport {
                        steps: u64::try_from(steps).unwrap_or(u64::MAX),
                        evaluation,
                        refuted,
                    })))
                }
                Err(error) => report_fault(error),
            }
        },
    )
}
