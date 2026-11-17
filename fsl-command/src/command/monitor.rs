// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-644: the `native-monitor/1` request and the record stream it runs. The
//! command compiles the program once, opens FR-643's [`MonitorSession`] and
//! hands the caller one event per clause status change, then a summary.
//! Nothing here writes to the process streams.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use filament_exact::Cancel;
use fsl_analyze::monitor::{
    MonitorClause, MonitorLimits, MonitorRefusal, SettleTrigger, Settlement,
};
use fsl_analyze::monitor_session::{
    ClauseStatus, MonitorObservation, MonitorOpen, MonitorPosition, MonitorSession, MonitorStamp,
    MonitorState, MonitorStep, MonitorTraceKind,
};
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::source::Source;
use fsl_replay::spine::{
    read_cover, read_ledger, read_lower_boundary, read_progress_assertion, read_sample_text,
    read_trace_binding, write_clause_run_result, write_ledger, OverObject, ProgressLowerBoundary,
    SpineLimits, TimestampedOrderKey,
};
use serde_json::{json, Value};

use super::{
    check_sources, intake_with_bytes, request, require_complete_edition, wire, FileCounts,
    NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
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

/// FR-285's code of the most severe category, in the order 30, 20, 10, 11,
/// 21, 22, then 0.
fn severity_exit(codes: impl Iterator<Item = u8>) -> u8 {
    const ORDER: [u8; 6] = [30, 20, 10, 11, 21, 22];
    codes
        .filter_map(|code| ORDER.iter().position(|ranked| *ranked == code))
        .min()
        .and_then(|rank| ORDER.get(rank).copied())
        .unwrap_or(0)
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
    compile: wire::CompileRequest,
}

fn read_inputs(intake: &mut super::Intake<'_>, request: wire::MonitorRequest) -> Result<Inputs> {
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
        compile: wire::CompileRequest {
            models: request.models,
            program: wire::CompileProgram {
                sources: Vec::new(),
                extraction: false,
            },
            refinement_internals: None,
            libraries: request.libraries,
            extensions: Vec::new(),
            clocks: request.clocks,
            output: None,
            handoff: None,
        },
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
    batch: bool,
    source: Records<'_>,
    sink: &mut dyn FnMut(&MonitorEvent),
    cancel: &Cancel,
) -> Result<MonitorSummary> {
    let request: wire::MonitorRequest = request(bytes)?;
    let records_file = request.records.clone();
    match (batch, &records_file) {
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
            observations: request.observations.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let program = intake.source(&request.program.source, "program.source")?;
    require_complete_edition(program.source())?;
    let records: Box<dyn Iterator<Item = Result<Vec<u8>>> + '_> = match source {
        Records::Batch => Box::new(file_lines(&mut intake, records_file.as_deref())?.into_iter()),
        Records::Stream(records) => records,
    };
    let inputs = read_inputs(&mut intake, request)?;
    let compile = &inputs.compile;
    check_sources(
        compile,
        &mut intake,
        std::slice::from_ref(&program),
        false,
        SpineLimits::default(),
        None,
        None,
        |checked, selections, _, packages, libraries, _, _, _, _| {
            let mut sources: Vec<Source> = vec![program.source().clone()];
            sources.extend(libraries.iter().map(|library| library.source().clone()));
            let limits = MonitorLimits {
                observation: Default::default(),
                normalization: SpineLimits::default().model,
                accounting: super::call_accounting(&wire::Accounting::default()),
                max_window_values: fsl_analyze::monitor::DEFAULT_MAX_WINDOW_VALUES,
                retained_positions: fsl_analyze::monitor::DEFAULT_RETAINED_POSITIONS,
                retained_identities: fsl_analyze::monitor::DEFAULT_RETAINED_IDENTITIES,
            };
            let open = MonitorOpen {
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
            let session = match &inputs.state {
                Some(state) => MonitorSession::resume(package, state, open, &limits, cancel),
                None => MonitorSession::open(package, open, &limits, cancel),
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
    for (index, line) in records.enumerate() {
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
            Err(other) => return Err(cause_of(other)),
        }
    }
    let outcome = session.finish(cancel).map_err(cause_of)?;
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
            "settled": settlement.trigger != SettleTrigger::Finish,
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
    let value = json!({
        "format": SUMMARY_FORMAT,
        "clauses": clauses,
        "records": read,
        "refused": refused,
        "ledger": ledger,
    });
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

/// FR-644: run the `native-monitor/1` request at `path` over `records`,
/// handing each event to `sink` before the next record is read. It writes
/// nothing to the process streams.
///
/// # Errors
///
/// The command-error envelope's cause for a request-file defect, a compile
/// refusal, an `open` or `resume` refusal, before any record is read.
pub fn monitor(
    path: &Path,
    records: impl Iterator<Item = std::io::Result<Vec<u8>>>,
    sink: &mut dyn FnMut(&MonitorEvent),
    limits: RunLimits,
    cancel: &Cancel,
) -> std::result::Result<MonitorSummary, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        drive(
            directory,
            bytes,
            limits.dependent_bytes,
            false,
            Records::Stream(Box::new(records.map(|line| {
                line.map_err(|error| RunCause::Io {
                    path: path.to_path_buf(),
                    error,
                })
            }))),
            sink,
            cancel,
        )
    })
}

/// FR-644 batch form: a `native-monitor/1` request whose `records` file is the
/// whole stream, run to its end as one `filament.monitor-run/1` document.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
) -> Result<RunResult> {
    let mut events = Vec::new();
    let summary = drive(
        directory,
        bytes,
        dependent_bytes,
        true,
        Records::Batch,
        &mut |event| events.push(event.as_value().clone()),
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
