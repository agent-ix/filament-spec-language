// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, first phase): [`check_model`] decides a safety item
//! over every behaviour of a model subject: a reachable invariant (TP-1), a
//! bounded MLTL clause (TP-2), an infinite-trace safety clause (TP-3), or the
//! deadlock-freedom item.
//!
//! Each item gets the deterministic bad-prefix monitor of its formula
//! (ADR-018 SM-6), built lazily in `fsl-eval`'s automaton kernel over the
//! clause's translation (`always holds(not deadlocked)` for the
//! deadlock-freedom item), and the engine explores the product of the
//! subject's state graph with it breadth-first in FR-101's canonical order:
//! initial states by ascending state-key bytes, successors by
//! transition-identity bytes then post-state key bytes. A product state is
//! (model state, monitor state of each instance), keyed by the model state's
//! FR-101 key and the monitor states' graph-local numbers. The translation
//! materializes its states only as the monitor reaches them, and the run's
//! automaton-state count is those states; both they and the monitor's own
//! states are bounded by `max_automaton_states` (ADR-018 IV-6).
//!
//! A clause's letter is read when its product state is created, for each
//! instance of the clause's `over` binding: every atom of the clause at every
//! position the claim reads (ADR-018 UE-1, FR-125). A TP-2 `on origin` claim
//! reads positions 0 to its horizon `h`, and its later positions evaluate no
//! atom; every other claim reads every position. The first product state in breadth-first
//! order whose letter is undefined, whose monitor state rejects, whose
//! expansion evaluates a contract conjunction undefined, whose bounded
//! closure rejects, or (for the deadlock-freedom item) that is deadlocked
//! ends the run with the canonical path to it (ADR-018 UE-4, DL-4, DL-7).
//! Once a bad state is created no later state is, and the states before it
//! are still expanded in order, so an earlier expansion-time violation wins.
//!
//! At a terminal model state a TP-2 clause closes its monitor by the bounded
//! profile's closed-boundary rule: the following positions read every atom
//! false (FSpec FR-091), until the monitor rejects or repeats. A TP-1 or TP-3
//! clause instead takes one terminal stutter edge, re-reading the terminal
//! letter (FR-125); its counterexample is the model prefix to the terminal
//! state, whose behaviour is that prefix stuttered forever.
//!
//! A run proves an item only when its frontier empties with no state bad:
//! every reachable product state was examined. Its `Holds` carries the
//! closure certificate the core checks before the proof settles (ADR-018
//! PC-3, FR-338): for a TP-1 clause under infinite-trace and the
//! deadlock-freedom item, every explored model state with the canonical key,
//! under each instance's context, of the carried root; for a bounded clause
//! `eventually[0,u] holds(A)` or `always[0,u] holds(A)` on origin under a
//! bounded or the infinite-trace profile, or `on each` (guarded or not) over a
//! model subject under a bounded profile (TP-2), every explored model state
//! with the key of the bounded monitor state it was read to, which the run
//! follows beside its own monitor's. A run that reaches its
//! horizon with a state left unexpanded settles V-5; a run that reaches a
//! limit or is cancelled settles V-7; neither is ever `Holds`.
//!
//! A protocol subject's steps that do not count (ADR-027 PB-3) leave interval
//! distance where it was, so a cycle of them can keep an interval eventuality
//! open for ever. The run records those steps between the product states that
//! owe an eventuality; after a frontier that empties, a cycle fair under the
//! scheduler constraints of FR-212 and the clause's own is a behaviour that
//! never meets the interval, and the run settles with its lasso. A cycle the
//! scheduler constraints exclude, because a thread can still move, is no
//! behaviour and the run proves.
//!
//! An infinite-trace TP-3 clause on origin over a model subject (past and
//! interval operators included) is keyed by the viable elementary
//! configurations of its bad-prefix monitor state (FR-338 `["safety",
//! branches]`): every explored model state with the key of the monitor state
//! it was read to. A TP-3 clause over a protocol subject, or any other
//! shape with no canonical translation, carries an empty closure certificate,
//! which the core checker cannot admit, so it never settles `proved`. A
//! liveness clause (TP-4) is decided by `liveness`,
//! which adds the second phase; an `on each` activation with a `when`
//! condition is read through [`ModelLetters::activation`] at each position
//! the clause reads. A clause with no instance is `Undecided(NoInstance)`
//! before any state is explored (FR-125).

#[cfg(test)]
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(test)]
use std::sync::Arc;
use std::sync::Mutex;

use filament_exact::{Cancel, Incomplete, Meter};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::automaton::{
    Atom, AutomatonError, AutomatonKind, BoundedState, BoundedTranslation, Gba, GbaBuilder, Guard,
    KeyRefusal, Letter, Monitor, SafetyAutomaton, SafetyState, StateId, SubjectKind, closure_shape,
    invariant_atom, is_safety_translated, translate, translate_counted,
};
use fsl_eval::fairness::{EdgeLabel, Vertex};
use fsl_eval::model_subject::{
    AtomStop, ModelInstance, ModelLetters, ModelSubject, ProtocolSubject,
};
use fsl_eval::simulation::StateKey;
use fsl_eval::simulation::model::{
    ClauseOutcome, ModelFinding, ModelSimState, ModelSystem, TerminalClass, check_model_bounds,
};
use fsl_eval::simulation::protocol::{
    Capacities, DataRace, MemoryModel, PackageProtocolProgram, ProtocolFairnessError,
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, Sc, SubjectMemory,
    UnboundCapacity, WeakAccessShape, activates_on_each, check_package_protocol_bounds,
    check_protocol_bounds, live_instance_bound,
};
use fsl_eval::simulation::{
    ConstraintValue, Cut, EncodingRefusal, Expansion, ExpansionLimit, ExpansionStop, FairLoop,
    Location, NotSimulated, Permutation, RequiresBound, SubjectSystem, SubjectTransition,
    SymmetryGroup, TransitionSystem, ZeroProgress, ZeroProgressError,
};
use fsl_eval::temporal::{AtomReading, CaptureReading};
use fsl_package::TemporalClockSelection;
use fsl_replay::undefined_finding;
use fsl_replay::{
    AppliedReduction, ClauseRootRefusal, ClosureCertificate, CounterexampleKind, IncompleteCause,
    Instance, InstanceBound, ItemRefusal, Lasso, MemoryBound, MemoryBoundMember, MemoryCause,
    ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ModelStep,
    ProductStateRef, ProofCertificate, Proviso, Race, ReachedLimit, ReductionCause, RunStats,
    SymmetryRefusal, TemporalCounterexample, Translation, UndecidedCause, UndefinedEvaluation,
    UndefinedLocus, UndefinedSite,
};
use fsl_semantics::check::{Activation, CaptureSite, FormulaNodeId, TemporalProfile};
use fsl_semantics::family::ClassifyFailure;
use fsl_semantics::model::observation::AdmissionFailure;

use super::ample::{Atoms, Por};
use super::concretise::{Lift, LiftStop};
use super::fair_walk::FairModel;
use super::false_extension::false_extension_rejects;
use super::reduce::{self, InstanceGroup, TableForm};
use fsl_route::reductions::Reductions;

pub(super) use fsl_eval::simulation::BOUND_POSITIONS;

/// One EN-1 run (FR-126 Inputs).
pub struct ModelCheckRequest<'a> {
    /// The model subject.
    pub subject: ModelSubject<'a>,
    /// Number of concurrent state expansions; results still commit in
    /// breadth-first canonical order.
    pub workers: usize,
    /// Explicit clock configurations selected for this subject's checked
    /// temporal declarations.
    pub temporal_clock_selections: &'a [TemporalClockSelection],
    /// The item.
    pub item: ModelCheckItem<'a>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
    /// The reductions the request selects (FR-159).
    pub reductions: Reductions,
    /// FR-315's limits for encoding this item's SMT queries.
    pub smt_encoding: fsl_replay::SmtEncodingLimits,
    /// FR-315's limits for running this item's SMT queries.
    pub smt_run: fsl_replay::SmtRunLimits,
}

/// The identity contribution of one EN-1 request's SMT limits.
///
/// Encoding bounds and solver-step bounds change the work a request can
/// complete, so they are identity-bearing. Solver wall-clock time is
/// nondeterministic and is deliberately omitted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModelCheckRequestIdentity {
    /// The caller's query-encoding bounds.
    pub smt_encoding: fsl_replay::SmtEncodingLimits,
    /// The deterministic solver-step bound.
    pub max_solver_steps: u64,
}

impl<'a> ModelCheckRequest<'a> {
    /// The identity-bearing SMT limits of this request (FR-315, FR-331).
    pub fn identity(&self) -> ModelCheckRequestIdentity {
        ModelCheckRequestIdentity {
            smt_encoding: self.smt_encoding,
            max_solver_steps: self.smt_run.max_solver_steps,
        }
    }

    /// The closure-certificate request for this run, carrying its exact
    /// encoding limits forward to FR-338's checker.
    pub fn certificate_request(&self) -> fsl_replay::CertificateRequest<'a> {
        fsl_replay::CertificateRequest {
            subject: ModelSubject {
                simulation: self.subject.simulation.clone(),
                proof_bounds: self.subject.proof_bounds.clone(),
            },
            temporal_clock_selections: self.temporal_clock_selections,
            item: self.item,
            smt_encoding: self.smt_encoding,
        }
    }
}

/// The model system `request` runs over. The derived deadlock-freedom item
/// classifies the terminal states of the identities its subject reads alone
/// (FR-124-AC-12); every other item reads every selection.
fn read_system<'a>(request: &ModelCheckRequest<'a>) -> Result<ModelSystem<'a>, AdmissionFailure> {
    let simulation = request.subject.simulation.clone();
    if matches!(request.item, ModelCheckItem::DeadlockFreedom) {
        ModelSystem::new_over_read_identities(simulation)
    } else {
        ModelSystem::new(simulation)
    }
}
/// Why [`check_model`] explored nothing (FR-126).
#[derive(Debug, thiserror::Error)]
pub enum ModelCheckRefusal {
    /// FR-126: the requested worker count is zero.
    #[error("invalid_runtime_input/invalid-value: model-check workers must be nonzero")]
    Workers,
    /// FR-106's admission of the subject's `ModelSystem` failed.
    #[error("the subject's model system did not admit: {0:?}")]
    Admission(Box<AdmissionFailure>),
    /// FR-336: a root of the subject is unbounded under its universes. The
    /// refusal names each unbounded root's `NodeKey` (as the key of
    /// [`RequiresBound::domains`]) and is returned before any state is
    /// explored. A `ProofBound` never makes a root finite here.
    #[error("the subject has an unbounded root and requires a bound")]
    RequiresBound(RequiresBound),
    /// FSpec FR-785: the protocol subject's capacity binding leaves these
    /// capacity names unbound; requires-bound, before any state is explored.
    #[error("the protocol subject leaves a symbolic capacity unbound and requires a bound")]
    UnboundCapacity(Vec<UnboundCapacity>),
    /// FR-101's pre-check could not classify the subject's extent.
    #[error("the subject's extent could not be classified: {0:?}")]
    Extent(Box<ClassifyFailure>),
    /// The subject's protocol system does not build (FR-205).
    #[error(transparent)]
    Protocol(ProtocolRefusal),
    /// The race-freedom item (FR-224) over a model subject, which has no
    /// memory model and so no data race.
    #[error("the race-freedom item is decided over a protocol subject only")]
    RaceOverModel,
    /// FR-951's invariant item over a protocol subject: an invariant item
    /// is decided over a model subject.
    #[error("the invariant item is decided over a model subject only")]
    InvariantOverProtocol,
    /// FR-730's loop-proof item over a model subject.
    #[error("a loop-proof item is decided over a protocol subject only")]
    LoopProofOverModel,
    /// The refinement's concrete subject is a protocol, whose refinement
    /// check is not built yet (FR-142, FR-145).
    #[error("a refinement over a concrete protocol is not checked yet")]
    ProtocolRefinement,
    /// The clause's property form is not one this engine decides yet: a
    /// liveness clause (TP-4), a refinement-form clause (ADR-020 decides a
    /// refinement as its own item), or a clause with no property form.
    #[error("EN-1 does not decide the property form {form:?} yet")]
    FormNotDecided {
        /// The clause's property form.
        form: Option<PropertyForm>,
    },
    /// A `holds` condition of the clause reads a capture (FR-337 "Capture
    /// observation"): FR-338's automaton-state keys carry no capture vector, so
    /// the engine returns no verdict (the standing decision of QSL-1092) and
    /// explores no state.
    #[error("a `holds` condition of clause `{clause}` reads the capture `{capture}`")]
    CaptureRead {
        /// The clause's name.
        clause: String,
        /// The first capture, in source order, that a `holds` condition reads.
        capture: String,
    },
    /// The clause has no property automaton: a past operator over a future
    /// one, or more atoms than a letter holds.
    #[error("the clause has no property automaton: {0}")]
    Translation(AutomatonError),
    /// A `holds` evaluation was refused, or completed no Boolean with none
    /// undefined, so the position has no letter (QSL-1039): for example an
    /// authored population bound exceeded (FR-380).
    #[error("a `holds` evaluation was refused: {0:?}")]
    AtomRefused(ClauseOutcome),
    /// A state the check reached cannot be observed, so its `holds` atoms
    /// have no reading (QSL-1039).
    #[error("a state cannot be observed: {0:?}")]
    StateUnobservable(ExpansionStop),
    /// A state or transition identity has no canonical encoding.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
    /// An automaton state key has no canonical encoding within the identity
    /// byte budget.
    #[error(transparent)]
    Key(#[from] KeyRefusal),
    /// The item has no canonical translation.
    #[error(transparent)]
    Item(ItemRefusal),
    /// A liveness clause this engine does not decide yet, or whose proof it
    /// cannot certify yet.
    #[error(transparent)]
    Liveness(#[from] super::liveness::LivenessRefusal),
    /// A hyper clause EN-1 does not decide, or whose subjects the request
    /// does not supply (FR-176).
    #[error(transparent)]
    Hyper(#[from] super::hyper::HyperRefusal),
    /// FR-151: a symmetry declaration is malformed.
    #[error(transparent)]
    Symmetry(SymmetryRefusal),
    /// FR-158 "Resolution": the request's constraint names no checked `at
    /// current` state clause of the package
    /// (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: no state constraint {name}")]
    UnknownConstraint {
        /// The request's constraint name.
        name: String,
    },
    /// A liveness clause over a protocol subject with an `ra` `parallel`,
    /// whose visibility fairness (FR-228) the liveness engine does not read
    /// yet (`unsupported_construct`/`not-yet-implemented`).
    #[error(
        "unsupported_construct/not-yet-implemented: EN-1 does not decide a liveness clause over an `ra` `parallel` yet"
    )]
    WeakLiveness,
    /// FR-155: partial-order reduction needs a footprint of an atom or a
    /// state constraint that reads through a form FSpec FR-383 gives no
    /// rule for.
    #[error(transparent)]
    Footprint(#[from] fsl_eval::simulation::model::FootprintError),
    /// A refinement whose abstract side is a protocol clause: the engine
    /// decides no abstract protocol step yet (FR-148)
    /// (`unsupported_construct/not-yet-implemented`).
    #[error(
        "unsupported_construct/not-yet-implemented: EN-1 does not decide a refinement over an abstract protocol subject yet"
    )]
    ProtocolAbstractSide,
    /// An invariant of the engine or the subject broke.
    #[error("model check broke invariant {0:?}")]
    Fault(InternalFault),
}

/// The item as the run reads it.
enum Item<'a> {
    Clause {
        /// Each instance of the `over` binding, in FR-125's canonical
        /// instance order.
        bindings: Vec<ModelInstance>,
        letters: &'a dyn ModelLetters,
        fairness: Vec<FairnessConstraint>,
        /// Authored targets over protocol identities.
        protocol_fairness: Vec<fsl_semantics::check::CheckedProtocolFairness>,
        /// The `holds` atom each letter atom reads: atom `i` is `atoms[i]`.
        atoms: Vec<FormulaNodeId>,
        /// The atom true where a guarded `on each` clause's `when`
        /// condition holds.
        when: Option<Atom>,
        /// Where the clause fixes its captures (FR-337 "Capture
        /// observation").
        captured: CaptureSite,
        /// The atom true at every represented position of a bounded `on
        /// each` clause.
        live: Option<Atom>,
        /// The atom true where the step into a position does not count for
        /// interval distance (ADR-027 PB-3).
        uncounted: Option<Atom>,
        /// How the subject's steps count.
        counting: Counting,
        /// Whether a terminal state closes the monitor by the bounded
        /// profile's closed-boundary rule (TP-2) rather than stuttering.
        bounded: bool,
        /// The last position the claim reads: the horizon `h` of a TP-2 `on
        /// origin` clause, none when it reads every position (UE-1).
        last_read: Option<u64>,
        /// Each instance's canonical bounded translation, whose states the
        /// run follows beside the monitor's to key a TP-2 proof's closure
        /// certificate (FR-338); empty for every other run.
        canonical: Vec<BoundedTranslation>,
    },
    Deadlock,
    /// FR-224's derived race-freedom item: `always holds(not raced)`, read
    /// from the step into each position.
    Race,
    /// FR-951's invariant item: `always holds(I)`, `I` the named
    /// `invariant` state clause over every conforming object of a state.
    Invariant {
        /// The invariant's declared name.
        name: &'a str,
    },
    /// FR-730's repeat-invariant obligation.
    LoopInvariant {
        /// The checked repeat node.
        repeat: fsl_semantics::check::ProtocolNodeId,
    },
    /// FR-730's repeat-termination obligation.
    LoopTermination {
        /// The checked repeat node.
        repeat: fsl_semantics::check::ProtocolNodeId,
    },
}

impl Item<'_> {
    /// The item's authored fairness set: empty for the derived deadlock-
    /// and race-freedom items.
    fn fairness(&self) -> &[FairnessConstraint] {
        match self {
            Self::Clause { fairness, .. } => fairness,
            Self::Deadlock
            | Self::Race
            | Self::Invariant { .. }
            | Self::LoopInvariant { .. }
            | Self::LoopTermination { .. } => &[],
        }
    }

    /// The item's authored protocol-node targets.
    fn protocol_fairness(&self) -> &[fsl_semantics::check::CheckedProtocolFairness] {
        match self {
            Self::Clause {
                protocol_fairness, ..
            } => protocol_fairness,
            Self::Deadlock
            | Self::Race
            | Self::Invariant { .. }
            | Self::LoopInvariant { .. }
            | Self::LoopTermination { .. } => &[],
        }
    }

    /// The number of instances, each with its own monitor state.
    fn instance_count(&self) -> usize {
        match self {
            Self::Clause { bindings, .. } => bindings.len(),
            Self::Deadlock
            | Self::Race
            | Self::Invariant { .. }
            | Self::LoopInvariant { .. }
            | Self::LoopTermination { .. } => 1,
        }
    }
}

