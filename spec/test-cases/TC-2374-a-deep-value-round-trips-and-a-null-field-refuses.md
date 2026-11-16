---
id: TC-2374
title: "A deep value round-trips and a null field refuses"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2374: A deep value round-trips and a null field refuses

## Description

A deep value round-trips and a null field refuses.

Scope: FR-650-AC-13.

## Test Procedure

Call an identity function over a 60-level nested sequence type with its fully nested value; call `id_rec` with `y` `null`. Test: `tests/it/run_value_spellings.rs` `a_deep_value_round_trips_and_null_refuses`.

Tag the tests `#[trace("TC-2374", "FR-650-AC-13")]`.

## Expected Results

- The deep call completes with the identical value, more than 100 JSON levels deep.
- The `null` field refuses `wrong-value-kind` at `/record/fields/y`, exit 20.
