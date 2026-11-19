// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-644: the `native-monitor/1` request and the record stream it runs. The
//! command compiles the program once, opens FR-643's [`MonitorSession`] and
//! hands the caller one event per clause status change, then a summary.
//! Nothing here writes to the process streams.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use filament_exact::Cancel;
use filament_semantic_value::catalog::Category;
use fsl_analyze::analyze::document::{Negotiate, RegistryEntry, SettledRun};
use fsl_analyze::analyze::Engine;
use fsl_analyze::monitor::{
    MonitorClause, MonitorLimits, MonitorRefusal, SettleTrigger, Settlement,
};
use fsl_analyze::monitor_session::{
    ClauseStatus, MonitorObservation, MonitorOpen, MonitorPosition, MonitorSession, MonitorStamp,
    MonitorState, MonitorStep, MonitorTraceKind,
};
use fsl_analyze::provider::ProviderOptions;
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::source::Source;
use fsl_replay::spine::{
    read_cover, read_ledger, read_lower_boundary, read_progress_assertion, read_sample_text,
    read_trace_binding, write_clause_run_result, write_ledger, OverObject, ProgressLowerBoundary,
    SpineLimits, TimestampedOrderKey,
};
use serde_json::{json, Value};

use super::settle::Settle;
use super::{
    check_sources, intake_with_bytes, request, request_integer_division, require_complete_edition,
    wire, FileCounts, NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
};

/// The format of one event.
pub const EVENT_FORMAT: &str = "filament.monitor-event/1";
/// The format of the summary.
pub const SUMMARY_FORMAT: &str = "filament.monitor-summary/1";
/// The format of the batch result.
pub const RUN_FORMAT: &str = "filament.monitor-run/1";

/// One `filament.monitor-event/1` object.
#[derive(Clone, Debug)]
pub struct MonitorEvent {
    value: Value,
}

impl MonitorEvent {
    /// The event as its JSON object.
    pub fn as_value(&self) -> &Value {
        &self.value
    }
}

impl std::fmt::Display for MonitorEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(formatter)
    }
}

/// The `filament.monitor-summary/1` object and the exit code FR-285 assigns
/// the most severe final category.
#[derive(Clone, Debug)]
pub struct MonitorSummary {
    value: Value,
    outcome: fsl_replay::OutcomeDocument,
    /// The exit code of the run.
    pub exit_code: u8,
}

impl MonitorSummary {
    /// FR-644: the summary of a request carrying `items`, with each item's
    /// FR-621 record and the FR-635 registry, and the exit code counting a
    /// settled item's category beside the clauses'.
    fn with_items(mut self, settled: &SettledRun) -> Self {
        if let Some(members) = self.value.as_object_mut() {
            members.insert("items".to_owned(), Value::Array(settled.items.clone()));
            members.insert(
                "registry".to_owned(),
                Value::Array(settled.registry.iter().map(RegistryEntry::wire).collect()),
            );
        }
        self.exit_code = severity_exit([self.exit_code, settled.exit_code].into_iter());
        self
    }

    /// The FR-286 `monitor` outcome document of the run (category and last
    /// stage).
    pub fn outcome(&self) -> &fsl_replay::OutcomeDocument {
        &self.outcome
    }

