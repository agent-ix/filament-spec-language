// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-641-AC-4 (TC-1663), FR-330-AC-16 (TC-840): a model-arm behaviour reaches
//! FR-330 as `TraceInput::Model` and reports the outcome of the lasso
//! evaluator over the states its steps reconstruct. The behaviours are the
//! ones EN-1 finds over the `Counter` subject, so the steps and post-state
//! keys are the model's own.

use filament_exact::{LimitKind, ScalarLimits, Value};
use fsl_eval::simulation::StateKey;
use fsl_foundation::witness::ValuePathSubject;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    run_clause, ClauseDisposition, ClauseRunReport, ClauseRunRequest, ClauseRunSelection,
    ClauseRunSource, ClauseRunStage, DependencyInput, OverObject, SpineLimits, TemporalVerdict,
    TraceInput,
};
use fsl_replay::{CounterexampleKind, ModelCheckLimit, TemporalCounterexample};
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::trace;

use crate::counter::{document, document_with_ranged_parameter, PACKAGE, UNLIMITED};
use crate::lasso_replay::found;
use crate::liveness::{counter, Fixture};

/// The work-unit bound a model run stops its first expansion at.
const STOPS_AT: u64 = 1;

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

fn is_zero(value: i64) -> Option<bool> {
    Some(value == 0)
}

fn at_most_one(value: i64) -> Option<bool> {
    Some(value <= 1)
}

/// The model-arm behaviour `payload` as a run's `TraceInput::Model`.
fn model_trace(fixture: &Fixture, payload: &TemporalCounterexample) -> TraceInput {
    let (steps, terminal_stutter, entry) = match payload.lasso {
        Some(lasso) => (payload.prefix.clone(), lasso.terminal_stutter, lasso.entry),
        None => (payload.prefix.clone(), false, payload.prefix.len()),
    };
    let (prefix, repeat) = steps.split_at(entry.min(steps.len()));
    TraceInput::Model {
        initial: fixture.initial[0].clone(),
        universes: fixture.initial.clone(),
        prefix: prefix.to_vec(),
        repeat: repeat.to_vec(),
        terminal_stutter,
    }
}

/// The run of the fixture's clause `C` over `trace`.
fn run(fixture: &Fixture, trace: TraceInput) -> ClauseRunReport {
    run_under(fixture, trace, UNLIMITED)
}

/// [`run`] under the accounting limits `accounting`.
fn run_under(fixture: &Fixture, trace: TraceInput, accounting: ScalarLimits) -> ClauseRunReport {
    run_clause(ClauseRunRequest {
        clocks: Default::default(),
        source: ClauseRunSource::Program {
            identity: SourceIdentity::new("test", "model-trace"),
            path: "model-trace.native".to_owned(),
            bytes: fixture.unit.clone(),
        },
        packages: fixture.documents.packages.clone(),
        dependencies: DependencyInput::default(),
        snapshots: fixture.documents.snapshots.clone(),
        invocations: Default::default(),
        selection: ClauseRunSelection::Temporal {
            name: "C".to_owned(),
            trace,
            over: OverObject {
                population: format!("ix://{PACKAGE}/counters"),
                key: "c".to_owned(),
            },
            ledger: None,
        },
        package_name: "fixture".to_owned(),
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        accounting,
    })
    .expect("a request that forms always reports")
}

/// The behaviour EN-1 finds against `always holds(c.value <= 1)`, and the
/// fixture of the same clause.
fn violating() -> (Fixture, TemporalCounterexample) {
    let fixture = counter(
        document_with_ranged_parameter(),
        "",
        "always holds(c.value <= 1)",
    );
    let payload = found(&fixture, at_most_one);
    assert!(
        matches!(payload.kind, CounterexampleKind::Formula),
        "{payload:?}"
    );
    (fixture, payload)
}

/// The lasso `0 -inc-> 1 -inc-> 2 -inc-> 3` ending in the terminal stutter at
/// 3, and the fixture of `always holds(c.value <= 1)` over it. The steps and
/// post-state keys are the ones EN-1 finds for a liveness clause over the
/// same subject, so they are the model's own.
fn stuttering() -> (Fixture, TemporalCounterexample) {
    let liveness = counter(document(false), "", "always eventually holds(c.value = 0)");
    let payload = found(&liveness, is_zero);
    assert_eq!(payload.prefix.len(), 3, "{payload:?}");
    assert!(
        matches!(payload.lasso, Some(lasso) if lasso.terminal_stutter && lasso.entry == 3),
        "{payload:?}"
    );
    let fixture = counter(document(false), "", "always holds(c.value <= 1)");
    (fixture, payload)
}

/// FR-641-AC-4, FR-330-AC-16: over the model lasso `0, 1, 2, 3` and its
/// terminal stutter, `always holds(c.value <= 1)` is violated at position 0.
/// The decisive atom is read at position 2, the first state where it is
/// false, over that state's observation: no quantifier decides it, so the
/// element is the atom's value `false` at the root of the counter's own
/// observation.
#[trace("TC-1663", "FR-641-AC-4", "TC-840", "FR-330-AC-16")]
#[test]
fn a_model_trace_reports_violated_with_a_decisive_position() {
    let (fixture, payload) = stuttering();
    let report = run(&fixture, model_trace(&fixture, &payload));
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Temporal(TemporalVerdict::Violated { at: 0 })
        ),
        "{:?}",
        report.disposition
    );
    let decisive = report
        .decisive
        .expect("a violation names its decisive atom");
    assert_eq!(decisive.position, Some(2));
    assert_eq!(decisive.quantifier, None);
    assert_eq!(decisive.index, None);
    assert!(
        matches!(decisive.element, Value::Boolean(false)),
        "{:?}",
        decisive.element
    );
    assert_eq!(
        decisive.value_path.observation.identity,
        "filament.simulation.state"
    );
    assert!(decisive.value_path.steps.is_empty());
    let ValuePathSubject::Object(reference) = &decisive.value_path.subject else {
        panic!("{:?}", decisive.value_path.subject);
    };
    assert_eq!(reference.object().as_str(), "c");
    assert!(report.usage.evaluation_admissions > 0, "{:?}", report.usage);
}

