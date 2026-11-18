---
id: TC-2972
title: "An attempt binder read is checked against the application record"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
---
# TC-2972: An attempt binder read is checked against the application record

## Description

Verify that an attempt binder of any declared type is admitted, that a read
gets the operation's result when the binder type is the result type, and that
otherwise each read is refused at the read. Scope: FR-218-AC-44.

## Test Procedure

1. Check `flag` with `as (t: Boolean)` and `check Gate using v { t }`.
2. Check `flag` with `as (t: Integer)` and `touch` with `as (t: Boolean)`, neither read.
3. Check `flag` with `as (t: Integer)` and `check Gate using v { t = 1 }`, and `touch` with `as (t: Boolean)` and `check Gate using v { t }`.
4. Check a mismatched binder read twice.

Test: `fsl-analyze/tests/it/protocol_roles.rs`.

## Expected Results

- Steps 1 and 2: each unit checks.
- Step 3: one refusal each, `ill_typed`/`type-mismatch` at the read `t`, never at the binder declaration.
- Step 4: two refusals, one at each read.
