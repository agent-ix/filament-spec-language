// SPDX-License-Identifier: AGPL-3.0-or-later
//! The exact mean of a fraction over a long window on a two-state model stays
//! inside the default work allowance: the independent product check expands
//! each model state once, however many (remaining, count) product states
//! stand over it.
use super::*;

const WINDOW: u64 = 10;

fn mean(window: u64) -> prob_world::Model {
    prob_world::health(
        "self.healthy = not fault",
        "true: 1, false: 9999",
        false,
        &format!(
            "probabilistic Up using ev on M over (n: M::Node) under every scheduler on origin \
             {{ mean of (fraction holds(n.healthy) over [0, {window}]) >= 0.9 }}\n"
        ),
    )
}

fn settle_under(world: &World, work_units: u64) -> fsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::fsl().unwrap();
    let mut request = world.request("Up", ModelCheckLimits::default());
    request.subject.simulation.meter.work_units = work_units;
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

#[trace("TC-638", "FR-203-AC-1")]
#[test]
fn a_ten_step_fraction_mean_on_two_states_proves_within_the_default_work_allowance() {
    let world = World::new(&mean(WINDOW));
    let record = settle_under(&world, 1_000_000);
    assert!(
        matches!(
            record.terminal().expect("a terminal record").value(),
            TerminalValue::Proved { .. }
        ),
        "{record:?}"
    );
    // One start position up, then ten steps that each stay up with 9999/10000.
    assert_eq!(
        record.exact_probabilistic.as_ref().unwrap().value,
        ratio(10_999, 11_000)
    );
}

#[trace("TC-638", "FR-203-AC-1")]
#[test]
fn the_ten_step_fraction_mean_needs_exactly_the_measured_work_units() {
    let world = World::new(&mean(WINDOW));
    let needed = 265_325;
    assert!(matches!(
        settle_under(&world, needed).terminal().unwrap().value(),
        TerminalValue::Proved { .. }
    ));
    let short = settle_under(&world, needed - 1);
    assert!(
        matches!(
            short.terminal().unwrap().value(),
            TerminalValue::Incomplete(IncompleteCause::LimitReached { .. })
        ),
        "{short:?}"
    );
}
