---
id: TC-1704
title: "A Reference result is written as its identity triple and passes back"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1704: A Reference result is written as its identity triple and passes back

## Description

A Reference result is written as its identity triple and passes back.

Scope: FR-650-AC-5.

## Test Procedure

Run TC-792's present lookup from a file request whose `call.snapshot` holds the object; pass the completed value to `same`.

Tag the tests `#[trace("TC-1704", "FR-650-AC-5")]`; they live in `tests/it/run_values.rs`.

## Expected Results

Both complete with `{"reference": {"universe": U, "type": T, "object_identity": O}}` equal to the returned reference, exit 0.
