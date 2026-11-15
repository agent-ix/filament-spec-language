---
id: TC-1667
title: "Monitor checkpoint, order and retention"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-1667: Monitor checkpoint, order and retention

## Description

Verify FR-643-AC-4 and AC-5: a checkpoint resumes to the same later reports, a mismatched resume refuses, a regressing stamp leaves the session unchanged, and the retained-position limit stops only the clause that exceeds it.

## Test Procedure

Checkpoint TC-1666's session after step 2 and resume it; resume under another package and clause list; feed a stamp at the retained watermark, then a correct one; open with `retained_positions` 2 over an infinite-trace and a bounded clause.

## Expected Results

- The resumed session reports steps 3 to 5 as the original; mismatches refuse `monitor-state-mismatch`.
- The regressing stamp refuses `Watermark` and the next observation is accepted.
- The infinite-trace clause reports `incomplete` naming `monitor.retained_positions`; the bounded clause settles.
