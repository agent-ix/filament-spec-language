// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared weak-memory fixture APIs are consumed by separate integration binaries"
)]
//! FR-219, FR-220, FR-221, FR-225, FR-227 and FR-229 (TC-664, TC-665,
//! TC-666, TC-670, TC-672, TC-674): request memory selection, EN-1's
//! access classification and pre-check, and `ProtocolSystem<Tso>` over
//! ADR-025 §10's store buffering (`crate::litmus`).
//!
//! State counts follow ADR-025 §10's convention: states inside the
//! `parallel` (some branch thread live) and joined states (the root at
//! `finish`). The protocol system also has the state before the fork and
//! the states after `finish`, which §10's table does not count.

use std::collections::BTreeSet;

use fsl_analyze::model_check::{ProtocolCheckRequest, SubjectContext, check_protocol};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ProtocolSubject};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};

use filament_exact::{Cancel, Meter};
use fsl_eval::simulation::Location;
use fsl_eval::simulation::protocol::{
    InstanceAccess, MemoryModel, MemorySelection, ProtocolRefusal, ProtocolSystem,
    ProtocolTransition, RestKind, ThreadPath, WeakAccesses, classify_accesses,
};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::MemoryModelName;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::CheckedPackage;
use fsl_replay::{
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord, ModelTraceReplayResult,
    ModelTraceSubject, ProtocolCertificateRequest, ProtocolTraceSubject, ReplayLimits,
    ReplayRefusal, TemporalCounterexample, replay_protocol_trace, settle_protocol_check,
};
use fsl_route::model_request::ModelRequest;
use fsl_semantics::check::{AccessClass, FormulaNodeId};

use crate::fill::Reached;
use crate::litmus::{
    Documents, PROTOCOL_PROFILE, attempt, core, document, field, key, package,
    protocol_profiled_package, sb_unit, unit_text_with,
};
use crate::model_settle_support::{envelope_at, request};

/// The request entry selecting `model` for `parallel`.
pub(crate) fn select(parallel: &str, model: MemoryModelName) -> MemorySelection {
    MemorySelection {
        parallel: parallel.to_owned(),
        model,
    }
}

/// One unit, its documents and its checked package.
pub(crate) struct Case {
    body: String,
    documents: Documents,
    checked: CheckedPackage,
    temporal_clock_selections: Vec<fsl_package::TemporalClockSelection>,
    /// The letters of the unit's one claim.
    letters: &'static dyn ModelLetters,
    /// The cell `SB`'s `over` input is bound to, when not the core.
    over: Option<ModelInstance>,
    /// The further profile lines of the unit's header.
    profiles: &'static str,
}

impl Case {
    pub(crate) fn new(body: &str) -> Self {
        Self::with_letters(body, &NoBothZero)
    }

    /// [`Self::new`] over the three cells `x`, `y` and `z`.
    pub(crate) fn with_three_cells(body: &str) -> Self {
        let mut case = Self::new(body);
        case.documents = Documents::with_cells(&["x", "y", "z"]);
        case
    }

