// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-186-AC-7: the admitted bounded timed Eventually form.

use super::*;

#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_with_closed_horizon_settles_at_ninety_nine_percent() {
    let claims = "\
        probabilistic Eventually using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    let record = settle(&world, "Eventually");
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("timed bounded claim has a terminal: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: fsl_replay::Certification::Certified,
    } = terminal.value()
    else {
        panic!("timed bounded claim settles with a checked proof: {record:#?}");
    };
    assert_eq!(value.value(), &ratio(99, 100));
}