    /// The summary as its JSON object.
    pub fn as_value(&self) -> &Value {
        &self.value
    }
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// The request-stage cause a session refusal stands for.
fn cause_of(failure: StageFailure<MonitorRefusal>) -> RunCause {
    match failure {
        StageFailure::Refused(refusal) => RunCause::MonitorRefused(Box::new(refusal)),
        StageFailure::Cancelled(_) => RunCause::MonitorCancelled,
        StageFailure::Fault(_) => RunCause::MonitorFault,
        StageFailure::Limit(limit) | StageFailure::LimitWithPayload(limit, _) => {
            invalid("limits", format!("{limit:?}"))
        }
        StageFailure::RefusedWithPayload(refusal, _) => RunCause::MonitorRefused(refusal),
    }
}

fn report_value(status: &ClauseStatus) -> Result<(Value, Value, Value)> {
    let bytes =
        write_clause_run_result(status.record().report()).map_err(RunCause::ClauseOutput)?;
    let document: Value = serde_json::from_slice(&bytes).map_err(RunCause::Output)?;
    let take = |member: &str| document.get(member).cloned().unwrap_or(Value::Null);
    Ok((take("disposition"), take("basis"), take("ledger")))
}

fn event(
    record: usize,
    position: Option<u64>,
    clause: &str,
    status: &ClauseStatus,
) -> Result<Value> {
    let (disposition, basis, _) = report_value(status)?;
    let mut event = json!({
        "format": EVENT_FORMAT,
        "record": record,
        "position": position,
        "clause": clause,
        "settled": status.is_settled(),
        "category": disposition.get("category").cloned().unwrap_or(Value::Null),
        "basis": basis,
        "disposition": disposition,
    });
    if let (Some(settlement), Some(members)) = (status.settlement(), event.as_object_mut()) {
        members.insert("settlement".to_owned(), settlement_value(settlement));
    }
    Ok(event)
}

/// FR-644: a settlement as its closed JSON object, counts as decimal strings.
fn settlement_value(settlement: &Settlement) -> Value {
    let mut members = serde_json::Map::new();
    members.insert("step".to_owned(), json!(settlement.step.to_string()));
    members.insert("trigger".to_owned(), json!(settlement.trigger.as_str()));
    if let SettleTrigger::Cancelled(cause) = settlement.trigger {
        members.insert("cause".to_owned(), json!(cancel_cause(cause)));
    }
    if let (SettleTrigger::Decided, Some(reading)) = (settlement.trigger, &settlement.reading) {
        members.insert("basis".to_owned(), json!(reading.basis.as_str()));
        members.insert("horizon".to_owned(), json!(reading.horizon.to_string()));
        members.insert(
            "position".to_owned(),
            json!(reading.position.map(|position| position.to_string())),
        );
    }
    Value::Object(members)
}

/// FR-276's wire spelling of a cancellation cause.
fn cancel_cause(cause: filament_exact::CancelCause) -> &'static str {
    match cause {
        filament_exact::CancelCause::Requested => "requested",
        filament_exact::CancelCause::Deadline => "deadline",
    }
}

/// The event of a session refusal: FR-643's record as the disposition.
fn session_refusal_event(
    record: usize,
    position: Option<u64>,
    refusal: &MonitorRefusal,
) -> Result<Value> {
    let document = refusal.document();
    let disposition = serde_json::to_value(&document).map_err(RunCause::Output)?;
    Ok(json!({
        "format": EVENT_FORMAT,
        "record": record,
        "position": position,
        "clause": Value::Null,
        "settled": false,
        "category": document.category().as_str(),
        "basis": "unavailable",
        "disposition": disposition,
    }))
}

fn refusal_event(record: usize, position: Option<u64>, member: &str, reason: &str) -> Value {
    json!({
        "format": EVENT_FORMAT,
        "record": record,
        "position": position,
        "clause": Value::Null,
        "settled": false,
        "category": "refusal",
        "basis": "unavailable",
        "disposition": {
            "stage": "request",
            "category": "refusal",
            "cause": "invalid-request",
            "member": member,
            "message": reason,
        },
    })
}

/// The code of the most severe category according to the shared catalog.
fn severity_exit(codes: impl Iterator<Item = u8>) -> u8 {
    Category::most_severe(codes).unwrap_or(0)
}

/// One record of the stream, read.
enum Record {
    Observation(Box<Observed>),
    Progress(fsl_replay::spine::ProgressAssertion),
    Checkpoint(String),
    End,
}

struct Observed {
    position: Position,
    stamp: Option<u64>,
    order_key: Option<TimestampedOrderKey>,
}

enum Position {
    Snapshot(Vec<u8>),
    Sample(String),
}

