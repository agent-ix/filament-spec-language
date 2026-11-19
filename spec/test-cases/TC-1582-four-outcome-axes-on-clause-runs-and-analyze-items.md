---
id: TC-1582
title: "Activation, execution, truth and settlement are four independent axes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-622
    type: verifies
---
# TC-1582: Activation, execution, truth and settlement are four independent axes

## Description

Verify that clause runs and analyze items carry FR-622's `axes` and function runs do not. Scope: FR-622-AC-1 to FR-622-AC-4, FR-267-AC-22, FR-100-AC-21.

## Test Procedure

1. Run clauses whose FR-516 results are active-true, active-false, inactive and unavailable, and read `axes`.
2. Run a function and read its `spine-run-result/1`.
3. Analyze items settling `proved`, `refuted`, `incomplete`, `unsupported`, and a schedulability item.
4. Run one clause twice, changing only the trace's scope closure.

## Expected Results

- Step 1: `axes` equals each result's four fields.
- Step 2: no `axes` member.
- Step 3: `axes` per FR-622's table; the schedulability item has none.
- Step 4: equal `truth`, `execution` and category; different `settlement`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
