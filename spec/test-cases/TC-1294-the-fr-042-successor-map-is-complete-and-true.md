---
id: TC-1294
title: "The FR-042 successor map is complete and true"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-500
    type: verifies
---
# TC-1294: The FR-042 successor map is complete and true

## Description

Verify FR-500-AC-1 to AC-3. Catches a criterion with no row, a successor test
that moved or lost its tag, and a staging row without an owner or deletion
condition. The gate-open deletion rule is a review/process check, not a
machine-recorded gate-state assertion.

## Test Procedure

1. List FR-042-AC-1 to AC-15 and confirm each has a row in FR-500.
2. For each successor `file:line` in FR-500, open the file and confirm the
   successor criterion tag is at or within the test at that line.
3. For each staging-table row, verify that the Owner and Deleted when cells
   are present and non-empty. Do not infer or maintain gate state in a file;
   the open-gate deletion rule is reviewed as the process requirement states.

## Expected Results

- Every criterion has a row; every successor location carries its tag; every
  staging-table row has a non-empty owner and deletion condition.
