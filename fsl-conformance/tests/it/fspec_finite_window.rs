// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-to-verdict execution of the standard's finite-window examples.
//! The window group is executable; the other fixture groups are explicitly
//! outside this adapter and cannot qualify as a complete corpus.

use std::collections::BTreeSet;

use filament_exact::Cancel;
use fsl_foundation::{Source, SourceIdentity};
use fsl_package::{ClockConfiguration, TemporalClockSelection};
use fsl_replay::spine::{
    self, ClauseDisposition, DependencyInput, PackageAuthentication, PackageTemporalRequest,
    ParseRequest, ProgressAssertion, ProgressClosure, ProgressCompleteness, SpineLimits,
    StateClauseExecutionPoints, TemporalIndeterminateReason, TemporalVerdict, TimestampedCover,
    TimestampedMissingValuation, TimestampedObservation, TimestampedWindow, TraceBinding,
    TraceInput,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{DocumentProvision, ObservationLimits};
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::{json, Value};

const PROFILE: &str = "filament.temporal.timestamped-event.finite-window/v1";
const FIXTURE: &str = "proposals/finite-window/finite-window-vectors.json";

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

struct Fixtures {
    windows: Vec<Window>,
    remaining: Vec<String>,
    total: usize,
}

impl Fixtures {
    fn require_complete(&self) -> Result<(), String> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "finite-window qualification incomplete: no source adapter for {}",
                self.remaining.join(", ")
            ))
        }
    }
}

