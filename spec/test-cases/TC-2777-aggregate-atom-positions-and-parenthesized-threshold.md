---
id: TC-2777
title: "A window-aggregate atom stands in any temporal position and a parenthesized threshold is typed"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-982
    type: verifies
---
# TC-2777: A window-aggregate atom stands in any temporal position and a parenthesized threshold is typed

## Description

A window-aggregate atom stands in any temporal position and a parenthesized threshold is typed.

Scope: FR-982-AC-3, FR-982-AC-4.

## Test Procedure

Parse the formulas of FR-982-AC-3 and check those of FR-982-AC-4 under the event-position profile and, with the window `past[0 ms, 1000 ms]`, under the timestamped-event profile. Tests: `fsl-cst/tests/it/aggregate_connectives.rs`, `fsl-semantics/tests/it/aggregate_threshold.rs`.

## Expected Results

- The parenthesized threshold and the middle atom parse; `holds(count where ..)` refuses at `where`.
- Under each profile `(s.k + 1)` checks and `(s.ok and s.done)` refuses `ill_typed`/`type-mismatch` at the parenthesis; no formula uses an interval-less operator.
