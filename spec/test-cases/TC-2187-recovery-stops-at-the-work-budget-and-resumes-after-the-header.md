---
id: TC-2187
title: "Recovery stops at the work budget and resumes after the header"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2187: Recovery stops at the work budget and resumes after the header

## Description

Recovery stops at the work budget and resumes after the header.

Scope: FR-762-AC-8, FR-762-AC-9, FR-762-AC-10.

## Test Procedure

1. Find the smallest `s1.work_units` at which TC-2148's unit records its first error, and parse under it. Test: `fsl-cst/tests/it/complete_cst.rs` `recovery_stops_at_the_work_budget`.
2. Parse a unit with `profile v = ;` in its header and a record missing a `;`. Test: `header_recovery_resumes_at_the_next_declaration`.
3. Parse a unit whose last declaration holds an error. Test: `an_error_with_no_resumption_point_is_last`.

Tag the tests `#[trace("TC-2187", "FR-762-AC-8")]`.

## Expected Results

- Step 1: the report ends with `diagnostic-list-cut` naming `s1.work_units`; no limit diagnostic; not admissible.
- Step 2: two diagnostics, at the header's `;` and at the record's `y`; the record's `Declaration` node is present.
- Step 3: the error is the last diagnostic; no `diagnostic-list-cut`.
