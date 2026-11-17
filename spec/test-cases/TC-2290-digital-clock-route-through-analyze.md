---
id: TC-2290
title: "The digital-clock route decides closed timed items and refuses strict ones"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: verifies
---
# TC-2290: The digital-clock route decides closed timed items and refuses strict ones

## Description

Verify the caller-selected digital-clock route: it decides a closed subject
and closed claim on EN-1 over the integer-clock reading with basis
`exhaustive`, and refuses a strict constraint or interval.
Scope: FR-632-AC-4, FR-243-AC-4.

## Test Procedure

1. Analyze `NoLateReply` over the all-closed `Rpc` with `T = 4 ms` and `options.method` `digital-clocks`.
2. Analyze the same item over ADR-026 §9's strict-constraint retry model.
3. Analyze `eventually[0 ms, 2 ms) holds(not c.late)` over the closed `Rpc` with `options.method` `digital-clocks`.

## Expected Results

- Step 1: `proved`, basis `exhaustive`, backend `en-1`, the same verdict as the zone route.
- Steps 2 and 3: `declined` `invalid-input` naming the strict constraint or interval; no engine runs.
- Assertions compare typed members and the exit code, never message text.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`digital_clocks_refuse_strict_constraints_and_intervals`.
