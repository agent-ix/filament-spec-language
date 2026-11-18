---
id: TC-2989
title: "Observe-only protocol boundaries at S3"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
---
# TC-2989: Observe-only protocol boundaries at S3

## Description

Verify the S3 check of observe-only roles and controls and of `recommend`. Scope: FR-218-AC-45 to FR-218-AC-47.

## Test Procedure

1. Check the observe-only role clause and the `observe sequence Review` clause of FR-218-AC-45, then both without `observe`; parse `observe observe sequence ...`; check a clause with a capture named `observe`.
2. Place each of an `attempt`, an `effect`, a `send`, a `commit` and an `event ... for` inside `Review`, then outside; check an `attempt` and a compensation template by an observe-only role; nest the attempt in a `choice` case inside `Review`; write two offending actions.
3. Check a `recommend` over an unknown operation, one with a foreign contract, and a guard and a `visible` entry reading its binder.

## Expected Results

- Step 1: both clauses check and record their marks; without `observe` they are equal except for the marks; the double mark fails `invalid_syntax`/`unexpected-token` at the second `observe`; the capture checks.
- Step 2: each offending action refuses `ill_typed`/`operator-ineligible` with the region text exactly its declaration, `action` and `boundary` as FR-218-AC-46 states; outside `Review` each checks; two actions give two refusals in source order and none at the role or the control header.
- Step 3: the operation and contract refusals equal those of an `attempt`; each binder read refuses `ill_typed`/`type-mismatch` at the read with `expected` no result.
- Assertions read typed refusal members and regions, never message text.
