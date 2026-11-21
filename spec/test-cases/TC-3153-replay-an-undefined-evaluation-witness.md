---
id: TC-3153
title: "Replay an undefined-evaluation witness"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: verifies
---
# TC-3153: Replay an undefined-evaluation witness

## Description

Verify that an undefined path is reproduced only when replay itself finds the
same first undefined read on a path of positive probability, under a
workload, over every scheduler and for unbounded forms.

Scope: FR-1044-AC-2 to FR-1044-AC-5.

## Test Procedure

Fixture: FR-196-AC-5's `Coin` model and its variant with `keep`, which never
deletes.

1. Replay FR-1044-AC-1's evidence with `position` 0, with the path `drop`
   with `b = 1`, with another cause, and against an item whose cause names
   another locus; settle `probability >= 0.5 [ always[0,3] holds(c.v = 1) ]`
   and replay a forged undefined path for it.
2. Over the `keep` variant under every scheduler, settle AC-1's claim; replay
   its path with the scheduler entry `keep` at the initial key.
3. Settle `probability >= 0.5 [ eventually holds(c.v = 1) ]` under the
   infinite-trace profile.
4. Replay AC-1's evidence pre-cancelled, cancelled at its final poll, with
   `replay.input_bytes` one below the request size and with each limit
   exhausted.

Tag the tests `#[trace("TC-3153", "FR-1044-AC-n")]`.

## Expected Results

- Step 1: `inconclusive`, `ReplayParity` for each forged case; the
  `c.v = 1` claim settles `refuted` with path-set evidence.
- Step 2: `refuted` with the path `drop` with `b = 0` and entry `drop`;
  `invalid_runtime_input`/`invalid-value`.
- Step 3: `refuted`, `where.position` 1, null monitor and accumulator in the
  pre-read key.
- Step 4: `incomplete` or the request refusal; no token, no refuted record.