/// FR-641-AC-4: the same lasso answers a clause that holds over every state it
/// visits.
#[trace("TC-1663", "FR-641-AC-4")]
#[test]
fn a_model_trace_holds_over_every_state_it_visits() {
    let liveness = counter(
        document_with_ranged_parameter(),
        "",
        "always eventually holds(c.value = 3)",
    );
    let payload = found(&liveness, is_three);
    assert!(payload.lasso.is_some(), "{payload:?}");
    let holds = counter(
        document_with_ranged_parameter(),
        "",
        "always holds(c.value <= 3)",
    );
    let report = run(&holds, model_trace(&holds, &payload));
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Temporal(TemporalVerdict::Holds)
        ),
        "{:?}",
        report.disposition
    );
}

/// FR-645-AC-2, FR-329-AC-3: a model behaviour whose loop holds no step is
/// refused `empty-lasso-loop` at stage `evaluate`, never read as a finite
/// prefix, and the request member it is about is the model's `loop`.
#[trace("TC-1670", "FR-645-AC-2", "FR-329-AC-3")]
#[test]
fn a_model_trace_with_an_empty_loop_is_refused() {
    let liveness = counter(
        document_with_ranged_parameter(),
        "",
        "always eventually holds(c.value = 3)",
    );
    let mut payload = found(&liveness, is_three);
    payload.lasso = None;
    let holds = counter(
        document_with_ranged_parameter(),
        "",
        "always holds(c.value <= 3)",
    );
    let trace = model_trace(&holds, &payload);
    assert_eq!(
        trace.request_pointer("empty-lasso-loop"),
        "selection.temporal.trace.model.loop"
    );
    let report = run(&holds, trace);
    let ClauseDisposition::TraceRefused(refusal) = &report.disposition else {
        panic!("{:?}", report.disposition);
    };
    assert_eq!(refusal.reason, "empty-lasso-loop");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
}

/// FR-203, FR-255: an expansion that stops at a caller-raisable bound is an
/// `incomplete` run naming the bound, its value and the setting that raises
/// it, never a verdict and never `pending` (exit 0).
#[trace("TC-1663", "FR-641-AC-4")]
#[test]
fn a_model_trace_whose_expansion_stops_is_incomplete_naming_the_limit() {
    let (fixture, payload) = stuttering();
    // One work unit admits the initial state and stops the first expansion.
    let accounting = ScalarLimits {
        work_units: STOPS_AT,
        ..UNLIMITED
    };
    let report = run_under(&fixture, model_trace(&fixture, &payload), accounting);
    let ClauseDisposition::Temporal(TemporalVerdict::ExpansionLimitReached(reached)) =
        &report.disposition
    else {
        panic!("{:?}", report.disposition);
    };
    assert_eq!(
        reached.limit,
        ModelCheckLimit::EvaluationMeter {
            counter: LimitKind::WorkUnits
        }
    );
    assert_eq!(reached.value, STOPS_AT);
    assert_eq!(reached.limit.setting(), "accounting.work_units");
    assert_eq!(
        report.disposition.category(),
        filament_semantic_value::catalog::Category::Incomplete
    );
    assert_ne!(report.disposition.exit_code(), 0);
    // The record reads back as the limit it wrote.
    let bytes = fsl_replay::spine::write_clause_run_result(&report).expect("the report writes");
    let written: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(
        written["disposition"]["temporal"]["limit"], "work_units",
        "{written}"
    );
    assert_eq!(
        written["disposition"]["temporal"]["setting"], "accounting.work_units",
        "{written}"
    );
    fsl_replay::spine::read_native_run_result(&bytes).expect("the written record reads back");
}

/// FR-330 "Admit": a post-state no successor of the step has refuses with
/// `invalid_runtime_input`/`invalid-value`, never a verdict.
#[trace("TC-1663", "FR-641-AC-4")]
#[test]
fn a_model_trace_with_an_unreachable_post_state_is_refused() {
    let (fixture, payload) = violating();
    let TraceInput::Model {
        initial,
        universes,
        mut prefix,
        mut repeat,
        terminal_stutter,
    } = model_trace(&fixture, &payload)
    else {
        unreachable!("model_trace builds a model trace");
    };
    // A post-state key no successor of the step has.
    if let Some(last) = repeat.last_mut().or(prefix.last_mut()) {
        last.post_state = StateKey::from_canonical_bytes(b"[]".to_vec());
    }
    let report = run(
        &fixture,
        TraceInput::Model {
            initial,
            universes,
            prefix,
            repeat,
            terminal_stutter,
        },
    );
    assert!(
        !matches!(report.disposition, ClauseDisposition::Temporal(_)),
        "{:?}",
        report.disposition
    );
}
