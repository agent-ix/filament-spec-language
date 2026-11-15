---
id: TC-1666
title: "Incremental monitor equals batch monitoring"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-1666: Incremental monitor equals batch monitoring

## Description

Verify FR-643-AC-1 to AC-3: a `MonitorSession` fed one observation at a time reports, after each, what FR-283's batch `monitor` reports over that prefix, settled results never change, and each step names the clauses that changed.

## Test Procedure

Open a session over FR-330's `Bounded`, `Reaches` and `always holds(c.value <= 1)` and feed snapshots `c.value` 0, 1, 2, 1, 0; after each step run `monitor` over the prefix. Repeat with a `Timestamped` session over FR-641-AC-1's observations with progress after each.

## Expected Results

- Every step's reports equal the prefix batch reports; `finish(Closed)` equals the closed batch run.
- The violation settled at `c.value` 2 is reported unchanged afterwards.
- The `Timestamped` session ends with FR-501-AC-4's outcome and the batch run's ledger.
