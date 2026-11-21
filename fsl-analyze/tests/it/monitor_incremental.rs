// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-643 (TC-1666, TC-1667): the stepping monitor over a record trace
//! subject. Each test feeds observations one at a time and compares the
//! session with FR-283's batch `monitor`, or with the batch trace run.

use filament_exact::Cancel;
use fsl_analyze::monitor::{
    MonitorClause, MonitorLimits, MonitorRefusal, MonitorVerdict, DEFAULT_RETAINED_POSITIONS,
};
use fsl_analyze::monitor_session::{
    MonitorObservation, MonitorOpen, MonitorPosition, MonitorSession, MonitorStamp, MonitorState,
    MonitorTraceKind,
};
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::{Source, SourceIdentity};
use fsl_package::{ClockConfiguration, TemporalClockSelection};
use fsl_replay::spine::{
    self, run_package_temporal, ContradictedDimension, DependencyInput, PackageAuthentication,
    PackageTemporalRequest, ParseRequest, ProgressAssertion, ProgressClosure, ProgressCompleteness,
    ProgressLowerBoundary, SpineLimits, StateClauseExecutionPoints, TimestampedObservation,
    TimestampedWindow, TraceBinding, TraceInput,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{DocumentProvision, ObservationLimits};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;

const PATH: &str = "monitor-incremental.native";
const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";
const EVENTS: &str = "filament.temporal.event-position.false-extension/v1";
const SAMPLES: &str = "filament.temporal.fixed-sample.false-extension/v1";

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
    profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
    profile i = \"filament.temporal.infinite-trace/v1\";\n\
    profile f = \"filament.temporal.fixed-sample.false-extension/v1\";\n\
    record Reading { level: Int[0, 9]; }\n\
    record Run { level: Int[0, 9]; stopped: Boolean; }\n\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    temporal Counted using w over (r: Reading) clock \"plant-ms\" on origin { always[0,5] (count over past[0 ms, 3 ms] min_count 0 <= 9) }\n\
    temporal Twos using w over (r: Reading) clock \"plant-ms\" on origin { always[0,5] (count where holds(r.level = 2) over past[0 ms, 3 ms] min_count 0 <= 1) }\n\
    temporal TwosMin using w over (r: Reading) clock \"plant-ms\" on origin { always[2,5] (sum of r.level where holds(r.level = 2) over past[0 ms, 3 ms] min_count 2 <= 100) }\n\
    temporal Median using w over (r: Reading) clock \"plant-ms\" on origin { always[0,5] (quantile 0.5 of r.level over past[0 ms, 3 ms] <= 1) }\n\
    temporal E using e over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n\
    temporal T using w over (r: Reading) clock \"plant-ms\" on origin { eventually[0,5] holds(r.level > 0) }\n\
    temporal Reaches using i over (r: Reading) clock \"steps\" on origin { eventually holds(r.level = 2) }\n\
    temporal Never using i over (r: Reading) clock \"steps\" on origin { always holds(r.level < 2) }\n    temporal Below using i over (r: Reading) clock \"steps\" on origin { always holds(r.level < 9) }\n    temporal Recent using e over (r: Reading) clock \"events\" on origin { always[0,9] (count where holds(r.level = 2) over past[0, 3] min_count 0 <= 1) }\n\
    temporal Bounded using e over (r: Reading) clock \"events\" on origin { eventually[0,1] holds(r.level = 2) }\n\
    temporal AtMostOne using i over (r: Reading) clock \"steps\" on origin { always holds(r.level <= 1) }\n\
    temporal Nine using i over (r: Reading) clock \"steps\" on origin { eventually holds(r.level = 9) }\n\
    temporal Sampled using f over (r: Reading) clock \"samples\" on origin { eventually[0,3] holds(r.level = 9) }\n\
    temporal Past using e over (r: Reading) clock \"events\" on each (q: Reading) { once[0,2] holds(r.level = 9) }\n\
    temporal Window using w over (r: Reading) clock \"plant-ms\" on each (q: Reading) { eventually[0,4] holds(r.level = 2) }\n\
    temporal Fast using e over (r: Run) clock \"events\" on origin { capture startFast: Boolean = r.level > 3; eventually[0,3] holds(r.stopped or not startFast) }\n";

struct World {
    unit: spine::CheckedUnit,
    sources: Vec<Source>,
    selections: Vec<TemporalClockSelection>,
    packages: PackageInput,
}

fn selections(parsed: &fsl_cst::ParsedSource) -> Vec<TemporalClockSelection> {
    use fsl_forms::DeclarationForm;
    let unit = fsl_forms::build_unit(parsed).expect("the unit forms");
    let profiles = &unit.selections().profiles;
    unit.forms()
        .iter()
        .filter_map(|form| {
            let DeclarationForm::Temporal(clause) = form.form() else {
                return None;
            };
            let selected = profiles
                .iter()
                .find(|profile| profile.alias == clause.profile.alias)
                .expect("the temporal profile is selected");
            let clock = match selected.identity.as_str() {
                EVENTS => ClockConfiguration::EventPosition {
                    sequence_authority: "events".into(),
                },
                STAMPED => ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "ms".into(),
                },
                SAMPLES => ClockConfiguration::FixedSample {
                    epoch: fsl_package::ClockExactNumber::Integer {
                        decimal: "0".into(),
                    },
                    period: fsl_package::ClockExactNumber::Integer {
                        decimal: "1".into(),
                    },
                    unit: "sample".into(),
                },
                _ => return None,
            };
            Some(TemporalClockSelection::new(
                parsed.source().reference().clone(),
                form.span(),
                clock,
            ))
        })
        .collect()
}

fn world() -> World {
    world_source(UNIT)
}

fn world_source(source: &str) -> World {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("agent-ix", "monitor-incremental");
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("no domain package");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: PATH,
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("selects")
        .into_value();
    let cst = fsl_cst::parse(
        identity,
        PATH,
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the unit parses");
    let selections = selections(&cst);
    let unit = spine::check_with_temporal_selections_and_execution_points(
        &parsed,
        "fixture".to_owned(),
        &StateClauseExecutionPoints::new(),
        &selections,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("the unit checks")
    .into_value();
    let mut sources = vec![unit.source().clone()];
    sources.extend(unit.libraries().iter().cloned());
    World {
        unit,
        sources,
        selections,
        packages,
    }
}

fn limits(retained_positions: u64) -> MonitorLimits {
    MonitorLimits {
        observation: ObservationLimits::default(),
        normalization: ModelNormalizationLimits::default(),
        accounting: UNLIMITED,
        max_window_values: fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
        retained_positions,
        retained_identities: fsl_analyze::monitor::DEFAULT_RETAINED_IDENTITIES,
    }
}

fn clauses(names: &[&str]) -> Vec<MonitorClause> {
    names
        .iter()
        .map(|name| MonitorClause {
            name: (*name).to_owned(),
            over: None,
        })
        .collect()
}

fn text(level: i64) -> String {
    format!(r#"{{"level":{{"integer":"{level}"}}}}"#)
}

fn stamped_binding() -> TraceBinding {
    TraceBinding {
        profile: STAMPED.to_owned(),
        clock_name: "plant-ms".to_owned(),
        parameters: ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into(),
        },
    }
}

fn events_binding() -> TraceBinding {
    TraceBinding {
        profile: EVENTS.to_owned(),
        clock_name: "events".to_owned(),
        parameters: ClockConfiguration::EventPosition {
            sequence_authority: "events".into(),
        },
    }
}

impl World {
    fn open<'a>(
        &'a self,
        names: &[&str],
        kind: MonitorTraceKind,
        binding: Option<TraceBinding>,
        retained: u64,
    ) -> Result<MonitorSession<'a>, StageFailure<MonitorRefusal>> {
        MonitorSession::open(
            self.unit.package(),
            MonitorOpen {
                clauses: clauses(names),
                kind,
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding,
                cover: None,
                ledger: None,
                packages: &self.packages,
                sources: &self.sources,
                temporal_selections: &self.selections,
            },
            &limits(retained),
            &Cancel::new(),
        )
    }

    fn batch_report(&self, name: &str, levels: &[i64]) -> fsl_replay::spine::ClauseRunReport {
        let trace = TraceInput::Finite(
            levels
                .iter()
                .map(|level| spine::sample_position(&text(*level)))
                .collect(),
        );
        run_package_temporal(&PackageTemporalRequest {
            package: self.unit.package(),
            packages: &self.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &DocumentProvision::new(),
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name,
            trace: &trace,
            over: None,
            ledger: None,
            sources: &self.sources,
            authentication: None,
            cancel: &Cancel::new(),
        })
    }

    /// The batch trace run over the samples `levels`, as a `finite` trace: each
    /// clause's disposition.
    fn batch(&self, names: &[&str], levels: &[i64]) -> Vec<String> {
        let trace = TraceInput::Finite(
            levels
                .iter()
                .map(|level| spine::sample_position(&text(*level)))
                .collect(),
        );
        names
            .iter()
            .map(|name| {
                let report = run_package_temporal(&PackageTemporalRequest {
                    package: self.unit.package(),
                    packages: &self.packages,
                    model_limits: ModelNormalizationLimits::default(),
                    snapshots: &DocumentProvision::new(),
                    observation_limits: ObservationLimits::default(),
                    accounting: UNLIMITED,
                    name,
                    trace: &trace,
                    over: None,
                    ledger: None,
                    sources: &self.sources,
                    authentication: None,
                    cancel: &Cancel::new(),
                });
                format!("{:?}", report.disposition)
            })
            .collect()
    }
}

fn sample<'a>(text: &'a str) -> MonitorObservation<'a> {
    MonitorObservation {
        position: MonitorPosition::Sample(text),
        stamp: None,
        order_key: None,
        missing: &[],
    }
}

fn stamped<'a>(text: &'a str, stamp: u64) -> MonitorObservation<'a> {
    MonitorObservation {
        position: MonitorPosition::Sample(text),
        stamp: Some(MonitorStamp::Tick(stamp)),
        order_key: None,
        missing: &[],
    }
}

fn progress(watermark: u64) -> ProgressAssertion {
    ProgressAssertion {
        watermark,
        completeness: ProgressCompleteness::Complete { through: watermark },
        scope: ProgressClosure::Open,
        execution: ProgressClosure::Open,
    }
}