fn fixtures(bytes: Option<&[u8]>) -> Result<Fixtures, String> {
    let bytes =
        bytes.ok_or_else(|| format!("the pinned FSpec package does not publish {FIXTURE}"))?;
    let document: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let windows: Vec<Window> = serde_json::from_value(document["window"].clone())
        .map_err(|error| format!("{FIXTURE}/window: {error}"))?;
    if windows.is_empty() {
        return Err("finite-window fixture has no window cases".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut remaining = Vec::new();
    for (group, entries) in document.as_object().ok_or("fixture must be an object")? {
        if let Some(entries) = entries.as_array() {
            for entry in entries {
                let id = entry["id"]
                    .as_str()
                    .ok_or("each fixture case needs an id")?;
                if !ids.insert(id.to_owned()) {
                    return Err(format!("duplicate finite-window case {id}"));
                }
                if group != "window" {
                    remaining.push(format!("{id} ({group})"));
                }
            }
        }
    }
    Ok(Fixtures {
        windows,
        remaining,
        total: ids.len(),
    })
}

struct Compiled {
    unit: spine::CheckedUnit,
    packages: PackageInput,
    sources: Vec<Source>,
    selections: Vec<TemporalClockSelection>,
}

fn compile(case: &Window) -> Result<Compiled, String> {
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
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{PROFILE}\";\n\
         record Reading {{ p: Boolean; serial: Integer; }}\n\
         temporal Window using t over (r: Reading) clock \"events\" on {activation} {{ {operator}[{},{}] holds(r.p) }}\n",
        case.a, case.b
    );
    let identity = SourceIdentity::new("fspec", &case.id);
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
    .map_err(|error| format!("{}: parse: {error:?}", case.id))?
    .into_value();
    let cst = fsl_cst::parse(
        identity,
        "finite-window.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .map_err(|error| format!("{}: CST: {error:?}", case.id))?;
    let forms =
        fsl_forms::build_unit(&cst).map_err(|error| format!("{}: forms: {error:?}", case.id))?;
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
        .map_err(|error| format!("{}: packages: {error:?}", case.id))?;
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(|error| format!("{}: select: {error:?}", case.id))?
        .into_value();
    let unit = spine::check_with_temporal_selections_and_execution_points(
        &parsed,
        case.id.clone(),
        &StateClauseExecutionPoints::new(),
        &selections,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("{}: check: {error:?}", case.id))?
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

fn run(case: &Window) -> Result<spine::ClauseRunReport, String> {
    let compiled = compile(case)?;
    let cancel = Cancel::new();
    let authentication = spine::authenticate_emission(
        compiled.unit.package(),
        &compiled.selections,
        &compiled.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .map_err(|error| format!("{}: compile/authenticate: {error:?}", case.id))?;
    let mut observations = Vec::new();
    let mut missing = Vec::new();
    for (index, (stamp, atom)) in case.positions.iter().enumerate() {
        let p = match atom.as_str() {
            "T" => true,
            "F" => false,
            "absent" => {
                missing.push(TimestampedMissingValuation {
                    position: u64::try_from(index).map_err(|error| error.to_string())?,
                    atom: 1,
                });
                false
            }
            other => return Err(format!("{}: unknown atom valuation {other}", case.id)),
        };
        let sample =
            json!({"p": {"boolean": p}, "serial": {"integer": index.to_string()}}).to_string();
        observations.push(TimestampedObservation {
            document: spine::sample_position(&sample),
            stamp: *stamp,
            order_key: None,
        });
    }
    let trace = TraceInput::Timestamped {
        observations,
        progress: ProgressAssertion {
            watermark: case.w,
            completeness: case
                .complete
                .map_or(ProgressCompleteness::Incomplete, |through| {
                    ProgressCompleteness::Complete { through }
                }),
            scope: closure(&case.scope)?,
            execution: closure(case.execution.as_deref().unwrap_or("closed"))?,
        },
        window: Box::new(TimestampedWindow {
            binding: Some(TraceBinding {
                profile: PROFILE.to_owned(),
                clock_name: "events".to_owned(),
                parameters: ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ms".into(),
                },
            }),
            authoritative_origin: match case.lower.as_deref() {
                None | Some("AuthoritativeOrigin") => true,
                Some("HistoryCutoff") => false,
                Some(other) => return Err(format!("{}: unknown lower boundary {other}", case.id)),
            },
            cover: case
                .uncertainty
                .map(|uncertainty| TimestampedCover::Bounded { uncertainty }),
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
        accounting: Default::default(),
        name: "Window",
        trace: &trace,
        over: None,
        ledger: None,
        sources: &compiled.sources,
        authentication: Some(PackageAuthentication::temporal(&authentication)),
        cancel: &cancel,
    }))
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
                    if atom != "r.p" {
                        return Err(format!(
                            "{}: missing valuation names {atom:?}, expected r.p",
                            case.id
                        ));
                    }
                    ("MissingValuation", Some(*position))
                }
                TemporalIndeterminateReason::ConflictingValuation { position, atom } => {
                    if atom != "r.p" {
                        return Err(format!(
                            "{}: conflicting valuation names {atom:?}, expected r.p",
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
                Ok((*position, "r.p".to_owned()))
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

fn standard() -> Fixtures {
    fixtures(filament_specification::published_file(FIXTURE))
        .unwrap_or_else(|error| panic!("{error}"))
}

/// Provenance: QSL-2048. Each result comes from parsed, checked, emitted FSL.
#[trace("FR-350-AC-20", "FR-350-AC-21", "FR-501-AC-4")]
#[test]
fn conformance_fspec_finite_window_source_windows() {
    let fixtures = standard();
    let mut errors = Vec::new();
    for case in &fixtures.windows {
        // The fixture's row and prose locate the independent expectation;
        // neither is used to manufacture the compiler's result.
        let _context = (case.row, &case.note, &case.listing_twin);
        match run(case).and_then(|report| compare(case, &report)) {
            Ok(()) => println!("passed: {}", case.id),
            Err(error) => errors.push(error),
        }
    }
    assert_eq!(
        fixtures.windows.len() + fixtures.remaining.len(),
        fixtures.total
    );
    for pending in &fixtures.remaining {
        println!("pending: {pending}: source adapter not implemented");
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    println!(
        "conformance: {} finite-window source windows matched",
        fixtures.windows.len()
    );
    if std::env::var_os("FSPEC_FULL_QUALIFICATION").is_some() {
        fixtures
            .require_complete()
            .unwrap_or_else(|error| panic!("{error}"));
    } else if let Err(error) = fixtures.require_complete() {
        println!("qualification: {error}");
    }
}

/// Provenance: QSL-2048. An altered expectation must fail against a real run.
#[trace("FR-350-AC-20")]
#[test]
fn an_altered_finite_window_expectation_reports_expected_and_actual() {
    let mut fixtures = standard();
    let case = fixtures
        .windows
        .iter_mut()
        .find(|case| case.id == "FWV-01")
        .expect("FWV-01");
    let actual = run(case).expect("the source compiles and runs");
    compare(case, &actual).expect("the unaltered independent expectation matches");
    case.expected.outcome = "true".to_owned();
    let error = compare(case, &actual).expect_err("an altered oracle fails");
    assert!(
        error.contains("FWV-01: outcome: expected \"true\", actual \"false\""),
        "{error}"
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
    let error = fixtures(Some(br#"{"window":[]}"#))
        .err()
        .expect("an empty corpus fails");
    assert_eq!(error, "finite-window fixture has no window cases");
}
