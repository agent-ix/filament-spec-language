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
that the same count in a function body is left to S6a under `Kernel`, and that
a bare collection literal as the count source refuses `ambiguous-literal` (AC-9).
Scope: FR-766-AC-8, FR-766-AC-9.

## Test Procedure

1. Over the ConfigVersion model, check the `invariant I` of FR-766-AC-8 through
   `PackageDeclarations::check` under each mode.
2. Check the function `f` of FR-766-AC-8 under each mode.
3. Check the invariant of FR-766-AC-9, whose `count` source is a bare literal, under each mode.

Test: `fsl-semantics/tests/it/check_mode.rs`
`a_count_range_in_a_state_clause_refuses_under_both_modes` and
`an_unannotated_literal_count_source_is_an_ambiguous_literal`.

## Expected Results

- Step 1: one `undefined_expression`/`unproved-range` refusal at the `count`
  in each mode; no checked graph.
- Step 2: `unproved-range` under `Linked`; checks under `Kernel`.
- Step 3: one `ill_typed`/`ambiguous-literal` refusal in each mode.
