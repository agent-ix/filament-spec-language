---
id: TC-2175
title: "A zero-delay cycle is decided with time divergence"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-2175: A zero-delay cycle is decided with time divergence

## Description

A zero-delay cycle is decided with time divergence.

Scope: FR-204-AC-16.

## Test Procedure

Build and decide `Deadline` over the guardless, resetless `Retx` over every scheduler. Test: `fsl-analyze/tests/it/digital_clocks.rs` `zero_delay_cycle_is_decided_with_time_divergence`.

Tag the tests `#[trace("TC-2175", "FR-204-AC-16")]`.

## Expected Results

- A product, no `ZeroDelayCycle`; exact minimum and maximum both `1`.
