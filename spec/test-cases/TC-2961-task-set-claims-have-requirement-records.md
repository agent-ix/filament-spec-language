---
id: TC-2961
title: "Task-set claims have requirement records that route to EN-7"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: verifies
---
# TC-2961: Task-set claims have requirement records that route to EN-7

## Description

Verify that S3 writes one `schedulability` requirement record per task-set
claim, that the request builder routes each to EN-7, and that the package
route reads equal records. Scope: FR-246-AC-4.

## Test Procedure

1. Check `Ctl` with `CtlFp`, `CtlEdf` and `R`; read `CheckedGraph::requirements`.
2. Run `items_from_requirements` over the records and `Engines::fsl()`'s registry.
3. Compile the unit and read the records from `schedulability_claims`.
4. Check `Ctl` with a period of 0 ms.

Test: `fsl-analyze/tests/it/schedulability_routing.rs`.

## Expected Results

- Step 1: three `schedulability` records, `Bounded`, no owner, each policy.
- Step 2: three items, candidates `["en-7"]`.
- Step 3: equal items.
- Step 4: no record.
