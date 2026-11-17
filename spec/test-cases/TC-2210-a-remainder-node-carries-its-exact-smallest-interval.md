---
id: TC-2210
title: "A remainder node carries its exact smallest interval"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
  - target: ix://agent-ix/filament-specification/TC-1660
    type: references
---
# TC-2210: A remainder node carries its exact smallest interval

## Description

Verify from source that `mod` and `rem` are typed by the exact smallest
interval of their remainders, not by the divisor bound `[0, m - 1]`,
including singleton dividends whose lower endpoint depends on the divisors of
the dividend, negative dividends, the truncating and floor laws and an
unbounded divisor under an exact nonzero guard. Scope: FR-495-AC-19.

## Test Procedure

1. Under `CheckMode::Linked` and no law header, check each `mod` case of
   FR-495-AC-19 into its stated interval, into the interval with the lower
   endpoint raised by one and into the interval with the upper endpoint
   lowered by one.
2. Repeat step 1 for the truncating `rem` case under a truncating law header
   and the floor `rem` case under a floor law header.
3. Check the two guarded functions over `b: Integer`.

Test: `fsl-semantics/tests/it/division_law_selection.rs`
`a_remainder_node_carries_its_exact_smallest_interval`, tagged
`#[trace("TC-2210", "FR-495-AC-19", "FSpec-TC-1660", "FSpec-FR-147-AC-14")]`.

## Expected Results

- Steps 1 and 2: each stated interval checks; each narrowed interval refuses
  `undefined_expression`/`unproved-range` at the operator, its only refusal.
- Step 3: the `Int[0, 9]` dividend checks into `Int[0, 9]`; the `Int[-1, 9]`
  dividend refuses `unproved-range` into `Int[0, 1000]`.
