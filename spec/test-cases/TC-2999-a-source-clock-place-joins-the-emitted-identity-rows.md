---
id: TC-2999
title: "A source clock place joins the emitted identity rows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-996
    type: verifies
---

# TC-2999: A source clock place joins the emitted identity rows

## Description

Verify that S4 lists every clock member of a timed source model as a clock
entry of its identity rows, and that a clock-place row the package alone
cannot contradict refuses at the identity-row join. Scope: FR-996-AC-3
(FSpec FR-789-AC-3).

## Test Procedure

1. Emit the FR-996-AC-1 unit and read `identities.json` from its handoff.
2. Read the package strictly and join the identity rows.
3. Apply each FR-996-AC-3 edit to the package or the identity rows; read and
   join each.

## Expected Results

- Step 1: `O` has the data entry `w` and the clock entry `d`, `W` the clock
  entry `c`.
- Step 2: both places are joined.
- Step 3: each edited package passes the strict read and refuses
  `invalid_package`/`malformed-clock-place` at the stated row at the join.

Tag each test `#[trace("FR-996-AC-3", "TC-2999", "FSpec-FR-789-AC-3", "FSpec-TC-2023")]`.
