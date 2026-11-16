---
id: TC-2176
title: "A time-locked state is terminal"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-2176: A time-locked state is terminal

## Description

A time-locked state is terminal.

Scope: FR-204-AC-17.

## Test Procedure

Build `Under` over `Retx` with `discrete { 5 ms: 1 }` under the one-weight workload. Test: `fsl-analyze/tests/it/digital_clocks.rs` `time_locked_state_is_terminal`.

Tag the tests `#[trace("TC-2176", "FR-204-AC-17")]`.

## Expected Results

- A product; no `NotStochastic` refusal.
