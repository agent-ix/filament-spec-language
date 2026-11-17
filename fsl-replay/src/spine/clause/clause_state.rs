// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820: a temporal clause advanced one position at a time from carried
//! evaluation state.
//!
//! A [`ClauseState`] reads each admitted record sample once, when it arrives,
//! into its letter (`fsl_eval::temporal::LetterTape`): the value of every
//! atom of the clause there, a window aggregate's included, which reads the
//! samples of its window while they are still held. Every later evaluation
//! is FR-330's own run over the letters, so each report is the batch report
//! over the prefix by construction, and the state holds a sample by value
//! only while a window aggregate of the newest position reads it or where a
//! reading did not reduce to a letter. Each position's identity is kept for
//! the counterexample the clause may still report.
//!
//! A `Progressed` or `Timestamped` trace's finite-window evaluation (FR-501),
//! and FR-504's open-scope reading behind `decided`, are carried between
//! steps (`CarriedWindow`): each evaluation reads only the positions an open
//! activation can still read. A `Timestamped` trace's letters take their
//! canonical order once progress covers them.
//!
//! Snapshot positions are admitted once and read into letters as well
//! (`snapshot`): the state keeps a snapshot's bytes only where a decisive
//! record can still read it, counted in [`ClauseState::retained`].

mod snapshot;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use filament_exact::{Cancel, CancelCause, Meter, Value, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::temporal::{
    push_sample, read_sample_letter, read_sample_stop, CarriedWindow, ClauseStream, FirstUndefined,
    InsufficientData, LetterTape, ObservedRun, StreamStop, TapeEntry, TapeLetter, TapeReading,
    TemporalOutcome,
};
use fsl_foundation::source::Source;
use fsl_semantics::check::{AggregateWindow, FormulaNode, TemporalClauseNode, TemporalProfile};
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    admit_record_sample, DocumentProvision, DocumentRef, ObservationLimits,
};
use serde_json::{json, Value as Json};

use super::package_authentication::PackageAuthentication;
use super::progress::{
    ProgressAssertion, ProgressClosure, ProgressCompleteness, ProgressLedger, ProgressLowerBoundary,
};
use super::temporal::with_subject_counterexample;
use super::temporal::{
    clause_trace_binding, run_package_temporal_over, streamed_sample_report, AdmissionMark,
    CarriedAdmission, PackageTemporalRequest, TapeInput, TimestampedCover, TimestampedObservation,
    TimestampedOrderKey, TimestampedWindow, TraceBinding, TraceInput,
};
use super::trace_subject::{
    decisive_reach, push_subject_sample, read_subject_letter, reread_subject_stop, subject_report,
    SubjectSamples,
};
use super::{
    ClauseDisposition, ClauseRunReport, ClauseRunUsage, ConsequentSourceEvents, OverObject,
    ProgressRefusal, TemporalVerdict,
};

/// The format member of [`ClauseState::to_bytes`].
pub const CLAUSE_STATE_FORMAT: &str = "filament.clause-state/1";

/// The kind of trace a carried clause state reads (FR-641).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseTraceKind {
    /// A finite trace or prefix.
    Finite,
    /// A finite event-position or fixed-sample trace with progress.
    Progressed,
    /// A timestamped-event trace with progress.
    Timestamped,
}

impl ClauseTraceKind {
    fn carries_progress(self) -> bool {
        matches!(self, Self::Progressed | Self::Timestamped)
    }
}

/// The limits each evaluation of a carried clause state runs under, as the
/// batch request carries them.
#[derive(Clone, Copy, Debug)]
pub struct ClauseLimits {
    /// The limits admission re-normalizes the domain packages under.
    pub model: fsl_semantics::model::accounting::ModelNormalizationLimits,
    /// FR-106's observation limits.
    pub observation: ObservationLimits,
    /// The evaluation meter's accounting limits.
    pub accounting: filament_exact::ScalarLimits,
    /// FR-194's window limit.
    pub max_window_values: u64,
}

/// FR-820's opening request: the members FR-641 gives the trace kind, as the
/// batch request (FR-330) carries them, and what each run reads.
#[derive(Clone)]
pub struct ClauseOpen<'a> {
    /// The trace kind.
    pub kind: ClauseTraceKind,
    /// Whether the first position is an authoritative origin.
    pub lower_boundary: ProgressLowerBoundary,
    /// The trace binding of a `Progressed` or `Timestamped` trace.
    pub binding: Option<TraceBinding>,
    /// How a `Timestamped` trace's stamps cover time.
    pub cover: Option<TimestampedCover>,
    /// The progress ledger the trace starts from (FR-502).
    pub ledger: Option<ProgressLedger>,
    /// The sources the package was checked from.
    pub sources: &'a [Source],
    /// What a clock-bearing trace is authenticated against; `None` reads it
    /// unauthenticated, as a `Finite` trace needs no binding.
    pub authentication: Option<Arc<PackageAuthentication>>,
    /// FR-056's package input.
    pub packages: &'a PackageInput,
    /// The object a model-subject clause's `over` parameter names.
    pub over: Option<OverObject>,
    /// The limits of each run.
    pub limits: ClauseLimits,
}

/// One position the caller admitted once (FR-106, FR-641).
#[derive(Clone, Debug)]
pub struct AdmittedPosition<'p> {
    /// The position's identity.
    pub reference: DocumentRef,
    /// Its stamp: the tick of a `Timestamped` trace, else its index.
    pub stamp: u64,
    /// Its order key, for a `Timestamped` trace.
    pub order_key: Option<TimestampedOrderKey>,
    /// The admitted value.
    pub value: AdmittedValue<'p>,
}

/// An admitted position's value.
#[derive(Clone, Debug)]
pub enum AdmittedValue<'p> {
    /// A record sample of a declared record trace subject.
    Sample(Value),
    /// A snapshot document's bytes.
    Snapshot(&'p [u8]),
}

/// What one call of a carried clause state returns (FR-820 Outputs).
#[derive(Clone, Debug)]
pub struct Advance {
    /// The clause's report over the positions seen, under the latest
    /// progress.
    pub report: Arc<ClauseRunReport>,
    /// The definite verdict of the clause's profile over the positions seen,
    /// when it has one: FR-504's open-scope reading of an event-position or
    /// fixed-sample clause with a decisive basis, an infinite-trace
    /// violation, or a decisive finite-window verdict.
    pub decided: Option<Arc<ClauseRunReport>>,
}

/// Why a carried clause state refused a call; the state is unchanged.
#[derive(Debug)]
pub enum ClauseRefusal {
    /// The caller cancelled the call.
    Cancelled(CancelCause),
    /// The progress the call asserts contradicts the retained progress
    /// (FR-502).
    Contradiction(Box<ProgressRefusal>),
    /// A state document that is not this clause's closed
    /// `filament.clause-state/1`.
    StateMismatch,
    /// An invariant of the state broke.
    Fault(InternalFault),
}

/// How a state reads its positions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reading {
    /// FR-330-AC-17's trace-subject letters (a finite event-position trace).
    Subject,
    /// FR-330's observed letters over record samples.
    Observed,
    /// FR-820's streaming evaluation of an infinite-trace clause over a
    /// `Finite` trace of record samples.
    Streamed,
    /// FR-820: snapshot positions (`over` names a model object), each
    /// admitted once and read into its letter.
    Snapshot,
    /// Every position held by value.
    Values,
    /// FR-330 refuses the run before it reads a position: an infinite-trace
    /// clause over a `Finite` trace of record samples with a window aggregate
    /// or a ledger, or a profile that does not read a `Finite` trace. The
    /// refusal names no position, so one position stands for the prefix and
    /// no position is held.
    Refused,
}

impl Reading {
    fn name(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Observed => "observed",
            Self::Streamed => "streamed",
            Self::Snapshot => "snapshot",
            Self::Values => "values",
            Self::Refused => "refused",
        }
    }
}

#[derive(Clone, Debug)]
struct Seen {
    reference: DocumentRef,
    stamp: u64,
    order_key: Option<TimestampedOrderKey>,
}

