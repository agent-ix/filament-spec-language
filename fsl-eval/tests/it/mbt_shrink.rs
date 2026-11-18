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
    let generated =
        fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler(), 0x2237, 0, 5)
            .expect("the finite model produces a failing trace");
    assert!(violates_small(&generated));
    assert_eq!(generated.trace.steps.len(), 5);

    let shrunk = shrink_model_trace(&system, &generated, violates_small)
        .expect("the failure-preserving prefix replays");

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
