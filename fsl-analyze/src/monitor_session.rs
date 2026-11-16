// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-643: the stepping monitor. A [`MonitorSession`] takes the observations
//! of one trace one at a time, carries the progress ledger and the stamp
//! order between steps, and reports after each step the status FR-283's batch
//! `monitor` gives over the trace observed so far.
//!
//! Each clause is evaluated by the same trace evaluator the batch route
//! uses, over the positions seen so far, so equality with the batch run holds
//! by construction. A session keeps every position a still-pending clause
//! can read (it prunes none), so `retained_positions` is enforced as the
//! FR-643 rule states it, not as a memory bound.

use std::collections::BTreeMap;

use filament_exact::{Cancel, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_foundation::source::Source;
use fsl_package::number::ExactRational;
use fsl_package::{CheckedPackage, TemporalClockSelection};
use fsl_replay::spine::{
    authenticate_emission, authenticate_package_trace, read_ledger, run_package_temporal,
    write_ledger, ClauseDisposition, ClauseRunReport, ClauseRunUsage, ConsequentSourceEvents,
    ContradictedDimension, LedgerReadError, PackageAuthentication, PackageTemporalRequest,
    ProgressAssertion, ProgressClosure, ProgressCompleteness, ProgressContradiction,
    ProgressLedger, ProgressLimitReached, ProgressLowerBoundary, ProgressRefusal, ProgressValue,
    SpineLimits, TemporalVerdict, TimestampedCover, TimestampedObservation, TimestampedOrderKey,
    TimestampedWindow, TraceBinding, TraceInput,
};
use fsl_semantics::check::TemporalProfile;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{document_reference, DocumentProvision, DocumentRef};
use serde_json::{json, Value};

use crate::monitor::{
    record_of, refused, request_failure, Failure, MonitorClause, MonitorLimits, MonitorRecord,
    MonitorRefusal, MonitorVerdict, Runs,
};

/// The format member of a checkpoint.
pub const STATE_FORMAT: &str = "filament.monitor-state/1";

/// The trace kind a session's observations form (FR-641).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorTraceKind {
    /// A finite trace or prefix.
    Finite,
    /// A finite event-position or fixed-sample trace with progress.
    Progressed,
    /// A timestamped-event trace with progress.
    Timestamped,
    /// A timed trace (FR-642); the clock kind it needs is not built.
    Timed,
}

impl MonitorTraceKind {
    fn name(self) -> &'static str {
        match self {
            Self::Finite => "finite",
            Self::Progressed => "progressed",
            Self::Timestamped => "timestamped",
            Self::Timed => "timed",
        }
    }

    fn carries_progress(self) -> bool {
        matches!(self, Self::Progressed | Self::Timestamped)
    }
}

/// A session's opening request (FR-643 Inputs).
pub struct MonitorOpen<'a> {
    /// The selected clauses, in request order.
    pub clauses: Vec<MonitorClause>,
    /// The kind of trace the observations form.
    pub kind: MonitorTraceKind,
    /// Whether the first position is an authoritative origin.
    pub lower_boundary: ProgressLowerBoundary,
    /// The trace binding a `Progressed` or `Timestamped` trace asserts.
    pub binding: Option<TraceBinding>,
    /// How a `Timestamped` trace's stamps cover time; exact when absent.
    pub cover: Option<TimestampedCover>,
    /// The progress ledger the session starts from (FR-502).
    pub ledger: Option<ProgressLedger>,
    /// FR-056's package input.
    pub packages: &'a PackageInput,
    /// The sources the package was checked from.
    pub sources: &'a [Source],
    /// The FR-050 clock configurations a progress-carrying kind needs.
    pub temporal_selections: &'a [TemporalClockSelection],
}