/// The record in `line`, or the member and reason it is refused for.
fn read_record(
    line: &[u8],
    snapshots: &BTreeMap<String, Vec<u8>>,
) -> std::result::Result<Record, (String, String)> {
    let refuse = |member: &str, reason: &str| (member.to_owned(), reason.to_owned());
    let value: Value =
        serde_json::from_slice(line).map_err(|error| refuse("record", &error.to_string()))?;
    let Value::Object(members) = value else {
        return Err(refuse("record", "a record is one JSON object"));
    };
    let allowed: &[&str] = if members.contains_key("observation") {
        &["observation", "stamp", "order_key"]
    } else {
        &["progress", "checkpoint", "end", "through"]
    };
    if let Some(unknown) = members
        .keys()
        .find(|name| !allowed.contains(&name.as_str()))
    {
        return Err(refuse(unknown, "is not a member of this record"));
    }
    if let Some(position) = members.get("observation") {
        let position = match position {
            Value::String(key) => Position::Snapshot(
                snapshots
                    .get(key)
                    .cloned()
                    .ok_or_else(|| refuse("observation", "names a key `observations` lacks"))?,
            ),
            Value::Object(object) if object.len() == 1 => {
                if let Some(snapshot) = object.get("snapshot") {
                    Position::Snapshot(
                        serde_json::to_vec(snapshot)
                            .map_err(|error| refuse("observation", &error.to_string()))?,
                    )
                } else if let Some(sample) = object.get("sample") {
                    let raw = serde_json::value::to_raw_value(sample)
                        .map_err(|error| refuse("observation", &error.to_string()))?;
                    Position::Sample(
                        read_sample_text(&raw, "observation")
                            .map_err(|error| refuse("observation", &error.reason))?,
                    )
                } else {
                    return Err(refuse("observation", "is a key, a snapshot or a sample"));
                }
            }
            _ => return Err(refuse("observation", "is a key, a snapshot or a sample")),
        };
        let stamp = match members.get("stamp") {
            None => None,
            Some(stamp) => Some(
                stamp
                    .as_u64()
                    .ok_or_else(|| refuse("stamp", "is a non-negative integer tick"))?,
            ),
        };
        let order_key = match members.get("order_key") {
            None => None,
            Some(Value::Object(key)) if key.len() == 2 => Some(TimestampedOrderKey {
                authority: key
                    .get("authority")
                    .and_then(Value::as_str)
                    .ok_or_else(|| refuse("order_key", "needs an authority string"))?
                    .to_owned(),
                key: key
                    .get("key")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| refuse("order_key", "needs an integer key"))?,
            }),
            Some(_) => return Err(refuse("order_key", "is {authority, key}")),
        };
        return Ok(Record::Observation(Box::new(Observed {
            position,
            stamp,
            order_key,
        })));
    }
    if members.len() != 1 {
        return Err(refuse(
            "record",
            "is exactly one of observation, progress, checkpoint or end",
        ));
    }
    if let Some(progress) = members.get("progress") {
        let wire: fsl_replay::spine::ProgressWire = serde_json::from_value(progress.clone())
            .map_err(|error| refuse("progress", &error.to_string()))?;
        return read_progress_assertion(&wire)
            .map(Record::Progress)
            .map_err(|error| refuse("progress", &error.reason));
    }
    if let Some(path) = members.get("checkpoint") {
        let path = path
            .as_str()
            .ok_or_else(|| refuse("checkpoint", "is a relative path"))?;
        let local = Path::new(path);
        let escapes = path.is_empty()
            || local
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir));
        if escapes {
            return Err(refuse(
                "checkpoint",
                "is a path below the request directory",
            ));
        }
        return Ok(Record::Checkpoint(path.to_owned()));
    }
    if members.contains_key("through") {
        return Err(refuse(
            "through",
            "is for the timed kind, which is not built yet",
        ));
    }
    match members.get("end") {
        Some(Value::Bool(true)) => Ok(Record::End),
        _ => Err(refuse(
            "end",
            "is true; the stream ends, the trace takes no closure",
        )),
    }
}

