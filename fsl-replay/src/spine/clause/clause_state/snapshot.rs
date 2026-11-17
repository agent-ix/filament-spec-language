// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-820: a carried clause state over snapshot positions (`over` names a
//! model object).
//!
//! Each snapshot is admitted once, when it arrives, and read into its letter
//! over the object `over` resolves to in the observation at position 0. A
//! position whose reading does not reduce keeps its object environment on the
//! tape. A `Finite` trace's report is the stream's outcome; a `Progressed` or
//! `Timestamped` trace's is FR-501's carried window over the tape. A
//! violation's decisive atom is found by the descent over the slice of the
//! tape it can read ([`ClauseStream::descent_floor`]), and its record reads
//! the snapshot at the atom's position, so the state keeps a snapshot's bytes
//! only where a decisive record can still read it: positions the stream may
//! still descend to with a `holds` atom that reads false, and positions held
//! as environments. Position 0's bytes are kept to resolve `over` again from
//! the state's bytes.

use filament_exact::Meter;
use fsl_eval::temporal::{
    descend_positions, push_object, read_object_letter, read_object_stop, ClauseStream,
    DecisiveAtom, DescentSlice, ObservedRun, StreamStop, TapeEntry, TapeReading,
};
use fsl_semantics::check::FormulaNode;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{
    admit_trace_snapshot, AdmissionFailure, DocumentProvision, DocumentRef, Provisions,
};
use std::collections::BTreeMap;

use super::super::temporal::{
    decisive_over, resolve_over, streamed_sample_report, with_decisive_by, OverMiss, SnapshotTape,
    TraceInput,
};
use super::super::{
    ClauseDisposition, ClauseRunReport, ClauseRunUsage, CompiledRun, ConsequentEventRefusal,
    ConsequentSourceEvents, TemporalVerdict,
};
use super::{ClauseRefusal, ClauseState, ClauseTraceKind};
use crate::proof_result::SettlementBasis;

/// What a snapshot state read of its positions besides the tape.
#[derive(Clone, Debug, Default)]
pub(super) struct SnapshotRead {
    /// The object `over` resolves to at position 0, once position 0 is read.
    pub(super) over: Option<Result<filament_exact::ObjectReference, OverMiss>>,
    /// The positions whose admission failed, by position: the first is the
    /// report. A trace that is not `Timestamped` admits nothing after it.
    pub(super) failed: BTreeMap<u64, AdmissionFailure>,
    /// The snapshot bytes the state keeps, by position.
    pub(super) bytes: BTreeMap<u64, Vec<u8>>,
    /// The position at which the stream's reading stopped.
    pub(super) stop: Option<u64>,
}

impl SnapshotRead {
    /// The number of positions whose snapshot the state holds.
    pub(super) fn retained(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(u64::MAX)
    }
}

/// Whether `entry` is a letter with a `holds` atom that reads false there: a
/// decisive atom reads false at its position.
fn reads_false(entry: Option<&TapeEntry>) -> bool {
    match entry {
        Some(TapeEntry::Read(letter)) => letter
            .atoms()
            .iter()
            .any(|(_, reading)| *reading == TapeReading::False),
        Some(TapeEntry::Held(_) | TapeEntry::Object(_)) | None => false,
    }
}

impl<'a> ClauseState<'a> {
    /// The compiled run the state's own reads go through.
    pub(super) fn compiled<'b>(&'b self, provision: &'b DocumentProvision) -> CompiledRun<'b> {
        CompiledRun {
            packages: self.open.packages,
            model_limits: self.open.limits.model,
            provisions: Provisions {
                snapshots: provision,
                invocations: provision,
            },
            observation_limits: self.open.limits.observation,
            accounting: self.open.limits.accounting,
            package: self.package,
            sources: self.open.sources,
            progress_capacity: fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        }
    }

