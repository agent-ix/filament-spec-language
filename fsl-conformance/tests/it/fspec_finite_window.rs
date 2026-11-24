// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-to-verdict execution of the standard's finite-window vectors
//! (QSL-2048): every entry of `finite-window-vectors.json` (window, order,
//! counter, covered, admit) and the tick-cover vectors it points at is
//! compiled from FSL source and run, and its outcome is compared with the
//! fixture. A case the compiler does not answer as the fixture says fails
//! unless the checked-in registry keys it to a ticket.

use std::collections::BTreeSet;

use filament_exact::{Cancel, ScalarLimits};
use fsl_eval::temporal::{ClockCover, Containment};
use fsl_foundation::{Source, SourceIdentity};
use fsl_package::{ClockConfiguration, TemporalClockSelection};
use fsl_replay::spine::{
    self, ClauseDisposition, DependencyInput, PackageAuthentication, PackageTemporalRequest,
    ParseRequest, ProgressAssertion, ProgressClosure, ProgressCompleteness, SpineLimits,
    StateClauseExecutionPoints, TemporalIndeterminateReason, TemporalTraceRefusal,
    TemporalVerdict, TimestampedCover, TimestampedMissingValuation, TimestampedObservation,
    TimestampedOrderKey, TimestampedWindow, TraceBinding, TraceInput,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{AdmissionFailure, DocumentProvision, ObservationLimits};
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::fspec_standard::{self, Settled};

const RUNNER: &str = "finite-window";
const PROFILE: &str = "filament.temporal.timestamped-event.finite-window/v1";
const FIXTURE: &str = "proposals/finite-window/finite-window-vectors.json";
const TICK_COVER: &str = "proposals/checked-package-v2/tick-cover-vectors.json";
const ACCOUNTING: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    outcome: String,
    #[serde(rename = "where")]
    at: Option<u64>,
    deadline: Option<u64>,
    basis: Option<String>,
    reason: Option<String>,
    position: Option<u64>,
    support: Option<Vec<(u64, String)>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    id: String,
    row: u64,
    op: String,
    a: u64,
    b: u64,
    activate: String,
    positions: Vec<(u64, String)>,
    w: u64,
    complete: Option<u64>,
    scope: String,
    execution: Option<String>,
    lower: Option<String>,
    uncertainty: Option<u64>,
    note: Option<String>,
    listing_twin: Option<String>,
    expected: Expected,
}