    /// The unit with `body`, its claim's atom read by `letters`.
    pub(crate) fn with_letters(body: &str, letters: &'static dyn ModelLetters) -> Self {
        let checked = package(body);
        let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&checked);
        Self {
            body: body.to_owned(),
            documents: Documents::new(),
            checked,
            temporal_clock_selections,
            letters,
            over: None,
            profiles: "",
        }
    }

    /// [`Self::with_letters`] with the protocol profile alias retained as a
    /// declaration but removed from the selected profiles, for FR-368's
    /// no-protocol-profile route case.
    pub(crate) fn without_protocol_profile(body: &str, letters: &'static dyn ModelLetters) -> Self {
        let checked = crate::litmus::package_without_protocol_profile_selection(body);
        let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&checked);
        Self {
            body: body.to_owned(),
            documents: Documents::new(),
            checked,
            temporal_clock_selections,
            letters,
            over: None,
            profiles: "",
        }
    }

    /// [`Self::with_letters`] with the protocol profile selected, so `SB`'s
    /// clause has its node.
    pub(crate) fn protocol_profiled(body: &str, letters: &'static dyn ModelLetters) -> Self {
        let mut case = Self::with_letters(body, letters);
        case.checked = protocol_profiled_package(body);
        case.temporal_clock_selections =
            crate::fill::tcb_01_event_position_selection(&case.checked);
        case.profiles = PROTOCOL_PROFILE;
        case
    }

    /// The unit with `body`, `SB` over the object `object` of `population`;
    /// `spare` adds the `Spare` object `z` to the universe.
    fn over_object(body: &str, population: &str, object: &str, spare: bool) -> Self {
        Self {
            documents: if spare {
                Documents::with_spare_cell()
            } else {
                Documents::new()
            },
            over: Some(ModelInstance {
                population: key(population),
                key: object.to_owned(),
            }),
            ..Self::new(body)
        }
    }

    /// The unit with `body`, `SB` over one of `x`, `y` or `z` in the cells
    /// population.
    fn over_cells(body: &str, object: &str) -> Self {
        let mut case = Self::over_object(body, "cells", object, false);
        case.documents = Documents::with_cells(&["x", "y", "z"]);
        case
    }

    /// The unit's checked package.
    pub(crate) fn checked(&self) -> &CheckedPackage {
        &self.checked
    }

    pub(crate) fn temporal_clock_selections(&self) -> &[fsl_package::TemporalClockSelection] {
        &self.temporal_clock_selections
    }

    pub(crate) fn subject(&self, selection: &[MemorySelection]) -> ProtocolSubject<'_> {
        let mut subject = self.documents.subject(&self.checked, "SB", selection);
        if let Some(over) = &self.over {
            subject.over = over.clone();
        }
        subject
    }

    /// EN-1's classification of `SB` under `selection`.
    pub(crate) fn accesses(&self, selection: &[MemorySelection]) -> WeakAccesses {
        let subject = self.subject(selection);
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        let protocol = self.checked.graph().protocol("SB").expect("SB");
        classify_accesses(
            &model,
            protocol,
            &subject.memory,
            &subject.over,
            &subject.capacities,
        )
        .expect("classifies")
    }

    /// The obligation identity of the deadlock-freedom item over `SB`
    /// under `selection`, written from the subject and the clause alone.
    pub(crate) fn obligation(
        &self,
        selection: &[MemorySelection],
    ) -> impl std::fmt::Debug + PartialEq {
        let [clause] = self.checked.graph().temporal_clauses() else {
            panic!("one declared clause");
        };
        let subject = self.subject(selection);
        let context = SubjectContext::new(&subject.model).expect("the package emits");
        let mut written = ModelRequest::new();
        let index = context
            .author_clause(&mut written, clause)
            .expect("the clause has a record");
        written.obligation(index).cloned().expect("an obligation")
    }

    /// The concrete protocol system of `SB` under `memory`: state keys keep
    /// every message ordinal and delivery identity (FR-205).
    pub(crate) fn concrete_system<M: MemoryModel>(
        &self,
        selection: &[MemorySelection],
        memory: M,
    ) -> Result<ProtocolSystem<'_, M>, ProtocolRefusal> {
        Self::concrete_build(&self.subject(selection), memory)
    }

    /// The protocol system of `SB` under `memory`, using the checker's cycle keys.
    pub(crate) fn system<M: MemoryModel>(
        &self,
        selection: &[MemorySelection],
        memory: M,
    ) -> Result<ProtocolSystem<'_, M>, ProtocolRefusal> {
        Self::build(&self.subject(selection), memory)
    }

    /// [`Self::subject`] with each expansion bound to `max_candidates`, and
    /// `absent` further `Cell` keys in the universe that the initial state
    /// does not hold.
    fn subject_within(
        &self,
        selection: &[MemorySelection],
        max_candidates: u64,
        absent: usize,
    ) -> ProtocolSubject<'_> {
        let mut subject = self.subject(selection);
        subject.model.simulation.expansion.max_candidates = max_candidates;
        let cells = subject
            .model
            .simulation
            .universes
            .iter_mut()
            .find(|universe| universe.population == key("cells"))
            .expect("the cells universe");
        cells.keys.extend((0..absent).map(|n| format!("absent{n}")));
        subject
    }

    /// [`Self::system`] over [`Self::subject_within`].
    fn system_within<M: MemoryModel>(
        &self,
        selection: &[MemorySelection],
        memory: M,
        max_candidates: u64,
        absent: usize,
    ) -> Result<ProtocolSystem<'_, M>, ProtocolRefusal> {
        Self::build(
            &self.subject_within(selection, max_candidates, absent),
            memory,
        )
    }

    /// The deadlock-freedom item over [`Self::subject_within`], and its
    /// FR-127 record.
    fn deadlock_within(
        &self,
        selection: &[MemorySelection],
        max_candidates: u64,
        absent: usize,
    ) -> (
        ModelCheckOutcome<ProtocolTransition>,
        ModelCheckRecord<ProtocolTransition>,
    ) {
        let outcome = check_protocol(
            ProtocolCheckRequest {
                workers: 1,
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject_within(selection, max_candidates, absent),
                item: ModelCheckItem::DeadlockFreedom,
                max_depth: None,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks");
        let record = settle_protocol_check(
            &outcome,
            &ModelCheckLimits::default(),
            &ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject_within(selection, max_candidates, absent),
                item: ModelCheckItem::DeadlockFreedom,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            &Cancel::new(),
            |payload| self.replay(selection, payload.clone()),
        );
        (outcome, record)
    }

    fn concrete_build<'s, M: MemoryModel>(
        subject: &ProtocolSubject<'s>,
        memory: M,
    ) -> Result<ProtocolSystem<'s, M>, ProtocolRefusal> {
        let model = ModelSystem::new(subject.model.simulation.clone()).expect("admits");
        ProtocolSystem::new(model, subject, memory)
    }

    fn build<'s, M: MemoryModel>(
        subject: &ProtocolSubject<'s>,
        memory: M,
    ) -> Result<ProtocolSystem<'s, M>, ProtocolRefusal> {
        Self::concrete_build(subject, memory).map(|system| system.with_cycle_keys(BTreeSet::new()))
    }

    /// EN-1 over `SB` with `item`, the selection and the store buffer bound.
    pub(crate) fn check(
        &self,
        selection: &[MemorySelection],
        item: ModelCheckItem<'_>,
        max_store_buffer: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        self.check_bounded(selection, item, max_store_buffer, None, None)
    }

    /// [`Self::check`] under the optional message and instance bounds too.
    fn check_bounded(
        &self,
        selection: &[MemorySelection],
        item: ModelCheckItem<'_>,
        max_store_buffer: Option<u64>,
        max_messages: Option<u64>,
        max_live_instances: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                workers: 1,
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(selection),
                item,
                max_depth: None,
                max_store_buffer,
                max_messages,
                max_live_instances,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// EN-1 over `SB`'s claim.
    pub(crate) fn claim(
        &self,
        selection: &[MemorySelection],
        max_store_buffer: Option<u64>,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        let clause = self.checked.graph().temporal_clauses()[0].clause();
        self.check(
            selection,
            ModelCheckItem::Clause {
                clause,
                letters: self.letters,
            },
            max_store_buffer,
        )
    }

    /// FR-127's record of `outcome` over the claim (the deadlock-freedom
    /// item when `deadlock`), its counterexample replayed by FR-217.
    pub(crate) fn settle(
        &self,
        selection: &[MemorySelection],
        outcome: &ModelCheckOutcome<ProtocolTransition>,
        deadlock: bool,
        max_store_buffer: Option<u64>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        self.settle_with_messages(selection, outcome, deadlock, max_store_buffer, None)
    }

    /// [`Self::settle`] over a subject whose protocol name is `protocol`.
    pub(crate) fn settle_naming(
        &self,
        selection: &[MemorySelection],
        outcome: &ModelCheckOutcome<ProtocolTransition>,
        max_store_buffer: Option<u64>,
        protocol: &str,
    ) -> ModelCheckRecord<ProtocolTransition> {
        let mut subject = self.subject(selection);
        protocol.clone_into(&mut subject.protocol);
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject,
                item: ModelCheckItem::DeadlockFreedom,
                max_store_buffer,
                max_messages: None,
                max_live_instances: None,
            },
            &Cancel::new(),
            |payload| self.replay(selection, payload.clone()),
        )
    }

    fn settle_with_messages(
        &self,
        selection: &[MemorySelection],
        outcome: &ModelCheckOutcome<ProtocolTransition>,
        deadlock: bool,
        max_store_buffer: Option<u64>,
        max_messages: Option<u64>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        let item = if deadlock {
            ModelCheckItem::DeadlockFreedom
        } else {
            ModelCheckItem::Clause {
                clause: self.checked.graph().temporal_clauses()[0].clause(),
                letters: self.letters,
            }
        };
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(selection),
                item,
                max_store_buffer,
                max_messages,
                max_live_instances: None,
            },
            &Cancel::new(),
            |payload| self.replay(selection, payload.clone()),
        )
    }

    /// FR-127's record of `outcome` over the race-freedom item (FR-224), its
    /// counterexample replayed by FR-217.
    pub(crate) fn settle_race(
        &self,
        selection: &[MemorySelection],
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(selection),
                item: ModelCheckItem::RaceFreedom,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            &Cancel::new(),
            |payload| self.replay_race(selection, payload.clone()),
        )
    }

    /// FR-217's replay of `payload`, a counterexample to the race-freedom
    /// item, under `selection`'s resolved models.
    pub(crate) fn replay_race(
        &self,
        selection: &[MemorySelection],
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text_with(self.profiles, &self.body).into_bytes();
        let (node, occurrence) = crate::model_settle_support::protocol_anchor(&self.checked, "SB");
        replay_protocol_trace(
            request(
                &self.checked,
                &[],
                &document(),
                self.documents.snapshots.iter(),
                &unit,
            ),
            &self.trace_subject(selection),
            &envelope_at(payload, node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }

    /// The subject FR-217's replay reads, its resolved models `selection`'s
    /// (FR-227 Inputs).
    pub(crate) fn trace_subject(&self, selection: &[MemorySelection]) -> ProtocolTraceSubject {
        let subject = self.subject(selection);
        ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: vec![self.documents.initial.clone()],
                universes: self.documents.universes(),
                expansion: subject.model.simulation.expansion,
                stutter_closure: false,
            },
            protocol: "SB".to_owned(),
            over: core(),
            roles: subject.roles.clone(),
            capacities: Vec::new(),
            memory: subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        }
    }

    /// FR-217's replay of `payload`, a counterexample to the claim, with
    /// the subject's resolved models replaced by `selection`'s.
    pub(crate) fn replay(
        &self,
        selection: &[MemorySelection],
        payload: TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let unit = unit_text_with(self.profiles, &self.body).into_bytes();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = node.identity().clone();
        let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
        replay_protocol_trace(
            request(
                &self.checked,
                &[],
                &document(),
                self.documents.snapshots.iter(),
                &unit,
            ),
            &self.trace_subject(selection),
            &envelope_at(payload, clause_node, occurrence),
            self.letters,
            ReplayLimits::default(),
        )
    }
}