/// FR-644 "Provider-routed items": the request's `items` with the
/// `subject`, `documents` and `deadline_ms` beside them, each read as FR-620
/// reads it in a `native-analyze/1` request; `None` for a request without
/// `items`, which carries none of the three.
fn read_routed(
    intake: &mut super::Intake<'_>,
    request: &mut wire::MonitorRequest,
) -> Result<Option<super::analyze::RoutedItems>> {
    let Some(items) = request.items.take() else {
        let beside = [
            ("subject", request.subject.is_some()),
            ("documents", !request.documents.is_empty()),
            ("deadline_ms", request.deadline_ms.is_some()),
        ];
        return match beside.iter().find(|(_, present)| *present) {
            Some((member, _)) => Err(invalid(member, "is read only beside items")),
            None => Ok(None),
        };
    };
    if items.is_empty() {
        return Err(invalid("items", "is a non-empty array of analysis items"));
    }
    super::analyze::validate_items(Some(&items))?;
    let documents = super::analyze::read_documents(intake, &request.documents, Some(&items))?;
    let subject = super::analyze::read_subject(request.subject.take(), &documents)?;
    Ok(Some(super::analyze::RoutedItems {
        documents,
        subject,
        memory: Vec::new(),
        items: Some(items),
        limits: super::analyze::analyze_limits(
            &wire::AnalyzeLimits::default(),
            request.deadline_ms,
        ),
        deadline_ms: request.deadline_ms,
    }))
}

/// What a monitor run needs besides the compiled unit.
struct Inputs {
    clauses: Vec<MonitorClause>,
    kind: MonitorTraceKind,
    lower_boundary: ProgressLowerBoundary,
    binding: Option<fsl_replay::spine::TraceBinding>,
    cover: Option<fsl_replay::spine::TimestampedCover>,
    ledger: Option<fsl_replay::spine::ProgressLedger>,
    state: Option<MonitorState>,
    snapshots: BTreeMap<String, Vec<u8>>,
    /// The provider-routed `items` and what they read, when the request
    /// carries `items` (FR-644 "Provider-routed items").
    routed: Option<super::analyze::RoutedItems>,
}

fn read_inputs(
    intake: &mut super::Intake<'_>,
    mut request: wire::MonitorRequest,
) -> Result<Inputs> {
    let routed = read_routed(intake, &mut request)?;
    let kind = match request.trace.kind {
        wire::MonitorKindWire::Finite => MonitorTraceKind::Finite,
        wire::MonitorKindWire::Progressed => MonitorTraceKind::Progressed,
        wire::MonitorKindWire::Timestamped => MonitorTraceKind::Timestamped,
        wire::MonitorKindWire::Timed => {
            return Err(invalid(
                "trace.kind",
                "a timed trace is read but not yet run: the timed clock kind is not built",
            ))
        }
    };
    let lower_boundary = request.trace.lower_boundary.map_or(
        ProgressLowerBoundary::AuthoritativeOrigin,
        read_lower_boundary,
    );
    let mut snapshots = BTreeMap::new();
    for (key, file) in &request.observations {
        let remaining = intake.remaining;
        snapshots.insert(key.clone(), intake.file(file, remaining)?);
    }
    let ledger = request
        .ledger
        .as_deref()
        .map(|path| {
            let bytes = intake.file(path, intake.remaining)?;
            read_ledger(&bytes).map_err(|error| invalid(&error.member, error.reason))
        })
        .transpose()?;
    let state = request
        .state
        .as_deref()
        .map(|path| {
            intake
                .file(path, intake.remaining)
                .map(MonitorState::from_bytes)
        })
        .transpose()?;
    let clauses = request
        .clauses
        .iter()
        .map(|clause| MonitorClause {
            name: clause.name.clone(),
            over: clause.over.as_ref().map(|over| OverObject {
                population: over.reference.population.clone(),
                key: over.reference.key.clone(),
            }),
        })
        .collect();
    Ok(Inputs {
        clauses,
        kind,
        lower_boundary,
        binding: request.trace.binding.map(read_trace_binding),
        cover: request.trace.cover.map(read_cover),
        ledger,
        state,
        snapshots,
        routed,
    })
}

