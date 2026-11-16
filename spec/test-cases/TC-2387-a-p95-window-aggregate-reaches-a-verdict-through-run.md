---
id: TC-2387
title: "A p95 window aggregate reaches a verdict through run"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-194
    type: verifies
---
# TC-2387: A p95 window aggregate reaches a verdict through run

## Description

A p95 window aggregate reaches a verdict through run.

Scope: FR-194-AC-13.

## Test Procedure

Run the quantile clauses over the 20 timestamped responses through `command::run`, and read the catalog category of `unsupported_construct`/`expression-form`. Test: `tests/it/clock_reach.rs` `a_quantile_aggregate_reaches_a_verdict_through_run`.

Tag the tests `#[trace("TC-2387", "FR-194-AC-13")]`.

## Expected Results

- `quantile 0.95` reports `violated`, exit 10; `quantile 0.9` holds, exit 0.
- The model-subject refusal is category refusal, exit 20.