/// One product state: its model state, key, depth, the step that first
/// reached it (`None` for the terminal stutter step), and the monitor state
/// of each instance after reading this position. An instance whose atom is
/// undefined there keeps the state it read the position from.
struct Node<State, Transition> {
    state: State,
    key: fsl_eval::simulation::StateKey,
    state_key: StateKey,
    depth: usize,
    /// The counted steps from the initial state (ADR-027 PB-3): the depth
    /// when every step counts.
    distance: usize,
    parent: Option<(usize, Option<Transition>)>,
    initial_index: usize,
    monitor: Vec<StateId>,
    /// The permutation the canonicaliser mapped the reached state to
    /// `state` with (FR-152): for an initial state the state the subject
    /// lists, for any other the successor its parent's step reached. The
    /// identity under the trivial group.
    permutation: Permutation,
    /// Whether the state constraint is false at `state` (FR-158): a
    /// boundary state is stored and read, never expanded.
    boundary: bool,
}

/// A product state's key: the model state's key bytes, the monitor state of
/// each instance and, where it decides which later positions the claim
/// reads, the counted distance (see [`Run::product`]).
type ProductKey = (Vec<u8>, Vec<StateId>, usize, Vec<BoundedState>);

/// Why the run ended before its frontier emptied.
enum Finish {
    Violated(Box<Violation>),
    Undecided(UndecidedCause),
    Stopped(IncompleteCause, Option<ReachedLimit>),
    Memory(MemoryCause),
}

/// A violation: its kind, the node it is at, and the `over` binding the
/// counterexample names.
struct Violation {
    kind: CounterexampleKind,
    at: usize,
    binding: Option<ModelInstance>,
}

impl Finish {
    fn violated(kind: CounterexampleKind, at: usize, binding: Option<ModelInstance>) -> Self {
        Self::Violated(Box::new(Violation { kind, at, binding }))
    }
}

type Step<T> = Result<T, Halt>;

/// What reading a position found, and the instance that found it.
type Finding = (CounterexampleKind, usize);

/// Each instance's monitor state and canonical bounded state after reading a
/// position, and what the read found.
type Reading = (Vec<StateId>, Vec<BoundedState>, Option<Finding>);

/// One successor of an expansion: the transition identity's order bytes, the
/// successor's state-key bytes and state key, the transition (none for the
/// terminal stutter), the canonical state and the permutation that
/// canonicalised it.
type Successor<S> = (
    Vec<u8>,
    Vec<u8>,
    StateKey,
    Option<<S as TransitionSystem>::TransitionId>,
    <S as TransitionSystem>::State,
    Permutation,
);

/// An ended run, or a refusal discovered after exploration started.
enum Halt {
    Finish(Finish),
    Refusal(ModelCheckRefusal),
}

impl From<Finish> for Halt {
    fn from(end: Finish) -> Self {
        Self::Finish(end)
    }
}

impl From<ModelCheckRefusal> for Halt {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<EncodingRefusal> for Halt {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(ModelCheckRefusal::Encoding(refusal))
    }
}

impl From<ProtocolFairnessError> for ModelCheckRefusal {
    fn from(error: ProtocolFairnessError) -> Self {
        match error {
            ProtocolFairnessError::UnknownOperation { name } => {
                super::liveness::LivenessRefusal::UnknownOperation { name }.into()
            }
            ProtocolFairnessError::UnknownProtocolTarget { .. } => {
                fault("s3-protocol-fairness-target-remains-resolved")
            }
            ProtocolFairnessError::Encoding(_) => fault("a-fairness-member-encodes"),
        }
    }
}

/// What a `holds` atom that gave no reading refuses the check with
/// (QSL-1039); the exhausted meter, which stops the check with a limit
/// instead, is returned as the `Err`.
pub(super) fn atom_refusal(stop: AtomStop) -> Result<ModelCheckRefusal, Incomplete> {
    match stop {
        AtomStop::Incomplete(incomplete) => Err(incomplete),
        AtomStop::Refused(outcome) => Ok(ModelCheckRefusal::AtomRefused(outcome)),
        AtomStop::Unobservable(stop) => Ok(ModelCheckRefusal::StateUnobservable(stop)),
        AtomStop::Fault(fault) => Ok(ModelCheckRefusal::Fault(fault)),
    }
}

pub(super) fn fault(invariant: &'static str) -> ModelCheckRefusal {
    ModelCheckRefusal::Fault(InternalFault::new("model-check", invariant))
}

pub(super) fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// The item, the automaton its monitor reads, and whether the run's proof
/// has a canonical closure certificate.
type Prepared<'a> = (Item<'a>, Box<dyn SafetyAutomaton>, bool);

/// How a subject's steps count for interval distance.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Counting {
    /// Every step counts: a model subject (ADR-018).
    Every,
    /// A step counts by its kind: a protocol subject (ADR-027 PB-3).
    ByKind,
}

/// FR-337 "Capture observation": refuse a clause whose `holds` condition
/// reads a capture, before any state is explored or charged.
pub(super) fn refuse_capture_read(item: ModelCheckItem<'_>) -> Result<(), ModelCheckRefusal> {
    let ModelCheckItem::Clause { clause, letters } = item else {
        return Ok(());
    };
    let Some(index) = letters.captured_read() else {
        return Ok(());
    };
    let capture = clause
        .captures()
        .nth(index)
        .map(|(name, _)| name.to_owned())
        .ok_or_else(|| fault("a-read-capture-is-declared"))?;
    Err(ModelCheckRefusal::CaptureRead {
        clause: clause.name().to_owned(),
        capture,
    })
}

/// FR-126: decide `request`'s item over every behaviour of its subject,
/// polling `cancel` before every expansion and every atom evaluation.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when the subject does not admit, a root is
/// unbounded, the item is a guarded `on each` clause or has no
/// property automaton, or an identity cannot be encoded; no outcome is
/// settled then.
pub fn check_model(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    check_model_with_stutter_closure(request, cancel, false)
}

/// Decide one model item, optionally adding the unfair self-loop at every
/// state required by FR-380's liveness premise form.
pub(crate) fn check_model_with_stutter_closure(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
    stutter_closure: bool,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    if request.workers == 0 {
        return Err(ModelCheckRefusal::Workers);
    }
    if let ModelCheckItem::Clause { clause, .. } = request.item {
        if clause.property_form() == Some(PropertyForm::Liveness) {
            // FR-159 (a) reads the clause alone, so it settles before the
            // liveness engine reads its fairness set.
            if !request.reductions.is_empty() {
                let form =
                    TableForm::of(&request.item).ok_or(ModelCheckRefusal::FormNotDecided {
                        form: clause.property_form(),
                    })?;
                if let Some(reduction) = reduce::not_preserving(&request.reductions, form) {
                    return Ok(settled(ReductionCause::NotPreserving {
                        reduction,
                        form: PropertyForm::Liveness,
                    }));
                }
            }
            return super::liveness::check(request, cancel, stutter_closure);
        }
    }
    if !request.reductions.is_empty() {
        return first_refuted_instance(request.limits, |cap, limits, listed| {
            check_reduced(&request, cancel, cap, limits, listed)
        });
    }
    first_refuted_instance(request.limits, |cap, limits, listed| {
        check_safety(&request, cancel, cap, limits, listed)
    })
}

/// FR-125: the reported instance of a refuted item is the first refuted
/// instance in canonical order. A search reports the earliest refutation it
/// reaches, which may belong to a later instance; then the item is searched
/// again over the instances before it (`search` takes the number of instances
/// to keep, the limits left, and lists the instances it ran over), until none
/// of them is refuted. The re-searches are charged to the same budget as the
/// first search, and their work is counted in the outcome's stats. A
/// re-search that ends undecided (a limit, a horizon, a memory bound) settles
/// the item with that typed outcome: the later instance is never reported as
/// the first refuted one.
fn first_refuted_instance(
    mut limits: ModelCheckLimits,
    mut search: impl FnMut(
        Option<usize>,
        ModelCheckLimits,
        &mut Vec<ModelInstance>,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal>,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let mut cap = None;
    let mut kept: Option<ModelCheckOutcome> = None;
    loop {
        let mut listed = Vec::new();
        let outcome = search(cap, limits, &mut listed)?;
        let position = match &outcome {
            ModelCheckOutcome::Violated { counterexample, .. } => counterexample
                .over_binding
                .as_ref()
                .and_then(|found| listed.iter().position(|binding| binding == found)),
            ModelCheckOutcome::Holds { .. }
            | ModelCheckOutcome::HoldsReduced { .. }
            | ModelCheckOutcome::Reduction { .. }
            | ModelCheckOutcome::Memory { .. }
            | ModelCheckOutcome::RecoveryView { .. }
            | ModelCheckOutcome::RefutedIgnoringConstraints { .. }
            | ModelCheckOutcome::BoundReached { .. }
            | ModelCheckOutcome::Undecided { .. }
            | ModelCheckOutcome::Stopped { .. } => None,
        };
        if let (Some(position), ModelCheckOutcome::Violated { stats, .. }) = (position, &outcome) {
            if position > 0 {
                limits.limits.max_states = limits
                    .limits
                    .max_states
                    .saturating_sub(usize::try_from(stats.product_states).unwrap_or(usize::MAX));
                limits.limits.max_transitions = limits
                    .limits
                    .max_transitions
                    .saturating_sub(usize::try_from(stats.product_edges).unwrap_or(usize::MAX));
                cap = Some(position);
                kept = Some(outcome);
                continue;
            }
        }
        let Some(earlier) = kept else {
            return Ok(outcome);
        };
        let work = outcome.stats();
        return Ok(match outcome {
            // The instances before the kept one were searched to the end and
            // none is refuted: the kept refutation is the first.
            ModelCheckOutcome::Holds { .. } | ModelCheckOutcome::HoldsReduced { .. } => {
                restat(earlier, |stats| sum(stats, work))
            }
            // A refutation of an earlier instance replaces the kept one.
            ModelCheckOutcome::Violated { .. } => {
                let kept_work = earlier.stats();
                restat(outcome, |stats| sum(stats, kept_work))
            }
            // Any other end leaves the earlier instances undecided.
            ModelCheckOutcome::Reduction { .. }
            | ModelCheckOutcome::Memory { .. }
            | ModelCheckOutcome::RecoveryView { .. }
            | ModelCheckOutcome::RefutedIgnoringConstraints { .. }
            | ModelCheckOutcome::BoundReached { .. }
            | ModelCheckOutcome::Undecided { .. }
            | ModelCheckOutcome::Stopped { .. } => {
                let kept_work = earlier.stats();
                restat(outcome, |stats| sum(stats, kept_work))
            }
        });
    }
}

/// `outcome` with its stats replaced by `change` of them.
fn restat(
    outcome: ModelCheckOutcome,
    change: impl FnOnce(RunStats) -> RunStats,
) -> ModelCheckOutcome {
    match outcome {
        ModelCheckOutcome::Holds { certificate, stats } => ModelCheckOutcome::Holds {
            certificate,
            stats: change(stats),
        },
        ModelCheckOutcome::HoldsReduced { reductions, stats } => ModelCheckOutcome::HoldsReduced {
            reductions,
            stats: change(stats),
        },
        ModelCheckOutcome::Reduction { cause, stats } => ModelCheckOutcome::Reduction {
            cause,
            stats: change(stats),
        },
        ModelCheckOutcome::Memory { cause, stats } => ModelCheckOutcome::Memory {
            cause,
            stats: change(stats),
        },
        ModelCheckOutcome::RecoveryView { predicate, stats } => ModelCheckOutcome::RecoveryView {
            predicate,
            stats: change(stats),
        },
        ModelCheckOutcome::Violated {
            counterexample,
            stats,
        } => ModelCheckOutcome::Violated {
            counterexample,
            stats: change(stats),
        },
        ModelCheckOutcome::RefutedIgnoringConstraints { stats } => {
            ModelCheckOutcome::RefutedIgnoringConstraints {
                stats: change(stats),
            }
        }
        ModelCheckOutcome::BoundReached { depth, stats } => ModelCheckOutcome::BoundReached {
            depth,
            stats: change(stats),
        },
        ModelCheckOutcome::Undecided { cause, stats } => ModelCheckOutcome::Undecided {
            cause,
            stats: change(stats),
        },
        ModelCheckOutcome::Stopped {
            cause,
            limit,
            stats,
        } => ModelCheckOutcome::Stopped {
            cause,
            limit,
            stats: change(stats),
        },
    }
}

/// One search of a safety item over the first `cap` instances (all when
/// `None`), within `limits`; `listed` receives the instances it ran over.
fn check_safety(
    request: &ModelCheckRequest<'_>,
    cancel: &Cancel,
    cap: Option<usize>,
    limits: ModelCheckLimits,
    listed: &mut Vec<ModelInstance>,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let max_automaton_states = request.limits.max_automaton_states;
    // The form is refused before admission reads any snapshot.
    let (mut item, automaton, certified) =
        prepare(request.item, max_automaton_states, Counting::Every)?;
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system =
        read_system(request).map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    // A loop-proof item is decided over a protocol subject only; a stated
    // subject kind below must not instance it over a model.
    if matches!(
        request.item,
        ModelCheckItem::LoopInvariant { .. } | ModelCheckItem::LoopTermination { .. }
    ) {
        return Err(ModelCheckRefusal::Item(ItemRefusal::LoopProofOverModel));
    }
    let (mut instances, root_cancelled) = if certified {
        match request.item.instances_for(
            &request.subject,
            &system,
            request.temporal_clock_selections,
            cancel,
            SubjectKind::Ordinary,
            max_automaton_states,
        ) {
            Ok(instances) => (instances, None),
            Err(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(cause))) => {
                (Vec::new(), Some(cause))
            }
            Err(ItemRefusal::Unbounded(refusal)) => return Err(not_simulated(refusal)),
            Err(refusal) => return Err(ModelCheckRefusal::Item(refusal)),
        }
    } else {
        (Vec::new(), None)
    };
    if let (
        Item::Clause {
            bindings,
            canonical,
            ..
        },
        ModelCheckItem::Clause { clause, .. },
    ) = (&mut item, request.item)
    {
        if certified {
            *canonical = bounded_translations(&instances);
        }
        // FR-125: the instances come from the subject's universes; a
        // certified proof's are those its certificate's translations carry.
        *bindings = if certified {
            instances
                .iter()
                .filter_map(|instance| instance.binding.clone())
                .collect()
        } else {
            system.instances(&clause.over().1).map_err(not_simulated)?
        };
        listed.clone_from(bindings);
        if let Some(cap) = cap {
            bindings.truncate(cap);
            canonical.truncate(cap);
        }
    }
    refuse_capture_read(request.item)?;
    if let Some(cap) = cap {
        instances.truncate(cap);
    }
    let mut run = Run {
        system: &system,
        fairness: item.fairness(),
        reducer: Some(&system),
        item: &item,
        workers: request.workers,
        limits,
        max_depth: request.max_depth,
        meter,
        max_candidates,
        cancel,
        nodes: Vec::new(),
        canonical: Vec::new(),
        known: BTreeMap::new(),
        uncounted_edges: Vec::new(),
        enabled: BTreeMap::new(),
        queue: VecDeque::new(),
        edges: 0,
        stutter_edges: 0,
        horizon_cut: false,
        first_bad: None,
        deferred_local: BTreeMap::new(),
        shared_names: BTreeSet::new(),
        monitor: Monitor::bad_prefix(automaton, max_automaton_states)
            .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
        group: SymmetryGroup::default(),
        constraint: None,
        boundary: 0,
        chooser: None,
        memory_bound: None,
        bound_limited: BTreeSet::new(),
        message_bound: None,
        message_bound_limited: BTreeSet::new(),
        instance_bound: None,
        instance_limited: BTreeSet::new(),
    };
    if let Some(source) = root_cancelled {
        return Ok(ModelCheckOutcome::Stopped {
            cause: IncompleteCause::Cancelled { source },
            limit: None,
            stats: run.stats(),
        });
    }
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: run.stats(),
        });
    }
    if item.instance_count() == 0 {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats: run.stats(),
        });
    }
    run.decide(&instances)
}

/// FR-215 "Model checking a protocol subject" (ADR-027 TS-6): one EN-1 run
/// over a protocol subject.
pub struct ProtocolCheckRequest<'a> {
    /// The protocol subject.
    pub subject: ProtocolSubject<'a>,
    /// Number of concurrent state expansions, merged in canonical BFS order.
    pub workers: usize,
    /// Explicit clock configurations selected for this subject's checked
    /// temporal declarations.
    pub temporal_clock_selections: &'a [TemporalClockSelection],
    /// The item: a safety clause over the subject's model, or the
    /// deadlock-freedom item, whose deadlocks are FR-211's.
    pub item: ModelCheckItem<'a>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The method bound `max_store_buffer` (FR-225, ADR-025 MB-2): the
    /// entries per thread's buffer under `tso`; `None` runs with 4.
    pub max_store_buffer: Option<u64>,
    /// The method bound `max_messages` (FR-225, ADR-025 MB-2): the live
    /// messages per location under `ra`; `None` runs with 4.
    pub max_messages: Option<u64>,
    /// Whether protocol liveness may diagnose lassos that violate memory
    /// fairness, without treating them as ordinary counterexamples.
    pub ignore_memory_constraints: bool,
    /// The method bound `max_live_instances` (FR-209, ADR-027 AE-4): the live
    /// instances of an `activation on each` protocol; `None` runs with 3. It
    /// is outside the subject and its obligation identity.
    pub max_live_instances: Option<u64>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
}

/// FR-215 (ADR-027 TS-5, TS-6): decide `request`'s item over every
/// behaviour of its protocol subject with [`check_model`]'s engine, over
/// the subject's protocol system under its resolved memory models
/// (FR-219): `ProtocolSystem<Sc>`, or `ProtocolSystem<Tso>` bounded by
/// `max_store_buffer` when a `parallel` resolves to `tso` (FR-221, FR-225),
/// or `ProtocolSystem<Ra>` bounded by `max_messages` when one resolves to
/// `ra` (FR-222).
/// The requires-bound pre-check reads the model's roots and the protocol's
/// before any state is explored, and EN-1's access-shape pre-check settles
/// `WeakAccessShape` before any state is explored (FR-220). A run that left
/// a store unexpanded under the bound and found no counterexample settles
/// `MemoryBoundReached` (ADR-025 MB-4). A deadlock counterexample names the
/// waiting threads of its last state (ADR-027 PD-4). A liveness clause is
/// decided by the liveness engine's two phases, its fairness filter reading
/// the clause's fairness set over the subject, scheduler constraints
/// included (FR-212).
///
/// # Errors
///
/// [`ModelCheckRefusal`] as [`check_model`] returns it,
/// and [`ModelCheckRefusal::Protocol`] when the protocol system does not
/// build.
pub fn check_protocol(
    request: ProtocolCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
    check_protocol_constrained(request, None, cancel)
}

