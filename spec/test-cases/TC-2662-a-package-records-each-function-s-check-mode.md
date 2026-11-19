---
id: TC-2662
title: "A package records each function's check mode and replay recompiles under it"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-952
    type: verifies
---
# TC-2662: A package records each function's check mode and replay recompiles under it

## Description

Verify that S4 writes `check_mode` on every function node, that replay
recompiles each body under its recorded mode, that a Kernel world applies
`Kernel` to function bodies on the probability path while declared clauses
stay `Linked`, and that the reader refuses a missing or wrong mode. Scope:
FR-952-AC-1, FR-952-AC-2, FR-952-AC-3, FR-952-AC-4.

## Test Procedure

1. Compile AC-1's units under `Linked` and `Kernel`; read every `function` node's `check_mode`.
2. Replay `r`'s `10 mod x` claim at `x = 0` from the Kernel package.
3. Build the probabilistic world under `Kernel` with the target calling `z`, then with `1 mod x.phase = 0` written in the clause.
4. Read packages with the member removed, set to `"strict"`, and set to `"kernel"` on an invariant clause function.

Test: `fsl-analyze/tests/it/check_mode_record.rs`; replay case in
`fsl-replay/src/execute/tests/application_claim.rs`
`tc_1784_replay_never_rechecks_a_claim_under_another_mode`.

## Expected Results

- Step 1: `linked` everywhere under `Linked`; `kernel` on the bodies and `linked` on the invariant under `Kernel`; the packages are unequal.
- Step 2: `reproduced-without-witness`.
- Step 3: `z` checks and evaluates undefined at `p = 0`; the clause form refuses `unproved-nonzero` at S3.
- Step 4: `malformed_wire` twice, then `invalid_package`/`invalid-value`.