/// FR-501 "Canonical order" of two positions, as the batch run orders a
/// `Timestamped` trace: ascending stamp, then order authority and key (a
/// keyed position before an unkeyed one), then identity.
fn canonical_order(left: &Seen, right: &Seen) -> std::cmp::Ordering {
    let key = |seen: &Seen| {
        seen.order_key
            .as_ref()
            .map(|order| (order.authority.clone(), order.key))
    };
    left.stamp
        .cmp(&right.stamp)
        .then_with(|| match (key(left), key(right)) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .then_with(|| left.reference.cmp(&right.reference))
}

/// FR-820's carried state of one clause over one trace.
#[derive(Clone)]
pub struct ClauseState<'a> {
    package: &'a fsl_package::CheckedPackage,
    node: &'a TemporalClauseNode,
    open: ClauseOpen<'a>,
    reading: Reading,
    seen: Vec<Seen>,
    tape: LetterTape,
    /// The samples held by value, by position.
    values: BTreeMap<u64, Value>,
    /// The positions held by value because a reading there did not reduce.
    pinned: BTreeSet<u64>,
    /// The captures fixed at the origin activation.
    captures: Vec<Value>,
    snapshots: DocumentProvision,
    progress: Option<ProgressAssertion>,
    /// The ledger each run starts from.
    ledger: Option<ProgressLedger>,
    report: Arc<ClauseRunReport>,
    /// The positions seen and the progress the latest report read.
    evaluated: Option<(usize, Option<ProgressAssertion>)>,
    /// The carried evaluation of a streamed reading.
    stream: Option<ClauseStream>,
    /// The position at which a streamed reading stopped, held by value so
    /// its stop is read again for each report.
    stop: Option<(u64, Value)>,
    /// The work limit a streamed reading reached, which ends its evaluation.
    exhausted: Option<filament_exact::Incomplete>,
    /// The atom values a trace-subject stream read at the positions its
    /// decisive aggregate may still read, by node and position.
    atom_values: BTreeMap<(usize, u64), Option<bool>>,
    /// The last atom a trace-subject stream read short of data.
    undefined_atom: Option<usize>,
    /// The first undefined reading a trace-subject stream met.
    first_undefined: Option<FirstUndefined>,
    /// The canonical text of each record sample held by value, by
    /// position, for a state that cites its samples by index.
    texts: BTreeMap<u64, String>,
    /// FR-820: the carried finite-window evaluation of the report's run over
    /// a `Progressed` or `Timestamped` trace.
    window: RefCell<CarriedWindow>,
    /// FR-820: the carried finite-window evaluation of FR-504's open-scope
    /// reading, which `decided` reports.
    open_window: RefCell<CarriedWindow>,
    /// The snapshot document the latest advance added, which a restore
    /// removes again.
    fresh_snapshot: Option<DocumentRef>,
    /// What a snapshot state read of its positions besides the tape.
    objects: snapshot::SnapshotRead,
    /// The decisive atom of a snapshot state's activation violated for
    /// good, once its descent found it.
    snapshot_cache: RefCell<Option<(u64, Option<fsl_eval::temporal::DecisiveAtom>)>>,
    /// FR-820: what the report's runs admitted of the trace and the ledger.
    admission: RefCell<CarriedAdmission>,
    /// FR-820: what FR-504's open-scope runs admitted of the trace.
    open_admission: RefCell<CarriedAdmission>,
    /// FR-820: the trace the report's runs read, listed once per position.
    trace: RefCell<Option<TraceInput>>,
    /// FR-820: the trace FR-504's open-scope runs read, listed the same way.
    open_trace: RefCell<Option<TraceInput>>,
}

/// FR-820 "No partial state": what [`ClauseState::restore`] puts back, the
/// state before one call. The positions and letters the state appends are
/// kept as their counts, so a mark costs the bounded part of the state, not
/// the trace.
#[derive(Clone)]
pub struct ClauseMark {
    seen: usize,
    tape: usize,
    snapshots: usize,
    fresh_snapshot: Option<DocumentRef>,
    values: BTreeMap<u64, Value>,
    pinned: BTreeSet<u64>,
    captures: Vec<Value>,
    progress: Option<ProgressAssertion>,
    ledger: Option<ProgressLedger>,
    report: Arc<ClauseRunReport>,
    evaluated: Option<(usize, Option<ProgressAssertion>)>,
    stream: Option<ClauseStream>,
    stop: Option<(u64, Value)>,
    exhausted: Option<filament_exact::Incomplete>,
    atom_values: BTreeMap<(usize, u64), Option<bool>>,
    undefined_atom: Option<usize>,
    first_undefined: Option<FirstUndefined>,
    texts: BTreeMap<u64, String>,
    /// A `Timestamped` state's positions since the last evaluation, which an
    /// evaluation puts in canonical order.
    tail: Option<(usize, Vec<Seen>, Vec<TapeEntry>)>,
    window: CarriedWindow,
    open_window: CarriedWindow,
    objects: snapshot::SnapshotRead,
    snapshot_cache: Option<(u64, Option<fsl_eval::temporal::DecisiveAtom>)>,
    admission: AdmissionMark,
    open_admission: AdmissionMark,
}

/// The `pending` report of a clause no evaluation has read yet.
pub fn pending_report() -> ClauseRunReport {
    let verdict = TemporalVerdict::Pending;
    ClauseRunReport {
        basis: verdict.basis(),
        disposition: ClauseDisposition::Temporal(verdict),
        usage: ClauseRunUsage::default(),
        witness: None,
        consequent_source_events: ConsequentSourceEvents::Unavailable,
        ledger: None,
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
    }
}

/// The state form of FR-100's incomplete record.
pub fn incomplete_json(incomplete: &filament_exact::Incomplete) -> Json {
    json!({
        "limit_kind": incomplete.limit_kind.as_str(),
        "limit": incomplete.limit,
        "consumed": incomplete.consumed,
        "next_charge": incomplete.next_charge.to_string(),
        "charge_point": incomplete.charge_point.as_str(),
    })
}

/// The incomplete record [`incomplete_json`] wrote.
pub fn incomplete_of(value: &Json) -> Option<filament_exact::Incomplete> {
    let text = |member: &str| value[member].as_str();
    Some(filament_exact::Incomplete {
        limit_kind: filament_exact::LimitKind::ALL
            .into_iter()
            .find(|kind| Some(kind.as_str()) == text("limit_kind"))?,
        limit: value["limit"].as_u64()?,
        consumed: value["consumed"].as_u64()?,
        next_charge: text("next_charge")?.parse().ok()?,
        charge_point: filament_exact::ChargePoint::ALL
            .into_iter()
            .find(|point| Some(point.as_str()) == text("charge_point"))?,
    })
}

/// FR-330-AC-18: the identity of the record sample at `index`, its index.
fn index_reference(index: u64) -> DocumentRef {
    DocumentRef {
        authority: super::temporal::SAMPLE_AUTHORITY.to_owned(),
        identity: index.to_string(),
    }
}

fn fault(what: &'static str) -> ClauseRefusal {
    ClauseRefusal::Fault(InternalFault::new("clause-state", what))
}

fn cancelled(cancel: &Cancel) -> Result<(), ClauseRefusal> {
    match cancel.cause() {
        Some(cause) => Err(ClauseRefusal::Cancelled(cause)),
        None => Ok(()),
    }
}

/// Whether the finite-window report of `report` has a decisive basis.
fn decisive_window(report: &ClauseRunReport) -> bool {
    report
        .finite_window
        .as_ref()
        .is_some_and(|window| window.basis.is_decisive())
}

impl<'a> ClauseState<'a> {
    /// FR-820: open the carried state of `clause` over a trace of
    /// `request.kind`.
    ///
    /// # Errors
    ///
    /// A fault for a clause the package does not declare.
    pub fn open(
        package: &'a fsl_package::CheckedPackage,
        clause: &'a TemporalClauseNode,
        request: &ClauseOpen<'a>,
    ) -> Result<Self, ClauseRefusal> {
        let declared = package
            .graph()
            .temporal_clauses()
            .iter()
            .any(|node| std::ptr::eq(node, clause));
        if !declared {
            return Err(fault("clause-is-declared-in-the-package"));
        }
        let sampled =
            request.over.is_none() && matches!(clause.clause().over().1, ValueType::Composite(_));
        let aggregates = clause
            .clause()
            .formula()
            .nodes()
            .iter()
            .any(|node| matches!(node, FormulaNode::Aggregate { .. }));
        let reading = match (sampled, request.kind, clause.clause().profile()) {
            // A `Finite` snapshot trace's report is the stream's, for the
            // profiles the stream reads; FR-330 refuses a ledger there before
            // it admits a snapshot. A clause whose `over` is not a reference is
            // refused before admission too.
            (false, kind, profile)
                if request.over.is_some()
                    && matches!(clause.clause().over().1, ValueType::Reference(_))
                    && (kind != ClauseTraceKind::Finite
                        || (request.ledger.is_none()
                            && matches!(
                                profile,
                                TemporalProfile::InfiniteTrace
                                    | TemporalProfile::EventPositionFalseExtension
                                    | TemporalProfile::FixedSampleFalseExtension
                            ))) =>
            {
                Reading::Snapshot
            }
            (false, ..) => Reading::Values,
            (true, ClauseTraceKind::Finite, TemporalProfile::EventPositionFalseExtension) => {
                Reading::Subject
            }
            // FR-330 refuses a window aggregate and a ledger on this route
            // before it evaluates; those runs keep the batch reading.
            (
                true,
                ClauseTraceKind::Finite,
                TemporalProfile::InfiniteTrace | TemporalProfile::FixedSampleFalseExtension,
            ) if !aggregates && request.ledger.is_none() => Reading::Streamed,
            (
                true,
                ClauseTraceKind::Finite,
                TemporalProfile::InfiniteTrace
                | TemporalProfile::TimestampedEventFiniteWindow
                | TemporalProfile::Timed,
            ) => Reading::Refused,
            (true, ..) => Reading::Observed,
        };
        let mut exhausted = None;
        let stream = match reading {
            Reading::Streamed | Reading::Subject => {
                let mut meter = Meter::new(request.limits.accounting);
                match ClauseStream::open(clause.clause(), &mut meter) {
                    Ok(Ok(stream)) => Some(stream),
                    Ok(Err(StreamStop::Incomplete(incomplete))) => {
                        exhausted = Some(incomplete);
                        None
                    }
                    Err(_) => return Err(fault("infinite-trace-clause-streams")),
                }
            }
            // A snapshot stream carries the report of a `Finite` trace and
            // the descent floor of every trace with a decisive record; a
            // profile it does not read is refused by the run before any
            // position is read.
            Reading::Snapshot if request.kind != ClauseTraceKind::Timestamped => {
                let mut meter = Meter::new(request.limits.accounting);
                match ClauseStream::open(clause.clause(), &mut meter) {
                    Ok(Ok(stream)) => Some(stream),
                    Ok(Err(StreamStop::Incomplete(incomplete))) => {
                        exhausted = Some(incomplete);
                        None
                    }
                    Err(_) => None,
                }
            }
            Reading::Observed | Reading::Values | Reading::Snapshot | Reading::Refused => None,
        };
        Ok(Self {
            stream,
            stop: None,
            exhausted,
            package,
            node: clause,
            open: request.clone(),
            reading,
            seen: Vec::new(),
            tape: LetterTape::new(),
            values: BTreeMap::new(),
            pinned: BTreeSet::new(),
            captures: Vec::new(),
            snapshots: DocumentProvision::new(),
            progress: None,
            ledger: request.ledger.clone(),
            report: Arc::new(pending_report()),
            evaluated: None,
            atom_values: BTreeMap::new(),
            undefined_atom: None,
            first_undefined: None,
            texts: BTreeMap::new(),
            fresh_snapshot: None,
            window: RefCell::new(CarriedWindow::default()),
            open_window: RefCell::new(CarriedWindow::default()),
            objects: snapshot::SnapshotRead::default(),
            snapshot_cache: RefCell::new(None),
            admission: RefCell::new(CarriedAdmission::default()),
            open_admission: RefCell::new(CarriedAdmission::default()),
            trace: RefCell::new(None),
            open_trace: RefCell::new(None),
        })
    }

