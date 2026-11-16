// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283: the `monitor` operation. It checks a supplied trace offline
//! against temporal clauses with the layer-5 trace evaluator replay uses
//! (ADR-029 OP-3, ADR-014 A-4), and reports one [`MonitorRecord`] per
//! selected clause.

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_exact::{Cancel, ScalarLimits, Value, ValueType};
use filament_semantic_value::catalog::{CatalogCode, CatalogCoded, Category, InternalFault};
use fsl_foundation::diagnostic::{Code, LimitExceeded, StageFailure, Staged};
use fsl_foundation::source::Source;
use fsl_foundation::{Setting, SourceIdentity};
use fsl_package::{CheckedPackage, TemporalClockSelection};
use fsl_replay::spine::{
    authenticate_emission, authenticate_package_trace, name_clause_atom, run_package_temporal,
    run_package_trace_subject, AuthenticationFailure, ClauseDisposition, ClauseRunReport,
    ClauseRunStage, OverObject, PackageAuthentication, PackageTemporalRequest,
    PackageTraceSubjectRequest, PackagesStage, ProgressRefusal, RunRefusal, SpineLimits,
    TemporalIndeterminateReason, TemporalTraceRefusal, TemporalUndefinedCause, TemporalVerdict,
    TraceInput,
};
use fsl_replay::{
    ClauseRunNotAnItem, Operation, OutcomeDiagnostic, OutcomeDocument, OutcomeItem, OutcomeStage,
};
use fsl_semantics::check::TemporalClauseNode;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    admit_trace_snapshot, AdmissionFailure, AdmissionRecord, DocumentProvision, DocumentRef,
    ObservationLimits,
};
use quire_canonical::{Node, NodeRef};

/// The format member of a trace document.
pub const TRACE_FORMAT: &str = "filament.monitor-trace/1";

/// One selected temporal clause: its declared name and the object its `over`
/// parameter binds, resolved in the observation at position 0.
#[derive(Clone, Debug)]
pub struct MonitorClause {
    /// The clause's declared name.
    pub name: String,
    /// The object the clause's `over` parameter names for a clause over a
    /// model subject; absent for a clause over a declared record trace
    /// subject, which FR-330's trace-subject route binds to each sample.
    pub over: Option<OverObject>,
}

/// The trace a `monitor` request supplies (FR-283 Inputs).
#[derive(Clone, Debug)]
pub enum MonitorTrace<'a> {
    /// The bytes of a `filament.monitor-trace/1` trace document.
    Document(&'a [u8]),
    /// A library caller's FR-330 trace, read as FR-330's `Temporal`
    /// selection reads it.
    Typed(&'a TraceInput),
    /// A declared record type and its non-empty finite sequence of record
    /// samples, FR-330's trace-subject route input.
    Samples {
        /// The declared record type of the samples.
        subject: ValueType,
        /// The samples, in trace-position order.
        samples: &'a [Value],
    },
}

/// A `monitor` request (FR-283 Inputs).
pub struct MonitorRequest<'a> {
    /// The selected clauses, in request order.
    pub clauses: Vec<MonitorClause>,
    /// The trace to check.
    pub trace: MonitorTrace<'a>,
    /// The snapshot documents a `Document` or `Typed` trace's positions name.
    pub snapshots: &'a DocumentProvision,
    /// The domain package bytes admission reads.
    pub packages: &'a PackageInput,
    /// The unit's and each library's source the package was checked from:
    /// a refused condition's locus and a named atom are rendered from them.
    pub sources: &'a [Source],
    /// The FR-050 clock configurations the package is emitted with to
    /// authenticate a `Progressed` or `Timestamped` trace's binding; empty
    /// for a request with no such trace.
    pub temporal_selections: &'a [TemporalClockSelection],
}

/// The limits a `monitor` request takes (FR-283 Inputs).
#[derive(Clone, Copy, Debug)]
pub struct MonitorLimits {
    /// The observation limits; `document_bytes`, `objects_per_document` and
    /// `values_per_document` also bound a trace document.
    pub observation: ObservationLimits,
    /// The model-normalization limits admission takes.
    pub normalization: ModelNormalizationLimits,
    /// The evaluator's accounting limits.
    pub accounting: ScalarLimits,
    /// FR-194's window limit, `monitor.max_window_values`, of every window
    /// aggregate a `Samples` run reads.
    pub max_window_values: u64,
    /// FR-643's `monitor.retained_positions`: the most positions a stepping
    /// session keeps at once for one clause.
    pub retained_positions: u64,
    /// FR-643's `monitor.retained_identities`: the most position identities one
    /// unsettled clause keeps for the counterexample it may still report.
    pub retained_identities: u64,
}

/// The published default of `monitor.retained_identities`.
pub const DEFAULT_RETAINED_IDENTITIES: u64 = 1_048_576;

/// The published default of `monitor.max_window_values` (FR-194).
pub const DEFAULT_MAX_WINDOW_VALUES: u64 = fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES;

/// The published default of `monitor.retained_positions`.
pub const DEFAULT_RETAINED_POSITIONS: u64 = 1_048_576;