/// The claim's one atom, `not (k.r1 = 0 and k.r2 = 0)`, read from the
/// bound core's registers.
pub(crate) struct NoBothZero;

impl ModelLetters for NoBothZero {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let both = field(state, "cores", &instance.key, "r1") == 0
            && field(state, "cores", &instance.key, "r2") == 0;
        Ok(if both {
            AtomReading::False
        } else {
            AtomReading::True
        })
    }
}

/// The branch thread `label` of `Both`'s first instance.
pub(crate) fn branch(label: &str) -> ThreadPath {
    ThreadPath::root(0).child("Both", label, 0)
}

/// ADR-025 §10's counts: states with a live branch thread, and states
/// whose root rests at `finish`.
pub(crate) fn counts<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    reached: &[Reached<M>],
) -> (usize, usize) {
    let mut inside = 0;
    let mut joined = 0;
    for state in reached {
        let threads = system.threads(&state.state).expect("threads");
        if threads.iter().any(|thread| !thread.path.steps.is_empty()) {
            inside += 1;
        } else if threads.iter().any(|thread| {
            thread.rest_point.kind == RestKind::Node
                && thread.rest_point.node.as_deref() == Some("Done")
        }) {
            joined += 1;
        }
    }
    (inside, joined)
}

/// The `(object key, field name)` of a location `accesses` has a cell for.
fn named(accesses: &WeakAccesses, location: &Location) -> (String, String) {
    let cell = accesses.cell(location).expect("a touched field has a cell");
    (cell.key.clone(), cell.name.clone())
}