    /// FR-820 "No partial state": the state as it is now, for
    /// [`Self::restore`] to put back after a later call.
    pub fn mark(&self) -> ClauseMark {
        ClauseMark {
            seen: self.seen.len(),
            tape: self.tape.len(),
            snapshots: self.snapshots.len(),
            fresh_snapshot: self.fresh_snapshot.clone(),
            values: self.values.clone(),
            pinned: self.pinned.clone(),
            captures: self.captures.clone(),
            progress: self.progress,
            ledger: self.ledger.clone(),
            report: Arc::clone(&self.report),
            evaluated: self.evaluated,
            stream: self.stream.clone(),
            stop: self.stop.clone(),
            exhausted: self.exhausted.clone(),
            atom_values: self.atom_values.clone(),
            undefined_atom: self.undefined_atom,
            first_undefined: self.first_undefined.clone(),
            texts: self.texts.clone(),
            tail: (self.open.kind == ClauseTraceKind::Timestamped
                && matches!(self.reading, Reading::Observed | Reading::Snapshot))
            .then(|| {
                let from = self
                    .evaluated
                    .map_or(0, |(seen, _)| seen)
                    .min(self.seen.len());
                (
                    from,
                    self.seen.get(from..).unwrap_or(&[]).to_vec(),
                    self.tape.entries().get(from..).unwrap_or(&[]).to_vec(),
                )
            }),
            window: self.window.borrow().clone(),
            open_window: self.open_window.borrow().clone(),
            objects: self.objects.clone(),
            snapshot_cache: *self.snapshot_cache.borrow(),
            admission: self.admission.borrow().mark(),
            open_admission: self.open_admission.borrow().mark(),
        }
    }

    /// FR-820 "No partial state": put back the state `mark` was taken of,
    /// undoing the one call made since.
    pub fn restore(&mut self, mark: ClauseMark) {
        if let Some((from, seen, entries)) = mark.tail {
            self.seen.truncate(from);
            self.seen.extend(seen);
            self.tape.truncate(from);
            for entry in entries {
                self.tape.push(entry);
            }
        }
        self.seen.truncate(mark.seen);
        self.tape.truncate(mark.tape);
        if self.snapshots.len() > mark.snapshots {
            if let Some(reference) = self.fresh_snapshot.take() {
                self.snapshots.remove(&reference);
            }
        }
        self.fresh_snapshot = mark.fresh_snapshot;
        self.values = mark.values;
        self.pinned = mark.pinned;
        self.captures = mark.captures;
        self.progress = mark.progress;
        self.ledger = mark.ledger;
        self.report = mark.report;
        self.evaluated = mark.evaluated;
        self.stream = mark.stream;
        self.stop = mark.stop;
        self.exhausted = mark.exhausted;
        self.atom_values = mark.atom_values;
        self.undefined_atom = mark.undefined_atom;
        self.first_undefined = mark.first_undefined;
        self.texts = mark.texts;
        self.window.replace(mark.window);
        self.open_window.replace(mark.open_window);
        self.objects = mark.objects;
        self.snapshot_cache.replace(mark.snapshot_cache);
        self.admission.get_mut().restore(mark.admission);
        self.open_admission.get_mut().restore(mark.open_admission);
        truncate_listing(&mut self.trace, self.seen.len());
        truncate_listing(&mut self.open_trace, self.seen.len());
    }

    /// The ledger each later run starts from: a monitor carries one ledger
    /// across its clauses (FR-643).
    pub fn carry_ledger(&mut self, ledger: Option<ProgressLedger>) {
        self.ledger = ledger;
    }

    /// The farthest offset a window aggregate of the clause reads back.
    fn window_reach(&self) -> Option<u64> {
        self.node
            .clause()
            .aggregates()
            .values()
            .filter_map(|aggregate| match &aggregate.window {
                AggregateWindow::Positions { upper, .. } => Some(*upper),
                AggregateWindow::Duration(_) => None,
            })
            .max()
    }

    /// FR-820: advance the state by `position`.
    ///
    /// # Errors
    ///
    /// A cancelled call or a contradiction; the state is unchanged.
    pub fn advance(
        &mut self,
        position: AdmittedPosition<'_>,
        cancel: &Cancel,
    ) -> Result<Advance, ClauseRefusal> {
        cancelled(cancel)?;
        if self.reading == Reading::Streamed {
            return self.advance_streamed(position, cancel);
        }
        let index = u64::try_from(self.seen.len()).map_err(|_| fault("position-in-u64"))?;
        let mark = self.mark();
        let mut meter = Meter::new(self.open.limits.accounting).with_cancel(cancel.clone());
        self.fresh_snapshot = match &position.value {
            AdmittedValue::Snapshot(_) if self.reading == Reading::Snapshot => None,
            AdmittedValue::Snapshot(bytes) => {
                let fresh = !self.snapshots.contains_key(&position.reference);
                self.snapshots
                    .insert(position.reference.clone(), bytes.to_vec());
                fresh.then(|| position.reference.clone())
            }
            AdmittedValue::Sample(_) => None,
        };
        let document = position.reference.clone();
        let reference = self.cite(index, position.reference);
        self.seen.push(Seen {
            reference,
            stamp: position.stamp,
            order_key: position.order_key,
        });
        match position.value {
            AdmittedValue::Sample(sample) => self.read(index, sample, &mut meter),
            AdmittedValue::Snapshot(bytes) if self.reading == Reading::Snapshot => {
                self.push_snapshot(index, &document, bytes);
                // A `Timestamped` trace's positions are read once progress
                // covers them, in canonical order (`order_covered`).
                if self.open.kind != ClauseTraceKind::Timestamped {
                    self.read_pending(index, &mut meter);
                }
            }
            AdmittedValue::Snapshot(_) => {}
        }
        let evaluated =
            cancelled(cancel).and_then(|()| self.evaluate(self.progress, &mut meter, cancel));
        if evaluated.is_err() {
            self.restore(mark);
        }
        evaluated
    }

    /// FR-330-AC-18, FR-643 "Retained state": whether the state cites each
    /// record sample by its index as it arrives, holding a sample's text only
    /// while it holds the sample by value. A `Timestamped` observed state
    /// names a sample by its text until progress covers it, because the
    /// sample's value orders it among the positions of its stamp (FR-501
    /// "Canonical order"); [`Self::order_covered`] then cites it by index.
    /// A snapshot is cited by its labels, and the values reading holds every
    /// position by value.
    fn indexed(&self) -> bool {
        match self.reading {
            Reading::Subject | Reading::Streamed | Reading::Refused => true,
            Reading::Observed => self.open.kind != ClauseTraceKind::Timestamped,
            Reading::Values | Reading::Snapshot => false,
        }
    }

    /// Whether a streamed state also reads each sample into its letter: FR-504's
    /// open-scope reading of a fixed-sample clause (`decided`) is carried over
    /// the letters, as the trace-subject reading's is.
    fn lettered(&self) -> bool {
        self.reading == Reading::Streamed
            && self.node.clause().profile() == TemporalProfile::FixedSampleFalseExtension
    }

    /// The identity the state keeps for the position `index` the caller names
    /// `reference`: its index, keeping the sample's text while it is held, for
    /// an indexed state.
    fn cite(&mut self, index: u64, reference: DocumentRef) -> DocumentRef {
        if !self.indexed() || reference.authority != super::temporal::SAMPLE_AUTHORITY {
            return reference;
        }
        // FR-502: the ledger reads the sample by its content identity.
        let cited = super::temporal::sample_index(
            index,
            &super::temporal::sample_content(&reference.identity),
        );
        // A refused run reads no sample, so its text is never needed.
        if self.reading != Reading::Refused {
            self.texts.insert(index, reference.identity);
        }
        cited
    }