type OrderKey = Option<(String, i64)>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Order {
    id: String,
    row: u64,
    op: String,
    positions: Vec<(u64, OrderKey)>,
    expected: String,
    refused_at: Option<u64>,
    same_order_as: Option<String>,
    canonical_order: Option<Vec<OrderKey>>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Counter {
    id: String,
    row: u64,
    w: u64,
    complete: u64,
    cover: String,
    uncertainty: Option<u64>,
    bound: u64,
    request_tick: u64,
    ack_tick: u64,
    expected: String,
    reason: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Covered {
    id: String,
    w: u64,
    stamp: u64,
    expected: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Admit {
    id: String,
    row: u64,
    stamps: Vec<u64>,
    expected: String,
    note: Option<String>,
}

struct Fixtures {
    windows: Vec<Window>,
    orders: Vec<Order>,
    counters: Vec<Counter>,
    covered: Vec<Covered>,
    admits: Vec<Admit>,
    tick_cover: Vec<Value>,
    /// Every entry id of the groups above, to count against the file.
    total: usize,
}

impl Fixtures {
    fn executed(&self) -> usize {
        self.windows.len()
            + self.orders.len()
            + self.counters.len()
            + self.covered.len()
            + self.admits.len()
    }
}

fn group<T: serde::de::DeserializeOwned>(document: &Value, name: &str) -> Result<Vec<T>, String> {
    serde_json::from_value(document[name].clone())
        .map_err(|error| format!("{FIXTURE}/{name}: {error}"))
}

fn fixtures_from(bytes: Option<&[u8]>, tick_cover: Option<&[u8]>) -> Result<Fixtures, String> {
    let bytes = bytes.ok_or_else(|| format!("no {FIXTURE} to read"))?;
    let document: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let windows: Vec<Window> = group(&document, "window")?;
    if windows.is_empty() {
        return Err("finite-window fixture has no window cases".to_owned());
    }
    let mut ids = BTreeSet::new();
    for (_, entries) in document.as_object().ok_or("fixture must be an object")? {
        if let Some(entries) = entries.as_array() {
            for entry in entries {
                let id = entry["id"]
                    .as_str()
                    .ok_or("each fixture case needs an id")?;
                if !ids.insert(id.to_owned()) {
                    return Err(format!("duplicate finite-window case {id}"));
                }
            }
        }
    }
    let tick_cover = match tick_cover {
        Some(bytes) => {
            let document: Value =
                serde_json::from_slice(bytes).map_err(|error| format!("{TICK_COVER}: {error}"))?;
            let cases: Vec<Value> = serde_json::from_value(document["cases"].clone())
                .map_err(|error| format!("{TICK_COVER}/cases: {error}"))?;
            cases
        }
        None => return Err(format!("no {TICK_COVER} to read")),
    };
    let named = &document["tick_cover_vectors"]["ids"];
    let named: Vec<&str> = named
        .as_array()
        .ok_or("tick_cover_vectors names no ids")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let have: Vec<&str> = tick_cover.iter().filter_map(|case| case["id"].as_str()).collect();
    if named != have {
        return Err(format!(
            "the fixture names tick-cover vectors {named:?}, the tick-cover file holds {have:?}"
        ));
    }
    let fixtures = Fixtures {
        windows,
        orders: group(&document, "order")?,
        counters: group(&document, "counter")?,
        covered: group(&document, "covered")?,
        admits: group(&document, "admit")?,
        tick_cover,
        total: ids.len(),
    };
    if fixtures.executed() != fixtures.total {
        return Err(format!(
            "the fixture lists {} entries and this runner reads {}: a group has no reader",
            fixtures.total,
            fixtures.executed()
        ));
    }
    Ok(fixtures)
}

fn load() -> Result<Fixtures, String> {
    let main = fspec_standard::read(FIXTURE)?;
    let ticks = fspec_standard::read(TICK_COVER)?;
    fixtures_from(Some(&main), Some(&ticks))
}

/// One observation of a source scenario.
struct Reading {
    stamp: u64,
    p: bool,
    q: bool,
    key: Option<TimestampedOrderKey>,
    p_missing: bool,
}

/// A clause over `Reading` and the trace it is run against.
struct Scenario {
    id: String,
    activation: String,
    body: String,
    readings: Vec<Reading>,
    watermark: u64,
    complete: Option<u64>,
    scope: ProgressClosure,
    execution: ProgressClosure,
    authoritative_origin: bool,
    cover: Option<TimestampedCover>,
}

struct Compiled {
    unit: spine::CheckedUnit,
    packages: PackageInput,
    sources: Vec<Source>,
    selections: Vec<TemporalClockSelection>,
}

fn compile(scenario: &Scenario) -> Result<Compiled, String> {
    let id = &scenario.id;
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{PROFILE}\";\n\
         record Reading {{ p: Boolean; q: Boolean; serial: Integer; }}\n\
         temporal Window using t over (r: Reading) clock \"events\" on {} {{ {} }}\n",
        scenario.activation, scenario.body
    );
    let identity = SourceIdentity::new("fspec", id);
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "finite-window.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|error| format!("{id}: parse: {error:?}"))?
    .into_value();
    let cst = fsl_cst::parse(
        identity,
        "finite-window.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .map_err(|error| format!("{id}: CST: {error:?}"))?;
    let forms = fsl_forms::build_unit(&cst).map_err(|error| format!("{id}: forms: {error:?}"))?;
    let selections = forms
        .forms()
        .iter()
        .filter(|form| matches!(form.form(), fsl_forms::DeclarationForm::Temporal(_)))
        .map(|form| {
            TemporalClockSelection::new(
                cst.source().reference().clone(),
                form.span(),
                ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ms".into(),
                },
            )
        })
        .collect::<Vec<_>>();
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .map_err(|error| format!("{id}: packages: {error:?}"))?;
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(|error| format!("{id}: select: {error:?}"))?
        .into_value();
    let unit = spine::check_with_temporal_selections_and_execution_points(
        &parsed,
        id.clone(),
        &StateClauseExecutionPoints::new(),
        &selections,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("{id}: check: {error:?}"))?
    .into_value();
    let mut sources = vec![unit.source().clone()];
    sources.extend(unit.libraries().iter().cloned());
    Ok(Compiled {
        unit,
        packages,
        sources,
        selections,
    })
}

fn closure(value: &str) -> Result<ProgressClosure, String> {
    match value {
        "closed" => Ok(ProgressClosure::Closed),
        "open" => Ok(ProgressClosure::Open),
        other => Err(format!("unknown closure {other}")),
    }
}

fn run(scenario: &Scenario) -> Result<spine::ClauseRunReport, String> {
    let id = &scenario.id;
    let compiled = compile(scenario)?;
    let cancel = Cancel::new();
    let authentication = spine::authenticate_emission(
        compiled.unit.package(),
        &compiled.selections,
        &compiled.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .map_err(|error| format!("{id}: compile/authenticate: {error:?}"))?;
    let mut observations = Vec::new();
    let mut missing = Vec::new();
    for (index, reading) in scenario.readings.iter().enumerate() {
        if reading.p_missing {
            missing.push(TimestampedMissingValuation {
                position: u64::try_from(index).map_err(|error| error.to_string())?,
                atom: 1,
            });
        }
        let sample = json!({
            "p": {"boolean": reading.p},
            "q": {"boolean": reading.q},
            "serial": {"integer": index.to_string()},
        })
        .to_string();
        observations.push(TimestampedObservation {
            document: spine::sample_position(&sample),
            stamp: reading.stamp,
            order_key: reading.key.clone(),
        });
    }
    let trace = TraceInput::Timestamped {
        observations,
        progress: ProgressAssertion {
            watermark: scenario.watermark,
            completeness: scenario
                .complete
                .map_or(ProgressCompleteness::Incomplete, |through| {
                    ProgressCompleteness::Complete { through }
                }),
            scope: scenario.scope,
            execution: scenario.execution,
        },
        window: Box::new(TimestampedWindow {
            binding: Some(TraceBinding {
                profile: PROFILE.to_owned(),
                clock_name: "events".to_owned(),
                parameters: ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ms".into(),
                },
            }),
            authoritative_origin: scenario.authoritative_origin,
            cover: scenario.cover.clone(),
            missing_valuations: missing,
            late: Default::default(),
        }),
    };
    Ok(spine::run_package_temporal(&PackageTemporalRequest {
        package: compiled.unit.package(),
        packages: &compiled.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: ACCOUNTING,
        name: "Window",
        trace: &trace,
        over: None,
        ledger: None,
        sources: &compiled.sources,
        authentication: Some(PackageAuthentication::temporal(&authentication)),
        cancel: &cancel,
    }))
}

fn window_scenario(case: &Window) -> Result<Scenario, String> {
    let operator = match case.op.as_str() {
        "always" => "always",
        "eventually" => "eventually",
        "once" => "once",
        other => {
            return Err(format!(
                "{}: no source operator adapter for {other}",
                case.id
            ))
        }
    };
    let activation = match case.activate.as_str() {
        "origin" => "origin",
        "each" => "each (q: Reading)",
        other => return Err(format!("{}: no activation adapter for {other}", case.id)),
    };
    let mut readings = Vec::new();
    for (stamp, atom) in &case.positions {
        let (p, p_missing) = match atom.as_str() {
            "T" => (true, false),
            "F" => (false, false),
            "absent" => (false, true),
            other => return Err(format!("{}: unknown atom valuation {other}", case.id)),
        };
        readings.push(Reading {
            stamp: *stamp,
            p,
            q: false,
            key: None,
            p_missing,
        });
    }
    Ok(Scenario {
        id: case.id.clone(),
        activation: activation.to_owned(),
        body: format!("{operator}[{},{}] holds(r.p)", case.a, case.b),
        readings,
        watermark: case.w,
        complete: case.complete,
        scope: closure(&case.scope)?,
        execution: closure(case.execution.as_deref().unwrap_or("closed"))?,
        authoritative_origin: match case.lower.as_deref() {
            None | Some("AuthoritativeOrigin") => true,
            Some("HistoryCutoff") => false,
            Some(other) => return Err(format!("{}: unknown lower boundary {other}", case.id)),
        },
        cover: case
            .uncertainty
            .map(|uncertainty| TimestampedCover::Counter { uncertainty }),
    })
}

fn run_window(case: &Window) -> Result<spine::ClauseRunReport, String> {
    run(&window_scenario(case)?)
}

fn compare(case: &Window, report: &spine::ClauseRunReport) -> Result<(), String> {
    let expected = &case.expected;
    let (outcome, at, deadline, reason, position) = match &report.disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Holds) => ("true", None, None, None, None),
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) => {
            ("false", Some(*at), None, None, None)
        }
        ClauseDisposition::Temporal(TemporalVerdict::PendingWindow { at, deadline }) => {
            ("pending", Some(*at), Some(*deadline), None, None)
        }
        ClauseDisposition::Temporal(TemporalVerdict::Indeterminate { at, reason }) => {
            let (name, position) = match reason {
                TemporalIndeterminateReason::MissingValuation { position, atom, .. } => {
                    if atom != "holds(r.p)" {
                        return Err(format!(
                            "{}: missing valuation names {atom:?}, expected holds(r.p)",
                            case.id
                        ));
                    }
                    ("MissingValuation", Some(*position))
                }
                TemporalIndeterminateReason::ConflictingValuation { position, atom } => {
                    if atom != "holds(r.p)" {
                        return Err(format!(
                            "{}: conflicting valuation names {atom:?}, expected holds(r.p)",
                            case.id
                        ));
                    }
                    ("ConflictingValuation", Some(*position))
                }
                TemporalIndeterminateReason::IncompleteInput => ("IncompleteInput", None),
                TemporalIndeterminateReason::MissingHistory => ("MissingHistory", None),
                TemporalIndeterminateReason::StraddlesBound => ("StraddlesBound", None),
            };
            ("indeterminate", Some(*at), None, Some(name), position)
        }
        other => {
            return Err(format!(
                "{}: expected {}, actual {other:?}",
                case.id, expected.outcome
            ))
        }
    };
    let mismatch = |field: &str, wanted: &dyn std::fmt::Debug, actual: &dyn std::fmt::Debug| {
        Err(format!(
            "{}: {field}: expected {wanted:?}, actual {actual:?}",
            case.id
        ))
    };
    if outcome != expected.outcome {
        return mismatch("outcome", &expected.outcome, &outcome);
    }
    if expected.at.is_some() && at != expected.at {
        return mismatch("activation", &expected.at, &at);
    }
    if deadline != expected.deadline {
        return mismatch("deadline", &expected.deadline, &deadline);
    }
    if reason != expected.reason.as_deref() {
        return mismatch("reason", &expected.reason, &reason);
    }
    if position != expected.position {
        return mismatch("position", &expected.position, &position);
    }
    let window = report
        .finite_window
        .as_ref()
        .ok_or_else(|| format!("{}: missing finite-window report", case.id))?;
    if let Some(expected) = &expected.basis {
        if window.basis.as_str() != expected {
            return mismatch("basis", expected, &window.basis);
        }
    }
    if let Some(expected) = &expected.support {
        let translated = expected
            .iter()
            .map(|(position, atom)| {
                if atom != "p" {
                    return Err(format!("{}: unsupported expected atom {atom}", case.id));
                }
                Ok((*position, "holds(r.p)".to_owned()))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let actual = window
            .support
            .iter()
            .map(|item| (item.position, item.atom.clone()))
            .collect::<Vec<_>>();
        if actual != translated {
            return mismatch("support", &translated, &actual);
        }
    }
    Ok(())
}

fn settled(result: Result<(), String>) -> Settled {
    match result {
        Ok(()) => Settled::Matched,
        Err(error) => Settled::Mismatch(error),
    }
}

/// The verdict word of a counter, covered or admit run, in the fixture's
/// spelling.
fn verdict_word(disposition: &ClauseDisposition) -> String {
    match disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Holds) => "true".to_owned(),
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. }) => "false".to_owned(),
        ClauseDisposition::Temporal(TemporalVerdict::PendingWindow { .. }) => "pending".to_owned(),
        ClauseDisposition::Temporal(TemporalVerdict::Indeterminate { reason, .. }) => {
            format!("indeterminate/{reason:?}")
        }
        other => format!("{other:?}"),
    }
}

