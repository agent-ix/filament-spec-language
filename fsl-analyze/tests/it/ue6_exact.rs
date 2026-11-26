// SPDX-License-Identifier: AGPL-3.0-or-later
//! UE-6, exact probabilistic route: `inc`'s precondition has no value at the
//! reachable state `v = 1`. ADR-018 UE-6: no claim over the subject is
//! proved or accepted, even one decided before that state.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_foundation::RequestIndex;
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
fn ue6_exact_scheduler_reach_at_start() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under every scheduler on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[test]
fn ue6_exact_workload_reach_at_start() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[test]
fn ue6_exact_workload_reach_at_the_state() {
    exact(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 1) ] }\n",
        ),
        "P",
    );
}

/// The atom `undefined_remainder(c.v) >= 0` has no value at position 0 (`v = 0`),
/// which `always[0,0]` reads.
#[test]
fn ue6_exact_atom_undefined_at_position_zero() {
    exact(
        &world(
            "probabilistic P using ev on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ always[0,0] holds(undefined_remainder(c.v) >= 0) ] }\n",
        ),
        "P",
    );
}
