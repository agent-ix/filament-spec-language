---
id: TC-2384
title: "An open prefix and an incomplete input give distinct run results"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
---
# TC-2384: An open prefix and an incomplete input give distinct run results

## Description

An open prefix and an incomplete input give distinct run results.

Scope: FR-641-AC-9.

## Test Procedure

Run the fixed-sample clause over one sample with an open scope at watermark 0, then with a closed scope at watermark 5 and completeness `incomplete`. Test: `tests/it/clock_reach.rs` `an_open_prefix_and_an_incomplete_input_differ`.

Tag the tests `#[trace("TC-2384", "FR-641-AC-9")]`.

## Expected Results

- The first reports `pending-window`, deadline 3, exit 0.
- The second reports `indeterminate` with `IncompleteInput`, exit 11, never `violated`.