    /// FR-820: advance a streamed reading by `position`. The sample is read
    /// once into the stream, which keeps no position by value; only a
    /// position whose reading stopped is held, so each report reads its stop
    /// again.
    fn advance_streamed(
        &mut self,
        position: AdmittedPosition<'_>,
        cancel: &Cancel,
    ) -> Result<Advance, ClauseRefusal> {
        let index = u64::try_from(self.seen.len()).map_err(|_| fault("position-in-u64"))?;
        let AdmittedValue::Sample(sample) = position.value else {
            return Err(fault("streamed-position-is-a-sample"));
        };
        let mut meter = Meter::new(self.open.limits.accounting).with_cancel(cancel.clone());
        let mut stream = self.stream.clone();
        let mut exhausted = self.exhausted.clone();
        let mut stop = self.stop.clone();
        let mut stop_text = None;
        if let (Some(carried), None) = (stream.as_mut(), exhausted.as_ref()) {
            match push_sample(self.package, self.node, carried, &sample, &mut meter) {
                Ok(read) => {
                    if !read.is_empty() && stop.is_none() {
                        stop = Some((index, sample.clone()));
                        if self.indexed() {
                            stop_text = Some(position.reference.identity.clone());
                        }
                    }
                }
                Err(StreamStop::Incomplete(incomplete)) => {
                    cancelled(cancel)?;
                    exhausted = Some(incomplete);
                }
            }
        }
        let mut held_text = None;
        if self.lettered() {
            let mut letter_meter = Meter::new(self.open.limits.accounting);
            let entry = read_sample_letter(self.package, self.node, &sample, &mut letter_meter);
            // A sample the letter did not reduce is held, and its text with it.
            if matches!(entry, TapeEntry::Held(_)) {
                held_text = Some(position.reference.identity.clone());
            }
            self.tape.push(entry);
        }
        let reference = if self.indexed() {
            super::temporal::sample_index(
                index,
                &super::temporal::sample_content(&position.reference.identity),
            )
        } else {
            position.reference
        };
        self.seen.push(Seen {
            reference,
            stamp: position.stamp,
            order_key: position.order_key,
        });
        let report = self.streamed_report(
            stream.as_ref(),
            stop.as_ref(),
            exhausted.as_ref(),
            &mut meter,
        );
        let report = Arc::new(report);
        let open_window = self.open_window.borrow().clone();
        let decided = cancelled(cancel).and_then(|()| self.decided(&report, None, cancel));
        let decided = match decided {
            Ok(decided) => decided,
            Err(refusal) => {
                self.seen.pop();
                self.tape.truncate(self.seen.len());
                self.open_window.replace(open_window);
                return Err(refusal);
            }
        };
        self.stream = stream;
        self.stop = stop;
        if let (Some((at, _)), Some(text)) = (self.stop.as_ref(), stop_text) {
            self.texts.insert(*at, text);
        }
        if let Some(text) = held_text {
            self.texts.insert(index, text);
        }
        self.exhausted = exhausted;
        self.report = Arc::clone(&report);
        self.evaluated = Some((self.seen.len(), None));
        Ok(Advance { report, decided })
    }

    /// FR-330's report over the positions seen, from the stream's outcome.
    fn streamed_report(
        &self,
        stream: Option<&ClauseStream>,
        stop: Option<&(u64, Value)>,
        exhausted: Option<&filament_exact::Incomplete>,
        meter: &mut Meter,
    ) -> ClauseRunReport {
        let result = match (exhausted, stream) {
            (Some(incomplete), _) => Ok(TemporalOutcome::Incomplete(incomplete.clone())),
            (None, Some(stream)) => stream.outcome(meter),
            (None, None) => Err(fsl_eval::temporal::TemporalRefusal::Fault(
                InternalFault::new("clause-state", "streamed-state-has-its-stream"),
            )),
        };
        let read = stop
            .map(|(index, sample)| read_sample_stop(self.package, self.node, sample, *index, meter))
            .unwrap_or_default();
        let observed = ObservedRun {
            result,
            stopped: read.stopped,
            undefined: read.undefined,
            decisive: None,
        };
        let usage = ClauseRunUsage::from_meter(meter, Default::default());
        streamed_sample_report(
            self.package,
            self.open.sources,
            self.node,
            observed,
            usage,
            || {
                TraceInput::Finite(
                    self.seen
                        .iter()
                        .map(|position| position.reference.clone())
                        .collect(),
                )
            },
        )
    }

    /// Read the sample at `index` into its letter, and drop the samples no
    /// window still reads.
    fn read(&mut self, index: u64, sample: Value, step: &mut Meter) {
        let mut meter = Meter::new(self.open.limits.accounting);
        match self.reading {
            Reading::Values | Reading::Streamed | Reading::Snapshot | Reading::Refused => {}
            Reading::Observed => {
                let entry = read_sample_letter(self.package, self.node, &sample, &mut meter);
                // A sample read into its letter is not held, nor its text.
                if matches!(entry, TapeEntry::Read(_)) {
                    self.texts.remove(&index);
                }
                self.tape.push(entry);
            }
            Reading::Subject => {
                self.values.insert(index, sample);
                let reach = self.window_reach();
                self.push_subject(index, reach, step);
                let read = read_subject_letter(
                    self.package,
                    self.node,
                    SubjectSamples::Tape {
                        tape: &self.tape,
                        values: &self.values,
                        captures: &self.captures,
                        len: index.saturating_add(1),
                    },
                    index,
                    self.open.limits.max_window_values,
                    &mut meter,
                );
                match read {
                    Some((letter, captures)) => {
                        if index == 0 {
                            self.captures = captures;
                        }
                        self.tape.push(TapeEntry::Read(letter));
                    }
                    None => {
                        // The run reads this sample again, and an aggregate
                        // there reads its window: both stay held.
                        let from = index.saturating_sub(reach.unwrap_or(0));
                        self.pinned.extend(from..=index);
                        let held = self.values.get(&index).cloned();
                        if let Some(sample) = held {
                            self.tape.push(TapeEntry::Held(sample));
                        }
                    }
                }
                // The window of the newest position stays held: the next
                // position's aggregates read all of it but its oldest sample.
                let keep_from = match reach {
                    Some(reach) => index.saturating_sub(reach),
                    None => index.saturating_add(1),
                };
                let pinned = &self.pinned;
                self.values
                    .retain(|position, _| *position >= keep_from || pinned.contains(position));
                let values = &self.values;
                self.texts
                    .retain(|position, _| values.contains_key(position));
            }
        }
    }

    /// FR-820: read the sample at `index`, which `values` holds, into the
    /// trace-subject stream on `meter`, keep the atom values its decisive
    /// aggregate may still read, and pin the window of a position at which the
    /// reading stopped, so the stop is read again from the state's bytes.
    fn push_subject(&mut self, index: u64, reach: Option<u64>, meter: &mut Meter) {
        let (Some(stream), None) = (self.stream.as_mut(), self.exhausted.as_ref()) else {
            return;
        };
        let stopped = stream.stopped();
        match push_subject_sample(
            self.package,
            self.node,
            stream,
            &self.values,
            &self.captures,
            self.open.limits.max_window_values,
            meter,
        ) {
            Ok(read) => {
                self.captures = read.captures;
                if read.undefined_atom.is_some() {
                    self.undefined_atom = read.undefined_atom;
                }
                if self.first_undefined.is_none() {
                    self.first_undefined = read.first_undefined;
                }
                if let Some((lower, upper)) = decisive_reach(self.node) {
                    self.atom_values.extend(read.atom_values);
                    let from = index.saturating_sub(upper);
                    let decided = stream
                        .violated()
                        .map(|at| (at.saturating_add(lower), at.saturating_add(upper)));
                    self.atom_values.retain(|(_, position), _| {
                        *position >= from
                            || decided
                                .is_some_and(|(start, end)| *position >= start && *position <= end)
                    });
                }
                if stopped.is_none() && stream.stopped() == Some(index) {
                    let from = index.saturating_sub(reach.unwrap_or(0));
                    self.pinned.extend(from..=index);
                }
            }
            Err(StreamStop::Incomplete(incomplete)) => self.exhausted = Some(incomplete),
        }
    }

    /// FR-330-AC-17's report over the positions seen, from the trace-subject
    /// stream's outcome.
    fn subject_stream_report(&self, meter: &mut Meter) -> ClauseRunReport {
        let result = match (&self.exhausted, &self.stream) {
            (Some(incomplete), _) => Ok(TemporalOutcome::Incomplete(incomplete.clone())),
            (None, Some(stream)) => stream.outcome(meter),
            (None, None) => Err(fsl_eval::temporal::TemporalRefusal::Fault(
                InternalFault::new("clause-state", "subject-state-has-its-stream"),
            )),
        };
        let mut report = subject_report(
            self.node,
            result,
            self.undefined_atom,
            &self.atom_values,
            ClauseRunUsage::from_meter(meter, Default::default()),
        );
        if matches!(
            report.disposition,
            ClauseDisposition::Temporal(
                TemporalVerdict::Violated { .. } | TemporalVerdict::Undefined { .. }
            )
        ) {
            let trace = TraceInput::Finite(
                self.seen
                    .iter()
                    .map(|position| position.reference.clone())
                    .collect(),
            );
            with_subject_counterexample(
                self.node,
                &trace,
                &mut report,
                self.first_undefined.clone(),
            );
        }
        report
    }