/// Where a run reads its records from.
enum Records<'r> {
    /// The request's `records` file.
    Batch,
    /// The caller's stream.
    Stream(Box<dyn Iterator<Item = Result<Vec<u8>>> + 'r>),
}

/// Drive one stream. `lines` yields the records; `sink` gets each event.
fn drive(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    source: Records<'_>,
    sink: &mut dyn FnMut(&MonitorEvent),
    settle: &Settle<'_>,
    cancel: &Cancel,
) -> Result<MonitorSummary> {
    let mut request: wire::MonitorRequest = request(bytes)?;
    let integer_division = request_integer_division(request.integer_division.take())?;
    let records_file = request.records.clone();
    match (matches!(source, Records::Batch), &records_file) {
        (true, None) => {
            return Err(invalid(
                "records",
                "a native-monitor/1 request run through run needs a records file",
            ))
        }
        (false, Some(_)) => {
            return Err(invalid(
                "records",
                "a records file is read by run; monitor reads the stream it is given",
            ))
        }
        _ => {}
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: request
                .observations
                .len()
                .saturating_add(request.documents.len()),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let program = intake.source(&request.program.source, "program.source")?;
    require_complete_edition(program.source())?;
    let compile = wire::CompileRequest {
        models: std::mem::take(&mut request.models),
        program: wire::CompileProgram {
            sources: Vec::new(),
            extraction: false,
        },
        refinement_internals: None,
        libraries: std::mem::take(&mut request.libraries),
        extensions: Vec::new(),
        clocks: std::mem::take(&mut request.clocks),
        output: None,
        handoff: None,
        integer_division: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    let program_source = program.source().clone();
    check_sources(
        &compile,
        &mut intake,
        std::slice::from_ref(&program),
        false,
        SpineLimits::default(),
        integer_division,
        None,
        |intake, checked, selections, _, packages, libraries, _, _, _, _| {
            // A missing integer-division selection refuses during compilation;
            // defer reading observations until that stage has succeeded.
            let records: Box<dyn Iterator<Item = Result<Vec<u8>>> + '_> = match source {
                Records::Batch => {
                    Box::new(file_lines(intake, records_file.as_deref())?.into_iter())
                }
                Records::Stream(records) => records,
            };
            let inputs = read_inputs(intake, request)?;
            // FR-644 "Provider-routed items": the items settle after the
            // program compiles and before the first record is read.
            let items = inputs
                .routed
                .as_ref()
                .map(|routed| {
                    // FR-644: the items settle under their own handle, which the
                    // stream's cancellation reaches, so their `deadline_ms` stops
                    // the items alone and every event stays the stream's.
                    chained(settle.cancel, |items_cancel| {
                        let settle = Settle {
                            negotiate: settle.negotiate,
                            engines: settle.engines,
                            cancel: items_cancel,
                        };
                        // FR-635: a provider's `package` is this compile's own
                        // emission, under the request's clock selections. The
                        // emission is remembered so the provider request reads
                        // these bytes; an emission the selections refuse leaves
                        // the provider items `package-not-emitted` (FSpec FR-331).
                        let _ = fsl_package::emit_dependency_with_temporal_selections_and_cancel(
                            checked.package(),
                            selections,
                            settle.cancel,
                            routed.limits.emit,
                        );
                        super::analyze::settle_items(
                            checked.package(),
                            &packages,
                            std::iter::once(&program).chain(libraries.iter().copied()),
                            routed,
                            &settle,
                        )
                    })
                })
                .transpose()?;
            let mut sources: Vec<Source> = vec![program_source];
            sources.extend(libraries.iter().map(|library| library.source().clone()));
            let limits = MonitorLimits {
                observation: Default::default(),
                normalization: SpineLimits::default().model,
                accounting: super::call_accounting(&wire::Accounting::default()),
                max_window_values: fsl_analyze::monitor::DEFAULT_MAX_WINDOW_VALUES,
                retained_positions: fsl_analyze::monitor::DEFAULT_RETAINED_POSITIONS,
                retained_identities: fsl_analyze::monitor::DEFAULT_RETAINED_IDENTITIES,
            };
            let open = || MonitorOpen {
                clauses: inputs.clauses.clone(),
                kind: inputs.kind,
                lower_boundary: inputs.lower_boundary,
                binding: inputs.binding.clone(),
                cover: inputs.cover,
                ledger: inputs.ledger.clone(),
                packages: &packages,
                sources: &sources,
                temporal_selections: selections,
            };
            let package = checked.package();
            let start = |under: &Cancel| match &inputs.state {
                Some(state) => MonitorSession::resume(package, state, open(), &limits, under),
                None => MonitorSession::open(package, open(), &limits, under),
            };
            // FR-853: a tripped handle still opens the session, so an `open`
            // or `resume` refusal wins over the cancellation; the stream then
            // reads no record and returns the cancelled summary.
            let session = match start(cancel) {
                Err(StageFailure::Cancelled(_)) => start(&Cancel::new()),
                started => started,
            }
            .map_err(cause_of)?;
            stream(
                directory,
                session,
                &inputs.snapshots,
                records,
                inputs
                    .clauses
                    .iter()
                    .map(|clause| clause.name.clone())
                    .collect(),
                sink,
                cancel,
            )
            .map(|summary| match &items {
                Some(settled) => summary.with_items(settled),
                None => summary,
            })
        },
    )
}

/// Run the records through `session` and write the summary.
fn stream(
    directory: &Path,
    mut session: MonitorSession<'_>,
    snapshots: &BTreeMap<String, Vec<u8>>,
    records: Box<dyn Iterator<Item = Result<Vec<u8>>> + '_>,
    names: Vec<String>,
    sink: &mut dyn FnMut(&MonitorEvent),
    cancel: &Cancel,
) -> Result<MonitorSummary> {
    let mut observed = 0_u64;
    let mut read = 0_u64;
    let mut refused = 0_u64;
    let mut emit = |value: Value| {
        sink(&MonitorEvent { value });
    };
    let mut records = records.enumerate();
    // FR-853: the handle is read before each record, after each step and when
    // the iterator ends; a tripped handle wins over the end of the iterator.
    let mut stopped = None;
    loop {
        if let Some(cause) = cancel.cause() {
            stopped = Some(cause);
            break;
        }
        let Some((index, line)) = records.next() else {
            stopped = cancel.cause();
            break;
        };
        let line = line?;
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        read += 1;
        let position = observed.checked_sub(1);
        let record = match read_record(&line, snapshots) {
            Ok(record) => record,
            Err((member, reason)) => {
                refused += 1;
                emit(refusal_event(index, position, &member, &reason));
                continue;
            }
        };
        let stepped = match record {
            Record::End => break,
            Record::Checkpoint(path) => {
                let target = directory.join(&path);
                std::fs::write(&target, session.checkpoint().map_err(cause_of)?.as_bytes())
                    .map_err(|error| RunCause::Io {
                        path: target,
                        error,
                    })?;
                continue;
            }
            Record::Progress(progress) => session.assert_progress(progress, cancel),
            Record::Observation(item) => {
                let position = match &item.position {
                    Position::Snapshot(bytes) => MonitorPosition::Snapshot(bytes),
                    Position::Sample(text) => MonitorPosition::Sample(text),
                };
                let step = session.step(
                    MonitorObservation {
                        position,
                        stamp: item.stamp.map(MonitorStamp::Tick),
                        order_key: item.order_key.clone(),
                    },
                    cancel,
                );
                if step.is_ok() {
                    observed += 1;
                }
                step
            }
        };
        match stepped {
            Ok(step) => {
                let position = observed.checked_sub(1);
                for value in step_events(index, position, &step)? {
                    emit(value);
                }
            }
            Err(StageFailure::Refused(refusal)) => {
                refused += 1;
                emit(session_refusal_event(index, position, &refusal)?);
            }
            // FR-853: a step the cancellation interrupts is atomic: the
            // session is unchanged, and the record is not counted as read.
            Err(StageFailure::Cancelled(cause)) => {
                read -= 1;
                stopped = Some(cause);
                break;
            }
            Err(other) => return Err(cause_of(other)),
        }
    }
    if stopped.is_none() {
        stopped = cancel.cause();
    }
    let outcome = match stopped {
        Some(cause) => session.cancelled(cause),
        None => session.finish(cancel),
    }
    .map_err(cause_of)?;
    let mut clauses = Vec::new();
    let mut codes = Vec::new();
    for (name, (record, settlement)) in names
        .iter()
        .zip(outcome.records.iter().zip(&outcome.settlements))
    {
        let status = ClauseStatus::Pending(record.clone());
        let (disposition, basis, _) = report_value(&status)?;
        codes.push(record.category.exit_code());
        clauses.push(json!({
            "clause": name,
            "settled": !matches!(
                settlement.trigger,
                SettleTrigger::Finish | SettleTrigger::Cancelled(_)
            ),
            "category": disposition.get("category").cloned().unwrap_or(Value::Null),
            "basis": basis,
            "disposition": disposition,
            "settlement": settlement_value(settlement),
        }));
    }
    let ledger = match &outcome.ledger {
        Some(ledger) => {
            write_ledger(ledger).map_err(|error| invalid(&error.member, error.reason))?
        }
        None => Value::Null,
    };
    let lifecycle = outcome.document().unwrap_or_else(|_| {
        fsl_replay::OutcomeDocument::new(
            fsl_replay::Operation::Monitor,
            None,
            filament_semantic_value::catalog::Category::InternalFailure,
        )
    });
    let mut value = json!({
        "format": SUMMARY_FORMAT,
        "clauses": clauses,
        "records": read,
        "refused": refused,
        "ledger": ledger,
    });
    if let (Some(cause), Some(members)) = (stopped, value.as_object_mut()) {
        members.insert("cancelled".to_owned(), json!(cancel_cause(cause)));
    }
    Ok(MonitorSummary {
        value,
        outcome: lifecycle,
        exit_code: severity_exit(codes.into_iter()),
    })
}

fn step_events(record: usize, position: Option<u64>, step: &MonitorStep) -> Result<Vec<Value>> {
    step.statuses
        .iter()
        .filter(|(name, _)| step.changed.contains(name))
        .map(|(name, status)| event(record, position, name, status))
        .collect()
}

fn file_lines(intake: &mut super::Intake<'_>, path: Option<&str>) -> Result<Vec<Result<Vec<u8>>>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let bytes = intake.file(path, intake.remaining)?;
    Ok(bytes
        .split(|byte| *byte == b'\n')
        .map(|line| Ok(line.to_vec()))
        .collect())
}

/// How `command::monitor` settles a request's provider-routed `items`
/// (FR-644 "Provider-routed items", FR-635 "One rule for every
/// provider-routing entry"): the caller's negotiator, the started process
/// providers and the run's provider limits. An empty `providers` is the
/// entry with no plugin configured.
pub struct MonitorProviders<'a> {
    /// The caller's negotiator, the one `analyze_with_providers` and
    /// `run_with_providers` take (FR-961).
    pub negotiate: &'a dyn Negotiate,
    /// The driver's started process providers, each its `Process`
    /// descriptor and transport.
    pub providers: Vec<Engine>,
    /// The run's FSpec FR-462 provider limits (FR-964).
    pub options: &'a ProviderOptions,
}

