---
id: TC-2391
title: "A reset below two collections holds a slot per level"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-2391: A reset below two collections holds a slot per level

## Description

A reset below two collections holds a slot per level.

Scope: FR-230-AC-67.

## Test Procedure

Emit `self.ts[0].ys[1] = 0 ms` over `ts` of `Timer` holding `ys`, with the precondition `size(self.ts) >= 1 and size(self.ts[0].ys) >= 2`, then with no precondition, then with `size(self.ts) >= 1` alone. Test: `fsl-package/src/emit/tests/timed_clauses.rs` `a_reset_below_two_collections_holds_a_slot_per_level`.

Tag the tests `#[trace("TC-2391", "FR-230-AC-67")]`.

## Expected Results

- One `clock_reset` with path field, slot, field, slot and slot terms `0` then `1`.
- With no precondition: `undefined_expression`/`unproved-presence` at `0`; with `size(self.ts) >= 1` alone: the same at `1`.