    /// FR-820: advance the state by a progress assertion, with no position.
    ///
    /// # Errors
    ///
    /// A cancelled call or a contradiction; the state is unchanged.
    pub fn assert_progress(
        &mut self,
        progress: &ProgressAssertion,
        cancel: &Cancel,
    ) -> Result<Advance, ClauseRefusal> {
        cancelled(cancel)?;
        let mut meter = Meter::new(self.open.limits.accounting).with_cancel(cancel.clone());
        let mark = self.mark();
        let advance = match self.evaluate(Some(*progress), &mut meter, cancel) {
            Ok(advance) => advance,
            Err(refusal) => {
                self.restore(mark);
                return Err(refusal);
            }
        };
        self.progress = Some(*progress);
        Ok(advance)
    }

    /// FR-820: the trace of the first `seen` positions as `shape` names them
    /// (`shape` lists no position), kept in `cell` between runs so a run
    /// lists only the positions added since the last one. A carried listing
    /// that is not a prefix of the positions seen is listed again.
    fn carried_trace<'c>(
        &self,
        cell: &'c RefCell<Option<TraceInput>>,
        seen: usize,
        mut shape: TraceInput,
    ) -> Result<std::cell::Ref<'c, TraceInput>, ClauseRefusal> {
        let positions = self.seen.get(..seen).unwrap_or(&self.seen);
        {
            let mut held = cell.borrow_mut();
            match (&mut shape, held.take()) {
                (
                    TraceInput::Progressed { trace: list, .. },
                    Some(TraceInput::Progressed { trace: kept, .. }),
                )
                | (TraceInput::Finite(list), Some(TraceInput::Finite(kept))) => *list = kept,
                (
                    TraceInput::Timestamped {
                        observations: list, ..
                    },
                    Some(TraceInput::Timestamped {
                        observations: kept, ..
                    }),
                ) => *list = kept,
                _ => {}
            }
            let prefix = |listed: usize, same: &dyn Fn(usize) -> bool| {
                if listed <= positions.len() && listed.checked_sub(1).is_none_or(same) {
                    listed
                } else {
                    0
                }
            };
            match &mut shape {
                TraceInput::Progressed { trace: list, .. } | TraceInput::Finite(list) => {
                    let keep = prefix(list.len(), &|at| {
                        list.get(at) == positions.get(at).map(|p| &p.reference)
                    });
                    list.truncate(keep);
                    list.extend(
                        positions
                            .get(keep..)
                            .unwrap_or(&[])
                            .iter()
                            .map(|p| p.reference.clone()),
                    );
                }
                TraceInput::Timestamped { observations, .. } => {
                    let keep = prefix(observations.len(), &|at| match (
                        observations.get(at),
                        positions.get(at),
                    ) {
                        (Some(listed), Some(position)) => {
                            listed.document == position.reference
                                && listed.stamp == position.stamp
                                && listed.order_key == position.order_key
                        }
                        _ => false,
                    });
                    observations.truncate(keep);
                    observations.extend(positions.get(keep..).unwrap_or(&[]).iter().map(
                        |position| TimestampedObservation {
                            document: position.reference.clone(),
                            stamp: position.stamp,
                            order_key: position.order_key.clone(),
                        },
                    ));
                }
                TraceInput::Lasso { .. } | TraceInput::Model { .. } => {}
            }
            *held = Some(shape);
        }
        std::cell::Ref::filter_map(cell.borrow(), Option::as_ref)
            .map_err(|_| fault("carried-trace-is-held"))
    }

    /// The trace of the first `seen` positions under `assertion`, as the
    /// report's run reads it.
    fn trace_of(
        &self,
        seen: usize,
        assertion: ProgressAssertion,
    ) -> Result<std::cell::Ref<'_, TraceInput>, ClauseRefusal> {
        self.carried_trace(&self.trace, seen, self.trace_shape(assertion))
    }

    /// The trace under `assertion`, listing no position.
    fn trace_shape(&self, assertion: ProgressAssertion) -> TraceInput {
        let refs = Vec::new;
        match (self.open.kind, self.open.binding.as_ref()) {
            (ClauseTraceKind::Progressed, Some(binding)) => TraceInput::Progressed {
                trace: refs(),
                progress: assertion,
                lower_boundary: self.open.lower_boundary,
                binding: binding.clone(),
            },
            (ClauseTraceKind::Timestamped, binding) => TraceInput::Timestamped {
                observations: Vec::new(),
                progress: assertion,
                window: Box::new(TimestampedWindow {
                    binding: binding.cloned(),
                    authoritative_origin: matches!(
                        self.open.lower_boundary,
                        ProgressLowerBoundary::AuthoritativeOrigin
                    ),
                    cover: self.open.cover,
                    missing_valuations: Vec::new(),
                    late: super::LateInputs::default(),
                }),
            },
            _ => TraceInput::Finite(refs()),
        }
    }

    fn open_assertion() -> ProgressAssertion {
        ProgressAssertion {
            watermark: 0,
            completeness: ProgressCompleteness::Incomplete,
            scope: ProgressClosure::Open,
            execution: ProgressClosure::Open,
        }
    }

    /// Whether `assertion` covers every position seen, so a run reads them; a
    /// progress-carrying kind is evaluated only then.
    fn evaluable(&self, assertion: Option<ProgressAssertion>) -> bool {
        if self.seen.is_empty() {
            return false;
        }
        if !self.open.kind.carries_progress() {
            return true;
        }
        let newest = self.seen.iter().map(|p| p.stamp).max().unwrap_or(0);
        assertion.is_some_and(|assertion| assertion.watermark >= newest)
    }

    /// FR-330's run over the first `seen` positions, as `trace` names them.
    /// `open` is FR-504's open-scope reading, which starts from no ledger and
    /// reads every aggregate letter.
    fn run(&self, trace: &TraceInput, seen: usize, open: bool, cancel: &Cancel) -> ClauseRunReport {
        let request = PackageTemporalRequest {
            package: self.package,
            packages: self.open.packages,
            model_limits: self.open.limits.model,
            snapshots: &self.snapshots,
            observation_limits: self.open.limits.observation,
            accounting: self.open.limits.accounting,
            name: self.node.clause().name(),
            trace,
            over: self.open.over.as_ref(),
            ledger: if open { None } else { self.ledger.as_ref() },
            sources: self.open.sources,
            authentication: self
                .open
                .authentication
                .as_deref()
                .map(PackageAuthentication::temporal),
            cancel,
        };
        let window = if open {
            &self.open_window
        } else {
            &self.window
        };
        let admission = if open {
            &self.open_admission
        } else {
            &self.admission
        };
        if self.reading == Reading::Snapshot {
            return self.with_snapshot_tape(|snapshot| {
                run_package_temporal_over(
                    &request,
                    Some(TapeInput {
                        tape: &self.tape,
                        values: &self.values,
                        captures: &self.captures,
                        open_aggregates: open,
                        window: Some(window),
                        snapshot: Some(snapshot),
                        admission: Some(admission),
                    }),
                )
            });
        }
        let shorter;
        let tape = match self.reading {
            Reading::Values | Reading::Snapshot => None,
            Reading::Streamed if !(open && self.lettered()) => None,
            Reading::Subject | Reading::Observed | Reading::Streamed | Reading::Refused => {
                let tape = if seen < self.tape.len() {
                    let mut prefix = LetterTape::new();
                    for entry in &self.tape.entries()[..seen] {
                        prefix.push(entry.clone());
                    }
                    shorter = prefix;
                    &shorter
                } else {
                    &self.tape
                };
                Some(TapeInput {
                    tape,
                    values: &self.values,
                    captures: &self.captures,
                    open_aggregates: open,
                    window: Some(window),
                    snapshot: None,
                    admission: Some(admission),
                })
            }
        };
        run_package_temporal_over(&request, tape)
    }

    /// Evaluate the clause over every position seen under `assertion`, and
    /// keep the report.
    fn evaluate(
        &mut self,
        assertion: Option<ProgressAssertion>,
        meter: &mut Meter,
        cancel: &Cancel,
    ) -> Result<Advance, ClauseRefusal> {
        if !self.evaluable(assertion) {
            return Ok(Advance {
                report: Arc::clone(&self.report),
                decided: None,
            });
        }
        self.order_covered(meter);
        let seen = self.seen.len();
        let report = match self.reading {
            Reading::Subject => self.subject_stream_report(meter),
            Reading::Snapshot if self.open.kind == ClauseTraceKind::Finite => {
                self.snapshot_finite_report(meter)
            }
            Reading::Refused => self.refused_report(cancel)?,
            Reading::Observed | Reading::Streamed | Reading::Values | Reading::Snapshot => {
                let trace = self.trace_of(seen, assertion.unwrap_or_else(Self::open_assertion))?;
                self.run(&trace, seen, false, cancel)
            }
        };
        cancelled(cancel)?;
        if let ClauseDisposition::ProgressContradiction(refusal) = report.disposition {
            return Err(ClauseRefusal::Contradiction(refusal));
        }
        let report = Arc::new(report);
        let decided = self.decided(&report, assertion, cancel)?;
        if self.reading == Reading::Snapshot && self.objects.failed.is_empty() {
            // The report found the decisive atom of a violation for good.
            self.drop_unread_bytes();
        }
        self.report = Arc::clone(&report);
        self.evaluated = Some((seen, assertion));
        Ok(Advance { report, decided })
    }

    /// FR-330's report of a refused reading. The run refuses before it reads
    /// a position, and the refusal names none, so the run over the first
    /// position is the run over every prefix: each step does the same bounded
    /// work and holds no position.
    fn refused_report(&self, cancel: &Cancel) -> Result<ClauseRunReport, ClauseRefusal> {
        let trace = TraceInput::Finite(vec![index_reference(0)]);
        let report = self.run(&trace, 1, false, cancel);
        if report.category() != filament_semantic_value::catalog::Category::Refusal {
            return Err(fault("finite-sample-reading-is-refused"));
        }
        Ok(report)
    }

    /// FR-501 "Canonical order": a `Timestamped` trace's positions since the
    /// last evaluation, which the progress now covers, take their canonical
    /// order on the tape and in the positions seen, so the carried evaluation
    /// reads them at the positions the batch run gives them. Every earlier
    /// position is stamped below them (FR-643 "Order").
    fn order_covered(&mut self, meter: &mut Meter) {
        if self.open.kind != ClauseTraceKind::Timestamped
            || !matches!(self.reading, Reading::Observed | Reading::Snapshot)
        {
            return;
        }
        let from = self
            .evaluated
            .map_or(0, |(seen, _)| seen)
            .min(self.seen.len());
        let Some(tail) = self.seen.get(from..) else {
            return;
        };
        let mut tail = tail.to_vec();
        let mut order: Vec<usize> = (0..tail.len()).collect();
        order.sort_by(|left, right| match (tail.get(*left), tail.get(*right)) {
            (Some(left), Some(right)) => canonical_order(left, right),
            _ => std::cmp::Ordering::Equal,
        });
        let first = u64::try_from(from).unwrap_or(u64::MAX);
        // FR-330-AC-18: an observed sample is cited by its canonical index from
        // here on, as the batch run names it, and its text is dropped. The
        // evaluator orders index identities as the indices
        // (`observation_identity`), so the order stays.
        let mut texts = vec![None; tail.len()];
        if self.reading == Reading::Observed {
            for (at, index) in order.iter().enumerate() {
                let Some(position) = tail.get_mut(*index) else {
                    continue;
                };
                if position.reference.authority == super::temporal::SAMPLE_AUTHORITY {
                    let place = first.saturating_add(u64::try_from(at).unwrap_or(u64::MAX));
                    let cited = super::temporal::sample_index(
                        place,
                        &super::temporal::sample_content(&position.reference.identity),
                    );
                    let named = std::mem::replace(&mut position.reference, cited);
                    if let Some(text) = texts.get_mut(*index) {
                        *text = Some(named.identity);
                    }
                }
            }
            self.seen.truncate(from);
            self.seen.extend(tail.iter().cloned());
        }
        if order.iter().enumerate().any(|(at, index)| at != *index) {
            let reordered: Vec<Seen> = order
                .iter()
                .filter_map(|index| tail.get(*index).cloned())
                .collect();
            self.seen.truncate(from);
            self.seen.extend(reordered);
            self.tape.reorder_tail(from, &order);
            // The carried admission holds the positions before them only.
            self.admission.get_mut().truncate(from);
            self.open_admission.get_mut().truncate(from);
            truncate_listing(&mut self.trace, from);
            truncate_listing(&mut self.open_trace, from);
            // A snapshot's bytes and failed admission move with its position.
            let moved = |position: u64| -> u64 {
                let offset = usize::try_from(position.saturating_sub(first)).ok();
                offset
                    .and_then(|offset| order.iter().position(|index| *index == offset))
                    .and_then(|at| u64::try_from(at).ok())
                    .map_or(position, |at| first.saturating_add(at))
            };
            let remap = |position: u64| {
                if position >= first {
                    moved(position)
                } else {
                    position
                }
            };
            self.objects.bytes = std::mem::take(&mut self.objects.bytes)
                .into_iter()
                .map(|(position, bytes)| (remap(position), bytes))
                .collect();
            self.objects.failed = std::mem::take(&mut self.objects.failed)
                .into_iter()
                .map(|(position, failure)| (remap(position), failure))
                .collect();
        }
        // A sample still held by value keeps its text, at its new place.
        for (at, index) in order.iter().enumerate() {
            let place = first.saturating_add(u64::try_from(at).unwrap_or(u64::MAX));
            let held = usize::try_from(place)
                .ok()
                .and_then(|place| self.tape.entries().get(place))
                .is_some_and(|entry| matches!(entry, TapeEntry::Held(_)));
            if let (true, Some(Some(text))) = (held, texts.get_mut(*index).map(Option::take)) {
                self.texts.insert(place, text);
            }
        }
        if self.reading == Reading::Snapshot {
            self.read_pending(first, meter);
        }
    }

    /// The definite verdict of the clause's profile over the positions seen.
    fn decided(
        &self,
        report: &Arc<ClauseRunReport>,
        assertion: Option<ProgressAssertion>,
        cancel: &Cancel,
    ) -> Result<Option<Arc<ClauseRunReport>>, ClauseRefusal> {
        Ok(match self.node.clause().profile() {
            TemporalProfile::InfiniteTrace => matches!(
                report.disposition,
                ClauseDisposition::Temporal(
                    TemporalVerdict::Violated { .. } | TemporalVerdict::Undefined { .. }
                )
            )
            .then(|| Arc::clone(report)),
            TemporalProfile::TimestampedEventFiniteWindow => {
                let closed = assertion.is_some_and(|a| a.scope == ProgressClosure::Closed)
                    && matches!(
                        report.disposition,
                        ClauseDisposition::Temporal(
                            TemporalVerdict::Violated { .. } | TemporalVerdict::Holds
                        )
                    );
                (decisive_window(report) || closed).then(|| Arc::clone(report))
            }
            TemporalProfile::EventPositionFalseExtension
            | TemporalProfile::FixedSampleFalseExtension => self.open_scope(assertion, cancel)?,
            TemporalProfile::Timed => None,
        })
    }

    /// FR-504's open-scope reading of the positions seen, when it is
    /// definite (a decisive basis). A fixed-sample clause over a
    /// `Progressed` trace reads under the latest `assertion` with scope
    /// `open`, so silent samples up to `min(W, through)` count; every other
    /// reading has `S = n - 1`. With no emission the reading is
    /// unauthenticated, as a `Finite` trace needs no binding.
    fn open_scope(
        &self,
        assertion: Option<ProgressAssertion>,
        cancel: &Cancel,
    ) -> Result<Option<Arc<ClauseRunReport>>, ClauseRefusal> {
        let clause = self.node.clause();
        let binding = match self.open.authentication.as_deref() {
            Some(authentication) => {
                clause_trace_binding(self.package, clause.name(), authentication.temporal())
                    .ok_or_else(|| fault("closed-trace-clause-has-a-clock-row"))?
            }
            None => TraceBinding {
                profile: clause.profile().identity().to_owned(),
                clock_name: clause.clock().to_owned(),
                parameters: fsl_package::ClockConfiguration::EventPosition {
                    sequence_authority: clause.clock().into(),
                },
            },
        };
        let Some(last) = self.seen.len().checked_sub(1) else {
            return Ok(None);
        };
        let last = u64::try_from(last).map_err(|_| fault("position-count-in-u64"))?;
        let progress = match (clause.profile(), self.open.kind, assertion) {
            (
                TemporalProfile::FixedSampleFalseExtension,
                ClauseTraceKind::Progressed,
                Some(held),
            ) => ProgressAssertion {
                scope: ProgressClosure::Open,
                ..held
            },
            _ => ProgressAssertion {
                watermark: last,
                completeness: ProgressCompleteness::Complete { through: last },
                scope: ProgressClosure::Open,
                execution: ProgressClosure::Open,
            },
        };
        let trace = self.carried_trace(
            &self.open_trace,
            self.seen.len(),
            TraceInput::Progressed {
                trace: Vec::new(),
                progress,
                lower_boundary: self.open.lower_boundary,
                binding,
            },
        )?;
        let report = self.run(&trace, self.seen.len(), true, cancel);
        cancelled(cancel)?;
        Ok(decisive_window(&report).then(|| Arc::new(report)))
    }

    /// FR-820: the latest report.
    pub fn report(&self) -> Arc<ClauseRunReport> {
        Arc::clone(&self.report)
    }

    /// FR-820: the number of admitted positions the state holds by value.
    pub fn retained(&self) -> u64 {
        match self.reading {
            Reading::Values => u64::try_from(self.seen.len()).unwrap_or(u64::MAX),
            Reading::Observed => self.tape.held().saturating_add(self.uncovered_texts()),
            Reading::Refused => 0,
            Reading::Snapshot => self.objects.retained(),
            Reading::Streamed => u64::from(self.stop.is_some()).saturating_add(self.tape.held()),
            Reading::Subject => u64::try_from(self.values.len()).unwrap_or(u64::MAX),
        }
    }

    /// The `Timestamped` positions progress does not cover yet whose sample
    /// is read into its letter but still named by its text, which orders it
    /// (FR-501 "Canonical order"): each holds that value.
    fn uncovered_texts(&self) -> u64 {
        if self.open.kind != ClauseTraceKind::Timestamped {
            return 0;
        }
        let from = self.evaluated.map_or(0, |(seen, _)| seen);
        let count = self
            .seen
            .iter()
            .zip(self.tape.entries())
            .skip(from)
            .filter(|(position, entry)| {
                position.reference.authority == super::temporal::SAMPLE_AUTHORITY
                    && matches!(entry, TapeEntry::Read(_))
            })
            .count();
        u64::try_from(count).unwrap_or(u64::MAX)
    }

    /// FR-820: the number of position identities the state keeps for the
    /// counterexample it may still report. FR-330's counterexample lists every
    /// position of the prefix it was run over, so an unsettled clause cites
    /// each position seen.
    pub fn cited(&self) -> u64 {
        u64::try_from(self.seen.len()).unwrap_or(u64::MAX)
    }

    /// FR-820: the state's bytes, read back by [`Self::from_bytes`].
    ///
    /// # Errors
    ///
    /// A fault when a capture's value has no state form.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ClauseRefusal> {
        let positions: Vec<Json> = self
            .seen
            .iter()
            .enumerate()
            .map(|(index, position)| {
                let letter = match self.tape.entries().get(index) {
                    Some(TapeEntry::Read(letter)) => letter_json(letter),
                    Some(TapeEntry::Held(_) | TapeEntry::Object(_)) | None => Json::Null,
                };
                let document = match self.reading {
                    Reading::Snapshot => u64::try_from(index)
                        .ok()
                        .and_then(|index| self.objects.bytes.get(&index)),
                    _ => self.snapshots.get(&position.reference),
                };
                json!({
                    "authority": position.reference.authority,
                    "identity": position.reference.identity,
                    "stamp": position.stamp,
                    "order_key": position.order_key.as_ref().map(|key|
                        json!({"authority": key.authority, "key": key.key})),
                    "letter": letter,
                    "document": document
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
                })
            })
            .collect();
        let captures = self
            .captures
            .iter()
            .map(|value| capture_json(value).ok_or_else(|| fault("capture-has-a-state-form")))
            .collect::<Result<Vec<_>, _>>()?;
        let stream = match (&self.stream, self.exhausted.is_some()) {
            (Some(stream), false) => stream
                .to_state()
                .ok_or_else(|| fault("stream-has-a-state-form"))?,
            (None, _) | (Some(_), true) => Json::Null,
        };
        let document = json!({
            "format": CLAUSE_STATE_FORMAT,
            "clause": self.node.clause().name(),
            "reading": self.reading.name(),
            "positions": positions,
            "values": self.values.keys().collect::<Vec<_>>(),
            "texts": self.texts.iter().map(|(index, text)| json!([index, text])).collect::<Vec<_>>(),
            "stream": stream,
            "exhausted": self.exhausted.as_ref().map(incomplete_json),
            "stop": self.stop.as_ref().map(|(index, _)| *index).or(self.objects.stop),
            "pinned": self.pinned.iter().collect::<Vec<_>>(),
            "atoms": self.atom_values.iter().map(|((atom, position), value)|
                json!([atom, position, value])).collect::<Vec<_>>(),
            "undefined_atom": self.undefined_atom,
            "captures": captures,
            "progress": self.progress.map(assertion_json),
            "evaluated": self.evaluated.map(|(seen, assertion)| json!({
                "seen": seen,
                "progress": assertion.map(assertion_json),
            })),
        });
        serde_json::to_vec(&document).map_err(|_| fault("state-serializes"))
    }

    /// FR-820: read back a state [`Self::to_bytes`] wrote for `clause`, under
    /// the same opening request. Its report is the evaluation the original
    /// last made, run again over the letters; no capture is evaluated again.
    ///
    /// # Errors
    ///
    /// [`ClauseRefusal::StateMismatch`] for bytes that are not this clause's
    /// closed `filament.clause-state/1`.
    pub fn from_bytes(
        package: &'a fsl_package::CheckedPackage,
        clause: &'a TemporalClauseNode,
        request: &ClauseOpen<'a>,
        bytes: &[u8],
        cancel: &Cancel,
    ) -> Result<Self, ClauseRefusal> {
        let mismatch = || ClauseRefusal::StateMismatch;
        let mut state = Self::open(package, clause, request)?;
        let document: Json = serde_json::from_slice(bytes).map_err(|_| mismatch())?;
        let object = document.as_object().ok_or_else(mismatch)?;
        const MEMBERS: [&str; 15] = [
            "exhausted",
            "texts",
            "atoms",
            "undefined_atom",
            "format",
            "stream",
            "stop",
            "clause",
            "reading",
            "positions",
            "values",
            "pinned",
            "captures",
            "progress",
            "evaluated",
        ];
        if object.len() != MEMBERS.len()
            || MEMBERS.iter().any(|member| !object.contains_key(*member))
            || object["format"] != CLAUSE_STATE_FORMAT
            || object["clause"] != clause.clause().name()
            || object["reading"] != state.reading.name()
        {
            return Err(mismatch());
        }
        let record = match &clause.clause().over().1 {
            ValueType::Composite(record) => Some(record.clone()),
            _ => None,
        };
        // A held sample is admitted again from its canonical text: the text
        // the state holds for it, or the identity of a position a reading
        // that runs the batch evaluation again names by its text.
        let mut texts = BTreeMap::new();
        for entry in object["texts"].as_array().ok_or_else(mismatch)? {
            let index = entry[0].as_u64().ok_or_else(mismatch)?;
            let text = entry[1].as_str().ok_or_else(mismatch)?;
            texts.insert(index, text.to_owned());
        }
        state.texts = texts.clone();
        let admit = |index: u64, reference: &DocumentRef| -> Result<Value, ClauseRefusal> {
            let record = record.as_ref().ok_or_else(mismatch)?;
            let text = texts.get(&index).unwrap_or(&reference.identity);
            admit_record_sample(package.graph().scope().types(), record, text.as_bytes())
                .map_err(|_| mismatch())
        };
        let positions = object["positions"].as_array().ok_or_else(mismatch)?;
        for position in positions {
            let seen = u64::try_from(state.seen.len()).map_err(|_| mismatch())?;
            let text = |member: &str| position[member].as_str().map(str::to_owned);
            let reference = DocumentRef {
                authority: text("authority").ok_or_else(mismatch)?,
                identity: text("identity").ok_or_else(mismatch)?,
            };
            let order_key = match &position["order_key"] {
                Json::Null => None,
                key => Some(TimestampedOrderKey {
                    authority: key["authority"].as_str().ok_or_else(mismatch)?.to_owned(),
                    key: key["key"].as_i64().ok_or_else(mismatch)?,
                }),
            };
            if state.reading == Reading::Snapshot {
                state.restore_snapshot_position(
                    seen,
                    &reference,
                    text("document"),
                    &position["letter"],
                )?;
            } else if let Some(document) = text("document") {
                state
                    .snapshots
                    .insert(reference.clone(), document.into_bytes());
            }
            if matches!(state.reading, Reading::Subject | Reading::Observed) || state.lettered() {
                let entry = match &position["letter"] {
                    Json::Null => TapeEntry::Held(admit(seen, &reference)?),
                    letter => TapeEntry::Read(letter_of(letter).ok_or_else(mismatch)?),
                };
                state.tape.push(entry);
            }
            state.seen.push(Seen {
                reference,
                stamp: position["stamp"].as_u64().ok_or_else(mismatch)?,
                order_key,
            });
        }
        let indices = |member: &str| -> Result<Vec<u64>, ClauseRefusal> {
            object[member]
                .as_array()
                .ok_or_else(mismatch)?
                .iter()
                .map(|index| index.as_u64().ok_or_else(mismatch))
                .collect()
        };
        for index in indices("values")? {
            let position = state
                .seen
                .get(usize::try_from(index).map_err(|_| mismatch())?)
                .ok_or_else(mismatch)?;
            let value = admit(index, &position.reference)?;
            state.values.insert(index, value);
        }
        state.pinned = indices("pinned")?.into_iter().collect();
        state.captures = object["captures"]
            .as_array()
            .ok_or_else(mismatch)?
            .iter()
            .map(|capture| capture_of(package, capture).ok_or_else(mismatch))
            .collect::<Result<_, _>>()?;
        state.progress = match &object["progress"] {
            Json::Null => None,
            value => Some(assertion_of(value).ok_or_else(mismatch)?),
        };
        state.exhausted = match &object["exhausted"] {
            Json::Null => None,
            value => Some(incomplete_of(value).ok_or_else(mismatch)?),
        };
        if state.reading == Reading::Subject {
            for entry in object["atoms"].as_array().ok_or_else(mismatch)? {
                let atom = entry[0]
                    .as_u64()
                    .and_then(|atom| usize::try_from(atom).ok());
                let position = entry[1].as_u64();
                let value = match &entry[2] {
                    Json::Null => None,
                    value => Some(value.as_bool().ok_or_else(mismatch)?),
                };
                let (Some(atom), Some(position)) = (atom, position) else {
                    return Err(mismatch());
                };
                state.atom_values.insert((atom, position), value);
            }
            state.undefined_atom = match &object["undefined_atom"] {
                Json::Null => None,
                atom => Some(
                    atom.as_u64()
                        .and_then(|atom| usize::try_from(atom).ok())
                        .ok_or_else(mismatch)?,
                ),
            };
            let mut meter = Meter::new(request.limits.accounting).with_cancel(cancel.clone());
            state.stream = match &object["stream"] {
                Json::Null => None,
                carried => Some(
                    ClauseStream::from_state(clause.clause(), carried, &mut meter)
                        .ok_or_else(mismatch)?,
                ),
            };
            if let Some(stream) = state.stream.as_mut() {
                // The stop's records are read again from the pinned window.
                let read = reread_subject_stop(
                    package,
                    clause,
                    stream,
                    &state.values,
                    &state.captures,
                    request.limits.max_window_values,
                    &mut meter,
                );
                state.first_undefined = read.first_undefined;
            }
        }
        if state.reading == Reading::Snapshot {
            let mut meter = Meter::new(request.limits.accounting).with_cancel(cancel.clone());
            state.restore_snapshot(&object["stream"], object["stop"].as_u64(), &mut meter)?;
            if request.kind == ClauseTraceKind::Finite {
                if !state.seen.is_empty() {
                    state.report = Arc::new(state.snapshot_finite_report(&mut meter));
                    state.evaluated = Some((state.seen.len(), None));
                }
                cancelled(cancel)?;
                return Ok(state);
            }
        }
        if state.reading == Reading::Streamed {
            let mut meter = Meter::new(request.limits.accounting).with_cancel(cancel.clone());
            if state.exhausted.is_some() {
                // A work limit ended the evaluation: its report is that
                // limit's, and the stream is not carried.
                state.report = Arc::new(state.streamed_report(
                    None,
                    None,
                    state.exhausted.as_ref(),
                    &mut meter,
                ));
                state.stream = None;
                state.evaluated = Some((state.seen.len(), None));
                return Ok(state);
            }
            let mut stream =
                ClauseStream::from_state(clause.clause(), &object["stream"], &mut meter)
                    .ok_or_else(mismatch)?;
            if let Some(index) = object["stop"].as_u64() {
                let position = state
                    .seen
                    .get(usize::try_from(index).map_err(|_| mismatch())?)
                    .ok_or_else(mismatch)?;
                let sample = admit(index, &position.reference)?;
                let read = read_sample_stop(package, clause, &sample, index, &mut meter);
                let refused = match &read.stopped {
                    Some(fsl_eval::temporal::HoldsStop::Refused { at, .. }) => Some(*at),
                    _ => None,
                };
                stream.restore_stop(
                    read.undefined
                        .map(|first| fsl_eval::temporal::UndefinedEvaluation {
                            at: first.position,
                            cause: first.cause,
                        }),
                    refused,
                );
                state.stop = Some((index, sample));
            }
            cancelled(cancel)?;
            state.report = Arc::new(state.streamed_report(
                Some(&stream),
                state.stop.as_ref(),
                None,
                &mut meter,
            ));
            state.stream = Some(stream);
            state.evaluated = Some((state.seen.len(), None));
            return Ok(state);
        }
        if let Json::Object(evaluated) = &object["evaluated"] {
            let seen = evaluated
                .get("seen")
                .and_then(Json::as_u64)
                .and_then(|seen| usize::try_from(seen).ok())
                .filter(|seen| *seen <= state.seen.len())
                .ok_or_else(mismatch)?;
            let assertion = match evaluated.get("progress") {
                None | Some(Json::Null) => None,
                Some(value) => Some(assertion_of(value).ok_or_else(mismatch)?),
            };
            state.report = Arc::new(if state.reading == Reading::Subject {
                let mut meter = Meter::new(request.limits.accounting).with_cancel(cancel.clone());
                state.subject_stream_report(&mut meter)
            } else if state.reading == Reading::Refused {
                state.refused_report(cancel)?
            } else {
                let trace = state.trace_of(seen, assertion.unwrap_or_else(Self::open_assertion))?;
                state.run(&trace, seen, false, cancel)
            });
            cancelled(cancel)?;
            state.evaluated = Some((seen, assertion));
        }
        Ok(state)
    }
}

