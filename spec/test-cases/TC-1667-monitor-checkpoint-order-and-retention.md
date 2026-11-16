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

Checkpoint TC-1666's session after step 2 and resume it; resume under another package and clause list; feed a stamp at the retained watermark, then a correct one; open a `Finite` session with `retained_positions` 2 over `Recent` (`count where holds(r.level = 2) over past[0, 3] min_count 0 <= 1`, a window of four positions), `Reaches` and `Never`, and feed levels 0, 1, 0.

## Expected Results

- The resumed session reports steps 3 to 5 as the original; mismatches refuse `monitor-state-mismatch`.
- The regressing stamp refuses `Watermark` and the next observation is accepted.
- `Recent` reports `incomplete` naming `monitor.retained_positions` at the third observation; `Reaches` and `Never`, which retain no value, stay `Pending`.