/// FR-644: run the `native-monitor/1` request at `path` over `records`,
/// handing each event to `sink` before the next record is read. It writes
/// nothing to the process streams.
///
/// FR-635 "One rule for every provider-routing entry": the registry is
/// built from `Engines::fsl()` and `routing`'s providers, under its
/// provider limits, before the request is read. The request's `items`, and
/// never its monitored `clauses`, settle over that registry as
/// `analyze_with_providers` settles them, negotiated by `routing`'s
/// negotiator under `cancel`, after the program compiles and before the
/// first record is read; the summary then carries their records and the
/// registry. A request without `items` calls neither the negotiator nor any
/// provider.
///
/// # Errors
///
/// `invalid-request` naming `providers` when a provider repeats a registered
/// `BackendId`, before the request is read; the command-error envelope's
/// cause for a request-file defect, a compile refusal, a whole-request item
/// refusal (FR-624), an `open` or `resume` refusal, before any record is
/// read.
pub fn monitor(
    path: &Path,
    records: impl Iterator<Item = std::io::Result<Vec<u8>>>,
    sink: &mut dyn FnMut(&MonitorEvent),
    routing: MonitorProviders<'_>,
    limits: RunLimits,
    cancel: &Cancel,
) -> std::result::Result<MonitorSummary, Box<RunError>> {
    let MonitorProviders {
        negotiate,
        providers,
        options,
    } = routing;
    let engines = super::analyze::engines_with(providers, options).map_err(|cause| {
        Box::new(RunError {
            cause,
            sources: crate::Sources::default(),
        })
    })?;
    let settle = Settle {
        negotiate,
        engines: &engines,
        cancel,
    };
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        drive(
            directory,
            bytes,
            limits.dependent_bytes,
            Records::Stream(Box::new(records.map(|line| {
                line.map_err(|error| RunCause::Io {
                    path: path.to_path_buf(),
                    error,
                })
            }))),
            sink,
            &settle,
            cancel,
        )
    })
}

