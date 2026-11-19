// SPDX-License-Identifier: AGPL-3.0-or-later
//! Statistical interval coverage and replay over a model with a known exact
//! probability (FR-189-AC-16, FR-193).

use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::statistical::{check_statistical, StatisticalRequest, DEFAULT_MAX_WITNESSES};
use fsl_foundation::selection::DefinitionRef;
use fsl_replay::statistical::{
    StatisticalLimits, StatisticalMethod, StatisticalOutcome, StatisticalVerdict,
};
use ix_trace_rs::trace;

use crate::prob_world::{self, World};

const CLAIM: &str = "probabilistic CoinGone using ev on M over (c: M::Coin) under Even on origin \
    with alpha 0.05, beta 0.05, indifference 0.2 \
    { probability >= 0.5 [ eventually[1,1] holds(c.v = 1) ] }";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
        .expect("a valid sampler reference")
}

fn world(claim: &str) -> World {
    World::new(&prob_world::Model {
        package: "test/cpcoin",
        object: "Coin",
        population: "coins",
        key: "c",
        integers: vec![("Bit", 0, 1)],
        fields: vec![("v", "ix://test/cpcoin/Bit".to_owned())],
        operations: vec![prob_world::Operation {
            name: "flip",
            parameters: vec![("b", "ix://test/cpcoin/Bit".to_owned())],
            modifies: vec!["v"],
            deletes: false,
        }],
        body: format!(
            "bound B on M::coins [1]\n\
             post Flipped using v on M::Coin::flip {{ self.v = b }}\n\
             random Coin::flip(b) ~ {{ 0: 1, 1: 1 }};\n\
             workload Even on M {{ weight Coin::flip = 1; }}\n{claim}"
        ),
        initial: serde_json::json!({"v": {"integer": "0"}}),
    })
}

fn decide(world: &World, seed: u64) -> StatisticalVerdict {
    let outcome = check_statistical(
        StatisticalRequest {
            subject: world.subject(),
            claim: world.claim("CoinGone"),
            method: StatisticalMethod::ClopperPearson,
            seed,
            sampler: sampler(),
            limits: StatisticalLimits::default(),
            max_witnesses: DEFAULT_MAX_WITNESSES,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    )
    .expect("the sampled claim is accepted by EN-4");
    match outcome {
        StatisticalOutcome::Completed(verdict) => *verdict,
        other => panic!("expected a completed statistical run, got {other:?}"),
    }
}

/// Over 1,024 deterministic seeds, the exact interval contains the model's
/// known stochastic probability `1/2` at least 90% of the time.
#[trace("TC-624")]
#[test]
fn clopper_pearson_coverage() {
    let world = world(CLAIM);
    let true_probability = ratio(1, 2);
    assert_eq!(decide(&world, 7), decide(&world, 7));
    let mut covered = 0_u64;

    for seed in 0..1_024 {
        let verdict = decide(&world, seed);
        assert_eq!(verdict.basis.method, StatisticalMethod::ClopperPearson);
        assert_eq!(verdict.basis.samples, 38);
        let (lower, upper) = verdict.tests[0]
            .interval
            .as_ref()
            .expect("Clopper–Pearson reports an interval");
        if lower <= &true_probability && &true_probability <= upper {
            covered += 1;
        }
    }

    assert!(
        covered * 100 >= 90 * 1_024,
        "coverage was {covered}/1024 for the true probability 1/2"
    );
}

/// FR-193-AC-2: a Clopper–Pearson witness replays against the same model and
/// provenance, reproducing the sampled path.
#[trace("TC-628", "FR-193-AC-2")]
#[test]
fn clopper_pearson_witness_replays() {
    let world = world(CLAIM);
    let verdict = decide(&world, 7);
    assert_eq!(verdict.basis.method, StatisticalMethod::ClopperPearson);
    assert!(!verdict.witnesses.is_empty(), "the sample has a witness");
    for witness in &verdict.witnesses {
        assert_eq!(
            crate::statistical_witness_replay::replay(
                &world,
                "CoinGone",
                &verdict.provenance,
                witness,
            ),
            Ok(fsl_replay::statistical::SampledWitnessReplay::Reproduced)
        );
    }
}
