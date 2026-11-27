// SPDX-License-Identifier: AGPL-3.0-or-later
//! UE-6, exact probabilistic route: `inc`'s precondition has no value at the
//! reachable state `v = 1`. ADR-018 UE-6: no claim over the subject is
//! proved or accepted, even one decided before that state.

use filament_exact::Cancel;
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_foundation::RequestIndex;
use fsl_replay::{ModelCheckLimits, TerminalValue, UndefinedLocus};
use ix_trace_rs::trace;

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

fn record(world: &World, name: &str) -> fsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::fsl().expect("engines");
    let item = exact_probabilistic_item(
        RequestIndex::new(0),
        world.request(name, ModelCheckLimits::default()),
    );
    analyze_items(&engines, vec![item], &Cancel::new())
        .into_iter()
        .next()
        .expect("a record")
}

/// The claim refutes with the contract conjunction `CanInc` that has no
/// value at the reachable state (ADR-018 UE-6): never proved or witnessed.
fn refutes_at(world: &World, name: &str, locus: impl Fn(&UndefinedLocus) -> bool) {
    let record = record(world, name);
    let value = record.terminal().map(|terminal| terminal.value().clone());
    assert_eq!(value, Some(TerminalValue::Refuted), "UE-6: refuted");
    let spelled = record
        .probabilistic_counterexample
        .as_ref()
        .unwrap_or_else(|| panic!("a counterexample: {:?}", record.settlement));
    let undefined = spelled
        .found
        .undefined()
        .expect("the reproduced witness is an undefined path");
    assert!(
        locus(&undefined.evaluation.site.locus),
        "UE-6: the evaluation names the clause: {:?}",
        undefined.evaluation.site.locus
    );
}

fn refutes(world: &World, name: &str) {
    refutes_at(world, name, |locus| {
        matches!(locus, UndefinedLocus::Contract(clause) if clause == "CanInc")
    });
}

#[trace("FR-196-AC-20")]
#[test]
fn ue6_exact_scheduler_reach_at_start() {
    refutes(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under every scheduler on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[trace("FR-196-AC-20")]
#[test]
fn ue6_exact_workload_reach_at_start() {
    refutes(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 0) ] }\n",
        ),
        "P",
    );
}

#[trace("FR-196-AC-20")]
#[test]
fn ue6_exact_workload_reach_at_the_state() {
    refutes(
        &world(
            "probabilistic P using inf on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ eventually holds(c.v = 1) ] }\n",
        ),
        "P",
    );
}

/// The atom `undefined_remainder(c.v) >= 0` has no value at position 0 (`v = 0`),
/// which `always[0,0]` reads.
#[trace("FR-196-AC-20")]
#[test]
fn ue6_exact_atom_undefined_at_position_zero() {
    refutes_at(
        &world(
            "probabilistic P using ev on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ always[0,0] holds(undefined_remainder(c.v) >= 0) ] }\n",
        ),
        "P",
        |locus| matches!(locus, UndefinedLocus::Atom(_)),
    );
}

/// A claim whose form reads only position 0 never reads the guard that is
/// undefined at position 1 (ADR-018 UE-1, ADR-028 XV-8): it stays proved.
#[trace("FR-196-AC-20")]
#[test]
fn ue6_exact_form_reading_only_position_zero_stays_proved() {
    let record = record(
        &world(
            "probabilistic P using ev on M over (c: M::Cell) under Even on origin \
             { probability >= 0.99 [ always[0,0] holds(c.v >= 0) ] }\n",
        ),
        "P",
    );
    let value = record.terminal().map(|terminal| terminal.value().clone());
    assert!(
        matches!(value, Some(TerminalValue::Proved { .. })),
        "{value:?}"
    );
}