/// [`check_protocol`] under the state constraint `constraint` (FR-158, FSL
/// FR-225-AC-4): a state where the constraint is false is stored and read but
/// never expanded, and a run that reached one settles `ConstraintReached`
/// before any memory bound it also reached, for a liveness clause as for a
/// safety item.
///
/// # Errors
///
/// As [`check_protocol`], and [`ModelCheckRefusal::UnknownConstraint`] when the
/// package declares no `at current` state clause named `constraint`.
pub fn check_protocol_constrained(
    request: ProtocolCheckRequest<'_>,
    constraint: Option<&str>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
    if request.workers == 0 {
        return Err(ModelCheckRefusal::Workers);
    }
    if matches!(request.item, ModelCheckItem::Invariant { .. }) {
        return Err(ModelCheckRefusal::InvariantOverProtocol);
    }
    if let ModelCheckItem::Clause { clause, .. } = request.item {
        if clause.property_form() == Some(PropertyForm::Liveness) {
            return check_protocol_liveness(request, constraint, cancel);
        }
    }
    let max_automaton_states = request.limits.max_automaton_states;
    let (mut item, automaton, certified) =
        prepare(request.item, max_automaton_states, Counting::ByKind)?;
    let subject = &request.subject;
    let simulation = subject.model.simulation.clone();
    let meter = simulation.meter;
    let max_candidates = simulation.expansion.max_candidates;
    let no_initial = simulation.initial.is_empty();
    let model = ModelSystem::new(simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    if let Some(name) = constraint {
        if !model.has_constraint(name) {
            return Err(ModelCheckRefusal::UnknownConstraint {
                name: name.to_owned(),
            });
        }
    }
    let definition = subject
        .model
        .simulation
        .input
        .protocol_definition(&subject.protocol)
        .ok_or_else(|| {
            ModelCheckRefusal::Protocol(ProtocolRefusal::UnknownProtocol {
                protocol: subject.protocol.clone(),
            })
        })?;
    let source_protocol = match &definition {
        fsl_eval::simulation::model::ProtocolDefinition::Source(protocol) => Some(*protocol),
        fsl_eval::simulation::model::ProtocolDefinition::Package(_) => None,
    };
    let capacities = Capacities::admit_input(
        &subject.model.simulation.input,
        &subject.protocol,
        &subject.capacities,
    )
    .map_err(ModelCheckRefusal::Protocol)?;
    if let Some(protocol) = source_protocol {
        check_protocol_bounds(
            &model,
            protocol,
            &capacities,
            subject.model.simulation.input.evaluator,
            BOUND_POSITIONS,
        )
        .map_err(not_simulated)?;
    } else if let fsl_eval::simulation::model::ProtocolDefinition::Package(form) = definition {
        let program = PackageProtocolProgram::with_memory(
            &subject.model.simulation.input,
            *form,
            subject.memory.clone(),
        )
        .map_err(|refusal| ModelCheckRefusal::Protocol(refusal.into_protocol()))?;
        check_package_protocol_bounds(&model, &program, &capacities, BOUND_POSITIONS)
            .map_err(not_simulated)?;
    }
    let live_instances = live_instance_bound(request.max_live_instances);
    let instance_bound = source_protocol
        .is_some_and(activates_on_each)
        .then_some(InstanceBound {
            value: live_instances,
        });
    let (instances, root_cancelled) = if certified {
        let instances = request.item.instances_for(
            &subject.model,
            &model,
            request.temporal_clock_selections,
            cancel,
            SubjectKind::Protocol,
            max_automaton_states,
        );
        match instances {
            Ok(instances) => (instances, None),
            Err(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(cause))) => {
                (Vec::new(), Some(cause))
            }
            Err(ItemRefusal::Unbounded(refusal)) => return Err(not_simulated(refusal)),
            Err(refusal) => return Err(ModelCheckRefusal::Item(refusal)),
        }
    } else {
        (Vec::new(), None)
    };
    if let (
        Item::Clause {
            bindings,
            canonical,
            ..
        },
        ModelCheckItem::Clause { clause, .. },
    ) = (&mut item, request.item)
    {
        if certified {
            *canonical = bounded_translations(&instances);
        }
        *bindings = if certified {
            instances
                .iter()
                .filter_map(|instance| instance.binding.clone())
                .collect()
        } else {
            model.instances(&clause.over().1).map_err(not_simulated)?
        };
    }
    refuse_capture_read(request.item)?;
    let protocol_run = ProtocolRun {
        item: &item,
        instances: &instances,
        root_cancelled,
        no_initial,
        limits: request.limits,
        workers: request.workers,
        max_depth: request.max_depth,
        meter,
        max_candidates,
        cancel,
        max_automaton_states,
        instance_bound,
        constraint,
    };
    let memory = SubjectMemory::of(
        &subject.memory,
        request.max_store_buffer,
        request.max_messages,
    );
    let built =
        match memory {
            SubjectMemory::Sc => {
                new_protocol_system(model, subject, Sc, protocol_run.loop_proof_target()).map(
                    |system| {
                        let bound = system.has_channels().then_some(MemoryBound {
                            member: MemoryBoundMember::MaxMessages,
                            value: request
                                .max_messages
                                .unwrap_or(fsl_eval::simulation::protocol::DEFAULT_MEMORY_BOUND),
                        });
                        protocol_run.decide(
                            &system
                                .with_channel_message_bound(request.max_messages.unwrap_or(
                                    fsl_eval::simulation::protocol::DEFAULT_MEMORY_BOUND,
                                ))
                                .with_max_live_instances(live_instances)
                                .with_cycle_keys(BTreeSet::new()),
                            automaton,
                            bound,
                        )
                    },
                )
            }
            SubjectMemory::Tso(tso) => {
                let bound = MemoryBound {
                    member: MemoryBoundMember::MaxStoreBuffer,
                    value: tso.max_store_buffer(),
                };
                new_protocol_system(model, subject, tso, protocol_run.loop_proof_target()).map(
                    |system| {
                        protocol_run.decide(
                            &system
                                .with_channel_message_bound(request.max_messages.unwrap_or(
                                    fsl_eval::simulation::protocol::DEFAULT_MEMORY_BOUND,
                                ))
                                .with_max_live_instances(live_instances)
                                .with_cycle_keys(BTreeSet::new()),
                            automaton,
                            Some(bound),
                        )
                    },
                )
            }
            SubjectMemory::Ra(ra) => {
                let bound = MemoryBound {
                    member: MemoryBoundMember::MaxMessages,
                    value: ra.max_messages(),
                };
                let channel_bound = ra.max_messages();
                new_protocol_system(model, subject, ra, protocol_run.loop_proof_target()).map(
                    |system| {
                        protocol_run.decide(
                            &system
                                .with_channel_message_bound(channel_bound)
                                .with_max_live_instances(live_instances)
                                .with_cycle_keys(BTreeSet::new()),
                            automaton,
                            Some(bound),
                        )
                    },
                )
            }
        };
    built.unwrap_or_else(|refusal| construction_refusal(refusal, &meter, max_candidates))
}

fn new_protocol_system<'a, M: MemoryModel>(
    model: ModelSystem<'a>,
    subject: &'a ProtocolSubject<'a>,
    memory: M,
    loop_proof_target: Option<fsl_eval::simulation::protocol::ProtocolLoopProofTarget>,
) -> Result<ProtocolSystem<'a, M>, ProtocolRefusal> {
    match subject
        .model
        .simulation
        .input
        .protocol_definition(&subject.protocol)
    {
        Some(fsl_eval::simulation::model::ProtocolDefinition::Source(_)) => {
            ProtocolSystem::new_with_loop_proof(model, subject, memory, loop_proof_target)
        }
        Some(fsl_eval::simulation::model::ProtocolDefinition::Package(_)) => {
            ProtocolSystem::new_package_with_loop_proof(model, subject, memory, loop_proof_target)
        }
        None => Err(ProtocolRefusal::UnknownProtocol {
            protocol: subject.protocol.clone(),
        }),
    }
}

/// FR-126 and FR-212 for a liveness clause over a protocol subject
/// ([`super::liveness::check_protocol`]), over `ProtocolSystem<Sc>`,
/// `ProtocolSystem<Tso>` bounded by `max_store_buffer` with flush fairness,
/// or `ProtocolSystem<Ra>` bounded by `max_messages` with visibility fairness
/// in its fairness set (FR-228).
fn check_protocol_liveness(
    request: ProtocolCheckRequest<'_>,
    constraint: Option<&str>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
    let subject = &request.subject;
    let model = ModelSystem::new(subject.model.simulation.clone())
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    if let Some(name) = constraint {
        if !model.has_constraint(name) {
            return Err(ModelCheckRefusal::UnknownConstraint {
                name: name.to_owned(),
            });
        }
    }
    let definition = subject
        .model
        .simulation
        .input
        .protocol_definition(&subject.protocol)
        .ok_or_else(|| {
            ModelCheckRefusal::Protocol(ProtocolRefusal::UnknownProtocol {
                protocol: subject.protocol.clone(),
            })
        })?;
    let source_protocol = match &definition {
        fsl_eval::simulation::model::ProtocolDefinition::Source(protocol) => Some(*protocol),
        fsl_eval::simulation::model::ProtocolDefinition::Package(_) => None,
    };
    let capacities = Capacities::admit_input(
        &subject.model.simulation.input,
        &subject.protocol,
        &subject.capacities,
    )
    .map_err(ModelCheckRefusal::Protocol)?;
    if let Some(protocol) = source_protocol {
        check_protocol_bounds(
            &model,
            protocol,
            &capacities,
            subject.model.simulation.input.evaluator,
            BOUND_POSITIONS,
        )
        .map_err(not_simulated)?;
    } else if let fsl_eval::simulation::model::ProtocolDefinition::Package(form) = definition {
        let program = PackageProtocolProgram::with_memory(
            &subject.model.simulation.input,
            *form,
            subject.memory.clone(),
        )
        .map_err(|refusal| ModelCheckRefusal::Protocol(refusal.into_protocol()))?;
        check_package_protocol_bounds(&model, &program, &capacities, BOUND_POSITIONS)
            .map_err(not_simulated)?;
    }
    let memory = SubjectMemory::of(
        &subject.memory,
        request.max_store_buffer,
        request.max_messages,
    );
    let live_instances = live_instance_bound(request.max_live_instances);
    let instance_bound = source_protocol
        .is_some_and(activates_on_each)
        .then_some(InstanceBound {
            value: live_instances,
        });
    let channel_bound = request
        .max_messages
        .unwrap_or(fsl_eval::simulation::protocol::DEFAULT_MEMORY_BOUND);
    let decided = match memory {
        SubjectMemory::Ra(ra) => {
            let bound = MemoryBound {
                member: MemoryBoundMember::MaxMessages,
                value: ra.max_messages(),
            };
            new_protocol_system(model, subject, ra, None).map(|system| {
                super::liveness::check_protocol(
                    subject,
                    &system
                        .with_channel_message_bound(channel_bound)
                        .with_max_live_instances(live_instances)
                        .with_cycle_keys(BTreeSet::new()),
                    Some(bound),
                    request.item,
                    request.temporal_clock_selections,
                    request.max_depth,
                    super::liveness::ProtocolLivenessOptions {
                        workers: request.workers,
                        limits: request.limits,
                        ignore_memory_constraints: request.ignore_memory_constraints,
                        instance_bound,
                        constraint,
                    },
                    cancel,
                )
            })
        }
        SubjectMemory::Tso(tso) => {
            let bound = MemoryBound {
                member: MemoryBoundMember::MaxStoreBuffer,
                value: tso.max_store_buffer(),
            };
            new_protocol_system(model, subject, tso, None).map(|system| {
                super::liveness::check_protocol(
                    subject,
                    &system
                        .with_channel_message_bound(channel_bound)
                        .with_max_live_instances(live_instances)
                        .with_cycle_keys(BTreeSet::new()),
                    Some(bound),
                    request.item,
                    request.temporal_clock_selections,
                    request.max_depth,
                    super::liveness::ProtocolLivenessOptions {
                        workers: request.workers,
                        limits: request.limits,
                        ignore_memory_constraints: request.ignore_memory_constraints,
                        instance_bound,
                        constraint,
                    },
                    cancel,
                )
            })
        }
        SubjectMemory::Sc => new_protocol_system(model, subject, Sc, None).map(|system| {
            super::liveness::check_protocol(
                subject,
                &system
                    .with_channel_message_bound(channel_bound)
                    .with_max_live_instances(live_instances)
                    .with_cycle_keys(BTreeSet::new()),
                None,
                request.item,
                request.temporal_clock_selections,
                request.max_depth,
                super::liveness::ProtocolLivenessOptions {
                    workers: request.workers,
                    limits: request.limits,
                    ignore_memory_constraints: request.ignore_memory_constraints,
                    instance_bound,
                    constraint,
                },
                cancel,
            )
        }),
    };
    decided.unwrap_or_else(|refusal| {
        construction_refusal(
            refusal,
            &subject.model.simulation.meter,
            subject.model.simulation.expansion.max_candidates,
        )
    })
}

/// What a refusal of `ProtocolSystem::new` settles, before any state is
/// explored. FR-220 "Access shape" and ADR-027 ST-12 settle V-8; a stop on
/// a budget, while EN-1's classification expands the attempts at the
/// initial state or while an initial state settles, is an FR-126 expansion
/// stop and settles `Stopped` naming the limit and its configured value
/// ([`expansion_stopped`]); every other refusal refuses.
fn construction_refusal(
    refusal: ProtocolRefusal,
    meter: &filament_exact::ScalarLimits,
    max_candidates: u64,
) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
    match refusal {
        ProtocolRefusal::WeakAccessShape(shape) => Ok(ModelCheckOutcome::Memory {
            cause: MemoryCause::WeakAccessShape(shape),
            stats: RunStats::default(),
        }),
        ProtocolRefusal::InitialStop(stop) => expansion_stopped(stop, meter, max_candidates)
            .map(|(cause, limit)| ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats: RunStats::default(),
            })
            .ok_or(ModelCheckRefusal::Protocol(refusal)),
        refusal => Err(ModelCheckRefusal::Protocol(refusal)),
    }
}

/// What [`check_protocol`] runs over its protocol system, whichever memory
/// model the subject's resolved models need.
struct ProtocolRun<'r> {
    item: &'r Item<'r>,
    instances: &'r [Instance],
    root_cancelled: Option<filament_exact::CancelCause>,
    no_initial: bool,
    limits: ModelCheckLimits,
    workers: usize,
    max_depth: Option<usize>,
    meter: filament_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    max_automaton_states: u64,
    /// The instance bound of an `activation on each` protocol.
    instance_bound: Option<InstanceBound>,
    /// The request's state constraint (FR-158).
    constraint: Option<&'r str>,
}

impl ProtocolRun<'_> {
    fn loop_proof_target(&self) -> Option<fsl_eval::simulation::protocol::ProtocolLoopProofTarget> {
        use fsl_eval::simulation::protocol::{
            ProtocolLoopProofKind as Kind, ProtocolLoopProofTarget,
        };
        match self.item {
            Item::LoopInvariant { repeat } => Some(ProtocolLoopProofTarget {
                repeat: *repeat,
                kind: Kind::Invariant,
            }),
            Item::LoopTermination { repeat } => Some(ProtocolLoopProofTarget {
                repeat: *repeat,
                kind: Kind::Termination,
            }),
            _ => None,
        }
    }

    /// The run over `system`, under the memory bound `bound` when its memory
    /// model has one (ADR-025 MB-3, MB-4), and the state constraint when the
    /// request names one.
    fn decide<M>(
        &self,
        system: &ProtocolSystem<'_, M>,
        automaton: Box<dyn SafetyAutomaton>,
        bound: Option<MemoryBound>,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal>
    where
        M: MemoryModel + Sync,
        M::Component: Send + Sync,
    {
        // FR-430 adds scheduler constraints to infinite-trace clauses, not
        // to the derived deadlock-freedom item.
        let fairness = match self.item {
            Item::Deadlock
            | Item::Race
            | Item::Invariant { .. }
            | Item::LoopInvariant { .. }
            | Item::LoopTermination { .. } => Vec::new(),
            Item::Clause { .. } => system
                .fairness_set_with_targets(self.item.fairness(), self.item.protocol_fairness())?,
        };
        let mut run = Run {
            system,
            fairness: &fairness,
            reducer: self
                .constraint
                .map(|_| system as &dyn Reducer<ProtocolState<M>>),
            item: self.item,
            workers: self.workers,
            limits: self.limits,
            max_depth: self.max_depth,
            meter: self.meter,
            max_candidates: self.max_candidates,
            cancel: self.cancel,
            nodes: Vec::new(),
            canonical: Vec::new(),
            known: BTreeMap::new(),
            uncounted_edges: Vec::new(),
            enabled: BTreeMap::new(),
            queue: VecDeque::new(),
            edges: 0,
            stutter_edges: 0,
            horizon_cut: false,
            first_bad: None,
            deferred_local: BTreeMap::new(),
            shared_names: BTreeSet::new(),
            monitor: Monitor::bad_prefix(automaton, self.max_automaton_states)
                .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
            group: SymmetryGroup::default(),
            constraint: self.constraint,
            boundary: 0,
            chooser: None,
            memory_bound: bound,
            bound_limited: BTreeSet::new(),
            message_bound: system.channel_message_bound().map(|value| MemoryBound {
                member: MemoryBoundMember::MaxMessages,
                value,
            }),
            message_bound_limited: BTreeSet::new(),
            instance_bound: self.instance_bound,
            instance_limited: BTreeSet::new(),
        };
        if let Some(source) = self.root_cancelled {
            return Ok(ModelCheckOutcome::Stopped {
                cause: IncompleteCause::Cancelled { source },
                limit: None,
                stats: run.stats(),
            });
        }
        if self.no_initial {
            return Ok(ModelCheckOutcome::Undecided {
                cause: UndecidedCause::NoInitialState,
                stats: run.stats(),
            });
        }
        if self.item.instance_count() == 0 {
            return Ok(ModelCheckOutcome::Undecided {
                cause: UndecidedCause::NoInstance,
                stats: run.stats(),
            });
        }
        run.decide(self.instances)
    }
}

/// FR-126: the `Stopped` cause and limit an expansion stop ends a run
/// with, naming the limit it reached and that limit's configured value (the
/// counter's bound in `meter`, or `max_candidates`); `None` for a stop that
/// broke an invariant.
fn expansion_stopped(
    stop: ExpansionStop,
    meter: &filament_exact::ScalarLimits,
    max_candidates: u64,
) -> Option<(IncompleteCause, Option<ReachedLimit>)> {
    let limit = match stop.limit {
        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter { counter },
            value: fsl_replay::meter_bound(meter, counter),
        },
        Some(ExpansionLimit::MaxCandidates) => ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: max_candidates,
        },
        None if stop.is_internal_failure() => return None,
        None => return Some((IncompleteCause::ResourceExhausted, None)),
    };
    Some((IncompleteCause::ResourceExhausted, Some(limit)))
}