fn refusal(failure: StageFailure<MonitorRefusal>) -> MonitorRefusal {
    match failure {
        StageFailure::Refused(refusal) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// FR-643-AC-1 (TC-1666), T-063: observations fed one at a time give, after
/// each, the verdicts the batch monitor gives over the prefix, and `finish`
/// the batch verdicts over the whole trace.
#[test]
#[trace("TC-1666", "FR-643-AC-1")]
fn feeding_one_at_a_time_equals_the_batch_over_each_prefix() {
    let world = world();
    let names = ["Reaches", "Never"];
    let levels = [0, 1, 2, 1, 0];
    let mut session = world
        .open(
            &names,
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        let step = session
            .step(sample(&observed), &Cancel::new())
            .expect("a step");
        let batch = world.batch(&names, &levels[..=index]);
        let stepped: Vec<String> = step
            .statuses
            .iter()
            .map(|(_, status)| format!("{:?}", status.record().report().disposition))
            .collect();
        assert_eq!(stepped, batch, "after observation {index}");
    }
    let outcome = session.finish(&Cancel::new()).expect("ends");
    let finished: Vec<String> = outcome
        .records
        .iter()
        .map(|record| format!("{:?}", record.report().disposition))
        .collect();
    assert_eq!(finished, world.batch(&names, &levels));
    let verdicts: Vec<MonitorVerdict> = outcome
        .records
        .iter()
        .map(|record| record.verdict)
        .collect();
    assert_eq!(
        verdicts,
        vec![MonitorVerdict::Tested, MonitorVerdict::Violated]
    );
}

/// FR-643-AC-3 (TC-1666): a violation settles at the deciding step, later
/// steps report the same result, and each step names the clauses that
/// changed.
#[test]
#[trace("TC-1666", "FR-643-AC-3")]
fn a_settled_clause_never_changes_and_each_step_names_what_changed() {
    let world = world();
    let mut session = world
        .open(
            &["Never"],
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let mut changes = Vec::new();
    for level in [0, 1, 2, 1, 0] {
        let observed = text(level);
        let step = session
            .step(sample(&observed), &Cancel::new())
            .expect("a step");
        changes.push((step.changed.len(), step.statuses[0].1.is_settled()));
    }
    // The first step names the clause (its first status), the deciding third
    // step names it again, and the others name none.
    assert_eq!(
        changes,
        vec![(1, false), (0, false), (1, true), (0, true), (0, true)]
    );
}

/// FR-643-AC-5 (TC-1667): a stamp at or below the retained watermark is
/// refused with a `Watermark` contradiction and leaves the session unchanged;
/// a correct observation is then taken, and the ledger equals the batch
/// run's.
#[test]
#[trace("TC-1667", "FR-643-AC-5", "TC-1666", "FR-643-AC-2")]
fn a_regressing_stamp_is_refused_and_the_ledger_equals_the_batch_run() {
    let world = world();
    let mut session = world
        .open(
            &["T"],
            MonitorTraceKind::Timestamped,
            Some(stamped_binding()),
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let cancel = Cancel::new();
    let mut last = None;
    let mut pending_checkpoint: Option<MonitorState> = None;
    for (stamp, level) in [(0, 0), (2, 0), (5, 1), (7, 0)] {
        let observed = text(level);
        session
            .step(stamped(&observed, stamp), &cancel)
            .expect("an observation");
        if stamp == 5 {
            // The decisive observation has settled T, before G5 is asserted.
            // Its metadata-only ledger continuation survives a checkpoint.
            let checkpoint = session.checkpoint().expect("checkpoints after settlement");
            let open = || MonitorOpen {
                clauses: clauses(&["T"]),
                kind: MonitorTraceKind::Timestamped,
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: Some(stamped_binding()),
                cover: None,
                ledger: None,
                packages: &world.packages,
                sources: &world.sources,
                temporal_selections: &world.selections,
            };
            for mutation in 0..5 {
                let mut altered: serde_json::Value =
                    serde_json::from_slice(checkpoint.as_bytes()).unwrap();
                match mutation {
                    0 => altered["clauses"][0]["progress_state"] = serde_json::Value::Null,
                    1 => {
                        altered["clauses"][0]
                            .as_object_mut()
                            .unwrap()
                            .remove("progress_state");
                    }
                    2 | 3 => {
                        let mut tracker: serde_json::Value = serde_json::from_str(
                            altered["clauses"][0]["progress_state"].as_str().unwrap(),
                        )
                        .unwrap();
                        if mutation == 2 {
                            tracker["first"] = altered["seen"].clone();
                            tracker["positions"] = serde_json::json!([]);
                            tracker["fixed"] = serde_json::json!(0);
                        } else {
                            tracker["fixed"] =
                                serde_json::json!(tracker["positions"].as_array().unwrap().len());
                        }
                        altered["clauses"][0]["progress_state"] =
                            serde_json::json!(serde_json::to_string(&tracker).unwrap());
                    }
                    4 => {
                        let tracker = altered["clauses"][0]["progress_state"].as_str().unwrap();
                        let mut tracker: serde_json::Value = serde_json::from_str(tracker).unwrap();
                        tracker["positions"].as_array_mut().unwrap().pop();
                        altered = serde_json::from_slice(
                            pending_checkpoint
                                .as_ref()
                                .expect("the pending checkpoint")
                                .as_bytes(),
                        )
                        .unwrap();
                        altered["clauses"][0]["progress_state"] =
                            serde_json::json!(serde_json::to_string(&tracker).unwrap());
                    }
                    _ => unreachable!(),
                }
                let altered = MonitorState::from_bytes(serde_json::to_vec(&altered).unwrap());
                let refused = MonitorSession::resume(
                    world.unit.package(),
                    &altered,
                    open(),
                    &limits(DEFAULT_RETAINED_POSITIONS),
                    &cancel,
                );
                assert!(
                    matches!(
                        refused,
                        Err(StageFailure::Refused(MonitorRefusal::State { .. }))
                    ),
                    "checkpoint mutation {mutation} must not lose pending progress facts"
                );
            }
            session = MonitorSession::resume(
                world.unit.package(),
                &checkpoint,
                open(),
                &limits(DEFAULT_RETAINED_POSITIONS),
                &cancel,
            )
            .expect("resumes after settlement");
        }
        last = Some(
            session
                .assert_progress(progress(stamp), &cancel)
                .expect("progress"),
        );
        if stamp == 2 {
            pending_checkpoint = Some(session.checkpoint().expect("checkpoints while pending"));
            let late = text(1);
            let refused = session
                .step(stamped(&late, 2), &cancel)
                .expect_err("the stamp is at the watermark");
            let MonitorRefusal::Contradiction(contradiction) = refusal(refused) else {
                panic!("a contradiction");
            };
            assert_eq!(
                contradiction.contradiction.dimension,
                ContradictedDimension::Watermark
            );
        }
    }
    let record = last.expect("a step").statuses[0].1.record().clone();
    let outcome = session.finish(&cancel).expect("ends");
    assert_eq!(outcome.records[0].verdict, record.verdict);

    // FR-643 "Batch equality": a settled clause "SHALL report exactly B(s)".
    // The step at 5 settles under G2; later real assertions advance the
    // session ledger, independently of that frozen report.
    // FR-643 "Settling": finite-window settles on "a verdict FR-501 reports
    // with a decisive basis", so the witness settles before G5 is asserted.
    let authentication = spine::authenticate_emission(
        world.unit.package(),
        &world.selections,
        &world.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .expect("emits");
    let observations: Vec<TimestampedObservation> = [(0, 0), (2, 0), (5, 1)]
        .into_iter()
        .map(|(stamp, level)| TimestampedObservation {
            document: spine::sample_position(&text(level)),
            stamp,
            order_key: None,
        })
        .collect();
    let trace = TraceInput::Timestamped {
        observations,
        progress: progress(2),
        window: Box::new(TimestampedWindow {
            binding: Some(stamped_binding()),
            authoritative_origin: true,
            cover: None,
            missing_valuations: Vec::new(),
            late: Default::default(),
        }),
    };
    let request = PackageTemporalRequest {
        package: world.unit.package(),
        packages: &world.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name: "T",
        trace: &trace,
        over: None,
        ledger: None,
        sources: &world.sources,
        authentication: Some(PackageAuthentication::temporal(&authentication)),
        cancel: &cancel,
    };
    let batch = run_package_temporal(&request);
    let from_session = outcome.records[0].report();
    assert_eq!(
        qsl_2750_report_fields(&from_session),
        qsl_2750_report_fields(&batch)
    );
    assert!(batch.ledger.is_some());
    let mut latest = trace.clone();
    let TraceInput::Timestamped {
        observations,
        progress: assertion,
        ..
    } = &mut latest
    else {
        panic!("the timestamped trace");
    };
    observations.push(TimestampedObservation {
        document: spine::sample_position(&text(0)),
        stamp: 7,
        order_key: None,
    });
    *assertion = progress(7);
    let latest_batch = run_package_temporal(&PackageTemporalRequest {
        trace: &latest,
        ..request
    });
    assert_eq!(outcome.ledger, latest_batch.ledger);
}

/// FR-643 Order (TC-1667): a progress assertion that regresses the retained
/// watermark is refused and the session keeps its state.
#[test]
#[trace("TC-1667", "FR-643-AC-5")]
fn a_regressing_progress_assertion_is_refused() {
    let world = world();
    let mut session = world
        .open(
            &["T"],
            MonitorTraceKind::Timestamped,
            Some(stamped_binding()),
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let cancel = Cancel::new();
    let first = text(0);
    session.step(stamped(&first, 3), &cancel).expect("steps");
    session
        .assert_progress(progress(3), &cancel)
        .expect("progress");
    let refused = session
        .assert_progress(progress(1), &cancel)
        .expect_err("the watermark regresses");
    assert!(matches!(refusal(refused), MonitorRefusal::Contradiction(_)));
    let next = text(1);
    session
        .step(stamped(&next, 4), &cancel)
        .expect("still steps");
}

/// FR-643-AC-2 (TC-1666): a progressed trace is monitored with progress
/// asserted after each observation.
#[test]
#[trace("TC-1666", "FR-643-AC-2")]
fn a_progressed_trace_is_monitored() {
    let world = world();
    let mut session = world
        .open(
            &["E"],
            MonitorTraceKind::Progressed,
            Some(events_binding()),
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let cancel = Cancel::new();
    let mut verdicts = Vec::new();
    for (index, level) in [0, 1, 2].into_iter().enumerate() {
        let observed = text(level);
        session.step(sample(&observed), &cancel).expect("steps");
        let step = session
            .assert_progress(progress(u64::try_from(index).expect("index")), &cancel)
            .expect("progress");
        verdicts.push(step.statuses[0].1.record().verdict);
    }
    assert_eq!(
        verdicts.last(),
        Some(&MonitorVerdict::Tested),
        "{verdicts:?}"
    );
}

/// FR-643-AC-4 (TC-1667): a checkpoint after step 2, resumed, gives the
/// reports of the uninterrupted session; another clause list refuses.
#[test]
#[trace("TC-1667", "FR-643-AC-4")]
fn a_resumed_checkpoint_continues_as_the_original_does() {
    let world = world();
    let names = ["Reaches", "Never"];
    let cancel = Cancel::new();
    let levels = [0, 1, 2, 1, 0];
    let mut whole = world
        .open(
            &names,
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let mut head = world
        .open(
            &names,
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    for level in &levels[..2] {
        let observed = text(*level);
        whole.step(sample(&observed), &cancel).expect("steps");
        head.step(sample(&observed), &cancel).expect("steps");
    }
    let state = head.checkpoint().expect("checkpoints");
    let request = |names: &[&str]| MonitorOpen {
        clauses: clauses(names),
        kind: MonitorTraceKind::Finite,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: None,
        cover: None,
        ledger: None,
        packages: &world.packages,
        sources: &world.sources,
        temporal_selections: &world.selections,
    };
    let mut resumed = MonitorSession::resume(
        world.unit.package(),
        &state,
        request(&names),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("resumes");
    for level in &levels[2..] {
        let observed = text(*level);
        let original = whole.step(sample(&observed), &cancel).expect("steps");
        let again = resumed.step(sample(&observed), &cancel).expect("steps");
        let verdicts = |step: &fsl_analyze::monitor_session::MonitorStep| {
            step.statuses
                .iter()
                .map(|(_, status)| {
                    let report = status.record().report();
                    (
                        status.record().verdict,
                        status.is_settled(),
                        format!("{:?}", report.disposition),
                        format!("{:?}", report.basis),
                        format!("{:?}", report.finite_window),
                        format!("{:?}", report.ledger),
                        status.settlement().cloned(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(verdicts(&original), verdicts(&again));
    }
    let other = MonitorSession::resume(
        world.unit.package(),
        &state,
        request(&["Never"]),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    );
    assert!(matches!(
        other.err().map(refusal),
        Some(MonitorRefusal::State { .. })
    ));
    let garbled = fsl_analyze::monitor_session::MonitorState::from_bytes(b"{}".to_vec());
    assert!(matches!(
        MonitorSession::resume(
            world.unit.package(),
            &garbled,
            request(&names),
            &limits(DEFAULT_RETAINED_POSITIONS),
            &cancel,
        )
        .err()
        .map(refusal),
        Some(MonitorRefusal::State { .. })
    ));
}

/// FR-645-AC-3 (TC-1671): a kind the clause's profile does not read refuses at
/// open, before any observation, with `trace-kind-mismatch`.
#[test]
#[trace("TC-1671", "FR-645-AC-3")]
fn a_kind_the_profile_does_not_read_refuses_at_open() {
    let world = world();
    let refused = world
        .open(
            &["T"],
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .err()
        .map(refusal);
    let Some(refused) = refused else {
        panic!("the open should refuse");
    };
    assert!(matches!(refused, MonitorRefusal::ClauseRefused { .. }));
    let document = serde_json::to_value(refused.document()).expect("a document");
    assert!(
        document["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("reason=trace-kind-mismatch")),
        "{document}"
    );
}

fn category(
    step: &fsl_analyze::monitor_session::MonitorStep,
    index: usize,
) -> filament_semantic_value::catalog::Category {
    step.statuses[index].1.record().category
}

/// FR-643-AC-6 (TC-2321): `Never` settles `violation` at step 3 and then
/// reports `B(3)`. `Reaches` reports `B(k)` while pending and settles
/// `success` at step 3, its good prefix (FR-328-AC-7, FR-643's amended
/// infinite-trace rule), then reports `B(3)`.
#[test]
#[trace("TC-2321", "FR-643-AC-6")]
fn never_settles_at_the_bad_prefix_and_reaches_at_its_good_prefix() {
    let world = world();
    let names = ["Reaches", "Never"];
    let levels = [0, 1, 2, 1];
    let cancel = Cancel::new();
    let mut session = world
        .open(
            &names,
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let mut at_three = None;
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        let step = session.step(sample(&observed), &cancel).expect("steps");
        assert_eq!(
            step.statuses[0].1.is_settled(),
            index >= 2,
            "Reaches at {index}"
        );
        assert_eq!(
            format!("{:?}", step.statuses[0].1.record().report().disposition),
            world.batch(&names, &levels[..=index.min(2)])[0]
        );
        let never = &step.statuses[1].1;
        assert_eq!(never.is_settled(), index >= 2, "Never at {index}");
        match index {
            2 => at_three = Some(format!("{:?}", never.record().report().disposition)),
            3 => {
                assert_eq!(
                    Some(format!("{:?}", never.record().report().disposition)),
                    at_three,
                    "the settled report is B(3)"
                );
                assert_ne!(
                    format!("{:?}", never.record().report().counterexample),
                    format!("{:?}", world.batch_report("Never", &levels).counterexample),
                    "B(4) lists four samples, the settled report three"
                );
                assert_eq!(
                    category(&step, 1),
                    filament_semantic_value::catalog::Category::Violation
                );
            }
            _ => {}
        }
    }
    let outcome = session.finish(&cancel).expect("ends");
    assert_eq!(
        format!("{:?}", outcome.records[0].report().disposition),
        world.batch(&["Reaches"], &levels[..3])[0]
    );
    assert_eq!(settled_before_end(&outcome), vec![true, true]);
}

/// FR-643-AC-7 (TC-2321): the event-position `E` stays `Pending` over levels
/// 0, 1 while the closed reading is `violation`; it settles at step 3 with
/// exactly `B(3)` (no open-scope `finite_window`, no ledger) and a
/// settlement naming the reading that decided it. A `finish` over 0, 1
/// settles it at the end.
#[test]
#[trace("TC-2321", "FR-643-AC-7")]
fn a_closed_trace_clause_settles_on_b_s_and_records_its_deciding_reading() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    for (levels, category_at_three, basis, position) in [
        (
            [0, 1, 2],
            Category::Success,
            fsl_replay::SettlementBasis::DecisiveWitness,
            2,
        ),
        (
            [0, 1, 1],
            Category::Violation,
            fsl_replay::SettlementBasis::DecisiveCounterexample,
            0,
        ),
    ] {
        let mut session = world
            .open(
                &["E"],
                MonitorTraceKind::Finite,
                None,
                DEFAULT_RETAINED_POSITIONS,
            )
            .expect("opens");
        for (index, level) in levels.iter().enumerate() {
            let observed = text(*level);
            let step = session.step(sample(&observed), &cancel).expect("steps");
            let status = &step.statuses[0].1;
            if index < 2 {
                assert!(!status.is_settled(), "pending at step {}", index + 1);
                assert_eq!(
                    status.record().category,
                    Category::Violation,
                    "closed reading"
                );
            } else {
                assert!(status.is_settled(), "{levels:?} step 3");
                assert_eq!(status.record().category, category_at_three);
                let report = status.record().report();
                let batch = world.batch_report("E", &levels);
                assert_eq!(
                    format!("{:?}", report.disposition),
                    format!("{:?}", batch.disposition),
                    "the settled report is B(3)"
                );
                assert_eq!(report.basis, batch.basis);
                assert_eq!(report.basis, fsl_replay::SettlementBasis::ClosedScope);
                assert!(report.finite_window.is_none(), "no open-scope window");
                assert!(report.ledger.is_none(), "no ledger on a Finite trace");
                assert_eq!(
                    status.settlement(),
                    Some(&fsl_analyze::monitor::Settlement {
                        step: 3,
                        trigger: fsl_analyze::monitor::SettleTrigger::Decided,
                        reading: Some(fsl_analyze::monitor::DecidingReading {
                            basis,
                            horizon: 2,
                            position: Some(position),
                        }),
                    })
                );
            }
        }
    }
    let mut session = world
        .open(
            &["E"],
            MonitorTraceKind::Finite,
            None,
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    for level in [0, 1] {
        session.step(sample(&text(level)), &cancel).expect("steps");
    }
    let outcome = session.finish(&cancel).expect("ends");
    assert_eq!(
        outcome.settlements,
        vec![fsl_analyze::monitor::Settlement {
            step: 2,
            trigger: fsl_analyze::monitor::SettleTrigger::Finish,
            reading: None,
        }]
    );
    assert_eq!(
        format!("{:?}", outcome.records[0].report().disposition),
        format!("{:?}", world.batch_report("E", &[0, 1]).disposition)
    );
    assert!(outcome.ledger.is_none());
}

/// FR-283-AC-17 (TC-2323), FR-820: a `Finite` session over an event-position
/// clause needs no binding or clock selection; with empty
/// `temporal_selections` it opens and reports what the batch `Finite` run
/// reports at every prefix.
#[test]
#[trace("TC-2323", "FR-283-AC-17")]
fn a_finite_session_needs_no_selection_for_a_closed_trace_clause() {
    let world = world();
    let cancel = Cancel::new();
    let mut session = MonitorSession::open(
        world.unit.package(),
        MonitorOpen {
            clauses: clauses(&["E"]),
            kind: MonitorTraceKind::Finite,
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: None,
            cover: None,
            ledger: None,
            packages: &world.packages,
            sources: &world.sources,
            temporal_selections: &[],
        },
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("opens without a selection");
    let levels = [0, 1, 2];
    for (index, level) in levels.iter().enumerate() {
        let step = session.step(sample(&text(*level)), &cancel).expect("steps");
        assert_eq!(
            format!("{:?}", step.statuses[0].1.record().report().disposition),
            world.batch(&["E"], &levels[..=index])[0],
            "prefix {index}"
        );
    }
    let outcome = session.finish(&cancel).expect("ends");
    assert_eq!(
        format!("{:?}", outcome.records[0].report().disposition),
        world.batch(&["E"], &levels)[0]
    );
}

/// FR-643 Order: an opening ledger that holds watermark 5 refuses an
/// observation stamped 3, and an assertion that regresses it, even when the
/// run is skipped.
#[test]
#[trace("TC-1667", "FR-643-AC-5")]
fn the_opening_ledger_orders_stamps_and_assertions() {
    let world = world();
    let cancel = Cancel::new();
    let open = |ledger| {
        MonitorSession::open(
            world.unit.package(),
            MonitorOpen {
                clauses: clauses(&["T"]),
                kind: MonitorTraceKind::Timestamped,
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: Some(stamped_binding()),
                cover: None,
                ledger,
                packages: &world.packages,
                sources: &world.sources,
                temporal_selections: &world.selections,
            },
            &limits(DEFAULT_RETAINED_POSITIONS),
            &cancel,
        )
        .expect("opens")
    };
    // A ledger holding watermark 5 comes from a session that asserted it.
    let mut earlier = open(None);
    earlier.step(stamped(&text(1), 5), &cancel).expect("steps");
    let held = earlier
        .assert_progress(progress(5), &cancel)
        .expect("asserts");
    let ledger = earlier
        .finish(&cancel)
        .expect("ends")
        .ledger
        .expect("a ledger");
    drop(held);
    let mut session = open(Some(ledger.clone()));
    let refused = session
        .step(stamped(&text(1), 3), &cancel)
        .expect_err("a stamp at or below the ledger's watermark");
    assert!(matches!(refusal(refused), MonitorRefusal::Contradiction(_)));
    let mut session = open(Some(ledger));
    session.step(stamped(&text(1), 10), &cancel).expect("steps");
    let refused = session
        .assert_progress(progress(2), &cancel)
        .expect_err("a regressing assertion");
    assert!(matches!(refusal(refused), MonitorRefusal::Contradiction(_)));
}

/// FR-643 Checkpoint: after a step at a stamp above the watermark the clause
/// keeps its earlier pending-window report, and a resumed session reports the
/// same.
#[test]
#[trace("TC-1667", "FR-643-AC-4")]
fn a_resumed_timestamped_session_keeps_the_pending_window_report() {
    use fsl_replay::spine::{ClauseDisposition, TemporalVerdict};
    let world = world();
    let cancel = Cancel::new();
    let request = || MonitorOpen {
        clauses: clauses(&["T"]),
        kind: MonitorTraceKind::Timestamped,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: Some(stamped_binding()),
        cover: None,
        ledger: None,
        packages: &world.packages,
        sources: &world.sources,
        temporal_selections: &world.selections,
    };
    let mut original = MonitorSession::open(
        world.unit.package(),
        request(),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("opens");
    original.step(stamped(&text(0), 0), &cancel).expect("step");
    original
        .assert_progress(progress(2), &cancel)
        .expect("asserts");
    let step = original.step(stamped(&text(0), 3), &cancel).expect("step");
    let expected = |step: &fsl_analyze::monitor_session::MonitorStep| {
        matches!(
            step.statuses[0].1.record().report().disposition,
            ClauseDisposition::Temporal(TemporalVerdict::PendingWindow { at: 0, deadline: 5 })
        )
    };
    assert!(expected(&step), "the original keeps PendingWindow");
    let state = original.checkpoint().expect("checkpoints");
    let mut resumed = MonitorSession::resume(
        world.unit.package(),
        &state,
        request(),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("resumes");
    let after = resumed.step(stamped(&text(0), 4), &cancel).expect("step");
    let reference = original.step(stamped(&text(0), 4), &cancel).expect("step");
    assert_eq!(
        format!("{:?}", after.statuses[0].1.record().report().disposition),
        format!(
            "{:?}",
            reference.statuses[0].1.record().report().disposition
        )
    );
    let held = MonitorSession::resume(
        world.unit.package(),
        &state,
        request(),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("resumes")
    .finish(&cancel)
    .expect("ends");
    assert!(matches!(
        held.records[0].report().disposition,
        ClauseDisposition::Temporal(TemporalVerdict::PendingWindow { at: 0, deadline: 5 })
    ));
}

/// FR-643-AC-11 (TC-2324): `retained_identities` ends a pending clause
/// `incomplete` while a settled clause keeps its report.
#[test]
#[trace("TC-2324", "FR-643-AC-11")]
fn retained_identities_end_a_pending_clause_incomplete() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    let mut session = MonitorSession::open(
        world.unit.package(),
        MonitorOpen {
            clauses: clauses(&["Never", "Below"]),
            kind: MonitorTraceKind::Finite,
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: None,
            cover: None,
            ledger: None,
            packages: &world.packages,
            sources: &world.sources,
            temporal_selections: &world.selections,
        },
        &MonitorLimits {
            retained_identities: 3,
            ..limits(DEFAULT_RETAINED_POSITIONS)
        },
        &cancel,
    )
    .expect("opens");
    let mut steps = Vec::new();
    for level in [0, 0, 2, 0, 1] {
        let observed = text(level);
        steps.push(session.step(sample(&observed), &cancel).expect("steps"));
    }
    let at_four = &steps[3];
    let step = &steps[4];
    assert_eq!(category(step, 0), Category::Violation);
    assert_eq!(category(step, 1), Category::Incomplete);
    assert!(step.statuses.iter().all(|(_, status)| status.is_settled()));
    // `Never` keeps B(3), whose counterexample lists three positions.
    let never = step.statuses[0].1.record().report();
    assert_eq!(
        format!("{:?}", never.disposition),
        format!("{:?}", world.batch_report("Never", &[0, 0, 2]).disposition)
    );
    assert_eq!(
        never
            .counterexample
            .as_ref()
            .map(|c| super::monitor::observed_counterexample(c).prefix.len()),
        world
            .batch_report("Never", &[0, 0, 2])
            .counterexample
            .as_ref()
            .map(|c| super::monitor::observed_counterexample(c).prefix.len())
    );
    // `Below` ended on `monitor.retained_identities` at step 4 and a further
    // observation leaves its report unchanged.
    let below = at_four.statuses[1].1.record().report();
    assert!(
        format!("{:?}", below.disposition).contains("monitor.retained_identities"),
        "{:?}",
        below.disposition
    );
    assert_eq!(
        format!("{:?}", below.disposition),
        format!("{:?}", step.statuses[1].1.record().report().disposition)
    );
    assert!(!steps[4].changed.iter().any(|name| name == "Below"));
}

/// FR-643-AC-5 (TC-1667): with `retained_positions` 2 over levels 0, 1, 0 the
/// event-position `Recent` reports `incomplete` while `Reaches` and `Never`
/// stay `Pending`. It waits on FR-820's value pruning.
#[test]
#[trace("TC-1667", "FR-643-AC-5")]
fn recent_reports_incomplete_at_the_retained_positions_limit() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    let mut session = world
        .open(
            &["Recent", "Reaches", "Never"],
            MonitorTraceKind::Finite,
            None,
            2,
        )
        .expect("opens");
    let mut last = None;
    for level in [0, 1, 0] {
        let observed = text(level);
        last = Some(session.step(sample(&observed), &cancel).expect("steps"));
    }
    let step = last.expect("a step");
    assert_eq!(category(&step, 0), Category::Incomplete);
    assert_eq!(
        step.statuses[0].1.settlement(),
        Some(&fsl_analyze::monitor::Settlement {
            step: 3,
            trigger: fsl_analyze::monitor::SettleTrigger::Limit,
            reading: None,
        })
    );
    assert_eq!(category(&step, 1), Category::Pending);
    assert_eq!(category(&step, 2), Category::Pending);
}

/// FR-643-AC-5 (TC-1667): once progress with scope `closed` is retained, a
/// further observation is refused on `Closure` and the session is unchanged.
#[test]
#[trace("TC-1667", "FR-643-AC-5")]
fn an_observation_after_a_closed_scope_is_refused() {
    let world = world();
    let cancel = Cancel::new();
    let mut session = world
        .open(
            &["T"],
            MonitorTraceKind::Timestamped,
            Some(stamped_binding()),
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let first = text(1);
    session.step(stamped(&first, 0), &cancel).expect("steps");
    session
        .assert_progress(
            ProgressAssertion {
                scope: ProgressClosure::Closed,
                ..progress(10)
            },
            &cancel,
        )
        .expect("progress");
    let next = text(0);
    let MonitorRefusal::Contradiction(contradiction) = refusal(
        session
            .step(stamped(&next, 11), &cancel)
            .expect_err("the scope is closed"),
    ) else {
        panic!("a contradiction");
    };
    assert_eq!(
        contradiction.contradiction.dimension,
        ContradictedDimension::Closure
    );
}

/// FR-643-AC-4 (TC-1667): a finite-window clause settled under an earlier
/// progress assertion resumes, from a checkpoint taken after a later one, to
/// the same settled report and the same later reports.
#[test]
#[trace("TC-1667", "FR-643-AC-4")]
fn a_clause_settled_under_an_earlier_assertion_resumes_to_the_same_report() {
    let world = world();
    let cancel = Cancel::new();
    let open = || MonitorOpen {
        clauses: clauses(&["T"]),
        kind: MonitorTraceKind::Timestamped,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: Some(stamped_binding()),
        cover: None,
        ledger: None,
        packages: &world.packages,
        sources: &world.sources,
        temporal_selections: &world.selections,
    };
    let mut original = MonitorSession::open(
        world.unit.package(),
        open(),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("opens");
    let first = text(1);
    original.step(stamped(&first, 0), &cancel).expect("steps");
    let settled = original
        .assert_progress(
            ProgressAssertion {
                scope: ProgressClosure::Closed,
                ..progress(5)
            },
            &cancel,
        )
        .expect("progress");
    assert!(
        settled.statuses[0].1.is_settled(),
        "settles under the closed scope"
    );
    let at_settle = format!(
        "{:?}",
        settled.statuses[0].1.record().report().finite_window
    );
    original
        .assert_progress(
            ProgressAssertion {
                scope: ProgressClosure::Closed,
                ..progress(8)
            },
            &cancel,
        )
        .expect("a later assertion");
    let state = original.checkpoint().expect("checkpoints");
    let mut resumed = MonitorSession::resume(
        world.unit.package(),
        &state,
        open(),
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("resumes");
    for session in [&mut original, &mut resumed] {
        let step = session
            .assert_progress(
                ProgressAssertion {
                    scope: ProgressClosure::Closed,
                    ..progress(9)
                },
                &cancel,
            )
            .expect("a further assertion");
        let status = &step.statuses[0].1;
        assert!(status.is_settled());
        assert_eq!(
            format!("{:?}", status.record().report().finite_window),
            at_settle,
            "the settled report is the one of the deciding assertion"
        );
    }
    let kept = original.finish(&cancel).expect("ends");
    let again = resumed.finish(&cancel).expect("ends");
    assert_eq!(
        format!("{:?}", kept.records[0].report().disposition),
        format!("{:?}", again.records[0].report().disposition)
    );
}

/// FR-643-AC-8 (TC-2322): a `Progressed` session whose latest assertion has
/// scope `open` and execution `closed` finishes with `B(n)` under it for the
/// unsettled clause.
#[test]
#[trace("TC-2322", "FR-643-AC-8")]
fn a_progressed_finish_returns_the_batch_report_under_the_latest_assertion() {
    let world = world();
    let cancel = Cancel::new();
    let mut session = world
        .open(
            &["E"],
            MonitorTraceKind::Progressed,
            Some(events_binding()),
            DEFAULT_RETAINED_POSITIONS,
        )
        .expect("opens");
    let levels = [0, 1];
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        session.step(sample(&observed), &cancel).expect("steps");
        session
            .assert_progress(progress(u64::try_from(index).expect("index")), &cancel)
            .expect("progress");
    }
    let latest = ProgressAssertion {
        execution: ProgressClosure::Closed,
        ..progress(1)
    };
    let step = session.assert_progress(latest, &cancel).expect("progress");
    assert!(!step.statuses[0].1.is_settled());
    let outcome = session.finish(&cancel).expect("ends");

    let authentication = spine::authenticate_emission(
        world.unit.package(),
        &world.selections,
        &world.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .expect("emits");
    let trace = TraceInput::Progressed {
        trace: levels
            .iter()
            .map(|level| spine::sample_position(&text(*level)))
            .collect(),
        progress: latest,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: events_binding(),
    };
    let batch = run_package_temporal(&PackageTemporalRequest {
        package: world.unit.package(),
        packages: &world.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name: "E",
        trace: &trace,
        over: None,
        ledger: None,
        sources: &world.sources,
        authentication: Some(PackageAuthentication::temporal(&authentication)),
        cancel: &cancel,
    });
    assert_eq!(
        format!("{:?}", outcome.records[0].report().disposition),
        format!("{:?}", batch.disposition)
    );
    assert_eq!(settled_before_end(&outcome), vec![false]);
}

/// For each clause, whether it settled before the end of the trace.
fn settled_before_end(outcome: &fsl_analyze::monitor::MonitorOutcome) -> Vec<bool> {
    outcome
        .settlements
        .iter()
        .map(|settlement| settlement.trigger != fsl_analyze::monitor::SettleTrigger::Finish)
        .collect()
}

/// FR-643-AC-1 (TC-1666) over snapshots: `Bounded` (event-position), `Reaches`
/// and `always holds(c.value <= 1)` fed `c.value` 0, 1, 2, 1, 0 one at a time
/// in a `Finite` session report `B(k)` while pending and `B(s)` once settled
/// (`Bounded` at step 2, the `always` clause at step 3, and `Reaches` at step 3,
/// its good prefix).
#[test]
#[trace("TC-1666", "FR-643-AC-1")]
fn snapshots_fed_one_at_a_time_equal_the_batch_with_settled_clauses_frozen() {
    use crate::counter::{document, mixed_package_with_clauses, Documents};
    use fsl_replay::spine::OverObject;
    let document = document(false);
    let (package, source) = mixed_package_with_clauses(
        &document,
        &[("Bounded", "", "eventually[0,1] holds(c.value = 2)")],
        &[
            ("Reaches", "", "eventually holds(c.value = 2)"),
            ("Small", "", "always holds(c.value <= 1)"),
        ],
    );
    let mut documents = Documents::new(document);
    let values = [0, 1, 2, 1, 0];
    let refs: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s-{index}"), &[("c1", *value)]))
        .collect();
    let bytes: Vec<Vec<u8>> = refs
        .iter()
        .map(|reference| {
            documents
                .snapshot_bytes()
                .find(|(held, _)| held.identity == reference.identity)
                .map(|(_, bytes)| bytes.clone())
                .expect("a provisioned snapshot")
        })
        .collect();
    let names = ["Bounded", "Reaches", "Small"];
    let over = || OverObject {
        population: "ix://test/counter/counters".to_owned(),
        key: "c1".to_owned(),
    };
    let cancel = Cancel::new();
    let mut session = MonitorSession::open(
        &package,
        MonitorOpen {
            clauses: names
                .iter()
                .map(|name| MonitorClause {
                    name: (*name).to_owned(),
                    over: Some(over()),
                })
                .collect(),
            kind: MonitorTraceKind::Finite,
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: None,
            cover: None,
            ledger: None,
            packages: &documents.packages,
            sources: std::slice::from_ref(&source),
            temporal_selections: &[],
        },
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("opens");
    let batch = |name: &str, upto: usize| {
        let trace = TraceInput::Finite(refs[..upto].to_vec());
        let snapshots: DocumentProvision = documents
            .snapshot_bytes()
            .map(|(reference, bytes)| (reference.clone(), bytes.clone()))
            .collect();
        let over = over();
        run_package_temporal(&PackageTemporalRequest {
            package: &package,
            packages: &documents.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &snapshots,
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name,
            trace: &trace,
            over: Some(&over),
            ledger: None,
            sources: std::slice::from_ref(&source),
            authentication: None,
            cancel: &Cancel::new(),
        })
    };
    let settles = [Some(2), Some(3), Some(3)];
    for (index, snapshot) in bytes.iter().enumerate() {
        let step = session
            .step(
                MonitorObservation {
                    position: MonitorPosition::Snapshot(snapshot),
                    stamp: None,
                    order_key: None,
                    missing: &[],
                },
                &cancel,
            )
            .expect("steps");
        for (clause, name) in names.iter().enumerate() {
            let at = match settles[clause] {
                Some(settled) if settled <= index + 1 => settled,
                _ => index + 1,
            };
            let status = &step.statuses[clause].1;
            assert_eq!(
                status.is_settled(),
                settles[clause].is_some_and(|settled| settled <= index + 1),
                "{name} after step {}",
                index + 1
            );
            assert_eq!(
                format!("{:?}", status.record().report().disposition),
                format!("{:?}", batch(name, at).disposition),
                "{name} after step {}",
                index + 1
            );
        }
    }
    let outcome = session.finish(&cancel).expect("ends");
    assert_eq!(settled_before_end(&outcome), vec![true, true, true]);
}

/// FR-283-AC-17 (TC-2323): a `Finite` session ignores the selection list
/// whatever it holds: a list that does not select `E`'s clock still opens and
/// reports what the batch `Finite` run reports.
#[test]
#[trace("TC-2323", "FR-283-AC-17")]
fn a_finite_session_ignores_a_non_matching_selection_list() {
    let world = world();
    let cancel = Cancel::new();
    let others = &world.selections[1..];
    assert!(others.len() < world.selections.len());
    let mut session = MonitorSession::open(
        world.unit.package(),
        MonitorOpen {
            clauses: clauses(&["E"]),
            kind: MonitorTraceKind::Finite,
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: None,
            cover: None,
            ledger: None,
            packages: &world.packages,
            sources: &world.sources,
            temporal_selections: others,
        },
        &limits(DEFAULT_RETAINED_POSITIONS),
        &cancel,
    )
    .expect("opens whatever the selections are");
    let levels = [0, 1, 2];
    for (index, level) in levels.iter().enumerate() {
        let step = session.step(sample(&text(*level)), &cancel).expect("steps");
        assert_eq!(
            format!("{:?}", step.statuses[0].1.record().report().disposition),
            world.batch(&["E"], &levels[..=index])[0],
            "prefix {index}"
        );
    }
}

// FR-820 (TC-2320, TC-2441): FR-643's sessions are built on the carried
// clause state, so its own properties are tested here, over the unit above.

use fsl_replay::spine::{
    AdmittedPosition, AdmittedValue, ClauseLimits, ClauseOpen, ClauseRunReport, ClauseState,
    ClauseTraceKind,
};

fn clause_node<'a>(world: &'a World, name: &str) -> &'a fsl_semantics::check::TemporalClauseNode {
    world
        .unit
        .package()
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
        .expect("the clause is declared")
}

fn clause_open<'a>(
    world: &'a World,
    kind: ClauseTraceKind,
    binding: Option<TraceBinding>,
    lower_boundary: ProgressLowerBoundary,
) -> ClauseOpen<'a> {
    ClauseOpen {
        kind,
        lower_boundary,
        binding,
        cover: None,
        ledger: None,
        sources: &world.sources,
        authentication: None,
        packages: &world.packages,
        over: None,
        limits: ClauseLimits {
            model: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            accounting: UNLIMITED,
            max_window_values: fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
            covered_positions: fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        },
    }
}

fn finite_state<'a>(world: &'a World, name: &str) -> ClauseState<'a> {
    ClauseState::open(
        world.unit.package(),
        clause_node(world, name),
        &clause_open(
            world,
            ClauseTraceKind::Finite,
            None,
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens")
}

/// The record sample `text` of the clause `name`'s subject at `index`.
fn admitted(world: &World, name: &str, text: &str, index: u64) -> AdmittedPosition<'static> {
    let filament_exact::ValueType::Composite(record) = &clause_node(world, name).clause().over().1
    else {
        panic!("a record subject");
    };
    let value = fsl_semantics::model::observation::admit_record_sample(
        world.unit.package().graph().scope().types(),
        record,
        text.as_bytes(),
    )
    .expect("the sample admits");
    AdmittedPosition {
        reference: spine::sample_position(text),
        stamp: index,
        order_key: None,
        value: AdmittedValue::Sample(value),
        absent: Vec::new(),
    }
}

/// Every member of a report but its usage, which counts the work the run
/// itself did.
fn members(report: &ClauseRunReport) -> String {
    format!(
        "{:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?}",
        report.disposition,
        report.basis,
        report.counterexample,
        report.decisive,
        report.finite_window,
        report.refused_at,
        report.ledger,
        report.witness.is_some(),
        report.consequent_source_events,
    )
}

/// FR-820-AC-1 (TC-2320): `Bounded`, `Reaches` and `always holds(level <= 1)`
/// over levels 0, 1, 2, 1, 0: the report after each `advance` is FR-330's
/// over the prefix of that length, member for member.
#[test]
#[trace("TC-2320", "FR-820-AC-1")]
fn each_advance_reports_the_batch_report_over_the_prefix() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [0, 1, 2, 1, 0];
    for name in ["Bounded", "Reaches", "AtMostOne"] {
        let mut state = finite_state(&world, name);
        for (index, level) in levels.iter().enumerate() {
            let observed = text(*level);
            let advance = state
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            let batch = world.batch_report(name, &levels[..=index]);
            assert_eq!(
                members(&advance.report),
                members(&batch),
                "{name} after position {index}"
            );
        }
    }
    // The violations carry their counterexamples, built from the identities.
    let mut never = finite_state(&world, "AtMostOne");
    let mut last = None;
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        last = Some(
            never
                .advance(
                    admitted(&world, "AtMostOne", &observed, index as u64),
                    &cancel,
                )
                .expect("advances"),
        );
    }
    let report = last.expect("a report").report;
    assert_eq!(
        report
            .counterexample
            .as_ref()
            .map(|c| super::monitor::observed_counterexample(c).prefix.len()),
        Some(5)
    );
}

/// FR-820-AC-2 (TC-2320): over 1,000 samples cycling 0 to 8 the
/// infinite-trace clauses hold no value, the window clause holds the
/// `min(k, 4)` values of its window, and every report is FR-330's; over 2,
/// 0, 2 the window clause reports `violation` at position 2.
#[test]
#[trace("TC-2320", "FR-820-AC-2")]
fn a_state_holds_only_the_values_its_clause_reads() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    let levels: Vec<i64> = (0..1000).map(|k| k % 9).collect();
    let mut below = finite_state(&world, "Below");
    let mut nine = finite_state(&world, "Nine");
    let mut recent = finite_state(&world, "Recent");
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        let k = index as u64 + 1;
        for (name, state) in [("Below", &mut below), ("Nine", &mut nine)] {
            let advance = state
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            assert_eq!(state.retained(), 0, "{name} after position {k}");
            if index % 97 == 0 || index == levels.len() - 1 {
                assert_eq!(
                    members(&advance.report),
                    members(&world.batch_report(name, &levels[..=index])),
                    "{name} after position {k}"
                );
            }
        }
        // `Recent`'s activation `always[0,9]` is open through position 9.
        if index < 10 {
            let advance = recent
                .advance(admitted(&world, "Recent", &observed, index as u64), &cancel)
                .expect("advances");
            assert_eq!(recent.retained(), k.min(4), "Recent after position {k}");
            assert_eq!(
                members(&advance.report),
                members(&world.batch_report("Recent", &levels[..=index])),
                "Recent after position {k}"
            );
        }
    }
    assert_eq!(
        world.batch_report("Recent", &levels[..10]).category(),
        Category::Success
    );
    let mut window = finite_state(&world, "Recent");
    let mut last = None;
    for (index, level) in [2, 0, 2].iter().enumerate() {
        let observed = text(*level);
        last = Some(
            window
                .advance(admitted(&world, "Recent", &observed, index as u64), &cancel)
                .expect("advances"),
        );
    }
    let report = last.expect("a report").report;
    assert_eq!(report.category(), Category::Violation);
    assert_eq!(
        members(&report),
        members(&world.batch_report("Recent", &[2, 0, 2]))
    );
}

/// FR-820-AC-3 (TC-2320): a state written after position 2 and read back
/// reports, for positions 3 and 4, what the uninterrupted state reports; a
/// cancelled `advance` leaves the bytes unchanged.
#[test]
#[trace("TC-2320", "FR-820-AC-3")]
fn a_state_read_back_from_its_bytes_continues_as_the_original() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [0, 1, 2, 1, 0];
    for name in ["Bounded", "Reaches", "AtMostOne", "Recent"] {
        let mut original = finite_state(&world, name);
        for (index, level) in levels[..3].iter().enumerate() {
            let observed = text(*level);
            original
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
        }
        let bytes = original.to_bytes().expect("writes");
        let stopped = Cancel::new();
        stopped.cancel(filament_exact::CancelCause::Requested);
        let observed = text(levels[3]);
        assert!(original
            .advance(admitted(&world, name, &observed, 3), &stopped)
            .is_err());
        assert_eq!(original.to_bytes().expect("writes"), bytes, "{name}");
        let mut resumed = ClauseState::from_bytes(
            world.unit.package(),
            clause_node(&world, name),
            &clause_open(
                &world,
                ClauseTraceKind::Finite,
                None,
                ProgressLowerBoundary::AuthoritativeOrigin,
            ),
            &bytes,
            &cancel,
        )
        .expect("reads back");
        assert_eq!(
            members(&resumed.report()),
            members(&original.report()),
            "{name}"
        );
        for (index, level) in levels.iter().enumerate().take(5).skip(3) {
            let observed = text(*level);
            let a = original
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            let b = resumed
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            assert_eq!(members(&a.report), members(&b.report), "{name} at {index}");
            assert_eq!(resumed.retained(), original.retained(), "{name} at {index}");
        }
    }
}

fn samples_binding() -> TraceBinding {
    TraceBinding {
        profile: SAMPLES.to_owned(),
        clock_name: "samples".to_owned(),
        parameters: ClockConfiguration::FixedSample {
            epoch: fsl_package::ClockExactNumber::Integer {
                decimal: "0".into(),
            },
            period: fsl_package::ClockExactNumber::Integer {
                decimal: "1".into(),
            },
            unit: "sample".into(),
        },
    }
}

/// FR-820-AC-4 (TC-2320): `E` over a `Finite` trace reports the closed
/// reading with no `decided` until level 2 decides it; a fixed-sample
/// `Progressed` state is decided by a watermark alone; a `HistoryCutoff`
/// boundary reports `MissingHistory`.
#[test]
#[trace("TC-2320", "FR-820-AC-4")]
fn the_open_scope_reading_and_the_lower_boundary_reach_the_state() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    let mut e = finite_state(&world, "E");
    for (index, level) in [0, 1].iter().enumerate() {
        let observed = text(*level);
        let advance = e
            .advance(admitted(&world, "E", &observed, index as u64), &cancel)
            .expect("advances");
        assert_eq!(advance.report.category(), Category::Violation);
        assert!(advance.decided.is_none(), "not decided at {index}");
    }
    let observed = text(2);
    let advance = e
        .advance(admitted(&world, "E", &observed, 2), &cancel)
        .expect("advances");
    let decided = advance.decided.expect("decided");
    assert_eq!(decided.category(), Category::Success);
    assert_eq!(
        decided.finite_window.as_ref().map(|window| window.basis),
        Some(fsl_replay::SettlementBasis::DecisiveWitness)
    );

    // FR-504-AC-1: one position with `p` false, then watermark 3 alone.
    let mut sampled = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Sampled"),
        &clause_open(
            &world,
            ClauseTraceKind::Progressed,
            Some(samples_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let observed = text(0);
    sampled
        .advance(admitted(&world, "Sampled", &observed, 0), &cancel)
        .expect("advances");
    sampled
        .assert_progress(&progress(0), &cancel)
        .expect("watermark 0");
    let advance = sampled
        .assert_progress(&progress(3), &cancel)
        .expect("watermark 3");
    assert!(
        matches!(
            advance.report.disposition,
            spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Violated { at: 0 })
        ),
        "{:?}",
        advance.report.disposition
    );
    assert_eq!(
        advance
            .report
            .finite_window
            .as_ref()
            .map(|window| window.basis),
        Some(fsl_replay::SettlementBasis::DecisiveCounterexample)
    );
    assert_eq!(
        advance.report.basis,
        fsl_replay::SettlementBasis::ClosedScope
    );
    assert!(advance.decided.is_some(), "a watermark alone decides it");

    // FR-504-AC-6: `once[0,2]` `on each`, two positions, `p` false.
    let mut past = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Past"),
        &clause_open(
            &world,
            ClauseTraceKind::Progressed,
            Some(events_binding()),
            ProgressLowerBoundary::HistoryCutoff,
        ),
    )
    .expect("opens");
    for index in 0..2 {
        let observed = text(0);
        past.advance(admitted(&world, "Past", &observed, index), &cancel)
            .expect("advances");
    }
    let advance = past
        .assert_progress(&progress(1), &cancel)
        .expect("progress");
    assert!(
        format!("{:?}", advance.report.disposition).contains("MissingHistory"),
        "{:?}",
        advance.report.disposition
    );
}

fn run_text(level: i64, stopped: bool) -> String {
    format!(r#"{{"level":{{"integer":"{level}"}},"stopped":{{"boolean":{stopped}}}}}"#)
}

/// FR-860-AC-6 (TC-2441): `Fast`'s state written after position 1 and read
/// back reports, after positions 2 and 3, what the uninterrupted state
/// reports, `violation` at position 0, with the capture carried in its bytes
/// and not counted in `retained()`.
#[test]
#[trace("TC-2441", "FR-860-AC-6")]
fn a_capture_is_kept_in_the_state_bytes() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [5, 1, 1, 1];
    let mut original = finite_state(&world, "Fast");
    for (index, level) in levels[..2].iter().enumerate() {
        let observed = run_text(*level, false);
        original
            .advance(admitted(&world, "Fast", &observed, index as u64), &cancel)
            .expect("advances");
        assert_eq!(original.retained(), 0, "the capture is not a held value");
    }
    let bytes = original.to_bytes().expect("writes");
    let written: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(
        written["captures"],
        serde_json::json!([r#"{"kind":"boolean","value":true}"#])
    );
    let mut resumed = ClauseState::from_bytes(
        world.unit.package(),
        clause_node(&world, "Fast"),
        &clause_open(
            &world,
            ClauseTraceKind::Finite,
            None,
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
        &bytes,
        &cancel,
    )
    .expect("reads back");
    let mut last = None;
    for (index, level) in levels.iter().enumerate().skip(2) {
        let observed = run_text(*level, false);
        let a = original
            .advance(admitted(&world, "Fast", &observed, index as u64), &cancel)
            .expect("advances");
        let b = resumed
            .advance(admitted(&world, "Fast", &observed, index as u64), &cancel)
            .expect("advances");
        assert_eq!(members(&a.report), members(&b.report), "at {index}");
        assert_eq!(resumed.retained(), 0);
        last = Some(b.report);
    }
    assert!(
        matches!(
            last.expect("a report").disposition,
            spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Violated { at: 0 })
        ),
        "the capture keeps the value it took at position 0"
    );
}

/// FR-643-AC-9 (TC-2322): over 1,000 samples cycling 0 to 8 with
/// `retained_positions` 4, `Below` and `Nine` stay `Pending` to the end with
/// no `incomplete`, `Recent` settles `success` at step 10, and the checkpoint
/// after the last sample holds no admitted value; over 2, 0, 2 `Recent`
/// settles `violation` at step 3.
#[test]
#[trace("TC-2322", "FR-643-AC-9")]
fn a_long_stream_keeps_only_the_values_its_clauses_read() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    let mut session = world
        .open(
            &["Below", "Nine", "Recent"],
            MonitorTraceKind::Finite,
            None,
            4,
        )
        .expect("opens");
    let mut last = None;
    for k in 0..1000 {
        let observed = text(k % 9);
        let step = session.step(sample(&observed), &cancel).expect("steps");
        assert!(!step.statuses[0].1.is_settled(), "Below at step {}", k + 1);
        assert!(!step.statuses[1].1.is_settled(), "Nine at step {}", k + 1);
        if k == 9 {
            assert_eq!(category(&step, 2), Category::Success);
            assert_eq!(
                step.statuses[2].1.settlement().map(|s| (s.step, s.trigger)),
                Some((10, fsl_analyze::monitor::SettleTrigger::Decided))
            );
        }
        if k < 9 {
            assert!(!step.statuses[2].1.is_settled(), "Recent at step {}", k + 1);
            // FR-643-AC-9: `Recent`'s state holds `min(k, 4)` values after
            // step `k`, the window of its newest position.
            let state: serde_json::Value =
                serde_json::from_slice(session.checkpoint().expect("checkpoints").as_bytes())
                    .expect("JSON");
            let recent: serde_json::Value = serde_json::from_str(
                state["clauses"][2]["state"]
                    .as_str()
                    .expect("Recent's unsettled state"),
            )
            .expect("a clause state");
            assert_eq!(recent["clause"], "Recent");
            assert_eq!(
                recent["values"].as_array().map(Vec::len),
                usize::try_from((k + 1).min(4)).ok(),
                "Recent at step {}",
                k + 1
            );
        }
        last = Some(step);
    }
    let step = last.expect("a step");
    assert_eq!(category(&step, 0), Category::Pending);
    assert_eq!(category(&step, 1), Category::Pending);
    let state: serde_json::Value =
        serde_json::from_slice(session.checkpoint().expect("checkpoints").as_bytes())
            .expect("JSON");
    for clause in state["clauses"].as_array().expect("clauses") {
        let Some(text) = clause["state"].as_str() else {
            continue;
        };
        let carried: serde_json::Value = serde_json::from_str(text).expect("a clause state");
        assert_eq!(carried["values"], serde_json::json!([]));
        assert_eq!(carried["texts"], serde_json::json!([]), "no sample is held");
        // A position identity names the sample's index, never its value; a
        // `Finite` trace carries no ledger, so no fingerprint (FR-993).
        for (index, position) in carried["positions"]
            .as_array()
            .expect("positions")
            .iter()
            .enumerate()
        {
            assert_eq!(position["identity"], index.to_string(), "{position}");
            assert!(position["fingerprint"].is_null(), "{position}");
            assert!(position["document"].is_null(), "{position}");
        }
        assert!(carried["stop"].is_null(), "no position stopped its reading");
        // A streamed clause carries its residuals, no letter per position; a
        // taped one holds a letter for each.
        let streamed = carried["reading"] == "streamed";
        assert!(carried["positions"]
            .as_array()
            .expect("positions")
            .iter()
            .all(|position| streamed || !position["letter"].is_null()));
        assert!(!streamed || carried["stream"].is_object());
    }

    let mut session = world
        .open(&["Recent"], MonitorTraceKind::Finite, None, 4)
        .expect("opens");
    let mut last = None;
    for level in [2, 0, 2] {
        let observed = text(level);
        last = Some(session.step(sample(&observed), &cancel).expect("steps"));
    }
    let step = last.expect("a step");
    assert_eq!(category(&step, 0), Category::Violation);
    assert_eq!(step.statuses[0].1.settlement().map(|s| s.step), Some(3));
}

/// The work units `usage` records.
fn work(usage: &fsl_replay::spine::ClauseRunUsage) -> u64 {
    usage
        .evaluation_consumed
        .iter()
        .find(|(kind, _)| *kind == filament_exact::LimitKind::WorkUnits)
        .map_or(0, |(_, consumed)| *consumed)
}

/// FR-643-AC-12 (TC-2628): over `always holds(level <= 1)` and levels 0, 1, 1
/// each report's usage is the session's `{cumulative, step}`, `step` the
/// call's own charge and `cumulative` the sum of the steps, while every
/// other member is `B(k)`; with a per-call work limit one unit below the
/// second call's `step` the clause settles `incomplete` on that limit at the
/// second call and the first call is unaffected.
#[test]
#[trace("TC-2628", "FR-643-AC-12")]
fn each_call_reports_its_own_usage_and_is_charged_its_own_limit() {
    use filament_semantic_value::catalog::Category;
    use fsl_analyze::monitor::SettleTrigger;
    let world = world();
    let cancel = Cancel::new();
    let levels = [0, 1, 1];
    let open = |accounting| {
        MonitorSession::open(
            world.unit.package(),
            MonitorOpen {
                clauses: clauses(&["AtMostOne"]),
                kind: MonitorTraceKind::Finite,
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: None,
                cover: None,
                ledger: None,
                packages: &world.packages,
                sources: &world.sources,
                temporal_selections: &world.selections,
            },
            &MonitorLimits {
                accounting,
                ..limits(DEFAULT_RETAINED_POSITIONS)
            },
            &cancel,
        )
        .expect("opens")
    };
    let mut session = open(UNLIMITED);
    let mut steps = Vec::new();
    let mut sum = 0;
    for (index, level) in levels.iter().enumerate() {
        let observed = text(*level);
        let step = session.step(sample(&observed), &cancel).expect("steps");
        let record = step.statuses[0].1.record();
        let usage = record.usage().expect("a session record carries its usage");
        assert!(work(&usage.step) > 0, "call {} charges work", index + 1);
        sum += work(&usage.step);
        assert_eq!(work(&usage.cumulative), sum, "after call {}", index + 1);
        assert_eq!(
            members(record.report()),
            members(&world.batch_report("AtMostOne", &levels[..=index])),
            "after call {}",
            index + 1
        );
        steps.push(work(&usage.step));
    }

    let second = steps[1];
    assert!(steps[0] < second, "the first call is within the limit");
    let mut limited = open(filament_exact::ScalarLimits {
        work_units: second - 1,
        ..UNLIMITED
    });
    let first = limited
        .step(sample(&text(levels[0])), &cancel)
        .expect("steps");
    assert!(!first.statuses[0].1.is_settled());
    assert_eq!(
        members(first.statuses[0].1.record().report()),
        members(&world.batch_report("AtMostOne", &levels[..1]))
    );
    let step = limited
        .step(sample(&text(levels[1])), &cancel)
        .expect("steps");
    assert_eq!(category(&step, 0), Category::Incomplete);
    assert_eq!(
        step.statuses[0].1.settlement().map(|s| (s.step, s.trigger)),
        Some((2, SettleTrigger::Limit))
    );
    let step = limited
        .step(sample(&text(levels[2])), &cancel)
        .expect("steps");
    let usage = step.statuses[0].1.record().usage().expect("usage");
    assert_eq!(work(&usage.step), 0, "a settled clause charges nothing");
}

/// FR-820 "Bytes", FR-643-AC-12: a state whose call exhausted a per-call
/// work limit writes that limit into its bytes, and the state read back
/// reports the same `incomplete` report.
#[test]
#[trace("TC-2320", "FR-820-AC-3")]
fn an_exhausted_state_reads_back_from_its_bytes_with_its_limit() {
    let world = world();
    let cancel = Cancel::new();
    for name in ["AtMostOne", "Bounded"] {
        let mut open = clause_open(
            &world,
            ClauseTraceKind::Finite,
            None,
            ProgressLowerBoundary::AuthoritativeOrigin,
        );
        open.limits.accounting = filament_exact::ScalarLimits {
            work_units: 3,
            ..UNLIMITED
        };
        let node = clause_node(&world, name);
        let mut state = ClauseState::open(world.unit.package(), node, &open).expect("opens");
        let mut exhausted = false;
        for (index, level) in [0, 1, 1, 0].iter().enumerate() {
            let observed = text(*level);
            let advance = state
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            if matches!(
                advance.report.disposition,
                spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Incomplete(_))
            ) {
                exhausted = true;
                break;
            }
        }
        assert!(exhausted, "{name}: the work limit ends the evaluation");
        let bytes = state.to_bytes().expect("writes");
        let document: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            document["exhausted"]["limit_kind"], "work_units",
            "{name}: the bytes carry the limit"
        );
        let resumed = ClauseState::from_bytes(world.unit.package(), node, &open, &bytes, &cancel)
            .expect("reads back");
        assert_eq!(
            members(&resumed.report()),
            members(&state.report()),
            "{name}"
        );
        assert_eq!(resumed.to_bytes().expect("writes"), bytes, "{name}");
    }
}

/// FR-820 "No partial state": `restore` puts back the state a `mark` was
/// taken of, so a session can undo one clause's call without copying it, and
/// the state then continues as one that never made the call.
#[test]
#[trace("TC-2320", "FR-820-AC-3")]
fn a_restored_mark_undoes_one_call() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [0, 1, 2, 1, 0];
    for name in ["Bounded", "Reaches", "AtMostOne", "Recent"] {
        let mut state = finite_state(&world, name);
        let mut uninterrupted = finite_state(&world, name);
        for (index, level) in levels[..2].iter().enumerate() {
            let observed = text(*level);
            for target in [&mut state, &mut uninterrupted] {
                target
                    .advance(admitted(&world, name, &observed, index as u64), &cancel)
                    .expect("advances");
            }
        }
        let bytes = state.to_bytes().expect("writes");
        let mark = state.mark();
        let observed = text(levels[2]);
        state
            .advance(admitted(&world, name, &observed, 2), &cancel)
            .expect("advances");
        state.restore(mark);
        assert_eq!(state.to_bytes().expect("writes"), bytes, "{name}");
        for (index, level) in levels.iter().enumerate().skip(2) {
            let observed = text(*level);
            let a = state
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            let b = uninterrupted
                .advance(admitted(&world, name, &observed, index as u64), &cancel)
                .expect("advances");
            assert_eq!(members(&a.report), members(&b.report), "{name} at {index}");
            assert_eq!(state.retained(), uninterrupted.retained(), "{name}");
        }
    }
}

/// The batch report of `name` over `trace`, read unauthenticated as a carried
/// state reads it.
fn batch_over(world: &World, name: &str, trace: &TraceInput) -> ClauseRunReport {
    run_package_temporal(&PackageTemporalRequest {
        package: world.unit.package(),
        packages: &world.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name,
        trace,
        over: None,
        ledger: None,
        sources: &world.sources,
        authentication: None,
        cancel: &Cancel::new(),
    })
}

/// FR-820 "Summarized positions" (TC-2320): a `Timestamped` and a
/// `Progressed` state carry the finite-window evaluation between steps. Each
/// report equals FR-330's over the prefix, no sample is held by value, and a
/// step's evaluation work stays bounded as the trace grows: it reads only the
/// positions an open activation can still read, not the whole trace again.
/// FR-504's open-scope reading (`decided`) is carried the same way.
#[test]
#[trace("TC-2320", "FR-820-AC-1", "FR-820-AC-2")]
fn a_carried_window_reads_only_the_open_activations() {
    let world = world();
    let cancel = Cancel::new();
    let count: u64 = 120;
    let level = |index: u64| i64::try_from(index % 7).expect("a level");

    // Timestamped: two observations share every fifth stamp and arrive out
    // of canonical order; progress covers each stamp after its observations.
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Window"),
        &clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let mut observations: Vec<TimestampedObservation> = Vec::new();
    let mut visits = Vec::new();
    for index in 0..count {
        let stamp = index - index / 5;
        let observed = text(level(index));
        let mut position = admitted(&world, "Window", &observed, stamp);
        position.stamp = stamp;
        state.advance(position, &cancel).expect("advances");
        observations.push(TimestampedObservation {
            document: spine::sample_position(&observed),
            stamp,
            order_key: None,
        });
        if index % 5 == 4 {
            continue;
        }
        let advance = state
            .assert_progress(&progress(stamp), &cancel)
            .expect("progress");
        let trace = TraceInput::Timestamped {
            observations: observations.clone(),
            progress: progress(stamp),
            window: Box::new(TimestampedWindow {
                binding: Some(stamped_binding()),
                authoritative_origin: true,
                cover: None,
                missing_valuations: Vec::new(),
                late: spine::LateInputs::default(),
            }),
        };
        let batch = batch_over(&world, "Window", &trace);
        assert_eq!(
            members(&advance.report),
            members(&batch),
            "Window after position {index}"
        );
        assert_eq!(state.retained(), 0, "Window after position {index}");
        visits.push((
            advance.report.usage.temporal_visits,
            batch.usage.temporal_visits,
        ));
    }
    let bound = visits[..10].iter().map(|(carried, _)| *carried).max();
    let late = visits[visits.len() - 10..]
        .iter()
        .map(|(carried, _)| *carried)
        .max();
    assert!(late <= bound.map(|bound| bound * 2), "{visits:?}");
    let (carried, batch) = visits[visits.len() - 1];
    assert!(carried * 10 < batch, "{visits:?}");

    // Progressed, event-position `on each`: the report and the open-scope
    // `decided` reading.
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Past"),
        &clause_open(
            &world,
            ClauseTraceKind::Progressed,
            Some(events_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let mut documents = Vec::new();
    let mut visits = Vec::new();
    for index in 0..count {
        let observed = text(if index % 11 == 3 { 9 } else { level(index) });
        state
            .advance(admitted(&world, "Past", &observed, index), &cancel)
            .expect("advances");
        documents.push(spine::sample_position(&observed));
        let advance = state
            .assert_progress(&progress(index), &cancel)
            .expect("progress");
        let trace = TraceInput::Progressed {
            trace: documents.clone(),
            progress: progress(index),
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: events_binding(),
        };
        let batch = batch_over(&world, "Past", &trace);
        assert_eq!(
            members(&advance.report),
            members(&batch),
            "Past after position {index}"
        );
        assert_eq!(state.retained(), 0, "Past after position {index}");
        visits.push((
            advance.report.usage.temporal_visits,
            batch.usage.temporal_visits,
        ));
    }
    let bound = visits[..10].iter().map(|(carried, _)| *carried).max();
    let late = visits[visits.len() - 10..]
        .iter()
        .map(|(carried, _)| *carried)
        .max();
    assert!(late <= bound.map(|bound| bound * 2), "{visits:?}");

    // Finite, event-position: `decided` is FR-504's open-scope reading,
    // carried, with bounded work per step.
    let mut state = finite_state(&world, "Past");
    let mut decided = Vec::new();
    for index in 0..count {
        let observed = text(if index % 11 == 3 { 9 } else { level(index) });
        let advance = state
            .advance(admitted(&world, "Past", &observed, index), &cancel)
            .expect("advances");
        if let Some(report) = advance.decided {
            decided.push(report.usage.temporal_visits);
        }
    }
    assert_eq!(decided.len(), usize::try_from(count).expect("a count"));
    let bound = decided[..10].iter().max().copied();
    let late = decided[decided.len() - 10..].iter().max().copied();
    assert!(late <= bound.map(|bound| bound * 2), "{decided:?}");
}

/// The batch report of `name` over `trace` from `ledger`, read
/// unauthenticated as a carried state reads it.
fn batch_from(
    world: &World,
    name: &str,
    trace: &TraceInput,
    ledger: Option<&spine::ProgressLedger>,
) -> ClauseRunReport {
    run_package_temporal(&PackageTemporalRequest {
        package: world.unit.package(),
        packages: &world.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name,
        trace,
        over: None,
        ledger,
        sources: &world.sources,
        authentication: None,
        cancel: &Cancel::new(),
    })
}

/// FR-820 "Summarized positions", FR-502 (TC-2320): a carried state's runs
/// admit only the positions added since the last one, into the
/// finite-window trace and into the progress ledger, whose covered set each
/// report shares. Over 120 positions each report equals FR-330's over the
/// prefix member for member, the ledger included, while a step admits at
/// most the positions it added and the batch run admits every position. This
/// holds for a `Timestamped` trace whose positions are reordered at a shared
/// stamp, for a `Progressed` trace opened from a ledger that already covers
/// its first position, and for a session that carries each report's ledger
/// into the next run.
#[test]
#[trace("TC-2320", "FR-820-AC-1", "FR-820-AC-2", "FR-502-AC-7")]
fn a_carried_state_admits_only_the_new_positions_into_its_trace_and_ledger() {
    let world = world();
    let cancel = Cancel::new();
    let count: u64 = 120;
    let level = |index: u64| i64::try_from(index % 7).expect("a level");
    // Each position is admitted once, into the trace and into the ledger,
    // by the step whose progress first covers it.
    let flat = |name: &str, work: &[(u64, u64, u64, u64)], most: u64, admitted: u64| {
        assert_eq!(
            work.iter().map(|step| step.0).sum::<u64>(),
            admitted,
            "{name}"
        );
        assert_eq!(
            work.iter().map(|step| step.1).sum::<u64>(),
            admitted,
            "{name}"
        );
        for (step, (trace, ledger, batch_trace, batch_ledger)) in work.iter().enumerate() {
            assert!(
                *trace <= most && *ledger <= most,
                "{name} step {step}: {work:?}"
            );
            assert!(*batch_trace > 0 && *batch_ledger > 0, "{name} step {step}");
        }
        let (_, _, batch_trace, batch_ledger) = work[work.len() - 1];
        assert!(
            batch_trace >= count - 1 && batch_ledger >= count - 1,
            "{work:?}"
        );
    };
    let usage = |carried: &ClauseRunReport, batch: &ClauseRunReport| {
        (
            carried.usage.trace_positions,
            carried.usage.ledger_positions,
            batch.usage.trace_positions,
            batch.usage.ledger_positions,
        )
    };

    // Timestamped: two observations share every fifth stamp and arrive out
    // of canonical order.
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Window"),
        &clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let mut observations: Vec<TimestampedObservation> = Vec::new();
    let mut work = Vec::new();
    for index in 0..count {
        let stamp = index - index / 5;
        let observed = text(level(index));
        let mut position = admitted(&world, "Window", &observed, stamp);
        position.stamp = stamp;
        state.advance(position, &cancel).expect("advances");
        observations.push(TimestampedObservation {
            document: spine::sample_position(&observed),
            stamp,
            order_key: None,
        });
        if index % 5 == 4 {
            continue;
        }
        let advance = state
            .assert_progress(&progress(stamp), &cancel)
            .expect("progress");
        let trace = TraceInput::Timestamped {
            observations: observations.clone(),
            progress: progress(stamp),
            window: Box::new(TimestampedWindow {
                binding: Some(stamped_binding()),
                authoritative_origin: true,
                cover: None,
                missing_valuations: Vec::new(),
                late: spine::LateInputs::default(),
            }),
        };
        let batch = batch_from(&world, "Window", &trace, None);
        assert!(advance.report.ledger.is_some(), "Window after {index}");
        assert_eq!(
            members(&advance.report),
            members(&batch),
            "Window after position {index}"
        );
        work.push(usage(&advance.report, &batch));
    }
    flat("Window", &work, 2, count - 1);

    // Progressed, from an opening ledger that covers the first position, and
    // as a session carrying each report's ledger into the next run.
    let first = text(level(0));
    let opening = batch_from(
        &world,
        "Past",
        &TraceInput::Progressed {
            trace: vec![spine::sample_position(&first)],
            progress: progress(0),
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: events_binding(),
        },
        None,
    )
    .ledger
    .expect("the opening ledger");
    for carried in [false, true] {
        let mut state = ClauseState::open(
            world.unit.package(),
            clause_node(&world, "Past"),
            &ClauseOpen {
                ledger: Some(opening.clone()),
                ..clause_open(
                    &world,
                    ClauseTraceKind::Progressed,
                    Some(events_binding()),
                    ProgressLowerBoundary::AuthoritativeOrigin,
                )
            },
        )
        .expect("opens");
        let mut documents = Vec::new();
        let mut batch_ledger = opening.clone();
        let mut work = Vec::new();
        for index in 0..count {
            let observed = text(if index % 11 == 3 { 9 } else { level(index) });
            state
                .advance(admitted(&world, "Past", &observed, index), &cancel)
                .expect("advances");
            documents.push(spine::sample_position(&observed));
            let advance = state
                .assert_progress(&progress(index), &cancel)
                .expect("progress");
            let trace = TraceInput::Progressed {
                trace: documents.clone(),
                progress: progress(index),
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: events_binding(),
            };
            let batch = batch_from(&world, "Past", &trace, Some(&batch_ledger));
            assert_eq!(
                members(&advance.report),
                members(&batch),
                "Past (carried {carried}) after position {index}"
            );
            let covered = advance.report.ledger.as_ref().and_then(|ledger| {
                ledger
                    .entries()
                    .next()
                    .map(|(_, entry)| entry.covered().count())
            });
            assert_eq!(covered, Some(usize::try_from(index).expect("an index") + 1));
            work.push(usage(&advance.report, &batch));
            if carried {
                state.carry_ledger(advance.report.ledger.clone());
                batch_ledger = batch.ledger.clone().expect("the batch ledger");
            }
        }
        flat("Past", &work, 1, count);
    }
}

/// The contradicted dimension of a progress-refused report, if any.
fn contradicted(report: &ClauseRunReport) -> Option<String> {
    match &report.disposition {
        spine::ClauseDisposition::ProgressContradiction(contradiction) => contradiction
            .record
            .fields
            .get("contradicted_dimension")
            .cloned(),
        _ => None,
    }
}

/// Whether every covered position of `ledger` is named by its index alone
/// and carries a content fingerprint (FR-993), holding no sample text.
fn content_named(ledger: &spine::ProgressLedger) -> bool {
    ledger
        .entries()
        .flat_map(|(_, entry)| entry.covered())
        .all(|position| {
            let index = position
                .identity
                .strip_prefix("13:record-sample")
                .and_then(|rest| rest.split_once(':'))
                .is_some_and(|(_, index)| index.parse::<u64>().is_ok());
            let fingerprint = position.fingerprint.as_deref().is_some_and(|fingerprint| {
                fingerprint.strip_prefix("sha256:").is_some_and(|digits| {
                    digits.len() == 64
                        && digits
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            });
            index && fingerprint && !format!("{position:?}").contains("level")
        })
}

/// FR-502-AC-10 (TC-1296) over record samples: once `Complete { through }`
/// is retained, a later request that lists the same sample at a covered
/// position is accepted, and one that lists a different sample there is
/// refused `Completeness`, for a `Progressed` and for a `Timestamped` trace.
/// The ledger names each sample by its content identity, not its text.
#[test]
#[trace("TC-1296", "FR-502-AC-10")]
fn a_different_record_sample_at_a_covered_position_is_refused() {
    let world = world();
    let progressed = |levels: &[i64], through: u64| TraceInput::Progressed {
        trace: levels
            .iter()
            .map(|level| spine::sample_position(&text(*level)))
            .collect(),
        progress: progress(through),
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: events_binding(),
    };
    let held = batch_from(&world, "Past", &progressed(&[1, 2], 1), None)
        .ledger
        .expect("the retained ledger");
    assert!(content_named(&held), "{held:?}");
    let same = batch_from(&world, "Past", &progressed(&[1, 2, 3], 2), Some(&held));
    assert_eq!(contradicted(&same), None, "{:?}", same.disposition);
    assert!(same.ledger.is_some_and(|ledger| ledger != held));
    let other = batch_from(&world, "Past", &progressed(&[1, 5, 3], 2), Some(&held));
    assert_eq!(contradicted(&other).as_deref(), Some("Completeness"));
    assert_eq!(other.ledger, Some(held));

    let stamped = |levels: &[i64], through: u64| TraceInput::Timestamped {
        observations: levels
            .iter()
            .zip(0u64..)
            .map(|(level, stamp)| TimestampedObservation {
                document: spine::sample_position(&text(*level)),
                stamp,
                order_key: None,
            })
            .collect(),
        progress: progress(through),
        window: Box::new(TimestampedWindow {
            binding: Some(stamped_binding()),
            authoritative_origin: true,
            cover: None,
            missing_valuations: Vec::new(),
            late: spine::LateInputs::default(),
        }),
    };
    let held = batch_from(&world, "Window", &stamped(&[1, 2], 1), None)
        .ledger
        .expect("the retained ledger");
    assert!(content_named(&held), "{held:?}");
    let same = batch_from(&world, "Window", &stamped(&[1, 2, 3], 2), Some(&held));
    assert_eq!(contradicted(&same), None, "{:?}", same.disposition);
    assert!(same.ledger.is_some_and(|ledger| ledger != held));
    let other = batch_from(&world, "Window", &stamped(&[1, 5, 3], 2), Some(&held));
    assert_eq!(contradicted(&other).as_deref(), Some("Completeness"));
    assert_eq!(other.ledger, Some(held));
}

/// FR-820-AC-1 (TC-2320) over snapshots: `Bounded`, `Reaches` and
/// `always holds(c.value <= 1)` over `c.value` 0, 1, 2, 1, 0. Each snapshot is
/// admitted once; the report after each `advance` is FR-330's over the prefix,
/// member for member (the violation's decisive record included), a state
/// read back from its bytes continues the same, and the state keeps only the
/// snapshots a decisive record can still read. Over 120 positions of an
/// `always` clause that holds, it keeps one snapshot and each step's work is
/// flat.
#[test]
#[trace("TC-2320", "FR-820-AC-1")]
fn each_snapshot_advance_reports_the_batch_report_over_the_prefix() {
    use crate::counter::{document, mixed_package_with_clauses, Documents};
    use fsl_replay::spine::OverObject;
    let document = document(false);
    let (package, source) = mixed_package_with_clauses(
        &document,
        &[("Bounded", "", "eventually[0,1] holds(c.value = 2)")],
        &[
            ("Reaches", "", "eventually holds(c.value = 2)"),
            ("Small", "", "always holds(c.value <= 1)"),
        ],
    );
    let mut documents = Documents::new(document);
    let mut values = vec![0, 1, 2, 1, 0];
    values.extend((0..115).map(|index| index % 2));
    let refs: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s-{index}"), &[("c1", *value)]))
        .collect();
    let provision: DocumentProvision = documents
        .snapshot_bytes()
        .map(|(reference, bytes)| (reference.clone(), bytes.clone()))
        .collect();
    let bytes: Vec<&[u8]> = refs
        .iter()
        .map(|reference| provision.get(reference).expect("provisioned").as_slice())
        .collect();
    let over = OverObject {
        population: "ix://test/counter/counters".to_owned(),
        key: "c1".to_owned(),
    };
    let sources = [source];
    let request = ClauseOpen {
        kind: ClauseTraceKind::Finite,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: None,
        cover: None,
        ledger: None,
        sources: &sources,
        authentication: None,
        packages: &documents.packages,
        over: Some(over.clone()),
        limits: ClauseLimits {
            model: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            accounting: UNLIMITED,
            max_window_values: fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
            covered_positions: fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        },
    };
    let node = |name: &str| {
        package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .expect("declared")
    };
    let batch = |name: &str, upto: usize| {
        run_package_temporal(&PackageTemporalRequest {
            package: &package,
            packages: &documents.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &provision,
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name,
            trace: &TraceInput::Finite(refs[..upto].to_vec()),
            over: Some(&over),
            ledger: None,
            sources: &sources,
            authentication: None,
            cancel: &Cancel::new(),
        })
    };
    let position = |index: usize| AdmittedPosition {
        reference: refs[index].clone(),
        stamp: index as u64,
        order_key: None,
        value: AdmittedValue::Snapshot(bytes[index]),
        absent: Vec::new(),
    };
    let cancel = Cancel::new();
    for name in ["Bounded", "Reaches", "Small"] {
        let mut state = ClauseState::open(&package, node(name), &request).expect("opens");
        for index in 0..5 {
            if index == 3 {
                // Read back from its bytes, the state continues as the
                // original does.
                let saved = state.to_bytes().expect("writes");
                state = ClauseState::from_bytes(&package, node(name), &request, &saved, &cancel)
                    .expect("reads back");
            }
            let advance = state.advance(position(index), &cancel).expect("advances");
            assert_eq!(
                members(&advance.report),
                members(&batch(name, index + 1)),
                "{name} after position {index}"
            );
            assert!(
                state.retained() <= 2,
                "{name} keeps {} snapshots after position {index}",
                state.retained()
            );
        }
    }
    // The violation of `always holds(c.value <= 1)` at position 2 carries its
    // decisive record, read from the one snapshot the state kept for it.
    let mut small = ClauseState::open(&package, node("Small"), &request).expect("opens");
    for index in 0..3 {
        small.advance(position(index), &cancel).expect("advances");
    }
    assert_eq!(
        small.report().decisive.as_ref().map(|_| ()),
        Some(()),
        "{:?}",
        small.report()
    );
    // `always holds(c.value <= 1)` from position 5 on holds over 0 and 1: no
    // snapshot but position 0 is kept, and each step does the same work.
    let mut held = ClauseState::open(&package, node("Small"), &request).expect("opens");
    let mut steps = Vec::new();
    for index in 5..values.len() {
        let advance = held.advance(position(index), &cancel).expect("advances");
        steps.push(work(&advance.report.usage));
        if index % 40 == 0 || index + 1 == values.len() {
            let batch = run_package_temporal(&PackageTemporalRequest {
                package: &package,
                packages: &documents.packages,
                model_limits: ModelNormalizationLimits::default(),
                snapshots: &provision,
                observation_limits: ObservationLimits::default(),
                accounting: UNLIMITED,
                name: "Small",
                trace: &TraceInput::Finite(refs[5..=index].to_vec()),
                over: Some(&over),
                ledger: None,
                sources: &sources,
                authentication: None,
                cancel: &Cancel::new(),
            });
            assert_eq!(members(&advance.report), members(&batch), "at {index}");
        }
        assert_eq!(held.retained(), 1, "at {index}");
    }
    let late = &steps[steps.len() - 20..];
    assert!(
        late.iter().all(|step| *step <= steps[10]),
        "per-step work stays flat: {steps:?}"
    );
}

/// FR-820-AC-1 (TC-2320) over a `Progressed` snapshot trace: the
/// event-position clauses `eventually[0,1] holds(c.value = 2)` and
/// `always[0,9] holds(c.value <= 1)` over `c.value` 0, 1, 2, 1, 0, each position
/// followed by progress through it. Each report is FR-330's over the prefix
/// under that progress, member for member, the violation's decisive record
/// included, also after a state read back from its bytes; the state keeps at
/// most two snapshots.
#[test]
#[trace("TC-2320", "FR-820-AC-1")]
fn a_progressed_snapshot_state_reports_the_batch_report() {
    use crate::counter::{document, mixed_package_with_clauses, Documents};
    use fsl_replay::spine::OverObject;
    let document = document(false);
    let (package, source) = mixed_package_with_clauses(
        &document,
        &[
            ("Bounded", "", "eventually[0,1] holds(c.value = 2)"),
            ("Small", "", "always[0,9] holds(c.value <= 1)"),
        ],
        &[],
    );
    let mut documents = Documents::new(document);
    let values = [0, 1, 2, 1, 0];
    let refs: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s-{index}"), &[("c1", *value)]))
        .collect();
    let provision: DocumentProvision = documents
        .snapshot_bytes()
        .map(|(reference, bytes)| (reference.clone(), bytes.clone()))
        .collect();
    let over = OverObject {
        population: "ix://test/counter/counters".to_owned(),
        key: "c1".to_owned(),
    };
    let binding = TraceBinding {
        profile: EVENTS.to_owned(),
        clock_name: "model-steps".to_owned(),
        parameters: ClockConfiguration::EventPosition {
            sequence_authority: "model-steps".into(),
        },
    };
    let sources = [source];
    let request = ClauseOpen {
        kind: ClauseTraceKind::Progressed,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: Some(binding.clone()),
        cover: None,
        ledger: None,
        sources: &sources,
        authentication: None,
        packages: &documents.packages,
        over: Some(over.clone()),
        limits: ClauseLimits {
            model: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            accounting: UNLIMITED,
            max_window_values: fsl_eval::temporal::DEFAULT_MAX_WINDOW_VALUES,
            covered_positions: fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        },
    };
    let cancel = Cancel::new();
    for name in ["Bounded", "Small"] {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .expect("declared");
        let mut state = ClauseState::open(&package, node, &request).expect("opens");
        for index in 0..values.len() {
            if index == 3 {
                let saved = state.to_bytes().expect("writes");
                state = ClauseState::from_bytes(&package, node, &request, &saved, &cancel)
                    .expect("reads back");
            }
            state
                .advance(
                    AdmittedPosition {
                        reference: refs[index].clone(),
                        stamp: index as u64,
                        order_key: None,
                        absent: Vec::new(),
                        value: AdmittedValue::Snapshot(
                            provision.get(&refs[index]).expect("provisioned"),
                        ),
                    },
                    &cancel,
                )
                .expect("advances");
            let advance = state
                .assert_progress(&progress(index as u64), &cancel)
                .expect("progresses");
            let batch = run_package_temporal(&PackageTemporalRequest {
                package: &package,
                packages: &documents.packages,
                model_limits: ModelNormalizationLimits::default(),
                snapshots: &provision,
                observation_limits: ObservationLimits::default(),
                accounting: UNLIMITED,
                name,
                trace: &TraceInput::Progressed {
                    trace: refs[..=index].to_vec(),
                    progress: progress(index as u64),
                    lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                    binding: binding.clone(),
                },
                over: Some(&over),
                ledger: None,
                sources: &sources,
                authentication: None,
                cancel: &Cancel::new(),
            });
            assert_eq!(
                members(&advance.report),
                members(&batch),
                "{name} after position {index}"
            );
            assert!(state.retained() <= 2, "{name} at {index}");
        }
        if name == "Small" {
            assert!(state.report().decisive.is_some(), "{:?}", state.report());
        }
    }
}

/// How many sample values `bytes` (or a report's debug form) spell: each
/// sample's canonical text gives its `level` member a value
/// (`"level":{...}`). The field name alone, which a clause's identity in an
/// evidence key spells (`"level",` among the clause JSON), is no sample value.
fn spelled_samples(text: &str) -> usize {
    text.matches(r#"\"level\":{"#).count()
}

/// FR-330-AC-18, FR-643 Decision, FR-820 "Identities" (TC-2320): every
/// reading of a record-sample trace cites each position by its index, never
/// by its value. Over equal samples at many positions, each state's bytes
/// name every position by its index, spell no more sample values than
/// `retained()` counts, and every report, its ledger and finite-window
/// record included, spells none, while each report still equals FR-330's
/// over the prefix. The readings: the trace-subject (`Past`), the
/// infinite-trace streamed (`Below`), the fixed-sample streamed (`Sampled`),
/// and, opened from a ledger, the refused infinite-trace reading (`Below`)
/// and the `Finite` observed fixed-sample reading (`Sampled`), and the observed
/// `Progressed` (`Past`) and `Timestamped` (`Window`) readings.
#[test]
#[trace("TC-2320", "FR-820-AC-1", "FR-643-AC-9", "FR-330-AC-18")]
fn every_reading_cites_its_samples_by_index() {
    let world = world();
    // The oracle sees a sample value: one sample spells one.
    assert_eq!(
        spelled_samples(&format!("{:?}", spine::sample_position(&text(2)))),
        1
    );
    let cancel = Cancel::new();
    let count: u64 = 30;
    let level = |index: u64| i64::try_from(index % 3).expect("a level");
    let check = |name: &str, state: &ClauseState<'_>, report: &ClauseRunReport, at: u64| {
        let bytes = String::from_utf8(state.to_bytes().expect("bytes")).expect("UTF-8");
        let carried: serde_json::Value = serde_json::from_str(&bytes).expect("JSON");
        for (index, position) in carried["positions"]
            .as_array()
            .expect("positions")
            .iter()
            .enumerate()
        {
            // FR-993: the index alone; a progress-carrying trace's sample
            // also carries its content fingerprint, as a member of its own.
            assert_eq!(
                position["identity"],
                index.to_string(),
                "{name} at {at}: {position}"
            );
            let fingerprint = &position["fingerprint"];
            assert!(
                fingerprint.is_null()
                    || fingerprint
                        .as_str()
                        .and_then(|text| text.strip_prefix("sha256:"))
                        .is_some_and(|digits| digits.len() == 64),
                "{name} at {at}: {position}"
            );
        }
        let retained = usize::try_from(state.retained()).expect("a count");
        assert!(
            spelled_samples(&bytes) <= retained,
            "{name} at {at}: {} spelled, {retained} retained: {bytes}",
            spelled_samples(&bytes)
        );
        let debug = format!("{report:?}");
        assert_eq!(spelled_samples(&debug), 0, "{name} at {at}: {debug}");
    };

    for (name, ledgered) in [
        ("Past", false),
        ("Below", false),
        ("Sampled", false),
        ("Below", true),
        ("Sampled", true),
    ] {
        let ledger = ledgered.then(spine::ProgressLedger::new);
        let mut state = ledger_state(&world, name, ledger.clone());
        let mut documents = Vec::new();
        for index in 0..count {
            let observed = text(level(index));
            documents.push(spine::sample_position(&observed));
            let advance = state
                .advance(admitted(&world, name, &observed, index), &cancel)
                .expect("advances");
            check(name, &state, &advance.report, index);
            if let Some(decided) = &advance.decided {
                check(name, &state, decided, index);
            }
            let trace = TraceInput::Finite(documents.clone());
            assert_eq!(
                members(&advance.report),
                members(&batch_from(&world, name, &trace, ledger.as_ref())),
                "{name} (ledger {ledgered}) at {index}"
            );
        }
    }

    // Observed `Progressed`: the ledger names each position by its index.
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Past"),
        &clause_open(
            &world,
            ClauseTraceKind::Progressed,
            Some(events_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let mut documents = Vec::new();
    for index in 0..count {
        let observed = text(level(index));
        state
            .advance(admitted(&world, "Past", &observed, index), &cancel)
            .expect("advances");
        documents.push(spine::sample_position(&observed));
        let advance = state
            .assert_progress(&progress(index), &cancel)
            .expect("progress");
        check("Past (progressed)", &state, &advance.report, index);
        let ledger = advance.report.ledger.as_ref().expect("a ledger");
        let identities: Vec<String> = ledger
            .entries()
            .flat_map(|(_, entry)| entry.covered().map(|position| position.identity))
            .collect();
        assert_eq!(identities.len(), documents.len(), "{identities:?}");
        let trace = TraceInput::Progressed {
            trace: documents.clone(),
            progress: progress(index),
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: events_binding(),
        };
        assert_eq!(
            members(&advance.report),
            members(&batch_over(&world, "Past", &trace)),
            "Past (progressed) at {index}"
        );
    }

    // Observed `Timestamped`: equal samples share a stamp; a sample is named
    // by its text only until progress covers it, counted in `retained()`.
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, "Window"),
        &clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    let mut observations = Vec::new();
    for index in 0..count {
        let stamp = index / 3;
        let observed = text(level(index / 3));
        let mut position = admitted(&world, "Window", &observed, stamp);
        position.stamp = stamp;
        state.advance(position, &cancel).expect("advances");
        observations.push(TimestampedObservation {
            document: spine::sample_position(&observed),
            stamp,
            order_key: None,
        });
        if index % 3 != 2 {
            assert_eq!(state.retained(), index % 3 + 1, "Window at {index}");
            continue;
        }
        let advance = state
            .assert_progress(&progress(stamp), &cancel)
            .expect("progress");
        assert_eq!(state.retained(), 0, "Window at {index}");
        check("Window", &state, &advance.report, index);
        let trace = TraceInput::Timestamped {
            observations: observations.clone(),
            progress: progress(stamp),
            window: Box::new(TimestampedWindow {
                binding: Some(stamped_binding()),
                authoritative_origin: true,
                cover: None,
                missing_valuations: Vec::new(),
                late: spine::LateInputs::default(),
            }),
        };
        assert_eq!(
            members(&advance.report),
            members(&batch_over(&world, "Window", &trace)),
            "Window at {index}"
        );
    }
}

/// A `Finite` state of `name` opened from `ledger`.
fn ledger_state<'a>(
    world: &'a World,
    name: &str,
    ledger: Option<spine::ProgressLedger>,
) -> ClauseState<'a> {
    let mut open = clause_open(
        world,
        ClauseTraceKind::Finite,
        None,
        ProgressLowerBoundary::AuthoritativeOrigin,
    );
    open.ledger = ledger;
    ClauseState::open(world.unit.package(), clause_node(world, name), &open).expect("opens")
}

/// FR-820 "Summarized positions" (TC-2320): the `Finite` states the observed
/// reading carried before, those opened from a ledger, do flat work per step.
/// FR-330 refuses a ledger over a `Finite` trace before it reads a position;
/// the infinite-trace state (`Below`) runs that refusal over one position and
/// holds none, and the fixed-sample state (`Sampled`) carries its `decided`
/// open-scope reading. Over 200 positions the last steps' work is at most
/// twice the first steps', and each report equals FR-330's over the prefix.
#[test]
#[trace("TC-2320", "FR-820-AC-1", "FR-820-AC-2")]
fn a_finite_observed_state_does_flat_work_per_step() {
    let world = world();
    let cancel = Cancel::new();
    let count: u64 = 200;
    let ledger = spine::ProgressLedger::new();
    for name in ["Below", "Sampled"] {
        let mut state = ledger_state(&world, name, Some(ledger.clone()));
        let mut documents = Vec::new();
        let mut steps = Vec::new();
        for index in 0..count {
            let value = i64::try_from(index % 7).expect("a level");
            let observed = text(value);
            documents.push(spine::sample_position(&observed));
            let advance = state
                .advance(admitted(&world, name, &observed, index), &cancel)
                .expect("advances");
            let decided = advance.decided.as_ref().map_or(0, |decided| {
                decided.usage.temporal_visits + work(&decided.usage)
            });
            steps.push(
                advance.report.usage.temporal_visits
                    + advance.report.usage.trace_positions
                    + work(&advance.report.usage)
                    + decided,
            );
            assert_eq!(state.retained(), 0, "{name} at {index}");
            if index % 50 == 0 || index == count - 1 {
                let trace = TraceInput::Finite(documents.clone());
                assert_eq!(
                    members(&advance.report),
                    members(&batch_from(&world, name, &trace, Some(&ledger))),
                    "{name} at {index}"
                );
            }
        }
        let early = steps[1..11].iter().max().copied().unwrap_or(0);
        let late = steps[steps.len() - 10..].iter().max().copied().unwrap_or(0);
        assert!(late <= early.max(1) * 2, "{name}: {steps:?}");
    }
}

/// FR-993-AC-1 (batch half), FR-993-AC-2 (TC-2810): a later batch request
/// that re-delivers a changed record sample at an index the input ledger
/// covers is refused `Completeness` and returns that ledger; the same
/// samples are accepted. Covered samples are named by index with a
/// fingerprint, and no ledger spells a sample's text or value. The carried
/// half (a `ClauseState` opened on the ledger, before and after
/// `from_bytes`) is covered by the `ClauseState` unit tests.
#[test]
#[trace("TC-2810", "FR-993-AC-1", "FR-993-AC-2")]
fn a_re_delivered_changed_sample_is_refused_in_batch_and_carried_evaluation() {
    let world = world();
    let run = |levels: &[i64], through: u64, ledger: Option<&spine::ProgressLedger>| {
        let trace = TraceInput::Progressed {
            trace: levels
                .iter()
                .map(|level| spine::sample_position(&text(*level)))
                .collect(),
            progress: progress(through),
            lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            binding: events_binding(),
        };
        run_package_temporal(&PackageTemporalRequest {
            package: world.unit.package(),
            packages: &world.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &DocumentProvision::new(),
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name: "E",
            trace: &trace,
            over: None,
            ledger,
            sources: &world.sources,
            authentication: None,
            cancel: &Cancel::new(),
        })
    };
    // The covered entries: the binding names the clause's declared types,
    // `integer` among them, which is no sample value.
    let valueless = |ledger: &spine::ProgressLedger| {
        let spelled = format!(
            "{:?}",
            ledger
                .entries()
                .flat_map(|(_, entry)| entry.covered())
                .collect::<Vec<_>>()
        );
        assert!(!spelled.contains("integer"), "a sample value: {spelled}");
        assert!(!spelled.contains("level"), "a sample text: {spelled}");
    };
    let held = run(&[1, 2], 1, None).ledger.expect("the retained ledger");
    valueless(&held);
    let other = run(&[1, 5, 3], 2, Some(&held));
    let refused = format!("{:?}", other.disposition);
    assert!(refused.contains("Completeness"), "{refused}");
    assert_eq!(other.ledger.as_ref(), Some(&held));
    let same = run(&[1, 2, 3], 2, Some(&held));
    let accepted = format!("{:?}", same.disposition);
    assert!(!accepted.contains("Completeness"), "{accepted}");
    valueless(&same.ledger.expect("the updated ledger"));
}

/// A `Progressed` `Past` state opened on `ledger` under a covered-position
/// bound of `covered_positions`.
fn progressed_state<'a>(
    world: &'a World,
    ledger: Option<spine::ProgressLedger>,
    covered_positions: u64,
) -> ClauseState<'a> {
    let mut open = clause_open(
        world,
        ClauseTraceKind::Progressed,
        Some(events_binding()),
        ProgressLowerBoundary::AuthoritativeOrigin,
    );
    open.ledger = ledger;
    open.limits.covered_positions = covered_positions;
    ClauseState::open(world.unit.package(), clause_node(world, "Past"), &open).expect("opens")
}

/// A `Progressed` trace of record samples at `levels` under
/// `Complete { through }`.
fn progressed_levels(levels: &[i64], through: u64) -> TraceInput {
    TraceInput::Progressed {
        trace: levels
            .iter()
            .map(|level| spine::sample_position(&text(*level)))
            .collect(),
        progress: progress(through),
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: events_binding(),
    }
}

/// Advance `state` over `levels` from position `from` on, then assert
/// `Complete { through }`.
fn deliver(
    world: &World,
    state: &mut ClauseState<'_>,
    levels: &[i64],
    from: usize,
    through: u64,
) -> Result<spine::Advance, spine::ClauseRefusal> {
    let cancel = Cancel::new();
    for (index, level) in levels.iter().enumerate().skip(from) {
        state.advance(
            admitted(world, "Past", &text(*level), index as u64),
            &cancel,
        )?;
    }
    state.assert_progress(&progress(through), &cancel)
}

/// `state` itself, or read back by `from_bytes` from its bytes under the
/// same opening request when `restored`.
fn reopen<'a>(
    world: &'a World,
    held: &spine::ProgressLedger,
    state: ClauseState<'a>,
    restored: bool,
) -> ClauseState<'a> {
    if !restored {
        return state;
    }
    let bytes = state.to_bytes().expect("bytes");
    let mut open = clause_open(
        world,
        ClauseTraceKind::Progressed,
        Some(events_binding()),
        ProgressLowerBoundary::AuthoritativeOrigin,
    );
    open.ledger = Some(held.clone());
    ClauseState::from_bytes(
        world.unit.package(),
        clause_node(world, "Past"),
        &open,
        &bytes,
        &Cancel::new(),
    )
    .expect("reads back")
}

/// FR-993-AC-1 (carried half), FR-820 "Content fingerprints", FR-502-AC-10
/// (TC-2810): a `ClauseState` opened on the ledger a batch run over levels
/// 1, 2 returned refuses levels 1, 5, 3 `Completeness`, unchanged, and
/// accepts levels 1, 2, 3 with the ledger the batch run returns, before and
/// after a `from_bytes` restore taken after the first position. No ledger
/// spells a sample's value.
#[test]
#[trace("TC-2810", "FR-993-AC-1", "FR-993-AC-2", "FR-820-AC-1", "FR-502-AC-10")]
fn a_carried_state_refuses_a_changed_sample_at_a_covered_index_before_and_after_restore() {
    let world = world();
    let cancel = Cancel::new();
    let held = batch_from(&world, "Past", &progressed_levels(&[1, 2], 1), None)
        .ledger
        .expect("the retained ledger");
    assert!(content_named(&held), "{held:?}");
    let accepted = batch_from(
        &world,
        "Past",
        &progressed_levels(&[1, 2, 3], 2),
        Some(&held),
    );
    let accepted_ledger = accepted.ledger.clone().expect("the updated ledger");
    assert!(content_named(&accepted_ledger), "{accepted_ledger:?}");

    for restored in [false, true] {
        // A changed sample at the covered index 1.
        let mut state = progressed_state(
            &world,
            Some(held.clone()),
            fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        );
        state
            .advance(admitted(&world, "Past", &text(1), 0), &cancel)
            .expect("advances");
        let mut state = reopen(&world, &held, state, restored);
        for (index, level) in [(1, 5), (2, 3)] {
            state
                .advance(admitted(&world, "Past", &text(level), index), &cancel)
                .expect("advances");
        }
        let before = state.to_bytes().expect("bytes");
        let refused = state.assert_progress(&progress(2), &cancel);
        let Err(spine::ClauseRefusal::Contradiction(contradiction)) = refused else {
            panic!("restored {restored}: a contradiction, got {refused:?}");
        };
        assert_eq!(
            contradiction.contradiction.dimension,
            ContradictedDimension::Completeness,
            "restored {restored}"
        );
        assert_eq!(
            state.to_bytes().expect("bytes"),
            before,
            "restored {restored}: the refused call leaves the state unchanged"
        );

        // The same samples are accepted with the batch run's ledger.
        let mut state = progressed_state(
            &world,
            Some(held.clone()),
            fsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        );
        state
            .advance(admitted(&world, "Past", &text(1), 0), &cancel)
            .expect("advances");
        let mut state = reopen(&world, &held, state, restored);
        let advance = deliver(&world, &mut state, &[1, 2, 3], 1, 2).expect("accepted");
        assert_eq!(
            members(&advance.report),
            members(&accepted),
            "restored {restored}"
        );
        assert_eq!(advance.report.ledger.as_ref(), Some(&accepted_ledger));
        let bytes = String::from_utf8(state.to_bytes().expect("bytes")).expect("UTF-8");
        assert_eq!(spelled_samples(&bytes), 0, "restored {restored}: {bytes}");
        let spelled = format!("{:?}", advance.report);
        assert_eq!(
            spelled_samples(&spelled),
            0,
            "restored {restored}: {spelled}"
        );
    }
}

/// FR-993-AC-3 (TC-2810): with `progress.covered_positions` 2, covering a
/// third record-sample position returns `ProgressLimitReached` naming the
/// setting and leaves the ledger as it was, two fingerprints held, in batch
/// and carried evaluation alike.
#[test]
#[trace("TC-2810", "FR-993-AC-3")]
fn the_covered_positions_bound_counts_each_fingerprinted_sample() {
    let world = world();
    let held = batch_from(&world, "Past", &progressed_levels(&[1, 2], 1), None)
        .ledger
        .expect("the retained ledger");
    let fingerprints = |ledger: &spine::ProgressLedger| {
        ledger
            .entries()
            .flat_map(|(_, entry)| entry.covered())
            .filter(|position| position.fingerprint.is_some())
            .count()
    };
    assert_eq!(fingerprints(&held), 2);
    let limited = |report: &ClauseRunReport| match &report.disposition {
        spine::ClauseDisposition::Temporal(spine::TemporalVerdict::ProgressLimitReached(
            reached,
        )) => Some((reached.value, reached.setting)),
        _ => None,
    };

    let batch = spine::run_package_temporal_bounded(
        &PackageTemporalRequest {
            package: world.unit.package(),
            packages: &world.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &DocumentProvision::new(),
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name: "Past",
            trace: &progressed_levels(&[1, 2, 3], 2),
            over: None,
            ledger: Some(&held),
            sources: &world.sources,
            authentication: None,
            cancel: &Cancel::new(),
        },
        2,
    );
    assert_eq!(
        limited(&batch),
        Some((2, "progress.covered_positions")),
        "{:?}",
        batch.disposition
    );

    let mut state = progressed_state(&world, Some(held.clone()), 2);
    let carried = deliver(&world, &mut state, &[1, 2, 3], 0, 2).expect("reports");
    assert_eq!(
        limited(&carried.report),
        Some((2, "progress.covered_positions")),
        "{:?}",
        carried.report.disposition
    );
    for report in [&batch, &*carried.report] {
        if let Some(ledger) = &report.ledger {
            assert_eq!(ledger, &held, "the ledger is unchanged");
            assert_eq!(fingerprints(ledger), 2);
        }
    }
    assert_eq!(members(&carried.report), members(&batch));
}
/// QSL-2138, FR-820, FR-194 "Evaluation": the carried clause state of a
/// closed timestamped trace reads a window aggregate (a filtered count and a
/// quantile) over the same window contents as the one-shot run, so after every
/// position, and after a `from_bytes` restore, its report is the batch report.
#[test]
#[trace("TC-629", "FR-194-AC-1", "FR-820-AC-1")]
fn a_carried_timestamped_state_reads_a_window_aggregate_as_the_one_shot_run() {
    let world = world();
    let cancel = Cancel::new();
    for (name, levels) in [
        ("Twos", [2, 0, 2, 2, 1, 1, 0, 2]),
        ("Median", [0, 1, 3, 3, 2, 0, 0, 1]),
    ] {
        let mut seen_violation = false;
        for restore_after in [None, Some(3_usize)] {
            let open = clause_open(
                &world,
                ClauseTraceKind::Timestamped,
                Some(stamped_binding()),
                ProgressLowerBoundary::AuthoritativeOrigin,
            );
            let mut state =
                ClauseState::open(world.unit.package(), clause_node(&world, name), &open)
                    .expect("opens");
            let mut observations: Vec<TimestampedObservation> = Vec::new();
            for (index, level) in levels.iter().enumerate() {
                if restore_after == Some(index) {
                    let bytes = state.to_bytes().expect("writes");
                    state = ClauseState::from_bytes(
                        world.unit.package(),
                        clause_node(&world, name),
                        &open,
                        &bytes,
                        &cancel,
                    )
                    .expect("reads back");
                }
                let stamp = index as u64;
                let observed = text(*level);
                state
                    .advance(admitted(&world, name, &observed, stamp), &cancel)
                    .expect("advances");
                observations.push(TimestampedObservation {
                    document: spine::sample_position(&observed),
                    stamp,
                    order_key: None,
                });
                let advance = state
                    .assert_progress(&progress(stamp), &cancel)
                    .expect("progress");
                let trace = TraceInput::Timestamped {
                    observations: observations.clone(),
                    progress: progress(stamp),
                    window: Box::new(TimestampedWindow {
                        binding: Some(stamped_binding()),
                        authoritative_origin: true,
                        cover: None,
                        missing_valuations: Vec::new(),
                        late: spine::LateInputs::default(),
                    }),
                };
                let batch = batch_over(&world, name, &trace);
                assert!(
                    !matches!(
                        advance.report.disposition,
                        spine::ClauseDisposition::TraceRefused(_)
                            | spine::ClauseDisposition::EvaluateFault(_)
                    ),
                    "{name} {restore_after:?} at {index}: {:?}",
                    advance.report.disposition
                );
                assert_eq!(
                    members(&advance.report),
                    members(&batch),
                    "{name} restored after {restore_after:?} at {index}"
                );
                seen_violation |= format!("{:?}", batch.disposition).contains("Violated");
            }
        }
        assert!(seen_violation, "{name}: the trace violates the clause");
    }
}

/// The `Timestamped` batch trace of `levels` stamped 0, 1, 2 and so on, under
/// `progress`.
fn stamped_levels(levels: &[i64], progress: ProgressAssertion) -> TraceInput {
    stamped_levels_missing(levels, progress, &[])
}

/// [`stamped_levels`] with `missing` entries: `(position, atom)` valuations
/// the observations hold none of (FR-641).
fn stamped_levels_missing(
    levels: &[i64],
    progress: ProgressAssertion,
    missing: &[(u64, u64)],
) -> TraceInput {
    TraceInput::Timestamped {
        observations: levels
            .iter()
            .enumerate()
            .map(|(stamp, level)| TimestampedObservation {
                document: spine::sample_position(&text(*level)),
                stamp: stamp as u64,
                order_key: None,
            })
            .collect(),
        progress,
        window: Box::new(TimestampedWindow {
            binding: Some(stamped_binding()),
            authoritative_origin: true,
            cover: None,
            missing_valuations: missing
                .iter()
                .map(|(position, atom)| spine::TimestampedMissingValuation {
                    position: *position,
                    atom: *atom,
                })
                .collect(),
            late: spine::LateInputs::default(),
        }),
    }
}

/// FR-820 "Summarized positions", QSL-2138: over a long closed timestamped
/// stream a carried window-aggregate state holds a sample by value only while
/// a window can still read it, so `retained()` is bounded by the window
/// (an open `always[0,5]` activation reads 5 further positions, each its
/// `past[0 ms, 3 ms]`: stamps `T - 8` to `T`), not the trace, and its report
/// still equals the batch report at every position, with and without a
/// `from_bytes` restore taken mid-stream. A restored state holds the same
/// samples as the uninterrupted one.
#[test]
#[trace("TC-2320", "FR-820-AC-2", "FR-194-AC-1")]
fn a_long_carried_timestamped_stream_holds_only_its_windows() {
    let world = world();
    let cancel = Cancel::new();
    let levels: Vec<i64> = (0..60).map(|index| (index * 7 + index / 3) % 5).collect();
    for name in ["Twos", "Median"] {
        let open = clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        );
        let mut state = ClauseState::open(world.unit.package(), clause_node(&world, name), &open)
            .expect("opens");
        let mut peak = 0;
        for (index, level) in levels.iter().enumerate() {
            if index == 30 {
                let bytes = state.to_bytes().expect("writes");
                let before = state.retained();
                state = ClauseState::from_bytes(
                    world.unit.package(),
                    clause_node(&world, name),
                    &open,
                    &bytes,
                    &cancel,
                )
                .expect("reads back");
                assert_eq!(state.retained(), before, "{name}: a restore holds the same");
            }
            let stamp = index as u64;
            state
                .advance(admitted(&world, name, &text(*level), stamp), &cancel)
                .expect("advances");
            let advance = state
                .assert_progress(&progress(stamp), &cancel)
                .expect("progress");
            let batch = batch_over(
                &world,
                name,
                &stamped_levels(&levels[..=index], progress(stamp)),
            );
            assert_eq!(
                members(&advance.report),
                members(&batch),
                "{name} at {index}"
            );
            peak = peak.max(state.retained());
        }
        assert!(
            peak <= 11,
            "{name}: retained {peak} samples over 60 positions, the horizon and windows hold 10"
        );
    }
}

/// The carried report and the batch report of clause `name` over `levels` after each
/// progress assertion of `steps` (completeness `through`, `watermark`), the
/// assertions made one after another on one carried state.
fn clause_progressed(
    name: &str,
    levels: &[i64],
    steps: &[(u64, u64)],
) -> Vec<(std::sync::Arc<ClauseRunReport>, ClauseRunReport)> {
    let world = world();
    let cancel = Cancel::new();
    let mut state = ClauseState::open(
        world.unit.package(),
        clause_node(&world, name),
        &clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        ),
    )
    .expect("opens");
    for (index, level) in levels.iter().enumerate() {
        state
            .advance(admitted(&world, name, &text(*level), index as u64), &cancel)
            .expect("advances");
    }
    steps
        .iter()
        .map(|(through, watermark)| {
            let assertion = ProgressAssertion {
                watermark: *watermark,
                completeness: ProgressCompleteness::Complete { through: *through },
                scope: ProgressClosure::Open,
                execution: ProgressClosure::Open,
            };
            let advance = state
                .assert_progress(&assertion, &cancel)
                .expect("progress");
            let batch = batch_over(&world, name, &stamped_levels(levels, assertion));
            (advance.report, batch)
        })
        .collect()
}

/// [`clause_progressed`] for `Twos`.
fn twos_progressed(
    levels: &[i64],
    steps: &[(u64, u64)],
) -> Vec<(std::sync::Arc<ClauseRunReport>, ClauseRunReport)> {
    clause_progressed("Twos", levels, steps)
}

/// The carried report and the batch report of `Twos` over `levels` with
/// completeness asserted only through 2 and the watermark at 5.
fn twos_complete_through_two(levels: &[i64]) -> (std::sync::Arc<ClauseRunReport>, ClauseRunReport) {
    twos_progressed(levels, &[(2, 5)]).remove(0)
}

fn is_violated(report: &ClauseRunReport) -> bool {
    matches!(
        report.disposition,
        spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Violated { .. })
    )
}

/// FR-194-AC-16, carried route: with completeness asserted only through 2 and
/// the watermark at 5, the carried report equals the batch report.
#[test]
#[trace("TC-629", "FR-194-AC-16", "FR-820-AC-1")]
fn a_carried_window_not_asserted_complete_equals_the_batch() {
    let (carried, batch) = twos_complete_through_two(&[2, 0, 2, 2, 1, 1]);
    assert_eq!(members(&carried), members(&batch));
}

/// FR-194-AC-16: the atom at stamp 3 reaches stamps the completeness through 2
/// does not cover, so a later event could still enter its window and the
/// result is never a final `violated`. The levels refute the clause only at
/// stamps 3, 4 and 5, where the window is not asserted complete (`[2, 0, 2, ..]`
/// would refute it at the covered stamp 2, which is final: see
/// `a_refutation_by_a_complete_atom_stays_final`).
#[test]
#[trace("TC-629", "FR-194-AC-16")]
fn a_window_not_asserted_complete_is_never_a_final_violation() {
    let (carried, batch) = twos_complete_through_two(&[2, 0, 0, 2, 2, 1]);
    for report in [&*carried, &batch] {
        assert!(!is_violated(report), "{:?}", report.disposition);
    }
    assert_eq!(members(&carried), members(&batch));
}

/// FR-194-AC-16: an atom whose window the completeness covers is definite, so
/// its refutation is final whatever the uncovered atoms read.
#[test]
#[trace("TC-629", "FR-194-AC-16")]
fn a_refutation_by_a_complete_atom_stays_final() {
    let (carried, batch) = twos_complete_through_two(&[2, 0, 2, 2, 1, 1]);
    assert!(is_violated(&batch), "{:?}", batch.disposition);
    assert_eq!(members(&carried), members(&batch));
}

/// FR-194-AC-16, progressed trace: the same open carried state first asserted
/// complete through 2 reports no violation, then asserted complete through 5
/// becomes the final `violated` the batch reports, and at every assertion the
/// carried report equals the batch report.
#[test]
#[trace("TC-629", "FR-194-AC-16", "FR-820-AC-1")]
fn a_window_becomes_a_final_verdict_once_completeness_covers_it() {
    let reports = twos_progressed(&[2, 0, 0, 2, 2, 1], &[(2, 5), (3, 5), (5, 5)]);
    let finality: Vec<bool> = reports
        .iter()
        .map(|(carried, batch)| {
            assert_eq!(members(carried), members(batch));
            is_violated(batch)
        })
        .collect();
    assert_eq!(finality, [false, true, true]);
}

/// FR-194-AC-16, `min_count`: `TwosMin` sums the 2s of `past[0 ms, 3 ms]`
/// with `min_count 2` at stamps 2 to 5. Over levels `[2, 0, 2, 0, 1, 1]` the
/// windows at stamps 4 and 5 hold one value, short of 2. With completeness
/// through 5 that is `InsufficientData`; with completeness through 2 those
/// windows are not asserted complete, so the atoms are two-valued, never
/// `InsufficientData`, and the result is indeterminate with reason
/// `IncompleteInput`, not undefined.
#[test]
#[trace("TC-629", "FR-194-AC-16")]
fn an_incomplete_window_short_of_min_count_is_two_valued() {
    let levels = [2, 0, 2, 0, 1, 1];
    let covered = clause_progressed("TwosMin", &levels, &[(5, 5)]).remove(0);
    assert_eq!(members(&covered.0), members(&covered.1));
    assert!(
        matches!(
            covered.1.disposition,
            spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Undefined { .. })
        ),
        "{:?}",
        covered.1.disposition
    );
    let (carried, batch) = clause_progressed("TwosMin", &levels, &[(2, 5)]).remove(0);
    assert_eq!(members(&carried), members(&batch));
    assert!(
        !format!("{:?}", batch.disposition).contains("InsufficientData"),
        "{:?}",
        batch.disposition
    );
    assert!(
        !matches!(
            batch.disposition,
            spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Undefined { .. })
        ),
        "{:?}",
        batch.disposition
    );
}

/// `Twos`'s aggregate atom, under `always` (node 0).
const TWOS_AGGREGATE: u64 = 1;

/// `admitted`, the observation holding no valuation of the atoms `absent`.
fn admitted_absent(
    world: &World,
    name: &str,
    text: &str,
    index: u64,
    absent: &[u64],
) -> AdmittedPosition<'static> {
    AdmittedPosition {
        absent: absent.to_vec(),
        ..admitted(world, name, text, index)
    }
}

/// FR-194-AC-17, carried route: a state whose observation at position 1
/// holds no valuation of the aggregate atom reports, at every step and after a
/// `from_bytes` restore taken mid-stream, what the batch run over the same
/// `missing` entry reports; the atoms whose windows hold position 1 are
/// `MissingValuation` and the closed run is `indeterminate` with reason
/// `missing-valuation` at position 1.
#[test]
#[trace("TC-629", "FR-194-AC-17", "FR-820-AC-1")]
fn a_carried_window_member_without_a_valuation_equals_the_batch() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [1_i64; 14];
    let open = clause_open(
        &world,
        ClauseTraceKind::Timestamped,
        Some(stamped_binding()),
        ProgressLowerBoundary::AuthoritativeOrigin,
    );
    let mut state =
        ClauseState::open(world.unit.package(), clause_node(&world, "Twos"), &open).expect("opens");
    for (index, level) in levels.iter().enumerate() {
        if index == 4 {
            let bytes = state.to_bytes().expect("writes");
            let before = state.retained();
            state = ClauseState::from_bytes(
                world.unit.package(),
                clause_node(&world, "Twos"),
                &open,
                &bytes,
                &cancel,
            )
            .expect("reads back");
            assert_eq!(state.retained(), before, "a restore holds the same");
            assert_eq!(state.to_bytes().expect("writes"), bytes);
            // The reader is strict: an absence list is ascending without repeats.
            let text = String::from_utf8(bytes.clone()).expect("UTF-8");
            assert!(text.contains(r#""absent":[1]"#), "{text}");
            for bad in [r#""absent":[1,1]"#, r#""absent":[2,1]"#] {
                let tampered = text.replace(r#""absent":[1]"#, bad);
                assert!(
                    ClauseState::from_bytes(
                        world.unit.package(),
                        clause_node(&world, "Twos"),
                        &open,
                        tampered.as_bytes(),
                        &cancel,
                    )
                    .is_err(),
                    "{bad}"
                );
            }
        }
        let stamp = index as u64;
        let absent: &[u64] = if index == 1 { &[TWOS_AGGREGATE] } else { &[] };
        state
            .advance(
                admitted_absent(&world, "Twos", &text(*level), stamp, absent),
                &cancel,
            )
            .expect("advances");
        let advance = state
            .assert_progress(&progress(stamp), &cancel)
            .expect("progress");
        let missing: Vec<(u64, u64)> = (index >= 1)
            .then_some((1, TWOS_AGGREGATE))
            .into_iter()
            .collect();
        let batch = batch_over(
            &world,
            "Twos",
            &stamped_levels_missing(&levels[..=index], progress(stamp), &missing),
        );
        assert_eq!(members(&advance.report), members(&batch), "at {index}");
        if index == levels.len() - 1 {
            let shown = format!("{:?}", batch.disposition);
            assert!(shown.contains("MissingValuation"), "{shown}");
            assert!(shown.contains("position: 1"), "{shown}");
        }
    }
}

/// FR-993, FR-194-AC-17: an absence is content of its position, so a state
/// that delivers a position without the absence the covered ledger holds for
/// it is refused as a `Completeness` contradiction, before and after a
/// `from_bytes` restore, including when a later position is still uncovered.
#[test]
#[trace("TC-629", "FR-194-AC-17", "FR-993-AC-1")]
fn a_changed_absence_at_a_covered_index_is_a_completeness_contradiction() {
    let world = world();
    let cancel = Cancel::new();
    let levels = [1_i64; 4];
    let held = batch_from(
        &world,
        "Twos",
        &stamped_levels_missing(&levels, progress(3), &[(1, TWOS_AGGREGATE)]),
        None,
    )
    .ledger
    .expect("the retained ledger");
    for (restored, uncovered) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut open = clause_open(
            &world,
            ClauseTraceKind::Timestamped,
            Some(stamped_binding()),
            ProgressLowerBoundary::AuthoritativeOrigin,
        );
        open.ledger = Some(held.clone());
        let mut state = ClauseState::open(world.unit.package(), clause_node(&world, "Twos"), &open)
            .expect("opens");
        let advance = state
            .advance(admitted(&world, "Twos", &text(1), 0), &cancel)
            .expect("advances");
        assert_eq!(advance.report.ledger.as_ref(), Some(&held));
        if restored {
            let bytes = state.to_bytes().expect("writes");
            state = ClauseState::from_bytes(
                world.unit.package(),
                clause_node(&world, "Twos"),
                &open,
                &bytes,
                &cancel,
            )
            .expect("reads back");
            assert_eq!(state.report().ledger.as_ref(), Some(&held));
        }
        for index in 1..4_u64 {
            // The covered index 1 arrives without its absence.
            state
                .advance(admitted(&world, "Twos", &text(1), index), &cancel)
                .expect("advances");
        }
        if uncovered {
            state
                .advance(admitted(&world, "Twos", &text(1), 5), &cancel)
                .expect("an uncovered observation asserts no progress");
        }
        let before = state.to_bytes().expect("writes before the assertion");
        let refused = state.assert_progress(&progress(3), &cancel);
        let Err(spine::ClauseRefusal::Contradiction(contradiction)) = refused else {
            panic!("restored {restored}, uncovered {uncovered}: a contradiction, got {refused:?}");
        };
        assert_eq!(
            contradiction.contradiction.dimension,
            ContradictedDimension::Completeness,
            "restored {restored}, uncovered {uncovered}"
        );
        assert_eq!(state.to_bytes().expect("writes after refusal"), before);
    }
}

/// FR-194: only a `Timestamped` state evaluates absent valuations, so any
/// other kind refuses a position that names some, before changing anything.
#[test]
#[trace("TC-629", "FR-194-AC-17")]
fn an_absent_valuation_needs_a_timestamped_state() {
    let world = world();
    let mut state = finite_state(&world, "Past");
    let before = state.to_bytes().expect("writes");
    let refused = state.advance(
        admitted_absent(&world, "Past", &text(1), 0, &[1]),
        &Cancel::new(),
    );
    assert!(
        matches!(refused, Err(spine::ClauseRefusal::Fault(_))),
        "{refused:?}"
    );
    assert_eq!(state.to_bytes().expect("writes"), before);
}
fn qsl_2750_world() -> World {
    world_source(&format!(
        "{UNIT}\nrecord Response {{ latency: Quantity<ms>; }}\n\
        temporal MissingQuantile using w over (r: Response) clock \"plant-ms\" on origin {{\n\
        always[0,300000] (quantile 0.95 of r.latency over past[0 ms,300 s] <= 5 ms) }}\n"
    ))
}

fn qsl_2750_atom(world: &World) -> u64 {
    u64::try_from(
        clause_node(world, "MissingQuantile")
            .clause()
            .formula()
            .nodes()
            .iter()
            .position(|node| matches!(node, fsl_semantics::check::FormulaNode::Aggregate { .. }))
            .expect("the real checked clause contains its aggregate atom"),
    )
    .unwrap()
}

fn qsl_2750_open(world: &World) -> MonitorOpen<'_> {
    MonitorOpen {
        clauses: clauses(&["MissingQuantile"]),
        kind: MonitorTraceKind::Timestamped,
        lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
        binding: Some(stamped_binding()),
        cover: None,
        ledger: None,
        packages: &world.packages,
        sources: &world.sources,
        temporal_selections: &world.selections,
    }
}

fn qsl_2750_response(latency: i64) -> String {
    serde_json::json!({"latency":{"kind":"quantity", "unit":"ms",
        "magnitude":{"kind":"rational","numerator":latency.to_string(),"denominator":"1"}}})
    .to_string()
}

fn qsl_2750_trace(
    observations: Vec<TimestampedObservation>,
    missing: Vec<spine::TimestampedMissingValuation>,
    assertion: ProgressAssertion,
) -> TraceInput {
    TraceInput::Timestamped {
        observations,
        progress: assertion,
        window: Box::new(TimestampedWindow {
            binding: Some(stamped_binding()),
            authoritative_origin: true,
            cover: None,
            missing_valuations: missing,
            late: Default::default(),
        }),
    }
}

#[test]
#[trace("FR-643-AC-13", "FR-194-AC-17")]
fn qsl_2750_missing_window_matches_batch_and_checkpoint() {
    let world = qsl_2750_world();
    qsl_2750_uncovered_reordering(&world);
    qsl_2750_late_values_do_not_settle_early(&world);
    let cancel = Cancel::new();
    let atom = qsl_2750_atom(&world);
    for absent in [false, true] {
        for restore in [false, true] {
            let mut session = MonitorSession::open(
                world.unit.package(),
                qsl_2750_open(&world),
                &limits(100),
                &cancel,
            )
            .unwrap();
            let mut frozen = None;
            let mut observed = Vec::new();
            let mut missing = Vec::new();
            let mut assertion = ProgressAssertion {
                watermark: 0,
                completeness: ProgressCompleteness::Incomplete,
                scope: ProgressClosure::Open,
                execution: ProgressClosure::Open,
            };
            for index in 0_u64..20 {
                let response = qsl_2750_response(match index {
                    18 => 6,
                    19 => 7,
                    _ => 1,
                });
                let entries = if absent && index == 18 {
                    vec![spine::TimestampedMissingValuation {
                        position: index,
                        atom,
                    }]
                } else {
                    Vec::new()
                };
                let step = session
                    .step(
                        MonitorObservation {
                            position: MonitorPosition::Sample(&response),
                            stamp: Some(MonitorStamp::Tick(index)),
                            order_key: None,
                            missing: &entries,
                        },
                        &cancel,
                    )
                    .unwrap();
                observed.push(TimestampedObservation {
                    document: spine::sample_position(&response),
                    stamp: index,
                    order_key: None,
                });
                missing.extend(entries);
                let batch = qsl_2750_batch_over(
                    &world,
                    "MissingQuantile",
                    &qsl_2750_trace(observed.clone(), missing.clone(), assertion),
                );
                qsl_2750_assert_status(&step.statuses[0].1, &batch, &mut frozen);
                if restore && index == 18 {
                    session = qsl_2750_restore_report(&world, session, &batch, &frozen, &cancel);
                }
                assertion = progress(index);
                let step = session.assert_progress(assertion, &cancel).unwrap();
                let batch = qsl_2750_batch_over(
                    &world,
                    "MissingQuantile",
                    &qsl_2750_trace(observed.clone(), missing.clone(), assertion),
                );
                qsl_2750_assert_status(&step.statuses[0].1, &batch, &mut frozen);
                if restore && index == 18 {
                    session = qsl_2750_restore_report(&world, session, &batch, &frozen, &cancel);
                }
            }
            assertion = ProgressAssertion {
                watermark: 300000,
                completeness: ProgressCompleteness::Complete { through: 300000 },
                scope: ProgressClosure::Closed,
                execution: ProgressClosure::Closed,
            };
            let step = session.assert_progress(assertion, &cancel).unwrap();
            let batch = qsl_2750_batch_over(
                &world,
                "MissingQuantile",
                &qsl_2750_trace(observed, missing, assertion),
            );
            qsl_2750_assert_status(&step.statuses[0].1, &batch, &mut frozen);
            let final_report = session.finish(&cancel).unwrap();
            qsl_2750_assert_finished(&final_report.records[0], &batch, &frozen);
            if absent {
                assert!(
                    matches!(
                        &batch.disposition,
                        spine::ClauseDisposition::Temporal(spine::TemporalVerdict::Indeterminate {
                            reason: spine::TemporalIndeterminateReason::MissingValuation {
                                position: 18,
                                ..
                            },
                            ..
                        })
                    ),
                    "{:?}",
                    batch.disposition
                );
            } else {
                assert_eq!(
                    final_report.records[0].category,
                    filament_semantic_value::catalog::Category::Violation
                );
            }
        }
    }
}

#[test]
#[trace("FR-643-AC-14")]
fn qsl_2750_missing_marker_refusals_preserve_session_state() {
    let world = qsl_2750_world();
    let cancel = Cancel::new();
    let response = qsl_2750_response(1);
    let atom = qsl_2750_atom(&world);
    let non_atom = u64::try_from(
        clause_node(&world, "MissingQuantile")
            .clause()
            .formula()
            .nodes()
            .iter()
            .position(|node| {
                !matches!(
                    node,
                    fsl_semantics::check::FormulaNode::Holds { .. }
                        | fsl_semantics::check::FormulaNode::Aggregate { .. }
                )
            })
            .expect("the surrounding always node is not an atom"),
    )
    .unwrap();
    let mut session = MonitorSession::open(
        world.unit.package(),
        qsl_2750_open(&world),
        &limits(100),
        &cancel,
    )
    .unwrap();
    session.step(stamped(&response, 0), &cancel).unwrap();
    let before = session.checkpoint().unwrap();
    for marker in [
        spine::TimestampedMissingValuation { position: 0, atom },
        spine::TimestampedMissingValuation { position: 2, atom },
        spine::TimestampedMissingValuation {
            position: 1,
            atom: non_atom,
        },
        spine::TimestampedMissingValuation {
            position: 1,
            atom: u64::MAX,
        },
    ] {
        let mut observation = stamped(&response, 1);
        observation.missing = std::slice::from_ref(&marker);
        assert!(matches!(
            session.step(observation, &cancel),
            Err(StageFailure::Refused(MonitorRefusal::Observation {
                member: "missing",
                cause: "invalid-value"
            }))
        ));
        assert_eq!(session.observations_seen(), 1);
        assert_eq!(session.checkpoint().unwrap().as_bytes(), before.as_bytes());
    }
    session.step(stamped(&response, 1), &cancel).unwrap();
    assert_eq!(session.observations_seen(), 2);
    let ordinary = self::world();
    for (kind, name, binding) in [
        (MonitorTraceKind::Finite, "Never", None),
        (
            MonitorTraceKind::Progressed,
            "Recent",
            Some(events_binding()),
        ),
    ] {
        let mut session = ordinary.open(&[name], kind, binding, 100).unwrap();
        let text = text(0);
        session.step(sample(&text), &cancel).unwrap();
        let before = session.checkpoint().unwrap();
        let marker = [spine::TimestampedMissingValuation {
            position: 1,
            atom: 0,
        }];
        let mut observation = sample(&text);
        observation.missing = &marker;
        assert!(matches!(
            session.step(observation, &cancel),
            Err(StageFailure::Refused(MonitorRefusal::Observation {
                member: "missing",
                cause: "invalid-value"
            }))
        ));
        assert_eq!(session.checkpoint().unwrap().as_bytes(), before.as_bytes());
        session.step(sample(&text), &cancel).unwrap();
    }
}

/// Provisional reports must leave the uncovered arrival order and values
/// available for a later event, even across a checkpoint.
fn qsl_2750_uncovered_reordering(world: &World) {
    let cancel = Cancel::new();
    let atom = qsl_2750_atom(world);
    let mut session = MonitorSession::open(
        world.unit.package(),
        qsl_2750_open(world),
        &limits(100),
        &cancel,
    )
    .unwrap();
    let initial = ProgressAssertion {
        watermark: 0,
        completeness: ProgressCompleteness::Incomplete,
        scope: ProgressClosure::Open,
        execution: ProgressClosure::Open,
    };
    let mut observed = Vec::new();
    let mut missing = Vec::new();
    let mut inputs = Vec::new();
    for (index, (stamp, value)) in [(2_u64, 6), (1, 1), (3, 7)].into_iter().enumerate() {
        let text = qsl_2750_response(value);
        let entries = if index == 0 {
            vec![spine::TimestampedMissingValuation { position: 0, atom }]
        } else {
            Vec::new()
        };
        let step = session
            .step(
                MonitorObservation {
                    position: MonitorPosition::Sample(&text),
                    stamp: Some(MonitorStamp::Tick(stamp)),
                    order_key: None,
                    missing: &entries,
                },
                &cancel,
            )
            .unwrap();
        observed.push(TimestampedObservation {
            document: spine::sample_position(&text),
            stamp,
            order_key: None,
        });
        missing.extend(entries);
        inputs.push((stamp, text));
        let batch = qsl_2750_batch_over(
            world,
            "MissingQuantile",
            &qsl_2750_trace(observed.clone(), missing.clone(), initial),
        );
        assert_eq!(
            qsl_2750_report_fields(step.statuses[0].1.record().report()),
            qsl_2750_report_fields(&batch),
            "uncovered step {index}"
        );
        let checkpoint = session.checkpoint().unwrap();
        let saved: serde_json::Value = serde_json::from_slice(checkpoint.as_bytes()).unwrap();
        assert!(
            saved["ledger"].is_null(),
            "a provisional report asserts no watermark"
        );
        assert!(saved["assertion"].is_null());
        let clause: serde_json::Value =
            serde_json::from_str(saved["clauses"][0]["state"].as_str().unwrap()).unwrap();
        assert!(
            clause["evaluated"].is_null(),
            "the uncovered canonical prefix is not frozen"
        );
        let positions = clause["positions"].as_array().unwrap();
        assert_eq!(positions.len(), inputs.len());
        for (position, (stamp, text)) in positions.iter().zip(&inputs) {
            assert_eq!(position["stamp"], *stamp, "arrival order stays mutable");
            assert_eq!(
                position["identity"], *text,
                "a later window can still read the sample"
            );
        }
        session = MonitorSession::resume(
            world.unit.package(),
            &checkpoint,
            qsl_2750_open(world),
            &limits(100),
            &cancel,
        )
        .unwrap();
        assert_eq!(
            session.checkpoint().unwrap().as_bytes(),
            checkpoint.as_bytes(),
            "resume preserves the uncovered state"
        );
    }
    let covered = ProgressAssertion {
        watermark: 300000,
        completeness: ProgressCompleteness::Complete { through: 300000 },
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
    };
    let step = session.assert_progress(covered, &cancel).unwrap();
    let batch = qsl_2750_batch_over(
        world,
        "MissingQuantile",
        &qsl_2750_trace(observed, missing, covered),
    );
    assert_eq!(
        qsl_2750_report_fields(step.statuses[0].1.record().report()),
        qsl_2750_report_fields(&batch),
        "the real assertion canonicalizes all original evidence"
    );
    let finished = session.finish(&cancel).unwrap();
    assert_eq!(
        qsl_2750_report_fields(finished.records[0].report()),
        qsl_2750_report_fields(&batch)
    );
}

/// Every ClauseRunReport field except usage, which FR-643 explicitly excludes
/// from batch equality. The deciding report is captured from the batch oracle.
fn qsl_2750_report_fields(report: &ClauseRunReport) -> String {
    format!(
        "{:?}",
        (
            &report.disposition,
            &report.basis,
            &report.witness,
            &report.state_clause_counterexample,
            &report.frame_field_values,
            &report.consequent_source_events,
            &report.ledger,
            &report.finite_window,
            &report.counterexample,
            &report.decisive,
            &report.authenticated_temporal_selection,
            &report.refused_at
        )
    )
}

fn qsl_2750_assert_status(
    status: &fsl_analyze::monitor_session::ClauseStatus,
    current: &ClauseRunReport,
    frozen: &mut Option<String>,
) {
    let record = status.record();
    assert_eq!(
        record.category,
        current.disposition.category(),
        "category agrees over every extension"
    );
    assert_eq!(
        record.report().disposition.truth(),
        current.disposition.truth(),
        "truth agrees over every extension"
    );
    let batch = qsl_2750_report_fields(current);
    let expected = if status.is_settled() {
        frozen.get_or_insert(batch)
    } else {
        assert!(frozen.is_none(), "a settled clause never becomes pending");
        &batch
    };
    assert_eq!(
        &qsl_2750_report_fields(record.report()),
        expected,
        "pending reports equal B(k); settled reports retain every member of B(s)"
    );
}

fn qsl_2750_assert_finished(
    record: &fsl_analyze::monitor::MonitorRecord,
    current: &ClauseRunReport,
    frozen: &Option<String>,
) {
    assert_eq!(record.category, current.disposition.category());
    assert_eq!(
        record.report().disposition.truth(),
        current.disposition.truth()
    );
    let batch = qsl_2750_report_fields(current);
    assert_eq!(
        &qsl_2750_report_fields(record.report()),
        frozen.as_ref().unwrap_or(&batch)
    );
}

/// Inspect the resumed report through finish on an independent resume, so the
/// primary session can continue without introducing a new counted call.
fn qsl_2750_restore_report<'a>(
    world: &'a World,
    session: MonitorSession<'a>,
    batch: &ClauseRunReport,
    frozen: &Option<String>,
    cancel: &Cancel,
) -> MonitorSession<'a> {
    let checkpoint = session.checkpoint().unwrap();
    let resume = || {
        MonitorSession::resume(
            world.unit.package(),
            &checkpoint,
            qsl_2750_open(world),
            &limits(100),
            cancel,
        )
        .unwrap()
    };
    let probe = resume().finish(cancel).unwrap();
    qsl_2750_assert_finished(&probe.records[0], batch, frozen);
    let resumed = resume();
    assert_eq!(
        resumed.checkpoint().unwrap().as_bytes(),
        checkpoint.as_bytes()
    );
    resumed
}

/// Without completeness, later on-time window members can change the
/// quantile. Reporting a prefix must not turn its provisional value final.
fn qsl_2750_late_values_do_not_settle_early(world: &World) {
    let cancel = Cancel::new();
    let mut session = MonitorSession::open(
        world.unit.package(),
        qsl_2750_open(world),
        &limits(100),
        &cancel,
    )
    .unwrap();
    let default = ProgressAssertion {
        watermark: 0,
        completeness: ProgressCompleteness::Incomplete,
        scope: ProgressClosure::Open,
        execution: ProgressClosure::Open,
    };
    let covered = ProgressAssertion {
        watermark: 600000,
        completeness: ProgressCompleteness::Complete { through: 600000 },
        scope: ProgressClosure::Closed,
        execution: ProgressClosure::Closed,
    };
    let mut observed = Vec::new();
    let mut frozen = None;
    for (index, stamp) in std::iter::once(200000_u64).chain(1..=20).enumerate() {
        let sample = qsl_2750_response(if index == 0 { 6 } else { 1 });
        let step = session
            .step(
                MonitorObservation {
                    position: MonitorPosition::Sample(&sample),
                    stamp: Some(MonitorStamp::Tick(stamp)),
                    order_key: None,
                    missing: &[],
                },
                &cancel,
            )
            .unwrap();
        observed.push(TimestampedObservation {
            document: spine::sample_position(&sample),
            stamp,
            order_key: None,
        });
        let batch = qsl_2750_batch_over(
            world,
            "MissingQuantile",
            &qsl_2750_trace(observed.clone(), Vec::new(), default),
        );
        qsl_2750_assert_status(&step.statuses[0].1, &batch, &mut frozen);
        assert!(
            !step.statuses[0].1.is_settled(),
            "uncovered quantile can still change at arrival {index}"
        );
        if index == 0 {
            // The first value would refute a completed trace, but no such
            // completeness has been asserted on the live session.
            let closed = qsl_2750_batch_over(
                world,
                "MissingQuantile",
                &qsl_2750_trace(observed.clone(), Vec::new(), covered),
            );
            assert_eq!(
                closed.disposition.category(),
                filament_semantic_value::catalog::Category::Violation
            );
            session = qsl_2750_restore_report(world, session, &batch, &frozen, &cancel);
        }
    }
    let step = session.assert_progress(covered, &cancel).unwrap();
    let batch = qsl_2750_batch_over(
        world,
        "MissingQuantile",
        &qsl_2750_trace(observed, Vec::new(), covered),
    );
    assert_eq!(
        batch.disposition.category(),
        filament_semantic_value::catalog::Category::Success,
        "twenty late values of one change the quantile from six to one"
    );
    qsl_2750_assert_status(&step.statuses[0].1, &batch, &mut frozen);
    let finished = session.finish(&cancel).unwrap();
    qsl_2750_assert_finished(&finished.records[0], &batch, &frozen);
}

/// The session authenticates its clock selections when it opens; its batch
/// oracle supplies that same proving-package context on every prefix.
fn qsl_2750_batch_over(world: &World, name: &str, trace: &TraceInput) -> ClauseRunReport {
    let cancel = Cancel::new();
    let authentication = spine::authenticate_emission(
        world.unit.package(),
        &world.selections,
        &world.packages,
        &SpineLimits::default(),
        &cancel,
    )
    .expect("the fixture's same temporal selections authenticate");
    run_package_temporal(&PackageTemporalRequest {
        package: world.unit.package(),
        packages: &world.packages,
        model_limits: ModelNormalizationLimits::default(),
        snapshots: &DocumentProvision::new(),
        observation_limits: ObservationLimits::default(),
        accounting: UNLIMITED,
        name,
        trace,
        over: None,
        ledger: None,
        sources: &world.sources,
        authentication: Some(authentication.temporal()),
        cancel: &cancel,
    })
}