fn reading(stamp: u64, p: bool, q: bool) -> Reading {
    Reading {
        stamp,
        p,
        q,
        key: None,
        p_missing: false,
    }
}

/// FR-501 rows 20 and 36: a request at tick 0 and an acknowledgement at
/// `ack_tick`, with a bound of `bound` ticks, under the fixture's cover.
fn counter_scenario(
    id: &str,
    bound: u64,
    request: u64,
    ack: u64,
    through: u64,
    cover: Option<TimestampedCover>,
) -> Scenario {
    Scenario {
        id: id.to_owned(),
        activation: "origin".to_owned(),
        body: format!("holds(r.p) implies eventually[0,{bound}] holds(r.q)"),
        readings: if request == ack {
            vec![reading(request, true, true)]
        } else {
            vec![reading(request, true, false), reading(ack, false, true)]
        },
        watermark: through,
        complete: Some(through),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover,
    }
}

fn settle_counter(case: &Counter) -> Settled {
    let cover = match (case.cover.as_str(), case.uncertainty) {
        ("counter", Some(uncertainty)) => Some(TimestampedCover::Counter { uncertainty }),
        ("exact", None) => Some(TimestampedCover::Exact),
        ("omitted", None) => None,
        (other, uncertainty) => {
            return Settled::Unsupported(format!(
                "{}: no cover adapter for {other} with uncertainty {uncertainty:?}",
                case.id
            ))
        }
    };
    let scenario = counter_scenario(
        &case.id,
        case.bound,
        case.request_tick,
        case.ack_tick,
        case.complete,
        cover,
    );
    let _context = (case.row, case.w, &case.note);
    let report = match run(&scenario) {
        Ok(report) => report,
        Err(error) => return Settled::Mismatch(error),
    };
    let actual = verdict_word(&report.disposition);
    let wanted = match &case.reason {
        Some(reason) => format!("{}/{reason}", case.expected),
        None => case.expected.clone(),
    };
    if actual == wanted {
        Settled::Matched
    } else {
        Settled::Mismatch(format!(
            "{}: expected {wanted:?}, actual {actual:?}",
            case.id
        ))
    }
}