    /// FR-106: admit the snapshot `bytes` of the document `reference`.
    pub(super) fn admit_snapshot(
        &self,
        reference: &DocumentRef,
        bytes: &[u8],
    ) -> Result<ObjectEnvironment, AdmissionFailure> {
        let graph = self.package.graph();
        let provision = DocumentProvision::from([(reference.clone(), bytes.to_vec())]);
        admit_trace_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            self.open.packages,
            self.open.limits.model,
            &provision,
            reference,
            self.open.limits.observation,
            &self.node.named_populations(),
        )
        .map(|(environment, _)| environment)
    }

    /// FR-820: admit the snapshot at `index` once and put it on the tape as
    /// its environment; [`Self::read_pending`] reads it into its letter.
    pub(super) fn push_snapshot(&mut self, index: u64, reference: &DocumentRef, bytes: &[u8]) {
        let timestamped = self.open.kind == ClauseTraceKind::Timestamped;
        if !timestamped && !self.objects.failed.is_empty() {
            return;
        }
        match self.admit_snapshot(reference, bytes) {
            Ok(environment) => {
                self.objects.bytes.insert(index, bytes.to_vec());
                self.tape.push(TapeEntry::Object(Box::new(environment)));
            }
            Err(failure) => {
                self.objects.bytes.insert(index, bytes.to_vec());
                self.objects.failed.insert(index, failure);
                // The position keeps its place on the tape; nothing reads it.
                self.tape
                    .push(TapeEntry::Read(fsl_eval::temporal::TapeLetter::default()));
            }
        }
    }

    /// FR-820: read the snapshot positions from `from` on, which the tape
    /// holds as environments in trace order, into their letters and the
    /// stream, resolving `over` at position 0, and drop the bytes no decisive
    /// record can read any more.
    pub(super) fn read_pending(&mut self, from: u64, meter: &mut Meter) {
        if let Some((first, _)) = self.objects.failed.first_key_value() {
            // FR-330: the first failed admission is the report; the bytes of
            // every later position, and of every other failure, are dropped.
            let first = *first;
            self.objects.failed.retain(|position, _| *position == first);
            self.objects
                .bytes
                .retain(|position, _| *position == first || *position == 0);
            return;
        }
        let len = u64::try_from(self.tape.len()).unwrap_or(u64::MAX);
        for index in from..len {
            let Some(position) = usize::try_from(index).ok() else {
                return;
            };
            if index == 0 {
                let environment = match self.tape.entry(0) {
                    Some(TapeEntry::Object(environment)) => Some(&**environment),
                    _ => None,
                };
                let provision = DocumentProvision::new();
                let resolved = match self.open.over.as_ref() {
                    Some(over) => resolve_over(
                        &self.compiled(&provision),
                        &self.node.clause().over().1,
                        over,
                        environment,
                    ),
                    None => Err(OverMiss::Missing),
                };
                self.objects.over = Some(resolved);
            }
            let Some(Ok(over)) = self.objects.over.clone() else {
                // `over` did not resolve: nothing is read, and only position
                // 0 is kept, to resolve it again from the state's bytes.
                self.objects.bytes.retain(|position, _| *position == 0);
                self.tape.truncate(position);
                for _ in position..usize::try_from(len).unwrap_or(usize::MAX) {
                    self.tape
                        .push(TapeEntry::Read(fsl_eval::temporal::TapeLetter::default()));
                }
                return;
            };
            let Some(TapeEntry::Object(environment)) = self.tape.entries().get(position).cloned()
            else {
                continue;
            };
            if let (Some(stream), None) = (self.stream.as_mut(), self.exhausted.as_ref()) {
                match push_object(
                    self.package,
                    self.node,
                    stream,
                    (&environment, &over),
                    meter,
                ) {
                    Ok(read) => {
                        if !read.is_empty() && self.objects.stop.is_none() {
                            self.objects.stop = Some(index);
                        }
                    }
                    Err(StreamStop::Incomplete(incomplete)) => self.exhausted = Some(incomplete),
                }
            }
            let mut letter_meter = Meter::new(self.open.limits.accounting);
            let entry = read_object_letter(
                self.package,
                self.node,
                *environment,
                &over,
                &mut letter_meter,
            );
            self.tape.replace(position, entry);
        }
        self.drop_unread_bytes();
    }

    /// The first position a decisive descent can still read: the stream's
    /// floor, position 0 without a stream, and none for a `Timestamped`
    /// trace, which carries no decisive record (FR-330).
    fn descent_floor(&self) -> Option<u64> {
        if self.open.kind == ClauseTraceKind::Timestamped {
            return None;
        }
        Some(match (&self.stream, &self.exhausted) {
            (Some(stream), None) => stream.descent_floor(),
            _ => 0,
        })
    }

    /// Keep the bytes of position 0, of positions held as environments, and
    /// of positions from the descent floor on whose letter reads a `holds`
    /// atom false.
    /// Once the decisive atom of the activation violated for good is found and
    /// no earlier activation is open, only its position is read again.
    pub(super) fn drop_unread_bytes(&mut self) {
        let floor = self.descent_floor();
        let found = match (self.stream.as_ref(), *self.snapshot_cache.borrow()) {
            (Some(stream), Some((at, atom)))
                if stream.violated() == Some(at) && !stream.has_open() =>
            {
                Some(atom.map(|atom| atom.position.index()))
            }
            _ => None,
        };
        let tape = &self.tape;
        self.objects.bytes.retain(|position, _| {
            let entry = tape.entry(*position);
            let readable = match found {
                Some(decisive) => decisive == Some(*position),
                None => floor.is_some_and(|floor| *position >= floor && reads_false(entry)),
            };
            *position == 0 || matches!(entry, Some(TapeEntry::Object(_))) || readable
        });
    }

    /// FR-330's decisive descent from the violated activation `at` over a
    /// trace of `len` positions, the first silent at `silent_from`. The atom
    /// of an activation violated for good is kept, so it is found once.
    fn descend(
        &self,
        at: u64,
        len: u64,
        silent_from: Option<u64>,
        meter: &mut Meter,
    ) -> Option<ObservedRun> {
        let Some(Ok(over)) = self.objects.over.as_ref() else {
            return None;
        };
        let for_good = self.stream.as_ref().and_then(ClauseStream::violated) == Some(at);
        if let Some((found_at, atom)) = self.snapshot_cache.borrow().as_ref() {
            if *found_at == at && for_good {
                return Some(ObservedRun {
                    result: Ok(fsl_eval::temporal::TemporalOutcome::Violated {
                        at: fsl_eval::temporal::TemporalPosition::new(at),
                    }),
                    stopped: None,
                    undefined: None,
                    decisive: Some(Ok(*atom)),
                });
            }
        }
        let from = self.descent_floor().unwrap_or(0).min(at);
        let run = descend_positions(
            self.package,
            self.node,
            DescentSlice {
                tape: &self.tape,
                over,
                from,
            },
            (len, at, silent_from),
            meter,
        )?;
        if let (true, Some(Ok(atom))) = (for_good, run.decisive.as_ref()) {
            self.snapshot_cache.replace(Some((at, *atom)));
        }
        Some(run)
    }

    /// The decisive record of `atom`, read over the snapshot at its position.
    fn record(
        &self,
        run: &CompiledRun<'_>,
        atom: DecisiveAtom,
        meter: &mut Meter,
    ) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>> {
        let failure = |invariant: &'static str| {
            Box::new(ClauseDisposition::EvaluateFault(
                filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
            ))
        };
        let position = atom.position.index();
        let seen = u64::try_from(self.seen.len()).unwrap_or(u64::MAX);
        // A silent sample and an aggregate atom have no `holds` observation.
        if position >= seen
            || !matches!(
                self.node.clause().formula().nodes().get(atom.atom.index()),
                Some(FormulaNode::Holds { .. })
            )
        {
            return Ok(None);
        }
        let Some(Ok(over)) = self.objects.over.as_ref() else {
            return Err(failure("decisive-snapshot-resolves-over"));
        };
        let document = usize::try_from(position)
            .ok()
            .and_then(|at| self.seen.get(at))
            .map(|seen| seen.reference.clone())
            .ok_or_else(|| failure("decisive-position-has-a-document"))?;
        let admitted;
        let objects = match self.tape.entry(position) {
            Some(TapeEntry::Object(environment)) => &**environment,
            _ => {
                let bytes = self
                    .objects
                    .bytes
                    .get(&position)
                    .ok_or_else(|| failure("decisive-position-has-an-environment"))?;
                admitted = self
                    .admit_snapshot(&document, bytes)
                    .map_err(|_| failure("decisive-snapshot-admits-again"))?;
                &admitted
            }
        };
        decisive_over(
            run,
            self.node,
            atom,
            over,
            objects,
            fsl_eval::value::ObservationIdentity {
                authority: document.authority,
                identity: document.identity,
            },
            meter,
        )
    }

    /// The tape input of a snapshot state's run, handed to `body`.
    pub(super) fn with_snapshot_tape<T>(&self, body: impl FnOnce(SnapshotTape<'_>) -> T) -> T {
        let missing = OverMiss::Missing;
        let descend = |at: u64, len: u64, silent: Option<u64>, meter: &mut Meter| {
            self.descend(at, len, silent, meter)
        };
        let record = |run: &CompiledRun<'_>, atom: DecisiveAtom, meter: &mut Meter| {
            self.record(run, atom, meter)
        };
        body(SnapshotTape {
            failure: self.objects.failed.values().next(),
            over: match self.objects.over.as_ref() {
                Some(Ok(over)) => Ok(over),
                Some(Err(miss)) => Err(miss),
                None => Err(&missing),
            },
            descend: &descend,
            record: &record,
        })
    }

    /// FR-330's report of a `Finite` snapshot trace from the stream's
    /// outcome: the first failed admission, then a `over` that did not
    /// resolve, then the stream's verdict with its counterexample and, for a
    /// violation, its decisive record.
    pub(super) fn snapshot_finite_report(&self, meter: &mut Meter) -> ClauseRunReport {
        let finish = |disposition: ClauseDisposition, meter: &Meter| ClauseRunReport {
            basis: match &disposition {
                ClauseDisposition::Temporal(verdict) => verdict.basis(),
                _ => SettlementBasis::Unavailable,
            },
            consequent_source_events: if disposition.truth().is_some() {
                ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
            } else {
                ConsequentSourceEvents::Unavailable
            },
            disposition,
            usage: ClauseRunUsage::from_meter(meter, Default::default()),
            ..super::pending_report()
        };
        if let Some(failure) = self.objects.failed.values().next() {
            return finish(ClauseDisposition::Admit(failure.clone()), meter);
        }
        let over = match (self.objects.over.as_ref(), self.open.over.as_ref()) {
            (Some(Ok(over)), _) => over,
            (Some(Err(miss)), Some(object)) => return finish(miss.disposition(object), meter),
            (None, _) | (Some(Err(_)), None) => {
                return finish(
                    ClauseDisposition::EvaluateFault(
                        filament_semantic_value::catalog::InternalFault::new(
                            "clause-state",
                            "snapshot-state-resolves-over",
                        ),
                    ),
                    meter,
                )
            }
        };
        let result = match (&self.exhausted, &self.stream) {
            (Some(incomplete), _) => Ok(fsl_eval::temporal::TemporalOutcome::Incomplete(
                incomplete.clone(),
            )),
            (None, Some(stream)) => stream.outcome(meter),
            (None, None) => Err(fsl_eval::temporal::TemporalRefusal::Fault(
                filament_semantic_value::catalog::InternalFault::new(
                    "clause-state",
                    "snapshot-state-has-its-stream",
                ),
            )),
        };
        let read = match self.objects.stop.map(|at| (at, self.tape.entry(at))) {
            Some((at, Some(TapeEntry::Object(environment)))) => {
                read_object_stop(self.package, self.node, (environment, over), at, meter)
            }
            _ => fsl_eval::temporal::SampleStop::default(),
        };
        let observed = ObservedRun {
            result,
            stopped: read.stopped,
            undefined: read.undefined,
            decisive: None,
        };
        let usage = ClauseRunUsage::from_meter(meter, Default::default());
        let mut report = streamed_sample_report(
            self.package,
            self.open.sources,
            self.node,
            observed,
            usage,
            || TraceInput::Finite(self.seen.iter().map(|p| p.reference.clone()).collect()),
        );
        if let ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) = report.disposition {
            let len = u64::try_from(self.seen.len()).unwrap_or(u64::MAX);
            let search = self
                .descend(at, len, None, meter)
                .and_then(|run| run.decisive);
            let provision = DocumentProvision::new();
            let run = self.compiled(&provision);
            let (disposition, counterexample, decisive) = with_decisive_by(
                search,
                report.disposition,
                report.counterexample.take(),
                |atom, meter| self.record(&run, atom, meter),
                meter,
            );
            let mut decided = finish(disposition, meter);
            decided.counterexample = counterexample;
            decided.decisive = decisive;
            decided.refused_at = report.refused_at;
            report = decided;
        }
        report
    }
}