/// What a clause run reports (FR-283 Outputs table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonitorVerdict {
    /// `Completed(false)`: the clause is violated.
    Violated,
    /// An undefined evaluation, settled as a violation.
    Undefined,
    /// A window aggregate short of data: category undefined.
    InsufficientData,
    /// A finite trace that neither shows a bad prefix nor can show more.
    Pending,
    /// A finite-window clause whose closed window leaves it unresolved.
    Indeterminate,
    /// `Completed(true)`: evidence for this trace only, never `proved`.
    Tested,
    /// A clause with a fairness set, over a trace that carries no
    /// enabledness.
    MissingFairnessPremise,
    /// The accounting budget ran out.
    Incomplete,
    /// A window aggregate outgrew `max_window_values`.
    WindowLimitReached,
    /// A `holds` condition was refused.
    ConditionRefused,
    /// The evaluator refused the trace.
    TraceRefused,
}

/// One selected clause's record, in request order.
#[derive(Clone, Debug)]
pub struct MonitorRecord {
    /// The clause's declared name.
    pub clause: String,
    /// The verdict.
    pub verdict: MonitorVerdict,
    /// The O-16 category.
    pub category: Category,
    /// The source text of the atom a `InsufficientData` or `Indeterminate`
    /// record names.
    atom: Option<String>,
    report: Arc<ClauseRunReport>,
    /// FR-643 "Usage": a stepping session's own work for the clause; `None`
    /// for a batch record, whose usage is its report's.
    usage: Option<MonitorUsage>,
}

/// FR-643 "Usage": the work a stepping session charged one clause, each
/// member one FR-109 `ClauseRunUsage` (admission work and evaluation meter
/// charges).
#[derive(Clone, Debug, Default)]
pub struct MonitorUsage {
    /// The work the session charged the clause since it opened: the sum of
    /// every `step`.
    pub cumulative: fsl_replay::spine::ClauseRunUsage,
    /// The work charged in the latest counted call; zero once the clause has
    /// settled.
    pub step: fsl_replay::spine::ClauseRunUsage,
}

impl MonitorUsage {
    /// The usage after a counted call that charged `step`.
    pub(crate) fn charged(&self, step: fsl_replay::spine::ClauseRunUsage) -> Self {
        let total = &self.cumulative;
        let mut cumulative = fsl_replay::spine::ClauseRunUsage {
            admission_consumed: fsl_semantics::model::observation::AdmissionUsage {
                document_bytes: total
                    .admission_consumed
                    .document_bytes
                    .saturating_add(step.admission_consumed.document_bytes),
                objects: total
                    .admission_consumed
                    .objects
                    .saturating_add(step.admission_consumed.objects),
                values: total
                    .admission_consumed
                    .values
                    .saturating_add(step.admission_consumed.values),
            },
            evaluation_admissions: total
                .evaluation_admissions
                .saturating_add(step.evaluation_admissions),
            temporal_visits: total.temporal_visits.saturating_add(step.temporal_visits),
            evaluation_consumed: total.evaluation_consumed.clone(),
        };
        for (kind, consumed) in &step.evaluation_consumed {
            match cumulative
                .evaluation_consumed
                .iter_mut()
                .find(|(held, _)| held == kind)
            {
                Some((_, sum)) => *sum = sum.saturating_add(*consumed),
                None => cumulative.evaluation_consumed.push((*kind, *consumed)),
            }
        }
        Self { cumulative, step }
    }

    /// The usage after a counted call that charged nothing.
    pub(crate) fn idle(&self) -> Self {
        Self {
            cumulative: self.cumulative.clone(),
            step: fsl_replay::spine::ClauseRunUsage::default(),
        }
    }
}

impl MonitorRecord {
    /// FR-643 "Usage": the session's `{cumulative, step}` usage of a
    /// stepping session's record; `None` for a batch record.
    pub fn usage(&self) -> Option<&MonitorUsage> {
        self.usage.as_ref()
    }

    /// This record with the session usage `usage`.
    pub(crate) fn with_usage(mut self, usage: MonitorUsage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// The clause run behind this record: its `position`, `counterexample`,
    /// `decisive`, cause and usage.
    pub fn report(&self) -> &ClauseRunReport {
        &self.report
    }

    /// The atom an `InsufficientData` or `Indeterminate` record names, by the
    /// source text of its condition.
    pub fn atom(&self) -> Option<&str> {
        self.atom.as_deref()
    }

    /// The counterexample of a `Violated` or `Undefined` record
    /// (FSpec FR-364), with `trace_position` equal to the record's position.
    pub fn counterexample(&self) -> Option<&fsl_replay::ObservedTemporalCounterexample> {
        self.report.counterexample.as_ref()
    }

    /// The FR-286 `monitor` item this record renders to, through
    /// [`OutcomeItem::from_clause_run`].
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for a run no item can carry.
    pub fn item(&self, index: usize) -> Result<OutcomeItem, ClauseRunNotAnItem> {
        OutcomeItem::from_clause_run(
            fsl_foundation::RequestIndex::new(index),
            &self.clause,
            &self.report,
        )
    }
}

/// What settled a clause (FR-643 "Settlement").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettleTrigger {
    /// The profile's evaluator decided it.
    Decided,
    /// A retention limit ended it `incomplete`.
    Limit,
    /// It settled at `finish`.
    Finish,
}

impl SettleTrigger {
    /// The wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decided => "decided",
            Self::Limit => "limit",
            Self::Finish => "finish",
        }
    }
}

/// The reading that decided a clause, for the `decided` trigger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecidingReading {
    /// The FSpec FR-243 basis of the deciding reading.
    pub basis: fsl_replay::SettlementBasis,
    /// The seen horizon of the deciding reading.
    pub horizon: u64,
    /// The position at which the reading reports its verdict, if it names one.
    pub position: Option<u64>,
}