/// FR-094-AC-11: a progress watermark `W` covers an instant stamped `W` and
/// not one stamped `W + 1`. The clause `always[0,stamp] holds(r.p)` over
/// readings that all satisfy `p` holds exactly when the watermark, which
/// the completeness assertion matches, covers the last instant.
fn settle_covered(case: &Covered) -> Settled {
    let mut readings = vec![reading(case.stamp, true, false)];
    if case.stamp > 0 {
        readings.insert(0, reading(0, true, false));
    }
    let scenario = Scenario {
        id: case.id.clone(),
        activation: "origin".to_owned(),
        body: format!("always[0,{}] holds(r.p)", case.stamp),
        readings,
        watermark: case.w,
        complete: Some(case.w),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover: None,
    };
    let report = match run(&scenario) {
        Ok(report) => report,
        Err(error) => return Settled::Mismatch(error),
    };
    let covered = matches!(
        report.disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Holds)
    );
    if covered == case.expected {
        Settled::Matched
    } else {
        Settled::Mismatch(format!(
            "{}: expected covered {}, actual {covered} ({})",
            case.id,
            case.expected,
            verdict_word(&report.disposition)
        ))
    }
}

/// FR-501 row 31: a trace with no instants is refused; one whose stamps are
/// listed out of order is read in canonical order.
fn settle_admit(case: &Admit) -> Settled {
    let _context = (case.row, &case.note);
    let readings: Vec<Reading> = case
        .stamps
        .iter()
        .map(|stamp| reading(*stamp, false, false))
        .collect();
    let top = case.stamps.iter().copied().max().unwrap_or(0);
    let scenario = Scenario {
        id: case.id.clone(),
        activation: "origin".to_owned(),
        body: "eventually[0,5] holds(r.p)".to_owned(),
        readings,
        watermark: top,
        complete: Some(top),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover: None,
    };
    let report = match run(&scenario) {
        Ok(report) => report,
        Err(error) => return Settled::Mismatch(error),
    };
    let refused = matches!(
        report.disposition,
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::EmptyTrace)
            | ClauseDisposition::Admit(AdmissionFailure::Refused(_))
    );
    let evaluated = matches!(report.disposition, ClauseDisposition::Temporal(_));
    match (case.expected.as_str(), refused, evaluated) {
        ("refuse", true, _) | ("evaluate", _, true) => Settled::Matched,
        (wanted, _, _) => Settled::Mismatch(format!(
            "{}: expected {wanted}, actual {:?}",
            case.id, report.disposition
        )),
    }
}

