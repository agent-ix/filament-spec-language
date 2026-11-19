// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-190: seeded model traces are reproducible, replayable and emit exact
//! domain boundaries with adjacent refusal partitions.

use super::*;

/// The same seed produces the same trace and boundary partitions; adjacent
/// refusal partitions generated from the model are rejected by model intake.
#[trace(
    "FR-752-AC-1",
    "FSpec-FR-190-AC-1",
    "FSpec-FR-190-AC-3",
    "FSpec-FR-190-AC-4"
)]
#[test]
fn seeded_model_traces_emit_replayable_boundary_partitions_deterministically() {
    let clauses = "pre CanInc using v on M::Counter::increment { self.value < 5 }\n\
                   post Inc using v on M::Counter::increment { self.value - pre(self.value) = 1 }";
    let mut world = world_with_level_bound(vec![increment()], clauses, 5);
    let initial = world.start("s0", &[Obj("c1", 0, 0, None)]);
    let generated_partitions;
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
        generated_partitions = generated.boundary_partitions.clone();
        assert_eq!(generated.trace.steps.len(), 5);
        assert_eq!(generated.effects.len(), 5);
        let counter_domain = IntegerInterval::new(0_i64.into(), 5_i64.into())
            .expect("the fixture has a nonempty counter domain");
        let partitions: Vec<_> = generated
            .boundary_partitions
            .iter()
            .filter(|partition| partition.domain == counter_domain)
            .collect();
        assert_eq!(
            partitions
                .iter()
                .map(|partition| partition.kind)
                .collect::<Vec<_>>(),
            [
                ModelBoundaryKind::LowerRefusal,
                ModelBoundaryKind::LowerEndpoint,
                ModelBoundaryKind::UpperEndpoint,
                ModelBoundaryKind::UpperRefusal,
            ],
            "generated output must include both endpoints and adjacent refusal partitions",
        );
        assert_eq!(
            partitions
                .iter()
                .map(|partition| partition.value.to_string())
                .collect::<Vec<_>>(),
            ["-1", "0", "5", "6"],
        );
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

    let counter_domain = IntegerInterval::new(0_i64.into(), 5_i64.into())
        .expect("the fixture has a nonempty counter domain");
    for partition in generated_partitions
        .into_iter()
        .filter(|partition| partition.domain == counter_domain)
        .filter(|partition| {
            matches!(
                partition.kind,
                ModelBoundaryKind::LowerRefusal | ModelBoundaryKind::UpperRefusal
            )
        })
    {
        let outside = partition
            .value
            .to_string()
            .parse::<i64>()
            .expect("the fixture's adjacent boundary fits in i64");
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
