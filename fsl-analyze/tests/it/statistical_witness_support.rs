// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared sampled-witness helper spans split integration test binaries"
)]
//! Shared sampled-witness replay helper for integration targets.

use filament_exact::Cancel;
use fsl_analyze::statistical::witness_reader;
use fsl_eval::simulation::model::ModelSystem;
use fsl_replay::statistical::{
    replay_sampled_witness, SampledWitness, SampledWitnessRefusal, SampledWitnessReplay,
    StatisticalProvenance, WitnessReplay,
};
use fsl_semantics::check::ProbScheduler;

use crate::prob_world::World;

fn sampler() -> fsl_foundation::selection::DefinitionRef {
    fsl_foundation::selection::DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
        .expect("a valid DefinitionRef")
}

pub(crate) fn replay(
    world: &World,
    claim: &str,
    provenance: &StatisticalProvenance,
    witness: &SampledWitness,
) -> Result<SampledWitnessReplay, SampledWitnessRefusal> {
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let cancel = Cancel::new();
    let checked = world.claim(claim);
    let ProbScheduler::Workload(name) = &checked.scheduler else {
        panic!("a workload claim");
    };
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload(name)
        .expect("the workload is declared");
    let mut reader = witness_reader(&system, &world.package, checked, &sampler(), &cancel)
        .expect("the claim is read");
    replay_sampled_witness(
        &WitnessReplay {
            system: &system,
            timed: None,
            workload,
            provenance,
            cancel: &cancel,
        },
        witness,
        &mut reader,
    )
}
