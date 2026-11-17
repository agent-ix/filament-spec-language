---
id: TC-2172
title: "A strict clock constraint is decided over regions"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-2172: A strict clock constraint is decided over regions

## Description

A strict clock constraint is decided over regions.

Scope: FR-204-AC-13.

## Test Procedure

Build `Deadline` over `Retx` with the guard `self.x > 1 ms` over every scheduler and decide it. Test: `fsl-analyze/tests/it/digital_clocks.rs` `strict_clock_constraint_is_decided_over_regions`.

Tag the tests `#[trace("TC-2172", "FR-204-AC-13")]`.

## Expected Results

- An MDP product, no refusal; exact minimum `99/100`, exact maximum `999/1000`.
