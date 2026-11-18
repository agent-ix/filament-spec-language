// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-190: seeded model traces are reproducible, replayable and cover exact
//! domain boundaries while the evaluator refuses adjacent out-of-domain values.

use super::*;

/// The same seed produces the same evaluator-accepted trace, reaches both
/// inclusive endpoints, and adjacent out-of-domain initial values are refused.
#[trace(
    "FR-752-AC-1",
    "FSpec-FR-190-AC-1",
    "FSpec-FR-190-AC-3",
    "FSpec-FR-190-AC-4"
)]
#[test]
fn seeded_model_traces_replay_deterministically_and_reach_domain_endpoints() {
    let clauses = "pre CanInc using v on M::Counter::increment { self.value < 5 }\n\
                   post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";
    let mut world = world_with_level_bound(vec![increment()], clauses, 5);
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    {
        let system = system(&world, vec![initial], &["c1", "c2"], UNLIMITED);
        let sampler = sampler();

        let generated =
            fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler, 42, 0, 5)
                .expect("the finite model produces a trace");
        let repeated =
            fsl_eval::simulation::model::sample_model(&system, 10_000, &sampler, 42, 0, 5)
                .expect("the same request produces a trace");

        assert_eq!(generated, repeated, "seeded output must be deterministic");
        assert_eq!(generated.trace.steps.len(), 5);
        assert_eq!(generated.effects.len(), 5);
        assert_eq!(
            generated
                .effects
                .first()
                .and_then(|effect| effect.changed.first())
                .and_then(|change| change.pre.as_deref()),
            Some(r#"{"type":"integer","value":"0"}"#),
            "the first generated step starts at the lower endpoint",
        );
        assert_eq!(
            generated
                .effects
                .last()
                .and_then(|effect| effect.changed.first())
                .and_then(|change| change.post.as_deref()),
            Some(r#"{"type":"integer","value":"5"}"#),
            "the generated trace reaches the upper endpoint",
        );
        assert!(fsl_eval::simulation::replay(
            &system,
            &generated.trace,
            fsl_foundation::IdentityLimits::default(),
        )
        .is_ok());
    }

    for outside in [-1, 6] {
        let initial = world.start(
            &format!("outside-{outside}"),
            &[Obj("c1", outside, 0, None)],
        );
        let failure = fsl_eval::simulation::model::ModelSystem::new(request(
            &world,
            vec![initial],
            universe(&["c1", "c2"]),
            UNLIMITED,
            1_000_000,
        ))
        .err();
        assert_eq!(
            refusal_of(failure),
            (
                fsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                "invalid-value"
            ),
            "adjacent outside value {outside} must be refused",
        );
    }
}