fn order_scenario(case: &Order, positions: &[(u64, OrderKey)], witness: Option<usize>) -> Scenario {
    let (activation, body) = match case.op.as_str() {
        "until" | "release" => ("origin", format!("holds(r.p) {}[0,1] holds(r.q)", case.op)),
        "since" | "triggered" => (
            "each (q: Reading)",
            format!("holds(r.p) {}[0,1] holds(r.q)", case.op),
        ),
        other => ("origin", format!("{other}[0,5] holds(r.p)")),
    };
    let (activation, body) = if witness.is_some() {
        ("origin", "eventually[0,5] holds(r.p)".to_owned())
    } else {
        (activation, body)
    };
    let top = positions.iter().map(|(stamp, _)| *stamp).max().unwrap_or(0);
    let top = if witness.is_some() { top + 6 } else { top };
    Scenario {
        id: case.id.clone(),
        activation: activation.to_owned(),
        body,
        readings: positions
            .iter()
            .enumerate()
            .map(|(index, (stamp, key))| Reading {
                stamp: *stamp,
                p: false,
                q: false,
                key: key.as_ref().map(|(authority, key)| TimestampedOrderKey {
                    authority: authority.clone(),
                    key: *key,
                }),
                p_missing: witness == Some(index),
            })
            .collect(),
        watermark: top,
        complete: Some(top),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover: None,
    }
}

