---
id: TC-2164
title: "The live-occurrence limit stops a deep recursion whatever the work budget"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-130
    type: verifies
---
# TC-2164: The live-occurrence limit stops a deep recursion whatever the work budget

## Description

The live-occurrence limit stops a deep recursion whatever the work budget.

Scope: NFR-130-AC-3.

## Test Procedure

Run `d` of NFR-130-AC-1 with `n = 1000000` through `command::run` with `call.accounting.work_units` 100000000000, once under the default live-occurrence limit and once with `call.evaluation.live_occurrences` 4000000. Test: `tests/it/spine_run.rs` `native_run_live_occurrence_limit_stops_deep_recursion`.

Tag the tests `#[trace("TC-2164", "NFR-130-AC-3")]`.

## Expected Results

- Default: exit 22, outcome `{"kind": "incomplete", "limit": "evaluation.live_occurrences"}`.
- Raised: exit 0, the completed value `0`.
