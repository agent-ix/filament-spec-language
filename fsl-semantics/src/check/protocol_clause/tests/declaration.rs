// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-884-AC-4: protocol declaration names are scoped to one unit.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use ix_trace_rs::trace;

const FLOW: &str = "protocol Flow using pp over (input: M::Actor) on origin {\n\
    role R on M::Actor;\n\
    run sequence Main { }\n\
    finish End as (outcome: Boolean) { true };\n\
}\n";

/// FR-884-AC-4: a protocol name is scoped to its source unit, so both units
/// may independently declare `Flow` and check successfully.
#[trace("TC-2509", "FR-884-AC-4")]
#[test]
fn one_name_in_two_units_is_admitted() {
    for _ in 0..2 {
        let (_, declarations) = super::assemble(FLOW);
        declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("each unit owns its `Flow`: {refusals:?}"));
    }
}