/// FR-501 rows 17, 18, 32, 35 and 40: an order-sensitive operator needs one
/// usable order among observations of one instant; an insensitive one does
/// not. `same_order_as` names a twin whose result must be identical, and
/// `canonical_order` the reading order the evaluator must use, which a
/// missing valuation placed at each listed position makes visible in the
/// position the indeterminate verdict reports.
fn settle_order(case: &Order, all: &[Order]) -> Settled {
    let _context = (case.row, &case.note);
    let report = match run(&order_scenario(case, &case.positions, None)) {
        Ok(report) => report,
        Err(error) => return Settled::Mismatch(error),
    };
    match case.expected.as_str() {
        "refuse" => {
            let wanted = case.refused_at;
            match &report.disposition {
                ClauseDisposition::TemporalRefusal(
                    TemporalTraceRefusal::SameStampOrderUnavailable { position },
                ) if Some(*position) == wanted => {}
                other => {
                    return Settled::Mismatch(format!(
                        "{}: expected a refusal at {wanted:?}, actual {other:?}",
                        case.id
                    ))
                }
            }
        }
        "evaluate" => {
            if !matches!(report.disposition, ClauseDisposition::Temporal(_)) {
                return Settled::Mismatch(format!(
                    "{}: expected an evaluation, actual {:?}",
                    case.id, report.disposition
                ));
            }
        }
        other => return Settled::Unsupported(format!("{}: unknown expectation {other}", case.id)),
    }
    if let Some(twin) = &case.same_order_as {
        let Some(twin) = all.iter().find(|candidate| &candidate.id == twin) else {
            return Settled::Unsupported(format!("{}: twin {twin} is not in the file", case.id));
        };
        let twin_report = match run(&order_scenario(twin, &twin.positions, None)) {
            Ok(report) => report,
            Err(error) => return Settled::Mismatch(error),
        };
        if format!("{:?}", twin_report.disposition) != format!("{:?}", report.disposition) {
            return Settled::Mismatch(format!(
                "{}: same order as {} but {:?} against {:?}",
                case.id, twin.id, report.disposition, twin_report.disposition
            ));
        }
    }
    if let Some(canonical) = &case.canonical_order {
        for (listed, key) in case.positions.iter().map(|(_, key)| key).enumerate() {
            let Some(rank) = canonical.iter().position(|candidate| candidate == key) else {
                return Settled::Unsupported(format!(
                    "{}: canonical order omits listed position {listed}",
                    case.id
                ));
            };
            let probe = match run(&order_scenario(case, &case.positions, Some(listed))) {
                Ok(report) => report,
                Err(error) => return Settled::Mismatch(error),
            };
            let support: Vec<u64> = match &probe.disposition {
                ClauseDisposition::Temporal(TemporalVerdict::Indeterminate {
                    reason: TemporalIndeterminateReason::MissingValuation { position, .. },
                    ..
                }) => vec![*position],
                other => {
                    return Settled::Mismatch(format!("{}: expected a missing valuation, actual {other:?}", case.id))
                }
            };
            let wanted = u64::try_from(rank).unwrap_or(u64::MAX);
            if support != [wanted] {
                return Settled::Mismatch(format!(
                    "{}: the reading listed at {listed} should be canonical position {wanted}, reported position {support:?}",
                    case.id
                ));
            }
        }
    }
    Settled::Matched
}

/// The TCV vectors: `within_bound` through source where the bound is a
/// non-negative tick count and the request and acknowledgement are distinct
/// readings (one instant carries one reading), the comparison functions
/// otherwise.
fn settle_tick_cover(case: &Value) -> (String, Settled) {
    let id = case["id"].as_str().unwrap_or("TCV-?").to_owned();
    let uncertainty = case["uncertainty"].as_u64().unwrap_or(0);
    let cover = ClockCover::Counter { uncertainty };
    let settled = match case["kind"].as_str() {
        Some("within_bound") => {
            let (Some(bound), Some(request), Some(ack)) = (
                case["bound"].as_i64(),
                case["request_tick"].as_u64(),
                case["ack_tick"].as_u64(),
            ) else {
                return (id, Settled::Unsupported("within_bound fields".to_owned()));
            };
            let word = match case["expected"].as_str() {
                Some(word) => word.to_owned(),
                None => return (id, Settled::Unsupported("no expected word".to_owned())),
            };
            let function = match cover.within_bound(bound, request, ack) {
                Containment::True => "true",
                Containment::False => "false",
                Containment::Indeterminate => "indeterminate",
            };
            if function != word {
                Settled::Mismatch(format!("{id}: expected {word}, evaluator {function}"))
            } else if let (Ok(bound), true) = (u64::try_from(bound), request != ack) {
                let through = ack.max(request + bound) + 1 + uncertainty;
                let scenario = counter_scenario(
                    &id,
                    bound,
                    request,
                    ack,
                    through,
                    Some(TimestampedCover::Counter { uncertainty }),
                );
                match run(&scenario) {
                    Ok(report) => {
                        let actual = verdict_word(&report.disposition);
                        let source_word = actual
                            .split_once('/')
                            .map_or(actual.as_str(), |(head, _)| head);
                        if source_word == word {
                            Settled::Matched
                        } else {
                            Settled::Mismatch(format!(
                                "{id}: expected {word}, source run {actual}"
                            ))
                        }
                    }
                    Err(error) => Settled::Mismatch(error),
                }
            } else {
                Settled::Matched
            }
        }
        Some("definitely_before") => {
            let (Some(a), Some(b), Some(expected)) = (
                case["a_tick"].as_u64(),
                case["b_tick"].as_u64(),
                case["expected"].as_bool(),
            ) else {
                return (id, Settled::Unsupported("definitely_before fields".to_owned()));
            };
            let actual = cover.definitely_before(a, b);
            if actual == expected {
                Settled::Matched
            } else {
                Settled::Mismatch(format!("{id}: expected {expected}, actual {actual}"))
            }
        }
        other => Settled::Unsupported(format!("{id}: no adapter for kind {other:?}")),
    };
    (id, settled)
}