impl ClauseState<'_> {
    /// Read back position `index` of a snapshot state's bytes: its letter, or
    /// its environment admitted again from the snapshot the bytes hold.
    pub(super) fn restore_snapshot_position(
        &mut self,
        index: u64,
        reference: &DocumentRef,
        document: Option<String>,
        letter: &serde_json::Value,
    ) -> Result<(), ClauseRefusal> {
        let mismatch = || ClauseRefusal::StateMismatch;
        let admitted = document.map(|text| {
            let bytes = text.into_bytes();
            let admitted = self.admit_snapshot(reference, &bytes);
            self.objects.bytes.insert(index, bytes);
            admitted
        });
        let entry = match (letter, admitted) {
            (serde_json::Value::Null, Some(Ok(environment))) => {
                TapeEntry::Object(Box::new(environment))
            }
            (serde_json::Value::Null, _) => return Err(mismatch()),
            (letter, admitted) => {
                if let Some(Err(failure)) = admitted {
                    self.objects.failed.insert(index, failure);
                }
                TapeEntry::Read(super::letter_of(letter).ok_or_else(mismatch)?)
            }
        };
        self.tape.push(entry);
        Ok(())
    }

    /// Read back what a snapshot state's bytes carry besides its positions:
    /// `over`, resolved again at position 0, the stream and the position at
    /// which its reading stopped, read again; and a `Finite` trace's report.
    pub(super) fn restore_snapshot(
        &mut self,
        carried: &serde_json::Value,
        stop: Option<u64>,
        meter: &mut Meter,
    ) -> Result<(), ClauseRefusal> {
        let mismatch = || ClauseRefusal::StateMismatch;
        if self.objects.failed.is_empty() {
            let readmitted;
            let environment = match self.tape.entry(0) {
                Some(TapeEntry::Object(environment)) => Some(&**environment),
                _ => match (self.seen.first(), self.objects.bytes.get(&0)) {
                    (Some(first), Some(bytes)) => {
                        readmitted = self
                            .admit_snapshot(&first.reference, bytes)
                            .map_err(|_| mismatch())?;
                        Some(&readmitted)
                    }
                    _ => None,
                },
            };
            if let (Some(environment), Some(over)) = (environment, self.open.over.as_ref()) {
                let provision = DocumentProvision::new();
                let resolved = resolve_over(
                    &self.compiled(&provision),
                    &self.node.clause().over().1,
                    over,
                    Some(environment),
                );
                self.objects.over = Some(resolved);
            }
        }
        if self.stream.is_some() && self.exhausted.is_none() {
            let mut stream = ClauseStream::from_state(self.node.clause(), carried, meter)
                .ok_or_else(mismatch)?;
            if let (Some(at), Some(Ok(over))) = (stop, self.objects.over.as_ref()) {
                if let Some(TapeEntry::Object(environment)) = self.tape.entry(at) {
                    let read =
                        read_object_stop(self.package, self.node, (environment, over), at, meter);
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
                }
            }
            self.stream = Some(stream);
        }
        self.objects.stop = stop;
        Ok(())
    }
}
