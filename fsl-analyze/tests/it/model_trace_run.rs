// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-641-AC-4 (TC-1663), FR-330-AC-16 (TC-840): a model-arm behaviour reaches
//! FR-330 as `TraceInput::Model` and reports the outcome of the lasso
//! evaluator over the states its steps reconstruct. The behaviours are the
//! ones EN-1 finds over the `Counter` subject, so the steps and post-state
//! keys are the model's own.

use fsl_eval::simulation::StateKey;
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    run_clause, ClauseDisposition, ClauseRunReport, ClauseRunRequest, ClauseRunSelection,
    ClauseRunSource, DependencyInput, OverObject, SpineLimits, TemporalVerdict, TraceInput,
};
use fsl_replay::{CounterexampleKind, TemporalCounterexample};
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::trace;

use crate::counter::{document_with_ranged_parameter, PACKAGE, UNLIMITED};
use crate::lasso_replay::found;
use crate::liveness::{counter, Fixture};

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
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
        accounting: UNLIMITED,
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

/// FR-641-AC-4, FR-330-AC-16: over the model lasso EN-1 found, `always
/// holds(c.value <= 1)` is violated at position 0 and its decisive atom is
/// read at a reconstructed state beyond it.
#[trace("TC-1663", "FR-641-AC-4", "TC-840", "FR-330-AC-16")]
#[test]
fn a_model_trace_reports_violated_with_a_decisive_position() {
    let (fixture, payload) = violating();
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
    assert!(decisive.position.is_some_and(|position| position >= 1));
}

/// FR-641-AC-4: the same lasso answers a clause that holds over every state it
/// visits, and a finite model prefix is a prefix of an unknown continuation.
#[trace("TC-1663", "FR-641-AC-4")]
#[test]
fn a_model_trace_holds_or_stays_pending() {
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

    let mut prefix = payload.clone();
    prefix.lasso = None;
    let report = run(&holds, model_trace(&holds, &prefix));
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Temporal(TemporalVerdict::Pending)
        ),
        "{:?}",
        report.disposition
    );
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