/// FR-644 batch form: a `native-monitor/1` request whose `records` file is the
/// whole stream, run to its end as one `filament.monitor-run/1` document. Its
/// `items` settle through the run's registry and negotiator (`settle`).
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    settle: &Settle<'_>,
) -> Result<RunResult> {
    let mut events = Vec::new();
    let summary = drive(
        directory,
        bytes,
        dependent_bytes,
        Records::Batch,
        &mut |event| events.push(event.as_value().clone()),
        settle,
        &Cancel::new(),
    )?;
    Ok(RunResult {
        exit_code: summary.exit_code,
        value: NativeResult::monitor_document(json!({
            "format": RUN_FORMAT,
            "events": events,
            "summary": summary.value,
        })),
        outcome: Some(summary.outcome),
        requirements: None,
        sources: crate::Sources::default(),
    })
}

/// Whether `bytes` is an envelope of the `native-monitor/1` format.
pub(super) fn is_monitor_request(bytes: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct Peek {
        format: String,
    }
    serde_json::from_slice::<Peek>(bytes).is_ok_and(|peek| {
        peek.format == fsl_foundation::wire_format::WireFormat::MonitorRequest.as_str()
    })
}

/// Run `work` under a fresh handle that `outer`'s cancellation reaches, with
/// its cause, while `work` runs; cancelling the fresh handle leaves `outer`
/// live. A monitor's items settle under it (FR-644 "Provider-routed items"),
/// so their deadline stops the items and never the stream.
fn chained<T>(outer: &Cancel, work: impl FnOnce(&Cancel) -> T) -> T {
    /// How often the forwarding thread reads `outer`.
    const POLL: std::time::Duration = std::time::Duration::from_millis(5);
    let inner = Cancel::new();
    if let Some(cause) = outer.cause() {
        inner.cancel(cause);
    }
    std::thread::scope(|scope| {
        let (done, finished) = std::sync::mpsc::channel::<()>();
        let (watched, forwarded) = (outer.clone(), inner.clone());
        scope.spawn(move || loop {
            if let Some(cause) = watched.cause() {
                forwarded.cancel(cause);
                break;
            }
            // The thread ends on `work`'s end, or when the channel closes.
            if !matches!(
                finished.recv_timeout(POLL),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ) {
                break;
            }
        });
        let value = work(&inner);
        // A closed channel also ends the thread.
        let _ = done.send(());
        value
    })
}

#[cfg(test)]
mod tests {
    use super::severity_exit;
    use ix_trace_rs::trace;

    #[trace("TC-769", "FR-285-AC-2")]
    #[test]
    fn monitor_exit_uses_the_catalog_severity_order() {
        for (codes, expected) in [([10, 22], 22), ([20, 21], 20)] {
            assert_eq!(severity_exit(codes.into_iter()), expected);
            assert_eq!(severity_exit(codes.into_iter().rev()), expected);
        }
    }
}
