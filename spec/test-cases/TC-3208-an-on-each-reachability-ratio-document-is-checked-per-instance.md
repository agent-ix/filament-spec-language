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

Scope: FR-201-AC-76.

## Test Procedure

Build the document for the `m`/`n` snapshot and check it at `rational(9, 10)` and `rational(99, 100)`; check the same document over a snapshot holding only `n`, and a `Vacuous` certificate there; change the numerator at `m`'s initial key to `1/2` and check. Test: `tools/fsl-probability-check/tests/it/on_each.rs`.

Tag the tests `#[trace("TC-3208", "FR-201-AC-76")]`.

## Expected Results

- Accepted at `9/10` with numerator `391/400`, denominator `1` at `m`; `n` vacuous (`0`, `0`).
- `ThresholdMissed` at `99/100`.
- Only `n`: the `Ratio` document `InequalityFails`; the `Vacuous` certificate accepted.
- The tampered numerator `InequalityFails` with `part` `numerator`.
