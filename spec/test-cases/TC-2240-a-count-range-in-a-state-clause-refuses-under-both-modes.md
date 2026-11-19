---
id: TC-2240
title: "A count range in a state clause refuses under both modes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-766
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: references
---
# TC-2240: A count range in a state clause refuses under both modes

## Description

Verify that an unproved `count<N>` result range in a declared `invariant`
refuses at check under `CheckMode::Kernel` as under `CheckMode::Linked`, and
that the same count in a function body is left to S6a under `Kernel`.
Scope: FR-766-AC-8.

## Test Procedure

1. Over the ConfigVersion model, check the `invariant I` of FR-766-AC-8 through
   `PackageDeclarations::check` under each mode.
2. Check the function `f` of FR-766-AC-8 under each mode.

Test: `fsl-semantics/tests/it/check_mode.rs`
`a_count_range_in_a_state_clause_refuses_under_both_modes`.

## Expected Results

- Step 1: one `undefined_expression`/`unproved-range` refusal at the `count`
  in each mode; no checked graph.
- Step 2: `unproved-range` under `Linked`; checks under `Kernel`.
