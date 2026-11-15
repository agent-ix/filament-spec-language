---
id: TC-1902
title: "Ordered-end positions and unbounded-end counts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-364
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: verifies
---
# TC-1902: Ordered-end positions and unbounded-end counts

## Description

Verify FR-364-AC-15: an ordered end's instances carry positions that order navigation, malformed positions refuse at admission, and an unbounded end checks only its lower bound.

## Test Procedure

Admit the snapshots of FR-364-AC-15 through FR-106 and navigate from `o1` over the ordered and unbounded ends; admit each malformed-position variant. Test: `fsl-replay/src/spine/clause/tests/ordered_end_positions.rs` (ordered order and malformed positions, through `run_clause` with snapshot JSON).

## Expected Results

Targets come back in ascending position; each malformed variant refuses `invalid_runtime_input`/`invalid-value` at the later instance; the unbounded end refuses `below-minimum` at zero links and returns four references at four.