/// FR-820: keep only the first `len` positions of a carried trace listing.
fn truncate_listing(cell: &mut RefCell<Option<TraceInput>>, len: usize) {
    match cell.get_mut() {
        Some(TraceInput::Progressed { trace: list, .. } | TraceInput::Finite(list)) => {
            list.truncate(len);
        }
        Some(TraceInput::Timestamped { observations, .. }) => observations.truncate(len),
        Some(TraceInput::Lasso { .. } | TraceInput::Model { .. }) | None => {}
    }
}

fn letter_json(letter: &TapeLetter) -> Json {
    let atoms: Vec<Json> = letter
        .atoms()
        .iter()
        .map(|(node, reading)| match reading {
            TapeReading::True => json!([node, true]),
            TapeReading::False => json!([node, false]),
            TapeReading::Insufficient(data) => {
                json!([node, {"count": data.count, "min_count": data.min_count}])
            }
        })
        .collect();
    json!({
        "atoms": atoms,
        "activation": letter.activation(),
        "captures": letter.captures(),
    })
}

fn letter_of(value: &Json) -> Option<TapeLetter> {
    let object = value.as_object()?;
    if object.len() != 3 {
        return None;
    }
    let atoms = object
        .get("atoms")?
        .as_array()?
        .iter()
        .map(|pair| {
            let [node, reading] = pair.as_array()?.as_slice() else {
                return None;
            };
            let node = usize::try_from(node.as_u64()?).ok()?;
            let reading = match reading {
                Json::Bool(true) => TapeReading::True,
                Json::Bool(false) => TapeReading::False,
                Json::Object(data) => TapeReading::Insufficient(InsufficientData {
                    count: data.get("count")?.as_u64()?,
                    min_count: data.get("min_count")?.as_u64()?,
                }),
                _ => return None,
            };
            Some((node, reading))
        })
        .collect::<Option<Vec<_>>>()?;
    let activation = match object.get("activation")? {
        Json::Null => None,
        Json::Bool(value) => Some(*value),
        _ => return None,
    };
    let captures = object.get("captures")?.as_bool()?;
    Some(TapeLetter::new(atoms, activation, captures))
}

