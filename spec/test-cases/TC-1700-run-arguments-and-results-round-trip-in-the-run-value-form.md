---
id: TC-1700
title: "Run arguments and results round-trip in the run value form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1700: Run arguments and results round-trip in the run value form

## Description

Run arguments and results round-trip in the run value form.

Scope: FR-650-AC-1.

## Test Procedure

Write FR-650's identity fixture and run, through `command`'s `run`, one native-run/1 `call` per row of FR-650's identity-fixture table.

Tag the tests `#[trace("TC-1700", "FR-650-AC-1")]`; they live in `tests/it/run_values.rs`.

## Expected Results

Each run exits 0 with one `spine-run-result/1` document whose `outcome` is `{"kind": "completed", "value": V}`, `V` the row's completed value.