/// Every case of the file and what running it produced.
fn run_all(fixtures: &Fixtures) -> Vec<(String, Settled)> {
    let mut cases = Vec::new();
    for case in &fixtures.windows {
        // The fixture's row and prose locate the independent expectation;
        // neither is used to manufacture the compiler's result.
        let _context = (case.row, &case.note, &case.listing_twin);
        cases.push((
            case.id.clone(),
            settled(run_window(case).and_then(|report| compare(case, &report))),
        ));
    }
    for case in &fixtures.orders {
        cases.push((case.id.clone(), settle_order(case, &fixtures.orders)));
    }
    for case in &fixtures.counters {
        cases.push((case.id.clone(), settle_counter(case)));
    }
    for case in &fixtures.covered {
        cases.push((case.id.clone(), settle_covered(case)));
    }
    for case in &fixtures.admits {
        cases.push((case.id.clone(), settle_admit(case)));
    }
    for case in &fixtures.tick_cover {
        cases.push(settle_tick_cover(case));
    }
    cases
}

fn standard() -> Fixtures {
    load().unwrap_or_else(|error| panic!("{error}"))
}

/// Provenance: QSL-2048. Every vector comes from parsed, checked, emitted
/// FSL; a vector that does not match must carry a ticket key.
#[trace("FR-350-AC-20", "FR-350-AC-21", "FR-501-AC-4")]
#[test]
fn conformance_fspec_finite_window_source_windows() {
    let fixtures = standard();
    let cases = run_all(&fixtures);
    let executed = cases.len();
    assert_eq!(
        executed,
        fixtures.total + fixtures.tick_cover.len(),
        "executed cases and listed cases differ"
    );
    let tally = fspec_standard::settle(RUNNER, cases, fspec_standard::registry());
    for id in &tally.matched {
        println!("passed: {id}");
    }
    if let Err(error) = fspec_standard::report(RUNNER, &tally) {
        panic!("{error}");
    }
    println!(
        "conformance: {} finite-window source windows matched",
        tally.matched.len()
    );
    println!(
        "qualification: {} of {executed} finite-window vectors matched, {} keyed to open tickets",
        tally.matched.len(),
        tally.keyed.len()
    );
}

/// Provenance: QSL-2048. Evidence atoms refer to compiled holds nodes, and
/// the fixture's uncertainty is explicitly a counter-cover uncertainty.
#[trace("FR-350-AC-20")]
#[test]
fn source_evidence_and_counter_cover_examples_match_the_standard() {
    let fixtures = standard();
    for id in [
        "FWV-13", "FWV-15", "FWV-48", "FWV-49", "FWV-50", "FWV-52", "FWV-53", "FWV-68",
    ] {
        let case = fixtures
            .windows
            .iter()
            .find(|case| case.id == id)
            .expect("the named standard case exists");
        let report = run_window(case).unwrap_or_else(|error| panic!("{error}"));
        compare(case, &report).unwrap_or_else(|error| panic!("{error}"));
    }
}

