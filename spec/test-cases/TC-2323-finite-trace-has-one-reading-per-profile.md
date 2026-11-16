---
id: TC-2323
title: "A finite monitor trace has one reading per profile"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: verifies
---
# TC-2323: A finite monitor trace has one reading per profile

## Description

Verify that a finite trace given to `monitor` is read closed under an
event-position profile and as a prefix under infinite-trace, with no member to
change it. Scope: FR-283-AC-17.

## Test Procedure

1. Monitor `Bounded` and `Reaches` over the `Finite` trace `c.value` 0, 1, 2.
2. Monitor a `Document` trace carrying a `closed` member.

## Expected Results

- Step 1: the closed false-extension record for `Bounded`; the finite-prefix record for `Reaches`.
- Step 2: refused `invalid_runtime_input`/`unknown-member`.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`.
