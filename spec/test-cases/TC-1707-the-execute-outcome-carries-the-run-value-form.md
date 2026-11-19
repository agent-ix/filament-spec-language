---
id: TC-1707
title: "The execute outcome carries the run value form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1707: The execute outcome carries the run value form

## Description

The execute outcome carries the run value form.

Scope: FR-650-AC-8.

## Test Procedure

Serialize the `execute` outcome of `seven` and of `id_rec` through FR-286.

Tag the tests `#[trace("TC-1707", "FR-650-AC-8")]`; they live in `tests/it/run_values.rs`.

## Expected Results

`result` `{"kind": "completed", "value": {"integer": "7"}}` and the record form, equal to the `spine-run-result/1` values.