/// Provenance: QSL-2048. An altered expectation must fail against a real
/// run, in every group of the file.
#[trace("FR-350-AC-20")]
#[test]
fn an_altered_finite_window_expectation_reports_expected_and_actual() {
    let mut fixtures = standard();
    let case = fixtures
        .windows
        .iter_mut()
        .find(|case| case.id == "FWV-01")
        .expect("FWV-01");
    let actual = run_window(case).expect("the source compiles and runs");
    compare(case, &actual).expect("the unaltered independent expectation matches");
    case.expected.outcome = "true".to_owned();
    let error = compare(case, &actual).expect_err("an altered oracle fails");
    assert!(
        error.contains("FWV-01: outcome: expected \"true\", actual \"false\""),
        "{error}"
    );
    let altered = Counter {
        expected: "false".to_owned(),
        ..fixtures.counters.remove(0)
    };
    assert!(
        matches!(settle_counter(&altered), Settled::Mismatch(_)),
        "an altered counter expectation is a mismatch"
    );
    let altered = Covered {
        expected: !fixtures.covered[0].expected,
        ..fixtures.covered.remove(0)
    };
    assert!(matches!(settle_covered(&altered), Settled::Mismatch(_)));
    let altered = Admit {
        expected: "refuse".to_owned(),
        ..fixtures.admits.remove(1)
    };
    assert!(matches!(settle_admit(&altered), Settled::Mismatch(_)));
    let all = fixtures.orders;
    let mut altered_order = all
        .iter()
        .find(|case| case.id == "FWV-35")
        .map(|case| Order {
            id: case.id.clone(),
            row: case.row,
            op: case.op.clone(),
            positions: case.positions.clone(),
            expected: "refuse".to_owned(),
            refused_at: Some(2),
            same_order_as: None,
            canonical_order: None,
            note: None,
        })
        .expect("FWV-35");
    assert!(matches!(
        settle_order(&altered_order, &all),
        Settled::Mismatch(_)
    ));
    altered_order.expected = "evaluate".to_owned();
    assert_eq!(settle_order(&altered_order, &all), Settled::Matched);
}

/// Provenance: QSL-2048. An absent standard cannot become a zero-case pass.
#[trace("FR-350-AC-20")]
#[test]
fn a_missing_standard_is_an_error_before_any_case_runs() {
    let missing = std::path::Path::new("/nonexistent/fspec-dir-for-the-self-test");
    let error = fspec_standard::read_from(Some(missing.as_os_str()), FIXTURE)
        .expect_err("a missing FSPEC_DIR fails");
    assert!(
        error.contains("FSPEC_DIR=") && error.contains("is not a directory"),
        "{error}"
    );
    let empty = tempfile::tempdir().expect("a temporary directory");
    let error = fspec_standard::read_from(Some(empty.path().as_os_str()), FIXTURE)
        .expect_err("a directory without the fixture fails");
    assert!(error.contains("does not hold"), "{error}");
    let error = fixtures_from(None, None).err().expect("no bytes is an error");
    assert!(error.contains("no proposals/finite-window"), "{error}");
    let error = fixtures_from(Some(br#"{"window":[]}"#), Some(b"{}"))
        .err()
        .expect("an empty corpus fails");
    assert_eq!(error, "finite-window fixture has no window cases");
}

/// Provenance: QSL-2048. A case that does not match needs a ticket key, and
/// a keyed case that matches must lose it.
#[trace("FR-350-AC-20")]
#[test]
fn a_mismatch_needs_a_ticket_key_and_a_fixed_case_loses_it() {
    static KEYED: &[crate::fspec_pending::Pending] = &[
        crate::fspec_pending::Pending {
            runner: RUNNER,
            case: "A",
            ticket: "QSL-1",
        },
        crate::fspec_pending::Pending {
            runner: RUNNER,
            case: "B",
            ticket: "QSL-2",
        },
    ];
    let cases = vec![
        ("A".to_owned(), Settled::Mismatch("wrong".to_owned())),
        ("B".to_owned(), Settled::Matched),
        ("C".to_owned(), Settled::Unsupported("no adapter".to_owned())),
        ("D".to_owned(), Settled::Mismatch("wrong".to_owned())),
    ];
    let tally = fspec_standard::settle(RUNNER, cases, KEYED);
    assert_eq!(tally.keyed.len(), 1);
    assert_eq!(tally.keyed[0].1, "QSL-1");
    assert_eq!(tally.failures.len(), 3, "{:?}", tally.failures);
    assert!(tally.failures.iter().any(|text| text.contains("B now matches")));
    assert!(tally.failures.iter().any(|text| text.contains("C not executed")));
    assert!(tally.failures.iter().any(|text| text.contains("D mismatch")));
    assert!(fspec_standard::is_ticket_key("QSL-2048"));
    assert!(!fspec_standard::is_ticket_key("2048"));
    assert!(!fspec_standard::is_ticket_key("QSL-"));
    static STRAY: &[crate::fspec_pending::Pending] = &[crate::fspec_pending::Pending {
        runner: RUNNER,
        case: "Z",
        ticket: "bad",
    }];
    let tally = fspec_standard::settle(RUNNER, Vec::new(), STRAY);
    assert_eq!(tally.failures.len(), 2, "{:?}", tally.failures);
}
