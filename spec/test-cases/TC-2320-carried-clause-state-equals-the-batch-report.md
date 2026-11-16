---
id: TC-2320
title: "Carried clause state equals the batch report and holds only the values it reads"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: verifies
---
# TC-2320: Carried clause state equals the batch report and holds only the values it reads

## Description

Verify FR-820's incremental evaluator: each `advance` returns FR-330's report
over the prefix, the state holds only the values a clause still reads, and the
state round-trips through bytes. Scope: FR-820-AC-1 to FR-820-AC-4.

## Test Procedure

1. Advance `Bounded`, `Reaches` and `always holds(c.value <= 1)` over `c.value` 0, 1, 2, 1, 0 and compare each report with FR-330's over the prefix.
2. Advance `always holds(r.level < 9)`, `eventually holds(r.level = 9)` and the `past[0, 3]` count clause (`<= 1`) over 1,000 samples cycling 0 to 8, reading `retained()` after each.
3. Write the state after position 2, read it back, and advance positions 3 and 4; cancel an `advance`.
4. Advance `E` over levels 0, 1, 2; assert progress with watermark 3 on a fixed-sample `Progressed` state with no new position; open with a `HistoryCutoff` lower boundary.

## Expected Results

- Step 1: every report equals FR-330's, member for member.
- Step 2: `retained()` is 0, 0 and `min(k, 4)`; the reports equal FR-330's; over levels 2, 0, 2 the window clause reports `violation` at position 2.
- Step 3: the resumed reports equal the uninterrupted ones; the cancelled call leaves the bytes unchanged.
- Step 4: `report` `violation` and no `decided` after 0, 1; `decided` `success` (`decisive-witness`) after 2; FR-504-AC-1's `Completed(false)`; FR-504-AC-6's `MissingHistory`.

Test: `fsl-replay` unit tests of `ClauseState` (the code ticket adds them).