/// How a settled clause settled (FR-643 "Settlement"): the report stays
/// `B(s)`, and this records the reading that made it final.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settlement {
    /// The call at which the clause settled.
    pub step: u64,
    /// What settled it.
    pub trigger: SettleTrigger,
    /// The deciding reading; present exactly for the `decided` trigger.
    pub reading: Option<DecidingReading>,
}

/// A completed `monitor` request.
#[derive(Debug)]
pub struct MonitorOutcome {
    /// One record per selected clause, in request order.
    pub records: Vec<MonitorRecord>,
    /// For each record, how its clause settled (FR-643 "Settlement"); a clause
    /// still pending at the end settles at `finish`. Empty for the one-shot
    /// `monitor`, which settles nothing early.
    pub settlements: Vec<Settlement>,
    /// The session's final progress ledger, for a kind that carries progress.
    pub ledger: Option<fsl_replay::spine::ProgressLedger>,
}

impl MonitorOutcome {
    /// The FR-286 `monitor` document of these records.
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for the first record no item can carry.
    pub fn document(&self) -> Result<OutcomeDocument, ClauseRunNotAnItem> {
        OutcomeDocument::from_clause_runs(
            Some(OutcomeStage::S6a),
            self.records
                .iter()
                .map(|record| (record.clause.as_str(), record.report.as_ref())),
        )
    }
}

/// Why a whole `monitor` request was refused; it holds no clause record.
#[derive(Debug)]
pub enum MonitorRefusal {
    /// The trace document is defective.
    TraceDocument {
        /// The catalog record of the first defect.
        record: Box<AdmissionRecord>,
        /// The JSON pointer of that defect.
        pointer: String,
    },
    /// The first trace position whose FR-106 admission failed.
    Admission {
        /// The position's index.
        position: usize,
        /// FR-106's failure.
        failure: Box<AdmissionFailure>,
    },
    /// FR-330 refused a selected clause at stage `admit` for a reason that is
    /// not one position's FR-106 admission, or `monitor`'s own subject check
    /// or emission refused it: a bounded-profile clause over a lasso, an
    /// `over` that resolves to no object, a profile and trace shape that do
    /// not fit, a binding that differs from the clause's rows, or a clause
    /// with no clock selection.
    ClauseRefused {
        /// The selected clause's name.
        clause: String,
        /// The refusal.
        failure: Box<AdmissionFailure>,
    },
    /// A selected clause names no temporal clause of the package.
    UnknownClause {
        /// The selected name.
        name: String,
    },
    /// A refused condition's locus, or a record's named atom, names a source
    /// that `sources` does not hold, or whose bytes do not hold the region's
    /// span on UTF-8 boundaries: `invalid_runtime_input`/`missing-member`
    /// naming `sources`.
    SourceNotSupplied {
        /// The selected clause whose record needs the source.
        clause: String,
        /// The source the region names.
        source: SourceIdentity,
    },
    /// FR-643: a session refuses an observation or progress assertion it
    /// cannot take, as `invalid_runtime_input` with `cause`, and keeps its
    /// state.
    Observation {
        /// The member at fault.
        member: &'static str,
        /// The catalog cause.
        cause: &'static str,
    },
    /// FR-643 "Order": an observation or progress assertion that contradicts
    /// the progress ledger (FR-502).
    Contradiction(Box<ProgressRefusal>),
    /// FR-643 "Checkpoint": a state that does not fit its package and request,
    /// at stage `admit`.
    State {
        /// The refused admission, `invalid_runtime_input`/`invalid-value` with
        /// `reason` `monitor-state-mismatch`.
        failure: Box<AdmissionFailure>,
    },
}

impl MonitorRefusal {
    /// The FR-286 `monitor` document of a whole-request refusal: the
    /// refusal's category and one diagnostic with its catalog code and cause,
    /// and no item.
    pub fn document(&self) -> OutcomeDocument {
        let refused = |catalog: CatalogCode, message: String| {
            let category =
                fsl_foundation::diagnostic::category_of(&catalog).unwrap_or(Category::Refusal);
            OutcomeDocument::new(Operation::Monitor, None, category).with_diagnostics(vec![
                OutcomeDiagnostic::new(Some(catalog.cause()), catalog.code(), None, message),
            ])
        };
        match self {
            Self::TraceDocument { record, pointer } => refused(
                record.catalog,
                format!("trace document defect at `{pointer}`"),
            ),
            Self::Admission { position, failure } => crate::analyze::refusal_document(
                Operation::Monitor,
                failure,
                format!(
                    "trace position {position}: {failure}{}",
                    record_fields_text(failure)
                ),
            ),
            Self::ClauseRefused { clause, failure } => crate::analyze::refusal_document(
                Operation::Monitor,
                failure,
                format!(
                    "clause `{clause}`: {failure}{}",
                    record_fields_text(failure)
                ),
            ),
            Self::UnknownClause { name } => refused(
                CatalogCode::new("missing_declaration", "missing-name"),
                format!("the package declares no temporal clause `{name}`"),
            ),
            Self::SourceNotSupplied { clause, source } => refused(
                CatalogCode::new("invalid_runtime_input", "missing-member"),
                format!("clause `{clause}` needs the source `{source:?}`, which `sources` lacks"),
            ),
            Self::Observation { member, cause } => refused(
                CatalogCode::new("invalid_runtime_input", cause),
                format!("the session refuses `{member}`: {cause}"),
            ),
            Self::Contradiction(refusal) => refused(
                refusal.record.catalog,
                "the supplied progress contradicts the retained ledger".to_owned(),
            ),
            Self::State { failure } => crate::analyze::refusal_document(
                Operation::Monitor,
                failure,
                "the checkpoint does not fit the package and request".to_owned(),
            ),
        }
    }
}

