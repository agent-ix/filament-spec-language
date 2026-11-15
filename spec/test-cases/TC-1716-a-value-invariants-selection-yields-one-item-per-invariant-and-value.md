---
id: TC-1716
title: "A value-invariants selection yields one item per invariant and value"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: verifies
---
# TC-1716: A value-invariants selection yields one item per invariant and value

## Description

A value-invariants selection yields one item per invariant and value.

Scope: FR-652-AC-3.

## Test Procedure

Run FR-652-AC-3's `value_invariants` selection over TC-1051's K2 fixture, then with a `pre` snapshot.

Tag the tests `#[trace("TC-1716", "FR-652-AC-3")]`; they live in `tests/it/run_items.rs`.

## Expected Results

One item per result with `invariant` and `path`, the `amount` item `violation`, exit 10; the `pre` snapshot gives one `wrong_snapshot` item.
