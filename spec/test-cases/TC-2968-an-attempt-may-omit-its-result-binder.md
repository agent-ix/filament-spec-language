---
id: TC-2968
title: "An attempt may omit its result binder"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: verifies
---
# TC-2968: An attempt may omit its result binder

## Description

Verify FR-218-AC-44 and FR-218-AC-45, and the binder-less case of FR-882-AC-4.

## Test Procedure

1. Check `Twice` with `attempt a1 by w on Spec::Counter::inc contracts [];` over a void operation, format it, and parse the formatted text.
2. Write the same attempt with `as (r: Boolean) { true }`.
3. Write an attempt over a Boolean-returning operation without a binder, then a guard naming its would-be binder.

## Expected Results

- Step 1: the unit checks and the formatter round-trips the source.
- Step 2: `ill_typed`/`type-mismatch` at the binder.
- Step 3: the attempt checks; the guard refuses `missing_declaration`/`missing-name`.