/// An outcome the reduction pre-check settles, with no state explored.
fn settled(cause: ReductionCause) -> ModelCheckOutcome {
    ModelCheckOutcome::Reduction {
        cause,
        stats: RunStats::default(),
    }
}

/// One product a reduced item runs: the `over` bindings it reads, the group
/// it is reduced by, and the keys its one bound instance stands for.
struct Plan {
    bindings: Vec<ModelInstance>,
    group: SymmetryGroup,
    orbit: Vec<String>,
}

/// The counts of `left` and `right` together.
fn sum(left: RunStats, right: RunStats) -> RunStats {
    RunStats {
        product_states: left.product_states.saturating_add(right.product_states),
        product_edges: left.product_edges.saturating_add(right.product_edges),
        stutter_edges: left.stutter_edges.saturating_add(right.stutter_edges),
        automaton_states: left.automaton_states.saturating_add(right.automaton_states),
        depth: left.depth.max(right.depth),
        bound_limited: left.bound_limited.saturating_add(right.bound_limited),
        message_bound_limited: left
            .message_bound_limited
            .saturating_add(right.message_bound_limited),
        instance_limited: left.instance_limited.saturating_add(right.instance_limited),
    }
}

/// FR-159, FR-151, FR-158 and FR-162 over a safety item: the pre-check,
/// then one product per [`Plan`]: per orbit of the `over` binding under
/// symmetry, else one. The first run that does not complete its method
/// settles the item; otherwise boundary states settle `ConstraintReached`
/// (before `BoundReached`), a horizon cut `BoundReached`, and a completed
/// search `HoldsReduced` (ADR-021 RV-1, RV-4, RV-5).
fn check_reduced(
    request: &ModelCheckRequest<'_>,
    cancel: &Cancel,
    cap: Option<usize>,
    limits: ModelCheckLimits,
    listed: &mut Vec<ModelInstance>,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let reductions = &request.reductions;
    let max_automaton_states = request.limits.max_automaton_states;
    // The form is refused before admission reads any snapshot.
    prepare(request.item, max_automaton_states, Counting::Every)?;
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system =
        read_system(request).map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    let group = match precheck(reductions, &request.item, &system)? {
        Admitted::Group(group) => group,
        Admitted::Settled(outcome) => return Ok(outcome),
    };
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: RunStats::default(),
        });
    }
    let symmetric = !reductions.symmetry.is_empty();
    let plans = match request.item {
        ModelCheckItem::Clause { clause, .. } => {
            let mut bindings = system.instances(&clause.over().1).map_err(not_simulated)?;
            listed.clone_from(&bindings);
            if let Some(cap) = cap {
                bindings.truncate(cap);
            }
            if bindings.is_empty() {
                return Ok(ModelCheckOutcome::Undecided {
                    cause: UndecidedCause::NoInstance,
                    stats: RunStats::default(),
                });
            }
            if symmetric {
                reduce::instance_groups(Some(&bindings), &group)
                    .into_iter()
                    .map(|instance: InstanceGroup| Plan {
                        bindings: instance.representative.into_iter().collect(),
                        group: instance.group,
                        orbit: instance.orbit,
                    })
                    .collect()
            } else {
                vec![Plan {
                    bindings,
                    group: SymmetryGroup::default(),
                    orbit: Vec::new(),
                }]
            }
        }
        ModelCheckItem::DeadlockFreedom | ModelCheckItem::Invariant { .. } => vec![Plan {
            bindings: Vec::new(),
            group: group.clone(),
            orbit: Vec::new(),
        }],
        ModelCheckItem::RaceFreedom => return Err(ModelCheckRefusal::RaceOverModel),
        ModelCheckItem::LoopInvariant { .. } | ModelCheckItem::LoopTermination { .. } => {
            return Err(ModelCheckRefusal::LoopProofOverModel);
        }
    };
    refuse_capture_read(request.item)?;
    let mut total = RunStats::default();
    let mut boundary: usize = 0;
    let mut horizon = false;
    let mut applied = Vec::new();
    for plan in plans {
        let (mut item, automaton, _) =
            prepare(request.item, max_automaton_states, Counting::Every)?;
        if let Item::Clause { bindings, .. } = &mut item {
            *bindings = plan.bindings;
        }
        let ample = if reductions.partial_order {
            Some(ample_of(&system, &item, reductions.constraint.as_deref())?)
        } else {
            None
        };
        let mut run = Run {
            system: &system,
            fairness: item.fairness(),
            reducer: Some(&system),
            item: &item,
            workers: request.workers,
            limits,
            max_depth: request.max_depth,
            meter,
            max_candidates,
            cancel,
            nodes: Vec::new(),
            canonical: Vec::new(),
            known: BTreeMap::new(),
            uncounted_edges: Vec::new(),
            enabled: BTreeMap::new(),
            queue: VecDeque::new(),
            edges: 0,
            stutter_edges: 0,
            horizon_cut: false,
            first_bad: None,
            deferred_local: BTreeMap::new(),
            shared_names: BTreeSet::new(),
            monitor: Monitor::bad_prefix(automaton, max_automaton_states)
                .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
            group: plan.group.clone(),
            constraint: reductions.constraint.as_deref(),
            boundary: 0,
            chooser: ample.as_ref().map(|ample| ample as &dyn Chooser<_, _>),
            memory_bound: None,
            bound_limited: BTreeSet::new(),
            message_bound: None,
            message_bound_limited: BTreeSet::new(),
            instance_bound: None,
            instance_limited: BTreeSet::new(),
        };
        let explored = run.explore();
        total = sum(total, run.stats());
        match explored {
            Ok(()) => {
                boundary = boundary.saturating_add(run.boundary);
                horizon |= run.horizon_cut;
                if symmetric {
                    applied.push(AppliedReduction::Symmetry {
                        groups: group.classes.clone(),
                        orbit: plan.orbit,
                        stabiliser: plan.group.classes,
                    });
                }
            }
            Err(Halt::Refusal(refusal)) => return Err(refusal),
            Err(Halt::Finish(Finish::Undecided(cause))) => {
                return Ok(ModelCheckOutcome::Undecided {
                    cause,
                    stats: total,
                });
            }
            Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                return Ok(ModelCheckOutcome::Stopped {
                    cause,
                    limit,
                    stats: total,
                });
            }
            Err(Halt::Finish(Finish::Memory(cause))) => {
                return Ok(ModelCheckOutcome::Memory {
                    cause,
                    stats: total,
                });
            }
            Err(Halt::Finish(Finish::Violated(violation))) => {
                let Violation { kind, at, binding } = *violation;
                // FR-161 "Length and limits": a stop while concretising
                // settles V-7.
                return match run.model_counterexample(kind, at, binding) {
                    Ok(counterexample) => Ok(ModelCheckOutcome::Violated {
                        counterexample: Box::new(counterexample),
                        stats: total,
                    }),
                    Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                        Ok(ModelCheckOutcome::Stopped {
                            cause,
                            limit,
                            stats: total,
                        })
                    }
                    Err(Halt::Finish(Finish::Undecided(cause))) => {
                        Ok(ModelCheckOutcome::Undecided {
                            cause,
                            stats: total,
                        })
                    }
                    Err(Halt::Finish(Finish::Memory(cause))) => Ok(ModelCheckOutcome::Memory {
                        cause,
                        stats: total,
                    }),
                    Err(Halt::Finish(Finish::Violated(_))) => {
                        Err(fault("a-concretised-path-ends-once"))
                    }
                    Err(Halt::Refusal(refusal)) => Err(refusal),
                };
            }
        }
    }
    if reductions.partial_order {
        applied.push(AppliedReduction::PartialOrder {
            proviso: Proviso::BreadthFirstRevisit,
        });
    }
    let max_depth = widen(request.max_depth.unwrap_or(0));
    Ok(if boundary > 0 {
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached {
                boundary_states: widen(boundary),
            },
            stats: total,
        }
    } else if horizon && reductions.partial_order {
        // ADR-021 RV-5: a reduced graph does not keep short counterexamples
        // short, so a partial-order run to the horizon proves no bound.
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::Horizon { max_depth },
            stats: total,
        }
    } else if horizon {
        ModelCheckOutcome::BoundReached {
            depth: max_depth,
            stats: total,
        }
    } else {
        ModelCheckOutcome::HoldsReduced {
            reductions: applied,
            stats: total,
        }
    })
}

/// The ample-set choice and the enforcing expansion of a partial-order run
/// (FR-156, FR-155 "Read enforcement"), read by [`Run`] through [`Chooser`].
trait Chooser<State, Transition> {
    /// `state`'s expansion, holding every step to its footprint.
    fn expand(
        &self,
        state: &State,
    ) -> Result<Expansion<Transition, State, ModelFinding>, ExpansionStop>;

    /// Expand a BFS batch without sharing coordinator state. Implementors
    /// may parallelize their pure system expansion and return results in the
    /// supplied state order.
    fn expand_batch(
        &self,
        states: Vec<State>,
        workers: usize,
    ) -> Vec<Result<Expansion<Transition, State, ModelFinding>, ExpansionStop>>;

    /// The ample set among `enabled` at `state`, where `fresh` says whether
    /// every successor through a transition is a product state not yet
    /// discovered (C3).
    fn choose(
        &self,
        state: &State,
        enabled: &[Transition],
        fresh: &dyn Fn(&Transition) -> bool,
    ) -> Result<Vec<Transition>, ExpansionStop>;
}

/// Map one bounded BFS batch on scoped threads, returning outcomes in input
/// order even when a later state finishes first.
#[cfg(test)]
struct WorkerTestHook {
    delay_first_job: bool,
    cancel_after_merged: Option<usize>,
    cancelled: AtomicBool,
    active_workers: AtomicUsize,
    peak_workers: AtomicUsize,
    next_batch: AtomicUsize,
    completed: Mutex<BTreeMap<usize, Vec<usize>>>,
}

#[cfg(test)]
thread_local! {
    static WORKER_TEST_HOOK: RefCell<Option<Arc<WorkerTestHook>>> = const { RefCell::new(None) };
    static WORKER_TEST_FRONTIER: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static WORKER_TEST_MERGE: Cell<Option<usize>> = const { Cell::new(None) };
    static WORKER_TEST_STOP_MERGE: Cell<Option<usize>> = const { Cell::new(None) };
}

#[cfg(test)]
fn cancel_at_test_merge(cancel: &Cancel, merged: usize) {
    WORKER_TEST_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow().as_ref() {
            if hook.cancel_after_merged == Some(merged)
                && !hook.cancelled.swap(true, Ordering::Relaxed)
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
                cancel.cancel(filament_exact::CancelCause::Requested);
            }
        }
    });
}

#[cfg(test)]
fn take_worker_test_frontier() -> Vec<Vec<u8>> {
    WORKER_TEST_FRONTIER.with(|frontier| std::mem::take(&mut *frontier.borrow_mut()))
}

#[cfg(test)]
fn take_worker_test_stop_merge() -> Option<usize> {
    WORKER_TEST_STOP_MERGE.with(Cell::take)
}

fn parallel_map<State, Output>(
    states: Vec<State>,
    workers: usize,
    expand: impl Fn(&State) -> Output + Sync,
) -> Vec<Output>
where
    State: Send,
    Output: Send,
{
    if workers <= 1 || states.len() <= 1 {
        return states.iter().map(expand).collect();
    }
    #[cfg(test)]
    let hook = WORKER_TEST_HOOK.with(|slot| slot.borrow().clone());
    #[cfg(test)]
    let batch = hook
        .as_ref()
        .map(|hook| hook.next_batch.fetch_add(1, Ordering::Relaxed));
    let queue = states.into_iter().enumerate().collect::<VecDeque<_>>();
    let count = queue.len();
    let queue = Mutex::new(queue);
    let outputs = Mutex::new((0..count).map(|_| None).collect::<Vec<Option<Output>>>());
    let thread_count = workers.min(count);
    std::thread::scope(|scope| {
        let mut threads = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            let queue = &queue;
            let outputs = &outputs;
            let expand = &expand;
            #[cfg(test)]
            let hook = hook.clone();
            #[cfg(test)]
            let batch = batch;
            threads.push(scope.spawn(move || loop {
                    let Some((position, state)) = queue
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .pop_front()
                    else {
                        break;
                    };
                    #[cfg(test)]
                    if let (Some(hook), Some(batch)) = (&hook, batch) {
                        let active = hook.active_workers.fetch_add(1, Ordering::Relaxed) + 1;
                        hook.peak_workers.fetch_max(active, Ordering::Relaxed);
                        if hook.delay_first_job && position == 0 {
                            std::thread::sleep(std::time::Duration::from_millis(20));
                        }
                        let output = expand(&state);
                        outputs.lock().unwrap_or_else(std::sync::PoisonError::into_inner)[position] =
                            Some(output);
                        hook.completed
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .entry(batch)
                            .or_default()
                            .push(position);
                        hook.active_workers.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let output = expand(&state);
                    outputs.lock().unwrap_or_else(std::sync::PoisonError::into_inner)[position] =
                        Some(output);
                }));
        }
        for thread in threads {
            if let Err(panic) = thread.join() {
                std::panic::resume_unwind(panic);
            }
        }
    });
    outputs
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .into_iter()
        .map(|output| output.expect("every worker slot has one result"))
        .collect()
}

/// A model subject's ample sets: [`Por`] over its `ModelSystem`.
struct ModelAmple<'r, 'a> {
    system: &'r ModelSystem<'a>,
    por: Por<'r, 'a>,
}

impl Chooser<ModelSimState, ModelTransition> for ModelAmple<'_, '_> {
    fn expand(
        &self,
        state: &ModelSimState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
        self.system.successors_enforced(state)
    }

    fn expand_batch(
        &self,
        states: Vec<ModelSimState>,
        workers: usize,
    ) -> Vec<Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop>> {
        parallel_map(states, workers, |state| {
            self.system.successors_enforced(state)
        })
    }

    fn choose(
        &self,
        state: &ModelSimState,
        enabled: &[ModelTransition],
        fresh: &dyn Fn(&ModelTransition) -> bool,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let mut enabled = enabled.to_vec();
        enabled.sort();
        enabled.dedup();
        self.por.ample(state, &enabled, fresh)
    }
}

/// The inputs of ample-set choice for `item` over `system` under the state
/// constraint `constraint` (FR-156 "Atom footprints", "Visibility").
fn ample_of<'r, 'a>(
    system: &'r ModelSystem<'a>,
    item: &'r Item<'r>,
    constraint: Option<&str>,
) -> Result<ModelAmple<'r, 'a>, ModelCheckRefusal> {
    let por = match item {
        Item::Clause {
            bindings,
            letters,
            fairness,
            atoms,
            when,
            captured,
            ..
        } => Por::new(
            system,
            Some(Atoms {
                letters: *letters,
                nodes: atoms,
                guarded: when.is_some() || *captured != CaptureSite::Never,
            }),
            bindings,
            fairness,
            constraint,
        ),
        Item::Deadlock | Item::Race | Item::LoopInvariant { .. } | Item::LoopTermination { .. } => {
            Por::new(system, None, &[], &[], constraint)
        }
        // FR-951: the invariant's letter reads what its body reads, so
        // every step that writes a location it reads is visible.
        Item::Invariant { name } => Por::new(system, None, &[], &[], constraint).and_then(|por| {
            Ok(por.with_visible_reads(
                system
                    .constraint_reads(name)
                    .map_err(ModelCheckRefusal::Footprint)?,
            ))
        }),
    }?;
    Ok(ModelAmple { system, por })
}

/// What FR-159's pre-check admits: the run's group, or an outcome it
/// settles before any state is explored.
pub(super) enum Admitted {
    /// Every selected reduction admits; the group is trivial with no
    /// symmetry.
    Group(SymmetryGroup),
    /// The pre-check settled the item.
    Settled(ModelCheckOutcome),
}

/// FR-159's pre-check over `system`, in its order: (a) the preservation
/// table, (b) admission of the symmetry declarations and closure of the
/// initial states under their group, (c) the reductions the system offers,
/// (d) resolution of the state constraint.
///
/// # Errors
///
/// [`ModelCheckRefusal`] for a malformed declaration, an unknown
/// constraint, or an identity with no canonical encoding.
pub(super) fn precheck(
    reductions: &Reductions,
    item: &ModelCheckItem<'_>,
    system: &ModelSystem<'_>,
) -> Result<Admitted, ModelCheckRefusal> {
    let form = TableForm::of(item).ok_or_else(|| fault("a-decided-item-has-a-table-row"))?;
    if let Some(reduction) = reduce::not_preserving(reductions, form) {
        let form = match item {
            ModelCheckItem::Clause { clause, .. } => clause
                .property_form()
                .ok_or_else(|| fault("a-decided-clause-has-a-form"))?,
            ModelCheckItem::DeadlockFreedom
            | ModelCheckItem::RaceFreedom
            | ModelCheckItem::Invariant { .. }
            | ModelCheckItem::LoopInvariant { .. }
            | ModelCheckItem::LoopTermination { .. } => PropertyForm::ReachableInvariant,
        };
        return Ok(Admitted::Settled(settled(ReductionCause::NotPreserving {
            reduction,
            form,
        })));
    }
    let group = reduce::admit(system, &reductions.symmetry).map_err(ModelCheckRefusal::Symmetry)?;
    let closed = reduce::closure(system, &group).map_err(|refusal| match refusal {
        reduce::ClosureRefusal::Encoding(refusal) => ModelCheckRefusal::Encoding(refusal),
        reduce::ClosureRefusal::Unordered => fault("a-permuted-collection-is-ordered"),
    })?;
    if let Some(cause) = closed {
        return Ok(Admitted::Settled(settled(cause)));
    }
    if let Some(reduction) = reduce::unoffered(reductions, system) {
        return Ok(Admitted::Settled(settled(ReductionCause::Unsupported {
            reduction,
        })));
    }
    if let Some(name) = &reductions.constraint {
        if !system.has_constraint(name) {
            return Err(ModelCheckRefusal::UnknownConstraint { name: name.clone() });
        }
    }
    Ok(Admitted::Group(group))
}

/// FR-336 and FR-125: an unbounded root, or an admitting population with no
/// universe, is `RequiresBound`; a stopped classification is `Extent`.
pub(super) fn not_simulated(refusal: NotSimulated) -> ModelCheckRefusal {
    match refusal {
        NotSimulated::RequiresBound(roots) => ModelCheckRefusal::RequiresBound(roots),
        NotSimulated::UnboundCapacity(names) => ModelCheckRefusal::UnboundCapacity(names),
        NotSimulated::Extent(failure) => ModelCheckRefusal::Extent(Box::new(failure)),
        _ => fault("bounds-precheck-returns-only-requires-bound-or-extent"),
    }
}