/// A captured value's state form (FR-860): a capture keeps the value it took
/// at its activation across a restore, so the state writes it. A value is
/// written in FR-269's element encoding, which carries its kind and type; a
/// `Timestamp`, which that encoding has no form for, is written as its
/// canonical nanosecond text.
fn capture_json(value: &Value) -> Option<Json> {
    match value {
        Value::Timestamp(stamp) => Some(json!({"timestamp": stamp.nanoseconds().to_string()})),
        value => crate::result::encode_state_value(value).map(Json::String),
    }
}

fn capture_of(package: &fsl_package::CheckedPackage, value: &Json) -> Option<Value> {
    match value {
        Json::String(element) => crate::result::decode_state_value(package, element),
        Json::Object(object) if object.len() == 1 => {
            let text = object.get("timestamp")?.as_str()?;
            filament_exact::Timestamp::from_canonical_text(text)
                .ok()
                .map(Value::Timestamp)
        }
        _ => None,
    }
}

/// A progress assertion as its state form.
pub fn assertion_json(assertion: ProgressAssertion) -> Json {
    json!({
        "watermark": assertion.watermark,
        "complete": match assertion.completeness {
            ProgressCompleteness::Complete { through } => json!(through),
            ProgressCompleteness::Incomplete => Json::Null,
        },
        "scope_closed": assertion.scope == ProgressClosure::Closed,
        "execution_closed": assertion.execution == ProgressClosure::Closed,
    })
}