/// The class of each instance of `node` and its shared location's name.
fn classes(accesses: &WeakAccesses, node: &str) -> Vec<(AccessClass, (String, String))> {
    accesses
        .attempt(node)
        .expect("a weak attempt")
        .instances
        .iter()
        .map(|instance| match &instance.access {
            InstanceAccess::Access {
                class,
                location: Some(location),
            } => (*class, named(accesses, location)),
            other => panic!("{node}: one shared location, got {other:?}"),
        })
        .collect()
}

/// The names of the locations `node`'s instances touch that are not
/// shared, field locations only.
fn local(accesses: &WeakAccesses, node: &str) -> BTreeSet<(String, String)> {
    accesses
        .attempt(node)
        .expect("a weak attempt")
        .instances
        .iter()
        .flat_map(|instance| instance.local.iter())
        .filter(|location| matches!(location, Location::Field { .. }))
        .map(|location| named(accesses, location))
        .collect()
}

fn name(key: &str, field: &str) -> (String, String) {
    (key.to_owned(), field.to_owned())
}

/// A `parallel` nested in `SB::Both`'s left branch.
fn nested_unit(inner: &str) -> String {
    let run = format!(
        "parallel Both {{ \
         branch left sequence L {{ {} parallel Inner {inner} {{ branch a sequence A {{ }} \
         branch b sequence B {{ }} }} join all [a, b]; {} }} \
         branch right sequence R {{ {} {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", "release", "IsX, StoreOne"),
        attempt("Ly", "c", "Core::read1", "acquire", "ReadsY1, Read1"),
        attempt("Sy", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Lx", "c", "Core::read2", "acquire", "ReadsX2, Read2"),
    );
    sb_unit(&run)
}

pub(crate) fn steps(name: &str, attempts: &[String]) -> String {
    format!("sequence {name} {{ {} }}", attempts.concat())
}

/// The store `node` of `value` (`StoreOne` or `StoreTwo`) to `cell`.
pub(crate) fn store(node: &str, cell: &str, value: &str) -> String {
    let select = if cell == "x" { "IsX" } else { "IsY" };
    attempt(
        node,
        "w",
        "Cell::set",
        "relaxed",
        &format!("{select}, {value}"),
    )
}

/// The `relaxed` load `node` of `cell` into register `n`.
pub(crate) fn load(node: &str, cell: &str, n: u8) -> String {
    load_ordered(node, cell, n, "relaxed")
}

/// The load `node` of `cell` into register `n` with `ordering` (empty for
/// none, as a local access must be).
pub(crate) fn load_ordered(node: &str, cell: &str, n: u8, ordering: &str) -> String {
    let select = if cell == "x" { "ReadsX" } else { "ReadsY" };
    attempt(
        node,
        "c",
        &format!("Core::read{n}"),
        ordering,
        &format!("{select}{n}, Read{n}"),
    )
}