/// Read the item: a TP-1, TP-2 or TP-3 clause's translation, or the
/// deadlock-freedom item's monitor of `always holds(not deadlocked)`.
fn prepare(
    item: ModelCheckItem<'_>,
    max_automaton_states: u64,
    counting: Counting,
) -> Result<Prepared<'_>, ModelCheckRefusal> {
    let ModelCheckItem::Clause { clause, letters } = item else {
        // The derived items read one atom, true at every position, so they
        // share an automaton.
        let derived = match (item, counting) {
            (ModelCheckItem::Invariant { clause }, _) => Item::Invariant { name: clause.name },
            (ModelCheckItem::LoopInvariant { repeat }, Counting::ByKind) => {
                Item::LoopInvariant { repeat }
            }
            (ModelCheckItem::LoopTermination { repeat }, Counting::ByKind) => {
                Item::LoopTermination { repeat }
            }
            (ModelCheckItem::RaceFreedom, Counting::Every) => {
                return Err(ModelCheckRefusal::RaceOverModel);
            }
            (ModelCheckItem::RaceFreedom, Counting::ByKind) => Item::Race,
            _ => Item::Deadlock,
        };
        let automaton = if matches!(
            item,
            ModelCheckItem::LoopInvariant { .. } | ModelCheckItem::LoopTermination { .. }
        ) {
            loop_proof_automaton()?
        } else {
            deadlock_automaton()?
        };
        return Ok((derived, Box::new(automaton), true));
    };
    let form = clause.property_form();
    let bounded = match form {
        Some(PropertyForm::ReachableInvariant | PropertyForm::Safety) => false,
        Some(PropertyForm::BoundedMltl) => true,
        Some(PropertyForm::Liveness | PropertyForm::Refinement { .. }) | None => {
            return Err(ModelCheckRefusal::FormNotDecided { form });
        }
    };
    let automaton = match counting {
        Counting::Every => translate(clause, max_automaton_states),
        Counting::ByKind => translate_counted(clause, max_automaton_states),
    }
    .map_err(ModelCheckRefusal::Translation)?;
    if automaton.kind() == AutomatonKind::Liveness {
        return Err(fault("a-safety-form-translates-to-a-safety-automaton"));
    }
    // A closure certificate keys an unguarded TP-1 translation, which reads
    // no `when` condition, so a guarded TP-1 clause's proof carries none. An
    // `on each` bounded operator keys the condition's reading into its
    // canonical state (FR-338 "Guarded `on each` activation"), but only a
    // model subject's steps all count, so a protocol run of one carries no
    // certificate.
    let guarded = matches!(clause.activation(), Activation::Each { guarded: true });
    let each = matches!(clause.activation(), Activation::Each { .. });
    // An infinite-trace TP-3 clause, `on origin` or `on each` guarded or not,
    // over a model or a protocol subject, is keyed by its monitor's viable
    // configurations (FR-338 `["safety", branches]`). A protocol subject's
    // steps that do not count are searched for a fair non-progress cycle
    // before the run settles `Holds` (`zero_progress`), so its certificate is
    // one the checker accepts only when that search finds none.
    let monitored = form == Some(PropertyForm::Safety)
        && invariant_atom(clause).is_none()
        && is_safety_translated(clause);
    let certified = (!guarded
        && form == Some(PropertyForm::ReachableInvariant)
        && clause.profile() == TemporalProfile::InfiniteTrace)
        || (closure_shape(clause).is_some() && (!each || counting == Counting::Every))
        || monitored;
    let last_read = match clause.activation() {
        Activation::Origin if bounded => clause.horizon(),
        Activation::Origin => closure_shape(clause).map(|(_, upper, _)| upper),
        Activation::Each { .. } => None,
    };
    let prepared = Item::Clause {
        bindings: Vec::new(),
        letters,
        fairness: clause.fairness().to_vec(),
        protocol_fairness: clause
            .protocol_fairness()
            .map(|(fairness, _)| fairness.clone())
            .collect(),
        atoms: automaton.atoms().to_vec(),
        when: automaton.when(),
        captured: clause.capture_site(),
        live: automaton.live(),
        uncounted: automaton.uncounted(),
        counting,
        bounded,
        last_read,
        canonical: Vec::new(),
    };
    Ok((prepared, Box::new(automaton), certified))
}

/// The canonical bounded translation of each instance, none for a run whose
/// instances translate otherwise.
fn bounded_translations(instances: &[Instance]) -> Vec<BoundedTranslation> {
    instances
        .iter()
        .filter_map(|instance| match &instance.translation {
            Translation::Bounded(translation) => Some(translation.clone()),
            Translation::Safety(_) | Translation::Monitored(_) | Translation::Liveness(_) => None,
        })
        .collect()
}

/// The automaton of `always a`, with `a` the letter's atom 0 (`not
/// deadlocked`): one state that reads `a` at every position, the initial
/// state included.
fn deadlock_automaton() -> Result<Gba, ModelCheckRefusal> {
    let holding = Guard::always().requiring(Atom::new(0).map_err(|_| fault("atom-0-exists"))?);
    let mut builder = GbaBuilder::new();
    builder
        .add_transition(Gba::INITIAL, holding, Gba::INITIAL)
        .map_err(|_| fault("the-initial-state-exists"))?;
    Ok(builder.build())
}

/// A loop-proof item's monitor imposes no additional state predicate: its
/// obligation is evaluated by the protocol system at repeat decisions.
fn loop_proof_automaton() -> Result<Gba, ModelCheckRefusal> {
    let mut builder = GbaBuilder::new();
    builder
        .add_transition(Gba::INITIAL, Guard::always(), Gba::INITIAL)
        .map_err(|_| fault("the-initial-state-exists"))?;
    Ok(builder.build())
}

/// FR-152 and FR-158 over a subject's states: what a reduced run reads of a
/// model subject. A protocol subject offers no reduction, so its run has no
/// reducer and a trivial group.
trait Reducer<State> {
    fn canonicalise(
        &self,
        state: &State,
        group: &SymmetryGroup,
    ) -> Result<(State, Permutation), ExpansionStop>;

    fn constraint_value(&self, state: &State, name: &str)
        -> Result<ConstraintValue, ExpansionStop>;
}

impl Reducer<ModelSimState> for ModelSystem<'_> {
    fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        ModelSystem::canonicalise(self, state, group)
    }

    fn constraint_value(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        ModelSystem::constraint_value(self, state, name)
    }
}

impl<M: MemoryModel> Reducer<ProtocolState<M>> for ProtocolSystem<'_, M> {
    // A protocol subject offers no symmetry reduction: its group is trivial and
    // a run never canonicalises under it.
    fn canonicalise(
        &self,
        state: &ProtocolState<M>,
        _group: &SymmetryGroup,
    ) -> Result<(ProtocolState<M>, Permutation), ExpansionStop> {
        Ok((state.clone(), Permutation::identity()))
    }

    fn constraint_value(
        &self,
        state: &ProtocolState<M>,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        self.model().constraint_value(state.model(), name)
    }
}

struct Run<'r, S: SubjectSystem> {
    system: &'r S,
    /// The item's fairness set over the subject, which every counterexample
    /// carries: empty for the deadlock-freedom item (FR-123, FR-212).
    fairness: &'r [<S::TransitionId as SubjectTransition>::Fairness],
    /// The system's reductions; `None` for a subject that offers none.
    reducer: Option<&'r dyn Reducer<S::State>>,
    item: &'r Item<'r>,
    /// Parallel expansion count; all results are committed by BFS position.
    workers: usize,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    meter: filament_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    nodes: Vec<Node<S::State, S::TransitionId>>,
    /// For each node, each instance's canonical bounded state after reading
    /// its position; empty unless the item has canonical translations.
    canonical: Vec<Vec<BoundedState>>,
    /// Each product state's index by its key: the model state's key bytes
    /// and the monitor state of each instance.
    known: BTreeMap<ProductKey, usize>,
    /// The product edges taken on a step that does not count for interval
    /// distance (ADR-027 PB-3), by node, kept only for a claim that reads
    /// the distinction.
    uncounted_edges: Vec<(usize, usize, S::TransitionId)>,
    /// The transitions each expanded node enables, kept for a claim that
    /// reads uncounted steps so the zero-progress search reads them from the
    /// charged expansion (FR-212). A reduced run (`chooser`) keeps none: its
    /// expansion is a subset, and the search expands those nodes in full.
    enabled: BTreeMap<usize, Vec<S::TransitionId>>,
    queue: VecDeque<usize>,
    edges: usize,
    /// The terminal stutter edges among `edges`.
    stutter_edges: usize,
    horizon_cut: bool,
    /// The first bad product state created and its violation; no state is
    /// created after it.
    first_bad: Option<(usize, Box<Violation>)>,
    /// Local ordered accesses that remain candidates until their names are
    /// shared or the reachable graph closes (FR-220).
    deferred_local: BTreeMap<(String, ModelTransition), DeferredLocalAccess>,
    /// Names that EN-1 classified shared in any reachable state (FR-220).
    shared_names: BTreeSet<Location>,
    monitor: Monitor<Box<dyn SafetyAutomaton>>,
    /// The group states are canonicalised under (FR-152); trivial with no
    /// symmetry.
    group: SymmetryGroup,
    /// The state constraint's clause name (FR-158).
    constraint: Option<&'r str>,
    /// The boundary states stored.
    boundary: usize,
    /// The ample-set choice when partial-order reduction is selected
    /// (FR-156); `None` for a subject that offers none.
    chooser: Option<&'r dyn Chooser<S::State, S::TransitionId>>,
    /// The memory bound the subject's memory model runs under (ADR-025
    /// MB-2); `None` for one with no bound.
    memory_bound: Option<MemoryBound>,
    /// The distinct model states, by state key, whose expansion cut a step at
    /// the memory bound (ADR-025 MB-3); a model state read under several
    /// monitor states counts once.
    bound_limited: BTreeSet<StateKey>,
    /// The protocol-channel bound and states where a send reached it.
    message_bound: Option<MemoryBound>,
    message_bound_limited: BTreeSet<StateKey>,
    /// The instance bound the subject's `activation on each` runs under
    /// (ADR-027 AE-4); `None` for a subject with none.
    instance_bound: Option<InstanceBound>,
    /// The distinct model states, by state key, left with an `activate` step
    /// unexpanded at the instance bound (ADR-027 AE-4); a model state read
    /// under several monitor states counts once.
    instance_limited: BTreeSet<StateKey>,
}

struct DeferredLocalAccess {
    names: BTreeSet<Location>,
    shape: Box<WeakAccessShape>,
}

