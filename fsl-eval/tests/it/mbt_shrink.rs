// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-190: model-trace shrinking retains a replayed failure and minimizes
//! the trace length.

use super::*;
use fsl_eval::simulation::model::{shrink_model_trace, ModelFinding};

fn violates_small(trace: &fsl_eval::simulation::model::ModelTrace) -> bool {
    trace.trace.findings.iter().any(|finding| {
        finding.findings.iter().any(|finding| {
            matches!(finding, ModelFinding::InvariantViolated { clause, .. } if clause == "Small")
        })
    })
}

fn first_small_violation_depth(trace: &fsl_eval::simulation::model::ModelTrace) -> Option<usize> {
    trace.trace.findings.iter().find_map(|finding| {
        finding
            .findings
            .iter()
            .any(|finding| {
                matches!(finding, ModelFinding::InvariantViolated { clause, .. } if clause == "Small")
            })
            .then_some(finding.depth)
    })
}

/// A seeded long failure shrinks to four increments, and the retained prefix
/// still replays the same native invariant violation.
#[trace("FR-752-AC-2", "FSpec-FR-190-AC-2", "FSpec-FR-190-AC-5")]
#[test]
fn a_seeded_counterexample_shrinks_to_the_shortest_failing_prefix() {
    const SMALL_AT_FOUR: &str =
        "invariant Small using v on M::Counter at current { self.value < 4 }";
    const INCREMENT_UNTIL_FIVE: &str =
        "pre CanInc using v on M::Counter::increment { self.value < 5 }\n\
         post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";
    let mut world = world_with_level_bound(
        vec![increment()],
        &clauses(&[SMALL_AT_FOUR, INCREMENT_UNTIL_FIVE]),
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1", "c2"], UNLIMITED);
    for seed in [1, 2, 3, 42] {
        let generated =
            fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler(), seed, 0, 5)
                .expect("the finite model produces a failing trace");
        assert!(violates_small(&generated));
        assert_eq!(generated.trace.steps.len(), 5);

        let shrunk = shrink_model_trace(&system, &generated, violates_small)
            .expect("the failure-preserving trace replays");

        assert_eq!(shrunk.trace.steps.len(), 4, "four increments reach value 4");
        assert!(
            violates_small(&shrunk),
            "shrinking preserves the same claim failure"
        );
        assert!(fsl_eval::simulation::replay(
            &system,
            &shrunk.trace,
            fsl_foundation::IdentityLimits::default(),
        )
        .is_ok());
    }
}

/// An interior no-op is deleted while the same invariant failure still
/// reproduces, demonstrating reduction beyond prefix truncation.
#[trace("FR-752-AC-2", "FSpec-FR-190-AC-2", "FSpec-FR-190-AC-5")]
#[test]
fn shrinking_removes_an_interior_invocation_and_replays_the_failure() {
    const SMALL_AT_FOUR: &str =
        "invariant Small using v on M::Counter at current { self.value < 4 }";
    const INCREMENT_UNTIL_FIVE: &str =
        "pre CanInc using v on M::Counter::increment { self.value < 5 }\n\
         post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";
    let mut world = world_with_level_bound(
        vec![
            increment(),
            operation("noop", &[], None, frame(&[], false, false)),
        ],
        &clauses(&[SMALL_AT_FOUR, INCREMENT_UNTIL_FIVE]),
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1", "c2"], UNLIMITED);
    let noop = operation_text("noop");
    let generated = (0..1024)
        .find_map(|seed| {
            let trace =
                fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler(), seed, 0, 8)
                    .expect("the finite model produces a trace");
            let violation_depth = first_small_violation_depth(&trace)?;
            let has_interior_noop = trace.trace.steps.iter().enumerate().any(|(index, step)| {
                index > 0 && index < violation_depth && step.transition.operation() == noop
            });
            has_interior_noop.then_some(trace)
        })
        .expect("the fixed seed range includes an interior no-op before the failure");

    let shrunk = shrink_model_trace(&system, &generated, violates_small)
        .expect("each accepted reduction replays natively");

    assert_eq!(shrunk.trace.steps.len(), 4, "four increments reach value 4");
    assert!(shrunk
        .trace
        .steps
        .iter()
        .all(|step| step.transition.operation() != noop));
    assert!(violates_small(&shrunk));
}

/// A parameter reduction follows the canonical typed invocation order and
/// keeps the same invariant failure after native replay.
#[trace("FR-752-AC-2", "FSpec-FR-190-AC-2", "FSpec-FR-190-AC-5")]
#[test]
fn shrinking_decreases_an_operation_argument_while_preserving_failure() {
    const SMALL: &str = "invariant Small using v on M::Counter at current { self.value < 2 }";
    const SET_TO: &str = "post SetsIt using v on M::Counter::setTo { self.value - n = 0 }";
    let level = ident("Level");
    let mut world = world_with_level_bound(
        vec![operation(
            "setTo",
            &[("n", &level)],
            None,
            frame(&["value"], false, false),
        )],
        &clauses(&[SMALL, SET_TO]),
        5,
    );
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let system = system(&world, vec![initial], &["c1", "c2"], UNLIMITED);
    let generated = (0..128)
        .find_map(|seed| {
            let trace =
                fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler(), seed, 0, 1)
                    .expect("the finite model produces a trace");
            let argument = trace.trace.steps.first()?.transition.arguments().first()?;
            (argument.contains(r#""value":"3""#) && violates_small(&trace)).then_some(trace)
        })
        .expect("the fixed seed range includes the value-3 counterexample");

    let shrunk = shrink_model_trace(&system, &generated, violates_small)
        .expect("each argument reduction replays natively");

    assert_eq!(shrunk.trace.steps.len(), 1);
    assert!(
        shrunk.trace.steps[0].transition.arguments()[0].contains(r#""value":"2""#),
        "value 2 is the immediate lower failing argument"
    );
    assert!(violates_small(&shrunk));
}
