---
id: TC-1671
title: "Trace kind mismatch refusals"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: verifies
---
# TC-1671: Trace kind mismatch refusals

## Description

Verify FR-645-AC-3 to AC-5: a trace kind the clause's profile does not read refuses at stage `admit` with the profile, the expected and supplied kinds and a message, and the in-process record differs only by the absent request pointer.

## Test Procedure

Run `Bounded` (event-position) over a `lasso`, a timestamped-event clause over `finite`, and a timed clause over `finite` snapshots, through `command::run` and in process.

## Expected Results

- `Bounded`: `expected` `["finite", "progressed"]`, `supplied` `lasso`; timestamped: `expected` `["timestamped"]`; timed: `expected` `["timed"]`, each with the table's message and the clause locus.
- In-process records are equal apart from the absent `request_pointer`.
