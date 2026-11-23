---
id: TC-2971
title: "A refinement image outside an abstract unit bound is InitialNotAbstract"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: verifies
---
# TC-2971: A refinement image outside an abstract unit bound is InitialNotAbstract

## Description

Verify that a well-typed mapping whose image violates an abstract unit bound
gives an empty completion set, settled as a refutation at position 0. Scope:
FR-630-AC-6.

## Test Procedure

1. Extend the Counter fixture with `elapsed: Quantity<ms>` on both sides, the abstract side under the unit bound `max 3`.
2. Check the refinement `Late` (`value = self.value; elapsed = self.elapsed + 2 ms;`, `inc` to `Spec::Counter::inc(self)`).
3. Analyze `Late` with no `abstract_subject` over the snapshot `a` at `value` 0, `elapsed` 2 ms.
4. Repeat over `elapsed` 1 ms.

Test: `fsl-analyze/tests/it/analyze_family_cli.rs`.

## Expected Results

- Step 2: the unit checks.
- Step 3: `refuted` with `InitialNotAbstract{initial: 0}` at position 0 and a one-state counterexample that replays; never `declined`, `invalid-input` or `mapping-undefined`.
- Step 4: `proved`.
