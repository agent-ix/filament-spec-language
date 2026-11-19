---
id: TC-3061
title: "Check a conditional certificate"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: verifies
---
# TC-3061: Check a conditional certificate

## Description

`fsl-probability-check` accepts a `Conditional` certificate only after
rebuilding the first activating states and the reset process itself.

Scope: FR-201-AC-52, FR-201-AC-53.

## Test Procedure

1. Write FSpec FR-407-AC-18's model as an `on each (x: T) when (g)` claim.
   Emit and check the `EveryScheduler` certificate for
   `probability >= rational(1, 2)`; check it again with `reset`'s initial
   value `1/4`, and for `rational(3, 5)`.
2. Under the workload choosing both actions with `1/2` each, check the
   `Workload` certificate at `rational(3, 5)`, then with the denominator
   replaced by 1.
3. Check a `Vacuous` certificate over the variant where `g` is unreachable,
   under both scheduler clauses, then over the model itself.
4. Check a `Conditional` certificate without `conditional`, and a `Workload`
   one whose `numerator` is a `Lower` certificate.

Run the checker as a separate process, as FR-201-AC-51 does.

## Expected Results

- Step 1: accepted `AtLeast`; `InequalityFails` at the initial state;
  `ThresholdMissed`.
- Step 2: accepted; `InequalityFails`.
- Step 3: accepted twice; `InequalityFails` at the initial state.
- Step 4: `Malformed` naming `conditional`; `Malformed` naming `numerator`.
