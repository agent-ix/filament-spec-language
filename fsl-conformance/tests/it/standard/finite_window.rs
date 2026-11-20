// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2048: every entry of the standard's finite-window vectors, run from
//! generated native temporal source through public parse, check, emission
//! authentication and timestamped temporal execution.
//!
//! The fixture supplies operators, bounds, positions, progress and expected
//! outcomes. Where a group states only a disposition (`order`, `admit`,
//! `counter`), the adapter's other inputs are fixed and named below; they do
//! not depend on the expectation.

use std::collections::BTreeSet;

use filament_exact::{Cancel, ScalarLimits};
use fsl_foundation::{Source, SourceIdentity};
use fsl_package::{ClockConfiguration, TemporalClockSelection};
use fsl_replay::spine::{
    self, ClauseDisposition, ClauseRunReport, DependencyInput, PackageAuthentication,
    PackageTemporalRequest, ParseRequest, ProgressAssertion, ProgressClosure, ProgressCompleteness,
    SpineLimits, StateClauseExecutionPoints, TemporalIndeterminateReason, TemporalTraceRefusal,
    TemporalVerdict, TimestampedCover, TimestampedMissingValuation, TimestampedObservation,
    TimestampedOrderKey, TimestampedWindow, TraceBinding, TraceInput,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{DocumentProvision, ObservationLimits};
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{gap, Corpus, Difference, KnownGap};

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

/// since/triggered evaluate over a same-stamp tie that has no usable order
/// where the standard refuses at the tie.
const SINCE_ORDER: &str =
    "fsl-eval temporal order admission: past operators (since, triggered) evaluate a same-stamp tie without a usable order key instead of refusing at the tie (until and release refuse)";

/// Cases that do not currently conform, asserted exactly.
const KNOWN_GAPS: &[KnownGap] = &[
    gap(
        "FWV-45",
        "QSL-2048 harness: the covered predicate has no public source-run observable",
    ),
    gap(
        "FWV-46",
        "QSL-2048 harness: the covered predicate has no public source-run observable",
    ),
    gap(
        "FWV-47",
        "QSL-2048 harness: the covered predicate has no public source-run observable",
    ),
    gap(
        "FWV-69",
        "QSL-2048 harness: the canonical observation order is not exposed by the run report",
    ),
    gap("FWV-33", SINCE_ORDER),
    gap("FWV-34", SINCE_ORDER),
    gap("FWV-62", SINCE_ORDER),
    gap("FWV-63", SINCE_ORDER),
    gap(
        "FWV-57",
        "fsl-replay clause admission: an empty trace is refused at Admit (invalid_runtime_input), not as the temporal EmptyTrace refusal; whether that satisfies the standard's \"refuse\" is a spec question",
    ),
];

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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Order {
    id: String,
    row: u64,
    op: String,
    positions: Vec<(u64, Option<(String, i64)>)>,
    expected: String,
    refused_at: Option<u64>,
    same_order_as: Option<String>,
    canonical_order: Option<Vec<Option<(String, i64)>>>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TickCoverReference {
    file: String,
    row: u64,
    ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixtures {
    version: Value,
    note: String,
    window: Vec<Window>,
    order: Vec<Order>,
    counter: Vec<Counter>,
    covered: Vec<Covered>,
    admit: Vec<Admit>,
    tick_cover_vectors: TickCoverReference,
}

impl Fixtures {
    fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.window.iter().map(|case| case.id.as_str()).collect();
        ids.extend(self.order.iter().map(|case| case.id.as_str()));
        ids.extend(self.counter.iter().map(|case| case.id.as_str()));
        ids.extend(self.covered.iter().map(|case| case.id.as_str()));
        ids.extend(self.admit.iter().map(|case| case.id.as_str()));
        ids
    }
}

fn fixtures(bytes: Option<&[u8]>) -> Result<Fixtures, String> {
    let bytes =
        bytes.ok_or_else(|| format!("the pinned FSpec package does not publish {FIXTURE}"))?;
    let fixtures: Fixtures =
        serde_json::from_slice(bytes).map_err(|error| format!("{FIXTURE}: {error}"))?;
    if fixtures.window.is_empty() {
        return Err("finite-window fixture has no window cases".to_owned());
    }
    let mut seen = BTreeSet::new();
    for id in fixtures.ids() {
        if !seen.insert(id) {
            return Err(format!("duplicate finite-window case {id}"));
        }
    }
    Ok(fixtures)
}

fn standard() -> Fixtures {
    fixtures(filament_specification::published_file(FIXTURE))
        .unwrap_or_else(|error| panic!("{error}"))
}

struct Compiled {
    unit: spine::CheckedUnit,
    packages: PackageInput,
    sources: Vec<Source>,
    selections: Vec<TemporalClockSelection>,
}

/// Parses, checks and returns the unit declaring `body` as the clause
/// `Window` over `Reading` records with atoms `p` and `q`.
fn compile(id: &str, activation: &str, body: &str) -> Result<Compiled, String> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{PROFILE}\";\n\
         record Reading {{ p: Boolean; q: Boolean; serial: Integer; }}\n\
         temporal Window using t over (r: Reading) clock \"events\" on {activation} {{ {body} }}\n"
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
    .map_err(|error| format!("parse: {error:?}"))?
    .into_value();
    let cst = fsl_cst::parse(
        identity,
        "finite-window.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .map_err(|error| format!("CST: {error:?}"))?;
    let forms = fsl_forms::build_unit(&cst).map_err(|error| format!("forms: {error:?}"))?;
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
        .map_err(|error| format!("packages: {error:?}"))?;
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(|error| format!("select: {error:?}"))?
        .into_value();
    let unit = spine::check_with_temporal_selections_and_execution_points(
        &parsed,
        id.to_owned(),
        &StateClauseExecutionPoints::new(),
        &selections,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("check: {error:?}"))?
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

/// One observation of the trace: its stamp, order key and atom valuations.
struct Position {
    stamp: u64,
    order_key: Option<TimestampedOrderKey>,
    p: Option<bool>,
    q: bool,
}

/// Every non-source input of one run.
struct Run<'a> {
    id: &'a str,
    activation: &'a str,
    body: String,
    positions: Vec<Position>,
    watermark: u64,
    complete: Option<u64>,
    scope: ProgressClosure,
    execution: ProgressClosure,
    authoritative_origin: bool,
    cover: Option<TimestampedCover>,
}

fn closure(value: &str) -> Result<ProgressClosure, String> {
    match value {
        "closed" => Ok(ProgressClosure::Closed),
        "open" => Ok(ProgressClosure::Open),
        other => Err(format!("unknown closure {other}")),
    }
}

fn execute(run: &Run<'_>) -> Result<ClauseRunReport, String> {
    let compiled = compile(run.id, run.activation, &run.body)?;
    let cancel = Cancel::new();
    let authentication = spine::authenticate_emission(
        compiled.unit.package(),
        &compiled.selections,
        &compiled.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .map_err(|error| format!("compile/authenticate: {error:?}"))?;
    let mut observations = Vec::new();
    let mut missing = Vec::new();
    for (index, position) in run.positions.iter().enumerate() {
        if position.p.is_none() {
            missing.push(TimestampedMissingValuation {
                position: u64::try_from(index).map_err(|error| error.to_string())?,
                atom: 1,
            });
        }
        let sample = json!({
            "p": {"boolean": position.p.unwrap_or(false)},
            "q": {"boolean": position.q},
            "serial": {"integer": index.to_string()},
        })
        .to_string();
        observations.push(TimestampedObservation {
            document: spine::sample_position(&sample),
            stamp: position.stamp,
            order_key: position.order_key.clone(),
        });
    }
    let trace = TraceInput::Timestamped {
        observations,
        progress: ProgressAssertion {
            watermark: run.watermark,
            completeness: run
                .complete
                .map_or(ProgressCompleteness::Incomplete, |through| {
                    ProgressCompleteness::Complete { through }
                }),
            scope: run.scope,
            execution: run.execution,
        },
        window: Box::new(TimestampedWindow {
            binding: Some(TraceBinding {
                profile: PROFILE.to_owned(),
                clock_name: "events".to_owned(),
                parameters: ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ms".into(),
                },
            }),
            authoritative_origin: run.authoritative_origin,
            cover: run.cover,
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

fn window_run(case: &Window) -> Result<Run<'_>, String> {
    let operator = match case.op.as_str() {
        "always" | "eventually" | "once" => case.op.as_str(),
        other => return Err(format!("no source operator adapter for {other}")),
    };
    let activation = match case.activate.as_str() {
        "origin" => "origin",
        "each" => "each (q: Reading)",
        other => return Err(format!("no activation adapter for {other}")),
    };
    let positions = case
        .positions
        .iter()
        .map(|(stamp, atom)| {
            let p = match atom.as_str() {
                "T" => Some(true),
                "F" => Some(false),
                "absent" => None,
                other => return Err(format!("unknown atom valuation {other}")),
            };
            Ok(Position {
                stamp: *stamp,
                order_key: None,
                p,
                q: false,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Run {
        id: &case.id,
        activation,
        body: format!("{operator}[{},{}] holds(r.p)", case.a, case.b),
        positions,
        watermark: case.w,
        complete: case.complete,
        scope: closure(&case.scope)?,
        execution: closure(case.execution.as_deref().unwrap_or("closed"))?,
        authoritative_origin: match case.lower.as_deref() {
            None | Some("AuthoritativeOrigin") => true,
            Some("HistoryCutoff") => false,
            Some(other) => return Err(format!("unknown lower boundary {other}")),
        },
        cover: case
            .uncertainty
            .map(|uncertainty| TimestampedCover::Counter { uncertainty }),
    })
}

type Observed = (
    &'static str,
    Option<u64>,
    Option<u64>,
    Option<&'static str>,
    Option<u64>,
);

fn verdict(report: &ClauseRunReport) -> Result<Observed, String> {
    Ok(match &report.disposition {
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
                        return Err(format!("missing valuation names {atom:?}"));
                    }
                    ("MissingValuation", Some(*position))
                }
                TemporalIndeterminateReason::ConflictingValuation { position, atom } => {
                    if atom != "holds(r.p)" {
                        return Err(format!("conflicting valuation names {atom:?}"));
                    }
                    ("ConflictingValuation", Some(*position))
                }
                TemporalIndeterminateReason::IncompleteInput => ("IncompleteInput", None),
                TemporalIndeterminateReason::MissingHistory => ("MissingHistory", None),
                TemporalIndeterminateReason::StraddlesBound => ("StraddlesBound", None),
            };
            ("indeterminate", Some(*at), None, Some(name), position)
        }
        other => return Err(format!("{other:?}")),
    })
}

fn compare_window(case: &Window, report: &ClauseRunReport) -> Result<(), Difference> {
    let expected = &case.expected;
    let (outcome, at, deadline, reason, position) =
        verdict(report).map_err(|observed| Difference::new(&expected.outcome, observed))?;
    let field = |name: &str, wanted: &dyn std::fmt::Debug, actual: &dyn std::fmt::Debug| {
        Err(Difference::new(
            format!("{name} {wanted:?}"),
            format!("{name} {actual:?}"),
        ))
    };
    if outcome != expected.outcome {
        return field("outcome", &expected.outcome, &outcome);
    }
    if expected.at.is_some() && at != expected.at {
        return field("activation", &expected.at, &at);
    }
    if deadline != expected.deadline {
        return field("deadline", &expected.deadline, &deadline);
    }
    if reason != expected.reason.as_deref() {
        return field("reason", &expected.reason, &reason);
    }
    if position != expected.position {
        return field("position", &expected.position, &position);
    }
    let Some(window) = report.finite_window.as_ref() else {
        return Err(Difference::new(
            "a finite-window report",
            "no finite-window report",
        ));
    };
    if let Some(expected) = &expected.basis {
        if window.basis.as_str() != expected {
            return field("basis", expected, &window.basis);
        }
    }
    if let Some(expected) = &expected.support {
        let mut translated = Vec::new();
        for (position, atom) in expected {
            if atom != "p" {
                return Err(Difference::new(
                    format!("support atom {atom}"),
                    "no adapter for that atom",
                ));
            }
            translated.push((*position, "holds(r.p)".to_owned()));
        }
        let actual = window
            .support
            .iter()
            .map(|item| (item.position, item.atom.clone()))
            .collect::<Vec<_>>();
        if actual != translated {
            return field("support", &translated, &actual);
        }
    }
    Ok(())
}

fn window_outcome(case: &Window) -> Result<(), Difference> {
    // The fixture's row and prose locate the independent expectation;
    // neither is used to manufacture the compiler's result.
    let _context = (case.row, &case.note, &case.listing_twin);
    let report = window_run(case)
        .and_then(|run| execute(&run))
        .map_err(|observed| Difference::new(&case.expected.outcome, observed))?;
    compare_window(case, &report)
}

/// The fixed atom valuations of an order case: they depend on the
/// observation itself, not on its listed place, so a reordered listing of
/// the same observations is the same trace.
fn order_position(stamp: u64, key: &Option<(String, i64)>) -> Position {
    let p = key.as_ref().map_or(stamp % 2 == 0, |(_, key)| key % 2 == 1);
    Position {
        stamp,
        order_key: key.as_ref().map(|(authority, key)| TimestampedOrderKey {
            authority: authority.clone(),
            key: *key,
        }),
        p: Some(p),
        q: !p,
    }
}

fn order_run(case: &Order) -> Result<Run<'_>, String> {
    let body = match case.op.as_str() {
        "eventually" => "eventually[0,5] holds(r.p)".to_owned(),
        "until" | "release" | "since" | "triggered" => {
            format!("holds(r.p) {}[0,5] holds(r.q)", case.op)
        }
        other => return Err(format!("no source operator adapter for {other}")),
    };
    let positions: Vec<Position> = case
        .positions
        .iter()
        .map(|(stamp, key)| order_position(*stamp, key))
        .collect();
    let last = positions
        .iter()
        .map(|position| position.stamp)
        .max()
        .unwrap_or(0);
    Ok(Run {
        id: &case.id,
        activation: "origin",
        body,
        positions,
        watermark: last,
        complete: Some(last),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover: None,
    })
}

/// `evaluate` or `refuse at <position>`, as an order case states it.
fn disposition(report: &ClauseRunReport) -> String {
    match &report.disposition {
        ClauseDisposition::Temporal(_) => "evaluate".to_owned(),
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::SameStampOrderUnavailable {
            position,
        }) => format!("refuse at {position}"),
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::EmptyTrace) => "refuse".to_owned(),
        other => format!("{other:?}"),
    }
}

fn order_outcome(case: &Order, fixtures: &Fixtures) -> Result<(), Difference> {
    let _context = (case.row, &case.note);
    let expected = match (case.expected.as_str(), case.refused_at) {
        ("refuse", Some(at)) => format!("refuse at {at}"),
        (other, _) => other.to_owned(),
    };
    let report = order_run(case)
        .and_then(|run| execute(&run))
        .map_err(|observed| Difference::new(&expected, observed))?;
    let observed = disposition(&report);
    if observed != expected {
        return Err(Difference::new(expected, observed));
    }
    if let Some(twin) = &case.same_order_as {
        let other = fixtures
            .order
            .iter()
            .find(|other| &other.id == twin)
            .ok_or_else(|| Difference::new(format!("the same order as {twin}"), "no such case"))?;
        let twin_report = order_run(other)
            .and_then(|run| execute(&run))
            .map_err(|observed| Difference::new(format!("the same order as {twin}"), observed))?;
        if format!("{:?}", twin_report.disposition) != format!("{:?}", report.disposition) {
            return Err(Difference::new(
                format!("the verdict of {twin}: {:?}", twin_report.disposition),
                format!("{:?}", report.disposition),
            ));
        }
    }
    if let Some(order) = &case.canonical_order {
        return Err(Difference::new(
            format!("canonical order {order:?}"),
            "the run report does not expose the admitted observation order",
        ));
    }
    Ok(())
}

/// A request at `request_tick` must be acknowledged within `bound`: the
/// clause is `eventually[0,bound] holds(ack)` activated at the request.
fn counter_outcome(case: &Counter) -> Result<(), Difference> {
    let _context = (case.row, &case.note);
    let expected = match &case.reason {
        Some(reason) => format!("{} {reason}", case.expected),
        None => case.expected.clone(),
    };
    let cover = match (case.cover.as_str(), case.uncertainty) {
        ("counter", Some(uncertainty)) => Some(TimestampedCover::Counter { uncertainty }),
        ("exact", None) => Some(TimestampedCover::Exact),
        ("omitted", None) => None,
        (other, uncertainty) => {
            return Err(Difference::new(
                expected,
                format!("no cover adapter for {other} with uncertainty {uncertainty:?}"),
            ))
        }
    };
    let position = |stamp, ack| Position {
        stamp,
        order_key: None,
        p: Some(ack),
        q: false,
    };
    let run = Run {
        id: &case.id,
        activation: "origin",
        body: format!("eventually[0,{}] holds(r.p)", case.bound),
        positions: vec![
            position(case.request_tick, false),
            position(case.ack_tick, true),
        ],
        watermark: case.w,
        complete: Some(case.complete),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover,
    };
    let report = execute(&run).map_err(|observed| Difference::new(&expected, observed))?;
    let (outcome, _, _, reason, _) =
        verdict(&report).map_err(|observed| Difference::new(&expected, observed))?;
    let observed = match reason {
        Some(reason) => format!("{outcome} {reason}"),
        None => outcome.to_owned(),
    };
    if observed == expected {
        Ok(())
    } else {
        Err(Difference::new(expected, observed))
    }
}

fn admit_outcome(case: &Admit) -> Result<(), Difference> {
    let _context = (case.row, &case.note);
    let positions: Vec<Position> = case
        .stamps
        .iter()
        .map(|stamp| Position {
            stamp: *stamp,
            order_key: None,
            p: Some(false),
            q: false,
        })
        .collect();
    let last = case.stamps.iter().copied().max().unwrap_or(0);
    let run = Run {
        id: &case.id,
        activation: "origin",
        body: "eventually[0,5] holds(r.p)".to_owned(),
        positions,
        watermark: last,
        complete: Some(last),
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
        authoritative_origin: true,
        cover: None,
    };
    let report = execute(&run).map_err(|observed| Difference::new(&case.expected, observed))?;
    let observed = disposition(&report);
    if observed == case.expected {
        Ok(())
    } else {
        Err(Difference::new(&case.expected, observed))
    }
}

fn covered_outcome(case: &Covered) -> Result<(), Difference> {
    Err(Difference::new(
        format!(
            "stamp {} covered by watermark {}: {}",
            case.stamp, case.w, case.expected
        ),
        "no public source-run observable reports coverage of one stamp",
    ))
}

fn run_corpus(fixtures: &Fixtures) -> Corpus {
    let _context = (&fixtures.version, &fixtures.note);
    let mut corpus = Corpus::new("finite-window", fixtures.ids().len());
    for case in &fixtures.window {
        corpus.record(&case.id, window_outcome(case));
    }
    for case in &fixtures.order {
        corpus.record(&case.id, order_outcome(case, fixtures));
    }
    for case in &fixtures.counter {
        corpus.record(&case.id, counter_outcome(case));
    }
    for case in &fixtures.covered {
        corpus.record(&case.id, covered_outcome(case));
    }
    for case in &fixtures.admit {
        corpus.record(&case.id, admit_outcome(case));
    }
    corpus
}

/// Provenance: QSL-2048. Every finite-window entry runs from parsed, checked,
/// emitted FSL; the failures are exactly the listed known gaps.
#[trace("FR-350-AC-20", "FR-350-AC-21", "FR-501-AC-4")]
#[test]
fn conformance_fspec_finite_window_source_windows() {
    let fixtures = standard();
    run_corpus(&fixtures).assert_settled(KNOWN_GAPS);
}

/// Provenance: QSL-2048. The tick-cover rows the finite-window file defers to
/// are published under the named ids; their vectors run in `fsl-eval`'s
/// finite-window tests.
#[trace("FR-350-AC-20")]
#[test]
fn the_referenced_tick_cover_vectors_are_published() {
    let fixtures = standard();
    let reference = &fixtures.tick_cover_vectors;
    assert_eq!(
        reference.file,
        "../checked-package-v2/tick-cover-vectors.json"
    );
    let _row = reference.row;
    let document: Value = serde_json::from_slice(
        filament_specification::published_file(TICK_COVER).expect("tick-cover vectors"),
    )
    .expect("tick-cover JSON");
    let published: BTreeSet<&str> = document["cases"]
        .as_array()
        .expect("tick-cover cases")
        .iter()
        .filter_map(|case| case["id"].as_str())
        .collect();
    for id in &reference.ids {
        assert!(published.contains(id.as_str()), "{id} is published");
    }
}

/// Provenance: QSL-2048. An altered expectation must fail against a real run.
#[trace("FR-350-AC-20")]
#[test]
fn an_altered_finite_window_expectation_reports_expected_and_actual() {
    let mut fixtures = standard();
    let case = fixtures
        .window
        .iter_mut()
        .find(|case| case.id == "FWV-01")
        .expect("FWV-01");
    window_outcome(case).expect("the unaltered independent expectation matches");
    case.expected.outcome = "true".to_owned();
    assert_eq!(
        window_outcome(case),
        Err(Difference::new("outcome \"true\"", "outcome \"false\""))
    );
    let errors = run_corpus(&fixtures)
        .settle(KNOWN_GAPS)
        .expect_err("the altered case is an unlisted mismatch");
    assert_eq!(
        errors,
        vec![
            "finite-window: FWV-01: expected outcome \"true\", observed outcome \"false\" (not a listed known gap)"
                .to_owned()
        ]
    );
}

/// Provenance: QSL-2048. An absent published corpus cannot become a zero-case pass.
#[trace("FR-350-AC-20")]
#[test]
fn a_missing_published_fixture_is_an_error_before_any_case_runs() {
    let error = fixtures(None)
        .err()
        .expect("the published fixture is required");
    assert_eq!(
        error,
        format!("the pinned FSpec package does not publish {FIXTURE}")
    );
}
