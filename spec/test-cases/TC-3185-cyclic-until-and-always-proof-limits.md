---
id: TC-3185
title: "Cyclic until and always proof limits"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3185: Cyclic until and always proof limits

## Description

Verify FR-201-AC-63: caller limits and cancellation hold throughout the
cyclic until and always proof checks.

## Test Procedure

Over FR-201-AC-59 and FR-201-AC-60, pre-cancel, cancel with Requested and
Deadline, and exhaust each byte, state, edge, work, storage, representation
and arithmetic limit during original admission, classification-graph
construction, end-component and bottom-component decomposition,
certificate-graph construction, predicate evaluation, full-key
materialization, exact sums and rank passes. Make a state predicate
undefined.

## Expected Results

Each run preserves its actual typed limit or cancellation cause with no
accepted truth or proof. The undefined predicate is not read as false.
