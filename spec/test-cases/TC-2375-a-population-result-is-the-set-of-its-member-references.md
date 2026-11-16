---
id: TC-2375
title: "A population result is the set of its member references"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2375: A population result is the set of its member references

## Description

A population result is the set of its member references.

Scope: FR-650-AC-14.

## Test Procedure

Call `all` with `{"population": P}` over the healthy current snapshot. Test: `tests/it/run_value_spellings.rs` `a_population_result_is_a_set_of_references`.

Tag the tests `#[trace("TC-2375", "FR-650-AC-14")]`.

## Expected Results

- The completed value is `{"set": [R1, R2]}` of the identity references of `root` and `child`, ascending by RFC 8785 bytes.
