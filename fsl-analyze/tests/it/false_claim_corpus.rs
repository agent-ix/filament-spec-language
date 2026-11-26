// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's false-claim corpus and oracle isolation checks.

use ix_trace_rs::trace;

/// The oracle stays isolated from producer and checker crates and is linked
/// only into tests. Runtime checks are added alongside each route's seed.
#[trace("FR-1051-AC-18")]
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
        assert!(
            allowed.contains(&name),
            "unexpected direct dependency {name}"
        );
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
    for forbidden in [
        "fsl-eval",
        "fsl-analyze",
        "fsl-replay",
        "fsl-hybrid-reference",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "reference dependency section names forbidden crate {forbidden}"
        );
    }
    let reference_source = include_str!("../../../fsl-definitional-reference/src/lib.rs");
    assert!(reference_source.contains("impl RewardEvaluator for ReferenceRewardEvaluator"));
    assert!(!reference_source.contains("S6aRewardEvaluator"));

    let analyze_manifest = include_str!("../../../fsl-analyze/Cargo.toml");
    let reference_entry = analyze_manifest
        .lines()
        .find(|line| {
            line.trim_start()
                .starts_with("fsl-definitional-reference =")
        })
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

/// FR-1051-AC-19: a disjunctive guard is refused before the reference can
/// return a truth value. The guard is read from the checked operation graph.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_disjunctive_operation_guards() {
    let mut model = crate::prob_world::coin("");
    model
        .body
        .push_str("\npre Disjunctive using v on M::Coin::drop { self.v = 0 or self.v = 1 }\n");
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the model with a disjunctive guard passes front-end checking");

    let refusal = fsl_definitional_reference::validate_conjunctive_guards(world.package.graph())
        .expect_err("the tiny reference refuses disjunctive guards");
    assert_eq!(refusal.construct, "disjunctive operation guard");
}

/// FR-1051-AC-19: some-scheduler quantiles are refused without producing a
/// truth value; the reference does not yet enumerate quantile distributions.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_some_scheduler_quantiles() {
    let model = crate::prob_world::tossed_coin(
        "probabilistic Q using ev on M over (c: M::Coin) under Even on origin \
         { quantile 0.5 of (steps from holds(c.v = 0) until holds(c.v = 1) within 2) <= 1 }",
    );
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the checked package admits the quantile claim");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("some-scheduler quantile is outside the reference fragment");
    assert_eq!(refusal.construct, "quantile claim over some scheduler");
}

/// FR-1051-AC-19: region constraints in the admitted reference fragment use
/// integer constants, even though the exact clock valuation is rational.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_non_integer_clock_constants() {
    const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let document = crate::timed_world::document(&["x"], &[("send", &["phase"])]);
    let body = format!(
        "{}pre Half using v on M::Call::send {{ self.x > rational(1, 2) ms }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(&document, PROFILES, &body);

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("a half-unit clock boundary is outside the region fragment");
    assert_eq!(
        refusal.construct,
        "clock compared with a non-integer constant"
    );
}

/// FR-1051-AC-19: an unbounded native integer has infinitely many valuations.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_unbounded_integer_fields() {
    let mut model = crate::prob_world::coin("");
    model.fields[0].1 = "ix://filament/native/Integer".to_owned();
    let world = crate::prob_world::World::try_new_with_evaluator(
        &model,
        &fsl_definitional_reference::ReferenceRewardEvaluator,
    )
    .expect("the front end admits an unbounded integer field");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(world.package.graph())
        .expect_err("the reference does not enumerate an unbounded integer field");
    assert_eq!(refusal.construct, "unbounded integer field");
}

/// FR-1051-AC-19: hybrid dynamics in the reference fragment use literal
/// rational rates rather than state-dependent flow expressions.
#[trace("FR-1051-AC-19")]
#[test]
fn reference_refuses_state_dependent_hybrid_rates() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid H using p { mode A invariant { x >= 0 } flow { x' = x; } }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("false-claim-corpus", "hybrid-rate"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source is within parser limits");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("the source builds as a unit");
    let package = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "false-claim-corpus".to_owned(),
        unit,
        fsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the hybrid unit assembles");
    let graph = package
        .check(
            filament_semantic_value::checking::CheckMode::Linked,
            filament_semantic_value::checking::CheckingLimits::default(),
            &fsl_definitional_reference::ReferenceRewardEvaluator,
        )
        .expect("the state-dependent rate passes package checking");

    let refusal = fsl_definitional_reference::validate_tiny_fragment(&graph)
        .expect_err("the reference refuses a state-dependent hybrid rate");
    assert_eq!(refusal.construct, "hybrid mode with a non-constant rate");
}
