// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-643: the stepping monitor. A [`MonitorSession`] takes the observations
//! of one trace one at a time, carries the progress ledger and the stamp
//! order between steps, and reports after each step the status FR-283's batch
//! `monitor` gives over the trace observed so far.
//!
//! Each clause is evaluated by FR-820's carried [`ClauseState`], advanced
//! once per call: it holds an observation's value only while the clause can
//! still read it and each position's identity while the clause may still
//! cite it, so `retained_positions` and `retained_identities` bound the
//! session's memory. A settled clause's state is dropped.

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_exact::{Cancel, Value, ValueType};
use filament_semantic_value::catalog::InternalFault;
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_foundation::node_wire::node_key_to_text;
use fsl_package::number::ExactRational;
use fsl_package::CheckedPackage;
use fsl_replay::spine::{
    assertion_json, assertion_of, authenticate_emission, authenticate_package_trace,
    incomplete_json, incomplete_of, pending_report, read_ledger, write_ledger, AdmittedPosition,
    AdmittedValue, ClauseDisposition, ClauseLimits, ClauseMark, ClauseOpen, ClauseRefusal,
    ClauseRunReport, ClauseState, ClauseTraceKind, ContradictedDimension, LedgerBinding,
    LedgerReadError, PackageAuthentication, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressContradiction, ProgressLedger, ProgressLimitReached,
    ProgressLowerBoundary, ProgressRefusal, ProgressValue, SpineLimits, TemporalVerdict,
    TimestampedCover, TimestampedObservation, TimestampedOrderKey, TimestampedWindow, TraceBinding,
    TraceInput,
};
use fsl_semantics::check::{TemporalClauseNode, TemporalProfile};
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    admit_record_sample, document_reference, AdmissionFailure,
};
use serde_json::{json, Value as Json};

