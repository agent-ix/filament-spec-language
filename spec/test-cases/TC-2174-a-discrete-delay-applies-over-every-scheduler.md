---
id: TC-2174
title: "A discrete delay applies over every scheduler"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-2174: A discrete delay applies over every scheduler

## Description

A discrete delay applies over every scheduler.

Scope: FR-204-AC-15.

## Test Procedure

Build `Deadline` over `Retx` with a `discrete` delay on `send` over every scheduler. Test: `fsl-analyze/tests/it/digital_clocks.rs` `discrete_delay_applies_over_every_scheduler`.

Tag the tests `#[trace("TC-2174", "FR-204-AC-15")]`.

## Expected Results

- An MDP product; no `DelayDistribution` refusal.