/// A stepping session's or a one-shot request's failure.
pub(crate) type Failure = StageFailure<MonitorRefusal>;

/// FR-283: check `request.trace` against each selected clause of `package`.
///
/// # Errors
///
/// [`StageFailure::Refused`] for a trace document defect, a position whose
/// admission fails, a clause refused at stage `admit`, an unknown clause or
/// a source the request does not supply; [`StageFailure::Cancelled`] when
/// `cancel` is cancelled; [`StageFailure::Fault`] when admission or the
/// evaluator faults.
pub fn monitor(
    package: &CheckedPackage,
    request: &MonitorRequest<'_>,
    limits: &MonitorLimits,
    cancel: &Cancel,
) -> Result<Staged<MonitorOutcome>, Failure> {
    monitor_with_evaluator(run_package_temporal, package, request, limits, cancel)
}

/// The trace evaluator [`monitor`] runs each selected clause through over a
/// `Document` or `Typed` trace.
pub type TraceEvaluator = for<'r> fn(&PackageTemporalRequest<'r>) -> ClauseRunReport;

/// [`monitor`] with the trace evaluator of the `Document` and `Typed`
/// routes supplied by the caller; [`monitor`] passes `run_package_temporal`.
/// FR-283-AC-13 names a test trace evaluator that returns
/// `TemporalRefusal::Fault`.
pub fn monitor_with_evaluator(
    evaluator: TraceEvaluator,
    package: &CheckedPackage,
    request: &MonitorRequest<'_>,
    limits: &MonitorLimits,
    cancel: &Cancel,
) -> Result<Staged<MonitorOutcome>, Failure> {
    let document;
    let input = match &request.trace {
        MonitorTrace::Document(bytes) => {
            document = read_trace(bytes, limits.observation)
                .map_err(|defect| {
                    let (record, pointer) = *defect;
                    let record = Box::new(record);
                    StageFailure::Refused(MonitorRefusal::TraceDocument { record, pointer })
                })?
                .into_input();
            Some(&document)
        }
        MonitorTrace::Typed(input) => Some(*input),
        MonitorTrace::Samples { .. } => None,
    };
    let runs = Runs {
        evaluator,
        package,
        request,
        limits,
        cancel,
    };
    let mut records = Vec::with_capacity(request.clauses.len());
    match input {
        Some(input) => {
            let authentication = runs.authentication(input)?;
            runs.admit_positions(input)?;
            for clause in &request.clauses {
                let report = runs.trace_run(clause, input, authentication.as_ref())?;
                records.push(runs.record_of(clause, report)?);
            }
        }
        None => {
            for clause in &request.clauses {
                let report = runs.samples_run(clause)?;
                records.push(runs.record_of(clause, report)?);
            }
        }
    }
    Ok(Staged::new(MonitorOutcome {
        records,
        settlements: Vec::new(),
        ledger: None,
    }))
}

/// One request's inputs, shared by the stages of [`monitor`].
pub(crate) struct Runs<'a> {
    evaluator: TraceEvaluator,
    package: &'a CheckedPackage,
    request: &'a MonitorRequest<'a>,
    limits: &'a MonitorLimits,
    cancel: &'a Cancel,
}