impl<S> Run<'_, S>
where
    S: SubjectSystem + Sync,
    S::State: Send,
    S::TransitionId: Send,
{
    #[cfg(test)]
    fn snapshot_worker_frontier(&self, active: usize) {
        let frontier = std::iter::once(active)
            .chain(self.queue.iter().copied())
            .filter_map(|index| {
                self.nodes
                    .get(index)
                    .map(|node| node.key.as_bytes().to_vec())
            })
            .collect();
        WORKER_TEST_FRONTIER.with(|slot| *slot.borrow_mut() = frontier);
    }

    /// Expand a bounded BFS batch on scoped workers. The coordinator consumes
    /// these indexed outcomes in queue order, so expansion completion order
    /// cannot change a finding, refusal, or budget boundary.
    fn expand_batch(
        &self,
        positions: &[usize],
    ) -> Vec<(
        usize,
        Result<(Expansion<S::TransitionId, S::State, ModelFinding>, Cut), ExpansionStop>,
    )>
    where
        ModelFinding: Send,
    {
        let system = self.system;
        let chooser = self.chooser;
        let states: Vec<_> = positions
            .iter()
            .map(|&position| (position, self.nodes[position].state.clone()))
            .collect();
        let outcomes = match chooser {
            Some(chooser) => chooser
                .expand_batch(
                    states.iter().map(|(_, state)| state.clone()).collect(),
                    self.workers,
                )
                .into_iter()
                .map(|result| result.map(|expansion| (expansion, Cut::default())))
                .collect(),
            None => parallel_map(
                states.iter().map(|(_, state)| state.clone()).collect(),
                self.workers,
                |state| system.successors_bounded(state),
            ),
        };
        states
            .into_iter()
            .zip(outcomes)
            .map(|((position, _), result)| (position, result))
            .collect()
    }

    fn record_weak_access_classification(&mut self, findings: &[ModelFinding]) -> Step<()> {
        for finding in findings {
            match finding {
                ModelFinding::WeakAccessDeferred { names, shape } => {
                    let Some(transition) = shape.transition.clone() else {
                        return Err(fault("a-deferred-local-access-has-a-transition").into());
                    };
                    let candidate = self
                        .deferred_local
                        .entry((shape.site.clone(), transition))
                        .or_insert_with(|| DeferredLocalAccess {
                            names: BTreeSet::new(),
                            shape: shape.clone(),
                        });
                    candidate.names.extend(names.iter().cloned());
                }
                ModelFinding::WeakAccessShared { location } => {
                    self.shared_names.insert(location.clone());
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn has_unresolved_local_access(&self) -> bool {
        self.deferred_local.values().any(|candidate| {
            candidate.names.is_empty()
                || candidate
                    .names
                    .iter()
                    .any(|name| !self.shared_names.contains(name))
        })
    }

    fn deferred_local_shape(&self) -> Result<Option<WeakAccessShape>, ModelCheckRefusal> {
        let failures = self
            .deferred_local
            .values()
            .filter(|candidate| {
                candidate.names.is_empty()
                    || candidate
                        .names
                        .iter()
                        .any(|name| !self.shared_names.contains(name))
            })
            .flat_map(|candidate| candidate.shape.failures.iter().cloned())
            .collect();
        WeakAccessShape::from_failures(failures)
            .map_err(|_| fault("a-deferred-weak-access-shape-encodes"))
    }

    fn reachability_is_exhaustive(&self) -> bool {
        !self.horizon_cut
            && self.boundary == 0
            && self.bound_limited.is_empty()
            && self.message_bound_limited.is_empty()
            && self.instance_limited.is_empty()
    }

    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            stutter_edges: widen(self.stutter_edges),
            automaton_states: widen(self.monitor.automaton().state_count()),
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
            bound_limited: widen(self.bound_limited.len()),
            message_bound_limited: widen(self.message_bound_limited.len()),
            instance_limited: widen(self.instance_limited.len()),
        }
    }

    /// The closure certificate of a run that proved its item: for a TP-1
    /// clause or the deadlock-freedom item, every explored model state under
    /// each instance's carried root; for a bounded clause, every explored
    /// model state under the canonical bounded state it was read to; for a
    /// TP-3 clause, under the key of its monitor's viable configurations.
    /// Sorted by model state then key. With no instance translation (a clause
    /// of another shape) it is empty.
    fn certificate(&self, instances: &[Instance]) -> Result<ClosureCertificate, ModelCheckRefusal> {
        let mut states = Vec::with_capacity(self.nodes.len().saturating_mul(instances.len()));
        for (index, instance) in instances.iter().enumerate() {
            match &instance.translation {
                Translation::Safety(translation) => {
                    let key = translation.key(SafetyState::Carried)?;
                    states.extend(self.nodes.iter().map(|node| ProductStateRef {
                        model_state: node.key.clone(),
                        automaton_state: key.clone(),
                    }));
                }
                Translation::Monitored(translation) => {
                    for node in &self.nodes {
                        let state = node
                            .monitor
                            .get(index)
                            .ok_or_else(|| fault("a-monitored-node-has-a-monitor-state"))?;
                        let configurations = self
                            .monitor
                            .configurations(*state, translation.max_key_bytes())
                            .map_err(ModelCheckRefusal::Translation)?;
                        if configurations.is_empty() {
                            return Err(fault("a-proved-run-reaches-no-rejecting-state"));
                        }
                        states.push(ProductStateRef {
                            model_state: node.key.clone(),
                            automaton_state: translation.key(&configurations)?,
                        });
                    }
                }
                // The producer refuses liveness clauses before instancing; only
                // the closure checker builds this variant.
                Translation::Liveness(_) => {
                    return Err(fault("a-producer-run-has-no-liveness-translation"));
                }
                Translation::Bounded(translation) => {
                    for (node, canonical) in self.nodes.iter().zip(&self.canonical) {
                        let state = canonical
                            .get(index)
                            .ok_or_else(|| fault("a-bounded-node-has-a-canonical-state"))?;
                        if state.is_rejecting() {
                            return Err(fault("a-proved-run-reaches-no-rejecting-state"));
                        }
                        states.push(ProductStateRef {
                            model_state: node.key.clone(),
                            automaton_state: translation.key(*state)?,
                        });
                    }
                }
            }
        }
        states.sort();
        states.dedup();
        Ok(ClosureCertificate { states })
    }

    fn poll(&self) -> Result<(), Finish> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(Finish::Stopped(IncompleteCause::Cancelled { source }, None));
            }
        }
        Ok(())
    }

    fn stopped(limit: ModelCheckLimit, value: u64) -> Finish {
        Finish::Stopped(
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    }

    fn automaton(error: AutomatonError) -> Halt {
        match error {
            AutomatonError::StateLimit { limit } => {
                Self::stopped(ModelCheckLimit::MaxAutomatonStates, limit).into()
            }
            AutomatonError::AtomOutOfRange { .. }
            | AutomatonError::UnknownState { .. }
            | AutomatonError::NotSafetyAutomaton { .. }
            | AutomatonError::KeyEncodingLimit { .. }
            | AutomatonError::InvertedInterval { .. }
            | AutomatonError::PastOverFuture
            | AutomatonError::RepeatedPunctualInterval
            | AutomatonError::ClockLimit { .. }
            | AutomatonError::ProfileNotTranslated { .. }
            | AutomatonError::Invariant { .. } => fault("monitor-steps").into(),
        }
    }

    fn rejects(&self, state: StateId) -> Step<bool> {
        self.monitor.is_rejecting(state).map_err(Self::automaton)
    }

    /// An unreduced run's outcome: explore, then settle what it found. Under
    /// the trivial group the path to a violation is already concrete, so no
    /// expansion runs that could stop.
    fn decide(
        &mut self,
        instances: &[Instance],
    ) -> Result<ModelCheckOutcome<S::TransitionId>, ModelCheckRefusal> {
        let explored = self.explore();
        if explored.is_ok() {
            if self.reachability_is_exhaustive() {
                if let Some(shape) = self.deferred_local_shape()? {
                    return Ok(ModelCheckOutcome::Memory {
                        cause: MemoryCause::WeakAccessShape(Box::new(shape)),
                        stats: self.stats(),
                    });
                }
            }
            if !self.has_unresolved_local_access() {
                if let Some((_, violation)) = self.first_bad.take() {
                    let Violation { kind, at, binding } = *violation;
                    let counterexample = match self.counterexample(kind, at, binding) {
                        Ok(counterexample) => counterexample,
                        Err(Halt::Refusal(refusal)) => return Err(refusal),
                        Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                            return Ok(ModelCheckOutcome::Stopped {
                                cause,
                                limit,
                                stats: self.stats(),
                            });
                        }
                        Err(Halt::Finish(_)) => {
                            return Err(fault("a-concretisation-stops-or-refuses"));
                        }
                    };
                    return Ok(ModelCheckOutcome::Violated {
                        counterexample: Box::new(counterexample),
                        stats: self.stats(),
                    });
                }
            }
        }
        match explored {
            // FR-158: a boundary state of the state constraint settles before
            // any bound (ADR-021 RV-4), then ADR-025 MB-4: a bound-limited
            // state settles first.
            Ok(()) => Ok(if self.constraint.is_some() && self.boundary > 0 {
                ModelCheckOutcome::Reduction {
                    cause: ReductionCause::ConstraintReached {
                        boundary_states: widen(self.boundary),
                    },
                    stats: self.stats(),
                }
            } else if let (Some(bound), true) = (
                self.message_bound,
                self.memory_bound
                    .is_none_or(|memory| memory.member != MemoryBoundMember::MaxMessages)
                    && !self.message_bound_limited.is_empty(),
            ) {
                ModelCheckOutcome::Memory {
                    cause: MemoryCause::ChannelBoundReached {
                        bound,
                        states: widen(self.message_bound_limited.len()),
                    },
                    stats: self.stats(),
                }
            } else if let (Some(bound), true) = (self.memory_bound, !self.bound_limited.is_empty())
            {
                ModelCheckOutcome::Memory {
                    cause: MemoryCause::BoundReached {
                        bound,
                        states: widen(self.bound_limited.len()),
                    },
                    stats: self.stats(),
                }
            } else if let (Some(bound), true) =
                (self.message_bound, !self.message_bound_limited.is_empty())
            {
                ModelCheckOutcome::Memory {
                    cause: MemoryCause::ChannelBoundReached {
                        bound,
                        states: widen(self.message_bound_limited.len()),
                    },
                    stats: self.stats(),
                }
            } else if let (Some(bound), true) =
                (self.instance_bound, !self.instance_limited.is_empty())
            {
                // ADR-027 AE-4: an instance-limited state settles before
                // the horizon, after a bound-limited one.
                ModelCheckOutcome::Undecided {
                    cause: UndecidedCause::InstanceBoundReached {
                        bound,
                        states: widen(self.instance_limited.len()),
                    },
                    stats: self.stats(),
                }
            } else if self.horizon_cut {
                ModelCheckOutcome::BoundReached {
                    depth: widen(self.max_depth.unwrap_or(0)),
                    stats: self.stats(),
                }
            } else if let Some(outcome) = self.zero_progress_outcome()? {
                outcome
            } else {
                ModelCheckOutcome::Holds {
                    certificate: ProofCertificate::Closure(self.certificate(instances)?),
                    stats: self.stats(),
                }
            }),
            Err(Halt::Refusal(refusal)) => Err(refusal),
            Err(Halt::Finish(Finish::Undecided(cause))) => Ok(ModelCheckOutcome::Undecided {
                cause,
                stats: self.stats(),
            }),
            Err(Halt::Finish(Finish::Stopped(cause, limit))) => Ok(ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats: self.stats(),
            }),
            Err(Halt::Finish(Finish::Memory(cause))) => Ok(ModelCheckOutcome::Memory {
                cause,
                stats: self.stats(),
            }),
            Err(Halt::Finish(Finish::Violated(violation))) => {
                let Violation { kind, at, binding } = *violation;
                let counterexample = match self.counterexample(kind, at, binding) {
                    Ok(counterexample) => counterexample,
                    Err(Halt::Refusal(refusal)) => return Err(refusal),
                    Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                        return Ok(ModelCheckOutcome::Stopped {
                            cause,
                            limit,
                            stats: self.stats(),
                        });
                    }
                    Err(Halt::Finish(_)) => return Err(fault("a-concretisation-stops-or-refuses")),
                };
                Ok(ModelCheckOutcome::Violated {
                    counterexample: Box::new(counterexample),
                    stats: self.stats(),
                })
            }
        }
    }

    /// The breadth-first first phase.
    fn explore(&mut self) -> Step<()> {
        self.poll()?;
        let start = self.monitor.initial().map_err(Self::automaton)?;
        let mut initial = Vec::new();
        for (index, state) in self.system.initial().into_iter().enumerate() {
            let (state, permutation) = self.canonical(state)?;
            let (key, state_key) = self.system.state_key(&state)?;
            initial.push((key, index, state_key, state, permutation));
        }
        // Equal keys coalesce into the first initial state that has the key.
        initial.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        initial.dedup_by(|later, earlier| later.0 == earlier.0);
        if initial.len() > self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            )
            .into());
        }
        let before = vec![start; self.item.instance_count()];
        for (key, index, state_key, state, permutation) in initial {
            if self.first_bad.is_some() && !self.has_unresolved_local_access() {
                break;
            }
            let node = Node {
                state,
                key: fsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                state_key,
                depth: 0,
                distance: 0,
                parent: None,
                initial_index: index,
                monitor: Vec::new(),
                permutation,
                boundary: false,
            };
            self.create(key, node, &before, None)?;
        }
        #[cfg(test)]
        let mut merged_states = 0usize;
        while !self.queue.is_empty() {
            let batch_positions: Vec<_> = self
                .queue
                .iter()
                .take(self.workers.max(1))
                .copied()
                .collect();
            let mut expanded_batch = self.expand_batch(&batch_positions).into_iter();
            for expected in batch_positions {
                #[cfg(test)]
                cancel_at_test_merge(self.cancel, merged_states);
                let (at, expanded) = expanded_batch
                    .next()
                    .unwrap_or_else(|| panic!("every queued expansion has a result"));
                if at != expected || self.queue.pop_front() != Some(at) {
                    return Err(fault("worker-results-preserve-breadth-first-order").into());
                }
                #[cfg(test)]
                {
                    WORKER_TEST_MERGE.with(|merge| merge.set(Some(merged_states)));
                    self.snapshot_worker_frontier(at);
                    merged_states = merged_states.saturating_add(1);
                }
                self.poll()?;
                if self.first_bad.as_ref().is_some_and(|(bad, _)| *bad == at)
                    && !self.has_unresolved_local_access()
                {
                    if let Some((_, violation)) = self.first_bad.take() {
                        return Err(Finish::Violated(violation).into());
                    }
                }
                if let Some(violation) = self.system.loop_proof_violation(&self.nodes[at].state) {
                    let target = match self.item {
                        Item::LoopInvariant { repeat } => {
                            (repeat, fsl_replay::LoopProofObligation::Invariant)
                        }
                        Item::LoopTermination { repeat } => {
                            (repeat, fsl_replay::LoopProofObligation::Termination)
                        }
                        _ => return Err(fault("loop-proof-failure-has-loop-proof-item").into()),
                    };
                    if *target.0 != violation.repeat {
                        return Err(fault("loop-proof-failure-names-requested-repeat").into());
                    }
                    use fsl_eval::simulation::protocol::ProtocolLoopProofFailure as Failure;
                    let failure = match violation.failure {
                        Failure::InvariantFalse => fsl_replay::LoopProofFailure::InvariantFalse,
                        Failure::Undefined => fsl_replay::LoopProofFailure::Undefined,
                        Failure::VariantDidNotDecrease => {
                            fsl_replay::LoopProofFailure::VariantDidNotDecrease
                        }
                        Failure::VariantBelowFloor => {
                            fsl_replay::LoopProofFailure::VariantBelowFloor
                        }
                        Failure::EvaluationIncomplete => {
                            return Err(
                                Finish::Undecided(UndecidedCause::UndecidedSuccessor).into()
                            );
                        }
                        Failure::EvaluationRefused => {
                            return Err(fault("a-checked-loop-proof-body-does-not-refuse").into());
                        }
                    };
                    return Err(Finish::violated(
                        CounterexampleKind::LoopProof(fsl_replay::LoopProofCounterexample {
                            repeat: violation.repeat.0,
                            obligation: target.1,
                            failure,
                        }),
                        at,
                        None,
                    )
                    .into());
                }
                // FR-158: a boundary state is never expanded, stutter-extended
                // or classified as a deadlock.
                if self.nodes[at].boundary {
                    continue;
                }
                let (expansion, cut) = match expanded {
                    Ok(expanded) => expanded,
                    Err(stop) => return Err(self.expansion_stop(stop)),
                };
                self.record_weak_access_classification(&expansion.findings)?;
                self.contracts(at, &expansion.findings)?;
                if cut.memory {
                    self.bound_limited.insert(self.nodes[at].state_key.clone());
                    // ADR-025 MB-3: a state whose only enabled steps are cut
                    // stores is a boundary state: never stutter-extended and
                    // never a deadlock.
                    if expansion.successors.is_empty() {
                        continue;
                    }
                }
                if cut.messages {
                    self.message_bound_limited
                        .insert(self.nodes[at].state_key.clone());
                    if expansion.successors.is_empty() {
                        continue;
                    }
                }
                // ADR-027 AE-4: an instance-limited state is never terminal. With
                // no expanded successor it is a boundary state, as MB-3's is:
                // never closed, stutter-extended or a deadlock.
                if cut.instances {
                    self.instance_limited
                        .insert(self.nodes[at].state_key.clone());
                    if expansion.successors.is_empty() {
                        continue;
                    }
                }
                if matches!(self.item, Item::Deadlock) {
                    let after = self.deadlock_letter(at, &expansion, cut)?;
                    self.nodes[at].monitor = vec![after];
                }
                if self.chooser.is_none()
                    && matches!(
                        self.item,
                        Item::Clause {
                            uncounted: Some(_),
                            ..
                        }
                    )
                {
                    let enabled = expansion
                        .successors
                        .iter()
                        .map(|(transition, _)| transition.clone())
                        .collect();
                    self.enabled.insert(at, enabled);
                }
                let terminal = expansion.successors.is_empty();
                if terminal {
                    self.close(at)?;
                }
                if self.first_bad.is_some() && !self.has_unresolved_local_access() {
                    continue;
                }
                let mut steps = Vec::with_capacity(expansion.successors.len());
                for (transition, state) in expansion.successors {
                    let order = self.system.transition_key(&transition)?;
                    let (state, permutation) = self.canonical(state)?;
                    let (key, state_key) = self.system.state_key(&state)?;
                    steps.push((order, key, state_key, Some(transition), state, permutation));
                }
                steps.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
                if terminal && self.stutters() {
                    let (key, state_key) = self.system.state_key(&self.nodes[at].state)?;
                    steps.push((
                        Vec::new(),
                        key,
                        state_key,
                        None,
                        self.nodes[at].state.clone(),
                        Permutation::identity(),
                    ));
                }
                let depth = self.nodes[at].depth;
                let before = self.nodes[at].monitor.clone();
                if self.max_depth.is_some_and(|horizon| depth >= horizon) {
                    // A terminal stutter that coalesces with an explored product
                    // state leaves nothing unexplored.
                    let unexplored = match steps.pop() {
                        Some((_, key, state_key, None, state, permutation)) => {
                            let node = Node {
                                state,
                                key: fsl_eval::simulation::StateKey::from_canonical_bytes(
                                    key.clone(),
                                ),
                                state_key,
                                depth: depth.saturating_add(1),
                                distance: self.distance_after(at, None),
                                parent: Some((at, None)),
                                initial_index: self.nodes[at].initial_index,
                                monitor: Vec::new(),
                                permutation,
                                boundary: false,
                            };
                            let (after, canonical, bad) = self.advance(&node, &before)?;
                            bad.is_some()
                                || !self.known.contains_key(&self.product(
                                    key,
                                    after,
                                    node.distance,
                                    &canonical,
                                ))
                        }
                        Some(_) => true,
                        None => false,
                    };
                    self.horizon_cut |= unexplored;
                    continue;
                }
                let mut reads = self.ample(at, &before, &mut steps)?.into_iter();
                for (_, key, state_key, transition, state, permutation) in steps {
                    if self.edges >= self.limits.limits.max_transitions {
                        return Err(Self::stopped(
                            ModelCheckLimit::MaxTransitions,
                            widen(self.limits.limits.max_transitions),
                        )
                        .into());
                    }
                    self.edges += 1;
                    if transition.is_none() {
                        self.stutter_edges += 1;
                    }
                    let node = Node {
                        state,
                        key: fsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                        state_key,
                        depth: depth.saturating_add(1),
                        distance: self.distance_after(at, transition.as_ref()),
                        parent: Some((at, transition)),
                        initial_index: self.nodes[at].initial_index,
                        monitor: Vec::new(),
                        permutation,
                        boundary: false,
                    };
                    self.create(key, node, &before, reads.next().flatten())?;
                    if self.first_bad.is_some() && !self.has_unresolved_local_access() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// FR-156: keep of `steps`, the canonical successors of node `at`, those
    /// through its ample set. A step through the terminal stutter edge or a
    /// state with a single enabled transition is kept; so is every step when
    /// no candidate satisfies C2 and C3.
    ///
    /// Judging C3 reads each successor's letter, so the readings come back,
    /// one per kept step, for `create` to use instead of reading the letter
    /// again; they are all `None` when nothing was judged.
    fn ample(
        &mut self,
        at: usize,
        before: &[StateId],
        steps: &mut Vec<Successor<S>>,
    ) -> Step<Vec<Option<Reading>>> {
        let Some(chooser) = self.chooser else {
            return Ok(vec![None; steps.len()]);
        };
        let mut enabled: Vec<S::TransitionId> = Vec::new();
        for transition in steps.iter().filter_map(|step| step.3.as_ref()) {
            if !enabled.contains(transition) {
                enabled.push(transition.clone());
            }
        }
        if enabled.len() <= 1 {
            return Ok(vec![None; steps.len()]);
        }
        // C3 on product states (POR-8): a step is fresh when its product
        // state is bad, which is always created, or not yet discovered.
        let depth = self.nodes[at].depth;
        let mut fresh: Vec<bool> = Vec::with_capacity(steps.len());
        let mut readings: Vec<Reading> = Vec::with_capacity(steps.len());
        for (_, key, state_key, transition, state, permutation) in steps.iter() {
            let node = Node {
                state: state.clone(),
                key: fsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                state_key: state_key.clone(),
                depth: depth.saturating_add(1),
                distance: self.distance_after(at, transition.as_ref()),
                parent: Some((at, transition.clone())),
                initial_index: self.nodes[at].initial_index,
                monitor: Vec::new(),
                permutation: permutation.clone(),
                boundary: false,
            };
            let (after, canonical, bad) = self.advance(&node, before)?;
            fresh.push(
                bad.is_some()
                    || !self.known.contains_key(&self.product(
                        key.clone(),
                        after.clone(),
                        node.distance,
                        &canonical,
                    )),
            );
            readings.push((after, canonical, bad));
        }
        let reaches_known = |transition: &S::TransitionId| {
            steps
                .iter()
                .zip(&fresh)
                .any(|(step, fresh)| step.3.as_ref() == Some(transition) && !fresh)
        };
        let chosen = chooser
            .choose(&self.nodes[at].state, &enabled, &|transition| {
                !reaches_known(transition)
            })
            .map_err(|stop| self.expansion_stop(stop))?;
        let kept: Vec<bool> = steps
            .iter()
            .map(|step| {
                step.3
                    .as_ref()
                    .is_none_or(|transition| chosen.contains(transition))
            })
            .collect();
        let mut kept_flags = kept.iter();
        steps.retain(|_| *kept_flags.next().unwrap_or(&true));
        Ok(readings
            .into_iter()
            .zip(kept)
            .filter_map(|(reading, kept)| kept.then_some(Some(reading)))
            .collect())
    }

    /// `state`'s canonical form under the run's group and the permutation
    /// that maps `state` to it (FR-152); `state` itself and the identity
    /// under the trivial group.
    fn canonical(&self, state: S::State) -> Step<(S::State, Permutation)> {
        if self.group.is_trivial() {
            return Ok((state, Permutation::identity()));
        }
        self.reducer
            .ok_or_else(|| fault("a-reduced-run-has-a-reducer"))?
            .canonicalise(&state, &self.group)
            .map_err(|stop| self.expansion_stop(stop))
    }

    /// Whether a terminal state takes the terminal stutter edge: a TP-1 or
    /// TP-3 clause under infinite-trace semantics (FR-125). The
    /// deadlock-freedom item's letter at a terminal state is decided by its
    /// classification alone, so a stutter adds nothing to it.
    fn stutters(&self) -> bool {
        matches!(self.item, Item::Clause { bounded: false, .. })
    }

    /// The counted distance of the node reached from node `at` by
    /// `transition`, `None` for the terminal stutter step, which counts
    /// (ADR-027 PB-3).
    fn distance_after(&self, at: usize, transition: Option<&S::TransitionId>) -> usize {
        let counted = transition.is_none_or(|transition| self.system.counts_step(transition));
        self.nodes[at].distance.saturating_add(usize::from(counted))
    }

    /// Record the edge into `node`, which is product node `target`, when its
    /// step does not count and the claim reads that distinction.
    fn note_uncounted_edge(&mut self, node: &Node<S::State, S::TransitionId>, target: usize) {
        if !matches!(
            self.item,
            Item::Clause {
                uncounted: Some(_),
                ..
            }
        ) {
            return;
        }
        if let Some((parent, Some(transition))) = &node.parent {
            if !self.system.counts_step(transition) {
                self.uncounted_edges
                    .push((*parent, target, transition.clone()));
            }
        }
    }

    /// The outcome that replaces `Holds` when a fair cycle of steps that do
    /// not count keeps an interval eventuality open (ADR-027 PB-3, FR-338
    /// "Protocol non-progress acceptance"), or when the search is cancelled.
    fn zero_progress_outcome(
        &self,
    ) -> Result<Option<ModelCheckOutcome<S::TransitionId>>, ModelCheckRefusal> {
        Ok(match self.zero_progress() {
            Ok(None) => None,
            Ok(Some(counterexample)) => Some(ModelCheckOutcome::Violated {
                counterexample: Box::new(counterexample),
                stats: self.stats(),
            }),
            Err(Halt::Refusal(refusal)) => return Err(refusal),
            Err(Halt::Finish(Finish::Stopped(cause, limit))) => Some(ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats: self.stats(),
            }),
            Err(Halt::Finish(_)) => return Err(fault("a-zero-progress-search-stops-or-finds")),
        })
    }

    /// The lasso of the first fair cycle of steps that do not count among
    /// the product states that owe an interval eventuality, for the first
    /// instance that has one. An uncounted step neither ages the debt nor,
    /// being on a cycle, discharges it for good, so the behaviour that takes
    /// the cycle forever never meets the interval; it is a counterexample when
    /// the scheduler constraints and the claim's own admit it, each read over
    /// the complete subject at every state of the cycle (FR-212).
    fn zero_progress(&self) -> Step<Option<TemporalCounterexample<S::TransitionId>>> {
        let Item::Clause {
            uncounted: Some(_),
            bindings,
            ..
        } = self.item
        else {
            return Ok(None);
        };
        if self.uncounted_edges.is_empty() {
            return Ok(None);
        }
        for instance in 0..bindings.len() {
            let owing = self.owing(instance)?;
            let owes = |node: &usize| owing.get(*node).copied().unwrap_or(false);
            let edges: Vec<&(usize, usize, S::TransitionId)> = self
                .uncounted_edges
                .iter()
                .filter(|(from, to, _)| owes(from) && owes(to))
                .collect();
            let mut touched: Vec<usize> = edges
                .iter()
                .flat_map(|(from, to, _)| [*from, *to])
                .collect();
            touched.sort_unstable();
            touched.dedup();
            let mut graph = ZeroProgress::<S>::new();
            let mut vertex_of = BTreeMap::new();
            for node in &touched {
                self.poll()?;
                let state = &self
                    .nodes
                    .get(*node)
                    .ok_or_else(|| fault("an-edge-names-a-node"))?
                    .state;
                let reexpanded;
                let enabled: Vec<&S::TransitionId> = match self.enabled.get(node) {
                    Some(enabled) => enabled.iter().collect(),
                    None => {
                        reexpanded = self
                            .system
                            .successors_bounded(state)
                            .map_err(|stop| self.expansion_stop(stop))?
                            .0;
                        reexpanded
                            .successors
                            .iter()
                            .map(|(transition, _)| transition)
                            .collect()
                    }
                };
                vertex_of.insert(*node, graph.vertex(self.system, enabled)?);
            }
            for (from, to, transition) in &edges {
                let (Some(from), Some(to)) = (vertex_of.get(from), vertex_of.get(to)) else {
                    return Err(fault("an-edge-joins-two-vertices").into());
                };
                graph.edge(self.system, *from, *to, transition)?;
            }
            let found = graph
                .fair_loop(self.system, self.item.fairness(), self.cancel)
                .map_err(|error| self.progress_stop(error))?;
            let Some(found) = found else {
                continue;
            };
            return self
                .zero_progress_lasso(instance, &touched, found)
                .map(Some);
        }
        Ok(None)
    }

    /// Whether each node's instance `instance` monitor state owes an
    /// interval eventuality.
    fn owing(&self, instance: usize) -> Step<Vec<bool>> {
        let mut owing = vec![false; self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            let Some(state) = node.monitor.get(instance) else {
                continue;
            };
            let members = self
                .monitor
                .members(*state)
                .ok_or_else(|| fault("a-node-has-a-monitor-state"))?;
            if let Some(slot) = owing.get_mut(index) {
                *slot = members
                    .iter()
                    .any(|member| self.monitor.automaton().owes_eventuality(*member));
            }
        }
        Ok(owing)
    }

    /// The halt of a failed search for a fair cycle.
    fn progress_stop(&self, error: ZeroProgressError) -> Halt {
        match error {
            ZeroProgressError::Encoding(refusal) => refusal.into(),
            ZeroProgressError::Fairness(error) => ModelCheckRefusal::from(error).into(),
            ZeroProgressError::Cancelled => match self.cancel.cause() {
                Some(source) => Finish::Stopped(IncompleteCause::Cancelled { source }, None).into(),
                None => fault("a-cancelled-run-has-a-cause").into(),
            },
            ZeroProgressError::Graph => fault("the-zero-progress-graph-is-built-from-nodes").into(),
        }
    }

    /// The counterexample whose stem is the breadth-first tree path to the
    /// first node of the fair component `found` and whose loop is CX-5's
    /// greedy walk inside it. `touched` numbers the component's nodes by
    /// vertex.
    fn zero_progress_lasso(
        &self,
        instance: usize,
        touched: &[usize],
        found: FairLoop<S::TransitionId>,
    ) -> Step<TemporalCounterexample<S::TransitionId>> {
        let FairLoop {
            graph,
            constraints,
            component,
            fair_vertices: _,
            removed,
            entry,
            transitions,
        } = found;
        let model = FairModel { graph, constraints };
        let walk = model.greedy_loop(&component, &removed, entry)?;
        let node_of = |vertex: Vertex| {
            touched
                .get(vertex.index())
                .copied()
                .ok_or_else(|| fault("a-vertex-is-a-node"))
        };
        let (initial_state, mut lift) = self.lift_path(node_of(entry)?)?;
        let entry_position = lift.len();
        let entry_key = lift.loop_key().map_err(|stop| self.lift_stop(stop))?;
        for (_, edge) in &walk {
            let EdgeLabel::Transition(identity) = edge.label else {
                return Err(fault("a-zero-progress-edge-is-a-transition").into());
            };
            let transition = usize::try_from(identity.value())
                .ok()
                .and_then(|number| transitions.get(number))
                .ok_or_else(|| fault("an-edge-names-a-transition"))?;
            let node = &self.nodes[node_of(edge.target)?];
            lift.edge(transition, &node.state, &Permutation::identity())
                .map_err(|stop| self.lift_stop(stop))?;
        }
        if lift.loop_key().map_err(|stop| self.lift_stop(stop))? != entry_key {
            return Err(fault("a-lifted-zero-progress-loop-closes-by-cycle-key").into());
        }
        let mut counterexample = self.finished(
            CounterexampleKind::Formula,
            node_of(entry)?,
            self.binding_of(instance),
            initial_state,
            lift.into_steps(),
        )?;
        counterexample.lasso = Some(Lasso {
            entry: entry_position,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        });
        Ok(counterexample)
    }

    /// The product key of a model state's `key` read to the monitor states
    /// `monitor` at counted distance `distance`. When the subject's steps
    /// count by kind and the claim reads positions up to a horizon, the
    /// distance decides which later positions it reads, so a state reached
    /// at two distances up to the horizon is two product states; otherwise
    /// the distance is not part of the key.
    fn product(
        &self,
        key: Vec<u8>,
        monitor: Vec<StateId>,
        distance: usize,
        canonical: &[BoundedState],
    ) -> ProductKey {
        let reach = match self.item {
            Item::Clause {
                counting: Counting::ByKind,
                last_read: Some(last),
                ..
            }
            // An infinite-trace bounded clause stutters its terminal state
            // past the horizon, so the stutters before and after it are two
            // product states under their own canonical bounded state.
            | Item::Clause {
                bounded: false,
                last_read: Some(last),
                ..
            } => usize::try_from(*last).map_or(usize::MAX, |last| last.saturating_add(1)),
            Item::Clause { .. }
            | Item::Deadlock
            | Item::Race
            | Item::Invariant { .. }
            | Item::LoopInvariant { .. }
            | Item::LoopTermination { .. } => 0,
        };
        // The canonical bounded state is part of the key: the general monitor
        // state alone merges `Open { remaining: 0 }` with `Idle`, which the
        // closure checker keeps apart (FR-338).
        (key, monitor, distance.min(reach), canonical.to_vec())
    }

    /// Create the product state of `node`'s model state reached with the
    /// monitor states `before`: read its letter for each instance and step
    /// the monitor, then coalesce it with an existing state of the same key
    /// or add it to the frontier. A state whose letter is undefined or whose
    /// monitor rejects is added as the run's first bad state.
    fn create(
        &mut self,
        key: Vec<u8>,
        mut node: Node<S::State, S::TransitionId>,
        before: &[StateId],
        read: Option<Reading>,
    ) -> Step<()> {
        let (after, canonical, mut bad) = match read {
            Some(read) => read,
            None => self.advance(&node, before)?,
        };
        node.monitor = after;
        let product = self.product(key, node.monitor.clone(), node.distance, &canonical);
        let target = self.known.get(&product).copied();
        self.note_uncounted_edge(&node, target.unwrap_or(self.nodes.len()));
        if bad.is_none() && target.is_some() {
            return Ok(());
        }
        if self.nodes.len() >= self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            )
            .into());
        }
        if bad.is_none() {
            if let Some(name) = self.constraint {
                let reducer = self
                    .reducer
                    .ok_or_else(|| fault("a-constrained-run-has-a-reducer"))?;
                match reducer.constraint_value(&node.state, name) {
                    Ok(ConstraintValue::Holds) => {}
                    Ok(ConstraintValue::Boundary) => {
                        node.boundary = true;
                        self.boundary = self.boundary.saturating_add(1);
                    }
                    // ADR-021 SC-2: an undefined constraint refutes the item
                    // at this state, which is not a boundary state.
                    Ok(ConstraintValue::Undefined { cause, .. }) => {
                        let site =
                            Self::site(&node, UndefinedLocus::Constraint(name.to_owned()), None);
                        bad = Some((
                            CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                                site,
                                cause,
                            }),
                            0,
                        ));
                    }
                    Err(stop) => return Err(self.expansion_stop(stop)),
                }
            }
        }
        let at = self.nodes.len();
        self.nodes.push(node);
        self.canonical.push(canonical);
        self.queue.push_back(at);
        match bad {
            Some((kind, instance)) => {
                let binding = self.binding_of(instance);
                if self.first_bad.is_none() {
                    self.first_bad = Some((at, Box::new(Violation { kind, at, binding })));
                }
            }
            None => {
                self.known.insert(product, at);
            }
        }
        Ok(())
    }

    /// Each instance's monitor state after reading `node`'s position from
    /// `before`, and the first finding with its instance: an undefined atom,
    /// which ends the read with the remaining instances left at `before`, or
    /// else the first rejecting monitor state.
    fn advance(
        &mut self,
        node: &Node<S::State, S::TransitionId>,
        before: &[StateId],
    ) -> Step<Reading> {
        if matches!(
            self.item,
            Item::Deadlock | Item::LoopInvariant { .. } | Item::LoopTermination { .. }
        ) {
            return Ok((before.to_vec(), Vec::new(), None));
        }
        if let Item::Invariant { name } = self.item {
            return self.invariant_letter(node, before, name);
        }
        if matches!(self.item, Item::Race) {
            let bad = match &node.parent {
                Some((parent, Some(transition))) => match transition.race() {
                    Some(race) => Some((self.race_found(race, *parent, node.depth)?, 0)),
                    None => None,
                },
                Some((_, None)) | None => None,
            };
            return Ok((before.to_vec(), Vec::new(), bad));
        }
        let counted = match &node.parent {
            Some((_, Some(transition))) => self.system.counts_step(transition),
            // An initial state has no step into it, and the terminal stutter
            // step counts (ADR-027 PB-3).
            Some((_, None)) | None => true,
        };
        let earlier: Vec<BoundedState> = node
            .parent
            .as_ref()
            .and_then(|(parent, _)| self.canonical.get(*parent))
            .cloned()
            .unwrap_or_default();
        let mut after = Vec::with_capacity(before.len());
        let mut canonical = Vec::new();
        let mut bad = None;
        for (instance, from) in before.iter().enumerate() {
            let (state, finding, holds, activates) = self.read(node, instance, *from, counted)?;
            after.push(state);
            if let Item::Clause {
                canonical: translations,
                ..
            } = self.item
            {
                if let Some(translation) = translations.get(instance) {
                    // A position the claim does not read carries no letter;
                    // a step out of the window closes it before one.
                    let holds = holds.unwrap_or(false);
                    canonical.push(match (earlier.get(instance), translation.each()) {
                        (Some(previous), false) => translation.step(*previous, counted, holds),
                        (Some(previous), true) => {
                            translation.step_each(*previous, counted, holds, activates)
                        }
                        (None, false) => translation.start(holds),
                        (None, true) => translation.start_each(holds, activates),
                    });
                }
            }
            match finding {
                Some(undefined @ CounterexampleKind::UndefinedEvaluation(_)) => {
                    bad = Some((undefined, instance));
                    break;
                }
                Some(kind) => {
                    bad.get_or_insert((kind, instance));
                }
                None => {}
            }
        }
        after.extend_from_slice(before.get(after.len()..).unwrap_or_default());
        Ok((after, canonical, bad))
    }

    /// Instance `instance`'s monitor state after reading `node`'s letter
    /// from `from`, evaluating every atom of the clause when the claim reads
    /// `node`'s position and none otherwise, and what the read found: an
    /// undefined atom, or a rejecting monitor state.
    fn read(
        &mut self,
        node: &Node<S::State, S::TransitionId>,
        instance: usize,
        from: StateId,
        counted: bool,
    ) -> Step<(StateId, Option<CounterexampleKind>, Option<bool>, bool)> {
        let Item::Clause {
            bindings,
            letters,
            atoms,
            when,
            captured,
            live,
            uncounted,
            last_read,
            ..
        } = self.item
        else {
            return Err(fault("only-a-clause-reads-letters").into());
        };
        let binding = bindings
            .get(instance)
            .ok_or_else(|| fault("instance-has-a-binding"))?;
        let mut letter = live.map_or_else(Letter::default, |live| Letter::default().with(live));
        if let Some(uncounted) = uncounted.filter(|_| !counted) {
            letter = letter.with(uncounted);
        }
        let reads_position = last_read.is_none_or(|last| widen(node.distance) <= last);
        let read: &[FormulaNodeId] = if reads_position { atoms } else { &[] };
        // UE-1: a guarded activation's condition is read before the atoms.
        if let Some(when) = (*when).filter(|_| reads_position) {
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let reading = letters
                .activation(binding, &self.system.atom_state(&node.state), &mut meter)
                .map_err(|stop| match atom_refusal(stop) {
                    Ok(refusal) => Halt::Refusal(refusal),
                    Err(incomplete) => Self::stopped(
                        ModelCheckLimit::EvaluationMeter {
                            counter: incomplete.limit_kind,
                        },
                        incomplete.limit,
                    )
                    .into(),
                })?;
            match reading {
                AtomReading::True => letter = letter.with(when),
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    let site = Self::site(node, UndefinedLocus::Activation, None);
                    let kind = CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site,
                        cause,
                    });
                    return Ok((from, Some(kind), None, false));
                }
            }
        }
        // FR-337: the captures are fixed once, at an activation: position 0 of
        // an `on origin` clause, each position an `on each` guard admits.
        let activates = (*when).is_none_or(|when| letter.holds(when));
        let capturing = match captured {
            CaptureSite::Never => false,
            CaptureSite::Origin => node.depth == 0,
            CaptureSite::Activations => activates,
        };
        if capturing && reads_position {
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let reading = letters
                .captures(binding, &self.system.atom_state(&node.state), &mut meter)
                .map_err(|stop| match atom_refusal(stop) {
                    Ok(refusal) => Halt::Refusal(refusal),
                    Err(incomplete) => Self::stopped(
                        ModelCheckLimit::EvaluationMeter {
                            counter: incomplete.limit_kind,
                        },
                        incomplete.limit,
                    )
                    .into(),
                })?;
            if let CaptureReading::Undefined { capture, cause } = reading {
                let site = Self::site(node, UndefinedLocus::Capture(capture), None);
                let kind =
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause });
                return Ok((from, Some(kind), None, false));
            }
        }
        // The first atom's reading, the one atom a canonical bounded clause
        // has.
        let mut holds = None;
        for (index, formula_atom) in read.iter().enumerate() {
            let atom = Atom::new(index).map_err(|_| fault("clause-atom-in-range"))?;
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let reading = letters
                .atom(
                    *formula_atom,
                    binding,
                    &self.system.atom_state(&node.state),
                    &mut meter,
                )
                .map_err(|stop| match atom_refusal(stop) {
                    Ok(refusal) => Halt::Refusal(refusal),
                    Err(incomplete) => Self::stopped(
                        ModelCheckLimit::EvaluationMeter {
                            counter: incomplete.limit_kind,
                        },
                        incomplete.limit,
                    )
                    .into(),
                })?;
            match reading {
                AtomReading::True => {
                    letter = letter.with(atom);
                    holds = holds.or(Some(true));
                }
                AtomReading::False => holds = holds.or(Some(false)),
                AtomReading::Undefined(cause) => {
                    let site = Self::site(node, UndefinedLocus::Atom(*formula_atom), None);
                    let kind = CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site,
                        cause,
                    });
                    return Ok((from, Some(kind), None, false));
                }
            }
        }
        let after = self.monitor.step(from, letter).map_err(Self::automaton)?;
        let finding = self.rejects(after)?.then_some(CounterexampleKind::Formula);
        Ok((after, finding, holds, activates))
    }

    /// FSpec FR-091's closed-boundary rule at the terminal node `at` of a
    /// TP-2 clause: each instance's monitor reads every following position
    /// with every atom false until it rejects, which violates the item at
    /// `at`, or repeats a state. No product state is created.
    fn close(&mut self, at: usize) -> Step<()> {
        if !matches!(self.item, Item::Clause { bounded: true, .. }) {
            return Ok(());
        }
        let reached = self.nodes[at].monitor.clone();
        for (instance, state) in reached.into_iter().enumerate() {
            if false_extension_rejects(&mut self.monitor, state).map_err(Self::automaton)? {
                let binding = self.binding_of(instance);
                return Err(Finish::violated(CounterexampleKind::Formula, at, binding).into());
            }
        }
        Ok(())
    }

    fn site(
        node: &Node<S::State, S::TransitionId>,
        locus: UndefinedLocus,
        transition: Option<ModelTransition>,
    ) -> UndefinedSite {
        UndefinedSite {
            position: widen(node.depth),
            state_key: node.key.clone(),
            locus,
            transition,
            time: None,
        }
    }

    /// ADR-018 UE-6 and V-6 (ADR-027 TS-4): the first contract conjunction
    /// or protocol constraint the expansion evaluated undefined ends the run
    /// with a violation; otherwise a refused one leaves it undecided.
    fn contracts(&self, at: usize, findings: &[ModelFinding]) -> Step<()> {
        if let Some(shape) = findings.iter().find_map(|finding| match finding {
            ModelFinding::WeakAccessShape(shape) => Some(shape),
            _ => None,
        }) {
            return Err(Finish::Memory(MemoryCause::WeakAccessShape(shape.clone())).into());
        }
        if let Some((locus, guard_of, cause)) = undefined_finding(findings) {
            let site = Self::site(&self.nodes[at], locus, guard_of);
            return Err(Finish::violated(
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                at,
                self.binding_of(0),
            )
            .into());
        }
        if findings.iter().any(ModelFinding::is_refused_contract) {
            return Err(Finish::Undecided(UndecidedCause::UndecidedSuccessor).into());
        }
        Ok(())
    }

    /// ADR-025 DR-5: the counterexample kind of the race `race` that the step
    /// from node `parent` into depth `later` completes: `earlier` is the
    /// position the canonical path to `parent` reached by the last access
    /// the race's summary names.
    fn race_found(&self, race: &DataRace, parent: usize, later: usize) -> Step<CounterexampleKind> {
        let mut cursor = parent;
        let earlier = loop {
            let node = &self.nodes[cursor];
            let Some((up, step)) = &node.parent else {
                return Err(fault("a-race-names-an-earlier-access").into());
            };
            if step
                .as_ref()
                .and_then(SubjectTransition::access)
                .is_some_and(|access| access.is_named_by(race))
            {
                break node.depth;
            }
            cursor = *up;
        };
        Ok(CounterexampleKind::Race(Race {
            location: race.location.clone(),
            earlier: widen(earlier),
            later: widen(later),
        }))
    }

    /// The `over` binding of a clause's instance `instance`, none for the
    /// derived items. A violation that is not a clause's own names
    /// the first instance.
    fn binding_of(&self, instance: usize) -> Option<ModelInstance> {
        match self.item {
            Item::Clause { bindings, .. } => bindings.get(instance).cloned(),
            Item::Deadlock
            | Item::Race
            | Item::Invariant { .. }
            | Item::LoopInvariant { .. }
            | Item::LoopTermination { .. } => None,
        }
    }

    /// FR-951 and FR-338: the invariant item's letter `I` at `node`, the
    /// invariant `name`'s body over every object of the state whose type
    /// conforms to its context type, read as FR-158 reads a state
    /// constraint. A false `I` violates the item at `node` with a `formula`
    /// counterexample, and an undefined one with the undefined-evaluation
    /// envelope at the invariant; the monitor of `always holds(I)` steps on
    /// the letter either way.
    fn invariant_letter(
        &mut self,
        node: &Node<S::State, S::TransitionId>,
        before: &[StateId],
        name: &str,
    ) -> Step<Reading> {
        self.poll()?;
        let reducer = self
            .reducer
            .ok_or_else(|| fault("an-invariant-run-reads-a-model-state"))?;
        let bad = match reducer.constraint_value(&node.state, name) {
            Ok(ConstraintValue::Holds) => None,
            Ok(ConstraintValue::Boundary) => Some((CounterexampleKind::Formula, 0)),
            Ok(ConstraintValue::Undefined { cause, .. }) => {
                let site = Self::site(node, UndefinedLocus::Invariant(name.to_owned()), None);
                Some((
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                    0,
                ))
            }
            Err(stop) => return Err(self.expansion_stop(stop)),
        };
        let letter = if bad.is_none() {
            Letter::from_bits(1)
        } else {
            Letter::default()
        };
        let mut after = Vec::with_capacity(before.len());
        for from in before {
            after.push(self.monitor.step(*from, letter).map_err(Self::automaton)?);
        }
        Ok((after, Vec::new(), bad))
    }

    /// FR-124 and ADR-018 DL-7: the deadlock-freedom item's letter at node
    /// `at`, `not deadlocked`, read from its expansion: the monitor rejects
    /// a terminal state no declaration intends, which ends the run with a
    /// deadlock. An undefined `terminal when` ends it with an undefined
    /// evaluation, before the monitor steps; a refused one leaves it
    /// undecided.
    fn deadlock_letter(
        &mut self,
        at: usize,
        expansion: &Expansion<S::TransitionId, S::State, ModelFinding>,
        cut: Cut,
    ) -> Step<StateId> {
        let class = match self
            .system
            .terminal_class(&self.nodes[at].state, expansion, cut)
        {
            Ok(class) => class,
            Err(stop) => return Err(self.expansion_stop(stop)),
        };
        let deadlocked = match class {
            TerminalClass::NotTerminal | TerminalClass::Intended => false,
            TerminalClass::Deadlocked => true,
            TerminalClass::Undefined { model, cause } => {
                let site = Self::site(&self.nodes[at], UndefinedLocus::Terminal(model), None);
                return Err(Finish::violated(
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                    at,
                    None,
                )
                .into());
            }
            TerminalClass::Undecided { .. } => {
                return Err(Finish::Undecided(UndecidedCause::UndecidedSuccessor).into());
            }
        };
        let letter = if deadlocked {
            Letter::default()
        } else {
            Letter::from_bits(1)
        };
        let before = *self.nodes[at]
            .monitor
            .first()
            .ok_or_else(|| fault("the-deadlock-item-has-one-instance"))?;
        let after = self.monitor.step(before, letter).map_err(Self::automaton)?;
        if self.rejects(after)? {
            return Err(Finish::violated(CounterexampleKind::Deadlock, at, None).into());
        }
        Ok(after)
    }

    /// An expansion stop: a named budget settles V-7 naming it, a broken
    /// invariant refuses, any other stop settles V-7 with no limit.
    fn expansion_stop(&self, stop: ExpansionStop) -> Halt {
        #[cfg(test)]
        WORKER_TEST_MERGE.with(|merge| WORKER_TEST_STOP_MERGE.set(merge.get()));
        expansion_stopped(stop, &self.meter, self.max_candidates).map_or_else(
            || fault("model-expansion-invariant").into(),
            |(cause, limit)| Finish::Stopped(cause, limit).into(),
        )
    }

    /// The canonical path to node `at`: its breadth-first tree path,
    /// concretised under symmetry (FR-161). A terminal stutter step is no
    /// model step: the behaviour is the model prefix to the terminal state,
    /// stuttered forever.
    fn path(&self, at: usize) -> Vec<usize> {
        let mut path = vec![at];
        let mut cursor = at;
        while let Some((parent, _)) = &self.nodes[cursor].parent {
            cursor = *parent;
            path.push(cursor);
        }
        path.reverse();
        path
    }

    /// The canonical path to node `at` of an unreduced run, its steps the
    /// tree path's. A terminal stutter step is no step: the behaviour is the
    /// prefix to the terminal state, stuttered forever. A deadlock names the
    /// waiting threads of its last state (ADR-027 PD-4).
    fn counterexample(
        &self,
        mut kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
    ) -> Step<TemporalCounterexample<S::TransitionId>> {
        if !self.group.is_trivial() {
            return Err(fault("an-unreduced-path-is-concrete").into());
        }
        let (initial_state, lift) = self.lift_path(at)?;
        lift.site(&mut kind).map_err(|stop| self.lift_stop(stop))?;
        self.finished(kind, at, over_binding, initial_state, lift.into_steps())
    }

    fn lift_path(&self, at: usize) -> Step<(usize, Lift<'_, S>)> {
        let path = self.path(at);
        let root = path.first().ok_or_else(|| fault("a-path-has-a-root"))?;
        let (initial_state, mut lift) = Lift::start(
            self.system,
            None,
            &self.group,
            &self.nodes[*root].state,
            self.limits
                .limits
                .max_transitions
                .saturating_sub(self.edges),
        )
        .map_err(|stop| self.lift_stop(stop))?;
        for index in path.iter().skip(1) {
            let node = &self.nodes[*index];
            let Some((_, Some(transition))) = node.parent.as_ref() else {
                continue;
            };
            lift.edge(transition, &node.state, &Permutation::identity())
                .map_err(|stop| self.lift_stop(stop))?;
        }
        Ok((initial_state, lift))
    }

    fn lift_stop(&self, stop: LiftStop) -> Halt {
        match stop {
            LiftStop::Expansion(stop) => self.expansion_stop(stop),
            LiftStop::Transitions => Self::stopped(
                ModelCheckLimit::MaxTransitions,
                widen(self.limits.limits.max_transitions),
            )
            .into(),
            LiftStop::Refusal(refusal) => refusal.into(),
        }
    }

    fn finished(
        &self,
        kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
        initial_state: usize,
        prefix: Vec<ModelStep<S::TransitionId>>,
    ) -> Step<TemporalCounterexample<S::TransitionId>> {
        let fairness = self.fairness.to_vec();
        let blocked = if matches!(kind, CounterexampleKind::Deadlock) {
            let steps: Vec<&S::TransitionId> = prefix.iter().map(|step| &step.transition).collect();
            self.system
                .blocked(&self.nodes[at].state, &steps)
                .map_err(|stop| self.expansion_stop(stop))?
        } else {
            Vec::new()
        };
        // A claim read on the terminal stutter step is violated by the
        // behaviour that stutters at the prefix's last state forever, whose
        // stutter step the prefix does not list: the marker says so.
        let stutter = matches!(kind, CounterexampleKind::Formula)
            && matches!(self.nodes[at].parent, Some((_, None)));
        let lasso = stutter.then_some(Lasso {
            entry: prefix.len(),
            terminal_stutter: true,
            terminal_stutter_delay: None,
        });
        Ok(TemporalCounterexample {
            initial_state,
            prefix,
            over_binding,
            fairness,
            kind,
            lasso,
            blocked,
            refinement: None,
            final_delay: None,
            closure_stutters: Vec::new(),
        })
    }
}

