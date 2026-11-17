// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-138-AC-9 (TC-543): refinement history rows admit enum, decimal, and
//! option-of-enum values.

use filament_exact::Value;
use ix_trace_rs::trace;

use super::refinement_declarations::{
    cas_refines_counter, checked_counter, only_refinement, with_refinement_rows,
};

/// FR-138-AC-9: finite history literals are checked as values of their
/// declared enum or decimal types and recorded with their checked updates.
#[trace("TC-543", "FR-138-AC-9")]
#[test]
fn decimal_enum_and_option_enum_history_rows_check() {
    let rows = "\n\
        history Impl::Counter decimal_value : Decimal[0, 100; 0, 2; nearest-even] = decimal(0, 0) {\n\
            on Impl::Counter::commitA: decimal(1, 0);\n\
        }\n\
        history Impl::Counter enum_value : Phase = Phase::idle {\n\
            on Impl::Counter::commitA: Phase::busy;\n\
        }\n\
        history Impl::Counter optional_phase : Option<Phase> = none {\n\
            on Impl::Counter::commitA: some(Phase::busy);\n\
        }\n";
    let source = format!(
        "enum Phase {{ idle, busy }}\n{}",
        with_refinement_rows(cas_refines_counter(), rows)
    );
    let graph = checked_counter(&source);
    let histories = only_refinement(&graph).history();
    assert_eq!(histories.len(), 3);
    assert!(matches!(&histories[0].initial, Value::Decimal(_)));
    assert!(matches!(&histories[1].initial, Value::Enum(_)));
    assert!(matches!(&histories[2].initial, Value::Option(_)));
    assert!(histories.iter().all(|row| row.updates.len() == 1));
}