impl Runs<'_> {
    fn cancelled(&self) -> Result<(), Failure> {
        match self.cancel.cause() {
            Some(cause) => Err(StageFailure::Cancelled(cause)),
            None => Ok(()),
        }
    }

    /// The temporal clause named `name`, when the package declares it.
    fn node(&self, name: &str) -> Option<&TemporalClauseNode> {
        self.package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
    }

    /// FR-522: emit the package with `temporal_selections` and authenticate a
    /// `Progressed` or `Timestamped` trace's binding for every selected
    /// clause, before any position is admitted.
    fn authentication(&self, input: &TraceInput) -> Result<Option<PackageAuthentication>, Failure> {
        if !matches!(
            input,
            TraceInput::Progressed { .. } | TraceInput::Timestamped { .. }
        ) {
            return Ok(None);
        }
        let Some(first) = self.request.clauses.first() else {
            return Ok(None);
        };
        let authentication = authenticate_emission(
            self.package,
            self.request.temporal_selections,
            self.request.packages,
            &SpineLimits::default(),
            self.cancel,
        )
        .map_err(|failure| Self::emission_failure(&self.request.clauses, failure, &first.name))?;
        for clause in &self.request.clauses {
            if let Err(disposition) = authenticate_package_trace(
                self.package,
                &clause.name,
                input,
                Some(authentication.temporal()),
            ) {
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
        Ok(Some(authentication))
    }

    /// The whole-request failure an emission failure stands for.
    ///
    /// FR-283 names only a selected clause's missing selection. The other
    /// defects of the request's `temporal_selections` and `packages` are
    /// refused as `ClauseRefused` on `first`, the first selected clause, with
    /// `invalid_runtime_input`/`invalid-value` naming the member; a missing
    /// selection of a declaration no selected clause owns is
    /// `missing_declaration`/`missing-selection` on `first` as well.
    pub(crate) fn emission_failure(
        clauses: &[MonitorClause],
        failure: AuthenticationFailure,
        first: &str,
    ) -> Failure {
        let refusal = |clause: &str, code, cause, member: &str, stage: Option<&str>| {
            let mut fields = BTreeMap::from([("member", member.to_owned())]);
            if let Some(stage) = stage {
                fields.insert("stage", stage.to_owned());
            }
            StageFailure::Refused(MonitorRefusal::ClauseRefused {
                clause: clause.to_owned(),
                failure: Box::new(refused(code, cause, fields)),
            })
        };
        match failure {
            AuthenticationFailure::Cancelled(cause) => StageFailure::Cancelled(cause),
            AuthenticationFailure::MissingSelection { clause } => {
                let owner = clause
                    .as_deref()
                    .filter(|name| clauses.iter().any(|c| c.name == *name))
                    .unwrap_or(first);
                refusal(
                    owner,
                    Code::MissingDeclaration,
                    "missing-selection",
                    "temporal_selections",
                    None,
                )
            }
            AuthenticationFailure::SelectionsDoNotFit => refusal(
                first,
                Code::InvalidRuntimeInput,
                "invalid-value",
                "temporal_selections",
                None,
            ),
            AuthenticationFailure::PackagesDoNotAdmit(stage) => refusal(
                first,
                Code::InvalidRuntimeInput,
                "invalid-value",
                "packages",
                Some(match stage {
                    PackagesStage::Closure => "closure",
                    PackagesStage::StrictRead => "strict-read",
                }),
            ),
            AuthenticationFailure::Fault(fault) => StageFailure::Fault(fault),
        }
    }

    /// FR-283 Behavior: every position of a `Document` or `Typed` trace is
    /// admitted, in position order, before any clause is evaluated, so an
    /// earlier clause's verdict never hides an admission failure.
    fn admit_positions(&self, input: &TraceInput) -> Result<(), Failure> {
        let graph = self.package.graph();
        for clause in &self.request.clauses {
            // A clause the package does not declare admits nothing here: its
            // run reports the missing name.
            let Some(node) = self.node(&clause.name) else {
                continue;
            };
            let named = node.named_populations();
            for (position, document) in input.documents().enumerate() {
                self.cancelled()?;
                let admitted = admit_trace_snapshot(
                    graph.model_inputs(),
                    graph.scope().types(),
                    self.request.packages,
                    self.limits.normalization,
                    self.request.snapshots,
                    document,
                    self.limits.observation,
                    &named,
                );
                match admitted {
                    Ok(_) => {}
                    Err(AdmissionFailure::Fault(fault)) => return Err(StageFailure::Fault(fault)),
                    Err(failure) => {
                        return Err(StageFailure::Refused(MonitorRefusal::Admission {
                            position,
                            failure: Box::new(failure),
                        }))
                    }
                }
            }
        }
        Ok(())
    }

    /// `monitor`'s own subject check of `clause` against the trace shape
    /// (FR-283 Behavior), before FR-330 admits the clause.
    fn subject_check(&self, clause: &MonitorClause, samples: bool) -> Result<(), Failure> {
        let Some(node) = self.node(&clause.name) else {
            return Err(StageFailure::Refused(MonitorRefusal::UnknownClause {
                name: clause.name.clone(),
            }));
        };
        let model_subject = matches!(node.clause().over().1, ValueType::Reference(_));
        let cause = if samples {
            (clause.over.is_some() || model_subject)
                .then_some(("invalid-value", Code::InvalidRuntimeInput))
        } else if !model_subject {
            Some(("invalid-value", Code::InvalidRuntimeInput))
        } else if clause.over.is_none() {
            Some(("wrong-role-mapping", Code::InvalidRuntimeInput))
        } else {
            None
        };
        match cause {
            Some((cause, code)) => Err(StageFailure::Refused(MonitorRefusal::ClauseRefused {
                clause: clause.name.clone(),
                failure: Box::new(refused(code, cause, BTreeMap::new())),
            })),
            None => Ok(()),
        }
    }

    /// One clause over a `Document` or `Typed` trace.
    fn trace_run(
        &self,
        clause: &MonitorClause,
        input: &TraceInput,
        authentication: Option<&PackageAuthentication>,
    ) -> Result<ClauseRunReport, Failure> {
        self.cancelled()?;
        self.subject_check(clause, false)?;
        let Some(over) = clause.over.as_ref() else {
            return Err(StageFailure::Fault(InternalFault::new(
                "monitor",
                "model-subject-clause-has-an-over",
            )));
        };
        let report = (self.evaluator)(&PackageTemporalRequest {
            package: self.package,
            packages: self.request.packages,
            model_limits: self.limits.normalization,
            snapshots: self.request.snapshots,
            observation_limits: self.limits.observation,
            accounting: self.limits.accounting,
            name: &clause.name,
            trace: input,
            over: Some(over),
            ledger: None,
            sources: self.request.sources,
            authentication: authentication.map(PackageAuthentication::temporal),
            cancel: self.cancel,
        });
        self.cancelled()?;
        Ok(report)
    }

    /// One clause over a `Samples` trace.
    fn samples_run(&self, clause: &MonitorClause) -> Result<ClauseRunReport, Failure> {
        let MonitorTrace::Samples { subject, samples } = &self.request.trace else {
            return Err(StageFailure::Fault(InternalFault::new(
                "monitor",
                "samples-run-has-a-samples-trace",
            )));
        };
        self.cancelled()?;
        self.subject_check(clause, true)?;
        let report = run_package_trace_subject(&PackageTraceSubjectRequest {
            package: self.package,
            sources: self.request.sources,
            packages: self.request.packages,
            model_limits: self.limits.normalization,
            name: &clause.name,
            subject,
            samples,
            accounting: self.limits.accounting,
            max_window_values: self.limits.max_window_values,
            cancel: self.cancel,
        });
        self.cancelled()?;
        Ok(report)
    }

    /// The record of `clause`'s run, or the whole-request failure the run's
    /// disposition stands for.
    fn record_of(
        &self,
        clause: &MonitorClause,
        report: ClauseRunReport,
    ) -> Result<MonitorRecord, Failure> {
        record_of(self.package, self.request.sources, &clause.name, report)
    }
}

/// The record of the run of the clause `clause`, or the whole-request
/// failure the run's disposition stands for.
pub(crate) fn record_of(
    package: &CheckedPackage,
    sources: &[Source],
    clause: &str,
    report: impl Into<Arc<ClauseRunReport>>,
) -> Result<MonitorRecord, Failure> {
    let report: Arc<ClauseRunReport> = report.into();
    let verdict = verdict_of(clause, &report.disposition)?;
    let atom = named_atom(package, sources, clause, &report.disposition)?;
    Ok(MonitorRecord {
        clause: clause.to_owned(),
        verdict,
        category: report.category(),
        atom,
        report,
        usage: None,
    })
}

/// The atom a record names, by the source text of its condition, read
/// from `sources` (FR-283 Behavior).
fn named_atom(
    package: &CheckedPackage,
    sources: &[Source],
    clause: &str,
    disposition: &ClauseDisposition,
) -> Result<Option<String>, Failure> {
    let ClauseDisposition::Temporal(verdict) = disposition else {
        return Ok(None);
    };
    match verdict {
        TemporalVerdict::Indeterminate {
            reason:
                TemporalIndeterminateReason::MissingValuation { atom, .. }
                | TemporalIndeterminateReason::ConflictingValuation { atom, .. },
            ..
        } => Ok(Some(atom.clone())),
        TemporalVerdict::Undefined {
            cause: TemporalUndefinedCause::InsufficientData { atom, .. },
            ..
        } => match name_clause_atom(package, sources, clause, *atom) {
            Ok(name) => Ok(Some(name)),
            Err(refusal) => Err(match *refusal {
                RunRefusal::SourceNotSupplied { reference } => {
                    StageFailure::Refused(MonitorRefusal::SourceNotSupplied {
                        clause: clause.to_owned(),
                        source: reference,
                    })
                }
                RunRefusal::Fault(fault) => StageFailure::Fault(fault),
                _ => StageFailure::Fault(InternalFault::new(
                    "monitor",
                    "atom-naming-refuses-with-a-source-or-fault",
                )),
            }),
        },
        _ => Ok(None),
    }
}

/// The whole-request failure `disposition` stands for, when it stands for
/// one.
pub(crate) fn request_failure(clause: &str, disposition: &ClauseDisposition) -> Option<Failure> {
    use ClauseDisposition as D;
    Some(match disposition {
        D::EvaluateFault(fault) => StageFailure::Fault(fault.clone()),
        // Every position admitted before the run, so this is FR-330's own
        // admit-stage refusal of the clause, or a fault of admission itself.
        D::Admit(AdmissionFailure::Fault(fault)) => StageFailure::Fault(fault.clone()),
        D::Admit(failure) => StageFailure::Refused(MonitorRefusal::ClauseRefused {
            clause: clause.to_owned(),
            failure: Box::new(failure.clone()),
        }),
        D::ArgumentRefusal(refusal) => match refusal.as_ref() {
            RunRefusal::SourceNotSupplied { reference } => {
                StageFailure::Refused(MonitorRefusal::SourceNotSupplied {
                    clause: clause.to_owned(),
                    source: reference.clone(),
                })
            }
            _ => return None,
        },
        D::TemporalRefusal(TemporalTraceRefusal::Fault(fault)) => {
            StageFailure::Fault(fault.clone())
        }
        // FR-283-AC-13: a trace of a kind the clause's profile does not read
        // is refused at admit as the request, naming the clause (FR-645).
        D::TraceRefused(refusal) if refusal.stage == ClauseRunStage::Admit => {
            StageFailure::Refused(MonitorRefusal::ClauseRefused {
                clause: clause.to_owned(),
                failure: Box::new(refused(
                    Code::InvalidRuntimeInput,
                    "invalid-value",
                    refusal.record_fields(),
                )),
            })
        }
        _ => return None,
    })
}

/// The verdict `disposition` stands for, or the whole-request failure.
pub(crate) fn verdict_of(
    clause: &str,
    disposition: &ClauseDisposition,
) -> Result<MonitorVerdict, Failure> {
    use ClauseDisposition as D;
    use TemporalVerdict as V;
    if let Some(failure) = request_failure(clause, disposition) {
        return Err(failure);
    }
    Ok(match disposition {
        D::Temporal(verdict) => match verdict {
            V::Holds => MonitorVerdict::Tested,
            V::Violated { .. } => MonitorVerdict::Violated,
            V::Undefined {
                cause: TemporalUndefinedCause::InsufficientData { .. },
                ..
            } => MonitorVerdict::InsufficientData,
            V::Undefined { .. } => MonitorVerdict::Undefined,
            V::Pending | V::PendingWindow { .. } | V::PendingNextEvent { .. } => {
                MonitorVerdict::Pending
            }
            V::Indeterminate { .. } => MonitorVerdict::Indeterminate,
            V::MissingFairnessPremise { .. } => MonitorVerdict::MissingFairnessPremise,
            V::Incomplete(_)
            | V::ActivationIncomplete { .. }
            | V::ProgressLimitReached(_)
            | V::ExpansionLimitReached(_) => MonitorVerdict::Incomplete,
            V::WindowLimitReached { .. } => MonitorVerdict::WindowLimitReached,
        },
        D::TemporalRefusal(_) | D::TraceRefused(_) => MonitorVerdict::TraceRefused,
        D::Evaluate(_) | D::ArgumentRefusal(_) => MonitorVerdict::ConditionRefused,
        other => {
            return Err(StageFailure::Fault(InternalFault::new(
                "monitor",
                match other {
                    D::Compile(_) | D::UnknownLanguage { .. } => "clause-run-compiled-source",
                    _ => "clause-run-disposition-is-a-temporal-result",
                },
            )))
        }
    })
}

/// A refused admission failure with `code` and `cause`.
/// The record's fields as ` (name=value, ...)`, empty when it has none, so a
/// rendered refusal keeps its `reason` and the members FR-645 names.
fn record_fields_text(failure: &AdmissionFailure) -> String {
    match failure {
        AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record)
            if !record.fields.is_empty() =>
        {
            let fields: Vec<String> = record
                .fields
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            format!(" ({})", fields.join(", "))
        }
        _ => String::new(),
    }
}

