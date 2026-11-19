---
id: TC-3045
title: "A positionally selected clock constraint is written with literal slot terms"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-3045: A positionally selected clock constraint is written with literal slot terms

## Description

Verify that a guard or time invariant reading a positionally selected
element's clock is written with literal slot terms and that each slot keeps
its own presence obligation. Scope: FR-230-AC-69.

## Test Procedure

Over a timed model with `ts: Sequence<Timer>[0, 3]`, `Timer` holding a clock
`x` and `ys: Sequence<Clock>[0, 2]`, check and emit:

1. `pre G { size(self.ts) >= 1 and self.ts[0].x >= 1 ms }` on `attemptUpdate`.
2. `pre G2 { size(self.ts) >= 1 and size(self.ts[0].ys) >= 2 and self.ts[0].ys[1] >= 1 ms }`.
3. `time invariant B when { size(self.ts) >= 1 } { self.ts[0].x <= 9 ms }`.
4. Step 1 without its size conjunct; step 2 with `size(self.ts) >= 1` as its
   only size conjunct; step 3 with `when { true }`.

## Expected Results

- Step 1: one `clock_guard`, one part, one `clock_constraint` on `ts`'s
  element clock template `x` with slot term `0` and no binder.
- Step 2: slot terms `0` then `1`.
- Step 3: one `time_invariant`, its upper bound with slot term `0`, `binders`
  empty.
- Steps 1 to 3 leave no unwritten member and give no `TimedModelNotEmitted`.
- Step 4: `undefined_expression`/`unproved-presence` at `0`, then at `1`,
  then at `0`.

Test: `fsl-package/src/emit/tests/timed_clauses.rs`.
