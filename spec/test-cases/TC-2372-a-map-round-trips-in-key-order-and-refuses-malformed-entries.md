---
id: TC-2372
title: "A Map round-trips in key order and refuses malformed entries"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2372: A Map round-trips in key order and refuses malformed entries

## Description

A Map round-trips in key order and refuses malformed entries.

Scope: FR-650-AC-11.

## Test Procedure

Call `id_map` with entries `b` then `a`, then with a repeated key, three entries, an entry without `value` and an entry with `extra`. Test: `tests/it/run_value_spellings.rs` `a_map_round_trips_in_key_order`.

Tag the tests `#[trace("TC-2372", "FR-650-AC-11")]`.

## Expected Results

- The completed map holds `a` then `b`.
- The refusals are `duplicate-member` at `/map/1/key`, `above-maximum` at `""`, `missing-member` at `/map/0` and `unknown-member` at `/map/0/extra`, each exit 20.