#[cfg(test)]
mod worker_determinism_tests {
    use super::*;
    use crate::counter;
    use filament_semantic_value::checking::CheckMode;
    use fsl_replay::{ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome};
    use fsl_route::reductions::Reductions;
    use ix_trace_rs::trace;

    const MAKE: &str = "pre CanMake using v on M::Counter::make { self.value = 1 }\n\
                        post Make using v on M::Counter::make { self.value = 2 }\n";

    struct Evidence {
        outcome: Result<ModelCheckOutcome, ModelCheckRefusal>,
        frontier: Vec<Vec<u8>>,
        stopped_at_merge: Option<usize>,
        merged_states: Option<usize>,
    }

    fn hook(delay_first_job: bool, cancel_after_merged: Option<usize>) -> Arc<WorkerTestHook> {
        Arc::new(WorkerTestHook {
            delay_first_job,
            cancel_after_merged,
            cancelled: AtomicBool::new(false),
            active_workers: AtomicUsize::new(0),
            peak_workers: AtomicUsize::new(0),
            next_batch: AtomicUsize::new(0),
            completed: Mutex::new(BTreeMap::new()),
        })
    }

    fn run(
        workers: usize,
        max_candidates: u64,
        cancel_after_merged: Option<usize>,
        set_to: bool,
    ) -> Evidence {
        let mut document = counter::document(false);
        if set_to {
            let mut source: serde_json::Value =
                serde_json::from_slice(&document).expect("the counter source parses");
            let counter_type = source["types"]
                .as_array_mut()
                .expect("types")
                .iter_mut()
                .find(|declared| declared["identity"] == counter::key("Counter").node)
                .expect("Counter type");
            let operations = counter_type["operations"]
                .as_array_mut()
                .expect("operations");
            let mut make = operations[0].clone();
            make["identity"] = serde_json::json!(counter::key("Counter/make").node);
            make["name"] = serde_json::json!("make");
            operations.push(make);
            document = serde_json::to_vec(&source).expect("the fixture serializes");
        }
        let body = if set_to {
            format!("{}{MAKE}", counter::CONTRACT)
        } else {
            counter::CONTRACT.to_owned()
        };
        let package = counter::package_with_mode(&document, &body, CheckMode::Linked);
        let mut documents = counter::Documents::new(document);
        let initial = vec![documents.start("initial", &[("a", 0), ("b", 0), ("c", 0), ("d", 0)])];
        let mut subject = counter::subject(
            &package,
            &documents,
            initial,
            &["a", "b", "c", "d"],
            counter::UNLIMITED,
        );
        subject.simulation.expansion.max_candidates = max_candidates;
        let cancel = Cancel::new();
        let worker_hook = hook(workers > 1, cancel_after_merged);
        WORKER_TEST_HOOK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&worker_hook)));
        WORKER_TEST_FRONTIER.with(|frontier| frontier.borrow_mut().clear());
        WORKER_TEST_MERGE.set(None);
        WORKER_TEST_STOP_MERGE.set(None);
        let outcome = check_model(
            ModelCheckRequest {
                subject,
                workers,
                temporal_clock_selections: &[],
                item: ModelCheckItem::DeadlockFreedom,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Reductions::default(),
            },
            &cancel,
        );
        WORKER_TEST_HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
        let frontier = take_worker_test_frontier();
        let stopped_at_merge = take_worker_test_stop_merge();
        Evidence {
            outcome,
            frontier,
            stopped_at_merge,
            merged_states: WORKER_TEST_MERGE.with(Cell::get),
        }
    }

    fn stopped(outcome: &ModelCheckOutcome) -> (&IncompleteCause, Option<ReachedLimit>, RunStats) {
        match outcome {
            ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats,
            } => (cause, *limit, *stats),
            other => panic!("expected a stopped run, got {other:?}"),
        }
    }

    fn completed_out_of_order(completions: &BTreeMap<usize, Vec<usize>>) -> bool {
        completions
            .values()
            .any(|batch| batch.len() > 1 && batch.first() != Some(&0) && batch.last() == Some(&0))
    }

    fn counter_frontier(states: &[Vec<u8>]) -> Vec<[i64; 4]> {
        states
            .iter()
            .map(|bytes| {
                let state: serde_json::Value =
                    serde_json::from_slice(bytes).expect("canonical state is JSON");
                let entries = state
                    .pointer("/semantic/entries/0/value/entries")
                    .and_then(serde_json::Value::as_array)
                    .expect("the counter population is in the semantic state");
                let mut values = [None; 4];
                for entry in entries {
                    let key = entry["key"]["identity"]
                        .as_str()
                        .expect("the state key has an object identity");
                    let index = match key {
                        "a" => 0,
                        "b" => 1,
                        "c" => 2,
                        "d" => 3,
                        other => panic!("unexpected counter key {other}"),
                    };
                    let value = entry["value"]["fields"][0]["value"]["value"]
                        .as_str()
                        .expect("the counter value is an integer")
                        .parse()
                        .expect("the counter value fits i64");
                    values[index] = Some(value);
                }
                values.map(|value| value.expect("all four counters remain present"))
            })
            .collect()
    }

    fn frontier_fingerprint(states: &[[i64; 4]]) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for state in states {
            for value in state {
                for byte in value.to_le_bytes() {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                }
            }
            hash = (hash ^ 0xff).wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// FR-126-AC-20's raw worker outcomes can finish out of order, while the
    /// helper returns each expansion at its original frontier position.
    #[trace("TC-1417", "FR-126-AC-20")]
    #[test]
    fn a_later_worker_finishes_first_without_reordering_results() {
        let worker_hook = hook(true, None);
        WORKER_TEST_HOOK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&worker_hook)));
        let output = parallel_map(vec![0, 1, 2], 2, |position| *position);
        WORKER_TEST_HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
        assert_eq!(output, [0, 1, 2]);
        assert!(completed_out_of_order(
            &worker_hook
                .completed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        ));
        assert_eq!(worker_hook.peak_workers.load(Ordering::Relaxed), 2);
    }

    /// The bounded setTo domain contributes its first extra candidates when
    /// a Counter reaches value 2; the checked assertion is the seventh BFS
    /// expansion and its pending frontier is stable for all worker counts.
    #[trace("TC-1417", "FR-126-AC-20")]
    #[test]
    fn seventh_candidate_stop_has_the_same_frontier_for_every_worker_count() {
        for workers in [1, 2, 4, 64] {
            let evidence = run(workers, 1_200, None, true);
            let outcome = evidence.outcome.expect("the worker request is valid");
            let (cause, limit, stats) = stopped(&outcome);
            assert_eq!(cause, &IncompleteCause::ResourceExhausted);
            assert_eq!(
                limit,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxCandidates,
                    value: 1_200,
                })
            );
            assert_eq!(evidence.stopped_at_merge, Some(6));
            assert_eq!(
                stats,
                RunStats {
                    product_states: 245,
                    product_edges: 1_920,
                    stutter_edges: 0,
                    automaton_states: 1,
                    depth: 2,
                    bound_limited: 0,
                    message_bound_limited: 0,
                    instance_limited: 0,
                }
            );
            let frontier = counter_frontier(&evidence.frontier);
            assert_eq!(frontier.len(), 239);
            assert_eq!(frontier.first(), Some(&[1, 0, 1, 1]));
            assert_eq!(frontier.last(), Some(&[3, 3, 2, 0]));
            assert_eq!(frontier_fingerprint(&frontier), 0xd911_fe24_9387_6e78);
        }
    }

    /// Cancellation after four coordinator-merged states has the same
    /// public stop and private frontier with one or four expansion workers.
    #[trace("TC-1417", "FR-126-AC-20")]
    #[test]
    fn cancellation_after_four_merges_keeps_the_same_frontier() {
        let sequential = run(1, 1_000_000, Some(4), false);
        let parallel = run(4, 1_000_000, Some(4), false);
        let sequential_outcome = sequential
            .outcome
            .expect("the single-worker request is valid");
        let parallel_outcome = parallel.outcome.expect("the four-worker request is valid");
        let sequential_stop = stopped(&sequential_outcome);
        let parallel_stop = stopped(&parallel_outcome);
        let sequential_frontier = counter_frontier(&sequential.frontier);
        assert_eq!(
            sequential_stop.0,
            &IncompleteCause::Cancelled {
                source: filament_exact::CancelCause::Requested,
            }
        );
        assert_eq!(sequential_stop.1, None);
        assert_eq!(
            sequential_stop.2,
            RunStats {
                product_states: 239,
                product_edges: 1_024,
                stutter_edges: 0,
                automaton_states: 1,
                depth: 2,
                bound_limited: 0,
                message_bound_limited: 0,
                instance_limited: 0,
            }
        );
        assert_eq!(parallel_stop, sequential_stop);
        assert_eq!(sequential.frontier, parallel.frontier);
        assert_eq!(sequential.merged_states, Some(4));
        assert_eq!(parallel.merged_states, Some(4));
        assert_eq!(sequential_frontier.len(), 235);
        assert_eq!(sequential_frontier.first(), Some(&[1, 0, 0, 3]));
        assert_eq!(sequential_frontier.last(), Some(&[3, 3, 3, 3]));
        assert_eq!(
            frontier_fingerprint(&sequential_frontier),
            0x4af4_e346_6be5_c4e7
        );
    }
}

