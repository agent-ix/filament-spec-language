// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3534 probabilistic probes: `inc`'s precondition has no value at the
//! reachable state `v = 1`. ADR-018 UE-6: no claim over the subject is
//! proved or accepted, even one decided before that state.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_foundation::RequestIndex;
use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod};
use fsl_replay::ModelCheckLimits;

use crate::prob_world::{self, World};

fn world(claim: &str) -> World {
    let mut model = prob_world::undetermined(claim);
    model.body = format!(
        "function undefined_remainder using vc(x: Integer): Integer pure {{ 6 mod x }}\n\
         pre CanInc using vc on M::Cell::inc {{ undefined_remainder(1 - self.v) >= 0 }}\n\
         post Inc using vc on M::Cell::inc {{ self.v - pre(self.v) = 1 }}\n\
         workload Even on M {{ weight Cell::inc = 1; }}\n{claim}"
    );
    World::new_with_mode(&model, CheckMode::Kernel)
}

fn exact(world: &World, name: &str) {
    let engines = Engines::fsl().expect("engines");
    let item = exact_probabilistic_item(
        RequestIndex::new(0),
        world.request(name, ModelCheckLimits::default()),
    );
    let records = analyze_items(&engines, vec![item], &Cancel::new());
    let text = format!("{:?}", records[0].terminal().map(|terminal| terminal.value()));
    println!("OBSERVED: {text:.300}");
    assert!(
        !text.contains("Proved") && !text.contains("Witnessed"),
        "UE-6: not proved or witnessed: {text:.300}"
    );
}

#[test]
fn probe_exact_scheduler_reach_at_start() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under every scheduler on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[test]
fn probe_exact_workload_reach_at_start() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[test]
fn probe_exact_workload_bounded_before_the_state() {
    exact(
        &world(
            "probabilistic P using ev on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ always[0,0] holds(c.v >= 0) ] }\n",
        ),
        "P",
    );
}

#[test]
fn probe_exact_workload_reach_at_the_state() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 1) ] }\n",
        ),
        "P",
    );
}

fn statistical(horizon: u32) {
    let claim = format!(
        "probabilistic Safe using ev on M over (c: M::Cell) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         {{ probability >= 0.5 [ always[0,{horizon}] holds(c.v >= 0) ] }}"
    );
    let verdict = crate::statistical_model::decided(
        &world(&claim),
        "Safe",
        StatisticalMethod::Okamoto,
    );
    let text = format!("{:?} witnesses={}", verdict.decision, verdict.witnesses.len());
    println!("OBSERVED: {text:.300}");
    assert!(
        !text.starts_with("Accept") && !text.starts_with("Holds"),
        "UE-6: not accepted: {text:.300}"
    );
    let _ = StatisticalLimits::default();
}

#[test]
fn probe_statistical_decided_before_the_state() {
    statistical(0);
}

#[test]
fn probe_statistical_walk_through_the_state() {
    statistical(2);
}

#[test]
fn probe_statistical_walk_ends_at_the_state() {
    statistical(1);
}
