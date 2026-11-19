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
rebuilding the first activating states and the reset process itself, and
checks the reset process over proper schedulers only.

Scope: FR-201-AC-52, FR-201-AC-53, FR-201-AC-54, FR-201-AC-55.

## Test Procedure

1. Write FSpec FR-407-AC-18's model as an `on each (x: T) when (g)` claim.
   Emit and check the `Optimum` certificate for
   `probability >= rational(1, 2)` over every scheduler; check it again with
   `reset`'s initial value `1/4`, and for `rational(3, 5)`.
2. Under the workload choosing both actions with `1/2` each, check the
   `Ratio` certificate at `rational(3, 5)`, then with the denominator
   replaced by 1.
3. Check a `Vacuous` certificate over the variant where `g` is unreachable,
   under both scheduler clauses, then over the model itself.
4. Check a `Conditional` certificate without `conditional`, one with mode
   `"workload"`, a `Ratio` certificate under every scheduler, and a `Ratio`
   one whose `numerator` is a `Lower` certificate.
5. Add the action `stay` at `s0`. Check the `Optimum` certificate with
   `reset` `x(s0) = 1/2`, policy `b`, ranks 1 and 2; then `x(s0) = 0` with
   policy `stay`; then `x(s0) = 0` with policy `b`; then, for
   `<= rational(9, 10)`, `y(s0) = 1` with policy `a`.

6. On the `stay` model, for `<= rational(9, 10)`, check a maximum `reset`
   with `y(s0) = 1/2`, policy `b` and no row for `u`; then with `u` at
   `1/2`; then a minimum `reset` with a `Randomized` entry at `s0`.

Run the checker as a separate process, as FR-201-AC-51 does.

## Expected Results

- Step 1: accepted `AtLeast`; `InequalityFails` at the initial state with
  `part` `reset`; `ThresholdMissed`.
- Step 2: accepted; `InequalityFails` with `part` `denominator`.
- Step 3: accepted twice; `InequalityFails` at the initial state.
- Step 4: `Malformed` naming `conditional`, `mode`, `mode` and `numerator`.
- Step 5: accepted; `RankBroken` at `s0` with `part` `reset`;
  `InequalityFails` at `s0`; accepted, and the claim is refuted.
- Step 6: `Malformed` at `values`; `InequalityFails` at `s0` with `part`
  `reset`; `Malformed` at `policy`.