impl Run<'_, ModelSystem<'_>> {
    /// The canonical path to node `at`: its breadth-first tree path,
    /// concretised under symmetry (FR-161).
    fn model_counterexample(
        &self,
        mut kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
    ) -> Step<TemporalCounterexample> {
        if self.group.is_trivial() {
            return self.counterexample(kind, at, over_binding);
        }
        let path = self.path(at);
        let (initial_state, prefix) = self.concretise(&path, &mut kind)?;
        self.finished(kind, at, over_binding, initial_state, prefix)
    }

    /// ADR-021 EI-6 (FR-161 "Start", "Steps"): the subject's initial state
    /// and the concrete steps of the canonical tree path `path`, each edge
    /// lifted by [`Lift`] and counted against what is left of
    /// `max_transitions`. An undefined evaluation's site is concretised at
    /// the path's end.
    fn concretise(
        &self,
        path: &[usize],
        kind: &mut CounterexampleKind,
    ) -> Step<(usize, Vec<ModelStep>)> {
        let first = &self.nodes[*path.first().ok_or_else(|| fault("a-path-has-a-root"))?];
        let budget = self
            .limits
            .limits
            .max_transitions
            .saturating_sub(self.edges);
        let lifted = (|| {
            let (initial, mut lift) = Lift::start(
                self.system,
                Some(self.system),
                &self.group,
                &first.state,
                budget,
            )?;
            for index in path.iter().skip(1) {
                let node = &self.nodes[*index];
                // The terminal stutter edge is no model step.
                if let Some((_, Some(transition))) = &node.parent {
                    lift.edge(transition, &node.state, &node.permutation)?;
                }
            }
            lift.site(kind)?;
            Ok((initial, lift.into_steps()))
        })();
        lifted.map_err(|stop| match stop {
            LiftStop::Expansion(stop) => self.expansion_stop(stop),
            LiftStop::Transitions => Self::stopped(
                ModelCheckLimit::MaxTransitions,
                widen(self.limits.limits.max_transitions),
            )
            .into(),
            LiftStop::Refusal(refusal) => refusal.into(),
        })
    }
}