/// One observation's position.
#[derive(Clone, Copy, Debug)]
pub enum MonitorPosition<'a> {
    /// A snapshot document's bytes; it names its own identity.
    Snapshot(&'a [u8]),
    /// A record sample as its canonical JSON object text, which is the
    /// position's identity (FR-641).
    Sample(&'a str),
}

/// An observation's stamp.
#[derive(Clone, Debug)]
pub enum MonitorStamp {
    /// A tick.
    Tick(u64),
    /// An exact rational, for a timed trace.
    Exact(ExactRational),
}

/// One observation (FR-643 Inputs).
#[derive(Clone, Debug)]
pub struct MonitorObservation<'a> {
    /// The position.
    pub position: MonitorPosition<'a>,
    /// The stamp the kind requires, if it takes one.
    pub stamp: Option<MonitorStamp>,
    /// The order key, where the kind allows one.
    pub order_key: Option<TimestampedOrderKey>,
}

/// How a trace ends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceEnd {
    /// The execution is over.
    Closed,
    /// The execution may go on.
    Open,
}

/// One clause's status after a step.
#[derive(Clone, Debug)]
pub enum ClauseStatus {
    /// The result is final and never changes.
    Settled(MonitorRecord),
    /// The result may still change with later observations.
    Pending(MonitorRecord),
}

impl ClauseStatus {
    /// The record of the status.
    pub fn record(&self) -> &MonitorRecord {
        match self {
            Self::Settled(record) | Self::Pending(record) => record,
        }
    }

    /// Whether the result is final.
    pub fn is_settled(&self) -> bool {
        matches!(self, Self::Settled(_))
    }
}

/// What one step reports.
#[derive(Clone, Debug)]
pub struct MonitorStep {
    /// Each clause's name and status, in `clauses` order.
    pub statuses: Vec<(String, ClauseStatus)>,
    /// The names of the clauses whose status changed at this step. The first
    /// step names every clause, since each has its first status then.
    pub changed: Vec<String>,
}

/// A session's checkpoint, bytes in the closed JSON form
/// `filament.monitor-state/1`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorState(Vec<u8>);

impl MonitorState {
    /// The state document's bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// A state read from `bytes`; [`MonitorSession::resume`] checks it.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

#[derive(Clone, Debug)]
struct Position {
    reference: DocumentRef,
    stamp: u64,
    order_key: Option<TimestampedOrderKey>,
    /// The snapshot text; `None` for a sample.
    document: Option<String>,
}

#[derive(Clone, Debug)]
struct Slot {
    record: MonitorRecord,
    settled: bool,
    /// The number of positions seen when the clause settled.
    settled_at: Option<usize>,
    /// Whether `retained_positions` ended the clause.
    limited: bool,
    announced: bool,
}

/// FR-643's stepping monitor.
pub struct MonitorSession<'a> {
    package: &'a CheckedPackage,
    request: MonitorOpen<'a>,
    limits: MonitorLimits,
    authentication: Option<PackageAuthentication>,
    samples: bool,
    ledger: Option<ProgressLedger>,
    positions: Vec<Position>,
    snapshots: DocumentProvision,
    assertion: Option<ProgressAssertion>,
    slots: Vec<Slot>,
}

fn observation(member: &'static str, cause: &'static str) -> Failure {
    StageFailure::Refused(MonitorRefusal::Observation { member, cause })
}

fn cancelled(cancel: &Cancel) -> Result<(), Failure> {
    match cancel.cause() {
        Some(cause) => Err(StageFailure::Cancelled(cause)),
        None => Ok(()),
    }
}

