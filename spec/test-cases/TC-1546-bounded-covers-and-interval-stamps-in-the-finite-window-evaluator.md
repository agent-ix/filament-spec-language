---
id: TC-1546
title: "Bounded covers and interval stamps in the finite-window evaluator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: verifies
---
# TC-1546: Bounded covers and interval stamps in the finite-window evaluator

## Description

Verify the `Bounded` cover and per-observation interval stamps of the timestamped finite-window evaluator.

Scope: FR-501-AC-27, FR-501-AC-28.

## Test Procedure

1. Under `Bounded { uncertainty: 2 }`, evaluate AC-27's response clause with `ack` stamped 5, 10 and 15 and with no `ack` at `W` 11 and 12; repeat under `Exact`.
2. Evaluate `until` and `eventually` over interval stamps `[3, 5]` and `[4, 6)`; supply `[6, 5]` and `[5, 5)`; supply `[0, 1]` under `Bounded { uncertainty: 5 }`.

Tag the tests `#[trace("TC-1546", "FR-501-AC-n")]`.

## Expected Results

- Step 1: true, `Indeterminate { StraddlesBound }`, false; `Pending` then false; under `Exact` true, true, false.
- Step 2: `until` refuses at the later position, `eventually` evaluates; both malformed stamps refuse; `[0, 1]` is read as given.
