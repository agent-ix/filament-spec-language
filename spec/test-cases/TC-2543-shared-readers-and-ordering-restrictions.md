---
id: TC-2543
title: "Node readers of shared locations load relaxed and mismatched orderings are restricted"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-902
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-220
    type: verifies
---

# TC-2543: Node readers of shared locations load relaxed and mismatched orderings are restricted

## Description

Verify that node readers of shared locations load relaxed and mismatched orderings are restricted. Scope: FR-902-AC-3, FR-902-AC-4.

Catches an ordering mismatch answered `unsupported` instead of restricted.

## Test Procedure

1. Explore FR-220-AC-15's `event` reader of `x.v` under `tso`; explore a `release` load and an `acq_rel` store under `tso` and `ra`.

## Expected Results

- Step 1: The reader has one successor per visible value and no `MultiLocation` entry; the load acts as `relaxed` and the store as `release`, and `ordering-restrictions` lists both.
