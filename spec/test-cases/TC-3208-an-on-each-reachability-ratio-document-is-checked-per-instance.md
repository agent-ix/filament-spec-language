---
id: TC-3208
title: "An on-each reachability ratio document is checked per instance"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: verifies
---
# TC-3208: An on-each reachability ratio document is checked per instance

## Description

The `on-each-reachability/v1` `Ratio` document of FR-875's `E` over `Link`, with `m` undelivered and `n` delivered, is accepted at `9/10` and rejected at `99/100`; the all-vacuous and tampered cases are rejected.

Scope: FR-201-AC-76, FR-201-AC-77.

## Test Procedure

Build the document for the `m`/`n` snapshot and check it at `rational(9, 10)` and `rational(99, 100)`; build the document over a snapshot holding only `n` and check it, and check a `Vacuous` document there. Test: `tools/fsl-probability-check/tests/it/on_each.rs`.

Tag the tests `#[trace("TC-3208", "FR-201-AC-76")]` and `#[trace("TC-3208", "FR-201-AC-77")]`.

## Expected Results

- Accepted at `9/10` with numerator `391/400`, denominator `1` at `m`; `n` vacuous (`0`, `0`).
- `ThresholdMissed` at `99/100`, at `m`'s initial key, no `part`.
- Only `n`: the `Ratio` document `InequalityFails` at `n`'s initial key, no `part`; the `Vacuous` document accepted.