pub(crate) fn refused(
    code: Code,
    cause: &'static str,
    fields: BTreeMap<&'static str, String>,
) -> AdmissionFailure {
    AdmissionFailure::Refused(Box::new(AdmissionRecord {
        catalog: CatalogCode::new(code.as_str(), cause),
        fields,
        path: Vec::new(),
        kernel: None,
    }))
}

/// A trace document read: its prefix and, for a lasso, its loop.
struct TraceDocument {
    prefix: Vec<DocumentRef>,
    repeat: Option<Vec<DocumentRef>>,
}

impl TraceDocument {
    fn into_input(self) -> TraceInput {
        match self.repeat {
            None => TraceInput::Finite(self.prefix),
            Some(repeat) => TraceInput::Lasso {
                prefix: self.prefix,
                repeat,
            },
        }
    }
}

type Defect = Box<(AdmissionRecord, String)>;

fn defect(code: Code, cause: &'static str, field: Option<&str>, pointer: &str) -> Defect {
    let mut fields = BTreeMap::new();
    if let Some(field) = field {
        fields.insert("field", field.to_owned());
    }
    Box::new((
        AdmissionRecord {
            catalog: CatalogCode::new(code.as_str(), cause),
            fields,
            path: Vec::new(),
            kernel: None,
        },
        pointer.to_owned(),
    ))
}

