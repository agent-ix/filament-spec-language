---
id: TC-2298
title: "A control node lowers to the hull of its leaves' lowered types"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: verifies
---
# TC-2298: A control node lowers to the hull of its leaves' lowered types

## Description

Verify the lowered type of a control node whose leaf is an integer division
lowered as `Integer`, and the narrow over a binder bound to such a node.
Scope: FR-495-AC-23.

## Test Procedure

1. Under `CheckMode::Linked`, compile `ifi`, `ifb` and `letif` of FR-495-AC-23 and read each lowered control node's `result_type`, the `mod` application's `result_type`, every narrow and the `mod` record.
2. Compile `letif` with result `Int[0, 5]`.

## Expected Results

- Step 1: `ifi`'s `if` has `result_type` equal to the `mod` application's (`Integer`); `ifb`'s `if` has the `Int[0, 11]` node and one narrow over the application; `letif` has exactly one narrow, over the read of `x`, and the `mod` record has no direct narrow and bound `Int[0, 11]`.
- Step 2: `undefined_expression`/`unproved-range` at the read of `x`.

Test: `tests/it/control_node_lowered_type.rs`
`a_control_node_takes_the_hull_of_its_leaves_lowered_types`.
