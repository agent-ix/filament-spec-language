---
id: TC-3153
title: "Replay an undefined-evaluation witness over the checked subject"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: verifies
---
# TC-3153: Replay an undefined-evaluation witness over the checked subject

## Description

Verify FR-202's checked-subject undefined entry: it reproduces an undefined
path only when its own control is still undecided at the read and the read is
the first undefined one, checks the witness scheduler under every scheduler,
and admits a repeated key only under a workload.

Scope: FR-202-AC-24 to FR-202-AC-26.

## Test Procedure

Fixture: FR-196-AC-5's `Coin` model and its variant with `keep`, which never
deletes.

1. Through the checked-subject entry, replay FR-202-AC-5's evidence for
   `probability >= T [ always[0,3] holds(c.v = 0) ]` at `T` 0.5 and 0.01;
   then a forged undefined path to position 1 for
   `probability >= 0.5 [ always[0,3] holds(c.v = 1) ]`.
2. Over the `keep` variant under every scheduler, replay the path `drop` with
   `b = 0` with the entry `drop`, then with the entry `keep`.
3. Under the infinite-trace profile replay the evidence for
   `probability >= 0.5 [ eventually holds(c.v = 1) ]`; then the path `drop`
   with `b = 1`, `drop` with `b = 0` under `Even` and under every scheduler
   over the `keep` variant.
4. Replay step 1's evidence pre-cancelled, cancelled at the final poll and
   with each expansion, work or storage limit exhausted.

Tag the tests `#[trace("TC-3153", "FR-202-AC-n")]`.

## Expected Results

- Step 1: a reproduced token at both thresholds with no emitted package;
  `ReplayParity` and no token for the forged path.
- Step 2: a reproduced token; `invalid_runtime_input`/`invalid-value`.
- Step 3: a reproduced token with `where` position 1 and null monitor and
  accumulator in the pre-read key; a reproduced token at position 2 under
  `Even` (probability `1/4`); `ReplayParity` under every scheduler.
- Step 4: `incomplete` with no token.
