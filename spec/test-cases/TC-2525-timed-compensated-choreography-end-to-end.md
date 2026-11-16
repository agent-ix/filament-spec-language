---
id: TC-2525
title: "A timed compensated choreography compiles, runs and projects end to end"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-892
    type: verifies
---
# TC-2525: A timed compensated choreography compiles, runs and projects end to end

## Description

Verify that a timed compensated choreography compiles, runs and projects end to end. Scope: FR-892-AC-1, FR-892-AC-2, FR-892-AC-3, FR-892-AC-4.

Catches a compile that drops the await's clock, or a message that misnames a template.

## Test Procedure

1. Compile the staged saga with `clocks` `{"c": ...}`, explore it, project it under `bounded`; compile it without `clocks`, then with only the template's clock.

## Expected Results

- Step 1: The package holds the template, the await and the `requires temporal` edge; deadlock freedom settles, with a `recovered` registration and a `timeout` reachable; projection carries the window; the refusals name `compensate template Refund` and then the await, never `temporal declaration`.
