---
id: TC-1784
title: "Replay recompiles under the recorded check mode"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: verifies
---
# TC-1784: Replay recompiles under the recorded check mode

## Description

Verify that a replay recompiles a package under the check mode recorded with
it, so a `Kernel` claim is reproduced under `Kernel` and a `Linked` claim is
never re-checked under `Kernel`. Scope: FR-098-AC-34.

## Test Procedure

1. Compile FR-098-AC-34's `r` under `CheckMode::Kernel`; read the recorded
   mode; replay the request at its `10 mod x` record's occurrence with
   `x = 0`.
2. Check the same source under `CheckMode::Linked`.
3. Compile FR-098-AC-34's `g` under `CheckMode::Linked`; read the recorded
   mode; replay a request at its `x mod y` record's occurrence with
   `x = 3`, `y = 2`.

## Expected Results

- Step 1: the package records `Kernel`; the replay recompiles under `Kernel`
  and settles `reproduced-without-witness`.
- Step 2: `undefined_expression`/`unproved-nonzero` at the `mod`; no claim.
- Step 3: the package records `Linked`; the replay recompiles under `Linked`
  and settles `inconclusive` with cause `Verdicts`.
