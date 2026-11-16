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
//! A trace the letters do not carry (snapshot positions, whose decisive
//! record reads the snapshot, and a timestamped trace, which the batch run
//! reorders into canonical order) holds every position by value, counted in
//! [`ClauseState::retained`], so a monitor's `retained_positions` still
//! bounds it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use filament_exact::{Cancel, CancelCause, Integer, Meter, Rational, Value, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::temporal::{
    read_sample_letter, InsufficientData, LetterTape, TapeEntry, TapeLetter, TapeReading,
};
use fsl_foundation::source::Source;
use fsl_semantics::check::{AggregateWindow, TemporalClauseNode, TemporalProfile};
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    admit_record_sample, DocumentProvision, DocumentRef, ObservationLimits,
};
use serde_json::{json, Value as Json};

use super::progress::{
    ProgressAssertion, ProgressClosure, ProgressCompleteness, ProgressLedger,
    ProgressLowerBoundary,
};
use super::temporal::{
    clause_trace_binding, run_package_temporal_over, PackageTemporalRequest, TapeInput,
    TimestampedCover, TimestampedObservation, TimestampedOrderKey,
    TimestampedWindow, TraceBinding, TraceInput,
};
use super::package_authentication::PackageAuthentication;
use super::trace_subject::{read_subject_letter, SubjectSamples};
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
    /// Every position held by value.
    Values,
}

impl Reading {
    fn name(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Observed => "observed",
            Self::Values => "values",
        }
    }
}

