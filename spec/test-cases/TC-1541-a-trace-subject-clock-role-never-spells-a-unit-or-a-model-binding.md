---
id: TC-1541
title: "A trace subject's clock role never spells a unit or a model binding"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-610
    type: verifies
---
# TC-1541: A trace subject's clock role never spells a unit or a model binding

## Description

Verify that a clock role over a trace subject is free text that never changes what a tick counts, that unit-spelled and model-binding role names are refused, and that a retired temporal profile spelling is an unknown profile.

Scope: FR-610-AC-1 to FR-610-AC-4.

## Test Procedure

Fixture: the record `Counter` with `level: Int[0, 9]`, a clause `eventually[0,3] holds(c.level = 3)`.

1. Check with `clock "events"` under infinite-trace, event-position and timestamped-event, and `clock "sample-clock"` under fixed-sample.
2. Check with `clock "ms"`, `"s"`, `"min"`, a declared time unit `tick`, and `"msg-clock"`.
3. Check with `clock "model-steps"` and `clock "model-time"`.
4. Select `quire.temporal.infinite-trace/v1` and then `filament.temporal.infinite-trace/v1`.

Tag the tests `#[trace("TC-1541", "FR-610-AC-n")]`.

## Expected Results

- Step 1: each checks, role as written, `[0,3]` as three ticks of the profile's kind.
- Step 2: each unit spelling refuses `ill_typed`/`type-mismatch` at the clock string naming role, profile and tick kind; `msg-clock` checks.
- Step 3: each refuses `ill_typed`/`type-mismatch` at the clock string.
- Step 4: `unknown_profile`/`unsupported-selection` at the identity literal; the `filament.*` spelling checks.