fn pending_report() -> ClauseRunReport {
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

fn limit_report(value: u64) -> ClauseRunReport {
    let verdict = TemporalVerdict::ProgressLimitReached(ProgressLimitReached {
        limit: "node count",
        value,
        setting: "monitor.retained_positions",
    });
    ClauseRunReport {
        basis: verdict.basis(),
        disposition: ClauseDisposition::Temporal(verdict),
        ..pending_report()
    }
}

fn state_mismatch() -> Failure {
    let fields = BTreeMap::from([
        ("reason", "monitor-state-mismatch".to_owned()),
        ("stage", "admit".to_owned()),
    ]);
    StageFailure::Refused(MonitorRefusal::State {
        failure: Box::new(refused(Code::InvalidRuntimeInput, "invalid-value", fields)),
    })
}

impl<'a> MonitorSession<'a> {
    /// FR-643: open a session over `request`'s clauses.
    ///
    /// # Errors
    ///
    /// A refusal for an unknown clause, a clause whose subject does not fit
    /// the others, a kind its profile does not read (FR-645
    /// `trace-kind-mismatch`), a missing binding or selection, or a kind
    /// that cannot run yet.
    pub fn open(
        package: &'a CheckedPackage,
        request: MonitorOpen<'a>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<Self, Failure> {
        if request.clauses.is_empty() {
            return Err(observation("clauses", "invalid-value"));
        }
        if request.kind == MonitorTraceKind::Timed {
            return Err(observation("kind", "invalid-value"));
        }
        if request.kind.carries_progress() && request.binding.is_none() {
            return Err(observation("binding", "missing-member"));
        }
        if request.ledger.is_some() && !request.kind.carries_progress() {
            return Err(observation("ledger", "invalid-value"));
        }
        let mut samples = None;
        for clause in &request.clauses {
            let Some(node) = package
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.clause().name() == clause.name)
            else {
                return Err(StageFailure::Refused(MonitorRefusal::UnknownClause {
                    name: clause.name.clone(),
                }));
            };
            let model = matches!(node.clause().over().1, ValueType::Reference(_));
            let fits = if model {
                clause.over.is_some()
            } else {
                clause.over.is_none()
            };
            if !fits {
                let cause = if model {
                    "wrong-role-mapping"
                } else {
                    "invalid-value"
                };
                return Err(StageFailure::Refused(MonitorRefusal::ClauseRefused {
                    clause: clause.name.clone(),
                    failure: Box::new(refused(Code::InvalidRuntimeInput, cause, BTreeMap::new())),
                }));
            }
            if *samples.get_or_insert(!model) != !model {
                return Err(observation("clauses", "invalid-value"));
            }
        }
        let first = request.clauses[0].name.clone();
        let authentication = if request.kind.carries_progress() {
            Some(
                authenticate_emission(
                    package,
                    request.temporal_selections,
                    request.packages,
                    &SpineLimits::default(),
                    cancel,
                )
                .map_err(|failure| Runs::emission_failure(&request.clauses, failure, &first))?,
            )
        } else {
            None
        };
        let session = Self {
            package,
            samples: samples.unwrap_or(false),
            ledger: request.ledger.clone(),
            request,
            limits: *limits,
            authentication,
            positions: Vec::new(),
            snapshots: DocumentProvision::new(),
            assertion: None,
            slots: Vec::new(),
        };
        session.check_kind()?;
        let mut session = session;
        session.slots = session.fresh_slots()?;
        Ok(session)
    }

    /// FR-645: refuse a clause whose profile does not read the kind, before
    /// any observation.
    fn check_kind(&self) -> Result<(), Failure> {
        let probe = self.trace_of(&[], self.dummy_assertion());
        for clause in &self.request.clauses {
            let checked = authenticate_package_trace(
                self.package,
                &clause.name,
                &probe,
                self.authentication
                    .as_ref()
                    .map(PackageAuthentication::temporal),
            );
            if let Err(disposition) = checked {
                return Err(
                    request_failure(&clause.name, &disposition).unwrap_or_else(|| {
                        StageFailure::Fault(InternalFault::new(
                            "monitor",
                            "trace-authentication-refuses-at-admit",
                        ))
                    }),
                );
            }
        }
        Ok(())
    }

    fn dummy_assertion(&self) -> ProgressAssertion {
        ProgressAssertion {
            watermark: 0,
            completeness: ProgressCompleteness::Incomplete,
            scope: ProgressClosure::Open,
            execution: ProgressClosure::Open,
        }
    }

    fn fresh_slots(&self) -> Result<Vec<Slot>, Failure> {
        self.request
            .clauses
            .iter()
            .map(|clause| {
                Ok(Slot {
                    record: record_of(
                        self.package,
                        self.request.sources,
                        &clause.name,
                        pending_report(),
                    )?,
                    settled: false,
                    settled_at: None,
                    limited: false,
                    announced: false,
                })
            })
            .collect()
    }

    /// The trace of `positions` under `assertion`.
    fn trace_of(&self, positions: &[Position], assertion: ProgressAssertion) -> TraceInput {
        let binding = self.request.binding.clone();
        let refs = || positions.iter().map(|p| p.reference.clone()).collect();
        match (self.request.kind, binding) {
            (MonitorTraceKind::Progressed, Some(binding)) => TraceInput::Progressed {
                trace: refs(),
                progress: assertion,
                lower_boundary: self.request.lower_boundary,
                binding,
            },
            (MonitorTraceKind::Timestamped, binding) => TraceInput::Timestamped {
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
                    binding,
                    authoritative_origin: matches!(
                        self.request.lower_boundary,
                        ProgressLowerBoundary::AuthoritativeOrigin
                    ),
                    cover: self.request.cover,
                    missing_valuations: Vec::new(),
                }),
            },
            _ => TraceInput::Finite(refs()),
        }
    }

    /// The position an observation names, checked against the kind and the
    /// retained watermark.
    fn admit_observation(
        &self,
        observation_in: &MonitorObservation<'_>,
    ) -> Result<Position, Failure> {
        let kind = self.request.kind;
        let index = u64::try_from(self.positions.len())
            .map_err(|_| observation("position", "invalid-value"))?;
        let stamp = match (&observation_in.stamp, kind) {
            (Some(MonitorStamp::Tick(tick)), MonitorTraceKind::Timestamped) => *tick,
            (None, MonitorTraceKind::Timestamped) => {
                return Err(observation("stamp", "missing-member"))
            }
            (Some(MonitorStamp::Exact(_)), MonitorTraceKind::Timestamped) => {
                return Err(observation("stamp", "wrong-value-kind"))
            }
            (None, _) => index,
            (Some(_), _) => return Err(observation("stamp", "unknown-member")),
        };
        if observation_in.order_key.is_some() && kind != MonitorTraceKind::Timestamped {
            return Err(observation("order_key", "unknown-member"));
        }
        if let Some(assertion) = self.assertion {
            if stamp <= assertion.watermark {
                let clock = self
                    .request
                    .binding
                    .as_ref()
                    .map(|binding| binding.clock_name.clone())
                    .unwrap_or_default();
                let contradiction = ProgressContradiction {
                    dimension: ContradictedDimension::Watermark,
                    clock,
                    retained: ProgressValue::Watermark(assertion.watermark),
                    asserted: ProgressValue::Watermark(stamp),
                };
                return Err(StageFailure::Refused(MonitorRefusal::Contradiction(
                    Box::new(ProgressRefusal::from(contradiction)),
                )));
            }
        }
        let (reference, document) = match (observation_in.position, self.samples) {
            (MonitorPosition::Sample(text), true) => {
                (fsl_replay::spine::sample_position(text), None)
            }
            (MonitorPosition::Snapshot(bytes), false) => {
                let reference = document_reference(bytes)
                    .ok_or_else(|| observation("position", "missing-member"))?;
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|_| observation("position", "invalid-value"))?;
                (reference, Some(text))
            }
            _ => return Err(observation("position", "wrong-value-kind")),
        };
        Ok(Position {
            reference,
            stamp,
            order_key: observation_in.order_key.clone(),
            document,
        })
    }

    /// FR-643: take one observation.
    ///
    /// A refused, cancelled or failed step leaves the session as it was.
    ///
    /// # Errors
    ///
    /// A refusal of the observation (order, form, admission) or of a clause.
    pub fn step(
        &mut self,
        observation: MonitorObservation<'_>,
        cancel: &Cancel,
    ) -> Result<MonitorStep, Failure> {
        cancelled(cancel)?;
        let position = self.admit_observation(&observation)?;
        let inserted = match &position.document {
            Some(text) => {
                let fresh = !self.snapshots.contains_key(&position.reference);
                self.snapshots
                    .insert(position.reference.clone(), text.clone().into_bytes());
                fresh
            }
            None => false,
        };
        self.positions.push(position.clone());
        match self.evaluate(self.assertion, cancel) {
            Ok((slots, ledger)) => Ok(self.commit(slots, ledger)),
            Err(failure) => {
                self.positions.pop();
                if inserted {
                    self.snapshots.remove(&position.reference);
                }
                Err(failure)
            }
        }
    }

    /// FR-643: take a progress assertion.
    ///
    /// # Errors
    ///
    /// A refusal for a kind that carries no progress, or the contradiction
    /// FR-502 finds; the session is unchanged.
    pub fn assert_progress(
        &mut self,
        progress: ProgressAssertion,
        cancel: &Cancel,
    ) -> Result<MonitorStep, Failure> {
        cancelled(cancel)?;
        if !self.request.kind.carries_progress() {
            return Err(observation("progress", "unknown-member"));
        }
        if let Some(contradiction) = self.regression(progress) {
            return Err(StageFailure::Refused(MonitorRefusal::Contradiction(
                Box::new(ProgressRefusal::from(contradiction)),
            )));
        }
        let (slots, ledger) = self.evaluate(Some(progress), cancel)?;
        self.assertion = Some(progress);
        Ok(self.commit(slots, ledger))
    }

    /// FR-502: what `next` retreats from in the assertion the session retains.
    fn regression(&self, next: ProgressAssertion) -> Option<ProgressContradiction> {
        let held = self.assertion?;
        let clock = self
            .request
            .binding
            .as_ref()
            .map(|binding| binding.clock_name.clone())
            .unwrap_or_default();
        let contradiction = |dimension, retained, asserted| {
            Some(ProgressContradiction {
                dimension,
                clock: clock.clone(),
                retained,
                asserted,
            })
        };
        if next.watermark < held.watermark {
            return contradiction(
                ContradictedDimension::Watermark,
                ProgressValue::Watermark(held.watermark),
                ProgressValue::Watermark(next.watermark),
            );
        }
        let retreats = match (held.completeness, next.completeness) {
            (ProgressCompleteness::Complete { .. }, ProgressCompleteness::Incomplete) => true,
            (
                ProgressCompleteness::Complete { through: held },
                ProgressCompleteness::Complete { through: next },
            ) => next < held,
            _ => false,
        };
        if retreats {
            return contradiction(
                ContradictedDimension::Completeness,
                ProgressValue::Completeness(held.completeness),
                ProgressValue::Completeness(next.completeness),
            );
        }
        if held.scope == ProgressClosure::Closed && next.scope == ProgressClosure::Open {
            return contradiction(
                ContradictedDimension::Closure,
                ProgressValue::DecisionScope(held.scope),
                ProgressValue::DecisionScope(next.scope),
            );
        }
        if held.execution == ProgressClosure::Closed && next.execution == ProgressClosure::Open {
            return contradiction(
                ContradictedDimension::Closure,
                ProgressValue::SurroundingExecution(held.execution),
                ProgressValue::SurroundingExecution(next.execution),
            );
        }
        None
    }

    fn commit(&mut self, slots: Vec<Slot>, ledger: Option<ProgressLedger>) -> MonitorStep {
        let mut changed = Vec::new();
        for (index, slot) in slots.iter().enumerate() {
            let before = self.slots.get(index);
            let same = before.is_some_and(|before| {
                before.announced
                    && before.settled == slot.settled
                    && before.record.verdict == slot.record.verdict
            });
            if !same {
                changed.push(slot.record.clause.clone());
            }
        }
        self.slots = slots
            .into_iter()
            .map(|slot| Slot {
                announced: true,
                ..slot
            })
            .collect();
        if ledger.is_some() {
            self.ledger = ledger;
        }
        MonitorStep {
            statuses: self
                .slots
                .iter()
                .map(|slot| (slot.record.clause.clone(), Self::status_of(slot)))
                .collect(),
            changed,
        }
    }

    fn status_of(slot: &Slot) -> ClauseStatus {
        if slot.settled {
            ClauseStatus::Settled(slot.record.clone())
        } else {
            ClauseStatus::Pending(slot.record.clone())
        }
    }

    /// Whether `assertion` covers every position seen, so the run can read
    /// them; a progress-carrying kind is evaluated only then.
    fn evaluable(&self, assertion: Option<ProgressAssertion>) -> bool {
        if self.positions.is_empty() {
            return false;
        }
        if !self.request.kind.carries_progress() {
            return true;
        }
        let newest = self.positions.iter().map(|p| p.stamp).max().unwrap_or(0);
        assertion.is_some_and(|assertion| assertion.watermark >= newest)
    }

    /// Evaluate every unsettled clause over the positions seen.
    fn evaluate(
        &self,
        assertion: Option<ProgressAssertion>,
        cancel: &Cancel,
    ) -> Result<(Vec<Slot>, Option<ProgressLedger>), Failure> {
        let mut slots = self.slots.clone();
        let mut ledger = self.ledger.clone();
        let evaluable = self.evaluable(assertion);
        let trace = self.trace_of(&self.positions, assertion.unwrap_or(self.dummy_assertion()));
        let mut carried: Option<ProgressLedger> = None;
        for (index, clause) in self.request.clauses.iter().enumerate() {
            cancelled(cancel)?;
            let slot = &mut slots[index];
            if slot.settled {
                continue;
            }
            if evaluable {
                let input = carried.as_ref().or(ledger.as_ref());
                let report = run_package_temporal(&PackageTemporalRequest {
                    package: self.package,
                    packages: self.request.packages,
                    model_limits: self.limits.normalization,
                    snapshots: &self.snapshots,
                    observation_limits: self.limits.observation,
                    accounting: self.limits.accounting,
                    name: &clause.name,
                    trace: &trace,
                    over: clause.over.as_ref(),
                    ledger: input,
                    sources: self.request.sources,
                    authentication: self
                        .authentication
                        .as_ref()
                        .map(PackageAuthentication::temporal),
                    cancel,
                });
                cancelled(cancel)?;
                if let ClauseDisposition::ProgressContradiction(refusal) = report.disposition {
                    return Err(StageFailure::Refused(MonitorRefusal::Contradiction(
                        refusal,
                    )));
                }
                if let Some(updated) = &report.ledger {
                    carried = Some(updated.clone());
                }
                slot.record = record_of(self.package, self.request.sources, &clause.name, report)?;
            }
            let (settled, limited) = self.classify(&clause.name, slot.record.verdict)?;
            slot.settled = settled;
            slot.limited = limited;
            if settled {
                slot.settled_at = Some(self.positions.len());
            }
            if !settled && self.over_limit(&clause.name)? {
                slot.record = record_of(
                    self.package,
                    self.request.sources,
                    &clause.name,
                    limit_report(self.limits.retained_positions),
                )?;
                slot.settled = true;
                slot.limited = true;
                slot.settled_at = Some(self.positions.len());
            }
        }
        if carried.is_some() {
            ledger = carried;
        }
        Ok((slots, ledger))
    }

    fn profile_of(&self, name: &str) -> Result<TemporalProfile, Failure> {
        self.package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .map(|node| node.clause().profile())
            .ok_or_else(|| {
                StageFailure::Fault(InternalFault::new("monitor", "session-clause-is-declared"))
            })
    }

    /// Whether a verdict is final. Only an infinite-trace clause settles
    /// early: a bad prefix and a satisfied eventuality stay so under any
    /// extension, which a closed-trace profile's verdict does not.
    fn classify(&self, name: &str, verdict: MonitorVerdict) -> Result<(bool, bool), Failure> {
        let infinite = self.profile_of(name)? == TemporalProfile::InfiniteTrace;
        let settled = infinite
            && matches!(
                verdict,
                MonitorVerdict::Violated | MonitorVerdict::Undefined | MonitorVerdict::Tested
            );
        Ok((settled, false))
    }

    /// FR-643 "Retained state": a pending clause that would retain more than
    /// `retained_positions` positions.
    fn over_limit(&self, name: &str) -> Result<bool, Failure> {
        let seen = u64::try_from(self.positions.len()).unwrap_or(u64::MAX);
        let node = self
            .package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .ok_or_else(|| {
                StageFailure::Fault(InternalFault::new("monitor", "session-clause-is-declared"))
            })?;
        let retained = if node.clause().profile() == TemporalProfile::InfiniteTrace {
            seen
        } else {
            node.clause()
                .horizon()
                .map_or(seen, |horizon| seen.min(horizon.saturating_add(1)))
        };
        Ok(retained > self.limits.retained_positions)
    }

    /// FR-643: end the trace and return FR-283's outcome over it.
    ///
    /// Closure of a progress-carrying trace is what its last progress
    /// assertion says; a finite trace is the same trace closed or open.
    ///
    /// # Errors
    ///
    /// A cancelled call.
    pub fn finish(
        self,
        _end: TraceEnd,
        cancel: &Cancel,
    ) -> Result<crate::monitor::MonitorOutcome, Failure> {
        cancelled(cancel)?;
        Ok(crate::monitor::MonitorOutcome {
            records: self.slots.into_iter().map(|slot| slot.record).collect(),
        })
    }

    fn package_identity(&self) -> String {
        let mut text = String::new();
        for source in self.request.sources {
            let identity = source.identity();
            text.push_str(&format!("{}:{};", identity.authority, identity.identity));
        }
        for node in self.package.graph().temporal_clauses() {
            text.push_str(&format!("{:?};", node.identity()));
        }
        text
    }

    fn selections(&self) -> Value {
        Value::Array(
            self.request
                .clauses
                .iter()
                .map(|clause| {
                    json!({"name": clause.name, "over": clause.over.as_ref().map(|over|
                        json!({"population": over.population, "key": over.key}))})
                })
                .collect(),
        )
    }

    /// FR-643: the session's checkpoint.
    pub fn checkpoint(&self) -> MonitorState {
        let positions: Vec<Value> = self
            .positions
            .iter()
            .map(|position| {
                json!({
                    "authority": position.reference.authority,
                    "identity": position.reference.identity,
                    "stamp": position.stamp,
                    "order_key": position.order_key.as_ref().map(|key|
                        json!({"authority": key.authority, "key": key.key})),
                    "document": position.document,
                })
            })
            .collect();
        let assertion = self.assertion.map(|assertion| {
            json!({
                "watermark": assertion.watermark,
                "complete": match assertion.completeness {
                    ProgressCompleteness::Complete { through } => json!(through),
                    ProgressCompleteness::Incomplete => Value::Null,
                },
                "scope_closed": assertion.scope == ProgressClosure::Closed,
                "execution_closed": assertion.execution == ProgressClosure::Closed,
            })
        });
        let ledger = self
            .ledger
            .as_ref()
            .and_then(|ledger| write_ledger(ledger).ok());
        let clauses: Vec<Value> = self
            .slots
            .iter()
            .map(|slot| json!({"settled_at": slot.settled_at, "limited": slot.limited}))
            .collect();
        let document = json!({
            "format": STATE_FORMAT,
            "package": self.package_identity(),
            "selections": self.selections(),
            "kind": self.request.kind.name(),
            "binding": format!("{:?}", self.request.binding),
            "lower_boundary": format!("{:?}", self.request.lower_boundary),
            "seen": self.positions.len(),
            "positions": positions,
            "assertion": assertion,
            "ledger": ledger,
            "clauses": clauses,
        });
        MonitorState(serde_json::to_vec(&document).unwrap_or_default())
    }

    /// FR-643: resume from `state` under the same package and request.
    ///
    /// # Errors
    ///
    /// `monitor-state-mismatch` for a state that is not a closed
    /// `filament.monitor-state/1` or whose package, clause selections, kind
    /// or binding differ from `package` and `request`.
    pub fn resume(
        package: &'a CheckedPackage,
        state: &MonitorState,
        request: MonitorOpen<'a>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<Self, Failure> {
        let mut session = Self::open(package, request, limits, cancel)?;
        let document: Value = serde_json::from_slice(&state.0).map_err(|_| state_mismatch())?;
        let object = document.as_object().ok_or_else(state_mismatch)?;
        const MEMBERS: [&str; 11] = [
            "format",
            "package",
            "selections",
            "kind",
            "binding",
            "lower_boundary",
            "seen",
            "positions",
            "assertion",
            "ledger",
            "clauses",
        ];
        if object.len() != MEMBERS.len()
            || MEMBERS.iter().any(|member| !object.contains_key(*member))
            || object["format"] != STATE_FORMAT
        {
            return Err(state_mismatch());
        }
        let same = object["package"] == json!(session.package_identity())
            && object["selections"] == session.selections()
            && object["kind"] == json!(session.request.kind.name())
            && object["binding"] == json!(format!("{:?}", session.request.binding))
            && object["lower_boundary"] == json!(format!("{:?}", session.request.lower_boundary));
        if !same {
            return Err(state_mismatch());
        }
        let positions = object["positions"].as_array().ok_or_else(state_mismatch)?;
        let mut restored = Vec::with_capacity(positions.len());
        for position in positions {
            let text = |member: &str| position[member].as_str().map(str::to_owned);
            let reference = DocumentRef {
                authority: text("authority").ok_or_else(state_mismatch)?,
                identity: text("identity").ok_or_else(state_mismatch)?,
            };
            let order_key = match &position["order_key"] {
                Value::Null => None,
                key => Some(TimestampedOrderKey {
                    authority: key["authority"]
                        .as_str()
                        .ok_or_else(state_mismatch)?
                        .to_owned(),
                    key: key["key"].as_i64().ok_or_else(state_mismatch)?,
                }),
            };
            restored.push(Position {
                reference,
                stamp: position["stamp"].as_u64().ok_or_else(state_mismatch)?,
                order_key,
                document: text("document"),
            });
        }
        if object["seen"].as_u64() != u64::try_from(restored.len()).ok() {
            return Err(state_mismatch());
        }
        let assertion = match &object["assertion"] {
            Value::Null => None,
            value => {
                let closure = |closed: bool| {
                    if closed {
                        ProgressClosure::Closed
                    } else {
                        ProgressClosure::Open
                    }
                };
                Some(ProgressAssertion {
                    watermark: value["watermark"].as_u64().ok_or_else(state_mismatch)?,
                    completeness: match &value["complete"] {
                        Value::Null => ProgressCompleteness::Incomplete,
                        through => ProgressCompleteness::Complete {
                            through: through.as_u64().ok_or_else(state_mismatch)?,
                        },
                    },
                    scope: closure(value["scope_closed"].as_bool().ok_or_else(state_mismatch)?),
                    execution: closure(
                        value["execution_closed"]
                            .as_bool()
                            .ok_or_else(state_mismatch)?,
                    ),
                })
            }
        };
        let ledger = match &object["ledger"] {
            Value::Null => None,
            value => Some(
                read_ledger(&serde_json::to_vec(value).map_err(|_| state_mismatch())?)
                    .map_err(|_: LedgerReadError| state_mismatch())?,
            ),
        };
        let clauses = object["clauses"].as_array().ok_or_else(state_mismatch)?;
        if clauses.len() != session.request.clauses.len() {
            return Err(state_mismatch());
        }
        for position in &restored {
            if let Some(text) = &position.document {
                session
                    .snapshots
                    .insert(position.reference.clone(), text.clone().into_bytes());
            }
        }
        session.positions = restored;
        session.assertion = assertion;
        if ledger.is_some() {
            session.ledger = ledger;
        }
        // Re-establish each clause's status: a settled clause from the
        // prefix it settled at, the others from every position.
        let all = std::mem::take(&mut session.positions);
        let mut slots = session.fresh_slots()?;
        for (index, saved) in clauses.iter().enumerate() {
            let at = saved["settled_at"]
                .as_u64()
                .and_then(|at| usize::try_from(at).ok());
            if saved["limited"].as_bool() == Some(true) {
                let clause = session.request.clauses[index].name.clone();
                slots[index].record = record_of(
                    package,
                    session.request.sources,
                    &clause,
                    limit_report(session.limits.retained_positions),
                )?;
                slots[index].settled = true;
                slots[index].limited = true;
                slots[index].settled_at = at;
            } else if let Some(at) = at {
                if at > all.len() {
                    return Err(state_mismatch());
                }
                session.positions = all[..at].to_vec();
                let (replayed, _) = session.evaluate(session.assertion, cancel)?;
                slots[index] = replayed[index].clone();
            }
        }
        session.positions = all;
        session.slots = slots;
        let (replayed, ledger) = session.evaluate(session.assertion, cancel)?;
        for (index, slot) in replayed.into_iter().enumerate() {
            if !session.slots[index].settled {
                session.slots[index] = slot;
            }
        }
        if ledger.is_some() {
            session.ledger = ledger;
        }
        for slot in &mut session.slots {
            slot.announced = true;
        }
        Ok(session)
    }
}
