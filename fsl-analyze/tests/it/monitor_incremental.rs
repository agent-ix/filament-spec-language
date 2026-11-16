// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-643 (TC-1666, TC-1667): the stepping monitor over a record trace
//! subject. Each test feeds observations one at a time and compares the
//! session with FR-283's batch `monitor`, or with the batch trace run.

use filament_exact::Cancel;
use fsl_analyze::monitor::{
    MonitorClause, MonitorLimits, MonitorRefusal, MonitorVerdict, DEFAULT_RETAINED_POSITIONS,
};
use fsl_analyze::monitor_session::{
    MonitorObservation, MonitorOpen, MonitorPosition, MonitorSession, MonitorStamp,
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

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile e = \"filament.temporal.event-position.false-extension/v1\";\n\
    profile w = \"filament.temporal.timestamped-event.finite-window/v1\";\n\
    profile i = \"filament.temporal.infinite-trace/v1\";\n\
    record Reading { level: Int[0, 9]; }\n\
    temporal E using e over (r: Reading) clock \"events\" on origin { eventually[0,2] holds(r.level = 2) }\n\
    temporal T using w over (r: Reading) clock \"plant-ms\" on origin { eventually[0,5] holds(r.level > 0) }\n\
    temporal Reaches using i over (r: Reading) clock \"steps\" on origin { eventually holds(r.level = 2) }\n\
    temporal Never using i over (r: Reading) clock \"steps\" on origin { always holds(r.level < 2) }\n    temporal Below using i over (r: Reading) clock \"steps\" on origin { always holds(r.level < 9) }\n    temporal Recent using e over (r: Reading) clock \"events\" on origin { always[0,9] (count where holds(r.level = 2) over past[0, 3] min_count 0 <= 1) }\n";

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
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("agent-ix", "monitor-incremental");
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("no domain package");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: PATH,
            bytes: UNIT.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("selects")
        .into_value();
    let cst = fsl_cst::parse(identity, PATH, UNIT.as_bytes(), fsl_cst::Limits::default())
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
    }
}

fn stamped<'a>(text: &'a str, stamp: u64) -> MonitorObservation<'a> {
    MonitorObservation {
        position: MonitorPosition::Sample(text),
        stamp: Some(MonitorStamp::Tick(stamp)),
        order_key: None,
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
        vec![MonitorVerdict::Pending, MonitorVerdict::Violated]
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
    for (stamp, level) in [(0, 0), (2, 0), (5, 1)] {
        let observed = text(level);
        session
            .step(stamped(&observed, stamp), &cancel)
            .expect("an observation");
        last = Some(
            session
                .assert_progress(progress(stamp), &cancel)
                .expect("progress"),
        );
        if stamp == 2 {
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

    // The batch run over the same three observations.
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
        progress: progress(5),
        window: Box::new(TimestampedWindow {
            binding: Some(stamped_binding()),
            authoritative_origin: true,
            cover: None,
            missing_valuations: Vec::new(),
        }),
    };
    let batch = run_package_temporal(&PackageTemporalRequest {
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
    });
    let from_session = outcome.records[0].report();
    assert_eq!(
        format!("{:?}", from_session.disposition),
        format!("{:?}", batch.disposition)
    );
    assert_eq!(from_session.ledger, batch.ledger);
    assert!(batch.ledger.is_some());
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
    let state = head.checkpoint();
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
                .map(|(_, status)| (status.record().verdict, status.is_settled()))
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

/// FR-643: a kind the clause's profile does not read refuses at open, before
/// any observation.
#[test]
#[trace("TC-1667", "FR-643-AC-1")]
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
    assert!(matches!(
        refused,
        Some(MonitorRefusal::ClauseRefused { .. })
    ));
}

fn category(
    step: &fsl_analyze::monitor_session::MonitorStep,
    index: usize,
) -> filament_semantic_value::catalog::Category {
    step.statuses[index].1.record().category
}

/// FR-643-AC-6 (TC-2321): `Never` settles `violation` at step 3 and then
/// reports `B(3)`; `Reaches` stays `Pending` and reports `B(k)`.
#[test]
#[trace("TC-2321", "FR-643-AC-6")]
fn never_settles_at_the_bad_prefix_and_reaches_stays_pending() {
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
        let batch = world.batch(&names, &levels[..=index]);
        assert!(
            !step.statuses[0].1.is_settled(),
            "Reaches pending at {index}"
        );
        assert_eq!(
            format!("{:?}", step.statuses[0].1.record().report().disposition),
            batch[0]
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
        world.batch(&["Reaches"], &levels)[0]
    );
    assert_eq!(outcome.settled_before_end, vec![false, true]);
}

/// FR-643-AC-7 (TC-2321): the event-position `E` stays `Pending` over levels
/// 0, 1 while the closed reading is `violation`, settles `success` with a
/// decisive witness on level 2, and `violation` with a decisive
/// counterexample once the window is wholly observed.
#[test]
#[trace("TC-2321", "FR-643-AC-7")]
fn a_closed_trace_clause_settles_on_its_decisive_reading() {
    use filament_semantic_value::catalog::Category;
    let world = world();
    let cancel = Cancel::new();
    for (levels, category_at_three) in [
        ([0, 1, 2], Category::Success),
        ([0, 1, 1], Category::Violation),
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
                assert!(
                    status.is_settled(),
                    "{levels:?} step 3: {:?} {:?}",
                    status.record().report().basis,
                    status.record().report().disposition
                );
                assert_eq!(status.record().category, category_at_three);
                let window = status
                    .record()
                    .report()
                    .finite_window
                    .as_ref()
                    .expect("a window report");
                assert!(window.basis.is_decisive(), "{:?}", window.basis);
            }
        }
    }
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
    let mut last = None;
    for level in [0, 0, 2, 0, 1] {
        let observed = text(level);
        last = Some(session.step(sample(&observed), &cancel).expect("steps"));
    }
    let step = last.expect("a step");
    assert_eq!(category(&step, 0), Category::Violation);
    assert_eq!(category(&step, 1), Category::Incomplete);
    assert!(step.statuses.iter().all(|(_, status)| status.is_settled()));
}

/// FR-643-AC-5 (TC-1667): with `retained_positions` 2 over levels 0, 1, 0 the
/// event-position `Recent` reports `incomplete` while `Reaches` and `Never`
/// stay `Pending`. It waits on FR-820's value pruning.
#[test]
#[trace("TC-1667", "FR-643-AC-5")]
#[ignore = "QSL-2307: FR-820 incremental ClauseState and value pruning"]
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
    let state = original.checkpoint();
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
    assert_eq!(outcome.settled_before_end, vec![false]);
}
