---
id: TC-2542
title: "Multi-location accesses are program-order single-location accesses and a multi-location RMW is a locked transaction"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-902
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-220
    type: verifies
---

# TC-2542: Multi-location accesses are program-order single-location accesses and a multi-location RMW is a locked transaction

## Description

Verify that multi-location accesses are program-order single-location accesses and a multi-location RMW is a locked transaction. Scope: FR-902-AC-1, FR-902-AC-2.

Catches a pre-check that still refuses a multi-location shape, or a decomposition that reads both locations as one snapshot.

## Test Procedure

1. Explore FR-220-AC-5's two-field attempt `Bx` beside a two-field load under `tso` and under `ra`; explore a two-location compare-and-swap under both models.

## Expected Results

- Step 1: No `weak-access-shape` result; `Bx`'s stores flush in canonical location order; a state reads the new first field with the old second field; the compare-and-swap is one step, enabled under `tso` only with an empty buffer, and no state shows it at one location only.