/// A limit defect of the trace document: `stage_limit_exceeded` with the
/// limit's cause and its `kind`, `bound`, `actual` and `setting` fields.
fn limit_defect(limit: &LimitExceeded) -> Defect {
    let mut found = defect(
        Code::StageLimitExceeded,
        limit.kind().catalog_cause(),
        None,
        "",
    );
    if let Some(fields) = limit.catalog_fields() {
        found.0.fields.extend(fields);
    }
    found
}

/// Charge every object and value form of the document, nested ones included,
/// against `observation.objects` and `observation.values` (FR-106).
fn charge_document(root: NodeRef<'_>, limits: ObservationLimits) -> Result<(), Defect> {
    let (mut objects, mut values) = (0_u64, 0_u64);
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        values = values.saturating_add(1);
        if values > limits.values_per_document {
            return Err(limit_defect(&LimitExceeded::new(
                Setting::ObservationValues,
                limits.values_per_document,
                u128::from(values),
            )));
        }
        match node.node() {
            Node::Object(members) => {
                objects = objects.saturating_add(1);
                if objects > limits.objects_per_document {
                    return Err(limit_defect(&LimitExceeded::new(
                        Setting::ObservationObjects,
                        limits.objects_per_document,
                        u128::from(objects),
                    )));
                }
                pending.extend(members.map(|(_, member)| member));
            }
            Node::Array(items) => pending.extend(items),
            _ => {}
        }
    }
    Ok(())
}

