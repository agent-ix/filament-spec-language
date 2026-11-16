---
id: TC-1703
title: "Reference and Population arguments resolve in the call's snapshot"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1703: Reference and Population arguments resolve in the call's snapshot

## Description

Reference and Population arguments resolve in the call's snapshot.

Scope: FR-650-AC-4.

## Test Procedure

Run FR-109-AC-4's `sameIdentity` as a `call` with `snapshot` the distinct-identities snapshot, with key-form and identity-form references, with key `ghost`, and without `snapshot`; run a Population-parameter function with a held and an unheld population.

Tag the tests `#[trace("TC-1703", "FR-650-AC-4")]`; they live in `tests/it/run_values.rs`.

## Expected Results

As FR-650-AC-4 states: `{"boolean": false}` for root/child in either reference form; `dangling_reference` at position 1; `missing-member` with `details.path` `/call/snapshot`; the held population completes and the unheld one refuses `wrong-role-mapping`.
