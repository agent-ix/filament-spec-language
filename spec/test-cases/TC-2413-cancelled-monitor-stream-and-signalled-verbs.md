---
id: TC-2413
title: "A cancelled monitor stream keeps its settled clauses and reports open ones incomplete; signals cancel driver verbs"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-853
    type: verifies
---
# TC-2413: A cancelled monitor stream keeps its settled clauses and reports open ones incomplete; signals cancel driver verbs

## Description

Verify `command::monitor`'s cancelled summary in process, and the driver's
signal handling end to end. Scope: FR-853-AC-1 to FR-853-AC-4; AC-4 is a
driver test in the filament-driver repository.

## Test Procedure

1. Monitor `Reaches` and `Never` over levels 0, 1 with an iterator that trips the handle with `Requested` and ends; repeat with `{"end": true}` and no cancellation.
2. Monitor both over levels 0, 2 then a `Deadline` trip; monitor `Never` alone over 0, 1 then a `Deadline` trip.
3. Call `command::monitor` with a handle already tripped, over a valid request and over one naming a missing clause.
4. Through the driver: `fsl monitor` with stdin held open, SIGINT after two records; `fsl analyze` and `fsl run` signalled with SIGTERM while running.

## Expected Results

- Step 1: both clauses `incomplete`, `settlement` `{"step": "2", "trigger": "cancelled", "cause": "requested"}`, `cancelled` `requested`, 2 records read, no `finish`, exit 22; the uncancelled stream settles by `finish`.
- Step 2: settled clauses keep their record-1 settlements, `cancelled` `deadline`, exit 10; `Never` alone cancelled with cause `deadline`, exit 22.
- Step 3: no record read, no event, every clause cancelled at step `"0"`, exit 22; the missing clause refuses as FR-644-AC-4.
- Step 4: the summary is the last stdout line within one second, exit 22; the cancelled outcomes carry cause `requested`, exit 22.

Test: `tests/it/run_wire.rs` (steps 1 to 3); filament-driver (step 4).