/// Read a trace document under FR-283 "Trace document", at its first defect.
fn read_trace(bytes: &[u8], limits: ObservationLimits) -> Result<TraceDocument, Defect> {
    let input_bytes = limits.document_bytes;
    let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual > input_bytes {
        return Err(limit_defect(&LimitExceeded::new(
            Setting::ObservationInputBytes,
            input_bytes,
            u128::from(actual),
        )));
    }
    let document = quire_canonical::read(bytes, input_bytes)
        .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", None, ""))?;
    let root = document.root();
    let Node::Object(members) = root.node() else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            "",
        ));
    };
    // The reader and kind refusals come first (FR-283 "Trace document"
    // order); the counts need a document that is an object.
    charge_document(root, limits)?;
    let names: Vec<&str> = members.map(|(name, _)| name).collect();
    if let Some(missing) = ["format", "prefix"]
        .into_iter()
        .find(|name| !names.contains(name))
    {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "missing-member",
            Some(missing),
            "",
        ));
    }
    if let Some(unknown) = names
        .iter()
        .find(|name| !["format", "prefix", "loop"].contains(name))
    {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "unknown-member",
            Some(unknown),
            "",
        ));
    }
    let member = |name: &str| root.get(name);
    let format_ok = member("format")
        .is_some_and(|format| matches!(format.node(), Node::String(text) if text == TRACE_FORMAT));
    if !format_ok {
        return Err(defect(
            Code::UnknownWire,
            "unsupported-wire",
            None,
            "/format",
        ));
    }
    let prefix = read_refs(member("prefix"), "/prefix")?;
    let repeat = member("loop")
        .map(|node| read_refs(Some(node), "/loop"))
        .transpose()?;
    if repeat.is_none() && prefix.is_empty() {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "invalid-value",
            None,
            "/prefix",
        ));
    }
    if repeat.as_ref().is_some_and(Vec::is_empty) {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "invalid-value",
            None,
            "/loop",
        ));
    }
    Ok(TraceDocument { prefix, repeat })
}

fn read_refs(node: Option<NodeRef<'_>>, pointer: &str) -> Result<Vec<DocumentRef>, Defect> {
    let wrong = |at: &str| defect(Code::InvalidRuntimeInput, "wrong-value-kind", None, at);
    let Some(Node::Array(items)) = node.map(|node| node.node()) else {
        return Err(wrong(pointer));
    };
    items
        .enumerate()
        .map(|(index, item)| read_ref(item, &format!("{pointer}/{index}")))
        .collect()
}

/// One `DocumentRef` element: `{"identity": {"authority", "identity"}}`.
/// Reads one document reference object; member names are wire words compared at
/// this JSON boundary.
#[ix_trace_rs::string_edge]
fn read_ref(item: NodeRef<'_>, pointer: &str) -> Result<DocumentRef, Defect> {
    let Node::Object(members) = item.node() else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            pointer,
        ));
    };
    let names: Vec<&str> = members.map(|(name, _)| name).collect();
    let Some(identity) = item.get("identity") else {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "missing-member",
            Some("identity"),
            pointer,
        ));
    };
    if let Some(unknown) = names.iter().find(|name| **name != "identity") {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "unknown-member",
            Some(unknown),
            pointer,
        ));
    }
    let identity_pointer = format!("{pointer}/identity");
    if !matches!(identity.node(), Node::Object(_)) {
        return Err(defect(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            None,
            &identity_pointer,
        ));
    }
    let label = |name: &'static str| -> Result<String, Defect> {
        match identity.get(name).map(|node| node.node()) {
            Some(Node::String(text)) => Ok(text.to_owned()),
            Some(_) => Err(defect(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
                None,
                &format!("{identity_pointer}/{name}"),
            )),
            None => Err(defect(
                Code::InvalidRuntimeInput,
                "missing-member",
                Some(name),
                &identity_pointer,
            )),
        }
    };
    let labels = SourceIdentity::new(label("authority")?, label("identity")?);
    if let Node::Object(members) = identity.node() {
        if let Some((unknown, _)) = members
            .into_iter()
            .find(|(name, _)| !["authority", "identity"].contains(name))
        {
            return Err(defect(
                Code::InvalidRuntimeInput,
                "unknown-member",
                Some(unknown),
                &identity_pointer,
            ));
        }
    }
    if let Some(blank) = labels.first_blank_label() {
        let mut found = defect(
            Code::InvalidSourceIdentity,
            "blank-label",
            None,
            &identity_pointer,
        );
        found.0.fields.insert("label", blank.as_str().to_owned());
        return Err(found);
    }
    let reference = DocumentRef {
        authority: labels.authority,
        identity: labels.identity,
    };
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault() -> InternalFault {
        InternalFault::new("test", "a-broken-invariant")
    }

    fn refused() -> AdmissionFailure {
        AdmissionFailure::Refused(Box::new(AdmissionRecord {
            catalog: CatalogCode::new("invalid_runtime_input", "invalid-value"),
            fields: BTreeMap::new(),
            path: Vec::new(),
            kernel: None,
        }))
    }

    /// FR-283 Outputs: a `TemporalRefusal::Fault` disposition maps to
    /// `StageFailure::Fault`, never a record. The full-request case is
    /// `tests/it/monitor.rs`.
    #[test]
    fn a_trace_evaluator_fault_fails_the_request() {
        let disposition = ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::Fault(fault()));
        assert!(matches!(
            verdict_of("P", &disposition),
            Err(StageFailure::Fault(_))
        ));
        let disposition = ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::EmptyLoop);
        assert!(matches!(
            verdict_of("P", &disposition),
            Ok(MonitorVerdict::TraceRefused)
        ));
    }

    /// FR-330: an admit-stage refusal that is no position's admission names
    /// its clause, and an admission fault fails the request.
    #[test]
    fn a_clause_admit_refusal_names_its_clause_and_a_fault_is_a_fault() {
        let Err(StageFailure::Refused(MonitorRefusal::ClauseRefused { clause, .. })) =
            verdict_of("P", &ClauseDisposition::Admit(refused()))
        else {
            panic!("a clause refusal");
        };
        assert_eq!(clause, "P");
        assert!(matches!(
            verdict_of(
                "P",
                &ClauseDisposition::Admit(AdmissionFailure::Fault(fault()))
            ),
            Err(StageFailure::Fault(_))
        ));
    }
}