#[derive(Clone, Debug)]
struct Seen {
    reference: DocumentRef,
    stamp: u64,
    order_key: Option<TimestampedOrderKey>,
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
        let reading = match (sampled, request.kind, clause.clause().profile()) {
            (false, ..) | (true, ClauseTraceKind::Timestamped, _) => Reading::Values,
            (true, ClauseTraceKind::Finite, TemporalProfile::EventPositionFalseExtension) => {
                Reading::Subject
            }
            (true, ..) => Reading::Observed,
        };
        Ok(Self {
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
        })
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
        let index = u64::try_from(self.seen.len()).map_err(|_| fault("position-in-u64"))?;
        let held_values = self.values.clone();
        let held_pins = self.pinned.clone();
        let held_captures = self.captures.clone();
        let snapshot = match &position.value {
            AdmittedValue::Snapshot(bytes) => {
                let fresh = !self.snapshots.contains_key(&position.reference);
                self.snapshots
                    .insert(position.reference.clone(), bytes.to_vec());
                fresh.then(|| position.reference.clone())
            }
            AdmittedValue::Sample(_) => None,
        };
        self.seen.push(Seen {
            reference: position.reference,
            stamp: position.stamp,
            order_key: position.order_key,
        });
        if let AdmittedValue::Sample(sample) = position.value {
            self.read(index, sample);
        }
        match self.evaluate(self.progress, cancel) {
            Ok(advance) => Ok(advance),
            Err(refusal) => {
                self.seen.pop();
                if self.reading != Reading::Values {
                    self.tape.pop();
                }
                self.values = held_values;
                self.pinned = held_pins;
                self.captures = held_captures;
                if let Some(reference) = snapshot {
                    self.snapshots.remove(&reference);
                }
                Err(refusal)
            }
        }
    }

    /// Read the sample at `index` into its letter, and drop the samples no
    /// window still reads.
    fn read(&mut self, index: u64, sample: Value) {
        let mut meter = Meter::new(self.open.limits.accounting);
        match self.reading {
            Reading::Values => {}
            Reading::Observed => {
                let entry = read_sample_letter(self.package, self.node, &sample, &mut meter);
                self.tape.push(entry);
            }
            Reading::Subject => {
                self.values.insert(index, sample);
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
                let reach = self.window_reach();
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
            }
        }
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
        let advance = self.evaluate(Some(*progress), cancel)?;
        self.progress = Some(*progress);
        Ok(advance)
    }

    /// The trace of the first `seen` positions under `assertion`.
    fn trace_of(&self, seen: usize, assertion: ProgressAssertion) -> TraceInput {
        let positions = &self.seen[..seen.min(self.seen.len())];
        let refs = || positions.iter().map(|p| p.reference.clone()).collect();
        match (self.open.kind, self.open.binding.as_ref()) {
            (ClauseTraceKind::Progressed, Some(binding)) => TraceInput::Progressed {
                trace: refs(),
                progress: assertion,
                lower_boundary: self.open.lower_boundary,
                binding: binding.clone(),
            },
            (ClauseTraceKind::Timestamped, binding) => TraceInput::Timestamped {
                observations: positions
                    .iter()
                    .map(|position| TimestampedObservation {
                        document: position.reference.clone(),
                        stamp: position.stamp,
                        order_key: position.order_key.clone(),
                    })
                    .collect(),
                progress: assertion,
                window: Box::new(TimestampedWindow {
                    binding: binding.cloned(),
                    authoritative_origin: matches!(
                        self.open.lower_boundary,
                        ProgressLowerBoundary::AuthoritativeOrigin
                    ),
                    cover: self.open.cover,
                    missing_valuations: Vec::new(),
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
            authentication: self.open.authentication.as_deref().map(PackageAuthentication::temporal),
            cancel,
        };
        let shorter;
        let tape = match self.reading {
            Reading::Values => None,
            Reading::Subject | Reading::Observed => {
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
        cancel: &Cancel,
    ) -> Result<Advance, ClauseRefusal> {
        if !self.evaluable(assertion) {
            return Ok(Advance {
                report: Arc::clone(&self.report),
                decided: None,
            });
        }
        let seen = self.seen.len();
        let trace = self.trace_of(seen, assertion.unwrap_or_else(Self::open_assertion));
        let report = self.run(&trace, seen, false, cancel);
        cancelled(cancel)?;
        if let ClauseDisposition::ProgressContradiction(refusal) = report.disposition {
            return Err(ClauseRefusal::Contradiction(refusal));
        }
        let report = Arc::new(report);
        let decided = self.decided(&report, assertion, cancel)?;
        self.report = Arc::clone(&report);
        self.evaluated = Some((seen, assertion));
        Ok(Advance { report, decided })
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
        let trace = TraceInput::Progressed {
            trace: self.seen.iter().map(|p| p.reference.clone()).collect(),
            progress,
            lower_boundary: self.open.lower_boundary,
            binding,
        };
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
        let held = match self.reading {
            Reading::Values => u64::try_from(self.seen.len()).unwrap_or(u64::MAX),
            Reading::Observed => self.tape.held(),
            Reading::Subject => u64::try_from(self.values.len()).unwrap_or(u64::MAX),
        };
        held
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
                    Some(TapeEntry::Held(_)) | None => Json::Null,
                };
                json!({
                    "authority": position.reference.authority,
                    "identity": position.reference.identity,
                    "stamp": position.stamp,
                    "order_key": position.order_key.as_ref().map(|key|
                        json!({"authority": key.authority, "key": key.key})),
                    "letter": letter,
                    "document": self.snapshots.get(&position.reference)
                        .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
                })
            })
            .collect();
        let captures = self
            .captures
            .iter()
            .map(|value| capture_json(value).ok_or_else(|| fault("capture-has-a-state-form")))
            .collect::<Result<Vec<_>, _>>()?;
        let document = json!({
            "format": CLAUSE_STATE_FORMAT,
            "clause": self.node.clause().name(),
            "reading": self.reading.name(),
            "positions": positions,
            "values": self.values.keys().collect::<Vec<_>>(),
            "pinned": self.pinned.iter().collect::<Vec<_>>(),
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
        const MEMBERS: [&str; 9] = [
            "format",
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
        // A sample position's identity is its canonical text, so a held
        // sample is admitted again from it.
        let admit = |reference: &DocumentRef| -> Result<Value, ClauseRefusal> {
            let record = record.as_ref().ok_or_else(mismatch)?;
            admit_record_sample(
                package.graph().scope().types(),
                record,
                reference.identity.as_bytes(),
            )
            .map_err(|_| mismatch())
        };
        let positions = object["positions"].as_array().ok_or_else(mismatch)?;
        for position in positions {
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
            if let Some(document) = text("document") {
                state
                    .snapshots
                    .insert(reference.clone(), document.into_bytes());
            }
            if state.reading != Reading::Values {
                let entry = match &position["letter"] {
                    Json::Null => TapeEntry::Held(admit(&reference)?),
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
            let value = admit(&position.reference)?;
            state.values.insert(index, value);
        }
        state.pinned = indices("pinned")?.into_iter().collect();
        state.captures = object["captures"]
            .as_array()
            .ok_or_else(mismatch)?
            .iter()
            .map(|capture| capture_of(capture).ok_or_else(mismatch))
            .collect::<Result<_, _>>()?;
        state.progress = match &object["progress"] {
            Json::Null => None,
            value => Some(assertion_of(value).ok_or_else(mismatch)?),
        };
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
            let trace = state.trace_of(seen, assertion.unwrap_or_else(Self::open_assertion));
            state.report = Arc::new(state.run(&trace, seen, false, cancel));
            cancelled(cancel)?;
            state.evaluated = Some((seen, assertion));
        }
        Ok(state)
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
/// at its activation across a restore, so the state writes it.
fn capture_json(value: &Value) -> Option<Json> {
    Some(match value {
        Value::Boolean(value) => json!({"boolean": value}),
        Value::Integer(value) => json!({"integer": value.to_string()}),
        Value::Rational(value) => json!({
            "rational": [value.numerator().to_string(), value.denominator().to_string()]
        }),
        Value::String(value) => json!({"string": value}),
        _ => return None,
    })
}

fn capture_of(value: &Json) -> Option<Value> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    let (kind, member) = object.iter().next()?;
    Some(match kind.as_str() {
        "boolean" => Value::Boolean(member.as_bool()?),
        "integer" => Value::Integer(member.as_str()?.parse::<Integer>().ok()?),
        "rational" => {
            let [numerator, denominator] = member.as_array()?.as_slice() else {
                return None;
            };
            Value::Rational(
                Rational::new(
                    numerator.as_str()?.parse::<Integer>().ok()?,
                    denominator.as_str()?.parse::<Integer>().ok()?,
                )
                .ok()?,
            )
        }
        "string" => Value::String(member.as_str()?.to_owned()),
        _ => return None,
    })
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