use crate::monitor::{
    record_of, refused, request_failure, DecidingReading, Failure, MonitorClause, MonitorLimits,
    MonitorRecord, MonitorRefusal, MonitorUsage, Runs, SettleTrigger, Settlement,
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
    /// A timed trace (FR-642); its batch run is not built.
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

    fn clause_kind(self) -> ClauseTraceKind {
        match self {
            Self::Progressed => ClauseTraceKind::Progressed,
            Self::Timestamped => ClauseTraceKind::Timestamped,
            Self::Finite | Self::Timed => ClauseTraceKind::Finite,
        }
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
    pub sources: &'a [fsl_foundation::source::Source],
    /// The FR-050 clock configurations a progress-carrying kind needs.
    pub temporal_selections: &'a [fsl_package::TemporalClockSelection],
}

/// One observation's position.
#[derive(Clone, Copy, Debug)]
pub enum MonitorPosition<'a> {
    /// A snapshot document's bytes; it names its own identity.
    Snapshot(&'a [u8]),
    /// A record sample as its canonical JSON object text (FR-641).
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

/// One clause's status after a step.
#[derive(Clone, Debug)]
pub enum ClauseStatus {
    /// The result is final and never changes.
    Settled(MonitorRecord, Settlement),
    /// The result may still change with later observations.
    Pending(MonitorRecord),
}

impl ClauseStatus {
    /// The record of the status.
    pub fn record(&self) -> &MonitorRecord {
        match self {
            Self::Settled(record, _) | Self::Pending(record) => record,
        }
    }

    /// Whether the result is final.
    pub fn is_settled(&self) -> bool {
        matches!(self, Self::Settled(..))
    }

    /// How the clause settled, for a settled status.
    pub fn settlement(&self) -> Option<&Settlement> {
        match self {
            Self::Settled(_, settlement) => Some(settlement),
            Self::Pending(_) => None,
        }
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

#[derive(Clone)]
struct Slot<'a> {
    record: MonitorRecord,
    /// How the clause settled, once it has.
    settlement: Option<Settlement>,
    /// The carried evaluation of an unsettled clause; dropped when it settles.
    state: Option<ClauseState<'a>>,
    /// The state at the call that decided a settled clause, which the
    /// checkpoint writes so a resumed session reports the same `B(s)`.
    decided_state: Option<Vec<u8>>,
    /// The setting whose limit ended the clause, if one did.
    limited: Option<&'static str>,
    announced: bool,
    /// FR-643 "Usage": the work the session charged the clause.
    usage: MonitorUsage,
}

/// The members one call changes in a slot, as they were before it, kept
/// until the call commits (FR-643: a refused or cancelled call leaves the
/// session unchanged).
struct Prior<'a> {
    record: MonitorRecord,
    settlement: Option<Settlement>,
    limited: Option<&'static str>,
    usage: MonitorUsage,
    /// The mark of an unsettled clause's state before the call.
    mark: Option<ClauseMark>,
    /// The state of a clause the call settled, which it would drop.
    dropped: Option<ClauseState<'a>>,
}

/// FR-643's stepping monitor.
pub struct MonitorSession<'a> {
    package: &'a CheckedPackage,
    request: MonitorOpen<'a>,
    limits: MonitorLimits,
    authentication: Option<Arc<PackageAuthentication>>,
    samples: bool,
    ledger: Option<ProgressLedger>,
    /// The number of positions seen.
    seen: u64,
    /// The newest accepted stamp.
    newest: Option<u64>,
    assertion: Option<ProgressAssertion>,
    slots: Vec<Slot<'a>>,
    /// The accepted `step` and `assert_*` calls so far; a refused or cancelled
    /// call does not count (FR-643 "Settlement").
    calls: u64,
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

fn limit_report(setting: &'static str, value: u64) -> ClauseRunReport {
    let verdict = TemporalVerdict::ProgressLimitReached(ProgressLimitReached {
        limit: "node count",
        value,
        setting,
    });
    ClauseRunReport {
        basis: verdict.basis(),
        disposition: ClauseDisposition::Temporal(verdict),
        ..pending_report()
    }
}

/// The `limited` mark of a clause that exhausted a per-call meter limit
/// (FR-643 "Usage"); its report names the limit itself.
const METER_LIMIT: &str = "accounting";

/// The `incomplete` report of a clause whose call exhausted the meter limit
/// `incomplete` names.
fn meter_limit_report(incomplete: filament_exact::Incomplete) -> ClauseRunReport {
    let verdict = TemporalVerdict::Incomplete(incomplete);
    ClauseRunReport {
        basis: verdict.basis(),
        disposition: ClauseDisposition::Temporal(verdict),
        ..pending_report()
    }
}

/// The reading `report` is, as a settlement records it (FR-643 "Settlement"):
/// the finite-window basis when the report has one, else the report's own;
/// `horizon` is the seen horizon; the position is the failing activation of a
/// violation, else the first position of the decision support.
fn reading_of(report: &ClauseRunReport, horizon: u64) -> DecidingReading {
    let basis = report
        .finite_window
        .as_ref()
        .map_or(report.basis, |window| window.basis);
    let position = match &report.disposition {
        ClauseDisposition::Temporal(
            TemporalVerdict::Violated { at } | TemporalVerdict::Undefined { at, .. },
        ) => Some(*at),
        _ => report
            .finite_window
            .as_ref()
            .and_then(|window| window.support.iter().map(|support| support.position).min()),
    };
    DecidingReading {
        basis,
        horizon,
        position,
    }
}

fn reading_json(reading: &DecidingReading) -> Json {
    json!({
        "basis": reading.basis.as_str(),
        "horizon": reading.horizon,
        "position": reading.position,
    })
}

fn reading_from(value: &Json) -> Result<DecidingReading, Failure> {
    let basis = serde_json::from_value(value["basis"].clone()).map_err(|_| state_mismatch())?;
    Ok(DecidingReading {
        basis,
        horizon: value["horizon"].as_u64().ok_or_else(state_mismatch)?,
        position: match &value["position"] {
            Json::Null => None,
            position => Some(position.as_u64().ok_or_else(state_mismatch)?),
        },
    })
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

fn state_fault(what: &'static str) -> Failure {
    StageFailure::Fault(InternalFault::new("monitor", what))
}

/// A carried clause state's refusal as the session's failure.
fn clause_failure(refusal: ClauseRefusal) -> Failure {
    match refusal {
        ClauseRefusal::Cancelled(cause) => StageFailure::Cancelled(cause),
        ClauseRefusal::Contradiction(refusal) => {
            StageFailure::Refused(MonitorRefusal::Contradiction(refusal))
        }
        ClauseRefusal::StateMismatch => state_mismatch(),
        ClauseRefusal::Fault(fault) => StageFailure::Fault(fault),
    }
}

/// Whether two reports give the same disposition: temporal verdicts by
/// value, any other disposition by category.
fn same_disposition(before: &ClauseRunReport, after: &ClauseRunReport) -> bool {
    match (&before.disposition, &after.disposition) {
        (ClauseDisposition::Temporal(a), ClauseDisposition::Temporal(b)) => a == b,
        (ClauseDisposition::Temporal(_), _) | (_, ClauseDisposition::Temporal(_)) => false,
        (a, b) => a.category() == b.category(),
    }
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
            let Some(node) = Self::node_in(package, &clause.name) else {
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
        // A `Finite` trace needs no binding or emission (FR-820, FR-283-AC-17):
        // only a progress-carrying kind authenticates, as the batch run does.
        let authentication = if request.kind.carries_progress() {
            Some(Arc::new(
                authenticate_emission(
                    package,
                    request.temporal_selections,
                    request.packages,
                    &SpineLimits::default(),
                    cancel,
                )
                .map_err(|failure| Runs::emission_failure(&request.clauses, failure, &first))?,
            ))
        } else {
            None
        };
        let mut session = Self {
            package,
            samples: samples.unwrap_or(false),
            ledger: request.ledger.clone(),
            request,
            limits: *limits,
            authentication,
            seen: 0,
            newest: None,
            assertion: None,
            slots: Vec::new(),
            calls: 0,
        };
        session.check_kind()?;
        session.slots = session.fresh_slots()?;
        Ok(session)
    }

    fn node_in(package: &'a CheckedPackage, name: &str) -> Option<&'a TemporalClauseNode> {
        package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
    }

    fn node(&self, name: &str) -> Result<&'a TemporalClauseNode, Failure> {
        Self::node_in(self.package, name).ok_or_else(|| state_fault("session-clause-is-declared"))
    }

    /// FR-645: refuse a clause whose profile does not read the kind, before
    /// any observation.
    fn check_kind(&self) -> Result<(), Failure> {
        let probe = self.probe_trace();
        for clause in &self.request.clauses {
            let checked = authenticate_package_trace(
                self.package,
                &clause.name,
                &probe,
                self.authentication
                    .as_deref()
                    .map(PackageAuthentication::temporal),
            );
            if let Err(disposition) = checked {
                return Err(request_failure(&clause.name, &disposition)
                    .unwrap_or_else(|| state_fault("trace-authentication-refuses-at-admit")));
            }
        }
        Ok(())
    }

    /// An empty trace of the session's kind.
    fn probe_trace(&self) -> TraceInput {
        let progress = ProgressAssertion {
            watermark: 0,
            completeness: ProgressCompleteness::Incomplete,
            scope: ProgressClosure::Open,
            execution: ProgressClosure::Open,
        };
        match (self.request.kind, self.request.binding.clone()) {
            (MonitorTraceKind::Progressed, Some(binding)) => TraceInput::Progressed {
                trace: Vec::new(),
                progress,
                lower_boundary: self.request.lower_boundary,
                binding,
            },
            (MonitorTraceKind::Timestamped, binding) => TraceInput::Timestamped {
                observations: Vec::<TimestampedObservation>::new(),
                progress,
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
            _ => TraceInput::Finite(Vec::new()),
        }
    }

    /// The opening request of `clause`'s carried state (FR-820).
    fn clause_open(&self, clause: &MonitorClause) -> ClauseOpen<'a> {
        ClauseOpen {
            kind: self.request.kind.clause_kind(),
            lower_boundary: self.request.lower_boundary,
            binding: self.request.binding.clone(),
            cover: self.request.cover,
            ledger: self.request.ledger.clone(),
            sources: self.request.sources,
            authentication: self.authentication.clone(),
            packages: self.request.packages,
            over: clause.over.clone(),
            limits: ClauseLimits {
                model: self.limits.normalization,
                observation: self.limits.observation,
                accounting: self.limits.accounting,
                max_window_values: self.limits.max_window_values,
            },
        }
    }

    fn fresh_slots(&self) -> Result<Vec<Slot<'a>>, Failure> {
        self.request
            .clauses
            .iter()
            .map(|clause| {
                let node = self.node(&clause.name)?;
                let state = ClauseState::open(self.package, node, &self.clause_open(clause))
                    .map_err(clause_failure)?;
                Ok(Slot {
                    record: record_of(
                        self.package,
                        self.request.sources,
                        &clause.name,
                        pending_report(),
                    )?,
                    settlement: None,
                    state: Some(state),
                    decided_state: None,
                    limited: None,
                    announced: false,
                    usage: MonitorUsage::default(),
                })
            })
            .collect()
    }

    /// The position an observation names, checked against the kind and the
    /// retained watermark.
    fn admit_observation<'o>(
        &self,
        observation_in: &MonitorObservation<'o>,
    ) -> Result<AdmittedPosition<'o>, Failure> {
        let kind = self.request.kind;
        let stamp = match (&observation_in.stamp, kind) {
            (Some(MonitorStamp::Tick(tick)), MonitorTraceKind::Timestamped) => *tick,
            (None, MonitorTraceKind::Timestamped) => {
                return Err(observation("stamp", "missing-member"))
            }
            (Some(MonitorStamp::Exact(_)), MonitorTraceKind::Timestamped) => {
                return Err(observation("stamp", "wrong-value-kind"))
            }
            (None, _) => self.seen,
            (Some(_), _) => return Err(observation("stamp", "unknown-member")),
        };
        if observation_in.order_key.is_some() && kind != MonitorTraceKind::Timestamped {
            return Err(observation("order_key", "unknown-member"));
        }
        let clock = || {
            self.request
                .binding
                .as_ref()
                .map(|binding| binding.clock_name.clone())
                .unwrap_or_default()
        };
        for assertion in self.retained() {
            if assertion.scope == ProgressClosure::Closed {
                return Err(StageFailure::Refused(MonitorRefusal::Contradiction(
                    Box::new(ProgressRefusal::from(ProgressContradiction {
                        dimension: ContradictedDimension::Closure,
                        clock: clock(),
                        retained: ProgressValue::DecisionScope(ProgressClosure::Closed),
                        asserted: ProgressValue::DecisionScope(ProgressClosure::Open),
                    })),
                )));
            }
            if stamp <= assertion.watermark {
                return Err(StageFailure::Refused(MonitorRefusal::Contradiction(
                    Box::new(ProgressRefusal::from(ProgressContradiction {
                        dimension: ContradictedDimension::Watermark,
                        clock: clock(),
                        retained: ProgressValue::Watermark(assertion.watermark),
                        asserted: ProgressValue::Watermark(stamp),
                    })),
                )));
            }
        }
        let (reference, value) = match (observation_in.position, self.samples) {
            (MonitorPosition::Sample(text), true) => {
                let reference = fsl_replay::spine::sample_position(text);
                let record = self
                    .request
                    .clauses
                    .first()
                    .and_then(|clause| Self::node_in(self.package, &clause.name))
                    .and_then(|node| match &node.clause().over().1 {
                        ValueType::Composite(record) => Some(record.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| state_fault("sample-clause-has-a-record-subject"))?;
                let sample: Value = admit_record_sample(
                    self.package.graph().scope().types(),
                    &record,
                    text.as_bytes(),
                )
                .map_err(|mut record| {
                    record.fields.insert("position", self.seen.to_string());
                    StageFailure::Refused(MonitorRefusal::Admission {
                        position: usize::try_from(self.seen).unwrap_or(usize::MAX),
                        failure: Box::new(AdmissionFailure::Refused(Box::new(record))),
                    })
                })?;
                (reference, AdmittedValue::Sample(sample))
            }
            (MonitorPosition::Snapshot(bytes), false) => {
                let reference = document_reference(bytes)
                    .ok_or_else(|| observation("position", "missing-member"))?;
                if std::str::from_utf8(bytes).is_err() {
                    return Err(observation("position", "invalid-value"));
                }
                (reference, AdmittedValue::Snapshot(bytes))
            }
            _ => return Err(observation("position", "wrong-value-kind")),
        };
        Ok(AdmittedPosition {
            reference,
            stamp,
            order_key: observation_in.order_key.clone(),
            value,
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
        let stamp = position.stamp;
        let seen = self.seen.saturating_add(1);
        let (priors, ledger) = self.advance(seen, self.assertion, cancel, |state, cancel| {
            state.advance(position.clone(), cancel)
        })?;
        self.seen = seen;
        self.newest = Some(self.newest.map_or(stamp, |newest| newest.max(stamp)));
        Ok(self.commit(priors, ledger))
    }

    /// FR-643: take a `Timed` trace's completeness stamp. Only the `Timed` kind
    /// takes one, and a `Timed` session cannot be opened until FR-642's timed
    /// batch run exists, so every session refuses it `invalid-request` naming
    /// `through`.
    ///
    /// # Errors
    ///
    /// The refusal above, or a cancelled call.
    pub fn assert_through(
        &mut self,
        _through: ExactRational,
        cancel: &Cancel,
    ) -> Result<MonitorStep, Failure> {
        cancelled(cancel)?;
        Err(observation("through", "invalid-request"))
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
        let (priors, ledger) =
            self.advance(self.seen, Some(progress), cancel, |state, cancel| {
                state.assert_progress(&progress, cancel)
            })?;
        self.assertion = Some(progress);
        Ok(self.commit(priors, ledger))
    }

    /// Advance every unsettled clause's state by `call`, settling each clause
    /// its profile decides or a limit ends. Each slot is advanced in place and
    /// its prior members kept until the call commits: a refused or cancelled
    /// call puts every slot back, so the session is unchanged, and no slot is
    /// copied whole.
    fn advance<F>(
        &mut self,
        seen: u64,
        assertion: Option<ProgressAssertion>,
        cancel: &Cancel,
        call: F,
    ) -> Result<(Vec<Prior<'a>>, Option<ProgressLedger>), Failure>
    where
        F: Fn(&mut ClauseState<'a>, &Cancel) -> Result<fsl_replay::spine::Advance, ClauseRefusal>,
    {
        let mut priors = Vec::with_capacity(self.slots.len());
        let mut ledger = self.ledger.clone();
        match self.advance_slots((seen, assertion), cancel, &call, &mut priors, &mut ledger) {
            Ok(()) => Ok((priors, ledger)),
            Err(failure) => {
                self.rollback(priors);
                Err(failure)
            }
        }
    }

    /// Put back the members each slot had before the call `priors` records.
    fn rollback(&mut self, priors: Vec<Prior<'a>>) {
        for (slot, prior) in self.slots.iter_mut().zip(priors) {
            slot.record = prior.record;
            slot.settlement = prior.settlement;
            slot.limited = prior.limited;
            slot.usage = prior.usage;
            if let Some(state) = prior.dropped {
                slot.state = Some(state);
            }
            if let Some(mark) = prior.mark {
                slot.decided_state = None;
                if let Some(state) = slot.state.as_mut() {
                    state.restore(mark);
                }
            }
        }
    }

    /// [`Self::advance`]'s pass over the slots, recording each slot's prior
    /// members in `priors` before it changes the slot.
    fn advance_slots<F>(
        &mut self,
        (seen, assertion): (u64, Option<ProgressAssertion>),
        cancel: &Cancel,
        call: &F,
        priors: &mut Vec<Prior<'a>>,
        ledger: &mut Option<ProgressLedger>,
    ) -> Result<(), Failure>
    where
        F: Fn(&mut ClauseState<'a>, &Cancel) -> Result<fsl_replay::spine::Advance, ClauseRefusal>,
    {
        let step = self.calls + 1;
        let last = seen.saturating_sub(1);
        for index in 0..self.slots.len() {
            cancelled(cancel)?;
            let name = self.request.clauses[index].name.as_str();
            let profile = self.node(name)?.clause().profile();
            let slot = &mut self.slots[index];
            priors.push(Prior {
                record: slot.record.clone(),
                settlement: slot.settlement.clone(),
                limited: slot.limited,
                usage: slot.usage.clone(),
                mark: slot.state.as_ref().map(ClauseState::mark),
                dropped: None,
            });
            let Some(state) = slot.state.as_mut() else {
                // FR-643 "Usage": a settled clause charges nothing more.
                slot.usage = slot.usage.idle();
                continue;
            };
            state.carry_ledger(ledger.clone());
            let advanced = call(state, cancel).map_err(clause_failure)?;
            slot.usage = slot.usage.charged(advanced.report.usage.clone());
            if seen == 0 {
                continue;
            }
            if let Some(updated) = &advanced.report.ledger {
                *ledger = Some(updated.clone());
            }
            slot.record = record_of(
                self.package,
                self.request.sources,
                name,
                Arc::clone(&advanced.report),
            )?;
            let horizon = match (profile, self.request.kind, assertion) {
                (TemporalProfile::TimestampedEventFiniteWindow, _, Some(a)) => a.watermark,
                (
                    TemporalProfile::FixedSampleFalseExtension,
                    MonitorTraceKind::Progressed,
                    Some(a),
                ) => match a.completeness {
                    ProgressCompleteness::Complete { through } => a.watermark.min(through),
                    ProgressCompleteness::Incomplete => last,
                },
                _ => last,
            };
            let settled = if let Some(decided) = &advanced.decided {
                slot.decided_state = Some(state.to_bytes().map_err(clause_failure)?);
                Some((SettleTrigger::Decided, Some(reading_of(decided, horizon))))
            } else if let ClauseDisposition::Temporal(TemporalVerdict::Incomplete(_)) =
                &advanced.report.disposition
            {
                // FR-643 "Usage": meter limits are charged per call, so a
                // clause whose call exhausted one settles `incomplete` on
                // that limit.
                slot.limited = Some(METER_LIMIT);
                Some((SettleTrigger::Limit, None))
            } else {
                // FR-643 "Retained state": a clause whose state would hold
                // more values or identities than its limit settles
                // `incomplete`, and its state is dropped.
                let over = if state.retained() > self.limits.retained_positions {
                    Some(("monitor.retained_positions", self.limits.retained_positions))
                } else if state.cited() > self.limits.retained_identities {
                    Some((
                        "monitor.retained_identities",
                        self.limits.retained_identities,
                    ))
                } else {
                    None
                };
                match over {
                    Some((setting, value)) => {
                        slot.record = record_of(
                            self.package,
                            self.request.sources,
                            name,
                            limit_report(setting, value),
                        )?;
                        slot.limited = Some(setting);
                        Some((SettleTrigger::Limit, None))
                    }
                    None => None,
                }
            };
            if let Some((trigger, reading)) = settled {
                slot.settlement = Some(Settlement {
                    step,
                    trigger,
                    reading,
                });
                if let Some(prior) = priors.last_mut() {
                    prior.dropped = slot.state.take();
                }
            }
        }
        Ok(())
    }

    /// FR-502: what `next` retreats from in the assertion the session retains.
    fn regression(&self, next: ProgressAssertion) -> Option<ProgressContradiction> {
        self.retained()
            .into_iter()
            .find_map(|held| self.retreat(held, next))
    }

    /// The assertions the session retains for Order (FR-643): its own latest
    /// one and the ledger's entry for each selected clause's binding, which
    /// includes the opening ledger.
    fn retained(&self) -> Vec<ProgressAssertion> {
        let mut held: Vec<ProgressAssertion> = self.assertion.into_iter().collect();
        let (Some(ledger), Some(source)) = (&self.ledger, self.request.sources.first()) else {
            return held;
        };
        for clause in &self.request.clauses {
            let Some(node) = Self::node_in(self.package, &clause.name) else {
                continue;
            };
            let binding = LedgerBinding {
                package: source.identity().identity.clone(),
                clause: node.identity(),
                occurrence: node.claim().ordinal(),
                clock: node.clause().clock().to_owned(),
                profile: node.clause().profile().identity().to_owned(),
            };
            if let Some(entry) = ledger.entry(&binding) {
                held.push(ProgressAssertion {
                    watermark: entry.watermark(),
                    completeness: entry.completeness(),
                    scope: entry.scope(),
                    execution: entry.execution(),
                });
            }
        }
        held
    }

    /// What `next` retreats from in `held`.
    fn retreat(
        &self,
        held: ProgressAssertion,
        next: ProgressAssertion,
    ) -> Option<ProgressContradiction> {
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

    fn commit(&mut self, priors: Vec<Prior<'a>>, ledger: Option<ProgressLedger>) -> MonitorStep {
        let mut changed = Vec::new();
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let before = priors.get(index);
            let same = before.is_some_and(|before| {
                slot.announced
                    && before.settlement.is_some() == slot.settlement.is_some()
                    && before.record.verdict == slot.record.verdict
                    && same_disposition(before.record.report(), slot.record.report())
            });
            if !same {
                changed.push(slot.record.clause.clone());
            }
            slot.announced = true;
        }
        if ledger.is_some() {
            self.ledger = ledger;
        }
        self.calls += 1;
        MonitorStep {
            statuses: self
                .slots
                .iter()
                .map(|slot| (slot.record.clause.clone(), Self::status_of(slot)))
                .collect(),
            changed,
        }
    }

    fn status_of(slot: &Slot<'_>) -> ClauseStatus {
        match &slot.settlement {
            Some(settlement) => ClauseStatus::Settled(
                slot.record.clone().with_usage(slot.usage.clone()),
                settlement.clone(),
            ),
            None => ClauseStatus::Pending(slot.record.clone().with_usage(slot.usage.clone())),
        }
    }

    /// FR-643: end the session and return FR-283's outcome over the trace.
    ///
    /// It adds no closure: a finite trace has one reading per profile and a
    /// progress-carrying trace is closed as its latest assertion says.
    ///
    /// # Errors
    ///
    /// A cancelled call.
    pub fn finish(self, cancel: &Cancel) -> Result<crate::monitor::MonitorOutcome, Failure> {
        cancelled(cancel)?;
        let seen = self.calls;
        let settlements = self
            .slots
            .iter()
            .map(|slot| {
                slot.settlement.clone().unwrap_or(Settlement {
                    step: seen,
                    trigger: SettleTrigger::Finish,
                    reading: None,
                })
            })
            .collect();
        let ledger = self
            .request
            .kind
            .carries_progress()
            .then(|| self.ledger.clone())
            .flatten();
        Ok(crate::monitor::MonitorOutcome {
            records: self.slots.into_iter().map(|slot| slot.record).collect(),
            settlements,
            ledger,
        })
    }

    /// The package as its checkpoint form: each source's identity and each
    /// temporal clause's checked node identity in its closed text spelling.
    fn package_identity(&self) -> Result<Json, Failure> {
        let sources: Vec<Json> = self
            .request
            .sources
            .iter()
            .map(|source| {
                let identity = source.identity();
                json!({"authority": identity.authority, "identity": identity.identity})
            })
            .collect();
        let clauses = self
            .package
            .graph()
            .temporal_clauses()
            .iter()
            .map(|node| {
                node_key_to_text(&node.identity())
                    .map(Json::String)
                    .map_err(|_| state_fault("clause-identity-has-text"))
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(json!({"sources": sources, "clauses": clauses}))
    }

    /// The trace binding as its checkpoint form.
    fn binding_identity(&self) -> Result<Json, Failure> {
        let Some(binding) = &self.request.binding else {
            return Ok(Json::Null);
        };
        let parameters = serde_json::to_value(&binding.parameters)
            .map_err(|_| state_fault("binding-parameters-serialize"))?;
        Ok(json!({
            "profile": binding.profile,
            "clock_name": binding.clock_name,
            "parameters": parameters,
        }))
    }

    fn boundary_identity(&self) -> &'static str {
        match self.request.lower_boundary {
            ProgressLowerBoundary::AuthoritativeOrigin => "authoritative_origin",
            ProgressLowerBoundary::HistoryCutoff => "history_cutoff",
        }
    }

    fn selections(&self) -> Json {
        Json::Array(
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

    /// FR-643: the session's checkpoint: the request's identity, the counted
    /// calls, the progress and ledger, each settled clause's settlement with
    /// the state that decided it, and each unsettled clause's FR-820 state
    /// bytes, which hold the values it retains.
    ///
    /// # Errors
    ///
    /// An internal fault when the ledger, the binding or a state cannot be
    /// written; no state is returned that lacks any of them.
    pub fn checkpoint(&self) -> Result<MonitorState, Failure> {
        let ledger = match &self.ledger {
            Some(ledger) => {
                write_ledger(ledger).map_err(|_| state_fault("ledger-has-its-wire-form"))?
            }
            None => Json::Null,
        };
        let text = |bytes: Vec<u8>| {
            String::from_utf8(bytes).map_err(|_| state_fault("clause-state-is-text"))
        };
        let clauses = self
            .slots
            .iter()
            .map(|slot| {
                let state = match &slot.state {
                    Some(state) => Some(text(state.to_bytes().map_err(clause_failure)?)?),
                    None => None,
                };
                let decided = slot.decided_state.clone().map(text).transpose()?;
                Ok(json!({
                    "state": state,
                    "decided_state": decided,
                    "settled_step": slot.settlement.as_ref().map(|s| s.step),
                    "trigger": slot.settlement.as_ref().map(|s| match s.trigger {
                        SettleTrigger::Decided => "decided",
                        SettleTrigger::Limit => "limit",
                        SettleTrigger::Finish => "finish",
                    }),
                    "reading": slot.settlement.as_ref()
                        .and_then(|s| s.reading.as_ref()).map(reading_json),
                    "limited": slot.limited,
                    "incomplete": match &slot.record.report().disposition {
                        ClauseDisposition::Temporal(TemporalVerdict::Incomplete(incomplete))
                            if slot.limited == Some(METER_LIMIT) => incomplete_json(incomplete),
                        _ => Json::Null,
                    },
                    "announced": slot.announced,
                }))
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        let document = json!({
            "format": STATE_FORMAT,
            "package": self.package_identity()?,
            "selections": self.selections(),
            "kind": self.request.kind.name(),
            "binding": self.binding_identity()?,
            "lower_boundary": self.boundary_identity(),
            "seen": self.seen,
            "newest": self.newest,
            "calls": self.calls,
            "assertion": self.assertion.map(assertion_json),
            "ledger": ledger,
            "clauses": clauses,
        });
        serde_json::to_vec(&document)
            .map(MonitorState)
            .map_err(|_| state_fault("state-serializes"))
    }

    /// FR-643: resume from `state` under the same package and request.
    ///
    /// # Errors
    ///
    /// `monitor-state-mismatch` for a state that is not a closed
    /// `filament.monitor-state/1`, or whose package, clause selections, kind
    /// or binding differ from `package` and `request`.
    pub fn resume(
        package: &'a CheckedPackage,
        state: &MonitorState,
        request: MonitorOpen<'a>,
        limits: &MonitorLimits,
        cancel: &Cancel,
    ) -> Result<Self, Failure> {
        let mut session = Self::open(package, request, limits, cancel)?;
        let document: Json = serde_json::from_slice(&state.0).map_err(|_| state_mismatch())?;
        let object = document.as_object().ok_or_else(state_mismatch)?;
        const MEMBERS: [&str; 12] = [
            "format",
            "package",
            "selections",
            "kind",
            "binding",
            "lower_boundary",
            "seen",
            "newest",
            "calls",
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
        let same = object["package"] == session.package_identity()?
            && object["selections"] == session.selections()
            && object["kind"] == json!(session.request.kind.name())
            && object["binding"] == session.binding_identity()?
            && object["lower_boundary"] == json!(session.boundary_identity());
        if !same {
            return Err(state_mismatch());
        }
        session.seen = object["seen"].as_u64().ok_or_else(state_mismatch)?;
        session.newest = match &object["newest"] {
            Json::Null => None,
            newest => Some(newest.as_u64().ok_or_else(state_mismatch)?),
        };
        session.calls = object["calls"].as_u64().ok_or_else(state_mismatch)?;
        session.assertion = match &object["assertion"] {
            Json::Null => None,
            value => Some(assertion_of(value).ok_or_else(state_mismatch)?),
        };
        if let ledger @ Json::Object(_) = &object["ledger"] {
            session.ledger = Some(
                read_ledger(&serde_json::to_vec(ledger).map_err(|_| state_mismatch())?)
                    .map_err(|_: LedgerReadError| state_mismatch())?,
            );
        }
        let clauses = object["clauses"].as_array().ok_or_else(state_mismatch)?;
        if clauses.len() != session.request.clauses.len() {
            return Err(state_mismatch());
        }
        for (index, saved) in clauses.iter().enumerate() {
            let clause = session.request.clauses[index].clone();
            let node = session.node(&clause.name)?;
            let open = session.clause_open(&clause);
            let restore = |bytes: &str| {
                ClauseState::from_bytes(package, node, &open, bytes.as_bytes(), cancel)
                    .map_err(clause_failure)
            };
            let step = saved["settled_step"].as_u64();
            let slot = &mut session.slots[index];
            slot.announced = saved["announced"].as_bool().ok_or_else(state_mismatch)?;
            match saved["trigger"].as_str() {
                None => {
                    let bytes = saved["state"].as_str().ok_or_else(state_mismatch)?;
                    let state = restore(bytes)?;
                    slot.record = record_of(
                        package,
                        session.request.sources,
                        &clause.name,
                        state.report(),
                    )?;
                    slot.state = Some(state);
                }
                Some("limit") if saved["limited"].as_str() == Some(METER_LIMIT) => {
                    let incomplete =
                        incomplete_of(&saved["incomplete"]).ok_or_else(state_mismatch)?;
                    slot.record = record_of(
                        package,
                        session.request.sources,
                        &clause.name,
                        meter_limit_report(incomplete),
                    )?;
                    slot.limited = Some(METER_LIMIT);
                    slot.state = None;
                    slot.settlement = Some(Settlement {
                        step: step.ok_or_else(state_mismatch)?,
                        trigger: SettleTrigger::Limit,
                        reading: None,
                    });
                }
                Some("limit") => {
                    let (setting, value) = match saved["limited"].as_str() {
                        Some("monitor.retained_positions") => (
                            "monitor.retained_positions",
                            session.limits.retained_positions,
                        ),
                        Some("monitor.retained_identities") => (
                            "monitor.retained_identities",
                            session.limits.retained_identities,
                        ),
                        _ => return Err(state_mismatch()),
                    };
                    slot.record = record_of(
                        package,
                        session.request.sources,
                        &clause.name,
                        limit_report(setting, value),
                    )?;
                    slot.limited = Some(setting);
                    slot.state = None;
                    slot.settlement = Some(Settlement {
                        step: step.ok_or_else(state_mismatch)?,
                        trigger: SettleTrigger::Limit,
                        reading: None,
                    });
                }
                Some("decided") => {
                    let bytes = saved["decided_state"].as_str().ok_or_else(state_mismatch)?;
                    let decided = restore(bytes)?;
                    slot.record = record_of(
                        package,
                        session.request.sources,
                        &clause.name,
                        decided.report(),
                    )?;
                    slot.decided_state = Some(bytes.as_bytes().to_vec());
                    slot.state = None;
                    slot.settlement = Some(Settlement {
                        step: step.ok_or_else(state_mismatch)?,
                        trigger: SettleTrigger::Decided,
                        reading: Some(reading_from(&saved["reading"])?),
                    });
                }
                Some(_) => return Err(state_mismatch()),
            }
        }
        Ok(session)
    }
}