/// A progress assertion read from its state form.
pub fn assertion_of(value: &Json) -> Option<ProgressAssertion> {
    let closure = |closed: bool| {
        if closed {
            ProgressClosure::Closed
        } else {
            ProgressClosure::Open
        }
    };
    Some(ProgressAssertion {
        watermark: value["watermark"].as_u64()?,
        completeness: match &value["complete"] {
            Json::Null => ProgressCompleteness::Incomplete,
            through => ProgressCompleteness::Complete {
                through: through.as_u64()?,
            },
        },
        scope: closure(value["scope_closed"].as_bool()?),
        execution: closure(value["execution_closed"].as_bool()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{Decimal, EnumMember, Integer, Timestamp, VariantId};
    use filament_semantic_value::checking::CheckMode;

    fn package() -> fsl_package::CheckedPackage {
        let declarations = fsl_semantics::check::PackageDeclarations::new(
            fsl_semantics::check::fixture_source(),
            "fixture",
            fsl_foundation::IdentityLimits::default(),
        );
        fsl_package::CheckedPackage::link(
            declarations
                .check(
                    CheckMode::Linked,
                    filament_semantic_value::checking::CheckingLimits::default(),
                )
                .expect("the empty fixture package checks"),
        )
    }

    /// FR-860: an enum, decimal or timestamp capture has a state form, and
    /// reads back to the value it was written from.
    #[test]
    fn enum_decimal_and_timestamp_captures_round_trip() {
        let package = package();
        let variant = VariantId::new(crate::fixture_node(7), "busy").expect("a variant has a name");
        let values = [
            Value::Enum(EnumMember::new(variant, 1)),
            Value::Decimal(Decimal::new(Integer::from(-1250_i64), 2)),
            Value::Timestamp(
                Timestamp::from_canonical_text("1700000000000000123").expect("canonical"),
            ),
            Value::Boolean(true),
        ];
        for value in values {
            let written = capture_json(&value).expect("the capture has a state form");
            let read = capture_of(&package, &written).expect("the state form reads back");
            assert_eq!(
                filament_exact::compare_keys(&read, &value),
                Some(std::cmp::Ordering::Equal),
                "{written}"
            );
        }
    }
}
