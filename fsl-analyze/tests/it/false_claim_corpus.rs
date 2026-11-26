// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's false-claim corpus and oracle isolation checks.

use ix_trace_rs::trace;

/// The oracle stays isolated from producer and checker crates and is linked
/// only into tests. Runtime checks are added alongside each route's seed.
#[trace("FR-1051-AC-13")]
#[test]
fn reference_crate_has_only_the_declared_semantic_dependencies() {
    let manifest = include_str!("../../../fsl-definitional-reference/Cargo.toml");
    let dependencies = manifest
        .split_once("[dependencies]")
        .and_then(|(_, section)| section.split_once("[lints.rust]").map(|(deps, _)| deps))
        .expect("reference manifest has a dependency section");
    let allowed = [
        "fsl-foundation",
        "fsl-semantics",
        "fsl-package",
        "filament-exact",
        "filament-checked",
        "filament-contract-model",
        "filament-semantic-value",
        "ix-trace-rs",
    ];
    for line in dependencies.lines().filter(|line| line.contains('=')) {
        let name = line
            .split_once('=')
            .map(|(name, _)| name.trim())
            .expect("dependency rows have a name");
        assert!(allowed.contains(&name), "unexpected direct dependency {name}");
    }
    for required in [
        "fsl-foundation =",
        "fsl-semantics =",
        "fsl-package =",
        "filament-exact =",
        "filament-checked =",
        "filament-contract-model =",
        "filament-semantic-value =",
    ] {
        assert!(
            dependencies.contains(required),
            "reference dependency section omits required crate {required}"
        );
    }
    for forbidden in ["fsl-eval", "fsl-analyze", "fsl-replay", "fsl-hybrid-reference"] {
        assert!(
            !dependencies.contains(forbidden),
            "reference dependency section names forbidden crate {forbidden}"
        );
    }
    let reference_source =
        include_str!("../../../fsl-definitional-reference/src/lib.rs");
    assert!(reference_source.contains("impl RewardEvaluator for ReferenceRewardEvaluator"));
    assert!(!reference_source.contains("S6aRewardEvaluator"));

    let analyze_manifest = include_str!("../../../fsl-analyze/Cargo.toml");
    let reference_entry = analyze_manifest
        .lines()
        .find(|line| line.trim_start().starts_with("fsl-definitional-reference ="))
        .expect("analyze manifest registers the oracle");
    let dev_dependencies = analyze_manifest
        .split_once("[dev-dependencies]")
        .map(|(_, section)| section)
        .expect("analyze manifest has dev dependencies");
    assert!(
        dev_dependencies.contains(reference_entry),
        "reference is a dev dependency only"
    );
}

/// FR-1051-AC-14: a disjunctive guard is refused before the reference can
/// return a truth value. The guard is read from the checked operation graph.
#[trace("FR-1051-AC-14")]
#[test]
fn reference_refuses_disjunctive_operation_guards() {
    let mut model = crate::prob_world::coin("");
    model.body.push_str(
        "\npre Disjunctive using v on M::Coin::drop { self.v = 0 or self.v = 1 }\n",
    );
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the model with a disjunctive guard passes front-end checking");

    let refusal = fsl_definitional_reference::validate_conjunctive_guards(
        world.package.graph(),
    )
    .expect_err("the tiny reference refuses disjunctive guards");
    assert_eq!(refusal.construct, "disjunctive operation guard");
}
