// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204-AC-21: bounded timed operators, lower bounds and nested monitors.

use super::*;

#[trace("FR-204-AC-21", "FR-186-AC-7")]
#[test]
fn timed_eventually_with_nonzero_lower_bound_settles_at_ninety_nine_percent() {
    let claims = "\
        probabilistic Eventually using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[1 ms, 4 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    let result = settle(&world, "Eventually");
    assert_eq!(proved_value(&result), ratio(99, 100), "{result:#?}");
}

#[trace("FR-204-AC-21", "FR-186-AC-7")]
#[test]
fn timed_until_with_nonzero_lower_bound_settles_at_ninety_nine_percent() {
    let claims = "\
        probabilistic Until using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ holds(not c.late) until[1 ms, 4 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    assert_eq!(proved_value(&settle(&world, "Until")), ratio(99, 100));
}

#[trace("FR-204-AC-21", "FR-186-AC-7")]
#[test]
fn timed_always_and_nested_punctual_operator_settle_at_literal_values() {
    let claims = "\
        probabilistic Always using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ always[0 ms, 4 ms] holds(c.late) ] }\n\
        probabilistic Nested using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] always[1 ms, 1 ms] holds(c.late) ] }\n";
    let world = retx(GUARD, true, TWO_DELAYS, claims);
    assert_eq!(proved_value(&settle(&world, "Always")), ratio(0, 1));
}

#[trace("FR-204-AC-21", "FR-186-AC-7")]
#[test]
fn timed_nested_eventually_always_retains_both_occurrences() {
    let claims = "\
        probabilistic Nested using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] always[1 ms, 1 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    assert_eq!(proved_value(&settle(&world, "Nested")), ratio(99, 100));
}
