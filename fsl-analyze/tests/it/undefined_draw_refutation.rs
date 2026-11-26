// SPDX-License-Identifier: AGPL-3.0-or-later
//! Undefined probabilistic evidence through a multi-post-state draw.

use crate::prob_world::{self, World};
use filament_exact::Cancel;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_foundation::RequestIndex;
use fsl_replay::{ModelCheckLimits, TerminalValue};
use ix_trace_rs::trace;
use serde_json::json;

/// FR-196-AC-12: an undefined path through a draw with multiple post-states
/// selects the post-state it actually took, so EN-5 can refute the item.
#[trace("TC-641", "FR-196-AC-12")]
#[test]
fn an_undefined_draw_with_multiple_post_states_refutes_the_item() {
    let mut world = World::new(&prob_world::coin(
        "enum Color { red, blue }\n\
         object Lamp { lit: Boolean; population lamps; \
         operation paint(c: Color, coin: Reference<M::Coin>) modifies self.lit deletes coin; }\n\
         pre Dark using v on Lamp::paint { not self.lit }\n\
         post Lit using v on Lamp::paint { self.lit }\n\
         pre Never using v on M::Coin::drop { false }\n\
         random Lamp::paint(c) ~ { Color::blue: 1 };\n\
         probabilistic Seen using ev on M over (c: M::Coin) under every scheduler on origin\n\
         { probability >= 0.5 [ always[0,1] holds(c.v = 0) ] }\n",
    ));
    world.add_source_object("lamps", "Lamp", "l", &json!({"lit": {"boolean": false}}));

    let record = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Seen", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);

    assert_eq!(
        record.terminal().map(|terminal| terminal.value()),
        Some(&TerminalValue::Refuted),
        "the scheduler's selected post-state that deletes the coin makes the read undefined; settlement: {:?}",
        record.settlement
    );
    assert!(record.probabilistic_counterexample.is_some());
}
